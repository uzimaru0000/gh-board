use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

use crate::app::App;
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if app.state.views.is_empty() {
        return;
    }

    let tab_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };

    let selected_style = Style::default()
        .fg(theme().text_inverted)
        .bg(theme().accent)
        .add_modifier(Modifier::BOLD);

    let unselected_style = Style::default().fg(theme().text_dim);

    let mut spans = Vec::new();
    spans.push(Span::raw(" "));

    // "All" tab (index 0)
    let all_selected = app.state.active_view.is_none();
    let all_style = if all_selected {
        selected_style
    } else {
        unselected_style
    };
    spans.push(Span::styled(" 0:All ", all_style));
    spans.push(Span::raw(" "));

    // View tabs (1-indexed)
    for (i, view) in app.state.views.iter().enumerate() {
        let is_selected = app.state.active_view == Some(i);
        let style = if is_selected {
            selected_style
        } else {
            unselected_style
        };
        let label = format!(" {}:{} ", i + 1, view.name);
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }

    let line = Line::from(spans);
    frame.render_widget(line, tab_area);
}
