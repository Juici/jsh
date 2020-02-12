use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::Mutex;

use crate::cli::event_loop::{EventLoop, EventLoopTx, Redraw};
use crate::cli::tty::{Event, KeyCode, KeyEvent, KeyModifiers, Tty};

// TODO: Add more to AppSpec.
pub struct AppSpec {
    pub tty: Tty,

    pub state: AppState,
}

pub struct App {
    event_loop: EventLoop,
    event_loop_tx: EventLoopTx,

    event_rx: Arc<Mutex<Receiver<Event>>>,
    redraw_rx: Arc<Mutex<Receiver<Redraw>>>,

    pub tty: Arc<Tty>,

    pub state: Arc<Mutex<AppState>>,
}

pub struct AppState {
    pub notes: Option<Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState { notes: None }
    }
}

impl App {
    pub fn new(spec: AppSpec) -> App {
        let AppSpec { tty, state } = spec;

        // TODO: Prompts.
        // TODO: Highlighting?

        let (mut event_loop, event_loop_tx) = EventLoop::new();

        const EVENT_CHANNEL_SIZE: usize = 128;
        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_SIZE);

        const REDRAW_CHANNEL_SIZE: usize = 8;
        let (redraw_tx, redraw_rx) = mpsc::channel(REDRAW_CHANNEL_SIZE);

        event_loop.set_event_tx(event_tx);
        event_loop.set_redraw_tx(redraw_tx);

        App {
            event_loop,
            event_loop_tx,

            event_rx: Arc::new(Mutex::new(event_rx)),
            redraw_rx: Arc::new(Mutex::new(redraw_rx)),

            tty: Arc::new(tty),

            state: Arc::new(Mutex::new(state)),
        }
    }

    pub async fn read_line(&mut self) -> Result<Return> {
        // TODO: Before read line.

        let _restore = self.tty.setup()?;

        let read_loop_handle: tokio::task::JoinHandle<Result<()>> = {
            let tty = self.tty.clone();
            let mut event_loop_tx = self.event_loop_tx.clone();

            tokio::spawn(async move {
                loop {
                    let read: Result<Option<Event>> = tty.read().await;

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

        let event_handle: tokio::task::JoinHandle<Result<()>> = {
            let event_rx = self.event_rx.clone();
            let mut event_loop_tx = self.event_loop_tx.clone();

            tokio::spawn(async move {
                while let Some(event) = event_rx.lock().await.recv().await {
                    match event {
                        // Ctrl-D, exit.
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('d'),
                            modifiers: KeyModifiers::CONTROL,
                        }) => {
                            event_loop_tx.send_return(Ok(Return::Exit)).await?;
                        }
                        // Ctrl-C, reset line buffer.
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: KeyModifiers::CONTROL,
                        }) => {
                            println!("reset line buffer");
                        }
                        Event::Resize(cols, rows) => {
                            event_loop_tx.send_redraw(true, Some((cols, rows))).await?;
                        }
                        event => {
                            // TODO: Handle event.
                            println!("handle event: {:?}", event);
                        }
                    }
                }

                Ok(())
            })
        };

        let redraw_handle: tokio::task::JoinHandle<Result<()>> = {
            let tty = self.tty.clone();
            let state = self.state.clone();

            let redraw_rx = self.redraw_rx.clone();

            tokio::spawn(async move {
                while let Some(redraw) = redraw_rx.lock().await.recv().await {
                    // TODO: Handle redraw.
                    println!("handle redraw: {:?}", redraw);

                    let Redraw { size, flags } = redraw;

                    let (_width, _height) = match size {
                        Some((w, h)) => (w, h),
                        None => tty.size()?,
                    };

                    let _notes: Vec<String> = {
                        let mut state = state.lock().await;

                        const EMPTY_NOTES: Vec<String> = Vec::new();
                        state.notes.take().unwrap_or(EMPTY_NOTES)
                    };

                    // TODO: Render notes.

                    if flags.is_final() {
                        // TODO: Redraw code area.
                        // TODO: Render app.

                        // TODO: tty.update_and_reset_buffer(...).await?;
                    } else {
                        // TODO: Redraw code area.
                        // TODO: Render app.

                        // TODO: tty.update_buffer(...).await?;
                    }
                }

                Ok(())
            })
        };

        tokio::select! {
            // Event loop.
            ret = self.event_loop.run() => ret,
            // Read loop.
            ret = read_loop_handle => match ret? {
                Ok(()) => Ok(Return::Exit), // The stream has closed.
                Err(err) => Err(err),
            },
            // Event handler.
            ret = event_handle => match ret? {
                Ok(()) => unreachable!(), // Should never return.
                Err(err) => Err(err),
            },
            // Redraw handler.
            ret = redraw_handle => match ret? {
                Ok(()) => unreachable!(), // Should never return.
                Err(err) => Err(err),
            },
        }
    }

    pub async fn redraw(&self) -> Result<()> {
        self.event_loop_tx.send_redraw_clone(false, None).await
    }

    pub async fn redraw_full(&self) -> Result<()> {
        self.event_loop_tx.send_redraw_clone(true, None).await
    }

    pub async fn commit_eof(&self) -> Result<()> {
        self.event_loop_tx
            .send_return_clone(Ok(Return::Exit))
            .await?;
        Ok(())
    }

    pub async fn commit_line(&self) -> Result<()> {
        // TODO: Copy code buffer and send to return_tx.
        Ok(())
    }

    pub async fn notify<S: Into<String>>(&self, note: S) -> Result<()> {
        // Mutate state.
        {
            let mut state = self.state.lock().await;

            let notes: &mut Option<Vec<String>> = &mut state.notes;

            let note = note.into();
            match notes {
                Some(notes) => notes.push(note),
                notes @ None => *notes = Some(vec![note]),
            }
        }

        self.redraw().await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Return {
    /// A command.
    Input(String),
    /// Exit (Ctrl-D).
    Exit,
}
