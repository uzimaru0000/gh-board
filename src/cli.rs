use anyhow::{bail, Context};
use clap::Subcommand;

use crate::command::CustomFieldValueInput;
use crate::github::client::GitHubClient;
use crate::model::project::ProjectSummary;

#[derive(Subcommand)]
pub enum CliCommand {
    /// List and view projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// View board data
    Board {
        #[command(subcommand)]
        action: BoardAction,
    },
    /// Manage cards on a project board
    Card {
        #[command(subcommand)]
        action: CardAction,
    },
    /// Manage comments on issues/PRs
    Comment {
        #[command(subcommand)]
        action: CommentAction,
    },
    /// List field definitions for a project
    Field {
        #[command(subcommand)]
        action: FieldAction,
    },
    /// List labels for a repository
    Label {
        #[command(subcommand)]
        action: LabelAction,
    },
    /// List assignable users for a repository
    Assignee {
        #[command(subcommand)]
        action: AssigneeAction,
    },
    /// List project items
    Item {
        #[command(subcommand)]
        action: ItemAction,
    },
    /// Output skills.md describing available CLI commands
    Skill,
}

#[derive(Subcommand)]
pub enum ProjectAction {
    /// List projects
    List {
        /// Login of the owner (org or user). Omit for current user.
        #[arg(long)]
        owner: Option<String>,
    },
    /// View project details
    View {
        /// Project number
        number: i32,
        /// Login of the owner
        #[arg(long)]
        owner: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum BoardAction {
    /// Get board as JSON
    View {
        /// Project number
        number: i32,
        /// Login of the owner
        #[arg(long)]
        owner: Option<String>,
        /// Field name to group by (e.g. "Status", "Sprint")
        #[arg(long)]
        group_by: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CardAction {
    /// Create a card on the project (draft issue by default, or a real issue with --type issue)
    Create {
        /// Project number
        number: i32,
        /// Card type: draft or issue
        #[arg(long, name = "type", default_value = "draft")]
        card_type: String,
        /// Card title
        #[arg(long)]
        title: String,
        /// Card body
        #[arg(long, default_value = "")]
        body: String,
        /// Login of the owner
        #[arg(long)]
        owner: Option<String>,
        /// Initial status name (must match a SingleSelect option)
        #[arg(long)]
        status: Option<String>,
        /// Repository in OWNER/REPO format (for --type issue; auto-detected if project has only one repo)
        #[arg(long)]
        repo: Option<String>,
    },
    /// Archive a card
    Archive {
        /// Project node ID
        project_id: String,
        /// Item node ID
        item_id: String,
    },
    /// Move a card (update a field value)
    Move {
        /// Project node ID
        project_id: String,
        /// Item node ID
        item_id: String,
        /// Field node ID
        #[arg(long)]
        field_id: String,
        /// Value to set
        #[arg(long)]
        value: String,
        /// Value type: single_select, iteration, text, number, date
        #[arg(long, default_value = "single_select")]
        value_type: String,
    },
    /// Edit a card (update title/body)
    Edit {
        /// Content node ID (Issue/PR/DraftIssue ID)
        content_id: String,
        /// Card type: draft, issue, pr
        #[arg(long, name = "type")]
        card_type: String,
        /// New title
        #[arg(long)]
        title: String,
        /// New body
        #[arg(long, default_value = "")]
        body: String,
    },
    /// Get card details (Issue only)
    Get {
        /// Content node ID
        content_id: String,
    },
}

#[derive(Subcommand)]
pub enum CommentAction {
    /// List comments on an issue/PR
    List {
        /// Content node ID (Issue or PR)
        content_id: String,
    },
    /// Add a comment
    Add {
        /// Content node ID (Issue or PR)
        content_id: String,
        /// Comment body
        #[arg(long)]
        body: String,
    },
    /// Update a comment
    Update {
        /// Comment node ID
        comment_id: String,
        /// New comment body
        #[arg(long)]
        body: String,
    },
}

#[derive(Subcommand)]
pub enum FieldAction {
    /// List field definitions for a project
    List {
        /// Project number
        number: i32,
        /// Login of the owner
        #[arg(long)]
        owner: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum LabelAction {
    /// List labels for a repository
    List {
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: String,
    },
}

#[derive(Subcommand)]
pub enum AssigneeAction {
    /// List assignable users for a repository
    List {
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: String,
    },
}

#[derive(Subcommand)]
pub enum ItemAction {
    /// List all items in a project (flat list with custom fields)
    List {
        /// Project number
        number: i32,
        /// Login of the owner
        #[arg(long)]
        owner: Option<String>,
    },
}

pub async fn run(cmd: CliCommand, github: GitHubClient) -> anyhow::Result<()> {
    match cmd {
        CliCommand::Project { action } => run_project(action, &github).await,
        CliCommand::Board { action } => run_board(action, &github).await,
        CliCommand::Card { action } => run_card(action, &github).await,
        CliCommand::Comment { action } => run_comment(action, &github).await,
        CliCommand::Field { action } => run_field(action, &github).await,
        CliCommand::Label { action } => run_label(action, &github).await,
        CliCommand::Assignee { action } => run_assignee(action, &github).await,
        CliCommand::Item { action } => run_item(action, &github).await,
        CliCommand::Skill => {
            print!("{}", include_str!(concat!(env!("OUT_DIR"), "/skills.md")));
            Ok(())
        }
    }
}

async fn resolve_project(
    github: &GitHubClient,
    number: i32,
    owner: Option<&str>,
) -> anyhow::Result<ProjectSummary> {
    if let Some(owner) = owner {
        github.get_owner_project_by_number(owner, number).await
    } else {
        github.get_viewer_project_by_number(number).await
    }
}

fn parse_repo(repo: &str) -> anyhow::Result<(&str, &str)> {
    let parts: Vec<&str> = repo.splitn(2, '/').collect();
    if parts.len() != 2 {
        bail!("Repository must be in OWNER/REPO format, got: {repo}");
    }
    Ok((parts[0], parts[1]))
}

fn print_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).context("Failed to serialize JSON")?
    );
    Ok(())
}

async fn run_project(action: ProjectAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        ProjectAction::List { owner } => {
            let projects = if let Some(owner) = owner {
                github.list_owner_projects(&owner).await?
            } else {
                github.list_viewer_projects().await?
            };
            print_json(&projects)
        }
        ProjectAction::View { number, owner } => {
            let project = resolve_project(github, number, owner.as_deref()).await?;
            print_json(&project)
        }
    }
}

async fn run_board(action: BoardAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        BoardAction::View {
            number,
            owner,
            group_by,
        } => {
            let project = resolve_project(github, number, owner.as_deref()).await?;
            let board = github
                .get_board(&project.id, &[], group_by.as_deref())
                .await?;
            print_json(&board)
        }
    }
}

async fn run_card(action: CardAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        CardAction::Create {
            number,
            card_type,
            title,
            body,
            owner,
            status,
            repo,
        } => match card_type.as_str() {
            "draft" => {
                let project = resolve_project(github, number, owner.as_deref()).await?;
                let item_id = github.create_draft_issue(&project.id, &title, &body).await?;
                if let Some(status_name) = status {
                    set_initial_status(github, &project.id, &item_id, &status_name).await?;
                }
                print_json(&serde_json::json!({ "item_id": item_id }))
            }
            "issue" => {
                let project = resolve_project(github, number, owner.as_deref()).await?;
                let board = github.get_board(&project.id, &[], None).await?;
                let repository = if let Some(repo) = &repo {
                    let (repo_owner, repo_name) = parse_repo(repo)?;
                    let _ = (repo_owner, repo_name);
                    board
                        .repositories
                        .iter()
                        .find(|r| r.name_with_owner == *repo)
                        .with_context(|| {
                            format!(
                                "Repository '{repo}' not linked to this project. Available: {}",
                                board
                                    .repositories
                                    .iter()
                                    .map(|r| r.name_with_owner.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })?
                } else if board.repositories.len() == 1 {
                    &board.repositories[0]
                } else if board.repositories.is_empty() {
                    bail!("No repositories linked to this project.")
                } else {
                    bail!(
                        "Multiple repositories linked to this project. Specify one with --repo: {}",
                        board
                            .repositories
                            .iter()
                            .map(|r| r.name_with_owner.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let issue_id = github.create_issue(&repository.id, &title, &body).await?;
                let item_id = github.add_project_item(&project.id, &issue_id).await?;
                if let Some(status_name) = status {
                    set_initial_status(github, &project.id, &item_id, &status_name).await?;
                }
                print_json(&serde_json::json!({
                    "item_id": item_id,
                    "issue_id": issue_id,
                }))
            }
            other => bail!("Unknown card type: '{other}'. Use: draft, issue"),
        },
        CardAction::Archive {
            project_id,
            item_id,
        } => {
            github.archive_card(&project_id, &item_id).await?;
            print_json(&serde_json::json!({ "ok": true }))
        }
        CardAction::Move {
            project_id,
            item_id,
            field_id,
            value,
            value_type,
        } => {
            let input = parse_field_value(&value_type, &value)?;
            github
                .update_custom_field(&project_id, &item_id, &field_id, &input)
                .await?;
            print_json(&serde_json::json!({ "ok": true }))
        }
        CardAction::Edit {
            content_id,
            card_type,
            title,
            body,
        } => {
            match card_type.as_str() {
                "draft" => github.update_draft_issue(&content_id, &title, &body).await?,
                "issue" => github.update_issue(&content_id, &title, &body).await?,
                "pr" => github.update_pull_request(&content_id, &title, &body).await?,
                other => bail!("Unknown card type: '{other}'. Use: draft, issue, pr"),
            }
            print_json(&serde_json::json!({ "ok": true }))
        }
        CardAction::Get { content_id } => {
            let card = github.fetch_issue_as_card(&content_id).await?;
            print_json(&card)
        }
    }
}

async fn run_comment(action: CommentAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        CommentAction::List { content_id } => {
            let comments = github.fetch_all_comments(&content_id).await?;
            print_json(&comments)
        }
        CommentAction::Add { content_id, body } => {
            let comment = github.add_comment(&content_id, &body).await?;
            print_json(&comment)
        }
        CommentAction::Update { comment_id, body } => {
            let comment = github.update_comment(&comment_id, &body).await?;
            print_json(&comment)
        }
    }
}

async fn run_field(action: FieldAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        FieldAction::List { number, owner } => {
            let project = resolve_project(github, number, owner.as_deref()).await?;
            let board = github.get_board(&project.id, &[], None).await?;
            print_json(&board.field_definitions)
        }
    }
}

async fn run_label(action: LabelAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        LabelAction::List { repo } => {
            let (owner, name) = parse_repo(&repo)?;
            let labels = github.get_repo_labels(owner, name).await?;
            print_json(&labels)
        }
    }
}

async fn run_assignee(action: AssigneeAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        AssigneeAction::List { repo } => {
            let (owner, name) = parse_repo(&repo)?;
            let users = github.get_assignable_users(owner, name).await?;
            let users: Vec<_> = users
                .into_iter()
                .map(|(id, login)| serde_json::json!({ "id": id, "login": login }))
                .collect();
            print_json(&users)
        }
    }
}

async fn run_item(action: ItemAction, github: &GitHubClient) -> anyhow::Result<()> {
    match action {
        ItemAction::List { number, owner } => {
            let project = resolve_project(github, number, owner.as_deref()).await?;
            let board = github.get_board(&project.id, &[], None).await?;
            let cards: Vec<_> = board.columns.into_iter().flat_map(|col| col.cards).collect();
            print_json(&cards)
        }
    }
}

fn parse_field_value(value_type: &str, value: &str) -> anyhow::Result<CustomFieldValueInput> {
    match value_type {
        "single_select" => Ok(CustomFieldValueInput::SingleSelect {
            option_id: value.to_string(),
        }),
        "iteration" => Ok(CustomFieldValueInput::Iteration {
            iteration_id: value.to_string(),
        }),
        "text" => Ok(CustomFieldValueInput::Text {
            text: value.to_string(),
        }),
        "number" => {
            let n: f64 = value
                .parse()
                .with_context(|| format!("Invalid number: {value}"))?;
            Ok(CustomFieldValueInput::Number { number: n })
        }
        "date" => Ok(CustomFieldValueInput::Date {
            date: value.to_string(),
        }),
        other => bail!("Unknown value type: '{other}'. Use: single_select, iteration, text, number, date"),
    }
}

/// Resolve a status name to a field value and set it on the item.
async fn set_initial_status(
    github: &GitHubClient,
    project_id: &str,
    item_id: &str,
    status_name: &str,
) -> anyhow::Result<()> {
    let board = github.get_board(project_id, &[], None).await?;
    for def in &board.field_definitions {
        if let crate::model::project::FieldDefinition::SingleSelect { id, name, options } = def
            && name == "Status"
        {
            if let Some(opt) = options.iter().find(|o| o.name == status_name) {
                let input = CustomFieldValueInput::SingleSelect {
                    option_id: opt.id.clone(),
                };
                github
                    .update_custom_field(project_id, item_id, id, &input)
                    .await?;
                return Ok(());
            }
            let available: Vec<_> = options.iter().map(|o| o.name.as_str()).collect();
            bail!(
                "Status '{status_name}' not found. Available: {}",
                available.join(", ")
            );
        }
    }
    bail!("No Status field found on this project");
}
