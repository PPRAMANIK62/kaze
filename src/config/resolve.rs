//! Environment variable substitution and API key resolution.

use super::types::{Config, ProviderEntry};

use crate::constants::{COMPACTION_AUTO_DEFAULT, COMPACTION_THRESHOLD_DEFAULT, COMPACTION_KEEP_RECENT_DEFAULT, COMPACTION_RESERVED_DEFAULT};

impl Config {
    /// Resolve {env:VAR_NAME} patterns in string fields.
    pub(super) fn resolve_substitutions(&mut self) {
        self.model = Self::resolve_str(&self.model);
        if let Some(ref mut sp) = self.system_prompt {
            *sp = Self::resolve_str(sp);
        }
        if let Some(ref mut dp) = self.default_provider {
            *dp = Self::resolve_str(dp);
        }
        Self::resolve_provider_entry(&mut self.provider.openai);
        Self::resolve_provider_entry(&mut self.provider.anthropic);
        Self::resolve_provider_entry(&mut self.provider.ollama);
        Self::resolve_provider_entry(&mut self.provider.openrouter);
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
            "openrouter" => &self.provider.openrouter,
            _ => &None,
        };
        entry.as_ref().and_then(|e| e.api_key.clone())
    }

    /// Get the configured default provider name, if any.
    pub fn provider_name(&self) -> Option<&str> {
        self.default_provider.as_deref()
    }

    /// Get the model name from config, stripping provider prefix if present.
    /// Returns None if the model is the compile-time default (meaning user hasn't configured it).
    pub fn model_name(&self) -> Option<String> {
        let m = &self.model;
        if m == crate::constants::DEFAULT_MODEL {
            return None; // treat default as "not configured"
        }
        // If model contains slash, extract just the model part
        if let Some((_prov, model)) = m.split_once('/') {
            Some(model.to_string())
        } else {
            Some(m.to_string())
        }
    }

    /// Whether automatic context compaction is enabled.
    pub fn compaction_auto_enabled(&self) -> bool {
        self.compaction.auto.unwrap_or(COMPACTION_AUTO_DEFAULT)
    }

    /// Usage ratio at which auto-compaction triggers.
    pub fn compaction_threshold(&self) -> f64 {
        self.compaction.auto_threshold.unwrap_or(COMPACTION_THRESHOLD_DEFAULT)
    }

    /// Number of recent messages to keep during compaction.
    pub fn compaction_keep_recent(&self) -> usize {
        self.compaction.keep_recent.unwrap_or(COMPACTION_KEEP_RECENT_DEFAULT)
    }

    /// Reserved token budget for the compaction summary.
    pub fn compaction_reserved(&self) -> usize {
        self.compaction.reserved.unwrap_or(COMPACTION_RESERVED_DEFAULT)
    }
}
