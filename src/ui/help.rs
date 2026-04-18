use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use rust_i18n::t;

use crate::action::Action;
use crate::keymap::{KeyBind, Keymap, KeymapMode};
use crate::ui::layout::modal_area_pct;
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
    description: String,
}

fn entry(action: Action, key: &str) -> HelpEntry {
    HelpEntry {
        action,
        description: t!(key).to_string(),
    }
}

pub fn render(frame: &mut Frame, area: Rect, keymap: &Keymap) {
    let popup = modal_area_pct(50, 70, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(format!(" {} ", t!("help.title")))
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
        entry(Action::MoveDown, "help.entries.move_down"),
        entry(Action::MoveUp, "help.entries.move_up"),
        entry(Action::MoveLeft, "help.entries.move_left"),
        entry(Action::MoveRight, "help.entries.move_right"),
        entry(Action::FirstItem, "help.entries.first_item"),
        entry(Action::LastItem, "help.entries.last_item"),
        entry(Action::NextTab, "help.entries.next_tab"),
    ];

    lines.push(Line::from(Span::styled(
        format!(" {}", t!("help.sections.navigation")),
        section_style,
    )));
    add_section_lines(&mut lines, keymap, KeymapMode::Board, &nav_entries, key_style, desc_style);

    // Actions section (from Board mode)
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {}", t!("help.sections.actions")),
        section_style,
    )));
    let action_entries = vec![
        entry(Action::GrabCard, "help.entries.grab_card"),
        entry(Action::NewCard, "help.entries.new_card"),
        entry(Action::ArchiveCard, "help.entries.archive_card"),
        entry(Action::ShowArchivedList, "help.entries.show_archived_list"),
        entry(Action::BulkSelectStart, "help.entries.bulk_select"),
        entry(Action::OpenDetail, "help.entries.open_detail"),
        entry(Action::SwitchProject, "help.entries.switch_project"),
        entry(Action::ChangeGrouping, "help.entries.change_grouping"),
        entry(Action::ToggleLayout, "help.entries.toggle_layout"),
        entry(Action::StartFilter, "help.entries.start_filter"),
        entry(Action::ClearFilter, "help.entries.clear_filter"),
        entry(Action::Refresh, "help.entries.refresh"),
        entry(Action::ShowHelp, "help.entries.show_help"),
        entry(Action::Quit, "help.entries.quit"),
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::Board, &action_entries, key_style, desc_style);

    // Detail View (Content) section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {}", t!("help.sections.detail_content")),
        section_style,
    )));
    let detail_entries = vec![
        entry(Action::MoveDown, "help.entries.detail_scroll"),
        entry(Action::MoveLeft, "help.entries.detail_table_scroll"),
        entry(Action::NextTab, "help.entries.detail_switch_sidebar"),
        entry(Action::OpenInBrowser, "help.entries.open_in_browser"),
        entry(Action::EditCard, "help.entries.edit_card"),
        entry(Action::NewComment, "help.entries.new_comment"),
        entry(Action::OpenCommentList, "help.entries.open_comment_list"),
        entry(Action::OpenReactionPicker, "help.entries.toggle_reaction"),
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::DetailContent, &detail_entries, key_style, desc_style);

    // Comment List section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {}", t!("help.sections.comment_list")),
        section_style,
    )));
    let comment_list_entries = vec![
        entry(Action::MoveDown, "help.entries.comment_next"),
        entry(Action::MoveUp, "help.entries.comment_prev"),
        entry(Action::EditComment, "help.entries.edit_own_comment"),
        entry(Action::NewComment, "help.entries.new_comment"),
        entry(Action::OpenReactionPicker, "help.entries.toggle_reaction_selected"),
        entry(Action::Back, "help.entries.back_to_detail"),
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::CommentList, &comment_list_entries, key_style, desc_style);

    // Detail View (Sidebar) section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {}", t!("help.sections.detail_sidebar")),
        section_style,
    )));
    let sidebar_entries = vec![
        entry(Action::MoveDown, "help.entries.sidebar_nav"),
        entry(Action::Select, "help.entries.sidebar_select"),
        entry(Action::ArchiveCard, "help.entries.archive_card"),
        entry(Action::NextTab, "help.entries.sidebar_switch_content"),
        entry(Action::Back, "help.entries.sidebar_back"),
    ];
    add_section_lines(&mut lines, keymap, KeymapMode::DetailSidebar, &sidebar_entries, key_style, desc_style);
    lines.push(Line::from(vec![
        Span::styled("  ──       ", key_style),
        Span::styled(t!("help.entries.custom_fields_hint").to_string(), desc_style),
    ]));

    // View switching (hardcoded, always shown)
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {}", t!("help.sections.views")),
        section_style,
    )));
    lines.push(Line::from(vec![
        Span::styled("  1-9     ", key_style),
        Span::styled(t!("help.entries.view_switch").to_string(), desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  0       ", key_style),
        Span::styled(t!("help.entries.view_clear").to_string(), desc_style),
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
            Span::styled(entry.description.clone(), desc_style),
        ]));
    }
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
