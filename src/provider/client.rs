//! LLM provider client and streaming implementation.
//!
//! Contains the [`Provider`] struct which wraps rig-core provider clients
//! behind enum dispatch, keeping provider-specific details out of the CLI
//! layer. Supports Anthropic, OpenAI, OpenRouter, and Ollama.

use anyhow::{Context, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::message::{Message as RigMessage, Text};
use rig::providers::{anthropic, openai, openrouter};
use rig::streaming::{StreamedAssistantContent, StreamingChat, StreamingPrompt};

use super::kind::ProviderKind;
use super::resolve::ModelSelection;
use crate::config::Config;
use crate::output::Renderer;

/// Internal enum wrapping provider-specific clients.
enum ClientKind {
    Anthropic(anthropic::Client),
    OpenAI(openai::Client),
    OpenRouter(openrouter::Client),
    Ollama(openai::Client),
}

/// A configured LLM provider ready to handle completion requests.
///
/// Wraps a rig-core provider client and the target model name. Supports
/// Anthropic, OpenAI, OpenRouter, and Ollama via internal enum dispatch. Agents are
/// constructed on each call since they are cheap to create and may use
/// different system prompts.
pub struct Provider {
    client: ClientKind,
    model: String,
}

/// Helper macro to reduce duplication across provider match arms.
///
/// Builds an agent from the given client, model, and optional system prompt,
/// then executes the provided block with the agent bound to `$agent`.
macro_rules! with_agent {
    ($client:expr, $model:expr, $sys:expr, |$agent:ident| $body:expr) => {{
        let $agent = if let Some(sys) = $sys {
            $client
                .agent($model)
                .preamble(sys)
                .max_tokens(crate::constants::MAX_TOKENS)
                .build()
        } else {
            $client
                .agent($model)
                .max_tokens(crate::constants::MAX_TOKENS)
                .build()
        };
        $body
    }};
}

/// Dispatches an operation across provider-specific clients.
///
/// Matches on [`ClientKind`] and executes the same block for each variant,
/// letting the compiler monomorphize per provider.
macro_rules! dispatch {
    ($self:expr, |$client:ident| $body:expr) => {
        match &$self.client {
            ClientKind::Anthropic($client) => $body,
            ClientKind::OpenAI($client) => $body,
            ClientKind::OpenRouter($client) => $body,
            ClientKind::Ollama($client) => $body,
        }
    };
}

/// Processes a streaming response, rendering tokens and accumulating the full text.
///
/// Handles text chunks, final responses, errors, and unknown items uniformly
/// across all providers.
macro_rules! process_stream {
    ($stream:expr, $renderer:expr, $full_response:expr) => {
        while let Some(chunk) = $stream.next().await {
            match chunk {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                    Text { text },
                ))) => {
                    $renderer.render_token(&text);
                    $full_response.push_str(&text);
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    // Stream complete
                }
                Err(err) => {
                    $renderer.render_error(&err.to_string());
                    anyhow::bail!("Streaming error: {}", err);
                }
                _ => {
                    // Tool calls, reasoning, etc. -- handled in later phases
                }
            }
        }
    };
}

impl Provider {
    /// Creates a new [`Provider`] from the loaded application config.
    ///
    /// Resolves the API key through kaze's config precedence chain
    /// (env var → config file → substitution) and builds the appropriate
    /// provider client. Defaults to Anthropic when no provider is specified.
    ///
    /// # Errors
    ///
    /// Returns an error if no API key is found for the selected provider
    /// or if client construction fails.
    pub fn from_config(config: &Config, selection: &ModelSelection) -> Result<Self> {
        match selection.provider {
            ProviderKind::Anthropic => {
                let api_key = config
                    .resolve_api_key("anthropic")
                    .context("No API key found for Anthropic. Set ANTHROPIC_API_KEY or configure it in config.toml")?;
                let client = anthropic::Client::new(&api_key)
                    .context("Failed to create Anthropic client")?;
                Ok(Self {
                    client: ClientKind::Anthropic(client),
                    model: selection.model.clone(),
                })
            }
            ProviderKind::OpenAI => {
                let api_key = config
                    .resolve_api_key("openai")
                    .context("No API key found for OpenAI. Set OPENAI_API_KEY or configure it in config.toml")?;
                let client =
                    openai::Client::new(&api_key).context("Failed to create OpenAI client")?;
                Ok(Self {
                    client: ClientKind::OpenAI(client),
                    model: selection.model.clone(),
                })
            }
            ProviderKind::OpenRouter => {
                let api_key = config
                    .resolve_api_key("openrouter")
                    .context("No API key found for OpenRouter. Set OPENROUTER_API_KEY or configure it in config.toml")?;
                let client = openrouter::Client::new(&api_key)
                    .context("Failed to create OpenRouter client")?;
                Ok(Self {
                    client: ClientKind::OpenRouter(client),
                    model: selection.model.clone(),
                })
            }
            ProviderKind::Ollama => {
                let base_url = config
                    .provider
                    .ollama
                    .as_ref()
                    .and_then(|o| o.base_url.as_deref())
                    .unwrap_or(crate::constants::OLLAMA_DEFAULT_BASE_URL);
                let client = openai::Client::builder()
                    .api_key("ollama")
                    .base_url(&format!("{}/v1", base_url))
                    .build()
                    .context("Failed to create Ollama client")?;
                Ok(Self {
                    client: ClientKind::Ollama(client),
                    model: selection.model.clone(),
                })
            }
        }
    }

