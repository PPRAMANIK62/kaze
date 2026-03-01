//! rig-core PromptHook implementation for kaze's permission system.

use std::sync::Arc;

use rig::agent::{PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;

use crate::permissions::{Permission, PermissionManager, PromptResponse};

/// Hook that checks permissions before rig-core executes any tool.
///
/// PromptHook requires `Clone + Send + Sync`. Using `Arc<PermissionManager>`
/// satisfies these bounds since Arc is Clone + Send + Sync when T: Send + Sync.
#[derive(Clone)]
pub struct KazePermissionHook {
    /// Shared because the hook is cloned by rig-core internally.
    manager: Arc<PermissionManager>,
}

impl KazePermissionHook {
    pub fn new(manager: Arc<PermissionManager>) -> Self {
        Self { manager }
    }
}

impl<M: CompletionModel> PromptHook<M> for KazePermissionHook {
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

        async move {
            match permission {
                Permission::Allow => ToolCallHookAction::cont(),
                Permission::Ask => match PermissionManager::prompt_user(&tool_name, &args) {
                    Ok(PromptResponse::Yes) => ToolCallHookAction::cont(),
                    Ok(PromptResponse::Always) => {
                        manager.set_session_override(&tool_name, Permission::Allow);
                        ToolCallHookAction::cont()
                    }
                    Ok(PromptResponse::No) => ToolCallHookAction::skip(format!(
                        "User denied permission for '{}'",
                        tool_name
                    )),
                    Err(_) => {
                        ToolCallHookAction::skip("Failed to read user input for permission prompt")
                    }
                },
                Permission::Deny => ToolCallHookAction::skip(format!(
                    "Tool '{}' is disabled by user configuration",
                    tool_name,
                )),
            }
        }
    }
}
