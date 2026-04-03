//! Plugin system for extending Claude Code functionality

pub mod tool;

use crate::error::CliError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A loaded plugin
#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub tools: Vec<String>,
    pub commands: Vec<String>,
    pub hooks: Vec<String>,
    #[allow(dead_code)]
    pub path: PathBuf,
}

/// Plugin manifest (plugin.json)
#[derive(Debug, serde::Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    #[allow(dead_code)]
    pub author: Option<String>,
    pub tools: Vec<String>,
    pub commands: Vec<String>,
    pub hooks: Vec<String>,
    #[allow(dead_code)]
    pub main: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub dependencies: Vec<String>,
}

/// Plugin registry
pub struct PluginRegistry {
    plugins: HashMap<String, Plugin>,
    plugin_dirs: Vec<PathBuf>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dirs: Self::default_plugin_dirs(),
        }
    }

    fn default_plugin_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(config) = dirs::config_dir() {
            dirs.push(config.join("claude-code").join("plugins"));
        }
        dirs.push(PathBuf::from(".claude/plugins"));
        dirs
    }

    /// Add a plugin search directory
    #[allow(dead_code)]
    pub fn add_plugin_dir(&mut self, path: PathBuf) {
        if !self.plugin_dirs.contains(&path) {
            self.plugin_dirs.push(path);
        }
    }

    /// Discover and load all plugins
    pub async fn load_all(&mut self) -> Result<(), CliError> {
        let dirs: Vec<PathBuf> = self.plugin_dirs.clone();
        for dir in dirs {
            if dir.exists() {
                self.load_plugins_in_dir(&dir).await?;
            }
        }
        Ok(())
    }

    async fn load_plugins_in_dir(&mut self, dir: &Path) -> Result<(), CliError> {
        let mut entries = tokio::fs::read_dir(dir).await
            .map_err(CliError::Io)?;

        while let Some(entry) = entries.next_entry().await
            .map_err(CliError::Io)? {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    self.load_plugin(&path).await?;
                }
            }
        }
        Ok(())
    }

    /// Load a single plugin from a directory
    pub async fn load_plugin(&mut self, path: &Path) -> Result<(), CliError> {
        let manifest_path = path.join("plugin.json");
        let content = tokio::fs::read_to_string(&manifest_path).await
            .map_err(CliError::Io)?;

        let manifest: PluginManifest = serde_json::from_str(&content)
            .map_err(CliError::Json)?;

        let plugin = Plugin {
            name: manifest.name.clone(),
            version: manifest.version,
            tools: manifest.tools,
            commands: manifest.commands,
            hooks: manifest.hooks,
            path: path.to_path_buf(),
        };

        self.plugins.insert(manifest.name.clone(), plugin);
        Ok(())
    }

    /// Get a plugin by name
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&Plugin> {
        self.plugins.get(name)
    }

    /// List all loaded plugins
    pub fn list(&self) -> Vec<&Plugin> {
        self.plugins.values().collect()
    }

    /// List tasks (compatibility alias for list)
    #[allow(dead_code)]
    pub async fn list_tasks(&self) -> Vec<Plugin> {
        self.plugins.values().cloned().collect()
    }

    /// Check if a tool is provided by any plugin
    #[allow(dead_code)]
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.plugins.values().any(|p| p.tools.contains(&tool_name.to_string()))
    }

    /// Get the path to a plugin's tool executable
    #[allow(dead_code)]
    pub fn tool_path(&self, tool_name: &str) -> Option<PathBuf> {
        for plugin in self.plugins.values() {
            if plugin.tools.contains(&tool_name.to_string()) {
                return Some(plugin.path.join("tools").join(tool_name));
            }
        }
        None
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
