//! Interactive chat REPL for kaze.
//!
//! Provides a multi-turn conversation loop using [`rustyline`] for readline
//! support (history, line editing). The full conversation history is sent
//! with each request so the LLM maintains context across turns.

use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{self, Write};

use crate::config::Config;
use crate::message::Message;
use crate::output::StdoutRenderer;
use crate::provider::{Provider, ModelSelection};
use crate::format;
use crate::session::Session;
use crate::tokens::ContextStatus;

/// Runs the interactive chat REPL.
///
/// Loads config, builds the provider, and enters a readline loop where each
/// user input is appended to a [`Session`] which persists messages as JSONL. The entire
/// history is sent with each request via [`Provider::stream_with_history`]
/// so the LLM sees all prior context.
///
/// # Readline behavior
///
/// - **Ctrl+C**: cancels current input, stays in REPL
/// - **Ctrl+D**: exits cleanly with "goodbye."
/// - Readline history is persisted to `~/.cache/kaze/chat_history.txt`
///
/// # Arguments
///
/// * `config` — The loaded kaze configuration.
/// * `session_id` — Optional session ID to resume an existing session.
/// * `selection` — The resolved provider + model to use.
pub async fn run_chat(config: Config, session_id: Option<String>, selection: &ModelSelection) -> Result<()> {
    let provider = Provider::from_config(&config, selection)?;

    // Create or resume session
    let mut session = if let Some(ref id) = session_id {
        let s = Session::load(id)?;
        let short = &s.id[..8];
        println!(
            "{} [session: {}] [model: {}]",
            "resuming".bold().cyan(),
            short.yellow(),
            s.model.yellow(),
        );
        println!();
        // Display previous messages
        for msg in &s.messages {
            if msg.role == crate::message::Role::System {
                continue;
            }
            println!("{}", format::format_message(msg));
            println!();
        }
        s
    } else {
        let mut s = Session::new(&config.model)?;
        let short = &s.id[..8];
        println!(
            "{} [session: {}] [model: {}] (Ctrl+D to exit)",
            "kaze chat".bold().cyan(),
            short.yellow(),
            config.model.yellow(),
        );
        println!();
        // Add system prompt if configured
        if let Some(ref sp) = config.system_prompt {
            s.append(Message::system(sp.clone()))?;
        }
        s
    };

    // Set up readline with persistent history
    let mut rl = DefaultEditor::new()?;
    let history_path = Config::cache_dir()?.join(crate::constants::HISTORY_FILENAME);
    if history_path.exists() {
        let _ = rl.load_history(&history_path);
    }

    let model_name = config.model.clone();

    loop {
        let readline = rl.readline(&format!("{} ", ">".green().bold()));

        match readline {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }


                // Slash commands
                if line.starts_with('/') {
                    match line.as_str() {
                        "/history" => {
                            for msg in &session.messages {
                                if msg.role == crate::message::Role::System {
                                    continue;
                                }
                                println!("{}", format::format_message(msg));
                                println!();
                            }
                            continue;
                        }
                        "/clear" => {
                            session.messages.retain(|m| m.role == crate::message::Role::System);
                            println!("{}", "History cleared.".dimmed());
                            continue;
                        }
                        "/help" => {
                            println!("{}", "Commands:".bold());
                            println!("  {} - show conversation history", "/history".cyan());
                            println!("  {} - clear conversation", "/clear".cyan());
                            println!("  {} - show this help", "/help".cyan());
                            println!("  {} - exit", "Ctrl+D".cyan());
                            continue;
                        }
                        _ => {
                            println!("{} Unknown command: {}", "?".yellow(), line);
                            continue;
                        }
                    }
                }

                let _ = rl.add_history_entry(&line);

                // Add user message to session (before provider call for crash safety)
                session.append(Message::user(&line))?;
                println!();

                let mut renderer = StdoutRenderer::new();

                // Stream response
                match provider.stream_with_history(&session.messages, &mut renderer).await {
                    Ok(response) => {
                        // Erase raw streamed output and reprint with formatting
                        let total_lines = renderer.visual_line_count();
                        // Move cursor up to start of streamed content, then clear to end of screen
                        print!("\x1b[{}A\x1b[J", total_lines);
                        io::stdout().flush().ok();

                        // Reprint with markdown-lite formatting (no role label in chat)
                        println!("{}", format::render_markdown_lite(&response));
                        println!();
                        // Count tokens across the full conversation
                        session.append(Message::assistant(response.clone()))?;
                        let msg_pairs: Vec<(String, String)> = session.messages.iter()
                            .map(|m| (m.role.to_string(), m.text().to_string()))
                            .collect();
                        let token_count = crate::tokens::count_conversation_tokens(&msg_pairs, &model_name)?;
                        let status = crate::tokens::check_context_usage(token_count, &model_name);

                        match status {
                            ContextStatus::Ok { used, limit } => {
                                println!(
                                    "{}",
                                    format!("Tokens: {}", crate::tokens::format_token_usage(used, limit)).dimmed()
                                );
                            }
                            ContextStatus::Warning { used, limit, percent } => {
                                println!(
                                    "{}",
                                    format!(
                                        "Tokens: {} ({}%) -- consider /compact",
                                        crate::tokens::format_token_usage(used, limit),
                                        percent,
                                    ).yellow()
                                );
                            }
                            ContextStatus::Critical { used, limit, percent } => {
                                println!(
                                    "{}",
                                    format!(
                                        "Tokens: {} ({}%) -- compacting...",
                                        crate::tokens::format_token_usage(used, limit),
                                        percent,
                                    ).red()
                                );
                                truncate_oldest_messages(&mut session.messages, &model_name);
                            }
                        }
                    }
                    Err(e) => {
                        // Pop the failed user message so user can retry
                        session.messages.pop();
                        eprintln!("{} {}", "error:".red().bold(), e);
                    }
                }
                println!();
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".dimmed());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "goodbye.".dimmed());
                break;
            }
            Err(e) => {
                eprintln!("{} {}", "error:".red().bold(), e);
                break;
            }
        }
    }

    // Save readline history
    if let Some(parent) = history_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = rl.save_history(&history_path);

    Ok(())
}

/// Remove the oldest non-system messages until under 70% of context window.
fn truncate_oldest_messages(messages: &mut Vec<Message>, model: &str) {
    let limit = crate::tokens::context_window_size(model);
    let target = (limit as f64 * 0.70) as usize;

    while messages.len() > 1 {
        let msg_pairs: Vec<(String, String)> = messages.iter()
            .map(|m| (m.role.to_string(), m.text().to_string()))
            .collect();
        let used = crate::tokens::count_conversation_tokens(&msg_pairs, model)
            .unwrap_or(0);
        if used <= target {
            break;
        }
        if let Some(pos) = messages.iter().position(|m| m.role != crate::message::Role::System) {
            messages.remove(pos);
        } else {
            break;
        }
    }
}
