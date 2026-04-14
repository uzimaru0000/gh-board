use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::model::state::{CreateCardField, CreateCardState, NewCardType};
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, state: &CreateCardState) {
    let popup = centered_rect(60, 18, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" New Card ")
        .title_style(
            Style::default()
                .fg(theme().green)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().green));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let label_style = Style::default()
        .fg(theme().accent)
        .add_modifier(Modifier::BOLD);
    let active_label = Style::default()
        .fg(theme().yellow)
        .add_modifier(Modifier::BOLD);
    let input_style = Style::default().fg(theme().text);
    let hint_style = Style::default().fg(theme().text_muted);

    // Layout: Type (2 lines) + gap + Title box (3 lines) + gap + Body (2 lines) + gap + hints
    let chunks = Layout::vertical([
        Constraint::Length(2), // Type
        Constraint::Length(1), // gap
        Constraint::Length(3), // Title (box)
        Constraint::Length(1), // gap
        Constraint::Length(2), // Body
        Constraint::Min(0),   // gap + hints
    ])
    .split(inner);

    // --- Type field ---
    let type_label_style = if state.focused_field == CreateCardField::Type {
        active_label
    } else {
        label_style
    };
    render_type_field(frame, chunks[0], &state.card_type, state.focused_field == CreateCardField::Type, type_label_style, input_style);

    // --- Title field (Box) ---
    let title_is_active = state.focused_field == CreateCardField::Title;
    let title_border_style = if title_is_active {
        Style::default().fg(theme().yellow)
    } else {
        Style::default().fg(theme().border_unfocused)
    };
    let title_label_style = if title_is_active { active_label } else { label_style };

    let title_block = Block::default()
        .title(Span::styled(" Title ", title_label_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(title_border_style);

    let title_inner = title_block.inner(chunks[2]);
    frame.render_widget(title_block, chunks[2]);

    let title_line = render_title_content(&state.title_input, state.title_cursor, title_is_active, input_style);
    frame.render_widget(Paragraph::new(title_line), title_inner);

    // --- Body field ---
    let body_label_style = if state.focused_field == CreateCardField::Body {
        active_label
    } else {
        label_style
    };
    render_body_field(frame, chunks[4], &state.body_input, state.focused_field == CreateCardField::Body, body_label_style, hint_style);

    // --- Hints ---
    let hint_area = chunks[5];
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

fn render_type_field(
    frame: &mut Frame,
    area: Rect,
    card_type: &NewCardType,
    is_active: bool,
    label_style: Style,
    input_style: Style,
) {
    let type_text = match card_type {
        NewCardType::Draft => "Draft Issue",
        NewCardType::Issue => "Issue",
    };

    let label_line = Line::from(Span::styled("  Type:", label_style));

    let value_line = if is_active {
        let border = Style::default().fg(theme().yellow);
        let arrow_style = Style::default()
            .fg(theme().yellow)
            .add_modifier(Modifier::BOLD);
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[", border),
            Span::styled(" < ", arrow_style),
            Span::styled(type_text.to_string(), input_style),
            Span::styled(" > ", arrow_style),
            Span::styled("]", border),
        ])
    } else {
        let border = Style::default().fg(theme().border_unfocused);
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[", border),
            Span::styled(format!(" {type_text} "), input_style),
            Span::styled("]", border),
        ])
    };

    frame.render_widget(Paragraph::new(vec![label_line, value_line]), area);
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
                Style::default().fg(theme().text_inverted).bg(theme().text),
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
        let msg_style = if is_active { hint_style } else { hint_style };
        let msg = if is_active {
            "  (press Enter to edit in $EDITOR)"
        } else {
            "  (empty)"
        };
        Line::from(Span::styled(msg, msg_style))
    } else {
        let preview: String = body.lines().next().unwrap_or("").chars().take(50).collect();
        let suffix = if body.lines().count() > 1 || preview.len() < body.lines().next().unwrap_or("").len() {
            "..."
        } else {
            ""
        };
        let text = format!("  {preview}{suffix}");
        if is_active {
            Line::from(vec![
                Span::styled(text, Style::default().fg(theme().text)),
                Span::styled("  (Enter to edit)", hint_style),
            ])
        } else {
            Line::from(Span::styled(text, Style::default().fg(theme().text)))
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
