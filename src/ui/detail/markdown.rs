use pulldown_cmark::{Event, Options as MdOptions, Parser, Tag, TagEnd};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::ui::theme::theme;

/// A line tagged as either table content (horizontally scrollable) or normal text (wrappable).
pub(super) struct TaggedLine {
    pub(super) line: Line<'static>,
    pub(super) is_table: bool,
}

pub(super) fn line_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum()
}

/// Wrap a line to fit within max_width, splitting at character boundaries.
pub(super) fn wrap_line(line: Line<'static>, max_width: usize) -> Vec<Line<'static>> {
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

        let style = span.style;
        let mut buf = String::new();
        let mut buf_width: usize = 0;

        for ch in span.content.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);

            if current_width + buf_width + ch_w > max_width
                && (buf_width > 0 || !current_spans.is_empty())
            {
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
pub(super) fn render_markdown(input: &str, tagged: &mut Vec<TaggedLine>) {
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
pub(super) fn render_table_lines(
    rows: &[Vec<String>],
    header_count: usize,
) -> Vec<Line<'static>> {
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
        render_markdown(
            "Before\n\n| H1 | H2 |\n|---|---|\n| a | b |\n\nAfter",
            &mut tagged,
        );
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
        let line = Line::from("あいうえお");
        let result = wrap_line(line, 6);
        assert_eq!(result.len(), 2);
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
