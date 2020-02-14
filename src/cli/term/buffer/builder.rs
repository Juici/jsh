use std::borrow::Cow;

use super::{Buffer, Cell, Line, Lines, Pos};

use crate::cli::term::style::{Style, StyleFlags};
use crate::cli::term::utils::wcswidth;
use crate::cli::ui::Text;

#[derive(Debug)]
pub struct BufferBuilder {
    pub width: u16,
    pub col: u16,
    pub indent: u16,

    pub eager_wrap: bool,

    pub lines: Lines,
    pub dot: Pos,
}

impl BufferBuilder {
    pub fn new(width: u16) -> BufferBuilder {
        BufferBuilder {
            width,
            col: 0,
            indent: 0,

            eager_wrap: false,

            lines: Lines(vec![Line::new(width)]),
            dot: Default::default(),
        }
    }

    pub fn cursor(&self) -> Pos {
        Pos {
            col: self.col,
            line: (self.lines.len() - 1) as u16,
        }
    }

    pub fn buffer(self) -> Buffer {
        let BufferBuilder {
            width, lines, dot, ..
        } = self;

        Buffer { width, lines, dot }
    }

    pub fn indent(&mut self, indent: u16) -> &mut Self {
        self.indent = indent;
        self
    }

    pub fn eager_wrap(&mut self, wrap: bool) -> &mut Self {
        self.eager_wrap = wrap;
        self
    }

    pub fn lines(&mut self, lines: Lines) -> &mut Self {
        self.col = lines.last().map_or(0, |line| line.width());
        self.lines = lines;
        self
    }

    pub fn dot(&mut self) -> &mut Self {
        self.dot = self.cursor();
        self
    }

    pub fn newline(&mut self) -> &mut Self {
        self.push_line();

        if self.indent > 0 {
            self.push_cell_n(
                Cell {
                    text: Cow::Borrowed(" "),
                    style: None,
                },
                self.indent,
            );
        }

        self
    }

    pub fn write_char_styled(&mut self, c: char, mut style: Style) -> &mut Self {
        let cell = match c {
            '\n' => return self.newline(),
            '\0'..='\x1f' | '\x7f' => {
                style.flags.insert(StyleFlags::REVERSE);

                // TODO: Look into using static string array lookup.
                let c = (c as u8) ^ 0x40;

                Cell {
                    text: Cow::Owned(format!("^{}", char::from(c))),
                    style: Some(style),
                }
            }
            _ => Cell {
                text: Cow::Owned(c.to_string()),
                style: Some(style),
            },
        };

        if self.col + wcswidth(&cell.text) > self.width {
            self.newline();
            self.push_cell(cell);
        } else {
            self.push_cell(cell);
            if self.col == self.width && self.eager_wrap {
                self.newline();
            }
        }

        self
    }

    pub fn write_char(&mut self, c: char) -> &mut Self {
        self.write_char_styled(c, Style::RESET)
    }

    pub fn write_spaces_styled(&mut self, n: usize, style: Style) -> &mut Self {
        for _ in 0..n {
            let cell = Cell {
                text: Cow::Borrowed(" "),
                style: Some(style),
            };

            const WCWIDTH_SPACE: u16 = 1;

            if self.col + WCWIDTH_SPACE > self.width {
                self.newline();
                self.push_cell(cell);
            } else {
                self.push_cell(cell);
                if self.col == self.width && self.eager_wrap {
                    self.newline();
                }
            }
        }
        self
    }

    pub fn write_spaces(&mut self, n: usize) -> &mut Self {
        self.write_spaces_styled(n, Style::RESET)
    }

    pub fn write_str_styled(&mut self, s: &str, style: Style) -> &mut Self {
        for c in s.chars() {
            self.write_char_styled(c, style);
        }
        self
    }

    pub fn write_str(&mut self, s: &str) -> &mut Self {
        self.write_str_styled(s, Style::RESET)
    }

    pub fn write_text(&mut self, text: &Text) -> &mut Self {
        for seg in text {
            self.write_str_styled(&seg.text, seg.style);
        }
        self
    }
}

impl BufferBuilder {
    fn push_line(&mut self) {
        self.lines.push(Line::new(self.width));
        self.col = 0;
    }

    fn push_cell(&mut self, cell: Cell) {
        self.col += wcswidth(&cell.text);

        // Push cell to last line, adds line if necessary.
        match self.lines.last_mut() {
            Some(line) => line.push(cell),
            None => {
                let mut line = Line::new(self.width);
                line.push(cell);

                self.lines.push(line);
            }
        }
    }

    fn push_cells(&mut self, cells: &[Cell]) {
        self.col += cells.iter().map(|cell| wcswidth(&cell.text)).sum::<u16>();

        // Push cell to last line, adds line if necessary.
        match self.lines.last_mut() {
            Some(line) => line.extend_from_slice(cells),
            None => {
                let mut line = Line::new(self.width);
                line.extend_from_slice(cells);

                self.lines.push(line);
            }
        }
    }

    fn push_cell_n(&mut self, cell: Cell, n: u16) {
        self.col += wcswidth(&cell.text) * n;

        // Push cell to last line, adds line if necessary.
        match self.lines.last_mut() {
            Some(line) => {
                line.reserve(n as usize);
                for _ in 0..n {
                    line.push(cell.clone());
                }
            }
            None => {
                let mut line = Line::new(self.width.max(n));
                for _ in 0..n {
                    line.push(cell.clone());
                }

                self.lines.push(line);
            }
        }
    }
}
