use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use crate::app::App;
use crate::app_state::AppState;
use crate::ui::scroll_fade::{draw_bottom_arrow, draw_top_arrow};
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 60, area);

    frame.render_widget(Clear, popup_area);

    let total = app.state.projects.len();
    let title = if total > 0 {
        format!(
            " Select Project {}/{} ",
            app.state.selected_project_index + 1,
            total
        )
    } else {
        " Select Project ".to_string()
    };
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(theme().accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent));

    if app.state.projects.is_empty() {
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);
        let msg = Line::from("No projects found.");
        frame.render_widget(msg, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .state.projects
        .iter()
        .map(|p| {
            let desc = p
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(50)
                .collect::<String>();
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("#{} ", p.number),
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        &p.title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::styled(
                    format!("  {desc}"),
                    Style::default().fg(theme().text_muted),
                )),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default().with_selected(Some(app.state.selected_project_index));
    frame.render_stateful_widget(list, popup_area, &mut state);

    // 矢印: popup のボーダー中央に上下矢印を描画
    let viewport_items = popup_area.height.saturating_sub(2) as usize / 2;
    if AppState::should_show_scrollbar(total, viewport_items) {
        let max_offset = total.saturating_sub(viewport_items);
        let approx_offset = app
            .state
            .selected_project_index
            .saturating_sub(viewport_items / 2)
            .min(max_offset);
        let has_above = approx_offset > 0;
        let has_below = approx_offset + viewport_items < total;
        let buf = frame.buffer_mut();
        if has_above {
            draw_top_arrow(buf, popup_area);
        }
        if has_below {
            draw_bottom_arrow(buf, popup_area);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
