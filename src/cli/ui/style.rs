pub use crate::cli::term::style::{Color, Style, StyleFlags};

pub struct Styler {
    style: Style,
}

impl Styler {
    pub(super) fn new(style: Style) -> Styler {
        Styler { style }
    }

    pub(super) fn build(self) -> Style {
        self.style
    }
}

impl Styler {
    pub fn fg<C>(&mut self, color: C) -> &mut Self
    where
        C: Into<Option<Color>>,
    {
        self.style.fg = color.into();
        self
    }

    pub fn bg<C>(&mut self, color: C) -> &mut Self
    where
        C: Into<Option<Color>>,
    {
        self.style.bg = color.into();
        self
    }

    pub fn bold(&mut self, bold: bool) -> &mut Self {
        self.style.flags.set(StyleFlags::BOLD, bold);
        self
    }

    pub fn dim(&mut self, dim: bool) -> &mut Self {
        self.style.flags.set(StyleFlags::DIM, dim);
        self
    }

    pub fn italic(&mut self, italic: bool) -> &mut Self {
        self.style.flags.set(StyleFlags::ITALIC, italic);
        self
    }

    pub fn underlined(&mut self, underlined: bool) -> &mut Self {
        self.style.flags.set(StyleFlags::UNDERLINED, underlined);
        self
    }

    pub fn blink(&mut self, blink: bool) -> &mut Self {
        self.style.flags.set(StyleFlags::BLINK, blink);
        self
    }

    pub fn reverse(&mut self, reverse: bool) -> &mut Self {
        self.style.flags.set(StyleFlags::REVERSE, reverse);
        self
    }
}
