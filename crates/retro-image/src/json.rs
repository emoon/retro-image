//! A small JSON reader for the formats that store pictures as JSON text.
//!
//! Source: the JSON grammar, ECMA-404 / RFC 8259 (<https://www.json.org/>,
//! <https://www.rfc-editor.org/rfc/rfc8259>). Written from the grammar
//! alone; no existing parser was consulted.
//!
//! Untrusted input: nesting is limited to [`MAX_DEPTH`] levels, and the
//! reader never indexes past the end of the text.

use alloc::string::String;
use alloc::vec::Vec;

/// Deepest array/object nesting accepted.
const MAX_DEPTH: usize = 32;

/// A parsed JSON value.
#[derive(Debug)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    /// Members in file order; [`Value::get`] finds the first of a repeated key.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Parses a complete JSON text (one value, optional surrounding
    /// whitespace and a UTF-8 byte order mark).
    pub(crate) fn parse(text: &[u8]) -> Option<Value> {
        let text = text.strip_prefix(b"\xef\xbb\xbf").unwrap_or(text);
        let mut reader = Reader { text, pos: 0 };
        let value = reader.value(0)?;
        reader.skip_space();
        (reader.pos == text.len()).then_some(value)
    }

    /// The member `key` of an object.
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub(crate) fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The number, if it is a whole number of modest size.
    pub(crate) fn as_int(&self) -> Option<i64> {
        match *self {
            Value::Number(n) if n > -9.0e15 && n < 9.0e15 && n == (n as i64) as f64 => {
                Some(n as i64)
            }
            _ => None,
        }
    }
}

struct Reader<'a> {
    text: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.pos).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        (self.peek() == Some(byte)).then(|| self.pos += 1)
    }

    fn keyword(&mut self, word: &[u8], value: Value) -> Option<Value> {
        let end = self.pos.checked_add(word.len())?;
        (self.text.get(self.pos..end)? == word).then(|| {
            self.pos = end;
            value
        })
    }

    fn value(&mut self, depth: usize) -> Option<Value> {
        self.skip_space();
        match self.peek()? {
            b'{' if depth < MAX_DEPTH => self.object(depth),
            b'[' if depth < MAX_DEPTH => self.array(depth),
            b'"' => self.string().map(Value::String),
            b't' => self.keyword(b"true", Value::Bool(true)),
            b'f' => self.keyword(b"false", Value::Bool(false)),
            b'n' => self.keyword(b"null", Value::Null),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn array(&mut self, depth: usize) -> Option<Value> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_space();
        if self.eat(b']').is_some() {
            return Some(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();
            match self.peek()? {
                b',' => self.pos += 1,
                b']' => {
                    self.pos += 1;
                    return Some(Value::Array(items));
                }
                _ => return None,
            }
        }
    }

    fn object(&mut self, depth: usize) -> Option<Value> {
        self.pos += 1;
        let mut members = Vec::new();
        self.skip_space();
        if self.eat(b'}').is_some() {
            return Some(Value::Object(members));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            self.skip_space();
            self.eat(b':')?;
            members.push((key, self.value(depth + 1)?));
            self.skip_space();
            match self.peek()? {
                b',' => self.pos += 1,
                b'}' => {
                    self.pos += 1;
                    return Some(Value::Object(members));
                }
                _ => return None,
            }
        }
    }

    fn number(&mut self) -> Option<Value> {
        let start = self.pos;
        self.eat(b'-');
        match self.peek()? {
            b'0' => self.pos += 1,
            b'1'..=b'9' => self.digits(),
            _ => return None,
        }
        if self.eat(b'.').is_some() {
            self.digits_required()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            self.digits_required()?;
        }
        let text = core::str::from_utf8(&self.text[start..self.pos]).ok()?;
        text.parse().ok().map(Value::Number)
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
    }

    fn digits_required(&mut self) -> Option<()> {
        let start = self.pos;
        self.digits();
        (self.pos > start).then_some(())
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = self.text.get(self.pos..self.pos.checked_add(4)?)?;
        let mut code = 0;
        for &d in digits {
            code = code * 16 + char::from(d).to_digit(16)?;
        }
        self.pos += 4;
        Some(code)
    }

    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let mut bytes = Vec::new();
        loop {
            let byte = self.peek()?;
            self.pos += 1;
            match byte {
                b'"' => return String::from_utf8(bytes).ok(),
                0..=0x1f => return None,
                b'\\' => {
                    let escape = self.peek()?;
                    self.pos += 1;
                    let ch = match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.unicode_escape()?,
                        _ => return None,
                    };
                    let mut buf = [0; 4];
                    bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                _ => bytes.push(byte),
            }
        }
    }

    /// After `\u`: one code unit, or a surrogate pair. A lone surrogate
    /// becomes U+FFFD.
    fn unicode_escape(&mut self) -> Option<char> {
        let first = self.hex4()?;
        if !(0xd800..0xdc00).contains(&first) {
            return Some(char::from_u32(first).unwrap_or('\u{fffd}'));
        }
        let saved = self.pos;
        if self.text.get(self.pos..self.pos + 2) == Some(b"\\u") {
            self.pos += 2;
            let second = self.hex4()?;
            if (0xdc00..0xe000).contains(&second) {
                let code = 0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00);
                return char::from_u32(code);
            }
        }
        self.pos = saved;
        Some('\u{fffd}')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_values_and_escapes() {
        let text = b"{\"a\":[1,-2.5e1,true,null],\"s\":\"x\\u00e9\\ud83d\\ude00\\n\"}";
        let v = Value::parse(text).unwrap();
        let items = v.get("a").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].as_int(), Some(1));
        assert_eq!(items[1].as_int(), Some(-25));
        assert_eq!(v.get("s").unwrap().as_str(), Some("x\u{e9}\u{1f600}\n"));
    }

    #[test]
    fn rejects_malformed_and_too_deep_input() {
        let bad: [&[u8]; 6] = [b"{", b"[1,]", b"{\"a\" 1}", b"01", b"\"\x01\"", b"[1] x"];
        for text in bad {
            assert!(Value::parse(text).is_none());
        }
        let deep = alloc::vec![b'['; MAX_DEPTH + 2];
        assert!(Value::parse(&deep).is_none());
    }
}
