//! Entry point for tool-augmented LLM interaction.
//!
//! Provides [`agent_loop`], a thin wrapper that delegates to rig-core's
//! `multi_turn()` streaming via [`Provider::stream_with_tools`]. The actual
//! send→tool→feedback iteration is handled entirely by rig-core; this module
//! renders stream events and captures the final assistant text response.

use crate::hooks::KazePermissionHook;
use anyhow::Result;

use crate::message::Message;
use crate::output::Renderer;
use crate::provider::Provider;
use crate::tools::ToolRegistry;

/// Runs a tool-augmented LLM interaction.
///
/// Delegates to rig-core's `multi_turn()` streaming: the LLM calls tools,
/// rig-core executes them via the registered adapters, and kaze renders
/// stream events as they arrive. Returns the final text response.
///
/// Only the final assistant text is appended to `messages`; intermediate
/// tool calls and results are not captured.
pub async fn agent_loop(
    provider: &Provider,
    messages: &mut Vec<Message>,
    tools: &ToolRegistry,
    renderer: &mut dyn Renderer,
    max_iterations: usize,
    hook: KazePermissionHook,
) -> Result<String> {
    let response = provider
        .stream_with_tools(messages, tools, renderer, max_iterations, hook)
        .await?;
    messages.push(Message::assistant(&response));
    Ok(response)
}
