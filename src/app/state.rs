use crate::agents::MonitoredAgent;
use crate::monitor::SystemStats;
use std::collections::HashSet;
use std::time::Instant;

/// A pane that is not running a recognized agent
#[derive(Debug, Clone)]
pub struct NonAgentPane {
    pub target: String,
    pub session: String,
    pub window: u32,
    pub window_name: String,
    pub pane: u32,
    pub command: String,
    pub path: String,
}

/// Which panel is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPanel {
    /// Agent list sidebar is focused
    #[default]
    Sidebar,
    /// Preview/message chain area is focused
    Preview,
    /// Input area is focused
    Input,
}

/// Semantic cursor that can point to either a session header, an agent, or a non-agent pane
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeCursor {
    /// Cursor is on a session header
    Session(String),
    /// Cursor is on an agent (index into root_agents)
    Agent(usize),
    /// Cursor is on a non-agent pane (index into non_agent_panes)
    NonAgentPane(usize),
}

impl Default for TreeCursor {
    fn default() -> Self {
        TreeCursor::Agent(0)
    }
}

/// Represents a navigable item in the tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavItem {
    Session(String),
    Agent(usize),
    NonAgentPane(usize),
}

/// Tree structure containing all monitored agents
#[derive(Debug, Clone, Default)]
pub struct AgentTree {
    /// Root agents (directly in tmux panes)
    pub root_agents: Vec<MonitoredAgent>,
    /// Panes that don't match any agent parser
    pub non_agent_panes: Vec<NonAgentPane>,
}

impl AgentTree {
    /// Creates an empty agent tree
    pub fn new() -> Self {
        Self {
            root_agents: Vec::new(),
            non_agent_panes: Vec::new(),
        }
    }

    /// Returns the total number of agents (including subagents)
    pub fn total_count(&self) -> usize {
        self.root_agents.iter().map(|a| 1 + a.subagents.len()).sum()
    }

    /// Returns the number of active agents (those needing attention)
    pub fn active_count(&self) -> usize {
        self.root_agents
            .iter()
            .filter(|a| a.status.needs_attention())
            .count()
    }

    /// Returns the total number of running subagents
    pub fn running_subagent_count(&self) -> usize {
        use crate::agents::SubagentStatus;
        self.root_agents
            .iter()
            .flat_map(|a| &a.subagents)
            .filter(|s| matches!(s.status, SubagentStatus::Running))
            .count()
    }

    /// Returns the number of processing agents
    pub fn processing_count(&self) -> usize {
        use crate::agents::AgentStatus;
        self.root_agents
            .iter()
            .filter(|a| matches!(a.status, AgentStatus::Processing { .. }))
            .count()
    }

    /// Gets an agent by index (for selection)
    pub fn get_agent(&self, index: usize) -> Option<&MonitoredAgent> {
        self.root_agents.get(index)
    }

    /// Gets a mutable agent by index
    pub fn get_agent_mut(&mut self, index: usize) -> Option<&mut MonitoredAgent> {
        self.root_agents.get_mut(index)
    }
}

/// Spinner frames for animation
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Main application state
#[derive(Debug)]
pub struct AppState {
    /// Tree of monitored agents
    pub agents: AgentTree,
    /// All tmux session names (including those without agents)
    pub all_sessions: Vec<String>,
    /// Semantic cursor (session header, agent, or non-agent pane)
    pub cursor: TreeCursor,
    /// Multi-selected agent indices
    pub selected_agents: HashSet<usize>,
    /// Collapsed session names
    pub collapsed_sessions: HashSet<String>,
    /// Which panel is focused
    pub focused_panel: FocusedPanel,
    /// Input buffer (always available)
    pub input_buffer: String,
    /// Cursor position within input buffer (byte offset)
    pub cursor_position: usize,
    /// Whether help is being shown
    pub show_help: bool,
    /// Help scroll offset
    pub help_scroll: u16,
    /// Whether subagent log is shown
    pub show_subagent_log: bool,
    /// Whether summary detail (TODOs and Tools) is shown
    pub show_summary_detail: bool,
    /// Whether the application should quit
    pub should_quit: bool,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Sidebar width in percentage (15-70)
    pub sidebar_width: u16,
    /// Animation tick counter
    pub tick: usize,
    /// Last tick time for animation throttling
    last_tick: Instant,
    /// System resource statistics
    pub system_stats: SystemStats,
    /// Pending kill confirmation: (target, timestamp)
    pub pending_kill: Option<(String, Instant)>,
    /// Spawn mode: Some(session_name) when active
    pub spawn_mode: Option<String>,
    /// Rename mode: Some(target) when active
    pub rename_mode: Option<String>,
}

