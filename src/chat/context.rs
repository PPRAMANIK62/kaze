//! Context management for the chat REPL.
//!
//! Handles token counting display, auto-compaction decisions,
//! truncation of oldest messages, and the compaction helper.

use anyhow::Result;
use colored::Colorize;

use crate::compaction::{self, CompactionResult};
use crate::config::Config;
use crate::message::{Message, Role};
use crate::provider::Provider;
use crate::session::Session;
use crate::tokens::ContextStatus;

/// Handle token counting display and auto-compaction after a successful response.
///
/// Counts tokens across the full conversation, displays usage with appropriate
/// coloring, and triggers compaction or truncation when context limits are reached.
pub(crate) async fn handle_context_management(
    session: &mut Session,
    provider: &Provider,
    model_name: &str,
    config: &Config,
) -> Result<()> {
    // Count tokens across the full conversation
    let msg_pairs: Vec<(String, String)> = session
        .messages
        .iter()
        .map(|m| (m.role.to_string(), m.text().to_string()))
        .collect();
    let token_count = crate::tokens::count_conversation_tokens(&msg_pairs, model_name)?;
    let status = crate::tokens::check_context_usage(token_count, model_name);

    let mut already_compacted = false;
    match status {
        ContextStatus::Ok { used, limit } => {
            println!(
                "{}",
                format!("Tokens: {}", crate::tokens::format_token_usage(used, limit)).dimmed()
            );
        }
        ContextStatus::Warning {
            used,
            limit,
            percent,
        } => {
            println!(
                "{}",
                format!(
                    "Tokens: {} ({}%) -- consider /compact",
                    crate::tokens::format_token_usage(used, limit),
                    percent,
                )
                .yellow()
            );
        }
        ContextStatus::Critical {
            used,
            limit,
            percent,
        } => {
            println!(
                "{}",
                format!(
                    "Tokens: {} ({}%) -- compacting...",
                    crate::tokens::format_token_usage(used, limit),
                    percent,
                )
                .red()
            );
            match perform_compaction(
                session,
                provider,
                model_name,
                config.compaction_keep_recent(),
                "Compacted",
                "compaction",
            )
            .await
            {
                Ok(CompactionResult::Compacted { .. }) => {
                    already_compacted = true;
                }
                Ok(CompactionResult::NothingToCompact) => {
                    // Fallback to truncation if compaction has nothing to do
                    truncate_oldest_messages(&mut session.messages, model_name);
                }
                Err(_) => {
                    // Fallback to truncation if compaction fails
                    truncate_oldest_messages(&mut session.messages, model_name);
                }
            }
        }
    }

    // Auto-compaction: trigger when usage exceeds threshold
    if !already_compacted && config.compaction_auto_enabled() {
        let limit = crate::tokens::context_window_size(model_name);
        let reserved = config.compaction_reserved();
        let effective_limit = limit.saturating_sub(reserved);
        let ratio = token_count as f64 / effective_limit.max(1) as f64;
        if ratio >= config.compaction_threshold() {
            match perform_compaction(
                session,
                provider,
                model_name,
                config.compaction_keep_recent(),
                "Auto-compacted",
                "auto_compaction",
            )
            .await
            {
                Ok(CompactionResult::Compacted { .. }) => {}
                Ok(CompactionResult::NothingToCompact) => {}
                Err(e) => {
                    eprintln!(
                        "{} auto-compaction failed: {}",
                        "warning:".yellow().bold(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

/// Remove the oldest non-system messages until under 70% of context window.
pub(crate) fn truncate_oldest_messages(messages: &mut Vec<Message>, model: &str) {
    let limit = crate::tokens::context_window_size(model);
    let target = (limit as f64 * 0.70) as usize;

    while messages.len() > 1 {
        let msg_pairs: Vec<(String, String)> = messages
            .iter()
            .map(|m| (m.role.to_string(), m.text().to_string()))
            .collect();
        let used = crate::tokens::count_conversation_tokens(&msg_pairs, model).unwrap_or(0);
        if used <= target {
            break;
        }
        if let Some(pos) = messages.iter().position(|m| m.role != Role::System) {
            messages.remove(pos);
        } else {
            break;
        }
    }
}

/// Perform compaction and handle the `Compacted` result (print summary + record event).
///
/// Returns the raw `CompactionResult` so callers can handle `NothingToCompact`
/// and errors with site-specific logic.
pub(crate) async fn perform_compaction(
    session: &mut Session,
    provider: &Provider,
    model_name: &str,
    keep_recent: usize,
    label: &str,
    event_name: &str,
) -> Result<CompactionResult> {
    let result =
        compaction::compact(&mut session.messages, provider, model_name, keep_recent).await?;

    if let CompactionResult::Compacted {
        messages_removed,
        tokens_before,
        tokens_after,
    } = &result
    {
        let saved = tokens_before.saturating_sub(*tokens_after);
        eprintln!(
            "{}",
            format!(
                "{} {} messages ({} → {} tokens, saved {})",
                label,
                messages_removed,
                crate::tokens::format_number(*tokens_before),
                crate::tokens::format_number(*tokens_after),
                crate::tokens::format_number(saved),
            )
            .dimmed()
        );
        let _ = session.append_event(&serde_json::json!({
            "event": event_name,
            "messages_removed": messages_removed,
            "tokens_before": tokens_before,
            "tokens_after": tokens_after,
        }));
    }

    Ok(result)
}
