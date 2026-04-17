mod markdown;
mod pr_status;
mod sidebar;

use markdown::{line_width, render_markdown, wrap_line, TaggedLine};
use sidebar::render_sidebar;

use unicode_width::UnicodeWidthStr;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::app_state::AppState;
use crate::model::project::{Card, CardType, IssueState, PrState, ReactionSummary};
use crate::model::state::DetailPane;
use crate::ui::layout::centered_rect_pct;
use crate::ui::scroll_fade::{draw_bottom_arrow, draw_left_arrow, draw_right_arrow, draw_top_arrow};
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let card = match app.state.current_detail_card() {
        Some(c) => c,
        None => return,
    };

    let popup = centered_rect_pct(80, 80, area);
    frame.render_widget(Clear, popup);

    // Fix: ポップアップ左境界をまたぐ全角文字をクリア
    if popup.x > 0 {
        let buf = frame.buffer_mut();
        for y in popup.top()..popup.bottom() {
            if let Some(cell) = buf.cell_mut((popup.x - 1, y))
                && cell.symbol().width() > 1
            {
                cell.reset();
            }
        }
    }

    let (type_icon, type_color) = match &card.card_type {
        CardType::Issue { state } => match state {
            IssueState::Open => ("\u{f41b} ", theme().green),
            IssueState::Closed => ("\u{f41d} ", theme().purple),
        },
        CardType::PullRequest { state } => match state {
            PrState::Open => ("\u{f407} ", theme().green),
            PrState::Closed => ("\u{f407} ", theme().red),
            PrState::Merged => ("\u{f407} ", theme().purple),
        },
        CardType::DraftIssue => ("\u{f404} ", theme().text_dim),
    };

    let number_str = card
        .number
        .map(|n| format!("#{n} "))
        .unwrap_or_default();

    let block_title = format!(" {type_icon}{number_str}{} ", card.title);

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
        .title_style(Style::default().fg(type_color).add_modifier(Modifier::BOLD))
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

    let vert_chunks =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let main_area = vert_chunks[0];
    let footer_area = vert_chunks[1];

    let sidebar_width = (main_area.width as usize * 30 / 100)
        .max(24)
        .min(main_area.width as usize - 4) as u16;
    let horiz_chunks = Layout::horizontal([
        Constraint::Min(1),
        Constraint::Length(sidebar_width),
    ])
    .split(main_area);
    let left_area = horiz_chunks[0];
    let right_area = horiz_chunks[1];

    render_content_pane(frame, left_area, app, card);
    render_sidebar(frame, right_area, app);

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
        spans.push(Span::styled(
            format!("{} {}  ", r.content.emoji(), r.count),
            style,
        ));
    }
    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
}

/// 左ペイン: 本文 + コメント
fn render_content_pane(frame: &mut Frame, area: Rect, app: &App, card: &Card) {
    let focused = app.state.detail_pane == DetailPane::Content;
    let border_color = if focused {
        theme().border_focused
    } else {
        theme().border_unfocused
    };

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

    if card.body.is_none() {
        use rattles::presets::prelude as presets;
        let spinner = presets::dots_circle().current_frame();
        let loading_text = format!("{spinner} Loading...");
        let loading_line = Line::from(Span::styled(
            loading_text,
            Style::default()
                .fg(theme().yellow)
                .add_modifier(Modifier::BOLD),
        ));
        let center_y = content_inner.y + content_inner.height / 2;
        let text_width = loading_line.width() as u16;
        let center_x = content_inner.x + content_inner.width.saturating_sub(text_width) / 2;
        let center_rect = Rect {
            x: center_x,
            y: center_y,
            width: text_width.min(content_inner.width),
            height: 1,
        };
        frame.render_widget(loading_line, center_rect);
        app.state.detail_max_scroll.set(0);
        app.state.detail_max_scroll_x.set(0);
        return;
    }

    match card.body.as_deref() {
        Some("") | None => {
            push_text(
                &mut tagged,
                Line::from(Span::styled(
                    "(No description)",
                    Style::default().fg(theme().text_muted),
                )),
            );
        }
        Some(body_text) => {
            render_markdown(body_text, &mut tagged);
        }
    }

    if !card.reactions.is_empty() {
        push_text(&mut tagged, Line::from(""));
        push_text(&mut tagged, reactions_line(&card.reactions));
    }

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

    let _ = border_color;

    let visible_lines: Vec<_> = final_lines
        .into_iter()
        .skip(scroll)
        .take(content_height)
        .collect();
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

    if visible_has_overflow_table {
        if scroll_x > 0 {
            draw_left_arrow(buf, area);
        }
        if scroll_x < max_scroll_x {
            draw_right_arrow(buf, area);
        }
    }
}
