#[derive(Clone, Debug)]
pub struct GrabState {
    pub origin_column: usize,
    pub origin_card_index: usize,
    pub item_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Board,
    ProjectSelect,
    Help,
    Filter,
    Confirm,
    CreateCard,
    Detail,
    RepoSelect,
    CardGrab,
    EditCard,
    CommentList,
    GroupBySelect,
    ReactionPicker,
    ArchivedList,
}

/// 現在表示中の「シーン」。Phase B リファクタで段階的にモード固有の state を
/// このバリアントに取り込み、`Option<FooState>` 幽霊状態をコンパイル時に排除する。
///
/// 移行済みバリアント (state を自前で保持):
/// - `ReactionPicker(ReactionPickerState)`
///
/// 未移行バリアントは現状 `mode: ViewMode` + `Option<FooState>` と共存する
/// 単なるタグで、`AppState::sync_scene_from_mode()` で `mode` から派生させる。
// Scene バリアントが持つ state (例: ArchivedListState の Card) には PartialEq が
// 未実装なので、Scene 全体でも PartialEq/Eq は derive しない。Scene の比較が
// 必要な場合は `matches!` やパターンマッチで明示的に扱う。
#[derive(Clone, Debug)]
pub enum Scene {
    Board,
    ProjectSelect,
    Help,
    Filter,
    Confirm(ConfirmState),
    CreateCard,
    Detail,
    RepoSelect(RepoSelectState),
    CardGrab(GrabState),
    EditCard,
    CommentList(CommentListState),
    GroupBySelect(GroupBySelectState),
    ReactionPicker(ReactionPickerState),
    ArchivedList(ArchivedListState),
}

/// Board の表示レイアウト。Kanban (Board) / Table / Roadmap の 3 種類をサポート。
/// `ViewMode::Board` のサブモードとして扱い、Detail/Filter/CardGrab 等の既存モーダルは
/// どのレイアウトからも開ける。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LayoutMode {
    #[default]
    Board,
    Table,
    Roadmap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailPane {
    Content,
    Sidebar,
}

/// サイドバーの固定セクションインデックス (0..4)。4 以降はカスタムフィールド、末尾は Delete。
pub const SIDEBAR_STATUS: usize = 0;
pub const SIDEBAR_ASSIGNEES: usize = 1;
pub const SIDEBAR_LABELS: usize = 2;
pub const SIDEBAR_MILESTONE: usize = 3;

/// 詳細ビューサイドバーの論理セクション。
/// インデックスは `AppState::sidebar_sections()` で動的に解決される。
#[derive(Clone, Debug, PartialEq)]
pub enum SidebarSection {
    Status,
    Assignees,
    Labels,
    Milestone,
    CustomField(usize),
    Parent,
    SubIssue(usize),
    Archive,
}

#[derive(Clone, Debug)]
pub enum SidebarEditMode {
    Labels {
        items: Vec<EditItem>,
        cursor: usize,
    },
    Assignees {
        items: Vec<EditItem>,
        cursor: usize,
    },
    CustomFieldSingleSelect {
        field_id: String,
        field_name: String,
        options: Vec<super::project::SingleSelectOption>,
        cursor: usize,
    },
    CustomFieldIteration {
        field_id: String,
        field_name: String,
        iterations: Vec<super::project::IterationOption>,
        cursor: usize,
    },
    CustomFieldText {
        field_id: String,
        field_name: String,
        input: String,
        cursor_pos: usize,
    },
    CustomFieldNumber {
        field_id: String,
        field_name: String,
        input: String,
        cursor_pos: usize,
    },
    CustomFieldDate {
        field_id: String,
        field_name: String,
        input: String,
        cursor_pos: usize,
    },
}

#[derive(Clone, Debug)]
pub struct EditItem {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub applied: bool,
}

#[derive(Clone, Debug)]
pub struct ConfirmState {
    pub action: ConfirmAction,
    pub title: String,
    pub return_to: ViewMode,
}

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    ArchiveCard { item_id: String },
}

