use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::command::Command;
use crate::event::AppEvent;
use crate::model::project::{Board, Card, ProjectSummary};
use crate::model::state::{
    ActiveFilter, ConfirmAction, ConfirmState, CreateCardField, CreateCardState, FilterState,
    LoadingState, ViewMode,
};

pub struct AppState {
    pub mode: ViewMode,
    pub should_quit: bool,

    // Board state
    pub board: Option<Board>,
    pub selected_column: usize,
    pub selected_card: usize,
    pub scroll_offset: usize,

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

    // Detail view
    pub detail_scroll: usize,
    pub detail_scroll_x: usize,
    pub detail_max_scroll: std::cell::Cell<usize>,
    pub detail_max_scroll_x: std::cell::Cell<usize>,

    // Loading
    pub loading: LoadingState,

    // CLI options
    pub owner: Option<String>,
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
            projects: Vec::new(),
            selected_project_index: 0,
            current_project: None,
            filter: FilterState::default(),
            confirm_state: None,
            create_card_state: CreateCardState::default(),
            detail_scroll: 0,
            detail_scroll_x: 0,
            detail_max_scroll: std::cell::Cell::new(0),
            detail_max_scroll_x: std::cell::Cell::new(0),
            loading: LoadingState::Idle,
            owner,
        }
    }

    pub fn start_loading_projects(&mut self) -> Command {
        self.loading = LoadingState::Loading("Loading projects...".into());
        Command::LoadProjects {
            owner: self.owner.clone(),
        }
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
            AppEvent::BoardLoaded(Ok(board)) => {
                self.board = Some(board);
                self.selected_column = 0;
                self.selected_card = 0;
                self.scroll_offset = 0;
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

    pub fn select_project_by_number(&mut self, number: i32) -> Command {
        if let Some(idx) = self.projects.iter().position(|p| p.number == number) {
            self.select_project(idx)
        } else {
            self.loading = LoadingState::Error(format!("Project #{number} not found"));
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
                self.start_delete_card();
                Command::None
            }
            KeyCode::Char('n') => {
                self.create_card_state = CreateCardState::default();
                self.mode = ViewMode::CreateCard;
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
                    match state.action {
                        ConfirmAction::DeleteCard { item_id } => self.delete_card(&item_id),
                    }
                } else {
                    Command::None
                };
                self.mode = ViewMode::Board;
                cmd
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.confirm_state = None;
                self.mode = ViewMode::Board;
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
                        CreateCardField::Title => CreateCardField::Body,
                        CreateCardField::Body => CreateCardField::Title,
                    };
            }
            KeyCode::Backspace => {
                let (input, cursor) = self.active_create_field_mut();
                if *cursor > 0 {
                    let prev = prev_char_pos(input, *cursor);
                    input.drain(prev..*cursor);
                    *cursor = prev;
                }
            }
            KeyCode::Left => {
                let (input, cursor) = self.active_create_field_mut();
                if *cursor > 0 {
                    *cursor = prev_char_pos(input, *cursor);
                }
            }
            KeyCode::Right => {
                let (input, cursor) = self.active_create_field_mut();
                if *cursor < input.len() {
                    *cursor = next_char_pos(input, *cursor);
                }
            }
            KeyCode::Char(c) => {
                let (input, cursor) = self.active_create_field_mut();
                input.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            _ => {}
        }
        Command::None
    }

    fn active_create_field_mut(&mut self) -> (&mut String, &mut usize) {
        match self.create_card_state.focused_field {
            CreateCardField::Title => (
                &mut self.create_card_state.title_input,
                &mut self.create_card_state.title_cursor,
            ),
            CreateCardField::Body => (
                &mut self.create_card_state.body_input,
                &mut self.create_card_state.body_cursor,
            ),
        }
    }

    fn start_delete_card(&mut self) {
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

        // カラムのステータスを設定するための情報
        let (field_id, option_id) = match &self.board {
            Some(board) => {
                let col = board.columns.get(self.selected_column);
                let option_id = col.map(|c| c.option_id.clone()).unwrap_or_default();
                (board.status_field_id.clone(), option_id)
            }
            None => return Command::None,
        };

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

    fn open_detail_view(&mut self) -> Command {
        if self.real_card_index().is_none() {
            return Command::None;
        }
        self.detail_scroll = 0;
        self.detail_scroll_x = 0;
        self.mode = ViewMode::Detail;
        Command::None
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = ViewMode::Board;
                Command::None
            }
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
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                Command::None
            }
            _ => Command::None,
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
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: Some(format!("https://example.com/{item_id}")),
            body: None,
            comments: vec![],
        }
    }

    fn make_card_with_labels(item_id: &str, title: &str, labels: Vec<(&str, &str)>) -> Card {
        Card {
            item_id: item_id.into(),
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: labels
                .into_iter()
                .map(|(name, color)| Label {
                    name: name.into(),
                    color: color.into(),
                })
                .collect(),
            url: None,
            body: None,
            comments: vec![],
        }
    }

    fn make_card_with_assignees(item_id: &str, title: &str, assignees: Vec<&str>) -> Card {
        Card {
            item_id: item_id.into(),
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: assignees.into_iter().map(String::from).collect(),
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
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
                    cards,
                })
                .collect(),
        }
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
        state.filter.active_filter = Some(ActiveFilter::Text("fix".into()));
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
        state.filter.active_filter = Some(ActiveFilter::Label("bug".into()));
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
        state.filter.active_filter = Some(ActiveFilter::Assignee("alice".into()));
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
        state.filter.active_filter = Some(ActiveFilter::Text("fix".into()));
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
        state.filter.active_filter = Some(ActiveFilter::Text("fix".into()));
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
        state.filter.active_filter = Some(ActiveFilter::Text("A".into()));

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
        state.filter.active_filter = Some(ActiveFilter::Text("fix".into()));
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
}
