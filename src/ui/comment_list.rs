use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let card = match app.state.selected_card_ref() {
        Some(c) => c,
        None => return,
    };

    let cls = match &app.state.comment_list_state {
        Some(s) => s,
        None => return,
    };

    let popup = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Comments ")
        .title_style(
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    let viewer = &app.state.viewer_login;
    let mut lines: Vec<Line<'static>> = Vec::new();

    if card.comments.is_empty() {
        lines.push(Line::from(Span::styled(
            "(No comments)",
            Style::default().fg(theme().text_muted),
        )));
    } else {
        for (i, comment) in card.comments.iter().enumerate() {
            let is_selected = i == cls.cursor;
            let is_own = comment.author == *viewer;
            let date_display = &comment.created_at[..10.min(comment.created_at.len())];

            let first_line = comment
                .body
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(40)
                .collect::<String>();

            let marker = if is_selected { "▶ " } else { "  " };

            let author_style = if is_selected {
                Style::default()
                    .fg(theme().accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme().yellow)
                    .add_modifier(Modifier::BOLD)
            };

            let body_style = if is_selected {
                Style::default().fg(theme().text)
            } else {
                Style::default().fg(theme().text_dim)
            };

            let edit_hint = if is_own && is_selected {
                Span::styled(" [e:edit]", Style::default().fg(theme().green))
            } else {
                Span::raw("")
            };

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), author_style),
                Span::styled(format!("@{}", comment.author), author_style),
                Span::styled(format!("  {date_display}"), Style::default().fg(theme().text_muted)),
                edit_hint,
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(first_line, body_style),
            ]));

            if i < card.comments.len() - 1 {
                lines.push(Line::from(""));
            }
        }
    }

    // Footer area
    let content_height = inner.height.saturating_sub(2) as usize;
    let vert = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let scroll = cls.cursor.saturating_sub(content_height / 2);
    let paragraph = Paragraph::new(lines).scroll((scroll as u16 * 3, 0));
    frame.render_widget(paragraph, vert[0]);

    let hint_style = Style::default()
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme().text_muted);

    let footer = Line::from(vec![
        Span::styled("j/k", hint_style),
        Span::styled(":nav  ", desc_style),
        Span::styled("e", hint_style),
        Span::styled(":edit  ", desc_style),
        Span::styled("c", hint_style),
        Span::styled(":new  ", desc_style),
        Span::styled("Esc", hint_style),
        Span::styled(":close", desc_style),
    ]);
    frame.render_widget(footer, vert[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
