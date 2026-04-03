//! Tool system types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::CliError;

/// Tool context passed during execution
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub agent_id: String,
    pub working_directory: PathBuf,
    pub can_use_tool: bool,
    pub parent_message_id: Option<String>,
    /// Additional environment variables for tool execution
    pub env: std::collections::HashMap<String, String>,
}

/// Core Tool trait that all tools must implement
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name of the tool
    fn name(&self) -> &str;

    /// Optional aliases for the tool
    fn aliases(&self) -> Vec<String> {
        vec![]
    }

    /// Human-readable description
    fn description(&self) -> String;

    /// JSON Schema for tool input
    fn input_schema(&self) -> serde_json::Value;

    /// Whether this tool can be called concurrently with itself
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    /// Whether this tool only reads data
    fn is_read_only(&self) -> bool {
        false
    }

    /// Whether this tool modifies or deletes data
    fn is_destructive(&self) -> bool {
        false
    }

    /// Whether this tool is currently enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Execute the tool with the given arguments
    async fn call(
        &self,
        args: serde_json::Value,
        context: ToolContext,
    ) -> Result<ToolResult, CliError>;

    /// Render a human-readable message for tool use
    fn render_use_message(&self, args: &serde_json::Value) -> String {
        format!("Using {} with args: {}", self.name(), args)
    }

    /// Render a human-readable message for tool result
    fn render_result_message(&self, result: &ToolResult) -> String {
        let preview = result
            .content
            .iter()
            .map(|b| b.preview())
            .collect::<Vec<_>>()
            .join("; ");
        if preview.len() > 200 {
            format!("{}...", &preview[..200])
        } else {
            preview
        }
    }

    /// Check if this tool requires permissions
    fn check_permissions(&self, _args: &serde_json::Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ResultContentBlock>,
    pub is_error: bool,
    pub metrics: Option<ToolMetrics>,
}

impl ToolResult {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ResultContentBlock::Text { text: text.into() }],
            is_error: false,
            metrics: None,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ResultContentBlock::Text { text: text.into() }],
            is_error: true,
            metrics: None,
        }
    }
}

/// Content block in a tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
    Image { source: super::ImageSource },
}

impl ResultContentBlock {
    /// Short preview of the content block
    pub fn preview(&self) -> String {
        match self {
            Self::Text { text } => text.clone(),
            Self::ToolUse { name, .. } => format!("[tool: {}]", name),
            Self::ToolResult { content, .. } => content.clone(),
            Self::Image { .. } => "[image]".to_string(),
        }
    }
}

/// Tool execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub duration_ms: u64,
    pub tokens_used: Option<u32>,
}

/// Permission check result
#[derive(Debug, Clone)]
pub enum PermissionCheck {
    Allowed,
    Denied(String),
    NeedsApproval {
        message: String,
        suggestions: Vec<crate::types::PermissionUpdate>,
    },
}
