use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::color::parse_hex_color;
use crate::model::project::{CardType, ColumnColor, CustomFieldValue, IssueState, PrState};
use crate::model::state::{
    DetailPane, SidebarEditMode, SidebarSection, SIDEBAR_ASSIGNEES, SIDEBAR_LABELS,
    SIDEBAR_MILESTONE, SIDEBAR_STATUS,
};
use crate::ui::card::column_color_to_tui;
use crate::ui::theme::theme;

use super::pr_status::{linked_prs_lines, pr_status_lines};

/// 右ペイン: サイドバー (Status, Assignees, Labels, Archive)
pub(super) fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let card = match app.state.current_detail_card() {
        Some(c) => c,
        None => return,
    };

    if let Some(edit) = &app.state.sidebar_edit {
        render_sidebar_edit(frame, area, edit);
        return;
    }

    let focused = app.state.detail_pane == DetailPane::Sidebar;
    let selected = app.state.sidebar_selected;

    let header_style = Style::default()
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme().text_muted);
    let selected_marker = if focused { "▶ " } else { "  " };

    let sections_layout = app.state.sidebar_sections();
    let mut section_line_offsets: Vec<u16> = vec![0; sections_layout.len()];
    let mut lines: Vec<Line<'static>> = Vec::new();
    let record = |idx_opt: Option<usize>, offsets: &mut [u16], lines_len: usize| {
        if let Some(i) = idx_opt
            && i < offsets.len()
        {
            offsets[i] = lines_len as u16;
        }
    };

    // ── Status section ──
    let status_header_style = if focused && selected == SIDEBAR_STATUS {
        Style::default()
            .fg(theme().accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    record(
        sections_layout
            .iter()
            .position(|s| matches!(s, SidebarSection::Status)),
        &mut section_line_offsets,
        lines.len(),
    );
    lines.push(Line::from(Span::styled("Status", status_header_style)));

    let board = app.state.board.as_ref();
    let current_col_name = board
        .and_then(|b| b.columns.get(app.state.selected_column))
        .map(|c| c.name.as_str())
        .unwrap_or("?");

    if app.state.status_select_open {
        if let Some(board) = board {
            for (i, col) in board.columns.iter().enumerate() {
                if col.option_id.is_empty() {
                    continue;
                }
                let is_cursor = i == app.state.status_select_cursor;
                let is_current = i == app.state.selected_column;
                let marker = if is_cursor { "▶ " } else { "  " };
                let style = if is_cursor {
                    Style::default()
                        .fg(theme().accent)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(theme().green)
                } else {
                    Style::default().fg(theme().text)
                };
                lines.push(Line::from(Span::styled(
                    format!("{marker}{}", col.name),
                    style,
                )));
            }
        }
    } else {
        let marker = if focused && selected == SIDEBAR_STATUS {
            selected_marker
        } else {
            "  "
        };
        let (state_label, state_color) = match &card.card_type {
            CardType::Issue { state } => match state {
                IssueState::Open => ("Open", theme().green),
                IssueState::Closed => ("Closed", theme().purple),
            },
            CardType::PullRequest { state } => match state {
                PrState::Open => ("Open", theme().green),
                PrState::Closed => ("Closed", theme().red),
                PrState::Merged => ("Merged", theme().purple),
            },
            CardType::DraftIssue => ("Draft", theme().text_dim),
        };
        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), dim_style),
            Span::styled(
                current_col_name.to_string(),
                Style::default().fg(theme().text),
            ),
            Span::styled(
                format!(" ({state_label})"),
                Style::default().fg(state_color),
            ),
        ]));
        lines.extend(pr_status_lines(card));
    }
    lines.push(Line::from(""));

    // ── Assignees section ──
    let assignee_header_style = if focused && selected == SIDEBAR_ASSIGNEES {
        Style::default()
            .fg(theme().accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    record(
        sections_layout
            .iter()
            .position(|s| matches!(s, SidebarSection::Assignees)),
        &mut section_line_offsets,
        lines.len(),
    );
    lines.push(Line::from(Span::styled("Assignees", assignee_header_style)));
    if card.assignees.is_empty() {
        lines.push(Line::from(Span::styled("  --", dim_style)));
    } else {
        for assignee in &card.assignees {
            lines.push(Line::from(Span::styled(
                format!("  @{assignee}"),
                Style::default().fg(theme().yellow),
            )));
        }
    }
    lines.push(Line::from(""));

    // ── Labels section ──
    let label_header_style = if focused && selected == SIDEBAR_LABELS {
        Style::default()
            .fg(theme().accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    record(
        sections_layout
            .iter()
            .position(|s| matches!(s, SidebarSection::Labels)),
        &mut section_line_offsets,
        lines.len(),
    );
    lines.push(Line::from(Span::styled("Labels", label_header_style)));
    if card.labels.is_empty() {
        lines.push(Line::from(Span::styled("  --", dim_style)));
    } else {
        for label in &card.labels {
            let color = parse_hex_color(&label.color).unwrap_or(theme().text_dim);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.name.clone(),
                    Style::default().fg(theme().text_inverted).bg(color),
                ),
            ]));
        }
    }
    lines.push(Line::from(""));

    // ── Milestone section ──
    let milestone_header_style = if focused && selected == SIDEBAR_MILESTONE {
        Style::default()
            .fg(theme().accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    record(
        sections_layout
            .iter()
            .position(|s| matches!(s, SidebarSection::Milestone)),
        &mut section_line_offsets,
        lines.len(),
    );
    lines.push(Line::from(Span::styled("Milestone", milestone_header_style)));
    let milestone_text = card.milestone.as_deref().unwrap_or("--");
    lines.push(Line::from(Span::styled(
        format!("  {milestone_text}"),
        if card.milestone.is_some() {
            Style::default().fg(theme().text)
        } else {
            dim_style
        },
    )));
    lines.push(Line::from(""));

    // ── Linked PRs section (Issue only) ──
    if matches!(card.card_type, CardType::Issue { .. }) {
        lines.push(Line::from(Span::styled("Linked PRs", header_style)));
        lines.extend(linked_prs_lines(card));
        lines.push(Line::from(""));
    }

    // ── Custom fields sections ──
    let sections = &sections_layout;
    let field_defs = app
        .state
        .board
        .as_ref()
        .map(|b| b.field_definitions.as_slice())
        .unwrap_or(&[]);
    for (i, field) in field_defs.iter().enumerate() {
        let sidebar_idx = sections
            .iter()
            .position(|s| matches!(s, SidebarSection::CustomField(j) if *j == i))
            .unwrap_or(0);
        let header = if focused && selected == sidebar_idx {
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD)
        } else {
            header_style
        };
        record(Some(sidebar_idx), &mut section_line_offsets, lines.len());
        lines.push(Line::from(Span::styled(field.name().to_string(), header)));
        let current = card
            .custom_fields
            .iter()
            .find(|v| v.field_id() == field.id());
        lines.push(render_custom_field_value_line(current));
        lines.push(Line::from(""));
    }

    // ── Parent / Sub-issues sections (Issue only) ──
    if matches!(card.card_type, CardType::Issue { .. }) {
        if let Some(parent) = &card.parent_issue {
            let parent_idx = sections
                .iter()
                .position(|s| matches!(s, SidebarSection::Parent));
            let focused_here = focused && parent_idx == Some(selected);
            let header_s = if focused_here {
                Style::default()
                    .fg(theme().accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                header_style
            };
            record(parent_idx, &mut section_line_offsets, lines.len());
            lines.push(Line::from(Span::styled("Parent", header_s)));
            let marker = if focused_here { selected_marker } else { "  " };
            let title_color = if focused_here {
                theme().accent
            } else {
                theme().text
            };
            lines.push(Line::from(vec![
                Span::raw(marker.to_string()),
                Span::styled(
                    format!("#{} ", parent.number),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::styled(parent.title.clone(), Style::default().fg(title_color)),
            ]));
            lines.push(Line::from(""));
        }

        if let Some(summary) = &card.sub_issues_summary
            && summary.total > 0
        {
            let header_text = format!("Sub-issues [{}/{}]", summary.completed, summary.total);
            lines.push(Line::from(Span::styled(header_text, header_style)));
            if card.sub_issues.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  ...".to_string(),
                    dim_style,
                )));
            } else {
                for (i, sub) in card.sub_issues.iter().enumerate() {
                    let idx_in_sections = sections
                        .iter()
                        .position(|s| matches!(s, SidebarSection::SubIssue(j) if *j == i));
                    let focused_here = focused && idx_in_sections == Some(selected);
                    let marker = if focused_here { selected_marker } else { "  " };
                    let (glyph, color) = match sub.state {
                        IssueState::Open => ("\u{f41b} ", theme().green),
                        IssueState::Closed => ("\u{f41d} ", theme().purple),
                    };
                    let title_color = if focused_here {
                        theme().accent
                    } else {
                        theme().text
                    };
                    record(idx_in_sections, &mut section_line_offsets, lines.len());
                    lines.push(Line::from(vec![
                        Span::raw(marker.to_string()),
                        Span::styled(glyph.to_string(), Style::default().fg(color)),
                        Span::styled(
                            format!("#{} ", sub.number),
                            Style::default().add_modifier(Modifier::DIM),
                        ),
                        Span::styled(sub.title.clone(), Style::default().fg(title_color)),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }
    }

    let block = Block::default().padding(Padding::horizontal(1));
    let inner = block.inner(area);
    let btn_width = inner.width as usize;

    // ── Archive button ──
    let archive_idx = app.state.sidebar_archive_index();
    let is_archive_focused = focused && selected == archive_idx;
    let btn_bg = if is_archive_focused {
        theme().yellow
    } else {
        theme().border_unfocused
    };
    let edge_style = Style::default().fg(btn_bg);
    let text_color = if is_archive_focused {
        theme().text_inverted
    } else {
        theme().text
    };
    let fill_style = Style::default().fg(text_color).bg(btn_bg);
    let label = "Archive";
    let pad_total = btn_width.saturating_sub(label.len());
    let pad_left = pad_total / 2;
    let pad_right = pad_total - pad_left;
    record(Some(archive_idx), &mut section_line_offsets, lines.len());
    lines.push(Line::from(Span::styled(
        "▄".repeat(btn_width),
        edge_style,
    )));
    lines.push(Line::from(Span::styled(
        format!("{}{label}{}", " ".repeat(pad_left), " ".repeat(pad_right)),
        fill_style,
    )));
    lines.push(Line::from(Span::styled(
        "▀".repeat(btn_width),
        edge_style,
    )));

    let total_lines = lines.len() as u16;
    let visible = inner.height;
    let max_scroll = total_lines.saturating_sub(visible);
    let scroll = if focused && selected < section_line_offsets.len() {
        let target = section_line_offsets[selected];
        let section_end = if selected + 1 < section_line_offsets.len() {
            section_line_offsets[selected + 1]
        } else {
            total_lines
        };
        let desired_top = target.saturating_sub(1);
        let desired_bottom = section_end.saturating_sub(visible);
        desired_top.max(desired_bottom).min(max_scroll)
    } else {
        0
    };

    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), inner);
}

fn render_sidebar_edit(frame: &mut Frame, area: Rect, edit: &SidebarEditMode) {
    let (title, items, cursor) = match edit {
        SidebarEditMode::Labels { items, cursor } => ("Labels", items.as_slice(), *cursor),
        SidebarEditMode::Assignees { items, cursor } => ("Assignees", items.as_slice(), *cursor),
        SidebarEditMode::CustomFieldSingleSelect { .. }
        | SidebarEditMode::CustomFieldIteration { .. } => {
            render_custom_field_select_edit(frame, area, edit);
            return;
        }
        SidebarEditMode::CustomFieldText { .. }
        | SidebarEditMode::CustomFieldNumber { .. }
        | SidebarEditMode::CustomFieldDate { .. } => {
            render_custom_field_text_edit(frame, area, edit);
            return;
        }
    };

    let header_style = Style::default()
        .fg(theme().accent)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme().text_muted);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("{title}  (Enter: toggle, Esc: close)"),
        header_style,
    )));
    lines.push(Line::from(""));

    for (i, item) in items.iter().enumerate() {
        let is_cursor = i == cursor;
        let check = if item.applied { "[x]" } else { "[ ]" };
        let marker = if is_cursor { "▶" } else { " " };

        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(
            format!("{marker} {check} "),
            if is_cursor {
                Style::default()
                    .fg(theme().accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                dim_style
            },
        ));

        if let Some(color_hex) = &item.color {
            let color = parse_hex_color(color_hex).unwrap_or(theme().text_dim);
            spans.push(Span::styled(
                item.name.clone(),
                Style::default().fg(theme().text_inverted).bg(color),
            ));
        } else {
            spans.push(Span::styled(
                format!("@{}", item.name),
                if is_cursor {
                    Style::default().fg(theme().text)
                } else {
                    Style::default().fg(theme().yellow)
                },
            ));
        }

        lines.push(Line::from(spans));
    }

    if items.is_empty() {
        lines.push(Line::from(Span::styled("  (none available)", dim_style)));
    }

    let block = Block::default().padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_custom_field_value_line(current: Option<&CustomFieldValue>) -> Line<'static> {
    let dim_style = Style::default().fg(theme().text_muted);
    match current {
        None => Line::from(Span::styled("  --", dim_style)),
        Some(CustomFieldValue::SingleSelect { name, color, .. }) => {
            let bg = color
                .as_ref()
                .map(column_color_to_tui)
                .unwrap_or(theme().border_unfocused);
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    name.clone(),
                    Style::default().fg(theme().text_inverted).bg(bg),
                ),
            ])
        }
        Some(CustomFieldValue::Number { number, .. }) => {
            let s = if number.fract() == 0.0 && number.abs() < 1e16 {
                format!("  {}", *number as i64)
            } else {
                format!("  {number}")
            };
            Line::from(Span::styled(s, Style::default().fg(theme().text)))
        }
        Some(CustomFieldValue::Text { text, .. }) => Line::from(Span::styled(
            format!("  {text}"),
            Style::default().fg(theme().text),
        )),
        Some(CustomFieldValue::Date { date, .. }) => Line::from(Span::styled(
            format!("  {date}"),
            Style::default().fg(theme().text),
        )),
        Some(CustomFieldValue::Iteration { title, .. }) => Line::from(Span::styled(
            format!("  ⟳ {title}"),
            Style::default().fg(theme().text),
        )),
    }
}

