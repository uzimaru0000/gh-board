use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::action::Action;
use crate::command::Command;
use crate::config::{LayoutModeConfig, ViewConfig};
use crate::event::AppEvent;
use crate::keymap::{Keymap, KeymapMode};
use crate::command::CustomFieldValueInput;
use crate::model::board_cache::BoardCache;
use crate::model::project::{
    Board, Card, CardType, CustomFieldValue, FieldDefinition, ProjectSummary,
};

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn is_valid_iso_date(s: &str) -> bool {
    // YYYY-MM-DD 形式のみ許容 (簡易バリデーション)
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let all_digits = |slice: &[u8]| slice.iter().all(|b| b.is_ascii_digit());
    if !all_digits(&bytes[0..4]) || !all_digits(&bytes[5..7]) || !all_digits(&bytes[8..10]) {
        return false;
    }
    let month: u32 = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}
use crate::model::state::{
    ActiveFilter, CommentListState, ConfirmAction, ConfirmState, CreateCardField,
    CreateCardState, DetailPane, EditCardField, EditCardState, EditItem, FilterState, GrabState,
    GroupBySelectState, LayoutMode, LoadingState, NewCardType, PendingIssueCreate,
    ReactionPickerState, ReactionTarget, RepoSelectState, SidebarEditMode, SidebarSection,
    ViewMode,
};
#[cfg(test)]
use crate::model::state::{SIDEBAR_ASSIGNEES, SIDEBAR_LABELS};

pub struct AppState {
    pub mode: ViewMode,
    pub should_quit: bool,

    // Board state
    pub board: Option<Board>,
    pub selected_column: usize,
    pub selected_card: usize,
    pub scroll_offset: usize,
    pub board_scroll_x: std::cell::Cell<usize>,

    // Layout (Board / Table / Roadmap)
    pub current_layout: LayoutMode,
    pub table_selected_row: usize,
    /// Table view 用の表示順。Board.columns を平坦化したものをデフォルトとし、
    /// Table での grab 並び替えはこのリストの順序を入れ替える (status は変えない)。
    /// 含まれない item_id は表示時にリスト末尾に付加される。
    pub table_item_order: Vec<String>,
    /// Roadmap view 用の選択行。`roadmap_rows()` のインデックス。
    pub roadmap_selected_row: usize,

    // Project selection
    pub projects: Vec<ProjectSummary>,
    pub selected_project_index: usize,
    pub current_project: Option<ProjectSummary>,
    pub project_filter_query: String,
    pub project_filter_cursor: usize,
    pub filtered_project_indices: Vec<usize>,

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
    /// Parent / Sub-issue を Enter で開いた時のオーバーレイスタック。
    /// 末尾が現在表示中のカード (current_detail_card)。空ならボードの selected を表示。
    pub detail_stack: Vec<Card>,
    /// FetchIssueDetail 中の判定 (重複発行防止)
    pub detail_loading_id: Option<String>,

    // Edit card
    pub edit_card_state: Option<EditCardState>,

    // Card grab
    pub grab_state: Option<GrabState>,

    // Comment list
    pub comment_list_state: Option<CommentListState>,

    // Group-by selector
    pub group_by_select_state: Option<GroupBySelectState>,

    // Reaction picker
    pub reaction_picker_state: Option<ReactionPickerState>,

    // Archived list
    pub archived_list: Option<crate::model::state::ArchivedListState>,

    // Loading
    pub loading: LoadingState,

    // サーバーサイドフィルタ結果のキャッシュ。フィルタ行き来時の blank 期間を無くす。
    pub board_cache: BoardCache,
    /// 直近に発行した LoadBoard の queries (BoardLoaded 時にキャッシュ保存するため)
    pub pending_board_queries: Option<Vec<String>>,

    // Views (saved filter presets)
    pub views: Vec<ViewConfig>,
    pub active_view: Option<usize>,

    // CLI options
    pub owner: Option<String>,

    // Viewer info
    pub viewer_login: String,

    // 起動時のグルーピング軸初期値 (config.toml [board] group_by)。
    pub preferred_grouping_field_name: Option<String>,

