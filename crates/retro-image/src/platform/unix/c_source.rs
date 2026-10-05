//! A tokenizer for the C source that XBM and XPM files are written in.
//!
//! No external format knowledge: only the lexical rules of C, whitespace and
//! the two comment forms, which both formats rely on.

/// A cursor over C source that yields words and single punctuation marks,
/// skipping whitespace and comments.
pub(super) struct Tokens<'a> {
    pub(super) data: &'a [u8],
    pub(super) pos: usize,
}

impl<'a> Tokens<'a> {
    pub(super) fn skip_blank(&mut self) {
        loop {
            let rest = &self.data[self.pos..];
            if rest.first().is_some_and(u8::is_ascii_whitespace) {
                self.pos += 1;
            } else if rest.starts_with(b"/*") {
                let end = rest[2..].windows(2).position(|w| w == b"*/");
                self.pos += end.map_or(rest.len(), |e| e + 4);
            } else if rest.starts_with(b"//") {
                let end = rest.iter().position(|&b| b == b'\n');
                self.pos += end.map_or(rest.len(), |e| e + 1);
            } else {
                return;
            }
        }
    }

    pub(super) fn next(&mut self) -> Option<&'a [u8]> {
        self.skip_blank();
        let rest = &self.data[self.pos..];
        let first = *rest.first()?;
        let len = if is_punctuation(first) {
            1
        } else {
            rest.iter()
                .position(|&b| b.is_ascii_whitespace() || is_punctuation(b))
                .unwrap_or(rest.len())
        };
        self.pos += len;
        Some(&rest[..len])
    }
}

fn is_punctuation(byte: u8) -> bool {
    matches!(byte, b',' | b'{' | b'}' | b'[' | b']' | b'=' | b';')
}
