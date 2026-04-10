use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::model::state::{EditCardField, EditCardState};

pub fn render(frame: &mut Frame, area: Rect, state: &EditCardState) {
    let popup = centered_rect(60, 14, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Edit Card ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let label_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let active_label = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let input_style = Style::default().fg(Color::White);
    let hint_style = Style::default().fg(Color::DarkGray);

    // Layout: Title box (3 lines) + gap + Body (2 lines) + gap + hints
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title (box)
        Constraint::Length(1), // gap
        Constraint::Length(2), // Body
        Constraint::Min(0),   // gap + hints
    ])
    .split(inner);

    // --- Title field (Box) ---
    let title_is_active = state.focused_field == EditCardField::Title;
    let title_border_style = if title_is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title_label_style = if title_is_active {
        active_label
    } else {
        label_style
    };

    let title_block = Block::default()
        .title(Span::styled(" Title ", title_label_style))
        .borders(Borders::ALL)
        .border_style(title_border_style);

    let title_inner = title_block.inner(chunks[0]);
    frame.render_widget(title_block, chunks[0]);

    let title_line = render_title_content(
        &state.title_input,
        state.title_cursor,
        title_is_active,
        input_style,
    );
    frame.render_widget(Paragraph::new(title_line), title_inner);

    // --- Body field ---
    let body_label_style = if state.focused_field == EditCardField::Body {
        active_label
    } else {
        label_style
    };
    render_body_field(
        frame,
        chunks[2],
        &state.body_input,
        state.focused_field == EditCardField::Body,
        body_label_style,
        hint_style,
    );

    // --- Hints ---
    let hint_area = chunks[3];
    if hint_area.height >= 2 {
        let hint_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", hint_style),
            Span::styled(":switch  ", hint_style),
            Span::styled("C-s", hint_style),
            Span::styled(":submit  ", hint_style),
            Span::styled("Esc", hint_style),
            Span::styled(":cancel", hint_style),
        ]);
        let hint_rect = Rect {
            x: hint_area.x,
            y: hint_area.y + hint_area.height.saturating_sub(1),
            width: hint_area.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(hint_line), hint_rect);
    }
}

fn render_title_content(
    input: &str,
    cursor_pos: usize,
    is_active: bool,
    input_style: Style,
) -> Line<'static> {
    if is_active {
        let (before, after) = input.split_at(cursor_pos);
        let cursor_char = after.chars().next().unwrap_or(' ');
        let rest = if after.is_empty() {
            String::new()
        } else {
            after[cursor_char.len_utf8()..].to_string()
        };

        Line::from(vec![
            Span::styled(before.to_string(), input_style),
            Span::styled(
                cursor_char.to_string(),
                Style::default().fg(Color::Black).bg(Color::White),
            ),
            Span::styled(rest, input_style),
        ])
    } else {
        Line::from(Span::styled(input.to_string(), input_style))
    }
}

fn render_body_field(
    frame: &mut Frame,
    area: Rect,
    body: &str,
    is_active: bool,
    label_style: Style,
    hint_style: Style,
) {
    let label_line = Line::from(Span::styled("  Body:", label_style));

    let value_line = if body.is_empty() {
        let msg = if is_active {
            "  (press Enter to edit in $EDITOR)"
        } else {
            "  (empty)"
        };
        Line::from(Span::styled(msg, hint_style))
    } else {
        let preview: String = body.lines().next().unwrap_or("").chars().take(50).collect();
        let suffix = if body.lines().count() > 1
            || preview.len() < body.lines().next().unwrap_or("").len()
        {
            "..."
        } else {
            ""
        };
        let text = format!("  {preview}{suffix}");
        if is_active {
            Line::from(vec![
                Span::styled(text, Style::default().fg(Color::White)),
                Span::styled("  (Enter to edit)", hint_style),
            ])
        } else {
            Line::from(Span::styled(text, Style::default().fg(Color::White)))
        }
    };

    frame.render_widget(Paragraph::new(vec![label_line, value_line]), area);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
