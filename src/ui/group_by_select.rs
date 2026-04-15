use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::model::project::Grouping;
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let state = match &app.state.group_by_select_state {
        Some(s) => s,
        None => return,
    };

    let current = app.state.board.as_ref().map(|b| &b.grouping);

    let popup = centered_rect(50, 60, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Group by ")
        .title_style(
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, cand) in state.candidates.iter().enumerate() {
        let is_selected = i == state.cursor;
        let is_current = current.is_some_and(|c| c == cand);

        let marker = if is_selected { "▶ " } else { "  " };
        let active_mark = if is_current { " *" } else { "" };

        let (kind, name) = match cand {
            Grouping::SingleSelect { field_name, .. } => ("SingleSelect", field_name.clone()),
            Grouping::Iteration { field_name, .. } => ("Iteration", field_name.clone()),
            Grouping::None => ("None", String::new()),
        };

        let name_style = if is_selected {
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme().text)
        };
        let kind_style = Style::default().fg(theme().text_muted);

        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), name_style),
            Span::styled(name, name_style),
            Span::styled(format!("{active_mark}  "), name_style),
            Span::styled(format!("[{kind}]"), kind_style),
        ]));
    }

    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    frame.render_widget(Paragraph::new(lines), vert[0]);

    let hint_style = Style::default()
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme().text_muted);
    let footer = Line::from(vec![
        Span::styled("j/k", hint_style),
        Span::styled(":nav  ", desc_style),
        Span::styled("Enter", hint_style),
        Span::styled(":apply  ", desc_style),
        Span::styled("Esc", hint_style),
        Span::styled(":close", desc_style),
    ]);
    frame.render_widget(footer, vert[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
