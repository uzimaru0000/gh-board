use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::model::state::{CreateCardField, CreateCardState};

pub fn render(frame: &mut Frame, area: Rect, state: &CreateCardState) {
    let popup = centered_rect(60, 14, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" New Draft Issue ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

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

    // Title field
    let title_label_style = if state.focused_field == CreateCardField::Title {
        active_label
    } else {
        label_style
    };

    let title_lines = render_input_field(
        "Title",
        &state.title_input,
        state.title_cursor,
        state.focused_field == CreateCardField::Title,
        title_label_style,
        input_style,
    );

    // Body field
    let body_label_style = if state.focused_field == CreateCardField::Body {
        active_label
    } else {
        label_style
    };

    let body_lines = render_input_field(
        "Body",
        &state.body_input,
        state.body_cursor,
        state.focused_field == CreateCardField::Body,
        body_label_style,
        input_style,
    );

    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.extend(title_lines);
    lines.push(Line::from(""));
    lines.extend(body_lines);
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Tab", hint_style),
        Span::styled(":switch field  ", hint_style),
        Span::styled("C-s", hint_style),
        Span::styled(":submit  ", hint_style),
        Span::styled("Esc", hint_style),
        Span::styled(":cancel", hint_style),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_input_field(
    label: &str,
    input: &str,
    cursor_pos: usize,
    is_active: bool,
    label_style: Style,
    input_style: Style,
) -> Vec<Line<'static>> {
    let label_line = Line::from(Span::styled(format!("  {label}:"), label_style));

    let (before, after) = input.split_at(cursor_pos);
    let cursor_char = after.chars().next().unwrap_or(' ');
    let rest = if after.is_empty() {
        String::new()
    } else {
        after[cursor_char.len_utf8()..].to_string()
    };

    let mut spans = vec![Span::raw("  ")];

    if is_active {
        let border = Style::default().fg(Color::Yellow);
        spans.push(Span::styled("[", border));
        spans.push(Span::styled(before.to_string(), input_style));
        spans.push(Span::styled(
            cursor_char.to_string(),
            Style::default().fg(Color::Black).bg(Color::White),
        ));
        spans.push(Span::styled(rest, input_style));
        spans.push(Span::styled("]", border));
    } else {
        let border = Style::default().fg(Color::DarkGray);
        spans.push(Span::styled("[", border));
        spans.push(Span::styled(input.to_string(), input_style));
        spans.push(Span::styled("]", border));
    }

    vec![label_line, Line::from(spans)]
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
