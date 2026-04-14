use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::command::Command;
use crate::event::AppEvent;
use crate::model::project::{Board, Card, CardType, ProjectSummary};
use crate::model::state::{
    ActiveFilter, CommentListState, ConfirmAction, ConfirmState, CreateCardField,
    CreateCardState, DetailPane, EditCardField, EditCardState, EditItem, FilterState, GrabState,
    LoadingState, NewCardType, PendingIssueCreate, RepoSelectState, SidebarEditMode, ViewMode,
    SIDEBAR_ASSIGNEES, SIDEBAR_DELETE, SIDEBAR_LABELS, SIDEBAR_SECTION_COUNT, SIDEBAR_STATUS,
};

pub struct AppState {
    pub mode: ViewMode,
    pub should_quit: bool,

    // Board state
    pub board: Option<Board>,
    pub selected_column: usize,
    pub selected_card: usize,
    pub scroll_offset: usize,
    pub board_scroll_x: std::cell::Cell<usize>,

    // Project selection
    pub projects: Vec<ProjectSummary>,
    pub selected_project_index: usize,
    pub current_project: Option<ProjectSummary>,

    // Filter
    pub filter: FilterState,

    // Confirm dialog
    pub confirm_state: Option<ConfirmState>,

    // Create card
    pub create_card_state: CreateCardState,

    // Repo selection
    pub repo_select_state: Option<RepoSelectState>,

    // Detail view
    pub detail_scroll: usize,
    pub detail_scroll_x: usize,
    pub detail_max_scroll: std::cell::Cell<usize>,
    pub detail_max_scroll_x: std::cell::Cell<usize>,
    pub detail_pane: DetailPane,
    pub sidebar_selected: usize,
    pub status_select_open: bool,
    pub status_select_cursor: usize,
    pub sidebar_edit: Option<SidebarEditMode>,

    // Edit card
    pub edit_card_state: Option<EditCardState>,

    // Card grab
    pub grab_state: Option<GrabState>,

    // Comment list
    pub comment_list_state: Option<CommentListState>,

    // Loading
    pub loading: LoadingState,

    // CLI options
    pub owner: Option<String>,

    // Viewer info
    pub viewer_login: String,
}

impl AppState {
    pub fn new(owner: Option<String>) -> Self {
        Self {
            mode: ViewMode::ProjectSelect,
            should_quit: false,
            board: None,
            selected_column: 0,
            selected_card: 0,
            scroll_offset: 0,
            board_scroll_x: std::cell::Cell::new(0),
            projects: Vec::new(),
            selected_project_index: 0,
            current_project: None,
            filter: FilterState::default(),
            confirm_state: None,
            create_card_state: CreateCardState::default(),
            repo_select_state: None,
            detail_scroll: 0,
            detail_scroll_x: 0,
            detail_max_scroll: std::cell::Cell::new(0),
            detail_max_scroll_x: std::cell::Cell::new(0),
            detail_pane: DetailPane::Content,
            sidebar_selected: 0,
            status_select_open: false,
            status_select_cursor: 0,
            sidebar_edit: None,
            edit_card_state: None,
            grab_state: None,
            comment_list_state: None,
            loading: LoadingState::Idle,
            owner,
            viewer_login: String::new(),
        }
    }

    pub fn start_loading_projects(&mut self) -> Command {
        self.loading = LoadingState::Loading("Loading projects...".into());
        Command::LoadProjects {
            owner: self.owner.clone(),
        }
    }

    pub fn start_loading_project_by_number(
        &mut self,
        owner: Option<String>,
        number: i32,
    ) -> Command {
        self.loading = LoadingState::Loading("Loading project...".into());
        Command::LoadProjectByNumber { owner, number }
    }

    pub fn start_loading_board(&mut self, project_id: &str) -> Command {
        self.loading = LoadingState::Loading("Loading board...".into());
        Command::LoadBoard {
            project_id: project_id.to_string(),
        }
    }

