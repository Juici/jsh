use std::io::{BufWriter, Stdout, Write};
use std::sync::Arc;

use anyhow::Result;
use crossterm::{cursor, terminal};

use super::buffer::{Buffer, Line, Pos};
use super::utils::wcswidth;

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

        if buffer.width != old_buffer.width && !old_buffer.lines.is_empty() {
            old_buffer.lines.clear();
            refresh = true;
        }

        let mut out = BufWriter::new(self.stdout.lock());

        // Hide cursor.
        write!(out, "{}", cursor::Hide)?;

        // Move cursor to start of buffer.
        match &old_buffer.dot.line {
            0 => {}
            &line => write!(out, "{}", cursor::MoveUp(line as u16))?,
        }
        out.write_all(b"\r")?;

        if refresh {
            // Clear screen.
            //
            // Note: tmux may save the screen, so we write a space before clearing then
            // return to first column.
            write!(out, " {}\r", CLEAR_FROM_CURSOR_DOWN)?;
        }

        let mut style = None;

        macro_rules! switch_style {
            ($new_style:expr) => {
                match ($new_style) {
                    #[allow(unused_assignments)]
                    new_style if style != new_style => {
                        match new_style {
                            Some(new_style) => write!(out, "\x1b[0;{}m", new_style)?,
                            None => out.write_all(b"\x1b[m")?,
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
                            out.write_all(cell.text.as_bytes())?;
                        }
                    }
                }
            };
        }

        if let Some(notes) = notes {
            for line in &notes.lines {
                write_cells!(line);
                switch_style!(None);
                writeln!(out, "{}", CLEAR_UNTIL_NEWLINE)?;
            }

            // // XXX Hacky.
            // if len(w.curBuf.Lines) > 0 {
            //     w.curBuf.Lines = w.curBuf.Lines[1:]
            // }
        }

        for (i, line) in buffer.lines.iter().enumerate() {
            let i: usize = i;
            let line: &Line = line;

            if i > 0 {
                out.write_all(b"\n")?;
            }

            // First cell where `buffer` and `old_buffer` differ for the line.
            let mut j = 0;

            // If not a full refresh attempt to avoid rewriting unchanged sections of line.
            if !refresh {
                if let Some(old_line) = old_buffer.lines.get(i) {
                    match line.find_difference(old_line) {
                        Some(diff) => j = diff,
                        // No need to update current line.
                        None => continue,
                    }

                    // Move to first differing column if necessary.
                    let first_col = line[..j].iter().map(|cell| wcswidth(&cell.text)).sum();
                    if first_col > 0 {
                        write!(out, "{}", cursor::MoveRight(first_col))?;
                    }

                    // Clear the rest of the line if necessary.
                    if j < old_line.len() {
                        switch_style!(None);
                        writeln!(out, "{}", CLEAR_UNTIL_NEWLINE)?;
                    }
                }
            }

            write_cells!(line[j..]);
        }

        if !refresh && old_buffer.lines.len() > buffer.lines.len() {
            // If the old buffer is higher, clear old content.
            switch_style!(None);
            write!(
                out,
                "{}\n{}{}",
                cursor::SavePosition,
                CLEAR_FROM_CURSOR_DOWN,
                cursor::RestorePosition,
            )?;
        }

        switch_style!(None);

        // Move the cursor to the buffer `dot`.
        let cursor = buffer.cursor();
        write_move(&mut out, cursor, buffer.dot)?;

        // Show cursor.
        write!(out, "{}", cursor::Show)?;

        // Flush buffer.
        out.flush()?;

        // Update old buffer.
        *old_buffer = buffer;

        Ok(())
    }
}

fn write_move<W: Write>(w: &mut W, from: Pos, to: Pos) -> Result<()> {
    if from.line < to.line {
        write!(w, "{}", cursor::MoveDown(to.line - from.line))?;
    } else if from.line > to.line {
        write!(w, "{}", cursor::MoveUp(from.line - to.line))?;
    }

    if to.col > 0 {
        write!(w, "\r{}", cursor::MoveRight(to.col))?;
    }

    Ok(())
}
