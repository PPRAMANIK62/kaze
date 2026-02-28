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
use crate::provider::Provider;
use crate::format;

/// Runs the interactive chat REPL.
///
/// Loads config, builds the provider, and enters a readline loop where each
/// user input is appended to a growing `Vec<Message>` history. The entire
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
/// * `_session` — Reserved for future session resume support.
pub async fn run_chat(config: Config, _session: Option<String>) -> Result<()> {
    let provider = Provider::from_config(&config)?;

    let mut history: Vec<Message> = Vec::new();

    // Add system prompt to history if configured
    if let Some(ref sp) = config.system_prompt {
        history.push(Message::system(sp.clone()));
    }

    println!(
        "{} [model: {}] (Ctrl+D to exit)",
        "kaze chat".bold().cyan(),
        config.model.yellow(),
    );
    println!();

    // Set up readline with persistent history
    let mut rl = DefaultEditor::new()?;
    let history_path = Config::cache_dir()?.join(crate::constants::HISTORY_FILENAME);
    if history_path.exists() {
        let _ = rl.load_history(&history_path);
    }

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
                            for msg in &history {
                                if msg.role == crate::message::Role::System {
                                    continue;
                                }
                                println!("{}", format::format_message(msg));
                                println!();
                            }
                            continue;
                        }
                        "/clear" => {
                            history.retain(|m| m.role == crate::message::Role::System);
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

                // Add user message to history
                history.push(Message::user(&line));
                println!();

                let mut renderer = StdoutRenderer::new();

                // Stream response
                match provider.stream_with_history(&history, &mut renderer).await {
                    Ok(response) => {
                        // Erase raw streamed output and reprint with formatting
                        let total_lines = renderer.visual_line_count();
                        // Move cursor up to start of streamed content, then clear to end of screen
                        print!("\x1b[{}A\x1b[J", total_lines);
                        io::stdout().flush().ok();

                        // Reprint with markdown-lite formatting (no role label in chat)
                        println!("{}", format::render_markdown_lite(&response));
                        println!();
                        println!(
                            "{}",
                            format!("[{} tokens]", renderer.token_count()).dimmed()
                        );

                        history.push(Message::assistant(response));
                    }
                    Err(e) => {
                        // Pop the failed user message so user can retry
                        history.pop();
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
