use super::queries::*;
use crate::model::project::{
    CiStatus, ColumnColor, PrStatus, ReactionContent, ReactionSummary, ReviewDecision,
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

pub(super) trait ReactionContentFromGraphQL {
    fn to_model(&self) -> Option<ReactionContent>;
}

/// 各 GraphQL クエリが生成する ReactionGroup 型から ReactionSummary の Vec を作る共通処理。
/// extract には `|g| (&g.content, g.reactors.total_count, g.viewer_has_reacted)` を渡す。
pub(super) fn collect_reactions<G, C>(
    groups: Option<&Vec<G>>,
    extract: impl Fn(&G) -> (&C, i64, bool),
) -> Vec<ReactionSummary>
where
    C: ReactionContentFromGraphQL,
{
    groups
        .into_iter()
        .flatten()
        .filter_map(|g| {
            let (content, count, viewer) = extract(g);
            Some(ReactionSummary {
                content: content.to_model()?,
                count: count as usize,
                viewer_has_reacted: viewer,
            })
        })
        .collect()
}

impl_reaction_content_from!(fetch_comments::ReactionContent);
impl_reaction_content_from!(add_comment::ReactionContent);
impl_reaction_content_from!(add_reaction_mutation::ReactionContent);
impl_reaction_content_from!(remove_reaction_mutation::ReactionContent);
impl_reaction_content_from!(fetch_issue::ReactionContent);
impl_reaction_content_from!(fetch_card_detail::ReactionContent);

pub(super) fn reaction_content_to_add_graphql(
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

pub(super) fn reaction_content_to_remove_graphql(
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

pub(super) fn convert_column_color(
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

pub(super) fn map_ci_status(state: &project_board::StatusState) -> CiStatus {
    match state {
        project_board::StatusState::SUCCESS => CiStatus::Success,
        project_board::StatusState::FAILURE => CiStatus::Failure,
        project_board::StatusState::PENDING => CiStatus::Pending,
        project_board::StatusState::ERROR => CiStatus::Error,
        project_board::StatusState::EXPECTED => CiStatus::Expected,
        _ => CiStatus::Pending,
    }
}

pub(super) fn map_review_decision(
    d: &project_board::PullRequestReviewDecision,
) -> Option<ReviewDecision> {
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

pub(super) fn build_pr_status(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_ci_status_covers_known_variants() {
        assert_eq!(map_ci_status(&project_board::StatusState::SUCCESS), CiStatus::Success);
        assert_eq!(map_ci_status(&project_board::StatusState::FAILURE), CiStatus::Failure);
        assert_eq!(map_ci_status(&project_board::StatusState::PENDING), CiStatus::Pending);
        assert_eq!(map_ci_status(&project_board::StatusState::ERROR), CiStatus::Error);
        assert_eq!(map_ci_status(&project_board::StatusState::EXPECTED), CiStatus::Expected);
    }

    #[test]
    fn map_ci_status_unknown_falls_back_to_pending() {
        assert_eq!(
            map_ci_status(&project_board::StatusState::Other("Unknown".into())),
            CiStatus::Pending,
        );
    }

    #[test]
    fn map_review_decision_variants() {
        assert_eq!(
            map_review_decision(&project_board::PullRequestReviewDecision::APPROVED),
            Some(ReviewDecision::Approved),
        );
        assert_eq!(
            map_review_decision(&project_board::PullRequestReviewDecision::CHANGES_REQUESTED),
            Some(ReviewDecision::ChangesRequested),
        );
        assert_eq!(
            map_review_decision(&project_board::PullRequestReviewDecision::REVIEW_REQUIRED),
            Some(ReviewDecision::ReviewRequired),
        );
        assert_eq!(
            map_review_decision(&project_board::PullRequestReviewDecision::Other("X".into())),
            None,
        );
    }

    #[test]
    fn convert_column_color_known() {
        use project_board::ProjectV2SingleSelectFieldOptionColor as C;
        assert_eq!(convert_column_color(&C::BLUE), Some(ColumnColor::Blue));
        assert_eq!(convert_column_color(&C::GRAY), Some(ColumnColor::Gray));
        assert_eq!(convert_column_color(&C::GREEN), Some(ColumnColor::Green));
        assert_eq!(convert_column_color(&C::ORANGE), Some(ColumnColor::Orange));
        assert_eq!(convert_column_color(&C::PINK), Some(ColumnColor::Pink));
        assert_eq!(convert_column_color(&C::PURPLE), Some(ColumnColor::Purple));
        assert_eq!(convert_column_color(&C::RED), Some(ColumnColor::Red));
        assert_eq!(convert_column_color(&C::YELLOW), Some(ColumnColor::Yellow));
    }

    #[test]
    fn convert_column_color_other_is_none() {
        assert_eq!(
            convert_column_color(&project_board::ProjectV2SingleSelectFieldOptionColor::Other(
                "X".into(),
            )),
            None,
        );
    }

    #[test]
    fn reaction_content_round_trip_add() {
        let all = [
            ReactionContent::ThumbsUp,
            ReactionContent::ThumbsDown,
            ReactionContent::Laugh,
            ReactionContent::Hooray,
            ReactionContent::Confused,
            ReactionContent::Heart,
            ReactionContent::Rocket,
            ReactionContent::Eyes,
        ];
        for c in all {
            let graphql = reaction_content_to_add_graphql(c);
            assert_eq!(graphql.to_model(), Some(c));
        }
    }

    #[test]
    fn reaction_content_round_trip_remove() {
        let all = [
            ReactionContent::ThumbsUp,
            ReactionContent::ThumbsDown,
            ReactionContent::Laugh,
            ReactionContent::Hooray,
            ReactionContent::Confused,
            ReactionContent::Heart,
            ReactionContent::Rocket,
            ReactionContent::Eyes,
        ];
        for c in all {
            let graphql = reaction_content_to_remove_graphql(c);
            assert_eq!(graphql.to_model(), Some(c));
        }
    }
}
