use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn wcwidth(c: char) -> u16 {
    match UnicodeWidthChar::width(c) {
        Some(width) => width as u16,
        None => 0,
    }
}

pub fn wcswidth(s: &str) -> u16 {
    UnicodeWidthStr::width(s) as u16
}
