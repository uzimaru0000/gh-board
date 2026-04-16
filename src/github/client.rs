use anyhow::{bail, Context};
use graphql_client::GraphQLQuery;
use tokio::process::Command;

use super::queries::*;
use crate::command::CustomFieldValueInput;
use crate::model::project::{
    Board, Card, CardDetail, CardType, CiStatus, Column, ColumnColor, Comment, CustomFieldValue,
    FieldDefinition, Grouping, IssueState, IterationOption, Label, LinkedPr, PaginationState,
    ParentIssueRef, PrState, PrStatus, ProjectSummary, ReactionContent, ReactionSummary,
    Repository, ReviewDecision, SingleSelectOption, SubIssueRef, SubIssuesSummary,
};

// ReactionContent 変換: 各 GraphQL クエリごとに自動生成される enum を model 側の型に変換する。
// 全クエリで同じ variant 名を共有しているため、macro で impl を展開する。
macro_rules! impl_reaction_content_from {
    ($module:path) => {
        impl ReactionContentFromGraphQL for $module {
            fn to_model(&self) -> Option<ReactionContent> {
                #[allow(unreachable_patterns)]
                match self {
                    Self::THUMBS_UP => Some(ReactionContent::ThumbsUp),
                    Self::THUMBS_DOWN => Some(ReactionContent::ThumbsDown),
                    Self::LAUGH => Some(ReactionContent::Laugh),
                    Self::HOORAY => Some(ReactionContent::Hooray),
                    Self::CONFUSED => Some(ReactionContent::Confused),
                    Self::HEART => Some(ReactionContent::Heart),
                    Self::ROCKET => Some(ReactionContent::Rocket),
                    Self::EYES => Some(ReactionContent::Eyes),
                    _ => None,
                }
            }
        }
    };
}

trait ReactionContentFromGraphQL {
    fn to_model(&self) -> Option<ReactionContent>;
}

impl_reaction_content_from!(fetch_comments::ReactionContent);
impl_reaction_content_from!(add_comment::ReactionContent);
impl_reaction_content_from!(add_reaction_mutation::ReactionContent);
impl_reaction_content_from!(remove_reaction_mutation::ReactionContent);
impl_reaction_content_from!(fetch_issue::ReactionContent);
impl_reaction_content_from!(fetch_card_detail::ReactionContent);

// Type aliases for readability
use project_board::{
    ProjectBoardNode,
    ProjectBoardNodeOnProjectV2FieldsNodes as FieldNodes,
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
    viewer_login: String,
}

impl GitHubClient {
    pub async fn new() -> anyhow::Result<Self> {
        let token = get_token().await?;
        let http = reqwest::Client::new();
        let mut client = Self {
            http,
            token,
            viewer_login: String::new(),
        };
        // Fetch viewer login
        let vars = viewer_login::Variables {};
        let data = client.query::<ViewerLogin>(vars).await?;
        client.viewer_login = data.viewer.login;
        Ok(client)
    }

