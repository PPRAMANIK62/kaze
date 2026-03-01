//! Token counting for kaze.
//!
//! Uses tiktoken-rs for accurate BPE tokenization. For OpenAI models, the
//! exact tokenizer is used. For Anthropic, Ollama, and unknown models,
//! cl100k_base (GPT-4 family) serves as a reasonable approximation.

use anyhow::Result;
use tiktoken_rs::get_bpe_from_model;
use std::collections::HashMap;
use std::sync::LazyLock;

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

static CONTEXT_WINDOWS: LazyLock<HashMap<&'static str, usize>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for info in crate::models::ANTHROPIC_MODELS
        .iter()
        .chain(crate::models::OPENAI_MODELS.iter())
        .chain(crate::models::OLLAMA_MODELS.iter())
    {
        m.insert(info.name, info.context_window);
    }
    m
});


pub fn context_window_size(model: &str) -> usize {
    CONTEXT_WINDOWS
        .get(model)
        .copied()
        .unwrap_or(crate::models::DEFAULT_CONTEXT_WINDOW)
}

pub const WARN_THRESHOLD: f64 = 0.80;
pub const DANGER_THRESHOLD: f64 = 0.95;

pub enum ContextStatus {
    Ok { used: usize, limit: usize },
    Warning { used: usize, limit: usize, percent: u8 },
    Critical { used: usize, limit: usize, percent: u8 },
}

pub fn check_context_usage(used: usize, model: &str) -> ContextStatus {
    let limit = context_window_size(model);
    let ratio = used as f64 / limit as f64;
    let percent = (ratio * 100.0) as u8;
    if ratio >= DANGER_THRESHOLD {
        ContextStatus::Critical { used, limit, percent }
    } else if ratio >= WARN_THRESHOLD {
        ContextStatus::Warning { used, limit, percent }
    } else {
        ContextStatus::Ok { used, limit }
    }
}
