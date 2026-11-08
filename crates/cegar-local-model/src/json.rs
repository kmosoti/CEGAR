//! Minimal strict JSON reader for one fixed response shape.
//!
//! This is not a general JSON library. It exists so the untrusted model adapter
//! can reject anything it does not fully understand instead of coercing it. It
//! is byte-bounded, depth-bounded, rejects duplicate object keys and rejects
//! trailing input.

/// Maximum nesting depth accepted.
const MAX_DEPTH: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Typed object lookup that rejects duplicates.
    pub fn field(&self, name: &str) -> Option<&Value> {
        let Self::Object(entries) = self else {
            return None;
        };
        entries
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Count of object entries, for exact-shape checks.
    pub fn object_len(&self) -> Option<usize> {
        match self {
            Self::Object(entries) => Some(entries.len()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub &'static str);

pub fn parse(input: &[u8]) -> Result<Value, ParseError> {
    let mut parser = Parser {
        bytes: input,
        cursor: 0,
    };
    parser.skip_whitespace();
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.cursor != parser.bytes.len() {
        return Err(ParseError("trailing input"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.cursor += 1;
        Some(byte)
    }

    fn expect(&mut self, byte: u8) -> Result<(), ParseError> {
        match self.bump() {
            Some(found) if found == byte => Ok(()),
            _ => Err(ParseError("unexpected byte")),
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.cursor += 1;
        }
    }

    fn literal(&mut self, text: &[u8]) -> Result<(), ParseError> {
        if self.bytes[self.cursor..].starts_with(text) {
            self.cursor += text.len();
            Ok(())
        } else {
            Err(ParseError("bad literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth > MAX_DEPTH {
            return Err(ParseError("nesting too deep"));
        }
        match self.peek() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Value::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Value::Bool(false))
            }
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(ParseError("unexpected value")),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.cursor += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => return Ok(Value::Array(items)),
                _ => return Err(ParseError("bad array")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.expect(b'{')?;
        let mut entries: Vec<(String, Value)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.cursor += 1;
            return Ok(Value::Object(entries));
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            if entries.iter().any(|(existing, _)| *existing == key) {
                return Err(ParseError("duplicate object key"));
            }
            self.skip_whitespace();
            self.expect(b':')?;
            self.skip_whitespace();
            let value = self.value(depth + 1)?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.bump() {
                Some(b',') => continue,
                Some(b'}') => return Ok(Value::Object(entries)),
                _ => return Err(ParseError("bad object")),
            }
        }
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.cursor;
        if self.peek() == Some(b'-') {
            self.cursor += 1;
        }
        let mut digits = 0usize;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.cursor += 1;
            digits += 1;
        }
        if digits == 0 {
            return Err(ParseError("bad number"));
        }
        // JSON forbids leading zeros.
        let first_digit = self.bytes[start + usize::from(self.bytes[start] == b'-')];
        if first_digit == b'0' && digits > 1 {
            return Err(ParseError("leading zero"));
        }
        if self.peek() == Some(b'.') {
            self.cursor += 1;
            let mut fraction = 0usize;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.cursor += 1;
                fraction += 1;
            }
            if fraction == 0 {
                return Err(ParseError("bad fraction"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.cursor += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.cursor += 1;
            }
            let mut exponent = 0usize;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.cursor += 1;
                exponent += 1;
            }
            if exponent == 0 {
                return Err(ParseError("bad exponent"));
            }
        }
        let text = core::str::from_utf8(&self.bytes[start..self.cursor])
            .map_err(|_| ParseError("bad utf8"))?;
        let value: f64 = text.parse().map_err(|_| ParseError("bad number"))?;
        if !value.is_finite() {
            return Err(ParseError("non-finite number"));
        }
        Ok(Value::Number(value))
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(ParseError("unterminated string")),
                Some(b'"') => return Ok(out),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'b') => out.push('\u{8}'),
                    Some(b'f') => out.push('\u{c}'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'u') => {
                        let code = self.hex4()?;
                        let ch = char::from_u32(code).ok_or(ParseError("bad escape"))?;
                        out.push(ch);
                    }
                    _ => return Err(ParseError("bad escape")),
                },
                Some(byte) if byte < 0x20 => return Err(ParseError("control byte in string")),
                Some(byte) => {
                    // Re-decode from the raw input so multi-byte UTF-8 is
                    // validated rather than passed through byte by byte.
                    let start = self.cursor - 1;
                    let mut end = start + 1;
                    while end < self.bytes.len() && (self.bytes[end] & 0xC0) == 0x80 {
                        end += 1;
                    }
                    let text = core::str::from_utf8(&self.bytes[start..end])
                        .map_err(|_| ParseError("bad utf8"))?;
                    out.push_str(text);
                    self.cursor = end;
                    if byte.is_ascii() {
                        let _ = byte;
                    }
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let byte = self.bump().ok_or(ParseError("short escape"))?;
            let digit = match byte {
                b'0'..=b'9' => (byte - b'0') as u32,
                b'a'..=b'f' => (byte - b'a' + 10) as u32,
                b'A'..=b'F' => (byte - b'A' + 10) as u32,
                _ => return Err(ParseError("bad hex digit")),
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_expected_response_shape() {
        let text = br#"{"model":"tinybert","results":[{"index":1,"score":9.18},{"index":0,"score":-10.8}]}"#;
        let value = parse(text).expect("parse");
        assert_eq!(
            value.field("model").and_then(Value::as_str),
            Some("tinybert")
        );
        let results = value
            .field("results")
            .and_then(Value::as_array)
            .expect("array");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].field("index").and_then(Value::as_f64), Some(1.0));
    }

    #[test]
    fn rejects_duplicate_keys() {
        assert!(parse(br#"{"a":1,"a":2}"#).is_err());
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(parse(br#"{"a":1}{}"#).is_err());
    }

    #[test]
    fn rejects_excessive_nesting() {
        let text = "[".repeat(32);
        assert!(parse(text.as_bytes()).is_err());
    }

    #[test]
    fn rejects_non_finite_and_bad_numbers() {
        assert!(parse(b"1e999").is_err());
        assert!(parse(b"-").is_err());
        assert!(parse(b"01").is_err());
    }

    #[test]
    fn decodes_unicode_escapes_and_utf8() {
        let value = parse("{\"k\":\"caf\u{e9}\\u00e9\"}".as_bytes()).expect("parse");
        assert_eq!(value.field("k").and_then(Value::as_str), Some("caféé"));
    }

    #[test]
    fn rejects_control_bytes_in_strings() {
        assert!(parse(b"\"a\x01b\"").is_err());
    }
}
