use std::io::{self, Stdin, Stdout};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::EventStream;
use futures::StreamExt;

use crate::cli::term;
use crate::cli::term::buffer::Buffer;
use crate::cli::term::error::TermError;
use crate::cli::term::writer::Writer;

pub use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub use crate::cli::term::RestoreTerm;

pub struct Tty {
    stdin: Arc<Stdin>,
    stdout: Arc<Stdout>,
    writer: Writer,
    event_stream: EventStream,
}

impl Tty {
    pub fn std() -> Tty {
        let stdin = Arc::new(io::stdin());
        let stdout = Arc::new(io::stdout());

        let writer = Writer::new(stdout.clone());

        Tty {
            stdin,
            stdout,
            writer,
            event_stream: EventStream::new(),
        }
    }

    /// Sets the terminal up.
    pub fn setup(&mut self) -> Result<RestoreTerm> {
        term::setup(&self.stdin, self.stdout.clone())
    }

    /// Returns the width and height of the terminal.
    pub fn size(&self) -> Result<(u16, u16)> {
        Ok(crossterm::terminal::size()?)
    }

    /// Reads an event from the terminal asynchronously.
    pub async fn read(&mut self) -> Result<Option<Event>> {
        Ok(self.event_stream.next().await.transpose()?)
    }

    /// Reads an event from the terminal, blocks until event is received.
    pub fn read_blocking(&mut self) -> Result<Event> {
        Ok(crossterm::event::read()?)
    }

    /// Checks if an event is available.
    pub fn poll(&self, duration: Duration) -> Result<bool> {
        Ok(crossterm::event::poll(duration)?)
    }

    /// Flushes all unread input from the buffer.
    pub fn flush_input(&mut self) -> Result<()> {
        Ok(termios::tcflush(self.stdin.as_raw_fd(), termios::TCIFLUSH)
            .map_err(TermError::FlushInput)?)
    }

    /// Returns a reference to the current buffer.
    pub fn buffer(&self) -> &Buffer {
        self.writer.buffer()
    }

    /// Returns a mutable reference to the current buffer.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        self.writer.buffer_mut()
    }

    /// Resets the current buffer.
    pub fn reset_buffer(&mut self) {
        self.writer.reset_buffer();
    }

    /// Updates the current buffer and draws it to the terminal.
    pub fn update_buffer(
        &mut self,
        notes: Option<Buffer>,
        buffer: Buffer,
        refresh: bool,
    ) -> Result<()> {
        self.writer.commit_buffer(notes, buffer, refresh)
    }
}
