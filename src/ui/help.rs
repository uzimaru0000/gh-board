use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::action::Action;
use crate::keymap::{KeyBind, Keymap, KeymapMode};
use crate::ui::theme::theme;

/// Format a list of KeyBinds into a display string like "j ↓"
fn format_keys(binds: &[&KeyBind]) -> String {
    if binds.is_empty() {
        return String::new();
    }
    let mut strs: Vec<String> = binds.iter().map(|b| format_key_display(b)).collect();
    strs.sort();
    strs.dedup();
    strs.join(" ")
}

/// Format a single KeyBind with arrow symbols for readability
fn format_key_display(bind: &KeyBind) -> String {
    let s = bind.to_string();
    match s.as_str() {
        "Down" => "↓".to_string(),
        "Up" => "↑".to_string(),
        "Left" => "←".to_string(),
        "Right" => "→".to_string(),
        other => other.to_string(),
    }
}

struct HelpEntry {
    action: Action,
    description: &'static str,
}

pub fn render(frame: &mut Frame, area: Rect, keymap: &Keymap) {
    let popup = centered_rect(50, 70, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Help ")
        .title_style(Style::default().fg(theme().accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent));

    let key_style = Style::default()
        .fg(theme().yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme().text);
    let section_style = Style::default()
        .fg(theme().accent)
        .add_modifier(Modifier::BOLD);

    let mut lines = vec![Line::from("")];

    // Navigation section (from Board mode)
    let nav_entries = vec![
        HelpEntry { action: Action::MoveDown, description: "Next card" },
        HelpEntry { action: Action::MoveUp, description: "Previous card" },
        HelpEntry { action: Action::MoveLeft, description: "Previous column" },
        HelpEntry { action: Action::MoveRight, description: "Next column" },
        HelpEntry { action: Action::FirstItem, description: "First card" },
        HelpEntry { action: Action::LastItem, description: "Last card" },
        HelpEntry { action: Action::NextTab, description: "Next column (wrap)" },
    ];

    lines.push(Line::from(Span::styled(" Navigation", section_style)));
    add_section_lines(&mut lines, keymap, KeymapMode::Board, &nav_entries, key_style, desc_style);

    // Actions section (from Board mode)
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Actions", section_style)));
    let action_entries = vec![
        HelpEntry { action: Action::GrabCard, description: "Grab card (move mode)" },
        HelpEntry { action: Action::NewCard, description: "New card (draft/issue)" },
        HelpEntry { action: Action::DeleteCard, description: "Delete card" },
        HelpEntry { action: Action::OpenDetail, description: "View card detail" },
        HelpEntry { action: Action::SwitchProject, description: "Switch project" },
        HelpEntry { action: Action::ChangeGrouping, description: "Change grouping field" },
        HelpEntry { action: Action::StartFilter, description: "Filter (label: assignee: milestone: |:OR)" },
        HelpEntry { action: Action::ClearFilter, description: "Clear filter / view" },
        HelpEntry { action: Action::Refresh, description: "Refresh" },
        HelpEntry { action: Action::ShowHelp, description: "Toggle help" },
        HelpEntry { action: Action::Quit, description: "Quit" },
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::Board, &action_entries, key_style, desc_style);

    // Detail View (Content) section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Detail View (Content)", section_style)));
    let detail_entries = vec![
        HelpEntry { action: Action::MoveDown, description: "Scroll" },
        HelpEntry { action: Action::MoveLeft, description: "Table scroll" },
        HelpEntry { action: Action::NextTab, description: "Switch to sidebar" },
        HelpEntry { action: Action::OpenInBrowser, description: "Open in browser" },
        HelpEntry { action: Action::EditCard, description: "Edit card" },
        HelpEntry { action: Action::NewComment, description: "New comment" },
        HelpEntry { action: Action::OpenCommentList, description: "Comment list" },
        HelpEntry { action: Action::OpenReactionPicker, description: "Toggle reaction" },
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::DetailContent, &detail_entries, key_style, desc_style);

    // Comment List section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Comment List", section_style)));
    let comment_list_entries = vec![
        HelpEntry { action: Action::MoveDown, description: "Next comment" },
        HelpEntry { action: Action::MoveUp, description: "Previous comment" },
        HelpEntry { action: Action::EditComment, description: "Edit own comment" },
        HelpEntry { action: Action::NewComment, description: "New comment" },
        HelpEntry { action: Action::OpenReactionPicker, description: "Toggle reaction on selected" },
        HelpEntry { action: Action::Back, description: "Back to detail" },
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::CommentList, &comment_list_entries, key_style, desc_style);

    // Detail View (Sidebar) section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Detail View (Sidebar)", section_style)));
    let sidebar_entries = vec![
        HelpEntry { action: Action::MoveDown, description: "Navigate sections (incl. custom fields)" },
        HelpEntry { action: Action::Select, description: "Edit / Select" },
        HelpEntry { action: Action::DeleteCard, description: "Delete card" },
        HelpEntry { action: Action::NextTab, description: "Switch to content" },
        HelpEntry { action: Action::Back, description: "Back to content" },
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::DetailSidebar, &sidebar_entries, key_style, desc_style);
    lines.push(Line::from(vec![
        Span::styled("  ──       ", key_style),
        Span::styled("Custom fields: Enter opens picker / text input (Enter saves, Esc cancels)", desc_style),
    ]));

    // View switching (hardcoded, always shown)
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Views", section_style)));
    lines.push(Line::from(vec![
        Span::styled("  1-9     ", key_style),
        Span::styled("Switch to view 1-9", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  0       ", key_style),
        Span::styled("Show all (clear view)", desc_style),
    ]));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

fn add_section_lines(
    lines: &mut Vec<Line>,
    keymap: &Keymap,
    mode: KeymapMode,
    entries: &[HelpEntry],
    key_style: Style,
    desc_style: Style,
) {
    for entry in entries {
        let binds = keymap.bindings_for_action(mode, entry.action);
        if binds.is_empty() {
            continue;
        }
        let keys_str = format_keys(&binds);
        // Pad for alignment
        let padded = format!("  {:<10}", keys_str);
        lines.push(Line::from(vec![
            Span::styled(padded, key_style),
            Span::styled(entry.description, desc_style),
        ]));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_keys_reflects_override() {
        let keymap = Keymap::default_keymap();
        let mut overrides = crate::config::KeysConfig::default();
        overrides.board.insert("refresh".into(), vec!["R".into()]);
        overrides.board.insert("start_filter".into(), vec!["/".into(), "f".into()]);
        let keymap = keymap.with_overrides(&overrides);

        // Refresh should show "R", not "r"
        let refresh_binds = keymap.bindings_for_action(KeymapMode::Board, Action::Refresh);
        let refresh_str = format_keys(&refresh_binds);
        assert_eq!(refresh_str, "R");

        // StartFilter should show "/" and "f"
        let filter_binds = keymap.bindings_for_action(KeymapMode::Board, Action::StartFilter);
        let filter_str = format_keys(&filter_binds);
        assert!(filter_str.contains("/"));
        assert!(filter_str.contains("f"));
    }
}
