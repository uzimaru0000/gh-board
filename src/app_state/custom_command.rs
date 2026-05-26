use crossterm::event::KeyEvent;

use crate::action::Action;
use crate::command::Command;
use crate::config::CustomCommandConfig;
use crate::keymap::KeymapMode;
use crate::model::project::{Card, CardType, ProjectSummary};
use crate::model::state::{Scene, ViewMode};

use super::AppState;

/// `https://github.com/<owner>/<repo>/issues/<n>` のような URL から (owner, repo) を抽出する。
/// 形式に合致しない場合 None。
pub(crate) fn parse_owner_repo_from_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let mut parts = rest.splitn(3, '/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// `command` 内の `{key}` placeholder を展開する。未対応の key は元の `{key}` 表記のまま残す。
/// `card` が `None` の場合、カード関連 placeholder は空文字に展開される。
pub(crate) fn expand_placeholders(
    template: &str,
    card: Option<&Card>,
    project: Option<&ProjectSummary>,
) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{'
            && let Some(close) = template[i + 1..].find('}')
            && let Some(value) = lookup_placeholder(&template[i + 1..i + 1 + close], card, project)
        {
            out.push_str(&value);
            i += 1 + close + 1;
            continue;
        }
        let ch_end = next_char_end(template, i);
        out.push_str(&template[i..ch_end]);
        i = ch_end;
    }
    out
}

fn next_char_end(s: &str, start: usize) -> usize {
    let mut p = start + 1;
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}

fn lookup_placeholder(
    key: &str,
    card: Option<&Card>,
    project: Option<&ProjectSummary>,
) -> Option<String> {
    let owner_repo = card
        .and_then(|c| c.url.as_deref())
        .and_then(parse_owner_repo_from_url);

    match key {
        "number" => Some(
            card.and_then(|c| c.number)
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ),
        "title" => Some(card.map(|c| c.title.clone()).unwrap_or_default()),
        "url" => Some(card.and_then(|c| c.url.clone()).unwrap_or_default()),
        "body" => Some(card.and_then(|c| c.body.clone()).unwrap_or_default()),
        "id" | "content_id" => Some(
            card.and_then(|c| c.content_id.clone())
                .unwrap_or_default(),
        ),
        "item_id" => Some(card.map(|c| c.item_id.clone()).unwrap_or_default()),
        "owner" => Some(
            owner_repo
                .as_ref()
                .map(|(o, _)| o.clone())
                .unwrap_or_default(),
        ),
        "repo" => Some(
            owner_repo
                .as_ref()
                .map(|(_, r)| r.clone())
                .unwrap_or_default(),
        ),
        "name_with_owner" => Some(
            owner_repo
                .as_ref()
                .map(|(o, r)| format!("{o}/{r}"))
                .unwrap_or_default(),
        ),
        "card_type" => Some(card_type_name(card).to_string()),
        "project_number" => Some(
            project
                .map(|p| p.number.to_string())
                .unwrap_or_default(),
        ),
        "project_title" => Some(project.map(|p| p.title.clone()).unwrap_or_default()),
        "project_url" => Some(project.map(|p| p.url.clone()).unwrap_or_default()),
        _ => None,
    }
}

fn card_type_name(card: Option<&Card>) -> &'static str {
    match card.map(|c| &c.card_type) {
        Some(CardType::Issue { .. }) => "issue",
        Some(CardType::PullRequest { .. }) => "pull_request",
        Some(CardType::DraftIssue) => "draft_issue",
        None => "",
    }
}

impl AppState {
    /// `[[command]]` を AppState にセットする。
    pub fn set_custom_commands(&mut self, commands: Vec<CustomCommandConfig>) {
        self.custom_commands = commands;
    }

