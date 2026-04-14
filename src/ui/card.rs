use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
};

use crate::model::project::{Card, CardType, IssueState, PrState};
use crate::ui::theme::THEME;

pub const CARD_HEIGHT: u16 = 5;

pub struct CardWidget<'a> {
    pub card: &'a Card,
    pub selected: bool,
    pub grabbing: bool,
}

impl Widget for CardWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (border_style, border_type) = if self.grabbing {
            (
                Style::default().fg(THEME.yellow),
                BorderType::Thick,
            )
        } else if self.selected {
            (
                Style::default().fg(THEME.border_focused),
                BorderType::Rounded,
            )
        } else {
            (
                Style::default().fg(THEME.border_unfocused),
                BorderType::Rounded,
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .padding(Padding::horizontal(1));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let type_indicator = match &self.card.card_type {
            CardType::Issue { state } => match state {
                IssueState::Open => Span::styled("● ", Style::default().fg(THEME.green)),
                IssueState::Closed => Span::styled("● ", Style::default().fg(THEME.purple)),
            },
            CardType::PullRequest { state } => match state {
                PrState::Open => Span::styled("⑂ ", Style::default().fg(THEME.green)),
                PrState::Closed => Span::styled("⑂ ", Style::default().fg(THEME.red)),
                PrState::Merged => Span::styled("⑂ ", Style::default().fg(THEME.purple)),
            },
            CardType::DraftIssue => Span::styled("○ ", Style::default().fg(THEME.text_dim)),
        };

        let number_str = self
            .card
            .number
            .map(|n| format!("#{n} "))
            .unwrap_or_default();

        let title_line = Line::from(vec![
            type_indicator,
            Span::styled(
                number_str,
                Style::default().add_modifier(Modifier::DIM),
            ),
            Span::raw(&self.card.title),
        ]);

        let assignee_line = if self.card.assignees.is_empty() {
            Line::from("")
        } else {
            let text = self
                .card
                .assignees
                .iter()
                .map(|a| format!("@{a}"))
                .collect::<Vec<_>>()
                .join(" ");
            Line::from(Span::styled(text, Style::default().fg(THEME.yellow)))
        };

        let label_line = if self.card.labels.is_empty() {
            Line::from("")
        } else {
            let spans: Vec<Span> = self
                .card
                .labels
                .iter()
                .enumerate()
                .flat_map(|(i, label)| {
                    let color = parse_hex_color(&label.color).unwrap_or(THEME.text_dim);
                    let mut spans = vec![Span::styled(
                        &label.name,
                        Style::default().fg(THEME.text_inverted).bg(color),
                    )];
                    if i < self.card.labels.len() - 1 {
                        spans.push(Span::raw(" "));
                    }
                    spans
                })
                .collect();
            Line::from(spans)
        };

        let lines = vec![title_line, assignee_line, label_line];
        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
