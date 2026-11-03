//! The JSON document model and its decoder.
//!
//! The decoder reads the grammar the Python `json` decoder reads,
//! refusing an invalid document with the runtime decode wording,
//! the fault named with its line and column and character position
//! counted in characters over the text as written. A repeated key
//! in one object is refused at the close of that object. Integer
//! tokens that do not fit in a signed 64 bit integer keep their
//! exact digit text. Nesting is bounded at 4000 levels and a lone
//! surrogate escape is refused, each as a boundary the contract
//! states.

/// The deepest nesting the decoder accepts.
pub const MAX_NESTING_DEPTH: usize = 4000;

/// A parsed JSON document, preserving the written order of fields.
#[derive(Debug, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Int(i64),
    BigInt(String),
    Float(f64),
    Text(String),
    List(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(pairs) => pairs
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Vec<(String, JsonValue)>> {
        match self {
            JsonValue::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            JsonValue::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn object(pairs: Vec<(&str, JsonValue)>) -> JsonValue {
        JsonValue::Object(
            pairs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }
}

/// Why a document did not decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseFailure {
    /// The document is not valid JSON under the ported grammar.
    /// The message carries the decode wording of the Python
    /// runtime, the fault and its position.
    Invalid(String),
    /// The document stands on a boundary the port states, a lone
    /// surrogate escape or nesting past the bound, which the
    /// reading refuses with the bare not valid JSON words.
    Boundary,
    /// One object carries the named key more than once.
    DuplicateKey(String),
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    /// The decode wording for a fault at a byte position, with the
    /// line and column and character position counted in
    /// characters, as the Python decoder counts them.
    fn decode_error(&self, message: &str, byte_pos: usize) -> ParseFailure {
        let prefix = &self.text[..byte_pos];
        let char_pos = prefix.chars().count();
        let line = prefix.matches('\n').count() + 1;
        let column = match prefix.rfind('\n') {
            Some(newline) => char_pos - prefix[..=newline].chars().count() + 1,
            None => char_pos + 1,
        };
        ParseFailure::Invalid(format!(
            "{message}: line {line} column {column} (char {char_pos})"
        ))
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, ParseFailure> {
        self.skip_whitespace();
        let start = self.pos;
        match self.peek() {
            Some(b'{') => self.parse_object(depth),
            Some(b'[') => self.parse_list(depth),
            Some(b'"') => Ok(JsonValue::Text(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true), start),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false), start),
            Some(b'n') => self.parse_literal("null", JsonValue::Null, start),
            Some(b'N') => self.parse_literal("NaN", JsonValue::Float(f64::NAN), start),
            Some(b'I') => self.parse_literal("Infinity", JsonValue::Float(f64::INFINITY), start),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            _ => Err(self.decode_error("Expecting value", start)),
        }
    }

    fn parse_literal(
        &mut self,
        word: &str,
        value: JsonValue,
        start: usize,
    ) -> Result<JsonValue, ParseFailure> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.decode_error("Expecting value", start))
        }
    }

    fn parse_list(&mut self, depth: usize) -> Result<JsonValue, ParseFailure> {
        if depth >= MAX_NESTING_DEPTH {
            return Err(ParseFailure::Boundary);
        }
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonValue::List(items));
        }
        loop {
            let item = self.parse_value(depth + 1)?;
            items.push(item);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonValue::List(items));
                }
                _ => return Err(self.decode_error("Expecting ',' delimiter", self.pos)),
            }
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, ParseFailure> {
        if depth >= MAX_NESTING_DEPTH {
            return Err(ParseFailure::Boundary);
        }
        self.pos += 1;
        let mut pairs: Vec<(String, JsonValue)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(pairs));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.decode_error(
                    "Expecting property name enclosed in double quotes",
                    self.pos,
                ));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(self.decode_error("Expecting ':' delimiter", self.pos));
            }
            self.pos += 1;
            let value = self.parse_value(depth + 1)?;
            pairs.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.decode_error("Expecting ',' delimiter", self.pos)),
            }
        }
        // The duplicate check stands at the close of the object, so
        // an inner object is checked before the object that carries
        // it, and the first key that stands twice in written order
        // is the key named. This matches the pair preserving hook
        // of the Python reading exactly.
        let mut seen: Vec<&str> = Vec::with_capacity(pairs.len());
        for (key, _) in &pairs {
            if seen.contains(&key.as_str()) {
                return Err(ParseFailure::DuplicateKey(key.clone()));
            }
            seen.push(key.as_str());
        }
        Ok(JsonValue::Object(pairs))
    }

    fn parse_string(&mut self) -> Result<String, ParseFailure> {
        // The caller has seen the opening quote.
        let opening = self.pos;
        self.pos += 1;
        let mut out = String::new();
        loop {
            let byte = match self.peek() {
                Some(byte) => byte,
                None => {
                    return Err(self.decode_error("Unterminated string starting at", opening));
                }
            };
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    let backslash = self.pos;
                    self.pos += 1;
                    let escape = match self.peek() {
                        Some(escape) => escape,
                        None => {
                            return Err(
                                self.decode_error("Unterminated string starting at", opening)
                            );
                        }
                    };
                    self.pos += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let unit = self.parse_hex4(backslash + 1)?;
                            if (0xD800..0xDC00).contains(&unit) {
                                // A high surrogate standing alone is
                                // a boundary the port refuses, where
                                // the Python decoder accepts it.
                                if self.peek() == Some(b'\\')
                                    && self.bytes.get(self.pos + 1) == Some(&b'u')
                                {
                                    let second_u = self.pos + 1;
                                    self.pos += 2;
                                    let low = self.parse_hex4(second_u)?;
                                    if !(0xDC00..0xE000).contains(&low) {
                                        return Err(ParseFailure::Boundary);
                                    }
                                    let code = 0x1_0000
                                        + ((u32::from(unit) - 0xD800) << 10)
                                        + (u32::from(low) - 0xDC00);
                                    out.push(char::from_u32(code).ok_or(ParseFailure::Boundary)?);
                                } else {
                                    return Err(ParseFailure::Boundary);
                                }
                            } else if (0xDC00..0xE000).contains(&unit) {
                                return Err(ParseFailure::Boundary);
                            } else {
                                out.push(
                                    char::from_u32(u32::from(unit))
                                        .ok_or(ParseFailure::Boundary)?,
                                );
                            }
                        }
                        _ => {
                            return Err(self.decode_error("Invalid \\escape", backslash));
                        }
                    }
                }
                0x00..=0x1F => {
                    return Err(self.decode_error("Invalid control character at", self.pos));
                }
                _ => {
                    // Copy one whole UTF-8 character. The input is a
                    // string slice, so the bytes are valid UTF-8 and
                    // the cursor stands on a character boundary.
                    let width = if byte < 0x80 {
                        1
                    } else if byte >> 5 == 0b110 {
                        2
                    } else if byte >> 4 == 0b1110 {
                        3
                    } else {
                        4
                    };
                    let text = std::str::from_utf8(&self.bytes[self.pos..self.pos + width])
                        .map_err(|_| ParseFailure::Boundary)?;
                    let ch = text.chars().next().ok_or(ParseFailure::Boundary)?;
                    out.push(ch);
                    self.pos += width;
                }
            }
        }
    }

    /// Read the four hexadecimal digits of a unicode escape. The
    /// position of the letter u stands in the refusal, as the
    /// Python decoder places it.
    fn parse_hex4(&mut self, u_pos: usize) -> Result<u16, ParseFailure> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let byte = match self.peek() {
                Some(byte) => byte,
                None => {
                    return Err(self.decode_error("Invalid \\uXXXX escape", u_pos));
                }
            };
            let digit = match byte {
                b'0'..=b'9' => u16::from(byte - b'0'),
                b'a'..=b'f' => u16::from(byte - b'a') + 10,
                b'A'..=b'F' => u16::from(byte - b'A') + 10,
                _ => return Err(self.decode_error("Invalid \\uXXXX escape", u_pos)),
            };
            value = value * 16 + digit;
            self.pos += 1;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, ParseFailure> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
            if self.bytes[self.pos..].starts_with(b"Infinity") {
                self.pos += "Infinity".len();
                return Ok(JsonValue::Float(f64::NEG_INFINITY));
            }
            if self.bytes[self.pos..].starts_with(b"NaN") {
                self.pos += "NaN".len();
                return Ok(JsonValue::Float(f64::NAN));
            }
        }
        // The integer part stands as a single zero or a nonzero
        // digit run, exactly the shape the runtime pattern matches.
        // A fraction or an exponent stands only when its digits
        // stand, and the token ends in front of a point or an
        // exponent marker whose digits do not stand.
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.decode_error("Expecting value", start)),
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') && matches!(self.bytes.get(self.pos + 1), Some(b'0'..=b'9')) {
            is_float = true;
            self.pos += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            let mut probe = self.pos + 1;
            if matches!(self.bytes.get(probe), Some(b'+') | Some(b'-')) {
                probe += 1;
            }
            if matches!(self.bytes.get(probe), Some(b'0'..=b'9')) {
                is_float = true;
                self.pos = probe;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
        }
        let token = &self.text[start..self.pos];
        if is_float {
            token
                .parse::<f64>()
                .map(JsonValue::Float)
                .map_err(|_| ParseFailure::Boundary)
        } else {
            match token.parse::<i64>() {
                Ok(integer) => Ok(JsonValue::Int(integer)),
                Err(_) => Ok(JsonValue::BigInt(token.to_string())),
            }
        }
    }
}

