//! Tests for the Plugin system

use open_cc::plugins::{
    Plugin, PluginManifest, PluginRegistry,
};
use std::path::PathBuf;

#[tokio::test]
async fn test_plugin_registry_new() {
    let registry = PluginRegistry::new();
    let tasks = registry.list_tasks().await;
    assert!(tasks.is_empty());
}

#[test]
fn test_plugin_manifest_deserialize() {
    let json = r#"{
        "name": "my-plugin",
        "version": "1.0.0",
        "description": "A test plugin",
        "author": "Test Author",
        "tools": ["tool-a", "tool-b"],
        "commands": ["/cmd1"],
        "hooks": ["hook-a"],
        "main": "index.js",
        "dependencies": []
    }"#;

    let manifest: PluginManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.name, "my-plugin");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.tools, &["tool-a", "tool-b"]);
    assert_eq!(manifest.commands, &["/cmd1"]);
    assert_eq!(manifest.hooks, &["hook-a"]);
    assert!(manifest.main.is_some());
}

#[test]
fn test_plugin_manifest_minimal() {
    let json = r#"{
        "name": "minimal",
        "version": "0.1.0",
        "tools": [],
        "commands": [],
        "hooks": []
    }"#;

    let manifest: PluginManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.name, "minimal");
    assert!(manifest.description.is_none());
    assert!(manifest.author.is_none());
    assert!(manifest.main.is_none());
}

#[test]
fn test_plugin_manifest_tools_only() {
    let json = r#"{
        "name": "tools-only",
        "version": "2.0.0",
        "tools": ["read-file", "search"],
        "commands": [],
        "hooks": []
    }"#;

    let manifest: PluginManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.tools.len(), 2);
    assert!(manifest.tools.contains(&"read-file".to_string()));
}

#[test]
fn test_plugin_from_manifest() {
    let manifest = PluginManifest {
        name: "derived-plugin".to_string(),
        version: "3.0.0".to_string(),
        description: Some("Derived from manifest".to_string()),
        author: Some("Author".to_string()),
        tools: vec!["t1".to_string(), "t2".to_string()],
        commands: vec!["/c1".to_string()],
        hooks: vec!["h1".to_string()],
        main: Some("main.js".to_string()),
        dependencies: vec![],
    };

    let plugin = Plugin {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        tools: manifest.tools.clone(),
        commands: manifest.commands.clone(),
        hooks: manifest.hooks.clone(),
        path: PathBuf::from("/plugins/derived-plugin"),
    };

    assert_eq!(plugin.name, "derived-plugin");
    assert_eq!(plugin.version, "3.0.0");
    assert_eq!(plugin.tools, &["t1", "t2"]);
    assert_eq!(plugin.commands, &["/c1"]);
    assert_eq!(plugin.hooks, &["h1"]);
    assert_eq!(plugin.path, PathBuf::from("/plugins/derived-plugin"));
}

#[test]
fn test_plugin_registry_has_tool() {
    let registry = PluginRegistry::new();
    assert!(!registry.has_tool("any-tool"));
}

#[test]
fn test_plugin_registry_tool_path() {
    let registry = PluginRegistry::new();
    let path = registry.tool_path("nonexistent");
    assert!(path.is_none());
}

#[test]
fn test_plugin_registry_get() {
    let registry = PluginRegistry::new();
    let plugin = registry.get("any-plugin");
    assert!(plugin.is_none());
}
