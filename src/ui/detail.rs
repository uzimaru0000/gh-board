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
use crate::app_state::AppState;
use crate::model::project::{
    Card, CardType, CiStatus, ColumnColor, CustomFieldValue, IssueState, PrState, ReactionSummary,
    ReviewDecision,
};
use crate::model::state::{
    DetailPane, SIDEBAR_ASSIGNEES, SIDEBAR_LABELS, SIDEBAR_MILESTONE, SIDEBAR_STATUS,
};
use crate::ui::card::parse_hex_color;
use crate::ui::scroll_fade::{draw_bottom_arrow, draw_left_arrow, draw_right_arrow, draw_top_arrow};
use crate::ui::theme::theme;

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
            if let Some(cell) = buf.cell_mut((popup.x - 1, y))
                && cell.symbol().width() > 1 {
                    cell.reset();
            }
        }
    }

    let (type_icon, type_color) = match &card.card_type {
        CardType::Issue { state } => match state {
            // nf-oct-issue_opened / nf-oct-issue_closed
            IssueState::Open => ("\u{f41b} ", theme().green),
            IssueState::Closed => ("\u{f41d} ", theme().purple),
        },
        CardType::PullRequest { state } => match state {
            // nf-oct-git_pull_request
            PrState::Open => ("\u{f407} ", theme().green),
            PrState::Closed => ("\u{f407} ", theme().red),
            PrState::Merged => ("\u{f407} ", theme().purple),
        },
        // nf-oct-note
        CardType::DraftIssue => ("\u{f404} ", theme().text_dim),
    };

    let number_str = card
        .number
        .map(|n| format!("#{n} "))
        .unwrap_or_default();

    let block_title = format!(" {type_icon}{number_str}{} ", card.title);

    // 縦/横スクロール量のカウンタ (前フレームの Cell 値を使用)
    let detail_max_scroll = app.state.detail_max_scroll.get();
    let detail_max_scroll_x = app.state.detail_max_scroll_x.get();
    let scroll_counter = {
        let mut parts: Vec<String> = Vec::new();
        if detail_max_scroll > 0 {
            let s = app.state.detail_scroll.min(detail_max_scroll);
            parts.push(format!("↕ {}/{}", s + 1, detail_max_scroll + 1));
        }
        if detail_max_scroll_x > 0 {
            let sx = app.state.detail_scroll_x.min(detail_max_scroll_x);
            parts.push(format!("↔ {}/{}", sx + 1, detail_max_scroll_x + 1));
        }
        if parts.is_empty() {
            None
        } else {
            Some(format!(" {} ", parts.join("  ")))
        }
    };

    let sidebar_focused = app.state.detail_pane == DetailPane::Sidebar;
    let border_color = if sidebar_focused {
        theme().border_unfocused
    } else {
        theme().border_focused
    };

    let mut block = Block::default()
        .title(block_title)
        .title_style(
            Style::default()
                .fg(type_color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    if let Some(counter) = scroll_counter {
        block = block.title_top(
            Line::from(Span::styled(
                counter,
                Style::default()
                    .fg(theme().accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .right_aligned(),
        );
    }

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
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme().text_muted);
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
                Span::styled("r", hint_style),
                Span::styled(":react  ", desc_style),
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

/// リアクション集計を1行で表す。count > 0 のもののみ表示。
pub(crate) fn reactions_line(reactions: &[ReactionSummary]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for r in reactions.iter().filter(|r| r.count > 0) {
        let reacted = r.viewer_has_reacted;
        let style = if reacted {
            Style::default()
                .fg(theme().green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme().text_muted)
        };
        spans.push(Span::styled(format!("{} {}  ", r.content.emoji(), r.count), style));
    }
    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
}

/// 左ペイン: 本文 + コメント
fn render_content_pane(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    card: &crate::model::project::Card,
) {
    let focused = app.state.detail_pane == DetailPane::Content;
    let border_color = if focused { theme().border_focused } else { theme().border_unfocused };

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme().border_unfocused))
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
                Style::default().fg(theme().text_muted),
            )),
        );
    } else {
        render_markdown(body_text, &mut tagged);
    }

    // Body reactions (if any)
    if !card.reactions.is_empty() {
        push_text(&mut tagged, Line::from(""));
        push_text(&mut tagged, reactions_line(&card.reactions));
    }

    // Comments
    if !card.comments.is_empty() {
        let separator = Line::from(Span::styled(
            "─".repeat(content_width),
            Style::default().fg(theme().text_muted),
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
                    .fg(theme().accent)
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
                            .fg(theme().yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {date_display}"),
                        Style::default().fg(theme().text_muted),
                    ),
                ]),
            );

            render_markdown(&comment.body, &mut tagged);

            if !comment.reactions.is_empty() {
                push_text(&mut tagged, reactions_line(&comment.reactions));
            }

            if i < card.comments.len() - 1 {
                push_text(&mut tagged, Line::from(""));
                push_text(
                    &mut tagged,
                    Line::from(Span::styled(
                        "· · ·",
                        Style::default().fg(theme().text_muted),
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

    // 表示中に各行がどの幅を占めるかを取得しておく (横フェード判定用)
    let visible_lines: Vec<_> = final_lines.into_iter().skip(scroll).take(content_height).collect();
    let visible_has_overflow_table = visible_lines
        .iter()
        .any(|tl| tl.is_table && line_width(&tl.line) > content_width);

    for (i, tl) in visible_lines.into_iter().enumerate() {
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

    // 矢印: 上下方向 (popup のボーダー中央に描画)
    let has_above = scroll > 0;
    let has_below = AppState::should_show_scrollbar(total_lines, content_height)
        && scroll + content_height < total_lines;
    let buf = frame.buffer_mut();
    if has_above {
        draw_top_arrow(buf, area);
    }
    if has_below {
        draw_bottom_arrow(buf, area);
    }

    // 矢印: 横方向 (表示中のテーブル行が領域を超えている場合のみ)
    if visible_has_overflow_table {
        if scroll_x > 0 {
            draw_left_arrow(buf, area);
        }
        if scroll_x < max_scroll_x {
            draw_right_arrow(buf, area);
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
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme().text_muted);
    let selected_marker = if focused { "▶ " } else { "  " };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // ── Status section ──
    let status_header_style = if focused && selected == SIDEBAR_STATUS {
        Style::default()
            .fg(theme().accent)
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
    lines.push(Line::from(Span::styled("Milestone", milestone_header_style)));
    let milestone_text = card
        .milestone
        .as_deref()
        .unwrap_or("--");
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
    let field_defs = app
        .state
        .board
        .as_ref()
        .map(|b| b.field_definitions.as_slice())
        .unwrap_or(&[]);
    for (i, field) in field_defs.iter().enumerate() {
        let sidebar_idx = 4 + i;
        let header = if focused && selected == sidebar_idx {
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD)
        } else {
            header_style
        };
        lines.push(Line::from(Span::styled(field.name().to_string(), header)));
        let current = card
            .custom_fields
            .iter()
            .find(|v| v.field_id() == field.id());
        lines.push(render_custom_field_value_line(current));
        lines.push(Line::from(""));
    }

    let block = Block::default().padding(Padding::horizontal(1));
    let inner = block.inner(area);
    let btn_width = inner.width as usize;

    // ── Delete button ──
    let delete_idx = app.state.sidebar_delete_index();
    let is_delete_focused = focused && selected == delete_idx;
    let btn_bg = if is_delete_focused {
        theme().red
    } else {
        theme().border_unfocused
    };
    let edge_style = Style::default().fg(btn_bg);
    let fill_style = Style::default().fg(theme().text).bg(btn_bg);
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

fn column_color_to_tui(color: &ColumnColor) -> ratatui::style::Color {
    use ratatui::style::Color;
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

fn render_custom_field_select_edit(
    frame: &mut Frame,
    area: Rect,
    edit: &crate::model::state::SidebarEditMode,
) {
    use crate::model::state::SidebarEditMode;
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
            // Clear ("None") row
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

fn render_custom_field_text_edit(
    frame: &mut Frame,
    area: Rect,
    edit: &crate::model::state::SidebarEditMode,
) {
    use crate::model::state::SidebarEditMode;
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

    let display = if input.is_empty() { "(empty — Enter で clear)" } else { input };
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

    let border_style = Style::default().fg(theme().text_muted);
    let header_style = Style::default()
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(theme().text);

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

fn pr_status_lines(card: &Card) -> Vec<Line<'static>> {
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

    // If PR but no info at all, show a hint
    if out.is_empty() {
        out.push(Line::from(Span::styled("  CI/Review: --".to_string(), dim_style)));
    }

    out
}

fn linked_prs_lines(card: &Card) -> Vec<Line<'static>> {
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
                // nf-oct-git_pull_request
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

    use crate::model::project::{LinkedPr, PrStatus};

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

    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
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
        assert_eq!(lines.len(), 5); // CI, Review, Reviewers header, alice, bob
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
