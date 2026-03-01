//! rig-core PromptHook for permissions and diff previews.
//!
//! [`KazeHook`] combines the permission system (from Step 23) with diff
//! preview generation (Step 24) into a single hook. For write_file and edit
//! tools, it generates a colored diff preview before prompting the user.

use std::path::PathBuf;
use std::sync::Arc;

use rig::agent::{PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;

use crate::diff;
use crate::permissions::{Permission, PermissionManager, PromptResponse};

/// Combined hook for permission checks and diff previews.
///
/// PromptHook requires `Clone + Send + Sync`. Using `Arc<PermissionManager>`
/// satisfies these bounds since Arc is Clone + Send + Sync when T: Send + Sync.
#[derive(Clone)]
pub struct KazeHook {
    /// Shared because the hook is cloned by rig-core internally.
    manager: Arc<PermissionManager>,
    /// Project root directory for resolving relative file paths.
    project_root: PathBuf,
}

impl KazeHook {
    pub fn new(manager: Arc<PermissionManager>, project_root: PathBuf) -> Self {
        Self {
            manager,
            project_root,
        }
    }

    /// For write_file and edit tools, generate a diff preview from the args.
    /// Returns None if args can't be parsed or the tool isn't a file-writing tool.
    ///
    /// NOTE: This uses `std::fs::read_to_string` (sync I/O) to read the current
    /// file contents. This is acceptable for a CLI tool on a single-threaded tokio
    /// runtime, because file reads are fast and the user is waiting at the terminal
    /// anyway. For the TUI (Phase 7), consider using `tokio::fs` instead.
    fn generate_diff(&self, tool_name: &str, args: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(args).ok()?;

        match tool_name {
            "write_file" => {
                let path_str = parsed.get("path")?.as_str()?;
                let new_content = parsed.get("content")?.as_str()?;
                let full_path = self.project_root.join(path_str);

                if full_path.exists() {
                    let old_content = std::fs::read_to_string(&full_path).ok()?;
                    Some(diff::unified_diff(&old_content, new_content, path_str))
                } else {
                    Some(diff::new_file_preview(new_content, path_str))
                }
            }
            "edit" => {
                let path_str = parsed.get("path")?.as_str()?;
                let full_path = self.project_root.join(path_str);
                let old_text = parsed.get("old_text")?.as_str()?;
                let new_text = parsed.get("new_text")?.as_str()?;

                // Read the full file, apply the edit, diff the result
                let original = std::fs::read_to_string(&full_path).ok()?;
                let modified = original.replacen(old_text, new_text, 1);
                Some(diff::unified_diff(&original, &modified, path_str))
            }
            _ => None,
        }
    }
}

impl<M: CompletionModel> PromptHook<M> for KazeHook {
    fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        args: &str,
    ) -> impl std::future::Future<Output = ToolCallHookAction> + Send {
        let permission = self.manager.check(tool_name, args);
        let tool_name = tool_name.to_string();
        let args = args.to_string();
        let manager = self.manager.clone();

        // Generate diff before entering the async block (needs &self)
        let diff_output = self.generate_diff(&tool_name, &args);

        async move {
            // Step 1: Check if the tool is outright denied
            if permission == Permission::Deny {
                return ToolCallHookAction::skip(format!(
                    "Tool '{}' is disabled by user configuration",
                    tool_name,
                ));
            }

            // Step 2: Show diff preview (always, for write_file and edit)
            if let Some(ref diff_str) = diff_output {
                eprintln!("\n{}", diff_str);
            }

            // Step 3: If permission is Ask, prompt the user
            if permission == Permission::Ask {
                match PermissionManager::prompt_user(&tool_name, &args) {
                    Ok(PromptResponse::Yes) => ToolCallHookAction::cont(),
                    Ok(PromptResponse::Always) => {
                        manager.set_session_override(&tool_name, Permission::Allow);
                        ToolCallHookAction::cont()
                    }
                    Ok(PromptResponse::No) => ToolCallHookAction::skip(format!(
                        "User rejected the change for '{}'",
                        tool_name
                    )),
                    Err(_) => {
                        ToolCallHookAction::skip("Failed to read user input for permission prompt")
                    }
                }
            } else {
                // Permission::Allow, diff was shown, proceed
                ToolCallHookAction::cont()
            }
        }
    }
}
