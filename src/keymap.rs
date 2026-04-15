use std::collections::HashMap;
use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBind {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBind {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn char(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn key(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn ctrl(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub fn from_key_event(key: &KeyEvent) -> Self {
        let mut modifiers = key.modifiers & (KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT);
        // Normalize: for uppercase ASCII chars, the SHIFT modifier is implicit in the char itself
        // crossterm sends KeyCode::Char('C') + SHIFT for uppercase C
        if let KeyCode::Char(c) = key.code
            && c.is_ascii_uppercase() {
                modifiers -= KeyModifiers::SHIFT;
            }
        Self {
            code: key.code,
            modifiers,
        }
    }

    /// Parse a key string like "j", "Enter", "C-c", "S-Tab", "Space", "Down"
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('-').collect();

        if parts.len() == 1 {
            // Simple key: "j", "Enter", "Esc", etc.
            return parse_key_name(parts[0]).map(KeyBind::key);
        }

        // Modifier prefix(es) + key
        let mut modifiers = KeyModifiers::NONE;
        for &part in &parts[..parts.len() - 1] {
            match part {
                "C" => modifiers |= KeyModifiers::CONTROL,
                "S" => modifiers |= KeyModifiers::SHIFT,
                "A" => modifiers |= KeyModifiers::ALT,
                _ => return Err(format!("unknown modifier: {part}")),
            }
        }

        let key_name = parts[parts.len() - 1];
        let code = parse_key_name(key_name)?;
        Ok(KeyBind::new(code, modifiers))
    }
}

impl fmt::Display for KeyBind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("C");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("A");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("S");
        }

        let key_str = match self.code {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::BackTab => "S-Tab".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            _ => format!("{:?}", self.code),
        };

        if parts.is_empty() {
            // Special case: BackTab already includes "S-" in its display
            if self.code == KeyCode::BackTab {
                write!(f, "S-Tab")
            } else {
                write!(f, "{key_str}")
            }
        } else {
            // Special case: BackTab with S modifier would be "S-S-Tab", just show "S-Tab"
            if self.code == KeyCode::BackTab {
                write!(f, "S-Tab")
            } else {
                write!(f, "{}-{key_str}", parts.join("-"))
            }
        }
    }
}

