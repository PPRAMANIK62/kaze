//! Adapter bridging kaze's [`Tool`] trait to rig-core's [`ToolDyn`] trait.
//!
//! This module provides [`RigToolAdapter`], which wraps a reference to a kaze
//! tool and implements rig-core's dynamic tool interface. This allows kaze's
//! built-in tools to be registered with rig-core's agent builder so that tool
//! definitions are included in LLM API requests.

use std::pin::Pin;
use std::sync::Arc;

use rig::completion::ToolDefinition as RigToolDefinition;
use rig::tool::{ToolDyn, ToolError};

use super::Tool;

/// Bridges a kaze [`Tool`] to rig-core's [`ToolDyn`] trait.
///
/// Wraps an `Arc<dyn Tool>` and translates between the two tool interfaces:
/// - `name()` → delegates to the kaze tool's name
/// - `definition()` → builds a [`RigToolDefinition`] from the kaze tool's metadata
/// - `call()` → parses the JSON string args, calls the kaze tool's `execute()`,
///   and returns the result string
pub struct RigToolAdapter {
    tool: Arc<dyn Tool>,
}

impl RigToolAdapter {
    /// Creates a new adapter wrapping the given kaze tool.
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self { tool }
    }
}

impl ToolDyn for RigToolAdapter {
    fn name(&self) -> String {
        self.tool.name().to_string()
    }

    fn definition<'a>(
        &'a self,
        _prompt: String,
    ) -> Pin<Box<dyn std::future::Future<Output = RigToolDefinition> + Send + 'a>> {
        let name = self.tool.name().to_string();
        let description = self.tool.description().to_string();
        let parameters = self.tool.schema();
        Box::pin(async move {
            RigToolDefinition {
                name,
                description,
                parameters,
            }
        })
    }

    fn call<'a>(
        &'a self,
        args: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<String, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let input: serde_json::Value =
                serde_json::from_str(&args).map_err(ToolError::JsonError)?;
            match self.tool.execute(input).await {
                Ok(result) => Ok(result.content),
                Err(e) => {
                    // Return tool errors as result strings instead of ToolError.
                    // rig-core wraps ToolError through ToolSetError → ToolServerError,
                    // causing triple-nested "ToolCallError: ToolCallError: ToolCallError:"
                    // prefixes. Returning Ok("Error: ...") avoids this while still
                    // letting the LLM see and react to the error.
                    Ok(format!("Error: {}", e))
                }
            }
        })
    }
}
