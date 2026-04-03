//! Todo write tool - update the session task checklist

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// A single todo item.
/// #[allow(dead_code)]: struct is used as a deserialization target; fields are validated
/// but the stub immediately discards the value.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TodoItem {
    content: String,
    status: String,
}

/// Input for the TodoWrite tool.
/// #[allow(dead_code)]: struct is used as a deserialization target; fields are validated
/// but the stub immediately discards the value.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TodoWriteInput {
    todos: Vec<TodoItem>,
}

pub struct TodoWriteTool;

impl TodoWriteTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "The updated todo list",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The task description"
                            },
                            "status": {
                                "type": "string",
                                "description": "The task status (e.g., in_progress, completed, pending)"
                            }
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "TodoWrite"
    }

    fn description(&self) -> String {
        "Manage the session task checklist.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: TodoWriteInput = serde_json::from_value(args)?;
        Ok(ToolResult::error(
            "This tool requires the TUI/coordinator system.",
        ))
    }
}
