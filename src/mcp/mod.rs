//! Model Context Protocol (MCP) implementation

pub mod adapter;
pub mod client;
pub mod loader;
pub mod protocol;
pub mod transport;

pub use adapter::McpToolAdapter;
pub use client::*;
pub use loader::load_mcp_tools;
pub use protocol::*;
pub use transport::*;
