use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
};

use crate::model::project::{Card, CardType, ColumnColor, CustomFieldValue, IssueState, PrState};
use crate::ui::theme::theme;

pub const CARD_HEIGHT: u16 = 6;

pub struct CardWidget<'a> {
    pub card: &'a Card,
    pub selected: bool,
    pub grabbing: bool,
}

impl Widget for CardWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (border_style, border_type) = if self.grabbing {
            (
                Style::default().fg(theme().yellow),
                BorderType::Thick,
            )
        } else if self.selected {
            (
                Style::default().fg(theme().border_focused),
                BorderType::Rounded,
            )
        } else {
            (
                Style::default().fg(theme().border_unfocused),
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
                IssueState::Open => Span::styled("● ", Style::default().fg(theme().green)),
                IssueState::Closed => Span::styled("● ", Style::default().fg(theme().purple)),
            },
            CardType::PullRequest { state } => match state {
                PrState::Open => Span::styled("⑂ ", Style::default().fg(theme().green)),
                PrState::Closed => Span::styled("⑂ ", Style::default().fg(theme().red)),
                PrState::Merged => Span::styled("⑂ ", Style::default().fg(theme().purple)),
            },
            CardType::DraftIssue => Span::styled("○ ", Style::default().fg(theme().text_dim)),
        };

        let number_str = self
            .card
            .number
            .map(|n| format!("#{n} "))
            .unwrap_or_default();

        let mut title_spans = vec![
            type_indicator,
            Span::styled(
                number_str,
                Style::default().add_modifier(Modifier::DIM),
            ),
            Span::raw(&self.card.title),
        ];
        if let Some(milestone) = &self.card.milestone {
            title_spans.push(Span::styled(
                format!(" [{milestone}]"),
                Style::default().fg(theme().text_muted),
            ));
        }
        let title_line = Line::from(title_spans);

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
            Line::from(Span::styled(text, Style::default().fg(theme().yellow)))
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
                    let color = parse_hex_color(&label.color).unwrap_or(theme().text_dim);
                    let mut spans = vec![Span::styled(
                        &label.name,
                        Style::default().fg(theme().text_inverted).bg(color),
                    )];
                    if i < self.card.labels.len() - 1 {
                        spans.push(Span::raw(" "));
                    }
                    spans
                })
                .collect();
            Line::from(spans)
        };

        let custom_field_line = if self.card.custom_fields.is_empty() {
            Line::from("")
        } else {
            let mut spans: Vec<Span> = Vec::new();
            for (i, v) in self.card.custom_fields.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw("  "));
                }
                match v {
                    CustomFieldValue::SingleSelect { name, color, .. } => {
                        let bg = color
                            .as_ref()
                            .map(column_color_to_tui)
                            .unwrap_or(theme().border_unfocused);
                        spans.push(Span::styled(
                            name.clone(),
                            Style::default().fg(theme().text_inverted).bg(bg),
                        ));
                    }
                    CustomFieldValue::Number { number, .. } => {
                        let text = if number.fract() == 0.0 && number.abs() < 1e16 {
                            format!("#{}", *number as i64)
                        } else {
                            format!("#{number}")
                        };
                        spans.push(Span::styled(text, Style::default().fg(theme().text_dim)));
                    }
                    CustomFieldValue::Text { text, .. } => {
                        spans.push(Span::styled(
                            text.clone(),
                            Style::default().fg(theme().text_dim),
                        ));
                    }
                    CustomFieldValue::Date { date, .. } => {
                        spans.push(Span::styled(
                            date.clone(),
                            Style::default().fg(theme().text_dim),
                        ));
                    }
                    CustomFieldValue::Iteration { title, .. } => {
                        spans.push(Span::styled(
                            format!("⟳ {title}"),
                            Style::default().fg(theme().text_dim),
                        ));
                    }
                }
            }
            Line::from(spans)
        };

        let lines = vec![title_line, assignee_line, label_line, custom_field_line];
        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

pub fn column_color_to_tui(color: &ColumnColor) -> Color {
    match color {
        ColumnColor::Blue => Color::Blue,
        ColumnColor::Gray => Color::DarkGray,
        ColumnColor::Green => Color::Green,
        ColumnColor::Orange => Color::Rgb(255, 165, 0),
        ColumnColor::Pink => Color::Rgb(255, 105, 180),
        ColumnColor::Purple => Color::Magenta,
        ColumnColor::Red => Color::Red,
        ColumnColor::Yellow => Color::Yellow,
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
