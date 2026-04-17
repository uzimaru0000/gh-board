use super::*;

impl AppState {
    pub(super) fn handle_confirm_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::Confirm, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        match action {
            Action::ConfirmYes => {
                if let Some(state) = self.confirm_state.take() {
                    let return_to = state.return_to;
                    let cmd = match state.action {
                        ConfirmAction::ArchiveCard { item_id } => self.archive_card(&item_id),
                    };
                    // カードが消える破壊的操作は Detail には留まれない → Board に戻る
                    self.mode = match &cmd {
                        Command::ArchiveCard { .. } => ViewMode::Board,
                        _ => return_to,
                    };
                    cmd
                } else {
                    Command::None
                }
            }
            Action::ConfirmNo => {
                let return_to = self
                    .confirm_state
                    .as_ref()
                    .map(|s| s.return_to.clone())
                    .unwrap_or(ViewMode::Board);
                self.confirm_state = None;
                self.mode = return_to;
                Command::None
            }
            Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    pub(super) fn handle_create_card_key(&mut self, key: KeyEvent) -> Command {
        // Global keys (ForceQuit, Back, Tab fields)
        if let Some(action) = self.keymap.resolve(KeymapMode::CreateCardGlobal, &key) {
            match action {
                Action::ForceQuit => {
                    self.should_quit = true;
                    return Command::None;
                }
                Action::Back => {
                    self.mode = ViewMode::Board;
                    return Command::None;
                }
                Action::NextField => {
                    let next = match self.create_card_state.focused_field {
                        CreateCardField::Type => CreateCardField::Title,
                        CreateCardField::Title => CreateCardField::Body,
                        CreateCardField::Body => CreateCardField::Submit,
                        CreateCardField::Submit => CreateCardField::Type,
                    };
                    // Submit が disable のときはスキップして次へ
                    self.create_card_state.focused_field =
                        if next == CreateCardField::Submit && !self.can_submit_create_card() {
                            CreateCardField::Type
                        } else {
                            next
                        };
                    return Command::None;
                }
                Action::PrevField => {
                    let prev = match self.create_card_state.focused_field {
                        CreateCardField::Type => CreateCardField::Submit,
                        CreateCardField::Title => CreateCardField::Type,
                        CreateCardField::Body => CreateCardField::Title,
                        CreateCardField::Submit => CreateCardField::Body,
                    };
                    self.create_card_state.focused_field =
                        if prev == CreateCardField::Submit && !self.can_submit_create_card() {
                            CreateCardField::Body
                        } else {
                            prev
                        };
                    return Command::None;
                }
                _ => {}
            }
        }

        match self.create_card_state.focused_field {
            CreateCardField::Type => {
                // Type field: toggle via keymap
                if let Some(Action::ToggleType) = self.keymap.resolve(KeymapMode::CreateCardType, &key) {
                    self.create_card_state.card_type = match self.create_card_state.card_type {
                        NewCardType::Draft => NewCardType::Issue,
                        NewCardType::Issue => NewCardType::Draft,
                    };
                }
            }
            CreateCardField::Body => {
                // Body field: open editor
                if let Some(Action::OpenEditor) = self.keymap.resolve(KeymapMode::CreateCardBody, &key) {
                    let content = self.create_card_state.body_input.clone();
                    return Command::OpenEditor { content };
                }
            }
            CreateCardField::Submit => {
                if let Some(Action::Submit) = self.keymap.resolve(KeymapMode::CreateCardSubmit, &key) {
                    if !self.can_submit_create_card() {
                        return Command::None;
                    }
                    return self.submit_create_card();
                }
            }
            CreateCardField::Title => {
                // Title field: text input (not configurable)
                match key.code {
                    KeyCode::Backspace => {
                        let cursor = &mut self.create_card_state.title_cursor;
                        if *cursor > 0 {
                            let prev = prev_char_pos(&self.create_card_state.title_input, *cursor);
                            self.create_card_state.title_input.drain(prev..*cursor);
                            *cursor = prev;
                        }
                    }
                    KeyCode::Left => {
                        let cursor = &mut self.create_card_state.title_cursor;
                        if *cursor > 0 {
                            *cursor = prev_char_pos(&self.create_card_state.title_input, *cursor);
                        }
                    }
                    KeyCode::Right => {
                        let cursor = &mut self.create_card_state.title_cursor;
                        if *cursor < self.create_card_state.title_input.len() {
                            *cursor = next_char_pos(&self.create_card_state.title_input, *cursor);
                        }
                    }
                    KeyCode::Char(c) => {
                        let cursor = &mut self.create_card_state.title_cursor;
                        self.create_card_state.title_input.insert(*cursor, c);
                        *cursor += c.len_utf8();
                    }
                    _ => {}
                }
            }
        }
        Command::None
    }

    pub(super) fn start_archive_card(&mut self, return_to: ViewMode) {
        let real_idx = match self.real_card_index() {
            Some(idx) => idx,
            None => return,
        };
        let card = self
            .board
            .as_ref()
            .and_then(|b| b.columns.get(self.selected_column))
            .and_then(|c| c.cards.get(real_idx));

        if let Some(card) = card {
            self.confirm_state = Some(ConfirmState {
                action: ConfirmAction::ArchiveCard {
                    item_id: card.item_id.clone(),
                },
                title: card.title.clone(),
                return_to,
            });
            self.mode = ViewMode::Confirm;
        }
    }

    pub(super) fn archive_card(&mut self, item_id: &str) -> Command {
        // 楽観的UI更新: ローカルモデルからカードを除去
        if let Some(board) = &mut self.board
            && let Some(col) = board.columns.get_mut(self.selected_column)
            && let Some(pos) = col.cards.iter().position(|c| c.item_id == item_id)
        {
            col.cards.remove(pos);
            let filtered_len = col
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
        }

        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };

        Command::ArchiveCard {
            project_id,
            item_id: item_id.to_string(),
        }
    }

