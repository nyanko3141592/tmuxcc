use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::app::{Action, AppState, Config, TreeCursor};
use crate::monitor::{MonitorTask, SystemStatsCollector};
use crate::parsers::ParserRegistry;
use crate::tmux::TmuxClient;

use super::components::{
    AgentTreeWidget, FooterWidget, HeaderWidget, HelpWidget, InputWidget, PanePreviewWidget,
    SubagentLogWidget,
};
use super::Layout;

/// Runs the main application loop
pub async fn run_app(config: Config) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize state
    let mut state = AppState::new();

    // Create tmux client and parser registry
    let tmux_client = Arc::new(TmuxClient::with_capture_lines(config.capture_lines));
    let parser_registry = Arc::new(ParserRegistry::new());

    // Check if tmux is available
    if !tmux_client.is_available() {
        state.set_error("tmux is not running".to_string());
    }

    // Create channel for monitor updates
    let (tx, mut rx) = mpsc::channel(32);

    // Start monitor task
    let monitor = MonitorTask::new(
        tmux_client.clone(),
        parser_registry.clone(),
        tx,
        Duration::from_millis(config.poll_interval_ms),
    );
    let monitor_handle = tokio::spawn(async move {
        monitor.run().await;
    });

    // Create system stats collector
    let mut system_stats = SystemStatsCollector::new();

    // Main loop
    let result = run_loop(
        &mut terminal,
        &mut state,
        &mut rx,
        &tmux_client,
        &mut system_stats,
    )
    .await;

    // Cleanup
    monitor_handle.abort();
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    rx: &mut mpsc::Receiver<crate::monitor::MonitorUpdate>,
    tmux_client: &TmuxClient,
    system_stats: &mut SystemStatsCollector,
) -> Result<()> {
    loop {
        // Advance animation tick
        state.tick();

        // Update system stats
        system_stats.refresh();
        state.system_stats = system_stats.stats().clone();

        // Draw UI
        terminal.draw(|frame| {
            let size = frame.area();
            let main_chunks = Layout::main_layout(size);

            // Header
            HeaderWidget::render(frame, main_chunks[0], state);

            // Always show input widget at bottom of right column
            let input_height = InputWidget::calculate_height(state.get_input(), 6);

            if state.show_subagent_log {
                // With subagent log: sidebar | summary+preview+input | subagent_log
                let (left, preview, subagent_log) =
                    Layout::content_layout_with_log(main_chunks[1], state.sidebar_width);
                AgentTreeWidget::render(frame, left, state);

                // Split preview area for summary, preview, and input
                let preview_chunks = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        ratatui::layout::Constraint::Length(15),
                        ratatui::layout::Constraint::Min(5),
                        ratatui::layout::Constraint::Length(input_height + 2),
                    ])
                    .split(preview);
                PanePreviewWidget::render_summary(frame, preview_chunks[0], state);
                PanePreviewWidget::render_detailed(frame, preview_chunks[1], state);
                InputWidget::render(frame, preview_chunks[2], state);
                SubagentLogWidget::render(frame, subagent_log, state);
            } else {
                // Normal: sidebar | summary+preview+input
                let (left, summary, preview, input_area) = Layout::content_layout_with_input(
                    main_chunks[1],
                    state.sidebar_width,
                    input_height,
                    state.show_summary_detail,
                );
                AgentTreeWidget::render(frame, left, state);
                if state.show_summary_detail {
                    PanePreviewWidget::render_summary(frame, summary, state);
                }
                PanePreviewWidget::render_detailed(frame, preview, state);
                InputWidget::render(frame, input_area, state);
            }

            // Footer
            FooterWidget::render(frame, main_chunks[2], state);

            // Help overlay
            if state.show_help {
                HelpWidget::render(frame, size, state.help_scroll);
            }
        })?;

        // Handle events with short timeout for responsive UI (~60fps)
        let timeout = Duration::from_millis(16);

        tokio::select! {
            // Handle monitor updates
            Some(update) = rx.recv() => {
                state.agents = update.agents;
                state.all_sessions = update.all_sessions;
                // Clamp cursor to valid range
                state.clamp_cursor();
                // Clean up invalid selections
                let max_idx = state.agents.root_agents.len();
                state.selected_agents.retain(|&idx| idx < max_idx);
            }

            // Handle keyboard and mouse events
            _ = tokio::time::sleep(timeout) => {
                // Process all pending events to avoid input lag
                while event::poll(Duration::from_millis(0))? {
                    let event = event::read()?;

                    // Handle mouse events
                    if let Event::Mouse(mouse) = event {
                        let size = terminal.size()?;
                        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                        let main_chunks = Layout::main_layout(area);
                        let footer_area = main_chunks[2];
                        let (sidebar, _, _, input_area) = Layout::content_layout_with_input(
                            main_chunks[1], state.sidebar_width, 3, state.show_summary_detail
                        );

                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                let x = mouse.column;
                                let y = mouse.row;

                                // Check footer button clicks first
                                if let Some(button) = FooterWidget::hit_test(x, y, footer_area, state) {
                                    use super::components::FooterButton;
                                    match button {
                                        FooterButton::Approve => {
                                            let indices = state.get_operation_indices();
                                            for idx in indices {
                                                if let Some(agent) = state.agents.get_agent(idx) {
                                                    if agent.status.needs_attention() {
                                                        let target = agent.target.clone();
                                                        let _ = tmux_client.send_keys(&target, "y");
                                                        let _ = tmux_client.send_keys(&target, "Enter");
                                                    }
                                                }
                                            }
                                            state.clear_selection();
                                        }
                                        FooterButton::Reject => {
                                            let indices = state.get_operation_indices();
                                            for idx in indices {
                                                if let Some(agent) = state.agents.get_agent(idx) {
                                                    if agent.status.needs_attention() {
                                                        let target = agent.target.clone();
                                                        let _ = tmux_client.send_keys(&target, "n");
                                                        let _ = tmux_client.send_keys(&target, "Enter");
                                                    }
                                                }
                                            }
                                            state.clear_selection();
                                        }
                                        FooterButton::ApproveAll => {
                                            for agent in &state.agents.root_agents {
                                                if agent.status.needs_attention() {
                                                    let _ = tmux_client.send_keys(&agent.target, "y");
                                                    let _ = tmux_client.send_keys(&agent.target, "Enter");
                                                }
                                            }
                                        }
                                        FooterButton::ToggleSelect => {
                                            state.toggle_selection();
                                        }
                                        FooterButton::Focus => {
                                            if let Some(target) = state.selected_pane_target() {
                                                let _ = tmux_client.focus_pane(&target);
                                            }
                                        }
                                        FooterButton::Help => {
                                            state.toggle_help();
                                        }
                                        FooterButton::Quit => {
                                            state.should_quit = true;
                                        }
                                    }
                                }
                                // Check if click is in sidebar - try to select agent
                                else if x >= sidebar.x && x < sidebar.x + sidebar.width
                                    && y >= sidebar.y && y < sidebar.y + sidebar.height
                                {
                                    state.focus_sidebar();
                                    // Calculate which agent was clicked based on row
                                    let rel_y = (y - sidebar.y).saturating_sub(1) as usize;
                                    let agents_count = state.agents.root_agents.len();
                                    if agents_count > 0 {
                                        // Estimate ~4 lines per agent (header + info + status)
                                        let estimated_idx = rel_y / 4;
                                        if estimated_idx < agents_count {
                                            state.select_agent(estimated_idx);
                                        }
                                    }
                                }
                                // Check if click is in input area
                                else if x >= input_area.x && x < input_area.x + input_area.width
                                    && y >= input_area.y && y < input_area.y + input_area.height
                                {
                                    state.focus_input();
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                state.select_prev();
                            }
                            MouseEventKind::ScrollDown => {
                                state.select_next();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Handle keyboard events
                    if let Event::Key(key) = event {
                        let action = map_key_to_action(key.code, key.modifiers, state);

                        // Clear pending_kill on any action that isn't KillPane
                        if !matches!(action, Action::KillPanePending | Action::KillPane) {
                            state.pending_kill = None;
                        }

                        // Clear rename_mode on non-rename/non-input actions
                        if state.rename_mode.is_some()
                            && !matches!(
                                action,
                                Action::RenameStart
                                    | Action::RenameExecute(_)
                                    | Action::RenameCancel
                                    | Action::InputChar(_)
                                    | Action::InputBackspace
                                    | Action::CursorLeft
                                    | Action::CursorRight
                                    | Action::CursorHome
                                    | Action::CursorEnd
                            )
                        {
                            state.rename_mode = None;
                            state.take_input();
                        }

                        match action {
                            Action::Quit => {
                                state.should_quit = true;
                            }
                            Action::NextAgent => {
                                state.select_next();
                            }
                            Action::PrevAgent => {
                                state.select_prev();
                            }
                            Action::ToggleSelection => {
                                state.toggle_selection();
                            }
                            Action::SelectAll => {
                                state.select_all();
                            }
                            Action::ClearSelection => {
                                state.clear_selection();
                            }
                            Action::Approve => {
                                let indices = state.get_operation_indices();
                                for idx in indices {
                                    if let Some(agent) = state.agents.get_agent(idx) {
                                        if agent.status.needs_attention() {
                                            let target = agent.target.clone();
                                            if let Err(e) = tmux_client.send_keys(&target, "y") {
                                                state.set_error(format!("Failed to approve: {}", e));
                                                break;
                                            }
                                            if let Err(e) = tmux_client.send_keys(&target, "Enter") {
                                                state.set_error(format!("Failed to send Enter: {}", e));
                                                break;
                                            }
                                        }
                                    }
                                }
                                state.clear_selection();
                            }
                            Action::Reject => {
                                let indices = state.get_operation_indices();
                                for idx in indices {
                                    if let Some(agent) = state.agents.get_agent(idx) {
                                        if agent.status.needs_attention() {
                                            let target = agent.target.clone();
                                            if let Err(e) = tmux_client.send_keys(&target, "n") {
                                                state.set_error(format!("Failed to reject: {}", e));
                                                break;
                                            }
                                            if let Err(e) = tmux_client.send_keys(&target, "Enter") {
                                                state.set_error(format!("Failed to send Enter: {}", e));
                                                break;
                                            }
                                        }
                                    }
                                }
                                state.clear_selection();
                            }
                            Action::ApproveAll => {
                                for agent in &state.agents.root_agents {
                                    if agent.status.needs_attention() {
                                        if let Err(e) = tmux_client.send_keys(&agent.target, "y") {
                                            state.set_error(format!("Failed to approve {}: {}", agent.target, e));
                                            break;
                                        }
                                        if let Err(e) = tmux_client.send_keys(&agent.target, "Enter") {
                                            state.set_error(format!("Failed to send Enter to {}: {}", agent.target, e));
                                            break;
                                        }
                                    }
                                }
                            }
                            Action::FocusPane => {
                                if let Some(target) = state.selected_pane_target() {
                                    if let Err(e) = tmux_client.focus_pane(&target) {
                                        state.set_error(format!("Failed to focus: {}", e));
                                    }
                                }
                            }
                            Action::ToggleCollapse => {
                                state.toggle_collapse();
                            }
                            Action::CollapseAll => {
                                state.collapse_all();
                            }
                            Action::ExpandAll => {
                                state.expand_all();
                            }
                            Action::ToggleSubagentLog => {
                                state.toggle_subagent_log();
                            }
                            Action::ToggleSummaryDetail => {
                                state.toggle_summary_detail();
                            }
                            Action::Refresh => {
                                state.clear_error();
                            }
                            Action::ShowHelp => {
                                state.toggle_help();
                            }
                            Action::HideHelp => {
                                state.show_help = false;
                            }
                            Action::FocusInput => {
                                state.focus_input();
                            }
                            Action::FocusSidebar => {
                                state.focus_sidebar();
                            }
                            Action::FocusPreview => {
                                state.focus_preview();
                            }
                            Action::CycleFocus => {
                                state.toggle_focus();
                            }
                            Action::ClearInput => {
                                state.take_input();
                            }
                            Action::InputChar(c) => {
                                state.input_char(c);
                            }
                            Action::InputNewline => {
                                state.input_newline();
                            }
                            Action::InputBackspace => {
                                state.input_backspace();
                            }
                            Action::CursorLeft => {
                                state.cursor_left();
                            }
                            Action::CursorRight => {
                                state.cursor_right();
                            }
                            Action::CursorHome => {
                                state.cursor_home();
                            }
                            Action::CursorEnd => {
                                state.cursor_end();
                            }
                            Action::SendInput => {
                                let input = state.take_input();
                                if !input.is_empty() {
                                    if let Some(agent) = state.selected_agent() {
                                        let target = agent.target.clone();
                                        // Send the input text
                                        if let Err(e) = tmux_client.send_keys(&target, &input) {
                                            state.set_error(format!("Failed to send input: {}", e));
                                        } else if let Err(e) = tmux_client.send_keys(&target, "Enter") {
                                            state.set_error(format!("Failed to send Enter: {}", e));
                                        }
                                    }
                                }
                                // Stay in input mode for consecutive inputs
                            }
                            Action::SendNumber(num) => {
                                if let Some(agent) = state.selected_agent() {
                                    let target = agent.target.clone();
                                    let num_str = num.to_string();
                                    if let Err(e) = tmux_client.send_keys(&target, &num_str) {
                                        state.set_error(format!("Failed to send number: {}", e));
                                    } else if let Err(e) = tmux_client.send_keys(&target, "Enter") {
                                        state.set_error(format!("Failed to send Enter: {}", e));
                                    }
                                }
                            }
                            Action::SidebarWider => {
                                state.sidebar_width = (state.sidebar_width + 5).min(70);
                            }
                            Action::SidebarNarrower => {
                                state.sidebar_width = state.sidebar_width.saturating_sub(5).max(15);
                            }
                            Action::SelectAgent(idx) => {
                                state.select_agent(idx);
                            }
                            Action::ScrollUp => {
                                if state.show_help {
                                    state.help_scroll = state.help_scroll.saturating_sub(1);
                                } else {
                                    state.select_prev();
                                }
                            }
                            Action::ScrollDown => {
                                if state.show_help {
                                    state.help_scroll = state.help_scroll.saturating_add(1);
                                } else {
                                    state.select_next();
                                }
                            }
                            Action::KillPanePending => {
                                // First 'd' press - set pending
                                if let Some(target) = state.selected_pane_target() {
                                    state.pending_kill = Some((target, std::time::Instant::now()));
                                }
                            }
                            Action::KillPane => {
                                // Second 'd' press confirmed - execute kill
                                if let Some((target, _)) = state.pending_kill.take() {
                                    if let Err(e) = tmux_client.kill_pane(&target) {
                                        state.set_error(format!("Failed to kill pane: {}", e));
                                    }
                                }
                            }
                            Action::SpawnStart => {
                                if let Some(session) = state.selected_session().map(|s| s.to_string()) {
                                    state.spawn_mode = Some(session);
                                }
                            }
                            Action::SpawnAgent(cmd) => {
                                if let Some(session) = state.spawn_mode.take() {
                                    // Look up cwd from any existing pane in the session
                                    let session_path = state
                                        .agents
                                        .root_agents
                                        .iter()
                                        .find(|a| a.session == session)
                                        .map(|a| a.path.clone())
                                        .or_else(|| {
                                            state.agents.non_agent_panes
                                                .iter()
                                                .find(|p| p.session == session)
                                                .map(|p| p.path.clone())
                                        });
                                    let cwd = session_path.as_deref();

                                    // Build window name: "{agent_type}-{basename_of_cwd}"
                                    let agent_short = cmd.split_whitespace().next().unwrap_or("agent");
                                    let dir_basename = cwd
                                        .and_then(|p| p.rsplit('/').find(|s| !s.is_empty()))
                                        .unwrap_or("agent");
                                    let window_name = format!("{}-{}", agent_short, dir_basename);

                                    if let Err(e) = tmux_client.new_window(&session, &window_name, &cmd, cwd) {
                                        state.set_error(format!("Failed to spawn: {}", e));
                                    }
                                }
                            }
                            Action::SpawnCancel => {
                                state.spawn_mode = None;
                            }
                            Action::RenameStart => {
                                if let Some(target) = state.selected_pane_target() {
                                    // Pre-fill input buffer with current window name
                                    let window_name = state
                                        .selected_pane_window_name()
                                        .unwrap_or_default();
                                    state.input_buffer = window_name.clone();
                                    state.cursor_position = window_name.len();
                                    state.rename_mode = Some(target);
                                }
                            }
                            Action::RenameExecute(name) => {
                                if let Some(target) = state.rename_mode.take() {
                                    let name = name.trim().to_string();
                                    if !name.is_empty() {
                                        if let Err(e) = tmux_client.rename_window(&target, &name) {
                                            state.set_error(format!("Failed to rename: {}", e));
                                        }
                                    }
                                    state.take_input();
                                }
                            }
                            Action::RenameCancel => {
                                state.rename_mode = None;
                                state.take_input();
                            }
                            Action::None => {}
                        }
                    }
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

fn map_key_to_action(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Action {
    // If help is shown, handle scroll or close
    if state.show_help {
        return match code {
            KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
            KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('h') => Action::HideHelp,
            _ => Action::None,  // ignore other keys while help is open
        };
    }

    // If rename mode is active, handle rename keys
    if state.rename_mode.is_some() {
        return match code {
            KeyCode::Enter => {
                let name = state.get_input().to_string();
                Action::RenameExecute(name)
            }
            KeyCode::Esc => Action::RenameCancel,
            KeyCode::Backspace => Action::InputBackspace,
            KeyCode::Left => Action::CursorLeft,
            KeyCode::Right => Action::CursorRight,
            KeyCode::Home => Action::CursorHome,
            KeyCode::End => Action::CursorEnd,
            KeyCode::Char(c) => Action::InputChar(c),
            _ => Action::None,
        };
    }

    // If spawn mode is active, handle spawn keys
    if state.spawn_mode.is_some() {
        return match code {
            KeyCode::Char('c') => {
                Action::SpawnAgent("claude --dangerously-skip-permissions".to_string())
            }
            KeyCode::Char('C') => Action::SpawnAgent("claude".to_string()),
            KeyCode::Char('x') => Action::SpawnAgent("codex".to_string()),
            KeyCode::Char('g') => Action::SpawnAgent("gemini".to_string()),
            KeyCode::Char('o') => Action::SpawnAgent("opencode".to_string()),
            KeyCode::Esc => Action::SpawnCancel,
            _ => Action::None,
        };
    }

    // If input panel is focused, handle input-specific keys
    if state.is_input_focused() {
        return match code {
            KeyCode::Esc => Action::FocusSidebar,
            KeyCode::Tab => Action::CycleFocus,
            KeyCode::BackTab => Action::FocusPreview,
            KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) => Action::InputNewline,
            KeyCode::Enter if modifiers.contains(KeyModifiers::ALT) => Action::InputNewline,
            KeyCode::Enter => Action::SendInput,
            KeyCode::Backspace => Action::InputBackspace,
            KeyCode::Left => Action::CursorLeft,
            KeyCode::Right => Action::CursorRight,
            KeyCode::Home => Action::CursorHome,
            KeyCode::End => Action::CursorEnd,
            KeyCode::Char(c) => Action::InputChar(c),
            _ => Action::None,
        };
    }

    // If preview panel is focused, handle preview-specific keys
    if state.is_preview_focused() {
        return match code {
            KeyCode::Esc => Action::FocusSidebar,
            KeyCode::Tab => Action::CycleFocus,
            KeyCode::BackTab => Action::FocusSidebar,
            KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
            KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('?') | KeyCode::Char('h') => Action::ShowHelp,
            _ => Action::None,
        };
    }

    // Sidebar focused
    match code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,

        KeyCode::Char('j') | KeyCode::Down => Action::NextAgent,
        KeyCode::Char('k') | KeyCode::Up => Action::PrevAgent,
        KeyCode::Tab => Action::CycleFocus,
        KeyCode::BackTab => Action::FocusInput,

        // Left/Right arrows for focus navigation
        KeyCode::Right => Action::FocusInput,
        KeyCode::Left => Action::None, // Already on sidebar

        // Enter toggles collapse when on a session header
        KeyCode::Enter => {
            if matches!(state.cursor, TreeCursor::Session(_)) {
                Action::ToggleCollapse
            } else {
                Action::None
            }
        }

        // Multi-selection
        KeyCode::Char(' ') => Action::ToggleSelection,
        KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => Action::SelectAll,

        // Approval - y/Y approve, N (shift only) reject
        KeyCode::Char('y') | KeyCode::Char('Y') => Action::Approve,
        KeyCode::Char('N') => Action::Reject,
        KeyCode::Char('a') | KeyCode::Char('A') => Action::ApproveAll,

        // Number keys for quick choice selection (1-9)
        KeyCode::Char(c @ '1'..='9') => {
            let num = c.to_digit(10).unwrap() as u8;
            Action::SendNumber(num)
        }

        // Focus pane with 'f'
        KeyCode::Char('f') | KeyCode::Char('F') => Action::FocusPane,

        // Kill pane with 'dd' (double-tap)
        KeyCode::Char('d') => {
            if let Some((ref target, instant)) = state.pending_kill {
                if instant.elapsed().as_millis() < 400 {
                    // Check that we're still on the same target
                    if let Some(current_target) = state.selected_pane_target() {
                        if current_target == *target {
                            return Action::KillPane;
                        }
                    }
                }
            }
            Action::KillPanePending
        }

        // Spawn agent with 'n'
        KeyCode::Char('n') => Action::SpawnStart,

        // Collapse/expand all
        KeyCode::Char('[') => Action::CollapseAll,
        KeyCode::Char(']') => Action::ExpandAll,

        KeyCode::Char('s') | KeyCode::Char('S') => Action::ToggleSubagentLog,
        KeyCode::Char('t') | KeyCode::Char('T') => Action::ToggleSummaryDetail,
        KeyCode::Char('r') => Action::Refresh,
        KeyCode::Char('R') => {
            // Rename when on a pane
            if state.selected_pane_target().is_some() {
                Action::RenameStart
            } else {
                Action::None
            }
        }

        // Sidebar resize (only < and >)
        KeyCode::Char('<') => Action::SidebarNarrower,
        KeyCode::Char('>') => Action::SidebarWider,

        KeyCode::Char('h') | KeyCode::Char('?') => Action::ShowHelp,

        KeyCode::Esc => {
            if !state.selected_agents.is_empty() {
                Action::ClearSelection
            } else if state.show_subagent_log {
                Action::ToggleSubagentLog
            } else {
                Action::None
            }
        }

        _ => Action::None,
    }
}
