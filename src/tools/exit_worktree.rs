//! ExitWorktree tool — leaves the current worktree and returns to the original session directory.

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

// ---------------------------------------------------------------------------
// ExitWorktreeTool
// ---------------------------------------------------------------------------

pub struct ExitWorktreeTool;

impl ExitWorktreeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExitWorktreeTool {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tool trait impl
// ---------------------------------------------------------------------------

#[async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        "ExitWorktree"
    }

    fn description(&self) -> String {
        "Leaves the current worktree and returns the session to its original working directory."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        _args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        Ok(ToolResult::error(
            "Worktree operations are not available in this context.",
        ))
    }

    fn render_use_message(&self, _args: &serde_json::Value) -> String {
        "Leaving worktree".to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let tool = ExitWorktreeTool::new();
        assert_eq!(tool.name(), "ExitWorktree");
    }

    #[test]
    fn test_tool_is_not_read_only() {
        let tool = ExitWorktreeTool::new();
        assert!(!tool.is_read_only());
    }

    #[test]
    fn test_input_schema_is_empty_object() {
        let tool = ExitWorktreeTool::new();
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_render_use_message() {
        let tool = ExitWorktreeTool::new();
        assert_eq!(tool.render_use_message(&serde_json::json!({})), "Leaving worktree");
    }
}
