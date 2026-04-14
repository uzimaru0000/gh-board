use pulldown_cmark::{Event, Options as MdOptions, Parser, Tag, TagEnd};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::model::project::{CardType, IssueState, PrState};
use crate::model::state::{
    DetailPane, SIDEBAR_ASSIGNEES, SIDEBAR_DELETE, SIDEBAR_LABELS, SIDEBAR_MILESTONE,
    SIDEBAR_STATUS,
};
use crate::ui::card::parse_hex_color;
use crate::ui::theme::THEME;

/// A line tagged as either table content (horizontally scrollable) or normal text (wrappable).
struct TaggedLine {
    line: Line<'static>,
    is_table: bool,
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let card = match app.state.selected_card_ref() {
        Some(c) => c,
        None => return,
    };

    let popup = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup);

    // Fix: ポップアップ左境界をまたぐ全角文字をクリア
    if popup.x > 0 {
        let buf = frame.buffer_mut();
        for y in popup.top()..popup.bottom() {
            if let Some(cell) = buf.cell_mut((popup.x - 1, y)) {
                if cell.symbol().width() > 1 {
                    cell.reset();
                }
            }
        }
    }

    let (type_icon, type_color) = match &card.card_type {
        CardType::Issue { state } => match state {
            IssueState::Open => ("● ", THEME.green),
            IssueState::Closed => ("● ", THEME.purple),
        },
        CardType::PullRequest { state } => match state {
            PrState::Open => ("⑂ ", THEME.green),
            PrState::Closed => ("⑂ ", THEME.red),
            PrState::Merged => ("⑂ ", THEME.purple),
        },
        CardType::DraftIssue => ("○ ", THEME.text_dim),
    };

    let number_str = card
        .number
        .map(|n| format!("#{n} "))
        .unwrap_or_default();

    let block_title = format!(" {type_icon}{number_str}{} ", card.title);

    let sidebar_focused = app.state.detail_pane == DetailPane::Sidebar;
    let border_color = if sidebar_focused {
        THEME.border_unfocused
    } else {
        THEME.border_focused
    };

    let block = Block::default()
        .title(block_title)
        .title_style(
            Style::default()
                .fg(type_color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    // ── 2-column layout: content (left) + sidebar (right) + footer ──
    let vert_chunks =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let main_area = vert_chunks[0];
    let footer_area = vert_chunks[1];

    // サイドバー幅: 最小24、全体の30%上限
    let sidebar_width = (main_area.width as usize * 30 / 100).max(24).min(main_area.width as usize - 4) as u16;
    let horiz_chunks = Layout::horizontal([
        Constraint::Min(1),
        Constraint::Length(sidebar_width),
    ])
    .split(main_area);
    let left_area = horiz_chunks[0];
    let right_area = horiz_chunks[1];

    // ── Left pane: body + comments ──
    render_content_pane(frame, left_area, app, card);

    // ── Right pane: sidebar ──
    render_sidebar(frame, right_area, app);

    // ── Footer ──
    let hint_style = Style::default()
        .fg(THEME.text)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(THEME.text_muted);
    let footer = if app.state.sidebar_edit.is_some() {
        Line::from(vec![
            Span::styled("j/k", hint_style),
            Span::styled(":nav  ", desc_style),
            Span::styled("Enter/Space", hint_style),
            Span::styled(":toggle  ", desc_style),
            Span::styled("Esc", hint_style),
            Span::styled(":close", desc_style),
        ])
    } else if sidebar_focused {
        Line::from(vec![
            Span::styled("Tab", hint_style),
            Span::styled(":content  ", desc_style),
            Span::styled("j/k", hint_style),
            Span::styled(":nav  ", desc_style),
            Span::styled("Enter", hint_style),
            Span::styled(":select  ", desc_style),
            Span::styled("d", hint_style),
            Span::styled(":delete  ", desc_style),
            Span::styled("Esc", hint_style),
            Span::styled(":back", desc_style),
        ])
    } else {
        let is_draft = matches!(card.card_type, CardType::DraftIssue);
        let mut spans = vec![
            Span::styled("Tab", hint_style),
            Span::styled(":sidebar  ", desc_style),
            Span::styled("j/k", hint_style),
            Span::styled(":scroll  ", desc_style),
        ];
        if !is_draft {
            spans.extend([
                Span::styled("c", hint_style),
                Span::styled(":comment  ", desc_style),
                Span::styled("C", hint_style),
                Span::styled(":comments  ", desc_style),
            ]);
        }
        spans.extend([
            Span::styled("Esc/q", hint_style),
            Span::styled(":close", desc_style),
        ]);
        Line::from(spans)
    };
    frame.render_widget(footer, footer_area);
}

/// 左ペイン: 本文 + コメント
fn render_content_pane(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    card: &crate::model::project::Card,
) {
    let focused = app.state.detail_pane == DetailPane::Content;
    let border_color = if focused { THEME.border_focused } else { THEME.border_unfocused };

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(THEME.border_unfocused))
        .padding(Padding::horizontal(1));

    let content_inner = block.inner(area);
    frame.render_widget(block, area);

    if content_inner.height == 0 || content_inner.width == 0 {
        return;
    }

    let content_width = content_inner.width as usize;
    let mut tagged: Vec<TaggedLine> = Vec::new();

    let push_text = |tagged: &mut Vec<TaggedLine>, line: Line<'static>| {
        tagged.push(TaggedLine {
            line,
            is_table: false,
        });
    };

    // Body
    let body_text = card.body.as_deref().unwrap_or("");
    if body_text.is_empty() {
        push_text(
            &mut tagged,
            Line::from(Span::styled(
                "(No description)",
                Style::default().fg(THEME.text_muted),
            )),
        );
    } else {
        render_markdown(body_text, &mut tagged);
    }

    // Comments
    if !card.comments.is_empty() {
        let separator = Line::from(Span::styled(
            "─".repeat(content_width),
            Style::default().fg(THEME.text_muted),
        ));
        push_text(&mut tagged, Line::from(""));
        push_text(&mut tagged, separator);
        push_text(&mut tagged, Line::from(""));

        let comment_header = format!(" {} Comments ", card.comments.len());
        push_text(
            &mut tagged,
            Line::from(Span::styled(
                comment_header,
                Style::default()
                    .fg(THEME.accent)
                    .add_modifier(Modifier::BOLD),
            )),
        );
        push_text(&mut tagged, Line::from(""));

        for (i, comment) in card.comments.iter().enumerate() {
            let date_display = &comment.created_at[..10.min(comment.created_at.len())];
            push_text(
                &mut tagged,
                Line::from(vec![
                    Span::styled(
                        format!("@{}", comment.author),
                        Style::default()
                            .fg(THEME.yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {date_display}"),
                        Style::default().fg(THEME.text_muted),
                    ),
                ]),
            );

            render_markdown(&comment.body, &mut tagged);

            if i < card.comments.len() - 1 {
                push_text(&mut tagged, Line::from(""));
                push_text(
                    &mut tagged,
                    Line::from(Span::styled(
                        "· · ·",
                        Style::default().fg(THEME.text_muted),
                    )),
                );
                push_text(&mut tagged, Line::from(""));
            }
        }
    }

    // ── Wrap non-table lines ──
    let mut final_lines: Vec<TaggedLine> = Vec::new();
    for tl in tagged {
        if tl.is_table {
            final_lines.push(tl);
        } else {
            for wrapped in wrap_line(tl.line, content_width) {
                final_lines.push(TaggedLine {
                    line: wrapped,
                    is_table: false,
                });
            }
        }
    }

    // ── Scroll ──
    let content_height = content_inner.height as usize;
    let total_lines = final_lines.len();
    let max_scroll = total_lines.saturating_sub(content_height);
    let max_table_width = final_lines
        .iter()
        .filter(|tl| tl.is_table)
        .map(|tl| line_width(&tl.line))
        .max()
        .unwrap_or(0);
    let max_scroll_x = max_table_width.saturating_sub(content_width);

    app.state.detail_max_scroll.set(max_scroll);
    app.state.detail_max_scroll_x.set(max_scroll_x);

    let scroll = app.state.detail_scroll.min(max_scroll);
    let scroll_x = app.state.detail_scroll_x.min(max_scroll_x);

    // ── Render ──
    let _ = border_color; // フォーカス表示用に将来使用可能
    let visible = final_lines.into_iter().skip(scroll).take(content_height);

    for (i, tl) in visible.enumerate() {
        let line_rect = Rect {
            x: content_inner.x,
            y: content_inner.y + i as u16,
            width: content_inner.width,
            height: 1,
        };
        if tl.is_table && line_width(&tl.line) > content_width {
            let p = Paragraph::new(tl.line).scroll((0, scroll_x as u16));
            frame.render_widget(p, line_rect);
        } else {
            frame.render_widget(tl.line, line_rect);
        }
    }
}

