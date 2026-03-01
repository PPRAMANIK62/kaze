use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{Tool, ToolResult};

use crate::constants::GLOB_MAX_RESULTS;

pub struct GlobTool {
    project_root: PathBuf,
}

impl GlobTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }
}

#[derive(Deserialize)]
struct GlobInput {
    pattern: String,
}

#[async_trait::async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str { "glob" }

    fn description(&self) -> &str {
        "List files matching a glob pattern relative to the project root."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. 'src/**/*.rs')"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolResult> {
        let input: GlobInput = serde_json::from_value(input)?;
        let full_pattern = self.project_root.join(&input.pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let root_canonical = self.project_root.canonicalize()?;

        let mut paths: Vec<String> = Vec::new();
        for entry in glob::glob(&pattern_str)? {
            if paths.len() >= GLOB_MAX_RESULTS {
                paths.push(format!("... truncated at {} results", GLOB_MAX_RESULTS));
                break;
            }
            let entry = entry?;
            // Skip entries outside project root
            if let Ok(canonical) = entry.canonicalize() {
                if !canonical.starts_with(&root_canonical) {
                    continue;
                }
            } else {
                continue; // Skip entries that can't be canonicalized (broken symlinks, etc.)
            }
            // Show paths relative to project root
            let relative = entry
                .strip_prefix(&self.project_root)
                .unwrap_or(&entry);
            paths.push(relative.display().to_string());
        }

        if paths.is_empty() {
            Ok(ToolResult::success("No files matched the pattern.".into()))
        } else {
            Ok(ToolResult::success(paths.join("\n")))
        }
    }
}
