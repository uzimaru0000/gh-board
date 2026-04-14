use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

use crate::app::App;
use crate::model::state::ViewMode;
use crate::ui::theme::theme;

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
            .fg(theme().text_inverted)
            .bg(theme().accent)
            .add_modifier(Modifier::BOLD),
    );

    let mut spans = vec![left, Span::raw(" ")];

    if app.state.filter.active_filter.is_some() {
        let filter_text = format!("[filter: {}]", app.state.filter.input);
        spans.push(Span::styled(
            filter_text,
            Style::default().fg(theme().yellow).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }

    if app.state.mode == ViewMode::CardGrab {
        spans.push(Span::styled(
            " GRAB ",
            Style::default()
                .fg(theme().text_inverted)
                .bg(theme().yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            " hjkl:move  Space/Esc:release ",
            Style::default().fg(theme().text_muted),
        ));
    } else {
        spans.push(Span::styled(
            "Enter:detail  Space:grab  H/L:move  n:new  d:delete  /:filter  ?:help  p:projects  r:refresh  q:quit ",
            Style::default().fg(theme().text_muted),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(line, status_area);
}
