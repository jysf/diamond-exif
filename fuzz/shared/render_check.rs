// The independent checkers for the report renderers (SPEC-015, DEC-010): a
// strict JSON parser (RFC 8259), a strict CSV parser (RFC 4180) and a naive
// second implementation of `safe_text`. Shared by `fuzz/fuzz_targets/
// audit_render.rs`, `tests/fuzz_layout.rs` and `tests/render_check.rs` with
// `include!`; it is not a crate root, it is text included at module level and
// defines one module, `render_check`. `cargo fmt` does not follow `include!`,
// so this file is formatted by hand (`rustfmt --edition 2021 <file>`).
//
// NOTHING here calls `exif_core::report`: a checker that used the writer's own
// helpers would agree with the writer's bugs. The escape list of DEC-010 part
// 2 is typed again below. The checkers are strict on purpose and are tested
// against a table of valid and invalid documents (`tests/render_check.rs`), so
// a lenient checker is itself shown red.

#[allow(dead_code)]
mod render_check {
    /// A parsed JSON value. Objects keep their key order; duplicate keys are
    /// rejected by the parser, so the order is the only thing a lookup needs.
    #[derive(Clone, Debug, PartialEq)]
    pub enum J {
        Null,
        Bool(bool),
        /// The number's source text (the checker does not interpret it).
        Num(String),
        Str(String),
        Arr(Vec<J>),
        Obj(Vec<(String, J)>),
    }

    impl J {
        pub fn get(&self, key: &str) -> Option<&J> {
            match self {
                J::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
                _ => None,
            }
        }

        pub fn keys(&self) -> Vec<&str> {
            match self {
                J::Obj(kv) => kv.iter().map(|(k, _)| k.as_str()).collect(),
                _ => Vec::new(),
            }
        }

        pub fn as_str(&self) -> Option<&str> {
            match self {
                J::Str(s) => Some(s),
                _ => None,
            }
        }

        pub fn as_arr(&self) -> Option<&Vec<J>> {
            match self {
                J::Arr(a) => Some(a),
                _ => None,
            }
        }

        /// A non-negative integer: digits only, no sign, fraction or exponent.
        pub fn as_uint(&self) -> Option<u64> {
            match self {
                J::Num(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
                    n.parse().ok()
                }
                _ => None,
            }
        }

        pub fn as_bool(&self) -> Option<bool> {
            match self {
                J::Bool(b) => Some(*b),
                _ => None,
            }
        }

        pub fn is_null(&self) -> bool {
            matches!(self, J::Null)
        }
    }

    /// The deepest nesting of arrays and objects the checker accepts.
    pub const JSON_MAX_DEPTH: usize = 32;

