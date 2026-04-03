//! MCP tool loader - connects to MCP servers and returns tools

use std::collections::HashMap;
use std::sync::Arc;

use crate::config::McpServerConfig;
use crate::error::CliError;
use crate::mcp::client::McpClient;
use crate::types::Tool;

use super::adapter::McpToolAdapter;

/// Load and connect to MCP servers, returning all available tools
pub async fn load_mcp_tools(
    servers: &HashMap<String, McpServerConfig>,
) -> Result<Vec<Arc<dyn Tool>>, CliError> {
    let mut tools = Vec::new();

    for (server_name, server_config) in servers {
        let server_tools = load_single_server(server_name, server_config).await?;
        tools.extend(server_tools);
    }

    Ok(tools)
}

/// Load tools from a single MCP server
async fn load_single_server(
    server_name: &str,
    config: &McpServerConfig,
) -> Result<Vec<Arc<dyn Tool>>, CliError> {
    match config.config_type {
        crate::config::McpServerType::Stdio => {
            load_stdio_server(server_name, config).await
        }
        _ => {
            // Currently only stdio is implemented
            eprintln!(
                "[Warning] MCP server '{}' uses unsupported transport type {:?}, skipping",
                server_name, config.config_type
            );
            Ok(Vec::new())
        }
    }
}

/// Connect to an MCP server via stdio
async fn load_stdio_server(
    server_name: &str,
    config: &McpServerConfig,
) -> Result<Vec<Arc<dyn Tool>>, CliError> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| CliError::Mcp(format!("MCP server '{}': missing command", server_name)))?;

    let args = config.args.as_deref().unwrap_or(&[]);
    let env = config.env.clone().unwrap_or_default();

    let client = Arc::new(McpClient::connect_stdio(command, args, &env).await?);
    let mcp_tools = client.list_tools().await?;

    let tools: Vec<Arc<dyn Tool>> = mcp_tools
        .into_iter()
        .map(|mcp_tool| {
            Arc::new(McpToolAdapter::new(
                mcp_tool.name,
                mcp_tool.description,
                mcp_tool.input_schema,
                Arc::clone(&client),
            )) as Arc<dyn Tool>
        })
        .collect();

    Ok(tools)
}
