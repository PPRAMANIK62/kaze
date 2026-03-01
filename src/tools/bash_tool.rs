//! Bash tool — shell command execution with safety measures.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

use super::{Tool, ToolResult};

use crate::constants::{BASH_DEFAULT_TIMEOUT_SECS, BASH_MAX_OUTPUT_SIZE, BASH_STRIPPED_ENV_VARS};

/// Tool that executes shell commands in a child process.
///
/// Commands run with a configurable timeout, output size cap, and
/// sensitive environment variables stripped. The working directory
/// is set to the project root.
pub struct BashTool {
    project_root: PathBuf,
}

impl BashTool {
    /// Create a new `BashTool` rooted at `project_root`.
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }
}

#[derive(Deserialize)]
struct BashInput {
    command: String,
    timeout: Option<u64>,
}

/// Truncate `output` to at most `BASH_MAX_OUTPUT_SIZE` bytes, appending a
/// notice when truncation occurs.
fn cap_output(output: &str) -> String {
    if output.len() <= BASH_MAX_OUTPUT_SIZE {
        return output.to_string();
    }
    // Find a valid UTF-8 boundary at or before the limit.
    let mut end = BASH_MAX_OUTPUT_SIZE;
    while end > 0 && !output.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n... output truncated at {} bytes",
        &output[..end],
        BASH_MAX_OUTPUT_SIZE
    )
}

#[async_trait::async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output. Commands run in the project root with a configurable timeout."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 30)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolResult> {
        let input: BashInput = serde_json::from_value(input)?;

        let timeout_secs = input.timeout.unwrap_or(BASH_DEFAULT_TIMEOUT_SECS);

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(&input.command);
        cmd.current_dir(&self.project_root);

        // Strip sensitive environment variables.
        for var in BASH_STRIPPED_ENV_VARS {
            cmd.env_remove(var);
        }

        // Capture stdout and stderr.
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd.spawn();
        let child = match child {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to execute command: {}",
                    e
                )));
            }
        };

        // Wait with timeout.
        let result =
            tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let mut text = stdout.to_string();
                if !stderr.is_empty() {
                    text.push_str("\n--- stderr ---\n");
                    text.push_str(&stderr);
                }

                let text = cap_output(&text);
                let code = output.status.code().unwrap_or(-1);

                if code != 0 {
                    Ok(ToolResult::error(format!(
                        "{}\nExit code: {}",
                        text.trim(),
                        code
                    )))
                } else {
                    Ok(ToolResult::success(text.trim().to_string()))
                }
            }
            Ok(Err(e)) => Ok(ToolResult::error(format!(
                "Failed to execute command: {}",
                e
            ))),
            Err(_) => Ok(ToolResult::error(format!(
                "Command timed out after {}s",
                timeout_secs
            ))),
        }
    }
}
