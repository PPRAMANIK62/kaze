pub mod bash_tool;
pub mod edit_tool;
pub mod glob_tool;
pub mod grep_tool;
pub mod read_file;
pub mod rig_adapter;
pub mod write_file;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

use bash_tool::BashTool;
use edit_tool::EditTool;
use glob_tool::GlobTool;
use grep_tool::GrepTool;
use read_file::ReadFileTool;
use write_file::WriteFileTool;

/// The result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(content: String) -> Self {
        Self {
            content,
            is_error: false,
        }
    }

    pub fn error(content: String) -> Self {
        Self {
            content,
            is_error: true,
        }
    }
}

/// Definition sent to the LLM so it knows what tools are available.
#[cfg(test)]
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON Schema
}

/// Every tool implements this trait.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Unique name the LLM uses to call this tool.
    fn name(&self) -> &str;

    /// Human-readable description for the LLM's system prompt.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn schema(&self) -> Value;

    /// Execute the tool with the given JSON input.
    async fn execute(&self, input: Value) -> Result<ToolResult>;
}

/// Holds all registered tools and dispatches calls by name.
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool. Called during startup.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(Arc::from(tool));
    }

    /// Produce definitions for the LLM (sent in the API request).
    #[cfg(test)]
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .iter()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.schema(),
            })
            .collect()
    }

    /// Look up a tool by name and execute it.
    #[cfg(test)]
    pub async fn execute(&self, name: &str, input: Value) -> Result<ToolResult> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", name))?;
        tool.execute(input).await
    }

    /// How many tools are registered.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Converts all registered tools into rig-core [`ToolDyn`] trait objects.
    ///
    /// Returns a fresh `Vec` each call so the result can be moved into an
    /// agent builder's `.tools()` without borrow/move conflicts.
    pub fn to_rig_tools(&self) -> Vec<Box<dyn rig::tool::ToolDyn>> {
        self.tools
            .iter()
            .map(|t| {
                Box::new(rig_adapter::RigToolAdapter::new(Arc::clone(t)))
                    as Box<dyn rig::tool::ToolDyn>
            })
            .collect()
    }
}

impl ToolRegistry {
    /// Create a registry with all built-in tools.
    pub fn with_builtins(project_root: PathBuf) -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(ReadFileTool::new(project_root.clone())));
        registry.register(Box::new(GlobTool::new(project_root.clone())));
        registry.register(Box::new(GrepTool::new(project_root.clone())));
        registry.register(Box::new(WriteFileTool::new(project_root.clone())));
        registry.register(Box::new(EditTool::new(project_root.clone())));
        registry.register(Box::new(BashTool::new(project_root)));
        registry
    }
}

#[cfg(test)]
mod tests;
