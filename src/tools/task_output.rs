//! TaskOutput tool — read output from a background task

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TaskOutputInput {
    task_id: String,
}

pub struct TaskOutputTool;

impl TaskOutputTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskOutputTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }

    fn description(&self) -> String {
        "Read output from a background task".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the background task to get output from"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn call(&self, args: serde_json::Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        let _input: TaskOutputInput = serde_json::from_value(args)?;
        Err(CliError::Other("Task management requires the coordinator system.".to_string()))
    }
}
