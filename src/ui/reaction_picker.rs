use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::app::App;
use crate::model::project::{ReactionContent, ReactionSummary};
use crate::model::state::{ReactionPickerState, ReactionTarget};
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, state: &ReactionPickerState, app: &App) {
    let popup = centered_rect(48, 7, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Reactions ")
        .title_style(
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent));

    let hint_style = Style::default()
        .fg(theme().yellow)
        .add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(theme().text_muted);

    let target_reactions = current_reactions(state, app);

    let emoji_line = Line::from(build_picker_spans(state.cursor, &target_reactions));

    let lines = vec![
        Line::from(""),
        emoji_line,
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("h/l", hint_style),
            Span::styled(":nav  ", muted),
            Span::styled("Enter", hint_style),
            Span::styled(":toggle  ", muted),
            Span::styled("Esc", hint_style),
            Span::styled(":close", muted),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

fn build_picker_spans(cursor: usize, reactions: &[ReactionSummary]) -> Vec<Span<'static>> {
    let all = ReactionContent::all();
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(all.len() * 3 + 1);
    spans.push(Span::raw("  "));
    for (i, content) in all.iter().enumerate() {
        let summary = reactions.iter().find(|r| r.content == *content);
        let reacted = summary.is_some_and(|s| s.viewer_has_reacted);
        let count = summary.map(|s| s.count).unwrap_or(0);

        let is_cursor = i == cursor;
        let emoji_style = if is_cursor {
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else if reacted {
            Style::default()
                .fg(theme().green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme().text)
        };

        let emoji_text = format!(" {} ", content.emoji());
        spans.push(Span::styled(emoji_text, emoji_style));

        if count > 0 {
            let count_style = if reacted {
                Style::default().fg(theme().green)
            } else {
                Style::default().fg(theme().text_muted)
            };
            spans.push(Span::styled(format!("{count} "), count_style));
        } else {
            spans.push(Span::raw(" "));
        }
    }
    spans
}

fn current_reactions(state: &ReactionPickerState, app: &App) -> Vec<ReactionSummary> {
    let board = match &app.state.board {
        Some(b) => b,
        None => return Vec::new(),
    };
    match &state.target {
        ReactionTarget::CardBody { content_id } => {
            for col in &board.columns {
                for card in &col.cards {
                    if card.content_id.as_deref() == Some(content_id.as_str()) {
                        return card.reactions.clone();
                    }
                }
            }
            Vec::new()
        }
        ReactionTarget::Comment {
            comment_id,
            content_id,
        } => {
            for col in &board.columns {
                for card in &col.cards {
                    if card.content_id.as_deref() == Some(content_id.as_str())
                        && let Some(c) = card.comments.iter().find(|c| &c.id == comment_id)
                    {
                        return c.reactions.clone();
                    }
                }
            }
            Vec::new()
        }
    }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
