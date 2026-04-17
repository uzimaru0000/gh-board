use anyhow::{bail, Context};

use super::super::convert::collect_reactions;
use super::super::queries::*;
use super::{build_board, GitHubClient};
use crate::model::project::{
    Board, Card, CardDetail, CardType, Comment, IssueState, Label, LinkedPr, PaginationState,
    ParentIssueRef, PrState, ProjectSummary, ReactionSummary, Repository, SubIssueRef,
    SubIssuesSummary,
};

// Type aliases for readability (shared with mod.rs)
use project_board::{
    ProjectBoardNode,
    ProjectBoardNodeOnProjectV2ItemsNodes as ItemNode,
};

impl GitHubClient {
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
                url: p.url,
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
                        url: p.url,
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
                        url: p.url,
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
            url: project.url,
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
                    url: project.url,
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
                    url: project.url,
                })
            }
        }
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

    /// アーカイブを含む全アイテムを返す内部 API。`get_board` のベースとなる。
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
                            let reactions = collect_reactions(
                                c.reaction_groups.as_ref(),
                                |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
                            );
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
                            let reactions = collect_reactions(
                                c.reaction_groups.as_ref(),
                                |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
                            );
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
                        reactions: collect_reactions(
                            c.reaction_groups.as_ref(),
                            |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
                        ),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let reactions = collect_reactions(
            issue.reaction_groups.as_ref(),
            |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
        );
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
                                reactions: collect_reactions(
                                    c.reaction_groups.as_ref(),
                                    |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
                                ),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let reactions = collect_reactions(
                    pr.reaction_groups.as_ref(),
                    |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
                );
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
                        reactions: collect_reactions(
                            c.reaction_groups.as_ref(),
                            |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
                        ),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn convert_card_detail_reactions(
        groups: &Option<Vec<fetch_card_detail::FetchCardDetailNodeOnIssueReactionGroups>>,
    ) -> Vec<ReactionSummary> {
        collect_reactions(
            groups.as_ref(),
            |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
        )
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
}
