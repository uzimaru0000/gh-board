use anyhow::{bail, Context};
use graphql_client::GraphQLQuery;
use tokio::process::Command;

use super::queries::*;
use crate::model::project::{
    Board, Card, CardType, Column, Comment, IssueState, Label, PrState, ProjectSummary, Repository,
};

// Type aliases for readability
use project_board::{
    ProjectBoardNode,
    ProjectBoardNodeOnProjectV2FieldsNodes as FieldNodes,
    ProjectBoardNodeOnProjectV2FieldsNodesOnProjectV2SingleSelectFieldOptions as SSOption,
    ProjectBoardNodeOnProjectV2ItemsNodes as ItemNode,
    ProjectBoardNodeOnProjectV2ItemsNodesContent as Content,
    ProjectBoardNodeOnProjectV2ItemsNodesFieldValuesNodes as FVNode,
    ProjectBoardNodeOnProjectV2ItemsNodesFieldValuesNodesOnProjectV2ItemFieldSingleSelectValueField
        as SSValueField,
};

#[derive(Clone)]
pub struct GitHubClient {
    http: reqwest::Client,
    token: String,
}

impl GitHubClient {
    pub async fn new() -> anyhow::Result<Self> {
        let token = get_token().await?;
        let http = reqwest::Client::new();
        Ok(Self { http, token })
    }

    async fn query<Q: GraphQLQuery>(
        &self,
        variables: Q::Variables,
    ) -> anyhow::Result<Q::ResponseData> {
        let body = Q::build_query(variables);
        let resp = self
            .http
            .post("https://api.github.com/graphql")
            .bearer_auth(&self.token)
            .header("User-Agent", "gh-board")
            .json(&body)
            .send()
            .await
            .context("Failed to send GraphQL request")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("GitHub API returned {status}: {text}");
        }

        let resp_body: graphql_client::Response<Q::ResponseData> =
            resp.json().await.context("Failed to parse response")?;

        if let Some(errors) = resp_body.errors {
            if !errors.is_empty() {
                let messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
                let is_scope_error = messages
                    .iter()
                    .any(|m| m.contains("INSUFFICIENT_SCOPES") || m.contains("scope"));
                if is_scope_error {
                    bail!(
                        "Token に project スコープがありません。\n\n\
                         以下のコマンドを実行してください:\n\n  \
                         gh auth refresh -s project\n"
                    );
                }
                bail!("GraphQL errors:\n{}", messages.join("\n"));
            }
        }