    /// Streams a prompt response, rendering tokens as they arrive via the given [`Renderer`].
    ///
    /// Builds a fresh agent, opens a streaming connection to the LLM, and
    /// forwards each text delta to `renderer`. Returns the full accumulated
    /// response text for later use (e.g. message history, session persistence).
    ///
    /// # Arguments
    ///
    /// * `prompt` — The user's message to send to the model.
    /// * `system_prompt` — An optional system-level instruction prepended to the conversation.
    /// * `renderer` — A [`Renderer`] implementation that displays tokens as they arrive.
    ///
    /// # Errors
    ///
    /// Returns an error if a streaming chunk fails (network error, invalid key, etc.).
    pub async fn stream(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        renderer: &mut dyn Renderer,
    ) -> Result<String> {
        let mut full_response = String::new();

        dispatch!(self, |client| {
            let mut stream = with_agent!(client, &self.model, system_prompt, |agent| {
                agent.stream_prompt(prompt).await
            });
            process_stream!(stream, renderer, full_response);
        });

        renderer.render_done();
        Ok(full_response)
    }

    /// Streams a response with full conversation history for multi-turn chat.
    ///
    /// Converts kaze's [`Message`](crate::message::Message) types to rig-core
    /// messages, extracts any system prompt as the agent preamble, and uses
    /// [`StreamingChat::stream_chat`] to stream with context.
    ///
    /// # Arguments
    ///
    /// * `history` — Full conversation history. System messages become the
    ///   preamble; the last user message becomes the prompt; everything else
    ///   is chat history.
    /// * `renderer` — A [`Renderer`] implementation for streaming output.
    pub async fn stream_with_history(
        &self,
        history: &[crate::message::Message],
        renderer: &mut dyn Renderer,
    ) -> Result<String> {
        // Extract system prompt from history (first System message becomes preamble)
        let system_prompt = history
            .iter()
            .find(|m| m.role == crate::message::Role::System)
            .map(|m| m.text());

        // Last message is the user's prompt
        let prompt_text = history
            .last()
            .map(|m| m.text().to_string())
            .unwrap_or_default();

        // Convert history to rig messages (skip system msgs and the last user msg)
        let chat_history: Vec<RigMessage> = history
            .iter()
            .take(history.len().saturating_sub(1))
            .filter(|m| m.role != crate::message::Role::System)
            .map(|m| match m.role {
                crate::message::Role::User => RigMessage::user(m.text()),
                crate::message::Role::Assistant => RigMessage::assistant(m.text()),
                _ => RigMessage::user(m.text()),
            })
            .collect();

        let mut full_response = String::new();

        dispatch!(self, |client| {
            let mut stream = with_agent!(client, &self.model, system_prompt, |agent| {
                agent
                    .stream_chat(prompt_text.clone(), chat_history.clone())
                    .await
            });
            process_stream!(stream, renderer, full_response);
        });

        renderer.render_done();
        Ok(full_response)
    }

    /// Sends a non-streaming prompt to the LLM and returns the full response.
    ///
    /// Used for internal tasks like context compaction where streaming
    /// output is not needed.
    pub async fn prompt(&self, prompt_text: &str) -> Result<String> {
        dispatch!(self, |client| {
            let response = with_agent!(client, &self.model, None::<&str>, |agent| {
                agent.prompt(prompt_text).await
            });
            Ok(response?)
        })
    }
}