type SelectEntry = (String, Option<ColumnColor>);

fn render_custom_field_select_edit(frame: &mut Frame, area: Rect, edit: &SidebarEditMode) {
    let title: &str;
    let entries: Vec<SelectEntry>;
    let cursor: usize;
    match edit {
        SidebarEditMode::CustomFieldSingleSelect {
            field_name,
            options,
            cursor: c,
            ..
        } => {
            title = field_name.as_str();
            entries = options
                .iter()
                .map(|o| (o.name.clone(), o.color.clone()))
                .collect();
            cursor = *c;
        }
        SidebarEditMode::CustomFieldIteration {
            field_name,
            iterations,
            cursor: c,
            ..
        } => {
            title = field_name.as_str();
            entries = iterations
                .iter()
                .map(|it| (format!("⟳ {}", it.title), None))
                .collect();
            cursor = *c;
        }
        _ => return,
    }
    let has_clear = true;

    let header_style = Style::default()
        .fg(theme().accent)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme().text_muted);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("{title}  (Enter: select, Esc: close)"),
        header_style,
    )));
    lines.push(Line::from(""));

    let total = entries.len() + if has_clear { 1 } else { 0 };
    for i in 0..total {
        let is_cursor = i == cursor;
        let marker = if is_cursor { "▶ " } else { "  " };
        let marker_span = Span::styled(
            marker.to_string(),
            if is_cursor {
                Style::default()
                    .fg(theme().accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                dim_style
            },
        );
        if i < entries.len() {
            let (name, color) = &entries[i];
            let body = if let Some(c) = color {
                Span::styled(
                    name.clone(),
                    Style::default()
                        .fg(theme().text_inverted)
                        .bg(column_color_to_tui(c)),
                )
            } else {
                Span::styled(name.clone(), Style::default().fg(theme().text))
            };
            lines.push(Line::from(vec![marker_span, body]));
        } else {
            lines.push(Line::from(vec![
                marker_span,
                Span::styled(
                    "(none / clear)".to_string(),
                    if is_cursor {
                        Style::default().fg(theme().text)
                    } else {
                        dim_style
                    },
                ),
            ]));
        }
    }

    let block = Block::default().padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_custom_field_text_edit(frame: &mut Frame, area: Rect, edit: &SidebarEditMode) {
    let (title, input, hint): (&str, &str, &str) = match edit {
        SidebarEditMode::CustomFieldText { field_name, input, .. } => {
            (field_name.as_str(), input.as_str(), "Enter: save, Esc: cancel")
        }
        SidebarEditMode::CustomFieldNumber { field_name, input, .. } => (
            field_name.as_str(),
            input.as_str(),
            "Enter: save (number), Esc: cancel",
        ),
        SidebarEditMode::CustomFieldDate { field_name, input, .. } => (
            field_name.as_str(),
            input.as_str(),
            "Enter: save (YYYY-MM-DD), Esc: cancel",
        ),
        _ => return,
    };

    let header_style = Style::default()
        .fg(theme().accent)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme().text_muted);

    let display = if input.is_empty() {
        "(empty — Enter で clear)"
    } else {
        input
    };
    let input_style = if input.is_empty() {
        dim_style
    } else {
        Style::default().fg(theme().text)
    };

    let lines = vec![
        Line::from(Span::styled(
            format!("{title}  ({hint})"),
            header_style,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", dim_style),
            Span::styled(display.to_string(), input_style),
            Span::styled("_", Style::default().fg(theme().accent)),
        ]),
    ];

    let block = Block::default().padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}
