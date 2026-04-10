use tokio::sync::mpsc;

use crate::app_state::AppState;
use crate::command::Command;
use crate::event::AppEvent;
use crate::github::client::GitHubClient;

pub struct App {
    pub state: AppState,
    github: GitHubClient,
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl App {
    pub fn new(
        github: GitHubClient,
        event_tx: mpsc::UnboundedSender<AppEvent>,
        owner: Option<String>,
    ) -> Self {
        Self {
            state: AppState::new(owner),
            github,
            event_tx,
        }
    }

    pub fn load_projects(&mut self) {
        let cmd = self.state.start_loading_projects();
        self.execute(cmd);
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        let cmd = self.state.handle_event(event);
        self.execute(cmd);
    }

    pub fn select_project_by_number(&mut self, number: i32) {
        let cmd = self.state.select_project_by_number(number);
        self.execute(cmd);
    }

    fn execute(&mut self, cmd: Command) {
        match cmd {
            Command::None => {}
            Command::LoadProjects { owner } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = if let Some(owner) = owner {
                        client.list_owner_projects(&owner).await
                    } else {
                        client.list_viewer_projects().await
                    };
                    let _ = tx.send(AppEvent::ProjectsLoaded(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::LoadBoard { project_id } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.get_board(&project_id).await;
                    let _ = tx.send(AppEvent::BoardLoaded(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::MoveCard {
                project_id,
                item_id,
                field_id,
                option_id,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client
                        .move_card(&project_id, &item_id, &field_id, &option_id)
                        .await;
                    let _ = tx.send(AppEvent::CardMoved(result.map_err(|e| e.to_string())));
                });
            }
            Command::DeleteCard {
                project_id,
                item_id,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.delete_card(&project_id, &item_id).await;
                    let _ = tx.send(AppEvent::CardDeleted(result.map_err(|e| e.to_string())));
                });
            }
            Command::CreateCard {
                project_id,
                title,
                body,
                field_id,
                option_id,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = async {
                        let item_id = client
                            .create_draft_issue(&project_id, &title, &body)
                            .await
                            .map_err(|e| e.to_string())?;
                        if !option_id.is_empty() {
                            client
                                .move_card(&project_id, &item_id, &field_id, &option_id)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        Ok(())
                    }
                    .await;
                    let _ = tx.send(AppEvent::CardCreated(result));
                });
            }
            Command::OpenUrl(url) => {
                let _ = open::that(&url);
            }
            Command::Batch(cmds) => {
                for cmd in cmds {
                    self.execute(cmd);
                }
            }
        }
    }
}
