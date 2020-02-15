mod style;

use std::fmt::{self, Display};

pub use self::style::{Style, Styler};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextSegment {
    pub text: String,
    pub style: Style,
}

impl TextSegment {
    pub fn plain<S>(text: S) -> TextSegment
    where
        S: Into<String>,
    {
        TextSegment {
            text: text.into(),
            style: Style::RESET,
        }
    }

    pub fn styled<S, F>(text: S, style: F) -> TextSegment
    where
        S: Into<String>,
        F: Fn(&mut Styler) -> &mut Styler,
    {
        let mut styler = Styler::new(Style::RESET);
        style(&mut styler);
        let style = styler.build();

        TextSegment {
            text: text.into(),
            style,
        }
    }
}

impl Display for TextSegment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.style == Style::RESET {
            f.write_str(&self.text)
        } else {
            write!(
                f,
                "\x1b[{style}m{text}\x1b[m",
                style = self.style,
                text = self.text
            )
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Text {
    segments: Vec<TextSegment>,
}

impl Text {
    pub const EMPTY: Text = Text {
        segments: Vec::new(),
    };

    pub fn plain<S>(text: S) -> Text
    where
        S: Into<String>,
    {
        Text {
            segments: vec![TextSegment::plain(text)],
        }
    }

    pub fn styled<S, F>(text: S, style: F) -> Text
    where
        S: Into<String>,
        F: Fn(&mut Styler) -> &mut Styler,
    {
        Text {
            segments: vec![TextSegment::styled(text, style)],
        }
    }

    pub fn push(&mut self, segment: TextSegment) {
        self.segments.push(segment);
    }

    pub fn iter(&self) -> std::slice::Iter<TextSegment> {
        self.segments.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<TextSegment> {
        self.segments.iter_mut()
    }

    pub fn split_at(&self, index: usize) -> (Text, Text) {
        let segs = &*self.segments;
        let mut offset = 0;
        let mut seg0 = None::<TextSegment>;

        let mut t1 = Vec::<TextSegment>::new();

        let mut to_consume = index;

        'consume: while to_consume > 0 {
            match seg0.take() {
                Some(mut seg) => {
                    if seg.text.len() < to_consume {
                        to_consume -= seg.text.len();
                        t1.push(seg);
                        offset += 1;
                    } else {
                        let out = seg.text.split_off(to_consume);

                        t1.push(TextSegment {
                            text: out,
                            style: seg.style,
                        });
                        seg0 = Some(seg);

                        to_consume = 0;
                    }
                }
                None => match segs.get(offset) {
                    Some(seg) => {
                        if seg.text.len() < to_consume {
                            t1.push(seg.clone());
                            to_consume -= seg.text.len();
                            offset += 1;
                        } else {
                            let (out, left) = seg.text.split_at(to_consume);

                            t1.push(TextSegment {
                                text: out.to_owned(),
                                style: seg.style,
                            });
                            seg0 = Some(TextSegment {
                                text: left.to_owned(),
                                style: seg.style,
                            });

                            to_consume = 0;
                        }
                    }
                    None => break 'consume,
                },
            }
        }

        let t2 = match seg0 {
            Some(seg0) => {
                let offset = offset + 1;

                let mut len = 1;
                if let Some(segs_left) = segs.len().checked_sub(offset) {
                    len += segs_left;
                }

                let mut vec = Vec::with_capacity(len);
                vec.push(seg0);
                vec.extend_from_slice(&segs[offset..]);
                vec
            }
            None if offset < segs.len() => segs[offset..].to_vec(),
            _ => Vec::new(),
        };

        let t1 = Text { segments: t1 };
        let t2 = Text { segments: t2 };

        (t1, t2)
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for seg in &self.segments {
            seg.fmt(f)?;
        }
        Ok(())
    }
}

impl IntoIterator for Text {
    type Item = TextSegment;
    type IntoIter = std::vec::IntoIter<TextSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

impl<'a> IntoIterator for &'a Text {
    type Item = &'a TextSegment;
    type IntoIter = std::slice::Iter<'a, TextSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Text {
    type Item = &'a mut TextSegment;
    type IntoIter = std::slice::IterMut<'a, TextSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl Extend<TextSegment> for Text {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = TextSegment>,
    {
        self.segments.extend(iter)
    }
}

impl<'a> Extend<&'a TextSegment> for Text {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = &'a TextSegment>,
    {
        self.segments.extend(iter.into_iter().cloned())
    }
}

impl<'a> Extend<&'a mut TextSegment> for Text {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = &'a mut TextSegment>,
    {
        // Apparently we don't fit the bounds for `iter.cloned()` so we just map to
        // clone.
        self.segments
            .extend(iter.into_iter().map(|seg| seg.clone()))
    }
}
