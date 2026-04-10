use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 2 {
        return;
    }

    let status_area = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };

    let project_name = app
        .state.board
        .as_ref()
        .map(|b| b.project_title.as_str())
        .unwrap_or("gh-board");

    let left = Span::styled(
        format!(" {project_name} "),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let mut spans = vec![left, Span::raw(" ")];

    if let Some(filter) = &app.state.filter.active_filter {
        let filter_text = match filter {
            crate::model::state::ActiveFilter::Text(q) => format!("[filter: {q}]"),
            crate::model::state::ActiveFilter::Label(q) => format!("[label: {q}]"),
            crate::model::state::ActiveFilter::Assignee(q) => format!("[assignee: {q}]"),
        };
        spans.push(Span::styled(
            filter_text,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }

    spans.push(Span::styled(
        "Enter:detail  H/L:move  n:new  d:delete  /:filter  ?:help  p:projects  r:refresh  q:quit ",
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);
    frame.render_widget(line, status_area);
}
