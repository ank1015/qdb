//! JSON: a value type, a parser and a writer. Enough for the API: objects,
//! arrays, strings, numbers (kept as i64 when whole, f64 otherwise),
//! booleans and null.

use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub fn obj() -> Json {
        Json::Obj(BTreeMap::new())
    }

    /// Add a field to an object (and return it, for chaining).
    pub fn with(mut self, key: &str, value: impl Into<Json>) -> Json {
        if let Json::Obj(m) = &mut self {
            m.insert(key.to_string(), value.into());
        }
        self
    }

    pub fn set(&mut self, key: &str, value: impl Into<Json>) {
        if let Json::Obj(m) = self {
            m.insert(key.to_string(), value.into());
        }
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            Json::Float(f) if f.fract() == 0.0 && f.abs() < 9e15 => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_i64().filter(|n| *n >= 0).map(|n| n as u64)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn is_obj(&self) -> bool {
        matches!(self, Json::Obj(_))
    }

    pub fn fields(&self) -> impl Iterator<Item = (&String, &Json)> {
        match self {
            Json::Obj(m) => Some(m.iter()),
            _ => None,
        }
        .into_iter()
        .flatten()
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }

    pub fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                let _ = write!(out, "{}", n);
            }
            Json::Float(f) => {
                if f.is_finite() {
                    let _ = write!(out, "{}", f);
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => quote(s, out),
            Json::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    /// Parse a whole text as one value.
    pub fn parse(text: &[u8]) -> Result<Json, String> {
        let mut p = Parser { s: text, i: 0, depth: 0 };
        p.ws();
        let v = p.value()?;
        p.ws();
        if p.i != text.len() {
            return Err(format!("unexpected text at byte {}", p.i));
        }
        Ok(v)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::Str(s.to_string())
    }
}
impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::Str(s)
    }
}
impl From<i64> for Json {
    fn from(n: i64) -> Json {
        Json::Int(n)
    }
}
impl From<i32> for Json {
    fn from(n: i32) -> Json {
        Json::Int(n as i64)
    }
}
impl From<u64> for Json {
    fn from(n: u64) -> Json {
        Json::Int(n.min(i64::MAX as u64) as i64)
    }
}
impl From<usize> for Json {
    fn from(n: usize) -> Json {
        Json::Int(n as i64)
    }
}
impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}
impl From<Vec<Json>> for Json {
    fn from(a: Vec<Json>) -> Json {
        Json::Arr(a)
    }
}
impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(o: Option<T>) -> Json {
        o.map_or(Json::Null, Into::into)
    }
}

/// Write a string as JSON, with the escapes it needs.
pub fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    depth: u32,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn err<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("{} at byte {}", what, self.i))
    }

    fn eat(&mut self, word: &[u8]) -> bool {
        if self.s[self.i..].starts_with(word) {
            self.i += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        if self.depth > 64 {
            return self.err("nested too deeply");
        }
        match self.s.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'{') => {
                self.i += 1;
                self.depth += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.eat(b"}") {
                    self.depth -= 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    if self.s.get(self.i) != Some(&b'"') {
                        return self.err("expected a string key");
                    }
                    let k = self.string()?;
                    self.ws();
                    if !self.eat(b":") {
                        return self.err("expected :");
                    }
                    self.ws();
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    if self.eat(b",") {
                        continue;
                    }
                    if self.eat(b"}") {
                        self.depth -= 1;
                        return Ok(Json::Obj(m));
                    }
                    return self.err("expected , or }");
                }
            }
            Some(b'[') => {
                self.i += 1;
                self.depth += 1;
                let mut a = Vec::new();
                self.ws();
                if self.eat(b"]") {
                    self.depth -= 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    self.ws();
                    a.push(self.value()?);
                    self.ws();
                    if self.eat(b",") {
                        continue;
                    }
                    if self.eat(b"]") {
                        self.depth -= 1;
                        return Ok(Json::Arr(a));
                    }
                    return self.err("expected , or ]");
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') if self.eat(b"true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat(b"false") => Ok(Json::Bool(false)),
            Some(b'n') if self.eat(b"null") => Ok(Json::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            _ => self.err("unexpected character"),
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.s[self.i] == b'-' {
            self.i += 1;
        }
        let mut float = false;
        while self.i < self.s.len() {
            match self.s[self.i] {
                b'0'..=b'9' => {}
                b'.' | b'e' | b'E' | b'+' | b'-' => float = true,
                _ => break,
            }
            self.i += 1;
        }
        let t = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        if !float {
            if let Ok(n) = t.parse::<i64>() {
                return Ok(Json::Int(n));
            }
        }
        t.parse::<f64>().map(Json::Float).or_else(|_| self.err("bad number"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.s.len() {
            return self.err("short \\u escape");
        }
        let t = std::str::from_utf8(&self.s[self.i..self.i + 4]).map_err(|_| "bad \\u escape".to_string())?;
        let v = u32::from_str_radix(t, 16).map_err(|_| format!("bad \\u escape at byte {}", self.i))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.i) else { return self.err("unterminated string") };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else { return self.err("unterminated string") };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut v = self.hex4()?;
                            if (0xd800..0xdc00).contains(&v) {
                                if !self.eat(b"\\u") {
                                    return self.err("lone surrogate");
                                }
                                let lo = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&lo) {
                                    return self.err("bad surrogate pair");
                                }
                                v = 0x10000 + ((v - 0xd800) << 10) + (lo - 0xdc00);
                            } else if (0xdc00..0xe000).contains(&v) {
                                return self.err("lone surrogate");
                            }
                            let ch = char::from_u32(v).ok_or_else(|| format!("bad character at byte {}", self.i))?;
                            let mut b = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                        }
                        _ => return self.err("bad escape"),
                    }
                }
                c if c < 0x20 => return self.err("control character in string"),
                c => out.push(c),
            }
        }
        String::from_utf8(out).or_else(|_| self.err("string is not UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = r#"{"a":[1,2.5,"x\"y\u00e9\n",true,null,{}],"b":-3,"c":"\ud83d\ude00"}"#;
        let v = Json::parse(t.as_bytes()).unwrap();
        assert_eq!(v.get("b").unwrap().as_i64(), Some(-3));
        assert_eq!(v.get("c").unwrap().as_str(), Some("😀"));
        let again = Json::parse(v.to_string().as_bytes()).unwrap();
        assert_eq!(v, again);
        for bad in ["", "{", "[1,]", "{\"a\" 1}", "\"\\x\"", "01x", "\"\\ud800\""] {
            assert!(Json::parse(bad.as_bytes()).is_err(), "{}", bad);
        }
    }
}
