//! Tool registry

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::Tool;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    aliases: HashMap<String, String>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: impl Tool + 'static) {
        let name = tool.name().to_string();
        let arc: Arc<dyn Tool> = Arc::new(tool);
        self.tools.insert(name.clone(), Arc::clone(&arc));

        for alias in arc.aliases() {
            self.aliases.insert(alias, name.clone());
        }
    }

    /// Get a tool by name or alias
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .get(name)
            .cloned()
            .or_else(|| {
                self.aliases
                    .get(name)
                    .and_then(|n| self.tools.get(n).cloned())
            })
    }

    /// Get all registered tools
    pub fn get_all(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Filter tools by allowed/denied lists
    pub fn filter(&self, allowed: &[String], denied: &[String]) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| {
                let name = t.name();
                if denied.iter().any(|d| d == name || d == "*") {
                    return false;
                }
                if allowed.is_empty() || allowed.iter().any(|a| a == name || a == "*") {
                    return true;
                }
                false
            })
            .cloned()
            .collect()
    }

    /// Get tool names
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Register all built-in tools
    pub fn register_builtins() -> Self {
        let mut registry = Self::new();

        registry.register(super::bash::BashTool::new());
        registry.register(super::file_read::FileReadTool::new());
        registry.register(super::file_write::FileWriteTool::new());
        registry.register(super::file_edit::FileEditTool::new());
        registry.register(super::grep::GrepTool::new());
        registry.register(super::glob::GlobTool::new());
        registry.register(super::task_tool::TaskTool::new(std::env::temp_dir()));
        registry.register(super::task_create::TaskCreateTool::new());
        registry.register(super::task_get::TaskGetTool::new());
        registry.register(super::task_list::TaskListTool::new());
        registry.register(super::task_output::TaskOutputTool::new());
        registry.register(super::task_stop::TaskStopTool::new());
        registry.register(super::task_update::TaskUpdateTool::new());
        registry.register(super::team_create::TeamCreateTool::new());
        registry.register(super::team_delete::TeamDeleteTool::new());
        registry.register(super::web_fetch::WebFetchTool::new());
        registry.register(super::web_search::WebSearchTool::new());
        registry.register(super::send_user_message::SendUserMessageTool::new());
        registry.register(super::send_message::SendMessageTool::new());
        registry.register(super::ask_question::AskQuestionTool::new());
        registry.register(super::config_tool::ConfigTool::new());
        registry.register(super::discover_skills::DiscoverSkillsTool::new());
        registry.register(super::list_mcp_resources::ListMcpResourcesTool::new());
        registry.register(super::mcp_auth::McpAuthTool::new());
        registry.register(super::read_mcp_resource::ReadMcpResourceTool::new());
        registry.register(super::send_user_file::SendUserFileTool::new());
        registry.register(super::snip::SnipTool::new());
        registry.register(super::terminal_capture::TerminalCaptureTool::new());
        registry.register(super::enter_plan_mode::EnterPlanModeTool::new());
        registry.register(super::exit_plan_mode::ExitPlanModeTool::new());
        registry.register(super::monitor::MonitorTool::new());
        registry.register(super::overflow_test::OverflowTestTool::new());
        registry.register(super::review_artifact::ReviewArtifactTool::new());
        registry.register(super::web_browser::WebBrowserTool::new());
        registry.register(super::workflow::WorkflowTool::new());
        registry.register(super::tungsten::TungstenTool::new());
        registry.register(super::verify_plan_execution::VerifyPlanExecutionTool::new());
        registry.register(super::schedule_cron::ScheduleCronTool::new());
        registry.register(super::skill::SkillTool::new());
        registry.register(super::sleep::SleepTool::new());
        registry.register(super::synthetic_output::SyntheticOutputTool::new());
        registry.register(super::todo_write::TodoWriteTool::new());
        registry.register(super::tool_search::ToolSearchTool::new());
        registry.register(super::enter_worktree::EnterWorktreeTool::new());
        registry.register(super::exit_worktree::ExitWorktreeTool::new());
        registry.register(super::lsp::LspTool::new());
        registry.register(super::notebook_edit::NotebookEditTool::new());
        registry.register(super::powershell::PowerShellTool::new());
        registry.register(super::remote_trigger::RemoteTriggerTool::new());
        registry.register(super::repl::ReplTool::new());

        registry
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
