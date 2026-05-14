use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::app_state::ViewTabRegion;
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if app.state.views.is_empty() {
        app.state.view_tab_regions.borrow_mut().clear();
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
    let mut regions: Vec<ViewTabRegion> = Vec::new();
    let space = " ";
    let space_w = space.width() as u16;

    let mut x_cursor = tab_area.x;
    spans.push(Span::raw(space));
    x_cursor = x_cursor.saturating_add(space_w);

    // "All" tab
    let all_label = " 0:All ";
    let all_w = all_label.width() as u16;
    let all_style = if app.state.active_view.is_none() {
        selected_style
    } else {
        unselected_style
    };
    spans.push(Span::styled(all_label, all_style));
    regions.push(ViewTabRegion {
        area: Rect {
            x: x_cursor,
            y: tab_area.y,
            width: all_w,
            height: 1,
        },
        view_index: None,
    });
    x_cursor = x_cursor.saturating_add(all_w);
    spans.push(Span::raw(space));
    x_cursor = x_cursor.saturating_add(space_w);

    // View tabs (1-indexed)
    for (i, view) in app.state.views.iter().enumerate() {
        let label = format!(" {}:{} ", i + 1, view.name);
        let w = label.width() as u16;
        let style = if app.state.active_view == Some(i) {
            selected_style
        } else {
            unselected_style
        };
        spans.push(Span::styled(label.clone(), style));
        regions.push(ViewTabRegion {
            area: Rect {
                x: x_cursor,
                y: tab_area.y,
                width: w,
                height: 1,
            },
            view_index: Some(i),
        });
        x_cursor = x_cursor.saturating_add(w);
        spans.push(Span::raw(space));
        x_cursor = x_cursor.saturating_add(space_w);
    }

    *app.state.view_tab_regions.borrow_mut() = regions;

    let line = Line::from(spans);
    frame.render_widget(line, tab_area);
}