/// The stack the parse thread carries. The descent is recursive
/// over the nesting bound of 4000 levels, which in a debug build
/// overflows the stack of an ordinary caller thread well before
/// the bound, so the parse runs on its own thread with a stack
/// sized for the whole bound. The reservation is virtual and the
/// pages are only touched to the depth a document reaches.
const PARSE_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Decode a JSON document from text on the calling thread.
fn parse_document(text: &str) -> Result<JsonValue, ParseFailure> {
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        pos: 0,
    };
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(parser.decode_error("Extra data", parser.pos));
    }
    Ok(value)
}

/// Decode a JSON document from text.
pub fn parse_json(text: &str) -> Result<JsonValue, ParseFailure> {
    if text.starts_with('\u{FEFF}') {
        return Err(ParseFailure::Invalid(
            "Unexpected UTF-8 BOM (decode using utf-8-sig): line 1 column 1 (char 0)".to_string(),
        ));
    }
    let owned = text.to_string();
    let spawned = std::thread::Builder::new()
        .stack_size(PARSE_STACK_BYTES)
        .spawn(move || parse_document(&owned));
    match spawned {
        Ok(handle) => match handle.join() {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        },
        // A thread that cannot be spawned leaves the parse on the
        // calling thread, where it stands or falls as it would
        // have stood before the parse carried its own stack.
        Err(_) => parse_document(text),
    }
}
