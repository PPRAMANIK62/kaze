pub mod read_file;
pub mod glob_tool;
pub mod grep_tool;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use read_file::ReadFileTool;
use glob_tool::GlobTool;
use grep_tool::GrepTool;

/// The result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(content: String) -> Self {
        Self { content, is_error: false }
    }

    pub fn error(content: String) -> Self {
        Self { content, is_error: true }
    }
}

/// Definition sent to the LLM so it knows what tools are available.
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

/// Built-in tools as an enum for exhaustive matching.
/// Tools are added here in subsequent steps (17-21).
pub enum BuiltinTool {
    // ReadFile(ReadFileTool),
    // Glob(GlobTool),
    // Grep(GrepTool),
    // WriteFile(WriteFileTool),
    // Edit(EditTool),
    // Bash(BashTool),
}

/// Holds all registered tools and dispatches calls by name.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool. Called during startup.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Produce definitions for the LLM (sent in the API request).
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
    pub async fn execute(&self, name: &str, input: Value) -> Result<ToolResult> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", name))?;
        tool.execute(input).await
    }

    /// How many tools are registered.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl ToolRegistry {
    /// Create a registry with all built-in tools.
    pub fn with_builtins(project_root: PathBuf) -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(ReadFileTool::new(project_root.clone())));
        registry.register(Box::new(GlobTool::new(project_root.clone())));
        registry.register(Box::new(GrepTool::new(project_root)));
        registry
    }
}

#[cfg(test)]
mod tests;
