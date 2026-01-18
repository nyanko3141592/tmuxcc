use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::AppState;

/// Button definitions for footer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterButton {
    Approve,
    Reject,
    ApproveAll,
    ToggleSelect,
    Focus,
    Help,
    Quit,
}

/// Footer widget showing clickable buttons (single line, no border)
pub struct FooterWidget;

impl FooterWidget {
    /// Button layout: returns (label, start_col, end_col, button_type)
    pub fn get_button_layout(state: &AppState) -> Vec<(&'static str, u16, u16, FooterButton)> {
        let mut buttons = Vec::new();
        let mut col: u16 = 0;

        if state.is_input_focused() {
            return buttons;
        }

        let items = [
            (" Y ", FooterButton::Approve),
            (" N ", FooterButton::Reject),
            (" A ", FooterButton::ApproveAll),
            (" ☐ ", FooterButton::ToggleSelect),
            (" F ", FooterButton::Focus),
            (" ? ", FooterButton::Help),
            (" Q ", FooterButton::Quit),
        ];

        for (label, btn_type) in items {
            buttons.push((label, col, col + label.len() as u16, btn_type));
            col += label.len() as u16 + 1;
        }

        buttons
    }

    /// Check if a click at (x, y) hits a button
    pub fn hit_test(x: u16, y: u16, area: Rect, state: &AppState) -> Option<FooterButton> {
        if y != area.y {
            return None;
        }
        if x < area.x || x >= area.x + area.width {
            return None;
        }

        let rel_x = x - area.x;
        let buttons = Self::get_button_layout(state);

        for (_, start, end, button) in buttons {
            if rel_x >= start && rel_x < end {
                return Some(button);
            }
        }

        None
    }

    pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
        let btn_y = Style::default().fg(Color::Black).bg(Color::Green);
        let btn_n = Style::default().fg(Color::Black).bg(Color::Red);
        let btn_a = Style::default().fg(Color::Black).bg(Color::Yellow);
        let btn_sel = Style::default().fg(Color::Black).bg(Color::Cyan);
        let btn_def = Style::default().fg(Color::Black).bg(Color::Gray);
        let sep = Style::default().fg(Color::DarkGray);
        let key = Style::default().fg(Color::Yellow);
        let txt = Style::default().fg(Color::White);

        let line: Line = if state.is_input_focused() {
            Line::from(vec![
                Span::styled(
                    " INPUT ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("│", sep),
                Span::styled(" Enter", key),
                Span::styled(":Send ", txt),
                Span::styled("S-Enter", key),
                Span::styled(":NL ", txt),
                Span::styled("Esc", key),
                Span::styled(":Back ", txt),
                Span::styled("←→", key),
                Span::styled(":Move", txt),
            ])
        } else {
            let mut spans = vec![
                Span::styled(" Y ", btn_y),
                Span::styled(" ", sep),
                Span::styled(" N ", btn_n),
                Span::styled(" ", sep),
                Span::styled(" A ", btn_a),
                Span::styled(" ", sep),
                Span::styled(" ☐ ", btn_sel),
                Span::styled(" ", sep),
                Span::styled(" F ", btn_def),
                Span::styled(" ", sep),
                Span::styled(" ? ", btn_def),
                Span::styled(" ", sep),
                Span::styled(" Q ", btn_def),
            ];

            if !state.selected_agents.is_empty() {
                spans.push(Span::styled(
                    format!(" ({}sel)", state.selected_agents.len()),
                    Style::default().fg(Color::Cyan),
                ));
            }

            if let Some(error) = &state.last_error {
                spans.push(Span::styled(" │ ", sep));
                spans.push(Span::styled(
                    format!("✗ {}", truncate_error(error, 30)),
                    Style::default().fg(Color::Red),
                ));
            }

            Line::from(spans)
        };

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

fn truncate_error(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max_len - 1).collect::<String>())
    }
}