/// 右ペイン: サイドバー (Status, Assignees, Labels, Delete)
fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let card = match app.state.selected_card_ref() {
        Some(c) => c,
        None => return,
    };

    // サイドバー編集モード (ラベル/アサイニー トグルリスト)
    if let Some(edit) = &app.state.sidebar_edit {
        render_sidebar_edit(frame, area, edit);
        return;
    }

    let focused = app.state.detail_pane == DetailPane::Sidebar;
    let selected = app.state.sidebar_selected;

    let header_style = Style::default()
        .fg(THEME.text)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(THEME.text_muted);
    let selected_marker = if focused { "▶ " } else { "  " };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // ── Status section ──
    let status_header_style = if focused && selected == SIDEBAR_STATUS {
        Style::default()
            .fg(THEME.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    lines.push(Line::from(Span::styled("Status", status_header_style)));

    let board = app.state.board.as_ref();
    let current_col_name = board
        .and_then(|b| b.columns.get(app.state.selected_column))
        .map(|c| c.name.as_str())
        .unwrap_or("?");

    if app.state.status_select_open {
        // ドロップダウン表示
        if let Some(board) = board {
            for (i, col) in board.columns.iter().enumerate() {
                if col.option_id.is_empty() {
                    continue; // "No Status" をスキップ
                }
                let is_cursor = i == app.state.status_select_cursor;
                let is_current = i == app.state.selected_column;
                let marker = if is_cursor { "▶ " } else { "  " };
                let style = if is_cursor {
                    Style::default()
                        .fg(THEME.accent)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(THEME.green)
                } else {
                    Style::default().fg(THEME.text)
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
                IssueState::Open => ("Open", THEME.green),
                IssueState::Closed => ("Closed", THEME.purple),
            },
            CardType::PullRequest { state } => match state {
                PrState::Open => ("Open", THEME.green),
                PrState::Closed => ("Closed", THEME.red),
                PrState::Merged => ("Merged", THEME.purple),
            },
            CardType::DraftIssue => ("Draft", THEME.text_dim),
        };
        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), dim_style),
            Span::styled(
                current_col_name.to_string(),
                Style::default().fg(THEME.text),
            ),
            Span::styled(
                format!(" ({state_label})"),
                Style::default().fg(state_color),
            ),
        ]));
    }
    lines.push(Line::from(""));

    // ── Assignees section ──
    let assignee_header_style = if focused && selected == SIDEBAR_ASSIGNEES {
        Style::default()
            .fg(THEME.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    lines.push(Line::from(Span::styled("Assignees", assignee_header_style)));
    if card.assignees.is_empty() {
        lines.push(Line::from(Span::styled("  --", dim_style)));
    } else {
        for assignee in &card.assignees {
            lines.push(Line::from(Span::styled(
                format!("  @{assignee}"),
                Style::default().fg(THEME.yellow),
            )));
        }
    }
    lines.push(Line::from(""));

    // ── Labels section ──
    let label_header_style = if focused && selected == SIDEBAR_LABELS {
        Style::default()
            .fg(THEME.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    lines.push(Line::from(Span::styled("Labels", label_header_style)));
    if card.labels.is_empty() {
        lines.push(Line::from(Span::styled("  --", dim_style)));
    } else {
        for label in &card.labels {
            let color = parse_hex_color(&label.color).unwrap_or(THEME.text_dim);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.name.clone(),
                    Style::default().fg(THEME.text_inverted).bg(color),
                ),
            ]));
        }
    }
    lines.push(Line::from(""));

    // ── Milestone section ──
    let milestone_header_style = if focused && selected == SIDEBAR_MILESTONE {
        Style::default()
            .fg(THEME.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        header_style
    };
    lines.push(Line::from(Span::styled("Milestone", milestone_header_style)));
    let milestone_text = card
        .milestone
        .as_deref()
        .unwrap_or("--");
    lines.push(Line::from(Span::styled(
        format!("  {milestone_text}"),
        if card.milestone.is_some() {
            Style::default().fg(THEME.text)
        } else {
            dim_style
        },
    )));
    lines.push(Line::from(""));

    let block = Block::default().padding(Padding::horizontal(1));
    let inner = block.inner(area);
    let btn_width = inner.width as usize;

    // ── Delete button ──
    let is_delete_focused = focused && selected == SIDEBAR_DELETE;
    let btn_bg = if is_delete_focused {
        THEME.red
    } else {
        THEME.border_unfocused
    };
    let edge_style = Style::default().fg(btn_bg);
    let fill_style = Style::default().fg(THEME.text).bg(btn_bg);
    let label = "Delete";
    let pad_total = btn_width.saturating_sub(label.len());
    let pad_left = pad_total / 2;
    let pad_right = pad_total - pad_left;
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

    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// サイドバー編集モードのトグルリスト描画
fn render_sidebar_edit(
    frame: &mut Frame,
    area: Rect,
    edit: &crate::model::state::SidebarEditMode,
) {
    use crate::model::state::SidebarEditMode;

    let (title, items, cursor) = match edit {
        SidebarEditMode::Labels { items, cursor } => ("Labels", items.as_slice(), *cursor),
        SidebarEditMode::Assignees { items, cursor } => ("Assignees", items.as_slice(), *cursor),
    };

    let header_style = Style::default()
        .fg(THEME.accent)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(THEME.text_muted);

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
                    .fg(THEME.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                dim_style
            },
        ));

        if let Some(color_hex) = &item.color {
            let color = parse_hex_color(color_hex).unwrap_or(THEME.text_dim);
            spans.push(Span::styled(
                item.name.clone(),
                Style::default().fg(THEME.text_inverted).bg(color),
            ));
        } else {
            spans.push(Span::styled(
                format!("@{}", item.name),
                if is_cursor {
                    Style::default().fg(THEME.text)
                } else {
                    Style::default().fg(THEME.yellow)
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

fn line_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum()
}

/// Wrap a line to fit within max_width, splitting at character boundaries.
fn wrap_line(line: Line<'static>, max_width: usize) -> Vec<Line<'static>> {
    if max_width == 0 {
        return vec![line];
    }

    let total: usize = line_width(&line);
    if total <= max_width {
        return vec![line];
    }

    let mut result: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_width: usize = 0;

    for span in line.spans {
        let span_width = UnicodeWidthStr::width(span.content.as_ref());

        if current_width + span_width <= max_width {
            current_spans.push(span);
            current_width += span_width;
            continue;
        }

        // Need to split this span character by character
        let style = span.style;
        let mut buf = String::new();
        let mut buf_width: usize = 0;

        for ch in span.content.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);

            if current_width + buf_width + ch_w > max_width && (buf_width > 0 || !current_spans.is_empty()) {
                if !buf.is_empty() {
                    current_spans.push(Span::styled(buf.clone(), style));
                }
                result.push(Line::from(std::mem::take(&mut current_spans)));
                current_width = 0;
                buf.clear();
                buf_width = 0;
            }

            buf.push(ch);
            buf_width += ch_w;
        }

        if !buf.is_empty() {
            current_spans.push(Span::styled(buf, style));
            current_width += buf_width;
        }
    }

    if !current_spans.is_empty() {
        result.push(Line::from(current_spans));
    }

    if result.is_empty() {
        result.push(Line::from(""));
    }

    result
}

/// Render markdown to tagged lines.
/// Non-table parts go through tui-markdown; tables are rendered directly as Lines.
fn render_markdown(input: &str, tagged: &mut Vec<TaggedLine>) {
    let mut opts = MdOptions::empty();
    opts.insert(MdOptions::ENABLE_TABLES);
    opts.insert(MdOptions::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(input, opts).into_offset_iter();

    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut header_count: usize = 0;
    let mut last_end: usize = 0;

    enum Segment {
        Markdown(String),
        Table(Vec<Line<'static>>),
    }
    let mut segments: Vec<Segment> = Vec::new();

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Table(_)) => {
                let before = &input[last_end..range.start];
                if !before.trim().is_empty() {
                    segments.push(Segment::Markdown(before.to_string()));
                }
                in_table = true;
                table_rows.clear();
                header_count = 0;
            }
            Event::End(TagEnd::Table) => {
                let table_lines = render_table_lines(&table_rows, header_count);
                segments.push(Segment::Table(table_lines));
                in_table = false;
                last_end = range.end;
            }
            Event::Start(Tag::TableHead) if in_table => {
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) if in_table => {
                table_rows.push(current_row.clone());
                header_count = table_rows.len();
            }
            Event::Start(Tag::TableRow) if in_table => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) if in_table => {
                table_rows.push(current_row.clone());
            }
            Event::Start(Tag::TableCell) if in_table => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) if in_table => {
                current_row.push(current_cell.clone());
            }
            Event::Text(ref text) if in_table => {
                current_cell.push_str(text);
            }
            Event::Code(ref code) if in_table => {
                current_cell.push('`');
                current_cell.push_str(code);
                current_cell.push('`');
            }
            Event::SoftBreak | Event::HardBreak if in_table => {
                current_cell.push(' ');
            }
            _ if in_table => {}
            _ => {}
        }
    }
    let remaining = &input[last_end..];
    if !remaining.trim().is_empty() {
        segments.push(Segment::Markdown(remaining.to_string()));
    }

    if segments.is_empty() {
        let rendered = tui_markdown::from_str(input);
        for line in rendered.into_iter() {
            tagged.push(TaggedLine {
                line: to_owned_line(line),
                is_table: false,
            });
        }
        return;
    }

    for segment in segments {
        match segment {
            Segment::Markdown(text) => {
                let rendered = tui_markdown::from_str(&text);
                for line in rendered.into_iter() {
                    tagged.push(TaggedLine {
                        line: to_owned_line(line),
                        is_table: false,
                    });
                }
            }
            Segment::Table(table_lines) => {
                for line in table_lines {
                    tagged.push(TaggedLine {
                        line,
                        is_table: true,
                    });
                }
            }
        }
    }
}

