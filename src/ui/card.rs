use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
};

use crate::color::parse_hex_color;
use crate::model::project::{
    Card, CardType, CiStatus, ColumnColor, CustomFieldValue, IssueState, PrState, PrStatus,
    ReviewDecision,
};
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
        if self.card.parent_issue.is_some() {
            title_spans.push(Span::styled(
                "↳ ",
                Style::default().fg(theme().text_dim),
            ));
        }
        if let Some(summary) = &self.card.sub_issues_summary
            && summary.total > 0
        {
            let text = format!("[{}/{}] ", summary.completed, summary.total);
            let color = if summary.completed >= summary.total {
                theme().green
            } else {
                theme().blue
            };
            title_spans.push(Span::styled(text, Style::default().fg(color)));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::{Label, ParentIssueRef, SubIssuesSummary};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn span_content(spans: &[Span<'_>]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn make_draft_card(title: &str) -> Card {
        Card {
            item_id: "1".into(),
            content_id: None,
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
            custom_fields: vec![],
            pr_status: None,
            linked_prs: vec![],
            reactions: vec![],
            archived: false,
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: vec![],
        }
    }

    fn render_card_widget(card: &Card, selected: bool, grabbing: bool, width: u16) -> String {
        let backend = TestBackend::new(width, CARD_HEIGHT);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(
                    CardWidget { card, selected, grabbing },
                    Rect::new(0, 0, width, CARD_HEIGHT),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..CARD_HEIGHT {
            for x in 0..width {
                out.push_str(buffer[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn card_widget_renders_title() {
        let card = make_draft_card("Hello world");
        let text = render_card_widget(&card, false, false, 30);
        assert!(text.contains("Hello world"), "buffer:\n{text}");
    }

    #[test]
    fn card_widget_shows_parent_indicator() {
        let mut card = make_draft_card("Child task");
        card.parent_issue = Some(ParentIssueRef {
            id: "parent".into(),
            number: 1,
            title: "Parent".into(),
            url: None,
        });
        let text = render_card_widget(&card, false, false, 30);
        assert!(text.contains("↳"), "buffer:\n{text}");
    }

    #[test]
    fn card_widget_shows_sub_issue_progress() {
        let mut card = make_draft_card("Parent task");
        card.sub_issues_summary = Some(SubIssuesSummary {
            completed: 2,
            total: 5,
        });
        let text = render_card_widget(&card, false, false, 30);
        assert!(text.contains("[2/5]"), "buffer:\n{text}");
    }

    #[test]
    fn card_widget_shows_label_name() {
        let mut card = make_draft_card("With label");
        card.labels = vec![Label {
            id: "label-bug".into(),
            name: "bug".into(),
            color: "ff0000".into(),
        }];
        let text = render_card_widget(&card, false, false, 30);
        assert!(text.contains("bug"), "buffer:\n{text}");
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
