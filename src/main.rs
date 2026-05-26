mod action;
mod app;
mod app_state;
mod cache;
mod cli;
mod clipboard;
mod color;
mod command;
mod config;
mod event;
mod github;
mod i18n;
mod keymap;
mod model;
mod ui;

rust_i18n::i18n!("locales", fallback = "en");

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    DefaultTerminal, Frame,
};

use ui::theme::theme;

use app::App;
use event::EventHandler;
use github::client::GitHubClient;
use model::state::{LoadingState, Scene, ViewMode};

#[derive(Parser)]
#[command(name = "gh-board", about = "View GitHub Projects V2 as a kanban board", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<cli::CliCommand>,

    /// Project number to open directly
    number: Option<i32>,

    /// Login of the owner. Use "@me" for the current user.
    #[arg(long)]
    owner: Option<String>,

    /// Disable the local board cache for this run.
    #[arg(long)]
    no_cache: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // CLI subcommand mode: no TUI initialization
    if let Some(cmd) = cli.command {
        let github = GitHubClient::new().await?;
        if let Err(e) = cli::run(cmd, github).await {
            let msg = serde_json::json!({ "error": format!("{e:#}") });
            eprintln!("{}", serde_json::to_string_pretty(&msg).unwrap_or_default());
            std::process::exit(1);
        }
        return Ok(());
    }

    // TUI mode
    i18n::init();
    let cfg = config::load_config().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}");
        config::Config::default()
    });
    let theme_cfg = config::load_theme_file().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load theme: {e}");
        None
    });
    let default_theme = config::ThemeConfig::default();
    ui::theme::init_theme(theme_cfg.as_ref().unwrap_or(&default_theme));

    let github = GitHubClient::new().await?;

    let mut terminal = ratatui::init();
    // マウスキャプチャを有効化 (ratatui::init は raw_mode と AlternateScreen のみ)
    let _ = crossterm::execute!(std::io::stdout(), EnableMouseCapture);
    let result = run(&mut terminal, github, cli, cfg).await;
    let _ = crossterm::execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();

    result
}

