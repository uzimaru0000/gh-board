use super::*;

impl AppState {
    pub(super) fn open_detail_view(&mut self) -> Command {
        if self.real_card_index().is_none() {
            return Command::None;
        }
        self.detail_scroll = 0;
        self.detail_scroll_x = 0;
        self.detail_pane = DetailPane::Content;
        self.sidebar_selected = 0;
        self.status_select_open = false;
        self.sidebar_edit = None;
        self.mode = ViewMode::Detail;

        let mut commands: Vec<Command> = Vec::new();
        if let Some(card) = self.selected_card_ref() {
            let content_id = card.content_id.clone();
            // body が未取得（ボード初期ロード時は None）なら遅延取得を発行
            if card.body.is_none() {
                if let Some(cid) = content_id.clone() {
                    commands.push(Command::FetchCardDetail {
                        item_id: card.item_id.clone(),
                        content_id: cid,
                    });
                }
            } else if card.comments.len() >= 20 {
                // body が取得済み（CardDetailLoaded 済み or FetchIssueDetail 経由）の場合のみ
                // コメント追加取得を判定
                if let Some(cid) = content_id.clone() {
                    commands.push(Command::FetchComments { content_id: cid });
                }
            }
            let needs_sub_issues = matches!(card.card_type, CardType::Issue { .. })
                && card
                    .sub_issues_summary
                    .as_ref()
                    .is_some_and(|s| s.total > 0);
            if needs_sub_issues
                && let Some(cid) = content_id
            {
                commands.push(Command::FetchSubIssues {
                    item_id: card.item_id.clone(),
                    content_id: cid,
                });
            }
        }
        match commands.len() {
            0 => Command::None,
            1 => commands.into_iter().next().unwrap(),
            _ => Command::Batch(commands),
        }
    }

    pub(super) fn handle_detail_key(&mut self, key: KeyEvent) -> Command {
        // ForceQuit (global)
        if let Some(Action::ForceQuit) = self.keymap.resolve(KeymapMode::DetailContent, &key) {
            self.should_quit = true;
            return Command::None;
        }

        // サイドバー編集モード (ラベル/アサイニー トグルリスト)
        if self.sidebar_edit.is_some() {
            return self.handle_sidebar_edit_key(key);
        }

        // ステータス選択ドロップダウンが開いている場合
        if self.status_select_open {
            return self.handle_status_select_key(key);
        }

        // Determine the keymap mode based on current detail pane
        let mode = match self.detail_pane {
            DetailPane::Content => KeymapMode::DetailContent,
            DetailPane::Sidebar => KeymapMode::DetailSidebar,
        };

        let action = match self.keymap.resolve(mode, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        match action {
            Action::Quit => {
                if !self.pop_detail_stack() {
                    self.mode = ViewMode::Board;
                }
                Command::None
            }
            Action::Back => {
                if self.detail_pane == DetailPane::Sidebar {
                    self.detail_pane = DetailPane::Content;
                } else if !self.pop_detail_stack() {
                    self.mode = ViewMode::Board;
                }
                Command::None
            }
            Action::NextTab | Action::PrevTab => {
                self.detail_pane = match self.detail_pane {
                    DetailPane::Content => DetailPane::Sidebar,
                    DetailPane::Sidebar => DetailPane::Content,
                };
                Command::None
            }
            _ => match self.detail_pane {
                DetailPane::Content => self.handle_detail_content_action(action),
                DetailPane::Sidebar => self.handle_detail_sidebar_action(action),
            },
        }
    }

    pub(super) fn handle_detail_content_action(&mut self, action: Action) -> Command {
        match action {
            Action::OpenInBrowser => {
                self.mode = ViewMode::Board;
                self.open_in_browser()
            }
            Action::MoveDown => {
                self.detail_scroll = self
                    .detail_scroll
                    .saturating_add(1)
                    .min(self.detail_max_scroll.get());
                Command::None
            }
            Action::MoveUp => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
                Command::None
            }
            Action::MoveLeft => {
                self.detail_scroll_x = self.detail_scroll_x.saturating_sub(2);
                Command::None
            }
            Action::MoveRight => {
                self.detail_scroll_x = self
                    .detail_scroll_x
                    .saturating_add(2)
                    .min(self.detail_max_scroll_x.get());
                Command::None
            }
            Action::EditCard => self.start_edit_card(),
            Action::NewComment => self.start_new_comment(),
            Action::OpenCommentList => self.open_comment_list(),
            Action::OpenReactionPicker => self.open_reaction_picker_for_card(),
            _ => Command::None,
        }
    }

