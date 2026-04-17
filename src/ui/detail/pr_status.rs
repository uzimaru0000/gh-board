use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::model::project::{Card, CardType, CiStatus, PrState, ReviewDecision};
use crate::ui::theme::theme;

pub(super) fn pr_status_lines(card: &Card) -> Vec<Line<'static>> {
    if !matches!(card.card_type, CardType::PullRequest { .. }) {
        return Vec::new();
    }
    let Some(status) = &card.pr_status else {
        return Vec::new();
    };

    let dim_style = Style::default().fg(theme().text_muted);
    let label_style = Style::default().fg(theme().text);
    let mut out: Vec<Line<'static>> = Vec::new();

    if let Some(ci) = &status.ci {
        let (glyph, label, color) = match ci {
            CiStatus::Success => ("\u{f42e}", "Success", theme().green),
            CiStatus::Failure => ("\u{f467}", "Failure", theme().red),
            CiStatus::Error => ("\u{f467}", "Error", theme().red),
            CiStatus::Pending => ("\u{f444}", "Pending", theme().yellow),
            CiStatus::Expected => ("\u{f444}", "Expected", theme().yellow),
        };
        out.push(Line::from(vec![
            Span::styled("  CI: ".to_string(), label_style),
            Span::styled(format!("{glyph} {label}"), Style::default().fg(color)),
        ]));
    }

    if let Some(rd) = &status.review_decision {
        let (glyph, label, color) = match rd {
            ReviewDecision::Approved => ("\u{f49e}", "Approved", theme().green),
            ReviewDecision::ChangesRequested => ("\u{f421}", "Changes requested", theme().red),
            ReviewDecision::ReviewRequired => ("\u{f441}", "Review required", theme().yellow),
        };
        out.push(Line::from(vec![
            Span::styled("  Review: ".to_string(), label_style),
            Span::styled(format!("{glyph} {label}"), Style::default().fg(color)),
        ]));
    }

    if !status.review_requests.is_empty() {
        out.push(Line::from(Span::styled("  Reviewers:".to_string(), label_style)));
        for reviewer in &status.review_requests {
            out.push(Line::from(Span::styled(
                format!("    @{reviewer}"),
                Style::default().fg(theme().yellow),
            )));
        }
    }

    if out.is_empty() {
        out.push(Line::from(Span::styled("  CI/Review: --".to_string(), dim_style)));
    }

    out
}

pub(super) fn linked_prs_lines(card: &Card) -> Vec<Line<'static>> {
    let dim_style = Style::default().fg(theme().text_muted);
    if card.linked_prs.is_empty() {
        return vec![Line::from(Span::styled("  --".to_string(), dim_style))];
    }
    card.linked_prs
        .iter()
        .map(|pr| {
            let color = match pr.state {
                PrState::Open => theme().green,
                PrState::Closed => theme().red,
                PrState::Merged => theme().purple,
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled("\u{f407} ".to_string(), Style::default().fg(color)),
                Span::styled(
                    format!("#{} ", pr.number),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::styled(pr.title.clone(), Style::default().fg(theme().text)),
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::{IssueState, LinkedPr, PrStatus};

    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn pr_card(pr_status: Option<PrStatus>) -> Card {
        Card {
            item_id: "i1".into(),
            content_id: Some("pr1".into()),
            title: "T".into(),
            number: Some(1),
            card_type: CardType::PullRequest { state: PrState::Open },
            assignees: vec![],
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
            custom_fields: vec![],
            pr_status,
            linked_prs: vec![],
            reactions: vec![],
            archived: false,
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: vec![],
        }
    }

    fn issue_card_with_linked(linked: Vec<LinkedPr>) -> Card {
        Card {
            item_id: "i1".into(),
            content_id: Some("issue1".into()),
            title: "T".into(),
            number: Some(1),
            card_type: CardType::Issue { state: IssueState::Open },
            assignees: vec![],
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
            custom_fields: vec![],
            pr_status: None,
            linked_prs: linked,
            reactions: vec![],
            archived: false,
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: vec![],
        }
    }

    #[test]
    fn linked_prs_lines_empty_placeholder() {
        let card = issue_card_with_linked(vec![]);
        let lines = linked_prs_lines(&card);
        assert_eq!(lines.len(), 1);
        assert!(line_text(&lines[0]).contains("--"));
    }

    #[test]
    fn linked_prs_lines_renders_entries() {
        let card = issue_card_with_linked(vec![
            LinkedPr {
                number: 42,
                title: "Fix".into(),
                url: "https://github.com/o/r/pull/42".into(),
                state: PrState::Merged,
            },
            LinkedPr {
                number: 43,
                title: "Follow-up".into(),
                url: "https://github.com/o/r/pull/43".into(),
                state: PrState::Open,
            },
        ]);
        let lines = linked_prs_lines(&card);
        assert_eq!(lines.len(), 2);
        assert!(line_text(&lines[0]).contains("#42"));
        assert!(line_text(&lines[0]).contains("Fix"));
        assert!(line_text(&lines[0]).contains("\u{f407}"));
        assert!(line_text(&lines[1]).contains("#43"));
    }

    #[test]
    fn pr_status_lines_empty_for_non_pr() {
        let mut card = pr_card(None);
        card.card_type = CardType::DraftIssue;
        assert!(pr_status_lines(&card).is_empty());
    }

    #[test]
    fn pr_status_lines_success_and_approved() {
        let card = pr_card(Some(PrStatus {
            ci: Some(CiStatus::Success),
            review_decision: Some(ReviewDecision::Approved),
            review_requests: vec!["alice".into(), "bob".into()],
        }));
        let lines = pr_status_lines(&card);
        assert_eq!(lines.len(), 5);
        assert!(line_text(&lines[0]).contains("\u{f42e}"));
        assert!(line_text(&lines[0]).contains("Success"));
        assert!(line_text(&lines[1]).contains("\u{f49e}"));
        assert!(line_text(&lines[1]).contains("Approved"));
        assert!(line_text(&lines[3]).contains("@alice"));
        assert!(line_text(&lines[4]).contains("@bob"));
    }

    #[test]
    fn pr_status_lines_failure_changes_requested() {
        let card = pr_card(Some(PrStatus {
            ci: Some(CiStatus::Failure),
            review_decision: Some(ReviewDecision::ChangesRequested),
            review_requests: vec![],
        }));
        let lines = pr_status_lines(&card);
        assert_eq!(lines.len(), 2);
        assert!(line_text(&lines[0]).contains("Failure"));
        assert!(line_text(&lines[1]).contains("Changes requested"));
    }

    #[test]
    fn pr_status_lines_none_pr_status_shows_placeholder() {
        let card = pr_card(None);
        assert!(pr_status_lines(&card).is_empty());
    }
}
