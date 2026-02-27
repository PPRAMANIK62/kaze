//! Configuration types and path resolution for kaze.
//!
//! Kaze stores its settings as TOML at the platform's XDG config path
//! (e.g. `~/.config/kaze/config.toml` on Linux) and session data under the
//! XDG data directory (`~/.local/share/kaze/`).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Root configuration for kaze, deserialized from `config.toml`.
///
/// All fields are optional so kaze can run with sensible defaults
/// when no config file exists.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    /// Default model identifier (e.g. `"gpt-4o"`) used when no provider-specific model is set.
    pub model: Option<String>,
    /// Per-provider settings. When `None`, provider defaults are used.
    pub provider: Option<ProviderConfig>,
}

/// Provider-specific configuration map.
///
/// Each field corresponds to a supported LLM provider. Only providers
/// the user has configured will be `Some`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    /// Configuration for the OpenAI API provider.
    pub openai: Option<ProviderEntry>,
    /// Configuration for the Anthropic API provider.
    pub anthropic: Option<ProviderEntry>,
    /// Configuration for the local Ollama provider.
    pub ollama: Option<ProviderEntry>,
}

/// Connection details for a single LLM provider.
///
/// Allows overriding the API key, endpoint URL, and model on a
/// per-provider basis.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderEntry {
    /// API key for authentication. Can also be set via environment variables.
    pub api_key: Option<String>,
    /// Custom base URL for the provider's API (useful for proxies or self-hosted instances).
    pub base_url: Option<String>,
    /// Model identifier to use with this provider, overriding the global default.
    pub model: Option<String>,
}

impl Config {
    /// Returns the platform-specific configuration directory for kaze.
    ///
    /// Returns `~/.config/kaze/` on Linux (`XDG_CONFIG_HOME/kaze`).
    ///
    /// # Errors
    ///
    /// Returns an error if the platform's config directory cannot be determined.
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("kaze");
        Ok(dir)
    }

    /// Returns the platform-specific data directory for kaze.
    ///
    /// Returns `~/.local/share/kaze/` on Linux (`XDG_DATA_HOME/kaze`).
    /// Used for storing session history and other persistent data.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform's data directory cannot be determined.
    pub fn data_dir() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
            .join("kaze");
        Ok(dir)
    }

    /// Returns the full path to the kaze configuration file.
    ///
    /// Returns `~/.config/kaze/config.toml` on Linux.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Config::config_dir`] fails.
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }
}