async fn run(terminal: &mut DefaultTerminal, github: GitHubClient, cli: Cli, cfg: config::Config) -> Result<()> {
    let (mut events, event_tx) = EventHandler::new(Duration::from_millis(80));

    // "@me" means the current viewer (same as no owner)
    let owner = cli.owner.filter(|o| o != "@me");

    let cache = if cli.no_cache {
        cache::DiskCache::disabled()
    } else {
        cache::DiskCache::new()
    };
    let mut app = App::new(github, event_tx, owner.clone(), cache);
    app.state.set_views(cfg.view);
    app.state.preferred_grouping_field_name = cfg.board.group_by.clone();

    let keymap = keymap::Keymap::default_keymap()
        .with_overrides(&cfg.keys)
        .register_custom_commands(&cfg.commands);
    app.state.set_keymap(keymap);
    app.state.set_custom_commands(cfg.commands);

    // When project number is specified, load that project directly (skip project list)
    if let Some(number) = cli.number {
        app.load_project_by_number(owner, number);
    } else {
        app.load_projects();
    }

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        if let Some(event) = events.next().await {
            app.handle_event(event);
        }

        // $EDITOR でボディ編集
        if let Some(content) = app.pending_editor.take() {
            events.pause();
            disable_raw_mode()?;
            crossterm::execute!(std::io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;

            let result = run_editor(&content);

            enable_raw_mode()?;
            crossterm::execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
            terminal.clear()?;
            events.resume();

            if let Ok(new_body) = result {
                if let Some(ctx) = app.pending_comment_editor.take() {
                    // コメント用エディタの結果
                    if !new_body.trim().is_empty() {
                        let cmd = if let Some(comment_id) = ctx.comment_id {
                            crate::command::Command::UpdateComment {
                                comment_id,
                                body: new_body,
                            }
                        } else {
                            crate::command::Command::AddComment {
                                subject_id: ctx.content_id,
                                body: new_body,
                            }
                        };
                        app.handle_event(event::AppEvent::Tick);
                        app.execute_cmd(cmd);
                    }
                } else {
                    match app.state.mode {
                        ViewMode::EditCard => {
                            if let Some(s) = app.state.edit_card_state_mut() {
                                s.body_input = new_body;
                            }
                        }
                        _ => {
                            app.state.create_card_state.body_input = new_body;
                        }
                    }
                }
            }
        }

        // [[command]] でユーザ定義された interactive コマンドを実行
        if let Some(pending) = app.pending_custom_command.take() {
            events.pause();
            disable_raw_mode()?;
            crossterm::execute!(std::io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;

            let status = run_custom_command(&pending.command_line);

            // command が成功した場合のみ post_command を続けて foreground 実行する (cleanup)。
            let post_status = match (&status, &pending.post_command_line) {
                (Ok(s), Some(post)) if s.success() => Some(run_custom_command(post)),
                _ => None,
            };

            // pause_after が指定されていれば、出力を残したままキー入力を待つ。
            // この時点では raw mode 無効 + 通常画面なのでコマンド出力が見えている。
            if pending.pause_after {
                wait_for_key()?;
            }

            enable_raw_mode()?;
            crossterm::execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
            terminal.clear()?;
            events.resume();

            app.state.toast = Some(custom_command_toast(&pending.name, status, post_status));
        }

        if app.state.should_quit {
            break;
        }
    }

    Ok(())
}

/// custom command (+ post_command) の実行結果から status line に出す toast を組み立てる。
/// post_command が失敗した場合は command 成功でも警告を出す。
fn custom_command_toast(
    name: &str,
    status: std::io::Result<std::process::ExitStatus>,
    post_status: Option<std::io::Result<std::process::ExitStatus>>,
) -> String {
    match status {
        Ok(s) if s.success() => match post_status {
            Some(Ok(p)) if p.success() => format!("✓ {name}"),
            Some(Ok(p)) => format!("✗ {name}: post-command failed (exit {})", p.code().unwrap_or(-1)),
            Some(Err(e)) => format!("✗ {name}: post-command error: {e}"),
            None => format!("✓ {name}"),
        },
        Ok(s) => format!("✗ {name} (exit {})", s.code().unwrap_or(-1)),
        Err(e) => format!("✗ {name}: {e}"),
    }
}

/// custom command の出力を残したまま、ユーザのキー入力を1つ待つ。
/// 呼び出し時点では raw mode は無効・通常画面の想定。一時的に raw mode を有効化して
/// 1 キー押下まで待ち、元に戻す (呼び出し元が直後に enable_raw_mode を行う)。
fn wait_for_key() -> std::io::Result<()> {
    use std::io::Write;

    print!("\r\n\x1b[1;36m[gh-board]\x1b[0m Press any key to return to the board...");
    std::io::stdout().flush()?;

    enable_raw_mode()?;
    // Key 押下イベントが来るまで待つ (Resize/Mouse 等は無視)。
    loop {
        match crossterm::event::read() {
            Ok(crossterm::event::Event::Key(_)) => break,
            Ok(_) => continue,
            Err(e) => {
                let _ = disable_raw_mode();
                return Err(e);
            }
        }
    }
    disable_raw_mode()?;
    Ok(())
}

fn run_custom_command(command_line: &str) -> std::io::Result<std::process::ExitStatus> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(command_line)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
}

fn run_editor(content: &str) -> Result<String> {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("gh-board-{}.md", std::process::id()));
    std::fs::write(&path, content)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    let result = if status.success() {
        std::fs::read_to_string(&path)?
    } else {
        content.to_string()
    };

    let _ = std::fs::remove_file(&path);
    Ok(result)
}

fn render_board_with_tabs(frame: &mut Frame, main_area: Rect, app: &App) {
    if !app.state.views.is_empty() {
        let tab_area = Rect {
            y: main_area.y,
            height: 1,
            ..main_area
        };
        let board_area = Rect {
            y: main_area.y + 1,
            height: main_area.height.saturating_sub(1),
            ..main_area
        };
        ui::tab_bar::render(frame, tab_area, app);
        render_layout(frame, board_area, app);
    } else {
        render_layout(frame, main_area, app);
    }
}

