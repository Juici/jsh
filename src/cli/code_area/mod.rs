mod view;

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

use self::view::View;

use crate::cli::app::Return;
use crate::cli::prompt::PromptHandle;
use crate::cli::term::buffer::Buffer;
use crate::cli::tty::{Event, KeyCode, KeyEvent};
use crate::cli::widget::{Handle, Render, Widget};

// TODO: Overlay handler.
// TODO: Highlighter.
pub struct CodeAreaSpec {
    pub state: CodeAreaState,

    pub prompt: PromptHandle,
    pub rprompt: PromptHandle,

    pub return_tx: Sender<Result<Return>>,
}

pub struct CodeArea {
    pub state: Arc<RwLock<CodeAreaState>>,

    pub prompt: PromptHandle,
    pub rprompt: PromptHandle,

    inserts: String,
    last_buffer: Option<CodeBuffer>,
    return_tx: Sender<Result<Return>>,
    // TODO: Pasting and paste buffer?
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodeAreaState {
    pub buffer: CodeBuffer,
    pub pending: PendingCode,
    pub hide_rprompt: bool,
}

/// Buffer for the CodeArea.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodeBuffer {
    /// Content of the buffer.
    pub content: String,
    /// Position of the dot (cursor), as a byte index.
    pub dot: usize,
}

/// Pending code, such as in a completion.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PendingCode {
    pub from: usize,
    pub to: usize,
    pub content: String,
}

impl CodeAreaState {
    pub fn reset_state(&mut self) {
        *self = CodeAreaState::default();
    }

    // TODO: Apply pending function.
}

impl CodeBuffer {
    pub fn insert_at_dot(&mut self, s: &str) {
        self.content.insert_str(self.dot, s);
        self.dot += s.len();
    }

    pub fn insert_char_at_dot(&mut self, c: char) {
        self.content.insert(self.dot, c);
        self.dot += c.len_utf8();
    }
}

impl Widget for CodeArea {}

#[async_trait]
impl Render for CodeArea {
    async fn render(&mut self, width: u16, height: u16) -> Buffer {
        let view = View::get(self).await;
        let mut buf = Buffer::builder(width);
        view.render_view(&mut buf);

        let mut buf = buf.buffer();

        // Truncate buffer to within height.
        match height {
            // We can show all lines, do nothing.
            _ if buf.lines.len() <= height as usize => {}
            // We can show all lines before the cursor, show as many lines after cursor as we can.
            _ if buf.dot.line < height => buf.trim_to_lines(0..height as usize),
            _ => {
                let from = usize::from(buf.dot.line - height + 1);
                let to = usize::from(buf.dot.line + 1);
                buf.trim_to_lines(from..to);
            }
        }

        buf
    }
}

#[async_trait]
impl Handle for CodeArea {
    async fn handle(&mut self, event: Event) -> bool {
        match event {
            Event::Key(event) => self.handle_key_event(event).await,
            _ => false,
        }
    }
}

impl CodeArea {
    pub fn new(spec: CodeAreaSpec) -> CodeArea {
        let CodeAreaSpec {
            state,
            return_tx,
            prompt,
            rprompt,
        } = spec;

        CodeArea {
            state: Arc::new(RwLock::new(state)),

            prompt,
            rprompt,

            inserts: String::new(),
            last_buffer: None,
            return_tx,
        }
    }

    pub async fn submit(&mut self) {
        self.return_tx
            .send(Ok(Return::Input(
                self.state.read().await.buffer.content.clone(),
            )))
            .await
            .unwrap(); // TODO: Remove unwrap?
    }

    #[inline]
    pub async fn mutate_state<F>(&mut self, f: F)
    where
        F: FnOnce(&mut CodeAreaState) -> (),
    {
        let mut state = self.state.write().await;
        f(&mut state);
    }

    #[inline]
    pub async fn clone_state(&self) -> CodeAreaState {
        self.state.read().await.clone()
    }
}

macro_rules! reset_inserts {
    ($code_area:expr) => {
        $code_area.inserts.truncate(0); // TODO: Maybe replace with `String::new()`.
        $code_area.last_buffer = None;
    };
}

impl CodeArea {
    fn reset_inserts(&mut self) {
        reset_inserts!(self);
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        // TODO: Overlay handler: handle key.

        match key.code {
            KeyCode::Enter => {
                self.reset_inserts();
                self.submit().await;
                true
            }
            KeyCode::Backspace => {
                self.reset_inserts();
                self.mutate_state(|state| {
                    let mut buf = &mut state.buffer;

                    // Check the cursor is not at the start of the buffer and the
                    // buffer is not empty.
                    if buf.dot > 0 && !buf.content.is_empty() {
                        if buf.dot == buf.content.len() {
                            buf.content.pop();
                            buf.dot = buf.content.len();
                        } else {
                            let c = buf.content.remove(buf.dot);
                            buf.dot -= c.len_utf8();
                        }
                    }
                })
                .await;

                true
            }
            KeyCode::Char(c) if key.modifiers.is_empty() => {
                let mut state = self.state.write().await;

                // Check if something has happened to the buffer, if so reset the state.
                match (&self.last_buffer, &state.buffer) {
                    (Some(last_buf), buf) if last_buf == buf => {}
                    _ => {
                        // Inline `self.reset_inserts()` due to borrow of `state`.
                        // Removes the need for an extra acquire of `state`.
                        reset_inserts!(self);
                    }
                }

                state.buffer.insert_char_at_dot(c);

                self.inserts.push(c);
                self.last_buffer = Some(state.buffer.clone());

                true
            }
            // Functional key with no binding.
            _ => {
                self.reset_inserts();
                false
            }
        }
    }
}
