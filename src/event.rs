use std::time::Duration;

use crossterm::event::{EventStream, KeyEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::model::project::{Board, ProjectSummary};

pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    ProjectsLoaded(Result<Vec<ProjectSummary>, String>),
    BoardLoaded(Result<Board, String>),
    CardMoved(Result<(), String>),
    CardDeleted(Result<(), String>),
    CardCreated(Result<(), String>),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> (Self, mpsc::UnboundedSender<AppEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

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
        });

        (Self { rx }, tx)
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
