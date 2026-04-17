use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};

use crate::app::App;
use crate::color::parse_hex_color;
use crate::model::project::{Board, Card, CardType, CustomFieldValue, FieldDefinition};
use crate::model::state::ViewMode;
use crate::ui::card::column_color_to_tui;
use crate::ui::statusline::loading_spinner_span;
use crate::ui::theme::theme;

const COL_WIDTH_NUMBER: u16 = 6;
const COL_WIDTH_STATUS: u16 = 14;
const COL_WIDTH_ASSIGNEES: u16 = 16;
const COL_WIDTH_LABELS: u16 = 20;
const COL_WIDTH_MILESTONE: u16 = 14;
const COL_WIDTH_CUSTOM: u16 = 14;

enum TableCol<'a> {
    Number,
    Title,
    Status,
    Assignees,
    Labels,
    Milestone,
    CustomField(&'a FieldDefinition),
}

fn build_columns(board: &Board) -> Vec<TableCol<'_>> {
    let mut cols: Vec<TableCol<'_>> = vec![
        TableCol::Number,
        TableCol::Title,
        TableCol::Status,
        TableCol::Assignees,
        TableCol::Labels,
        TableCol::Milestone,
    ];
    // grouping field と被る custom field は除外 (Status 列で表示済み)
    let grouping_field_id = board.grouping.field_id();
    for fd in &board.field_definitions {
        if Some(fd.id()) == grouping_field_id {
            continue;
        }
        cols.push(TableCol::CustomField(fd));
    }
    cols
}

fn col_constraint(c: &TableCol<'_>) -> Constraint {
    match c {
        TableCol::Number => Constraint::Length(COL_WIDTH_NUMBER),
        TableCol::Title => Constraint::Min(20),
        TableCol::Status => Constraint::Length(COL_WIDTH_STATUS),
        TableCol::Assignees => Constraint::Length(COL_WIDTH_ASSIGNEES),
        TableCol::Labels => Constraint::Length(COL_WIDTH_LABELS),
        TableCol::Milestone => Constraint::Length(COL_WIDTH_MILESTONE),
        TableCol::CustomField(_) => Constraint::Length(COL_WIDTH_CUSTOM),
    }
}

fn col_header<'a>(c: &'a TableCol<'a>) -> &'a str {
    match c {
        TableCol::Number => "#",
        TableCol::Title => "Title",
        TableCol::Status => "Status",
        TableCol::Assignees => "Assignees",
        TableCol::Labels => "Labels",
        TableCol::Milestone => "Milestone",
        TableCol::CustomField(fd) => fd.name(),
    }
}

fn type_marker(ct: &CardType) -> Span<'static> {
    match ct {
        CardType::Issue { .. } => Span::styled("\u{f41b}", Style::default().fg(theme().green)),
        CardType::PullRequest { .. } => {
            Span::styled("\u{f407}", Style::default().fg(theme().blue))
        }
        CardType::DraftIssue => Span::styled("\u{f404}", Style::default().fg(theme().text_dim)),
    }
}

