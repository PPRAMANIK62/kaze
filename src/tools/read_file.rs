use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use super::{Tool, ToolResult};

/// Maximum file size to read (default 100KB).
const MAX_READ_SIZE: u64 = 100 * 1024;

pub struct ReadFileTool {
    /// Project root directory. Paths are resolved relative to this.
    project_root: PathBuf,
}

impl ReadFileTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Resolve and validate that the path stays within the project root.
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
struct ReadFileInput {
    path: String,
}

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }

    fn description(&self) -> &str {
        "Read the contents of a file. Path is relative to the project root."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path relative to project root"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolResult> {
        let input: ReadFileInput = serde_json::from_value(input)?;
        let path = self.resolve_path(&input.path)?;

        let metadata = std::fs::metadata(&path)?;
        if metadata.len() > MAX_READ_SIZE {
            return Ok(ToolResult::error(format!(
                "File too large: {} bytes (max {})",
                metadata.len(),
                MAX_READ_SIZE
            )));
        }

        let content = std::fs::read(&path)?;
        // Check for binary content (null bytes in first 8KB)
        let check_len = content.len().min(8192);
        if content[..check_len].contains(&0) {
            return Ok(ToolResult::error(
                "Binary file detected. Cannot display binary content.".into(),
            ));
        }

        let text = String::from_utf8(content)
            .map_err(|_| anyhow::anyhow!("File is not valid UTF-8"))?;
        Ok(ToolResult::success(text))
    }
}