    pub(super) fn start_new_comment(&mut self) -> Command {
        let card = match self.selected_card_ref() {
            Some(c) => c,
            None => return Command::None,
        };
        // DraftIssue はコメント不可
        if matches!(card.card_type, CardType::DraftIssue) {
            return Command::None;
        }
        let content_id = match &card.content_id {
            Some(id) => id.clone(),
            None => return Command::None,
        };
        Command::OpenEditorForComment {
            content_id,
            existing: None,
        }
    }

    pub(super) fn open_comment_list(&mut self) -> Command {
        let card = match self.selected_card_ref() {
            Some(c) => c,
            None => return Command::None,
        };
        // DraftIssue はコメント不可
        if matches!(card.card_type, CardType::DraftIssue) {
            return Command::None;
        }
        let content_id = match &card.content_id {
            Some(id) => id.clone(),
            None => return Command::None,
        };
        self.enter_comment_list(CommentListState {
            cursor: 0,
            content_id,
        });
        Command::None
    }

    pub(super) fn start_edit_card(&mut self) -> Command {
        let card = match self.selected_card_ref() {
            Some(c) => c,
            None => return Command::None,
        };
        let content_id = match &card.content_id {
            Some(id) => id.clone(),
            None => return Command::None,
        };
        let item_id = card.item_id.clone();
        let card_type = card.card_type.clone();
        let title = card.title.clone();
        let title_cursor = title.len();
        let body = card.body.clone().unwrap_or_default();
        self.edit_card_state = Some(EditCardState {
            content_id,
            item_id,
            card_type,
            title_input: title,
            title_cursor,
            body_input: body,
            focused_field: EditCardField::Title,
        });
        self.mode = ViewMode::EditCard;
        Command::None
    }

    pub(super) fn submit_edit_card(&mut self) -> Command {
        let edit_state = match &self.edit_card_state {
            Some(s) => s,
            None => return Command::None,
        };
        let title = edit_state.title_input.trim().to_string();
        if title.is_empty() {
            return Command::None;
        }
        let body = edit_state.body_input.clone();
        let content_id = edit_state.content_id.clone();
        let card_type = edit_state.card_type.clone();
        let item_id = edit_state.item_id.clone();

        // 楽観的更新
        if let Some(board) = &mut self.board {
            for col in &mut board.columns {
                if let Some(card) = col.cards.iter_mut().find(|c| c.item_id == item_id) {
                    card.title = title.clone();
                    card.body = Some(body.clone());
                    break;
                }
            }
        }

        self.mode = ViewMode::Detail;
        self.edit_card_state = None;
        Command::UpdateCard {
            content_id,
            card_type,
            title,
            body,
        }
    }

    pub(super) fn handle_edit_card_key(&mut self, key: KeyEvent) -> Command {
        // Global keys (ForceQuit, Submit, Back, Tab fields)
        if let Some(action) = self.keymap.resolve(KeymapMode::EditCardGlobal, &key) {
            match action {
                Action::ForceQuit => {
                    self.should_quit = true;
                    return Command::None;
                }
                Action::Submit => {
                    return self.submit_edit_card();
                }
                Action::Back => {
                    self.mode = ViewMode::Detail;
                    self.edit_card_state = None;
                    return Command::None;
                }
                Action::NextField => {
                    if let Some(ref mut state) = self.edit_card_state {
                        state.focused_field = match state.focused_field {
                            EditCardField::Title => EditCardField::Body,
                            EditCardField::Body => EditCardField::Title,
                        };
                    }
                    return Command::None;
                }
                _ => {}
            }
        }

        let focused = self
            .edit_card_state
            .as_ref()
            .map(|s| s.focused_field.clone());
        match focused {
            Some(EditCardField::Title) => {
                // Title: text input (not configurable)
                let state = match self.edit_card_state.as_mut() {
                    Some(s) => s,
                    None => return Command::None,
                };
                match key.code {
                    KeyCode::Backspace if state.title_cursor > 0 => {
                        let prev = prev_char_pos(&state.title_input, state.title_cursor);
                        state.title_input.drain(prev..state.title_cursor);
                        state.title_cursor = prev;
                    }
                    KeyCode::Left if state.title_cursor > 0 => {
                        state.title_cursor =
                            prev_char_pos(&state.title_input, state.title_cursor);
                    }
                    KeyCode::Right if state.title_cursor < state.title_input.len() => {
                        state.title_cursor =
                            next_char_pos(&state.title_input, state.title_cursor);
                    }
                    KeyCode::Char(c) => {
                        state.title_input.insert(state.title_cursor, c);
                        state.title_cursor += c.len_utf8();
                    }
                    _ => {}
                }
                Command::None
            }
            Some(EditCardField::Body) => {
                if let Some(Action::OpenEditor) = self.keymap.resolve(KeymapMode::EditCardBody, &key) {
                    let content = self
                        .edit_card_state
                        .as_ref()
                        .map(|s| s.body_input.clone())
                        .unwrap_or_default();
                    Command::OpenEditor { content }
                } else {
                    Command::None
                }
            }
            None => Command::None,
        }
    }

