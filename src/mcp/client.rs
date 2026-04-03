//! MCP client implementation

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;

use crate::error::CliError;

use super::protocol::*;

/// MCP client for connecting to MCP servers
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
    next_id: Arc<Mutex<i64>>,
}

impl McpClient {
    /// Connect to an MCP server via stdio
    pub async fn connect_stdio(command: &str, args: &[String], env: &HashMap<String, String>) -> Result<Self, CliError> {
        let transport = Box::new(StdioTransport::new(command, args, env).await?);
        let mut client = Self {
            transport,
            capabilities: ServerCapabilities::default(),
            server_info: ServerInfo {
                name: "unknown".to_string(),
                version: "0.0.0".to_string(),
            },
            next_id: Arc::new(Mutex::new(1)),
        };
        client.initialize().await?;
        Ok(client)
    }

    /// Initialize the MCP session
    async fn initialize(&mut self) -> Result<(), CliError> {
        let id = { *self.next_id.lock().await };
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: McpRequestId::Number(id),
            method: methods::INITIALIZE.to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": ClientCapabilities::default(),
                "clientInfo": {
                    "name": "claude-code-rust",
                    "version": "2.1.88"
                }
            })),
        };

        self.transport.send(McpMessage::Request(request)).await?;
        let response = self.transport.recv().await?;

        if let McpMessage::Response(resp) = response {
            if let Some(result) = resp.result {
                if let Ok(init_result) = serde_json::from_value::<InitializeResult>(result) {
                    self.capabilities = init_result.capabilities;
                    self.server_info = init_result.server_info;
                }
            }
        }

        // Send initialized notification
        let notification = McpNotification {
            jsonrpc: "2.0".to_string(),
            method: "initialized".to_string(),
            params: None,
        };
        self.transport.send(McpMessage::Notification(notification)).await?;

        Ok(())
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, CliError> {
        let id = {
            let mut next = self.next_id.lock().await;
            *next += 1;
            *next
        };
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: McpRequestId::Number(id),
            method: methods::TOOLS_LIST.to_string(),
            params: None,
        };

        self.transport.send(McpMessage::Request(request)).await?;
        let response = self.transport.recv().await?;

        if let McpMessage::Response(resp) = response {
            if let Some(result) = resp.result {
                let list_result = serde_json::from_value::<ToolListResult>(result)?;
                return Ok(list_result.tools);
            }
        }

        Err(CliError::Mcp("Failed to list tools".into()))
    }

    /// Call a tool
    pub async fn call_tool(&self, name: &str, arguments: Option<serde_json::Value>) -> Result<ToolCallResult, CliError> {
        let id = {
            let mut next = self.next_id.lock().await;
            *next += 1;
            *next
        };
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: McpRequestId::Number(id),
            method: methods::TOOLS_CALL.to_string(),
            params: Some(serde_json::json!({
                "name": name,
                "arguments": arguments.unwrap_or(serde_json::json!({}))
            })),
        };

        self.transport.send(McpMessage::Request(request)).await?;
        let response = self.transport.recv().await?;

        if let McpMessage::Response(resp) = response {
            if let Some(result) = resp.result {
                let call_result = serde_json::from_value::<ToolCallResult>(result)?;
                return Ok(call_result);
            }
        }

        Err(CliError::Mcp("Failed to call tool".into()))
    }
}

/// MCP transport trait
#[async_trait::async_trait]
pub trait McpTransport: Send + Sync {
    async fn send(&self, msg: McpMessage) -> Result<(), CliError>;
    async fn recv(&self) -> Result<McpMessage, CliError>;
}

/// Stdio transport implementation
pub struct StdioTransport {
    // Kept to ensure the child process lives as long as the transport.
    // When dropped (with kill_on_drop=true), the process is terminated.
    #[allow(dead_code)]
    child: tokio::process::Child,
    writer: tokio::sync::Mutex<tokio::io::BufWriter<tokio::process::ChildStdin>>,
    reader: tokio::sync::Mutex<tokio::io::BufReader<tokio::process::ChildStdout>>,
}

impl StdioTransport {
    pub async fn new(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self, CliError> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        // Merge env
        cmd.envs(env);

        let mut child = cmd.spawn()
            .map_err(|e| CliError::Mcp(format!("failed to spawn {command}: {e}")))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| CliError::Mcp("no stdin".into()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| CliError::Mcp("no stdout".into()))?;

        Ok(Self {
            child,
            writer: tokio::sync::Mutex::new(tokio::io::BufWriter::new(stdin)),
            reader: tokio::sync::Mutex::new(tokio::io::BufReader::new(stdout)),
        })
    }
}

#[async_trait::async_trait]
impl McpTransport for StdioTransport {
    async fn send(&self, msg: McpMessage) -> Result<(), CliError> {
        let json = serde_json::to_string(&msg)
            .map_err(|e| CliError::Mcp(format!("json error: {e}")))?;
        let line = format!("{json}\n");
        let mut writer = self.writer.lock().await;
        tokio::io::AsyncWriteExt::write_all(&mut *writer, line.as_bytes()).await
            .map_err(|e| CliError::Mcp(format!("write error: {e}")))?;
        tokio::io::AsyncWriteExt::flush(&mut *writer).await
            .map_err(|e| CliError::Mcp(format!("flush error: {e}")))?;
        Ok(())
    }

    async fn recv(&self) -> Result<McpMessage, CliError> {
        let mut line = String::new();
        let mut reader = self.reader.lock().await;
        reader.read_line(&mut line).await
            .map_err(|e| CliError::Mcp(format!("read error: {e}")))?;
        let msg: McpMessage = serde_json::from_str(&line)
            .map_err(|e| CliError::Mcp(format!("parse error: {e}")))?;
        Ok(msg)
    }
}
