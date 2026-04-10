use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(50, 70, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Help ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Navigation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
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
        Line::from(Span::styled(
            " Actions",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
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
            Span::styled("Filter cards", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  C-u     ", key_style),
            Span::styled("Clear filter", desc_style),
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