impl AppState {
    /// Creates a new AppState with default settings
    pub fn new() -> Self {
        Self {
            agents: AgentTree::new(),
            all_sessions: Vec::new(),
            cursor: TreeCursor::default(),
            selected_agents: HashSet::new(),
            collapsed_sessions: HashSet::new(),
            focused_panel: FocusedPanel::Sidebar,
            input_buffer: String::new(),
            cursor_position: 0,
            show_help: false,
            help_scroll: 0,
            show_subagent_log: false,
            show_summary_detail: true,
            should_quit: false,
            last_error: None,
            sidebar_width: 35,
            tick: 0,
            last_tick: Instant::now(),
            system_stats: SystemStats::new(),
            pending_kill: None,
            spawn_mode: None,
            rename_mode: None,
        }
    }

    /// Build the flat navigation list: session headers + visible agents + non-agent panes in display order
    pub fn build_nav_items(&self) -> Vec<NavItem> {
        use std::collections::BTreeMap;

        // Group agents by session
        let mut agent_sessions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (idx, agent) in self.agents.root_agents.iter().enumerate() {
            agent_sessions
                .entry(&agent.session)
                .or_default()
                .push(idx);
        }

        // Group non-agent panes by session
        let mut nap_sessions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (idx, nap) in self.agents.non_agent_panes.iter().enumerate() {
            nap_sessions
                .entry(&nap.session)
                .or_default()
                .push(idx);
        }

        // Collect all session names (from agents, non-agent panes, and all_sessions)
        let mut all_session_names: BTreeMap<&str, ()> = BTreeMap::new();
        for s in agent_sessions.keys() {
            all_session_names.insert(s, ());
        }
        for s in nap_sessions.keys() {
            all_session_names.insert(s, ());
        }
        for s in &self.all_sessions {
            all_session_names.insert(s.as_str(), ());
        }

        let mut items = Vec::new();
        for (session, _) in &all_session_names {
            items.push(NavItem::Session(session.to_string()));
            if !self.collapsed_sessions.contains(*session) {
                if let Some(agent_indices) = agent_sessions.get(session) {
                    for &idx in agent_indices {
                        items.push(NavItem::Agent(idx));
                    }
                }
                if let Some(nap_indices) = nap_sessions.get(session) {
                    for &idx in nap_indices {
                        items.push(NavItem::NonAgentPane(idx));
                    }
                }
            }
        }
        items
    }

    /// Returns the agent index when cursor is on an agent
    pub fn selected_agent_index(&self) -> Option<usize> {
        match &self.cursor {
            TreeCursor::Agent(idx) => Some(*idx),
            _ => None,
        }
    }

    /// Returns the non-agent pane index when cursor is on one
    pub fn selected_non_agent_index(&self) -> Option<usize> {
        match &self.cursor {
            TreeCursor::NonAgentPane(idx) => Some(*idx),
            _ => None,
        }
    }

    /// Returns the currently selected non-agent pane
    pub fn selected_non_agent_pane(&self) -> Option<&NonAgentPane> {
        self.selected_non_agent_index()
            .and_then(|idx| self.agents.non_agent_panes.get(idx))
    }

    /// Returns the currently selected agent (None when cursor is on a session or non-agent pane)
    pub fn selected_agent(&self) -> Option<&MonitoredAgent> {
        self.selected_agent_index()
            .and_then(|idx| self.agents.get_agent(idx))
    }

    /// Returns the currently selected agent mutably
    pub fn selected_agent_mut(&mut self) -> Option<&mut MonitoredAgent> {
        self.selected_agent_index()
            .and_then(|idx| self.agents.get_agent_mut(idx))
    }

    /// Returns the target of whatever pane the cursor is on (agent or non-agent)
    pub fn selected_pane_target(&self) -> Option<String> {
        match &self.cursor {
            TreeCursor::Agent(idx) => self.agents.get_agent(*idx).map(|a| a.target.clone()),
            TreeCursor::NonAgentPane(idx) => {
                self.agents.non_agent_panes.get(*idx).map(|p| p.target.clone())
            }
            TreeCursor::Session(_) => None,
        }
    }