        resp_body.data.context("No data in GraphQL response")
    }

    pub async fn list_viewer_projects(&self) -> anyhow::Result<Vec<ProjectSummary>> {
        let vars = viewer_projects::Variables { cursor: None };
        let data = self.query::<ViewerProjects>(vars).await?;
        Ok(data
            .viewer
            .projects_v2
            .nodes
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .filter(|p| !p.closed)
            .map(|p| ProjectSummary {
                id: p.id,
                title: p.title,
                number: p.number as i32,
                description: p.short_description,
            })
            .collect())
    }

    pub async fn list_owner_projects(&self, owner: &str) -> anyhow::Result<Vec<ProjectSummary>> {
        let org_vars = org_projects::Variables {
            login: owner.to_string(),
            cursor: None,
        };
        match self.query::<OrgProjects>(org_vars).await {
            Ok(data) => {
                let org = data.organization.context("Organization not found")?;
                Ok(org
                    .projects_v2
                    .nodes
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .filter(|p| !p.closed)
                    .map(|p| ProjectSummary {
                        id: p.id,
                        title: p.title,
                        number: p.number as i32,
                        description: p.short_description,
                    })
                    .collect())
            }
            Err(_) => {
                let user_vars = user_projects::Variables {
                    login: owner.to_string(),
                    cursor: None,
                };
                let data = self
                    .query::<UserProjects>(user_vars)
                    .await
                    .context(format!("Failed to fetch projects for '{owner}'"))?;
                let user = data.user.context("User not found")?;
                Ok(user
                    .projects_v2
                    .nodes
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .filter(|p| !p.closed)
                    .map(|p| ProjectSummary {
                        id: p.id,
                        title: p.title,
                        number: p.number as i32,
                        description: p.short_description,
                    })
                    .collect())
            }
        }
    }

    pub async fn move_card(
        &self,
        project_id: &str,
        item_id: &str,
        field_id: &str,
        option_id: &str,
    ) -> anyhow::Result<()> {
        let vars = move_card::Variables {
            project_id: project_id.to_string(),
            item_id: item_id.to_string(),
            field_id: field_id.to_string(),
            option_id: option_id.to_string(),
        };
        self.query::<MoveCard>(vars).await?;
        Ok(())
    }

    pub async fn reorder_card(
        &self,
        project_id: &str,
        item_id: &str,
        after_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let vars = reorder_card::Variables {
            project_id: project_id.to_string(),
            item_id: item_id.to_string(),
            after_id: after_id.map(String::from),
        };
        self.query::<ReorderCard>(vars).await?;
        Ok(())
    }

    pub async fn delete_card(&self, project_id: &str, item_id: &str) -> anyhow::Result<()> {
        let vars = delete_card::Variables {
            project_id: project_id.to_string(),
            item_id: item_id.to_string(),
        };
        self.query::<DeleteCard>(vars).await?;
        Ok(())
    }

    pub async fn create_draft_issue(
        &self,
        project_id: &str,
        title: &str,
        body: &str,
    ) -> anyhow::Result<String> {
        let vars = create_draft_issue::Variables {
            project_id: project_id.to_string(),
            title: title.to_string(),
            body: Some(body.to_string()),
        };
        let data = self.query::<CreateDraftIssue>(vars).await?;
        let item = data
            .add_project_v2_draft_issue
            .and_then(|p| p.project_item)
            .context("Failed to create draft issue")?;
        Ok(item.id)
    }

    pub async fn create_issue(
        &self,
        repository_id: &str,
        title: &str,
        body: &str,
    ) -> anyhow::Result<String> {
        let vars = create_issue::Variables {
            repository_id: repository_id.to_string(),
            title: title.to_string(),
            body: Some(body.to_string()),
        };
        let data = self.query::<CreateIssue>(vars).await?;
        let issue = data
            .create_issue
            .and_then(|p| p.issue)
            .context("Failed to create issue")?;
        Ok(issue.id)
    }

    pub async fn add_project_item(
        &self,
        project_id: &str,
        content_id: &str,
    ) -> anyhow::Result<String> {
        let vars = add_project_item::Variables {
            project_id: project_id.to_string(),
            content_id: content_id.to_string(),
        };
        let data = self.query::<AddProjectItem>(vars).await?;
        let item = data
            .add_project_v2_item_by_id
            .and_then(|p| p.item)
            .context("Failed to add item to project")?;
        Ok(item.id)
    }

    pub async fn get_repo_labels(
        &self,
        owner: &str,
        name: &str,
    ) -> anyhow::Result<Vec<Label>> {
        let vars = repo_labels::Variables {
            owner: owner.to_string(),
            name: name.to_string(),
        };
        let data = self.query::<RepoLabels>(vars).await?;
        let repo = data.repository.context("Repository not found")?;
        Ok(repo
            .labels
            .and_then(|l| l.nodes)
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(|l| Label {
                id: l.id,
                name: l.name,
                color: l.color,
            })
            .collect())
    }

    pub async fn get_assignable_users(
        &self,
        owner: &str,
        name: &str,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let vars = assignable_users::Variables {
            owner: owner.to_string(),
            name: name.to_string(),
        };
        let data = self.query::<AssignableUsers>(vars).await?;
        let repo = data.repository.context("Repository not found")?;
        Ok(repo
            .assignable_users
            .nodes
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(|u| (u.id, u.login))
            .collect())
    }

    pub async fn add_labels(
        &self,
        content_id: &str,
        label_ids: Vec<String>,
    ) -> anyhow::Result<()> {
        let vars = add_labels::Variables {
            labelable_id: content_id.to_string(),
            label_ids,
        };
        self.query::<AddLabels>(vars).await?;
        Ok(())
    }

    pub async fn remove_labels(
        &self,
        content_id: &str,
        label_ids: Vec<String>,
    ) -> anyhow::Result<()> {
        let vars = remove_labels::Variables {
            labelable_id: content_id.to_string(),
            label_ids,
        };
        self.query::<RemoveLabels>(vars).await?;
        Ok(())
    }

    pub async fn add_assignees(
        &self,
        content_id: &str,
        assignee_ids: Vec<String>,
    ) -> anyhow::Result<()> {
        let vars = add_assignees::Variables {
            assignable_id: content_id.to_string(),
            assignee_ids,
        };
        self.query::<AddAssignees>(vars).await?;
        Ok(())
    }

    pub async fn remove_assignees(
        &self,
        content_id: &str,
        assignee_ids: Vec<String>,
    ) -> anyhow::Result<()> {
        let vars = remove_assignees::Variables {
            assignable_id: content_id.to_string(),
            assignee_ids,
        };
        self.query::<RemoveAssignees>(vars).await?;
        Ok(())
    }

    pub async fn get_board(&self, project_id: &str) -> anyhow::Result<Board> {
        let mut all_items: Vec<ItemNode> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut title = String::new();
        let mut field_nodes = None;
        let mut repositories: Vec<Repository> = Vec::new();

        loop {
            let vars = project_board::Variables {
                project_id: project_id.to_string(),
                items_cursor: cursor,
            };
            let data = self.query::<ProjectBoard>(vars).await?;
            let node = data.node.context("Project not found")?;

            let pv2 = match node {
                ProjectBoardNode::ProjectV2(pv2) => pv2,
                _ => bail!("Node is not a ProjectV2"),
            };

            let has_next = pv2.items.page_info.has_next_page;
            let next_cursor = pv2.items.page_info.end_cursor;

            if let Some(nodes) = pv2.items.nodes {
                all_items.extend(nodes.into_iter().flatten());
            }

            if title.is_empty() {
                title = pv2.title;
                field_nodes = pv2.fields.nodes;
                repositories = pv2
                    .repositories
                    .nodes
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .map(|r| Repository {
                        id: r.id,
                        name_with_owner: r.name_with_owner,
                    })
                    .collect();
            }

            if has_next {
                cursor = next_cursor;
            } else {
                break;
            }
        }

        build_board(title, field_nodes, all_items, repositories)
    }
}

