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
    BulkSelect,
    CommandPalette,
}

/// 現在表示中の「シーン」。Phase B リファクタで段階的にモード固有の state を
/// このバリアントに取り込み、`Option<FooState>` 幽霊状態をコンパイル時に排除する。
///
/// 移行済みバリアント (state を自前で保持):
/// - `ReactionPicker(ReactionPickerState)`
///
/// 未移行バリアントは現状 `mode: ViewMode` + `Option<FooState>` と共存する
/// 単なるタグで、`AppState::sync_scene_from_mode()` で `mode` から派生させる。
// Scene バリアントが持つ state には PartialEq が未実装なので、Scene 全体でも
// PartialEq/Eq は derive しない。Scene の比較が必要な場合は `matches!` や
// パターンマッチで明示的に扱う。
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
    EditCard(EditCardState),
    CommentList(CommentListState),
    GroupBySelect(GroupBySelectState),
    ReactionPicker(ReactionPickerState),
    BulkSelect,
    CommandPalette,
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
    ArchiveMultipleCards { item_ids: Vec<String> },
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
    pub action: RepoSelectAction,
}

#[derive(Clone, Debug)]
pub enum RepoSelectAction {
    /// CreateCard モーダル経由の新規 Issue 作成
    CreateIssue(PendingIssueCreate),
    /// Draft Issue を Issue にコンバート (convertProjectV2DraftIssueItemToIssue)。
    /// `title` はモーダルに「Convert "<title>" → Issue」と表示するためだけに保持する。
    ConvertDraft { item_id: String, title: String },
}

#[derive(Clone, Debug)]
pub struct PendingIssueCreate {
    pub title: String,
    pub body: String,
    pub initial_status: Option<super::super::command::InitialStatus>,
    pub initial_label_names: Vec<String>,
    pub initial_assignee_logins: Vec<String>,
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
    /// `repo:<owner>/<name>` - card URL から導出した `owner/name` に部分一致
    Repo(String),
    /// `no:<field>` - 指定フィールドが未設定なカードにマッチ。
    /// field は label / assignee(s) / milestone / custom field 名 (Status など)
    No(String),
    /// `is:<kind>` - カードの種別や状態でマッチ
    Is(IsKind),
    /// `-<cond>` - 条件の否定
    Not(Box<FilterCondition>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IsKind {
    Open,
    Closed,
    Merged,
    Issue,
    Pr,
    Draft,
}

impl IsKind {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "open" => Some(IsKind::Open),
            "closed" => Some(IsKind::Closed),
            "merged" => Some(IsKind::Merged),
            "issue" => Some(IsKind::Issue),
            "pr" | "pull-request" | "pullrequest" => Some(IsKind::Pr),
            "draft" | "draft-issue" => Some(IsKind::Draft),
            _ => None,
        }
    }

    fn as_query_value(&self) -> &'static str {
        match self {
            IsKind::Open => "open",
            IsKind::Closed => "closed",
            IsKind::Merged => "merged",
            IsKind::Issue => "issue",
            IsKind::Pr => "pr",
            IsKind::Draft => "draft-issue",
        }
    }
}

impl FilterCondition {
    pub fn parse_token(token: &str) -> Self {
        // `-<cond>` は否定。`--` や裸の `-` は通常テキスト扱い。
        if let Some(rest) = token.strip_prefix('-')
            && !rest.is_empty()
            && !rest.starts_with('-')
        {
            return FilterCondition::Not(Box::new(FilterCondition::parse_token(rest)));
        }
        if let Some(rest) = token.strip_prefix("label:") {
            FilterCondition::Label(rest.to_string())
        } else if let Some(rest) = token.strip_prefix("assignee:") {
            FilterCondition::Assignee(rest.to_string())
        } else if let Some(rest) = token.strip_prefix("milestone:") {
            FilterCondition::Milestone(rest.to_string())
        } else if let Some(rest) = token.strip_prefix("repo:") {
            FilterCondition::Repo(rest.to_string())
        } else if let Some(rest) = token.strip_prefix("no:") {
            FilterCondition::No(rest.to_string())
        } else if let Some(rest) = token.strip_prefix("is:") {
            match IsKind::parse(rest) {
                Some(kind) => FilterCondition::Is(kind),
                None => FilterCondition::Text(token.to_string()),
            }
        } else {
            FilterCondition::Text(token.to_string())
        }
    }

