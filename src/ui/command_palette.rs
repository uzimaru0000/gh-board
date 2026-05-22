use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::layout::modal_area_pct;
use crate::ui::theme::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let commands = &app.state.custom_commands;
    let cursor = app.state.command_palette_cursor;

    let popup = modal_area_pct(60, 70, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Custom Commands ")
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

    if commands.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No custom commands defined. Add [[command]] entries to config.toml.",
            Style::default().fg(theme().text_muted),
        )));
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, cmd) in commands.iter().enumerate() {
        let is_selected = i == cursor;
        let marker = if is_selected { "▶ " } else { "  " };
        let name_style = if is_selected {
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme().text)
        };
        let key_style = Style::default().fg(theme().text_muted);
        let desc_style = Style::default().fg(theme().text_dim);

        let key_label = cmd
            .key
            .as_deref()
            .map(|k| format!(" [{k}]"))
            .unwrap_or_default();

        let mut spans = vec![
            Span::styled(marker.to_string(), name_style),
            Span::styled(cmd.name.clone(), name_style),
            Span::styled(key_label, key_style),
        ];
        if let Some(desc) = &cmd.description {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(desc.clone(), desc_style));
        }
        lines.push(Line::from(spans));
    }

    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let total = lines.len();
    let viewport = vert[0].height as usize;
    let scroll = compute_scroll(cursor, viewport, total);
    frame.render_widget(
        Paragraph::new(lines).scroll((scroll as u16, 0)),
        vert[0],
    );

    let hint_style = Style::default()
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme().text_muted);
    let footer = Line::from(vec![
        Span::styled("j/k", hint_style),
        Span::styled(":nav  ", desc_style),
        Span::styled("Enter", hint_style),
        Span::styled(":run  ", desc_style),
        Span::styled("Esc", hint_style),
        Span::styled(":close", desc_style),
    ]);
    frame.render_widget(footer, vert[1]);
}

fn compute_scroll(cursor: usize, viewport: usize, total: usize) -> usize {
    if viewport == 0 || total <= viewport {
        return 0;
    }
    if cursor < viewport {
        return 0;
    }
    let max_scroll = total - viewport;
    (cursor + 1 - viewport).min(max_scroll)
}

#[cfg(test)]
mod tests {
    use super::compute_scroll;

    #[test]
    fn scroll_zero_when_fits() {
        assert_eq!(compute_scroll(0, 10, 5), 0);
        assert_eq!(compute_scroll(4, 10, 5), 0);
    }

    #[test]
    fn scroll_follows_cursor() {
        // viewport 3, total 10
        assert_eq!(compute_scroll(0, 3, 10), 0);
        assert_eq!(compute_scroll(2, 3, 10), 0);
        assert_eq!(compute_scroll(3, 3, 10), 1);
        assert_eq!(compute_scroll(5, 3, 10), 3);
        assert_eq!(compute_scroll(9, 3, 10), 7);
    }
}
