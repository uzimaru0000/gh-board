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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailPane {
    Content,
    Sidebar,
}

/// サイドバーのセクション数
pub const SIDEBAR_SECTION_COUNT: usize = 4;
/// サイドバーセクションのインデックス
pub const SIDEBAR_STATUS: usize = 0;
pub const SIDEBAR_ASSIGNEES: usize = 1;
pub const SIDEBAR_LABELS: usize = 2;
pub const SIDEBAR_DELETE: usize = 3;

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
pub struct RepoSelectState {
    pub selected_index: usize,
    pub pending_create: PendingIssueCreate,
}

#[derive(Clone, Debug)]
pub struct PendingIssueCreate {
    pub title: String,
    pub body: String,
    pub field_id: String,
    pub option_id: String,
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
