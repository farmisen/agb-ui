use core::fmt;

/// Bytes a [`Line`] holds: a line across a 30-column screen in the widest characters
/// (4 bytes each), and a few more.
const BYTES: usize = 128;

/// A line of text put together with `write!`, on the stack: a description, a title, a
/// number. What doesn't fit is dropped at a character's boundary, and `write!` reports
/// an error.
#[derive(Clone, Copy)]
pub struct Line {
    buf: [u8; BYTES],
    len: usize,
}

impl Line {
    pub const fn new() -> Self {
        Self {
            buf: [0; BYTES],
            len: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        // Only whole characters go in, so this is always UTF-8.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Keep the first `n` characters.
    pub fn truncate_chars(&mut self, n: usize) {
        if let Some((end, _)) = self.as_str().char_indices().nth(n) {
            self.len = end;
        }
    }
}

/// The text, not the bytes past it that `clear` and `truncate_chars` leave.
impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Line {}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            let end = self.len + c.len_utf8();
            let dst = self.buf.get_mut(self.len..end).ok_or(fmt::Error)?;
            c.encode_utf8(dst);
            self.len = end;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;

    #[test]
    fn drops_what_does_not_fit_at_a_character() {
        let mut line = Line::new();
        assert!(line.write_str(&"·".repeat(BYTES)).is_err());
        assert_eq!(line.as_str(), "·".repeat(BYTES / 2));
    }

    #[test]
    fn truncates_by_characters() {
        let mut line = Line::new();
        let _ = write!(line, "A·B·C");
        line.truncate_chars(3);
        assert_eq!(line.as_str(), "A·B");
        line.truncate_chars(10);
        assert_eq!(line.as_str(), "A·B");
    }

    #[test]
    fn compares_the_text_only() {
        let (mut a, mut b) = (Line::new(), Line::new());
        let _ = a.write_str("ABC");
        a.truncate_chars(1);
        let _ = b.write_str("A");
        assert!(a == b);
    }
}
