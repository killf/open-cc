//! RemoteTrigger tool — manage scheduled remote agent triggers via the API.
//
// Input: action (String), trigger_id (Option<String>), body (Option<serde_json::Value>)
// name: "RemoteTrigger"
// is_read_only: false (depends on action — list/get are ro, create/update/run are rw)

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Valid actions for the RemoteTrigger tool.
const REMOTE_TRIGGER_ACTIONS: &[&str] = &["list", "get", "create", "update", "run"];

/// Input schema for the RemoteTrigger tool.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RemoteTriggerInput {
    /// Action to perform: list, get, create, update, or run.
    action: String,
    /// Required for get, update, and run actions.
    #[serde(default)]
    trigger_id: Option<String>,
    /// JSON body for create and update actions.
    #[serde(default)]
    body: Option<serde_json::Value>,
}

pub struct RemoteTriggerTool;

impl RemoteTriggerTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": REMOTE_TRIGGER_ACTIONS,
                    "description": "The action to perform: list, get, create, update, or run"
                },
                "trigger_id": {
                    "type": "string",
                    "pattern": "^[\\w-]+$",
                    "description": "Required for get, update, and run actions"
                },
                "body": {
                    "type": "object",
                    "description": "JSON body for create and update actions"
                }
            },
            "required": ["action"]
        })
    }
}

impl Default for RemoteTriggerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &str {
        "RemoteTrigger"
    }

    fn description(&self) -> String {
        "Manages scheduled remote agent triggers via the API. Supports listing, \
         retrieving, creating, updating, and running remote trigger definitions. \
         Requires authentication with a claude.ai account."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        // list and get are read-only; create/update/run are not
        // We return false conservatively since the input determines the actual mode.
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: RemoteTriggerInput = serde_json::from_value(args)
            .map_err(|e| CliError::ToolExecution(format!("Invalid input: {e}")))?;

        Err(CliError::ToolExecution(
            "This tool requires the TUI/system integration.".to_string(),
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<RemoteTriggerInput>(args.clone()) {
            match (&input.action[..], &input.trigger_id) {
                ("list", _) => "Listing remote triggers".to_string(),
                ("get", Some(id)) => format!("Getting remote trigger: {id}"),
                ("create", _) => "Creating remote trigger".to_string(),
                ("update", Some(id)) => format!("Updating remote trigger: {id}"),
                ("run", Some(id)) => format!("Running remote trigger: {id}"),
                (action, _) => format!("RemoteTrigger action: {action}"),
            }
        } else {
            "Running RemoteTrigger action".to_string()
        }
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        if result.is_error {
            "RemoteTrigger action failed.".to_string()
        } else {
            "RemoteTrigger action completed.".to_string()
        }
    }
}
