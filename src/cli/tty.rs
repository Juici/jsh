use std::io::{self, Stdin, Stdout};
use std::ops::{Deref, DerefMut};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;

use anyhow::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::cli::term;
use crate::cli::term::buffer::Buffer;
use crate::cli::term::error::TermError;
use crate::cli::term::writer::Writer;

pub use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub use crate::cli::term::RestoreTerm;

pub struct Tty {
    stdin: Arc<Stdin>,
    stdout: Arc<Stdout>,

    writer: RwLock<Writer>,
    event_stream: Mutex<EventStream>,
}

impl Tty {
    pub fn std() -> Tty {
        let stdin = Arc::new(io::stdin());
        let stdout = Arc::new(io::stdout());

        let writer = Writer::new(stdout.clone());

        Tty {
            stdin,
            stdout,

            writer: RwLock::new(writer),
            event_stream: Mutex::new(EventStream::new()),
        }
    }

    /// Sets the terminal up.
    pub fn setup(&self) -> Result<RestoreTerm> {
        term::setup(&self.stdin, self.stdout.clone())
    }

    /// Returns the width and height of the terminal.
    pub fn size(&self) -> Result<(u16, u16)> {
        Ok(crossterm::terminal::size()?)
    }

    /// Reads an event from the terminal asynchronously.
    pub async fn read(&self) -> Result<Option<Event>> {
        Ok(self.event_stream.lock().await.next().await.transpose()?)
    }

    /// Flushes all unread input from the buffer.
    pub fn flush_input(&self) -> Result<()> {
        Ok(termios::tcflush(self.stdin.as_raw_fd(), termios::TCIFLUSH)
            .map_err(TermError::FlushInput)?)
    }

    /// Returns a reference to the current buffer.
    pub async fn buffer<'a>(&'a self) -> BufferGuard<'a> {
        let writer = self.writer.read().await;
        BufferGuard { writer }
    }

    /// Returns a mutable reference to the current buffer.
    pub async fn buffer_mut<'a>(&'a self) -> BufferMutGuard<'a> {
        let writer = self.writer.write().await;
        BufferMutGuard { writer }
    }

    /// Resets the current buffer.
    pub async fn reset_buffer(&self) {
        self.writer.write().await.reset_buffer();
    }

    /// Updates the current buffer and draws it to the terminal.
    pub async fn update_buffer(
        &self,
        notes: Option<Buffer>,
        buffer: Buffer,
        refresh: bool,
    ) -> Result<()> {
        self.writer
            .write()
            .await
            .commit_buffer(notes, buffer, refresh)
    }

    /// Updates the current buffer and draws it to the terminal, then resets it.
    pub async fn update_and_reset_buffer(
        &self,
        notes: Option<Buffer>,
        buffer: Buffer,
        refresh: bool,
    ) -> Result<()> {
        let mut writer = self.writer.write().await;

        writer.commit_buffer(notes, buffer, refresh)?;
        writer.reset_buffer();

        Ok(())
    }
}

pub struct BufferGuard<'a> {
    writer: RwLockReadGuard<'a, Writer>,
}

impl<'a> Deref for BufferGuard<'a> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        self.writer.buffer()
    }
}

pub struct BufferMutGuard<'a> {
    writer: RwLockWriteGuard<'a, Writer>,
}

impl<'a> Deref for BufferMutGuard<'a> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        self.writer.buffer()
    }
}

impl<'a> DerefMut for BufferMutGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.writer.buffer_mut()
    }
}
