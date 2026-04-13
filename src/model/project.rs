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
}

#[derive(Clone, Debug)]
pub struct Column {
    pub option_id: String,
    pub name: String,
    pub cards: Vec<Card>,
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