    /// 現在の選択カード (detail スタック優先) と project を元に、`commands[idx]` を展開して
    /// `Command::RunCustomCommand` を返す。
    /// `idx` が範囲外の場合は `Command::None` を返す。
    pub fn run_custom_command(&self, idx: usize) -> Command {
        let Some(cfg) = self.custom_commands.get(idx) else {
            return Command::None;
        };
        let card = self
            .current_detail_card()
            .or_else(|| self.selected_card_ref());
        let project = self.current_project.as_ref();
        let command_line = expand_placeholders(&cfg.command, card, project);
        let post_command_line = cfg
            .post_command
            .as_ref()
            .map(|c| expand_placeholders(c, card, project));
        Command::RunCustomCommand {
            name: cfg.name.clone(),
            command_line,
            post_command_line,
            interactive: cfg.interactive,
            pause_after: cfg.pause_after,
        }
    }

    /// コマンドパレットを開く。`return_to` には呼び出し元の ViewMode (Board / Detail / CommentList) を渡す。
    /// custom_commands が空なら何もしない (toast でユーザに伝える)。
    pub(crate) fn open_command_palette(&mut self, return_to: ViewMode) -> Command {
        if self.custom_commands.is_empty() {
            self.toast = Some("No custom commands defined in config.toml".into());
            return Command::None;
        }
        self.command_palette_cursor = 0;
        self.command_palette_return_to = return_to;
        self.mode = ViewMode::CommandPalette;
        self.scene = Scene::CommandPalette;
        Command::None
    }

    /// コマンドパレットを閉じて元の ViewMode に戻る。
    pub(crate) fn close_command_palette(&mut self) {
        let return_to = self.command_palette_return_to.clone();
        self.mode = return_to.clone();
        // Detail / CommentList は state を持つので、現状の scene を維持できるよう
        // mode タグから派生させる (Detail/Board のみ。CommentList は呼び出し元が
        // 必ず state 経由で復帰させる仕様)。
        self.scene = match return_to {
            ViewMode::Detail => Scene::Detail,
            ViewMode::Board => Scene::Board,
            // CommentList は state 必須なので一旦 Board に戻す (現状の運用上不到達)。
            _ => Scene::Board,
        };
    }

