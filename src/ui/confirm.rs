use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::model::state::{ConfirmAction, ConfirmState};
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, state: &ConfirmState) {
    let popup = centered_rect(50, 7, area);
    frame.render_widget(Clear, popup);

    let accent = match state.action {
        ConfirmAction::ArchiveCard { .. } => theme().yellow,
    };

    let block = Block::default()
        .title(" Confirm ")
        .title_style(
            Style::default()
                .fg(accent)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent));

    let key_style = Style::default()
        .fg(theme().yellow)
        .add_modifier(Modifier::BOLD);

    let message = match state.action {
        ConfirmAction::ArchiveCard { .. } => format!("  Archive \"{}\"?", state.title),
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            message,
            Style::default().fg(theme().text),
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