    /// Projects V2 の query 構文に変換する。
    /// 値に空白が含まれる場合はダブルクオートで囲む。
    /// サーバー側で表現できない条件 (client-side でのみ絞る条件) は None。
    pub fn to_query_token(&self) -> Option<String> {
        let token = match self {
            FilterCondition::Text(s) => quote_if_needed(s),
            FilterCondition::Label(s) => format!("label:\"{s}\""),
            FilterCondition::Assignee(s) => {
                let stripped = s.strip_prefix('@').unwrap_or(s);
                format!("assignee:{}", quote_if_needed(stripped))
            }
            FilterCondition::Milestone(s) => format!("milestone:\"{s}\""),
            // サーバー側の repo: は `owner/name` の完全指定しか受け付けないため、
            // name だけの指定は client-side の部分一致に任せる
            FilterCondition::Repo(s) if s.contains('/') => format!("repo:{}", quote_if_needed(s)),
            FilterCondition::Repo(_) => return None,
            FilterCondition::No(s) => format!("no:{}", quote_if_needed(s)),
            FilterCondition::Is(kind) => format!("is:{}", kind.as_query_value()),
            FilterCondition::Not(inner) => format!("-{}", inner.to_query_token()?),
        };
        Some(token)
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
            FilterCondition::Repo(query) => {
                let q = query.to_lowercase();
                card.url
                    .as_deref()
                    .and_then(super::project::parse_owner_repo_from_url)
                    .is_some_and(|(owner, repo)| {
                        format!("{owner}/{repo}").to_lowercase().contains(&q)
                    })
            }
            FilterCondition::No(field) => match field.to_ascii_lowercase().as_str() {
                "label" | "labels" => card.labels.is_empty(),
                "assignee" | "assignees" => card.assignees.is_empty(),
                "milestone" => card.milestone.is_none(),
                other => !card
                    .custom_fields
                    .iter()
                    .any(|cf| field_name_of(cf).eq_ignore_ascii_case(other)),
            },
            FilterCondition::Is(kind) => match kind {
                IsKind::Open => matches!(
                    card.card_type,
                    super::project::CardType::Issue {
                        state: super::project::IssueState::Open
                    } | super::project::CardType::PullRequest {
                        state: super::project::PrState::Open
                    }
                ),
                IsKind::Closed => matches!(
                    card.card_type,
                    super::project::CardType::Issue {
                        state: super::project::IssueState::Closed
                    } | super::project::CardType::PullRequest {
                        state: super::project::PrState::Closed
                            | super::project::PrState::Merged
                    }
                ),
                IsKind::Merged => matches!(
                    card.card_type,
                    super::project::CardType::PullRequest {
                        state: super::project::PrState::Merged
                    }
                ),
                IsKind::Issue => {
                    matches!(card.card_type, super::project::CardType::Issue { .. })
                }
                IsKind::Pr => matches!(
                    card.card_type,
                    super::project::CardType::PullRequest { .. }
                ),
                IsKind::Draft => {
                    matches!(card.card_type, super::project::CardType::DraftIssue)
                }
            },
            FilterCondition::Not(inner) => !inner.matches(card),
        }
    }
}