    struct P<'a> {
        b: &'a [u8],
        i: usize,
    }

    impl<'a> P<'a> {
        fn peek(&self) -> Option<u8> {
            self.b.get(self.i).copied()
        }

        fn err<T>(&self, what: &str) -> Result<T, String> {
            Err(format!("json: {what} at byte {}", self.i))
        }

        fn ws(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.i += 1;
            }
        }

        fn lit(&mut self, word: &str, v: J) -> Result<J, String> {
            if self.b[self.i..].starts_with(word.as_bytes()) {
                self.i += word.len();
                Ok(v)
            } else {
                self.err("bad literal")
            }
        }

        fn value(&mut self, depth: usize) -> Result<J, String> {
            self.ws();
            match self.peek() {
                None => self.err("unexpected end"),
                Some(b'n') => self.lit("null", J::Null),
                Some(b't') => self.lit("true", J::Bool(true)),
                Some(b'f') => self.lit("false", J::Bool(false)),
                Some(b'"') => Ok(J::Str(self.string()?)),
                Some(b'[') => self.array(depth),
                Some(b'{') => self.object(depth),
                Some(b'-' | b'0'..=b'9') => self.number(),
                Some(_) => self.err("unexpected character"),
            }
        }

        fn number(&mut self) -> Result<J, String> {
            let start = self.i;
            if self.peek() == Some(b'-') {
                self.i += 1;
            }
            match self.peek() {
                Some(b'0') => {
                    self.i += 1;
                    if matches!(self.peek(), Some(b'0'..=b'9')) {
                        return self.err("leading zero");
                    }
                }
                Some(b'1'..=b'9') => {
                    while matches!(self.peek(), Some(b'0'..=b'9')) {
                        self.i += 1;
                    }
                }
                _ => return self.err("digit expected"),
            }
            if self.peek() == Some(b'.') {
                self.i += 1;
                if !matches!(self.peek(), Some(b'0'..=b'9')) {
                    return self.err("digit expected after '.'");
                }
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            if matches!(self.peek(), Some(b'e' | b'E')) {
                self.i += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.i += 1;
                }
                if !matches!(self.peek(), Some(b'0'..=b'9')) {
                    return self.err("digit expected in exponent");
                }
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            let text = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("");
            Ok(J::Num(text.to_string()))
        }

        fn hex4(&mut self) -> Result<u32, String> {
            let mut n = 0u32;
            for _ in 0..4 {
                let d = match self.peek() {
                    Some(c @ b'0'..=b'9') => u32::from(c - b'0'),
                    Some(c @ b'a'..=b'f') => u32::from(c - b'a') + 10,
                    Some(c @ b'A'..=b'F') => u32::from(c - b'A') + 10,
                    _ => return self.err("bad \\u escape"),
                };
                n = n * 16 + d;
                self.i += 1;
            }
            Ok(n)
        }

        fn string(&mut self) -> Result<String, String> {
            // Opening quote.
            self.i += 1;
            let mut buf: Vec<u8> = Vec::new();
            loop {
                let Some(c) = self.peek() else {
                    return self.err("unterminated string");
                };
                match c {
                    b'"' => {
                        self.i += 1;
                        return String::from_utf8(buf).map_err(|_| "json: bad utf-8".to_string());
                    }
                    0x00..=0x1F => return self.err("raw control character in string"),
                    b'\\' => {
                        self.i += 1;
                        let Some(e) = self.peek() else {
                            return self.err("unterminated escape");
                        };
                        self.i += 1;
                        let ch = match e {
                            b'"' => '"',
                            b'\\' => '\\',
                            b'/' => '/',
                            b'b' => '\u{8}',
                            b'f' => '\u{c}',
                            b'n' => '\n',
                            b'r' => '\r',
                            b't' => '\t',
                            b'u' => {
                                let hi = self.hex4()?;
                                if (0xD800..0xDC00).contains(&hi) {
                                    // A high surrogate needs a low one right after.
                                    if self.peek() != Some(b'\\') {
                                        return self.err("lone high surrogate");
                                    }
                                    self.i += 1;
                                    if self.peek() != Some(b'u') {
                                        return self.err("lone high surrogate");
                                    }
                                    self.i += 1;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&lo) {
                                        return self.err("high surrogate without a low one");
                                    }
                                    let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                                    match char::from_u32(cp) {
                                        Some(c) => c,
                                        None => return self.err("bad code point"),
                                    }
                                } else if (0xDC00..0xE000).contains(&hi) {
                                    return self.err("lone low surrogate");
                                } else {
                                    match char::from_u32(hi) {
                                        Some(c) => c,
                                        None => return self.err("bad code point"),
                                    }
                                }
                            }
                            _ => return self.err("bad escape"),
                        };
                        let mut tmp = [0u8; 4];
                        buf.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                    }
                    _ => {
                        buf.push(c);
                        self.i += 1;
                    }
                }
            }
        }

        fn array(&mut self, depth: usize) -> Result<J, String> {
            if depth >= JSON_MAX_DEPTH {
                return self.err("nesting too deep");
            }
            self.i += 1;
            let mut items = Vec::new();
            self.ws();
            if self.peek() == Some(b']') {
                self.i += 1;
                return Ok(J::Arr(items));
            }
            loop {
                items.push(self.value(depth + 1)?);
                self.ws();
                match self.peek() {
                    Some(b',') => {
                        self.i += 1;
                    }
                    Some(b']') => {
                        self.i += 1;
                        return Ok(J::Arr(items));
                    }
                    _ => return self.err("',' or ']' expected"),
                }
            }
        }

        fn object(&mut self, depth: usize) -> Result<J, String> {
            if depth >= JSON_MAX_DEPTH {
                return self.err("nesting too deep");
            }
            self.i += 1;
            let mut kv: Vec<(String, J)> = Vec::new();
            self.ws();
            if self.peek() == Some(b'}') {
                self.i += 1;
                return Ok(J::Obj(kv));
            }
            loop {
                self.ws();
                if self.peek() != Some(b'"') {
                    return self.err("object key expected");
                }
                let k = self.string()?;
                if kv.iter().any(|(e, _)| *e == k) {
                    return self.err("duplicate key");
                }
                self.ws();
                if self.peek() != Some(b':') {
                    return self.err("':' expected");
                }
                self.i += 1;
                let v = self.value(depth + 1)?;
                kv.push((k, v));
                self.ws();
                match self.peek() {
                    Some(b',') => {
                        self.i += 1;
                    }
                    Some(b'}') => {
                        self.i += 1;
                        return Ok(J::Obj(kv));
                    }
                    _ => return self.err("',' or '}' expected"),
                }
            }
        }
    }

    /// Parse one JSON document, strictly (RFC 8259): the input must be valid
    /// UTF-8, there is exactly one value, nesting is at most 32 deep, no
    /// trailing comma, no leading zero, no raw control character in a string,
    /// only valid escapes, well-formed surrogate pairs, no duplicate keys, and
    /// nothing but whitespace after the value.
    pub fn parse_json(bytes: &[u8]) -> Result<J, String> {
        if std::str::from_utf8(bytes).is_err() {
            return Err("json: input is not valid UTF-8".to_string());
        }
        let mut p = P { b: bytes, i: 0 };
        let v = p.value(0)?;
        p.ws();
        if p.i != bytes.len() {
            return p.err("trailing data");
        }
        Ok(v)
    }

    /// Parse a CSV document strictly (RFC 4180): UTF-8, CRLF line ends and a
    /// final CRLF, every row with as many fields as the first, a field quoted
    /// with doubled inner quotes (and free to hold CR and LF), no quote in an
    /// unquoted field, no bare CR or LF.
    pub fn parse_csv(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
        if std::str::from_utf8(bytes).is_err() {
            return Err("csv: input is not valid UTF-8".to_string());
        }
        if bytes.is_empty() {
            return Err("csv: empty document".to_string());
        }
        let b = bytes;
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut row: Vec<String> = Vec::new();
        let mut i = 0usize;
        loop {
            // One field starts at `i`.
            let mut field: Vec<u8> = Vec::new();
            if b.get(i) == Some(&b'"') {
                i += 1;
                loop {
                    match b.get(i) {
                        None => return Err("csv: unterminated quoted field".to_string()),
                        Some(b'"') => {
                            if b.get(i + 1) == Some(&b'"') {
                                field.push(b'"');
                                i += 2;
                            } else {
                                i += 1;
                                break;
                            }
                        }
                        Some(c) => {
                            field.push(*c);
                            i += 1;
                        }
                    }
                }
                match b.get(i) {
                    Some(b',') | Some(b'\r') => {}
                    _ => return Err(format!("csv: junk after a closing quote at byte {i}")),
                }
            } else {
                loop {
                    match b.get(i) {
                        None => return Err("csv: missing final CRLF".to_string()),
                        Some(b'"') => return Err(format!("csv: bare quote at byte {i}")),
                        Some(b'\n') => return Err(format!("csv: bare LF at byte {i}")),
                        Some(b',') | Some(b'\r') => break,
                        Some(c) => {
                            field.push(*c);
                            i += 1;
                        }
                    }
                }
            }
            row.push(String::from_utf8(field).map_err(|_| "csv: bad utf-8".to_string())?);
            match b.get(i) {
                Some(b',') => {
                    i += 1;
                }
                Some(b'\r') => {
                    if b.get(i + 1) != Some(&b'\n') {
                        return Err(format!("csv: bare CR at byte {i}"));
                    }
                    i += 2;
                    rows.push(std::mem::take(&mut row));
                    if i == b.len() {
                        break;
                    }
                }
                _ => return Err("csv: missing final CRLF".to_string()),
            }
        }
        let width = rows[0].len();
        for (n, r) in rows.iter().enumerate() {
            if r.len() != width {
                return Err(format!("csv: row {n} has {} fields, want {width}", r.len()));
            }
        }
        Ok(rows)
    }

    // ── a naive second implementation of `safe_text` ────────────────────────

    /// DEC-010 part 2, typed again and written char by char: the characters
    /// that are escaped as `\u{NNNN}` (besides C1).
    fn escaped_as_u(c: char) -> bool {
        let n = c as u32;
        n == 0x061C
            || n == 0x200E
            || n == 0x200F
            || (0x202A..=0x202E).contains(&n)
            || (0x2066..=0x2069).contains(&n)
            || n == 0x2028
            || n == 0x2029
    }

    /// The expected display-safe text of `bytes`. Decodes one character at a
    /// time (trying one to four bytes), so it does not share the library's
    /// error-length logic: a byte that starts no valid character is `\xNN`.
    pub fn naive_safe_text(bytes: &[u8]) -> String {
        let mut out = String::new();
        let mut i = 0usize;
        while i < bytes.len() {
            let mut decoded: Option<(char, usize)> = None;
            for n in 1..=4usize {
                if i + n > bytes.len() {
                    break;
                }
                if let Ok(s) = std::str::from_utf8(&bytes[i..i + n]) {
                    if let Some(c) = s.chars().next() {
                        decoded = Some((c, n));
                    }
                    break;
                }
            }
            match decoded {
                None => {
                    out.push_str(&format!("\\x{:02x}", bytes[i]));
                    i += 1;
                }
                Some((c, n)) => {
                    i += n;
                    let cp = c as u32;
                    if c == '\\' {
                        out.push_str("\\\\");
                    } else if c == '"' {
                        out.push_str("\\\"");
                    } else if c == '\n' {
                        out.push_str("\\n");
                    } else if c == '\r' {
                        out.push_str("\\r");
                    } else if c == '\t' {
                        out.push_str("\\t");
                    } else if cp < 0x20 || cp == 0x7F {
                        out.push_str(&format!("\\x{cp:02x}"));
                    } else if (0x80..=0x9F).contains(&cp) || escaped_as_u(c) {
                        out.push_str(&format!("\\u{{{cp:04x}}}"));
                    } else {
                        out.push(c);
                    }
                }
            }
        }
        out
    }
}
