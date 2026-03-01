//! Permission configuration and runtime checking for tool execution.
//!
//! Provides [`PermissionManager`] which loads permission rules from config
//! and checks whether each tool call should be allowed, require user
//! confirmation, or be denied entirely.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Mutex;

/// Permission level for a tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Allow,
    Ask,
    Deny,
}

/// Configuration for the permission system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionConfig {
    /// Per-tool permissions: tool_name -> Permission
    #[serde(default)]
    pub tools: HashMap<String, Permission>,

    /// Per-command permissions for bash: command_pattern -> Permission
    #[serde(default)]
    pub bash_commands: HashMap<String, Permission>,
}

/// Manages runtime permission checks.
pub struct PermissionManager {
    config: PermissionConfig,
    /// Session-level overrides (e.g., user chose "always allow" during session).
    /// Wrapped in Mutex because PromptHook requires &self (not &mut self).
    session_overrides: Mutex<HashMap<String, Permission>>,
}

impl PermissionManager {
    pub fn new(config: PermissionConfig) -> Self {
        Self {
            config,
            session_overrides: Mutex::new(HashMap::new()),
        }
    }

    /// Create with sensible defaults (bash=ask, everything else=allow).
    #[allow(dead_code)]
    pub fn with_defaults() -> Self {
        let mut tools = HashMap::new();
        tools.insert("read_file".into(), Permission::Allow);
        tools.insert("glob".into(), Permission::Allow);
        tools.insert("grep".into(), Permission::Allow);
        tools.insert("write_file".into(), Permission::Allow);
        tools.insert("edit".into(), Permission::Allow);
        tools.insert("bash".into(), Permission::Ask);

        Self::new(PermissionConfig {
            tools,
            bash_commands: HashMap::new(),
        })
    }

    /// Check permission for a tool call. Returns the action to take.
    pub fn check(&self, tool_name: &str, args: &str) -> Permission {
        // Session overrides take priority
        if let Some(perm) = self.session_overrides.lock().unwrap().get(tool_name) {
            return perm.clone();
        }

        // For bash, check command-specific permissions first
        if tool_name == "bash" {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(args) {
                if let Some(command) = parsed.get("command").and_then(|c| c.as_str()) {
                    if let Some(perm) = self.match_bash_command(command) {
                        return perm;
                    }
                }
            }
        }

        // Fall back to tool-level permission
        self.config
            .tools
            .get(tool_name)
            .cloned()
            .unwrap_or(Permission::Ask) // Unknown tools default to ask
    }

    /// Match a bash command against wildcard patterns.
    fn match_bash_command(&self, command: &str) -> Option<Permission> {
        for (pattern, perm) in &self.config.bash_commands {
            if Self::wildcard_match(pattern, command) {
                return Some(perm.clone());
            }
        }
        None
    }

    /// Simple wildcard matching: "git *" matches "git status", "git push", etc.
    fn wildcard_match(pattern: &str, text: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix(" *") {
            text.starts_with(prefix)
        } else {
            pattern == text
        }
    }

    /// Prompt the user for permission. Returns the user's choice.
    pub fn prompt_user(tool_name: &str, args: &str) -> Result<PromptResponse> {
        let display = if args.len() > 200 {
            format!("{}...", &args[..200])
        } else {
            args.to_string()
        };

        eprint!(
            "\nTool '{}' wants to execute:\n{}\n\nAllow? [y]es / [n]o / [a]lways: ",
            tool_name, display
        );
        io::stderr().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        match response.trim().to_lowercase().as_str() {
            "y" | "yes" => Ok(PromptResponse::Yes),
            "n" | "no" => Ok(PromptResponse::No),
            "a" | "always" => Ok(PromptResponse::Always),
            _ => Ok(PromptResponse::No),
        }
    }

    /// Set a session-level override (used when user chooses "always").
    pub fn set_session_override(&self, tool_name: &str, perm: Permission) {
        self.session_overrides
            .lock()
            .unwrap()
            .insert(tool_name.to_string(), perm);
    }
}

#[derive(Debug, PartialEq)]
pub enum PromptResponse {
    Yes,
    No,
    Always,
}
