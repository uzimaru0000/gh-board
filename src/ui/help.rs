use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(50, 70, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Help ")
        .title_style(Style::default().fg(theme().accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent));

    let key_style = Style::default()
        .fg(theme().yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme().text);
    let section_style = Style::default()
        .fg(theme().accent)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(" Navigation", section_style)),
        Line::from(vec![
            Span::styled("  j/↓     ", key_style),
            Span::styled("Next card", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  k/↑     ", key_style),
            Span::styled("Previous card", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  h/←     ", key_style),
            Span::styled("Previous column", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  l/→     ", key_style),
            Span::styled("Next column", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  g       ", key_style),
            Span::styled("First card", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  G       ", key_style),
            Span::styled("Last card", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Tab     ", key_style),
            Span::styled("Next column (wrap)", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Actions", section_style)),
        Line::from(vec![
            Span::styled("  Space   ", key_style),
            Span::styled("Grab card (move mode)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  H       ", key_style),
            Span::styled("Move card left", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  L       ", key_style),
            Span::styled("Move card right", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  n       ", key_style),
            Span::styled("New card (draft/issue)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  d       ", key_style),
            Span::styled("Delete card", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", key_style),
            Span::styled("View card detail", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  p       ", key_style),
            Span::styled("Switch project", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  /       ", key_style),
            Span::styled("Filter (label: assignee: milestone: |:OR)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  C-u     ", key_style),
            Span::styled("Clear filter / view", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  1-9     ", key_style),
            Span::styled("Switch to view 1-9", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  0       ", key_style),
            Span::styled("Show all (clear view)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  r       ", key_style),
            Span::styled("Refresh", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  ?       ", key_style),
            Span::styled("Toggle help", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  q/Esc   ", key_style),
            Span::styled("Quit", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Detail View (Content)", section_style)),
        Line::from(vec![
            Span::styled("  j/k     ", key_style),
            Span::styled("Scroll", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  h/l     ", key_style),
            Span::styled("Table scroll", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Tab     ", key_style),
            Span::styled("Switch to sidebar", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Enter/o ", key_style),
            Span::styled("Open in browser", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  e       ", key_style),
            Span::styled("Edit card", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Detail View (Sidebar)", section_style)),
        Line::from(vec![
            Span::styled("  j/k     ", key_style),
            Span::styled("Navigate sections", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", key_style),
            Span::styled("Edit / Select", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  d       ", key_style),
            Span::styled("Delete card", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Tab     ", key_style),
            Span::styled("Switch to content", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Esc     ", key_style),
            Span::styled("Back to content", desc_style),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
