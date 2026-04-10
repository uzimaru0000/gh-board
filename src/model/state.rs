#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Board,
    ProjectSelect,
    Help,
    Filter,
    Confirm,
    CreateCard,
    Detail,
}

#[derive(Clone, Debug)]
pub struct ConfirmState {
    pub action: ConfirmAction,
    pub title: String,
}

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    DeleteCard { item_id: String },
}

#[derive(Clone, Debug)]
pub struct CreateCardState {
    pub title_input: String,
    pub title_cursor: usize,
    pub body_input: String,
    pub body_cursor: usize,
    pub focused_field: CreateCardField,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateCardField {
    Title,
    Body,
}

impl Default for CreateCardState {
    fn default() -> Self {
        Self {
            title_input: String::new(),
            title_cursor: 0,
            body_input: String::new(),
            body_cursor: 0,
            focused_field: CreateCardField::Title,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LoadingState {
    Idle,
    Loading(String),
    Error(String),
}

#[derive(Clone, Debug)]
pub struct FilterState {
    pub input: String,
    pub cursor_pos: usize,
    pub active_filter: Option<ActiveFilter>,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            active_filter: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ActiveFilter {
    Text(String),
    Label(String),
    Assignee(String),
}

impl ActiveFilter {
    pub fn parse(input: &str) -> Self {
        if let Some(rest) = input.strip_prefix("label:") {
            ActiveFilter::Label(rest.to_string())
        } else if let Some(rest) = input.strip_prefix("assignee:") {
            ActiveFilter::Assignee(rest.to_string())
        } else {
            ActiveFilter::Text(input.to_string())
        }
    }

    pub fn matches(&self, card: &super::project::Card) -> bool {
        match self {
            ActiveFilter::Text(query) => {
                let q = query.to_lowercase();
                card.title.to_lowercase().contains(&q)
            }
            ActiveFilter::Label(query) => {
                let q = query.to_lowercase();
                card.labels.iter().any(|l| l.name.to_lowercase().contains(&q))
            }
            ActiveFilter::Assignee(query) => {
                let q = query.strip_prefix('@').unwrap_or(query).to_lowercase();
                card.assignees.iter().any(|a| a.to_lowercase().contains(&q))
            }
        }
    }
}
