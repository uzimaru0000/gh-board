use super::*;
use crate::command::CustomFieldValueInput;
use crate::model::project::Grouping;

impl AppState {
    pub(super) fn enter_bulk_select(&mut self) {
        self.bulk_selected.clear();
        self.mode = ViewMode::BulkSelect;
        self.scene = Scene::BulkSelect;
    }

    pub(super) fn exit_bulk_select(&mut self) {
        self.bulk_selected.clear();
        self.mode = ViewMode::Board;
        self.scene = scene_from_mode_tag(&self.mode);
    }

    /// 現在選択中のカードを bulk_selected に追加/削除する。
    /// フィルタ適用時は real_card_index() 経由で実カードを参照する。
    /// Table layout では table_selected_row を基準に解決する。
    pub(super) fn bulk_toggle_current_card(&mut self) {
        let Some(item_id) = self.current_card_item_id() else {
            return;
        };
        if !self.bulk_selected.remove(&item_id) {
            self.bulk_selected.insert(item_id);
        }
    }

    /// 現在のカーソル位置にあるカードの item_id を返す (layout 対応)。
    fn current_card_item_id(&self) -> Option<String> {
        let board = self.board.as_ref()?;
        if self.current_layout == LayoutMode::Table {
            let rows = self.table_rows();
            let (ci, ri) = *rows.get(self.table_selected_row)?;
            return board
                .columns
                .get(ci)
                .and_then(|c| c.cards.get(ri))
                .map(|card| card.item_id.clone());
        }
        let real_idx = self.real_card_index()?;
        board
            .columns
            .get(self.selected_column)
            .and_then(|c| c.cards.get(real_idx))
            .map(|card| card.item_id.clone())
    }

    /// 現在の表示範囲 (Board ならカラム、Table なら全行) の全カードを bulk_selected に追加。
    pub(super) fn bulk_select_current_column(&mut self) {
        let Some(board) = &self.board else {
            return;
        };
        if self.current_layout == LayoutMode::Table {
            let rows = self.table_rows();
            for (ci, ri) in rows {
                if let Some(card) = board.columns.get(ci).and_then(|c| c.cards.get(ri)) {
                    self.bulk_selected.insert(card.item_id.clone());
                }
            }
            return;
        }
        let Some(col) = board.columns.get(self.selected_column) else {
            return;
        };
        for card in &col.cards {
            if self
                .filter
                .active_filter
                .as_ref()
                .is_none_or(|f| f.matches(card))
            {
                self.bulk_selected.insert(card.item_id.clone());
            }
        }
    }

    /// BulkArchive を Confirm 経由で発行するための遷移。
    /// 選択が空なら何もしない。確定時には Batch(ArchiveCard ...) が発行される。
    pub(super) fn start_bulk_archive(&mut self) {
        if self.bulk_selected.is_empty() {
            return;
        }
        let item_ids: Vec<String> = self.bulk_selected.iter().cloned().collect();
        let title = format!("{} cards", item_ids.len());
        self.enter_confirm(ConfirmState {
            action: ConfirmAction::ArchiveMultipleCards { item_ids },
            title,
            return_to: ViewMode::Board,
        });
    }

