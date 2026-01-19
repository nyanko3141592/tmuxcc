use crate::app::AppState;
use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub struct HeaderWidget;

impl HeaderWidget {
    pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
        let total = state.agents.root_agents.len();
        let processing = state.agents.processing_count();
        let pending = state.agents.active_count();
        let time = Local::now().format("%H:%M").to_string();

        let mut spans = vec![
            Span::styled(
                " TmuxCC ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" {} agents ", total),
                Style::default().fg(Color::White),
            ),
        ];

        // Processing count
        if processing > 0 {
            spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!(" {} {} working ", state.spinner_frame(), processing),
                Style::default().fg(Color::Yellow),
            ));
        }

        // Pending count
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        if pending > 0 {
            spans.push(Span::styled(
                format!(" ⚠ {} pending ", pending),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(" ✓ ready ", Style::default().fg(Color::Green)));
        }

        // System stats: CPU
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        let cpu_color = if state.system_stats.cpu_usage > 80.0 {
            Color::Red
        } else if state.system_stats.cpu_usage > 50.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        spans.push(Span::styled(
            format!(" CPU {:4.1}% ", state.system_stats.cpu_usage),
            Style::default().fg(cpu_color),
        ));

        // System stats: Memory
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        let mem_percent = state.system_stats.memory_percent();
        let mem_color = if mem_percent > 80.0 {
            Color::Red
        } else if mem_percent > 60.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        spans.push(Span::styled(
            format!(
                " MEM {} ({:.0}%) ",
                state.system_stats.memory_display(),
                mem_percent
            ),
            Style::default().fg(mem_color),
        ));

        // Time
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!(" {} ", time),
            Style::default().fg(Color::DarkGray),
        ));

        let line = Line::from(spans);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray));

        let paragraph = Paragraph::new(line).block(block);
        frame.render_widget(paragraph, area);
    }
}