fn field_name_of(cf: &super::project::CustomFieldValue) -> &str {
    use super::project::CustomFieldValue::{Date, Iteration, Number, SingleSelect, Text};
    match cf {
        SingleSelect { field_name, .. }
        | Text { field_name, .. }
        | Number { field_name, .. }
        | Date { field_name, .. }
        | Iteration { field_name, .. } => field_name,
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
    /// サーバーで表現できない条件は落とす (client-side の `matches` が最終的に絞る)。
    /// いずれかの group がサーバー側無条件になる場合は全件取得と同じなので空 vec を返す。
    pub fn to_server_queries(&self) -> Vec<String> {
        let queries: Vec<String> = self
            .groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .filter_map(FilterCondition::to_query_token)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        if queries.iter().any(String::is_empty) {
            return Vec::new();
        }
        queries
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
            FilterCondition::Text("fix".into())
                .to_query_token()
                .as_deref(),
            Some("fix")
        );
    }

    #[test]
    fn test_to_query_token_text_with_space() {
        assert_eq!(
            FilterCondition::Text("hello world".into())
                .to_query_token()
                .as_deref(),
            Some("\"hello world\"")
        );
    }

    #[test]
    fn test_to_query_token_label() {
        assert_eq!(
            FilterCondition::Label("bug".into())
                .to_query_token()
                .as_deref(),
            Some("label:\"bug\"")
        );
    }

    #[test]
    fn test_to_query_token_assignee() {
        assert_eq!(
            FilterCondition::Assignee("alice".into())
                .to_query_token()
                .as_deref(),
            Some("assignee:alice")
        );
    }

    #[test]
    fn test_to_query_token_assignee_strips_at_prefix() {
        assert_eq!(
            FilterCondition::Assignee("@alice".into())
                .to_query_token()
                .as_deref(),
            Some("assignee:alice")
        );
    }

    #[test]
    fn test_parse_single_repo() {
        let f = ActiveFilter::parse("repo:acme/widget");
        assert_eq!(
            f,
            ActiveFilter {
                groups: vec![vec![FilterCondition::Repo("acme/widget".into())]]
            }
        );
    }

    #[test]
    fn test_parse_negated_repo() {
        assert_eq!(
            FilterCondition::parse_token("-repo:acme/widget"),
            FilterCondition::Not(Box::new(FilterCondition::Repo("acme/widget".into())))
        );
    }

    #[test]
    fn test_to_query_token_repo() {
        assert_eq!(
            FilterCondition::Repo("acme/widget".into())
                .to_query_token()
                .as_deref(),
            Some("repo:acme/widget")
        );
    }

    #[test]
    fn test_to_query_token_repo_name_only_is_client_side_only() {
        // owner を含まない repo: はサーバー側で解釈できない (完全指定が必要) ので
        // サーバーには渡さず client-side の部分一致だけで絞る
        assert_eq!(
            FilterCondition::Repo("widget".into()).to_query_token(),
            None
        );
        assert_eq!(
            FilterCondition::Not(Box::new(FilterCondition::Repo("widget".into()))).to_query_token(),
            None
        );
    }

    #[test]
    fn test_to_server_queries_repo_name_only_keeps_other_tokens() {
        let f = ActiveFilter::parse("repo:widget label:bug");
        assert_eq!(f.to_server_queries(), vec!["label:\"bug\"".to_string()]);
    }

    #[test]
    fn test_to_server_queries_repo_name_only_alone_means_no_server_filter() {
        assert_eq!(
            ActiveFilter::parse("repo:widget").to_server_queries(),
            Vec::<String>::new()
        );
        // OR のいずれかの group が無条件なら全件取得と同じ
        assert_eq!(
            ActiveFilter::parse("repo:widget | label:bug").to_server_queries(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_to_query_token_milestone() {
        assert_eq!(
            FilterCondition::Milestone("v1.0".into())
                .to_query_token()
                .as_deref(),
            Some("milestone:\"v1.0\"")
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

    // ===== no:/is:/- (Not) プレフィックスの拡張 =====

    use super::super::project::{
        Card, CardType, CustomFieldValue, IssueState, Label as ProjectLabel, PrState,
    };

    fn card_defaults() -> Card {
        Card {
            item_id: "i".into(),
            content_id: None,
            title: "title".into(),
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

    #[test]
    fn test_parse_no_label() {
        assert_eq!(
            FilterCondition::parse_token("no:label"),
            FilterCondition::No("label".into())
        );
    }

    #[test]
    fn test_parse_no_status_custom() {
        assert_eq!(
            FilterCondition::parse_token("no:status"),
            FilterCondition::No("status".into())
        );
    }

    #[test]
    fn test_parse_is_open() {
        assert_eq!(
            FilterCondition::parse_token("is:open"),
            FilterCondition::Is(IsKind::Open)
        );
    }

    #[test]
    fn test_parse_is_pr() {
        assert_eq!(
            FilterCondition::parse_token("is:pr"),
            FilterCondition::Is(IsKind::Pr)
        );
    }

    #[test]
    fn test_parse_is_unknown_fallbacks_to_text() {
        assert_eq!(
            FilterCondition::parse_token("is:foo"),
            FilterCondition::Text("is:foo".into())
        );
    }

    #[test]
    fn test_parse_not_label() {
        assert_eq!(
            FilterCondition::parse_token("-label:bug"),
            FilterCondition::Not(Box::new(FilterCondition::Label("bug".into())))
        );
    }

    #[test]
    fn test_parse_not_no_assignee() {
        assert_eq!(
            FilterCondition::parse_token("-no:assignee"),
            FilterCondition::Not(Box::new(FilterCondition::No("assignee".into())))
        );
    }

    #[test]
    fn test_parse_bare_dash_is_text() {
        assert_eq!(
            FilterCondition::parse_token("-"),
            FilterCondition::Text("-".into())
        );
    }

    #[test]
    fn test_matches_no_label_when_empty() {
        let card = card_defaults();
        assert!(FilterCondition::No("label".into()).matches(&card));
    }

    #[test]
    fn test_matches_no_label_false_when_has_label() {
        let mut card = card_defaults();
        card.labels.push(ProjectLabel {
            id: "x".into(),
            name: "bug".into(),
            color: "000".into(),
        });
        assert!(!FilterCondition::No("label".into()).matches(&card));
    }

    #[test]
    fn test_matches_no_assignee_plural_alias() {
        let card = card_defaults();
        assert!(FilterCondition::No("assignees".into()).matches(&card));
    }

    #[test]
    fn test_matches_no_milestone() {
        let card = card_defaults();
        assert!(FilterCondition::No("milestone".into()).matches(&card));
    }

    #[test]
    fn test_matches_no_custom_field_when_absent() {
        let card = card_defaults();
        assert!(FilterCondition::No("Status".into()).matches(&card));
    }

    #[test]
    fn test_matches_no_custom_field_case_insensitive() {
        let mut card = card_defaults();
        card.custom_fields.push(CustomFieldValue::SingleSelect {
            field_id: "f".into(),
            field_name: "Status".into(),
            option_id: "o".into(),
            name: "Todo".into(),
            color: None,
        });
        // 大文字小文字を無視して一致するので "no:status" では未設定とは判定しない
        assert!(!FilterCondition::No("status".into()).matches(&card));
    }

    #[test]
    fn test_matches_is_open_issue() {
        let mut card = card_defaults();
        card.card_type = CardType::Issue {
            state: IssueState::Open,
        };
        assert!(FilterCondition::Is(IsKind::Open).matches(&card));
        assert!(!FilterCondition::Is(IsKind::Closed).matches(&card));
        assert!(FilterCondition::Is(IsKind::Issue).matches(&card));
        assert!(!FilterCondition::Is(IsKind::Pr).matches(&card));
    }

    #[test]
    fn test_matches_is_closed_merged_pr() {
        let mut card = card_defaults();
        card.card_type = CardType::PullRequest {
            state: PrState::Merged,
        };
        // Merged は closed の一種として扱う (GitHub 検索構文と同様)
        assert!(FilterCondition::Is(IsKind::Closed).matches(&card));
        assert!(FilterCondition::Is(IsKind::Merged).matches(&card));
        assert!(FilterCondition::Is(IsKind::Pr).matches(&card));
        assert!(!FilterCondition::Is(IsKind::Open).matches(&card));
    }

    #[test]
    fn test_matches_is_draft() {
        let card = card_defaults();
        assert!(FilterCondition::Is(IsKind::Draft).matches(&card));
        assert!(!FilterCondition::Is(IsKind::Issue).matches(&card));
    }

    #[test]
    fn test_matches_repo_partial() {
        let mut card = card_defaults();
        card.url = Some("https://github.com/acme/widget/issues/1".into());
        assert!(FilterCondition::Repo("widget".into()).matches(&card));
        assert!(FilterCondition::Repo("ACME/widget".into()).matches(&card));
        assert!(!FilterCondition::Repo("other".into()).matches(&card));

        let no_url = card_defaults();
        assert!(!FilterCondition::Repo("widget".into()).matches(&no_url));
    }

    #[test]
    fn test_matches_not_inverts() {
        let mut card = card_defaults();
        card.labels.push(ProjectLabel {
            id: "x".into(),
            name: "bug".into(),
            color: "000".into(),
        });
        let cond = FilterCondition::Not(Box::new(FilterCondition::Label("bug".into())));
        assert!(!cond.matches(&card));
        let cond2 = FilterCondition::Not(Box::new(FilterCondition::Label("enhancement".into())));
        assert!(cond2.matches(&card));
    }

    #[test]
    fn test_to_query_token_no() {
        assert_eq!(
            FilterCondition::No("label".into())
                .to_query_token()
                .as_deref(),
            Some("no:label")
        );
    }

    #[test]
    fn test_to_query_token_is() {
        assert_eq!(
            FilterCondition::Is(IsKind::Open).to_query_token().as_deref(),
            Some("is:open")
        );
        assert_eq!(
            FilterCondition::Is(IsKind::Draft).to_query_token().as_deref(),
            Some("is:draft-issue")
        );
    }

    #[test]
    fn test_to_query_token_not() {
        assert_eq!(
            FilterCondition::Not(Box::new(FilterCondition::Label("bug".into()))).to_query_token().as_deref(),
            Some("-label:\"bug\"")
        );
    }

    #[test]
    fn test_to_server_queries_with_new_ops() {
        let f = ActiveFilter::parse("no:assignee is:open -label:wontfix");
        assert_eq!(
            f.to_server_queries(),
            vec!["no:assignee is:open -label:\"wontfix\"".to_string()]
        );
    }
}