    pub fn viewer_login(&self) -> &str {
        &self.viewer_login
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

        if let Some(errors) = resp_body.errors
            && !errors.is_empty() {
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

    pub async fn get_viewer_project_by_number(
        &self,
        number: i32,
    ) -> anyhow::Result<ProjectSummary> {
        let vars = viewer_project_by_number::Variables {
            number: number as i64,
        };
        let data = self.query::<ViewerProjectByNumber>(vars).await?;
        let project = data
            .viewer
            .project_v2
            .context(format!("Project #{number} not found"))?;
        Ok(ProjectSummary {
            id: project.id,
            title: project.title,
            number: project.number as i32,
            description: project.short_description,
        })
    }

    pub async fn get_owner_project_by_number(
        &self,
        owner: &str,
        number: i32,
    ) -> anyhow::Result<ProjectSummary> {
        let org_vars = org_project_by_number::Variables {
            login: owner.to_string(),
            number: number as i64,
        };
        match self.query::<OrgProjectByNumber>(org_vars).await {
            Ok(data) => {
                let org = data.organization.context("Organization not found")?;
                let project = org
                    .project_v2
                    .context(format!("Project #{number} not found"))?;
                Ok(ProjectSummary {
                    id: project.id,
                    title: project.title,
                    number: project.number as i32,
                    description: project.short_description,
                })
            }
            Err(_) => {
                let user_vars = user_project_by_number::Variables {
                    login: owner.to_string(),
                    number: number as i64,
                };
                let data = self
                    .query::<UserProjectByNumber>(user_vars)
                    .await
                    .context(format!("Failed to fetch project #{number} for '{owner}'"))?;
                let user = data.user.context("User not found")?;
                let project = user
                    .project_v2
                    .context(format!("Project #{number} not found"))?;
                Ok(ProjectSummary {
                    id: project.id,
                    title: project.title,
                    number: project.number as i32,
                    description: project.short_description,
                })
            }
        }
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

    pub async fn archive_card(&self, project_id: &str, item_id: &str) -> anyhow::Result<()> {
        let vars = archive_card::Variables {
            project_id: project_id.to_string(),
            item_id: item_id.to_string(),
        };
        self.query::<ArchiveCard>(vars).await?;
        Ok(())
    }

    pub async fn unarchive_card(&self, project_id: &str, item_id: &str) -> anyhow::Result<()> {
        let vars = unarchive_card::Variables {
            project_id: project_id.to_string(),
            item_id: item_id.to_string(),
        };
        self.query::<UnarchiveCard>(vars).await?;
        Ok(())
    }

    /// archived 済みアイテムだけを取得し、平坦な Vec<Card> を返す。
    /// Projects V2 items() の `query` 引数は archive フラグを安定して解釈しないため、
    /// 全件取得してクライアント側で `card.archived == true` のものだけを残す。
    pub async fn get_archived_items(&self, project_id: &str) -> anyhow::Result<Vec<Card>> {
        let board = self.get_board_raw(project_id, &[], None).await?;
        let mut cards: Vec<Card> = Vec::new();
        for col in board.columns {
            for card in col.cards {
                if card.archived {
                    cards.push(card);
                }
            }
        }
        Ok(cards)
    }

    pub async fn update_custom_field(
        &self,
        project_id: &str,
        item_id: &str,
        field_id: &str,
        value: &CustomFieldValueInput,
    ) -> anyhow::Result<()> {
        // GitHub API は ProjectV2FieldValue に含めるフィールドが "ちょうど 1 つ" であることを要求する。
        // graphql_client の自動生成型は Option::None を `null` としてシリアライズするため、
        // 排他制約に引っかかる。ここでは value オブジェクトを手動で JSON 化して mutation を送る。
        let value_json = match value {
            CustomFieldValueInput::SingleSelect { option_id } => {
                serde_json::json!({ "singleSelectOptionId": option_id })
            }
            CustomFieldValueInput::Iteration { iteration_id } => {
                serde_json::json!({ "iterationId": iteration_id })
            }
            CustomFieldValueInput::Text { text } => serde_json::json!({ "text": text }),
            CustomFieldValueInput::Number { number } => serde_json::json!({ "number": number }),
            CustomFieldValueInput::Date { date } => serde_json::json!({ "date": date }),
        };
        let query = r#"
            mutation UpdateFieldValue(
              $projectId: ID!
              $itemId: ID!
              $fieldId: ID!
              $value: ProjectV2FieldValue!
            ) {
              updateProjectV2ItemFieldValue(
                input: {
                  projectId: $projectId
                  itemId: $itemId
                  fieldId: $fieldId
                  value: $value
                }
              ) {
                projectV2Item { id }
              }
            }
        "#;
        let body = serde_json::json!({
            "query": query,
            "variables": {
                "projectId": project_id,
                "itemId": item_id,
                "fieldId": field_id,
                "value": value_json,
            }
        });
        self.raw_graphql(body).await
    }

    /// 手書きの GraphQL body (query + variables) を実行する。
    /// 主に Option フィールドを含まないシリアライズが必要な mutation で使用する。
    async fn raw_graphql(&self, body: serde_json::Value) -> anyhow::Result<()> {
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
        let json: serde_json::Value = resp.json().await.context("Failed to parse response")?;
        if let Some(errors) = json.get("errors").and_then(|e| e.as_array())
            && !errors.is_empty()
        {
            let messages: Vec<String> = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()).map(String::from))
                .collect();
            bail!("GraphQL errors:\n{}", messages.join("\n"));
        }
        Ok(())
    }

    pub async fn clear_custom_field(
        &self,
        project_id: &str,
        item_id: &str,
        field_id: &str,
    ) -> anyhow::Result<()> {
        let vars = clear_field_value::Variables {
            project_id: project_id.to_string(),
            item_id: item_id.to_string(),
            field_id: field_id.to_string(),
        };
        self.query::<ClearFieldValue>(vars).await?;
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

    pub async fn update_draft_issue(
        &self,
        draft_issue_id: &str,
        title: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        let vars = update_draft_issue::Variables {
            draft_issue_id: draft_issue_id.to_string(),
            title: title.to_string(),
            body: Some(body.to_string()),
        };
        self.query::<UpdateDraftIssue>(vars).await?;
        Ok(())
    }

    pub async fn update_issue(
        &self,
        issue_id: &str,
        title: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        let vars = update_issue::Variables {
            id: issue_id.to_string(),
            title: title.to_string(),
            body: Some(body.to_string()),
        };
        self.query::<UpdateIssue>(vars).await?;
        Ok(())
    }

    pub async fn update_pull_request(
        &self,
        pr_id: &str,
        title: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        let vars = update_pull_request::Variables {
            pull_request_id: pr_id.to_string(),
            title: title.to_string(),
            body: Some(body.to_string()),
        };
        self.query::<UpdatePullRequest>(vars).await?;
        Ok(())
    }

    /// 通常のボードロード。アーカイブ済みアイテムは除外する。
    pub async fn get_board(
        &self,
        project_id: &str,
        queries: &[String],
        preferred_group_by_field_name: Option<&str>,
    ) -> anyhow::Result<Board> {
        let mut board = self
            .get_board_raw(project_id, queries, preferred_group_by_field_name)
            .await?;
        for col in &mut board.columns {
            col.cards.retain(|c| !c.archived);
        }
        Ok(board)
    }

    /// プログレッシブレンダリング: 1ページ目を取得し Board + ページネーション状態を返す。
    /// アーカイブカードは除外する。
    pub async fn get_board_first_page(
        &self,
        project_id: &str,
        queries: &[String],
        preferred_group_by_field_name: Option<&str>,
    ) -> anyhow::Result<(Board, Vec<PaginationState>)> {
        let query_iter: Vec<Option<String>> = if queries.is_empty() {
            vec![None]
        } else {
            queries.iter().cloned().map(Some).collect()
        };

        let mut seen_item_ids: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut all_items: Vec<ItemNode> = Vec::new();
        let mut title = String::new();
        let mut field_nodes = None;
        let mut repositories: Vec<Repository> = Vec::new();
        let mut remaining_pagination: Vec<PaginationState> = Vec::new();

        for query in &query_iter {
            let vars = project_board::Variables {
                project_id: project_id.to_string(),
                items_cursor: None,
                query: query.clone(),
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
                for item in nodes.into_iter().flatten() {
                    if seen_item_ids.insert(item.id.clone()) {
                        all_items.push(item);
                    }
                }
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

            if has_next
                && let Some(cursor) = next_cursor
            {
                remaining_pagination.push(PaginationState {
                    query: query.clone(),
                    cursor,
                });
            }
        }

        let mut board = build_board(
            title,
            field_nodes,
            all_items,
            repositories,
            preferred_group_by_field_name,
        )?;
        for col in &mut board.columns {
            col.cards.retain(|c| !c.archived);
        }
        Ok((board, remaining_pagination))
    }

    /// プログレッシブレンダリング: 2ページ目以降を取得しカード一覧を返す。
    pub async fn get_board_next_page(
        &self,
        project_id: &str,
        pagination: Vec<PaginationState>,
        preferred_group_by_field_name: Option<&str>,
    ) -> anyhow::Result<(Vec<Card>, Vec<PaginationState>)> {
        let mut all_cards: Vec<Card> = Vec::new();
        let mut remaining: Vec<PaginationState> = Vec::new();

        for page_state in pagination {
            let vars = project_board::Variables {
                project_id: project_id.to_string(),
                items_cursor: Some(page_state.cursor),
                query: page_state.query.clone(),
            };
            let data = self.query::<ProjectBoard>(vars).await?;
            let node = data.node.context("Project not found")?;
            let pv2 = match node {
                ProjectBoardNode::ProjectV2(pv2) => pv2,
                _ => bail!("Node is not a ProjectV2"),
            };

            let has_next = pv2.items.page_info.has_next_page;
            let next_cursor = pv2.items.page_info.end_cursor;

            // field_definitions は1ページ目で取得済みなのでここでは必要だが、
            // convert_item + custom_fields のパースにはフィールド定義が必要
            let field_nodes_for_page = pv2.fields.nodes;

            if let Some(nodes) = pv2.items.nodes {
                let items: Vec<ItemNode> = nodes.into_iter().flatten().collect();
                // build_board を使って正しく custom_fields をパースする
                let board = build_board(
                    String::new(),
                    field_nodes_for_page,
                    items,
                    Vec::new(),
                    preferred_group_by_field_name,
                )?;
                for col in board.columns {
                    for card in col.cards {
                        if !card.archived {
                            all_cards.push(card);
                        }
                    }
                }
            }

            if has_next
                && let Some(cursor) = next_cursor
            {
                remaining.push(PaginationState {
                    query: page_state.query,
                    cursor,
                });
            }
        }

        Ok((all_cards, remaining))
    }

    /// アーカイブを含む全アイテムを返す内部 API。`get_archived_items` から再利用する。
    async fn get_board_raw(
        &self,
        project_id: &str,
        queries: &[String],
        preferred_group_by_field_name: Option<&str>,
    ) -> anyhow::Result<Board> {
        // queries が空ならフィルタなしで 1 回ロード。
        // 複数 queries は OR として個別に fetch して item_id で dedup する。
        let query_iter: Vec<Option<String>> = if queries.is_empty() {
            vec![None]
        } else {
            queries.iter().cloned().map(Some).collect()
        };

        let mut seen_item_ids: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut all_items: Vec<ItemNode> = Vec::new();
        let mut title = String::new();
        let mut field_nodes = None;
        let mut repositories: Vec<Repository> = Vec::new();

        for query in query_iter {
            let mut cursor: Option<String> = None;
            loop {
                let vars = project_board::Variables {
                    project_id: project_id.to_string(),
                    items_cursor: cursor,
                    query: query.clone(),
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
                    for item in nodes.into_iter().flatten() {
                        if seen_item_ids.insert(item.id.clone()) {
                            all_items.push(item);
                        }
                    }
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
        }

        build_board(title, field_nodes, all_items, repositories, preferred_group_by_field_name)
    }

    pub async fn fetch_all_comments(&self, content_id: &str) -> anyhow::Result<Vec<Comment>> {
        let mut all_comments = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let vars = fetch_comments::Variables {
                id: content_id.to_string(),
                cursor,
            };
            let data = self.query::<FetchComments>(vars).await?;

            let node = data.node.context("Node not found")?;

            let (has_next, end_cursor) = match node {
                fetch_comments::FetchCommentsNode::Issue(issue) => {
                    if let Some(nodes) = issue.comments.nodes {
                        for c in nodes.into_iter().flatten() {
                            let reactions = c
                                .reaction_groups
                                .as_ref()
                                .map(|gs| {
                                    gs.iter()
                                        .filter_map(|g| {
                                            Some(ReactionSummary {
                                                content: g.content.to_model()?,
                                                count: g.reactors.total_count as usize,
                                                viewer_has_reacted: g.viewer_has_reacted,
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            all_comments.push(Comment {
                                id: c.id,
                                author: c.author.as_ref().map(|a| a.login.clone()).unwrap_or_else(|| "ghost".into()),
                                body: c.body,
                                created_at: c.created_at,
                                reactions,
                            });
                        }
                    }
                    (issue.comments.page_info.has_next_page, issue.comments.page_info.end_cursor)
                }
                fetch_comments::FetchCommentsNode::PullRequest(pr) => {
                    if let Some(nodes) = pr.comments.nodes {
                        for c in nodes.into_iter().flatten() {
                            let reactions = c
                                .reaction_groups
                                .as_ref()
                                .map(|gs| {
                                    gs.iter()
                                        .filter_map(|g| {
                                            Some(ReactionSummary {
                                                content: g.content.to_model()?,
                                                count: g.reactors.total_count as usize,
                                                viewer_has_reacted: g.viewer_has_reacted,
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            all_comments.push(Comment {
                                id: c.id,
                                author: c.author.as_ref().map(|a| a.login.clone()).unwrap_or_else(|| "ghost".into()),
                                body: c.body,
                                created_at: c.created_at,
                                reactions,
                            });
                        }
                    }
                    (pr.comments.page_info.has_next_page, pr.comments.page_info.end_cursor)
                }
                _ => bail!("Unexpected node type for comments"),
            };

            if has_next {
                cursor = end_cursor;
            } else {
                break;
            }
        }

        Ok(all_comments)
    }

    /// Issue を id で取得し、Card 化して返す (Parent / Sub-issue モーダル表示用)。
    /// item_id は board 上に対応するカードがあればそれ、なければ content_id を流用 (ボード外 issue)。
    pub async fn fetch_issue_as_card(&self, content_id: &str) -> anyhow::Result<Card> {
        let vars = fetch_issue::Variables {
            id: content_id.to_string(),
        };
        let data = self.query::<FetchIssue>(vars).await?;
        let node = data.node.context("Node not found")?;
        let fetch_issue::FetchIssueNode::Issue(issue) = node else {
            bail!("Node is not an Issue");
        };
        let assignees = issue
            .assignees
            .nodes
            .as_ref()
            .map(|n| n.iter().flatten().map(|u| u.login.clone()).collect())
            .unwrap_or_default();
        let labels = issue
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
            .unwrap_or_default();
        let comments = issue
            .comments
            .nodes
            .as_ref()
            .map(|n| {
                n.iter()
                    .flatten()
                    .map(|c| Comment {
                        id: c.id.clone(),
                        author: c
                            .author
                            .as_ref()
                            .map(|a| a.login.clone())
                            .unwrap_or_else(|| "ghost".into()),
                        body: c.body.clone(),
                        created_at: c.created_at.clone(),
                        reactions: c
                            .reaction_groups
                            .as_ref()
                            .map(|gs| {
                                gs.iter()
                                    .filter_map(|g| {
                                        Some(ReactionSummary {
                                            content: g.content.to_model()?,
                                            count: g.reactors.total_count as usize,
                                            viewer_has_reacted: g.viewer_has_reacted,
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let reactions = issue
            .reaction_groups
            .as_ref()
            .map(|gs| {
                gs.iter()
                    .filter_map(|g| {
                        Some(ReactionSummary {
                            content: g.content.to_model()?,
                            count: g.reactors.total_count as usize,
                            viewer_has_reacted: g.viewer_has_reacted,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let state = match issue.state {
            fetch_issue::IssueState::CLOSED => IssueState::Closed,
            _ => IssueState::Open,
        };
        Ok(Card {
            // Parent/Sub-issue はボード上の ProjectV2 item ではないので item_id を持たない。
            // 代わりに content_id を流用しておく (ナビゲーション用途では未使用)。
            item_id: issue.id.clone(),
            content_id: Some(issue.id.clone()),
            title: issue.title.clone(),
            number: Some(issue.number as i32),
            card_type: CardType::Issue { state },
            assignees,
            labels,
            url: Some(issue.url.clone()),
            body: Some(issue.body.clone()),
            comments,
            milestone: issue.milestone.as_ref().map(|m| m.title.clone()),
            custom_fields: Vec::new(),
            pr_status: None,
            linked_prs: Vec::new(),
            reactions,
            archived: false,
            parent_issue: issue.parent.as_ref().map(|p| ParentIssueRef {
                id: p.id.clone(),
                number: p.number as i32,
                title: p.title.clone(),
                url: Some(p.url.clone()),
            }),
            sub_issues_summary: Some(SubIssuesSummary {
                completed: issue.sub_issues_summary.completed as i32,
                total: issue.sub_issues_summary.total as i32,
            }),
            sub_issues: Vec::new(),
        })
    }

    pub async fn fetch_card_detail(&self, content_id: &str) -> anyhow::Result<CardDetail> {
        let vars = fetch_card_detail::Variables {
            id: content_id.to_string(),
        };
        let data = self.query::<FetchCardDetail>(vars).await?;
        let node = data.node.context("Node not found")?;

        match node {
            fetch_card_detail::FetchCardDetailNode::Issue(issue) => {
                let comments = Self::convert_card_detail_comments(&issue.comments.nodes);
                let reactions = Self::convert_card_detail_reactions(&issue.reaction_groups);
                let linked_prs = issue
                    .closed_by_pull_requests_references
                    .as_ref()
                    .and_then(|c| c.nodes.as_ref())
                    .map(|nodes| {
                        nodes
                            .iter()
                            .flatten()
                            .map(|pr| LinkedPr {
                                number: pr.number as i32,
                                title: pr.title.clone(),
                                url: pr.url.clone(),
                                state: match pr.state {
                                    fetch_card_detail::PullRequestState::CLOSED => PrState::Closed,
                                    fetch_card_detail::PullRequestState::MERGED => PrState::Merged,
                                    _ => PrState::Open,
                                },
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(CardDetail {
                    body: issue.body.clone(),
                    comments,
                    reactions,
                    linked_prs,
                })
            }
            fetch_card_detail::FetchCardDetailNode::PullRequest(pr) => {
                let comments = pr
                    .comments
                    .nodes
                    .as_ref()
                    .map(|n| {
                        n.iter()
                            .flatten()
                            .map(|c| Comment {
                                id: c.id.clone(),
                                author: c
                                    .author
                                    .as_ref()
                                    .map(|a| a.login.clone())
                                    .unwrap_or_else(|| "ghost".into()),
                                body: c.body.clone(),
                                created_at: c.created_at.clone(),
                                reactions: c
                                    .reaction_groups
                                    .as_ref()
                                    .map(|gs| {
                                        gs.iter()
                                            .filter_map(|g| {
                                                Some(ReactionSummary {
                                                    content: g.content.to_model()?,
                                                    count: g.reactors.total_count as usize,
                                                    viewer_has_reacted: g.viewer_has_reacted,
                                                })
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let reactions = pr
                    .reaction_groups
                    .as_ref()
                    .map(|gs| {
                        gs.iter()
                            .filter_map(|g| {
                                Some(ReactionSummary {
                                    content: g.content.to_model()?,
                                    count: g.reactors.total_count as usize,
                                    viewer_has_reacted: g.viewer_has_reacted,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(CardDetail {
                    body: pr.body.clone(),
                    comments,
                    reactions,
                    linked_prs: Vec::new(),
                })
            }
            fetch_card_detail::FetchCardDetailNode::DraftIssue(draft) => Ok(CardDetail {
                body: draft.body.clone(),
                comments: Vec::new(),
                reactions: Vec::new(),
                linked_prs: Vec::new(),
            }),
            _ => bail!("Unexpected node type for card detail"),
        }
    }

    fn convert_card_detail_comments(
        nodes: &Option<
            Vec<
                Option<
                    fetch_card_detail::FetchCardDetailNodeOnIssueCommentsNodes,
                >,
            >,
        >,
    ) -> Vec<Comment> {
        nodes
            .as_ref()
            .map(|n| {
                n.iter()
                    .flatten()
                    .map(|c| Comment {
                        id: c.id.clone(),
                        author: c
                            .author
                            .as_ref()
                            .map(|a| a.login.clone())
                            .unwrap_or_else(|| "ghost".into()),
                        body: c.body.clone(),
                        created_at: c.created_at.clone(),
                        reactions: c
                            .reaction_groups
                            .as_ref()
                            .map(|gs| {
                                gs.iter()
                                    .filter_map(|g| {
                                        Some(ReactionSummary {
                                            content: g.content.to_model()?,
                                            count: g.reactors.total_count as usize,
                                            viewer_has_reacted: g.viewer_has_reacted,
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn convert_card_detail_reactions(
        groups: &Option<Vec<fetch_card_detail::FetchCardDetailNodeOnIssueReactionGroups>>,
    ) -> Vec<ReactionSummary> {
        groups
            .as_ref()
            .map(|gs| {
                gs.iter()
                    .filter_map(|g| {
                        Some(ReactionSummary {
                            content: g.content.to_model()?,
                            count: g.reactors.total_count as usize,
                            viewer_has_reacted: g.viewer_has_reacted,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn fetch_sub_issues(&self, content_id: &str) -> anyhow::Result<Vec<SubIssueRef>> {
        let vars = fetch_sub_issues::Variables {
            id: content_id.to_string(),
        };
        let data = self.query::<FetchSubIssues>(vars).await?;
        let node = data.node.context("Node not found")?;
        let mut sub_issues = Vec::new();
        if let fetch_sub_issues::FetchSubIssuesNode::Issue(issue) = node
            && let Some(nodes) = issue.sub_issues.nodes
        {
            for n in nodes.into_iter().flatten() {
                let state = match n.state {
                    fetch_sub_issues::IssueState::CLOSED => IssueState::Closed,
                    _ => IssueState::Open,
                };
                sub_issues.push(SubIssueRef {
                    id: n.id,
                    number: n.number as i32,
                    title: n.title,
                    state,
                    url: Some(n.url),
                });
            }
        }
        Ok(sub_issues)
    }

    pub async fn add_comment(
        &self,
        subject_id: &str,
        body: &str,
    ) -> anyhow::Result<Comment> {
        let vars = add_comment::Variables {
            subject_id: subject_id.to_string(),
            body: body.to_string(),
        };
        let data = self.query::<AddComment>(vars).await?;
        let node = data
            .add_comment
            .and_then(|ac| ac.comment_edge)
            .and_then(|e| e.node)
            .context("Failed to get added comment")?;

        let reactions = node
            .reaction_groups
            .as_ref()
            .map(|gs| {
                gs.iter()
                    .filter_map(|g| {
                        Some(ReactionSummary {
                            content: g.content.to_model()?,
                            count: g.reactors.total_count as usize,
                            viewer_has_reacted: g.viewer_has_reacted,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Comment {
            id: node.id,
            author: node
                .author
                .as_ref()
                .map(|a| a.login.clone())
                .unwrap_or_else(|| "ghost".into()),
            body: node.body,
            created_at: node.created_at,
            reactions,
        })
    }

    pub async fn update_comment(
        &self,
        comment_id: &str,
        body: &str,
    ) -> anyhow::Result<Comment> {
        let vars = update_issue_comment::Variables {
            id: comment_id.to_string(),
            body: body.to_string(),
        };
        let data = self.query::<UpdateIssueComment>(vars).await?;
        let node = data
            .update_issue_comment
            .and_then(|u| u.issue_comment)
            .context("Failed to get updated comment")?;

        Ok(Comment {
            id: node.id,
            author: String::new(), // Will be filled by caller
            body: node.body,
            created_at: String::new(), // Will be filled by caller
            reactions: Vec::new(),
        })
    }

    pub async fn add_reaction(
        &self,
        subject_id: &str,
        content: ReactionContent,
    ) -> anyhow::Result<()> {
        let vars = add_reaction_mutation::Variables {
            subject_id: subject_id.to_string(),
            content: reaction_content_to_add_graphql(content),
        };
        let _ = self.query::<AddReactionMutation>(vars).await?;
        Ok(())
    }

    pub async fn remove_reaction(
        &self,
        subject_id: &str,
        content: ReactionContent,
    ) -> anyhow::Result<()> {
        let vars = remove_reaction_mutation::Variables {
            subject_id: subject_id.to_string(),
            content: reaction_content_to_remove_graphql(content),
        };
        let _ = self.query::<RemoveReactionMutation>(vars).await?;
        Ok(())
    }
}

fn reaction_content_to_add_graphql(
    c: ReactionContent,
) -> add_reaction_mutation::ReactionContent {
    use add_reaction_mutation::ReactionContent as E;
    match c {
        ReactionContent::ThumbsUp => E::THUMBS_UP,
        ReactionContent::ThumbsDown => E::THUMBS_DOWN,
        ReactionContent::Laugh => E::LAUGH,
        ReactionContent::Hooray => E::HOORAY,
        ReactionContent::Confused => E::CONFUSED,
        ReactionContent::Heart => E::HEART,
        ReactionContent::Rocket => E::ROCKET,
        ReactionContent::Eyes => E::EYES,
    }
}

fn reaction_content_to_remove_graphql(
    c: ReactionContent,
) -> remove_reaction_mutation::ReactionContent {
    use remove_reaction_mutation::ReactionContent as E;
    match c {
        ReactionContent::ThumbsUp => E::THUMBS_UP,
        ReactionContent::ThumbsDown => E::THUMBS_DOWN,
        ReactionContent::Laugh => E::LAUGH,
        ReactionContent::Hooray => E::HOORAY,
        ReactionContent::Confused => E::CONFUSED,
        ReactionContent::Heart => E::HEART,
        ReactionContent::Rocket => E::ROCKET,
        ReactionContent::Eyes => E::EYES,
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
    preferred_group_by_field_name: Option<&str>,
) -> anyhow::Result<Board> {
    let field_nodes = field_nodes.unwrap_or_default();

    // 全 SingleSelect / Iteration を field_definitions に格納 (Status も含める)。
    // Status 特別扱いは廃止: Column への分配は後段で grouping に基づき一括処理。
    let mut field_definitions: Vec<FieldDefinition> = Vec::new();

    for node in field_nodes.into_iter().flatten() {
        match node {
            FieldNodes::ProjectV2SingleSelectField(ssf) => {
                field_definitions.push(FieldDefinition::SingleSelect {
                    id: ssf.id,
                    name: ssf.name,
                    options: ssf
                        .options
                        .into_iter()
                        .map(|o| SingleSelectOption {
                            id: o.id,
                            name: o.name,
                            color: convert_column_color(&o.color),
                        })
                        .collect(),
                });
            }
            FieldNodes::ProjectV2Field(f) => {
                use project_board::ProjectV2FieldType;
                let name = f.name;
                let id = f.id;
                let def = match f.data_type {
                    ProjectV2FieldType::TEXT => FieldDefinition::Text { id, name },
                    ProjectV2FieldType::NUMBER => FieldDefinition::Number { id, name },
                    ProjectV2FieldType::DATE => FieldDefinition::Date { id, name },
                    // TITLE/ASSIGNEES/LABELS/MILESTONE/LINKED_PULL_REQUESTS/REPOSITORY/REVIEWERS
                    // 等の組み込みフィールドはスキップ
                    _ => continue,
                };
                field_definitions.push(def);
            }
            FieldNodes::ProjectV2IterationField(f) => {
                let completed = f
                    .configuration
                    .completed_iterations
                    .into_iter()
                    .map(|it| IterationOption {
                        id: it.id,
                        title: it.title,
                        start_date: it.start_date,
                        duration: it.duration as i32,
                        completed: true,
                    });
                let upcoming = f
                    .configuration
                    .iterations
                    .into_iter()
                    .map(|it| IterationOption {
                        id: it.id,
                        title: it.title,
                        start_date: it.start_date,
                        duration: it.duration as i32,
                        completed: false,
                    });
                let mut iterations: Vec<IterationOption> = completed.chain(upcoming).collect();
                iterations.sort_by(|a, b| a.start_date.cmp(&b.start_date));
                field_definitions.push(FieldDefinition::Iteration {
                    id: f.id,
                    name: f.name,
                    iterations,
                });
            }
        }
    }

    // 全カードの custom_fields を抽出 (Status 分も含めて等しく格納)
    let mut cards: Vec<Card> = Vec::with_capacity(items.len());
    for item in items {
        let mut card = convert_item(&item);

        let fv_nodes = item.field_values.nodes.unwrap_or_default();
        for fv in fv_nodes.iter().flatten() {
            match fv {
                FVNode::ProjectV2ItemFieldSingleSelectValue(sv) => {
                    if let SSValueField::ProjectV2SingleSelectField(f) = &sv.field
                        && let Some(option_id) = sv.option_id.clone()
                    {
                        let def = field_definitions.iter().find(|d| d.id() == f.id);
                        let Some(FieldDefinition::SingleSelect { name: field_name, options, .. }) = def else {
                            continue;
                        };
                        let color = options
                            .iter()
                            .find(|o| o.id == option_id)
                            .and_then(|o| o.color.clone());
                        card.custom_fields.push(CustomFieldValue::SingleSelect {
                            field_id: f.id.clone(),
                            field_name: field_name.clone(),
                            option_id,
                            name: sv.name.clone().unwrap_or_default(),
                            color,
                        });
                    }
                }
                FVNode::ProjectV2ItemFieldTextValue(tv) => {
                    use project_board::ProjectBoardNodeOnProjectV2ItemsNodesFieldValuesNodesOnProjectV2ItemFieldTextValueField as TField;
                    if let TField::ProjectV2Field(f) = &tv.field
                        && let Some(FieldDefinition::Text { name: field_name, .. }) =
                            field_definitions.iter().find(|d| d.id() == f.id)
                    {
                        card.custom_fields.push(CustomFieldValue::Text {
                            field_id: f.id.clone(),
                            field_name: field_name.clone(),
                            text: tv.text.clone().unwrap_or_default(),
                        });
                    }
                }
                FVNode::ProjectV2ItemFieldNumberValue(nv) => {
                    use project_board::ProjectBoardNodeOnProjectV2ItemsNodesFieldValuesNodesOnProjectV2ItemFieldNumberValueField as NField;
                    if let NField::ProjectV2Field(f) = &nv.field
                        && let Some(n) = nv.number
                        && let Some(FieldDefinition::Number { name: field_name, .. }) =
                            field_definitions.iter().find(|d| d.id() == f.id)
                    {
                        card.custom_fields.push(CustomFieldValue::Number {
                            field_id: f.id.clone(),
                            field_name: field_name.clone(),
                            number: n,
                        });
                    }
                }
                FVNode::ProjectV2ItemFieldDateValue(dv) => {
                    use project_board::ProjectBoardNodeOnProjectV2ItemsNodesFieldValuesNodesOnProjectV2ItemFieldDateValueField as DField;
                    if let DField::ProjectV2Field(f) = &dv.field
                        && let Some(d) = dv.date.clone()
                        && let Some(FieldDefinition::Date { name: field_name, .. }) =
                            field_definitions.iter().find(|d| d.id() == f.id)
                    {
                        card.custom_fields.push(CustomFieldValue::Date {
                            field_id: f.id.clone(),
                            field_name: field_name.clone(),
                            date: d,
                        });
                    }
                }
                FVNode::ProjectV2ItemFieldIterationValue(iv) => {
                    use project_board::ProjectBoardNodeOnProjectV2ItemsNodesFieldValuesNodesOnProjectV2ItemFieldIterationValueField as IField;
                    if let IField::ProjectV2IterationField(f) = &iv.field
                        && let Some(FieldDefinition::Iteration { name: field_name, .. }) =
                            field_definitions.iter().find(|d| d.id() == f.id)
                    {
                        card.custom_fields.push(CustomFieldValue::Iteration {
                            field_id: f.id.clone(),
                            field_name: field_name.clone(),
                            iteration_id: iv.iteration_id.clone(),
                            title: iv.title.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        cards.push(card);
    }

    let grouping = choose_grouping(&field_definitions, preferred_group_by_field_name);
    let columns = build_columns_for_grouping(&grouping, &field_definitions, cards);

    Ok(Board {
        project_title,
        grouping,
        columns,
        repositories,
        field_definitions,
    })
}

/// `preferred_name` にマッチする groupable field があればそれを選び、
/// なければ "Status" → 最初の SingleSelect → 最初の Iteration の順でフォールバック。
pub fn choose_grouping(
    field_definitions: &[FieldDefinition],
    preferred_name: Option<&str>,
) -> Grouping {
    let match_by_name = |name: &str| {
        field_definitions.iter().find_map(|def| match def {
            FieldDefinition::SingleSelect { id, name: n, .. } if n == name => {
                Some(Grouping::SingleSelect {
                    field_id: id.clone(),
                    field_name: n.clone(),
                })
            }
            FieldDefinition::Iteration { id, name: n, .. } if n == name => {
                Some(Grouping::Iteration {
                    field_id: id.clone(),
                    field_name: n.clone(),
                })
            }
            _ => None,
        })
    };

    if let Some(name) = preferred_name
        && let Some(g) = match_by_name(name)
    {
        return g;
    }
    if let Some(g) = match_by_name("Status") {
        return g;
    }
    // fallback: 最初の SingleSelect → 最初の Iteration
    for def in field_definitions {
        match def {
            FieldDefinition::SingleSelect { id, name, .. } => {
                return Grouping::SingleSelect {
                    field_id: id.clone(),
                    field_name: name.clone(),
                };
            }
            FieldDefinition::Iteration { id, name, .. } => {
                return Grouping::Iteration {
                    field_id: id.clone(),
                    field_name: name.clone(),
                };
            }
            _ => {}
        }
    }
    Grouping::None
}

/// 指定した grouping とカード群から columns を構築する。
/// 先頭に "No <field_name>" カラム (`option_id` が空) を置き、対応する値を持たないカードを集める。
pub fn build_columns_for_grouping(
    grouping: &Grouping,
    field_definitions: &[FieldDefinition],
    cards: Vec<Card>,
) -> Vec<Column> {
    match grouping {
        Grouping::SingleSelect { field_id, field_name } => {
            let options = field_definitions.iter().find_map(|d| match d {
                FieldDefinition::SingleSelect { id, options, .. } if id == field_id => {
                    Some(options.clone())
                }
                _ => None,
            });
            let Some(options) = options else {
                return vec![no_value_column(field_name, cards)];
            };
            let mut columns: Vec<Column> = options
                .iter()
                .map(|opt| Column {
                    option_id: opt.id.clone(),
                    name: opt.name.clone(),
                    color: opt.color.clone(),
                    cards: Vec::new(),
                })
                .collect();
            let mut no_value: Vec<Card> = Vec::new();
            for card in cards {
                let matched = card.custom_fields.iter().find_map(|fv| match fv {
                    CustomFieldValue::SingleSelect {
                        field_id: fid,
                        option_id,
                        ..
                    } if fid == field_id => Some(option_id.clone()),
                    _ => None,
                });
                match matched {
                    Some(opt_id) => {
                        if let Some(col) = columns.iter_mut().find(|c| c.option_id == opt_id) {
                            col.cards.push(card);
                        } else {
                            no_value.push(card);
                        }
                    }
                    None => no_value.push(card),
                }
            }
            if !no_value.is_empty() {
                columns.insert(0, no_value_column(field_name, no_value));
            }
            columns
        }
        Grouping::Iteration { field_id, field_name } => {
            let iterations = field_definitions.iter().find_map(|d| match d {
                FieldDefinition::Iteration { id, iterations, .. } if id == field_id => {
                    Some(iterations.clone())
                }
                _ => None,
            });
            let Some(iterations) = iterations else {
                return vec![no_value_column(field_name, cards)];
            };
            let mut columns: Vec<Column> = iterations
                .iter()
                .map(|it| Column {
                    option_id: it.id.clone(),
                    name: format!("{} ({})", it.title, it.start_date),
                    color: None,
                    cards: Vec::new(),
                })
                .collect();
            let mut no_value: Vec<Card> = Vec::new();
            for card in cards {
                let matched = card.custom_fields.iter().find_map(|fv| match fv {
                    CustomFieldValue::Iteration {
                        field_id: fid,
                        iteration_id,
                        ..
                    } if fid == field_id => Some(iteration_id.clone()),
                    _ => None,
                });
                match matched {
                    Some(it_id) => {
                        if let Some(col) = columns.iter_mut().find(|c| c.option_id == it_id) {
                            col.cards.push(card);
                        } else {
                            no_value.push(card);
                        }
                    }
                    None => no_value.push(card),
                }
            }
            if !no_value.is_empty() {
                columns.insert(0, no_value_column(field_name, no_value));
            }
            columns
        }
        Grouping::None => {
            if cards.is_empty() {
                Vec::new()
            } else {
                vec![Column {
                    option_id: String::new(),
                    name: "No Grouping".to_string(),
                    color: None,
                    cards,
                }]
            }
        }
    }
}

fn no_value_column(field_name: &str, cards: Vec<Card>) -> Column {
    Column {
        option_id: String::new(),
        name: format!("No {field_name}"),
        color: None,
        cards,
    }
}

fn convert_column_color(
    color: &project_board::ProjectV2SingleSelectFieldOptionColor,
) -> Option<ColumnColor> {
    use project_board::ProjectV2SingleSelectFieldOptionColor::*;
    Some(match color {
        BLUE => ColumnColor::Blue,
        GRAY => ColumnColor::Gray,
        GREEN => ColumnColor::Green,
        ORANGE => ColumnColor::Orange,
        PINK => ColumnColor::Pink,
        PURPLE => ColumnColor::Purple,
        RED => ColumnColor::Red,
        YELLOW => ColumnColor::Yellow,
        Other(_) => return None,
    })
}

fn map_ci_status(state: &project_board::StatusState) -> CiStatus {
    match state {
        project_board::StatusState::SUCCESS => CiStatus::Success,
        project_board::StatusState::FAILURE => CiStatus::Failure,
        project_board::StatusState::PENDING => CiStatus::Pending,
        project_board::StatusState::ERROR => CiStatus::Error,
        project_board::StatusState::EXPECTED => CiStatus::Expected,
        _ => CiStatus::Pending,
    }
}

fn map_review_decision(d: &project_board::PullRequestReviewDecision) -> Option<ReviewDecision> {
    match d {
        project_board::PullRequestReviewDecision::APPROVED => Some(ReviewDecision::Approved),
        project_board::PullRequestReviewDecision::CHANGES_REQUESTED => {
            Some(ReviewDecision::ChangesRequested)
        }
        project_board::PullRequestReviewDecision::REVIEW_REQUIRED => {
            Some(ReviewDecision::ReviewRequired)
        }
        _ => None,
    }
}

fn build_pr_status(
    pr: &project_board::ProjectBoardNodeOnProjectV2ItemsNodesContentOnPullRequest,
) -> PrStatus {
    use project_board::ProjectBoardNodeOnProjectV2ItemsNodesContentOnPullRequestReviewRequestsNodesRequestedReviewer as Reviewer;

    let ci = pr
        .commits
        .nodes
        .as_ref()
        .and_then(|nodes| nodes.iter().flatten().next())
        .and_then(|c| c.commit.status_check_rollup.as_ref())
        .map(|r| map_ci_status(&r.state));

    let review_decision = pr.review_decision.as_ref().and_then(map_review_decision);

    let review_requests = pr
        .review_requests
        .as_ref()
        .and_then(|rr| rr.nodes.as_ref())
        .map(|nodes| {
            nodes
                .iter()
                .flatten()
                .filter_map(|req| req.requested_reviewer.as_ref())
                .filter_map(|r| match r {
                    Reviewer::User(u) => Some(u.login.clone()),
                    Reviewer::Team(t) => Some(format!("team/{}", t.name)),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    PrStatus {
        ci,
        review_decision,
        review_requests,
    }
}

fn convert_item(item: &ItemNode) -> Card {
    match &item.content {
        Some(Content::Issue(issue)) => Card {
            pr_status: None,
            linked_prs: Vec::new(),
            parent_issue: issue.parent.as_ref().map(|p| ParentIssueRef {
                id: p.id.clone(),
                number: p.number as i32,
                title: p.title.clone(),
                url: Some(p.url.clone()),
            }),
            sub_issues_summary: Some(SubIssuesSummary {
                completed: issue.sub_issues_summary.completed as i32,
                total: issue.sub_issues_summary.total as i32,
            }),
            sub_issues: Vec::new(),
            item_id: item.id.clone(),
            archived: item.is_archived,
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
            body: None,
            milestone: issue.milestone.as_ref().map(|m| m.title.clone()),
            comments: Vec::new(),
            custom_fields: Vec::new(),
            reactions: Vec::new(),
        },
        Some(Content::PullRequest(pr)) => Card {
            pr_status: Some(build_pr_status(pr)),
            linked_prs: Vec::new(),
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: Vec::new(),
            item_id: item.id.clone(),
            archived: item.is_archived,
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
            body: None,
            milestone: pr.milestone.as_ref().map(|m| m.title.clone()),
            comments: Vec::new(),
            custom_fields: Vec::new(),
            reactions: Vec::new(),
        },
        Some(Content::DraftIssue(draft)) => Card {
            pr_status: None,
            linked_prs: Vec::new(),
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: Vec::new(),
            item_id: item.id.clone(),
            archived: item.is_archived,
            content_id: Some(draft.id.clone()),
            title: draft.title.clone(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: Vec::new(),
            labels: Vec::new(),
            url: None,
            body: None,
            comments: Vec::new(),
            milestone: None,
            custom_fields: Vec::new(),
            reactions: Vec::new(),
        },
        None => Card {
            pr_status: None,
            linked_prs: Vec::new(),
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: Vec::new(),
            item_id: item.id.clone(),
            archived: item.is_archived,
            content_id: None,
            title: "(no content)".to_string(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: Vec::new(),
            labels: Vec::new(),
            url: None,
            body: None,
            comments: Vec::new(),
            milestone: None,
            custom_fields: Vec::new(),
            reactions: Vec::new(),
        },
    }
}