    /// 現在の詳細ビュー対象カード (detail_stack 優先、なければ board 上の selected)
    pub fn current_detail_card(&self) -> Option<&Card> {
        if let Some(last) = self.detail_stack.last() {
            return Some(last);
        }
        self.selected_card_ref()
    }

    /// サイドバーの論理セクションを動的に列挙する。
    /// selected card に parent / sub-issues があれば追加し、末尾は Archive。
    pub fn sidebar_sections(&self) -> Vec<SidebarSection> {
        let mut sections = vec![
            SidebarSection::Status,
            SidebarSection::Assignees,
            SidebarSection::Labels,
            SidebarSection::Milestone,
        ];
        for (i, _) in self.field_definitions().iter().enumerate() {
            sections.push(SidebarSection::CustomField(i));
        }
        if let Some(card) = self.current_detail_card()
            && matches!(card.card_type, CardType::Issue { .. })
        {
            if card.parent_issue.is_some() {
                sections.push(SidebarSection::Parent);
            }
            let has_subs = card
                .sub_issues_summary
                .as_ref()
                .is_some_and(|s| s.total > 0);
            if has_subs {
                // summary はあっても sub_issues がまだ空 (ロード前) の可能性もある。
                // 実データが揃っていればその数だけ選択可能行を出す。
                for (i, _) in card.sub_issues.iter().enumerate() {
                    sections.push(SidebarSection::SubIssue(i));
                }
            }
        }
        sections.push(SidebarSection::Archive);
        sections
    }

    pub fn sidebar_section_at(&self, index: usize) -> Option<SidebarSection> {
        self.sidebar_sections().into_iter().nth(index)
    }

    /// サイドバーの総セクション数
    pub fn sidebar_section_count(&self) -> usize {
        self.sidebar_sections().len()
    }

    /// Archive セクションのインデックス (動的)
    pub fn sidebar_archive_index(&self) -> usize {
        self.sidebar_section_count().saturating_sub(1)
    }