async fn get_token() -> anyhow::Result<String> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .await
        .context(
            "gh コマンドが見つかりません。GitHub CLI をインストールしてください:\n\n  \
             https://cli.github.com/\n",
        )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "認証トークンを取得できませんでした。\n\n\
             以下のコマンドでログインしてください:\n\n  \
             gh auth login\n\n\
             詳細: {stderr}"
        );
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

// --- Board building ---

fn build_board(
    project_title: String,
    field_nodes: Option<Vec<Option<FieldNodes>>>,
    items: Vec<ItemNode>,
    repositories: Vec<Repository>,
) -> anyhow::Result<Board> {
    let field_nodes = field_nodes.unwrap_or_default();

    // Find Status field (or first single-select as fallback)
    let mut first_ss: Option<(String, Vec<SSOption>)> = None;
    let mut status_match: Option<(String, Vec<SSOption>)> = None;

    for node in field_nodes.into_iter().flatten() {
        if let FieldNodes::ProjectV2SingleSelectField(ssf) = node {
            if ssf.name == "Status" {
                status_match = Some((ssf.id, ssf.options));
                break;
            }
            if first_ss.is_none() {
                first_ss = Some((ssf.id, ssf.options));
            }
        }
    }

    let (status_field_id, options) = status_match
        .or(first_ss)
        .unwrap_or_else(|| (String::new(), vec![]));

    let mut columns: Vec<Column> = options
        .iter()
        .map(|opt| Column {
            option_id: opt.id.clone(),
            name: opt.name.clone(),
            cards: Vec::new(),
        })
        .collect();

    let mut no_status_cards = Vec::new();

    for item in items {
        let card = convert_item(&item);

        let fv_nodes = item.field_values.nodes.unwrap_or_default();
        let status_option_id = fv_nodes.iter().flatten().find_map(|fv| {
            if let FVNode::ProjectV2ItemFieldSingleSelectValue(sv) = fv {
                if let SSValueField::ProjectV2SingleSelectField(f) = &sv.field {
                    if f.id == status_field_id {
                        return sv.option_id.clone();
                    }
                }
            }
            None
        });

        match status_option_id {
            Some(opt_id) => {
                if let Some(col) = columns.iter_mut().find(|c| c.option_id == opt_id) {
                    col.cards.push(card);
                } else {
                    no_status_cards.push(card);
                }
            }
            None => no_status_cards.push(card),
        }
    }

    if !no_status_cards.is_empty() {
        columns.insert(
            0,
            Column {
                option_id: String::new(),
                name: "No Status".to_string(),
                cards: no_status_cards,
            },
        );
    }

    Ok(Board {
        project_title,
        status_field_id,
        columns,
        repositories,
    })
}

