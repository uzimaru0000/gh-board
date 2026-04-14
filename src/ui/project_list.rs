use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use crate::app::App;
use crate::ui::theme::THEME;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 60, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Select Project ")
        .title_style(Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(THEME.accent));

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
                    Style::default().fg(THEME.text_muted),
                )),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(THEME.accent)
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
