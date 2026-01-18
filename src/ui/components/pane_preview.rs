use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::agents::AgentStatus;
use crate::app::AppState;

/// Parsed summary info from Claude Code content
struct ClaudeCodeSummary {
    /// Current status/activity line (✽ ...)
    current_activity: Option<String>,
    /// TODO items: (is_completed, text)
    todos: Vec<(bool, String)>,
    /// Recent tool executions (⏺ ...)
    recent_tools: Vec<String>,
}

impl ClaudeCodeSummary {
    fn parse(content: &str) -> Self {
        let mut current_activity = None;
        let mut todos = Vec::new();
        let mut recent_tools = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Current activity: ✽ text... or · text...
            if trimmed.starts_with('✽') || trimmed.starts_with('·') {
                let activity = trimmed
                    .trim_start_matches('✽')
                    .trim_start_matches('·')
                    .trim();
                // Extract just the main part (before parentheses with timing info)
                let main_part = activity.split('(').next().unwrap_or(activity).trim();
                if !main_part.is_empty() {
                    current_activity = Some(main_part.to_string());
                }
            }

            // TODOs: ☐ (pending) or ☑/✓ (completed)
            if trimmed.starts_with('☐') {
                let text = trimmed.trim_start_matches('☐').trim();
                todos.push((false, text.to_string()));
            } else if trimmed.starts_with('☑') || trimmed.starts_with('✓') {
                let text = trimmed
                    .trim_start_matches('☑')
                    .trim_start_matches('✓')
                    .trim();
                todos.push((true, text.to_string()));
            }

            // Tool executions: ⏺ Tool(...) or ⏺ text
            if trimmed.starts_with('⏺') {
                let tool_text = trimmed.trim_start_matches('⏺').trim();
                // Only keep recent non-completed ones, or limit to last few
                if !tool_text.contains("completed") && !tool_text.contains("finished")
                    && recent_tools.len() < 3 {
                        // Truncate long tool lines (character-based for UTF-8 safety)
                        let char_count = tool_text.chars().count();
                        let short = if char_count > 60 {
                            let truncated: String = tool_text.chars().take(57).collect();
                            format!("{}...", truncated)
                        } else {
                            tool_text.to_string()
                        };
                        recent_tools.push(short);
                    }
            }
        }

        Self {
            current_activity,
            todos,
            recent_tools,
        }
    }
}

/// Widget for previewing the selected pane content
pub struct PanePreviewWidget;

