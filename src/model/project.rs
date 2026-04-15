#[derive(Clone, Debug)]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub number: i32,
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Board {
    pub project_title: String,
    pub grouping: Grouping,
    pub columns: Vec<Column>,
    pub repositories: Vec<Repository>,
    pub field_definitions: Vec<FieldDefinition>,
}

/// カンバンをグルーピングする軸 (SingleSelect or Iteration)。
/// grouping が未決定のプロジェクト (groupable field がない) では None と同等の扱いで空 columns を返す。
#[derive(Clone, Debug, PartialEq)]
pub enum Grouping {
    SingleSelect { field_id: String, field_name: String },
    Iteration { field_id: String, field_name: String },
    /// Status 相当の field が見つからない場合
    None,
}

impl Grouping {
    pub fn field_id(&self) -> Option<&str> {
        match self {
            Grouping::SingleSelect { field_id, .. } | Grouping::Iteration { field_id, .. } => {
                Some(field_id)
            }
            Grouping::None => None,
        }
    }

    pub fn field_name(&self) -> Option<&str> {
        match self {
            Grouping::SingleSelect { field_name, .. } | Grouping::Iteration { field_name, .. } => {
                Some(field_name)
            }
            Grouping::None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Column {
    pub option_id: String,
    pub name: String,
    pub color: Option<ColumnColor>,
    pub cards: Vec<Card>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColumnColor {
    Blue,
    Gray,
    Green,
    Orange,
    Pink,
    Purple,
    Red,
    Yellow,
}

#[derive(Clone, Debug)]
pub struct Card {
    pub item_id: String,
    pub content_id: Option<String>,
    pub title: String,
    pub number: Option<i32>,
    pub card_type: CardType,
    pub assignees: Vec<String>,
    pub labels: Vec<Label>,
    pub url: Option<String>,
    pub body: Option<String>,
    pub comments: Vec<Comment>,
    pub milestone: Option<String>,
    pub custom_fields: Vec<CustomFieldValue>,
    pub pr_status: Option<PrStatus>,
    pub linked_prs: Vec<LinkedPr>,
    pub reactions: Vec<ReactionSummary>,
    pub archived: bool,
    pub parent_issue: Option<ParentIssueRef>,
    pub sub_issues_summary: Option<SubIssuesSummary>,
    pub sub_issues: Vec<SubIssueRef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParentIssueRef {
    pub id: String,
    pub number: i32,
    pub title: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubIssueRef {
    pub id: String,
    pub number: i32,
    pub title: String,
    pub state: IssueState,
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubIssuesSummary {
    pub completed: i32,
    pub total: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkedPr {
    pub number: i32,
    pub title: String,
    pub url: String,
    pub state: PrState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrStatus {
    pub ci: Option<CiStatus>,
    pub review_decision: Option<ReviewDecision>,
    pub review_requests: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CiStatus {
    Success,
    Failure,
    Pending,
    Error,
    Expected,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    ReviewRequired,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldDefinition {
    SingleSelect {
        id: String,
        name: String,
        options: Vec<SingleSelectOption>,
    },
    Text {
        id: String,
        name: String,
    },
    Number {
        id: String,
        name: String,
    },
    Date {
        id: String,
        name: String,
    },
    Iteration {
        id: String,
        name: String,
        iterations: Vec<IterationOption>,
    },
}

impl FieldDefinition {
    pub fn id(&self) -> &str {
        match self {
            FieldDefinition::SingleSelect { id, .. }
            | FieldDefinition::Text { id, .. }
            | FieldDefinition::Number { id, .. }
            | FieldDefinition::Date { id, .. }
            | FieldDefinition::Iteration { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            FieldDefinition::SingleSelect { name, .. }
            | FieldDefinition::Text { name, .. }
            | FieldDefinition::Number { name, .. }
            | FieldDefinition::Date { name, .. }
            | FieldDefinition::Iteration { name, .. } => name,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SingleSelectOption {
    pub id: String,
    pub name: String,
    pub color: Option<ColumnColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IterationOption {
    pub id: String,
    pub title: String,
    pub start_date: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CustomFieldValue {
    SingleSelect {
        field_id: String,
        option_id: String,
        name: String,
        color: Option<ColumnColor>,
    },
    Text {
        field_id: String,
        text: String,
    },
    Number {
        field_id: String,
        number: f64,
    },
    Date {
        field_id: String,
        date: String,
    },
    Iteration {
        field_id: String,
        iteration_id: String,
        title: String,
    },
}

impl CustomFieldValue {
    pub fn field_id(&self) -> &str {
        match self {
            CustomFieldValue::SingleSelect { field_id, .. }
            | CustomFieldValue::Text { field_id, .. }
            | CustomFieldValue::Number { field_id, .. }
            | CustomFieldValue::Date { field_id, .. }
            | CustomFieldValue::Iteration { field_id, .. } => field_id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub reactions: Vec<ReactionSummary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReactionContent {
    ThumbsUp,
    ThumbsDown,
    Laugh,
    Hooray,
    Confused,
    Heart,
    Rocket,
    Eyes,
}

impl ReactionContent {
    pub fn emoji(self) -> &'static str {
        match self {
            ReactionContent::ThumbsUp => "👍",
            ReactionContent::ThumbsDown => "👎",
            ReactionContent::Laugh => "😄",
            ReactionContent::Hooray => "🎉",
            ReactionContent::Confused => "😕",
            ReactionContent::Heart => "❤️",
            ReactionContent::Rocket => "🚀",
            ReactionContent::Eyes => "👀",
        }
    }

    pub fn all() -> [ReactionContent; 8] {
        [
            ReactionContent::ThumbsUp,
            ReactionContent::ThumbsDown,
            ReactionContent::Laugh,
            ReactionContent::Hooray,
            ReactionContent::Confused,
            ReactionContent::Heart,
            ReactionContent::Rocket,
            ReactionContent::Eyes,
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReactionSummary {
    pub content: ReactionContent,
    pub count: usize,
    pub viewer_has_reacted: bool,
}

/// `reactions` の `content` に対応するエントリをトグルし、新しい viewer_has_reacted の値を返す。
/// 追加後: true、削除後: false。
pub fn apply_reaction_toggle(
    reactions: &mut Vec<ReactionSummary>,
    content: ReactionContent,
) -> bool {
    if let Some(pos) = reactions.iter().position(|r| r.content == content) {
        let entry = &mut reactions[pos];
        if entry.viewer_has_reacted {
            entry.viewer_has_reacted = false;
            entry.count = entry.count.saturating_sub(1);
            if entry.count == 0 {
                reactions.remove(pos);
            }
            false
        } else {
            entry.viewer_has_reacted = true;
            entry.count += 1;
            true
        }
    } else {
        reactions.push(ReactionSummary {
            content,
            count: 1,
            viewer_has_reacted: true,
        });
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CardType {
    Issue { state: IssueState },
    PullRequest { state: PrState },
    DraftIssue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Clone, Debug)]
pub struct Repository {
    pub id: String,
    pub name_with_owner: String,
}
