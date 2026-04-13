use graphql_client::GraphQLQuery;

type URI = String;
type DateTime = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/viewer_projects.graphql",
    response_derives = "Debug"
)]
pub struct ViewerProjects;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/org_projects.graphql",
    response_derives = "Debug"
)]
pub struct OrgProjects;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/user_projects.graphql",
    response_derives = "Debug"
)]
pub struct UserProjects;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/project_board.graphql",
    response_derives = "Debug"
)]
pub struct ProjectBoard;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/move_card.graphql",
    response_derives = "Debug"
)]
pub struct MoveCard;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/delete_card.graphql",
    response_derives = "Debug"
)]
pub struct DeleteCard;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/create_draft_issue.graphql",
    response_derives = "Debug"
)]
pub struct CreateDraftIssue;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/create_issue.graphql",
    response_derives = "Debug"
)]
pub struct CreateIssue;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/add_project_item.graphql",
    response_derives = "Debug"
)]
pub struct AddProjectItem;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/reorder_card.graphql",
    response_derives = "Debug"
)]
pub struct ReorderCard;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/repo_labels.graphql",
    response_derives = "Debug"
)]
pub struct RepoLabels;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/assignable_users.graphql",
    response_derives = "Debug"
)]
pub struct AssignableUsers;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/add_labels.graphql",
    response_derives = "Debug"
)]
pub struct AddLabels;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/remove_labels.graphql",
    response_derives = "Debug"
)]
pub struct RemoveLabels;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/add_assignees.graphql",
    response_derives = "Debug"
)]
pub struct AddAssignees;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/remove_assignees.graphql",
    response_derives = "Debug"
)]
pub struct RemoveAssignees;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/update_draft_issue.graphql",
    response_derives = "Debug"
)]
pub struct UpdateDraftIssue;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/update_issue.graphql",
    response_derives = "Debug"
)]
pub struct UpdateIssue;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/update_pull_request.graphql",
    response_derives = "Debug"
)]
pub struct UpdatePullRequest;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/fetch_comments.graphql",
    response_derives = "Debug"
)]
pub struct FetchComments;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/add_comment.graphql",
    response_derives = "Debug"
)]
pub struct AddComment;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/update_issue_comment.graphql",
    response_derives = "Debug"
)]
pub struct UpdateIssueComment;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/github/graphql/viewer_login.graphql",
    response_derives = "Debug"
)]
pub struct ViewerLogin;
