//! TaskUpdate tool — update a task's fields or status

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TaskUpdateInput {
    task_id: String,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    active_form: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    add_blocks: Option<Vec<String>>,
    #[serde(default)]
    add_blocked_by: Option<Vec<String>>,
    #[serde(default)]
    owner: Option<String>,
}

pub struct TaskUpdateTool;

impl TaskUpdateTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskUpdateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }

    fn description(&self) -> String {
        "Update a task's fields or status".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the task to update"
                },
                "subject": {
                    "type": "string",
                    "description": "New subject for the task"
                },
                "description": {
                    "type": "string",
                    "description": "New description for the task"
                },
                "activeForm": {
                    "type": "string",
                    "description": "Present continuous form shown in spinner when in_progress (e.g., \"Running tests\")"
                },
                "status": {
                    "type": "string",
                    "description": "New status for the task",
                    "enum": ["pending", "in_progress", "completed", "failed"]
                },
                "add_blocks": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Task IDs that this task blocks"
                },
                "add_blocked_by": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Task IDs that block this task"
                },
                "owner": {
                    "type": "string",
                    "description": "New owner for the task"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn call(&self, args: serde_json::Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        let _input: TaskUpdateInput = serde_json::from_value(args)?;
        Err(CliError::Other("Task management requires the coordinator system.".to_string()))
    }
}
