//! Configuration types and path resolution for kaze.
//!
//! Kaze stores its settings as TOML at the platform's XDG config path
//! (e.g. `~/.config/kaze/config.toml` on Linux) and session data under the
//! XDG data directory (`~/.local/share/kaze/`).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    /// Optional system prompt prepended to all conversations.
    #[serde(default)]
    pub system_prompt: Option<String>,
}

/// Returns the default model identifier (`"claude-sonnet-4-5"`).
///
/// Used by serde's `#[serde(default)]` attribute during deserialization.
fn default_model() -> String {
    "claude-sonnet-4-5".to_string()
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

impl Default for Config {
    fn default() -> Self {
        Self {
            model: default_model(),
            provider: ProviderConfig::default(),
            system_prompt: None,
        }
    }
}

impl Config {
    /// Load config with precedence: project > global > defaults.
    /// Creates default config file if none exists.
    pub fn load() -> Result<Self> {
        let global = Self::load_global()?;
        let project = Self::load_project()?;

        let mut config = global;
        if let Some(proj) = project {
            config = Self::merge(config, proj);
        }

        config.resolve_substitutions();
        Ok(config)
    }

    /// Loads the global config from `~/.config/kaze/config.toml`.
    ///
    /// If no config file exists, creates one with sensible defaults
    /// (including `{env:VAR}` placeholders for API keys) and returns it.
    fn load_global() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let default_toml = format!(
                r#"model = "{}"

[provider]

[provider.anthropic]
api_key = "{{env:ANTHROPIC_API_KEY}}"

[provider.openai]
api_key = "{{env:OPENAI_API_KEY}}"
"#,
                default_model()
            );
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, &default_toml)
                .with_context(|| format!("Failed to write default config to {:?}", path))?;
            let config: Config = toml::from_str(&default_toml)
                .with_context(|| "Failed to parse default config".to_string())?;
            return Ok(config);
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config at {:?}", path))?;
        Ok(config)
    }

    /// Look for kaze.toml in current dir, then walk up to git root.
    fn load_project() -> Result<Option<Config>> {
        let mut dir = std::env::current_dir()?;
        loop {
            let candidate = dir.join("kaze.toml");
            if candidate.exists() {
                let contents = fs::read_to_string(&candidate)?;
                let config: Config = toml::from_str(&contents)?;
                return Ok(Some(config));
            }
            // Stop at git root or filesystem root
            if dir.join(".git").exists() || !dir.pop() {
                break;
            }
        }
        Ok(None)
    }

    /// Merge project config over global config.
    /// Project values win when present.
    fn merge(global: Config, project: Config) -> Config {
        Config {
            model: if project.model != default_model() {
                project.model
            } else {
                global.model
            },
            provider: global.provider, // TODO: deep merge providers
            system_prompt: project.system_prompt.or(global.system_prompt),
        }
    }

    /// Resolve {env:VAR_NAME} patterns in string fields.
    fn resolve_substitutions(&mut self) {
        self.model = Self::resolve_str(&self.model);
        if let Some(ref mut sp) = self.system_prompt {
            *sp = Self::resolve_str(sp);
        }
        Self::resolve_provider_entry(&mut self.provider.openai);
        Self::resolve_provider_entry(&mut self.provider.anthropic);
        Self::resolve_provider_entry(&mut self.provider.ollama);
    }

    /// Resolves `{env:VAR}` patterns in a single provider entry's `api_key` and `base_url`.
    fn resolve_provider_entry(entry: &mut Option<ProviderEntry>) {
        if let Some(ref mut e) = entry {
            if let Some(ref mut key) = e.api_key {
                *key = Self::resolve_str(key);
            }
            if let Some(ref mut url) = e.base_url {
                *url = Self::resolve_str(url);
            }
        }
    }

    /// Replace {env:VAR} with the environment variable value.
    fn resolve_str(s: &str) -> String {
        let mut result = s.to_string();
        while let Some(start) = result.find("{env:") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 5..start + end];
                let value = std::env::var(var_name).unwrap_or_default();
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    value,
                    &result[start + end + 1..]
                );
            } else {
                break;
            }
        }
        result
    }

    /// Resolve API key for a provider: env var first, then config value.
    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        // Check env var first (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)
        let env_key = format!("{}_API_KEY", provider.to_uppercase());
        if let Ok(val) = std::env::var(&env_key) {
            if !val.is_empty() {
                return Some(val);
            }
        }

        // Fall back to config
        let entry = match provider {
            "openai" => &self.provider.openai,
            "anthropic" => &self.provider.anthropic,
            "ollama" => &self.provider.ollama,
            _ => &None,
        };
        entry.as_ref().and_then(|e| e.api_key.clone())
    }

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