fn to_owned_line(line: Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .into_iter()
            .map(|span| Span::styled(span.content.into_owned(), span.style))
            .collect::<Vec<_>>(),
    )
}

/// Render table rows directly into ratatui Lines with box-drawing borders.
fn render_table_lines(rows: &[Vec<String>], header_count: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if rows.is_empty() {
        return lines;
    }

    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return lines;
    }

    let border_style = Style::default().fg(THEME.text_muted);
    let header_style = Style::default()
        .fg(THEME.text)
        .add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(THEME.text);

    let mut widths = vec![0usize; num_cols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(UnicodeWidthStr::width(cell.as_str()));
        }
    }
    for w in &mut widths {
        *w = (*w).max(3);
    }

    let h_border = |left: char, mid: char, right: char, fill: char| -> Line<'static> {
        let mut s = String::new();
        s.push(left);
        for (i, &w) in widths.iter().enumerate() {
            s.extend(std::iter::repeat_n(fill, w + 2));
            if i < num_cols - 1 {
                s.push(mid);
            }
        }
        s.push(right);
        Line::from(Span::styled(s, border_style))
    };

    lines.push(h_border('┌', '┬', '┐', '─'));

    for (row_idx, row) in rows.iter().enumerate() {
        let is_header_row = row_idx < header_count;
        let style = if is_header_row {
            header_style
        } else {
            cell_style
        };

        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled("│", border_style));
        for (i, &w) in widths.iter().enumerate() {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            let cell_width = UnicodeWidthStr::width(cell);
            let padding = w - cell_width;
            let content = format!(" {cell}{} ", " ".repeat(padding));
            spans.push(Span::styled(content, style));
            spans.push(Span::styled("│", border_style));
        }
        lines.push(Line::from(spans));

        if row_idx == header_count.saturating_sub(1) && header_count > 0 {
            lines.push(h_border('├', '╪', '┤', '═'));
        } else if row_idx < rows.len() - 1 {
            lines.push(h_border('├', '┼', '┤', '─'));
        }
    }

    lines.push(h_border('└', '┴', '┘', '─'));
    lines
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown_no_table() {
        let mut tagged = Vec::new();
        render_markdown("Hello **world**", &mut tagged);
        assert!(!tagged.is_empty());
        assert!(tagged.iter().all(|t| !t.is_table));
    }

    #[test]
    fn test_render_markdown_table_tagged() {
        let mut tagged = Vec::new();
        render_markdown("| A | B |\n|---|---|\n| 1 | 2 |", &mut tagged);
        assert!(tagged.iter().any(|t| t.is_table));
        let table_text: String = tagged
            .iter()
            .filter(|t| t.is_table)
            .flat_map(|t| t.line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(table_text.contains('┌'));
        assert!(table_text.contains('│'));
    }

    #[test]
    fn test_render_markdown_mixed() {
        let mut tagged = Vec::new();
        render_markdown("Before\n\n| H1 | H2 |\n|---|---|\n| a | b |\n\nAfter", &mut tagged);
        let has_table = tagged.iter().any(|t| t.is_table);
        let has_text = tagged.iter().any(|t| !t.is_table);
        assert!(has_table);
        assert!(has_text);
    }

    #[test]
    fn test_wrap_line_short() {
        let line = Line::from("short");
        let result = wrap_line(line, 80);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_wrap_line_long() {
        let line = Line::from("a".repeat(20));
        let result = wrap_line(line, 10);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_wrap_line_unicode() {
        // Each CJK char is 2 display-width
        let line = Line::from("あいうえお"); // 10 display width
        let result = wrap_line(line, 6);
        assert_eq!(result.len(), 2); // 6 + 4
    }

    #[test]
    fn test_render_table_lines_unicode_width() {
        let rows = vec![
            vec!["名前".to_string(), "Value".to_string()],
            vec!["テスト".to_string(), "OK".to_string()],
        ];
        let lines = render_table_lines(&rows, 1);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains('═'));
        assert!(text.contains("名前"));
        assert!(text.contains("テスト"));
    }
}