    /// 選択済みカードを target_column に一括で移動する。
    /// grouping が SingleSelect でない場合は no-op。
    /// 楽観的に board を書き換え、Batch(MoveCard ...) を返す。
    pub(super) fn bulk_move_selected_to(&mut self, direction: BulkMoveDirection) -> Command {
        if self.bulk_selected.is_empty() {
            return Command::None;
        }
        let Some(project_id) = self.current_project.as_ref().map(|p| p.id.clone()) else {
            return Command::None;
        };
        let board = match &self.board {
            Some(b) => b,
            None => return Command::None,
        };
        let (field_id, _) = match &board.grouping {
            Grouping::SingleSelect {
                field_id,
                field_name,
            } => (field_id.clone(), field_name.clone()),
            _ => return Command::None,
        };

        // 現在の各カードのカラムを走査し、隣接カラムへの移動先 option_id を解決する。
        // "No <field>" (空 option_id) には移動しない。
        let mut moves: Vec<(String, String)> = Vec::new();
        let board_mut = self.board.as_mut().unwrap();
        // HashSet は順序未定義なので、items を一度 Vec にスナップショット
        let targets: Vec<String> = self.bulk_selected.iter().cloned().collect();
        for item_id in &targets {
            let Some((src_col, card_idx)) = find_card_position(board_mut, item_id) else {
                continue;
            };
            let target_col = match direction {
                BulkMoveDirection::Left if src_col > 0 => src_col - 1,
                BulkMoveDirection::Right if src_col + 1 < board_mut.columns.len() => src_col + 1,
                _ => continue,
            };
            let target_option_id = board_mut.columns[target_col].option_id.clone();
            if target_option_id.is_empty() {
                continue;
            }
            // 楽観的更新: カードを移動 + custom_fields の Status を書き換え
            let mut card = board_mut.columns[src_col].cards.remove(card_idx);
            card.custom_fields.retain(|fv| fv.field_id() != field_id);
            if let Grouping::SingleSelect {
                field_id: fid,
                field_name: fname,
            } = &board_mut.grouping
            {
                let (name, color) = board_mut
                    .field_definitions
                    .iter()
                    .find_map(|d| match d {
                        FieldDefinition::SingleSelect { id, options, .. } if id == fid => options
                            .iter()
                            .find(|o| o.id == target_option_id)
                            .map(|o| (o.name.clone(), o.color.clone())),
                        _ => None,
                    })
                    .unwrap_or_default();
                card.custom_fields.push(CustomFieldValue::SingleSelect {
                    field_id: fid.clone(),
                    field_name: fname.clone(),
                    option_id: target_option_id.clone(),
                    name,
                    color,
                });
            }
            board_mut.columns[target_col].cards.push(card);
            moves.push((item_id.clone(), target_option_id));
        }

        if moves.is_empty() {
            return Command::None;
        }

        let cmds: Vec<Command> = moves
            .into_iter()
            .map(|(item_id, option_id)| Command::MoveCard {
                project_id: project_id.clone(),
                item_id,
                field_id: field_id.clone(),
                value: CustomFieldValueInput::SingleSelect { option_id },
            })
            .collect();
        if cmds.len() == 1 {
            cmds.into_iter().next().unwrap()
        } else {
            Command::Batch(cmds)
        }
    }

    pub(super) fn handle_bulk_select_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::BulkSelect, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        // ForceQuit は global で解決されるがここでも明示
        if matches!(action, Action::ForceQuit) {
            self.should_quit = true;
            return Command::None;
        }

        let is_table = self.current_layout == LayoutMode::Table;
        let current_col_len = self.filtered_card_indices(self.selected_column).len();
        let col_count = self.board.as_ref().map(|b| b.columns.len()).unwrap_or(0);
        let row_count = if is_table { self.table_rows().len() } else { 0 };

