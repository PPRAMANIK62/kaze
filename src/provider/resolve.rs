//! Model resolution logic for kaze.
//!
//! Resolves which provider and model to use based on CLI flags, config file,
//! and hardcoded defaults. Supports `provider/model` shorthand syntax.

use anyhow::Result;

use super::kind::{default_model_for, ProviderKind};
use crate::config::Config;

use crate::constants::DEFAULT_PROVIDER;

/// Resolved provider + model pair.
pub struct ModelSelection {
    pub provider: ProviderKind,
    pub model: String,
}

/// Resolve which provider and model to use.
/// Priority: CLI flags > config.toml > defaults.
///
/// Accepts these formats:
///   --model anthropic/claude-sonnet-4-5  (provider/model shorthand, only when --provider is omitted)
///   --provider openrouter --model "org/model-name"  (slash preserved as model name)
///   --provider anthropic --model claude-sonnet-4-5
///   --provider anthropic  (uses provider's default model)
///   (nothing)  (uses config.toml, then hardcoded default)
pub fn resolve_model(
    cli_provider: Option<&str>,
    cli_model: Option<&str>,
    config: &Config,
) -> Result<ModelSelection> {
    // If --model contains a slash AND no explicit --provider, parse as provider/model shorthand
    if cli_provider.is_none() {
        if let Some(model_str) = cli_model {
            if let Some((prov, model)) = model_str.split_once('/') {
                return Ok(ModelSelection {
                    provider: ProviderKind::from_str(prov)?,
                    model: model.to_string(),
                });
            }
        }
    }

    // Resolve provider
    let provider_str = cli_provider
        .or(config.provider_name())
        .unwrap_or(DEFAULT_PROVIDER);
    let provider = ProviderKind::from_str(provider_str)?;

    // Resolve model
    let model = cli_model
        .map(String::from)
        .or_else(|| config.model_name())
        .unwrap_or_else(|| default_model_for(&provider).to_string());

    Ok(ModelSelection { provider, model })
}
