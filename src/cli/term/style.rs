use std::fmt::{self, Display};

use crossterm::style::{SetAttribute, SetBackgroundColor, SetForegroundColor};

pub use crossterm::style::{Attribute, Attributes, Color};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attributes,
}

impl Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}",
            SetForegroundColor(self.fg),
            SetBackgroundColor(self.bg)
        )?;

        for attr in Attribute::iterator() {
            if self.attrs.has(attr) {
                write!(f, "{}", SetAttribute(attr))?;
            }
        }

        Ok(())
    }
}

// bitflags::bitflags! {
//    pub struct StyleFlags: u8 {
//        const BOLD       = 1 << 0;
//        const DIM        = 1 << 1;
//        const ITALIC     = 1 << 2;
//        const UNDERLINED = 1 << 3;
//        const BLINK      = 1 << 4;
//        const REVERSE    = 1 << 5;
//    }
//}

//#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub struct Style {
//    pub fg: Option<Color>,
//    pub bg: Option<Color>,
//    pub flags: StyleFlags,
//}
// impl Display for Style {
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        let mut req_sep = false;
//
//        {
//            let mut add_style = |style: StyleFlags, code: u8| {
//                if self.flags.contains(style) {
//                    if req_sep {
//                        f.write_char(';')?;
//                    } else {
//                        req_sep = true;
//                    }
//                    write!(f, "{}", code)?;
//                }
//                Ok(())
//            };
//
//            add_style(StyleFlags::BOLD, 1)?;
//            add_style(StyleFlags::DIM, 2)?;
//            add_style(StyleFlags::ITALIC, 3)?;
//            add_style(StyleFlags::UNDERLINED, 4)?;
//            add_style(StyleFlags::BLINK, 5)?;
//            add_style(StyleFlags::REVERSE, 6)?;
//        }
//
//        if let Some(fg) = self.fg {
//            if req_sep {
//                f.write_char(';')?;
//            } else {
//                req_sep = true;
//            }
//            fg.write_fg(f)?;
//        }
//        if let Some(bg) = self.bg {
//            if req_sep {
//                f.write_char(';')?;
//            }
//            bg.write_fg(f)?;
//        }
//
//        Ok(())
//    }
//}

// impl Style {
//    pub fn set_bold(&mut self, set: bool) {
//        self.flags.set(StyleFlags::BOLD, set);
//    }
//
//    pub const fn bold(&self) -> bool {
//        self.flags.contains(StyleFlags::BOLD)
//    }
//
//    pub fn set_dim(&mut self, set: bool) {
//        self.flags.set(StyleFlags::DIM, set);
//    }
//
//    pub const fn dim(&self) -> bool {
//        self.flags.contains(StyleFlags::DIM)
//    }
//
//    pub fn set_italic(&mut self, set: bool) {
//        self.flags.set(StyleFlags::ITALIC, set);
//    }
//
//    pub const fn italic(&self) -> bool {
//        self.flags.contains(StyleFlags::ITALIC)
//    }
//
//    pub fn set_underlined(&mut self, set: bool) {
//        self.flags.set(StyleFlags::UNDERLINED, set);
//    }
//
//    pub const fn underlined(&self) -> bool {
//        self.flags.contains(StyleFlags::UNDERLINED)
//    }
//
//    pub fn set_blink(&mut self, set: bool) {
//        self.flags.set(StyleFlags::BLINK, set);
//    }
//
//    pub const fn blink(&self) -> bool {
//        self.flags.contains(StyleFlags::BLINK)
//    }
//
//    pub fn set_reverse(&mut self, set: bool) {
//        self.flags.set(StyleFlags::REVERSE, set);
//    }
//
//    pub const fn reverse(&self) -> bool {
//        self.flags.contains(StyleFlags::REVERSE)
//    }
//}

// pub trait WriteColor: Copy {
//    fn write_fg<W: Write>(self, out: &mut W) -> fmt::Result;
//    fn write_bg<W: Write>(self, out: &mut W) -> fmt::Result;
//}
//
//#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub enum Color {
//    Ansi(AnsiColor),
//    AnsiBright(AnsiBrightColor),
//    Xterm256(Xterm256Color),
//    TrueColor(TrueColor),
//}
// impl WriteColor for Color {
//    fn write_fg<W: Write>(self, out: &mut W) -> fmt::Result {
//        match self {
//            Color::Ansi(color) => color.write_fg(out),
//            Color::AnsiBright(color) => color.write_fg(out),
//            Color::Xterm256(color) => color.write_fg(out),
//            Color::TrueColor(color) => color.write_fg(out),
//        }
//    }
//
//    fn write_bg<W: Write>(self, out: &mut W) -> fmt::Result {
//        match self {
//            Color::Ansi(color) => color.write_bg(out),
//            Color::AnsiBright(color) => color.write_bg(out),
//            Color::Xterm256(color) => color.write_bg(out),
//            Color::TrueColor(color) => color.write_bg(out),
//        }
//    }
//}
//
//#[repr(u8)]
//#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub enum AnsiColor {
//    Black = 0,
//    Red = 1,
//    Green = 2,
//    Yellow = 3,
//    Blue = 4,
//    Magenta = 5,
//    Cyan = 6,
//    White = 7,
//}
// impl AnsiColor {
//    fn value(self) -> u8 {
//        match self {
//            AnsiColor::Black => 0,
//            AnsiColor::Red => 1,
//            AnsiColor::Green => 2,
//            AnsiColor::Yellow => 3,
//            AnsiColor::Blue => 4,
//            AnsiColor::Magenta => 5,
//            AnsiColor::Cyan => 6,
//            AnsiColor::White => 7,
//        }
//    }
//}
// impl WriteColor for AnsiColor {
//    fn write_fg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "{}", 30 + self.value())
//    }
//
//    fn write_bg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "{}", 40 + self.value())
//    }
//}
//
//#[repr(u8)]
//#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub enum AnsiBrightColor {
//    Black = 0,
//    Red = 1,
//    Green = 2,
//    Yellow = 3,
//    Blue = 4,
//    Magenta = 5,
//    Cyan = 6,
//    White = 7,
//}
// impl AnsiBrightColor {
//    fn value(self) -> u8 {
//        match self {
//            AnsiBrightColor::Black => 0,
//            AnsiBrightColor::Red => 1,
//            AnsiBrightColor::Green => 2,
//            AnsiBrightColor::Yellow => 3,
//            AnsiBrightColor::Blue => 4,
//            AnsiBrightColor::Magenta => 5,
//            AnsiBrightColor::Cyan => 6,
//            AnsiBrightColor::White => 7,
//        }
//    }
//}
// impl WriteColor for AnsiBrightColor {
//    fn write_fg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "{}", 90 + self.value())
//    }
//
//    fn write_bg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "{}", 100 + self.value())
//    }
//}
//
//#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub struct Xterm256Color(pub u8);
//
// impl WriteColor for Xterm256Color {
//    fn write_fg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "38;5;{}", self.0)
//    }
//
//    fn write_bg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "48;5;{}", self.0)
//    }
//}
//
//#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub struct TrueColor {
//    pub r: u8,
//    pub g: u8,
//    pub b: u8,
//}
// impl WriteColor for TrueColor {
//    fn write_fg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "38;2;{};{};{}", self.r, self.g, self.b)
//    }
//
//    fn write_bg<W: Write>(self, out: &mut W) -> fmt::Result {
//        write!(out, "48;2;{};{};{}", self.r, self.g, self.b)
//    }
//}