impl PanePreviewWidget {
    /// Render a summary view with TODO and current activity
    pub fn render_summary(frame: &mut Frame, area: Rect, state: &AppState) {
        let agent = state.selected_agent();

        let (title, lines) = if let Some(agent) = agent {
            let title = format!(" {} ", agent.agent_type.short_name());
            let summary = ClaudeCodeSummary::parse(&agent.last_content);

            let mut styled_lines: Vec<Line> = Vec::new();

            // Current activity
            if let Some(activity) = summary.current_activity {
                styled_lines.push(Line::from(vec![
                    Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                    Span::styled(activity, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]));
                styled_lines.push(Line::from(""));
            }

            // Running tools
            if !summary.recent_tools.is_empty() {
                for tool in summary.recent_tools {
                    styled_lines.push(Line::from(vec![
                        Span::styled("  ⏺ ", Style::default().fg(Color::Cyan)),
                        Span::styled(tool, Style::default().fg(Color::White)),
                    ]));
                }
                styled_lines.push(Line::from(""));
            }

            // TODOs
            if !summary.todos.is_empty() {
                styled_lines.push(Line::from(vec![
                    Span::styled("TODOs:", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                ]));
                for (completed, text) in summary.todos {
                    let (icon, style) = if completed {
                        ("☑ ", Style::default().fg(Color::DarkGray))
                    } else {
                        ("☐ ", Style::default().fg(Color::White))
                    };
                    styled_lines.push(Line::from(vec![
                        Span::styled(format!("  {}", icon), style),
                        Span::styled(text, style),
                    ]));
                }
            }

            // If no summary info, show status
            if styled_lines.is_empty() {
                let status_text = match &agent.status {
                    AgentStatus::Idle => "Ready for input",
                    AgentStatus::Processing { activity } => activity.as_str(),
                    AgentStatus::AwaitingApproval { approval_type, .. } => {
                        styled_lines.push(Line::from(vec![
                            Span::styled("⚠ ", Style::default().fg(Color::Red)),
                            Span::styled(
                                format!("Waiting: {}", approval_type),
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            ),
                        ]));
                        ""
                    }
                    AgentStatus::Error { message } => message.as_str(),
                    AgentStatus::Unknown => "...",
                };
                if !status_text.is_empty() && styled_lines.is_empty() {
                    styled_lines.push(Line::from(vec![
                        Span::styled(status_text, Style::default().fg(Color::Gray)),
                    ]));
                }
            }

            (title, styled_lines)
        } else {
            (
                " Summary ".to_string(),
                vec![Line::from(vec![Span::styled(
                    "No agent selected",
                    Style::default().fg(Color::DarkGray),
                )])],
            )
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, area);
    }

    pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
        let agent = state.selected_agent();

        let (title, content) = if let Some(agent) = agent {
            let title = format!(" Preview: {} ({}) ", agent.target, agent.agent_type);

            // Show approval details if awaiting
            let content = if let AgentStatus::AwaitingApproval {
                approval_type,
                details,
            } = &agent.status
            {
                format!(
                    "⚠ {} wants: {}\n\nDetails: {}\n\nPress [Y] to approve or [N] to reject",
                    agent.agent_type, approval_type, details
                )
            } else {
                // Show last portion of pane content
                let lines: Vec<&str> = agent.last_content.lines().collect();
                let start = lines.len().saturating_sub(20);
                lines[start..].join("\n")
            };

            (title, content)
        } else {
            (
                " Preview ".to_string(),
                "No agent selected".to_string(),
            )
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray));

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, area);
    }

    /// Renders a detailed preview with syntax highlighting for diffs
    pub fn render_detailed(frame: &mut Frame, area: Rect, state: &AppState) {
        let agent = state.selected_agent();

        // Calculate available lines (area height minus border)
        let available_lines = area.height.saturating_sub(2) as usize;

        let (title, lines) = if let Some(agent) = agent {
            let title = format!(" {} ({}) ", agent.target, agent.agent_type);

            let mut styled_lines: Vec<Line> = Vec::new();

            // Take enough lines to fill the area
            let content_lines: Vec<&str> = agent.last_content.lines().collect();
            let start = content_lines.len().saturating_sub(available_lines);

            for line in &content_lines[start..] {
                let spans = if line.starts_with('+') && !line.starts_with("+++") {
                    vec![Span::styled(*line, Style::default().fg(Color::Green))]
                } else if line.starts_with('-') && !line.starts_with("---") {
                    vec![Span::styled(*line, Style::default().fg(Color::Red))]
                } else if line.starts_with("@@") {
                    vec![Span::styled(*line, Style::default().fg(Color::Cyan))]
                } else if line.contains("[y/n]") || line.contains("[Y/n]") {
                    vec![Span::styled(
                        *line,
                        Style::default().fg(Color::Yellow),
                    )]
                } else if line.contains("⚠") || line.contains("Error") || line.contains("error") {
                    vec![Span::styled(*line, Style::default().fg(Color::Red))]
                } else if line.starts_with("❯") || line.starts_with(">") {
                    vec![Span::styled(*line, Style::default().fg(Color::Cyan))]
                } else {
                    vec![Span::raw(*line)]
                };

                styled_lines.push(Line::from(spans));
            }

            (title, styled_lines)
        } else {
            (
                " Preview ".to_string(),
                vec![Line::from(vec![Span::styled(
                    "No agent selected",
                    Style::default().fg(Color::DarkGray),
                )])],
            )
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}