    /// Returns the window name of whatever pane the cursor is on
    pub fn selected_pane_window_name(&self) -> Option<String> {
        match &self.cursor {
            TreeCursor::Agent(idx) => {
                self.agents.get_agent(*idx).map(|a| a.window_name.clone())
            }
            TreeCursor::NonAgentPane(idx) => {
                self.agents.non_agent_panes.get(*idx).map(|p| p.window_name.clone())
            }
            TreeCursor::Session(_) => None,
        }
    }

    /// Returns the session name the cursor is on or in
    pub fn selected_session(&self) -> Option<&str> {
        match &self.cursor {
            TreeCursor::Session(s) => Some(s),
            TreeCursor::Agent(idx) => self
                .agents
                .get_agent(*idx)
                .map(|a| a.session.as_str()),
            TreeCursor::NonAgentPane(idx) => self
                .agents
                .non_agent_panes
                .get(*idx)
                .map(|p| p.session.as_str()),
        }
    }

    /// Advance the animation tick (throttled to ~10fps for spinner)
    pub fn tick(&mut self) {
        const TICK_INTERVAL_MS: u128 = 80; // ~12fps for smooth spinner
        if self.last_tick.elapsed().as_millis() >= TICK_INTERVAL_MS {
            self.tick = self.tick.wrapping_add(1);
            self.last_tick = Instant::now();
        }
    }

