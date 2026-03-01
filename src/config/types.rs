//! Struct definitions and serde defaults for kaze configuration.

use crate::permissions::PermissionConfig;
use serde::{Deserialize, Serialize};

/// Root configuration for kaze, deserialized from `config.toml`.
///
/// Fields use serde defaults so kaze can run with sensible defaults
/// when no config file exists.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Default model identifier (e.g. `"claude-sonnet-4-5"`).
    #[serde(default = "default_model")]
    pub model: String,
    /// Per-provider settings.
    #[serde(default)]
    pub provider: ProviderConfig,
    /// Default provider name (e.g., "anthropic", "openai").
    #[serde(default)]
    pub default_provider: Option<String>,
    /// Optional system prompt prepended to all conversations.
    #[serde(default = "default_system_prompt")]
    pub system_prompt: Option<String>,
    /// Context compaction settings.
    #[serde(default)]
    pub compaction: CompactionConfig,
    /// Permission settings for tool execution.
    #[serde(default)]
    pub permissions: PermissionConfig,
}

/// Returns the default model identifier (`"claude-sonnet-4-5"`).
///
/// Used by serde's `#[serde(default)]` attribute during deserialization.
pub(super) fn default_model() -> String {
    crate::constants::DEFAULT_MODEL.to_string()
}

/// Returns the default system prompt for new conversations.
///
/// Used by serde's `#[serde(default)]` attribute during deserialization
/// so configs without an explicit `system_prompt` still get a sensible default.
fn default_system_prompt() -> Option<String> {
    Some(crate::constants::DEFAULT_SYSTEM_PROMPT.to_string())
}

/// Provider-specific configuration map.
///
/// Each field corresponds to a supported LLM provider. Only providers
/// the user has configured will be `Some`.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProviderConfig {
    /// Configuration for the OpenAI API provider.
    pub openai: Option<ProviderEntry>,
    /// Configuration for the Anthropic API provider.
    pub anthropic: Option<ProviderEntry>,
    /// Configuration for the local Ollama provider.
    pub ollama: Option<ProviderEntry>,
    /// Configuration for the OpenRouter API provider.
    pub openrouter: Option<ProviderEntry>,
}

/// Connection details for a single LLM provider.
///
/// Allows overriding the API key, endpoint URL, and model on a
/// per-provider basis.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderEntry {
    /// API key for authentication. Can also be set via environment variables.
    pub api_key: Option<String>,
    /// Custom base URL for the provider's API (useful for proxies or self-hosted instances).
    pub base_url: Option<String>,
    /// Model identifier to use with this provider, overriding the global default.
    pub model: Option<String>,
}

/// Configuration for LLM-based context compaction.
///
/// Controls when and how kaze summarizes old conversation messages
/// to free up context window space.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct CompactionConfig {
    /// Threshold ratio (0.0–1.0) at which auto-compaction triggers.
    pub auto_threshold: Option<f64>,
    /// Whether automatic compaction is enabled.
    pub auto: Option<bool>,
    /// Number of most-recent messages to preserve during compaction.
    pub keep_recent: Option<usize>,
    /// Reserved token budget for the compaction summary itself.
    pub reserved: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: default_model(),
            provider: ProviderConfig::default(),
            system_prompt: default_system_prompt(),
            default_provider: None,
            compaction: CompactionConfig::default(),
            permissions: PermissionConfig::default(),
        }
    }
}
