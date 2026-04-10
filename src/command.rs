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
    OpenUrl(String),
    Batch(Vec<Command>),
}
