//! EnterWorktree tool — creates an isolated git worktree and switches the session into it.

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Validates that a worktree slug contains only letters, digits, dots, underscores, and dashes,
/// with a total length of at most 64 characters.
fn validate_worktree_slug(s: &str) -> Result<(), String> {
    if s.len() > 64 {
        return Err(format!(
            "worktree slug must be at most 64 characters, got {}",
            s.len()
        ));
    }
    if !s.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-') {
        return Err(format!(
            "worktree slug may only contain letters, digits, dots, underscores, and dashes, got: {s}"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// EnterWorktreeTool
// ---------------------------------------------------------------------------

pub struct EnterWorktreeTool;

impl EnterWorktreeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnterWorktreeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct EnterWorktreeInput {
    /// Optional name for the worktree. Each "/"-separated segment may contain only
    /// letters, digits, dots, underscores, and dashes; max 64 chars total.
    /// A random name is generated if not provided.
    name: Option<String>,
}

// ---------------------------------------------------------------------------
// Tool trait impl
// ---------------------------------------------------------------------------

#[async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        "EnterWorktree"
    }

    fn description(&self) -> String {
        "Creates an isolated worktree (via git or configured hooks) and switches the session into it."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Optional name for the worktree. Each \"/\"-separated segment may contain only letters, digits, dots, underscores, and dashes; max 64 chars total. A random name is generated if not provided."
                }
            }
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: EnterWorktreeInput = match serde_json::from_value(args) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult::error(format!("invalid input: {e}")));
            }
        };

        if let Some(ref name) = input.name {
            if let Err(msg) = validate_worktree_slug(name) {
                return Ok(ToolResult::error(msg));
            }
        }

        Ok(ToolResult::error(
            "Worktree operations are not available in this context.",
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<EnterWorktreeInput>(args.clone()) {
            if let Some(name) = &input.name {
                format!("Creating worktree: {name}")
            } else {
                "Creating worktree with auto-generated name".to_string()
            }
        } else {
            "Creating worktree".to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_worktree_slug_valid() {
        assert!(validate_worktree_slug("my-worktree").is_ok());
        assert!(validate_worktree_slug("feature_123").is_ok());
        assert!(validate_worktree_slug("v1.2.3").is_ok());
        assert!(validate_worktree_slug("a-b_c.d").is_ok());
        assert!(validate_worktree_slug("").is_ok());
    }

    #[test]
    fn test_validate_worktree_slug_invalid_chars() {
        let result = validate_worktree_slug("worktree/branch");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("may only contain"));

        let result = validate_worktree_slug("worktree space");
        assert!(result.is_err());

        let result = validate_worktree_slug("worktree@branch");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_worktree_slug_too_long() {
        let long_name = "a".repeat(65);
        let result = validate_worktree_slug(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at most 64 characters"));
    }

    #[test]
    fn test_tool_name() {
        let tool = EnterWorktreeTool::new();
        assert_eq!(tool.name(), "EnterWorktree");
    }

    #[test]
    fn test_tool_is_not_read_only() {
        let tool = EnterWorktreeTool::new();
        assert!(!tool.is_read_only());
    }

    #[test]
    fn test_input_schema_has_name_field() {
        let tool = EnterWorktreeTool::new();
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].get("name").is_some());
    }

    #[test]
    fn test_render_use_message_with_name() {
        let tool = EnterWorktreeTool::new();
        let args = serde_json::json!({ "name": "my-branch" });
        assert_eq!(tool.render_use_message(&args), "Creating worktree: my-branch");
    }

    #[test]
    fn test_render_use_message_without_name() {
        let tool = EnterWorktreeTool::new();
        let args = serde_json::json!({});
        assert_eq!(
            tool.render_use_message(&args),
            "Creating worktree with auto-generated name"
        );
    }
}
