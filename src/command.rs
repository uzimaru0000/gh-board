#[derive(Debug, PartialEq)]
pub enum Command {
    None,
    LoadProjects {
        owner: Option<String>,
    },
    LoadBoard {
        project_id: String,
    },
    MoveCard {
        project_id: String,
        item_id: String,
        field_id: String,
        option_id: String,
    },
    DeleteCard {
        project_id: String,
        item_id: String,
    },
    CreateCard {
        project_id: String,
        title: String,
        body: String,
        field_id: String,
        option_id: String,
    },
    CreateIssue {
        project_id: String,
        repository_id: String,
        title: String,
        body: String,
        field_id: String,
        option_id: String,
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
    OpenUrl(String),
    Batch(Vec<Command>),
}
