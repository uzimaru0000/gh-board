use pulldown_cmark::{Event, Options as MdOptions, Parser, Tag, TagEnd};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::model::project::{CardType, IssueState, PrState};
use crate::ui::card::parse_hex_color;

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

    let (type_icon, type_color) = match &card.card_type {
        CardType::Issue { state } => match state {
            IssueState::Open => ("● ", Color::Green),
            IssueState::Closed => ("● ", Color::Magenta),
        },
        CardType::PullRequest { state } => match state {
            PrState::Open => ("⑂ ", Color::Green),
            PrState::Closed => ("⑂ ", Color::Red),
            PrState::Merged => ("⑂ ", Color::Magenta),
        },
        CardType::DraftIssue => ("○ ", Color::Gray),
    };

    let number_str = card
        .number
        .map(|n| format!("#{n} "))
        .unwrap_or_default();

    let block_title = format!(" {type_icon}{number_str}{} ", card.title);

    let block = Block::default()
        .title(block_title)
        .title_style(
            Style::default()
                .fg(type_color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 || inner.width == 0 {
        return;
    }

    // Split inner area: scrollable content + fixed footer
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let content_area = chunks[0];
    let footer_area = chunks[1];
    let content_width = content_area.width as usize;

    // ── Build tagged lines ──
    let mut tagged: Vec<TaggedLine> = Vec::new();

    // Helper to push a non-table line
    let push_text = |tagged: &mut Vec<TaggedLine>, line: Line<'static>| {
        tagged.push(TaggedLine {
            line,
            is_table: false,
        });
    };

    // State line
    let (state_label, state_color) = match &card.card_type {
        CardType::Issue { state } => match state {
            IssueState::Open => ("Open", Color::Green),
            IssueState::Closed => ("Closed", Color::Magenta),
        },
        CardType::PullRequest { state } => match state {
            PrState::Open => ("Open", Color::Green),
            PrState::Closed => ("Closed", Color::Red),
            PrState::Merged => ("Merged", Color::Magenta),
        },
        CardType::DraftIssue => ("Draft", Color::Gray),
    };
    push_text(
        &mut tagged,
        Line::from(Span::styled(
            state_label,
            Style::default()
                .fg(state_color)
                .add_modifier(Modifier::BOLD),
        )),
    );

    // Assignees
    if !card.assignees.is_empty() {
        let text = card
            .assignees
            .iter()
            .map(|a| format!("@{a}"))
            .collect::<Vec<_>>()
            .join(" ");
        push_text(
            &mut tagged,
            Line::from(Span::styled(text, Style::default().fg(Color::Yellow))),
        );
    }

    // Labels
    if !card.labels.is_empty() {
        let spans: Vec<Span> = card
            .labels
            .iter()
            .enumerate()
            .flat_map(|(i, label)| {
                let color = parse_hex_color(&label.color).unwrap_or(Color::Gray);
                let mut spans = vec![Span::styled(
                    label.name.clone(),
                    Style::default().fg(Color::Black).bg(color),
                )];
                if i < card.labels.len() - 1 {
                    spans.push(Span::raw(" "));
                }
                spans
            })
            .collect();
        push_text(&mut tagged, Line::from(spans));
    }

    // Separator
    let separator = Line::from(Span::styled(
        "─".repeat(content_width),
        Style::default().fg(Color::DarkGray),
    ));
    push_text(&mut tagged, Line::from(""));
    push_text(&mut tagged, separator.clone());
    push_text(&mut tagged, Line::from(""));

    // Body (markdown rendered)
    let body_text = card.body.as_deref().unwrap_or("");

    if body_text.is_empty() {
        push_text(
            &mut tagged,
            Line::from(Span::styled(
                "(No description)",
                Style::default().fg(Color::DarkGray),
            )),
        );
    } else {
        render_markdown(body_text, &mut tagged);
    }

    // Comments
    if !card.comments.is_empty() {
        push_text(&mut tagged, Line::from(""));
        push_text(&mut tagged, separator);
        push_text(&mut tagged, Line::from(""));

        let comment_header = format!(" {} Comments ", card.comments.len());
        push_text(
            &mut tagged,
            Line::from(Span::styled(
                comment_header,
                Style::default()
                    .fg(Color::Cyan)
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
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {date_display}"),
                        Style::default().fg(Color::DarkGray),
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
                        Style::default().fg(Color::DarkGray),
                    )),
                );
                push_text(&mut tagged, Line::from(""));
            }
        }
    }

    // ── Process lines: wrap non-table, keep table ──
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

    // ── Compute & store scroll limits ──
    let content_height = content_area.height as usize;
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

    // ── Render line by line ──
    let visible = final_lines
        .into_iter()
        .skip(scroll)
        .take(content_height);

    for (i, tl) in visible.enumerate() {
        let line_rect = Rect {
            x: content_area.x,
            y: content_area.y + i as u16,
            width: content_area.width,
            height: 1,
        };
        if tl.is_table && line_width(&tl.line) > content_width {
            let p = Paragraph::new(tl.line).scroll((0, scroll_x as u16));
            frame.render_widget(p, line_rect);
        } else {
            frame.render_widget(tl.line, line_rect);
        }
    }

    // ── Fixed footer ──
    let hint_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::DarkGray);
    let footer = Line::from(vec![
        Span::styled("Esc/q", hint_style),
        Span::styled(":close  ", desc_style),
        Span::styled("Enter/o", hint_style),
        Span::styled(":open in browser  ", desc_style),
        Span::styled("j/k", hint_style),
        Span::styled(":scroll  ", desc_style),
        Span::styled("h/l", hint_style),
        Span::styled(":table scroll", desc_style),
    ]);
    frame.render_widget(footer, footer_area);
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

    let border_style = Style::default().fg(Color::DarkGray);
    let header_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(Color::White);

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