fn parse_key_name(name: &str) -> Result<KeyCode, String> {
    match name {
        "Enter" | "Return" | "CR" => Ok(KeyCode::Enter),
        "Esc" | "Escape" => Ok(KeyCode::Esc),
        "Tab" => Ok(KeyCode::Tab),
        "Backspace" | "BS" => Ok(KeyCode::Backspace),
        "Space" => Ok(KeyCode::Char(' ')),
        "Up" => Ok(KeyCode::Up),
        "Down" => Ok(KeyCode::Down),
        "Left" => Ok(KeyCode::Left),
        "Right" => Ok(KeyCode::Right),
        s if s.len() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap())),
        _ => Err(format!("unknown key: {name}")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeymapMode {
    Board,
    ProjectSelect,
    Help,
    Confirm,
    CardGrab,
    RepoSelect,
    DetailContent,
    DetailSidebar,
    StatusSelect,
    SidebarEdit,
    CommentList,
    GroupBySelect,
    ReactionPicker,
    ArchivedList,
    CreateCardType,
    CreateCardBody,
    CreateCardSubmit,
    EditCardBody,
    // Structural keys for text-input modes (Esc, Enter, Ctrl+c, Ctrl+s, Tab, BackTab)
    FilterStructural,
    CreateCardGlobal,
    EditCardGlobal,
}

pub struct Keymap {
    global: HashMap<KeyBind, Action>,
    modes: HashMap<KeymapMode, HashMap<KeyBind, Action>>,
}

impl Keymap {
    pub fn resolve(&self, mode: KeymapMode, key: &KeyEvent) -> Option<Action> {
        let bind = KeyBind::from_key_event(key);
        // Mode-specific takes precedence over global
        self.modes
            .get(&mode)
            .and_then(|m| m.get(&bind))
            .or_else(|| self.global.get(&bind))
            .copied()
    }

    /// Build a keymap with all current hardcoded defaults
    pub fn default_keymap() -> Self {
        let mut keymap = Keymap {
            global: HashMap::new(),
            modes: HashMap::new(),
        };

        // Global bindings (shared across many modes)
        keymap.global.insert(KeyBind::ctrl('c'), Action::ForceQuit);

        // Board mode
        let mut board = HashMap::new();
        board.insert(KeyBind::char('j'), Action::MoveDown);
        board.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        board.insert(KeyBind::char('k'), Action::MoveUp);
        board.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        board.insert(KeyBind::char('h'), Action::MoveLeft);
        board.insert(KeyBind::key(KeyCode::Left), Action::MoveLeft);
        board.insert(KeyBind::char('l'), Action::MoveRight);
        board.insert(KeyBind::key(KeyCode::Right), Action::MoveRight);
        board.insert(KeyBind::char('g'), Action::FirstItem);
        board.insert(KeyBind::char('G'), Action::LastItem);
        board.insert(KeyBind::key(KeyCode::Tab), Action::NextTab);
        board.insert(KeyBind::key(KeyCode::BackTab), Action::PrevTab);
        board.insert(KeyBind::key(KeyCode::Enter), Action::OpenDetail);
        board.insert(KeyBind::char('p'), Action::SwitchProject);
        board.insert(KeyBind::char('r'), Action::Refresh);
        board.insert(KeyBind::char('?'), Action::ShowHelp);
        board.insert(KeyBind::char('/'), Action::StartFilter);
        board.insert(KeyBind::ctrl('u'), Action::ClearFilter);
        board.insert(KeyBind::char('a'), Action::ArchiveCard);
        board.insert(KeyBind::char('v'), Action::ShowArchivedList);
        board.insert(KeyBind::char('n'), Action::NewCard);
        board.insert(KeyBind::char(' '), Action::GrabCard);
        board.insert(KeyBind::ctrl('g'), Action::ChangeGrouping);
        board.insert(KeyBind::char('q'), Action::Quit);
        board.insert(KeyBind::key(KeyCode::Esc), Action::Quit);
        keymap.modes.insert(KeymapMode::Board, board);

        // ProjectSelect mode (文字キーはフィルタ入力に使うため割り当てない)
        let mut project_select = HashMap::new();
        project_select.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        project_select.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        project_select.insert(KeyBind::key(KeyCode::Enter), Action::Select);
        project_select.insert(KeyBind::key(KeyCode::Esc), Action::Quit);
        keymap.modes.insert(KeymapMode::ProjectSelect, project_select);

        // Help mode
        let mut help = HashMap::new();
        help.insert(KeyBind::char('?'), Action::Back);
        help.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        help.insert(KeyBind::char('q'), Action::Back);
        keymap.modes.insert(KeymapMode::Help, help);

        // Filter structural keys
        let mut filter = HashMap::new();
        filter.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        filter.insert(KeyBind::key(KeyCode::Enter), Action::Select);
        keymap.modes.insert(KeymapMode::FilterStructural, filter);

        // Confirm mode
        let mut confirm = HashMap::new();
        confirm.insert(KeyBind::char('y'), Action::ConfirmYes);
        confirm.insert(KeyBind::char('n'), Action::ConfirmNo);
        confirm.insert(KeyBind::key(KeyCode::Esc), Action::ConfirmNo);
        keymap.modes.insert(KeymapMode::Confirm, confirm);

        // CreateCard global keys (Esc, Tab, BackTab)
        let mut create_card_global = HashMap::new();
        create_card_global.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        create_card_global.insert(KeyBind::key(KeyCode::Tab), Action::NextField);
        create_card_global.insert(KeyBind::key(KeyCode::BackTab), Action::PrevField);
        keymap.modes.insert(KeymapMode::CreateCardGlobal, create_card_global);

        // CreateCard type field
        let mut create_card_type = HashMap::new();
        create_card_type.insert(KeyBind::key(KeyCode::Left), Action::ToggleType);
        create_card_type.insert(KeyBind::key(KeyCode::Right), Action::ToggleType);
        create_card_type.insert(KeyBind::char('h'), Action::ToggleType);
        create_card_type.insert(KeyBind::char('l'), Action::ToggleType);
        keymap.modes.insert(KeymapMode::CreateCardType, create_card_type);

        // CreateCard body field
        let mut create_card_body = HashMap::new();
        create_card_body.insert(KeyBind::key(KeyCode::Enter), Action::OpenEditor);
        keymap.modes.insert(KeymapMode::CreateCardBody, create_card_body);

        // CreateCard submit button
        let mut create_card_submit = HashMap::new();
        create_card_submit.insert(KeyBind::key(KeyCode::Enter), Action::Submit);
        keymap.modes.insert(KeymapMode::CreateCardSubmit, create_card_submit);

        // CardGrab mode
        let mut card_grab = HashMap::new();
        card_grab.insert(KeyBind::char('j'), Action::MoveDown);
        card_grab.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        card_grab.insert(KeyBind::char('k'), Action::MoveUp);
        card_grab.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        card_grab.insert(KeyBind::char('h'), Action::MoveLeft);
        card_grab.insert(KeyBind::key(KeyCode::Left), Action::MoveLeft);
        card_grab.insert(KeyBind::char('l'), Action::MoveRight);
        card_grab.insert(KeyBind::key(KeyCode::Right), Action::MoveRight);
        card_grab.insert(KeyBind::char(' '), Action::ConfirmGrab);
        card_grab.insert(KeyBind::key(KeyCode::Esc), Action::CancelGrab);
        keymap.modes.insert(KeymapMode::CardGrab, card_grab);

        // RepoSelect mode
        let mut repo_select = HashMap::new();
        repo_select.insert(KeyBind::char('j'), Action::MoveDown);
        repo_select.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        repo_select.insert(KeyBind::char('k'), Action::MoveUp);
        repo_select.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        repo_select.insert(KeyBind::key(KeyCode::Enter), Action::Select);
        repo_select.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        repo_select.insert(KeyBind::char('q'), Action::Back);
        keymap.modes.insert(KeymapMode::RepoSelect, repo_select);

        // Detail content
        let mut detail_content = HashMap::new();
        detail_content.insert(KeyBind::char('q'), Action::Quit);
        detail_content.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        detail_content.insert(KeyBind::key(KeyCode::Tab), Action::NextTab);
        detail_content.insert(KeyBind::key(KeyCode::BackTab), Action::PrevTab);
        detail_content.insert(KeyBind::char('o'), Action::OpenInBrowser);
        detail_content.insert(KeyBind::key(KeyCode::Enter), Action::OpenInBrowser);
        detail_content.insert(KeyBind::char('j'), Action::MoveDown);
        detail_content.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        detail_content.insert(KeyBind::char('k'), Action::MoveUp);
        detail_content.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        detail_content.insert(KeyBind::char('h'), Action::MoveLeft);
        detail_content.insert(KeyBind::key(KeyCode::Left), Action::MoveLeft);
        detail_content.insert(KeyBind::char('l'), Action::MoveRight);
        detail_content.insert(KeyBind::key(KeyCode::Right), Action::MoveRight);
        detail_content.insert(KeyBind::char('e'), Action::EditCard);
        detail_content.insert(KeyBind::char('c'), Action::NewComment);
        detail_content.insert(KeyBind::char('C'), Action::OpenCommentList);
        detail_content.insert(KeyBind::char('r'), Action::OpenReactionPicker);
        keymap.modes.insert(KeymapMode::DetailContent, detail_content);

        // Detail sidebar
        let mut detail_sidebar = HashMap::new();
        detail_sidebar.insert(KeyBind::char('q'), Action::Quit);
        detail_sidebar.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        detail_sidebar.insert(KeyBind::key(KeyCode::Tab), Action::NextTab);
        detail_sidebar.insert(KeyBind::key(KeyCode::BackTab), Action::PrevTab);
        detail_sidebar.insert(KeyBind::char('j'), Action::MoveDown);
        detail_sidebar.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        detail_sidebar.insert(KeyBind::char('k'), Action::MoveUp);
        detail_sidebar.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        detail_sidebar.insert(KeyBind::key(KeyCode::Enter), Action::Select);
        detail_sidebar.insert(KeyBind::char('a'), Action::ArchiveCard);
        keymap.modes.insert(KeymapMode::DetailSidebar, detail_sidebar);

        // Status select dropdown
        let mut status_select = HashMap::new();
        status_select.insert(KeyBind::char('j'), Action::MoveDown);
        status_select.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        status_select.insert(KeyBind::char('k'), Action::MoveUp);
        status_select.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        status_select.insert(KeyBind::key(KeyCode::Enter), Action::Select);
        status_select.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        keymap.modes.insert(KeymapMode::StatusSelect, status_select);

        // Sidebar edit (labels/assignees toggle)
        let mut sidebar_edit = HashMap::new();
        sidebar_edit.insert(KeyBind::char('q'), Action::Back);
        sidebar_edit.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        sidebar_edit.insert(KeyBind::char('j'), Action::MoveDown);
        sidebar_edit.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        sidebar_edit.insert(KeyBind::char('k'), Action::MoveUp);
        sidebar_edit.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        sidebar_edit.insert(KeyBind::key(KeyCode::Enter), Action::ToggleItem);
        sidebar_edit.insert(KeyBind::char(' '), Action::ToggleItem);
        keymap.modes.insert(KeymapMode::SidebarEdit, sidebar_edit);

        // CommentList mode
        let mut comment_list = HashMap::new();
        comment_list.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        comment_list.insert(KeyBind::char('q'), Action::Back);
        comment_list.insert(KeyBind::char('j'), Action::MoveDown);
        comment_list.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        comment_list.insert(KeyBind::char('k'), Action::MoveUp);
        comment_list.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        comment_list.insert(KeyBind::char('e'), Action::EditComment);
        comment_list.insert(KeyBind::char('c'), Action::NewComment);
        comment_list.insert(KeyBind::char('r'), Action::OpenReactionPicker);
        keymap.modes.insert(KeymapMode::CommentList, comment_list);

        // GroupBySelect mode
        let mut group_by_select = HashMap::new();
        group_by_select.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        group_by_select.insert(KeyBind::char('q'), Action::Back);
        group_by_select.insert(KeyBind::char('j'), Action::MoveDown);
        group_by_select.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        group_by_select.insert(KeyBind::char('k'), Action::MoveUp);
        group_by_select.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        group_by_select.insert(KeyBind::key(KeyCode::Enter), Action::Select);
        keymap.modes.insert(KeymapMode::GroupBySelect, group_by_select);

        // ReactionPicker mode
        let mut reaction_picker = HashMap::new();
        reaction_picker.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        reaction_picker.insert(KeyBind::char('q'), Action::Back);
        reaction_picker.insert(KeyBind::char('h'), Action::MoveLeft);
        reaction_picker.insert(KeyBind::key(KeyCode::Left), Action::MoveLeft);
        reaction_picker.insert(KeyBind::char('l'), Action::MoveRight);
        reaction_picker.insert(KeyBind::key(KeyCode::Right), Action::MoveRight);
        reaction_picker.insert(KeyBind::key(KeyCode::Enter), Action::ToggleReaction);
        reaction_picker.insert(KeyBind::char(' '), Action::ToggleReaction);
        keymap.modes.insert(KeymapMode::ReactionPicker, reaction_picker);

        // ArchivedList mode
        let mut archived_list = HashMap::new();
        archived_list.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        archived_list.insert(KeyBind::char('q'), Action::Back);
        archived_list.insert(KeyBind::char('j'), Action::MoveDown);
        archived_list.insert(KeyBind::key(KeyCode::Down), Action::MoveDown);
        archived_list.insert(KeyBind::char('k'), Action::MoveUp);
        archived_list.insert(KeyBind::key(KeyCode::Up), Action::MoveUp);
        archived_list.insert(KeyBind::key(KeyCode::Enter), Action::OpenDetail);
        archived_list.insert(KeyBind::char('u'), Action::UnarchiveCard);
        archived_list.insert(KeyBind::char('r'), Action::Refresh);
        keymap.modes.insert(KeymapMode::ArchivedList, archived_list);

        // EditCard global keys
        let mut edit_card_global = HashMap::new();
        edit_card_global.insert(KeyBind::ctrl('s'), Action::Submit);
        edit_card_global.insert(KeyBind::key(KeyCode::Esc), Action::Back);
        edit_card_global.insert(KeyBind::key(KeyCode::Tab), Action::NextField);
        edit_card_global.insert(KeyBind::key(KeyCode::BackTab), Action::NextField);
        keymap.modes.insert(KeymapMode::EditCardGlobal, edit_card_global);

        // EditCard body field
        let mut edit_card_body = HashMap::new();
        edit_card_body.insert(KeyBind::key(KeyCode::Enter), Action::OpenEditor);
        keymap.modes.insert(KeymapMode::EditCardBody, edit_card_body);

        keymap
    }

    /// Merge user overrides on top of defaults.
    /// Each entry in `overrides` maps an action name to a list of key strings.
    /// When an action is overridden, all default bindings for that action in the specified mode
    /// are removed and replaced with the new bindings.
    pub fn with_overrides(mut self, keys_config: &crate::config::KeysConfig) -> Self {
        apply_mode_overrides(&mut self.global, &keys_config.global);

        let mode_configs: &[(&str, KeymapMode)] = &[
            ("board", KeymapMode::Board),
            ("project_select", KeymapMode::ProjectSelect),
            ("help", KeymapMode::Help),
            ("confirm", KeymapMode::Confirm),
            ("card_grab", KeymapMode::CardGrab),
            ("repo_select", KeymapMode::RepoSelect),
            ("detail_content", KeymapMode::DetailContent),
            ("detail_sidebar", KeymapMode::DetailSidebar),
            ("status_select", KeymapMode::StatusSelect),
            ("sidebar_edit", KeymapMode::SidebarEdit),
            ("comment_list", KeymapMode::CommentList),
            ("group_by_select", KeymapMode::GroupBySelect),
            ("reaction_picker", KeymapMode::ReactionPicker),
            ("archived_list", KeymapMode::ArchivedList),
            ("create_card_type", KeymapMode::CreateCardType),
            ("create_card_body", KeymapMode::CreateCardBody),
            ("edit_card_body", KeymapMode::EditCardBody),
            ("filter", KeymapMode::FilterStructural),
            ("create_card", KeymapMode::CreateCardGlobal),
            ("edit_card", KeymapMode::EditCardGlobal),
        ];

        for (config_key, mode) in mode_configs {
            let section = keys_config.section(config_key);
            if !section.is_empty() {
                let mode_map = self.modes.entry(*mode).or_default();
                apply_mode_overrides(mode_map, &section);
            }
        }

        self
    }

    /// Get all bindings for a given action in a specific mode (mode + global fallback).
    pub fn bindings_for_action(&self, mode: KeymapMode, action: Action) -> Vec<&KeyBind> {
        let mut result = Vec::new();
        // Collect from mode-specific
        if let Some(mode_map) = self.modes.get(&mode) {
            for (bind, act) in mode_map {
                if *act == action {
                    result.push(bind);
                }
            }
        }
        // Collect from global (only if not shadowed by mode)
        for (bind, act) in &self.global {
            if *act == action {
                let shadowed = self
                    .modes
                    .get(&mode)
                    .is_some_and(|m| m.contains_key(bind));
                if !shadowed {
                    result.push(bind);
                }
            }
        }
        result
    }

    /// Get all bindings for a mode, grouped by action.
    #[allow(dead_code)]
    pub fn bindings_for_mode(&self, mode: KeymapMode) -> HashMap<Action, Vec<KeyBind>> {
        let mut result: HashMap<Action, Vec<KeyBind>> = HashMap::new();

        // Add global bindings first
        for (bind, action) in &self.global {
            result.entry(*action).or_default().push(bind.clone());
        }

        // Mode-specific override/add
        if let Some(mode_map) = self.modes.get(&mode) {
            // Remove global bindings that are shadowed
            for bind in mode_map.keys() {
                for (_, binds) in result.iter_mut() {
                    binds.retain(|b| b != bind);
                }
            }
            // Add mode-specific
            for (bind, action) in mode_map {
                result.entry(*action).or_default().push(bind.clone());
            }
        }

        // Remove empty entries
        result.retain(|_, v| !v.is_empty());
        result
    }
}

/// Apply overrides: for each action in the override map, remove all existing bindings for that
/// action and add the new ones.
fn apply_mode_overrides(
    bindings: &mut HashMap<KeyBind, Action>,
    overrides: &HashMap<String, Vec<String>>,
) {
    for (action_name, key_strs) in overrides {
        let action = match parse_action_name(action_name) {
            Some(a) => a,
            None => continue, // skip unknown actions
        };

        // Remove all existing bindings for this action
        bindings.retain(|_, a| *a != action);

        // Add new bindings
        for key_str in key_strs {
            if let Ok(bind) = KeyBind::parse(key_str) {
                bindings.insert(bind, action);
            }
        }
    }
}

fn parse_action_name(name: &str) -> Option<Action> {
    match name {
        "quit" => Some(Action::Quit),
        "force_quit" => Some(Action::ForceQuit),
        "back" => Some(Action::Back),
        "move_down" => Some(Action::MoveDown),
        "move_up" => Some(Action::MoveUp),
        "move_left" => Some(Action::MoveLeft),
        "move_right" => Some(Action::MoveRight),
        "first_item" => Some(Action::FirstItem),
        "last_item" => Some(Action::LastItem),
        "next_tab" => Some(Action::NextTab),
        "prev_tab" => Some(Action::PrevTab),
        "open_detail" => Some(Action::OpenDetail),
        "grab_card" => Some(Action::GrabCard),
        "new_card" => Some(Action::NewCard),
        "archive_card" => Some(Action::ArchiveCard),
        "unarchive_card" => Some(Action::UnarchiveCard),
        "show_archived_list" => Some(Action::ShowArchivedList),
        "start_filter" => Some(Action::StartFilter),
        "clear_filter" => Some(Action::ClearFilter),
        "refresh" => Some(Action::Refresh),
        "show_help" => Some(Action::ShowHelp),
        "switch_project" => Some(Action::SwitchProject),
        "change_grouping" => Some(Action::ChangeGrouping),
        "open_in_browser" => Some(Action::OpenInBrowser),
        "edit_card" => Some(Action::EditCard),
        "new_comment" => Some(Action::NewComment),
        "open_comment_list" => Some(Action::OpenCommentList),
        "select" => Some(Action::Select),
        "confirm_yes" => Some(Action::ConfirmYes),
        "confirm_no" => Some(Action::ConfirmNo),
        "confirm_grab" => Some(Action::ConfirmGrab),
        "cancel_grab" => Some(Action::CancelGrab),
        "edit_comment" => Some(Action::EditComment),
        "submit" => Some(Action::Submit),
        "next_field" => Some(Action::NextField),
        "prev_field" => Some(Action::PrevField),
        "toggle_type" => Some(Action::ToggleType),
        "open_editor" => Some(Action::OpenEditor),
        "toggle_item" => Some(Action::ToggleItem),
        "open_reaction_picker" => Some(Action::OpenReactionPicker),
        "toggle_reaction" => Some(Action::ToggleReaction),
        _ => None,
    }
}

/// Convert an Action to its config name (for display/serialization)
#[allow(dead_code)]
pub fn action_name(action: Action) -> &'static str {
    match action {
        Action::Quit => "quit",
        Action::ForceQuit => "force_quit",
        Action::Back => "back",
        Action::MoveDown => "move_down",
        Action::MoveUp => "move_up",
        Action::MoveLeft => "move_left",
        Action::MoveRight => "move_right",
        Action::FirstItem => "first_item",
        Action::LastItem => "last_item",
        Action::NextTab => "next_tab",
        Action::PrevTab => "prev_tab",
        Action::OpenDetail => "open_detail",
        Action::GrabCard => "grab_card",
        Action::NewCard => "new_card",
        Action::ArchiveCard => "archive_card",
        Action::UnarchiveCard => "unarchive_card",
        Action::ShowArchivedList => "show_archived_list",
        Action::StartFilter => "start_filter",
        Action::ClearFilter => "clear_filter",
        Action::Refresh => "refresh",
        Action::ShowHelp => "show_help",
        Action::SwitchProject => "switch_project",
        Action::ChangeGrouping => "change_grouping",
        Action::OpenInBrowser => "open_in_browser",
        Action::EditCard => "edit_card",
        Action::NewComment => "new_comment",
        Action::OpenCommentList => "open_comment_list",
        Action::Select => "select",
        Action::ConfirmYes => "confirm_yes",
        Action::ConfirmNo => "confirm_no",
        Action::ConfirmGrab => "confirm_grab",
        Action::CancelGrab => "cancel_grab",
        Action::EditComment => "edit_comment",
        Action::Submit => "submit",
        Action::NextField => "next_field",
        Action::PrevField => "prev_field",
        Action::ToggleType => "toggle_type",
        Action::OpenEditor => "open_editor",
        Action::ToggleItem => "toggle_item",
        Action::OpenReactionPicker => "open_reaction_picker",
        Action::ToggleReaction => "toggle_reaction",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    // ========== KeyBind::parse tests ==========

    #[test]
    fn test_parse_simple_char() {
        assert_eq!(KeyBind::parse("j").unwrap(), KeyBind::char('j'));
        assert_eq!(KeyBind::parse("q").unwrap(), KeyBind::char('q'));
        assert_eq!(KeyBind::parse("H").unwrap(), KeyBind::char('H'));
    }

    #[test]
    fn test_parse_special_keys() {
        assert_eq!(KeyBind::parse("Enter").unwrap(), KeyBind::key(KeyCode::Enter));
        assert_eq!(KeyBind::parse("Esc").unwrap(), KeyBind::key(KeyCode::Esc));
        assert_eq!(KeyBind::parse("Tab").unwrap(), KeyBind::key(KeyCode::Tab));
        assert_eq!(KeyBind::parse("Space").unwrap(), KeyBind::char(' '));
        assert_eq!(KeyBind::parse("Backspace").unwrap(), KeyBind::key(KeyCode::Backspace));
        assert_eq!(KeyBind::parse("Up").unwrap(), KeyBind::key(KeyCode::Up));
        assert_eq!(KeyBind::parse("Down").unwrap(), KeyBind::key(KeyCode::Down));
        assert_eq!(KeyBind::parse("Left").unwrap(), KeyBind::key(KeyCode::Left));
        assert_eq!(KeyBind::parse("Right").unwrap(), KeyBind::key(KeyCode::Right));
    }

    #[test]
    fn test_parse_ctrl_modifier() {
        assert_eq!(KeyBind::parse("C-c").unwrap(), KeyBind::ctrl('c'));
        assert_eq!(KeyBind::parse("C-u").unwrap(), KeyBind::ctrl('u'));
        assert_eq!(KeyBind::parse("C-s").unwrap(), KeyBind::ctrl('s'));
    }

    #[test]
    fn test_parse_shift_modifier() {
        let expected = KeyBind::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(KeyBind::parse("S-Tab").unwrap(), expected);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(KeyBind::parse("X-c").is_err());
        assert!(KeyBind::parse("FooBar").is_err());
    }

    // ========== KeyBind::Display tests ==========

    #[test]
    fn test_display_simple() {
        assert_eq!(KeyBind::char('j').to_string(), "j");
        assert_eq!(KeyBind::char('H').to_string(), "H");
    }

    #[test]
    fn test_display_special() {
        assert_eq!(KeyBind::key(KeyCode::Enter).to_string(), "Enter");
        assert_eq!(KeyBind::key(KeyCode::Esc).to_string(), "Esc");
        assert_eq!(KeyBind::char(' ').to_string(), "Space");
    }

    #[test]
    fn test_display_ctrl() {
        assert_eq!(KeyBind::ctrl('c').to_string(), "C-c");
        assert_eq!(KeyBind::ctrl('u').to_string(), "C-u");
    }

    #[test]
    fn test_display_backtab() {
        assert_eq!(KeyBind::key(KeyCode::BackTab).to_string(), "S-Tab");
    }

    // ========== Keymap::resolve tests ==========

    #[test]
    fn test_resolve_board_navigation() {
        let keymap = Keymap::default_keymap();
        let key = make_key_event(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), Some(Action::MoveDown));

        let key = make_key_event(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), Some(Action::MoveDown));

        let key = make_key_event(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), Some(Action::MoveUp));

        let key = make_key_event(KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), Some(Action::MoveLeft));

        let key = make_key_event(KeyCode::Char('l'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), Some(Action::MoveRight));
    }

    #[test]
    fn test_resolve_board_actions() {
        let keymap = Keymap::default_keymap();

        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(Action::ArchiveCard)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('v'), KeyModifiers::NONE)),
            Some(Action::ShowArchivedList)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(Action::NewCard)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(Action::GrabCard)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Action::OpenDetail)
        );
    }

    #[test]
    fn test_resolve_global_force_quit() {
        let keymap = Keymap::default_keymap();
        let key = make_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL);
        // ForceQuit is global, should work in any mode
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), Some(Action::ForceQuit));
        assert_eq!(keymap.resolve(KeymapMode::ProjectSelect, &key), Some(Action::ForceQuit));
        assert_eq!(keymap.resolve(KeymapMode::DetailContent, &key), Some(Action::ForceQuit));
    }

    #[test]
    fn test_resolve_mode_overrides_global() {
        let keymap = Keymap::default_keymap();
        // 'q' in Board = Quit (mode-specific), not from global
        let key = make_key_event(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), Some(Action::Quit));
    }

    #[test]
    fn test_resolve_unknown_key() {
        let keymap = Keymap::default_keymap();
        let key = make_key_event(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(KeymapMode::Board, &key), None);
    }

    #[test]
    fn test_resolve_detail_content() {
        let keymap = Keymap::default_keymap();
        assert_eq!(
            keymap.resolve(KeymapMode::DetailContent, &make_key_event(KeyCode::Char('e'), KeyModifiers::NONE)),
            Some(Action::EditCard)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::DetailContent, &make_key_event(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(Action::NewComment)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::DetailContent, &make_key_event(KeyCode::Char('C'), KeyModifiers::NONE)),
            Some(Action::OpenCommentList)
        );
    }

    #[test]
    fn test_resolve_card_grab() {
        let keymap = Keymap::default_keymap();
        assert_eq!(
            keymap.resolve(KeymapMode::CardGrab, &make_key_event(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(Action::ConfirmGrab)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::CardGrab, &make_key_event(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Action::CancelGrab)
        );
    }

    #[test]
    fn test_resolve_confirm() {
        let keymap = Keymap::default_keymap();
        assert_eq!(
            keymap.resolve(KeymapMode::Confirm, &make_key_event(KeyCode::Char('y'), KeyModifiers::NONE)),
            Some(Action::ConfirmYes)
        );
        assert_eq!(
            keymap.resolve(KeymapMode::Confirm, &make_key_event(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(Action::ConfirmNo)
        );
    }

    // ========== Keymap::with_overrides tests ==========

    #[test]
    fn test_override_board_keys() {
        let keymap = Keymap::default_keymap();
        let mut overrides = crate::config::KeysConfig::default();
        overrides.board.insert("move_down".into(), vec!["n".into()]);

        let keymap = keymap.with_overrides(&overrides);

        // 'n' should now be MoveDown in Board
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(Action::MoveDown)
        );
        // 'j' should no longer be MoveDown (removed by override)
        // But Down arrow should still work if not overridden via separate entry
        // Note: override only removes bindings for the overridden action
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('j'), KeyModifiers::NONE)),
            None
        );
    }

    #[test]
    fn test_override_preserves_other_actions() {
        let keymap = Keymap::default_keymap();
        let mut overrides = crate::config::KeysConfig::default();
        overrides.board.insert("move_down".into(), vec!["n".into()]);

        let keymap = keymap.with_overrides(&overrides);

        // move_up should be unchanged
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('k'), KeyModifiers::NONE)),
            Some(Action::MoveUp)
        );
    }

    #[test]
    fn test_override_global() {
        let keymap = Keymap::default_keymap();
        let mut overrides = crate::config::KeysConfig::default();
        overrides.global.insert("force_quit".into(), vec!["C-q".into()]);

        let keymap = keymap.with_overrides(&overrides);

        // C-c should no longer be ForceQuit
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            None
        );
        // C-q should be ForceQuit
        assert_eq!(
            keymap.resolve(KeymapMode::Board, &make_key_event(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            Some(Action::ForceQuit)
        );
    }

    // ========== bindings_for_action tests ==========

    #[test]
    fn test_bindings_for_action() {
        let keymap = Keymap::default_keymap();
        let bindings = keymap.bindings_for_action(KeymapMode::Board, Action::MoveDown);
        assert!(bindings.contains(&&KeyBind::char('j')));
        assert!(bindings.contains(&&KeyBind::key(KeyCode::Down)));
    }

    #[test]
    fn test_bindings_for_action_after_override() {
        let keymap = Keymap::default_keymap();
        let mut overrides = crate::config::KeysConfig::default();
        overrides.board.insert("refresh".into(), vec!["R".into()]);
        overrides.board.insert("start_filter".into(), vec!["/".into(), "f".into()]);

        let keymap = keymap.with_overrides(&overrides);

        // refresh should now show R, not r
        let refresh_binds = keymap.bindings_for_action(KeymapMode::Board, Action::Refresh);
        assert!(refresh_binds.contains(&&KeyBind::char('R')));
        assert!(!refresh_binds.contains(&&KeyBind::char('r')));

        // start_filter should show / and f
        let filter_binds = keymap.bindings_for_action(KeymapMode::Board, Action::StartFilter);
        assert!(filter_binds.contains(&&KeyBind::char('/')));
        assert!(filter_binds.contains(&&KeyBind::char('f')));
    }
}
