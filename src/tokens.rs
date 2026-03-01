//! Token counting for kaze.
//!
//! Uses tiktoken-rs for accurate BPE tokenization. For OpenAI models, the
//! exact tokenizer is used. For Anthropic, Ollama, and unknown models,
//! cl100k_base (GPT-4 family) serves as a reasonable approximation.

use anyhow::Result;
use tiktoken_rs::get_bpe_from_model;

/// Count tokens for a text string using the appropriate tokenizer for the model.
///
/// For OpenAI models, uses the exact BPE tokenizer.
/// For Anthropic/Ollama, falls back to cl100k_base as a reasonable approximation.
pub fn count_tokens(text: &str, model: &str) -> Result<usize> {
    let bpe = get_bpe_from_model(model).unwrap_or_else(|_| {
        tiktoken_rs::cl100k_base().expect("Failed to load cl100k_base tokenizer")
    });
    Ok(bpe.encode_ordinary(text).len())
}

/// Count tokens across all messages in a conversation.
/// Each message has ~4 tokens overhead for role markers.
pub fn count_conversation_tokens(
    messages: &[(String, String)], // (role, content) pairs
    model: &str,
) -> Result<usize> {
    let bpe = get_bpe_from_model(model).unwrap_or_else(|_| {
        tiktoken_rs::cl100k_base().expect("Failed to load cl100k_base tokenizer")
    });
    let mut total = 0;
    for (_role, content) in messages {
        total += 4; // ~4 tokens overhead per message
        total += bpe.encode_ordinary(content).len();
    }
    total += 2; // conversation framing
    Ok(total)
}

/// Format a token count for display. Example: "1,234 / 128,000"
pub fn format_token_usage(used: usize, limit: usize) -> String {
    format!("{} / {}", format_number(used), format_number(limit))
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
