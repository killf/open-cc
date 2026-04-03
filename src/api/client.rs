//! Anthropic API client

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::ModelProvider;
use crate::error::CliError;
use crate::types::{AssistantContent, ContentBlock as ToolContentBlock, Message, TokenUsage};

use super::auth::{get_base_url, resolve_api_key};

/// API client for Claude API
#[derive(Clone)]
pub struct ApiClient {
    #[allow(dead_code)]
    provider: ModelProvider,
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl ApiClient {
    /// Create a new API client
    pub async fn new(
        provider: ModelProvider,
        api_key: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<Self, CliError> {
        let api_key = resolve_api_key(provider, api_key).await?;
        let base_url = get_base_url(provider, base_url);

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(CliError::Http)?;

        Ok(Self {
            provider,
            api_key,
            base_url,
            http_client,
        })
    }

    /// Send a non-streaming chat request
    pub async fn chat(
        &self,
        session: &crate::types::Session,
        model: &str,
        max_tokens: u32,
        tools: Option<&[std::sync::Arc<dyn crate::types::Tool>]>,
    ) -> Result<ChatResponse, CliError> {
        let url = format!("{}/v1/messages", self.base_url);
        let system = extract_system_message(&session.messages, session.system_prompt.as_deref());
        let api_messages = messages_into_api(session.messages.clone());
        let api_tools = tools.map(tools_slice_into_api);

        let body = ChatRequestBody {
            model: model.to_string(),
            messages: api_messages,
            system,
            max_tokens,
            temperature: None,
            stop_sequences: None,
            tools: api_tools,
            stream: false,
        };

        let body_str = serde_json::to_string(&body)?;

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(CliError::Http)?;

        let status = response.status();

        if status.as_u16() == 401 {
            return Err(CliError::Api("authentication failed".into()));
        }
        if status.as_u16() == 429 {
            return Err(CliError::Api("rate limited".into()));
        }
        if status.as_u16() == 400 {
            let body = response.text().await.unwrap_or_default();
            return Err(CliError::Api(format!("bad request: {body}")));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(CliError::Api(format!("status {status}: {body}")));
        }

        response
            .json::<ChatResponse>()
            .await
            .map_err(|e| CliError::Api(format!("json error: {e}")))
    }

    /// Send a streaming chat request
    pub async fn chat_streaming<F>(
        &self,
        session: &crate::types::Session,
        model: &str,
        max_tokens: u32,
        tools: Option<&[std::sync::Arc<dyn crate::types::Tool>]>,
        mut on_text: F,
    ) -> Result<ChatResponse, CliError>
    where
        F: FnMut(String) + Send,
    {
        let url = format!("{}/v1/messages", self.base_url);
        let system = extract_system_message(&session.messages, session.system_prompt.as_deref());
        let api_messages = messages_into_api(session.messages.clone());
        let api_tools = tools.map(tools_slice_into_api);

        let body = ChatRequestBody {
            model: model.to_string(),
            messages: api_messages,
            system,
            max_tokens,
            temperature: None,
            stop_sequences: None,
            tools: api_tools,
            stream: true,
        };

        let body_str = serde_json::to_string(&body)?;

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("anthropic-beta", "interleaved-2025-05-14")
            .body(body_str)
            .send()
            .await
            .map_err(CliError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(CliError::Api(format!("status {status}: {msg}")));
        }

        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new();
        let mut content_blocks = Vec::new();
        let mut stop_reason = None;
        let usage = TokenUsage::default();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(CliError::Http)?;
            let text = String::from_utf8_lossy(&bytes).to_string();

            for line in text.lines() {
                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }
                let data = &line[7..];
                if data == "[DONE]" {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<SseEvent>(data) {
                    match event {
                        SseEvent::ContentBlockStart { index: _, content_type: _, text: t } => {
                            if !accumulated_text.is_empty() {
                                content_blocks.push(ToolContentBlock::Text {
                                    text: std::mem::take(&mut accumulated_text),
                                });
                            }
                            if let Some(text) = t {
                                accumulated_text = text;
                            }
                        }
                        SseEvent::ContentBlockDelta { delta } => {
                            accumulated_text.push_str(&delta);
                            on_text(delta);
                        }
                        SseEvent::ContentBlockStop => {
                            if !accumulated_text.is_empty() {
                                content_blocks.push(ToolContentBlock::Text {
                                    text: std::mem::take(&mut accumulated_text),
                                });
                            }
                        }
                        SseEvent::MessageDelta { stop_reason: sr, .. } => {
                            stop_reason = sr;
                        }
                        SseEvent::MessageStop => {}
                        SseEvent::Ping => {}
                        SseEvent::Unknown => {}
                    }
                }
            }
        }

        Ok(ChatResponse {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            r#type: "message".to_string(),
            role: "assistant".to_string(),
            content: AssistantContent {
                content: content_blocks,
                model: model.to_string(),
                stop_reason: stop_reason.clone(),
            },
            model: model.to_string(),
            stop_reason,
            stop_sequence: None,
            usage,
        })
    }

    /// Estimate cost for a given model and token usage
    pub fn estimate_cost(&self, usage: &TokenUsage, model: &str) -> f64 {
        let (input_cost, output_cost) = match model {
            m if m.contains("claude-opus-4") => (0.015, 0.075),
            m if m.contains("claude-sonnet-4") => (0.003, 0.015),
            m if m.contains("claude-haiku-3") => (0.00025, 0.00125),
            _ => (0.008, 0.024),
        };

        (usage.input_tokens as f64 / 1000.0) * input_cost
            + (usage.output_tokens as f64 / 1000.0) * output_cost
    }
}

fn extract_system_message(messages: &[Message], fallback: Option<&str>) -> Option<String> {
    messages
        .first()
        .and_then(|m| match m {
            Message::System { message, .. } => Some(message.clone()),
            _ => None,
        })
        .filter(|s| !s.is_empty())
        .or_else(|| fallback.map(|s| s.to_string()))
}

fn messages_into_api(messages: Vec<Message>) -> Vec<ApiMessage> {
    messages
        .into_iter()
        .filter_map(|m| match m {
            Message::User { content } => Some(ApiMessage {
                role: "user".to_string(),
                content: content
                    .content
                    .into_iter()
                    .map(|b| match b {
                        ToolContentBlock::Text { text } => ApiContent::Text { text },
                        ToolContentBlock::Image { source } => ApiContent::Image {
                            source: source.data,
                            media_type: source.media_type,
                        },
                        _ => ApiContent::Text { text: "[complex content]".to_string() },
                    })
                    .collect(),
            }),
            Message::Assistant { content } => content.map(|c| ApiMessage {
                role: "assistant".to_string(),
                content: c
                    .content
                    .into_iter()
                    .map(|b| match b {
                        ToolContentBlock::Text { text } => ApiContent::Text { text },
                        _ => ApiContent::Text { text: "[tool use]".to_string() },
                    })
                    .collect(),
            }),
            Message::ToolResult { tool_use_id, content, .. } => Some(ApiMessage {
                role: "user".to_string(),
                content: vec![ApiContent::ToolResult { tool_use_id, content }],
            }),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Serialize)]
struct ChatRequestBody {
    model: String,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ApiTool {
    name: String,
    description: String,
    #[serde(rename = "input_schema")]
    schema: serde_json::Value,
}

fn tools_slice_into_api(tools: &[std::sync::Arc<dyn crate::types::Tool>]) -> Vec<ApiTool> {
    tools
        .iter()
        .map(|t| ApiTool {
            name: t.name().to_string(),
            description: t.description(),
            schema: t.input_schema(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: Vec<ApiContent>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ApiContent {
    Text { text: String },
    Image { source: String, media_type: String },
    #[serde(rename = "tool_result")]
    ToolResult { #[serde(rename = "tool_use_id")] tool_use_id: String, content: String },
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChatResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub role: String,
    pub content: AssistantContent,
    pub model: String,
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_sequence: Option<serde_json::Value>,
    pub usage: TokenUsage,
}

/// SSE event types from the streaming API
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum SseEvent {
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        #[serde(rename = "content_block")]
        content_type: Option<String>,
        text: Option<String>,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: String },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop,
    #[serde(rename = "message_delta")]
    MessageDelta {
        stop_reason: Option<String>,
        #[serde(default)]
        stop_sequence: Option<String>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(other)]
    Unknown,
}
