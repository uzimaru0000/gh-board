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
    OpenUrl(String),
    Batch(Vec<Command>),
}
