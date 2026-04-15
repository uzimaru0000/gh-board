use tokio::sync::mpsc;

use crate::app_state::AppState;
use crate::command::Command;
use crate::event::AppEvent;
use crate::github::client::GitHubClient;

pub struct CommentEditorContext {
    pub content_id: String,
    pub comment_id: Option<String>,
}

pub struct App {
    pub state: AppState,
    pub pending_editor: Option<String>,
    pub pending_comment_editor: Option<CommentEditorContext>,
    github: GitHubClient,
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl App {
    pub fn new(
        github: GitHubClient,
        event_tx: mpsc::UnboundedSender<AppEvent>,
        owner: Option<String>,
    ) -> Self {
        let viewer_login = github.viewer_login().to_string();
        let mut state = AppState::new(owner);
        state.viewer_login = viewer_login;
        Self {
            state,
            pending_editor: None,
            pending_comment_editor: None,
            github,
            event_tx,
        }
    }

    pub fn load_projects(&mut self) {
        let cmd = self.state.start_loading_projects();
        self.execute(cmd);
    }

    pub fn load_project_by_number(&mut self, owner: Option<String>, number: i32) {
        let cmd = self.state.start_loading_project_by_number(owner, number);
        self.execute(cmd);
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        let cmd = self.state.handle_event(event);
        self.execute(cmd);
    }

    pub fn execute_cmd(&mut self, cmd: Command) {
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
            Command::LoadProjectByNumber { owner, number } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = if let Some(owner) = owner {
                        client.get_owner_project_by_number(&owner, number).await
                    } else {
                        client.get_viewer_project_by_number(number).await
                    };
                    let _ = tx.send(AppEvent::ProjectLoaded(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::LoadBoard {
                project_id,
                preferred_grouping_field_name,
                queries,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client
                        .get_board(
                            &project_id,
                            &queries,
                            preferred_grouping_field_name.as_deref(),
                        )
                        .await;
                    let _ = tx.send(AppEvent::BoardLoaded(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::MoveCard {
                project_id,
                item_id,
                field_id,
                value,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client
                        .update_custom_field(&project_id, &item_id, &field_id, &value)
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
                initial_status,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = async {
                        let item_id = client
                            .create_draft_issue(&project_id, &title, &body)
                            .await
                            .map_err(|e| e.to_string())?;
                        if let Some(status) = initial_status {
                            let value = crate::command::CustomFieldValueInput::SingleSelect {
                                option_id: status.option_id,
                            };
                            client
                                .update_custom_field(
                                    &project_id,
                                    &item_id,
                                    &status.field_id,
                                    &value,
                                )
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        Ok(())
                    }
                    .await;
                    let _ = tx.send(AppEvent::CardCreated(result));
                });
            }
            Command::CreateIssue {
                project_id,
                repository_id,
                title,
                body,
                initial_status,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = async {
                        let issue_id = client
                            .create_issue(&repository_id, &title, &body)
                            .await
                            .map_err(|e| e.to_string())?;
                        let item_id = client
                            .add_project_item(&project_id, &issue_id)
                            .await
                            .map_err(|e| e.to_string())?;
                        if let Some(status) = initial_status {
                            let value = crate::command::CustomFieldValueInput::SingleSelect {
                                option_id: status.option_id,
                            };
                            client
                                .update_custom_field(
                                    &project_id,
                                    &item_id,
                                    &status.field_id,
                                    &value,
                                )
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        Ok(())
                    }
                    .await;
                    let _ = tx.send(AppEvent::CardCreated(result));
                });
            }
            Command::ReorderCard {
                project_id,
                item_id,
                after_id,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client
                        .reorder_card(&project_id, &item_id, after_id.as_deref())
                        .await;
                    let _ = tx.send(AppEvent::CardReordered(result.map_err(|e| e.to_string())));
                });
            }
            Command::FetchLabels { owner, repo } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.get_repo_labels(&owner, &repo).await;
                    let _ = tx.send(AppEvent::LabelsLoaded(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::FetchAssignees { owner, repo } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.get_assignable_users(&owner, &repo).await;
                    let _ = tx.send(AppEvent::AssigneesLoaded(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::ToggleLabel {
                content_id,
                label_id,
                add,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = if add {
                        client.add_labels(&content_id, vec![label_id]).await
                    } else {
                        client.remove_labels(&content_id, vec![label_id]).await
                    };
                    let _ = tx.send(AppEvent::LabelToggled(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::ToggleAssignee {
                content_id,
                user_id,
                add,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = if add {
                        client.add_assignees(&content_id, vec![user_id]).await
                    } else {
                        client.remove_assignees(&content_id, vec![user_id]).await
                    };
                    let _ = tx.send(AppEvent::AssigneeToggled(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::UpdateCard {
                content_id,
                card_type,
                title,
                body,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = match card_type {
                        crate::model::project::CardType::DraftIssue => {
                            client.update_draft_issue(&content_id, &title, &body).await
                        }
                        crate::model::project::CardType::Issue { .. } => {
                            client.update_issue(&content_id, &title, &body).await
                        }
                        crate::model::project::CardType::PullRequest { .. } => {
                            client.update_pull_request(&content_id, &title, &body).await
                        }
                    };
                    let _ = tx.send(AppEvent::CardUpdated(result.map_err(|e| e.to_string())));
                });
            }
            Command::AddComment {
                subject_id,
                body,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.add_comment(&subject_id, &body).await;
                    let _ = tx.send(AppEvent::CommentAdded(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::UpdateComment { comment_id, body } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.update_comment(&comment_id, &body).await;
                    let _ = tx.send(AppEvent::CommentUpdated(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::FetchComments { content_id } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                let cid = content_id.clone();
                tokio::spawn(async move {
                    let result = client.fetch_all_comments(&cid).await;
                    let _ = tx.send(AppEvent::CommentsLoaded(
                        result.map(|comments| (cid, comments)).map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::OpenEditorForComment {
                content_id,
                existing,
            } => {
                let body = existing
                    .as_ref()
                    .map(|(_, b)| b.clone())
                    .unwrap_or_default();
                self.pending_editor = Some(body);
                self.pending_comment_editor = Some(CommentEditorContext {
                    content_id,
                    comment_id: existing.map(|(id, _)| id),
                });
            }
            Command::OpenEditor { content } => {
                self.pending_editor = Some(content);
            }
            Command::AddReaction {
                subject_id,
                content,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.add_reaction(&subject_id, content).await;
                    let _ = tx.send(AppEvent::ReactionToggled(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::RemoveReaction {
                subject_id,
                content,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client.remove_reaction(&subject_id, content).await;
                    let _ = tx.send(AppEvent::ReactionToggled(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::OpenUrl(url) => {
                let _ = open::that(&url);
            }
            Command::UpdateCustomField {
                project_id,
                item_id,
                field_id,
                value,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client
                        .update_custom_field(&project_id, &item_id, &field_id, &value)
                        .await;
                    let _ = tx.send(AppEvent::CustomFieldUpdated(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::ClearCustomField {
                project_id,
                item_id,
                field_id,
            } => {
                let client = self.github.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let result = client
                        .clear_custom_field(&project_id, &item_id, &field_id)
                        .await;
                    let _ = tx.send(AppEvent::CustomFieldUpdated(
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }
            Command::Batch(cmds) => {
                for cmd in cmds {
                    self.execute(cmd);
                }
            }
        }
    }
}
