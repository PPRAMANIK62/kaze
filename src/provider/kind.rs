//! Provider kind enumeration and default model mapping.
//!
//! Defines [`ProviderKind`] which identifies which LLM backend to use,
//! and [`default_model_for`] which returns the default model for each provider.

use anyhow::{anyhow, Result};

/// Identifies which LLM provider to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// Anthropic (Claude models).
    Anthropic,
    /// OpenAI (GPT models, via the Responses API).
    OpenAI,
    /// OpenRouter (multi-provider gateway).
    OpenRouter,
    /// Ollama (local models via OpenAI-compatible API).
    Ollama,
}

impl ProviderKind {
    /// Parses a provider name string into a [`ProviderKind`].
    ///
    /// Matching is case-insensitive. Returns an error for unknown providers.
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic),
            "openai" => Ok(Self::OpenAI),
            "openrouter" => Ok(Self::OpenRouter),
            "ollama" => Ok(Self::Ollama),
            other => Err(anyhow!(
                "Unknown provider: {other}. Supported: anthropic, openai, openrouter, ollama"
            )),
        }
    }
}

/// Returns the default model identifier for a given provider.
pub fn default_model_for(provider: &ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Anthropic => crate::constants::DEFAULT_MODEL,
        ProviderKind::OpenAI => crate::constants::DEFAULT_OPENAI_MODEL,
        ProviderKind::OpenRouter => crate::constants::DEFAULT_OPENROUTER_MODEL,
        ProviderKind::Ollama => crate::constants::OLLAMA_DEFAULT_MODEL,
    }
}