fn render_layout(frame: &mut Frame, area: Rect, app: &App) {
    use crate::model::state::LayoutMode;
    match app.state.current_layout {
        LayoutMode::Board => ui::board::render(frame, area, app),
        LayoutMode::Table => ui::table::render(frame, area, app),
        LayoutMode::Roadmap => {
            // Iteration field が無い project では Board にフォールバックする
            let has_iter = app
                .state
                .board
                .as_ref()
                .is_some_and(|b| b.has_iteration_field());
            if has_iter {
                ui::roadmap::render(frame, area, app);
            } else {
                ui::board::render(frame, area, app);
            }
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Reserve bottom line for status bar
    let main_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    match app.state.scene {
        Scene::Board => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
        }
        Scene::ProjectSelect => {
            if app.state.board.is_some() {
                render_board_with_tabs(frame, main_area, app);
                ui::statusline::render(frame, area, app);
            }
            ui::project_list::render(frame, area, app);
        }
        Scene::Help => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::help::render(frame, area, &app.state.keymap);
        }
        Scene::Filter => {
            render_board_with_tabs(frame, main_area, app);
            ui::filter_bar::render(frame, area, app);
        }
        Scene::Confirm(ref state) => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::confirm::render(frame, area, state);
        }
        Scene::CreateCard => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            let inherit_labels = app.state.derive_initial_label_names_from_filter();
            let inherit_assignees = app.state.derive_initial_assignee_logins_from_filter();
            ui::create_card::render(
                frame,
                area,
                &app.state.create_card_state,
                &inherit_labels,
                &inherit_assignees,
            );
        }
        Scene::Detail => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::detail::render(frame, area, app);
        }
        Scene::RepoSelect(ref rs) => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            let repos = app
                .state
                .board
                .as_ref()
                .map(|b| b.repositories.as_slice())
                .unwrap_or(&[]);
            ui::repo_select::render(frame, area, repos, rs);
        }
        Scene::EditCard(ref edit_state) => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::edit_card::render(frame, area, edit_state);
        }
        Scene::CardGrab(_) => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
        }
        Scene::CommentList(_) => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::detail::render(frame, area, app);
            ui::comment_list::render(frame, area, app);
        }
        Scene::GroupBySelect(_) => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::group_by_select::render(frame, area, app);
        }
        Scene::ReactionPicker(ref picker) => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::detail::render(frame, area, app);
            if matches!(picker.return_to, ViewMode::CommentList) {
                ui::comment_list::render(frame, area, app);
            }
            ui::reaction_picker::render(frame, area, picker, app);
        }
        Scene::BulkSelect => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
        }
        Scene::CommandPalette => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::command_palette::render(frame, area, app);
        }
    }

    // Loading/error overlay (Refreshing は statusline 側で控えめに表示)
    match &app.state.loading {
        LoadingState::Loading(msg) => render_loading(frame, area, msg),
        LoadingState::Error(msg) => render_error(frame, area, msg),
        LoadingState::Idle | LoadingState::Refreshing => {}
    }
}

fn render_loading(frame: &mut Frame, area: Rect, msg: &str) {
    use rattles::presets::prelude as presets;

    let popup = centered_rect(40, 3, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().yellow));

    let spinner = presets::dots_circle().current_frame();
    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled(format!("{spinner} "), Style::default().fg(theme().yellow)),
        Span::raw(msg),
    ]))
    .block(block)
    .centered();

    frame.render_widget(paragraph, popup);
}

fn render_error(frame: &mut Frame, area: Rect, msg: &str) {
    let popup = centered_rect(60, 20, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Error ")
        .title_style(
            Style::default()
                .fg(theme().red)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().red));

    let cmd_style = Style::default()
        .fg(theme().text)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(theme().red);

    let mut lines: Vec<Line> = vec![Line::from("")];
    for line in msg.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !line.starts_with(char::is_alphabetic) && line.starts_with(' ') {
            // Command lines (indented) get highlighted
            lines.push(Line::from(Span::styled(line, cmd_style)));
        } else {
            lines.push(Line::from(Span::styled(line, text_style)));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press any key to dismiss, q to quit",
        Style::default().fg(theme().text_muted),
    )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(paragraph, popup);
}

fn centered_rect(percent_x: u16, lines: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(lines)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