fn cell_for<'a>(card: &'a Card, col: &TableCol<'_>, board_column_name: &'a str) -> Cell<'a> {
    match col {
        TableCol::Number => match card.number {
            Some(n) => Cell::from(format!("#{n}")),
            None => Cell::from(Span::styled("-", Style::default().fg(theme().text_dim))),
        },
        TableCol::Title => {
            let mut spans = vec![type_marker(&card.card_type), Span::raw(" ")];
            if card.parent_issue.is_some() {
                spans.push(Span::styled("↳ ", Style::default().fg(theme().text_dim)));
            }
            if let Some(summary) = &card.sub_issues_summary
                && summary.total > 0
            {
                let color = if summary.completed >= summary.total {
                    theme().green
                } else {
                    theme().blue
                };
                spans.push(Span::styled(
                    format!("[{}/{}] ", summary.completed, summary.total),
                    Style::default().fg(color),
                ));
            }
            spans.push(Span::raw(card.title.as_str()));
            Cell::from(Line::from(spans))
        }
        TableCol::Status => {
            // grouping field の値を card.custom_fields から検索。なければカラム名で代用。
            Cell::from(Span::styled(
                board_column_name.to_string(),
                Style::default().fg(theme().text_dim),
            ))
        }
        TableCol::Assignees => {
            if card.assignees.is_empty() {
                Cell::from("")
            } else {
                Cell::from(Span::styled(
                    card.assignees
                        .iter()
                        .map(|a| format!("@{a}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                    Style::default().fg(theme().yellow),
                ))
            }
        }
        TableCol::Labels => {
            if card.labels.is_empty() {
                Cell::from("")
            } else {
                let spans: Vec<Span> = card
                    .labels
                    .iter()
                    .enumerate()
                    .flat_map(|(i, label)| {
                        let bg = parse_hex_color(&label.color).unwrap_or(theme().text_dim);
                        let mut s = vec![Span::styled(
                            label.name.clone(),
                            Style::default().fg(theme().text_inverted).bg(bg),
                        )];
                        if i < card.labels.len() - 1 {
                            s.push(Span::raw(" "));
                        }
                        s
                    })
                    .collect();
                Cell::from(Line::from(spans))
            }
        }
        TableCol::Milestone => match &card.milestone {
            Some(m) => Cell::from(Span::styled(
                m.clone(),
                Style::default().fg(theme().text_muted),
            )),
            None => Cell::from(""),
        },
        TableCol::CustomField(fd) => {
            let value = card.custom_fields.iter().find(|v| v.field_id() == fd.id());
            match value {
                Some(CustomFieldValue::SingleSelect { name, color, .. }) => {
                    let bg = color
                        .as_ref()
                        .map(column_color_to_tui)
                        .unwrap_or(theme().border_unfocused);
                    Cell::from(Span::styled(
                        name.clone(),
                        Style::default().fg(theme().text_inverted).bg(bg),
                    ))
                }
                Some(CustomFieldValue::Number { number, .. }) => {
                    let text = if number.fract() == 0.0 && number.abs() < 1e16 {
                        format!("{}", *number as i64)
                    } else {
                        format!("{number}")
                    };
                    Cell::from(text)
                }
                Some(CustomFieldValue::Text { text, .. }) => Cell::from(text.clone()),
                Some(CustomFieldValue::Date { date, .. }) => Cell::from(date.clone()),
                Some(CustomFieldValue::Iteration { title, .. }) => Cell::from(format!("⟳ {title}")),
                None => Cell::from(""),
            }
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let board = match &app.state.board {
        Some(b) => b,
        None => return,
    };
    if board.columns.is_empty() {
        return;
    }

    let cols = build_columns(board);
    let constraints: Vec<Constraint> = cols.iter().map(col_constraint).collect();

    let rows: Vec<Row> = app
        .state
        .table_rows()
        .iter()
        .map(|&(col_idx, card_idx)| {
            let column = &board.columns[col_idx];
            let card = &column.cards[card_idx];
            Row::new(
                cols.iter()
                    .map(|c| cell_for(card, c, &column.name))
                    .collect::<Vec<_>>(),
            )
            .height(1)
        })
        .collect();

    let total_rows = rows.len();
    let header_cells: Vec<Cell> = cols
        .iter()
        .map(|c| {
            Cell::from(Span::styled(
                col_header(c).to_string(),
                Style::default()
                    .fg(theme().text_dim)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    let mut title_spans = vec![Span::from(format!(
        " {} ({}) ",
        board.project_title, total_rows
    ))];
    if let Some(spinner) = loading_spinner_span(&app.state.loading) {
        title_spans.push(spinner);
    }
    let block = Block::default()
        .title(Line::from(title_spans))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().border_focused));

    let grabbing = app.state.mode == ViewMode::CardGrab;
    let highlight_style = if grabbing {
        Style::default()
            .bg(theme().yellow)
            .fg(theme().text_inverted)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .bg(theme().accent)
            .fg(theme().text_inverted)
            .add_modifier(Modifier::BOLD)
    };
    let highlight_symbol = if grabbing { "▶ " } else { "  " };

    let table = Table::new(rows, constraints)
        .header(header)
        .row_highlight_style(highlight_style)
        .highlight_symbol(highlight_symbol)
        .column_spacing(1)
        .block(block);

    let mut state = TableState::default();
    let selected = if total_rows == 0 {
        None
    } else {
        Some(app.state.table_selected_row.min(total_rows - 1))
    };
    state.select(selected);

    frame.render_stateful_widget(table, area, &mut state);
}
