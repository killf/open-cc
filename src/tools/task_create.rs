//! TaskCreate tool — create a task in the task list

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TaskCreateInput {
    subject: String,
    description: String,
    #[serde(default)]
    active_form: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

pub struct TaskCreateTool;

impl TaskCreateTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskCreateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }

    fn description(&self) -> String {
        "Create a task in the task list".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "subject": {
                    "type": "string",
                    "description": "A brief title for the task"
                },
                "description": {
                    "type": "string",
                    "description": "What needs to be done"
                },
                "activeForm": {
                    "type": "string",
                    "description": "Present continuous form shown in spinner when in_progress (e.g., \"Running tests\")"
                },
                "metadata": {
                    "type": "object",
                    "description": "Arbitrary metadata to attach to the task"
                }
            },
            "required": ["subject", "description"]
        })
    }

    async fn call(&self, args: serde_json::Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        let _input: TaskCreateInput = serde_json::from_value(args)?;
        Err(CliError::Other("Task management requires the coordinator system.".to_string()))
    }
}
