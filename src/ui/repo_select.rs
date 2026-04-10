use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::model::project::Repository;
use crate::model::state::RepoSelectState;

pub fn render(frame: &mut Frame, area: Rect, repos: &[Repository], state: &RepoSelectState) {
    let height = (repos.len() as u16 + 4).min(20);
    let popup = centered_rect(50, height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Select Repository ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let hint_style = Style::default().fg(Color::DarkGray);
    let selected_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(Color::White);

    let mut lines: Vec<Line> = Vec::new();

    for (i, repo) in repos.iter().enumerate() {
        let style = if i == state.selected_index {
            selected_style
        } else {
            normal_style
        };
        let prefix = if i == state.selected_index {
            " > "
        } else {
            "   "
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{}", repo.name_with_owner),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", hint_style),
        Span::styled(":select  ", hint_style),
        Span::styled("Enter", hint_style),
        Span::styled(":confirm  ", hint_style),
        Span::styled("Esc", hint_style),
        Span::styled(":cancel", hint_style),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
