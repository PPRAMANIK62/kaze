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
use rig::message::{
    AssistantContent, Message as RigMessage, Text, ToolCall as RigToolCall, ToolFunction,
};
use rig::providers::{anthropic, openai, openrouter};
use rig::streaming::{
    StreamedAssistantContent, StreamedUserContent, StreamingChat, StreamingPrompt,
};
use rig::OneOrMany;

use std::collections::HashMap;

use super::kind::ProviderKind;
use super::resolve::ModelSelection;
use crate::config::Config;
use crate::output::Renderer;
use crate::tools::ToolRegistry;

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

/// Builds an agent with tools registered for LLM function calling.
///
/// Like [`with_agent!`] but adds rig-core tool definitions via `.tools()`.
/// The type-state change from `NoToolConfig` to `WithBuilderTools` means
/// this must be a separate macro — the two builder paths produce different types.
macro_rules! with_agent_tools {
    ($client:expr, $model:expr, $sys:expr, $rig_tools:expr, |$agent:ident| $body:expr) => {{
        let $agent = if let Some(sys) = $sys {
            $client
                .agent($model)
                .preamble(sys)
                .max_tokens(crate::constants::MAX_TOKENS)
                .tools($rig_tools)
                .build()
        } else {
            $client
                .agent($model)
                .max_tokens(crate::constants::MAX_TOKENS)
                .tools($rig_tools)
                .build()
        };
        $body
    }};
}

/// Processes a multi-turn streaming response where rig-core drives tool execution.
///
/// Handles all [`MultiTurnStreamItem`] variants:
/// - `StreamAssistantItem(Text)` → render token + accumulate text
/// - `StreamAssistantItem(ToolCall)` → render tool start, track name by internal ID
/// - `StreamUserItem(ToolResult)` → render tool result
/// - `FinalResponse` → stream complete
/// - Everything else (ToolCallDelta, Reasoning) → ignored
macro_rules! process_stream_with_tools {
    ($stream:expr, $renderer:expr, $full_response:expr, $tool_names:expr) => {
        while let Some(chunk) = $stream.next().await {
            match chunk {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                    Text { text },
                ))) => {
                    $renderer.render_token(&text);
                    $full_response.push_str(&text);
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall {
                        tool_call,
                        internal_call_id,
                    },
                )) => {
                    let name = tool_call.function.name.clone();
                    $renderer.tool_start(&name, &tool_call.function.arguments);
                    $tool_names.insert(internal_call_id, name);
                }
                Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult {
                    tool_result,
                    internal_call_id,
                })) => {
                    let name = $tool_names
                        .get(&internal_call_id)
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                    let result_text: String = tool_result
                        .content
                        .into_iter()
                        .filter_map(|c| match c {
                            rig::message::ToolResultContent::Text(t) => Some(t.text),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    $renderer.tool_result(name, &result_text);
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    // Stream complete
                }
                Err(err) => {
                    $renderer.render_error(&err.to_string());
                    anyhow::bail!("Streaming error: {}", err);
                }
                _ => {
                    // ToolCallDelta, Reasoning, etc. — rig-core handles internally
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
                    .base_url(format!("{}/v1", base_url))
                    .build()
                    .context("Failed to create Ollama client")?;
                Ok(Self {
                    client: ClientKind::Ollama(client),
                    model: selection.model.clone(),
                })
            }
        }
    }

    // Part of public API, used in future phases
    #[allow(dead_code)]
    /// Streams a prompt response, rendering tokens as they arrive via the given [`Renderer`].
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

    // Part of public API, used in future phases
    #[allow(dead_code)]
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

    /// Streams a multi-turn response with tool execution driven by rig-core.
    ///
    /// Builds an agent with rig-core tool adapters registered and uses
    /// [`StreamingChat::stream_chat`] with [`multi_turn`] so rig-core
    /// automatically executes tool calls and feeds results back to the LLM.
    /// kaze subscribes to the stream purely for rendering.
    ///
    /// # Arguments
    ///
    /// * `history` — Full conversation history including system, user, assistant,
    ///   and tool result messages.
    /// * `tools` — The tool registry whose definitions are sent to the LLM.
    /// * `renderer` — A [`Renderer`] for streaming text tokens and tool events.
    /// * `max_turns` — Maximum number of tool-calling round-trips rig-core may perform.
    pub async fn stream_with_tools(
        &self,
        history: &[crate::message::Message],
        tools: &ToolRegistry,
        renderer: &mut dyn Renderer,
        max_turns: usize,
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
            .filter_map(convert_message_to_rig)
            .collect();

        let mut full_response = String::new();
        let mut tool_names: HashMap<String, String> = HashMap::new();

        dispatch!(self, |client| {
            // Build rig_tools inside dispatch! so each match arm gets a fresh Vec
            let rig_tools = tools.to_rig_tools();
            let mut stream =
                with_agent_tools!(client, &self.model, system_prompt, rig_tools, |agent| {
                    agent
                        .stream_chat(prompt_text.clone(), chat_history.clone())
                        .multi_turn(max_turns)
                        .await
                });
            process_stream_with_tools!(stream, renderer, full_response, tool_names);
        });

        renderer.render_done();
        Ok(full_response)
    }
}

/// Converts a kaze [`Message`](crate::message::Message) to a rig-core [`RigMessage`].
///
/// Handles all message roles:
/// - **User** → `RigMessage::User` with text content
/// - **Assistant** (text only) → `RigMessage::Assistant` with text content
/// - **Assistant** (with tool calls) → `RigMessage::Assistant` with `ToolCall` content items
/// - **Tool** (result) → `RigMessage::User` with `ToolResult` content
/// - **System** → `None` (system messages are extracted as preamble separately)
fn convert_message_to_rig(msg: &crate::message::Message) -> Option<RigMessage> {
    match msg.role {
        crate::message::Role::User => Some(RigMessage::user(msg.text())),
        crate::message::Role::Assistant => {
            if msg.tool_calls.is_empty() {
                Some(RigMessage::assistant(msg.text()))
            } else {
                // Build assistant message with tool call content items
                let mut items: Vec<AssistantContent> = Vec::new();
                let text = msg.text();
                if !text.is_empty() {
                    items.push(AssistantContent::Text(Text {
                        text: text.to_string(),
                    }));
                }
                for tc in &msg.tool_calls {
                    items.push(AssistantContent::ToolCall(RigToolCall::new(
                        tc.id.clone(),
                        ToolFunction::new(tc.name.clone(), tc.arguments.clone()),
                    )));
                }
                Some(RigMessage::Assistant {
                    id: None,
                    content: OneOrMany::many(items)
                        .unwrap_or_else(|_| OneOrMany::one(AssistantContent::text(""))),
                })
            }
        }
        crate::message::Role::Tool => {
            let tool_call_id = match &msg.tool_call_id {
                Some(id) => id.clone(),
                None => {
                    eprintln!("warning: Tool message missing tool_call_id, using empty string");
                    String::new()
                }
            };
            Some(RigMessage::tool_result(tool_call_id, msg.text()))
        }
        crate::message::Role::System => None,
    }
}