    /// Get the current spinner frame
    pub fn spinner_frame(&self) -> &'static str {
        SPINNER_FRAMES[self.tick % SPINNER_FRAMES.len()]
    }

    /// Check if input panel is focused
    pub fn is_input_focused(&self) -> bool {
        self.focused_panel == FocusedPanel::Input
    }

    /// Check if preview panel is focused
    pub fn is_preview_focused(&self) -> bool {
        self.focused_panel == FocusedPanel::Preview
    }

    /// Focus on the input panel
    pub fn focus_input(&mut self) {
        self.focused_panel = FocusedPanel::Input;
    }

    /// Focus on the sidebar
    pub fn focus_sidebar(&mut self) {
        self.focused_panel = FocusedPanel::Sidebar;
    }

    /// Focus on the preview
    pub fn focus_preview(&mut self) {
        self.focused_panel = FocusedPanel::Preview;
    }

    /// Cycle focus: Sidebar → Preview → Input → Sidebar
    pub fn toggle_focus(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Sidebar => FocusedPanel::Preview,
            FocusedPanel::Preview => FocusedPanel::Input,
            FocusedPanel::Input => FocusedPanel::Sidebar,
        };
    }

    /// Add a character to the input buffer at cursor position
    pub fn input_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    /// Add a newline to the input buffer at cursor position
    pub fn input_newline(&mut self) {
        self.input_buffer.insert(self.cursor_position, '\n');
        self.cursor_position += 1;
    }

    /// Delete the character before the cursor
    pub fn input_backspace(&mut self) {
        if self.cursor_position > 0 {
            // Find the previous character boundary
            let prev_boundary = self.input_buffer[..self.cursor_position]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input_buffer.remove(prev_boundary);
            self.cursor_position = prev_boundary;
        }
    }

    /// Get the current input buffer
    pub fn get_input(&self) -> &str {
        &self.input_buffer
    }

    /// Get the current cursor position
    pub fn get_cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Take and clear the input buffer
    pub fn take_input(&mut self) -> String {
        self.cursor_position = 0;
        std::mem::take(&mut self.input_buffer)
    }

    /// Move cursor left by one character
    pub fn cursor_left(&mut self) {
        if self.cursor_position > 0 {
            // Find the previous character boundary
            self.cursor_position = self.input_buffer[..self.cursor_position]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right by one character
    pub fn cursor_right(&mut self) {
        if self.cursor_position < self.input_buffer.len() {
            // Find the next character boundary
            if let Some(c) = self.input_buffer[self.cursor_position..].chars().next() {
                self.cursor_position += c.len_utf8();
            }
        }
    }

    /// Move cursor to the beginning of the input
    pub fn cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to the end of the input
    pub fn cursor_end(&mut self) {
        self.cursor_position = self.input_buffer.len();
    }

    /// Selects the next item in navigation order
    pub fn select_next(&mut self) {
        let nav_items = self.build_nav_items();
        if nav_items.is_empty() {
            return;
        }

        // Find current position
        let current_pos = self.find_nav_position(&nav_items);
        let next_pos = match current_pos {
            Some(pos) => (pos + 1) % nav_items.len(),
            None => 0,
        };

        self.set_cursor_from_nav(&nav_items[next_pos]);
    }

    /// Selects the previous item in navigation order
    pub fn select_prev(&mut self) {
        let nav_items = self.build_nav_items();
        if nav_items.is_empty() {
            return;
        }

        let current_pos = self.find_nav_position(&nav_items);
        let prev_pos = match current_pos {
            Some(pos) => {
                if pos == 0 {
                    nav_items.len() - 1
                } else {
                    pos - 1
                }
            }
            None => 0,
        };

        self.set_cursor_from_nav(&nav_items[prev_pos]);
    }

    /// Find the current cursor's position in the nav items list
    fn find_nav_position(&self, nav_items: &[NavItem]) -> Option<usize> {
        nav_items.iter().position(|item| match (&self.cursor, item) {
            (TreeCursor::Session(s1), NavItem::Session(s2)) => s1 == s2,
            (TreeCursor::Agent(i1), NavItem::Agent(i2)) => i1 == i2,
            (TreeCursor::NonAgentPane(i1), NavItem::NonAgentPane(i2)) => i1 == i2,
            _ => false,
        })
    }

    /// Set cursor from a NavItem
    fn set_cursor_from_nav(&mut self, item: &NavItem) {
        self.cursor = match item {
            NavItem::Session(s) => TreeCursor::Session(s.clone()),
            NavItem::Agent(idx) => TreeCursor::Agent(*idx),
            NavItem::NonAgentPane(idx) => TreeCursor::NonAgentPane(*idx),
        };
    }

    /// Selects an agent by index
    pub fn select_agent(&mut self, index: usize) {
        if index < self.agents.root_agents.len() {
            self.cursor = TreeCursor::Agent(index);
        }
    }

    /// Toggles selection of the current agent
    pub fn toggle_selection(&mut self) {
        if let Some(idx) = self.selected_agent_index() {
            if self.selected_agents.contains(&idx) {
                self.selected_agents.remove(&idx);
            } else {
                self.selected_agents.insert(idx);
            }
        }
    }

    /// Selects all agents
    pub fn select_all(&mut self) {
        for i in 0..self.agents.root_agents.len() {
            self.selected_agents.insert(i);
        }
    }

    /// Clears all selections
    pub fn clear_selection(&mut self) {
        self.selected_agents.clear();
    }

    /// Returns indices to operate on (selected agents, or current if none selected)
    pub fn get_operation_indices(&self) -> Vec<usize> {
        if self.selected_agents.is_empty() {
            if let Some(idx) = self.selected_agent_index() {
                vec![idx]
            } else {
                vec![]
            }
        } else {
            let mut indices: Vec<usize> = self.selected_agents.iter().copied().collect();
            indices.sort();
            indices
        }
    }

    /// Check if an agent is in multi-selection
    pub fn is_multi_selected(&self, index: usize) -> bool {
        self.selected_agents.contains(&index)
    }

    /// Toggle collapse of the current session
    pub fn toggle_collapse(&mut self) {
        if let Some(session) = self.selected_session().map(|s| s.to_string()) {
            if self.collapsed_sessions.contains(&session) {
                self.collapsed_sessions.remove(&session);
            } else {
                self.collapsed_sessions.insert(session);
            }
        }
    }

    /// Collapse all sessions
    pub fn collapse_all(&mut self) {
        for session in &self.all_sessions {
            self.collapsed_sessions.insert(session.clone());
        }
    }

    /// Expand all sessions
    pub fn expand_all(&mut self) {
        self.collapsed_sessions.clear();
    }

    /// Toggles help display
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            self.help_scroll = 0;
        }
    }

    /// Toggles subagent log display
    pub fn toggle_subagent_log(&mut self) {
        self.show_subagent_log = !self.show_subagent_log;
    }

    /// Toggles summary detail (TODOs and Tools) display
    pub fn toggle_summary_detail(&mut self) {
        self.show_summary_detail = !self.show_summary_detail;
    }

    /// Sets an error message
    pub fn set_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    /// Clears the error message
    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    /// Clamp cursor to valid range after agents update
    pub fn clamp_cursor(&mut self) {
        match &self.cursor {
            TreeCursor::Agent(idx) => {
                if *idx >= self.agents.root_agents.len() {
                    if self.agents.root_agents.is_empty() {
                        self.cursor = TreeCursor::Agent(0);
                    } else {
                        self.cursor =
                            TreeCursor::Agent(self.agents.root_agents.len() - 1);
                    }
                }
            }
            TreeCursor::NonAgentPane(idx) => {
                if *idx >= self.agents.non_agent_panes.len() {
                    if self.agents.non_agent_panes.is_empty() {
                        self.cursor = TreeCursor::Agent(0);
                    } else {
                        self.cursor =
                            TreeCursor::NonAgentPane(self.agents.non_agent_panes.len() - 1);
                    }
                }
            }
            TreeCursor::Session(session) => {
                // Check if session still exists in agents, non-agent panes, or all_sessions
                let exists = self
                    .agents
                    .root_agents
                    .iter()
                    .any(|a| a.session == *session)
                    || self
                        .agents
                        .non_agent_panes
                        .iter()
                        .any(|p| p.session == *session)
                    || self.all_sessions.iter().any(|s| s == session);
                if !exists {
                    self.cursor = TreeCursor::Agent(0);
                }
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentType;

    fn make_agent(session: &str, target: &str, window: u32, pane: u32) -> MonitoredAgent {
        MonitoredAgent::new(
            format!("{}-1", target),
            target.to_string(),
            session.to_string(),
            window,
            "code".to_string(),
            pane,
            "/home/user/project".to_string(),
            AgentType::ClaudeCode,
            1000,
        )
    }

    #[test]
    fn test_app_state_navigation() {
        let mut state = AppState::new();

        // Add some agents in same session
        state
            .agents
            .root_agents
            .push(make_agent("main", "main:0.0", 0, 0));
        state
            .agents
            .root_agents
            .push(make_agent("main", "main:0.1", 0, 1));

        // Cursor starts at Agent(0)
        assert_eq!(state.cursor, TreeCursor::Agent(0));

        // Next: Agent(0) -> Agent(1)
        state.select_next();
        assert_eq!(state.selected_agent_index(), Some(1));

        // Next wraps: Agent(1) -> Session("main")
        state.select_next();
        assert_eq!(state.cursor, TreeCursor::Session("main".to_string()));

        // Next: Session("main") -> Agent(0)
        state.select_next();
        assert_eq!(state.selected_agent_index(), Some(0));

        // Prev wraps: Agent(0) -> Session("main")
        state.select_prev();
        assert_eq!(state.cursor, TreeCursor::Session("main".to_string()));

        // Prev: Session("main") -> Agent(1) (wrap to end)
        state.select_prev();
        assert_eq!(state.selected_agent_index(), Some(1));
    }

    #[test]
    fn test_selected_session() {
        let mut state = AppState::new();
        state
            .agents
            .root_agents
            .push(make_agent("dev", "dev:0.0", 0, 0));

        state.cursor = TreeCursor::Session("dev".to_string());
        assert_eq!(state.selected_session(), Some("dev"));
        assert_eq!(state.selected_agent_index(), None);
        assert!(state.selected_agent().is_none());

        state.cursor = TreeCursor::Agent(0);
        assert_eq!(state.selected_session(), Some("dev"));
        assert_eq!(state.selected_agent_index(), Some(0));
        assert!(state.selected_agent().is_some());
    }

    #[test]
    fn test_collapsed_navigation() {
        let mut state = AppState::new();
        state
            .agents
            .root_agents
            .push(make_agent("alpha", "alpha:0.0", 0, 0));
        state
            .agents
            .root_agents
            .push(make_agent("beta", "beta:0.0", 0, 0));

        // Collapse alpha session
        state.collapsed_sessions.insert("alpha".to_string());

        // Nav items should be: Session(alpha), Session(beta), Agent(1)
        let nav = state.build_nav_items();
        assert_eq!(nav.len(), 3);
        assert_eq!(nav[0], NavItem::Session("alpha".to_string()));
        assert_eq!(nav[1], NavItem::Session("beta".to_string()));
        assert_eq!(nav[2], NavItem::Agent(1));
    }
}
