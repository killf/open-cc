//! LSP protocol types

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// LSP JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRequest {
    pub jsonrpc: String,
    pub id: LspRequestId,
    pub method: String,
    pub params: serde_json::Value,
}

/// LSP request ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspRequestId {
    Number(i64),
    String(String),
}

/// LSP notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// LSP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponse {
    pub jsonrpc: String,
    pub id: LspRequestId,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<LspError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspError {
    pub code: i32,
    pub message: String,
}

/// LSP position (0-indexed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

/// LSP text document identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspTextDocumentIdentifier {
    pub uri: String,
}

/// LSP text document position params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspTextDocumentPositionParams {
    pub text_document: LspTextDocumentIdentifier,
    pub position: LspPosition,
}

/// LSP completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub range: Option<LspRange>,
}

/// LSP completion list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCompletionList {
    pub is_incomplete: bool,
    pub items: Vec<LspCompletionItem>,
}

/// LSP hover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspHover {
    pub contents: LspHoverContents,
    pub range: Option<LspRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspHoverContents {
    String(String),
    MarkedStrings(Vec<LspMarkedString>),
    MarkupContent(LspMarkupContent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspMarkedString {
    pub language: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspMarkupContent {
    pub kind: String,
    pub value: String,
}

/// LSP initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspInitializeResult {
    pub capabilities: LspServerCapabilities,
    pub server_info: Option<LspServerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerCapabilities {
    pub text_document_sync: Option<serde_json::Value>,
    pub hover_provider: Option<bool>,
    pub completion_provider: Option<LspCompletionOptions>,
    pub definition_provider: Option<bool>,
    pub references_provider: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCompletionOptions {
    pub resolve_provider: Option<bool>,
    pub trigger_characters: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerInfo {
    pub name: String,
    pub version: Option<String>,
}

/// LSP publish diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPublishDiagnosticsParams {
    pub uri: String,
    pub diagnostics: Vec<LspDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<u32>,
    pub code: Option<LspDiagnosticCode>,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspDiagnosticCode {
    Number(i64),
    String(String),
}
