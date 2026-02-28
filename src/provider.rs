//! LLM provider abstraction for kaze.
//!
//! Wraps rig-core's Anthropic client behind a [`Provider`] struct, keeping
//! provider-specific details out of the CLI layer.

use anyhow::{Context, Result};
use rig::completion::Prompt;
use rig::client::CompletionClient;
use rig::providers::anthropic;

use crate::config::Config;

/// A configured LLM provider ready to handle completion requests.
///
/// Wraps a rig-core [`anthropic::Client`] and the target model name.
/// Agents are constructed on each [`complete()`](Provider::complete) call
/// since they are cheap to create and may use different system prompts.
pub struct Provider {
    client: anthropic::Client,
    model: String,
}

impl Provider {
    /// Creates a new [`Provider`] from the loaded application config.
    ///
    /// Resolves the API key through kaze's config precedence chain
    /// (env var → config file → substitution) and builds an Anthropic client.
    ///
    /// # Errors
    ///
    /// Returns an error if no API key is found for the `"anthropic"` provider.
    pub fn from_config(config: &Config) -> Result<Self> {
        let api_key = config
            .resolve_api_key("anthropic")
            .context("No API key found for Anthropic. Set ANTHROPIC_API_KEY or configure it in config.toml")?;

        let client = anthropic::Client::new(&api_key).context("Failed to create Anthropic client")?;

        Ok(Self {
            client,
            model: config.model.clone(),
        })
    }

    /// Sends a prompt to the configured model and returns the full response.
    ///
    /// Builds a fresh agent for each call, optionally attaching a system
    /// prompt as the agent's preamble.
    ///
    /// # Arguments
    ///
    /// * `prompt` — The user's message to send to the model.
    /// * `system_prompt` — An optional system-level instruction prepended
    ///   to the conversation.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM API call fails (network error,
    /// invalid key, rate limit, etc.).
    pub async fn complete(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String> {
        let response = if let Some(sys) = system_prompt {
            let agent = self.client
                .agent(&self.model)
                .preamble(sys)
                .max_tokens(4096)
                .build();
            agent.prompt(prompt).await.context("LLM API call failed")?
        } else {
            let agent = self.client
                .agent(&self.model)
                .max_tokens(4096)
                .build();
            agent.prompt(prompt).await.context("LLM API call failed")?
        };

        Ok(response)
    }
}