    pub(super) fn show_archived_list(&mut self) -> Command {
        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };
        self.archived_list = Some(crate::model::state::ArchivedListState {
            cards: Vec::new(),
            selected: 0,
            loading: true,
            error: None,
        });
        self.mode = ViewMode::ArchivedList;
        Command::LoadArchivedItems { project_id }
    }

    pub(super) fn handle_archived_list_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::ArchivedList, &key) {
            Some(a) => a,
            None => return Command::None,
        };
        match action {
            Action::Back => {
                self.mode = ViewMode::Board;
                Command::None
            }
            Action::MoveDown => {
                if let Some(state) = self.archived_list.as_mut()
                    && !state.cards.is_empty()
                {
                    state.selected = (state.selected + 1).min(state.cards.len() - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                if let Some(state) = self.archived_list.as_mut() {
                    state.selected = state.selected.saturating_sub(1);
                }
                Command::None
            }
            Action::UnarchiveCard => self.unarchive_selected_in_list(),
            Action::OpenDetail => {
                // 現状の Detail は board のカードを参照する設計のため、
                // ここではブラウザで開く代替動作にとどめる。
                if let Some(state) = self.archived_list.as_ref()
                    && let Some(card) = state.cards.get(state.selected)
                    && let Some(url) = card.url.clone()
                {
                    return Command::OpenUrl(url);
                }
                Command::None
            }
            Action::Refresh => self.show_archived_list(),
            Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    pub(super) fn unarchive_selected_in_list(&mut self) -> Command {
        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };
        let state = match self.archived_list.as_mut() {
            Some(s) => s,
            None => return Command::None,
        };
        if state.cards.is_empty() {
            return Command::None;
        }
        let idx = state.selected.min(state.cards.len() - 1);
        let item_id = state.cards[idx].item_id.clone();
        state.cards.remove(idx);
        if state.cards.is_empty() {
            state.selected = 0;
        } else if state.selected >= state.cards.len() {
            state.selected = state.cards.len() - 1;
        }
        Command::UnarchiveCard {
            project_id,
            item_id,
        }
    }

    pub fn can_submit_create_card(&self) -> bool {
        !self.create_card_state.title_input.trim().is_empty()
    }

    pub(super) fn submit_create_card(&mut self) -> Command {
        let title = self.create_card_state.title_input.trim().to_string();
        if title.is_empty() {
            return Command::None;
        }
        let body = self.create_card_state.body_input.clone();

        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };

        let initial_status = match &self.board {
            Some(board) => {
                // SingleSelect グルーピング + 非 "No Status" カラムのときのみ初期値を設定
                match &board.grouping {
                    crate::model::project::Grouping::SingleSelect { field_id, .. } => {
                        let col = board.columns.get(self.selected_column);
                        col.and_then(|c| {
                            if c.option_id.is_empty() {
                                None
                            } else {
                                Some(crate::command::InitialStatus {
                                    field_id: field_id.clone(),
                                    option_id: c.option_id.clone(),
                                })
                            }
                        })
                    }
                    _ => None,
                }
            }
            None => return Command::None,
        };

        match self.create_card_state.card_type {
            NewCardType::Draft => {
                self.mode = ViewMode::Board;
                self.loading = LoadingState::Loading("Creating card...".into());
                Command::CreateCard {
                    project_id,
                    title,
                    body,
                    initial_status,
                }
            }
            NewCardType::Issue => {
                let repos = self
                    .board
                    .as_ref()
                    .map(|b| &b.repositories)
                    .cloned()
                    .unwrap_or_default();

                if repos.is_empty() {
                    self.loading = LoadingState::Error(
                        "No repositories linked to this project.".into(),
                    );
                    return Command::None;
                }

                if repos.len() == 1 {
                    self.mode = ViewMode::Board;
                    self.loading = LoadingState::Loading("Creating issue...".into());
                    return Command::CreateIssue {
                        project_id,
                        repository_id: repos[0].id.clone(),
                        title,
                        body,
                        initial_status,
                    };
                }

                // 複数リポジトリ → セレクタ表示
                self.repo_select_state = Some(RepoSelectState {
                    selected_index: 0,
                    pending_create: PendingIssueCreate {
                        title,
                        body,
                        initial_status,
                    },
                });
                self.mode = ViewMode::RepoSelect;
                Command::None
            }
        }
    }

    pub(super) fn handle_repo_select_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::RepoSelect, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let repo_count = self
            .board
            .as_ref()
            .map(|b| b.repositories.len())
            .unwrap_or(0);

        match action {
            Action::ForceQuit => {
                self.should_quit = true;
            }
            Action::MoveDown => {
                if let Some(rs) = &mut self.repo_select_state
                    && rs.selected_index + 1 < repo_count {
                        rs.selected_index += 1;
                    }
            }
            Action::MoveUp => {
                if let Some(rs) = &mut self.repo_select_state {
                    rs.selected_index = rs.selected_index.saturating_sub(1);
                }
            }
            Action::Select => {
                return self.submit_repo_selection();
            }
            Action::Back | Action::Quit => {
                self.repo_select_state = None;
                self.mode = ViewMode::Board;
            }
            _ => {}
        }
        Command::None
    }

    pub(super) fn submit_repo_selection(&mut self) -> Command {
        let rs = match self.repo_select_state.take() {
            Some(rs) => rs,
            None => return Command::None,
        };

        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };

        let repository_id = self
            .board
            .as_ref()
            .and_then(|b| b.repositories.get(rs.selected_index))
            .map(|r| r.id.clone());

        let repository_id = match repository_id {
            Some(id) => id,
            None => return Command::None,
        };

        self.mode = ViewMode::Board;
        self.loading = LoadingState::Loading("Creating issue...".into());

        Command::CreateIssue {
            project_id,
            repository_id,
            title: rs.pending_create.title,
            body: rs.pending_create.body,
            initial_status: rs.pending_create.initial_status,
        }
    }

    pub(super) fn handle_help_key(&mut self, key: KeyEvent) {
        if let Some(Action::Back) = self.keymap.resolve(KeymapMode::Help, &key) {
            self.mode = ViewMode::Board;
        }
    }

    /// Board モードから `G` 押下で呼ばれる。
    /// 利用可能な groupable field (SingleSelect + Iteration) をリスト化し GroupBySelect モードへ移行。
    pub(super) fn open_group_by_select(&mut self) {
        let Some(board) = &self.board else {
            return;
        };
        let mut candidates: Vec<crate::model::project::Grouping> = Vec::new();
        for def in &board.field_definitions {
            match def {
                crate::model::project::FieldDefinition::SingleSelect { id, name, .. } => {
                    candidates.push(crate::model::project::Grouping::SingleSelect {
                        field_id: id.clone(),
                        field_name: name.clone(),
                    });
                }
                crate::model::project::FieldDefinition::Iteration { id, name, .. } => {
                    candidates.push(crate::model::project::Grouping::Iteration {
                        field_id: id.clone(),
                        field_name: name.clone(),
                    });
                }
                _ => {}
            }
        }
        if candidates.is_empty() {
            return;
        }
        // 現在の軸にカーソルを合わせる
        let cursor = candidates
            .iter()
            .position(|g| g == &board.grouping)
            .unwrap_or(0);
        self.group_by_select_state = Some(GroupBySelectState { cursor, candidates });
        self.mode = ViewMode::GroupBySelect;
    }

    pub(super) fn handle_group_by_select_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::GroupBySelect, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let candidate_count = self
            .group_by_select_state
            .as_ref()
            .map(|s| s.candidates.len())
            .unwrap_or(0);

        match action {
            Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
            Action::Back | Action::Quit => {
                self.group_by_select_state = None;
                self.mode = ViewMode::Board;
                Command::None
            }
            Action::MoveDown => {
                if let Some(ref mut s) = self.group_by_select_state
                    && candidate_count > 0
                {
                    s.cursor = (s.cursor + 1).min(candidate_count - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                if let Some(ref mut s) = self.group_by_select_state {
                    s.cursor = s.cursor.saturating_sub(1);
                }
                Command::None
            }
            Action::Select => {
                let grouping = match self.group_by_select_state.take() {
                    Some(s) => s.candidates.into_iter().nth(s.cursor),
                    None => None,
                };
                self.mode = ViewMode::Board;
                if let Some(g) = grouping {
                    self.apply_grouping(g);
                }
                Command::None
            }
            _ => Command::None,
        }
    }

    /// 現在の Board を新しい grouping で再構築する。
    /// 既存カードを flatten して `build_columns_for_grouping` で再分配する。
    pub fn apply_grouping(&mut self, grouping: crate::model::project::Grouping) {
        let Some(board) = &mut self.board else {
            return;
        };
        // カードを flatten
        let mut all_cards: Vec<Card> = Vec::new();
        for col in board.columns.drain(..) {
            all_cards.extend(col.cards);
        }
        let new_columns = crate::github::client::build_columns_for_grouping(
            &grouping,
            &board.field_definitions,
            all_cards,
        );
        // 以降のリロード (エラー時や `r` キー) で現在の軸を維持するため、
        // preferred_grouping_field_name をセッション中は新しい軸に追随させる。
        self.preferred_grouping_field_name =
            grouping.field_name().map(|s| s.to_string());
        board.columns = new_columns;
        board.grouping = grouping;
        // カーソルをクランプ
        if board.columns.is_empty() {
            self.selected_column = 0;
            self.selected_card = 0;
        } else {
            self.selected_column = self.selected_column.min(board.columns.len() - 1);
            self.selected_card = 0;
        }
        self.scroll_offset = 0;
    }

    pub(super) fn open_reaction_picker_for_card(&mut self) -> Command {
        let card = match self.selected_card_ref() {
            Some(c) => c,
            None => return Command::None,
        };
        // DraftIssue はリアクション不可
        if matches!(card.card_type, CardType::DraftIssue) {
            return Command::None;
        }
        let content_id = match &card.content_id {
            Some(id) => id.clone(),
            None => return Command::None,
        };
        let return_to = self.mode.clone();
        self.reaction_picker_state = Some(ReactionPickerState {
            target: ReactionTarget::CardBody { content_id },
            cursor: 0,
            return_to,
        });
        self.mode = ViewMode::ReactionPicker;
        Command::None
    }

    pub(super) fn open_reaction_picker_for_comment(&mut self) -> Command {
        let cls = match &self.comment_list_state {
            Some(s) => s,
            None => return Command::None,
        };
        let card = match self.selected_card_ref() {
            Some(c) => c,
            None => return Command::None,
        };
        let comment = match card.comments.get(cls.cursor) {
            Some(c) => c,
            None => return Command::None,
        };
        let content_id = cls.content_id.clone();
        let comment_id = comment.id.clone();
        let return_to = self.mode.clone();
        self.reaction_picker_state = Some(ReactionPickerState {
            target: ReactionTarget::Comment {
                comment_id,
                content_id,
            },
            cursor: 0,
            return_to,
        });
        self.mode = ViewMode::ReactionPicker;
        Command::None
    }

    pub(super) fn handle_reaction_picker_key(&mut self, key: KeyEvent) -> Command {
        use crate::model::project::ReactionContent;
        let action = match self.keymap.resolve(KeymapMode::ReactionPicker, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        match action {
            Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
            Action::Back | Action::Quit => {
                let prev = self
                    .reaction_picker_state
                    .as_ref()
                    .map(|s| s.return_to.clone())
                    .unwrap_or(ViewMode::Detail);
                self.reaction_picker_state = None;
                self.mode = prev;
                Command::None
            }
            Action::MoveLeft => {
                if let Some(ref mut st) = self.reaction_picker_state {
                    let len = ReactionContent::all().len();
                    st.cursor = if st.cursor == 0 { len - 1 } else { st.cursor - 1 };
                }
                Command::None
            }
            Action::MoveRight => {
                if let Some(ref mut st) = self.reaction_picker_state {
                    let len = ReactionContent::all().len();
                    st.cursor = (st.cursor + 1) % len;
                }
                Command::None
            }
            Action::ToggleReaction => self.toggle_selected_reaction(),
            _ => Command::None,
        }
    }

    /// リアクションを楽観的に toggle し、AddReaction/RemoveReaction コマンドを返す
    pub(super) fn toggle_selected_reaction(&mut self) -> Command {
        use crate::model::project::{apply_reaction_toggle, ReactionContent};
        let (target, cursor) = match &self.reaction_picker_state {
            Some(s) => (s.target.clone(), s.cursor),
            None => return Command::None,
        };
        let content = ReactionContent::all()[cursor];
        let (subject_id, now_reacted) = match &target {
            ReactionTarget::CardBody { content_id } => {
                let card = match self.find_card_by_content_id_mut(content_id) {
                    Some(c) => c,
                    None => return Command::None,
                };
                let reacted = apply_reaction_toggle(&mut card.reactions, content);
                (content_id.clone(), reacted)
            }
            ReactionTarget::Comment {
                comment_id,
                content_id,
            } => {
                let card = match self.find_card_by_content_id_mut(content_id) {
                    Some(c) => c,
                    None => return Command::None,
                };
                let comment = match card.comments.iter_mut().find(|c| &c.id == comment_id) {
                    Some(c) => c,
                    None => return Command::None,
                };
                let reacted = apply_reaction_toggle(&mut comment.reactions, content);
                (comment_id.clone(), reacted)
            }
        };

        if now_reacted {
            Command::AddReaction {
                subject_id,
                content,
            }
        } else {
            Command::RemoveReaction {
                subject_id,
                content,
            }
        }
    }
}
