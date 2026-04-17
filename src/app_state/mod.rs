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
use crate::model::state::{
    ActiveFilter, CommentListState, ConfirmAction, ConfirmState, CreateCardField,
    CreateCardState, DetailPane, EditCardField, EditCardState, EditItem, FilterState, GrabState,
    GroupBySelectState, LayoutMode, LoadingState, NewCardType, PendingIssueCreate,
    ReactionPickerState, ReactionTarget, RepoSelectState, Scene, SidebarEditMode, SidebarSection,
    ViewMode,
};
#[cfg(test)]
use crate::model::state::{SIDEBAR_ASSIGNEES, SIDEBAR_LABELS};

mod board;
mod detail;
mod filter;
mod modal;

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

    /// プログレッシブレンダリング: ボードロードの世代番号。リロード/フィルタ変更時にインクリメントし、
    /// 古い BoardPageLoaded を無視する。
    pub board_generation: u64,

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

    /// 現在表示中の Scene (Phase B リファクタ中の shim。mode + Option<FooState> から
    /// `sync_scene_from_mode` で派生するだけの add-only フィールド)。
    pub scene: Scene,
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
            board_generation: 0,
            views: Vec::new(),
            active_view: None,
            owner,
            viewer_login: String::new(),
            preferred_grouping_field_name: None,
            keymap: Keymap::default_keymap(),
            scene: Scene::ProjectSelect,
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
        self.board_generation = self.board_generation.wrapping_add(1);
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
        self.sync_scene_from_mode();
        cmd
    }

    /// `mode` + `Option<FooState>` から `scene` を派生させる shim。
    /// Phase B のリファクタが完了し各 Scene バリアントが自前で state を持つように
    /// なった時点で撤去する。
    pub(crate) fn sync_scene_from_mode(&mut self) {
        self.scene = Scene::from(&self.mode);
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
            AppEvent::BoardPageLoaded(Ok(page_data)) => {
                // generation が古ければ無視 (リロード/フィルタ変更で obsolete)
                if page_data.generation != self.board_generation {
                    return Command::None;
                }
                if let Some(board) = &mut self.board {
                    // 新しいカードを既存カラムに option_id マッチで追加
                    for card in page_data.cards {
                        let mut placed = false;
                        let field_id = board.grouping.field_id().map(|s| s.to_string());
                        if let Some(fid) = &field_id {
                            for cf in &card.custom_fields {
                                if let Some((cf_field_id, cf_option_id)) = cf.field_and_option_id()
                                    && cf_field_id == fid
                                {
                                    for col in &mut board.columns {
                                        if col.option_id == cf_option_id {
                                            col.cards.push(card.clone());
                                            placed = true;
                                            break;
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        if !placed {
                            // "No <field>" カラム (option_id が空) に追加
                            if let Some(col) = board.columns.iter_mut().find(|c| c.option_id.is_empty()) {
                                col.cards.push(card);
                            } else if let Some(col) = board.columns.first_mut() {
                                col.cards.push(card);
                            }
                        }
                    }
                }
                self.rebuild_table_order();
                if page_data.remaining.is_empty() {
                    // 全ページ完了
                    self.loading = LoadingState::Idle;
                    Command::None
                } else {
                    // まだページがある → 次ページを取得
                    self.loading = LoadingState::Refreshing;
                    if let Some(project) = &self.current_project {
                        Command::LoadBoardNextPage {
                            project_id: project.id.clone(),
                            preferred_grouping_field_name: self
                                .preferred_grouping_field_name
                                .clone(),
                            pagination: page_data.remaining,
                            generation: self.board_generation,
                        }
                    } else {
                        Command::None
                    }
                }
            }
            AppEvent::BoardPageLoaded(Err(e)) => {
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
            AppEvent::CardDetailLoaded(Ok((item_id, detail))) => {
                // Board 上のカードに body/comments/reactions/linked_prs をマージ
                if let Some(board) = &mut self.board {
                    for col in &mut board.columns {
                        for card in &mut col.cards {
                            if card.item_id == item_id {
                                card.body = Some(detail.body.clone());
                                card.comments = detail.comments.clone();
                                card.reactions = detail.reactions.clone();
                                card.linked_prs = detail.linked_prs.clone();
                            }
                        }
                    }
                }
                // detail_stack 上のカードにもマージ
                for card in &mut self.detail_stack {
                    if card.item_id == item_id {
                        card.body = Some(detail.body.clone());
                        card.comments = detail.comments.clone();
                        card.reactions = detail.reactions.clone();
                        card.linked_prs = detail.linked_prs.clone();
                    }
                }
                // コメントが 20 件以上なら追加で全件取得
                if detail.comments.len() >= 20 {
                    // content_id を Board のカードから取得
                    if let Some(board) = &self.board {
                        for col in &board.columns {
                            for card in &col.cards {
                                if card.item_id == item_id
                                    && let Some(cid) = &card.content_id
                                {
                                    return Command::FetchComments {
                                        content_id: cid.clone(),
                                    };
                                }
                            }
                        }
                    }
                }
                Command::None
            }
            AppEvent::CardDetailLoaded(Err(e)) => {
                self.loading = LoadingState::Error(e);
                Command::None
            }
            AppEvent::Tick | AppEvent::Resize(_, _) => Command::None,
        }
    }

    /// Board/Table/Roadmap ハンドラで共通のアクション処理。
    /// `Some(cmd)` を返したら呼び出し側はそれを返す。`None` は未処理。
    pub(super) fn try_handle_common_board_action(
        &mut self,
        action: Action,
    ) -> Option<Command> {
        match action {
            Action::Quit | Action::ForceQuit => {
                self.should_quit = true;
                Some(Command::None)
            }
            Action::SwitchProject => Some(self.enter_project_select()),
            Action::Refresh => {
                if let Some(project) = &self.current_project {
                    let id = project.id.clone();
                    Some(self.start_loading_board(&id))
                } else {
                    Some(Command::None)
                }
            }
            Action::ShowHelp => {
                self.mode = ViewMode::Help;
                Some(Command::None)
            }
            Action::ChangeGrouping => {
                self.open_group_by_select();
                Some(Command::None)
            }
            Action::ToggleLayout => {
                self.toggle_layout();
                Some(Command::None)
            }
            Action::StartFilter => {
                self.filter.input.clear();
                self.filter.cursor_pos = 0;
                self.mode = ViewMode::Filter;
                Some(Command::None)
            }
            Action::ShowArchivedList => Some(self.show_archived_list()),
            Action::NewCard => {
                self.create_card_state = CreateCardState::default();
                self.mode = ViewMode::CreateCard;
                Some(Command::None)
            }
            _ => None,
        }
    }

    /// View switching (1-9, 0) キーの処理。マッチしたら Some(cmd) を返す。
    pub(super) fn try_handle_view_switch(&mut self, key: &KeyEvent) -> Option<Command> {
        if let KeyCode::Char(c @ '1'..='9') = key.code
            && key.modifiers == KeyModifiers::NONE
        {
            return Some(self.switch_to_view((c as usize) - ('1' as usize)));
        }
        if key.code == KeyCode::Char('0') && key.modifiers == KeyModifiers::NONE {
            return Some(self.clear_view());
        }
        None
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
        // body を設定して FetchCardDetail が発行されないようにする
        card.body = Some("body".into());
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
        // body を設定して FetchCardDetail が発行されないようにする
        card.body = Some("body".into());
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(cmd, Command::None);
    }

    #[test]
    fn test_detail_no_fetch_sub_issues_for_draft() {
        let mut card = make_draft_card("1", "Draft", "body");
        // body を設定して FetchCardDetail が発行されないようにする
        card.body = Some("body".into());
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
                field_name: "Priority".into(),
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
                field_name: "Priority".into(),
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
                            field_name: "Priority".into(),
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
                            field_name: "Sprint".into(),
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
                field_name: "Status".into(),
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

    // ── Step 2: Detail 遅延取得 ──────────────────────────

    #[test]
    fn test_open_detail_fetches_card_detail_when_body_is_none() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_issue_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);

        // body が None のカードで Detail を開く → FetchCardDetail が返る
        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);
        assert!(
            matches!(
                cmd,
                Command::FetchCardDetail {
                    ref item_id,
                    ref content_id,
                } if item_id == "1" && content_id == "issue_1"
            ),
            "Expected FetchCardDetail, got {:?}",
            cmd
        );
    }

    #[test]
    fn test_open_detail_does_not_fetch_when_body_present() {
        let mut card = make_issue_card("1", "Card A");
        card.body = Some("some body".into());
        let board = make_board(vec![("Todo", "opt_1", vec![card])]);
        let mut state = make_state_with_board(board);

        let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(state.mode, ViewMode::Detail);
        // body が既にあるので FetchCardDetail は発行されない
        assert!(
            !matches!(cmd, Command::FetchCardDetail { .. }),
            "Should not fetch card detail when body is present, got {:?}",
            cmd
        );
    }

    #[test]
    fn test_card_detail_loaded_merges_into_board() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_issue_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;

        let detail = CardDetail {
            body: "Hello world".into(),
            comments: vec![Comment {
                id: "c1".into(),
                author: "alice".into(),
                body: "A comment".into(),
                created_at: "2024-01-01T00:00:00Z".into(),
                reactions: vec![],
            }],
            reactions: vec![],
            linked_prs: vec![],
        };

        let cmd = state.handle_event(AppEvent::CardDetailLoaded(Ok(("1".into(), detail))));
        assert_eq!(cmd, Command::None);

        // ボード上のカードに body / comments がマージされている
        let card = &state.board.as_ref().unwrap().columns[0].cards[0];
        assert_eq!(card.body.as_deref(), Some("Hello world"));
        assert_eq!(card.comments.len(), 1);
        assert_eq!(card.comments[0].author, "alice");
    }

    #[test]
    fn test_card_detail_loaded_triggers_fetch_comments_when_20() {
        let board = make_board(vec![(
            "Todo",
            "opt_1",
            vec![make_issue_card("1", "Card A")],
        )]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Detail;

        // 20 件のコメントで CardDetailLoaded
        let comments: Vec<Comment> = (0..20)
            .map(|i| Comment {
                id: format!("c{i}"),
                author: "alice".into(),
                body: format!("comment {i}"),
                created_at: "2024-01-01T00:00:00Z".into(),
                reactions: vec![],
            })
            .collect();

        let detail = CardDetail {
            body: "body".into(),
            comments,
            reactions: vec![],
            linked_prs: vec![],
        };

        let cmd = state.handle_event(AppEvent::CardDetailLoaded(Ok(("1".into(), detail))));
        // 20 件以上なので FetchComments が返る
        assert!(
            matches!(cmd, Command::FetchComments { ref content_id } if content_id == "issue_1"),
            "Expected FetchComments, got {:?}",
            cmd
        );
    }

    // ── Step 3: プログレッシブレンダリング ──────────────

    #[test]
    fn test_board_page_loaded_adds_cards_to_correct_column() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
            ("Done", "opt_2", vec![]),
        ]);
        let mut state = make_state_with_board(board);
        state.mode = ViewMode::Board;
        let current_gen = state.board_generation;

        // "opt_2" (Done) にマッチする custom_fields を持つカードを追加
        let mut new_card = make_card("2", "Card B");
        new_card.custom_fields = vec![CustomFieldValue::SingleSelect {
            field_id: "field_1".into(),
            field_name: "Status".into(),
            option_id: "opt_2".into(),
            name: "Done".into(),
            color: None,
        }];

        let page_data = crate::event::BoardPageData {
            cards: vec![new_card],
            remaining: vec![],
            generation: current_gen,
        };

        let cmd = state.handle_event(AppEvent::BoardPageLoaded(Ok(page_data)));
        assert_eq!(cmd, Command::None);

        // Done カラムにカードが追加されている
        let board = state.board.as_ref().unwrap();
        assert_eq!(board.columns[0].cards.len(), 1); // Todo: 1
        assert_eq!(board.columns[1].cards.len(), 1); // Done: 1
        assert_eq!(board.columns[1].cards[0].title, "Card B");
    }

    #[test]
    fn test_board_page_loaded_ignores_stale_generation() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        let stale = state.board_generation.wrapping_sub(1);

        let page_data = crate::event::BoardPageData {
            cards: vec![make_card("2", "Card B")],
            remaining: vec![],
            generation: stale,
        };

        let cmd = state.handle_event(AppEvent::BoardPageLoaded(Ok(page_data)));
        assert_eq!(cmd, Command::None);

        // stale なので Todo カラムにカードは追加されない
        let board = state.board.as_ref().unwrap();
        assert_eq!(board.columns[0].cards.len(), 1);
    }

    #[test]
    fn test_board_page_loaded_with_remaining_issues_next_page() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.current_project = Some(ProjectSummary {
            id: "proj_1".into(),
            title: "Test".into(),
            number: 1,
            description: None,
        });
        let current_gen = state.board_generation;

        let remaining = vec![PaginationState {
            query: None,
            cursor: "cursor_2".into(),
        }];

        let page_data = crate::event::BoardPageData {
            cards: vec![],
            remaining,
            generation: current_gen,
        };

        let cmd = state.handle_event(AppEvent::BoardPageLoaded(Ok(page_data)));
        // 残りページがあるので LoadBoardNextPage が返る
        assert!(
            matches!(
                cmd,
                Command::LoadBoardNextPage {
                    ref project_id,
                    generation,
                    ..
                } if project_id == "proj_1" && generation == current_gen
            ),
            "Expected LoadBoardNextPage, got {:?}",
            cmd
        );
        assert_eq!(state.loading, LoadingState::Refreshing);
    }

    #[test]
    fn test_board_page_loaded_no_remaining_sets_idle() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.loading = LoadingState::Refreshing;
        let current_gen = state.board_generation;

        let page_data = crate::event::BoardPageData {
            cards: vec![],
            remaining: vec![],
            generation: current_gen,
        };

        let cmd = state.handle_event(AppEvent::BoardPageLoaded(Ok(page_data)));
        assert_eq!(cmd, Command::None);
        assert_eq!(state.loading, LoadingState::Idle);
    }

    #[test]
    fn test_start_loading_board_increments_generation() {
        let board = make_board(vec![
            ("Todo", "opt_1", vec![make_card("1", "Card A")]),
        ]);
        let mut state = make_state_with_board(board);
        state.current_project = Some(ProjectSummary {
            id: "proj_1".into(),
            title: "Test".into(),
            number: 1,
            description: None,
        });
        let before = state.board_generation;
        state.start_loading_board("proj_1");
        assert_eq!(state.board_generation, before + 1);
    }
}
