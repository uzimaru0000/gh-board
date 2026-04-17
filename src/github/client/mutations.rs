use anyhow::Context;

use super::super::convert::{
    collect_reactions, reaction_content_to_add_graphql, reaction_content_to_remove_graphql,
};
use super::super::queries::*;
use super::GitHubClient;
use crate::command::CustomFieldValueInput;
use crate::model::project::{Comment, ReactionContent};

impl GitHubClient {
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

        let reactions = collect_reactions(
            node.reaction_groups.as_ref(),
            |g| (&g.content, g.reactors.total_count, g.viewer_has_reacted),
        );
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
