use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::cli::event_loop::{EventLoop, EventLoopTx};
use crate::cli::tty::{Event, Tty};

pub struct AppSpec {
    pub tty: Tty,
    // TODO: Add more to AppSpec.
}

pub struct App {
    event_loop: EventLoop,
    event_loop_tx: EventLoopTx,

    pub tty: Arc<RwLock<Tty>>,
}

impl App {
    pub fn new(spec: AppSpec) -> App {
        let (event_loop, event_loop_tx) = EventLoop::new();

        let AppSpec { tty } = spec;

        App {
            event_loop,
            event_loop_tx,

            tty: Arc::new(RwLock::new(tty)),
        }
    }

    pub async fn read_line(&mut self) -> Result<Return> {
        let _restore = self.tty.write().await.setup()?;

        let read_loop_handle: tokio::task::JoinHandle<Result<()>> = {
            let tty = self.tty.clone();
            let mut event_loop_tx = self.event_loop_tx.clone();

            tokio::task::spawn(async move {
                loop {
                    let read: Result<Option<Event>> = tty.write().await.read().await;

                    match read {
                        // Send event to event loop.
                        Ok(Some(event)) => event_loop_tx.send_input(event).await?,
                        // Input stream closed.
                        Ok(None) => return Ok(()),
                        // Read error.
                        Err(err) => {
                            // TODO: Specific error handling?
                            return Err(err);
                        }
                    }
                }
            })
        };

        tokio::select! {
            ret = read_loop_handle => match ret? {
                Ok(()) => Ok(Return::Exit),
                Err(err) => Err(err),
            },
            ret = self.event_loop.run() => ret,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Return {
    /// A command.
    Input(String),
    /// Break from command (Ctrl-C).
    Break,
    /// Exit (Ctrl-D).
    Exit,
}
