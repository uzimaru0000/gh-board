#[derive(Debug, PartialEq)]
pub enum Command {
    None,
    LoadProjects {
        owner: Option<String>,
    },
    LoadProjectByNumber {
        owner: Option<String>,
        number: i32,
    },
    LoadBoard {
        project_id: String,
        preferred_grouping_field_name: Option<String>,
        /// サーバーサイドフィルタ用の Projects V2 query 文字列。
        /// 空 vec の場合はフィルタなし。複数の場合は OR として各クエリを実行しマージ。
        queries: Vec<String>,
    },
    MoveCard {
        project_id: String,
        item_id: String,
        field_id: String,
        value: CustomFieldValueInput,
    },
    ArchiveCard {
        project_id: String,
        item_id: String,
    },
    UnarchiveCard {
        project_id: String,
        item_id: String,
    },
    LoadArchivedItems {
        project_id: String,
    },
    CreateCard {
        project_id: String,
        title: String,
        body: String,
        /// 作成後に設定する初期フィールド値 (SingleSelect 限定。Iteration や None 軸では None)
        initial_status: Option<InitialStatus>,
    },
    CreateIssue {
        project_id: String,
        repository_id: String,
        title: String,
        body: String,
        initial_status: Option<InitialStatus>,
    },
    OpenEditor {
        content: String,
    },
    ReorderCard {
        project_id: String,
        item_id: String,
        after_id: Option<String>,
    },
    FetchLabels {
        owner: String,
        repo: String,
    },
    FetchAssignees {
        owner: String,
        repo: String,
    },
    ToggleLabel {
        content_id: String,
        label_id: String,
        add: bool,
    },
    ToggleAssignee {
        content_id: String,
        user_id: String,
        add: bool,
    },
    UpdateCard {
        content_id: String,
        card_type: crate::model::project::CardType,
        title: String,
        body: String,
    },
    AddComment {
        subject_id: String,
        body: String,
    },
    UpdateComment {
        comment_id: String,
        body: String,
    },
    FetchComments {
        content_id: String,
    },
    FetchSubIssues {
        item_id: String,
        content_id: String,
    },
    /// Parent / Sub-issue の Issue 詳細を取得 (detail_stack に積み上げて表示)
    FetchIssueDetail {
        content_id: String,
    },
    OpenEditorForComment {
        content_id: String,
        existing: Option<(String, String)>,
    },
    AddReaction {
        subject_id: String,
        content: crate::model::project::ReactionContent,
    },
    RemoveReaction {
        subject_id: String,
        content: crate::model::project::ReactionContent,
    },
    OpenUrl(String),
    UpdateCustomField {
        project_id: String,
        item_id: String,
        field_id: String,
        value: CustomFieldValueInput,
    },
    ClearCustomField {
        project_id: String,
        item_id: String,
        field_id: String,
    },
    Batch(Vec<Command>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum CustomFieldValueInput {
    SingleSelect { option_id: String },
    Iteration { iteration_id: String },
    Text { text: String },
    Number { number: f64 },
    Date { date: String },
}

#[derive(Debug, PartialEq, Clone)]
pub struct InitialStatus {
    pub field_id: String,
    pub option_id: String,
}
