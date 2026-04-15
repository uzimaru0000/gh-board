mod action;
mod app;
mod app_state;
mod command;
mod config;
mod event;
mod github;
mod keymap;
mod model;
mod ui;

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
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
use model::state::{LoadingState, ViewMode};

#[derive(Parser)]
#[command(name = "gh-board", about = "View GitHub Projects V2 as a kanban board")]
struct Cli {
    /// Project number to open directly
    number: Option<i32>,

    /// Login of the owner. Use "@me" for the current user.
    #[arg(long)]
    owner: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::load_config().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}");
        config::Config::default()
    });
    ui::theme::init_theme(&cfg.theme);

    let cli = Cli::parse();
    let github = GitHubClient::new().await?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, github, cli, cfg).await;
    ratatui::restore();

    result
}

async fn run(terminal: &mut DefaultTerminal, github: GitHubClient, cli: Cli, cfg: config::Config) -> Result<()> {
    let (mut events, event_tx) = EventHandler::new(Duration::from_millis(80));

    // "@me" means the current viewer (same as no owner)
    let owner = cli.owner.filter(|o| o != "@me");

    let mut app = App::new(github, event_tx, owner.clone());
    app.state.set_views(cfg.view);

    let keymap = keymap::Keymap::default_keymap().with_overrides(&cfg.keys);
    app.state.set_keymap(keymap);

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
            crossterm::execute!(std::io::stdout(), LeaveAlternateScreen)?;

            let result = run_editor(&content);

            enable_raw_mode()?;
            crossterm::execute!(std::io::stdout(), EnterAlternateScreen)?;
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
                            if let Some(ref mut s) = app.state.edit_card_state {
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

        if app.state.should_quit {
            break;
        }
    }

    Ok(())
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
        ui::board::render(frame, board_area, app);
    } else {
        ui::board::render(frame, main_area, app);
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

    match app.state.mode {
        ViewMode::Board => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
        }
        ViewMode::ProjectSelect => {
            if app.state.board.is_some() {
                render_board_with_tabs(frame, main_area, app);
                ui::statusline::render(frame, area, app);
            }
            ui::project_list::render(frame, area, app);
        }
        ViewMode::Help => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::help::render(frame, area, &app.state.keymap);
        }
        ViewMode::Filter => {
            render_board_with_tabs(frame, main_area, app);
            ui::filter_bar::render(frame, area, app);
        }
        ViewMode::Confirm => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            if let Some(state) = &app.state.confirm_state {
                ui::confirm::render(frame, area, state);
            }
        }
        ViewMode::CreateCard => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::create_card::render(frame, area, &app.state.create_card_state);
        }
        ViewMode::Detail => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::detail::render(frame, area, app);
        }
        ViewMode::RepoSelect => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            if let Some(rs) = &app.state.repo_select_state {
                let repos = app
                    .state
                    .board
                    .as_ref()
                    .map(|b| b.repositories.as_slice())
                    .unwrap_or(&[]);
                ui::repo_select::render(frame, area, repos, rs);
            }
        }
        ViewMode::EditCard => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            if let Some(ref edit_state) = app.state.edit_card_state {
                ui::edit_card::render(frame, area, edit_state);
            }
        }
        ViewMode::CardGrab => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
        }
        ViewMode::CommentList => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::detail::render(frame, area, app);
            ui::comment_list::render(frame, area, app);
        }
        ViewMode::ReactionPicker => {
            render_board_with_tabs(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::detail::render(frame, area, app);
            if let Some(ref picker) = app.state.reaction_picker_state {
                if matches!(picker.return_to, ViewMode::CommentList) {
                    ui::comment_list::render(frame, area, app);
                }
                ui::reaction_picker::render(frame, area, picker, app);
            }
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
