//! LLM-based context compaction for kaze.
//!
//! When the conversation context window fills up, compaction summarizes
//! older messages into a single system-level summary, preserving key
//! decisions and technical details while freeing token budget.

use anyhow::{Context, Result};

use crate::message::Message;
use crate::provider::Provider;
use crate::tokens;

use crate::constants::COMPACTION_PROMPT;

/// Result of a compaction attempt.
pub enum CompactionResult {
    /// Not enough messages to compact (only system + recent remain).
    NothingToCompact,
    /// Successfully compacted older messages into a summary.
    Compacted {
        /// Number of messages removed and replaced by the summary.
        messages_removed: usize,
        /// Approximate token count before compaction.
        tokens_before: usize,
        /// Approximate token count after compaction.
        tokens_after: usize,
    },
}

/// Compact older messages in the conversation by summarizing them via the LLM.
///
/// Keeps the system prompt (index 0) and the most recent `keep_recent`
/// messages intact. Everything in between is summarized into a single
/// system message containing `[Previous context summary]: ...`.
///
/// # Arguments
///
/// * `messages` — Mutable conversation history. Modified in-place.
/// * `provider` — The configured LLM provider for generating the summary.
/// * `model` — Model name used for token counting.
/// * `keep_recent` — Number of most-recent messages to preserve.
pub async fn compact(
    messages: &mut Vec<Message>,
    provider: &Provider,
    model: &str,
    keep_recent: usize,
) -> Result<CompactionResult> {
    // Need at least: system prompt + something to compact + keep_recent messages
    if messages.len() <= 1 + keep_recent {
        return Ok(CompactionResult::NothingToCompact);
    }

    // Count tokens before compaction
    let msg_pairs_before: Vec<(String, String)> = messages
        .iter()
        .map(|m| (m.role.to_string(), m.text().to_string()))
        .collect();
    let tokens_before =
        tokens::count_conversation_tokens(&msg_pairs_before, model).unwrap_or(0);

    // Identify the range to compact: everything between system prompt and recent messages
    let compact_end = messages.len().saturating_sub(keep_recent);
    if compact_end <= 1 {
        return Ok(CompactionResult::NothingToCompact);
    }

    // Build text blob from old messages (indices 1..compact_end)
    let mut text_blob = String::new();
    for msg in &messages[1..compact_end] {
        text_blob.push_str(&format!("[{}]: {}\n\n", msg.role, msg.text()));
    }

    // Ask the LLM to summarize
    let prompt_text = format!("{}{}", COMPACTION_PROMPT, text_blob);
    let summary = provider
        .prompt(&prompt_text)
        .await
        .context("Failed to generate compaction summary")?;

    let messages_removed = compact_end - 1;

    // Replace old messages with summary: drain 1..compact_end and insert summary
    messages.drain(1..compact_end);
    messages.insert(
        1,
        Message::system(format!("[Previous context summary]: {}", summary)),
    );

    // Count tokens after compaction
    let msg_pairs_after: Vec<(String, String)> = messages
        .iter()
        .map(|m| (m.role.to_string(), m.text().to_string()))
        .collect();
    let tokens_after =
        tokens::count_conversation_tokens(&msg_pairs_after, model).unwrap_or(0);

    Ok(CompactionResult::Compacted {
        messages_removed,
        tokens_before,
        tokens_after,
    })
}
