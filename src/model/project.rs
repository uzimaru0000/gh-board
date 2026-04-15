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
    pub status_field_id: String,
    pub columns: Vec<Column>,
    pub repositories: Vec<Repository>,
    pub field_definitions: Vec<FieldDefinition>,
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
