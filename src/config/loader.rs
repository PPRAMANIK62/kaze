//! File loading and merging for kaze configuration.

use anyhow::{Context, Result};
use std::fs;

use super::types::{default_model, CompactionConfig, Config};

impl Config {
    /// Loads the global config from `~/.config/kaze/config.toml`.
    ///
    /// If no config file exists, creates one with sensible defaults
    /// (including `{env:VAR}` placeholders for API keys) and returns it.
    pub(super) fn load_global() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let default_toml = format!(
                r#"model = "{}"

[provider]

[provider.anthropic]
api_key = "{{env:ANTHROPIC_API_KEY}}"

[provider.openai]
api_key = "{{env:OPENAI_API_KEY}}"

[provider.openrouter]
api_key = "{{env:OPENROUTER_API_KEY}}"

[provider.ollama]
base_url = "http://localhost:11434"
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
    pub(super) fn load_project() -> Result<Option<Config>> {
        let mut dir = std::env::current_dir()?;
        loop {
            let candidate = dir.join(crate::constants::PROJECT_CONFIG_FILENAME);
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
    pub(super) fn merge(global: Config, project: Config) -> Config {
        Config {
            model: if project.model != default_model() {
                project.model
            } else {
                global.model
            },
            provider: global.provider, // TODO: deep merge providers
            system_prompt: project.system_prompt.or(global.system_prompt),
            default_provider: project.default_provider.or(global.default_provider),
            compaction: CompactionConfig {
                auto_threshold: project
                    .compaction
                    .auto_threshold
                    .or(global.compaction.auto_threshold),
                auto: project.compaction.auto.or(global.compaction.auto),
                keep_recent: project
                    .compaction
                    .keep_recent
                    .or(global.compaction.keep_recent),
                reserved: project.compaction.reserved.or(global.compaction.reserved),
            },
        }
    }
}
