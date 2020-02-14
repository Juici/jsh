use std::io::{BufWriter, Stdout, Write};
use std::sync::Arc;

use anyhow::Result;
use crossterm::{cursor, terminal};

use super::buffer::{Buffer, Line, Pos};

const CLEAR_UNTIL_NEWLINE: terminal::Clear = terminal::Clear(terminal::ClearType::UntilNewLine);
const CLEAR_FROM_CURSOR_DOWN: terminal::Clear =
    terminal::Clear(terminal::ClearType::FromCursorDown);

pub struct Writer {
    stdout: Arc<Stdout>,
    buffer: Buffer,
}

impl Writer {
    pub fn new(stdout: Arc<Stdout>) -> Writer {
        Writer {
            stdout,
            buffer: Buffer::EMPTY,
        }
    }

    /// Returns a reference the current buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Returns a mutable reference to the current buffer.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Resets the current buffer.
    pub fn reset_buffer(&mut self) {
        self.buffer = Buffer::EMPTY;
    }

    /// Updates the terminal to reflect the current buffer.
    pub fn commit_buffer(
        &mut self,
        notes: Option<Buffer>,
        buffer: Buffer,
        mut refresh: bool,
    ) -> Result<()> {
        let old_buffer = &mut self.buffer;

        // Check if the screen width has changed, if so force full refresh.
        if buffer.width != old_buffer.width && !old_buffer.lines.is_empty() {
            old_buffer.lines.clear();
            refresh = true;
        }

        let mut out = BufWriter::new(self.stdout.lock());

        // Hide cursor.
        crossterm::queue!(out, cursor::Hide)?;

        // Move cursor to start of buffer.
        match old_buffer.dot.line {
            0 => {}
            line => crossterm::queue!(out, cursor::MoveUp(line as u16))?,
        }
        out.write_all(b"\r")?;

        if refresh {
            // Clear screen.
            //
            // Note: tmux may save the screen, so we write a space before clearing then
            // return to first column.
            out.write_all(b" ")?;
            crossterm::queue!(out, CLEAR_FROM_CURSOR_DOWN)?;
            out.write_all(b"\r")?;
        }

        let mut style = None;

        macro_rules! switch_style {
            ($new_style:expr) => {
                match ($new_style) {
                    #[allow(unused_assignments)]
                    new_style if style != new_style => {
                        match new_style {
                            Some(new_style) => write!(out, "\x1b[0;{}m", new_style)?,
                            None => out.write_all(b"\x1b[0;m")?,
                        }
                        style = new_style;
                    }
                    _ => {}
                }
            };
        }

        macro_rules! write_cells {
            ($cells:expr) => {
                match (&($cells)) {
                    cells => {
                        for cell in cells.into_iter() {
                            switch_style!(cell.style);
                            write!(out, "{}", cell.text)?;
                        }
                    }
                }
            };
        }

        if let Some(notes) = notes {
            for line in &notes.lines {
                write_cells!(line);
                switch_style!(None);
                crossterm::queue!(out, CLEAR_UNTIL_NEWLINE)?;
                out.write_all(b"\n")?;
            }

            // // XXX Hacky.
            // if len(w.curBuf.Lines) > 0 {
            //     w.curBuf.Lines = w.curBuf.Lines[1:]
            // }
        }

        'write_lines: for (i, line) in buffer.lines.iter().enumerate() {
            if i > 0 {
                out.write_all(b"\n")?;
            }

            // First cell where `buffer` and `old_buffer` differ for the line.
            let mut j = 0;

            // If not a full refresh, attempt to avoid rewriting unchanged sections of line.
            if !refresh {
                if let Some(old_line) = old_buffer.lines.get(i) {
                    // Find the offset of the first difference, if found the offset is guaranteed to
                    // be at most `line.len()`.
                    match line.find_difference(old_line) {
                        Some(diff) => j = diff,
                        // No need to update current line.
                        None => continue 'write_lines,
                    }

                    // Move to first differing column if necessary.
                    let first_col = Line::width_slice(&line[..j]);
                    if first_col > 0 {
                        crossterm::queue!(out, cursor::MoveRight(first_col))?;
                    }

                    // Clear the rest of the line if necessary.
                    if j < old_line.len() {
                        switch_style!(None);
                        crossterm::queue!(out, CLEAR_UNTIL_NEWLINE)?;
                    }
                }
            }

            // Write any remaining cells in the cell.
            if j < line.len() {
                write_cells!(line[j..]);
            }
        }

        if !refresh && old_buffer.lines.len() > buffer.lines.len() {
            // If the old buffer is higher, clear old content.
            switch_style!(None);

            // write!(out, "\n{}{}", CLEAR_FROM_CURSOR_DOWN,
            // cursor::MoveUp(1))?;

            // out.write_all(b"\n")?;
            // crossterm::queue!(out, CLEAR_FROM_CURSOR_DOWN,
            // cursor::MoveUp(1))?;

            // crossterm::queue!(
            //     out,
            //     cursor::SavePosition,
            //     cursor::MoveDown(1),
            //     cursor::MoveToColumn(0),
            //     CLEAR_FROM_CURSOR_DOWN,
            //     cursor::RestorePosition,
            // )?;

            crossterm::queue!(out, cursor::SavePosition)?;
            out.write_all(b"\n")?;
            crossterm::queue!(out, CLEAR_FROM_CURSOR_DOWN, cursor::RestorePosition)?;
        }
        switch_style!(None);

        // Move the cursor to the buffer `dot`.
        let cursor = buffer.cursor();
        write_delta_pos(&mut out, cursor, buffer.dot)?;

        // Show cursor.
        crossterm::queue!(out, cursor::Show)?;

        // Flush buffer.
        out.flush()?;

        // Update old buffer.
        *old_buffer = buffer;

        Ok(())
    }
}

fn write_delta_pos<W: Write>(w: &mut W, from: Pos, to: Pos) -> Result<()> {
    match to.line.checked_sub(from.line) {
        Some(0) | None => match from.line.checked_sub(to.line) {
            Some(0) | None => {}
            Some(up) => crossterm::queue!(w, cursor::MoveUp(up))?,
        },
        Some(down) => crossterm::queue!(w, cursor::MoveDown(down))?,
    }

    w.write_all(b"\r")?;
    if to.col > 0 {
        crossterm::queue!(w, cursor::MoveRight(to.col))?;
    }

    Ok(())
}
