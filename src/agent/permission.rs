//! Permission checking

use crate::types::{
    PermissionDecision, PermissionMode,
    Tool as ToolTrait,
};

/// Permission checker for tool execution
pub struct PermissionChecker {
    mode: PermissionMode,
}

impl PermissionChecker {
    pub fn new(mode: PermissionMode) -> Self {
        Self { mode }
    }

    /// Check if a tool can be executed
    pub async fn check_tool(
        &self,
        tool: &dyn ToolTrait,
        args: &serde_json::Value,
        _content: &str,
    ) -> PermissionDecision {
        match self.mode {
            PermissionMode::BypassPermissions => PermissionDecision::Allow,
            PermissionMode::AcceptEdits => PermissionDecision::Allow,
            PermissionMode::DontAsk => PermissionDecision::Allow,
            PermissionMode::Plan => {
                PermissionDecision::Ask {
                    message: format!(
                        "The {} tool would be executed with args: {}",
                        tool.name(),
                        args
                    ),
                    suggestions: vec![],
                }
            }
            PermissionMode::Auto => {
                let perm_check = tool.check_permissions(args);
                match perm_check {
                    crate::types::PermissionCheck::Allowed => PermissionDecision::Allow,
                    crate::types::PermissionCheck::Denied(msg) => {
                        PermissionDecision::Deny(msg)
                    }
                    crate::types::PermissionCheck::NeedsApproval { message, suggestions } => {
                        PermissionDecision::Ask {
                            message,
                            suggestions,
                        }
                    }
                }
            }
            PermissionMode::Default | PermissionMode::Bubble => {
                let perm_check = tool.check_permissions(args);
                match perm_check {
                    crate::types::PermissionCheck::Allowed => PermissionDecision::Allow,
                    crate::types::PermissionCheck::Denied(msg) => {
                        PermissionDecision::Deny(msg)
                    }
                    crate::types::PermissionCheck::NeedsApproval { message, suggestions } => {
                        PermissionDecision::Ask {
                            message,
                            suggestions,
                        }
                    }
                }
            }
        }
    }
}
