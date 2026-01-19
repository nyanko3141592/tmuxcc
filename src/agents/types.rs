use std::fmt;
use std::time::Instant;

use super::subagent::Subagent;

/// Types of AI agents that can be monitored
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentType {
    ClaudeCode,
    OpenCode,
    CodexCli,
    GeminiCli,
    Unknown,
}

impl AgentType {
    /// Returns the display name of the agent
    pub fn display_name(&self) -> &str {
        match self {
            AgentType::ClaudeCode => "Claude Code",
            AgentType::OpenCode => "OpenCode",
            AgentType::CodexCli => "Codex CLI",
            AgentType::GeminiCli => "Gemini CLI",
            AgentType::Unknown => "Unknown",
        }
    }

    /// Returns a short name for the agent (for compact display)
    pub fn short_name(&self) -> &str {
        match self {
            AgentType::ClaudeCode => "Claude",
            AgentType::OpenCode => "Open",
            AgentType::CodexCli => "Codex",
            AgentType::GeminiCli => "Gemini",
            AgentType::Unknown => "???",
        }
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Types of approvals that agents may request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalType {
    FileEdit,
    FileCreate,
    FileDelete,
    ShellCommand,
    McpTool,
    /// User question with choices (AskUserQuestion tool)
    UserQuestion {
        /// Available choices (label only)
        choices: Vec<String>,
        /// Whether multiple selections are allowed
        multi_select: bool,
    },
    Other(String),
}

impl ApprovalType {
    /// Returns a short description for UI display
    pub fn short_desc(&self) -> &str {
        match self {
            ApprovalType::FileEdit => "Edit",
            ApprovalType::FileCreate => "Create",
            ApprovalType::FileDelete => "Delete",
            ApprovalType::ShellCommand => "Shell",
            ApprovalType::McpTool => "MCP",
            ApprovalType::UserQuestion { .. } => "Question",
            ApprovalType::Other(_) => "Other",
        }
    }

    /// Returns true if this is a y/n type approval
    pub fn is_yes_no(&self) -> bool {
        matches!(
            self,
            ApprovalType::FileEdit
                | ApprovalType::FileCreate
                | ApprovalType::FileDelete
                | ApprovalType::ShellCommand
                | ApprovalType::McpTool
                | ApprovalType::Other(_)
        )
    }

    /// Returns true if this is a user question with choices
    pub fn is_question(&self) -> bool {
        matches!(self, ApprovalType::UserQuestion { .. })
    }
}

impl fmt::Display for ApprovalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApprovalType::FileEdit => write!(f, "File Edit"),
            ApprovalType::FileCreate => write!(f, "File Create"),
            ApprovalType::FileDelete => write!(f, "File Delete"),
            ApprovalType::ShellCommand => write!(f, "Shell Command"),
            ApprovalType::McpTool => write!(f, "MCP Tool"),
            ApprovalType::UserQuestion { choices, .. } => {
                write!(f, "Question ({} choices)", choices.len())
            }
            ApprovalType::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Status of an AI agent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is idle and ready for input
    Idle,
    /// Agent is actively processing
    Processing { activity: String },
    /// Agent is waiting for user approval
    AwaitingApproval {
        approval_type: ApprovalType,
        details: String,
    },
    /// Agent encountered an error
    Error { message: String },
    /// Unable to determine agent status
    Unknown,
}

impl AgentStatus {
    /// Returns true if the agent needs user attention
    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            AgentStatus::AwaitingApproval { .. } | AgentStatus::Error { .. }
        )
    }

    /// Returns a short status indicator for UI
    pub fn indicator(&self) -> &str {
        match self {
            AgentStatus::Idle => "●",
            AgentStatus::Processing { .. } => "◐",
            AgentStatus::AwaitingApproval { .. } => "⚠",
            AgentStatus::Error { .. } => "✗",
            AgentStatus::Unknown => "?",
        }
    }

    /// Returns a short status text
    pub fn short_text(&self) -> String {
        match self {
            AgentStatus::Idle => "Idle".to_string(),
            AgentStatus::Processing { activity } => {
                if activity.is_empty() {
                    "Processing".to_string()
                } else {
                    activity.clone()
                }
            }
            AgentStatus::AwaitingApproval { approval_type, .. } => {
                format!("APPROVAL NEEDED [{}]", approval_type.short_desc())
            }
            AgentStatus::Error { message } => format!("Error: {}", message),
            AgentStatus::Unknown => "Unknown".to_string(),
        }
    }
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.indicator(), self.short_text())
    }
}

