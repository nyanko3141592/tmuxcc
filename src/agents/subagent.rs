use chrono::{DateTime, Local};
use std::fmt;

/// Types of subagents that can be spawned
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentType {
    Explore,
    Plan,
    Bash,
    General,
    CodeSimplifier,
    Custom(String),
}

impl SubagentType {
    /// Parse subagent type from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "explore" => SubagentType::Explore,
            "plan" => SubagentType::Plan,
            "bash" => SubagentType::Bash,
            "general" | "general-purpose" => SubagentType::General,
            "code-simplifier" => SubagentType::CodeSimplifier,
            other => SubagentType::Custom(other.to_string()),
        }
    }

    /// Returns the display name
    pub fn display_name(&self) -> &str {
        match self {
            SubagentType::Explore => "Explore",
            SubagentType::Plan => "Plan",
            SubagentType::Bash => "Bash",
            SubagentType::General => "General",
            SubagentType::CodeSimplifier => "Simplifier",
            SubagentType::Custom(s) => s,
        }
    }
}

impl fmt::Display for SubagentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Status of a subagent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentStatus {
    Running,
    Completed,
    Failed,
    Unknown,
}

impl SubagentStatus {
    /// Returns a status indicator character
    pub fn indicator(&self) -> &str {
        match self {
            SubagentStatus::Running => "▶",
            SubagentStatus::Completed => "✓",
            SubagentStatus::Failed => "✗",
            SubagentStatus::Unknown => "?",
        }
    }
}

impl fmt::Display for SubagentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            SubagentStatus::Running => "Running",
            SubagentStatus::Completed => "Completed",
            SubagentStatus::Failed => "Failed",
            SubagentStatus::Unknown => "Unknown",
        };
        write!(f, "{} {}", self.indicator(), text)
    }
}

/// Represents a subagent spawned by a parent agent
#[derive(Debug, Clone)]
pub struct Subagent {
    /// Unique identifier
    pub id: String,
    /// Type of subagent
    pub subagent_type: SubagentType,
    /// Current status
    pub status: SubagentStatus,
    /// Description of what the subagent is doing
    pub description: String,
    /// When the subagent was started
    pub started_at: DateTime<Local>,
}

impl Subagent {
    /// Creates a new Subagent
    pub fn new(id: String, subagent_type: SubagentType, description: String) -> Self {
        Self {
            id,
            subagent_type,
            status: SubagentStatus::Running,
            description,
            started_at: Local::now(),
        }
    }

    /// Creates a subagent with a specific status
    pub fn with_status(mut self, status: SubagentStatus) -> Self {
        self.status = status;
        self
    }

    /// Returns a formatted duration since start
    pub fn duration_str(&self) -> String {
        let duration = Local::now().signed_duration_since(self.started_at);
        let secs = duration.num_seconds();

        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m{}s", secs / 60, secs % 60)
        } else {
            format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_type_from_str() {
        assert_eq!(SubagentType::parse("explore"), SubagentType::Explore);
        assert_eq!(SubagentType::parse("Explore"), SubagentType::Explore);
        assert_eq!(SubagentType::parse("plan"), SubagentType::Plan);
        assert_eq!(
            SubagentType::parse("custom-agent"),
            SubagentType::Custom("custom-agent".to_string())
        );
    }

    #[test]
    fn test_subagent_status_indicator() {
        assert_eq!(SubagentStatus::Running.indicator(), "▶");
        assert_eq!(SubagentStatus::Completed.indicator(), "✓");
        assert_eq!(SubagentStatus::Failed.indicator(), "✗");
    }

    #[test]
    fn test_subagent_creation() {
        let subagent = Subagent::new(
            "sub-1".to_string(),
            SubagentType::Explore,
            "searching codebase".to_string(),
        );
        assert_eq!(subagent.subagent_type, SubagentType::Explore);
        assert_eq!(subagent.status, SubagentStatus::Running);
    }
}
