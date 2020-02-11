use std::io::{BufWriter, Stdin, Stdout, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::Arc;

use anyhow::Result;
use termios::Termios;

use super::error::TermError;
use super::utils::wcwidth;

macro_rules! missing_eol_char {
    () => {
        '⏎'
    };
}

const MISSING_EOL_CHAR: char = missing_eol_char!();
const MISSING_EOL: &str = concat!("\x1b[7m", missing_eol_char!(), "\x1b[m");

#[must_use = "if unused the terminal state will immediately restore"]
pub struct RestoreTerm {
    fd: RawFd,
    saved_term: Termios,
    _restore_vt: RestoreVt,
}

pub fn setup(stdin: &Stdin, stdout: Arc<Stdout>) -> Result<RestoreTerm> {
    let fd = stdin.as_raw_fd();

    let mut term = Termios::from_fd(fd).map_err(TermError::GetAttributes)?;

    // Copy current state, to be restored later.
    let saved_term = term;

    term.c_lflag &= !termios::ICANON; // Disable buffered input.
    term.c_lflag &= !termios::ECHO; // Disable echoed output.

    term.c_lflag &= !termios::ISIG; // Intercept signals.

    term.c_cc[termios::VMIN] = 1; // Minimum of 1 character for non-canon read.
    term.c_cc[termios::VTIME] = 0; // Timeout instantly for non-canon read.

    // Enforce `crnl` translation on read line.
    // Assuming user won't set `inlcr` or `-onlcr`.
    term.c_iflag |= termios::ICRNL;

    // Apply changes.
    termios::tcsetattr(fd, termios::TCSANOW, &term).map_err(TermError::GetAttributes)?;

    // Setup VT.
    let _restore_vt = setup_vt(stdout).map_err(TermError::SetupVt)?;

    Ok(RestoreTerm {
        fd,
        saved_term,
        _restore_vt,
    })
}

impl Drop for RestoreTerm {
    fn drop(&mut self) {
        // Restore saved terminal state, panic on failure.
        termios::tcsetattr(self.fd, termios::TCSANOW, &self.saved_term).unwrap();
    }
}

#[must_use = "if unused the VT will immediately restore"]
struct RestoreVt {
    stdout: Arc<Stdout>,
}

fn setup_vt(stdout: Arc<Stdout>) -> Result<RestoreVt> {
    let (cols, _) = crossterm::terminal::size()?;
    let pad = (cols - wcwidth(MISSING_EOL_CHAR)) as usize;

    {
        let mut buf = BufWriter::new(stdout.lock());

        // Write the `MISSING_EOL` character if the cursor is not in the first
        // column.
        //
        // 1. "\x1b[?7h"
        //
        //    Enable auto wrap.
        //
        // 2. "{:pad$}"
        //
        //    Write `MISSING_EOL` character with enough padding for the total
        //    width to equal the width of the terminal.
        //
        //    If the cursor was in the first column, we are still on the same line.
        //    Otherwise, we are now on the next line.
        //
        // 3. "\r \r"
        //
        //    Move cursor to first column, write one space and move back to the first
        //    column. If the cursor was in the first column, we have erased the
        //    `MISSING_EOL` character. Otherwise, we are now on the next line and this
        //    is a no-op.
        write!(buf, "\x1b[?7h{:pad$}\r \r", MISSING_EOL, pad = pad)?;

        // Disable auto wrap.
        buf.write_all(b"\x1b[?7l")?;

        // Enable bracketed paste mode.
        buf.write_all(b"\x1b[?2004h")?;

        // Flush stdout.
        buf.flush()?;
    }

    Ok(RestoreVt { stdout })
}

impl Drop for RestoreVt {
    fn drop(&mut self) {
        let mut buf = BufWriter::new(self.stdout.lock());

        // Enable auto wrap.
        buf.write_all(b"\x1b[?7h").unwrap();

        // Disable bracketed paste mode.
        buf.write_all(b"\x1b[?2004l").unwrap();

        // Move the cursor to the first column.
        buf.write_all(b"\r").unwrap();

        // Flush stdout.
        buf.flush().unwrap();
    }
}
