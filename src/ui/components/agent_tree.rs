use std::collections::BTreeMap;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

use crate::agents::{AgentStatus, AgentType, ApprovalType, MonitoredAgent, SubagentStatus};
use crate::app::{AppState, NonAgentPane, TreeCursor};

/// Widget for displaying agents in a tree organized by session/window
pub struct AgentTreeWidget;

/// An item in a window: either an agent or a non-agent pane
enum WindowItem<'a> {
    Agent(usize, &'a MonitoredAgent),
    NonAgent(usize, &'a NonAgentPane),
}

/// Type alias for window key (window number, window name)
type WindowKey<'a> = (u32, &'a str);

/// Type alias for windows map
type WindowsMap<'a> = BTreeMap<WindowKey<'a>, Vec<WindowItem<'a>>>;

/// Type alias for sessions map
type SessionsMap<'a> = BTreeMap<&'a str, WindowsMap<'a>>;

/// Represents the hierarchical structure: Session -> Window -> Items
struct SessionWindowTree<'a> {
    sessions: SessionsMap<'a>,
}

impl<'a> SessionWindowTree<'a> {
    fn new(
        agents: &'a [MonitoredAgent],
        non_agent_panes: &'a [NonAgentPane],
        all_sessions: &'a [String],
    ) -> Self {
        let mut sessions: SessionsMap<'a> = BTreeMap::new();

        for (idx, agent) in agents.iter().enumerate() {
            sessions
                .entry(&agent.session)
                .or_default()
                .entry((agent.window, &agent.window_name))
                .or_default()
                .push(WindowItem::Agent(idx, agent));
        }

        for (idx, nap) in non_agent_panes.iter().enumerate() {
            sessions
                .entry(&nap.session)
                .or_default()
                .entry((nap.window, &nap.window_name))
                .or_default()
                .push(WindowItem::NonAgent(idx, nap));
        }

        // Ensure all sessions appear even if empty
        for s in all_sessions {
            sessions.entry(s.as_str()).or_default();
        }

        Self { sessions }
    }
}

impl AgentTreeWidget {
    pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
        let agents = &state.agents.root_agents;
        let non_agent_panes = &state.agents.non_agent_panes;
        let active_count = state.agents.active_count();
        let subagent_count = state.agents.running_subagent_count();
        let selected_count = state.selected_agents.len();

        // Build title
        let title = if selected_count > 0 {
            format!(" {} sel | {} pending ", selected_count, active_count)
        } else if subagent_count > 0 {
            format!(" {} pending | {} subs ", active_count, subagent_count)
        } else if active_count > 0 {
            format!(" {} pending ", active_count)
        } else {
            format!(" {} agents ", agents.len())
        };

