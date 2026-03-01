//! Slash command handlers for the chat REPL.
//!
//! Dispatches `/history`, `/clear`, `/help`, and `/compact` commands.
//! Returns a [`CommandAction`] so the REPL loop can decide how to proceed.

use anyhow::Result;
use colored::Colorize;

use crate::compaction::CompactionResult;
use crate::format;
use crate::message::Role;
use crate::provider::Provider;
use crate::session::Session;

use super::context;

/// Action returned by slash command handling.
pub(crate) enum CommandAction {
    /// Command was handled successfully; continue the REPL loop.
    Continue,
    /// Unknown command was entered.
    Unknown(String),
}

/// Dispatch and handle a slash command.
///
/// Matches the input against known commands and executes the appropriate
/// handler. Returns [`CommandAction::Unknown`] for unrecognized commands.
pub(crate) async fn handle_slash_command(
    command: &str,
    session: &mut Session,
    provider: &Provider,
    model_name: &str,
    keep_recent: usize,
) -> Result<CommandAction> {
    match command {
        "/history" => {
            for msg in &session.messages {
                if msg.role == Role::System {
                    continue;
                }
                println!("{}", format::format_message(msg));
                println!();
            }
            Ok(CommandAction::Continue)
        }
        "/clear" => {
            session.messages.retain(|m| m.role == Role::System);
            println!("{}", "History cleared.".dimmed());
            Ok(CommandAction::Continue)
        }
        "/help" => {
            println!("{}", "Commands:".bold());
            println!("  {} - show conversation history", "/history".cyan());
            println!("  {} - clear conversation", "/clear".cyan());
            println!(
                "  {} - summarize old context to free tokens",
                "/compact".cyan()
            );
            println!("  {} - show this help", "/help".cyan());
            println!("  {} - exit", "Ctrl+D".cyan());
            Ok(CommandAction::Continue)
        }
        "/compact" => {
            match context::perform_compaction(
                session,
                provider,
                model_name,
                keep_recent,
                "Compacted",
                "compaction",
            )
            .await
            {
                Ok(CompactionResult::NothingToCompact) => {
                    eprintln!("{}", "Nothing to compact.".dimmed());
                }
                Ok(CompactionResult::Compacted { .. }) => {}
                Err(e) => {
                    eprintln!("{} compaction failed: {}", "error:".red().bold(), e);
                }
            }
            Ok(CommandAction::Continue)
        }
        _ => Ok(CommandAction::Unknown(command.to_string())),
    }
}
