use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
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
    let matched = app.state.filtered_project_indices.len();
    let title = if app.state.project_filter_query.is_empty() {
        if total > 0 {
            format!(
                " Select Project {}/{} ",
                app.state.selected_project_index + 1,
                total
            )
        } else {
            " Select Project ".to_string()
        }
    } else if matched > 0 {
        format!(
            " Select Project {}/{} (of {}) ",
            app.state.selected_project_index + 1,
            matched,
            total
        )
    } else {
        format!(" Select Project 0/0 (of {total}) ")
    };

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let [filter_area, list_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(inner);

    render_filter_bar(frame, filter_area, &app.state.project_filter_query);

    if app.state.projects.is_empty() {
        let msg = Paragraph::new("No projects found.").style(Style::default().fg(theme().text_muted));
        frame.render_widget(msg, list_area);
        return;
    }

    if app.state.filtered_project_indices.is_empty() {
        let msg = Paragraph::new("No matches.").style(Style::default().fg(theme().text_muted));
        frame.render_widget(msg, list_area);
        return;
    }

    let items: Vec<ListItem> = app
        .state
        .filtered_project_indices
        .iter()
        .filter_map(|i| app.state.projects.get(*i))
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
                    Span::styled(&p.title, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(Span::styled(
                    format!("  {desc}"),
                    Style::default().fg(theme().text_muted),
                )),
            ])
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default().with_selected(Some(app.state.selected_project_index));
    frame.render_stateful_widget(list, list_area, &mut state);

    // 矢印: list_area のボーダーがないので popup_area に対して描く
    let viewport_items = list_area.height as usize / 2;
    if AppState::should_show_scrollbar(matched, viewport_items) {
        let max_offset = matched.saturating_sub(viewport_items);
        let approx_offset = app
            .state
            .selected_project_index
            .saturating_sub(viewport_items / 2)
            .min(max_offset);
        let has_above = approx_offset > 0;
        let has_below = approx_offset + viewport_items < matched;
        let buf = frame.buffer_mut();
        if has_above {
            draw_top_arrow(buf, popup_area);
        }
        if has_below {
            draw_bottom_arrow(buf, popup_area);
        }
    }
}

fn render_filter_bar(frame: &mut Frame, area: Rect, query: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().text_muted))
        .title(" Filter ");

    let content: Line = if query.is_empty() {
        Line::from(Span::styled(
            "type to search…",
            Style::default().fg(theme().text_muted),
        ))
    } else {
        Line::from(vec![
            Span::raw(query),
            Span::styled("█", Style::default().fg(theme().accent)),
        ])
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
