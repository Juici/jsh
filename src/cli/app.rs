use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

use crate::cli::tty::{Event, KeyCode, KeyEvent, KeyModifiers, Tty};

// TODO: Add more to AppSpec.
pub struct AppSpec {
    pub tty: Tty,

    pub state: AppState,
}

pub struct App {
    redraw_tx: Sender<Redraw>,
    redraw_rx: Receiver<Redraw>,

    return_tx: Sender<Result<Return>>,
    return_rx: Receiver<Result<Return>>,

    pub tty: Tty,

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

        const REDRAW_CHANNEL_SIZE: usize = 8;
        let (redraw_tx, redraw_rx) = mpsc::channel(REDRAW_CHANNEL_SIZE);

        let (return_tx, return_rx) = mpsc::channel(1);

        App {
            redraw_tx,
            redraw_rx,

            return_tx,
            return_rx,

            tty,

            state: Arc::new(Mutex::new(state)),
        }
    }

    async fn handle_event(&mut self, event: Event) -> Result<()> {
        // TODO

        match event {
            // EOF on Ctrl-D.
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
            }) => self.commit_eof().await?,
            // Discard line on Ctrl-C.
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                // TODO: Reset line buffer state.
                // TODO: Trigger prompts.

                println!("handle ctrl-c");
            }
            Event::Resize(cols, rows) => {
                self.redraw_tx
                    .send(Redraw {
                        size: Some((cols, rows)),
                        flags: RedrawFlags::FULL,
                    })
                    .await?;
            }
            event => {
                // TODO: Send event to code area.
                // TODO: Update prompts.

                println!("handle event: {:?}", event);
            }
        }

        Ok(())
    }

    async fn handle_redraw(&mut self, redraw: Redraw) -> Result<()> {
        // TODO
        println!("handle redraw: {:?}", redraw);

        let Redraw { size, flags } = redraw;

        let (_width, _height) = match size {
            Some((w, h)) => (w, h),
            None => self.tty.size()?,
        };

        let _notes: Vec<String> = {
            let mut state = self.state.lock().await;

            const EMPTY_NOTES: Vec<String> = Vec::new();
            state.notes.take().unwrap_or(EMPTY_NOTES)
        };

        // TODO: Render notes.

        if flags.is_final() {
            // TODO: Redraw code area.
            // TODO: Render app.

            // TODO: tty.update_buffer(...).await?;
            self.tty.reset_buffer();
        } else {
            // TODO: Redraw code area.
            // TODO: Render app.

            // TODO: tty.update_buffer(...).await?;
        }

        Ok(())
    }

    pub async fn read_line(&mut self) -> Result<Return> {
        // TODO: Before read line.
        // TODO: Drop for after read line and reset states.

        let _restore = self.tty.setup()?;

        // Redraw state.
        let mut redraw = Redraw {
            size: None,
            flags: RedrawFlags::empty(),
        };

        // TODO: Trigger prompts.

        loop {
            // Redraw.
            self.handle_redraw(redraw).await?;
            redraw.flags = RedrawFlags::empty();

            tokio::select! {
                // Received event from terminal.
                event = self.tty.read() => {
                    match event {
                        // Handle event.
                        Ok(Some(event)) => {
                            self.handle_event(event).await?;

                            // Keep consuming available events to minimize redraws.
                            'consume_events: loop {
                                // Received return message.
                                if let Ok(ret) = self.return_rx.try_recv() {
                                    // Final redraw.
                                    redraw.flags.insert(RedrawFlags::FINAL);
                                    self.handle_redraw(redraw).await?;

                                    return ret;
                                }

                                // Handle available events.
                                match self.tty.try_read().await? {
                                    Some(event) => self.handle_event(event).await?,
                                    None => break 'consume_events,
                                }
                            }
                        }
                        // Input stream closed.
                        Ok(None) => return Ok(Return::Exit),
                        // Read error.
                        Err(err) => {
                            // TODO: Handle recoverable and unrecoverable events.
                            return Err(err);
                        }
                    }
                }
                // Received redraw message.
                Some(Redraw { size, flags }) = self.redraw_rx.recv() => {
                    // Update size if sent.
                    if let Some(size) = size {
                        redraw.size = Some(size);
                    }
                    redraw.flags.insert(flags);
                }
                // Received return message.
                Some(ret) = self.return_rx.recv() => return ret,
                // TODO: Prompt updates.
                // TODO: Highlighter updates.
            }
        }
    }

    //    pub async fn redraw(&mut self) -> Result<()> {
    //        self.redraw_tx
    //            .send(Redraw {
    //                size: None,
    //                flags: RedrawFlags::empty(),
    //            })
    //            .await?;
    //        Ok(())
    //    }
    //
    //    pub async fn redraw_full(&mut self) -> Result<()> {
    //        self.redraw_tx
    //            .send(Redraw {
    //                size: None,
    //                flags: RedrawFlags::FULL,
    //            })
    //            .await?;
    //        Ok(())
    //    }

    pub async fn commit_eof(&mut self) -> Result<()> {
        self.return_tx.send(Ok(Return::Exit)).await?;
        Ok(())
    }

    //    pub async fn commit_line(&mut self) -> Result<()> {
    //        // TODO: Copy code buffer and send to return_tx.
    //        Ok(())
    //    }
    //
    //    pub async fn notify<S: Into<String>>(&mut self, note: S) -> Result<()> {
    //        // Mutate state.
    //        {
    //            let mut state = self.state.lock().await;
    //
    //            let notes: &mut Option<Vec<String>> = &mut state.notes;
    //
    //            let note = note.into();
    //            match notes {
    //                Some(notes) => notes.push(note),
    //                notes @ None => *notes = Some(vec![note]),
    //            }
    //        }
    //
    //        self.redraw().await?;
    //
    //        Ok(())
    //    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Return {
    /// A command.
    Input(String),
    /// Exit (Ctrl-D).
    Exit,
}

bitflags::bitflags! {
    pub struct RedrawFlags: u8 {
        const FULL = 0b01;
        const FINAL = 0b10;
    }
}

impl RedrawFlags {
    pub fn is_final(self) -> bool {
        self.contains(RedrawFlags::FINAL)
    }

    pub fn is_full(self) -> bool {
        self.contains(RedrawFlags::FULL)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Redraw {
    pub size: Option<(u16, u16)>,
    pub flags: RedrawFlags,
}
