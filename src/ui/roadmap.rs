use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};
use unicode_width::UnicodeWidthChar;

use crate::app::App;
use crate::model::project::{Card, CardType, CustomFieldValue, IterationOption};
use crate::model::roadmap::{TimelineSegment, roadmap_timeline, today_utc};
use crate::ui::statusline::loading_spinner_span;
use crate::ui::theme::theme;

const LEFT_COL_WIDTH: u16 = 40;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let board = match &app.state.board {
        Some(b) => b,
        None => return,
    };
    if board.columns.is_empty() {
        return;
    }
    let (iter_field_id, _iter_field_name, iterations) = match board.iteration_field() {
        Some(x) => x,
        None => return,
    };

    let rows_count = app.state.roadmap_rows().len();
    let mut title_spans = vec![Span::from(format!(
        " {} Roadmap ({}) ",
        board.project_title, rows_count
    ))];
    if let Some(spinner) = loading_spinner_span(&app.state.loading) {
        title_spans.push(spinner);
    }
    let block = Block::default()
        .title(Line::from(title_spans))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().border_focused));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Timeline 領域の幅 (左ペイン + 1 スペース + 右ペイン = inner.width)
    let timeline_width = inner.width.saturating_sub(LEFT_COL_WIDTH + 1);
    if timeline_width == 0 {
        return;
    }

    let today = today_utc();
    let segments = roadmap_timeline(iterations, today, timeline_width);
    if segments.is_empty() {
        return;
    }

    let iter_field_id = iter_field_id.to_string();
    let rows_data = app.state.roadmap_rows();
    let all_iterations: &[IterationOption] = iterations;

    // ヘッダー: 左=Title, 右=iteration 名を start_col/width に配置
    let header = Row::new(vec![
        Cell::from(Span::styled(
            "Title",
            Style::default()
                .fg(theme().text_dim)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(timeline_header_line(&segments, timeline_width)),
    ])
    .height(1);

    // 各カード行
    let rows: Vec<Row> = rows_data
        .iter()
        .map(|&(col_idx, card_idx)| {
            let card = &board.columns[col_idx].cards[card_idx];
            let title_cell = Cell::from(card_title_line(card));
            let timeline_cell = Cell::from(card_timeline_line(
                card,
                &iter_field_id,
                all_iterations,
                &segments,
                timeline_width,
            ));
            Row::new(vec![title_cell, timeline_cell]).height(1)
        })
        .collect();

    let total_rows = rows.len();
    let constraints = [
        Constraint::Length(LEFT_COL_WIDTH),
        Constraint::Min(timeline_width),
    ];

    let table = Table::new(rows, constraints)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(theme().accent)
                .fg(theme().text_inverted)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("  ")
        .column_spacing(1);

    let mut state = TableState::default();
    let selected = if total_rows == 0 {
        None
    } else {
        Some(app.state.roadmap_selected_row.min(total_rows - 1))
    };
    state.select(selected);

    frame.render_stateful_widget(table, inner, &mut state);
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

fn card_title_line(card: &Card) -> Line<'_> {
    let mut spans = vec![type_marker(&card.card_type), Span::raw(" ")];
    if card.parent_issue.is_some() {
        spans.push(Span::styled("↳ ", Style::default().fg(theme().text_dim)));
    }
    spans.push(Span::raw(card.title.as_str()));
    Line::from(spans)
}

fn timeline_header_line<'a>(segments: &'a [TimelineSegment], total_width: u16) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = Vec::with_capacity(segments.len() * 2);
    let mut cursor: u16 = 0;
    for seg in segments {
        if seg.start_col > cursor {
            spans.push(Span::raw(" ".repeat((seg.start_col - cursor) as usize)));
            cursor = seg.start_col;
        }
        let w = seg.width as usize;
        let label = truncate_pad(&seg.title, w);
        let style = if seg.contains_today {
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD)
        } else if seg.completed {
            Style::default().fg(theme().text_dim)
        } else {
            Style::default()
                .fg(theme().text_muted)
                .add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(label, style));
        cursor = cursor.saturating_add(seg.width);
    }
    if cursor < total_width {
        spans.push(Span::raw(" ".repeat((total_width - cursor) as usize)));
    }
    Line::from(spans)
}

fn card_timeline_line<'a>(
    card: &Card,
    iter_field_id: &str,
    all_iterations: &[IterationOption],
    segments: &'a [TimelineSegment],
    total_width: u16,
) -> Line<'a> {
    let iteration_id = card.custom_fields.iter().find_map(|fv| match fv {
        CustomFieldValue::Iteration {
            field_id,
            iteration_id,
            ..
        } if field_id == iter_field_id => Some(iteration_id.clone()),
        _ => None,
    });

    let Some(iteration_id) = iteration_id else {
        return unscheduled_line(total_width);
    };

    let Some((idx, seg)) = segments
        .iter()
        .enumerate()
        .find(|(_, s)| s.iteration_id == iteration_id)
    else {
        // 表示範囲外の iteration。start_date を参照して方向矢印を出す
        return out_of_range_line(all_iterations, segments, &iteration_id, total_width);
    };

    let color = palette_color(idx);
    let bar = build_bar(seg.width as usize);

    let mut spans: Vec<Span<'a>> = Vec::new();
    if seg.start_col > 0 {
        spans.push(Span::raw(" ".repeat(seg.start_col as usize)));
    }
    spans.push(Span::styled(bar, Style::default().fg(color)));
    let filled = seg.start_col.saturating_add(seg.width);
    if filled < total_width {
        spans.push(Span::raw(" ".repeat((total_width - filled) as usize)));
    }
    Line::from(spans)
}

fn build_bar(width: usize) -> String {
    match width {
        0 => String::new(),
        1 => "█".to_string(),
        2 => "▐▌".to_string(),
        n => {
            let mut s = String::with_capacity(n * 3);
            s.push('▐');
            for _ in 0..(n - 2) {
                s.push('█');
            }
            s.push('▌');
            s
        }
    }
}

fn unscheduled_line<'a>(total_width: u16) -> Line<'a> {
    let placeholder = "- not scheduled -";
    let padded = truncate_pad(placeholder, total_width as usize);
    Line::from(Span::styled(
        padded,
        Style::default().fg(theme().text_muted),
    ))
}

/// 表示ウィンドウ外の iteration を指す矢印マーカーを描画する。
/// card の iteration start_date が window の先頭より前なら ◀、末尾より後なら ▶。
fn out_of_range_line<'a>(
    all_iterations: &[IterationOption],
    segments: &'a [TimelineSegment],
    iteration_id: &str,
    total_width: u16,
) -> Line<'a> {
    let card_date = all_iterations
        .iter()
        .find(|it| it.id == iteration_id)
        .map(|it| it.start_date.as_str())
        .unwrap_or("");
    let first_date = segments.first().map(|s| s.start_date.as_str()).unwrap_or("");
    let direction = if !card_date.is_empty() && card_date < first_date {
        "◀"
    } else {
        "▶"
    };
    let padded = truncate_pad(direction, total_width as usize);
    Line::from(Span::styled(
        padded,
        Style::default().fg(theme().text_dim),
    ))
}

fn truncate_pad(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut out = String::with_capacity(width);
    let mut used = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > width {
            break;
        }
        out.push(ch);
        used += cw;
    }
    while used < width {
        out.push(' ');
        used += 1;
    }
    out
}

fn palette_color(idx: usize) -> ratatui::style::Color {
    let palette = [
        theme().blue,
        theme().green,
        theme().orange,
        theme().purple,
        theme().pink,
    ];
    palette[idx % palette.len()]
}
