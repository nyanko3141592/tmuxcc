use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::Layout;

/// Help popup widget
pub struct HelpWidget;

impl HelpWidget {
    pub fn render(frame: &mut Frame, area: Rect, scroll: u16) {
        let popup_area = Layout::centered_popup(area, 70, 80);

        frame.render_widget(Clear, popup_area);

        let key_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::White);
        let section_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let dim_style = Style::default().fg(Color::DarkGray);
        let context_style = Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::ITALIC);

        let help_text = vec![
            // ── Global ──
            section("Global (work in any panel)", section_style),
            blank(),
            key(
                "  Tab          ",
                "Cycle focus: Sidebar -> Preview -> Input",
                key_style,
                desc_style,
            ),
            key(
                "  Shift+Tab    ",
                "Cycle focus backwards",
                key_style,
                desc_style,
            ),
            key("  ? / h        ", "Toggle this help", key_style, desc_style),
            key("  q            ", "Quit", key_style, desc_style),
            key("  Ctrl+c       ", "Quit", key_style, desc_style),
            blank(),
            // ── Sidebar ──
            section("Sidebar", section_style),
            context(
                "  Active when the left panel has a cyan border.",
                context_style,
            ),
            blank(),
            key(
                "  j / Down     ",
                "Next item (session, agent, or pane)",
                key_style,
                desc_style,
            ),
            key("  k / Up       ", "Previous item", key_style, desc_style),
            blank(),
            Line::from(vec![Span::styled("  Sessions", desc_style)]),
            key(
                "  Enter/Space  ",
                "Collapse/expand (when cursor on session header)",
                key_style,
                desc_style,
            ),
            key(
                "  [            ",
                "Collapse all sessions",
                key_style,
                desc_style,
            ),
            key(
                "  ]            ",
                "Expand all sessions",
                key_style,
                desc_style,
            ),
            blank(),
            Line::from(vec![Span::styled("  Agent actions", desc_style)]),
            key(
                "  y / Y        ",
                "Approve pending request(s)",
                key_style,
                desc_style,
            ),
            key(
                "  N            ",
                "Reject pending request(s)",
                key_style,
                desc_style,
            ),
            key(
                "  a / A        ",
                "Approve ALL pending requests",
                key_style,
                desc_style,
            ),
            key(
                "  1-9          ",
                "Send numbered choice to agent",
                key_style,
                desc_style,
            ),
            blank(),
            Line::from(vec![Span::styled("  Pane management", desc_style)]),
            key(
                "  f / F        ",
                "Focus pane in tmux (switches session too)",
                key_style,
                desc_style,
            ),
            key(
                "  dd           ",
                "Kill pane (double-tap d for safety)",
                key_style,
                desc_style,
            ),
            key(
                "  R            ",
                "Rename pane's tmux window",
                key_style,
                desc_style,
            ),
            blank(),
            Line::from(vec![Span::styled("  Spawn agent", desc_style)]),
            key(
                "  n            ",
                "Enter spawn mode for the current session",
                key_style,
                desc_style,
            ),
            Line::from(vec![
                Span::styled("                 ", dim_style),
                Span::styled(
                    "Then: c=Claude  C=Claude(safe)  x=Codex  g=Gemini  o=OpenCode  Esc=cancel",
                    dim_style,
                ),
            ]),
            blank(),
            Line::from(vec![Span::styled("  Flash navigation", desc_style)]),
            key(
                "  g            ",
                "Flash-focus: show labels, jump cursor to target",
                key_style,
                desc_style,
            ),
            key(
                "  G            ",
                "Flash-go: show labels, jump + attach to tmux target",
                key_style,
                desc_style,
            ),
            Line::from(vec![
                Span::styled("                 ", dim_style),
                Span::styled(
                    "Press the hint label key to jump. Esc to cancel.",
                    dim_style,
                ),
            ]),
            blank(),
            Line::from(vec![Span::styled("  Multi-select", desc_style)]),
            key(
                "  Space        ",
                "Toggle selection on current agent",
                key_style,
                desc_style,
            ),
            key(
                "  Ctrl+a       ",
                "Select all agents",
                key_style,
                desc_style,
            ),
            key("  Esc          ", "Clear selection", key_style, desc_style),
            blank(),
            Line::from(vec![Span::styled("  View toggles", desc_style)]),
            key(
                "  s / S        ",
                "Toggle subagent log panel",
                key_style,
                desc_style,
            ),
            key(
                "  t / T        ",
                "Toggle TODO/Tools summary panel",
                key_style,
                desc_style,
            ),
            key(
                "  <  /  >      ",
                "Resize sidebar narrower / wider",
                key_style,
                desc_style,
            ),
            key(
                "  r            ",
                "Refresh agent list",
                key_style,
                desc_style,
            ),
            blank(),
            // ── Preview ──
            section("Preview Panel", section_style),
            context(
                "  Active when the right panel has a cyan border. Tab to get here.",
                context_style,
            ),
            blank(),
            key("  j / Down     ", "Scroll down", key_style, desc_style),
            key("  k / Up       ", "Scroll up", key_style, desc_style),
            key("  Esc          ", "Back to sidebar", key_style, desc_style),
            blank(),
            // ── Input ──
            section("Input Panel", section_style),
            context(
                "  Active when the bottom input box has a cyan border. Tab twice to get here.",
                context_style,
            ),
            blank(),
            key(
                "  Enter        ",
                "Send typed text to selected agent",
                key_style,
                desc_style,
            ),
            key(
                "  Shift+Enter  ",
                "Insert newline (multi-line input)",
                key_style,
                desc_style,
            ),
            key("  Esc          ", "Back to sidebar", key_style, desc_style),
            blank(),
            blank(),
            Line::from(vec![Span::styled(
                "  j/k to scroll this help  ·  q or Esc to close",
                dim_style,
            )]),
        ];

        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        frame.render_widget(paragraph, popup_area);
    }
}

fn section(title: &str, style: Style) -> Line<'_> {
    Line::from(vec![Span::styled(title, style)])
}

fn context(text: &str, style: Style) -> Line<'_> {
    Line::from(vec![Span::styled(text, style)])
}

fn blank() -> Line<'static> {
    Line::from(vec![])
}

fn key<'a>(k: &'a str, desc: &'a str, key_style: Style, desc_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(k, key_style),
        Span::styled(desc, desc_style),
    ])
}
