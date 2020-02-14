mod builder;

use std::borrow::Cow;
use std::fmt::{self, Debug, Write};
use std::iter::IntoIterator;
use std::ops::{Bound, Deref, DerefMut, RangeBounds};

use crate::cli::term::style::Style;
use crate::cli::term::utils::wcswidth;

pub use self::builder::BufferBuilder;

const DEFAULT_LINE: u16 = 0;
const DEFAULT_COL: u16 = 0;

/// An indivisible unit on the screen. It is not necessarily 1 column wide.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cell {
    // TODO: Replace with an enum { String(Cow<'static, str>), Char(char) }.
    pub text: Cow<'static, str>,
    pub style: Option<Style>,
}

/// A line/column position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Pos {
    pub col: u16,
    pub line: u16,
}

impl Pos {
    #[inline]
    pub fn new(col: u16, line: u16) -> Pos {
        Pos { col, line }
    }
}

impl Default for Pos {
    #[inline]
    fn default() -> Self {
        Pos {
            col: DEFAULT_COL,
            line: DEFAULT_LINE,
        }
    }
}

/// A single line.
#[derive(Clone, Debug)]
pub struct Line(Vec<Cell>);

impl Line {
    pub fn new(width: u16) -> Line {
        Line(Vec::with_capacity(width as usize))
    }

    pub fn width(&self) -> u16 {
        Self::width_slice(self)
    }

    #[inline]
    pub fn width_slice(slice: &[Cell]) -> u16 {
        slice
            .iter()
            .map(|cell| wcswidth(&cell.text))
            .fold(0u16, std::ops::Add::add)
    }

    /// Find the column of the first difference between this and another line.
    pub fn find_difference(&self, other: &Line) -> Option<usize> {
        for (i, cell) in self.iter().enumerate() {
            match other.get(i) {
                Some(other_cell) if cell == other_cell => {}
                _ => return Some(i),
            }
        }

        if self.len() < other.len() {
            return Some(self.len());
        }

        None
    }
}

impl Deref for Line {
    type Target = Vec<Cell>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Line {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for Line {
    type Item = Cell;
    type IntoIter = <Vec<Cell> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Line {
    type Item = &'a Cell;
    type IntoIter = <&'a Vec<Cell> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

/// Multiple lines.
#[derive(Clone, Debug)]
pub struct Lines(Vec<Line>);

impl Deref for Lines {
    type Target = Vec<Line>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Lines {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for Lines {
    type Item = Line;
    type IntoIter = <Vec<Line> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Lines {
    type Item = &'a Line;
    type IntoIter = <&'a Vec<Line> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

#[derive(Clone)]
pub struct Buffer {
    /// The width of the screen.
    pub width: u16,
    /// The content of the buffer.
    pub lines: Lines,
    /// The position the user perceives as the position of the cursor.
    pub dot: Pos,
}

impl Buffer {
    pub const EMPTY: Buffer = Buffer {
        width: 0,
        lines: Lines(Vec::new()),
        dot: Pos {
            col: DEFAULT_COL,
            line: DEFAULT_LINE,
        },
    };

    pub fn builder(width: u16) -> BufferBuilder {
        BufferBuilder::new(width)
    }

    pub fn new(width: u16) -> Buffer {
        let lines = Lines(vec![Line::new(width)]);
        let dot = Pos::default();

        Buffer { width, lines, dot }
    }

    /// Returns the column the cursor is in.
    pub fn column(&self) -> u16 {
        match self.lines.0.last() {
            Some(line) => line.width(),
            None => DEFAULT_COL,
        }
    }

    /// Returns the current position of the cursor.
    pub fn cursor(&self) -> Pos {
        match self.lines.0.len().checked_sub(1) {
            Some(line) => {
                // SAFETY: `line` is guaranteed to be < `self.lines.0.len()`.
                let last_line = unsafe { self.lines.get_unchecked(line) };

                let line = line as u16;
                let col = last_line.width();

                Pos { line, col }
            }
            None => Pos::default(),
        }
    }

    pub fn trim_to_lines<R: RangeBounds<usize>>(&mut self, range: R) {
        // Inclusive start bound.
        let start = match range.start_bound() {
            Bound::Unbounded | Bound::Included(0) => None,
            Bound::Included(&start) => Some(start),
            Bound::Excluded(&start) => start.checked_add(1),
        };

        // Exclusive end bound.
        let end = match range.end_bound() {
            Bound::Included(&end) => end.checked_add(1),
            Bound::Excluded(&end) => Some(end),
            Bound::Unbounded => None,
        };
        // Limit end bound to range of lines.
        let end = match end {
            Some(end) if end >= self.lines.len() => None,
            end => end,
        };

        match (start, end) {
            // No-op.
            (None, None) => {}

            // Reallocation required for shifted start.
            (Some(start), end) => {
                let slice = match end {
                    Some(end) => &self.lines[start..end],
                    None => &self.lines[start..],
                };

                self.lines = Lines(slice.to_vec());

                // Shift dot up.
                self.dot.line = self.dot.line.saturating_sub(start as u16);
            }

            (None, Some(end)) => {
                self.lines.truncate(end);
            }
        }
    }

    pub fn extend(&mut self, buffer: &Buffer, move_dot: bool) {
        if move_dot {
            self.dot.line = buffer.dot.line + self.lines.len() as u16;
            self.dot.col = buffer.dot.col;
        }
        self.lines.extend_from_slice(&buffer.lines);
    }

    pub fn new_line(&mut self, move_dot: bool, width: Option<u16>) {
        if move_dot {
            self.dot.line += 1;
            self.dot.col = DEFAULT_COL;
        }
        self.lines.push(Line::new(width.unwrap_or(self.width)))
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Header.
        writeln!(
            f,
            "Buffer {{ width: {}, dot: ({}, {}), lines: {} }}",
            self.width,
            self.dot.col,
            self.dot.line,
            self.lines.len(),
        )?;

        // Top border.
        writeln!(f, "┌{:─<width$}┐", "", width = self.width as usize)?;

        for line in &self.lines {
            // Left border.
            f.write_char('│')?;

            let mut last_style = None;
            let mut used_width = 0;

            for cell in line {
                // Apply appropriate styles.
                if cell.style != last_style {
                    match (last_style, cell.style) {
                        (None, Some(style)) => write!(f, "\x1b[{}m", style)?,
                        (_, None) => f.write_str("\x1b[m")?,
                        (Some(_), Some(style)) => write!(f, "\x1b[;{}m", style)?,
                    }

                    last_style = cell.style;
                }

                // Write cell text.
                f.write_str(&cell.text)?;
                // Add cell text width to `used_width`.
                used_width += wcswidth(&cell.text);
            }

            // Write reset string if ending the line with a style.
            if let Some(_) = &last_style {
                f.write_str("\x1b[m")?;
            }

            if let Some(rem) = self.width.checked_sub(used_width + 1) {
                write!(f, "${:rem$}", "", rem = rem as usize)?;
            }

            // Right border and new line.
            f.write_str("│\n")?;
        }

        // Bottom border.
        writeln!(f, "└{:─<width$}┘", "", width = self.width as usize)?;

        Ok(())
    }
}
