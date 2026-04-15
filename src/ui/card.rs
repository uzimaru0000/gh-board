use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
};

use crate::model::project::{Card, CardType, CiStatus, IssueState, PrState, PrStatus, ReviewDecision};
use crate::ui::theme::theme;

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

        let type_indicator = type_indicator_span(&self.card.card_type);

        let number_str = self
            .card
            .number
            .map(|n| format!("#{n} "))
            .unwrap_or_default();

        let mut title_spans = vec![type_indicator];
        if matches!(self.card.card_type, CardType::PullRequest { .. })
            && let Some(status) = &self.card.pr_status
        {
            title_spans.extend(pr_status_spans(status));
        }
        title_spans.push(Span::styled(
            number_str,
            Style::default().add_modifier(Modifier::DIM),
        ));
        title_spans.push(Span::raw(&self.card.title));
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

        let lines = vec![title_line, assignee_line, label_line];
        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

fn type_indicator_span(ct: &CardType) -> Span<'static> {
    match ct {
        CardType::Issue { state } => match state {
            // nf-oct-issue_opened
            IssueState::Open => Span::styled("\u{f41b} ", Style::default().fg(theme().green)),
            // nf-oct-issue_closed
            IssueState::Closed => Span::styled("\u{f41d} ", Style::default().fg(theme().purple)),
        },
        CardType::PullRequest { state } => {
            // nf-oct-git_pull_request
            let color = match state {
                PrState::Open => theme().green,
                PrState::Closed => theme().red,
                PrState::Merged => theme().purple,
            };
            Span::styled("\u{f407} ", Style::default().fg(color))
        }
        // nf-oct-note
        CardType::DraftIssue => Span::styled("\u{f404} ", Style::default().fg(theme().text_dim)),
    }
}

pub fn pr_status_spans(status: &PrStatus) -> Vec<Span<'static>> {
    let mut out: Vec<Span<'static>> = Vec::new();
    if let Some(ci) = &status.ci {
        let (glyph, color) = match ci {
            // nf-oct-check
            CiStatus::Success => ("\u{f42e}", theme().green),
            // nf-oct-x
            CiStatus::Failure | CiStatus::Error => ("\u{f467}", theme().red),
            // nf-oct-dot_fill
            CiStatus::Pending | CiStatus::Expected => ("\u{f444}", theme().yellow),
        };
        out.push(Span::styled(
            format!("{glyph} "),
            Style::default().fg(color),
        ));
    }
    if let Some(rd) = &status.review_decision {
        let (glyph, color) = match rd {
            // nf-oct-thumbsup
            ReviewDecision::Approved => ("\u{f49e}", theme().green),
            // nf-oct-alert
            ReviewDecision::ChangesRequested => ("\u{f421}", theme().red),
            // nf-oct-eye
            ReviewDecision::ReviewRequired => ("\u{f441}", theme().yellow),
        };
        out.push(Span::styled(
            format!("{glyph} "),
            Style::default().fg(color),
        ));
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn span_content(spans: &[Span<'_>]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn pr_status_spans_empty_when_all_none() {
        let status = PrStatus::default();
        assert!(pr_status_spans(&status).is_empty());
    }

    #[test]
    fn pr_status_spans_success_approved() {
        let status = PrStatus {
            ci: Some(CiStatus::Success),
            review_decision: Some(ReviewDecision::Approved),
            review_requests: vec![],
        };
        let spans = pr_status_spans(&status);
        assert_eq!(spans.len(), 2);
        assert_eq!(span_content(&spans), "\u{f42e} \u{f49e} ");
    }

    #[test]
    fn pr_status_spans_failure_changes_requested() {
        let status = PrStatus {
            ci: Some(CiStatus::Failure),
            review_decision: Some(ReviewDecision::ChangesRequested),
            review_requests: vec![],
        };
        let spans = pr_status_spans(&status);
        assert_eq!(spans.len(), 2);
        assert_eq!(span_content(&spans), "\u{f467} \u{f421} ");
    }

    #[test]
    fn pr_status_spans_pending_required() {
        let status = PrStatus {
            ci: Some(CiStatus::Pending),
            review_decision: Some(ReviewDecision::ReviewRequired),
            review_requests: vec!["alice".into()],
        };
        let spans = pr_status_spans(&status);
        assert_eq!(spans.len(), 2);
        assert_eq!(span_content(&spans), "\u{f444} \u{f441} ");
    }

    #[test]
    fn pr_status_spans_only_ci() {
        let status = PrStatus {
            ci: Some(CiStatus::Error),
            review_decision: None,
            review_requests: vec![],
        };
        let spans = pr_status_spans(&status);
        assert_eq!(spans.len(), 1);
        assert_eq!(span_content(&spans), "\u{f467} ");
    }
}
