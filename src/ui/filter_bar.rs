use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 1 {
        return;
    }

    let bar_area = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };

    let prompt = Span::styled(
        "/",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let input_text = &app.state.filter.input;
    let cursor_pos = app.state.filter.cursor_pos;

    // カーソル前後のテキストを分割
    let (before, after) = input_text.split_at(cursor_pos);
    let cursor_char = after.chars().next().unwrap_or(' ');
    let rest = if after.is_empty() {
        ""
    } else {
        &after[cursor_char.len_utf8()..]
    };

    let before_span = Span::styled(before, Style::default().fg(Color::White));
    let cursor_span = Span::styled(
        cursor_char.to_string(),
        Style::default()
            .fg(Color::Black)
            .bg(Color::White),
    );
    let after_span = Span::styled(rest, Style::default().fg(Color::White));

    let hint = Span::styled(
        " (Enter:apply  Esc:cancel  label:  assignee:)",
        Style::default().fg(Color::DarkGray),
    );

    let line = Line::from(vec![prompt, before_span, cursor_span, after_span, hint]);
    frame.render_widget(line, bar_area);
}