fn convert_item(item: &ItemNode) -> Card {
    match &item.content {
        Some(Content::Issue(issue)) => Card {
            item_id: item.id.clone(),
            content_id: Some(issue.id.clone()),
            title: issue.title.clone(),
            number: Some(issue.number as i32),
            card_type: CardType::Issue {
                state: match issue.state {
                    project_board::IssueState::CLOSED => IssueState::Closed,
                    _ => IssueState::Open,
                },
            },
            assignees: issue
                .assignees
                .nodes
                .as_ref()
                .map(|n| n.iter().flatten().map(|u| u.login.clone()).collect())
                .unwrap_or_default(),
            labels: issue
                .labels
                .as_ref()
                .and_then(|l| l.nodes.as_ref())
                .map(|n| {
                    n.iter()
                        .flatten()
                        .map(|l| Label {
                            id: l.id.clone(),
                            name: l.name.clone(),
                            color: l.color.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            url: Some(issue.url.clone()),
            body: Some(issue.body.clone()),
            comments: issue
                .comments
                .nodes
                .as_ref()
                .map(|n| {
                    n.iter()
                        .flatten()
                        .map(|c| Comment {
                            author: c
                                .author
                                .as_ref()
                                .map(|a| a.login.clone())
                                .unwrap_or_else(|| "ghost".into()),
                            body: c.body.clone(),
                            created_at: c.created_at.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        Some(Content::PullRequest(pr)) => Card {
            item_id: item.id.clone(),
            content_id: Some(pr.id.clone()),
            title: pr.title.clone(),
            number: Some(pr.number as i32),
            card_type: CardType::PullRequest {
                state: match pr.state {
                    project_board::PullRequestState::CLOSED => PrState::Closed,
                    project_board::PullRequestState::MERGED => PrState::Merged,
                    _ => PrState::Open,
                },
            },
            assignees: pr
                .assignees
                .nodes
                .as_ref()
                .map(|n| n.iter().flatten().map(|u| u.login.clone()).collect())
                .unwrap_or_default(),
            labels: pr
                .labels
                .as_ref()
                .and_then(|l| l.nodes.as_ref())
                .map(|n| {
                    n.iter()
                        .flatten()
                        .map(|l| Label {
                            id: l.id.clone(),
                            name: l.name.clone(),
                            color: l.color.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            url: Some(pr.url.clone()),
            body: Some(pr.body.clone()),
            comments: pr
                .comments
                .nodes
                .as_ref()
                .map(|n| {
                    n.iter()
                        .flatten()
                        .map(|c| Comment {
                            author: c
                                .author
                                .as_ref()
                                .map(|a| a.login.clone())
                                .unwrap_or_else(|| "ghost".into()),
                            body: c.body.clone(),
                            created_at: c.created_at.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        Some(Content::DraftIssue(draft)) => Card {
            item_id: item.id.clone(),
            content_id: None,
            title: draft.title.clone(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: Vec::new(),
            labels: Vec::new(),
            url: None,
            body: Some(draft.body.clone()),
            comments: Vec::new(),
        },
        None => Card {
            item_id: item.id.clone(),
            content_id: None,
            title: "(no content)".to_string(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: Vec::new(),
            labels: Vec::new(),
            url: None,
            body: None,
            comments: Vec::new(),
        },
    }
}
