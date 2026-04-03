//! TaskStop tool — stop a running background task
//!
//! Also available as `KillShell` alias for backward compatibility.

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TaskStopInput {
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    shell_id: Option<String>,
}

pub struct TaskStopTool;

impl TaskStopTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskStopTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str {
        "TaskStop"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["KillShell".to_string()]
    }

    fn description(&self) -> String {
        "Stop a running background task by ID".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the background task to stop"
                },
                "shell_id": {
                    "type": "string",
                    "description": "Deprecated: use task_id instead"
                }
            },
            "required": []
        })
    }

    async fn call(&self, args: serde_json::Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        let _input: TaskStopInput = serde_json::from_value(args)?;
        Err(CliError::Other("Task management requires the coordinator system.".to_string()))
    }
}
