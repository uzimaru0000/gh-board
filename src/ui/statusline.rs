use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

use crate::action::Action;
use crate::app::App;
use crate::keymap::{KeyBind, KeymapMode};
use crate::model::state::ViewMode;
use crate::ui::theme::theme;

/// Format the first (shortest) keybind for an action
fn short_key(app: &App, mode: KeymapMode, action: Action) -> String {
    let binds = app.state.keymap.bindings_for_action(mode, action);
    if binds.is_empty() {
        return String::new();
    }
    let mut strs: Vec<String> = binds.iter().map(|b| format_key(b)).collect();
    strs.sort_by_key(|s| s.len());
    strs[0].clone()
}

fn format_key(bind: &KeyBind) -> String {
    let s = bind.to_string();
    match s.as_str() {
        "Down" => "↓".to_string(),
        "Up" => "↑".to_string(),
        "Left" => "←".to_string(),
        "Right" => "→".to_string(),
        other => other.to_string(),
    }
}

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

    if let Some(grouping_name) = app
        .state
        .board
        .as_ref()
        .and_then(|b| b.grouping.field_name())
    {
        spans.push(Span::styled(
            format!("[group: {grouping_name}]"),
            Style::default()
                .fg(theme().blue)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }

    if let Some(view_idx) = app.state.active_view {
        if let Some(view) = app.state.views.get(view_idx) {
            let view_text = format!("[view: {}]", view.name);
            spans.push(Span::styled(
                view_text,
                Style::default().fg(theme().accent).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
        }
    } else if app.state.filter.active_filter.is_some() {
        let filter_text = format!("[filter: {}]", app.state.filter.input);
        spans.push(Span::styled(
            filter_text,
            Style::default().fg(theme().yellow).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }

    if app.state.mode == ViewMode::CardGrab {
        let grab_hints = format!(
            " hjkl:move  {}:release ",
            build_hint_pair(app, KeymapMode::CardGrab, &[
                (Action::ConfirmGrab, "confirm"),
                (Action::CancelGrab, "cancel"),
            ]),
        );
        spans.push(Span::styled(
            " GRAB ",
            Style::default()
                .fg(theme().text_inverted)
                .bg(theme().yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            grab_hints,
            Style::default().fg(theme().text_muted),
        ));
    } else {
        let mode = KeymapMode::Board;
        let hints: Vec<String> = [
            (Action::OpenDetail, "detail"),
            (Action::GrabCard, "grab"),
            (Action::NewCard, "new"),
            (Action::DeleteCard, "delete"),
            (Action::StartFilter, "filter"),
            (Action::ShowHelp, "help"),
            (Action::SwitchProject, "projects"),
            (Action::Refresh, "refresh"),
            (Action::Quit, "quit"),
        ]
        .iter()
        .filter_map(|(action, desc)| {
            let k = short_key(app, mode, *action);
            if k.is_empty() {
                None
            } else {
                Some(format!("{k}:{desc}"))
            }
        })
        .collect();

        spans.push(Span::styled(
            format!("{} ", hints.join("  ")),
            Style::default().fg(theme().text_muted),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(line, status_area);
}

fn build_hint_pair(app: &App, mode: KeymapMode, pairs: &[(Action, &str)]) -> String {
    pairs
        .iter()
        .filter_map(|(action, desc)| {
            let k = short_key(app, mode, *action);
            if k.is_empty() {
                None
            } else {
                Some(format!("{k}:{desc}"))
            }
        })
        .collect::<Vec<_>>()
        .join("  ")
}
