use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
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

/// Footer widget showing clickable buttons
pub struct FooterWidget;

impl FooterWidget {
    /// Button layout: returns (label, start_col, end_col, button_type)
    pub fn get_button_layout(state: &AppState) -> Vec<(&'static str, u16, u16, FooterButton)> {
        let mut buttons = Vec::new();
        let mut col: u16 = 1;

        if state.is_input_focused() {
            // No buttons in input mode
            return buttons;
        }

        // Line 1 buttons
        let btn_approve = " ✓ Yes ";
        buttons.push((btn_approve, col, col + btn_approve.len() as u16, FooterButton::Approve));
        col += btn_approve.len() as u16 + 1;

        let btn_reject = " ✗ No ";
        buttons.push((btn_reject, col, col + btn_reject.len() as u16, FooterButton::Reject));
        col += btn_reject.len() as u16 + 1;

        let btn_all = " ⚡All ";
        buttons.push((btn_all, col, col + btn_all.len() as u16, FooterButton::ApproveAll));
        col += btn_all.len() as u16 + 1;

        let btn_select = " ☐ Sel ";
        buttons.push((btn_select, col, col + btn_select.len() as u16, FooterButton::ToggleSelect));
        col += btn_select.len() as u16 + 1;

        let btn_focus = " ◎ Focus ";
        buttons.push((btn_focus, col, col + btn_focus.len() as u16, FooterButton::Focus));
        col += btn_focus.len() as u16 + 1;

        let btn_help = " ? ";
        buttons.push((btn_help, col, col + btn_help.len() as u16, FooterButton::Help));
        col += btn_help.len() as u16 + 1;

        let btn_quit = " Q ";
        buttons.push((btn_quit, col, col + btn_quit.len() as u16, FooterButton::Quit));

        buttons
    }

    /// Check if a click at (x, y) relative to footer area hits a button
    pub fn hit_test(x: u16, y: u16, area: Rect, state: &AppState) -> Option<FooterButton> {
        // Check if within footer bounds (accounting for border)
        if x < area.x + 1 || x >= area.x + area.width - 1 {
            return None;
        }
        if y != area.y + 1 {
            // Only first line has buttons
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
        let sep_style = Style::default().fg(Color::DarkGray);
        let input_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
        let btn_style = Style::default().fg(Color::Black).bg(Color::Gray);
        let btn_approve = Style::default().fg(Color::Black).bg(Color::Green);
        let btn_reject = Style::default().fg(Color::Black).bg(Color::Red);
        let btn_all = Style::default().fg(Color::Black).bg(Color::Yellow);
        let btn_focus = Style::default().fg(Color::Black).bg(Color::Cyan);
        let key_style = Style::default().fg(Color::Yellow);
        let text_style = Style::default().fg(Color::White);

        let lines: Vec<Line> = if state.is_input_focused() {
            vec![
                Line::from(vec![
                    Span::styled(" INPUT ", input_style),
                    Span::styled("│", sep_style),
                    Span::styled(" [Enter]", key_style),
                    Span::styled(" Send ", text_style),
                    Span::styled("[S-Enter]", key_style),
                    Span::styled(" Newline ", text_style),
                    Span::styled("[←→]", key_style),
                    Span::styled(" Move cursor ", text_style),
                ]),
                Line::from(vec![
                    Span::styled(" [Esc]", key_style),
                    Span::styled(" Back to sidebar ", text_style),
                    Span::styled("[Home/End]", key_style),
                    Span::styled(" Jump ", text_style),
                ]),
            ]
        } else {
            // Clickable buttons
            let mut line1 = vec![
                Span::styled(" ✓ Yes ", btn_approve),
                Span::raw(" "),
                Span::styled(" ✗ No ", btn_reject),
                Span::raw(" "),
                Span::styled(" ⚡All ", btn_all),
                Span::raw(" "),
                Span::styled(" ☐ Sel ", btn_style),
                Span::raw(" "),
                Span::styled(" ◎ Focus ", btn_focus),
                Span::raw(" "),
                Span::styled(" ? ", btn_style),
                Span::raw(" "),
                Span::styled(" Q ", btn_style),
            ];

            if !state.selected_agents.is_empty() {
                line1.push(Span::styled(
                    format!(" ({})", state.selected_agents.len()),
                    Style::default().fg(Color::Cyan),
                ));
            }

            let mut line2 = vec![
                Span::styled(" Mouse: click buttons above │ scroll to navigate │ click agent to select ", text_style),
            ];

            if let Some(error) = &state.last_error {
                line2.push(Span::styled("│", sep_style));
                line2.push(Span::styled(
                    format!(" ✗ {} ", truncate_error(error, 25)),
                    Style::default().fg(Color::Red),
                ));
            }

            vec![Line::from(line1), Line::from(line2)]
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray));

        let paragraph = Paragraph::new(lines).block(block);
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
