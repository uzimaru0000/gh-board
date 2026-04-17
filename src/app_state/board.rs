use super::*;

impl AppState {
    pub(super) fn handle_board_key(&mut self, key: KeyEvent) -> Command {
        if self.current_layout == LayoutMode::Table {
            return self.handle_table_key(key);
        }
        if self.current_layout == LayoutMode::Roadmap {
            return self.handle_roadmap_key(key);
        }

        let board = match &self.board {
            Some(b) => b,
            None => return Command::None,
        };

        if board.columns.is_empty() {
            match self.keymap.resolve(KeymapMode::Board, &key) {
                Some(Action::Quit) | Some(Action::ForceQuit) => self.should_quit = true,
                Some(Action::SwitchProject) => return self.enter_project_select(),
                Some(Action::ShowHelp) => self.mode = ViewMode::Help,
                _ => {}
            }
            return Command::None;
        }

        if let Some(cmd) = self.try_handle_view_switch(&key) {
            return cmd;
        }

        let action = match self.keymap.resolve(KeymapMode::Board, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        if let Some(cmd) = self.try_handle_common_board_action(action) {
            return cmd;
        }

        let current_col_len = self.filtered_card_indices(self.selected_column).len();
        let col_count = self.board.as_ref().map(|b| b.columns.len()).unwrap_or(0);

        match action {
            Action::MoveDown => {
                if current_col_len > 0 {
                    self.selected_card = (self.selected_card + 1).min(current_col_len - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                self.selected_card = self.selected_card.saturating_sub(1);
                Command::None
            }
            Action::MoveLeft => {
                if self.selected_column > 0 {
                    self.selected_column -= 1;
                    self.clamp_card_selection();
                }
                Command::None
            }
            Action::MoveRight => {
                if self.selected_column < col_count - 1 {
                    self.selected_column += 1;
                    self.clamp_card_selection();
                }
                Command::None
            }
            Action::FirstItem => {
                self.selected_card = 0;
                Command::None
            }
            Action::LastItem => {
                if current_col_len > 0 {
                    self.selected_card = current_col_len - 1;
                }
                Command::None
            }
            Action::NextTab => {
                self.selected_column = (self.selected_column + 1) % col_count;
                self.clamp_card_selection();
                Command::None
            }
            Action::PrevTab => {
                if self.selected_column == 0 {
                    self.selected_column = col_count - 1;
                } else {
                    self.selected_column -= 1;
                }
                self.clamp_card_selection();
                Command::None
            }
            Action::OpenDetail => self.open_detail_view(),
            Action::ClearFilter => {
                self.active_view = None;
                self.filter.active_filter = None;
                self.clamp_card_selection();
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            Action::ArchiveCard => {
                self.start_archive_card(ViewMode::Board);
                Command::None
            }
            Action::GrabCard => {
                if let Some(real_idx) = self.real_card_index() {
                    let item_id = self.board.as_ref().unwrap().columns[self.selected_column]
                        .cards[real_idx]
                        .item_id
                        .clone();
                    self.grab_state = Some(GrabState {
                        origin_column: self.selected_column,
                        origin_card_index: real_idx,
                        item_id,
                    });
                    self.mode = ViewMode::CardGrab;
                }
                Command::None
            }
            _ => Command::None,
        }
    }

    /// Table view 用キーハンドラ。LayoutMode::Table 中の handle_board_key から呼ばれる。
    /// 行ベース (table_selected_row) でナビゲーションし、Detail/Archive/Grab 等の遷移時には
    /// `set_selection_from_table_row` で Board 用の (selected_column, selected_card) を埋め直してから
    /// 既存ロジックを再利用する。
    pub(super) fn handle_table_key(&mut self, key: KeyEvent) -> Command {
        let board = match &self.board {
            Some(b) => b,
            None => return Command::None,
        };

        if board.columns.is_empty() {
            match self.keymap.resolve(KeymapMode::Table, &key) {
                Some(Action::Quit) | Some(Action::ForceQuit) => self.should_quit = true,
                Some(Action::SwitchProject) => return self.enter_project_select(),
                Some(Action::ShowHelp) => self.mode = ViewMode::Help,
                Some(Action::ToggleLayout) => self.toggle_layout(),
                _ => {}
            }
            return Command::None;
        }

        if let Some(cmd) = self.try_handle_view_switch(&key) {
            return cmd;
        }

        let action = match self.keymap.resolve(KeymapMode::Table, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        if let Some(cmd) = self.try_handle_common_board_action(action) {
            return cmd;
        }

        let row_count = self.table_rows().len();

        match action {
            Action::MoveDown => {
                if row_count > 0 {
                    self.table_selected_row = (self.table_selected_row + 1).min(row_count - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                self.table_selected_row = self.table_selected_row.saturating_sub(1);
                Command::None
            }
            Action::FirstItem => {
                self.table_selected_row = 0;
                Command::None
            }
            Action::LastItem => {
                if row_count > 0 {
                    self.table_selected_row = row_count - 1;
                }
                Command::None
            }
            Action::OpenDetail => {
                self.set_selection_from_table_row(self.table_selected_row);
                self.open_detail_view()
            }
            Action::ClearFilter => {
                self.active_view = None;
                self.filter.active_filter = None;
                self.table_selected_row = 0;
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            Action::ArchiveCard => {
                self.set_selection_from_table_row(self.table_selected_row);
                self.start_archive_card(ViewMode::Board);
                Command::None
            }
            Action::GrabCard => {
                self.set_selection_from_table_row(self.table_selected_row);
                if let Some(real_idx) = self.real_card_index() {
                    let item_id = self.board.as_ref().unwrap().columns[self.selected_column]
                        .cards[real_idx]
                        .item_id
                        .clone();
                    self.grab_state = Some(GrabState {
                        origin_column: self.selected_column,
                        origin_card_index: real_idx,
                        item_id,
                    });
                    self.mode = ViewMode::CardGrab;
                }
                Command::None
            }
            _ => Command::None,
        }
    }

    /// Roadmap layout のキーハンドラ。
    /// 行ベース (roadmap_selected_row) でナビゲーションし、Detail/Archive 等の遷移時は
    /// `set_selection_from_roadmap_row` で Board 用の (selected_column, selected_card) を
    /// 書き戻してから既存ロジックを再利用する。
    /// Iteration / Date の期間編集は MVP スコープ外のため Space / h / l は no-op。
    pub(super) fn handle_roadmap_key(&mut self, key: KeyEvent) -> Command {
        let board = match &self.board {
            Some(b) => b,
            None => return Command::None,
        };

        if board.columns.is_empty() {
            match self.keymap.resolve(KeymapMode::Roadmap, &key) {
                Some(Action::Quit) | Some(Action::ForceQuit) => self.should_quit = true,
                Some(Action::SwitchProject) => return self.enter_project_select(),
                Some(Action::ShowHelp) => self.mode = ViewMode::Help,
                Some(Action::ToggleLayout) => self.toggle_layout(),
                _ => {}
            }
            return Command::None;
        }

        if let Some(cmd) = self.try_handle_view_switch(&key) {
            return cmd;
        }

        let action = match self.keymap.resolve(KeymapMode::Roadmap, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        if let Some(cmd) = self.try_handle_common_board_action(action) {
            return cmd;
        }

        let row_count = self.roadmap_rows().len();

        match action {
            Action::MoveDown => {
                if row_count > 0 {
                    self.roadmap_selected_row =
                        (self.roadmap_selected_row + 1).min(row_count - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                self.roadmap_selected_row = self.roadmap_selected_row.saturating_sub(1);
                Command::None
            }
            Action::FirstItem => {
                self.roadmap_selected_row = 0;
                Command::None
            }
            Action::LastItem => {
                if row_count > 0 {
                    self.roadmap_selected_row = row_count - 1;
                }
                Command::None
            }
            Action::OpenDetail => {
                self.set_selection_from_roadmap_row(self.roadmap_selected_row);
                self.open_detail_view()
            }
            Action::ClearFilter => {
                self.active_view = None;
                self.filter.active_filter = None;
                self.roadmap_selected_row = 0;
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            Action::ArchiveCard => {
                self.set_selection_from_roadmap_row(self.roadmap_selected_row);
                self.start_archive_card(ViewMode::Board);
                Command::None
            }
            _ => Command::None,
        }
    }

    pub(super) fn move_card_to(&mut self, target_column: usize) -> Command {
        let real_idx = match self.real_card_index() {
            Some(idx) => idx,
            None => return Command::None,
        };

        let board = match &mut self.board {
            Some(b) => b,
            None => return Command::None,
        };

        let src_col = self.selected_column;
        if src_col == target_column || target_column >= board.columns.len() {
            return Command::None;
        }

        // "No Status" カラム（option_id が空）への移動はスキップ
        let target_option_id = board.columns[target_column].option_id.clone();
        if target_option_id.is_empty() {
            return Command::None;
        }
        if board.columns[src_col].cards.is_empty() {
            return Command::None;
        }

        // 楽観的UI更新: ローカルモデルでカードを移動
        let mut card = board.columns[src_col].cards.remove(real_idx);
        let item_id = card.item_id.clone();
        // 楽観的更新: card.custom_fields にも新しい grouping 値を反映させ、
        // 次回の軸切替でも一貫した表示になるようにする。
        if let Some(field_id) = board.grouping.field_id() {
            let field_id = field_id.to_string();
            let target_key = board.columns[target_column].option_id.clone();
            card.custom_fields.retain(|fv| fv.field_id() != field_id);
            match &board.grouping {
                crate::model::project::Grouping::SingleSelect { field_name: gfn, .. } => {
                    let (name, color) = board
                        .field_definitions
                        .iter()
                        .find_map(|d| match d {
                            crate::model::project::FieldDefinition::SingleSelect {
                                id,
                                options,
                                ..
                            } if id == &field_id => options
                                .iter()
                                .find(|o| o.id == target_key)
                                .map(|o| (o.name.clone(), o.color.clone())),
                            _ => None,
                        })
                        .unwrap_or_default();
                    card.custom_fields.push(CustomFieldValue::SingleSelect {
                        field_id: field_id.clone(),
                        field_name: gfn.clone(),
                        option_id: target_key.clone(),
                        name,
                        color,
                    });
                }
                crate::model::project::Grouping::Iteration { field_name: gfn, .. } => {
                    let title = board
                        .field_definitions
                        .iter()
                        .find_map(|d| match d {
                            crate::model::project::FieldDefinition::Iteration {
                                id,
                                iterations,
                                ..
                            } if id == &field_id => iterations
                                .iter()
                                .find(|it| it.id == target_key)
                                .map(|it| it.title.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    card.custom_fields.push(CustomFieldValue::Iteration {
                        field_id: field_id.clone(),
                        field_name: gfn.clone(),
                        iteration_id: target_key.clone(),
                        title,
                    });
                }
                crate::model::project::Grouping::None => {}
            }
        }
        board.columns[target_column].cards.push(card);

        // フィルタ後の表示インデックスを再計算して調整
        let filtered_len = board.columns[src_col]
            .cards
            .iter()
            .filter(|c| {
                self.filter
                    .active_filter
                    .as_ref()
                    .is_none_or(|f| f.matches(c))
            })
            .count();
        if filtered_len == 0 {
            self.selected_card = 0;
        } else {
            self.selected_card = self.selected_card.min(filtered_len - 1);
        }

        // Command を返す（非同期 API 呼び出しはシェル側で実行）
        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };
        let Some(field_id) = board.grouping.field_id().map(|s| s.to_string()) else {
            return Command::None;
        };
        let value = match &board.grouping {
            crate::model::project::Grouping::SingleSelect { .. } => {
                crate::command::CustomFieldValueInput::SingleSelect {
                    option_id: target_option_id,
                }
            }
            crate::model::project::Grouping::Iteration { .. } => {
                crate::command::CustomFieldValueInput::Iteration {
                    iteration_id: target_option_id,
                }
            }
            crate::model::project::Grouping::None => return Command::None,
        };

        Command::MoveCard {
            project_id,
            item_id,
            field_id,
            value,
        }
    }

    pub(super) fn handle_card_grab_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::CardGrab, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        match action {
            Action::MoveDown => {
                if self.current_layout == LayoutMode::Table {
                    self.grab_table_move_vertical(1);
                    self.table_selected_row = self.current_table_row();
                } else {
                    self.move_card_down();
                }
                Command::None
            }
            Action::MoveUp => {
                if self.current_layout == LayoutMode::Table {
                    self.grab_table_move_vertical(-1);
                    self.table_selected_row = self.current_table_row();
                } else {
                    self.move_card_up();
                }
                Command::None
            }
            Action::MoveLeft => {
                // Table モードではカラムが行内に展開されないので no-op
                if self.current_layout != LayoutMode::Table {
                    self.grab_move_card_horizontal(-1);
                }
                Command::None
            }
            Action::MoveRight => {
                if self.current_layout != LayoutMode::Table {
                    self.grab_move_card_horizontal(1);
                }
                Command::None
            }
            Action::ConfirmGrab => {
                let cmd = self.confirm_grab();
                if self.current_layout == LayoutMode::Table {
                    self.table_selected_row = self.current_table_row();
                }
                cmd
            }
            Action::CancelGrab => {
                let cmd = self.cancel_grab();
                if self.current_layout == LayoutMode::Table {
                    self.table_selected_row = self.current_table_row();
                }
                cmd
            }
            Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    /// Table view 中の grab で j/k を押した時の移動。
    /// `table_item_order` 上で隣接する 2 つの item_id の位置を入れ替えるだけで、
    /// status (column) は変更しない。Board の column 構造はそのまま。
    pub(super) fn grab_table_move_vertical(&mut self, direction: i32) {
        let real_idx = match self.real_card_index() {
            Some(idx) => idx,
            None => return,
        };
        let cur_col = self.selected_column;
        let cur_item_id = match self
            .board
            .as_ref()
            .and_then(|b| b.columns.get(cur_col))
            .and_then(|c| c.cards.get(real_idx))
            .map(|c| c.item_id.clone())
        {
            Some(s) => s,
            None => return,
        };

        let rows = self.table_rows();
        let cur_pos = match rows
            .iter()
            .position(|&(c, r)| c == cur_col && r == real_idx)
        {
            Some(p) => p,
            None => return,
        };
        let next_pos = (cur_pos as i32) + direction;
        if next_pos < 0 || next_pos as usize >= rows.len() {
            return;
        }
        let (target_col, target_real_idx) = rows[next_pos as usize];
        let target_item_id = match self
            .board
            .as_ref()
            .and_then(|b| b.columns.get(target_col))
            .and_then(|c| c.cards.get(target_real_idx))
            .map(|c| c.item_id.clone())
        {
            Some(s) => s,
            None => return,
        };

        let cur_abs = self
            .table_item_order
            .iter()
            .position(|i| i == &cur_item_id);
        let tgt_abs = self
            .table_item_order
            .iter()
            .position(|i| i == &target_item_id);
        if let (Some(a), Some(b)) = (cur_abs, tgt_abs) {
            self.table_item_order.swap(a, b);
        }
    }

    pub(super) fn grab_move_card_horizontal(&mut self, direction: i32) {
        let target_column = if direction < 0 {
            if self.selected_column == 0 {
                return;
            }
            // "No Status" カラムをスキップ
            if self
                .board
                .as_ref()
                .map(|b| b.columns[self.selected_column - 1].option_id.is_empty())
                .unwrap_or(false)
                && self.selected_column >= 2
            {
                self.selected_column - 2
            } else {
                self.selected_column - 1
            }
        } else {
            let max = self
                .board
                .as_ref()
                .map(|b| b.columns.len())
                .unwrap_or(0);
            if self.selected_column + 1 >= max {
                return;
            }
            self.selected_column + 1
        };

        let real_idx = match self.real_card_index() {
            Some(idx) => idx,
            None => return,
        };

        let board = match &mut self.board {
            Some(b) => b,
            None => return,
        };

        let src_col = self.selected_column;
        if src_col == target_column || target_column >= board.columns.len() {
            return;
        }

        let target_option_id = board.columns[target_column].option_id.clone();
        if target_option_id.is_empty() {
            return;
        }
        if board.columns[src_col].cards.is_empty() {
            return;
        }

        // 挿入位置: 元のインデックスを維持、ターゲットカラムのカード数でクランプ
        let target_len = board.columns[target_column].cards.len();
        let insert_idx = real_idx.min(target_len);

        // 楽観的UI更新: remove → insert
        let card = board.columns[src_col].cards.remove(real_idx);
        board.columns[target_column].cards.insert(insert_idx, card);

        // フォーカスを移動先に追従
        self.selected_column = target_column;
        if self.filter.active_filter.is_none() {
            self.selected_card = insert_idx;
        } else {
            let new_indices = self.filtered_card_indices(target_column);
            if let Some(pos) = new_indices.iter().position(|&i| i == insert_idx) {
                self.selected_card = pos;
            }
        }
    }

    pub(super) fn move_card_up(&mut self) {
        let real_idx = match self.real_card_index() {
            Some(idx) => idx,
            None => return,
        };

        if real_idx == 0 {
            return;
        }

        let board = match &mut self.board {
            Some(b) => b,
            None => return,
        };

        let col = self.selected_column;
        board.columns[col].cards.swap(real_idx, real_idx - 1);

        if self.filter.active_filter.is_none() {
            self.selected_card = self.selected_card.saturating_sub(1);
        } else {
            let new_indices = self.filtered_card_indices(col);
            if let Some(pos) = new_indices.iter().position(|&i| i == real_idx - 1) {
                self.selected_card = pos;
            }
        }
    }

    pub(super) fn move_card_down(&mut self) {
        let real_idx = match self.real_card_index() {
            Some(idx) => idx,
            None => return,
        };

        let board = match &mut self.board {
            Some(b) => b,
            None => return,
        };

        let col = self.selected_column;
        let card_count = board.columns[col].cards.len();

        if real_idx >= card_count - 1 {
            return;
        }

        board.columns[col].cards.swap(real_idx, real_idx + 1);

        if self.filter.active_filter.is_none() {
            self.selected_card += 1;
        } else {
            let new_indices = self.filtered_card_indices(col);
            if let Some(pos) = new_indices.iter().position(|&i| i == real_idx + 1) {
                self.selected_card = pos;
            }
        }
    }

    pub(super) fn confirm_grab(&mut self) -> Command {
        self.mode = ViewMode::Board;
        let grab = match self.grab_state.take() {
            Some(g) => g,
            None => return Command::None,
        };

        let current_column = self.selected_column;
        let current_card_index = self.real_card_index().unwrap_or(0);
        let board = match &mut self.board {
            Some(b) => b,
            None => return Command::None,
        };
        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };

        let column_changed = current_column != grab.origin_column;
        let position_changed = current_card_index != grab.origin_card_index;

        if !column_changed && !position_changed {
            return Command::None;
        }

        let after_id = if current_card_index > 0 {
            Some(
                board.columns[current_column].cards[current_card_index - 1]
                    .item_id
                    .clone(),
            )
        } else {
            None
        };

        if column_changed {
            let Some(field_id) = board.grouping.field_id().map(|s| s.to_string()) else {
                return Command::None;
            };
            let target_key = board.columns[current_column].option_id.clone();
            let value = match &board.grouping {
                crate::model::project::Grouping::SingleSelect { .. } => {
                    crate::command::CustomFieldValueInput::SingleSelect {
                        option_id: target_key.clone(),
                    }
                }
                crate::model::project::Grouping::Iteration { .. } => {
                    crate::command::CustomFieldValueInput::Iteration {
                        iteration_id: target_key.clone(),
                    }
                }
                crate::model::project::Grouping::None => return Command::None,
            };

            // 楽観的更新: 移動先カードの custom_fields に新しい grouping 値を反映
            // (grab 経路は move_card_to を通らないため、ここで明示的に更新する)
            if let Some(card) = board.columns[current_column]
                .cards
                .iter_mut()
                .find(|c| c.item_id == grab.item_id)
            {
                card.custom_fields.retain(|fv| fv.field_id() != field_id);
                match &board.grouping {
                    crate::model::project::Grouping::SingleSelect { field_name: gfn, .. } => {
                        let (name, color) = board
                            .field_definitions
                            .iter()
                            .find_map(|d| match d {
                                crate::model::project::FieldDefinition::SingleSelect {
                                    id,
                                    options,
                                    ..
                                } if id == &field_id => options
                                    .iter()
                                    .find(|o| o.id == target_key)
                                    .map(|o| (o.name.clone(), o.color.clone())),
                                _ => None,
                            })
                            .unwrap_or_default();
                        card.custom_fields.push(CustomFieldValue::SingleSelect {
                            field_id: field_id.clone(),
                            field_name: gfn.clone(),
                            option_id: target_key.clone(),
                            name,
                            color,
                        });
                    }
                    crate::model::project::Grouping::Iteration { field_name: gfn, .. } => {
                        let title = board
                            .field_definitions
                            .iter()
                            .find_map(|d| match d {
                                crate::model::project::FieldDefinition::Iteration {
                                    id,
                                    iterations,
                                    ..
                                } if id == &field_id => iterations
                                    .iter()
                                    .find(|it| it.id == target_key)
                                    .map(|it| it.title.clone()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        card.custom_fields.push(CustomFieldValue::Iteration {
                            field_id: field_id.clone(),
                            field_name: gfn.clone(),
                            iteration_id: target_key.clone(),
                            title,
                        });
                    }
                    crate::model::project::Grouping::None => {}
                }
            }

            Command::Batch(vec![
                Command::MoveCard {
                    project_id: project_id.clone(),
                    item_id: grab.item_id.clone(),
                    field_id,
                    value,
                },
                Command::ReorderCard {
                    project_id,
                    item_id: grab.item_id,
                    after_id,
                },
            ])
        } else {
            Command::ReorderCard {
                project_id,
                item_id: grab.item_id,
                after_id,
            }
        }
    }

    pub(super) fn cancel_grab(&mut self) -> Command {
        self.mode = ViewMode::Board;
        let grab = match self.grab_state.take() {
            Some(g) => g,
            None => return Command::None,
        };

        // カードを元の位置に戻す
        let board = match &mut self.board {
            Some(b) => b,
            None => return Command::None,
        };

        // 現在の位置からカードを探して削除
        let mut found_card = None;
        for col in board.columns.iter_mut() {
            if let Some(pos) = col.cards.iter().position(|c| c.item_id == grab.item_id) {
                found_card = Some(col.cards.remove(pos));
                break;
            }
        }

        // 元の位置に戻す
        if let Some(card) = found_card
            && grab.origin_column < board.columns.len() {
                let insert_idx =
                    grab.origin_card_index.min(board.columns[grab.origin_column].cards.len());
                board.columns[grab.origin_column]
                    .cards
                    .insert(insert_idx, card);
            }

        self.selected_column = grab.origin_column;
        self.selected_card = grab.origin_card_index;
        Command::None
    }

    /// フィルタ適用後のカードインデックス一覧を返す。
    /// フィルタが無い場合は全カードのインデックスをそのまま返す。
    pub fn filtered_card_indices(&self, col_idx: usize) -> Vec<usize> {
        let board = match &self.board {
            Some(b) => b,
            None => return Vec::new(),
        };
        let column = match board.columns.get(col_idx) {
            Some(c) => c,
            None => return Vec::new(),
        };
        column
            .cards
            .iter()
            .enumerate()
            .filter(|(_, card)| {
                self.filter
                    .active_filter
                    .as_ref()
                    .is_none_or(|f| f.matches(card))
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    /// selected_card (フィルタ後の表示インデックス) → 元の cards インデックスに変換
    pub fn real_card_index(&self) -> Option<usize> {
        let indices = self.filtered_card_indices(self.selected_column);
        indices.get(self.selected_card).copied()
    }

    /// 現在の board から column-major 順で table_item_order を再構築する。
    /// Board のロード/リフレッシュ時に呼ぶ。
    pub fn rebuild_table_order(&mut self) {
        self.table_item_order = match &self.board {
            Some(b) => b
                .columns
                .iter()
                .flat_map(|c| c.cards.iter().map(|cd| cd.item_id.clone()))
                .collect(),
            None => Vec::new(),
        };
    }

    pub(super) fn find_card_position(&self, item_id: &str) -> Option<(usize, usize)> {
        let board = self.board.as_ref()?;
        for (ci, col) in board.columns.iter().enumerate() {
            if let Some(ri) = col.cards.iter().position(|c| c.item_id == item_id) {
                return Some((ci, ri));
            }
        }
        None
    }

    /// Table view の表示順 (`table_item_order`) に基づいて `(col_idx, real_card_idx)` を返す。
    /// `table_item_order` に含まれない (新規追加) カードは末尾に column-major 順で付加。
    /// フィルタも適用する。
    pub fn table_rows(&self) -> Vec<(usize, usize)> {
        let board = match &self.board {
            Some(b) => b,
            None => return Vec::new(),
        };
        let mut out = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for item_id in &self.table_item_order {
            if let Some((ci, ri)) = self.find_card_position(item_id) {
                let card = &board.columns[ci].cards[ri];
                if self
                    .filter
                    .active_filter
                    .as_ref()
                    .is_none_or(|f| f.matches(card))
                {
                    out.push((ci, ri));
                }
                seen.insert(item_id.clone());
            }
        }
        for (ci, col) in board.columns.iter().enumerate() {
            for (ri, card) in col.cards.iter().enumerate() {
                if !seen.contains(&card.item_id)
                    && self
                        .filter
                        .active_filter
                        .as_ref()
                        .is_none_or(|f| f.matches(card))
                {
                    out.push((ci, ri));
                }
            }
        }
        out
    }

    /// Board → Table 切替時に、現在の (selected_column, selected_card) を
    /// table_selected_row に変換する。
    pub fn current_table_row(&self) -> usize {
        let real = match self.real_card_index() {
            Some(r) => r,
            None => return 0,
        };
        self.table_rows()
            .iter()
            .position(|&(col, idx)| col == self.selected_column && idx == real)
            .unwrap_or(0)
    }

    /// Table → Board 切替時に、table_selected_row を
    /// (selected_column, selected_card) に書き戻す。
    pub fn set_selection_from_table_row(&mut self, row: usize) {
        let rows = self.table_rows();
        if let Some(&(col, real_idx)) = rows.get(row) {
            self.selected_column = col;
            let display_idx = self
                .filtered_card_indices(col)
                .iter()
                .position(|&i| i == real_idx)
                .unwrap_or(0);
            self.selected_card = display_idx;
        }
    }

    /// Roadmap view の表示順 `(col_idx, real_card_idx)` を返す。
    /// MVP では `table_rows()` と同じ順序を利用する (column-major flatten + filter 適用)。
    /// 各 row は 1 カードに対応し、iteration が未設定のカードも含まれる (render 側で bar を省略)。
    pub fn roadmap_rows(&self) -> Vec<(usize, usize)> {
        self.table_rows()
    }

    /// Board → Roadmap 切替時に、現在の (selected_column, selected_card) を
    /// roadmap_selected_row に変換する。
    /// 現状 `roadmap_rows()` は `table_rows()` と同一順序のため `current_table_row()`
    /// と等価。将来 roadmap 独自の並び順を導入したときに差別化する。
    #[allow(dead_code)]
    pub fn current_roadmap_row(&self) -> usize {
        let real = match self.real_card_index() {
            Some(r) => r,
            None => return 0,
        };
        self.roadmap_rows()
            .iter()
            .position(|&(col, idx)| col == self.selected_column && idx == real)
            .unwrap_or(0)
    }

    /// Roadmap → Board 切替時に、roadmap_selected_row を
    /// (selected_column, selected_card) に書き戻す。
    pub fn set_selection_from_roadmap_row(&mut self, row: usize) {
        let rows = self.roadmap_rows();
        if let Some(&(col, real_idx)) = rows.get(row) {
            self.selected_column = col;
            let display_idx = self
                .filtered_card_indices(col)
                .iter()
                .position(|&i| i == real_idx)
                .unwrap_or(0);
            self.selected_card = display_idx;
        }
    }

    pub(super) fn clamp_card_selection(&mut self) {
        let filtered_len = self.filtered_card_indices(self.selected_column).len();
        if filtered_len == 0 {
            self.selected_card = 0;
        } else {
            self.selected_card = self.selected_card.min(filtered_len - 1);
        }
        self.scroll_offset = 0;
    }

    /// カードを別カラムに移動し、選択状態を移動先に追従させる (詳細ビュー用)
    pub(super) fn move_card_to_and_follow(&mut self, target_column: usize) -> Command {
        let item_id = match self.selected_card_ref() {
            Some(c) => c.item_id.clone(),
            None => return Command::None,
        };

        let cmd = self.move_card_to(target_column);
        if matches!(cmd, Command::None) {
            return cmd;
        }

        // 移動先カラムに追従
        self.selected_column = target_column;
        if let Some(board) = &self.board
            && let Some(col) = board.columns.get(target_column)
                && let Some(real_idx) = col.cards.iter().position(|c| c.item_id == item_id) {
                    let filtered = self.filtered_card_indices(target_column);
                    self.selected_card = filtered
                        .iter()
                        .position(|&i| i == real_idx)
                        .unwrap_or(0);
                }

        cmd
    }
}
