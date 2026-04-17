use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::model::project::CardType;
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Archived items ")
        .title_style(
            Style::default()
                .fg(theme().yellow)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().yellow));

    let state = match app.state.archived_list_state() {
        Some(s) => s,
        None => {
            frame.render_widget(block, area);
            return;
        }
    };

    if state.loading {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let p = Paragraph::new("Loading archived items...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme().text_muted));
        frame.render_widget(p, inner);
        return;
    }

    if let Some(err) = state.error.as_ref() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let p = Paragraph::new(format!("Error: {err}"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme().red));
        frame.render_widget(p, inner);
        return;
    }

    if state.cards.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let p = Paragraph::new("No archived items.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme().text_muted));
        frame.render_widget(p, inner);
        return;
    }

    let items: Vec<ListItem> = state
        .cards
        .iter()
        .map(|card| {
            let kind = match &card.card_type {
                CardType::Issue { .. } => "ISS",
                CardType::PullRequest { .. } => "PR ",
                CardType::DraftIssue => "DR ",
            };
            let number = card
                .number
                .map(|n| format!("#{n}"))
                .unwrap_or_else(|| "—".to_string());
            let labels = if card.labels.is_empty() {
                String::new()
            } else {
                let names: Vec<String> = card
                    .labels
                    .iter()
                    .map(|l| format!("[{}]", l.name))
                    .collect();
                format!("  {}", names.join(" "))
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{kind} "),
                    Style::default().fg(theme().text_muted),
                ),
                Span::styled(
                    format!("{number} "),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::styled(
                    card.title.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    labels,
                    Style::default().fg(theme().text_muted),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(theme().yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default().with_selected(Some(state.selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}