        match action {
            Action::MoveDown => {
                if is_table {
                    if row_count > 0 {
                        self.table_selected_row =
                            (self.table_selected_row + 1).min(row_count - 1);
                    }
                } else if current_col_len > 0 {
                    self.selected_card = (self.selected_card + 1).min(current_col_len - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                if is_table {
                    self.table_selected_row = self.table_selected_row.saturating_sub(1);
                } else {
                    self.selected_card = self.selected_card.saturating_sub(1);
                }
                Command::None
            }
            Action::MoveLeft => {
                if !is_table && self.selected_column > 0 {
                    self.selected_column -= 1;
                    self.clamp_card_selection();
                }
                Command::None
            }
            Action::MoveRight => {
                if !is_table && col_count > 0 && self.selected_column + 1 < col_count {
                    self.selected_column += 1;
                    self.clamp_card_selection();
                }
                Command::None
            }
            Action::BulkSelectToggle => {
                self.bulk_toggle_current_card();
                Command::None
            }
            Action::BulkSelectAll => {
                self.bulk_select_current_column();
                Command::None
            }
            Action::BulkSelectClear => {
                self.exit_bulk_select();
                Command::None
            }
            Action::BulkArchive => {
                self.start_bulk_archive();
                Command::None
            }
            Action::BulkMoveLeft => {
                let cmd = self.bulk_move_selected_to(BulkMoveDirection::Left);
                // 選択解除 + BulkSelect のまま維持 (複数段階の操作を可能に)
                self.bulk_selected.clear();
                cmd
            }
            Action::BulkMoveRight => {
                let cmd = self.bulk_move_selected_to(BulkMoveDirection::Right);
                self.bulk_selected.clear();
                cmd
            }
            _ => Command::None,
        }
    }

    /// 選択済みカードを一括アーカイブする Command を構築する (Confirm 確定時に呼ぶ)。
    /// 楽観的に board からも削除する。
    pub(super) fn archive_multiple_cards(&mut self, item_ids: &[String]) -> Command {
        let Some(project_id) = self.current_project.as_ref().map(|p| p.id.clone()) else {
            return Command::None;
        };
        if let Some(board) = &mut self.board {
            for item_id in item_ids {
                for col in &mut board.columns {
                    if let Some(pos) = col.cards.iter().position(|c| c.item_id == *item_id) {
                        col.cards.remove(pos);
                        break;
                    }
                }
            }
        }
        self.clamp_card_selection();
        self.bulk_selected.clear();

        let cmds: Vec<Command> = item_ids
            .iter()
            .map(|item_id| Command::ArchiveCard {
                project_id: project_id.clone(),
                item_id: item_id.clone(),
            })
            .collect();
        if cmds.is_empty() {
            Command::None
        } else if cmds.len() == 1 {
            cmds.into_iter().next().unwrap()
        } else {
            Command::Batch(cmds)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BulkMoveDirection {
    Left,
    Right,
}

fn find_card_position(board: &Board, item_id: &str) -> Option<(usize, usize)> {
    for (ci, col) in board.columns.iter().enumerate() {
        if let Some(idx) = col.cards.iter().position(|c| c.item_id == item_id) {
            return Some((ci, idx));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::{Card, CardType, Column, ColumnColor, FieldDefinition, Grouping, SingleSelectOption};
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_shift(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_card(item_id: &str) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: None,
            title: format!("card-{item_id}"),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
            custom_fields: vec![],
            pr_status: None,
            linked_prs: vec![],
            reactions: vec![],
            archived: false,
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: vec![],
        }
    }

    fn make_board() -> Board {
        Board {
            project_title: "p".into(),
            grouping: Grouping::SingleSelect {
                field_id: "F".into(),
                field_name: "Status".into(),
            },
            columns: vec![
                Column {
                    option_id: "todo".into(),
                    name: "Todo".into(),
                    color: Some(ColumnColor::Green),
                    cards: vec![make_card("a"), make_card("b"), make_card("c")],
                },
                Column {
                    option_id: "doing".into(),
                    name: "Doing".into(),
                    color: Some(ColumnColor::Yellow),
                    cards: vec![make_card("d")],
                },
            ],
            repositories: vec![],
            field_definitions: vec![FieldDefinition::SingleSelect {
                id: "F".into(),
                name: "Status".into(),
                options: vec![
                    SingleSelectOption {
                        id: "todo".into(),
                        name: "Todo".into(),
                        color: Some(ColumnColor::Green),
                    },
                    SingleSelectOption {
                        id: "doing".into(),
                        name: "Doing".into(),
                        color: Some(ColumnColor::Yellow),
                    },
                ],
            }],
        }
    }

    fn make_state() -> AppState {
        let mut state = AppState::new(None);
        state.board = Some(make_board());
        state.current_project = Some(crate::model::project::ProjectSummary {
            id: "PRJ".into(),
            title: "p".into(),
            number: 1,
            description: None,
            url: "https://example.com/projects/1".into(),
        });
        state.mode = ViewMode::Board;
        state
    }

    #[test]
    fn bulk_select_start_enters_mode_and_resets() {
        let mut state = make_state();
        state.bulk_selected.insert("stale".into());
        state.enter_bulk_select();
        assert_eq!(state.mode, ViewMode::BulkSelect);
        assert!(state.bulk_selected.is_empty());
    }

    #[test]
    fn bulk_select_toggle_adds_and_removes() {
        let mut state = make_state();
        state.enter_bulk_select();
        state.selected_column = 0;
        state.selected_card = 0;
        let cmd = state.handle_bulk_select_key(key(KeyCode::Char(' ')));
        assert_eq!(cmd, Command::None);
        assert!(state.bulk_selected.contains("a"));

        // もう一度 Space → 解除
        let _ = state.handle_bulk_select_key(key(KeyCode::Char(' ')));
        assert!(!state.bulk_selected.contains("a"));
    }

    #[test]
    fn bulk_select_movement_works() {
        let mut state = make_state();
        state.enter_bulk_select();
        state.selected_column = 0;
        state.selected_card = 0;
        let _ = state.handle_bulk_select_key(key(KeyCode::Char('j')));
        assert_eq!(state.selected_card, 1);
        let _ = state.handle_bulk_select_key(key(KeyCode::Char('l')));
        assert_eq!(state.selected_column, 1);
    }

    #[test]
    fn bulk_select_clear_returns_to_board() {
        let mut state = make_state();
        state.enter_bulk_select();
        state.bulk_selected.insert("a".into());
        let _ = state.handle_bulk_select_key(key(KeyCode::Esc));
        assert_eq!(state.mode, ViewMode::Board);
        assert!(state.bulk_selected.is_empty());
    }

    #[test]
    fn bulk_select_all_selects_current_column() {
        let mut state = make_state();
        state.enter_bulk_select();
        state.selected_column = 0;
        let _ = state.handle_bulk_select_key(key(KeyCode::Char('*')));
        assert_eq!(state.bulk_selected.len(), 3);
        assert!(state.bulk_selected.contains("a"));
        assert!(state.bulk_selected.contains("b"));
        assert!(state.bulk_selected.contains("c"));
    }

    #[test]
    fn bulk_move_right_emits_move_commands_and_updates_board() {
        let mut state = make_state();
        state.enter_bulk_select();
        state.bulk_selected.insert("a".into());
        state.bulk_selected.insert("b".into());
        let cmd = state.handle_bulk_select_key(key_shift('L'));
        // 楽観的に board から移動済み
        let board = state.board.as_ref().unwrap();
        let todo = &board.columns[0].cards;
        let doing = &board.columns[1].cards;
        assert!(todo.iter().all(|c| c.item_id != "a" && c.item_id != "b"));
        assert_eq!(doing.len(), 3); // d + a + b
        // Command は Batch(MoveCard x 2) もしくは MoveCard (1 件の場合)
        match cmd {
            Command::Batch(cmds) => assert_eq!(cmds.len(), 2),
            _ => panic!("expected Batch"),
        }
        // 選択はクリアされる
        assert!(state.bulk_selected.is_empty());
    }

    #[test]
    fn bulk_move_left_skips_when_no_left_column() {
        let mut state = make_state();
        state.enter_bulk_select();
        state.bulk_selected.insert("a".into()); // すでに Todo (一番左)
        let cmd = state.handle_bulk_select_key(key_shift('H'));
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn bulk_archive_goes_through_confirm() {
        let mut state = make_state();
        state.enter_bulk_select();
        state.bulk_selected.insert("a".into());
        state.bulk_selected.insert("b".into());
        let cmd = state.handle_bulk_select_key(key(KeyCode::Char('a')));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::Confirm);
    }
}