        let border_color = if !state.is_input_focused() {
            Color::Cyan
        } else {
            Color::Gray
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let tree = SessionWindowTree::new(agents, non_agent_panes, &state.all_sessions);

        if tree.sessions.is_empty() {
            let empty_text = List::new(vec![ListItem::new(Line::from(vec![Span::styled(
                "  No sessions detected",
                Style::default().fg(Color::DarkGray),
            )]))])
            .block(block);
            frame.render_widget(empty_text, area);
            return;
        }

        let mut items: Vec<ListItem> = Vec::new();
        let available_width = area.width.saturating_sub(4) as usize;

        // Track which list item index corresponds to the cursor for ListState
        let mut cursor_list_index: Option<usize> = None;

        for (session, windows) in tree.sessions.iter() {
            let is_session_cursor = state.cursor == TreeCursor::Session(session.to_string());
            let is_collapsed = state.collapsed_sessions.contains(*session);

            let session_style = if is_session_cursor {
                Style::default().bg(Color::Rgb(50, 50, 70))
            } else {
                Style::default()
            };

            // Count agents and total panes for this session
            let mut agent_count = 0u32;
            let mut total_pane_count = 0u32;
            for ww in windows.values() {
                for item in ww {
                    total_pane_count += 1;
                    if matches!(item, WindowItem::Agent(..)) {
                        agent_count += 1;
                    }
                }
            }

            if is_collapsed {
                // Collapsed view: single line with status counts
                let mut awaiting = 0u32;
                let mut processing = 0u32;
                let mut idle = 0u32;
                let mut error = 0u32;
                let mut non_agent_count = 0u32;
                for ww in windows.values() {
                    for item in ww {
                        match item {
                            WindowItem::Agent(_, agent) => match &agent.status {
                                AgentStatus::AwaitingApproval { .. } => awaiting += 1,
                                AgentStatus::Processing { .. } => processing += 1,
                                AgentStatus::Idle => idle += 1,
                                AgentStatus::Error { .. } => error += 1,
                                AgentStatus::Unknown => idle += 1,
                            },
                            WindowItem::NonAgent(..) => non_agent_count += 1,
                        }
                    }
                }

                let mut counts = Vec::new();
                if awaiting > 0 {
                    counts.push(Span::styled(
                        format!(" \u{26a0}{}", awaiting),
                        Style::default().fg(Color::Red),
                    ));
                }
                if processing > 0 {
                    counts.push(Span::styled(
                        format!(" \u{2699}{}", processing),
                        Style::default().fg(Color::Yellow),
                    ));
                }
                if idle > 0 {
                    counts.push(Span::styled(
                        format!(" \u{25cf}{}", idle),
                        Style::default().fg(Color::Green),
                    ));
                }
                if error > 0 {
                    counts.push(Span::styled(
                        format!(" \u{2717}{}", error),
                        Style::default().fg(Color::Red),
                    ));
                }
                if non_agent_count > 0 {
                    counts.push(Span::styled(
                        format!(" \u{25cb}{}", non_agent_count),
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                let mut spans = vec![
                    Span::styled("\u{25b6} ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        *session,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                if agent_count == 0 && total_pane_count == 0 {
                    spans.push(Span::styled(
                        " (no agents)",
                        Style::default().fg(Color::DarkGray),
                    ));
                } else {
                    spans.extend(counts);
                }

                if is_session_cursor {
                    cursor_list_index = Some(items.len());
                }
                items.push(ListItem::new(Line::from(spans)).style(session_style));
            } else {
                // Expanded view
                let mut session_spans = vec![
                    Span::styled("\u{25bc} ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        *session,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                if agent_count == 0 && total_pane_count == 0 {
                    session_spans.push(Span::styled(
                        " (no agents)",
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                let session_line = Line::from(session_spans);
                if is_session_cursor {
                    cursor_list_index = Some(items.len());
                }
                items.push(ListItem::new(session_line).style(session_style));

                for (window_idx, ((window_num, window_name), window_items)) in
                    windows.iter().enumerate()
                {
                    let is_last_window = window_idx == windows.len() - 1;
                    let window_prefix = if is_last_window {
                        "\u{2514}\u{2500}"
                    } else {
                        "\u{251c}\u{2500}"
                    };

                    // Window header
                    let window_line = Line::from(vec![
                        Span::styled(
                            format!(" {} ", window_prefix),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("{}: {}", window_num, window_name),
                            Style::default().fg(Color::White),
                        ),
                    ]);
                    items.push(ListItem::new(window_line));

                    for (item_idx, window_item) in window_items.iter().enumerate() {
                        let is_last_item = item_idx == window_items.len() - 1;

                        match window_item {
                            WindowItem::Agent(original_idx, agent) => {
                                let is_cursor = state.cursor == TreeCursor::Agent(*original_idx);
                                let is_selected = state.is_multi_selected(*original_idx);

                                let cont_prefix = if is_last_window {
                                    "    "
                                } else {
                                    " \u{2502}  "
                                };

                                let tree_prefix = if is_last_window {
                                    if is_last_item && agent.subagents.is_empty() {
                                        "    \u{2514}\u{2500}"
                                    } else {
                                        "    \u{251c}\u{2500}"
                                    }
                                } else if is_last_item && agent.subagents.is_empty() {
                                    " \u{2502}  \u{2514}\u{2500}"
                                } else {
                                    " \u{2502}  \u{251c}\u{2500}"
                                };

                                let select_indicator = if is_selected && is_cursor {
                                    "\u{2503}\u{2611}"
                                } else if is_selected {
                                    " \u{2611}"
                                } else if is_cursor {
                                    "\u{2503} "
                                } else {
                                    "  "
                                };

                                // Status indicator and text
                                let (status_char, status_text, status_style) = match &agent.status {
                                    AgentStatus::Idle => {
                                        ("\u{25cf}", "Idle", Style::default().fg(Color::Green))
                                    }
                                    AgentStatus::Processing { .. } => (
                                        state.spinner_frame(),
                                        "Working",
                                        Style::default().fg(Color::Yellow),
                                    ),
                                    AgentStatus::AwaitingApproval { .. } => (
                                        "\u{26a0}",
                                        "Waiting",
                                        Style::default()
                                            .fg(Color::Red)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    AgentStatus::Error { .. } => {
                                        ("\u{2717}", "Error", Style::default().fg(Color::Red))
                                    }
                                    AgentStatus::Unknown => (
                                        "\u{25cb}",
                                        "Unknown",
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                };

                                let type_style = match agent.agent_type {
                                    AgentType::ClaudeCode => Style::default().fg(Color::Magenta),
                                    AgentType::OpenCode => Style::default().fg(Color::Blue),
                                    AgentType::CodexCli => Style::default().fg(Color::Green),
                                    AgentType::GeminiCli => Style::default().fg(Color::Yellow),
                                    AgentType::Unknown => Style::default().fg(Color::DarkGray),
                                };

                                let item_style = if is_cursor {
                                    Style::default().bg(Color::Rgb(50, 50, 70))
                                } else if is_selected {
                                    Style::default().bg(Color::Rgb(35, 35, 50))
                                } else {
                                    Style::default()
                                };

                                if is_cursor {
                                    cursor_list_index = Some(items.len());
                                }

                                // Main line: status + path
                                let line = Line::from(vec![
                                    Span::styled(
                                        select_indicator,
                                        if is_selected {
                                            Style::default().fg(Color::Cyan)
                                        } else {
                                            Style::default().fg(Color::White)
                                        },
                                    ),
                                    Span::styled(tree_prefix, Style::default().fg(Color::DarkGray)),
                                    Span::styled(status_char, status_style),
                                    Span::raw(" "),
                                    Span::styled(
                                        agent.abbreviated_path(),
                                        Style::default().fg(Color::Cyan),
                                    ),
                                ]);
                                items.push(ListItem::new(line).style(item_style));

                                // Info line: type | status | pid | uptime | context
                                let mut info_parts = vec![
                                    Span::raw("  "),
                                    Span::styled(
                                        format!("{}\u{2502}  ", cont_prefix),
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                    Span::styled(agent.agent_type.short_name(), type_style),
                                    Span::styled(
                                        " \u{2502} ",
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                    Span::styled(status_text, status_style),
                                    Span::styled(
                                        " \u{2502} ",
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                    Span::styled(
                                        format!("pid:{}", agent.pid),
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                    Span::styled(
                                        " \u{2502} ",
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                    Span::styled(
                                        agent.uptime_str(),
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                ];

                                // Context bar if available
                                if let Some(ctx) = agent.context_remaining {
                                    let bar_color = if ctx > 50 {
                                        Color::Green
                                    } else if ctx > 20 {
                                        Color::Yellow
                                    } else {
                                        Color::Red
                                    };
                                    info_parts.push(Span::styled(
                                        " \u{2502} ",
                                        Style::default().fg(Color::DarkGray),
                                    ));
                                    info_parts.push(Span::styled(
                                        context_bar(ctx),
                                        Style::default().fg(bar_color),
                                    ));
                                }

                                items.push(ListItem::new(Line::from(info_parts)).style(item_style));

                                // Status details
                                match &agent.status {
                                    AgentStatus::AwaitingApproval {
                                        approval_type,
                                        details,
                                    } => {
                                        let approval_line = Line::from(vec![
                                            Span::raw("  "),
                                            Span::styled(
                                                format!("{}\u{2502}  ", cont_prefix),
                                                Style::default().fg(Color::DarkGray),
                                            ),
                                            Span::styled(
                                                "\u{26a0} ",
                                                Style::default().fg(Color::Red),
                                            ),
                                            Span::styled(
                                                format!("{}", approval_type),
                                                Style::default()
                                                    .fg(Color::Red)
                                                    .add_modifier(Modifier::BOLD),
                                            ),
                                        ]);
                                        items.push(ListItem::new(approval_line).style(item_style));

                                        if !details.is_empty() {
                                            let detail_text = truncate_str(
                                                details,
                                                available_width.saturating_sub(14),
                                            );
                                            let detail_line = Line::from(vec![
                                                Span::raw("  "),
                                                Span::styled(
                                                    format!("{}\u{2502}  ", cont_prefix),
                                                    Style::default().fg(Color::DarkGray),
                                                ),
                                                Span::styled(
                                                    "  \u{2192} ",
                                                    Style::default().fg(Color::DarkGray),
                                                ),
                                                Span::styled(
                                                    detail_text,
                                                    Style::default().fg(Color::White),
                                                ),
                                            ]);
                                            items
                                                .push(ListItem::new(detail_line).style(item_style));
                                        }

                                        if let ApprovalType::UserQuestion { choices, .. } =
                                            approval_type
                                        {
                                            for (i, choice) in choices.iter().take(4).enumerate() {
                                                let choice_text = truncate_str(
                                                    choice,
                                                    available_width.saturating_sub(14),
                                                );
                                                let choice_line = Line::from(vec![
                                                    Span::raw("  "),
                                                    Span::styled(
                                                        format!("{}\u{2502}  ", cont_prefix),
                                                        Style::default().fg(Color::DarkGray),
                                                    ),
                                                    Span::styled(
                                                        format!("  {}. ", i + 1),
                                                        Style::default().fg(Color::Yellow),
                                                    ),
                                                    Span::styled(
                                                        choice_text,
                                                        Style::default().fg(Color::White),
                                                    ),
                                                ]);
                                                items.push(
                                                    ListItem::new(choice_line).style(item_style),
                                                );
                                            }
                                            if choices.len() > 4 {
                                                let more_line = Line::from(vec![
                                                    Span::raw("  "),
                                                    Span::styled(
                                                        format!("{}\u{2502}  ", cont_prefix),
                                                        Style::default().fg(Color::DarkGray),
                                                    ),
                                                    Span::styled(
                                                        format!(
                                                            "     ...+{} more",
                                                            choices.len() - 4
                                                        ),
                                                        Style::default().fg(Color::DarkGray),
                                                    ),
                                                ]);
                                                items.push(
                                                    ListItem::new(more_line).style(item_style),
                                                );
                                            }
                                        }
                                    }
                                    AgentStatus::Processing { activity } => {
                                        if !activity.is_empty() {
                                            let activity_text = truncate_str(
                                                activity,
                                                available_width.saturating_sub(14),
                                            );
                                            let activity_line = Line::from(vec![
                                                Span::raw("  "),
                                                Span::styled(
                                                    format!("{}\u{2502}  ", cont_prefix),
                                                    Style::default().fg(Color::DarkGray),
                                                ),
                                                Span::styled(
                                                    format!("{} ", state.spinner_frame()),
                                                    Style::default().fg(Color::Yellow),
                                                ),
                                                Span::styled(
                                                    activity_text,
                                                    Style::default().fg(Color::Yellow),
                                                ),
                                            ]);
                                            items.push(
                                                ListItem::new(activity_line).style(item_style),
                                            );
                                        }
                                    }
                                    AgentStatus::Error { message } => {
                                        let error_text = truncate_str(
                                            message,
                                            available_width.saturating_sub(14),
                                        );
                                        let error_line = Line::from(vec![
                                            Span::raw("  "),
                                            Span::styled(
                                                format!("{}\u{2502}  ", cont_prefix),
                                                Style::default().fg(Color::DarkGray),
                                            ),
                                            Span::styled(
                                                "\u{2717} ",
                                                Style::default().fg(Color::Red),
                                            ),
                                            Span::styled(
                                                error_text,
                                                Style::default().fg(Color::Red),
                                            ),
                                        ]);
                                        items.push(ListItem::new(error_line).style(item_style));
                                    }
                                    _ => {}
                                }

                                // Subagents
                                for (sub_idx, subagent) in agent.subagents.iter().enumerate() {
                                    let is_last_sub = sub_idx == agent.subagents.len() - 1;
                                    let sub_branch = if is_last_sub {
                                        "\u{2514}\u{2500}"
                                    } else {
                                        "\u{251c}\u{2500}"
                                    };

                                    let (sub_char, sub_style) = match subagent.status {
                                        SubagentStatus::Running => (
                                            state.spinner_frame(),
                                            Style::default().fg(Color::Cyan),
                                        ),
                                        SubagentStatus::Completed => {
                                            ("\u{2713}", Style::default().fg(Color::Green))
                                        }
                                        SubagentStatus::Failed => {
                                            ("\u{2717}", Style::default().fg(Color::Red))
                                        }
                                        SubagentStatus::Unknown => {
                                            ("?", Style::default().fg(Color::DarkGray))
                                        }
                                    };

                                    let duration =
                                        if matches!(subagent.status, SubagentStatus::Running) {
                                            format!(" ({})", subagent.duration_str())
                                        } else {
                                            String::new()
                                        };

                                    let sub_line = Line::from(vec![
                                        Span::raw("  "),
                                        Span::styled(
                                            format!("{}{}", cont_prefix, sub_branch),
                                            Style::default().fg(Color::DarkGray),
                                        ),
                                        Span::styled(sub_char, sub_style),
                                        Span::raw(" "),
                                        Span::styled(
                                            subagent.subagent_type.display_name(),
                                            Style::default()
                                                .fg(Color::White)
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::styled(duration, Style::default().fg(Color::Yellow)),
                                    ]);
                                    items.push(ListItem::new(sub_line));

                                    if !subagent.description.is_empty() {
                                        let desc_prefix =
                                            if is_last_sub { "   " } else { "\u{2502}  " };
                                        let desc_text = truncate_str(
                                            &subagent.description,
                                            available_width.saturating_sub(14),
                                        );
                                        let desc_line = Line::from(vec![
                                            Span::raw("  "),
                                            Span::styled(
                                                format!("{}{}", cont_prefix, desc_prefix),
                                                Style::default().fg(Color::DarkGray),
                                            ),
                                            Span::styled("  ", Style::default()),
                                            Span::styled(
                                                desc_text,
                                                Style::default().fg(Color::DarkGray),
                                            ),
                                        ]);
                                        items.push(ListItem::new(desc_line));
                                    }
                                }
                            }
                            WindowItem::NonAgent(nap_idx, nap) => {
                                let is_cursor = state.cursor == TreeCursor::NonAgentPane(*nap_idx);

                                let tree_prefix = if is_last_window {
                                    if is_last_item {
                                        "    \u{2514}\u{2500}"
                                    } else {
                                        "    \u{251c}\u{2500}"
                                    }
                                } else if is_last_item {
                                    " \u{2502}  \u{2514}\u{2500}"
                                } else {
                                    " \u{2502}  \u{251c}\u{2500}"
                                };

                                let cursor_indicator = if is_cursor { "\u{2503} " } else { "  " };

                                let item_style = if is_cursor {
                                    Style::default().bg(Color::Rgb(50, 50, 70))
                                } else {
                                    Style::default()
                                };

                                if is_cursor {
                                    cursor_list_index = Some(items.len());
                                }

                                let line = Line::from(vec![
                                    Span::styled(
                                        cursor_indicator,
                                        Style::default().fg(Color::White),
                                    ),
                                    Span::styled(tree_prefix, Style::default().fg(Color::DarkGray)),
                                    Span::styled("\u{25cb} ", Style::default().fg(Color::DarkGray)),
                                    Span::styled(
                                        &nap.command,
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                ]);
                                items.push(ListItem::new(line).style(item_style));
                            }
                        }
                    }
                }
            }
        }

        let list = List::new(items).block(block);
        let mut list_state = ListState::default();
        list_state.select(cursor_list_index);
        frame.render_stateful_widget(list, area, &mut list_state);
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}..",
            s.chars()
                .take(max_len.saturating_sub(2))
                .collect::<String>()
        )
    }
}

fn context_bar(percent: u8) -> String {
    let total_blocks = 10;
    let filled = (percent as usize * total_blocks) / 100;
    let empty = total_blocks - filled;
    format!(
        "{}{}\u{2502}{:>3}%",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
        percent
    )
}
