use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::model::state::ConfirmState;

pub fn render(frame: &mut Frame, area: Rect, state: &ConfirmState) {
    let popup = centered_rect(50, 7, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Confirm ")
        .title_style(
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Delete \"{}\"?", state.title),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("y", key_style),
            Span::raw(":Yes  "),
            Span::styled("n/Esc", key_style),
            Span::raw(":Cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