    // Keymap
    pub keymap: Keymap,
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
            current_layout: LayoutMode::Board,
            table_selected_row: 0,
            table_item_order: Vec::new(),
            roadmap_selected_row: 0,
            projects: Vec::new(),
            selected_project_index: 0,
            current_project: None,
            project_filter_query: String::new(),
            project_filter_cursor: 0,
            filtered_project_indices: Vec::new(),
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
            detail_stack: Vec::new(),
            detail_loading_id: None,
            edit_card_state: None,
            grab_state: None,
            comment_list_state: None,
            group_by_select_state: None,
            reaction_picker_state: None,
            archived_list: None,
            loading: LoadingState::Idle,
            board_cache: BoardCache::new(8),
            pending_board_queries: None,
            views: Vec::new(),
            active_view: None,
            owner,
            viewer_login: String::new(),
            preferred_grouping_field_name: None,
            keymap: Keymap::default_keymap(),
        }
    }

    pub fn set_views(&mut self, views: Vec<ViewConfig>) {
        self.views = views;
    }

    /// Board / Table / Roadmap の表示レイアウトを cycle でトグルする。
    /// Board → Table → Roadmap → Board の順で循環。
    /// Iteration field が無い project では Roadmap を skip する。
    /// 切替時に選択状態を双方向に同期し、ユーザーが見ていたカードを保つ。
    pub fn toggle_layout(&mut self) {
        let has_iter = self
            .board
            .as_ref()
            .is_some_and(|b| b.has_iteration_field());
        match self.current_layout {
            LayoutMode::Board => {
                self.table_selected_row = self.current_table_row();
                self.current_layout = LayoutMode::Table;
            }
            LayoutMode::Table => {
                if has_iter {
                    self.roadmap_selected_row = self.table_selected_row;
                    self.current_layout = LayoutMode::Roadmap;
                } else {
                    self.set_selection_from_table_row(self.table_selected_row);
                    self.current_layout = LayoutMode::Board;
                }
            }
            LayoutMode::Roadmap => {
                self.set_selection_from_roadmap_row(self.roadmap_selected_row);
                self.current_layout = LayoutMode::Board;
            }
        }
    }

    pub fn set_keymap(&mut self, keymap: Keymap) {
        self.keymap = keymap;
    }

    fn resolve_at_me(&self, input: &str) -> String {
        if self.viewer_login.is_empty() {
            return input.to_string();
        }
        input.replace("@me", &format!("@{}", self.viewer_login))
    }

    fn switch_to_view(&mut self, idx: usize) -> Command {
        if idx >= self.views.len() {
            return Command::None;
        }
        self.active_view = Some(idx);
        let filter_str = self.resolve_at_me(&self.views[idx].filter.clone());
        self.filter.input = filter_str.clone();
        self.filter.cursor_pos = filter_str.len();
        if filter_str.is_empty() {
            self.filter.active_filter = None;
        } else {
            self.filter.active_filter = Some(ActiveFilter::parse(&filter_str));
        }
        self.selected_card = 0;
        self.scroll_offset = 0;
        self.current_layout = match self.views[idx].layout {
            Some(LayoutModeConfig::Table) => LayoutMode::Table,
            Some(LayoutModeConfig::Roadmap) => LayoutMode::Roadmap,
            _ => LayoutMode::Board,
        };
        self.table_selected_row = 0;
        self.roadmap_selected_row = 0;
        if let Some(project) = &self.current_project {
            let id = project.id.clone();
            self.start_loading_board(&id)
        } else {
            Command::None
        }
    }

    fn clear_view(&mut self) -> Command {
        self.active_view = None;
        self.filter.active_filter = None;
        self.filter.input.clear();
        self.filter.cursor_pos = 0;
        self.selected_card = 0;
        self.scroll_offset = 0;
        self.current_layout = LayoutMode::Board;
        self.table_selected_row = 0;
        self.roadmap_selected_row = 0;
        if let Some(project) = &self.current_project {
            let id = project.id.clone();
            self.start_loading_board(&id)
        } else {
            Command::None
        }
    }

    pub fn start_loading_projects(&mut self) -> Command {
        self.loading = LoadingState::Loading("Loading projects...".into());
        Command::LoadProjects {
            owner: self.owner.clone(),
        }
    }

    /// ProjectSelect モードに遷移する。projects 未ロードなら取得を開始する。
    pub fn enter_project_select(&mut self) -> Command {
        self.mode = ViewMode::ProjectSelect;
        if self.projects.is_empty() {
            self.start_loading_projects()
        } else {
            Command::None
        }
    }

    pub fn start_loading_project_by_number(
        &mut self,
        owner: Option<String>,
        number: i32,
    ) -> Command {
        self.mode = ViewMode::Board;
        self.loading = LoadingState::Loading("Loading project...".into());
        Command::LoadProjectByNumber { owner, number }
    }

    pub fn start_loading_board(&mut self, project_id: &str) -> Command {
        let queries = self
            .filter
            .active_filter
            .as_ref()
            .map(|f| f.to_server_queries())
            .unwrap_or_default();

        // キャッシュヒット時は board を即時差し替えて blank 期間を無くす
        if let Some(cached) = self.board_cache.get(&queries) {
            self.board = Some(cached);
            self.selected_column = 0;
            self.selected_card = 0;
            self.scroll_offset = 0;
            self.board_scroll_x.set(0);
            self.rebuild_table_order();
        }

        self.loading = if self.board.is_some() {
            LoadingState::Refreshing
        } else {
            LoadingState::Loading("Loading board...".into())
        };
        self.pending_board_queries = Some(queries.clone());
        Command::LoadBoard {
            project_id: project_id.to_string(),
            preferred_grouping_field_name: self.preferred_grouping_field_name.clone(),
            queries,
        }
    }

    /// mutation 後に呼び、server と乖離した可能性のあるキャッシュを全破棄する。
    fn invalidate_board_cache(&mut self) {
        self.board_cache.clear();
    }

    /// Command の中に board を書き換える mutation が含まれているか判定する。
    fn command_mutates_board(cmd: &Command) -> bool {
        match cmd {
            Command::MoveCard { .. }
            | Command::ArchiveCard { .. }
            | Command::UnarchiveCard { .. }
            | Command::CreateCard { .. }
            | Command::CreateIssue { .. }
            | Command::ReorderCard { .. }
            | Command::ToggleLabel { .. }
            | Command::ToggleAssignee { .. }
            | Command::UpdateCard { .. } => true,
            Command::Batch(cmds) => cmds.iter().any(Self::command_mutates_board),
            _ => false,
        }
    }

    pub fn handle_event(&mut self, event: AppEvent) -> Command {
        let cmd = self.handle_event_inner(event);
        if Self::command_mutates_board(&cmd) {
            self.invalidate_board_cache();
        }
        cmd
    }

    fn handle_event_inner(&mut self, event: AppEvent) -> Command {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::ProjectsLoaded(Ok(projects)) => {
                self.projects = projects;
                self.loading = LoadingState::Idle;
                self.recompute_filtered_projects();
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
                // 対応する queries でキャッシュに保存 (次回以降の即時表示に使う)
                if let Some(queries) = self.pending_board_queries.take() {
                    self.board_cache.put(queries, board.clone());
                }
                self.board = Some(board);
                self.selected_column = 0;
                self.selected_card = 0;
                self.scroll_offset = 0;
                self.board_scroll_x.set(0);
                self.rebuild_table_order();
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
                // エラーは画面に残す (自動リロードしない)。
                // 楽観的にローカル状態は変わっているが、次の `r` で同期できる。
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::CardArchived(Ok(())) => {
                // 楽観的更新済み
                Command::None
            }
            AppEvent::CardArchived(Err(e)) => {
                self.loading = LoadingState::Error(e);
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CardUnarchived(Ok(_item_id)) => {
                // archived リストからは楽観的に除去済み。ボードをリフレッシュして復元カードを反映。
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CardUnarchived(Err(e)) => {
                self.loading = LoadingState::Error(e.clone());
                if let Some(state) = self.archived_list.as_mut() {
                    state.error = Some(e);
                }
                Command::None
            }
            AppEvent::ArchivedItemsLoaded(Ok(cards)) => {
                if let Some(state) = self.archived_list.as_mut() {
                    state.cards = cards;
                    state.loading = false;
                    state.error = None;
                    if state.selected >= state.cards.len() {
                        state.selected = state.cards.len().saturating_sub(1);
                    }
                }
                Command::None
            }
            AppEvent::ArchivedItemsLoaded(Err(e)) => {
                if let Some(state) = self.archived_list.as_mut() {
                    state.loading = false;
                    state.error = Some(e);
                }
                Command::None
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
                if let Some(card) = self.selected_card_mut()
                    && let Some(c) = card.comments.iter_mut().find(|c| c.id == comment.id) {
                        c.body = comment.body;
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
            AppEvent::SubIssuesLoaded(Ok((item_id, sub_issues))) => {
                // detail_stack のトップにも反映 (FetchIssueDetail で開いた Issue が item_id を共有)
                if let Some(top) = self.detail_stack.last_mut()
                    && top.item_id == item_id
                {
                    top.sub_issues = sub_issues.clone();
                }
                if let Some(board) = &mut self.board {
                    for col in &mut board.columns {
                        for card in &mut col.cards {
                            if card.item_id == item_id {
                                card.sub_issues = sub_issues;
                                return Command::None;
                            }
                        }
                    }
                }
                Command::None
            }
            AppEvent::SubIssuesLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::IssueDetailLoaded(Ok(card)) => {
                self.detail_loading_id = None;
                let card = *card;
                let needs_subs = matches!(card.card_type, CardType::Issue { .. })
                    && card
                        .sub_issues_summary
                        .as_ref()
                        .is_some_and(|s| s.total > 0);
                let item_id = card.item_id.clone();
                let content_id = card.content_id.clone();
                self.push_detail_stack(card);
                self.mode = ViewMode::Detail;
                if needs_subs
                    && let Some(cid) = content_id
                {
                    return Command::FetchSubIssues {
                        item_id,
                        content_id: cid,
                    };
                }
                Command::None
            }
            AppEvent::IssueDetailLoaded(Err(e)) => {
                self.detail_loading_id = None;
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::ReactionToggled(Ok(())) => {
                // 楽観的更新済み
                Command::None
            }
            AppEvent::ReactionToggled(Err(e)) => {
                self.loading = LoadingState::Error(e);
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            AppEvent::CustomFieldUpdated(Ok(())) => {
                // 楽観的更新済み
                Command::None
            }
            AppEvent::CustomFieldUpdated(Err(e)) => {
                // エラーは画面に残す (自動リロードしない)。
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
            self.project_filter_query.clear();
            self.project_filter_cursor = 0;
            self.recompute_filtered_projects();
            // モーダルを閉じ、Board 画面を Loading 表示にする。
            // 別プロジェクトの board/キャッシュは持ち越さない。
            self.mode = ViewMode::Board;
            self.board = None;
            self.invalidate_board_cache();
            self.start_loading_board(&project.id)
        } else {
            Command::None
        }
    }

    pub fn real_project_index(&self) -> Option<usize> {
        self.filtered_project_indices
            .get(self.selected_project_index)
            .copied()
    }

    pub fn recompute_filtered_projects(&mut self) {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;

        if self.project_filter_query.is_empty() {
            self.filtered_project_indices = (0..self.projects.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let pattern = &self.project_filter_query;
            let mut scored: Vec<(i64, usize, usize)> = self
                .projects
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let haystack = match &p.description {
                        Some(d) if !d.is_empty() => format!("{} {}", p.title, d),
                        _ => p.title.clone(),
                    };
                    matcher
                        .fuzzy_match(&haystack, pattern)
                        .map(|score| (score, i, i))
                })
                .collect();
            // スコア降順、同点は元順 (tie-breaker に元 index の昇順)
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
            self.filtered_project_indices = scored.into_iter().map(|(_, i, _)| i).collect();
        }
        self.selected_project_index = 0;
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

        // Ignore keys while loading (Refreshing はバックグラウンド更新なので操作可能)
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
            ViewMode::Filter => self.handle_filter_key(key),
            ViewMode::Confirm => self.handle_confirm_key(key),
            ViewMode::CreateCard => self.handle_create_card_key(key),
            ViewMode::Detail => self.handle_detail_key(key),
            ViewMode::RepoSelect => self.handle_repo_select_key(key),
            ViewMode::CardGrab => self.handle_card_grab_key(key),
            ViewMode::EditCard => self.handle_edit_card_key(key),
            ViewMode::CommentList => self.handle_comment_list_key(key),
            ViewMode::GroupBySelect => self.handle_group_by_select_key(key),
            ViewMode::ReactionPicker => self.handle_reaction_picker_key(key),
            ViewMode::ArchivedList => self.handle_archived_list_key(key),
        }
    }

    fn handle_board_key(&mut self, key: KeyEvent) -> Command {
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

        // View switching (1-9, 0) は特殊処理のまま
        if let KeyCode::Char(c @ '1'..='9') = key.code
            && key.modifiers == KeyModifiers::NONE {
                return self.switch_to_view((c as usize) - ('1' as usize));
            }
        if key.code == KeyCode::Char('0') && key.modifiers == KeyModifiers::NONE {
            return self.clear_view();
        }

        let action = match self.keymap.resolve(KeymapMode::Board, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let current_col_len = self.filtered_card_indices(self.selected_column).len();
        let col_count = self.board.as_ref().map(|b| b.columns.len()).unwrap_or(0);

        match action {
            Action::Quit | Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
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
            Action::SwitchProject => self.enter_project_select(),
            Action::Refresh => {
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            Action::ShowHelp => {
                self.mode = ViewMode::Help;
                Command::None
            }
            Action::ChangeGrouping => {
                self.open_group_by_select();
                Command::None
            }
            Action::ToggleLayout => {
                self.toggle_layout();
                Command::None
            }
            Action::StartFilter => {
                self.filter.input.clear();
                self.filter.cursor_pos = 0;
                self.mode = ViewMode::Filter;
                Command::None
            }
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
            Action::ShowArchivedList => self.show_archived_list(),
            Action::NewCard => {
                self.create_card_state = CreateCardState::default();
                self.mode = ViewMode::CreateCard;
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
    fn handle_table_key(&mut self, key: KeyEvent) -> Command {
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

        // View switching (1-9, 0) は Board と同じ
        if let KeyCode::Char(c @ '1'..='9') = key.code
            && key.modifiers == KeyModifiers::NONE
        {
            return self.switch_to_view((c as usize) - ('1' as usize));
        }
        if key.code == KeyCode::Char('0') && key.modifiers == KeyModifiers::NONE {
            return self.clear_view();
        }

        let action = match self.keymap.resolve(KeymapMode::Table, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let row_count = self.table_rows().len();

        match action {
            Action::Quit | Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
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
            Action::SwitchProject => self.enter_project_select(),
            Action::Refresh => {
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            Action::ShowHelp => {
                self.mode = ViewMode::Help;
                Command::None
            }
            Action::ChangeGrouping => {
                self.open_group_by_select();
                Command::None
            }
            Action::ToggleLayout => {
                self.toggle_layout();
                Command::None
            }
            Action::StartFilter => {
                self.filter.input.clear();
                self.filter.cursor_pos = 0;
                self.mode = ViewMode::Filter;
                Command::None
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
            Action::ShowArchivedList => self.show_archived_list(),
            Action::NewCard => {
                self.create_card_state = CreateCardState::default();
                self.mode = ViewMode::CreateCard;
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
    fn handle_roadmap_key(&mut self, key: KeyEvent) -> Command {
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

        // View switching (1-9, 0) は Board と同じ
        if let KeyCode::Char(c @ '1'..='9') = key.code
            && key.modifiers == KeyModifiers::NONE
        {
            return self.switch_to_view((c as usize) - ('1' as usize));
        }
        if key.code == KeyCode::Char('0') && key.modifiers == KeyModifiers::NONE {
            return self.clear_view();
        }

        let action = match self.keymap.resolve(KeymapMode::Roadmap, &key) {
            Some(a) => a,
            None => return Command::None,
        };

        let row_count = self.roadmap_rows().len();

        match action {
            Action::Quit | Action::ForceQuit => {
                self.should_quit = true;
                Command::None
            }
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
            Action::SwitchProject => self.enter_project_select(),
            Action::Refresh => {
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    self.start_loading_board(&id)
                } else {
                    Command::None
                }
            }
            Action::ShowHelp => {
                self.mode = ViewMode::Help;
                Command::None
            }
            Action::ChangeGrouping => {
                self.open_group_by_select();
                Command::None
            }
            Action::ToggleLayout => {
                self.toggle_layout();
                Command::None
            }
            Action::StartFilter => {
                self.filter.input.clear();
                self.filter.cursor_pos = 0;
                self.mode = ViewMode::Filter;
                Command::None
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
            Action::ShowArchivedList => self.show_archived_list(),
            Action::NewCard => {
                self.create_card_state = CreateCardState::default();
                self.mode = ViewMode::CreateCard;
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
        let mut card = board.columns[src_col].cards.remove(real_idx);
        let item_id = card.item_id.clone();
        // 楽観的更新: card.custom_fields にも新しい grouping 値を反映させ、
        // 次回の軸切替でも一貫した表示になるようにする。
        if let Some(field_id) = board.grouping.field_id() {
            let field_id = field_id.to_string();
            let target_key = board.columns[target_column].option_id.clone();
            card.custom_fields.retain(|fv| fv.field_id() != field_id);
            match &board.grouping {
                crate::model::project::Grouping::SingleSelect { .. } => {
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
                        option_id: target_key.clone(),
                        name,
                        color,
                    });
                }
                crate::model::project::Grouping::Iteration { .. } => {
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

    fn handle_project_select_key(&mut self, key: KeyEvent) -> Command {
        // 1. 構造キー優先 (Esc/Enter/Down/Up/ForceQuit)
        if let Some(action) = self.keymap.resolve(KeymapMode::ProjectSelect, &key) {
            match action {
                Action::ForceQuit => {
                    self.should_quit = true;
                    return Command::None;
                }
                Action::Quit => {
                    if !self.project_filter_query.is_empty() {
                        self.project_filter_query.clear();
                        self.project_filter_cursor = 0;
                        self.recompute_filtered_projects();
                    } else if self.board.is_some() {
                        self.mode = ViewMode::Board;
                    } else {
                        self.should_quit = true;
                    }
                    return Command::None;
                }
                Action::MoveDown => {
                    if !self.filtered_project_indices.is_empty() {
                        self.selected_project_index = (self.selected_project_index + 1)
                            .min(self.filtered_project_indices.len() - 1);
                    }
                    return Command::None;
                }
                Action::MoveUp => {
                    self.selected_project_index = self.selected_project_index.saturating_sub(1);
                    return Command::None;
                }
                Action::Select => {
                    return match self.real_project_index() {
                        Some(idx) => self.select_project(idx),
                        None => Command::None,
                    };
                }
                _ => {}
            }
        }

        // 2. テキスト入力 (常時入力可)
        match key.code {
            KeyCode::Backspace => {
                if self.project_filter_cursor > 0 {
                    let new_len = self
                        .project_filter_query
                        .char_indices()
                        .take_while(|(i, _)| *i < self.project_filter_cursor)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.project_filter_query.truncate(new_len);
                    self.project_filter_cursor = new_len;
                    self.recompute_filtered_projects();
                }
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.project_filter_query
                    .insert(self.project_filter_cursor, c);
                self.project_filter_cursor += c.len_utf8();
                self.recompute_filtered_projects();
            }
            _ => {}
        }
        Command::None
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Command {
        // Check structural keys first (ForceQuit, Back, Select)
        if let Some(action) = self.keymap.resolve(KeymapMode::FilterStructural, &key) {
            match action {
                Action::ForceQuit => {
                    self.should_quit = true;
                    return Command::None;
                }
                Action::Back => {
                    self.mode = ViewMode::Board;
                    return Command::None;
                }
                Action::Select => {
                    self.active_view = None;
                    let resolved = self.resolve_at_me(&self.filter.input.clone());
                    if resolved.is_empty() {
                        self.filter.active_filter = None;
                    } else {
                        self.filter.active_filter = Some(ActiveFilter::parse(&resolved));
                    }
                    self.selected_card = 0;
                    self.scroll_offset = 0;
                    self.mode = ViewMode::Board;
                    return if let Some(project) = &self.current_project {
                        let id = project.id.clone();
                        self.start_loading_board(&id)
                    } else {
                        Command::None
                    };
                }
                _ => {}
            }
        }

        // Text input handling (not configurable)
        match key.code {
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
        Command::None
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> Command {
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

    fn handle_create_card_key(&mut self, key: KeyEvent) -> Command {
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

    fn start_archive_card(&mut self, return_to: ViewMode) {
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

    fn archive_card(&mut self, item_id: &str) -> Command {
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

    fn show_archived_list(&mut self) -> Command {
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

    fn handle_archived_list_key(&mut self, key: KeyEvent) -> Command {
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

    fn unarchive_selected_in_list(&mut self) -> Command {
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

    fn handle_card_grab_key(&mut self, key: KeyEvent) -> Command {
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
    fn grab_table_move_vertical(&mut self, direction: i32) {
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
                    crate::model::project::Grouping::SingleSelect { .. } => {
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
                            option_id: target_key.clone(),
                            name,
                            color,
                        });
                    }
                    crate::model::project::Grouping::Iteration { .. } => {
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

    fn handle_repo_select_key(&mut self, key: KeyEvent) -> Command {
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
            initial_status: rs.pending_create.initial_status,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        if let Some(Action::Back) = self.keymap.resolve(KeymapMode::Help, &key) {
            self.mode = ViewMode::Board;
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

    fn find_card_position(&self, item_id: &str) -> Option<(usize, usize)> {
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

    fn clamp_card_selection(&mut self) {
        let filtered_len = self.filtered_card_indices(self.selected_column).len();
        if filtered_len == 0 {
            self.selected_card = 0;
        } else {
            self.selected_card = self.selected_card.min(filtered_len - 1);
        }
        self.scroll_offset = 0;
    }

    pub fn should_show_scrollbar(total: usize, viewport: usize) -> bool {
        viewport > 0 && total > viewport
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
        // sub-issue サマリーを持つ Issue の場合、子 Issue 一覧を取得
        let mut commands: Vec<Command> = Vec::new();
        if let Some(card) = self.selected_card_ref() {
            let content_id = card.content_id.clone();
            if card.comments.len() >= 20
                && let Some(cid) = content_id.clone()
            {
                commands.push(Command::FetchComments { content_id: cid });
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

    fn handle_detail_key(&mut self, key: KeyEvent) -> Command {
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

    fn handle_detail_content_action(&mut self, action: Action) -> Command {
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
                    KeyCode::Backspace => {
                        if state.title_cursor > 0 {
                            let prev = prev_char_pos(&state.title_input, state.title_cursor);
                            state.title_input.drain(prev..state.title_cursor);
                            state.title_cursor = prev;
                        }
                    }
                    KeyCode::Left => {
                        if state.title_cursor > 0 {
                            state.title_cursor =
                                prev_char_pos(&state.title_input, state.title_cursor);
                        }
                    }
                    KeyCode::Right => {
                        if state.title_cursor < state.title_input.len() {
                            state.title_cursor =
                                next_char_pos(&state.title_input, state.title_cursor);
                        }
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


    fn field_definitions(&self) -> &[FieldDefinition] {
        self.board
            .as_ref()
            .map(|b| b.field_definitions.as_slice())
            .unwrap_or(&[])
    }

    fn handle_detail_sidebar_action(&mut self, action: Action) -> Command {
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

    fn open_parent_detail(&mut self) -> Command {
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

    fn open_sub_issue_detail(&mut self, idx: usize) -> Command {
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

    fn open_custom_field_edit(&mut self, field_idx: usize) {
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

    fn handle_comment_list_key(&mut self, key: KeyEvent) -> Command {
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
                self.comment_list_state = None;
                self.mode = ViewMode::Detail;
                Command::None
            }
            Action::MoveDown => {
                if let Some(ref mut cls) = self.comment_list_state
                    && comment_count > 0 {
                        cls.cursor = (cls.cursor + 1).min(comment_count - 1);
                    }
                Command::None
            }
            Action::MoveUp => {
                if let Some(ref mut cls) = self.comment_list_state {
                    cls.cursor = cls.cursor.saturating_sub(1);
                }
                Command::None
            }
            Action::EditComment => {
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
            Action::NewComment => {
                let content_id = match &self.comment_list_state {
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

    /// Board モードから `G` 押下で呼ばれる。
    /// 利用可能な groupable field (SingleSelect + Iteration) をリスト化し GroupBySelect モードへ移行。
    fn open_group_by_select(&mut self) {
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

    fn handle_group_by_select_key(&mut self, key: KeyEvent) -> Command {
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

    fn open_reaction_picker_for_card(&mut self) -> Command {
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

    fn open_reaction_picker_for_comment(&mut self) -> Command {
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

    fn handle_reaction_picker_key(&mut self, key: KeyEvent) -> Command {
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
    fn toggle_selected_reaction(&mut self) -> Command {
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

    fn find_card_by_content_id_mut(&mut self, content_id: &str) -> Option<&mut Card> {
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

    fn handle_status_select_key(&mut self, key: KeyEvent) -> Command {
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

    fn commit_custom_field_selection(&mut self) -> Command {
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

    fn apply_custom_field_optimistic(
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

    fn handle_custom_field_text_key(&mut self, key: KeyEvent) -> Command {
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
            KeyCode::Backspace => {
                if *cursor_pos > 0 {
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

    fn commit_custom_field_text(&mut self) -> Command {
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
                field_id, input, ..
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
    use crate::command::CustomFieldValueInput;
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

    fn make_card_with_custom_fields(
        item_id: &str,
        title: &str,
        custom_fields: Vec<CustomFieldValue>,
    ) -> Card {
        Card {
            item_id: item_id.into(),
            content_id: Some(format!("content_{item_id}")),
            title: title.into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
            custom_fields,
            pr_status: None,
            linked_prs: vec![],
            reactions: vec![],
            archived: false,
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: vec![],
        }
    }

    fn make_board(columns: Vec<(&str, &str, Vec<Card>)>) -> Board {
        Board {
            project_title: "Test Project".into(),
            grouping: crate::model::project::Grouping::SingleSelect {
                field_id: "field_1".into(),
                field_name: "Status".into(),
            },
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
            field_definitions: vec![],
        }
    }

    fn make_board_with_fields(
        columns: Vec<(&str, &str, Vec<Card>)>,
        field_definitions: Vec<FieldDefinition>,
    ) -> Board {
        let mut board = make_board(columns);
        board.field_definitions = field_definitions;
        board
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
        state.rebuild_table_order();
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

    // ========== カードアーカイブ ==========

    #[test]
    fn test_archive_card() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B")],
        )]);
        let mut state = make_state_with_board(board);

        // a で確認ダイアログ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        assert_eq!(state.mode, ViewMode::Confirm);
        assert!(state.confirm_state.is_some());

        // y でアーカイブ実行
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('y'))));
        assert_eq!(
            cmd,
            Command::ArchiveCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
            }
        );
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 1);
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_archive_card_clamp() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B")],
        )]);
        let mut state = make_state_with_board(board);
        state.selected_card = 1; // 最後のカード

        // a → y
        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('y'))));

        // 削除後にクランプされる
        assert_eq!(state.selected_card, 0);
    }

    #[test]
    fn test_start_archive_sets_confirm() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "My Card")],
        )]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));

        assert_eq!(state.mode, ViewMode::Confirm);
        let confirm = state.confirm_state.as_ref().unwrap();
        assert_eq!(confirm.title, "My Card");
        match &confirm.action {
            ConfirmAction::ArchiveCard { item_id } => assert_eq!(item_id, "1"),
        }
    }

    #[test]
    fn test_archive_confirm_cancel_no_command() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card")],
        )]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('n'))));

        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 1);
    }

    // ========== ArchivedList ビュー ==========

    #[test]
    fn test_show_archived_list_starts_loading() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('v'))));

        assert_eq!(state.mode, ViewMode::ArchivedList);
        assert!(state.archived_list.as_ref().unwrap().loading);
        assert_eq!(
            cmd,
            Command::LoadArchivedItems {
                project_id: "proj_1".into()
            }
        );
    }

    #[test]
    fn test_archived_items_loaded_populates_state() {
        let board = make_board(vec![("Todo", "opt_1", vec![])]);
        let mut state = make_state_with_board(board);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('v'))));

        let archived = vec![make_card("a1", "Archived A"), make_card("a2", "Archived B")];
        state.handle_event(AppEvent::ArchivedItemsLoaded(Ok(archived)));

        let s = state.archived_list.as_ref().unwrap();
        assert!(!s.loading);
        assert_eq!(s.cards.len(), 2);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn test_archived_list_navigation_and_unarchive() {
        let board = make_board(vec![("Todo", "opt_1", vec![])]);
        let mut state = make_state_with_board(board);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('v'))));
        state.handle_event(AppEvent::ArchivedItemsLoaded(Ok(vec![
            make_card("a1", "A"),
            make_card("a2", "B"),
        ])));

        // j で次のカード
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.archived_list.as_ref().unwrap().selected, 1);

        // u で UnarchiveCard コマンド + リストから除去
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('u'))));
        assert_eq!(
            cmd,
            Command::UnarchiveCard {
                project_id: "proj_1".into(),
                item_id: "a2".into()
            }
        );
        assert_eq!(state.archived_list.as_ref().unwrap().cards.len(), 1);
    }

    #[test]
    fn test_archived_list_back_returns_to_board() {
        let board = make_board(vec![("Todo", "opt_1", vec![])]);
        let mut state = make_state_with_board(board);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('v'))));

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Board);
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
        state.create_card_state.focused_field = CreateCardField::Submit;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(
            cmd,
            Command::CreateCard {
                project_id: "proj_1".into(),
                title: "New Card".into(),
                body: "Description".into(),
                initial_status: Some(crate::command::InitialStatus {
                    field_id: "field_1".into(),
                    option_id: "opt_1".into(),
                }),
            }
        );
        assert_eq!(state.mode, ViewMode::Board);
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

    fn projects_for_filter() -> Vec<ProjectSummary> {
        vec![
            ProjectSummary {
                id: "p1".into(),
                title: "Alpha Board".into(),
                number: 1,
                description: Some("team alpha".into()),
            },
            ProjectSummary {
                id: "p2".into(),
                title: "Beta Roadmap".into(),
                number: 2,
                description: None,
            },
            ProjectSummary {
                id: "p3".into(),
                title: "Kanban Proto".into(),
                number: 3,
                description: Some("alpha experiment".into()),
            },
        ]
    }

    #[test]
    fn test_project_filter_initial_state() {
        let state = AppState::new(None);
        assert!(state.project_filter_query.is_empty());
        assert_eq!(state.project_filter_cursor, 0);
        assert!(state.filtered_project_indices.is_empty());
    }

    #[test]
    fn test_recompute_after_projects_loaded() {
        let mut state = AppState::new(None);
        state.handle_event(AppEvent::ProjectsLoaded(Ok(projects_for_filter())));
        assert_eq!(state.filtered_project_indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_filter_query_char_input() {
        let mut state = AppState::new(None);
        state.handle_event(AppEvent::ProjectsLoaded(Ok(projects_for_filter())));
        state.mode = ViewMode::ProjectSelect;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('p'))));

        assert_eq!(state.project_filter_query, "alp");
        // "alp" は Alpha Board (title) と Kanban Proto (description "alpha") にマッチ
        assert!(state.filtered_project_indices.contains(&0));
        assert!(state.filtered_project_indices.contains(&2));
        assert!(!state.filtered_project_indices.contains(&1));
        // Alpha Board の方がスコアが高い (title 完全連続一致) 想定
        assert_eq!(state.filtered_project_indices[0], 0);
        assert_eq!(state.selected_project_index, 0);
    }

    #[test]
    fn test_filter_backspace_removes_char() {
        let mut state = AppState::new(None);
        state.handle_event(AppEvent::ProjectsLoaded(Ok(projects_for_filter())));
        state.mode = ViewMode::ProjectSelect;

        for c in ['a', 'b', 'c'] {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        assert_eq!(state.project_filter_query, "abc");

        state.handle_event(AppEvent::Key(key(KeyCode::Backspace)));
        assert_eq!(state.project_filter_query, "ab");
        assert_eq!(state.project_filter_cursor, 2);
    }

    #[test]
    fn test_esc_with_query_clears_only() {
        let mut state = AppState::new(None);
        state.handle_event(AppEvent::ProjectsLoaded(Ok(projects_for_filter())));
        state.mode = ViewMode::ProjectSelect;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        assert_eq!(state.project_filter_query, "a");

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert!(state.project_filter_query.is_empty());
        assert_eq!(state.mode, ViewMode::ProjectSelect);
        assert!(!state.should_quit);
        assert_eq!(state.filtered_project_indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_esc_with_empty_query_returns_to_board() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::ProjectSelect;

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));

        assert_eq!(state.mode, ViewMode::Board);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_esc_with_empty_query_no_board_quits() {
        let mut state = AppState::new(None);
        state.mode = ViewMode::ProjectSelect;
        // board is None, query is empty

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));

        assert!(state.should_quit);
    }

    #[test]
    fn test_arrow_keys_clamp_within_filtered() {
        let mut state = AppState::new(None);
        state.handle_event(AppEvent::ProjectsLoaded(Ok(projects_for_filter())));
        state.mode = ViewMode::ProjectSelect;

        // "alp" で 2 件にヒット
        for c in ['a', 'l', 'p'] {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        assert_eq!(state.filtered_project_indices.len(), 2);
        assert_eq!(state.selected_project_index, 0);

        state.handle_event(AppEvent::Key(key(KeyCode::Down)));
        assert_eq!(state.selected_project_index, 1);
        state.handle_event(AppEvent::Key(key(KeyCode::Down)));
        assert_eq!(state.selected_project_index, 1);
        state.handle_event(AppEvent::Key(key(KeyCode::Up)));
        assert_eq!(state.selected_project_index, 0);
    }

    #[test]
    fn test_enter_selects_real_project() {
        let mut state = AppState::new(None);
        state.handle_event(AppEvent::ProjectsLoaded(Ok(projects_for_filter())));
        state.mode = ViewMode::ProjectSelect;

        // "kan" で Kanban Proto のみに絞られる (id = p3)
        for c in ['k', 'a', 'n'] {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        assert_eq!(state.filtered_project_indices, vec![2]);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            state.current_project.as_ref().map(|p| p.id.as_str()),
            Some("p3")
        );
        assert!(matches!(cmd, Command::LoadBoard { ref project_id, .. } if project_id == "p3"));
        // select_project 後はフィルタがリセットされる
        assert!(state.project_filter_query.is_empty());
        assert_eq!(state.filtered_project_indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_select_project_closes_modal_and_enters_loading() {
        // 既に別プロジェクトの board が表示されている状態でプロジェクト切替
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::ProjectSelect;
        state.projects = projects_for_filter();
        state.recompute_filtered_projects();

        let cmd = state.select_project(1);

        // モーダルを閉じて Board 画面を Loading に
        assert_eq!(state.mode, ViewMode::Board);
        assert!(state.board.is_none());
        assert!(matches!(state.loading, LoadingState::Loading(_)));
        assert!(matches!(cmd, Command::LoadBoard { .. }));
    }

    #[test]
    fn test_start_loading_project_by_number_switches_to_board_mode() {
        // `gh board <number>` 指定時は ProjectSelect 画面を表示せずにロード
        let mut state = AppState::new(None);
        assert_eq!(state.mode, ViewMode::ProjectSelect);

        let cmd = state.start_loading_project_by_number(Some("octocat".into()), 42);

        assert_eq!(state.mode, ViewMode::Board);
        assert!(matches!(state.loading, LoadingState::Loading(_)));
        assert_eq!(
            cmd,
            Command::LoadProjectByNumber {
                owner: Some("octocat".into()),
                number: 42,
            }
        );
    }

    #[test]
    fn test_switch_project_loads_when_projects_empty() {
        // `gh board <number>` 指定で projects が未ロードのまま Board に居る状態
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.owner = Some("octocat".into());
        assert!(state.projects.is_empty());

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('p'))));

        assert_eq!(state.mode, ViewMode::ProjectSelect);
        assert_eq!(
            cmd,
            Command::LoadProjects {
                owner: Some("octocat".into()),
            }
        );
    }

    #[test]
    fn test_switch_project_noop_when_projects_loaded() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.projects = projects_for_filter();
        state.recompute_filtered_projects();

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('p'))));

        assert_eq!(state.mode, ViewMode::ProjectSelect);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_enter_with_no_matches_is_noop() {
        let mut state = AppState::new(None);
        state.handle_event(AppEvent::ProjectsLoaded(Ok(projects_for_filter())));
        state.mode = ViewMode::ProjectSelect;

        for c in ['z', 'z', 'z', 'z'] {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        assert!(state.filtered_project_indices.is_empty());

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
        assert!(state.current_project.is_none());
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
                preferred_grouping_field_name: None,
                queries: vec![],
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
    fn test_card_moved_error_shows_error() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::CardMoved(Err("API error".into())));

        // エラーは画面に残し、自動リロードはしない (ユーザーが `r` で手動リフレッシュ)
        assert!(matches!(state.loading, LoadingState::Error(ref m) if m == "API error"));
        assert_eq!(cmd, Command::None);
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
    fn test_filter_enter_emits_load_board_with_queries() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A"), make_card("2", "Card B")],
        )]);
        let mut state = make_state_with_board(board);

        // / でフィルタモードに入り label:bug を入力
        state.handle_event(AppEvent::Key(key(KeyCode::Char('/'))));
        for c in "label:bug".chars() {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }

        // Enter で確定: LoadBoard が server-side query 付きで返ること
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec!["label:\"bug\"".into()],
            }
        );
        assert!(state.filter.active_filter.is_some());
    }

    #[test]
    fn test_filter_enter_or_emits_multiple_queries() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('/'))));
        for c in "label:bug | label:enh".chars() {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec!["label:\"bug\"".into(), "label:\"enh\"".into()],
            }
        );
    }

    #[test]
    fn test_filter_enter_empty_emits_load_board_no_query() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("label:bug"));

        // / で空のまま Enter
        state.handle_event(AppEvent::Key(key(KeyCode::Char('/'))));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec![],
            }
        );
        assert!(state.filter.active_filter.is_none());
    }

    #[test]
    fn test_clear_filter_emits_load_board_no_query() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("label:bug"));

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec![],
            }
        );
        assert!(state.filter.active_filter.is_none());
    }

    #[test]
    fn test_board_loaded_populates_cache() {
        let mut state = make_state_with_board(make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A")],
        )]));
        state.filter.active_filter = Some(ActiveFilter::parse("label:bug"));
        // 先に start_loading_board を呼んで pending_board_queries を仕込む
        let _ = state.start_loading_board("proj_1");

        let new_board = make_board(vec![("Todo", "opt_1", vec![make_card("2", "B")])]);
        state.handle_event(AppEvent::BoardLoaded(Ok(new_board)));

        // cache に queries="label:bug" の board が保存されている
        assert_eq!(
            state.board_cache.keys(),
            vec![vec!["label:\"bug\"".to_string()]]
        );
    }

    #[test]
    fn test_filter_change_uses_cache_immediately() {
        let mut state = make_state_with_board(make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("old", "old board card")],
        )]));

        // あらかじめ label:bug のキャッシュを仕込む
        let cached = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("cached_1", "cached bug")],
        )]);
        state
            .board_cache
            .put(vec!["label:\"bug\"".into()], cached);

        // / label:bug Enter
        state.handle_event(AppEvent::Key(key(KeyCode::Char('/'))));
        for c in "label:bug".chars() {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        // Enter 時点で board は cache の内容に差し替わっている
        let b = state.board.as_ref().unwrap();
        assert_eq!(b.columns[0].cards[0].item_id, "cached_1");
    }

    #[test]
    fn test_mutation_invalidates_cache() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A"), make_card("2", "B")]),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state
            .board_cache
            .put(vec!["label:\"bug\"".into()], make_board(vec![]));
        assert_eq!(state.board_cache.len(), 1);

        // grab モードで Done へ移動 → MoveCard が返り、cache は clear される
        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.board_cache.len(), 0);
    }

    #[test]
    fn test_refresh_with_active_filter_includes_queries() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.filter.active_filter = Some(ActiveFilter::parse("label:bug"));

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec!["label:\"bug\"".into()],
            }
        );
    }

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
        // Submit を reachable にするためタイトルを埋める
        state.create_card_state.title_input = "x".into();

        // デフォルトは Type
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);

        // Tab → Title
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Title);

        // Tab → Body
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Body);

        // Tab → Submit
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Submit);

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
        state.create_card_state.title_input = "x".into();

        // デフォルトは Type
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);

        // S-Tab → Submit (逆方向ラップ)
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Submit);

        // S-Tab → Body
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Body);

        // S-Tab → Title
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Title);

        // S-Tab → Type
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);
    }

    #[test]
    fn test_create_card_tab_skips_submit_when_disabled() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();
        // タイトル空なので Submit は disable
        state.create_card_state.focused_field = CreateCardField::Body;

        // Body から Tab → Submit をスキップして Type へ
        state.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Type);
    }

    #[test]
    fn test_create_card_backtab_skips_submit_when_disabled() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();
        // タイトル空なので Submit は disable
        state.create_card_state.focused_field = CreateCardField::Type;

        // Type から BackTab → 逆方向ラップは Submit だが disable なので Body にスキップ
        state.handle_event(AppEvent::Key(key(KeyCode::BackTab)));
        assert_eq!(state.create_card_state.focused_field, CreateCardField::Body);
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
        state.create_card_state.focused_field = CreateCardField::Submit;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(
            cmd,
            Command::CreateCard {
                project_id: "proj_1".into(),
                title: "My Draft".into(),
                body: "body".into(),
                initial_status: Some(crate::command::InitialStatus {
                    field_id: "field_1".into(),
                    option_id: "opt_1".into(),
                }),
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
        state.create_card_state.focused_field = CreateCardField::Submit;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(
            cmd,
            Command::CreateIssue {
                project_id: "proj_1".into(),
                repository_id: "repo_1".into(),
                title: "My Issue".into(),
                body: "body".into(),
                initial_status: Some(crate::command::InitialStatus {
                    field_id: "field_1".into(),
                    option_id: "opt_1".into(),
                }),
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
        state.create_card_state.focused_field = CreateCardField::Submit;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

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
        state.create_card_state.focused_field = CreateCardField::Submit;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(cmd, Command::None);
        assert!(matches!(state.loading, LoadingState::Error(_)));
    }

    #[test]
    fn test_create_card_submit_button_empty_title_no_op() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();
        state.create_card_state.focused_field = CreateCardField::Submit;
        // title_input は空のまま

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(cmd, Command::None);
        // モーダルを閉じず維持する (disable されたボタンを押したのと同じ)
        assert_eq!(state.mode, ViewMode::CreateCard);
    }

    #[test]
    fn test_create_card_submit_button_whitespace_title_no_op() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();
        state.create_card_state.focused_field = CreateCardField::Submit;
        state.create_card_state.title_input = "   ".into();

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::CreateCard);
    }

    #[test]
    fn test_create_card_can_submit_reflects_title() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state = CreateCardState::default();

        assert!(!state.can_submit_create_card());

        state.create_card_state.title_input = "   ".into();
        assert!(!state.can_submit_create_card());

        state.create_card_state.title_input = "Valid".into();
        assert!(state.can_submit_create_card());
    }

    #[test]
    fn test_create_card_ctrl_s_no_longer_submits() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::CreateCard;
        state.create_card_state.card_type = NewCardType::Draft;
        state.create_card_state.title_input = "My Draft".into();
        state.create_card_state.focused_field = CreateCardField::Type;

        let cmd = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));

        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::CreateCard);
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
                initial_status: Some(crate::command::InitialStatus {
                    field_id: "field_1".into(),
                    option_id: "opt_1".into(),
                }),
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
                initial_status: Some(crate::command::InitialStatus {
                    field_id: "field_1".into(),
                    option_id: "opt_1".into(),
                }),
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
                    value: CustomFieldValueInput::SingleSelect { option_id: "opt_1".into() },
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
                    value: CustomFieldValueInput::SingleSelect { option_id: "opt_2".into() },
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
                    value: CustomFieldValueInput::SingleSelect { option_id: "opt_2".into() },
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

        // エラー後にリロードが発動するので Refreshing 状態になる (既存ボードあり)
        assert!(matches!(state.loading, LoadingState::Refreshing));
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec![],
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
                value: CustomFieldValueInput::SingleSelect { option_id: "opt_3".into() },
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

    // ========== 詳細ビュー: アーカイブ ==========

    #[test]
    fn test_detail_sidebar_archive_button_opens_confirm() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = state.sidebar_archive_index();

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Confirm);
        assert!(state.confirm_state.is_some());
        let cs = state.confirm_state.as_ref().unwrap();
        assert!(matches!(cs.action, ConfirmAction::ArchiveCard { .. }));
        assert_eq!(cs.return_to, ViewMode::Detail);
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_sidebar_a_key_archives() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state.sidebar_selected = 0; // Status (a はどのセクションでも動く)

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        assert_eq!(state.mode, ViewMode::Confirm);
        assert!(state.confirm_state.is_some());
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_archive_confirm_yes_returns_to_board() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;

        // a → Confirm
        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        assert_eq!(state.mode, ViewMode::Confirm);

        // y → アーカイブ実行、Board に戻る
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('y'))));
        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(state.board.as_ref().unwrap().columns[0].cards.len(), 0);
        assert_eq!(
            cmd,
            Command::ArchiveCard {
                project_id: "proj_1".into(),
                item_id: "1".into(),
            }
        );
    }

    #[test]
    fn test_detail_archive_confirm_cancel_returns_to_detail() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;

        // a → Confirm
        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
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
            reactions: vec![],
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
            reactions: vec![],
        };
        let _ = state.handle_event(AppEvent::CommentUpdated(Ok(updated)));

        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.comments[0].body, "updated body");
    }

    #[test]
    fn test_detail_opens_fetch_sub_issues_when_has_summary() {
        use crate::model::project::SubIssuesSummary;
        let mut card = make_issue_card("1", "Parent");
        card.sub_issues_summary = Some(SubIssuesSummary { completed: 1, total: 3 });
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert_eq!(
            cmd,
            Command::FetchSubIssues {
                item_id: "1".into(),
                content_id: "issue_1".into(),
            }
        );
    }

    #[test]
    fn test_detail_no_fetch_sub_issues_when_summary_empty() {
        use crate::model::project::SubIssuesSummary;
        let mut card = make_issue_card("1", "No subs");
        card.sub_issues_summary = Some(SubIssuesSummary { completed: 0, total: 0 });
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_no_fetch_sub_issues_for_draft() {
        let card = make_draft_card("1", "Draft", "body");
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_sidebar_sections_includes_parent_and_subs() {
        use crate::model::project::{IssueState, ParentIssueRef, SubIssueRef, SubIssuesSummary};
        let mut card = make_issue_card("1", "Parent");
        card.parent_issue = Some(ParentIssueRef {
            id: "p1".into(),
            number: 9,
            title: "Parent Issue".into(),
            url: None,
        });
        card.sub_issues_summary = Some(SubIssuesSummary { completed: 0, total: 2 });
        card.sub_issues = vec![
            SubIssueRef {
                id: "s1".into(),
                number: 10,
                title: "A".into(),
                state: IssueState::Open,
                url: None,
            },
            SubIssueRef {
                id: "s2".into(),
                number: 11,
                title: "B".into(),
                state: IssueState::Closed,
                url: None,
            },
        ];
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let state = make_state_with_board(board);

        let sections = state.sidebar_sections();
        // Status, Assignees, Labels, Milestone, Parent, SubIssue(0), SubIssue(1), Archive
        assert_eq!(sections.len(), 8);
        assert!(matches!(sections[4], SidebarSection::Parent));
        assert!(matches!(sections[5], SidebarSection::SubIssue(0)));
        assert!(matches!(sections[6], SidebarSection::SubIssue(1)));
        assert!(matches!(sections[7], SidebarSection::Archive));
    }

    #[test]
    fn test_enter_on_sub_issue_fetches_issue_detail() {
        use crate::model::project::{IssueState, SubIssueRef, SubIssuesSummary};
        let mut card = make_issue_card("1", "Parent");
        card.sub_issues_summary = Some(SubIssuesSummary { completed: 0, total: 1 });
        card.sub_issues = vec![SubIssueRef {
            id: "sub1".into(),
            number: 10,
            title: "Child".into(),
            state: IssueState::Open,
            url: None,
        }];
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        // sections: Status(0) Assignees(1) Labels(2) Milestone(3) Parent?(no)  SubIssue(0)=4 Archive=5
        state.sidebar_selected = 4;

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(
            cmd,
            Command::FetchIssueDetail {
                content_id: "sub1".into(),
            }
        );
        assert_eq!(state.detail_loading_id.as_deref(), Some("sub1"));
    }

    #[test]
    fn test_issue_detail_loaded_pushes_stack() {
        let card = make_issue_card("1", "Parent");
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;

        let mut overlay = make_issue_card("sub1", "Child");
        overlay.item_id = "sub1".into();
        overlay.content_id = Some("sub1".into());
        overlay.number = Some(10);

        let _ = state.handle_event(AppEvent::IssueDetailLoaded(Ok(Box::new(overlay.clone()))));
        assert_eq!(state.detail_stack.len(), 1);
        assert_eq!(state.current_detail_card().unwrap().title, "Child");
        assert!(state.detail_loading_id.is_none());
    }

    #[test]
    fn test_esc_pops_detail_stack_before_closing() {
        let card = make_issue_card("1", "Parent");
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;
        let overlay = make_issue_card("sub1", "Child");
        state.push_detail_stack(overlay);
        assert_eq!(state.detail_stack.len(), 1);

        // 1回目 Esc: stack を pop、Detail のまま
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert_eq!(state.detail_stack.len(), 0);

        // 2回目 Esc: 通常通り Board に戻る
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Board);
    }

    #[test]
    fn test_sub_issues_loaded_updates_card() {
        use crate::model::project::{IssueState, SubIssueRef};
        let card = make_issue_card("1", "Parent");
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let subs = vec![
            SubIssueRef {
                id: "sub1".into(),
                number: 10,
                title: "Child A".into(),
                state: IssueState::Open,
                url: Some("https://github.com/owner/repo/issues/10".into()),
            },
            SubIssueRef {
                id: "sub2".into(),
                number: 11,
                title: "Child B".into(),
                state: IssueState::Closed,
                url: None,
            },
        ];
        let _ = state.handle_event(AppEvent::SubIssuesLoaded(Ok(("1".into(), subs))));

        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.sub_issues.len(), 2);
        assert_eq!(card.sub_issues[0].number, 10);
        assert_eq!(card.sub_issues[1].state, IssueState::Closed);
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

    // ========== リアクションテスト ==========

    fn make_draft_card_with_id(item_id: &str) -> Card {
        let mut c = make_card(item_id, "Draft");
        c.card_type = CardType::DraftIssue;
        c
    }

    #[test]
    fn test_detail_r_opens_reaction_picker_for_card_body() {
        let card = make_issue_card_with_comments("1", "Card A", vec![]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        // Detail ビューを開く
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);

        // r キー → ReactionPicker
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::ReactionPicker);
        let picker = state.reaction_picker_state.as_ref().unwrap();
        assert!(matches!(picker.target, ReactionTarget::CardBody { .. }));
        assert_eq!(picker.return_to, ViewMode::Detail);
        assert_eq!(picker.cursor, 0);
    }

    #[test]
    fn test_detail_r_does_nothing_for_draft_issue() {
        let card = make_draft_card_with_id("1");
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));
        // DraftIssue では Picker に遷移しない
        assert_eq!(state.mode, ViewMode::Detail);
        assert!(state.reaction_picker_state.is_none());
    }

    #[test]
    fn test_comment_list_r_opens_reaction_picker_for_comment() {
        let comments = vec![
            make_comment("c1", "alice", "hi"),
            make_comment("c2", "bob", "hello"),
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

        // j で 2 個目のコメントを選択
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));

        // r キー → ReactionPicker (対象は c2)
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.mode, ViewMode::ReactionPicker);
        let picker = state.reaction_picker_state.as_ref().unwrap();
        match &picker.target {
            ReactionTarget::Comment { comment_id, .. } => {
                assert_eq!(comment_id, "c2");
            }
            _ => panic!("Expected Comment target"),
        }
        assert_eq!(picker.return_to, ViewMode::CommentList);
    }

    #[test]
    fn test_reaction_picker_navigation_wraps() {
        let card = make_issue_card_with_comments("1", "Card A", vec![]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));

        // cursor = 0 → h (MoveLeft) で 7 にラップ
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('h'))));
        assert_eq!(state.reaction_picker_state.as_ref().unwrap().cursor, 7);

        // l (MoveRight) で 0 に戻る
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.reaction_picker_state.as_ref().unwrap().cursor, 0);

        // l で 1
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.reaction_picker_state.as_ref().unwrap().cursor, 1);
    }

    #[test]
    fn test_reaction_picker_enter_adds_reaction_optimistic() {
        let card = make_issue_card_with_comments("1", "Card A", vec![]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));

        // cursor = 0 (ThumbsUp) で Enter → AddReaction + 楽観的更新
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match cmd {
            Command::AddReaction { subject_id, content } => {
                assert_eq!(subject_id, "issue_1");
                assert_eq!(content, ReactionContent::ThumbsUp);
            }
            other => panic!("Expected AddReaction, got {:?}", other),
        }
        // 楽観的更新
        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.reactions.len(), 1);
        assert_eq!(card.reactions[0].content, ReactionContent::ThumbsUp);
        assert_eq!(card.reactions[0].count, 1);
        assert!(card.reactions[0].viewer_has_reacted);
    }

    #[test]
    fn test_reaction_picker_enter_removes_reaction_optimistic() {
        // 既にリアクション済みのカード
        let mut card = make_issue_card_with_comments("1", "Card A", vec![]);
        card.reactions.push(ReactionSummary {
            content: ReactionContent::ThumbsUp,
            count: 3,
            viewer_has_reacted: true,
        });
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));

        // cursor = 0 (ThumbsUp) で Enter → RemoveReaction + 楽観的更新
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match cmd {
            Command::RemoveReaction { subject_id, content } => {
                assert_eq!(subject_id, "issue_1");
                assert_eq!(content, ReactionContent::ThumbsUp);
            }
            other => panic!("Expected RemoveReaction, got {:?}", other),
        }
        // 楽観的更新: count=2, viewer_has_reacted=false
        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.reactions.len(), 1);
        assert_eq!(card.reactions[0].count, 2);
        assert!(!card.reactions[0].viewer_has_reacted);
    }

    #[test]
    fn test_reaction_picker_esc_returns_to_previous_viewmode() {
        let card = make_issue_card_with_comments("1", "Card A", vec![]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));
        assert_eq!(state.mode, ViewMode::ReactionPicker);

        // Esc → Detail に戻る
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert!(state.reaction_picker_state.is_none());
    }

    #[test]
    fn test_reaction_picker_esc_returns_to_comment_list() {
        let comments = vec![make_comment("c1", "alice", "hi")];
        let card = make_issue_card_with_comments("1", "Card A", comments);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        let _ = state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('C'),
            KeyModifiers::SHIFT,
        )));
        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Char('r'))));
        assert_eq!(state.mode, ViewMode::ReactionPicker);

        let _ = state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        // CommentList に戻るべき
        assert_eq!(state.mode, ViewMode::CommentList);
    }

    #[test]
    fn test_reaction_toggled_error_triggers_refresh() {
        let card = make_issue_card_with_comments("1", "Card A", vec![]);
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);
        // current_project を設定して refresh 可能にする
        state.current_project = Some(ProjectSummary {
            id: "pid".into(),
            title: "Proj".into(),
            number: 1,
            description: None,
        });

        let cmd = state.handle_event(AppEvent::ReactionToggled(Err("API err".into())));
        // エラー時はボードリロード (Loading で上書きされる)
        assert!(matches!(cmd, Command::LoadBoard { .. }));
    }

    #[test]
    fn test_apply_reaction_toggle_add_new() {
        let mut reactions: Vec<ReactionSummary> = vec![];
        let now = apply_reaction_toggle(&mut reactions, ReactionContent::Heart);
        assert!(now);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].content, ReactionContent::Heart);
        assert_eq!(reactions[0].count, 1);
        assert!(reactions[0].viewer_has_reacted);
    }

    #[test]
    fn test_apply_reaction_toggle_remove_last() {
        let mut reactions = vec![ReactionSummary {
            content: ReactionContent::Heart,
            count: 1,
            viewer_has_reacted: true,
        }];
        let now = apply_reaction_toggle(&mut reactions, ReactionContent::Heart);
        assert!(!now);
        assert!(reactions.is_empty());
    }

    #[test]
    fn test_apply_reaction_toggle_increment_when_others_reacted() {
        // 他人が 2 reacted しているが自分はまだ
        let mut reactions = vec![ReactionSummary {
            content: ReactionContent::Rocket,
            count: 2,
            viewer_has_reacted: false,
        }];
        let now = apply_reaction_toggle(&mut reactions, ReactionContent::Rocket);
        assert!(now);
        assert_eq!(reactions[0].count, 3);
        assert!(reactions[0].viewer_has_reacted);
    }

    // --- should_show_scrollbar tests ---

    #[test]
    fn test_should_show_scrollbar_fits() {
        assert!(!AppState::should_show_scrollbar(5, 10));
        assert!(!AppState::should_show_scrollbar(10, 10));
    }

    #[test]
    fn test_should_show_scrollbar_overflow() {
        assert!(AppState::should_show_scrollbar(11, 10));
        assert!(AppState::should_show_scrollbar(1000, 3));
    }

    #[test]
    fn test_should_show_scrollbar_zero_viewport() {
        assert!(!AppState::should_show_scrollbar(100, 0));
    }

    #[test]
    fn test_should_show_scrollbar_zero_total() {
        assert!(!AppState::should_show_scrollbar(0, 10));
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
                preferred_grouping_field_name: None,
                queries: vec![],
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

    // ========== View (保存済みフィルタ/タブ) ==========

    fn make_state_with_views(board: Board, views: Vec<(&str, &str)>) -> AppState {
        let mut state = make_state_with_board(board);
        state.views = views
            .into_iter()
            .map(|(name, filter)| crate::config::ViewConfig {
                name: name.to_string(),
                filter: filter.to_string(),
                layout: None,
            })
            .collect();
        state
    }

    #[test]
    fn test_switch_to_view_by_number_key() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card_with_labels("1", "Bug A", vec![("bug", "red")]),
                make_card("2", "Feature B"),
            ]),
        ]);
        let mut state = make_state_with_views(board, vec![("Bugs", "label:bug")]);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        // View 切替は server-side filter 付きで Board をリロードする
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec!["label:\"bug\"".into()],
            }
        );
        assert_eq!(state.active_view, Some(0));
        assert!(state.filter.active_filter.is_some());
        assert_eq!(state.filter.input, "label:bug");
    }

    #[test]
    fn test_switch_to_view_0_clears() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card_with_labels("1", "Bug A", vec![("bug", "red")]),
                make_card("2", "Feature B"),
            ]),
        ]);
        let mut state = make_state_with_views(board, vec![("Bugs", "label:bug")]);

        // Switch to view 1 first
        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.active_view, Some(0));

        // Press 0 to clear
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('0'))));
        // クリアでも全件再ロードを発火
        assert_eq!(
            cmd,
            Command::LoadBoard {
                project_id: "proj_1".into(),
                preferred_grouping_field_name: None,
                queries: vec![],
            }
        );
        assert_eq!(state.active_view, None);
        assert!(state.filter.active_filter.is_none());
        assert!(state.filter.input.is_empty());
    }

    #[test]
    fn test_switch_view_clamps_card_selection() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card_with_labels("1", "Bug A", vec![("bug", "red")]),
                make_card("2", "Feature B"),
                make_card("3", "Feature C"),
            ]),
        ]);
        let mut state = make_state_with_views(board, vec![("Bugs", "label:bug")]);
        state.selected_card = 2; // Select the last card

        // Switch to "Bugs" view — only 1 card matches, so selection should clamp
        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.selected_card, 0);
    }

    #[test]
    fn test_manual_filter_deselects_view() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_views(board, vec![("Bugs", "label:bug")]);

        // Switch to view 1
        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.active_view, Some(0));

        // Enter manual filter mode
        state.handle_event(AppEvent::Key(key(KeyCode::Char('/'))));
        assert_eq!(state.mode, ViewMode::Filter);

        // Type filter text and apply
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('e'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('s'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(state.mode, ViewMode::Board);
        assert_eq!(state.active_view, None);
        assert!(state.filter.active_filter.is_some());
    }

    #[test]
    fn test_ctrl_u_clears_view_and_filter() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_views(board, vec![("Bugs", "label:bug")]);

        // Switch to view 1
        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.active_view, Some(0));
        assert!(state.filter.active_filter.is_some());

        // Ctrl+U
        state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(state.active_view, None);
        assert!(state.filter.active_filter.is_none());
    }

    #[test]
    fn test_number_key_beyond_views_ignored() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_views(board, vec![("Bugs", "label:bug")]);

        // Press 3 when only 1 view exists
        state.handle_event(AppEvent::Key(key(KeyCode::Char('3'))));
        assert_eq!(state.active_view, None);
        assert!(state.filter.active_filter.is_none());
    }

    #[test]
    fn test_no_views_number_keys_ignored() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);

        // Press 1 with no views
        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.active_view, None);
        assert!(state.filter.active_filter.is_none());
    }

    #[test]
    fn test_switch_view_filter_applies() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card_with_labels("1", "Bug A", vec![("bug", "red")]),
                make_card("2", "Feature B"),
                make_card_with_labels("3", "Bug C", vec![("bug", "red")]),
            ]),
        ]);
        let mut state = make_state_with_views(board, vec![("Bugs", "label:bug")]);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));

        // Only bug cards should be visible
        let filtered = state.filtered_card_indices(0);
        assert_eq!(filtered, vec![0, 2]);
    }

    #[test]
    fn test_switch_between_views() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card_with_labels("1", "Bug A", vec![("bug", "red")]),
                make_card_with_assignees("2", "Task B", vec!["alice"]),
                make_card_with_labels("3", "Bug C", vec![("bug", "red")]),
            ]),
        ]);
        let mut state = make_state_with_views(board, vec![
            ("Bugs", "label:bug"),
            ("Alice", "assignee:alice"),
        ]);

        // Switch to view 1 (Bugs)
        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.active_view, Some(0));
        assert_eq!(state.filtered_card_indices(0), vec![0, 2]);

        // Switch to view 2 (Alice)
        state.handle_event(AppEvent::Key(key(KeyCode::Char('2'))));
        assert_eq!(state.active_view, Some(1));
        assert_eq!(state.filtered_card_indices(0), vec![1]);
        assert_eq!(state.filter.input, "assignee:alice");
    }

    #[test]
    fn test_view_assignee_at_me_resolves_to_viewer() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card_with_assignees("1", "My Task", vec!["uzimaru0000"]),
                make_card_with_assignees("2", "Other Task", vec!["alice"]),
            ]),
        ]);
        let mut state = make_state_with_views(board, vec![("My Tasks", "assignee:@me")]);
        state.viewer_login = "uzimaru0000".to_string();

        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.filtered_card_indices(0), vec![0]);
        assert_eq!(state.filter.input, "assignee:@uzimaru0000");
    }

    #[test]
    fn test_manual_filter_at_me_resolves_to_viewer() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![
                make_card_with_assignees("1", "My Task", vec!["uzimaru0000"]),
                make_card_with_assignees("2", "Other Task", vec!["alice"]),
            ]),
        ]);
        let mut state = make_state_with_board(board);
        state.viewer_login = "uzimaru0000".to_string();

        // Enter filter mode and type assignee:@me
        state.handle_event(AppEvent::Key(key(KeyCode::Char('/'))));
        for c in "assignee:@me".chars() {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        assert_eq!(state.filtered_card_indices(0), vec![0]);
    }

    // ========== カスタムフィールド (Issue #8) ==========

    fn priority_field() -> FieldDefinition {
        FieldDefinition::SingleSelect {
            id: "fld_priority".into(),
            name: "Priority".into(),
            options: vec![
                SingleSelectOption {
                    id: "opt_p0".into(),
                    name: "P0".into(),
                    color: Some(ColumnColor::Red),
                },
                SingleSelectOption {
                    id: "opt_p1".into(),
                    name: "P1".into(),
                    color: Some(ColumnColor::Orange),
                },
                SingleSelectOption {
                    id: "opt_p2".into(),
                    name: "P2".into(),
                    color: Some(ColumnColor::Gray),
                },
            ],
        }
    }

    fn estimate_field() -> FieldDefinition {
        FieldDefinition::Number {
            id: "fld_estimate".into(),
            name: "Estimate".into(),
        }
    }

    fn notes_field() -> FieldDefinition {
        FieldDefinition::Text {
            id: "fld_notes".into(),
            name: "Notes".into(),
        }
    }

    fn due_field() -> FieldDefinition {
        FieldDefinition::Date {
            id: "fld_due".into(),
            name: "Due".into(),
        }
    }

    fn sprint_field() -> FieldDefinition {
        FieldDefinition::Iteration {
            id: "fld_sprint".into(),
            name: "Sprint".into(),
            iterations: vec![
                IterationOption {
                    id: "it_1".into(),
                    title: "Sprint 1".into(),
                    start_date: "2026-04-01".into(),
                    duration: 14,
                    completed: false,
                },
                IterationOption {
                    id: "it_2".into(),
                    title: "Sprint 2".into(),
                    start_date: "2026-04-15".into(),
                    duration: 14,
                    completed: false,
                },
            ],
        }
    }

    fn setup_detail_with_fields(
        card: Card,
        fields: Vec<FieldDefinition>,
    ) -> AppState {
        let board = make_board_with_fields(
            vec![("Todo", "opt_1", vec![card])],
            fields,
        );
        let mut state = make_state_with_board(board);
        state.selected_column = 0;
        state.selected_card = 0;
        state.mode = ViewMode::Detail;
        state.detail_pane = DetailPane::Sidebar;
        state
    }

    #[test]
    fn test_sidebar_navigation_extends_with_custom_fields() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(
            card,
            vec![priority_field(), estimate_field()],
        );

        // Status (0) → Assignees (1) → Labels (2) → Milestone (3) → Priority (4) → Estimate (5) → Delete (6)
        state.sidebar_selected = 3; // Milestone
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 4); // Priority
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 5); // Estimate
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 6); // Delete
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 6); // clamp
    }

    #[test]
    fn test_enter_on_single_select_opens_edit_mode() {
        let card = make_card_with_custom_fields(
            "1",
            "Card A",
            vec![CustomFieldValue::SingleSelect {
                field_id: "fld_priority".into(),
                option_id: "opt_p1".into(),
                name: "P1".into(),
                color: Some(ColumnColor::Orange),
            }],
        );
        let mut state = setup_detail_with_fields(card, vec![priority_field()]);
        state.sidebar_selected = 4; // Priority

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
        match &state.sidebar_edit {
            Some(SidebarEditMode::CustomFieldSingleSelect {
                field_id,
                options,
                cursor,
                ..
            }) => {
                assert_eq!(field_id, "fld_priority");
                assert_eq!(options.len(), 3);
                // 現在の値 (P1) の行にカーソルがある
                assert_eq!(*cursor, 1);
            }
            _ => panic!("expected CustomFieldSingleSelect, got {:?}", state.sidebar_edit),
        }
    }

    #[test]
    fn test_single_select_toggle_sets_value_and_returns_update_command() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![priority_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter))); // open

        // P0 を選択 (cursor 0 → Enter)
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match cmd {
            Command::UpdateCustomField {
                field_id,
                value: CustomFieldValueInput::SingleSelect { option_id },
                ..
            } => {
                assert_eq!(field_id, "fld_priority");
                assert_eq!(option_id, "opt_p0");
            }
            other => panic!("expected UpdateCustomField, got {other:?}"),
        }

        // 楽観的更新: card.custom_fields に SingleSelect { option_id: opt_p0 } が入っている
        let card = state.selected_card_ref().unwrap();
        let v = card.custom_fields.iter().find(|v| v.field_id() == "fld_priority");
        match v {
            Some(CustomFieldValue::SingleSelect { option_id, name, .. }) => {
                assert_eq!(option_id, "opt_p0");
                assert_eq!(name, "P0");
            }
            other => panic!("expected SingleSelect P0 set, got {other:?}"),
        }

        // 編集モードは閉じている
        assert!(state.sidebar_edit.is_none());
    }

    #[test]
    fn test_single_select_clear_returns_clear_command() {
        let card = make_card_with_custom_fields(
            "1",
            "Card A",
            vec![CustomFieldValue::SingleSelect {
                field_id: "fld_priority".into(),
                option_id: "opt_p1".into(),
                name: "P1".into(),
                color: Some(ColumnColor::Orange),
            }],
        );
        let mut state = setup_detail_with_fields(card, vec![priority_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter))); // open

        // "None" 行は末尾 (options.len() = 3)
        for _ in 0..5 {
            state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        }
        // カーソルは末尾の None にクランプ
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match cmd {
            Command::ClearCustomField { field_id, .. } => {
                assert_eq!(field_id, "fld_priority");
            }
            other => panic!("expected ClearCustomField, got {other:?}"),
        }

        // 楽観更新: custom_fields からこの field が消えている
        let card = state.selected_card_ref().unwrap();
        assert!(
            card.custom_fields
                .iter()
                .all(|v| v.field_id() != "fld_priority")
        );
    }

    #[test]
    fn test_number_field_opens_text_input() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![estimate_field()]);
        state.sidebar_selected = 4; // Estimate

        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match &state.sidebar_edit {
            Some(SidebarEditMode::CustomFieldNumber { field_id, input, .. }) => {
                assert_eq!(field_id, "fld_estimate");
                assert!(input.is_empty());
            }
            other => panic!("expected CustomFieldNumber, got {other:?}"),
        }
    }

    #[test]
    fn test_number_field_accepts_digits_and_commits_on_enter() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![estimate_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter))); // open

        state.handle_event(AppEvent::Key(key(KeyCode::Char('3'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('.'))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('5'))));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        match cmd {
            Command::UpdateCustomField {
                field_id,
                value: CustomFieldValueInput::Number { number },
                ..
            } => {
                assert_eq!(field_id, "fld_estimate");
                assert!((number - 3.5).abs() < 1e-9);
            }
            other => panic!("expected UpdateCustomField Number, got {other:?}"),
        }
        // 楽観更新
        let card = state.selected_card_ref().unwrap();
        match card
            .custom_fields
            .iter()
            .find(|v| v.field_id() == "fld_estimate")
        {
            Some(CustomFieldValue::Number { number, .. }) => {
                assert!((*number - 3.5).abs() < 1e-9);
            }
            other => panic!("expected Number set, got {other:?}"),
        }
        assert!(state.sidebar_edit.is_none());
    }

    #[test]
    fn test_number_field_invalid_input_keeps_edit_open() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![estimate_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        state.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
        // 編集は閉じないまま
        assert!(matches!(
            state.sidebar_edit,
            Some(SidebarEditMode::CustomFieldNumber { .. })
        ));
    }

    #[test]
    fn test_text_field_commits_value() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![notes_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter))); // open

        for c in "hi".chars() {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match cmd {
            Command::UpdateCustomField {
                value: CustomFieldValueInput::Text { text },
                ..
            } => {
                assert_eq!(text, "hi");
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn test_date_field_validates_yyyy_mm_dd() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![due_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        for c in "2026-04-30".chars() {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match cmd {
            Command::UpdateCustomField {
                value: CustomFieldValueInput::Date { date },
                ..
            } => assert_eq!(date, "2026-04-30"),
            other => panic!("expected Date, got {other:?}"),
        }
    }

    #[test]
    fn test_date_field_invalid_keeps_open() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![due_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));

        for c in "2026/04/30".chars() {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(c))));
        }
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
        assert!(matches!(
            state.sidebar_edit,
            Some(SidebarEditMode::CustomFieldDate { .. })
        ));
    }

    #[test]
    fn test_iteration_field_selects_iteration() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![sprint_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter))); // open
        match &state.sidebar_edit {
            Some(SidebarEditMode::CustomFieldIteration { iterations, .. }) => {
                assert_eq!(iterations.len(), 2);
            }
            other => panic!("expected CustomFieldIteration, got {other:?}"),
        }

        // Sprint 2 を選択 (cursor=1 → j)
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        match cmd {
            Command::UpdateCustomField {
                value: CustomFieldValueInput::Iteration { iteration_id },
                ..
            } => assert_eq!(iteration_id, "it_2"),
            other => panic!("expected Iteration, got {other:?}"),
        }
    }

    #[test]
    fn test_esc_cancels_custom_field_edit() {
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![estimate_field()]);
        state.sidebar_selected = 4;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert!(state.sidebar_edit.is_some());

        state.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert!(state.sidebar_edit.is_none());
    }

    #[test]
    fn test_custom_field_edit_disabled_in_empty_fields() {
        // field_definitions が空なら Delete は 4 のまま
        let card = make_card_with_custom_fields("1", "Card A", vec![]);
        let mut state = setup_detail_with_fields(card, vec![]);
        state.sidebar_selected = 3;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 4); // Delete
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.sidebar_selected, 4);
    }

    // ========== グループ化軸の切替 ==========

    use crate::model::project::{Grouping, IterationOption, SingleSelectOption};

    /// Priority SingleSelect field (2 options) と Sprint Iteration field (2 iterations) を持つ
    /// Board を作成するヘルパー。初期 grouping は Status (dummy)。
    fn make_board_with_grouping_fields() -> Board {
        Board {
            project_title: "Test".into(),
            grouping: Grouping::SingleSelect {
                field_id: "field_status".into(),
                field_name: "Status".into(),
            },
            columns: vec![Column {
                option_id: "opt_todo".into(),
                name: "Todo".into(),
                color: None,
                cards: vec![
                    make_card_with_custom_fields(
                        "1",
                        "A",
                        vec![CustomFieldValue::SingleSelect {
                            field_id: "field_priority".into(),
                            option_id: "opt_p1".into(),
                            name: "P1".into(),
                            color: None,
                        }],
                    ),
                    make_card_with_custom_fields(
                        "2",
                        "B",
                        vec![CustomFieldValue::Iteration {
                            field_id: "field_sprint".into(),
                            iteration_id: "it_1".into(),
                            title: "Sprint 1".into(),
                        }],
                    ),
                    make_card_with_custom_fields("3", "C", vec![]),
                ],
            }],
            repositories: vec![],
            field_definitions: vec![
                FieldDefinition::SingleSelect {
                    id: "field_priority".into(),
                    name: "Priority".into(),
                    options: vec![
                        SingleSelectOption {
                            id: "opt_p0".into(),
                            name: "P0".into(),
                            color: None,
                        },
                        SingleSelectOption {
                            id: "opt_p1".into(),
                            name: "P1".into(),
                            color: None,
                        },
                    ],
                },
                FieldDefinition::Iteration {
                    id: "field_sprint".into(),
                    name: "Sprint".into(),
                    iterations: vec![
                        IterationOption {
                            id: "it_1".into(),
                            title: "Sprint 1".into(),
                            start_date: "2026-04-01".into(),
                            duration: 14,
                            completed: false,
                        },
                        IterationOption {
                            id: "it_2".into(),
                            title: "Sprint 2".into(),
                            start_date: "2026-04-15".into(),
                            duration: 14,
                            completed: false,
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn test_change_grouping_opens_modal() {
        let mut state = make_state_with_board(make_board_with_grouping_fields());
        // Ctrl+g を押す
        state.handle_event(AppEvent::Key(key_with_mod(
            KeyCode::Char('g'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(state.mode, ViewMode::GroupBySelect);
        let s = state.group_by_select_state.as_ref().expect("state set");
        assert_eq!(s.candidates.len(), 2); // Priority + Sprint
    }

    #[test]
    fn test_apply_grouping_rebuilds_columns_for_single_select() {
        let mut state = make_state_with_board(make_board_with_grouping_fields());
        state.apply_grouping(Grouping::SingleSelect {
            field_id: "field_priority".into(),
            field_name: "Priority".into(),
        });
        let board = state.board.as_ref().unwrap();
        assert!(matches!(board.grouping, Grouping::SingleSelect { .. }));
        // P0 カラム, P1 カラム, + 値未設定 (2,3) が "No Priority" カラム (先頭)
        assert_eq!(board.columns.len(), 3);
        assert_eq!(board.columns[0].name, "No Priority");
        assert_eq!(board.columns[0].cards.len(), 2); // item_id 2, 3
        assert_eq!(board.columns[1].name, "P0");
        assert_eq!(board.columns[1].cards.len(), 0);
        assert_eq!(board.columns[2].name, "P1");
        assert_eq!(board.columns[2].cards.len(), 1); // item_id 1
        assert_eq!(board.columns[2].cards[0].item_id, "1");
    }

    #[test]
    fn test_apply_grouping_rebuilds_columns_for_iteration() {
        let mut state = make_state_with_board(make_board_with_grouping_fields());
        state.apply_grouping(Grouping::Iteration {
            field_id: "field_sprint".into(),
            field_name: "Sprint".into(),
        });
        let board = state.board.as_ref().unwrap();
        assert!(matches!(board.grouping, Grouping::Iteration { .. }));
        // Sprint1 + Sprint2 カラム + "No Sprint" (item_id 1, 3 が対応値なし)
        assert_eq!(board.columns.len(), 3);
        assert_eq!(board.columns[0].name, "No Sprint");
        assert_eq!(board.columns[0].cards.len(), 2);
        // Sprint 1 カラムは iteration_id "it_1" に対応、item_id 2 が入っている
        let s1 = board
            .columns
            .iter()
            .find(|c| c.option_id == "it_1")
            .expect("Sprint 1 column");
        assert_eq!(s1.cards.len(), 1);
        assert_eq!(s1.cards[0].item_id, "2");
    }

    #[test]
    fn test_grab_confirm_sends_iteration_id_for_iteration_axis() {
        // Iteration 軸で Space → h/l → Space で確定したとき、
        // Command::MoveCard の value が Iteration { iteration_id } になること。
        let mut state = make_state_with_board(make_board_with_grouping_fields());
        state.apply_grouping(Grouping::Iteration {
            field_id: "field_sprint".into(),
            field_name: "Sprint".into(),
        });
        let sprint1_idx = state
            .board
            .as_ref()
            .unwrap()
            .columns
            .iter()
            .position(|c| c.option_id == "it_1")
            .unwrap();
        let sprint2_idx = state
            .board
            .as_ref()
            .unwrap()
            .columns
            .iter()
            .position(|c| c.option_id == "it_2")
            .unwrap();
        state.selected_column = sprint1_idx;
        state.selected_card = 0;

        state.handle_event(AppEvent::Key(key(KeyCode::Char(' ')))); // grab
        let direction_key = if sprint2_idx > sprint1_idx { 'l' } else { 'h' };
        let steps = (sprint2_idx as isize - sprint1_idx as isize).unsigned_abs();
        for _ in 0..steps {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(direction_key))));
        }
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));

        match cmd {
            Command::Batch(cmds) => {
                let move_cmd = cmds.iter().find(|c| matches!(c, Command::MoveCard { .. }));
                match move_cmd {
                    Some(Command::MoveCard {
                        field_id,
                        value: CustomFieldValueInput::Iteration { iteration_id },
                        ..
                    }) => {
                        assert_eq!(field_id, "field_sprint");
                        assert_eq!(iteration_id, "it_2");
                    }
                    _ => panic!("expected MoveCard with Iteration in Batch, got {cmds:?}"),
                }
            }
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    #[test]
    fn test_config_group_by_selects_named_field_on_load() {
        use crate::github::client::{build_columns_for_grouping, choose_grouping};
        let field_defs = vec![
            FieldDefinition::SingleSelect {
                id: "field_status".into(),
                name: "Status".into(),
                options: vec![],
            },
            FieldDefinition::SingleSelect {
                id: "field_priority".into(),
                name: "Priority".into(),
                options: vec![SingleSelectOption {
                    id: "opt_p0".into(),
                    name: "P0".into(),
                    color: None,
                }],
            },
        ];
        let chosen = choose_grouping(&field_defs, Some("Priority"));
        match chosen {
            Grouping::SingleSelect {
                field_id,
                field_name,
            } => {
                assert_eq!(field_id, "field_priority");
                assert_eq!(field_name, "Priority");
            }
            other => panic!("expected Priority SingleSelect, got {other:?}"),
        }
        // 存在しない名前 → Status にフォールバック
        let chosen = choose_grouping(&field_defs, Some("Nonexistent"));
        match chosen {
            Grouping::SingleSelect { field_name, .. } => assert_eq!(field_name, "Status"),
            other => panic!("expected Status fallback, got {other:?}"),
        }
        // build_columns_for_grouping が "No Priority" カラム + P0 カラムを返す (カード未配置)
        let cols = build_columns_for_grouping(
            &Grouping::SingleSelect {
                field_id: "field_priority".into(),
                field_name: "Priority".into(),
            },
            &field_defs,
            vec![],
        );
        assert_eq!(cols.len(), 1); // 値なしカードがないので "No Priority" カラムも作られない
        assert_eq!(cols[0].name, "P0");
    }

    #[test]
    fn test_grab_confirm_updates_custom_fields_optimistically() {
        // Priority 軸で Space → h/l → Space のパスで移動したとき、
        // card.custom_fields も移動先の値に更新されること。
        let mut state = make_state_with_board(make_board_with_grouping_fields());
        state.apply_grouping(Grouping::SingleSelect {
            field_id: "field_priority".into(),
            field_name: "Priority".into(),
        });
        let p1_idx = state
            .board
            .as_ref()
            .unwrap()
            .columns
            .iter()
            .position(|c| c.option_id == "opt_p1")
            .unwrap();
        let p0_idx = state
            .board
            .as_ref()
            .unwrap()
            .columns
            .iter()
            .position(|c| c.option_id == "opt_p0")
            .unwrap();
        state.selected_column = p1_idx;
        state.selected_card = 0;

        // Space で grab 開始
        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::CardGrab);

        // h または l で target まで移動 (P0 が P1 の左なら h, 右なら l)
        let direction_key = if p0_idx < p1_idx { 'h' } else { 'l' };
        let steps = (p1_idx as isize - p0_idx as isize).unsigned_abs();
        for _ in 0..steps {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(direction_key))));
        }

        // Space で確定
        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::Board);

        // P0 カラムに item_id "1" がいて、Priority=P0 に更新されている
        let board = state.board.as_ref().unwrap();
        let p0_col = &board.columns[p0_idx];
        let moved = p0_col
            .cards
            .iter()
            .find(|c| c.item_id == "1")
            .expect("card moved to P0");
        let pri = moved
            .custom_fields
            .iter()
            .find(|fv| fv.field_id() == "field_priority")
            .expect("Priority field present");
        match pri {
            CustomFieldValue::SingleSelect { option_id, .. } => {
                assert_eq!(option_id, "opt_p0");
            }
            _ => panic!("expected SingleSelect"),
        }
    }

    #[test]
    fn test_grab_confirm_preserves_other_custom_fields() {
        // 実運用: カードには Status と Priority の SingleSelect が両方設定されている状況。
        // Priority 軸で grab 移動したとき、Status は維持、Priority だけ書き換わる。
        let mut board = make_board_with_grouping_fields();
        board.columns[0].cards[0]
            .custom_fields
            .push(CustomFieldValue::SingleSelect {
                field_id: "field_status".into(),
                option_id: "opt_todo".into(),
                name: "Todo".into(),
                color: None,
            });
        let mut state = make_state_with_board(board);
        state.apply_grouping(Grouping::SingleSelect {
            field_id: "field_priority".into(),
            field_name: "Priority".into(),
        });
        let p1_idx = state
            .board
            .as_ref()
            .unwrap()
            .columns
            .iter()
            .position(|c| c.option_id == "opt_p1")
            .unwrap();
        let p0_idx = state
            .board
            .as_ref()
            .unwrap()
            .columns
            .iter()
            .position(|c| c.option_id == "opt_p0")
            .unwrap();
        state.selected_column = p1_idx;
        state.selected_card = 0;

        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        let direction_key = if p0_idx < p1_idx { 'h' } else { 'l' };
        let steps = (p1_idx as isize - p0_idx as isize).unsigned_abs();
        for _ in 0..steps {
            state.handle_event(AppEvent::Key(key(KeyCode::Char(direction_key))));
        }
        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));

        let board = state.board.as_ref().unwrap();
        let p0_col = &board.columns[p0_idx];
        let moved = p0_col
            .cards
            .iter()
            .find(|c| c.item_id == "1")
            .expect("card moved to P0");

        // Priority は P0 に更新 (1 件のみ)
        let priorities: Vec<&CustomFieldValue> = moved
            .custom_fields
            .iter()
            .filter(|fv| fv.field_id() == "field_priority")
            .collect();
        assert_eq!(priorities.len(), 1);
        match priorities[0] {
            CustomFieldValue::SingleSelect { option_id, .. } => {
                assert_eq!(option_id, "opt_p0");
            }
            _ => panic!("expected SingleSelect"),
        }
        // Status は維持
        let statuses: Vec<&CustomFieldValue> = moved
            .custom_fields
            .iter()
            .filter(|fv| fv.field_id() == "field_status")
            .collect();
        assert_eq!(statuses.len(), 1);
    }

    #[test]
    fn test_change_grouping_resets_selection() {
        let mut state = make_state_with_board(make_board_with_grouping_fields());
        state.selected_column = 0;
        state.selected_card = 2;
        state.apply_grouping(Grouping::SingleSelect {
            field_id: "field_priority".into(),
            field_name: "Priority".into(),
        });
        assert_eq!(state.selected_card, 0);
        assert!(state.selected_column < state.board.as_ref().unwrap().columns.len());
    }

    // ========== LayoutMode (Table view) ==========

    #[test]
    fn test_toggle_layout_with_t_key() {
        let mut state = make_state_with_board(make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A")],
        )]));
        assert_eq!(state.current_layout, LayoutMode::Board);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        assert_eq!(state.current_layout, LayoutMode::Table);
        // Iteration field が無いので Roadmap は skip し Board に戻る
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        assert_eq!(state.current_layout, LayoutMode::Board);
    }

    #[test]
    fn test_toggle_layout_three_way_cycle_with_iteration_field() {
        use crate::model::project::{FieldDefinition, IterationOption};
        let board = make_board_with_fields(
            vec![(
                "Todo",
                "opt_1",
                vec![make_card("1", "A")],
            )],
            vec![FieldDefinition::Iteration {
                id: "fld_it".into(),
                name: "Iteration".into(),
                iterations: vec![IterationOption {
                    id: "it_1".into(),
                    title: "Sprint 1".into(),
                    start_date: "2026-04-01".into(),
                    duration: 14,
                    completed: false,
                }],
            }],
        );
        let mut state = make_state_with_board(board);

        assert_eq!(state.current_layout, LayoutMode::Board);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        assert_eq!(state.current_layout, LayoutMode::Table);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        assert_eq!(state.current_layout, LayoutMode::Roadmap);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        assert_eq!(state.current_layout, LayoutMode::Board);
    }

    #[test]
    fn test_roadmap_move_down_and_back_to_board() {
        use crate::model::project::{FieldDefinition, IterationOption};
        let board = make_board_with_fields(
            vec![(
                "Todo",
                "opt_1",
                vec![make_card("1", "A"), make_card("2", "B")],
            )],
            vec![FieldDefinition::Iteration {
                id: "fld_it".into(),
                name: "Iteration".into(),
                iterations: vec![IterationOption {
                    id: "it_1".into(),
                    title: "Sprint 1".into(),
                    start_date: "2026-04-01".into(),
                    duration: 14,
                    completed: false,
                }],
            }],
        );
        let mut state = make_state_with_board(board);
        state.current_layout = LayoutMode::Roadmap;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.roadmap_selected_row, 1);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.roadmap_selected_row, 1); // 末尾でクランプ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.roadmap_selected_row, 0);

        // Roadmap → Board で選択が同期される
        state.roadmap_selected_row = 1;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('t'))));
        assert_eq!(state.current_layout, LayoutMode::Board);
        assert_eq!(state.selected_card, 1);
    }

    #[test]
    fn test_table_move_down() {
        let mut state = make_state_with_board(make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A"), make_card("2", "B")]),
            ("Done", "opt_2", vec![make_card("3", "C")]),
        ]));
        state.current_layout = LayoutMode::Table;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.table_selected_row, 1);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.table_selected_row, 2);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));
        assert_eq!(state.table_selected_row, 2); // 末尾でクランプ
        state.handle_event(AppEvent::Key(key(KeyCode::Char('k'))));
        assert_eq!(state.table_selected_row, 1);
    }

    #[test]
    fn test_table_first_last_item() {
        let mut state = make_state_with_board(make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A"), make_card("2", "B")]),
            ("Done", "opt_2", vec![make_card("3", "C")]),
        ]));
        state.current_layout = LayoutMode::Table;
        state.handle_event(AppEvent::Key(key(KeyCode::Char('G'))));
        assert_eq!(state.table_selected_row, 2);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('g'))));
        assert_eq!(state.table_selected_row, 0);
    }

    #[test]
    fn test_table_enter_opens_detail_with_correct_card() {
        let mut state = make_state_with_board(make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A"), make_card("2", "B")]),
            ("Done", "opt_2", vec![make_card("3", "C")]),
        ]));
        state.current_layout = LayoutMode::Table;
        state.table_selected_row = 2;
        state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert_eq!(state.selected_column, 1);
        assert_eq!(state.selected_card, 0);
    }

    #[test]
    fn test_table_skips_filtered_rows() {
        let mut state = make_state_with_board(make_board(vec![(
            "Todo",
            "opt_1",
            vec![
                make_card("1", "Fix bug"),
                make_card("2", "Add feature"),
                make_card("3", "Fix typo"),
            ],
        )]));
        state.current_layout = LayoutMode::Table;
        state.filter.active_filter = Some(ActiveFilter::parse("fix"));
        let rows = state.table_rows();
        assert_eq!(rows, vec![(0, 0), (0, 2)]);
    }

    #[test]
    fn test_switch_view_restores_table_layout() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.set_views(vec![crate::config::ViewConfig {
            name: "Bugs".into(),
            filter: "label:bug".into(),
            layout: Some(crate::config::LayoutModeConfig::Table),
        }]);
        assert_eq!(state.current_layout, LayoutMode::Board);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.active_view, Some(0));
        assert_eq!(state.current_layout, LayoutMode::Table);
    }

    #[test]
    fn test_switch_view_default_board_layout() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.set_views(vec![crate::config::ViewConfig {
            name: "All".into(),
            filter: String::new(),
            layout: None,
        }]);
        // 事前に Table にしておく
        state.current_layout = LayoutMode::Table;

        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.current_layout, LayoutMode::Board);
    }

    #[test]
    fn test_table_grab_reorders_within_same_column() {
        let mut state = make_state_with_board(make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_card("1", "A"), make_card("2", "B")],
        )]));
        state.current_layout = LayoutMode::Table;
        state.table_selected_row = 0;

        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));

        // 物理的な column.cards は変わっていない (status 不変)
        let cols = &state.board.as_ref().unwrap().columns;
        assert_eq!(cols[0].cards[0].title, "A");
        assert_eq!(cols[0].cards[1].title, "B");

        // table_rows は B, A の順 (表示順だけ入れ替わる)
        let rows = state.table_rows();
        assert_eq!(rows, vec![(0, 1), (0, 0)]);
        assert_eq!(state.table_selected_row, 1);
    }

    #[test]
    fn test_table_grab_reorders_across_columns_without_status_change() {
        let mut state = make_state_with_board(make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A")]),
            ("Done", "opt_2", vec![make_card("3", "C")]),
        ]));
        state.current_layout = LayoutMode::Table;
        state.table_selected_row = 0; // A (Todo)

        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        state.handle_event(AppEvent::Key(key(KeyCode::Char('j'))));

        // 物理的な配置は変わらず (A は Todo のまま、C は Done のまま)
        let cols = &state.board.as_ref().unwrap().columns;
        assert_eq!(cols[0].cards[0].title, "A");
        assert_eq!(cols[1].cards[0].title, "C");

        // table_rows の表示順は C, A
        let rows = state.table_rows();
        assert_eq!(rows, vec![(1, 0), (0, 0)]);
    }

    #[test]
    fn test_table_grab_horizontal_is_noop() {
        let mut state = make_state_with_board(make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A")]),
            ("Done", "opt_2", vec![make_card("3", "C")]),
        ]));
        state.current_layout = LayoutMode::Table;
        state.table_selected_row = 0;

        state.handle_event(AppEvent::Key(key(KeyCode::Char(' '))));
        assert_eq!(state.mode, ViewMode::CardGrab);

        // l (右) は Table モードでは no-op (Todo のままで Done に行かない)
        state.handle_event(AppEvent::Key(key(KeyCode::Char('l'))));
        assert_eq!(state.selected_column, 0);
        let cols = &state.board.as_ref().unwrap().columns;
        assert_eq!(cols[0].cards.len(), 1);
        assert_eq!(cols[1].cards.len(), 1);
    }

    #[test]
    fn test_clear_view_resets_to_board() {
        let board = make_board(vec![("Todo", "opt_1", vec![make_card("1", "A")])]);
        let mut state = make_state_with_board(board);
        state.set_views(vec![crate::config::ViewConfig {
            name: "Bugs".into(),
            filter: "label:bug".into(),
            layout: Some(crate::config::LayoutModeConfig::Table),
        }]);
        state.handle_event(AppEvent::Key(key(KeyCode::Char('1'))));
        assert_eq!(state.current_layout, LayoutMode::Table);

        state.handle_event(AppEvent::Key(key(KeyCode::Char('0'))));
        assert_eq!(state.current_layout, LayoutMode::Board);
        assert_eq!(state.active_view, None);
    }

    #[test]
    fn test_toggle_layout_preserves_selected_card() {
        let mut state = make_state_with_board(make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "A"), make_card("2", "B")]),
            ("Done", "opt_2", vec![make_card("3", "C")]),
        ]));
        // Done カラムの最初のカードを選択
        state.selected_column = 1;
        state.selected_card = 0;
        state.toggle_layout();
        assert_eq!(state.current_layout, LayoutMode::Table);
        assert_eq!(state.table_selected_row, 2);

        // Table → Board でも復元
        state.toggle_layout();
        assert_eq!(state.current_layout, LayoutMode::Board);
        assert_eq!(state.selected_column, 1);
        assert_eq!(state.selected_card, 0);
    }
}
