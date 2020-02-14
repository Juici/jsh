use std::fmt::{self, Display, Write};

bitflags::bitflags! {
    pub struct StyleFlags: u8 {
        const BOLD        = 1 << 0;
        const DIM         = 1 << 1;
        const ITALIC      = 1 << 2;
        const UNDERLINED  = 1 << 3;
        const BLINK       = 1 << 4;
        const REVERSE     = 1 << 5;
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub flags: StyleFlags,
}

impl Style {
    pub const RESET: Style = Style {
        fg: None,
        bg: None,
        flags: StyleFlags::empty(),
    };
}

impl Default for Style {
    fn default() -> Self {
        Style::RESET
    }
}

impl Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut req_sep = false;

        macro_rules! write_sep {
            () => {
                // Unused assignments should be optimized out.
                #[allow(unused_assignments)]
                {
                    if req_sep {
                        f.write_char(';')?;
                    } else {
                        req_sep = true;
                    }
                }
            };
        }

        macro_rules! add_style {
            ($flag:expr, $code:literal) => {{
                let flag: StyleFlags = $flag;
                let code: u8 = $code;

                if self.flags.contains(flag) {
                    write_sep!();
                    write!(f, "{}", code)?;
                }
            }};
        }

        add_style!(StyleFlags::BOLD, 1);
        add_style!(StyleFlags::DIM, 2);
        add_style!(StyleFlags::ITALIC, 3);
        add_style!(StyleFlags::UNDERLINED, 4);
        add_style!(StyleFlags::BLINK, 5);
        add_style!(StyleFlags::REVERSE, 6);

        if let Some(fg) = self.fg {
            write_sep!();
            fg.write_fg(f)?;
        }

        if let Some(bg) = self.bg {
            write_sep!();
            bg.write_bg(f)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,

    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,

    Xterm256(u8),

    TrueColor { r: u8, g: u8, b: u8 },
}

macro_rules! write_ansi {
    ($dst:expr, fg[$n:expr]) => {{
        const N: u8 = 30 + $n;
        $dst.write_fmt(format_args!("{}", N))
    }};
    ($dst:expr, bg[$n:expr]) => {{
        const N: u8 = 40 + $n;
        $dst.write_fmt(format_args!("{}", N))
    }};
}

macro_rules! write_ansi_bright {
    ($dst:expr, fg[$n:expr]) => {{
        const N: u8 = 90 + $n;
        $dst.write_fmt(format_args!("{}", N))
    }};
    ($dst:expr, bg[$n:expr]) => {{
        const N: u8 = 100 + $n;
        $dst.write_fmt(format_args!("{}", N))
    }};
}

impl Color {
    fn write_fg(self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Color::*;

        match self {
            Black => write_ansi!(f, fg[0]),
            Red => write_ansi!(f, fg[1]),
            Green => write_ansi!(f, fg[2]),
            Yellow => write_ansi!(f, fg[3]),
            Blue => write_ansi!(f, fg[4]),
            Magenta => write_ansi!(f, fg[5]),
            Cyan => write_ansi!(f, fg[6]),
            White => write_ansi!(f, fg[7]),

            BrightBlack => write_ansi_bright!(f, fg[0]),
            BrightRed => write_ansi_bright!(f, fg[1]),
            BrightGreen => write_ansi_bright!(f, fg[2]),
            BrightYellow => write_ansi_bright!(f, fg[3]),
            BrightBlue => write_ansi_bright!(f, fg[4]),
            BrightMagenta => write_ansi_bright!(f, fg[5]),
            BrightCyan => write_ansi_bright!(f, fg[6]),
            BrightWhite => write_ansi_bright!(f, fg[7]),

            Xterm256(n) => write!(f, "38;5;{}", n),

            TrueColor { r, g, b } => write!(f, "38;2;{};{};{}", r, g, b),
        }
    }

    fn write_bg(self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Color::*;

        match self {
            Black => write_ansi!(f, bg[0]),
            Red => write_ansi!(f, bg[1]),
            Green => write_ansi!(f, bg[2]),
            Yellow => write_ansi!(f, bg[3]),
            Blue => write_ansi!(f, bg[4]),
            Magenta => write_ansi!(f, bg[5]),
            Cyan => write_ansi!(f, bg[6]),
            White => write_ansi!(f, bg[7]),

            BrightBlack => write_ansi_bright!(f, bg[0]),
            BrightRed => write_ansi_bright!(f, bg[1]),
            BrightGreen => write_ansi_bright!(f, bg[2]),
            BrightYellow => write_ansi_bright!(f, bg[3]),
            BrightBlue => write_ansi_bright!(f, bg[4]),
            BrightMagenta => write_ansi_bright!(f, bg[5]),
            BrightCyan => write_ansi_bright!(f, bg[6]),
            BrightWhite => write_ansi_bright!(f, bg[7]),

            Xterm256(n) => write!(f, "48;5;{}", n),

            TrueColor { r, g, b } => write!(f, "48;2;{};{};{}", r, g, b),
        }
    }
}
