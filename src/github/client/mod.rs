use anyhow::{bail, Context};
use graphql_client::GraphQLQuery;
use tokio::process::Command;

use super::convert::{build_pr_status, convert_column_color};
use super::queries::*;
use crate::model::project::{
    Board, Card, CardType, Column, CustomFieldValue, FieldDefinition, Grouping, IssueState,
    IterationOption, Label, ParentIssueRef, PrState, Repository, SingleSelectOption,
    SubIssuesSummary,
};

mod mutations;
mod queries;

// Type aliases for readability
use project_board::{
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

pub(super) fn build_board(
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