    pub(super) fn field_definitions(&self) -> &[FieldDefinition] {
        self.board
            .as_ref()
            .map(|b| b.field_definitions.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn handle_detail_sidebar_action(&mut self, action: Action) -> Command {
        match action {
            Action::MoveDown => {
                let max = self.sidebar_section_count().saturating_sub(1);
                self.sidebar_selected = (self.sidebar_selected + 1).min(max);
                Command::None
            }
            Action::MoveUp => {
                self.sidebar_selected = self.sidebar_selected.saturating_sub(1);
                Command::None
            }
            Action::Select => {
                let section = self.sidebar_section_at(self.sidebar_selected);
                // detail_stack 上の Issue (ボード外) は編集系操作を無効にし、
                // ナビゲーション (Parent / SubIssue) のみ受け付ける
                let is_stacked = !self.detail_stack.is_empty();
                match section {
                    Some(SidebarSection::Parent) => self.open_parent_detail(),
                    Some(SidebarSection::SubIssue(i)) => self.open_sub_issue_detail(i),
                    _ if is_stacked => Command::None,
                    Some(SidebarSection::Status) => {
                        self.status_select_open = true;
                        self.status_select_cursor = self.selected_column;
                        Command::None
                    }
                    Some(SidebarSection::Labels) => self.open_label_edit(),
                    Some(SidebarSection::Assignees) => self.open_assignee_edit(),
                    Some(SidebarSection::Archive) => {
                        self.start_archive_card(ViewMode::Detail);
                        Command::None
                    }
                    Some(SidebarSection::CustomField(i)) => {
                        self.open_custom_field_edit(i);
                        Command::None
                    }
                    Some(SidebarSection::Milestone) | None => Command::None,
                }
            }
            Action::ArchiveCard => {
                self.start_archive_card(ViewMode::Detail);
                Command::None
            }
            _ => Command::None,
        }
    }

    pub(super) fn open_parent_detail(&mut self) -> Command {
        let Some(card) = self.current_detail_card() else {
            return Command::None;
        };
        let Some(parent) = card.parent_issue.as_ref() else {
            return Command::None;
        };
        let id = parent.id.clone();
        if self.detail_loading_id.as_deref() == Some(&id) {
            return Command::None;
        }
        self.detail_loading_id = Some(id.clone());
        Command::FetchIssueDetail { content_id: id }
    }

    pub(super) fn open_sub_issue_detail(&mut self, idx: usize) -> Command {
        let Some(card) = self.current_detail_card() else {
            return Command::None;
        };
        let Some(sub) = card.sub_issues.get(idx) else {
            return Command::None;
        };
        let id = sub.id.clone();
        if self.detail_loading_id.as_deref() == Some(&id) {
            return Command::None;
        }
        self.detail_loading_id = Some(id.clone());
        Command::FetchIssueDetail { content_id: id }
    }

    /// detail_stack を 1 段戻す。空なら false (詳細ビュー自体を閉じる)。
    pub fn pop_detail_stack(&mut self) -> bool {
        if self.detail_stack.pop().is_some() {
            self.sidebar_selected = 0;
            self.detail_scroll = 0;
            self.detail_scroll_x = 0;
            self.detail_pane = DetailPane::Content;
            self.sidebar_edit = None;
            self.status_select_open = false;
            true
        } else {
            false
        }
    }

    /// FetchIssueDetail で取得した Card を detail_stack に push し、表示状態をリセット
    pub fn push_detail_stack(&mut self, card: Card) {
        self.detail_stack.push(card);
        self.sidebar_selected = 0;
        self.detail_scroll = 0;
        self.detail_scroll_x = 0;
        self.detail_pane = DetailPane::Content;
        self.sidebar_edit = None;
        self.status_select_open = false;
    }

    pub(super) fn open_custom_field_edit(&mut self, field_idx: usize) {
        let (field, current_value) = {
            let Some(board) = self.board.as_ref() else {
                return;
            };
            let Some(field) = board.field_definitions.get(field_idx).cloned() else {
                return;
            };
            let current = self
                .selected_card_ref()
                .and_then(|c| {
                    c.custom_fields
                        .iter()
                        .find(|v| v.field_id() == field.id())
                        .cloned()
                });
            (field, current)
        };

        let edit = match &field {
            FieldDefinition::SingleSelect { id, name, options } => {
                let current_option_id = match &current_value {
                    Some(CustomFieldValue::SingleSelect { option_id, .. }) => Some(option_id),
                    _ => None,
                };
                let cursor = current_option_id
                    .and_then(|oid| options.iter().position(|o| &o.id == oid))
                    .unwrap_or(0);
                SidebarEditMode::CustomFieldSingleSelect {
                    field_id: id.clone(),
                    field_name: name.clone(),
                    options: options.clone(),
                    cursor,
                }
            }
            FieldDefinition::Iteration {
                id,
                name,
                iterations,
            } => {
                let current_iteration_id = match &current_value {
                    Some(CustomFieldValue::Iteration { iteration_id, .. }) => Some(iteration_id),
                    _ => None,
                };
                let cursor = current_iteration_id
                    .and_then(|iid| iterations.iter().position(|it| &it.id == iid))
                    .unwrap_or(0);
                SidebarEditMode::CustomFieldIteration {
                    field_id: id.clone(),
                    field_name: name.clone(),
                    iterations: iterations.clone(),
                    cursor,
                }
            }
            FieldDefinition::Text { id, name } => {
                let input = match &current_value {
                    Some(CustomFieldValue::Text { text, .. }) => text.clone(),
                    _ => String::new(),
                };
                let cursor_pos = input.chars().count();
                SidebarEditMode::CustomFieldText {
                    field_id: id.clone(),
                    field_name: name.clone(),
                    input,
                    cursor_pos,
                }
            }
            FieldDefinition::Number { id, name } => {
                let input = match &current_value {
                    Some(CustomFieldValue::Number { number, .. }) => format_number(*number),
                    _ => String::new(),
                };
                let cursor_pos = input.chars().count();
                SidebarEditMode::CustomFieldNumber {
                    field_id: id.clone(),
                    field_name: name.clone(),
                    input,
                    cursor_pos,
                }
            }
            FieldDefinition::Date { id, name } => {
                let input = match &current_value {
                    Some(CustomFieldValue::Date { date, .. }) => date.clone(),
                    _ => String::new(),
                };
                let cursor_pos = input.chars().count();
                SidebarEditMode::CustomFieldDate {
                    field_id: id.clone(),
                    field_name: name.clone(),
                    input,
                    cursor_pos,
                }
            }
        };
        self.sidebar_edit = Some(edit);
    }

    pub(super) fn handle_comment_list_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::CommentList, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let comment_count = self
            .selected_card_ref()
            .map(|c| c.comments.len())
            .unwrap_or(0);

        match action {
            Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
            Action::Back | Action::Quit => {
                self.exit_comment_list();
                Command::None
            }
            Action::MoveDown => {
                if let Some(cls) = self.comment_list_state_mut()
                    && comment_count > 0
                {
                    cls.cursor = (cls.cursor + 1).min(comment_count - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                if let Some(cls) = self.comment_list_state_mut() {
                    cls.cursor = cls.cursor.saturating_sub(1);
                }
                Command::None
            }
            Action::EditComment => {
                let cls = match self.comment_list_state() {
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
                // 自分のコメントのみ編集可能
                if comment.author != self.viewer_login {
                    return Command::None;
                }
                let content_id = cls.content_id.clone();
                let comment_id = comment.id.clone();
                let body = comment.body.clone();
                Command::OpenEditorForComment {
                    content_id,
                    existing: Some((comment_id, body)),
                }
            }
            Action::NewComment => {
                let content_id = match self.comment_list_state() {
                    Some(s) => s.content_id.clone(),
                    None => return Command::None,
                };
                Command::OpenEditorForComment {
                    content_id,
                    existing: None,
                }
            }
            Action::OpenReactionPicker => self.open_reaction_picker_for_comment(),
            _ => Command::None,
        }
    }

    pub(super) fn find_card_by_content_id_mut(&mut self, content_id: &str) -> Option<&mut Card> {
        let board = self.board.as_mut()?;
        for col in &mut board.columns {
            for card in &mut col.cards {
                if card.content_id.as_deref() == Some(content_id) {
                    return Some(card);
                }
            }
        }
        None
    }

    pub(super) fn handle_status_select_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::StatusSelect, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let column_count = self
            .board
            .as_ref()
            .map(|b| b.columns.len())
            .unwrap_or(0);

        match action {
            Action::MoveDown => {
                if self.status_select_cursor + 1 < column_count {
                    self.status_select_cursor += 1;
                }
                Command::None
            }
            Action::MoveUp => {
                self.status_select_cursor = self.status_select_cursor.saturating_sub(1);
                Command::None
            }
            Action::Select => {
                self.status_select_open = false;
                let target = self.status_select_cursor;
                if target == self.selected_column {
                    return Command::None;
                }
                self.move_card_to_and_follow(target)
            }
            Action::Back => {
                self.status_select_open = false;
                Command::None
            }
            _ => Command::None,
        }
    }

    /// カードの URL からリポジトリの owner/name を抽出
    pub(super) fn repo_from_card(&self) -> Option<(String, String)> {
        let card = self.selected_card_ref()?;
        let url = card.url.as_deref()?;
        // https://github.com/{owner}/{repo}/issues/123
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 5 {
            Some((parts[3].to_string(), parts[4].to_string()))
        } else {
            None
        }
    }

    pub(super) fn open_label_edit(&mut self) -> Command {
        let card = self.selected_card_ref();
        if card.map(|c| c.content_id.is_none()).unwrap_or(true) {
            return Command::None; // DraftIssue はラベル編集不可
        }
        if let Some((owner, repo)) = self.repo_from_card() {
            Command::FetchLabels { owner, repo }
        } else {
            Command::None
        }
    }

    pub(super) fn open_assignee_edit(&mut self) -> Command {
        let card = self.selected_card_ref();
        if card.map(|c| c.content_id.is_none()).unwrap_or(true) {
            return Command::None; // DraftIssue はアサイニー編集不可
        }
        if let Some((owner, repo)) = self.repo_from_card() {
            Command::FetchAssignees { owner, repo }
        } else {
            Command::None
        }
    }

    pub(super) fn handle_sidebar_edit_key(&mut self, key: KeyEvent) -> Command {
        // テキスト入力系は SidebarEdit のキーマップを通さず直接処理
        if matches!(
            self.sidebar_edit,
            Some(SidebarEditMode::CustomFieldText { .. })
                | Some(SidebarEditMode::CustomFieldNumber { .. })
                | Some(SidebarEditMode::CustomFieldDate { .. })
        ) {
            return self.handle_custom_field_text_key(key);
        }

        let action = match self.keymap.resolve(KeymapMode::SidebarEdit, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let edit = match &mut self.sidebar_edit {
            Some(e) => e,
            None => return Command::None,
        };

        let (items_len, cursor) = match edit {
            SidebarEditMode::Labels { items, cursor } => (items.len(), cursor),
            SidebarEditMode::Assignees { items, cursor } => (items.len(), cursor),
            SidebarEditMode::CustomFieldSingleSelect { options, cursor, .. } => {
                (options.len() + 1, cursor) // +1 は "None" (クリア)
            }
            SidebarEditMode::CustomFieldIteration { iterations, cursor, .. } => {
                (iterations.len() + 1, cursor)
            }
            SidebarEditMode::CustomFieldText { .. }
            | SidebarEditMode::CustomFieldNumber { .. }
            | SidebarEditMode::CustomFieldDate { .. } => unreachable!("dispatched above"),
        };

        match action {
            Action::Back | Action::Quit => {
                self.sidebar_edit = None;
                Command::None
            }
            Action::MoveDown => {
                if *cursor + 1 < items_len {
                    *cursor += 1;
                }
                Command::None
            }
            Action::MoveUp => {
                *cursor = cursor.saturating_sub(1);
                Command::None
            }
            Action::ToggleItem => {
                self.toggle_sidebar_edit_item()
            }
            _ => Command::None,
        }
    }

    pub(super) fn toggle_sidebar_edit_item(&mut self) -> Command {
        let content_id = match self.selected_card_ref() {
            Some(c) => match &c.content_id {
                Some(id) => id.clone(),
                None => return Command::None,
            },
            None => return Command::None,
        };

        let edit = match &mut self.sidebar_edit {
            Some(e) => e,
            None => return Command::None,
        };

        match edit {
            SidebarEditMode::Labels { items, cursor } => {
                let idx = *cursor;
                if let Some(item) = items.get_mut(idx) {
                    item.applied = !item.applied;
                    let add = item.applied;
                    let label_id = item.id.clone();
                    let label_name = item.name.clone();
                    let label_color = item.color.clone().unwrap_or_default();

                    // 楽観的 UI 更新: Card のラベルを更新
                    if let Some(real_idx) = self.real_card_index()
                        && let Some(board) = &mut self.board
                            && let Some(col) = board.columns.get_mut(self.selected_column)
                                && let Some(card) = col.cards.get_mut(real_idx) {
                                    if add {
                                        card.labels.push(crate::model::project::Label {
                                            id: label_id.clone(),
                                            name: label_name,
                                            color: label_color,
                                        });
                                    } else {
                                        card.labels.retain(|l| l.id != label_id);
                                    }
                                }

                    return Command::ToggleLabel {
                        content_id,
                        label_id,
                        add,
                    };
                }
                Command::None
            }
            SidebarEditMode::Assignees { items, cursor } => {
                let idx = *cursor;
                if let Some(item) = items.get_mut(idx) {
                    item.applied = !item.applied;
                    let add = item.applied;
                    let user_id = item.id.clone();
                    let login = item.name.clone();

                    // 楽観的 UI 更新: Card のアサイニーを更新
                    if let Some(real_idx) = self.real_card_index()
                        && let Some(board) = &mut self.board
                            && let Some(col) = board.columns.get_mut(self.selected_column)
                                && let Some(card) = col.cards.get_mut(real_idx) {
                                    if add {
                                        card.assignees.push(login);
                                    } else {
                                        card.assignees.retain(|a| {
                                            !a.eq_ignore_ascii_case(&login)
                                        });
                                    }
                                }

                    return Command::ToggleAssignee {
                        content_id,
                        user_id,
                        add,
                    };
                }
                Command::None
            }
            SidebarEditMode::CustomFieldSingleSelect { .. }
            | SidebarEditMode::CustomFieldIteration { .. } => {
                self.commit_custom_field_selection()
            }
            SidebarEditMode::CustomFieldText { .. }
            | SidebarEditMode::CustomFieldNumber { .. }
            | SidebarEditMode::CustomFieldDate { .. } => Command::None,
        }
    }

    pub(super) fn commit_custom_field_selection(&mut self) -> Command {
        let project_id = match self.current_project.as_ref() {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };
        let item_id = match self.selected_card_ref() {
            Some(c) => c.item_id.clone(),
            None => return Command::None,
        };

        // 編集モードを取り出してクローズ
        let edit = self.sidebar_edit.take();
        let (field_id, cmd, updated_value): (String, Command, Option<CustomFieldValue>) = match edit
        {
            Some(SidebarEditMode::CustomFieldSingleSelect {
                field_id,
                field_name,
                options,
                cursor,
                ..
            }) => {
                if cursor >= options.len() {
                    // None (クリア)
                    (
                        field_id.clone(),
                        Command::ClearCustomField {
                            project_id,
                            item_id,
                            field_id,
                        },
                        None,
                    )
                } else {
                    let opt = &options[cursor];
                    let new_val = CustomFieldValue::SingleSelect {
                        field_id: field_id.clone(),
                        field_name: field_name.clone(),
                        option_id: opt.id.clone(),
                        name: opt.name.clone(),
                        color: opt.color.clone(),
                    };
                    (
                        field_id.clone(),
                        Command::UpdateCustomField {
                            project_id,
                            item_id,
                            field_id,
                            value: CustomFieldValueInput::SingleSelect {
                                option_id: opt.id.clone(),
                            },
                        },
                        Some(new_val),
                    )
                }
            }
            Some(SidebarEditMode::CustomFieldIteration {
                field_id,
                field_name,
                iterations,
                cursor,
                ..
            }) => {
                if cursor >= iterations.len() {
                    (
                        field_id.clone(),
                        Command::ClearCustomField {
                            project_id,
                            item_id,
                            field_id,
                        },
                        None,
                    )
                } else {
                    let it = &iterations[cursor];
                    let new_val = CustomFieldValue::Iteration {
                        field_id: field_id.clone(),
                        field_name: field_name.clone(),
                        iteration_id: it.id.clone(),
                        title: it.title.clone(),
                    };
                    (
                        field_id.clone(),
                        Command::UpdateCustomField {
                            project_id,
                            item_id,
                            field_id,
                            value: CustomFieldValueInput::Iteration {
                                iteration_id: it.id.clone(),
                            },
                        },
                        Some(new_val),
                    )
                }
            }
            other => {
                // 想定外: 復元
                self.sidebar_edit = other;
                return Command::None;
            }
        };

        self.apply_custom_field_optimistic(&field_id, updated_value);
        cmd
    }

    pub(super) fn apply_custom_field_optimistic(
        &mut self,
        field_id: &str,
        new_value: Option<CustomFieldValue>,
    ) {
        let Some(real_idx) = self.real_card_index() else {
            return;
        };
        let Some(board) = self.board.as_mut() else {
            return;
        };
        let Some(col) = board.columns.get_mut(self.selected_column) else {
            return;
        };
        let Some(card) = col.cards.get_mut(real_idx) else {
            return;
        };
        card.custom_fields.retain(|v| v.field_id() != field_id);
        if let Some(v) = new_value {
            card.custom_fields.push(v);
        }
    }

    pub(super) fn handle_custom_field_text_key(&mut self, key: KeyEvent) -> Command {
        // キーマップは使わず直接 KeyCode を解釈 (Space が ToggleItem に奪われないため)
        match key.code {
            KeyCode::Esc => {
                self.sidebar_edit = None;
                return Command::None;
            }
            KeyCode::Enter => return self.commit_custom_field_text(),
            _ => {}
        }

        let edit = match &mut self.sidebar_edit {
            Some(e) => e,
            None => return Command::None,
        };
        let (input, cursor_pos): (&mut String, &mut usize) = match edit {
            SidebarEditMode::CustomFieldText {
                input, cursor_pos, ..
            }
            | SidebarEditMode::CustomFieldNumber {
                input, cursor_pos, ..
            }
            | SidebarEditMode::CustomFieldDate {
                input, cursor_pos, ..
            } => (input, cursor_pos),
            _ => return Command::None,
        };

        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let byte = input
                    .char_indices()
                    .nth(*cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(input.len());
                input.insert(byte, c);
                *cursor_pos += 1;
            }
            KeyCode::Backspace if *cursor_pos > 0 => {
                let prev_byte = input
                    .char_indices()
                    .nth(*cursor_pos - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let cur_byte = input
                    .char_indices()
                    .nth(*cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(input.len());
                input.drain(prev_byte..cur_byte);
                *cursor_pos -= 1;
            }
            KeyCode::Left => {
                *cursor_pos = cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                let max = input.chars().count();
                *cursor_pos = (*cursor_pos + 1).min(max);
            }
            _ => {}
        }
        Command::None
    }

    pub(super) fn commit_custom_field_text(&mut self) -> Command {
        let project_id = match self.current_project.as_ref() {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };
        let item_id = match self.selected_card_ref() {
            Some(c) => c.item_id.clone(),
            None => return Command::None,
        };
        let edit = self.sidebar_edit.take();
        match edit {
            Some(SidebarEditMode::CustomFieldText {
                field_id, field_name, input, ..
            }) => {
                if input.is_empty() {
                    self.apply_custom_field_optimistic(&field_id, None);
                    Command::ClearCustomField {
                        project_id,
                        item_id,
                        field_id,
                    }
                } else {
                    let new_val = CustomFieldValue::Text {
                        field_id: field_id.clone(),
                        field_name: field_name.clone(),
                        text: input.clone(),
                    };
                    self.apply_custom_field_optimistic(&field_id, Some(new_val));
                    Command::UpdateCustomField {
                        project_id,
                        item_id,
                        field_id,
                        value: CustomFieldValueInput::Text { text: input },
                    }
                }
            }
            Some(SidebarEditMode::CustomFieldNumber {
                field_id,
                field_name,
                input,
                cursor_pos,
            }) => {
                if input.is_empty() {
                    self.apply_custom_field_optimistic(&field_id, None);
                    return Command::ClearCustomField {
                        project_id,
                        item_id,
                        field_id,
                    };
                }
                match input.trim().parse::<f64>() {
                    Ok(n) if n.is_finite() => {
                        let new_val = CustomFieldValue::Number {
                            field_id: field_id.clone(),
                            field_name: field_name.clone(),
                            number: n,
                        };
                        self.apply_custom_field_optimistic(&field_id, Some(new_val));
                        Command::UpdateCustomField {
                            project_id,
                            item_id,
                            field_id,
                            value: CustomFieldValueInput::Number { number: n },
                        }
                    }
                    _ => {
                        // バリデーション失敗: 復元
                        self.sidebar_edit = Some(SidebarEditMode::CustomFieldNumber {
                            field_id,
                            field_name,
                            input,
                            cursor_pos,
                        });
                        Command::None
                    }
                }
            }
            Some(SidebarEditMode::CustomFieldDate {
                field_id,
                field_name,
                input,
                cursor_pos,
            }) => {
                if input.is_empty() {
                    self.apply_custom_field_optimistic(&field_id, None);
                    return Command::ClearCustomField {
                        project_id,
                        item_id,
                        field_id,
                    };
                }
                if is_valid_iso_date(&input) {
                    let new_val = CustomFieldValue::Date {
                        field_id: field_id.clone(),
                        field_name: field_name.clone(),
                        date: input.clone(),
                    };
                    self.apply_custom_field_optimistic(&field_id, Some(new_val));
                    Command::UpdateCustomField {
                        project_id,
                        item_id,
                        field_id,
                        value: CustomFieldValueInput::Date { date: input },
                    }
                } else {
                    self.sidebar_edit = Some(SidebarEditMode::CustomFieldDate {
                        field_id,
                        field_name,
                        input,
                        cursor_pos,
                    });
                    Command::None
                }
            }
            other => {
                self.sidebar_edit = other;
                Command::None
            }
        }
    }

    pub fn selected_card_ref(&self) -> Option<&Card> {
        let real_idx = self.real_card_index()?;
        self.board
            .as_ref()?
            .columns
            .get(self.selected_column)?
            .cards
            .get(real_idx)
    }

    pub(super) fn selected_card_mut(&mut self) -> Option<&mut Card> {
        let real_idx = self.real_card_index()?;
        self.board
            .as_mut()?
            .columns
            .get_mut(self.selected_column)?
            .cards
            .get_mut(real_idx)
    }

    pub(super) fn open_in_browser(&self) -> Command {
        let real_idx = match self.real_card_index() {
            Some(idx) => idx,
            None => return Command::None,
        };
        let url = self
            .board
            .as_ref()
            .and_then(|b| b.columns.get(self.selected_column))
            .and_then(|c| c.cards.get(real_idx))
            .and_then(|card| card.url.as_ref());

        match url {
            Some(url) => Command::OpenUrl(url.clone()),
            None => Command::None,
        }
    }
}
