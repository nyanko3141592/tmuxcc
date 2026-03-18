use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{AppState, FlashTarget, generate_flash_labels};

/// Input widget for text entry at the bottom of the right column
pub struct InputWidget;

impl InputWidget {
    pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
        let buffer = state.get_input();
        let cursor_pos = state.get_cursor_position();
        let is_focused = state.is_input_focused();

        // Get target agent name
        let target_name = state
            .selected_agent()
            .map(|a| a.abbreviated_path())
            .unwrap_or_else(|| "None".to_string());

        // Check for flash label on input area
        let flash_label = if state.flash_mode.is_some() {
            let targets = state.build_flash_targets();
            let labels = generate_flash_labels(targets.len());
            targets
                .iter()
                .zip(labels.iter())
                .find(|(t, _)| matches!(t, FlashTarget::InputArea))
                .map(|(_, l)| l.clone())
        } else {
            None
        };

        let title = if let Some(ref label) = flash_label {
            format!(" [{}] Input → {} ", label, target_name)
        } else {
            format!(" Input → {} ", target_name)
        };

        let border_color = if is_focused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let block = if let Some(ref label) = flash_label {
            Block::default()
                .title(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        label.as_str(),
                        Style::default()
                            .fg(Color::Magenta)
                            .bg(Color::Rgb(50, 0, 50))
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                    Span::raw(format!(" Input → {} ", target_name)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
        } else {
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
        };

        // Build content with cursor (only show cursor when focused)
        let lines: Vec<Line> = Self::build_lines_with_cursor(buffer, cursor_pos, is_focused);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);

        // Set cursor position for IME support (only when focused)
        if is_focused {
            Self::set_cursor_position(frame, area, buffer, cursor_pos);
        }
    }

    /// Build lines with cursor indicator
    fn build_lines_with_cursor(
        buffer: &str,
        cursor_pos: usize,
        is_focused: bool,
    ) -> Vec<Line<'static>> {
        let cursor_style = Style::default().fg(Color::Black).bg(Color::Green);
        let text_style = Style::default().fg(Color::White);
        let hint_style = Style::default().fg(Color::DarkGray);

        if buffer.is_empty() {
            if is_focused {
                return vec![Line::from(vec![
                    Span::styled("█", cursor_style),
                    Span::styled(
                        " (Shift+Enter: newline, Enter: send, Esc: back)",
                        hint_style,
                    ),
                ])];
            } else {
                return vec![Line::from(vec![Span::styled(
                    "← arrow key to input",
                    hint_style,
                )])];
            }
        }

        // Split text before and after cursor
        let before_cursor = &buffer[..cursor_pos];
        let after_cursor = &buffer[cursor_pos..];

        // Get the character at cursor (or empty if at end)
        let cursor_char = after_cursor.chars().next();
        let after_cursor_rest = if let Some(c) = cursor_char {
            &after_cursor[c.len_utf8()..]
        } else {
            ""
        };

        // Build lines with cursor in the correct position
        let mut lines = Vec::new();
        let before_lines: Vec<&str> = before_cursor.split('\n').collect();
        let after_lines: Vec<&str> = after_cursor_rest.split('\n').collect();

        // Process all lines before cursor line
        for line_text in &before_lines[..before_lines.len().saturating_sub(1)] {
            lines.push(Line::from(vec![Span::styled(
                line_text.to_string(),
                text_style,
            )]));
        }

        // Build the cursor line
        let cursor_line_before = before_lines.last().unwrap_or(&"");
        let cursor_line_after_first = after_lines.first().unwrap_or(&"");

        if is_focused {
            let cursor_display = if let Some(c) = cursor_char {
                if c == '\n' {
                    // Cursor is at newline, show block cursor
                    "█".to_string()
                } else {
                    c.to_string()
                }
            } else {
                // At end of buffer
                "█".to_string()
            };

            lines.push(Line::from(vec![
                Span::styled(cursor_line_before.to_string(), text_style),
                Span::styled(cursor_display, cursor_style),
                Span::styled(cursor_line_after_first.to_string(), text_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(cursor_line_before.to_string(), text_style),
                Span::styled(
                    format!(
                        "{}{}",
                        cursor_char.map(|c| c.to_string()).unwrap_or_default(),
                        cursor_line_after_first
                    ),
                    text_style,
                ),
            ]));
        }

        // Process remaining lines after cursor line
        for line_text in &after_lines[1..] {
            lines.push(Line::from(vec![Span::styled(
                line_text.to_string(),
                text_style,
            )]));
        }

        lines
    }

    /// Set cursor position for IME (Input Method Editor) support
    fn set_cursor_position(frame: &mut Frame, area: Rect, buffer: &str, cursor_pos: usize) {
        // Get text before cursor
        let before_cursor = &buffer[..cursor_pos];

        // Calculate line number and column using display width
        let lines: Vec<&str> = before_cursor.split('\n').collect();
        let line_count = lines.len();
        let last_line = lines.last().unwrap_or(&"");
        // Use unicode width for proper full-width character handling
        let column_width = last_line.width() as u16;

        let cursor_y = area.y + 1 + (line_count.saturating_sub(1)) as u16;
        let cursor_x = area.x + 1 + column_width;

        // Ensure cursor is within bounds
        let cursor_x = cursor_x.min(area.x + area.width.saturating_sub(2));
        let cursor_y = cursor_y.min(area.y + area.height.saturating_sub(2));

        frame.set_cursor_position((cursor_x, cursor_y));
    }

    /// Calculate required height based on buffer content
    pub fn calculate_height(buffer: &str, max_height: u16) -> u16 {
        let line_count = buffer.split('\n').count() as u16;
        // Minimum 1 line, maximum max_height lines
        line_count.max(1).min(max_height)
    }
}
