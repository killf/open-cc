//! WebSocket transport for remote MCP servers

use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;

use crate::error::CliError;
use crate::mcp::protocol::McpMessage;
use crate::mcp::client::McpTransport;

use async_trait::async_trait;

type WsSink = Pin<Box<dyn futures::Sink<tokio_tungstenite::tungstenite::Message, Error=tokio_tungstenite::tungstenite::Error> + Send>>;
type WsStream = Pin<Box<dyn futures::Stream<Item=Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>> + Send>>;

pub struct WebSocketTransport {
    #[allow(dead_code)]
    url: String,
    sender: tokio::sync::Mutex<WsSink>,
    receiver: tokio::sync::Mutex<WsStream>,
}

impl WebSocketTransport {
    pub async fn connect(url: &str, _headers: &HashMap<String, String>) -> Result<Self, CliError> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| CliError::Mcp(format!("WebSocket connect failed: {e}")))?;

        let (sender, receiver) = ws_stream.split();
        Ok(Self {
            url: url.to_string(),
            sender: tokio::sync::Mutex::new(Box::pin(sender)),
            receiver: tokio::sync::Mutex::new(Box::pin(receiver)),
        })
    }
}

#[async_trait]
impl McpTransport for WebSocketTransport {
    async fn send(&self, msg: McpMessage) -> Result<(), CliError> {
        let json = serde_json::to_string(&msg)
            .map_err(|e| CliError::Mcp(format!("json error: {e}")))?;
        let mut sender = self.sender.lock().await;
        sender.send(tokio_tungstenite::tungstenite::Message::text(json))
            .await
            .map_err(|e| CliError::Mcp(format!("WebSocket send failed: {e}")))?;
        Ok(())
    }

    async fn recv(&self) -> Result<McpMessage, CliError> {
        let mut receiver = self.receiver.lock().await;
        while let Some(msg) = receiver.next().await {
            let msg = msg.map_err(|e| CliError::Mcp(format!("WebSocket recv failed: {e}")))?;
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                if let Ok(mcp_msg) = serde_json::from_str::<McpMessage>(&text) {
                    return Ok(mcp_msg);
                }
            }
        }
        Err(CliError::Mcp("WebSocket stream ended".into()))
    }
}
