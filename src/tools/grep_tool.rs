use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Tool, ToolResult};

use crate::constants::{GREP_MAX_MATCHES, BINARY_DETECTION_BYTES};

pub struct GrepTool {
    project_root: PathBuf,
}

impl GrepTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Search files under `search_root` for lines matching `regex`.
    /// Optionally filter files by an include glob pattern.
    fn search(&self, regex: &Regex, search_root: &Path, include: Option<&str>) -> Vec<String> {
        let include_pattern = include.and_then(|pat| {
            let full = self.project_root.join("**").join(pat);
            glob::Pattern::new(&full.to_string_lossy()).ok()
        });

        let mut matches = Vec::new();
        self.walk_and_search(search_root, regex, &include_pattern, &mut matches);
        matches
    }

    /// Recursively walk directories, searching files for regex matches.
    fn walk_and_search(
        &self,
        dir: &Path,
        regex: &Regex,
        include: &Option<glob::Pattern>,
        matches: &mut Vec<String>,
    ) {
        if matches.len() >= GREP_MAX_MATCHES {
            return;
        }

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return, // silently skip unreadable dirs
        };

        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            if matches.len() >= GREP_MAX_MATCHES {
                return;
            }

            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if path.is_dir() {
                // Skip hidden dirs, target/, node_modules/
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }
                self.walk_and_search(&path, regex, include, matches);
            } else if path.is_file() {
                // Apply include filter if present
                if let Some(ref pattern) = include {
                    if !pattern.matches_path(&path) {
                        continue;
                    }
                }
                self.search_file(&path, regex, matches);
            }
        }
    }

    /// Search a single file for regex matches, appending results as `path:line:content`.
    fn search_file(&self, path: &Path, regex: &Regex, matches: &mut Vec<String>) {
        // Read file, silently skip binary/unreadable
        let content = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => return,
        };

        // Check for binary content (null bytes in first 8KB)
        let check_len = content.len().min(BINARY_DETECTION_BYTES);
        if content[..check_len].contains(&0) {
            return;
        }

        let text = match String::from_utf8(content) {
            Ok(s) => s,
            Err(_) => return,
        };

        let relative = path
            .strip_prefix(&self.project_root)
            .unwrap_or(path);

        for (line_num, line) in text.lines().enumerate() {
            if matches.len() >= GREP_MAX_MATCHES {
                return;
            }
            if regex.is_match(line) {
                matches.push(format!(
                    "{}:{}:{}",
                    relative.display(),
                    line_num + 1,
                    line
                ));
            }
        }
    }
}

#[derive(Deserialize)]
struct GrepInput {
    pattern: String,
    path: Option<String>,
    include: Option<String>,
}

#[async_trait::async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }

    fn description(&self) -> &str {
        "Search file contents using a regex pattern. Returns matching lines with file paths and line numbers."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (relative to project root, defaults to '.')"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '*.rs')"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolResult> {
        let input: GrepInput = serde_json::from_value(input)?;

        // Validate regex
        let regex = match Regex::new(&input.pattern) {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::error(format!("Invalid regex: {}", e))),
        };

        // Resolve search path
        let search_root = if let Some(ref path) = input.path {
            let resolved = self.project_root.join(path);
            let canonical = resolved.canonicalize().map_err(|_| {
                anyhow::anyhow!("Search path does not exist: {}", path)
            })?;
            let root_canonical = self.project_root.canonicalize()?;
            if !canonical.starts_with(&root_canonical) {
                return Ok(ToolResult::error(
                    "Search path escapes project directory".into(),
                ));
            }
            canonical
        } else {
            self.project_root.clone()
        };

        let matches = self.search(&regex, &search_root, input.include.as_deref());

        if matches.is_empty() {
            Ok(ToolResult::success("No matches found.".into()))
        } else {
            let truncated = if matches.len() >= GREP_MAX_MATCHES {
                format!("\n... truncated at {} matches", GREP_MAX_MATCHES)
            } else {
                String::new()
            };
            Ok(ToolResult::success(format!("{}{}", matches.join("\n"), truncated)))
        }
    }
}
