use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::{Mutex, RwLock};

use crate::cli::code_area::{CodeArea, CodeAreaSpec, CodeAreaState};
use crate::cli::term::buffer::Buffer;
use crate::cli::tty::{Event, KeyCode, KeyEvent, KeyModifiers, Tty};
use crate::cli::widget::{Handle, Render};

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

    code_area: CodeArea,

    pub tty: Tty,

    pub state: Arc<Mutex<AppState>>,
}

pub struct AppState {
    pub notes: Option<Vec<String>>,
}

impl AppState {
    pub fn reset_state(&mut self) {
        *self = AppState::default();
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState { notes: None }
    }
}

struct AfterLine {
    app_state: Arc<Mutex<AppState>>,
    code_area_state: Arc<RwLock<CodeAreaState>>,
}

impl Drop for AfterLine {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            self.app_state.lock().await.reset_state();
            self.code_area_state.write().await.reset_state();
        });
    }
}

impl App {
    pub fn new(spec: AppSpec) -> App {
        let AppSpec { tty, state } = spec;

        // TODO: Prompts.
        // TODO: Highlighting?
        // TODO: CodeArea.

        const REDRAW_CHANNEL_SIZE: usize = 8;
        let (redraw_tx, redraw_rx) = mpsc::channel(REDRAW_CHANNEL_SIZE);

        let (return_tx, return_rx) = mpsc::channel(1);

        let code_area = CodeArea::new(CodeAreaSpec {
            state: CodeAreaState::default(),
            return_tx: return_tx.clone(),
        });

        App {
            redraw_tx,
            redraw_rx,

            return_tx,
            return_rx,

            code_area,

            tty,

            state: Arc::new(Mutex::new(state)),
        }
    }

    #[inline]
    pub async fn mutate_state<F>(&self, f: F)
    where
        F: FnOnce(&mut AppState) -> (),
    {
        let mut state = self.state.lock().await;
        f(&mut state);
    }

    async fn reset_all_states(&mut self) {
        self.mutate_state(AppState::reset_state).await;

        self.code_area
            .mutate_state(CodeAreaState::reset_state)
            .await;
    }

    async fn handle_event(&mut self, event: Event) -> Result<()> {
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
                self.reset_all_states().await;

                // TODO: Trigger prompts.
            }
            // Event::Key(KeyEvent {
            //     code: KeyCode::Char('?'),
            //     ..
            // }) => {
            //     println!("buffer: {:?}", self.tty.buffer());
            // }
            Event::Resize(cols, rows) => {
                self.redraw_tx
                    .send(Redraw {
                        size: Some((cols, rows)),
                        flags: RedrawFlags::FULL,
                    })
                    .await?;
            }
            event => {
                self.code_area.handle(event).await;

                // TODO: Update prompts.
            }
        }

        Ok(())
    }

    async fn handle_redraw(&mut self, redraw: Redraw) -> Result<()> {
        let Redraw { size, flags } = redraw;

        let (width, height) = match size {
            Some((w, h)) => (w, h),
            None => self.tty.size()?,
        };

        let buf_notes: Option<Buffer> = {
            let mut state = self.state.lock().await;

            match state.notes.take() {
                Some(notes) => Self::render_notes(notes, width).await,
                None => None,
            }
        };

        if flags.is_final() {
            let mut buf = Self::render_app(&mut self.code_area, width, height).await;
            buf.new_line(true, Some(width));

            self.tty.update_buffer(buf_notes, buf, flags.is_full())?;
            self.tty.reset_buffer();
        } else {
            let buf = Self::render_app(&mut self.code_area, width, height).await;

            self.tty.update_buffer(buf_notes, buf, flags.is_full())?;
        }

        Ok(())
    }

    async fn render_notes(notes: Vec<String>, width: u16) -> Option<Buffer> {
        if notes.is_empty() {
            return None;
        }

        let mut buf = Buffer::builder(width);
        for (i, note) in notes.into_iter().enumerate() {
            if i > 0 {
                buf.newline();
            }
            buf.write_str(&note);
        }

        Some(buf.buffer())
    }

    async fn render_app(code_area: &mut CodeArea, width: u16, height: u16) -> Buffer {
        let buf = code_area.render(width, height).await;

        // TODO: Addon?

        buf
    }

    pub async fn read_line(&mut self) -> Result<Return> {
        // TODO: Before read line.

        // TODO: Drop for after read line and reset states.

        // Setup line and hold restore drop handle.
        let _restore = self.tty.setup()?;

        // After line drop handle.
        let _after_line = AfterLine {
            app_state: self.state.clone(),
            code_area_state: self.code_area.state.clone(),
        };

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
                            // TODO: Handle recoverable and unrecoverable errors.
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

    pub async fn commit_eof(&mut self) -> Result<()> {
        self.return_tx.send(Ok(Return::Exit)).await?;
        Ok(())
    }

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
