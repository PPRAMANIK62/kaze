//! Interactive chat REPL for kaze.
//!
//! Provides a multi-turn conversation loop using [`rustyline`] for readline
//! support (history, line editing). The full conversation history is sent
//! with each request so the LLM maintains context across turns.

use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::config::Config;
use crate::message::Message;
use crate::output::StdoutRenderer;
use crate::provider::Provider;

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
    let history_path = Config::cache_dir()?.join("chat_history.txt");
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

                let _ = rl.add_history_entry(&line);

                // Add user message to history
                history.push(Message::user(&line));
                println!();

                let mut renderer = StdoutRenderer::new();

                // Stream response
                match provider.stream_with_history(&history, &mut renderer).await {
                    Ok(response) => {
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
