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
    DeleteCard { item_id: String },
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

#[derive(Clone, Debug)]
pub struct GroupBySelectState {
    pub cursor: usize,
    pub candidates: Vec<super::project::Grouping>,
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

#[derive(Clone, Debug)]
pub enum LoadingState {
    Idle,
    Loading(String),
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
}