/// Represents a monitored AI agent in a tmux pane
#[derive(Debug, Clone)]
pub struct MonitoredAgent {
    /// Unique identifier for this agent
    pub id: String,
    /// Tmux target (e.g., "main:0.1")
    pub target: String,
    /// Session name
    pub session: String,
    /// Window index
    pub window: u32,
    /// Window name
    pub window_name: String,
    /// Pane index
    pub pane: u32,
    /// Current working directory
    pub path: String,
    /// Type of AI agent
    pub agent_type: AgentType,
    /// Current status
    pub status: AgentStatus,
    /// Detected subagents
    pub subagents: Vec<Subagent>,
    /// Last captured pane content
    pub last_content: String,
    /// Process ID
    pub pid: u32,
    /// When this agent was first detected
    pub started_at: Instant,
    /// When the pane content was last updated
    pub last_updated: Instant,
    /// Context remaining percentage (0-100), if detectable
    pub context_remaining: Option<u8>,
    /// Git branch name (if in a git repo)
    pub git_branch: Option<String>,
}

impl MonitoredAgent {
    /// Creates a new MonitoredAgent
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        target: String,
        session: String,
        window: u32,
        window_name: String,
        pane: u32,
        path: String,
        agent_type: AgentType,
        pid: u32,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            target,
            session,
            window,
            window_name,
            pane,
            path,
            agent_type,
            status: AgentStatus::Unknown,
            subagents: Vec::new(),
            last_content: String::new(),
            pid,
            started_at: now,
            last_updated: now,
            context_remaining: None,
            git_branch: None,
        }
    }

    /// Returns the duration since this agent was first detected
    pub fn uptime(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Returns a human-readable uptime string
    pub fn uptime_str(&self) -> String {
        let secs = self.uptime().as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Returns a human-readable last updated string
    pub fn last_updated_str(&self) -> String {
        let secs = self.last_updated.elapsed().as_secs();
        if secs < 5 {
            "now".to_string()
        } else if secs < 60 {
            format!("{}s ago", secs)
        } else {
            format!("{}m ago", secs / 60)
        }
    }

    /// Updates the last_updated timestamp
    pub fn touch(&mut self) {
        self.last_updated = Instant::now();
    }

    /// Returns a short path (last component or abbreviated)
    pub fn short_path(&self) -> String {
        if self.path.is_empty() {
            return "~".to_string();
        }
        // Get just the last directory name
        self.path
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or(&self.path)
            .to_string()
    }

    /// Returns an abbreviated path like /U/t/D/H/TmuxCC
    pub fn abbreviated_path(&self) -> String {
        if self.path.is_empty() {
            return "~".to_string();
        }

        let parts: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return "/".to_string();
        }

        if parts.len() == 1 {
            return format!("/{}", parts[0]);
        }

        // Abbreviate all but the last component
        let abbreviated: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|s| s.chars().next().unwrap_or('?').to_string())
            .collect();

        format!("/{}/{}", abbreviated.join("/"), parts.last().unwrap())
    }

    /// Returns the number of active subagents
    pub fn active_subagent_count(&self) -> usize {
        use super::subagent::SubagentStatus;
        self.subagents
            .iter()
            .filter(|s| matches!(s.status, SubagentStatus::Running))
            .count()
    }

    /// Returns true if any subagent is currently running
    pub fn has_active_subagents(&self) -> bool {
        self.active_subagent_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::ClaudeCode.display_name(), "Claude Code");
        assert_eq!(AgentType::OpenCode.short_name(), "Open");
    }

    #[test]
    fn test_agent_status_needs_attention() {
        assert!(!AgentStatus::Idle.needs_attention());
        assert!(!AgentStatus::Processing {
            activity: "thinking".to_string()
        }
        .needs_attention());
        assert!(AgentStatus::AwaitingApproval {
            approval_type: ApprovalType::FileEdit,
            details: "test".to_string()
        }
        .needs_attention());
        assert!(AgentStatus::Error {
            message: "test".to_string()
        }
        .needs_attention());
    }

    #[test]
    fn test_monitored_agent() {
        let agent = MonitoredAgent::new(
            "agent-1".to_string(),
            "main:0.1".to_string(),
            "main".to_string(),
            0,
            "code".to_string(),
            1,
            "/home/user/project".to_string(),
            AgentType::ClaudeCode,
            12345,
        );
        assert_eq!(agent.target, "main:0.1");
        assert_eq!(agent.active_subagent_count(), 0);
        assert_eq!(agent.short_path(), "project");
    }
}