    pub(super) fn handle_command_palette_key(&mut self, key: KeyEvent) -> Command {
        let action = match self.keymap.resolve(KeymapMode::CommandPalette, &key) {
            Some(a) => a,
            None => return Command::None,
        };
        let len = self.custom_commands.len();
        match action {
            Action::Back | Action::Quit => {
                self.close_command_palette();
                Command::None
            }
            Action::MoveDown => {
                if len > 0 {
                    self.command_palette_cursor =
                        (self.command_palette_cursor + 1).min(len - 1);
                }
                Command::None
            }
            Action::MoveUp => {
                self.command_palette_cursor = self.command_palette_cursor.saturating_sub(1);
                Command::None
            }
            Action::Select => {
                let idx = self.command_palette_cursor;
                self.close_command_palette();
                self.run_custom_command(idx)
            }
            _ => Command::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::{Card, CardType, IssueState};

    fn make_issue_card() -> Card {
        Card {
            item_id: "ITEM_1".into(),
            content_id: Some("CONTENT_1".into()),
            title: "Bug: foo".into(),
            number: Some(42),
            card_type: CardType::Issue {
                state: IssueState::Open,
            },
            assignees: vec![],
            labels: vec![],
            url: Some("https://github.com/acme/widget/issues/42".into()),
            body: Some("Some body".into()),
            comments: vec![],
            milestone: None,
            custom_fields: vec![],
            pr_status: None,
            linked_prs: vec![],
            reactions: vec![],
            archived: false,
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: vec![],
        }
    }

    fn make_draft_card() -> Card {
        Card {
            item_id: "ITEM_DRAFT".into(),
            content_id: Some("DRAFT_CID".into()),
            title: "Draft".into(),
            number: None,
            card_type: CardType::DraftIssue,
            assignees: vec![],
            labels: vec![],
            url: None,
            body: None,
            comments: vec![],
            milestone: None,
            custom_fields: vec![],
            pr_status: None,
            linked_prs: vec![],
            reactions: vec![],
            archived: false,
            parent_issue: None,
            sub_issues_summary: None,
            sub_issues: vec![],
        }
    }

    fn make_project() -> ProjectSummary {
        ProjectSummary {
            id: "PRJ".into(),
            title: "My Project".into(),
            number: 7,
            description: None,
            url: "https://github.com/users/acme/projects/7".into(),
        }
    }

    #[test]
    fn parse_owner_repo_basic() {
        assert_eq!(
            parse_owner_repo_from_url("https://github.com/acme/widget/issues/42"),
            Some(("acme".into(), "widget".into()))
        );
        assert_eq!(
            parse_owner_repo_from_url("https://github.com/acme/widget/pull/100"),
            Some(("acme".into(), "widget".into()))
        );
    }

    #[test]
    fn parse_owner_repo_invalid() {
        assert_eq!(parse_owner_repo_from_url(""), None);
        assert_eq!(parse_owner_repo_from_url("https://example.com/a/b"), None);
        assert_eq!(parse_owner_repo_from_url("not-a-url"), None);
    }

    #[test]
    fn expand_basic_card_fields() {
        let card = make_issue_card();
        let project = make_project();
        let out = expand_placeholders(
            "echo number={number} title={title} url={url} repo={repo} owner={owner}",
            Some(&card),
            Some(&project),
        );
        assert_eq!(
            out,
            "echo number=42 title=Bug: foo url=https://github.com/acme/widget/issues/42 repo=widget owner=acme"
        );
    }

    #[test]
    fn expand_name_with_owner() {
        let card = make_issue_card();
        let out = expand_placeholders("repo={name_with_owner}", Some(&card), None);
        assert_eq!(out, "repo=acme/widget");
    }

    #[test]
    fn expand_id_and_item_id() {
        let card = make_issue_card();
        let out = expand_placeholders("id={id} item={item_id}", Some(&card), None);
        assert_eq!(out, "id=CONTENT_1 item=ITEM_1");
    }

    #[test]
    fn expand_with_draft_card_uses_empty_strings() {
        let card = make_draft_card();
        let out = expand_placeholders(
            "n={number} url={url} owner={owner} repo={repo}",
            Some(&card),
            None,
        );
        assert_eq!(out, "n= url= owner= repo=");
    }

    #[test]
    fn expand_unknown_placeholder_left_intact() {
        let card = make_issue_card();
        // 未知の placeholder は `{...}` を含めて残す (将来拡張用)
        let out = expand_placeholders("a={number} b={mystery}", Some(&card), None);
        assert_eq!(out, "a=42 b={mystery}");
    }

    #[test]
    fn expand_without_card_returns_empty_for_card_fields() {
        let project = make_project();
        let out = expand_placeholders(
            "n={number} t={title} p={project_number}",
            None,
            Some(&project),
        );
        assert_eq!(out, "n= t= p=7");
    }

    #[test]
    fn expand_complex_shell_example() {
        let card = make_issue_card();
        let template = "git worktree add ../{repo}.resolve-{number} -b resolve-{number} && cd ../{repo}.resolve-{number} && claude '{url}'";
        let out = expand_placeholders(template, Some(&card), None);
        assert_eq!(
            out,
            "git worktree add ../widget.resolve-42 -b resolve-42 && cd ../widget.resolve-42 && claude 'https://github.com/acme/widget/issues/42'"
        );
    }

    #[test]
    fn expand_card_type() {
        let card = make_issue_card();
        let out = expand_placeholders("{card_type}", Some(&card), None);
        assert_eq!(out, "issue");
        let card = make_draft_card();
        let out = expand_placeholders("{card_type}", Some(&card), None);
        assert_eq!(out, "draft_issue");
    }

    #[test]
    fn expand_project_fields() {
        let project = make_project();
        let out = expand_placeholders(
            "#{project_number} {project_title} ({project_url})",
            None,
            Some(&project),
        );
        assert_eq!(
            out,
            "#7 My Project (https://github.com/users/acme/projects/7)"
        );
    }

    #[test]
    fn expand_brace_without_close_left_intact() {
        let card = make_issue_card();
        let out = expand_placeholders("hello {number broken", Some(&card), None);
        assert_eq!(out, "hello {number broken");
    }
}
