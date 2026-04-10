mod app;
mod app_state;
mod command;
mod event;
mod github;
mod model;
mod ui;

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    DefaultTerminal, Frame,
};

use app::App;
use event::EventHandler;
use github::client::GitHubClient;
use model::state::{LoadingState, ViewMode};

#[derive(Parser)]
#[command(name = "gh-board", about = "View GitHub Projects V2 as a kanban board")]
struct Cli {
    /// GitHub user or organization login
    #[arg(long)]
    owner: Option<String>,

    /// Project number to open directly
    #[arg(long, short)]
    project: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let github = GitHubClient::new().await?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, github, cli).await;
    ratatui::restore();

    result
}

async fn run(terminal: &mut DefaultTerminal, github: GitHubClient, cli: Cli) -> Result<()> {
    let (mut events, event_tx) = EventHandler::new(Duration::from_millis(250));
    let mut app = App::new(github, event_tx, cli.owner);

    let target_project = cli.project;

    // Start loading projects
    app.load_projects();

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        if let Some(event) = events.next().await {
            // If we have a target project number and projects just loaded, auto-select it
            if let event::AppEvent::ProjectsLoaded(Ok(_)) = &event {
                app.handle_event(event);
                if let Some(number) = target_project {
                    app.select_project_by_number(number);
                }
            } else {
                app.handle_event(event);
            }
        }

        if app.state.should_quit {
            break;
        }
    }

    Ok(())
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
            ui::board::render(frame, main_area, app);
            ui::statusline::render(frame, area, app);
        }
        ViewMode::ProjectSelect => {
            if app.state.board.is_some() {
                ui::board::render(frame, main_area, app);
                ui::statusline::render(frame, area, app);
            }
            ui::project_list::render(frame, area, app);
        }
        ViewMode::Help => {
            ui::board::render(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::help::render(frame, area);
        }
        ViewMode::Filter => {
            ui::board::render(frame, main_area, app);
            ui::filter_bar::render(frame, area, app);
        }
        ViewMode::Confirm => {
            ui::board::render(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            if let Some(state) = &app.state.confirm_state {
                ui::confirm::render(frame, area, state);
            }
        }
        ViewMode::CreateCard => {
            ui::board::render(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::create_card::render(frame, area, &app.state.create_card_state);
        }
        ViewMode::Detail => {
            ui::board::render(frame, main_area, app);
            ui::statusline::render(frame, area, app);
            ui::detail::render(frame, area, app);
        }
    }

    // Loading/error overlay
    match &app.state.loading {
        LoadingState::Loading(msg) => render_loading(frame, area, msg),
        LoadingState::Error(msg) => render_error(frame, area, msg),
        LoadingState::Idle => {}
    }
}

fn render_loading(frame: &mut Frame, area: Rect, msg: &str) {
    let popup = centered_rect(40, 5, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
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
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let cmd_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(Color::Red);

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
        Style::default().fg(Color::DarkGray),
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
