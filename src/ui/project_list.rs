use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 60, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Select Project ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

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
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default().with_selected(Some(app.state.selected_project_index));
    frame.render_stateful_widget(list, popup_area, &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
