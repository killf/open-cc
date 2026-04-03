//! Message types for Claude Code CLI

use serde::{Deserialize, Serialize};

/// Root message type, matching the TypeScript enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    User { content: UserContent },
    Assistant { content: Option<AssistantContent> },
    Progress { data: ProgressData },
    System {
        subtype: String,
        level: Option<String>,
        message: String,
    },
    Attachment { path: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    HookResult {
        hook_name: String,
        result: serde_json::Value,
    },
    Tombstone,
    GroupedToolUse { tool_uses: Vec<ToolUseSummary> },
}

/// User content with multiple content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    pub content: Vec<ContentBlock>,
}

impl UserContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }

    /// Get a short text preview of the first text block, if any
    #[allow(dead_code)]
    pub fn text_preview(&self) -> Option<String> {
        for block in &self.content {
            if let ContentBlock::Text { text } = block {
                return Some(text.clone());
            }
        }
        None
    }
}

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
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
    Image { source: ImageSource },
}

/// Image content source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Assistant content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantContent {
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
}

impl AssistantContent {
    /// Get a short text preview of the first text block, if any
    #[allow(dead_code)]
    pub fn text_preview(&self) -> Option<String> {
        for block in &self.content {
            if let ContentBlock::Text { text } = block {
                return Some(text.clone());
            }
        }
        None
    }
}

/// Progress data for streaming updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    pub message: Option<String>,
    pub progress: Option<f64>,
}

/// Summary of a tool use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseSummary {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}