#[derive(Clone, Debug, Default)]
pub struct ArchivedListState {
    pub cards: Vec<super::project::Card>,
    pub selected: usize,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CreateCardState {
    pub card_type: NewCardType,
    pub title_input: String,
    pub title_cursor: usize,
    pub body_input: String,
    pub focused_field: CreateCardField,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateCardField {
    Type,
    Title,
    Body,
    Submit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewCardType {
    Draft,
    Issue,
}

#[derive(Clone, Debug)]
pub struct EditCardState {
    pub content_id: String,
    pub item_id: String,
    pub card_type: super::project::CardType,
    pub title_input: String,
    pub title_cursor: usize,
    pub body_input: String,
    pub focused_field: EditCardField,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditCardField {
    Title,
    Body,
}

impl Default for CreateCardState {
    fn default() -> Self {
        Self {
            card_type: NewCardType::Draft,
            title_input: String::new(),
            title_cursor: 0,
            body_input: String::new(),
            focused_field: CreateCardField::Type,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommentListState {
    pub cursor: usize,
    pub content_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupBySelectState {
    pub cursor: usize,
    pub candidates: Vec<super::project::Grouping>,
}

#[derive(Clone, Debug)]
pub struct ReactionPickerState {
    pub target: ReactionTarget,
    pub cursor: usize,
    pub return_to: ViewMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReactionTarget {
    /// Issue/PR 本体のリアクション (subject_id = content_id)
    CardBody { content_id: String },
    /// コメントのリアクション (subject_id = comment_id)
    Comment {
        comment_id: String,
        content_id: String,
    },
}

#[derive(Clone, Debug)]
pub struct RepoSelectState {
    pub selected_index: usize,
    pub pending_create: PendingIssueCreate,
}

#[derive(Clone, Debug)]
pub struct PendingIssueCreate {
    pub title: String,
    pub body: String,
    pub initial_status: Option<super::super::command::InitialStatus>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoadingState {
    Idle,
    Loading(String),
    /// 既存ボードを表示したままバックグラウンドで再フェッチ中。オーバーレイは出さない。
    Refreshing,
    Error(String),
}

#[derive(Clone, Debug)]
#[derive(Default)]
pub struct FilterState {
    pub input: String,
    pub cursor_pos: usize,
    pub active_filter: Option<ActiveFilter>,
}


#[derive(Clone, Debug, PartialEq)]
pub enum FilterCondition {
    Text(String),
    Label(String),
    Assignee(String),
    Milestone(String),
}

impl FilterCondition {
    pub fn parse_token(token: &str) -> Self {
        if let Some(rest) = token.strip_prefix("label:") {
            FilterCondition::Label(rest.to_string())
        } else if let Some(rest) = token.strip_prefix("assignee:") {
            FilterCondition::Assignee(rest.to_string())
        } else if let Some(rest) = token.strip_prefix("milestone:") {
            FilterCondition::Milestone(rest.to_string())
        } else {
            FilterCondition::Text(token.to_string())
        }
    }

    /// Projects V2 の query 構文に変換する。
    /// 値に空白が含まれる場合はダブルクオートで囲む。
    pub fn to_query_token(&self) -> String {
        match self {
            FilterCondition::Text(s) => quote_if_needed(s),
            FilterCondition::Label(s) => format!("label:\"{s}\""),
            FilterCondition::Assignee(s) => {
                let stripped = s.strip_prefix('@').unwrap_or(s);
                format!("assignee:{}", quote_if_needed(stripped))
            }
            FilterCondition::Milestone(s) => format!("milestone:\"{s}\""),
        }
    }

    pub fn matches(&self, card: &super::project::Card) -> bool {
        match self {
            FilterCondition::Text(query) => {
                let q = query.to_lowercase();
                card.title.to_lowercase().contains(&q)
            }
            FilterCondition::Label(query) => {
                let q = query.to_lowercase();
                card.labels
                    .iter()
                    .any(|l| l.name.to_lowercase().contains(&q))
            }
            FilterCondition::Assignee(query) => {
                let q = query.strip_prefix('@').unwrap_or(query).to_lowercase();
                card.assignees
                    .iter()
                    .any(|a| a.to_lowercase().contains(&q))
            }
            FilterCondition::Milestone(query) => {
                let q = query.to_lowercase();
                card.milestone
                    .as_ref()
                    .is_some_and(|m| m.to_lowercase().contains(&q))
            }
        }
    }
}

/// 複合フィルタ: groups は OR 結合、各 group 内の条件は AND 結合
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveFilter {
    pub groups: Vec<Vec<FilterCondition>>,
}

impl ActiveFilter {
    pub fn parse(input: &str) -> Self {
        let groups: Vec<Vec<FilterCondition>> = input
            .split('|')
            .map(|group| {
                group
                    .split_whitespace()
                    .map(FilterCondition::parse_token)
                    .collect()
            })
            .filter(|g: &Vec<FilterCondition>| !g.is_empty())
            .collect();
        ActiveFilter { groups }
    }

    pub fn matches(&self, card: &super::project::Card) -> bool {
        if self.groups.is_empty() {
            return true;
        }
        // OR of ANDs: いずれかのグループの全条件がマッチすればOK
        self.groups
            .iter()
            .any(|group| group.iter().all(|cond| cond.matches(card)))
    }

    /// Projects V2 の `items(query:)` 引数に渡す query 文字列を、
    /// OR グループごとに 1 個ずつ生成する。
    /// 空 vec の場合はサーバー側フィルタなし。
    pub fn to_server_queries(&self) -> Vec<String> {
        self.groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(FilterCondition::to_query_token)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }
}

fn quote_if_needed(s: &str) -> String {
    if s.contains(char::is_whitespace) {
        format!("\"{s}\"")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_text() {
        let f = ActiveFilter::parse("fix");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![vec![FilterCondition::Text("fix".into())]]
            }
        );
    }

    #[test]
    fn test_parse_single_label() {
        let f = ActiveFilter::parse("label:bug");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![vec![FilterCondition::Label("bug".into())]]
            }
        );
    }

    #[test]
    fn test_parse_single_assignee() {
        let f = ActiveFilter::parse("assignee:alice");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![vec![FilterCondition::Assignee("alice".into())]]
            }
        );
    }

    #[test]
    fn test_parse_single_milestone() {
        let f = ActiveFilter::parse("milestone:v1.0");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![vec![FilterCondition::Milestone("v1.0".into())]]
            }
        );
    }

    #[test]
    fn test_parse_and_combination() {
        let f = ActiveFilter::parse("label:bug assignee:alice");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![vec![
                    FilterCondition::Label("bug".into()),
                    FilterCondition::Assignee("alice".into()),
                ]]
            }
        );
    }

    #[test]
    fn test_parse_or_combination() {
        let f = ActiveFilter::parse("label:bug | label:enhancement");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![
                    vec![FilterCondition::Label("bug".into())],
                    vec![FilterCondition::Label("enhancement".into())],
                ]
            }
        );
    }

    #[test]
    fn test_parse_complex() {
        let f = ActiveFilter::parse("label:bug assignee:alice | label:enhancement");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![
                    vec![
                        FilterCondition::Label("bug".into()),
                        FilterCondition::Assignee("alice".into()),
                    ],
                    vec![FilterCondition::Label("enhancement".into())],
                ]
            }
        );
    }

    #[test]
    fn test_parse_empty() {
        let f = ActiveFilter::parse("");
        assert_eq!(f, ActiveFilter { groups: vec![] });
    }

    #[test]
    fn test_parse_only_pipe() {
        let f = ActiveFilter::parse("|");
        assert_eq!(f, ActiveFilter { groups: vec![] });
    }

    #[test]
    fn test_to_query_token_text() {
        assert_eq!(
            FilterCondition::Text("fix".into()).to_query_token(),
            "fix"
        );
    }

    #[test]
    fn test_to_query_token_text_with_space() {
        assert_eq!(
            FilterCondition::Text("hello world".into()).to_query_token(),
            "\"hello world\""
        );
    }

    #[test]
    fn test_to_query_token_label() {
        assert_eq!(
            FilterCondition::Label("bug".into()).to_query_token(),
            "label:\"bug\""
        );
    }

    #[test]
    fn test_to_query_token_assignee() {
        assert_eq!(
            FilterCondition::Assignee("alice".into()).to_query_token(),
            "assignee:alice"
        );
    }

    #[test]
    fn test_to_query_token_assignee_strips_at_prefix() {
        assert_eq!(
            FilterCondition::Assignee("@alice".into()).to_query_token(),
            "assignee:alice"
        );
    }

    #[test]
    fn test_to_query_token_milestone() {
        assert_eq!(
            FilterCondition::Milestone("v1.0".into()).to_query_token(),
            "milestone:\"v1.0\""
        );
    }

    #[test]
    fn test_to_server_queries_empty() {
        let f = ActiveFilter::parse("");
        assert_eq!(f.to_server_queries(), Vec::<String>::new());
    }

    #[test]
    fn test_to_server_queries_single_group() {
        let f = ActiveFilter::parse("label:bug assignee:alice");
        assert_eq!(
            f.to_server_queries(),
            vec!["label:\"bug\" assignee:alice".to_string()]
        );
    }

    #[test]
    fn test_to_server_queries_or_groups() {
        let f = ActiveFilter::parse("label:bug | label:enhancement");
        assert_eq!(
            f.to_server_queries(),
            vec![
                "label:\"bug\"".to_string(),
                "label:\"enhancement\"".to_string(),
            ]
        );
    }

    #[test]
    fn test_to_server_queries_complex() {
        let f = ActiveFilter::parse("label:bug assignee:alice | label:enhancement");
        assert_eq!(
            f.to_server_queries(),
            vec![
                "label:\"bug\" assignee:alice".to_string(),
                "label:\"enhancement\"".to_string(),
            ]
        );
    }
}
