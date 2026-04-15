use std::time::Duration;

use crossterm::event::{EventStream, KeyEvent};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::model::project::{Board, Comment, Label, ProjectSummary};

pub enum AppEvent {
    Key(KeyEvent),
    #[allow(dead_code)]
    Resize(u16, u16),
    Tick,
    ProjectsLoaded(Result<Vec<ProjectSummary>, String>),
    ProjectLoaded(Result<ProjectSummary, String>),
    BoardLoaded(Result<Board, String>),
    CardMoved(Result<(), String>),
    CardDeleted(Result<(), String>),
    CardCreated(Result<(), String>),
    CardReordered(Result<(), String>),
    LabelsLoaded(Result<Vec<Label>, String>),
    AssigneesLoaded(Result<Vec<(String, String)>, String>),
    LabelToggled(Result<(), String>),
    AssigneeToggled(Result<(), String>),
    CardUpdated(Result<(), String>),
    CommentAdded(Result<Comment, String>),
    CommentUpdated(Result<Comment, String>),
    CommentsLoaded(Result<(String, Vec<Comment>), String>),
    CustomFieldUpdated(Result<(), String>),
    ReactionToggled(Result<(), String>),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
    task: JoinHandle<()>,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> (Self, mpsc::UnboundedSender<AppEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let task = Self::spawn_reader(tick_rate, tx.clone());

        (
            Self {
                rx,
                tx: tx.clone(),
                task,
                tick_rate,
            },
            tx,
        )
    }

    fn spawn_reader(
        tick_rate: Duration,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    _ = tick_interval.tick() => {
                        if event_tx.send(AppEvent::Tick).is_err() {
                            break;
                        }
                    }
                    event = reader.next() => {
                        match event {
                            Some(Ok(crossterm::event::Event::Key(key))) => {
                                if event_tx.send(AppEvent::Key(key)).is_err() {
                                    break;
                                }
                            }
                            Some(Ok(crossterm::event::Event::Resize(w, h))) => {
                                if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                                    break;
                                }
                            }
                            Some(Err(_)) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        })
    }

    pub fn pause(&mut self) {
        self.task.abort();
    }

    pub fn resume(&mut self) {
        self.task = Self::spawn_reader(self.tick_rate, self.tx.clone());
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