    pub fn handle_event(&mut self, event: AppEvent) -> Command {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::ProjectsLoaded(Ok(projects)) => {
                self.projects = projects;
                self.loading = LoadingState::Idle;
                if self.projects.len() == 1 {
                    self.select_project(0)
                } else {
                    self.mode = ViewMode::ProjectSelect;
                    Command::None
                }
            }
            AppEvent::ProjectsLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::ProjectLoaded(Ok(project)) => {
                self.current_project = Some(project.clone());
                self.start_loading_board(&project.id)
            }
            AppEvent::ProjectLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::BoardLoaded(Ok(board)) => {
                self.board = Some(board);
                self.selected_column = 0;
                self.selected_card = 0;
                self.scroll_offset = 0;
                self.board_scroll_x.set(0);
                self.loading = LoadingState::Idle;
                self.mode = ViewMode::Board;
                Command::None
            }
            AppEvent::BoardLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::CardMoved(Ok(())) => {
                // 楽観的更新済みなので何もしない
                Command::None
            }
            AppEvent::CardMoved(Err(e)) => {
                self.loading = LoadingState::Error(e);
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CardDeleted(Ok(())) => {
                // 楽観的更新済み
                Command::None
            }
            AppEvent::CardDeleted(Err(e)) => {
                self.loading = LoadingState::Error(e);
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CardCreated(Ok(())) => {
                // 作成成功: ボードをリフレッシュして新しいカードを表示
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CardCreated(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::CardReordered(Ok(())) => {
                // 楽観的更新済みなので何もしない
                Command::None
            }
            AppEvent::CardReordered(Err(e)) => {
                self.loading = LoadingState::Error(e);
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::LabelsLoaded(Ok(labels)) => {
                // カードの現在のラベルと照合して applied を設定
                let card_labels: Vec<String> = self
                    .selected_card_ref()
                    .map(|c| c.labels.iter().map(|l| l.id.clone()).collect())
                    .unwrap_or_default();
                let items: Vec<EditItem> = labels
                    .into_iter()
                    .map(|l| EditItem {
                        applied: card_labels.contains(&l.id),
                        id: l.id,
                        name: l.name,
                        color: Some(l.color),
                    })
                    .collect();
                self.sidebar_edit = Some(SidebarEditMode::Labels { items, cursor: 0 });
                Command::None
            }
            AppEvent::LabelsLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                self.sidebar_edit = None;
                Command::None
            }
            AppEvent::AssigneesLoaded(Ok(users)) => {
                let card_assignees: Vec<String> = self
                    .selected_card_ref()
                    .map(|c| c.assignees.clone())
                    .unwrap_or_default();
                let items: Vec<EditItem> = users
                    .into_iter()
                    .map(|(id, login)| EditItem {
                        applied: card_assignees
                            .iter()
                            .any(|a| a.eq_ignore_ascii_case(&login)),
                        id,
                        name: login,
                        color: None,
                    })
                    .collect();
                self.sidebar_edit = Some(SidebarEditMode::Assignees { items, cursor: 0 });
                Command::None
            }
            AppEvent::AssigneesLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                self.sidebar_edit = None;
                Command::None
            }
            AppEvent::LabelToggled(Ok(())) | AppEvent::AssigneeToggled(Ok(())) => {
                // 楽観的更新済み
                Command::None
            }
            AppEvent::LabelToggled(Err(e)) | AppEvent::AssigneeToggled(Err(e)) => {
                self.loading = LoadingState::Error(e);
                // エラー時はボードをリロード
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CardUpdated(Ok(())) => {
                // 楽観的更新済み
                Command::None
            }
            AppEvent::CardUpdated(Err(e)) => {
                self.loading = LoadingState::Error(e);
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CommentAdded(Ok(comment)) => {
                // 楽観的更新: コメントをカードに追加
                if let Some(card) = self.selected_card_mut() {
                    card.comments.push(comment);
                }
                Command::None
            }
            AppEvent::CommentAdded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::CommentUpdated(Ok(comment)) => {
                // コメント更新: 対象コメントの body を更新
                if let Some(card) = self.selected_card_mut() {
                    if let Some(c) = card.comments.iter_mut().find(|c| c.id == comment.id) {
                        c.body = comment.body;
                    }
                }
                Command::None
            }
            AppEvent::CommentUpdated(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::CommentsLoaded(Ok((content_id, comments))) => {
                // ページネーション結果: カードのコメントを全件で差し替え
                if let Some(board) = &mut self.board {
                    for col in &mut board.columns {
                        for card in &mut col.cards {
                            if card.content_id.as_deref() == Some(&content_id) {
                                card.comments = comments;
                                return Command::None;
                            }
                        }
                    }
                }
                Command::None
            }
            AppEvent::CommentsLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::Tick | AppEvent::Resize(_, _) => Command::None,
        }
    }

    pub fn select_project(&mut self, index: usize) -> Command {
        if let Some(project) = self.projects.get(index) {
            let project = project.clone();
            self.current_project = Some(project.clone());
            self.start_loading_board(&project.id)
        } else {
            Command::None
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Command {
        if key.kind != KeyEventKind::Press {
            return Command::None;
        }

        // Clear error on any key press
        if matches!(self.loading, LoadingState::Error(_)) {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.should_quit = true;
                return Command::None;
            }
            self.loading = LoadingState::Idle;
            return Command::None;
        }

        // Ignore keys while loading
        if matches!(self.loading, LoadingState::Loading(_)) {
            if matches!(key.code, KeyCode::Char('q')) {
                self.should_quit = true;
            }
            return Command::None;
        }

        match self.mode {
            ViewMode::Board => self.handle_board_key(key),
            ViewMode::ProjectSelect => self.handle_project_select_key(key),
            ViewMode::Help => {
                self.handle_help_key(key);
                Command::None
            }
            ViewMode::Filter => {
                self.handle_filter_key(key);
                Command::None
            }
            ViewMode::Confirm => self.handle_confirm_key(key),
            ViewMode::CreateCard => self.handle_create_card_key(key),
            ViewMode::Detail => self.handle_detail_key(key),
            ViewMode::RepoSelect => self.handle_repo_select_key(key),
            ViewMode::CardGrab => self.handle_card_grab_key(key),
            ViewMode::EditCard => self.handle_edit_card_key(key),
            ViewMode::CommentList => self.handle_comment_list_key(key),
        }
    }

    fn handle_board_key(&mut self, key: KeyEvent) -> Command {
        let board = match &self.board {
            Some(b) => b,
            None => return Command::None,
        };

        if board.columns.is_empty() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Char('p') => self.mode = ViewMode::ProjectSelect,
                KeyCode::Char('?') => self.mode = ViewMode::Help,
                _ => {}
            }
            return Command::None;
        }

        let current_col_len = self.filtered_card_indices(self.selected_column).len();

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                Command::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if current_col_len > 0 {
                    self.selected_card = (self.selected_card + 1).min(current_col_len - 1);
                }
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_card = self.selected_card.saturating_sub(1);
                Command::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.selected_column > 0 {
                    self.selected_column -= 1;
                    self.clamp_card_selection();
                }
                Command::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.selected_column < board.columns.len() - 1 {
                    self.selected_column += 1;
                    self.clamp_card_selection();
                }
                Command::None
            }
            KeyCode::Char('g') => {
                self.selected_card = 0;
                Command::None
            }
            KeyCode::Char('G') => {
                if current_col_len > 0 {
                    self.selected_card = current_col_len - 1;
                }
                Command::None
            }
            KeyCode::Tab => {
                self.selected_column = (self.selected_column + 1) % board.columns.len();
                self.clamp_card_selection();
                Command::None
            }
            KeyCode::BackTab => {
                if self.selected_column == 0 {
                    self.selected_column = board.columns.len() - 1;
                } else {
                    self.selected_column -= 1;
                }
                self.clamp_card_selection();
                Command::None
            }
            KeyCode::Enter => self.open_detail_view(),
            KeyCode::Char('p') => {
                self.mode = ViewMode::ProjectSelect;
                Command::None
            }
            KeyCode::Char('r') => {
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            KeyCode::Char('?') => {
                self.mode = ViewMode::Help;
                Command::None
            }
            KeyCode::Char('/') => {
                self.filter.input.clear();
                self.filter.cursor_pos = 0;
                self.mode = ViewMode::Filter;
                Command::None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.filter.active_filter = None;
                self.clamp_card_selection();
                Command::None
            }
            KeyCode::Char('d') => {
                self.start_delete_card(ViewMode::Board);
                Command::None
            }
            KeyCode::Char('n') => {
                self.create_card_state = CreateCardState::default();
                self.mode = ViewMode::CreateCard;
                Command::None
            }
            KeyCode::Char(' ') => {
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
            KeyCode::Char('H') => self.move_card_left(),
            KeyCode::Char('L') => self.move_card_right(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    fn move_card_to(&mut self, target_column: usize) -> Command {
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
        let card = board.columns[src_col].cards.remove(real_idx);
        let item_id = card.item_id.clone();
        board.columns[target_column].cards.push(card);

        // フィルタ後の表示インデックスを再計算して調整
        let filtered_len = board.columns[src_col]
            .cards
            .iter()
            .filter(|c| {
                self.filter
                    .active_filter
                    .as_ref()
                    .map_or(true, |f| f.matches(c))
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
        let field_id = board.status_field_id.clone();

        Command::MoveCard {
            project_id,
            item_id,
            field_id,
            option_id: target_option_id,
        }
    }

    fn move_card_left(&mut self) -> Command {
        if self.selected_column == 0 {
            return Command::None;
        }
        // "No Status" カラムをスキップ
        let target = if self.selected_column >= 1
            && self
                .board
                .as_ref()
                .map(|b| b.columns[self.selected_column - 1].option_id.is_empty())
                .unwrap_or(false)
            && self.selected_column >= 2
        {
            self.selected_column - 2
        } else {
            self.selected_column - 1
        };
        self.move_card_to(target)
    }

    fn move_card_right(&mut self) -> Command {
        let max = self
            .board
            .as_ref()
            .map(|b| b.columns.len())
            .unwrap_or(0);
        if self.selected_column + 1 >= max {
            return Command::None;
        }
        self.move_card_to(self.selected_column + 1)
    }

    fn handle_project_select_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.board.is_some() {
                    self.mode = ViewMode::Board;
                } else {
                    self.should_quit = true;
                }
                Command::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.projects.is_empty() {
                    self.selected_project_index =
                        (self.selected_project_index + 1).min(self.projects.len() - 1);
                }
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_project_index = self.selected_project_index.saturating_sub(1);
                Command::None
            }
            KeyCode::Enter => self.select_project(self.selected_project_index),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = ViewMode::Board;
            }
            KeyCode::Enter => {
                if self.filter.input.is_empty() {
                    self.filter.active_filter = None;
                } else {
                    self.filter.active_filter = Some(ActiveFilter::parse(&self.filter.input));
                }
                self.selected_card = 0;
                self.scroll_offset = 0;
                self.mode = ViewMode::Board;
            }
            KeyCode::Backspace => {
                if self.filter.cursor_pos > 0 {
                    let prev = prev_char_pos(&self.filter.input, self.filter.cursor_pos);
                    self.filter.input.drain(prev..self.filter.cursor_pos);
                    self.filter.cursor_pos = prev;
                }
            }
            KeyCode::Left => {
                if self.filter.cursor_pos > 0 {
                    self.filter.cursor_pos =
                        prev_char_pos(&self.filter.input, self.filter.cursor_pos);
                }
            }
            KeyCode::Right => {
                if self.filter.cursor_pos < self.filter.input.len() {
                    self.filter.cursor_pos =
                        next_char_pos(&self.filter.input, self.filter.cursor_pos);
                }
            }
            KeyCode::Char(c) => {
                self.filter.input.insert(self.filter.cursor_pos, c);
                self.filter.cursor_pos += c.len_utf8();
            }
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Char('y') => {
                let cmd = if let Some(state) = self.confirm_state.take() {
                    let return_to = state.return_to;
                    let cmd = match state.action {
                        ConfirmAction::DeleteCard { item_id } => self.delete_card(&item_id),
                    };
                    // 削除後: カードが消えるので Detail には留まれない → Board に戻る
                    // それ以外の action は return_to に従う
                    self.mode = match &cmd {
                        Command::DeleteCard { .. } => ViewMode::Board,
                        _ => return_to,
                    };
                    cmd
                } else {
                    Command::None
                };
                cmd
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                let return_to = self
                    .confirm_state
                    .as_ref()
                    .map(|s| s.return_to.clone())
                    .unwrap_or(ViewMode::Board);
                self.confirm_state = None;
                self.mode = return_to;
                Command::None
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    fn handle_create_card_key(&mut self, key: KeyEvent) -> Command {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Command::None;
        }
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.submit_create_card();
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = ViewMode::Board;
            }
            KeyCode::Tab => {
                self.create_card_state.focused_field =
                    match self.create_card_state.focused_field {
                        CreateCardField::Type => CreateCardField::Title,
                        CreateCardField::Title => CreateCardField::Body,
                        CreateCardField::Body => CreateCardField::Type,
                    };
            }
            KeyCode::BackTab => {
                self.create_card_state.focused_field =
                    match self.create_card_state.focused_field {
                        CreateCardField::Type => CreateCardField::Body,
                        CreateCardField::Title => CreateCardField::Type,
                        CreateCardField::Body => CreateCardField::Title,
                    };
            }
            // Type field: ← → / h l でトグル
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l')
                if self.create_card_state.focused_field == CreateCardField::Type =>
            {
                self.create_card_state.card_type = match self.create_card_state.card_type {
                    NewCardType::Draft => NewCardType::Issue,
                    NewCardType::Issue => NewCardType::Draft,
                };
            }
            // Body field: Enter で $EDITOR 起動
            KeyCode::Enter
                if self.create_card_state.focused_field == CreateCardField::Body =>
            {
                let content = self.create_card_state.body_input.clone();
                return Command::OpenEditor { content };
            }
            // Title field: テキスト編集
            KeyCode::Backspace
                if self.create_card_state.focused_field == CreateCardField::Title =>
            {
                let cursor = &mut self.create_card_state.title_cursor;
                if *cursor > 0 {
                    let prev = prev_char_pos(&self.create_card_state.title_input, *cursor);
                    self.create_card_state.title_input.drain(prev..*cursor);
                    *cursor = prev;
                }
            }
            KeyCode::Left
                if self.create_card_state.focused_field == CreateCardField::Title =>
            {
                let cursor = &mut self.create_card_state.title_cursor;
                if *cursor > 0 {
                    *cursor = prev_char_pos(&self.create_card_state.title_input, *cursor);
                }
            }
            KeyCode::Right
                if self.create_card_state.focused_field == CreateCardField::Title =>
            {
                let cursor = &mut self.create_card_state.title_cursor;
                if *cursor < self.create_card_state.title_input.len() {
                    *cursor = next_char_pos(&self.create_card_state.title_input, *cursor);
                }
            }
            KeyCode::Char(c)
                if self.create_card_state.focused_field == CreateCardField::Title =>
            {
                let cursor = &mut self.create_card_state.title_cursor;
                self.create_card_state.title_input.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            _ => {}
        }
        Command::None
    }

    fn start_delete_card(&mut self, return_to: ViewMode) {
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
                action: ConfirmAction::DeleteCard {
                    item_id: card.item_id.clone(),
                },
                title: card.title.clone(),
                return_to,
            });
            self.mode = ViewMode::Confirm;
        }
    }

    fn delete_card(&mut self, item_id: &str) -> Command {
        // 楽観的UI更新: ローカルモデルからカードを削除
        if let Some(board) = &mut self.board {
            if let Some(col) = board.columns.get_mut(self.selected_column) {
                if let Some(pos) = col.cards.iter().position(|c| c.item_id == item_id) {
                    col.cards.remove(pos);
                    // フィルタ後の表示カード数で選択を調整
                    let filtered_len = col
                        .cards
                        .iter()
                        .filter(|c| {
                            self.filter
                                .active_filter
                                .as_ref()
                                .map_or(true, |f| f.matches(c))
                        })
                        .count();
                    if filtered_len == 0 {
                        self.selected_card = 0;
                    } else {
                        self.selected_card = self.selected_card.min(filtered_len - 1);
                    }
                }
            }
        }

        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };

        Command::DeleteCard {
            project_id,
            item_id: item_id.to_string(),
        }
    }

    fn submit_create_card(&mut self) -> Command {
        let title = self.create_card_state.title_input.trim().to_string();
        if title.is_empty() {
            return Command::None;
        }
        let body = self.create_card_state.body_input.clone();

        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };

        let (field_id, option_id) = match &self.board {
            Some(board) => {
                let col = board.columns.get(self.selected_column);
                let option_id = col.map(|c| c.option_id.clone()).unwrap_or_default();
                (board.status_field_id.clone(), option_id)
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
                    field_id,
                    option_id,
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
                        field_id,
                        option_id,
                    };
                }

                // 複数リポジトリ → セレクタ表示
                self.repo_select_state = Some(RepoSelectState {
                    selected_index: 0,
                    pending_create: PendingIssueCreate {
                        title,
                        body,
                        field_id,
                        option_id,
                    },
                });
                self.mode = ViewMode::RepoSelect;
                Command::None
            }
        }
    }

    fn handle_card_grab_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_card_down();
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_card_up();
                Command::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.grab_move_card_horizontal(-1);
                Command::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.grab_move_card_horizontal(1);
                Command::None
            }
            KeyCode::Char(' ') => self.confirm_grab(),
            KeyCode::Esc => self.cancel_grab(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    fn grab_move_card_horizontal(&mut self, direction: i32) {
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

    fn move_card_up(&mut self) {
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

    fn move_card_down(&mut self) {
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

    fn confirm_grab(&mut self) -> Command {
        self.mode = ViewMode::Board;
        let grab = match self.grab_state.take() {
            Some(g) => g,
            None => return Command::None,
        };

        let board = match &self.board {
            Some(b) => b,
            None => return Command::None,
        };
        let project_id = match &self.current_project {
            Some(p) => p.id.clone(),
            None => return Command::None,
        };

        let current_column = self.selected_column;
        let current_card_index = self.real_card_index().unwrap_or(0);

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
            let field_id = board.status_field_id.clone();
            let option_id = board.columns[current_column].option_id.clone();
            Command::Batch(vec![
                Command::MoveCard {
                    project_id: project_id.clone(),
                    item_id: grab.item_id.clone(),
                    field_id,
                    option_id,
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

    fn cancel_grab(&mut self) -> Command {
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
        if let Some(card) = found_card {
            if grab.origin_column < board.columns.len() {
                let insert_idx =
                    grab.origin_card_index.min(board.columns[grab.origin_column].cards.len());
                board.columns[grab.origin_column]
                    .cards
                    .insert(insert_idx, card);
            }
        }

        self.selected_column = grab.origin_column;
        self.selected_card = grab.origin_card_index;
        Command::None
    }

    fn handle_repo_select_key(&mut self, key: KeyEvent) -> Command {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Command::None;
        }

        let repo_count = self
            .board
            .as_ref()
            .map(|b| b.repositories.len())
            .unwrap_or(0);

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(rs) = &mut self.repo_select_state {
                    if rs.selected_index + 1 < repo_count {
                        rs.selected_index += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(rs) = &mut self.repo_select_state {
                    rs.selected_index = rs.selected_index.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                return self.submit_repo_selection();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.repo_select_state = None;
                self.mode = ViewMode::Board;
            }
            _ => {}
        }
        Command::None
    }

    fn submit_repo_selection(&mut self) -> Command {
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
            field_id: rs.pending_create.field_id,
            option_id: rs.pending_create.option_id,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = ViewMode::Board;
            }
            _ => {}
        }
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
                    .map_or(true, |f| f.matches(card))
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    /// selected_card (フィルタ後の表示インデックス) → 元の cards インデックスに変換
    pub fn real_card_index(&self) -> Option<usize> {
        let indices = self.filtered_card_indices(self.selected_column);
        indices.get(self.selected_card).copied()
    }

    fn clamp_card_selection(&mut self) {
        let filtered_len = self.filtered_card_indices(self.selected_column).len();
        if filtered_len == 0 {
            self.selected_card = 0;
        } else {
            self.selected_card = self.selected_card.min(filtered_len - 1);
        }
        self.scroll_offset = 0;
    }

    pub fn compute_board_scroll_x(
        selected_column: usize,
        current_scroll: usize,
        visible_cols: usize,
        total_cols: usize,
    ) -> usize {
        if visible_cols == 0 || total_cols == 0 {
            return 0;
        }
        let mut scroll = current_scroll;
        if selected_column < scroll {
            scroll = selected_column;
        } else if selected_column >= scroll + visible_cols {
            scroll = selected_column - visible_cols + 1;
        }
        scroll.min(total_cols.saturating_sub(visible_cols))
    }

    fn open_detail_view(&mut self) -> Command {
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

        // コメントが20件（上限）の場合、追加コメントを取得
        if let Some(card) = self.selected_card_ref() {
            if card.comments.len() >= 20 {
                if let Some(content_id) = card.content_id.clone() {
                    return Command::FetchComments { content_id };
                }
            }
        }
        Command::None
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> Command {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
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

        match key.code {
            KeyCode::Char('q') => {
                self.mode = ViewMode::Board;
                Command::None
            }
            KeyCode::Esc => {
                if self.detail_pane == DetailPane::Sidebar {
                    self.detail_pane = DetailPane::Content;
                } else {
                    self.mode = ViewMode::Board;
                }
                Command::None
            }
            KeyCode::Tab => {
                self.detail_pane = match self.detail_pane {
                    DetailPane::Content => DetailPane::Sidebar,
                    DetailPane::Sidebar => DetailPane::Content,
                };
                Command::None
            }
            KeyCode::BackTab => {
                self.detail_pane = match self.detail_pane {
                    DetailPane::Content => DetailPane::Sidebar,
                    DetailPane::Sidebar => DetailPane::Content,
                };
                Command::None
            }
            _ => match self.detail_pane {
                DetailPane::Content => self.handle_detail_content_key(key),
                DetailPane::Sidebar => self.handle_detail_sidebar_key(key),
            },
        }
    }

    fn handle_detail_content_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Char('o') | KeyCode::Enter => {
                self.mode = ViewMode::Board;
                self.open_in_browser()
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.detail_scroll = self
                    .detail_scroll
                    .saturating_add(1)
                    .min(self.detail_max_scroll.get());
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
                Command::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.detail_scroll_x = self.detail_scroll_x.saturating_sub(2);
                Command::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.detail_scroll_x = self
                    .detail_scroll_x
                    .saturating_add(2)
                    .min(self.detail_max_scroll_x.get());
                Command::None
            }
            KeyCode::Char('e') => self.start_edit_card(),
            KeyCode::Char('c') => self.start_new_comment(),
            KeyCode::Char('C') => self.open_comment_list(),
            _ => Command::None,
        }
    }

    fn start_new_comment(&mut self) -> Command {
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

    fn open_comment_list(&mut self) -> Command {
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
        self.comment_list_state = Some(CommentListState {
            cursor: 0,
            content_id,
        });
        self.mode = ViewMode::CommentList;
        Command::None
    }

    fn start_edit_card(&mut self) -> Command {
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

    fn submit_edit_card(&mut self) -> Command {
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

    fn handle_edit_card_key(&mut self, key: KeyEvent) -> Command {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Command::None;
        }

        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.submit_edit_card();
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = ViewMode::Detail;
                self.edit_card_state = None;
                Command::None
            }
            KeyCode::Tab | KeyCode::BackTab => {
                if let Some(ref mut state) = self.edit_card_state {
                    state.focused_field = match state.focused_field {
                        EditCardField::Title => EditCardField::Body,
                        EditCardField::Body => EditCardField::Title,
                    };
                }
                Command::None
            }
            _ => {
                let focused = self
                    .edit_card_state
                    .as_ref()
                    .map(|s| s.focused_field.clone());
                match focused {
                    Some(EditCardField::Title) => self.handle_edit_card_title_key(key),
                    Some(EditCardField::Body) => self.handle_edit_card_body_key(key),
                    None => Command::None,
                }
            }
        }
    }

    fn handle_edit_card_title_key(&mut self, key: KeyEvent) -> Command {
        let state = match self.edit_card_state.as_mut() {
            Some(s) => s,
            None => return Command::None,
        };
        match key.code {
            KeyCode::Backspace => {
                if state.title_cursor > 0 {
                    let prev = prev_char_pos(&state.title_input, state.title_cursor);
                    state.title_input.drain(prev..state.title_cursor);
                    state.title_cursor = prev;
                }
                Command::None
            }
            KeyCode::Left => {
                if state.title_cursor > 0 {
                    state.title_cursor =
                        prev_char_pos(&state.title_input, state.title_cursor);
                }
                Command::None
            }
            KeyCode::Right => {
                if state.title_cursor < state.title_input.len() {
                    state.title_cursor =
                        next_char_pos(&state.title_input, state.title_cursor);
                }
                Command::None
            }
            KeyCode::Char(c) => {
                state.title_input.insert(state.title_cursor, c);
                state.title_cursor += c.len_utf8();
                Command::None
            }
            _ => Command::None,
        }
    }

    fn handle_edit_card_body_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Enter => {
                let content = self
                    .edit_card_state
                    .as_ref()
                    .map(|s| s.body_input.clone())
                    .unwrap_or_default();
                Command::OpenEditor { content }
            }
            _ => Command::None,
        }
    }

    fn handle_detail_sidebar_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.sidebar_selected =
                    (self.sidebar_selected + 1).min(SIDEBAR_SECTION_COUNT - 1);
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sidebar_selected = self.sidebar_selected.saturating_sub(1);
                Command::None
            }
            KeyCode::Enter => match self.sidebar_selected {
                SIDEBAR_STATUS => {
                    self.status_select_open = true;
                    self.status_select_cursor = self.selected_column;
                    Command::None
                }
                SIDEBAR_LABELS => self.open_label_edit(),
                SIDEBAR_ASSIGNEES => self.open_assignee_edit(),
                SIDEBAR_DELETE => {
                    self.start_delete_card(ViewMode::Detail);
                    Command::None
                }
                _ => Command::None,
            },
            KeyCode::Char('d') => {
                self.start_delete_card(ViewMode::Detail);
                Command::None
            }
            _ => Command::None,
        }
    }

    fn handle_comment_list_key(&mut self, key: KeyEvent) -> Command {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Command::None;
        }

        let comment_count = self
            .selected_card_ref()
            .map(|c| c.comments.len())
            .unwrap_or(0);

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.comment_list_state = None;
                self.mode = ViewMode::Detail;
                Command::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(ref mut cls) = self.comment_list_state {
                    if comment_count > 0 {
                        cls.cursor = (cls.cursor + 1).min(comment_count - 1);
                    }
                }
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(ref mut cls) = self.comment_list_state {
                    cls.cursor = cls.cursor.saturating_sub(1);
                }
                Command::None
            }
            KeyCode::Char('e') => {
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
            KeyCode::Char('c') => {
                let content_id = match &self.comment_list_state {
                    Some(s) => s.content_id.clone(),
                    None => return Command::None,
                };
                Command::OpenEditorForComment {
                    content_id,
                    existing: None,
                }
            }
            _ => Command::None,
        }
    }

    fn handle_status_select_key(&mut self, key: KeyEvent) -> Command {
        let column_count = self
            .board
            .as_ref()
            .map(|b| b.columns.len())
            .unwrap_or(0);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.status_select_cursor + 1 < column_count {
                    self.status_select_cursor += 1;
                }
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.status_select_cursor = self.status_select_cursor.saturating_sub(1);
                Command::None
            }
            KeyCode::Enter => {
                self.status_select_open = false;
                let target = self.status_select_cursor;
                if target == self.selected_column {
                    return Command::None;
                }
                self.move_card_to_and_follow(target)
            }
            KeyCode::Esc => {
                self.status_select_open = false;
                Command::None
            }
            _ => Command::None,
        }
    }

    /// カードを別カラムに移動し、選択状態を移動先に追従させる (詳細ビュー用)
    fn move_card_to_and_follow(&mut self, target_column: usize) -> Command {
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
        if let Some(board) = &self.board {
            if let Some(col) = board.columns.get(target_column) {
                if let Some(real_idx) = col.cards.iter().position(|c| c.item_id == item_id) {
                    let filtered = self.filtered_card_indices(target_column);
                    self.selected_card = filtered
                        .iter()
                        .position(|&i| i == real_idx)
                        .unwrap_or(0);
                }
            }
        }

        cmd
    }

    /// カードの URL からリポジトリの owner/name を抽出
    fn repo_from_card(&self) -> Option<(String, String)> {
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

    fn open_label_edit(&mut self) -> Command {
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

    fn open_assignee_edit(&mut self) -> Command {
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

    fn handle_sidebar_edit_key(&mut self, key: KeyEvent) -> Command {
        let edit = match &mut self.sidebar_edit {
            Some(e) => e,
            None => return Command::None,
        };

        let (items_len, cursor) = match edit {
            SidebarEditMode::Labels { items, cursor } => (items.len(), cursor),
            SidebarEditMode::Assignees { items, cursor } => (items.len(), cursor),
        };

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.sidebar_edit = None;
                Command::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if *cursor + 1 < items_len {
                    *cursor += 1;
                }
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *cursor = cursor.saturating_sub(1);
                Command::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.toggle_sidebar_edit_item()
            }
            _ => Command::None,
        }
    }

    fn toggle_sidebar_edit_item(&mut self) -> Command {
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
                    if let Some(real_idx) = self.real_card_index() {
                        if let Some(board) = &mut self.board {
                            if let Some(col) = board.columns.get_mut(self.selected_column) {
                                if let Some(card) = col.cards.get_mut(real_idx) {
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
                            }
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
                    if let Some(real_idx) = self.real_card_index() {
                        if let Some(board) = &mut self.board {
                            if let Some(col) = board.columns.get_mut(self.selected_column) {
                                if let Some(card) = col.cards.get_mut(real_idx) {
                                    if add {
                                        card.assignees.push(login);
                                    } else {
                                        card.assignees.retain(|a| {
                                            !a.eq_ignore_ascii_case(&login)
                                        });
                                    }
                                }
                            }
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

    fn selected_card_mut(&mut self) -> Option<&mut Card> {
        let real_idx = self.real_card_index()?;
        self.board
            .as_mut()?
            .columns
            .get_mut(self.selected_column)?
            .cards
            .get_mut(real_idx)
    }

    fn open_in_browser(&self) -> Command {
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

pub fn prev_char_pos(s: &str, pos: usize) -> usize {
    let mut p = pos - 1;
    while !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

pub fn next_char_pos(s: &str, pos: usize) -> usize {
    let mut p = pos + 1;
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_card(item_id: &str, title: &str) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: None,
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: Some(format!("https://example.com/{item_id}")),
            body: None,
            comments: vec![],
            milestone: None,
        }
    }

    fn make_card_with_labels(item_id: &str, title: &str, labels: Vec<(&str, &str)>) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: None,
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: labels
                .into_iter()
                .map(|(name, color)| Label {
                    id: format!("label_{name}"),
                    name: name.into(),
                    color: color.into(),
                })
                .collect(),
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
        }
    }

    fn make_card_with_assignees(item_id: &str, title: &str, assignees: Vec<&str>) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: None,
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: assignees.into_iter().map(String::from).collect(),
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
        }
    }

    fn make_card_with_milestone(item_id: &str, title: &str, milestone: &str) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: None,
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: Some(milestone.into()),
        }
    }

    fn make_board(columns: Vec<(&str, &str, Vec<Card>)>) -> Board {
        Board {
            project_title: "Test Project".into(),
            status_field_id: "field_1".into(),
            columns: columns
                .into_iter()
                .map(|(name, option_id, cards)| Column {
                    name: name.into(),
                    option_id: option_id.into(),
                    color: None,
                    cards,
                })
                .collect(),
            repositories: vec![],
        }
    }

    fn make_board_with_repos(
        columns: Vec<(&str, &str, Vec<Card>)>,
        repos: Vec<(&str, &str)>,
    ) -> Board {
        let mut board = make_board(columns);
        board.repositories = repos
            .into_iter()
            .map(|(id, name)| Repository {
                id: id.into(),
                name_with_owner: name.into(),
            })
            .collect();
        board
    }

    fn make_state_with_board(board: Board) -> AppState {
        let mut state = AppState::new(None);
        state.board = Some(board);
        state.mode = ViewMode::Board;
        state.current_project = Some(ProjectSummary {
            id: "proj_1".into(),
            title: "Test".into(),
            number: 1,
            description: None,
        });
        state
    }

    // ========== ナビゲーション ==========

    #[test]
    fn test_move_down() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card("1", "Card 1"),
                make_card("2", "Card 2"),
                make_card("3", "Card 3"),
            ],
        )]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.selected_card, 1);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.selected_card, 2);

        // 末尾でクランプ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.selected_card, 2);
    }

    #[test]
    fn test_move_up() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card 1"), make_card("2", "Card 2")],
        )]);
        let mut state = make_state_with_board(board);
        state.selected_card = 1;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.selected_card, 0);

        // 0で停止
        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.selected_card, 0);
    }

    #[test]
    fn test_move_right_column() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A"), make_card("2", "B"), make_card("3", "C")]),
            ("Done", "opt_2", vec![make_card("4", "D")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_card = 2;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.selected_column, 1);
        // カード選択がクランプされる (Done には 1 枚しかない)
        assert_eq!(state.selected_card, 0);
    }

    #[test]
    fn test_move_left_column() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A")]),
            ("Done", "opt_2", vec![make_card("2", "B")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 1;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('h'))));
        assert_eq!(state.selected_column, 0);

        // 左端で停止
        state.handle_event(AppEvent::Key(key(KeyCode::Char('h'))));
        assert_eq!(state.selected_column, 0);
    }

    #[test]
    fn test_tab_wraps() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A")]),
            ("Doing", "opt_2", vec![make_card("2", "B")]),
            ("Done", "opt_3", vec![make_card("3", "C")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 2;

        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.selected_column, 0);
    }

    #[test]
    fn test_backtab_wraps() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A")]),
            ("Doing", "opt_2", vec![make_card("2", "B")]),
            ("Done", "opt_3", vec![make_card("3", "C")]),
        ]);
        let mut state = make_state_with_board(board);
        assert_eq!(state.selected_column, 0);

        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.selected_column, 2);
    }

    #[test]
    fn test_go_top() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B"), make_card("3", "C")],
        )]);
        let mut state = make_state_with_board(board);
        state.selected_card = 2;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('g'))));
        assert_eq!(state.selected_card, 0);
    }

    #[test]
    fn test_go_bottom() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B"), make_card("3", "C")],
        )]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('G'))));
        assert_eq!(state.selected_card, 2);
    }

    // ========== フィルタ ==========

    #[test]
    fn test_filtered_indices_no_filter() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B")],
        )]);
        let state = make_state_with_board(board);
        assert_eq!(state.filtered_card_indices(0), vec![0, 1]);
    }

    #[test]
    fn test_filtered_indices_text() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card("1", "Fix bug"),
                make_card("2", "Add feature"),
                make_card("3", "Fix typo"),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("fix"));
        assert_eq!(state.filtered_card_indices(0), vec![0, 2]);
    }

    #[test]
    fn test_filtered_indices_label() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card_with_labels("1", "Card 1", vec![("bug", "ff0000")]),
                make_card_with_labels("2", "Card 2", vec![("enhancement", "00ff00")]),
                make_card_with_labels("3", "Card 3", vec![("bug", "ff0000"), ("urgent", "ffff00")]),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("label:bug"));
        assert_eq!(state.filtered_card_indices(0), vec![0, 2]);
    }

    #[test]
    fn test_filtered_indices_assignee() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card_with_assignees("1", "Card 1", vec!["alice"]),
                make_card_with_assignees("2", "Card 2", vec!["bob"]),
                make_card_with_assignees("3", "Card 3", vec!["alice", "bob"]),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("assignee:alice"));
        assert_eq!(state.filtered_card_indices(0), vec![0, 2]);
    }

    #[test]
    fn test_real_card_index() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card("1", "Fix bug"),
                make_card("2", "Add feature"),
                make_card("3", "Fix typo"),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("fix"));
        // filtered indices = [0, 2]
        state.selected_card = 1; // 2番目のフィルタ結果 = index 2
        assert_eq!(state.real_card_index(), Some(2));
    }

    #[test]
    fn test_navigation_with_filter() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card("1", "Fix bug"),
                make_card("2", "Add feature"),
                make_card("3", "Fix typo"),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("fix"));
        // filtered = [0, 2], len = 2

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.selected_card, 1); // 2番目のフィルタ結果

        // 末尾クランプ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.selected_card, 1);
    }

    #[test]
    fn test_clear_filter() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B")],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("A"));

        state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        )));
        assert!(state.filter.active_filter.is_none());
    }

    // ========== カード移動 ==========

    #[test]
    fn test_move_card_right() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A"), make_card("2", "B")]),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('L'))));

        // 楽観的更新: カードが移動している
        let board = state.board.as_ref().unwrap();
        assert_eq!(board.columns[0].cards.len(), 1);
        assert_eq!(board.columns[1].cards.len(), 1);
        assert_eq!(board.columns[1].cards[0].item_id, "1");

        // 正しい Command が返る
        assert_eq!(
            cmd,
            Command::MoveCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
                field_id: "field_1".into(),
                option_id: "opt_2".into(),
            }
        );
    }

    #[test]
    fn test_move_card_left() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![]),
            ("Done", "opt_2", vec![make_card("1", "A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 1;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('H'))));

        let board = state.board.as_ref().unwrap();
        assert_eq!(board.columns[0].cards.len(), 1);
        assert_eq!(board.columns[1].cards.len(), 0);

        assert_eq!(
            cmd,
            Command::MoveCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
                field_id: "field_1".into(),
                option_id: "opt_1".into(),
            }
        );
    }

    #[test]
    fn test_move_card_skip_no_status() {
        let board = make_board(vec![
            ("No Status", "", vec![]),
            ("Todo", "opt_1", vec![make_card("1", "A")]),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 1;

        // 左に移動しようとするが "No Status" はスキップされる
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('H'))));
        assert_eq!(cmd, Command::None);
        // カードはそのまま
        assert_eq!(state.board.as_ref().unwrap().columns[1].cards.len(), 1);
    }

    #[test]
    fn test_move_card_clamp_selection() {
        let board = make_board(vec![
            (
                "Todo",
                "opt_1",
                vec![make_card("1", "A"), make_card("2", "B"), make_card("3", "C")],
            ),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_card = 2; // 最後のカード

        state.handle_event(AppEvent::Key(key(KeyCode::Char('L'))));

        // カード移動後、selected_card はクランプされる
        assert_eq!(state.selected_card, 1); // 残り2枚の最後
    }

    #[test]
    fn test_move_card_with_filter() {
        let board = make_board(vec![
            (
                "Todo",
                "opt_1",
                vec![
                    make_card("1", "Fix bug"),
                    make_card("2", "Add feature"),
                    make_card("3", "Fix typo"),
                ],
            ),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("fix"));
        state.selected_card = 1; // フィルタ後の2番目 = real index 2 (Fix typo)

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('L'))));

        // "Fix typo" (item_id=3) が移動されるべき
        assert_eq!(
            cmd,
            Command::MoveCard {
                project_id: "proj_1".into(),
                item_id: "3".into(),
                field_id: "field_1".into(),
                option_id: "opt_2".into(),
            }
        );
    }

    // ========== カード削除 ==========

    #[test]
    fn test_delete_card() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B")],
        )]);
        let mut state = make_state_with_board(board);

        // d で確認ダイアログ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('d'))));
        assert_eq!(state.mode, ViewMode::Confirm);
        assert!(state.confirm_state.is_some());

        // y で削除実行
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('y'))));
        assert_eq!(
            cmd,
            Command::DeleteCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
            }
        );
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 1);
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_delete_card_clamp() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B")],
        )]);
        let mut state = make_state_with_board(board);
        state.selected_card = 1; // 最後のカード

        // d → y
        state.handle_event(AppEvent::Key(key(KeyCode::Char('d'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('y'))));

        // 削除後にクランプされる
        assert_eq!(state.selected_card, 0);
    }

    #[test]
    fn test_start_delete_sets_confirm() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "My Card")],
        )]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('d'))));

        assert_eq!(state.mode, ViewMode::Confirm);
        let confirm = state.confirm_state.as_ref().unwrap();
        assert_eq!(confirm.title, "My Card");
        match &confirm.action {
            ConfirmAction::DeleteCard { item_id } => assert_eq!(item_id, "1"),
        }
    }

    // ========== カード作成 ==========

    #[test]
    fn test_submit_create_card() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state.title_input = "New Card".into();
        state.create_card_state.body_input = "Description".into();

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));

        assert_eq!(
            cmd,
            Command::CreateCard {
                project_id: "proj_1".into(),
                title: "New Card".into(),
                body: "Description".into(),
                field_id: "field_1".into(),
                option_id: "opt_1".into(),
            }
        );
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_submit_empty_title_noop() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state.title_input = "  ".into(); // 空白のみ

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));

        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::CreateCard); // モードは変わらない
    }

    // ========== イベント処理 ==========

    #[test]
    fn test_projects_loaded() {
        let mut state = AppState::new(None);
        let projects = vec![
            ProjectSummary {
                id: "p1".into(),
                title: "Project 1".into(),
                number: 1,
                description: None,
            },
            ProjectSummary {
                id: "p2".into(),
                title: "Project 2".into(),
                number: 2,
                description: None,
            },
        ];

        let cmd = state.handle_event(AppEvent::ProjectsLoaded(Ok(projects)));

        assert_eq!(state.projects.len(), 2);
        assert!(matches!(state.loading, LoadingState::Idle));
        assert_eq!(state.mode, ViewMode::ProjectSelect);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_projects_loaded_single_auto_selects() {
        let mut state = AppState::new(None);
        let projects = vec![ProjectSummary {
            id: "p1".into(),
            title: "Project 1".into(),
            number: 1,
            description: None,
        }];

        let cmd = state.handle_event(AppEvent::ProjectsLoaded(Ok(projects)));

        // 1つしかない場合は自動選択 → LoadBoard が返る
        assert!(state.current_project.is_some());
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "p1".into(),
            }
        );
    }

    #[test]
    fn test_board_loaded() {
        let mut state = AppState::new(None);
        state.selected_column = 2;
        state.selected_card = 5;
        state.scroll_offset = 3;

        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let cmd = state.handle_event(AppEvent::BoardLoaded(Ok(board)));

        assert!(state.board.is_some());
        assert_eq!(state.selected_column, 0);
        assert_eq!(state.selected_card, 0);
        assert_eq!(state.scroll_offset, 0);
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_card_moved_error_reloads() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::CardMoved(Err("API error".into())));

        // エラー後にリロードが発動するので Loading 状態になる
        assert!(matches!(state.loading, LoadingState::Loading(_)));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
            }
        );
    }

    #[test]
    fn test_error_cleared_on_keypress() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.loading = LoadingState::Error("some error".into());

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));

        assert!(matches!(state.loading, LoadingState::Idle));
    }

    // ========== モード遷移 ==========

    #[test]
    fn test_slash_enters_filter() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('/'))));
        assert_eq!(state.mode, ViewMode::Filter);
    }

    #[test]
    fn test_question_enters_help() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('?'))));
        assert_eq!(state.mode, ViewMode::Help);
    }

    #[test]
    fn test_esc_returns_to_board() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Help;

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_n_enters_create_card() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('n'))));
        assert_eq!(state.mode, ViewMode::CreateCard);
    }

    // ========== ユーティリティ ==========

    #[test]
    fn test_prev_char_pos_ascii() {
        assert_eq!(prev_char_pos("hello", 3), 2);
        assert_eq!(prev_char_pos("hello", 1), 0);
    }

    #[test]
    fn test_prev_char_pos_unicode() {
        let s = "あいう";
        // "あ" = 3 bytes, "い" = 3 bytes, "う" = 3 bytes
        assert_eq!(prev_char_pos(s, 6), 3); // "い" の先頭
        assert_eq!(prev_char_pos(s, 3), 0); // "あ" の先頭
    }

    #[test]
    fn test_next_char_pos_ascii() {
        assert_eq!(next_char_pos("hello", 0), 1);
        assert_eq!(next_char_pos("hello", 3), 4);
    }

    #[test]
    fn test_next_char_pos_unicode() {
        let s = "あいう";
        assert_eq!(next_char_pos(s, 0), 3); // "い" の先頭
        assert_eq!(next_char_pos(s, 3), 6); // "う" の先頭
    }

    // ========== 詳細ビュー ==========

    #[test]
    fn test_enter_opens_detail_view() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert_eq!(state.detail_scroll, 0);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_enter_no_card_noop() {
        let board = make_board(vec![("Todo", "opt_1", vec![])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_esc_returns_to_board() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_q_returns_to_board() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('q'))));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_o_opens_browser() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('o'))));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(cmd, Command::OpenUrl("https://example.com/1".into()));
    }

    #[test]
    fn test_detail_enter_opens_browser() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(cmd, Command::OpenUrl("https://example.com/1".into()));
    }

    #[test]
    fn test_detail_scroll_down() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_max_scroll.set(100);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.detail_scroll, 1);

        state.handle_event(AppEvent::Key(key(KeyCode::Down)));
        assert_eq!(state.detail_scroll, 2);
    }

    #[test]
    fn test_detail_scroll_up() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_max_scroll.set(100);
        state.detail_scroll = 3;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.detail_scroll, 2);

        state.handle_event(AppEvent::Key(key(KeyCode::Up)));
        assert_eq!(state.detail_scroll, 1);

        // 0 でクランプ
        state.detail_scroll = 0;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_detail_scroll_horizontal() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_max_scroll_x.set(100);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.detail_scroll_x, 2);

        state.handle_event(AppEvent::Key(key(KeyCode::Right)));
        assert_eq!(state.detail_scroll_x, 4);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('h'))));
        assert_eq!(state.detail_scroll_x, 2);

        state.handle_event(AppEvent::Key(key(KeyCode::Left)));
        assert_eq!(state.detail_scroll_x, 0);

        // 0 でクランプ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('h'))));
        assert_eq!(state.detail_scroll_x, 0);
    }

    #[test]
    fn test_detail_scroll_clamp_at_max() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_max_scroll.set(3);
        state.detail_max_scroll_x.set(4);

        // 縦スクロール上限
        state.detail_scroll = 3;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.detail_scroll, 3);

        // 横スクロール上限
        state.detail_scroll_x = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.detail_scroll_x, 4);
    }

    // ========== カード作成: タイプ選択 ==========

    #[test]
    fn test_create_card_default_type_is_draft() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);

        // n キーで CreateCard モードへ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('n'))));
        assert_eq!(state.mode, ViewMode::CreateCard);
        assert_eq!(state.create_card_state.card_type, NewCardType::Draft);
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);
    }

    #[test]
    fn test_create_card_type_toggle() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();

        // Type フィールドにフォーカスされた状態で → を押す → Issue に切替
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);
        state.handle_event(AppEvent::Key(key(KeyCode::Right)));
        assert_eq!(state.create_card_state.card_type, NewCardType::Issue);

        // もう一度 → → Draft に戻る
        state.handle_event(AppEvent::Key(key(KeyCode::Right)));
        assert_eq!(state.create_card_state.card_type, NewCardType::Draft);

        // ← でも切替可能
        state.handle_event(AppEvent::Key(key(KeyCode::Left)));
        assert_eq!(state.create_card_state.card_type, NewCardType::Issue);

        // l でも切替可能
        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.create_card_state.card_type, NewCardType::Draft);

        // h でも切替可能
        state.handle_event(AppEvent::Key(key(KeyCode::Char('h'))));
        assert_eq!(state.create_card_state.card_type, NewCardType::Issue);
    }

    #[test]
    fn test_create_card_tab_cycles_fields() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();

        // デフォルトは Type
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);

        // Tab → Title
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Title);

        // Tab → Body
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Body);

        // Tab → Type (ラップ)
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);
    }

    #[test]
    fn test_create_card_backtab_cycles_fields_reverse() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();

        // デフォルトは Type
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);

        // S-Tab → Body (逆方向ラップ)
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Body);

        // S-Tab → Title
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Title);

        // S-Tab → Type
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);
    }

    // ========== カード作成: submit ==========

    #[test]
    fn test_submit_draft_uses_existing_flow() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state.card_type = NewCardType::Draft;
        state.create_card_state.title_input = "My Draft".into();
        state.create_card_state.body_input = "body".into();

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));

        assert_eq!(
            cmd,
            Command::CreateCard {
                project_id: "proj_1".into(),
                title: "My Draft".into(),
                body: "body".into(),
                field_id: "field_1".into(),
                option_id: "opt_1".into(),
            }
        );
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_submit_issue_single_repo_creates_immediately() {
        let board = make_board_with_repos(
            vec![("Todo", "opt_1", vec![make_card("1", "A")])],
            vec![("repo_1", "owner/repo")],
        );
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state.card_type = NewCardType::Issue;
        state.create_card_state.title_input = "My Issue".into();
        state.create_card_state.body_input = "body".into();

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));

        assert_eq!(
            cmd,
            Command::CreateIssue {
                project_id: "proj_1".into(),
                repository_id: "repo_1".into(),
                title: "My Issue".into(),
                body: "body".into(),
                field_id: "field_1".into(),
                option_id: "opt_1".into(),
            }
        );
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_submit_issue_multiple_repos_opens_selector() {
        let board = make_board_with_repos(
            vec![("Todo", "opt_1", vec![make_card("1", "A")])],
            vec![("repo_1", "owner/repo1"), ("repo_2", "owner/repo2")],
        );
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state.card_type = NewCardType::Issue;
        state.create_card_state.title_input = "My Issue".into();
        state.create_card_state.body_input = "body".into();

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));

        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::RepoSelect);
        assert!(state.repo_select_state.is_some());
        let rs = state.repo_select_state.as_ref().unwrap();
        assert_eq!(rs.selected_index, 0);
        assert_eq!(rs.pending_create.title, "My Issue");
    }

    #[test]
    fn test_submit_issue_no_repos_shows_error() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        // repos は空 (make_board はデフォルトで空)
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state.card_type = NewCardType::Issue;
        state.create_card_state.title_input = "My Issue".into();

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));

        assert_eq!(cmd, Command::None);
        assert!(matches!(state.loading, LoadingState::Error(_)));
    }

    // ========== RepoSelect ==========

    fn setup_repo_select_state() -> AppState {
        let board = make_board_with_repos(
            vec![("Todo", "opt_1", vec![make_card("1", "A")])],
            vec![("repo_1", "owner/repo1"), ("repo_2", "owner/repo2"), ("repo_3", "owner/repo3")],
        );
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::RepoSelect;
        state.repo_select_state = Some(RepoSelectState {
            selected_index: 0,
            pending_create: PendingIssueCreate {
                title: "My Issue".into(),
                body: "body".into(),
                field_id: "field_1".into(),
                option_id: "opt_1".into(),
            },
        });
        state
    }

    #[test]
    fn test_repo_select_jk_navigation() {
        let mut state = setup_repo_select_state();

        // j で下に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.repo_select_state.as_ref().unwrap().selected_index, 1);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.repo_select_state.as_ref().unwrap().selected_index, 2);

        // 末尾でクランプ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.repo_select_state.as_ref().unwrap().selected_index, 2);

        // k で上に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.repo_select_state.as_ref().unwrap().selected_index, 1);
    }

    #[test]
    fn test_repo_select_enter_creates_issue() {
        let mut state = setup_repo_select_state();

        // 2番目のリポジトリを選択
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(
            cmd,
            Command::CreateIssue {
                project_id: "proj_1".into(),
                repository_id: "repo_2".into(),
                title: "My Issue".into(),
                body: "body".into(),
                field_id: "field_1".into(),
                option_id: "opt_1".into(),
            }
        );
        assert_eq!(state.mode, ViewMode::Board);
        assert!(state.repo_select_state.is_none());
    }

    #[test]
    fn test_repo_select_esc_cancels() {
        let mut state = setup_repo_select_state();

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));

        assert_eq!(state.mode, ViewMode::Board);
        assert!(state.repo_select_state.is_none());
    }

    // ========== Body: $EDITOR ==========

    #[test]
    fn test_body_enter_opens_editor() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();
        state.create_card_state.focused_field = CreateCardField::Body;
        state.create_card_state.body_input = "existing body".into();

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(
            cmd,
            Command::OpenEditor {
                content: "existing body".into()
            }
        );
    }

    #[test]
    fn test_body_char_input_ignored() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();
        state.create_card_state.focused_field = CreateCardField::Body;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('x'))));

        assert_eq!(state.create_card_state.body_input, "");
    }

    #[test]
    fn test_body_backspace_ignored() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();
        state.create_card_state.focused_field = CreateCardField::Body;
        state.create_card_state.body_input = "hello".into();

        state.handle_event(AppEvent::Key(key(KeyCode::Backspace)));

        assert_eq!(state.create_card_state.body_input, "hello");
    }

    // ========== カード選択モード (CardGrab) ==========

    /// テストヘルパー: Board モードから Space で CardGrab に入る
    fn enter_grab(state: &mut AppState) {
        state.mode = ViewMode::Board;
        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::CardGrab);
        assert!(state.grab_state.is_some());
    }

    #[test]
    fn test_space_enters_card_grab_mode() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;

        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::CardGrab);
        let grab = state.grab_state.as_ref().unwrap();
        assert_eq!(grab.origin_column, 0);
        assert_eq!(grab.origin_card_index, 0);
        assert_eq!(grab.item_id, "1");
    }

    #[test]
    fn test_space_no_card_does_nothing() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![]),
        ]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::Board);
        assert!(state.grab_state.is_none());
    }

    #[test]
    fn test_space_confirms_grab_no_move() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        enter_grab(&mut state);

        // 移動せずに Space で確定 → Command::None
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::Board);
        assert!(state.grab_state.is_none());
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_esc_cancels_grab_restores_position() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        enter_grab(&mut state);

        // j で下に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.selected_card, 1);

        // Esc でキャンセル → 元に戻る
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(state.selected_column, 0);
        assert_eq!(state.selected_card, 0);
        let cards = &state.board.as_ref().unwrap().columns[0].cards;
        assert_eq!(cards[0].title, "Card A");
        assert_eq!(cards[1].title, "Card B");
    }

    #[test]
    fn test_grab_move_down_returns_none() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
                make_card("3", "Card C"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        enter_grab(&mut state);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));

        // 移動中は Command::None
        assert_eq!(cmd, Command::None);
        // カード順序は更新される: B, A, C
        let cards = &state.board.as_ref().unwrap().columns[0].cards;
        assert_eq!(cards[0].title, "Card B");
        assert_eq!(cards[1].title, "Card A");
        assert_eq!(cards[2].title, "Card C");
        assert_eq!(state.selected_card, 1);
        assert_eq!(state.mode, ViewMode::CardGrab);
    }

    #[test]
    fn test_grab_move_down_then_confirm() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
                make_card("3", "Card C"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        enter_grab(&mut state);

        // j で下に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));

        // Space で確定 → ReorderCard
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(
            cmd,
            Command::ReorderCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
                after_id: Some("2".into()),
            }
        );
    }

    #[test]
    fn test_grab_move_up_returns_none() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
                make_card("3", "Card C"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 2;
        enter_grab(&mut state);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));

        assert_eq!(cmd, Command::None);
        // カード順序: A, C, B
        let cards = &state.board.as_ref().unwrap().columns[0].cards;
        assert_eq!(cards[0].title, "Card A");
        assert_eq!(cards[1].title, "Card C");
        assert_eq!(cards[2].title, "Card B");
        assert_eq!(state.selected_card, 1);
    }

    #[test]
    fn test_grab_move_up_then_confirm() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
                make_card("3", "Card C"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 2;
        enter_grab(&mut state);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(
            cmd,
            Command::ReorderCard {
                project_id: "proj_1".into(),
                item_id: "3".into(),
                after_id: Some("1".into()),
            }
        );
    }

    #[test]
    fn test_grab_move_up_to_top_then_confirm() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 1;
        enter_grab(&mut state);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));

        let cards = &state.board.as_ref().unwrap().columns[0].cards;
        assert_eq!(cards[0].title, "Card B");
        assert_eq!(cards[1].title, "Card A");
        assert_eq!(state.selected_card, 0);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(
            cmd,
            Command::ReorderCard {
                project_id: "proj_1".into(),
                item_id: "2".into(),
                after_id: None,
            }
        );
    }

    #[test]
    fn test_grab_move_card_down_at_bottom() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 1;
        enter_grab(&mut state);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(cmd, Command::None);
        // カード順序は変わらない
        let cards = &state.board.as_ref().unwrap().columns[0].cards;
        assert_eq!(cards[0].title, "Card A");
        assert_eq!(cards[1].title, "Card B");
    }

    #[test]
    fn test_grab_move_card_up_at_top() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card A"),
                make_card("2", "Card B"),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        enter_grab(&mut state);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_grab_move_left_returns_none_and_confirm_sends_batch() {
        // Card A は Done の index 1。h で Todo に移動 → Space で確定
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("2", "Card B"), make_card("3", "Card C")]),
            ("Done", "opt_2", vec![make_card("4", "Card D"), make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 1;
        state.selected_card = 1;
        enter_grab(&mut state);

        // h で左に移動 → Command::None
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('h'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::CardGrab);
        assert_eq!(state.selected_column, 0);
        assert_eq!(state.selected_card, 1);
        let cards = &state.board.as_ref().unwrap().columns[0].cards;
        assert_eq!(cards[0].title, "Card B");
        assert_eq!(cards[1].title, "Card A");
        assert_eq!(cards[2].title, "Card C");

        // Space で確定 → Batch(MoveCard + ReorderCard)
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(
            cmd,
            Command::Batch(vec![
                Command::MoveCard {
                    project_id: "proj_1".into(),
                    item_id: "1".into(),
                    field_id: "field_1".into(),
                    option_id: "opt_1".into(),
                },
                Command::ReorderCard {
                    project_id: "proj_1".into(),
                    item_id: "1".into(),
                    after_id: Some("2".into()),
                },
            ])
        );
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_grab_move_right_returns_none_and_confirm_sends_batch() {
        // Card A は Todo の index 0。l で Done に移動 → Space で確定
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A"), make_card("2", "Card B")]),
            ("Done", "opt_2", vec![make_card("3", "Card C"), make_card("4", "Card D")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        enter_grab(&mut state);

        // l で右に移動 → Command::None
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::CardGrab);
        assert_eq!(state.selected_column, 1);
        assert_eq!(state.selected_card, 0);
        let cards = &state.board.as_ref().unwrap().columns[1].cards;
        assert_eq!(cards[0].title, "Card A");
        assert_eq!(cards[1].title, "Card C");
        assert_eq!(cards[2].title, "Card D");

        // Space で確定 → Batch(MoveCard + ReorderCard)
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(
            cmd,
            Command::Batch(vec![
                Command::MoveCard {
                    project_id: "proj_1".into(),
                    item_id: "1".into(),
                    field_id: "field_1".into(),
                    option_id: "opt_2".into(),
                },
                Command::ReorderCard {
                    project_id: "proj_1".into(),
                    item_id: "1".into(),
                    after_id: None,
                },
            ])
        );
    }

    #[test]
    fn test_grab_move_right_clamp_to_end_and_confirm() {
        // Card A は Todo の index 2。Done はカード1枚なので末尾 (index 1) に挿入
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card("1", "Card X"),
                make_card("2", "Card Y"),
                make_card("3", "Card A"),
            ]),
            ("Done", "opt_2", vec![make_card("4", "Card Z")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 2;
        enter_grab(&mut state);

        // l で右に移動
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.selected_column, 1);
        assert_eq!(state.selected_card, 1);
        let cards = &state.board.as_ref().unwrap().columns[1].cards;
        assert_eq!(cards[0].title, "Card Z");
        assert_eq!(cards[1].title, "Card A");

        // Space で確定
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(
            cmd,
            Command::Batch(vec![
                Command::MoveCard {
                    project_id: "proj_1".into(),
                    item_id: "3".into(),
                    field_id: "field_1".into(),
                    option_id: "opt_2".into(),
                },
                Command::ReorderCard {
                    project_id: "proj_1".into(),
                    item_id: "3".into(),
                    after_id: Some("4".into()),
                },
            ])
        );
    }

    #[test]
    fn test_esc_cancels_grab_across_columns() {
        // Card A を Todo から Done に移動後、Esc で元に戻る
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A"), make_card("2", "Card B")]),
            ("Done", "opt_2", vec![make_card("3", "Card C")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        enter_grab(&mut state);

        // l で右に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.selected_column, 1);

        // Esc でキャンセル
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(state.selected_column, 0);
        assert_eq!(state.selected_card, 0);
        // Todo に Card A が戻っている
        let todo_cards = &state.board.as_ref().unwrap().columns[0].cards;
        assert_eq!(todo_cards[0].title, "Card A");
        assert_eq!(todo_cards[1].title, "Card B");
        // Done は元通り
        let done_cards = &state.board.as_ref().unwrap().columns[1].cards;
        assert_eq!(done_cards[0].title, "Card C");
    }

    #[test]
    fn test_reorder_error_reloads_board() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::CardReordered(Err("API error".into())));

        // エラー後にリロードが発動するので Loading 状態になる
        assert!(matches!(state.loading, LoadingState::Loading(_)));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
            }
        );
    }

    // ========== 詳細ビュー: ペイン切り替え ==========

    #[test]
    fn test_detail_tab_switches_pane() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        assert_eq!(state.detail_pane, DetailPane::Content);

        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.detail_pane, DetailPane::Sidebar);

        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.detail_pane, DetailPane::Content);
    }

    #[test]
    fn test_detail_esc_from_sidebar_returns_to_content() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.detail_pane, DetailPane::Content);
        assert_eq!(state.mode, ViewMode::Detail);
    }

    #[test]
    fn test_detail_esc_from_content_returns_to_board() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Board);
    }

    // ========== 詳細ビュー: サイドバーナビゲーション ==========

    #[test]
    fn test_detail_sidebar_jk_navigation() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        assert_eq!(state.sidebar_selected, 0); // Status

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 1); // Assignees

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 2); // Labels

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 3); // Milestone

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 4); // Delete

        // 下限でクランプ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 4);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.sidebar_selected, 3);
    }

    // ========== 詳細ビュー: ステータス変更 (ドロップダウン) ==========

    #[test]
    fn test_detail_status_select_opens() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = 0; // Status

        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert!(state.status_select_open);
        assert_eq!(state.status_select_cursor, 0); // 現在のカラム
    }

    #[test]
    fn test_detail_status_select_move_and_confirm() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
            ("In Progress", "opt_2", vec![]),
            ("Done", "opt_3", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = 0;

        // Enter でドロップダウンを開く
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert!(state.status_select_open);

        // j で "In Progress" に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.status_select_cursor, 1);

        // j でさらに "Done" に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.status_select_cursor, 2);

        // Enter で確定
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert!(!state.status_select_open);
        assert_eq!(state.mode, ViewMode::Detail);
        assert_eq!(state.selected_column, 2); // 移動先に追従
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 0);
        assert_eq!(state.board.as_ref().unwrap().columns[2].cards.len(), 1);
        assert_eq!(
            cmd,
            Command::MoveCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
                field_id: "field_1".into(),
                option_id: "opt_3".into(),
            }
        );
    }

    #[test]
    fn test_detail_status_select_same_column_noop() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = 0;

        // ドロップダウンを開く
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        // そのまま Enter (同じカラム)
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 1);
    }

    #[test]
    fn test_detail_status_select_esc_cancels() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = 0;

        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert!(state.status_select_open);

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert!(!state.status_select_open);
        assert_eq!(state.detail_pane, DetailPane::Sidebar);
    }

    // ========== 詳細ビュー: 削除 ==========

    #[test]
    fn test_detail_sidebar_delete_opens_confirm() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = SIDEBAR_DELETE;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Confirm);
        assert!(state.confirm_state.is_some());
        let cs = state.confirm_state.as_ref().unwrap();
        assert!(matches!(cs.action, ConfirmAction::DeleteCard { .. }));
        assert_eq!(cs.return_to, ViewMode::Detail);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_sidebar_d_key_deletes() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = 0; // Status (d はどのセクションでも動く)

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('d'))));
        assert_eq!(state.mode, ViewMode::Confirm);
        assert!(state.confirm_state.is_some());
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_delete_confirm_yes_returns_to_board() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;

        // d → Confirm
        state.handle_event(AppEvent::Key(key(KeyCode::Char('d'))));
        assert_eq!(state.mode, ViewMode::Confirm);

        // y → 削除実行、Board に戻る
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('y'))));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 0);
        assert_eq!(
            cmd,
            Command::DeleteCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
            }
        );
    }

    #[test]
    fn test_detail_delete_confirm_cancel_returns_to_detail() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;

        // d → Confirm
        state.handle_event(AppEvent::Key(key(KeyCode::Char('d'))));
        assert_eq!(state.mode, ViewMode::Confirm);

        // n → キャンセル、Detail に戻る
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('n'))));
        assert_eq!(state.mode, ViewMode::Detail);
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 1);
        assert_eq!(cmd, Command::None);
    }

    // ========== 詳細ビュー: ラベル編集 ==========

    fn make_issue_card(item_id: &str, title: &str) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: Some(format!("issue_{item_id}")),
            title: title.into(),
            number: Some(1),
            card_type: CardType::Issue {
                state: crate::model::project::IssueState::Open,
            },
            assignees: vec!["alice".into()],
            labels: vec![Label {
                id: "lbl_bug".into(),
                name: "bug".into(),
                color: "d73a4a".into(),
            }],
            url: Some("https://github.com/owner/repo/issues/1".into()),
            body: None,
            comments: vec![],
            milestone: None,
        }
    }

    #[test]
    fn test_detail_label_edit_opens_fetch() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_issue_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = SIDEBAR_LABELS;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::FetchLabels {
                owner: "owner".into(),
                repo: "repo".into(),
            }
        );
    }

    #[test]
    fn test_detail_label_toggle() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_issue_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;

        // LabelsLoaded イベントでエディットモードが開く
        let labels = vec![
            crate::model::project::Label {
                id: "lbl_bug".into(),
                name: "bug".into(),
                color: "d73a4a".into(),
            },
            crate::model::project::Label {
                id: "lbl_feat".into(),
                name: "feature".into(),
                color: "0075ca".into(),
            },
        ];
        state.handle_event(AppEvent::LabelsLoaded(Ok(labels)));
        assert!(state.sidebar_edit.is_some());

        // bug は既に適用済み、feature は未適用
        if let Some(SidebarEditMode::Labels { items, .. }) = &state.sidebar_edit {
            assert!(items[0].applied); // bug
            assert!(!items[1].applied); // feature
        } else {
            panic!("Expected Labels edit mode");
        }

        // j で feature に移動
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));

        // Enter で feature をトグル (追加)
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::ToggleLabel {
                content_id: "issue_1".into(),
                label_id: "lbl_feat".into(),
                add: true,
            }
        );

        // 楽観的更新: カードにラベルが追加されている
        let card = state.selected_card_ref().unwrap();
        assert_eq!(card.labels.len(), 2);

        // Esc で閉じる
        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert!(state.sidebar_edit.is_none());
    }

    #[test]
    fn test_detail_assignee_toggle() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_issue_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = SIDEBAR_ASSIGNEES;

        // FetchAssignees コマンドが返る
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::FetchAssignees {
                owner: "owner".into(),
                repo: "repo".into(),
            }
        );

        // AssigneesLoaded
        let users = vec![
            ("user_alice".into(), "alice".into()),
            ("user_bob".into(), "bob".into()),
        ];
        state.handle_event(AppEvent::AssigneesLoaded(Ok(users)));
        assert!(state.sidebar_edit.is_some());

        if let Some(SidebarEditMode::Assignees { items, .. }) = &state.sidebar_edit {
            assert!(items[0].applied); // alice (already assigned)
            assert!(!items[1].applied); // bob
        }

        // j で bob に移動、Enter でトグル
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::ToggleAssignee {
                content_id: "issue_1".into(),
                user_id: "user_bob".into(),
                add: true,
            }
        );

        // 楽観的更新
        let card = state.selected_card_ref().unwrap();
        assert_eq!(card.assignees.len(), 2);
    }

    #[test]
    fn test_detail_draft_issue_no_label_edit() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Draft")]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = SIDEBAR_LABELS;

        // DraftIssue は content_id が None なので編集不可
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
    }

    // ========== カード編集 ==========

    fn make_draft_card(item_id: &str, title: &str, body: &str) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: Some(format!("draft_{item_id}")),
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: None,
            body: Some(body.into()),
            comments: vec![],
            milestone: None,
        }
    }

    #[test]
    fn test_detail_e_on_draft_enters_edit_mode() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "Draft Card", "Draft body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::EditCard);
        let edit = state.edit_card_state.as_ref().unwrap();
        assert_eq!(edit.title_input, "Draft Card");
        assert_eq!(edit.body_input, "Draft body");
        assert_eq!(edit.content_id, "draft_1");
        assert_eq!(edit.item_id, "1");
    }

    #[test]
    fn test_detail_e_on_issue_enters_edit_mode() {
        let mut card = make_issue_card("1", "Issue Card");
        card.body = Some("Issue body".into());
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::EditCard);
        let edit = state.edit_card_state.as_ref().unwrap();
        assert_eq!(edit.title_input, "Issue Card");
        assert_eq!(edit.body_input, "Issue body");
    }

    #[test]
    fn test_detail_e_without_content_id_is_noop() {
        // make_card は content_id: None
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "No Content ID")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::Detail);
        assert!(state.edit_card_state.is_none());
    }

    #[test]
    fn test_edit_card_title_input() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "Old", "body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));

        // Title にフォーカスされている状態で文字入力
        state.handle_event(AppEvent::Key(key(KeyCode::Char('X'))));
        let edit = state.edit_card_state.as_ref().unwrap();
        assert_eq!(edit.title_input, "OldX");
        assert_eq!(edit.title_cursor, 4);
    }

    #[test]
    fn test_edit_card_title_backspace() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "ABC", "body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));

        state.handle_event(AppEvent::Key(key(KeyCode::Backspace)));
        let edit = state.edit_card_state.as_ref().unwrap();
        assert_eq!(edit.title_input, "AB");
        assert_eq!(edit.title_cursor, 2);
    }

    #[test]
    fn test_edit_card_tab_switches_field() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "T", "body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));

        assert_eq!(
            state.edit_card_state.as_ref().unwrap().focused_field,
            EditCardField::Title
        );

        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(
            state.edit_card_state.as_ref().unwrap().focused_field,
            EditCardField::Body
        );

        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(
            state.edit_card_state.as_ref().unwrap().focused_field,
            EditCardField::Title
        );
    }

    #[test]
    fn test_edit_card_body_enter_opens_editor() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "T", "existing body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));

        // Body にフォーカス
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::OpenEditor {
                content: "existing body".into()
            }
        );
    }

    #[test]
    fn test_edit_card_ctrl_s_submits() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "Old Title", "Old Body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));

        // タイトルを変更
        // カーソルを先頭に移動して全消しして新しいタイトルを入力する代わりに、
        // 末尾に追記する
        state.handle_event(AppEvent::Key(key(KeyCode::Char('!'))));

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            cmd,
            Command::UpdateCard {
                content_id: "draft_1".into(),
                card_type: CardType::DraftIssue,
                title: "Old Title!".into(),
                body: "Old Body".into(),
            }
        );
        // Detail に戻る
        assert_eq!(state.mode, ViewMode::Detail);
        assert!(state.edit_card_state.is_none());
        // 楽観的更新
        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.title, "Old Title!");
        assert_eq!(card.body.as_deref(), Some("Old Body"));
    }

    #[test]
    fn test_edit_card_ctrl_s_empty_title_noop() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "X", "body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));

        // タイトルを空にする
        state.handle_event(AppEvent::Key(key(KeyCode::Backspace)));

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(cmd, Command::None);
        // EditCard モードのまま
        assert_eq!(state.mode, ViewMode::EditCard);
    }

    #[test]
    fn test_edit_card_esc_cancels() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "Original", "body")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Content;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));

        // タイトルを変更してからキャンセル
        state.handle_event(AppEvent::Key(key(KeyCode::Char('Z'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));

        assert_eq!(state.mode, ViewMode::Detail);
        assert!(state.edit_card_state.is_none());
        // ボードは変更なし
        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.title, "Original");
    }

    #[test]
    fn test_card_updated_error_reloads() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_draft_card("1", "T", "B")],
        )]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::CardUpdated(Err("API error".into())));
        // start_loading_board が Loading に上書きするのでリロードが走ることを確認
        assert!(matches!(cmd, Command::LoadBoard { .. }));
    }

    // ========== コメント関連ヘルパー ==========

    fn make_comment(id: &str, author: &str, body: &str) -> Comment {
        Comment {
            id: id.into(),
            author: author.into(),
            body: body.into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        }
    }

    fn make_issue_card_with_comments(
        item_id: &str,
        title: &str,
        comments: Vec<Comment>,
    ) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: Some(format!("issue_{item_id}")),
            title: title.into(),
            number: Some(1),
            card_type: CardType::Issue {
                state: IssueState::Open,
            },
            assignees: vec![],
            labels: vec![],
            url: Some("https://github.com/owner/repo/issues/1".into()),
            body: Some("body".into()),
            comments,
            milestone: None,
        }
    }

    // ========== コメント投稿テスト ==========

    #[test]
    fn test_detail_c_opens_editor_for_new_comment() {
        let card = make_issue_card_with_comments("1", "Card A", vec![]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        // Detail ビューに入る
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert_eq!(state.detail_pane, DetailPane::Content);

        // c で新規コメント用エディタが開く
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('c'))));
        assert!(
            matches!(cmd, Command::OpenEditorForComment { existing: None, .. }),
            "Expected OpenEditorForComment with existing=None, got {:?}",
            cmd
        );
    }

    #[test]
    fn test_detail_c_does_nothing_for_draft_issue() {
        let card = make_card("1", "Draft");
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);

        // DraftIssue では c は何もしない
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('c'))));
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_shift_c_opens_comment_list() {
        let comments = vec![
            make_comment("c1", "alice", "Hello"),
            make_comment("c2", "bob", "World"),
        ];
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);

        // C (Shift+c) でコメント一覧を開く
        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('C'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(state.mode, ViewMode::CommentList);
        assert!(state.comment_list_state.is_some());
        assert_eq!(state.comment_list_state.as_ref().unwrap().cursor, 0);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_comment_list_navigation() {
        let comments = vec![
            make_comment("c1", "alice", "Hello"),
            make_comment("c2", "bob", "World"),
            make_comment("c3", "carol", "!"),
        ];
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('C'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(state.mode, ViewMode::CommentList);

        // j で下に移動
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.comment_list_state.as_ref().unwrap().cursor, 1);

        // もう一度 j
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.comment_list_state.as_ref().unwrap().cursor, 2);

        // 末尾を超えない
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.comment_list_state.as_ref().unwrap().cursor, 2);

        // k で上に移動
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.comment_list_state.as_ref().unwrap().cursor, 1);
    }

    #[test]
    fn test_comment_list_edit_own_comment() {
        let comments = vec![
            make_comment("c1", "me", "My comment"),
            make_comment("c2", "other", "Other comment"),
        ];
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        state.viewer_login = "me".into();

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('C'),
            KeyModifiers::SHIFT,
        )));

        // 自分のコメント（cursor=0, author="me"）で e → OpenEditorForComment
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));
        assert!(
            matches!(cmd, Command::OpenEditorForComment { existing: Some(_), .. }),
            "Expected OpenEditorForComment with existing=Some, got {:?}",
            cmd
        );
    }

    #[test]
    fn test_comment_list_edit_other_comment_does_nothing() {
        let comments = vec![
            make_comment("c1", "other", "Other comment"),
        ];
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        state.viewer_login = "me".into();

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('C'),
            KeyModifiers::SHIFT,
        )));

        // 他人のコメント（cursor=0, author="other"）で e → None
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_comment_list_new_comment() {
        let comments = vec![make_comment("c1", "alice", "Hello")];
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('C'),
            KeyModifiers::SHIFT,
        )));

        // CommentList で c → 新規コメント
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('c'))));
        assert!(
            matches!(cmd, Command::OpenEditorForComment { existing: None, .. }),
            "Expected OpenEditorForComment, got {:?}",
            cmd
        );
    }

    #[test]
    fn test_comment_list_esc_returns_to_detail() {
        let comments = vec![make_comment("c1", "alice", "Hello")];
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('C'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(state.mode, ViewMode::CommentList);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert!(state.comment_list_state.is_none());
    }

    #[test]
    fn test_comments_loaded_replaces_card_comments() {
        let card = make_issue_card_with_comments("1", "Card A", vec![
            make_comment("c1", "alice", "old"),
        ]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let new_comments = vec![
            make_comment("c1", "alice", "old"),
            make_comment("c2", "bob", "new1"),
            make_comment("c3", "carol", "new2"),
        ];
        let _ = state.handle_event(AppEvent::CommentsLoaded(Ok((
            "issue_1".into(),
            new_comments,
        ))));

        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.comments.len(), 3);
        assert_eq!(card.comments[2].author, "carol");
    }

    #[test]
    fn test_comment_added_appends_to_card() {
        let card = make_issue_card_with_comments("1", "Card A", vec![
            make_comment("c1", "alice", "first"),
        ]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        // Detail ビューに入る
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        let new_comment = make_comment("c2", "me", "new comment");
        let _ = state.handle_event(AppEvent::CommentAdded(Ok(new_comment)));

        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.comments.len(), 2);
        assert_eq!(card.comments[1].body, "new comment");
    }

    #[test]
    fn test_comment_updated_modifies_body() {
        let card = make_issue_card_with_comments("1", "Card A", vec![
            make_comment("c1", "me", "old body"),
        ]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        let updated = Comment {
            id: "c1".into(),
            author: String::new(), // update_comment returns empty author
            body: "updated body".into(),
            created_at: String::new(),
        };
        let _ = state.handle_event(AppEvent::CommentUpdated(Ok(updated)));

        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.comments[0].body, "updated body");
    }

    #[test]
    fn test_detail_opens_fetch_comments_when_20() {
        let comments: Vec<Comment> = (0..20)
            .map(|i| make_comment(&format!("c{i}"), "alice", &format!("comment {i}")))
            .collect();
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        // Detail を開く → コメント20件 → FetchComments が返る
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert!(
            matches!(cmd, Command::FetchComments { .. }),
            "Expected FetchComments when 20 comments, got {:?}",
            cmd
        );
    }

    // --- compute_board_scroll_x tests ---

    #[test]
    fn test_board_scroll_x_no_scroll_when_all_fit() {
        // 3 columns, 5 visible → no scroll needed
        assert_eq!(AppState::compute_board_scroll_x(0, 0, 5, 3), 0);
        assert_eq!(AppState::compute_board_scroll_x(2, 0, 5, 3), 0);
    }

    #[test]
    fn test_board_scroll_x_follows_right() {
        // 10 columns, 3 visible, selecting column 3 (0-indexed)
        assert_eq!(AppState::compute_board_scroll_x(3, 0, 3, 10), 1);
        // selecting column 5
        assert_eq!(AppState::compute_board_scroll_x(5, 0, 3, 10), 3);
    }

    #[test]
    fn test_board_scroll_x_follows_left() {
        // current scroll = 5, selecting column 3
        assert_eq!(AppState::compute_board_scroll_x(3, 5, 3, 10), 3);
        // selecting column 0
        assert_eq!(AppState::compute_board_scroll_x(0, 5, 3, 10), 0);
    }

    #[test]
    fn test_board_scroll_x_clamp_at_end() {
        // 10 columns, 3 visible, selecting last column
        assert_eq!(AppState::compute_board_scroll_x(9, 0, 3, 10), 7);
        // cannot scroll past total - visible
        assert_eq!(AppState::compute_board_scroll_x(9, 9, 3, 10), 7);
    }

    #[test]
    fn test_board_scroll_x_stays_when_in_range() {
        // current scroll = 2, selected = 3, visible = 3 → column 3 is in range [2,3,4]
        assert_eq!(AppState::compute_board_scroll_x(3, 2, 3, 10), 2);
    }

    #[test]
    fn test_board_scroll_x_zero_visible() {
        assert_eq!(AppState::compute_board_scroll_x(0, 0, 0, 10), 0);
    }

    #[test]
    fn test_board_scroll_x_zero_total() {
        assert_eq!(AppState::compute_board_scroll_x(0, 0, 3, 0), 0);
    }

    // ========== 複合フィルタ (AND/OR) ==========

    #[test]
    fn test_filtered_indices_and() {
        // label:bug AND assignee:alice → Card 1 のみマッチ
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                {
                    let mut c = make_card_with_labels("1", "Card 1", vec![("bug", "ff0000")]);
                    c.assignees = vec!["alice".into()];
                    c
                },
                make_card_with_labels("2", "Card 2", vec![("bug", "ff0000")]),
                make_card_with_assignees("3", "Card 3", vec!["alice"]),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("label:bug assignee:alice"));
        assert_eq!(state.filtered_card_indices(0), vec![0]);
    }

    #[test]
    fn test_filtered_indices_or() {
        // label:bug OR label:enhancement → Card 1, 2 がマッチ
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card_with_labels("1", "Card 1", vec![("bug", "ff0000")]),
                make_card_with_labels("2", "Card 2", vec![("enhancement", "00ff00")]),
                make_card("3", "Card 3"),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("label:bug | label:enhancement"));
        assert_eq!(state.filtered_card_indices(0), vec![0, 1]);
    }

    #[test]
    fn test_filtered_indices_complex_and_or() {
        // (label:bug AND assignee:alice) OR label:enhancement
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                {
                    let mut c = make_card_with_labels("1", "Card 1", vec![("bug", "ff0000")]);
                    c.assignees = vec!["alice".into()];
                    c
                },
                make_card_with_labels("2", "Card 2", vec![("bug", "ff0000")]),
                make_card_with_labels("3", "Card 3", vec![("enhancement", "00ff00")]),
                make_card("4", "Card 4"),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter =
            Some(ActiveFilter::parse("label:bug assignee:alice | label:enhancement"));
        assert_eq!(state.filtered_card_indices(0), vec![0, 2]);
    }

    #[test]
    fn test_filtered_indices_milestone() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card_with_milestone("1", "Card 1", "v1.0"),
                make_card_with_milestone("2", "Card 2", "v2.0"),
                make_card("3", "Card 3"), // milestone なし
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("milestone:v1"));
        assert_eq!(state.filtered_card_indices(0), vec![0]);
    }

    #[test]
    fn test_filtered_indices_milestone_no_match() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card("1", "Card 1"),
                make_card("2", "Card 2"),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("milestone:v1"));
        let empty: Vec<usize> = vec![];
        assert_eq!(state.filtered_card_indices(0), empty);
    }

    #[test]
    fn test_filtered_indices_text_and_milestone() {
        // text "Fix" AND milestone:v1 → Card 1 のみ
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card_with_milestone("1", "Fix bug", "v1.0"),
                make_card_with_milestone("2", "Add feature", "v1.0"),
                make_card_with_milestone("3", "Fix typo", "v2.0"),
            ],
        )]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("fix milestone:v1"));
        assert_eq!(state.filtered_card_indices(0), vec![0]);
    }

    // ========== Project direct loading (skip project list) ==========

    #[test]
    fn test_start_loading_project_by_number_with_owner() {
        let mut state = AppState::new(Some("myorg".into()));
        let cmd = state.start_loading_project_by_number(Some("myorg".into()), 5);
        assert!(matches!(state.loading, LoadingState::Loading(_)));
        assert_eq!(
            cmd,
            Command::LoadProjectByNumber {
                owner: Some("myorg".into()),
                number: 5,
            }
        );
    }

    #[test]
    fn test_start_loading_project_by_number_without_owner() {
        let mut state = AppState::new(None);
        let cmd = state.start_loading_project_by_number(None, 3);
        assert!(matches!(state.loading, LoadingState::Loading(_)));
        assert_eq!(
            cmd,
            Command::LoadProjectByNumber {
                owner: None,
                number: 3,
            }
        );
    }

    #[test]
    fn test_project_loaded_sets_current_project_and_loads_board() {
        let mut state = AppState::new(None);
        state.loading = LoadingState::Loading("Loading project...".into());

        let project = ProjectSummary {
            id: "proj_42".into(),
            title: "My Project".into(),
            number: 5,
            description: None,
        };
        let cmd = state.handle_event(AppEvent::ProjectLoaded(Ok(project)));

        assert!(state.current_project.is_some());
        let cp = state.current_project.as_ref().unwrap();
        assert_eq!(cp.id, "proj_42");
        assert_eq!(cp.title, "My Project");
        assert_eq!(cp.number, 5);
        assert!(matches!(state.loading, LoadingState::Loading(_)));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_42".into(),
            }
        );
    }

    #[test]
    fn test_project_loaded_error() {
        let mut state = AppState::new(None);
        state.loading = LoadingState::Loading("Loading project...".into());

        let cmd = state.handle_event(AppEvent::ProjectLoaded(Err(
            "Project #99 not found".into(),
        )));

        assert!(matches!(state.loading, LoadingState::Error(_)));
        assert_eq!(cmd, Command::None);
    }
}
