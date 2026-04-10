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
