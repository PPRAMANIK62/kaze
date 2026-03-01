//! Write-file tool — writes content to a file, creating parent directories as needed.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Tool, ToolResult};

/// Tool that writes string content to a file within the project root.
///
/// Parent directories are created automatically. Path traversal outside
/// the project root is rejected.
///
/// # Errors
///
/// Returns an error if the resolved path escapes the project root or if
/// the filesystem write fails.
pub struct WriteFileTool {
    /// Project root directory. Paths are resolved relative to this.
    project_root: PathBuf,
}

impl WriteFileTool {
    /// Create a new `WriteFileTool` rooted at the given directory.
    ///
    /// # Errors
    ///
    /// None — construction is infallible.
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Resolve and validate that the path stays within the project root.
    ///
    /// Unlike `ReadFileTool::resolve_path`, the target file may not exist yet,
    /// so we canonicalize the *parent* directory instead of the file itself.
    /// Parent directories are created if they don't already exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the resolved path would escape the project root.
    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        let resolved = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.project_root.join(path)
        };

        let parent = resolved
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory: {}", path))?;

        // Create parent directories if they don't exist yet.
        fs::create_dir_all(parent)?;

        let parent_canonical = parent.canonicalize()?;
        let root_canonical = self.project_root.canonicalize()?;

        if !parent_canonical.starts_with(&root_canonical) {
            anyhow::bail!("Path escapes project directory: {}", path);
        }

        let filename = resolved
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Path has no filename: {}", path))?;

        Ok(parent_canonical.join(filename))
    }
}

#[derive(Deserialize)]
struct WriteFileInput {
    path: String,
    content: String,
}

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates parent directories as needed. Path is relative to the project root."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path relative to project root"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolResult> {
        let input: WriteFileInput = serde_json::from_value(input)?;
        let path = self.resolve_path(&input.path)?;

        fs::write(&path, &input.content)?;

        let bytes_written = input.content.len();
        Ok(ToolResult::success(format!(
            "Wrote {} bytes to {}",
            bytes_written, input.path
        )))
    }
}
