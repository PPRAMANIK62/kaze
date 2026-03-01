//! Edit tool — search-and-replace based file editing within the project root.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use super::{Tool, ToolResult};
use crate::constants::DIFF_CONTEXT_LINES;

/// Tool that performs search-and-replace edits on existing files.
///
/// Finds exact text matches and replaces them, optionally replacing all
/// occurrences. Path traversal outside the project root is rejected.
///
/// # Errors
///
/// Returns an error if the resolved path escapes the project root, the
/// file does not exist, or the filesystem read/write fails.
pub struct EditTool {
    /// Project root directory. Paths are resolved relative to this.
    project_root: PathBuf,
}

impl EditTool {
    /// Create a new `EditTool` rooted at the given directory.
    ///
    /// # Errors
    ///
    /// None — construction is infallible.
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Resolve and validate that the path stays within the project root.
    ///
    /// The target file must already exist, so we canonicalize it directly.
    ///
    /// # Errors
    ///
    /// Returns an error if the resolved path would escape the project root
    /// or the file does not exist.
    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        let resolved = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.project_root.join(path)
        };
        let canonical = resolved.canonicalize()?;
        let root_canonical = self.project_root.canonicalize()?;
        if !canonical.starts_with(&root_canonical) {
            anyhow::bail!("Path escapes project directory: {}", path);
        }
        Ok(canonical)
    }
}

#[derive(Deserialize)]
struct EditInput {
    path: String,
    old_text: String,
    new_text: String,
    #[serde(default)]
    replace_all: bool,
}

/// Produce a simplified before/after diff with context lines around each change.
fn format_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut changed = vec![false; old_lines.len()];
    // Mark lines that differ between old and new.
    for (i, old_line) in old_lines.iter().enumerate() {
        if new_lines.get(i) != Some(old_line) {
            changed[i] = true;
        }
    }
    // Also mark if new has more lines than old.
    if new_lines.len() > old_lines.len() {
        if let Some(last) = changed.last_mut() {
            *last = true;
        }
    }

    // Expand context window around changed lines.
    let mut visible = vec![false; old_lines.len()];
    for (i, is_changed) in changed.iter().enumerate() {
        if *is_changed {
            let start = i.saturating_sub(DIFF_CONTEXT_LINES);
            let end = (i + DIFF_CONTEXT_LINES + 1).min(old_lines.len());
            for v in &mut visible[start..end] {
                *v = true;
            }
        }
    }

    let mut output = String::new();
    output.push_str("--- before\n+++ after\n");

    let mut in_hunk = false;
    for (i, old_line) in old_lines.iter().enumerate() {
        if visible[i] {
            if !in_hunk {
                output.push_str(&format!("@@ line {} @@\n", i + 1));
                in_hunk = true;
            }
            if changed[i] {
                output.push_str(&format!("-{old_line}\n"));
                if let Some(new_line) = new_lines.get(i) {
                    output.push_str(&format!("+{new_line}\n"));
                }
            } else {
                output.push_str(&format!(" {old_line}\n"));
            }
        } else {
            in_hunk = false;
        }
    }

    // Show any extra new lines beyond old length.
    if new_lines.len() > old_lines.len() {
        output.push_str(&format!("@@ line {} @@\n", old_lines.len() + 1));
        for new_line in &new_lines[old_lines.len()..] {
            output.push_str(&format!("+{new_line}\n"));
        }
    }

    output
}

#[async_trait::async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Search and replace text in an existing file. Finds exact text matches and replaces them. \
         Path is relative to the project root."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path relative to project root"
                },
                "old_text": {
                    "type": "string",
                    "description": "Exact text to search for in the file"
                },
                "new_text": {
                    "type": "string",
                    "description": "Text to replace old_text with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false, replaces first only)"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolResult> {
        let input: EditInput = serde_json::from_value(input)?;
        let path = self.resolve_path(&input.path)?;

        let content = std::fs::read_to_string(&path)?;

        if !content.contains(&input.old_text) {
            return Ok(ToolResult::error(format!(
                "Text not found in {}. Make sure the old_text matches exactly, \
                 including whitespace and indentation.",
                input.path
            )));
        }

        let new_content = if input.replace_all {
            content.replace(&input.old_text, &input.new_text)
        } else {
            content.replacen(&input.old_text, &input.new_text, 1)
        };

        std::fs::write(&path, &new_content)?;

        let diff = format_diff(&content, &new_content);
        Ok(ToolResult::success(format!(
            "Edited {}\n\n{}",
            input.path, diff
        )))
    }
}
