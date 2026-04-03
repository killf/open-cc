//! TaskGet tool — retrieve a task by ID

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TaskGetInput {
    task_id: String,
}

pub struct TaskGetTool;

impl TaskGetTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskGetTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }

    fn description(&self) -> String {
        "Retrieve a task by ID".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the task to retrieve"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn call(&self, args: serde_json::Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        let _input: TaskGetInput = serde_json::from_value(args)?;
        Err(CliError::Other("Task management requires the coordinator system.".to_string()))
    }
}
