//! AgentTool — spawn a sub-agent as a tool

use async_trait::async_trait;
use std::sync::Arc;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolResult};

/// Backend for spawning sub-agents. Implemented by the binary.
#[async_trait]
pub trait AgentBackend: Send + Sync {
    async fn run_agent(
        &self,
        prompt: String,
        system_prompt: Option<String>,
    ) -> Result<String, CliError>;
}

pub struct AgentTool {
    backend: Arc<dyn AgentBackend>,
}

impl AgentTool {
    pub fn new(backend: Arc<dyn AgentBackend>) -> Self {
        Self { backend }
    }
}

#[derive(serde::Deserialize)]
struct AgentToolInput {
    prompt: String,
    #[serde(default)]
    system_prompt: Option<String>,
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["subagent".to_string()]
    }

    fn description(&self) -> String {
        "Spawn a sub-agent to handle a sub-task. Returns the agent's final output.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Task for the sub-agent"
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt override"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: AgentToolInput = serde_json::from_value(args)
            .map_err(|e| CliError::Other(format!("invalid agent input: {e}")))?;
        let output = self
            .backend
            .run_agent(input.prompt, input.system_prompt)
            .await?;
        Ok(ToolResult {
            content: vec![ResultContentBlock::Text { text: output }],
            is_error: false,
            metrics: None,
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        args.get("prompt")
            .and_then(|v| v.as_str())
            .map(|s| format!("agent: {}", &s[..s.len().min(80)]))
            .unwrap_or_else(|| "agent".to_string())
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|b| {
                if let ResultContentBlock::Text { text } = b {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }
}
