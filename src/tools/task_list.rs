//! TaskList tool — list all tasks

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

pub struct TaskListTool;

impl TaskListTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "TaskList"
    }

    fn description(&self) -> String {
        "List all tasks in the task list".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn call(&self, _args: serde_json::Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        Err(CliError::Other("Task management requires the coordinator system.".to_string()))
    }
}
