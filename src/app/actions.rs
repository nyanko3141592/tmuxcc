/// Actions that can be performed in the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Quit the application
    Quit,
    /// Navigate to next agent
    NextAgent,
    /// Navigate to previous agent
    PrevAgent,
    /// Toggle selection of current agent
    ToggleSelection,
    /// Select all agents
    SelectAll,
    /// Clear selection
    ClearSelection,
    /// Approve the current/selected request(s)
    Approve,
    /// Reject the current/selected request(s)
    Reject,
    /// Approve all pending requests
    ApproveAll,
    /// Focus on the selected tmux pane
    FocusPane,
    /// Toggle subagent log view
    ToggleSubagentLog,
    /// Toggle summary detail (TODOs and Tools) view
    ToggleSummaryDetail,
    /// Refresh agent list
    Refresh,
    /// Show help
    ShowHelp,
    /// Hide help
    HideHelp,
    /// Focus on input panel
    FocusInput,
    /// Focus on sidebar
    FocusSidebar,
    /// Focus on preview panel
    FocusPreview,
    /// Cycle focus to next panel
    CycleFocus,
    /// Send input to selected agent
    SendInput,
    /// Clear input buffer
    ClearInput,
    /// Add character to input
    InputChar(char),
    /// Add newline to input
    InputNewline,
    /// Delete last character
    InputBackspace,
    /// Move cursor left
    CursorLeft,
    /// Move cursor right
    CursorRight,
    /// Move cursor to beginning
    CursorHome,
    /// Move cursor to end
    CursorEnd,
    /// Send a specific number (for choice selection)
    SendNumber(u8),
    /// Increase sidebar width
    SidebarWider,
    /// Decrease sidebar width
    SidebarNarrower,
    /// Select agent by index (mouse click)
    SelectAgent(usize),
    /// Scroll up in sidebar
    ScrollUp,
    /// Scroll down in sidebar
    ScrollDown,
    /// Toggle collapse of current session
    ToggleCollapse,
    /// Collapse all sessions
    CollapseAll,
    /// Expand all sessions
    ExpandAll,
    /// Kill pane (first press of dd)
    KillPanePending,
    /// Kill pane (confirmed)
    KillPane,
    /// Start spawn mode
    SpawnStart,
    /// Spawn a specific agent type
    SpawnAgent(String),
    /// Cancel spawn mode
    SpawnCancel,
    /// Start rename mode for the selected pane's window
    RenameStart,
    /// Execute the rename with the given name
    RenameExecute(String),
    /// Cancel rename mode
    RenameCancel,
    /// No action (used for unbound keys)
    None,
}

impl Action {
    /// Returns a description of the action for help display
    pub fn description(&self) -> &str {
        match self {
            Action::Quit => "Quit application",
            Action::NextAgent => "Select next agent",
            Action::PrevAgent => "Select previous agent",
            Action::ToggleSelection => "Toggle selection",
            Action::SelectAll => "Select all agents",
            Action::ClearSelection => "Clear selection",
            Action::Approve => "Approve selected request(s)",
            Action::Reject => "Reject selected request(s)",
            Action::ApproveAll => "Approve all pending requests",
            Action::FocusPane => "Focus on selected pane in tmux",
            Action::ToggleSubagentLog => "Toggle subagent log",
            Action::ToggleSummaryDetail => "Toggle TODO/Tools display",
            Action::Refresh => "Refresh agent list",
            Action::ShowHelp => "Show help",
            Action::HideHelp => "Hide help",
            Action::FocusInput => "Focus input panel",
            Action::FocusSidebar => "Focus sidebar",
            Action::FocusPreview => "Focus preview panel",
            Action::CycleFocus => "Cycle focus between panels",
            Action::SendInput => "Send input",
            Action::ClearInput => "Clear input",
            Action::InputChar(_) => "Type character",
            Action::InputNewline => "Insert newline",
            Action::InputBackspace => "Delete character",
            Action::CursorLeft => "Move cursor left",
            Action::CursorRight => "Move cursor right",
            Action::CursorHome => "Move cursor to start",
            Action::CursorEnd => "Move cursor to end",
            Action::SendNumber(_) => "Send choice number",
            Action::SidebarWider => "Widen sidebar",
            Action::SidebarNarrower => "Narrow sidebar",
            Action::SelectAgent(_) => "Select agent",
            Action::ScrollUp => "Scroll up",
            Action::ScrollDown => "Scroll down",
            Action::ToggleCollapse => "Toggle collapse session",
            Action::CollapseAll => "Collapse all sessions",
            Action::ExpandAll => "Expand all sessions",
            Action::KillPanePending => "Kill pane (press d again)",
            Action::KillPane => "Kill pane",
            Action::SpawnStart => "Spawn new agent",
            Action::SpawnAgent(_) => "Spawn agent",
            Action::SpawnCancel => "Cancel spawn",
            Action::RenameStart => "Rename window",
            Action::RenameExecute(_) => "Execute rename",
            Action::RenameCancel => "Cancel rename",
            Action::None => "",
        }
    }
}
