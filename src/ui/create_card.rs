use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::model::state::{CreateCardField, CreateCardState, NewCardType};
use crate::ui::layout::modal_area_fixed;
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, state: &CreateCardState) {
    let popup = modal_area_fixed(60, 21, area);
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

    // Layout: Type(2) + gap(1) + Title(3) + gap(1) + Body(2) + gap(1) + Submit(3) + hints
    let chunks = Layout::vertical([
        Constraint::Length(2), // Type
        Constraint::Length(1), // gap
        Constraint::Length(3), // Title (box)
        Constraint::Length(1), // gap
        Constraint::Length(2), // Body
        Constraint::Length(1), // gap
        Constraint::Length(3), // Submit button
        Constraint::Min(0),    // hints
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

    // Submit ボタンと同じく外側に 1 col の余白を取り、縦ラインを揃える
    let title_outer = Block::default().padding(Padding::horizontal(1));
    let title_area = title_outer.inner(chunks[2]);
    frame.render_widget(title_outer, chunks[2]);

    let title_block = Block::default()
        .title(Span::styled(" Title ", title_label_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(title_border_style);

    let title_inner = title_block.inner(title_area);
    frame.render_widget(title_block, title_area);

    let title_line = render_title_content(&state.title_input, state.title_cursor, title_is_active, input_style);
    frame.render_widget(Paragraph::new(title_line), title_inner);

    // --- Body field ---
    let body_label_style = if state.focused_field == CreateCardField::Body {
        active_label
    } else {
        label_style
    };
    render_body_field(frame, chunks[4], &state.body_input, state.focused_field == CreateCardField::Body, body_label_style, hint_style);

    // --- Submit button ---
    let submit_focused = state.focused_field == CreateCardField::Submit;
    let submit_enabled = !state.title_input.trim().is_empty();
    render_submit_button(frame, chunks[6], submit_focused, submit_enabled);

    // --- Hints ---
    let hint_area = chunks[7];
    if hint_area.height >= 1 {
        let hint_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", hint_style),
            Span::styled(":switch  ", hint_style),
            Span::styled("Enter", hint_style),
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
        let msg_style = hint_style;
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

fn render_submit_button(frame: &mut Frame, area: Rect, is_focused: bool, is_enabled: bool) {
    let outer = Block::default().padding(Padding::horizontal(1));
    let btn_area = outer.inner(area);
    frame.render_widget(outer, area);

    if is_enabled && is_focused {
        // active: 塗りつぶし
        let bg = theme().green;
        let fg = theme().text;
        let edge = Style::default().fg(bg);
        let fill = Style::default().fg(fg).bg(bg);
        let width = btn_area.width as usize;
        let label = "Submit";
        let pad_total = width.saturating_sub(label.len());
        let pad_left = pad_total / 2;
        let pad_right = pad_total - pad_left;

        let lines = vec![
            Line::from(Span::styled("▄".repeat(width), edge)),
            Line::from(Span::styled(
                format!("{}{label}{}", " ".repeat(pad_left), " ".repeat(pad_right)),
                fill,
            )),
            Line::from(Span::styled("▀".repeat(width), edge)),
        ];
        frame.render_widget(Paragraph::new(lines), btn_area);
    } else {
        // disable / unfocused: 枠のみ
        let (border_fg, label_fg) = if is_enabled {
            (theme().border_unfocused, theme().text)
        } else {
            (theme().border_unfocused, theme().text_muted)
        };
        let button = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_fg));
        let inner = button.inner(btn_area);
        frame.render_widget(button, btn_area);
        let label_line = Line::from(Span::styled("Submit", Style::default().fg(label_fg)))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(Paragraph::new(label_line), inner);
    }
}

