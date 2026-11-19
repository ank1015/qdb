//! A session: the variables and the current directory that last from one
//! unit to the next. It lives in memory and belongs to the caller; it is
//! never stored in the database. Also here: the arithmetic of `$((...))`,
//! which reads and sets variables.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Var {
    value: Vec<u8>,
    exported: bool,
}

/// The variables and the current directory, and the status of the last
/// command (`$?`). Every session starts the same way: the current
/// directory is `/`, `HOME` and `PWD` are `/`, `OLDPWD` is unset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    vars: HashMap<Vec<u8>, Var>,
    /// Absolute, without `.`, `..` or doubled slashes; `/` for the root.
    cwd: Vec<u8>,
    pub status: i32,
}

impl Default for Session {
    fn default() -> Session {
        Session::new()
    }
}

/// True for a name a variable can have.
pub fn is_name(name: &[u8]) -> bool {
    !name.is_empty() && !name[0].is_ascii_digit() && name.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

impl Session {
    pub fn new() -> Session {
        let mut s = Session { vars: HashMap::new(), cwd: b"/".to_vec(), status: 0 };
        s.set(b"HOME", b"/".to_vec());
        s.set(b"PWD", b"/".to_vec());
        s.set(b"IFS", b" \t\n".to_vec());
        s
    }

    pub fn get(&self, name: &[u8]) -> Option<&[u8]> {
        self.vars.get(name).map(|v| v.value.as_slice())
    }

    pub fn set(&mut self, name: &[u8], value: Vec<u8>) {
        match self.vars.get_mut(name) {
            Some(v) => v.value = value,
            None => {
                self.vars.insert(name.to_vec(), Var { value, exported: false });
            }
        }
    }

    pub fn unset(&mut self, name: &[u8]) {
        self.vars.remove(name);
    }

    /// Mark a variable as exported. One that is not set stays unset, as
    /// in bash, where it is exported once it is given a value; nothing
    /// here reads the mark, because no command reads its environment.
    pub fn export(&mut self, name: &[u8]) {
        if let Some(v) = self.vars.get_mut(name) {
            v.exported = true;
        }
    }

    /// The characters fields are split on: `IFS`, or space, tab and
    /// newline when it is unset.
    pub fn ifs(&self) -> &[u8] {
        self.get(b"IFS").unwrap_or(b" \t\n")
    }

    pub fn cwd(&self) -> &[u8] {
        &self.cwd
    }

    /// Move to `path`, which the caller has found to be a folder. `PWD`
    /// and `OLDPWD` follow.
    pub fn chdir(&mut self, path: &[u8]) {
        let new = absolute(&self.cwd, path);
        let old = self.get(b"PWD").map(|p| p.to_vec()).unwrap_or_else(|| self.cwd.clone());
        self.set(b"OLDPWD", old);
        self.set(b"PWD", new.clone());
        self.cwd = new;
    }
}

/// `path` as an absolute path without `.`, `..` or doubled slashes, taken
/// from the folder `cwd` unless it starts with a slash.
pub fn absolute(cwd: &[u8], path: &[u8]) -> Vec<u8> {
    let mut parts: Vec<&[u8]> = Vec::new();
    let both = [if path.starts_with(b"/") { &b""[..] } else { cwd }, path];
    for comp in both.iter().flat_map(|p| p.split(|c| *c == b'/')) {
        match comp {
            b"" | b"." => {}
            b".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    let mut out = Vec::new();
    for p in parts {
        out.push(b'/');
        out.extend_from_slice(p);
    }
    if out.is_empty() {
        out.push(b'/');
    }
    out
}

// --------------------------------------------------------------- arithmetic

/// Evaluate the inside of `$((...))`, after its own expansions, on 64-bit
/// integers that wrap. The error is bash's message, with the expression
/// and the token it stopped at.
pub fn arith(expr: &[u8], sess: &mut Session) -> Result<i64, String> {
    evaluate(expr, sess, 0)
}

fn evaluate(expr: &[u8], sess: &mut Session, depth: u32) -> Result<i64, String> {
    if depth > 1024 {
        return Err(format!("{}: expression recursion level exceeded (error token is \"{}\")", show(expr), show(expr)));
    }
    let mut p = Arith { s: expr, i: 0, last: 0, tok: Tok::End, was_name: false, sess, name_val: 0, noeval: 0, depth };
    p.next()?;
    if p.tok == Tok::End {
        return Ok(0);
    }
    let v = p.comma()?;
    if p.tok != Tok::End {
        return Err(p.error("syntax error in expression"));
    }
    Ok(v.v)
}

fn show(expr: &[u8]) -> String {
    let start = expr.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(expr.len());
    String::from_utf8_lossy(&expr[start..]).into_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Num(i64),
    Name(Vec<u8>),
    Op(&'static str),
    End,
}

#[derive(Clone)]
struct Val {
    v: i64,
    /// The variable this value was read from, when the operand was
    /// nothing but its name: what an assignment can assign to.
    name: Option<Vec<u8>>,
}

struct Arith<'a> {
    s: &'a [u8],
    i: usize,
    /// Where the last token that was not the end began.
    last: usize,
    tok: Tok,
    /// The token before this one was a name.
    was_name: bool,
    sess: &'a mut Session,
    /// The value of the name that is the token. bash finds it when it
    /// reads the name, before it knows whether the expression is well
    /// formed, so that is when a value that is no expression is reported.
    name_val: i64,
    noeval: u32,
    depth: u32,
}

const OPS: &[&str] = &[
    "<<=", ">>=", "**", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "&=", "^=", "|=", "+", "-", "*", "/", "%", "<", ">", "=", "!", "~", "&", "|", "^",
    "?", ":", ",", "(", ")",
];

const ASSIGN: &[&str] = &["=", "+=", "-=", "*=", "/=", "%=", "<<=", ">>=", "&=", "^=", "|="];

impl Arith<'_> {
    fn error(&self, what: &str) -> String {
        format!("{}: {} (error token is \"{}\")", show(self.s), what, String::from_utf8_lossy(&self.s[self.last.min(self.s.len())..]))
    }

    fn next(&mut self) -> Result<(), String> {
        self.was_name = matches!(self.tok, Tok::Name(_));
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
        if self.i >= self.s.len() {
            self.tok = Tok::End;
            return Ok(());
        }
        self.last = self.i;
        let c = self.s[self.i];
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = self.i;
            while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_') {
                self.i += 1;
            }
            let name = self.s[start..self.i].to_vec();
            let stepped = matches!(self.tok, Tok::Op("++") | Tok::Op("--"));
            self.tok = Tok::Name(name.clone());
            // bash looks at the token after the name before it finds
            // the name's value: what is wrong with that token is
            // reported first, and a name that `=` follows is not
            // evaluated, unless `++` or `--` stands before it.
            let (i, last) = (self.i, self.last);
            self.noeval += 1;
            let peeked = self.next();
            self.noeval -= 1;
            peeked?;
            let assigned = self.is("=");
            (self.i, self.last, self.was_name) = (i, last, false);
            self.tok = Tok::Name(name.clone());
            self.name_val = if assigned && !stepped { 0 } else { self.value(&name)? };
            return Ok(());
        }
        if c.is_ascii_digit() {
            let start = self.i;
            while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || matches!(self.s[self.i], b'#' | b'@' | b'_')) {
                self.i += 1;
            }
            let text = &self.s[start..self.i];
            self.tok = Tok::Num(number(text).map_err(|what| self.error(what))?);
            return Ok(());
        }
        let rest = &self.s[self.i..];
        if rest.starts_with(b"++") || rest.starts_with(b"--") {
            let op = if c == b'+' { "++" } else { "--" };
            if self.was_name {
                self.i += 2;
                self.tok = Tok::Op(if c == b'+' { "post++" } else { "post--" });
                return Ok(());
            }
            let after = rest[2..].iter().find(|c| !c.is_ascii_whitespace());
            if after.is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_') {
                self.i += 2;
                self.tok = Tok::Op(op);
                return Ok(());
            }
            self.i += 1;
            self.tok = Tok::Op(if c == b'+' { "+" } else { "-" });
            return Ok(());
        }
        match OPS.iter().find(|op| rest.starts_with(op.as_bytes())) {
            Some(op) => {
                self.i += op.len();
                self.tok = Tok::Op(op);
                Ok(())
            }
            None => {
                // After an operand bash calls it an operator it does not
                // know; anywhere else, a missing operand.
                let after_operand = matches!(self.tok, Tok::Num(_) | Tok::Name(_) | Tok::Op("post++") | Tok::Op("post--"));
                self.i += 1;
                self.tok = Tok::Op("?bad");
                Err(self.error(if after_operand { "syntax error: invalid arithmetic operator" } else { "syntax error: operand expected" }))
            }
        }
    }

    fn is(&self, op: &str) -> bool {
        matches!(&self.tok, Tok::Op(o) if *o == op)
    }

    fn comma(&mut self) -> Result<Val, String> {
        let mut v = self.assign()?;
        while self.is(",") {
            self.next()?;
            v = self.assign()?;
        }
        Ok(v)
    }

    fn assign(&mut self) -> Result<Val, String> {
        let left = self.cond()?;
        let op = match &self.tok {
            Tok::Op(o) if ASSIGN.contains(o) => *o,
            _ => return Ok(left),
        };
        let Some(name) = left.name.clone() else {
            return Err(self.error("attempted assignment to non-variable"));
        };
        self.next()?;
        let at = self.last;
        let right = self.assign()?;
        let v = if op == "=" { right.v } else { self.binary(&op[..op.len() - 1], left.v, right.v, at)? };
        if self.noeval == 0 {
            self.sess.set(&name, v.to_string().into_bytes());
        }
        Ok(Val { v, name: None })
    }

    fn cond(&mut self) -> Result<Val, String> {
        let c = self.level(0)?;
        if !self.is("?") {
            return Ok(c);
        }
        // The side that is not taken is read with nothing evaluated,
        // its first token included.
        let skip = c.v == 0;
        self.noeval += skip as u32;
        let a = self.next().and_then(|_| if self.tok == Tok::End || self.is(":") { Err(self.error("expression expected")) } else { self.comma() });
        self.noeval -= skip as u32;
        let a = a?;
        if !self.is(":") {
            return Err(self.error("`:' expected for conditional expression"));
        }
        self.noeval += !skip as u32;
        let b = self.next().and_then(|_| if self.tok == Tok::End { Err(self.error("expression expected")) } else { self.cond() });
        self.noeval -= !skip as u32;
        let b = b?;
        Ok(Val { v: if skip { b.v } else { a.v }, name: None })
    }

    /// The binary operators from `||` (level 0) up to `* / %` (level 9).
    fn level(&mut self, n: usize) -> Result<Val, String> {
        const LEVELS: [&[&str]; 10] = [&["||"], &["&&"], &["|"], &["^"], &["&"], &["==", "!="], &["<=", ">=", "<", ">"], &["<<", ">>"], &["+", "-"], &["*", "/", "%"]];
        if n == LEVELS.len() {
            return self.power();
        }
        let mut left = self.level(n + 1)?;
        loop {
            let op = match &self.tok {
                Tok::Op(o) if LEVELS[n].contains(o) => *o,
                _ => return Ok(left),
            };
            // `||` and `&&` do not evaluate a right side that cannot matter.
            let skip = (op == "||" && left.v != 0) || (op == "&&" && left.v == 0);
            self.noeval += skip as u32;
            let mut at = 0;
            let right = self.next().and_then(|_| {
                at = self.last;
                self.level(n + 1)
            });
            self.noeval -= skip as u32;
            let right = right?;
            let v = match op {
                "||" => (left.v != 0 || right.v != 0) as i64,
                "&&" => (left.v != 0 && right.v != 0) as i64,
                _ => self.binary(op, left.v, right.v, at)?,
            };
            left = Val { v, name: None };
        }
    }

    fn power(&mut self) -> Result<Val, String> {
        let base = self.unary()?;
        if !self.is("**") {
            return Ok(base);
        }
        self.next()?;
        let exp = self.power()?;
        if exp.v < 0 {
            if self.noeval > 0 {
                return Ok(Val { v: 0, name: None });
            }
            return Err(self.error("exponent less than 0"));
        }
        let (mut r, mut b, mut e) = (1i64, base.v, exp.v);
        while e > 0 {
            if e & 1 == 1 {
                r = r.wrapping_mul(b);
            }
            b = b.wrapping_mul(b);
            e >>= 1;
        }
        Ok(Val { v: r, name: None })
    }

    fn unary(&mut self) -> Result<Val, String> {
        let op = match &self.tok {
            Tok::Op(o) if matches!(*o, "!" | "~" | "-" | "+") => *o,
            _ => return self.primary(),
        };
        self.next()?;
        let v = self.unary()?.v;
        Ok(Val {
            v: match op {
                "!" => (v == 0) as i64,
                "~" => !v,
                "-" => v.wrapping_neg(),
                _ => v,
            },
            name: None,
        })
    }

    fn primary(&mut self) -> Result<Val, String> {
        match self.tok.clone() {
            Tok::Num(v) => {
                self.next()?;
                Ok(Val { v, name: None })
            }
            Tok::Name(name) => {
                let v = self.name_val;
                self.next()?;
                if self.is("post++") || self.is("post--") {
                    let d = if self.is("post++") { 1 } else { -1 };
                    if self.noeval == 0 {
                        self.sess.set(&name, v.wrapping_add(d).to_string().into_bytes());
                    }
                    self.next()?;
                    return Ok(Val { v, name: None });
                }
                Ok(Val { v, name: Some(name) })
            }
            Tok::Op("++") | Tok::Op("--") => {
                let d = if self.is("++") { 1 } else { -1 };
                self.next()?;
                let Tok::Name(name) = self.tok.clone() else {
                    return Err(self.error("identifier expected after pre-increment or pre-decrement"));
                };
                let v = self.name_val.wrapping_add(d);
                if self.noeval == 0 {
                    self.sess.set(&name, v.to_string().into_bytes());
                }
                self.next()?;
                Ok(Val { v, name: None })
            }
            Tok::Op("(") => {
                self.next()?;
                let v = self.comma()?;
                if !self.is(")") {
                    return Err(self.error("missing `)'"));
                }
                self.next()?;
                Ok(Val { v: v.v, name: None })
            }
            _ => Err(self.error("syntax error: operand expected")),
        }
    }

    /// The value of a variable: a number, or an expression to evaluate.
    fn value(&mut self, name: &[u8]) -> Result<i64, String> {
        if self.noeval > 0 {
            return Ok(0);
        }
        let text = match self.sess.get(name) {
            None => return Ok(0),
            Some(t) => t.to_vec(),
        };
        let plain = std::str::from_utf8(&text).ok().filter(|t| !t.is_empty() && !t.starts_with('0') && t.bytes().all(|c| c.is_ascii_digit())).and_then(|t| t.parse::<i64>().ok());
        match plain {
            Some(v) => Ok(v),
            None => evaluate(&text, self.sess, self.depth + 1),
        }
    }

    fn binary(&mut self, op: &str, a: i64, b: i64, at: usize) -> Result<i64, String> {
        Ok(match op {
            "+" => a.wrapping_add(b),
            "-" => a.wrapping_sub(b),
            "*" => a.wrapping_mul(b),
            "/" | "%" => {
                if b == 0 {
                    if self.noeval > 0 {
                        return Ok(0);
                    }
                    self.last = at;
                    return Err(self.error("division by 0"));
                }
                if op == "/" {
                    a.wrapping_div(b)
                } else {
                    a.wrapping_rem(b)
                }
            }
            "<<" => a.wrapping_shl(b as u32),
            ">>" => a.wrapping_shr(b as u32),
            "<" => (a < b) as i64,
            ">" => (a > b) as i64,
            "<=" => (a <= b) as i64,
            ">=" => (a >= b) as i64,
            "==" => (a == b) as i64,
            "!=" => (a != b) as i64,
            "&" => a & b,
            "|" => a | b,
            "^" => a ^ b,
            _ => unreachable!("operator {}", op),
        })
    }
}

/// A number as bash writes them: decimal, `0` octal, `0x` hexadecimal, or
/// `base#digits` with a base from 2 to 64.
fn number(text: &[u8]) -> Result<i64, &'static str> {
    let (base, digits): (u32, &[u8]) = if let Some(at) = text.iter().position(|c| *c == b'#') {
        let base = std::str::from_utf8(&text[..at]).ok().and_then(|t| t.parse::<u32>().ok()).filter(|b| (2..=64).contains(b)).ok_or("invalid arithmetic base")?;
        if text[at + 1..].is_empty() {
            return Err("invalid integer constant");
        }
        (base, &text[at + 1..])
    } else if text.len() > 1 && (text.starts_with(b"0x") || text.starts_with(b"0X")) {
        (16, &text[2..])
    } else if text.len() > 1 && text[0] == b'0' {
        (8, &text[1..])
    } else {
        (10, text)
    };
    let mut v: i64 = 0;
    for c in digits {
        let d = match *c {
            b'0'..=b'9' => (*c - b'0') as u32,
            b'a'..=b'z' => (*c - b'a') as u32 + 10,
            b'A'..=b'Z' if base <= 36 => (*c - b'A') as u32 + 10,
            b'A'..=b'Z' => (*c - b'A') as u32 + 36,
            b'@' => 62,
            b'_' => 63,
            _ => return Err("value too great for base"),
        };
        if d >= base {
            return Err("value too great for base");
        }
        v = v.wrapping_mul(base as i64).wrapping_add(d as i64);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(e: &str) -> Result<i64, String> {
        arith(e.as_bytes(), &mut Session::new())
    }

    #[test]
    fn operators() {
        for (e, v) in [
            ("1+2*3", 7),
            ("(1+2)*3", 9),
            ("2**10", 1024),
            ("-2**2", 4),
            ("7%3", 1),
            ("-7/2", -3),
            ("1?2:3", 2),
            ("0?2:3", 3),
            ("x=4,x*2", 8),
            ("~5", -6),
            ("1<<62", 4611686018427387904),
            ("16#ff", 255),
            ("2#101", 5),
            ("0x1F", 31),
            ("017", 15),
            ("36#z", 35),
            ("64#_", 63),
            ("1 ? 2 : 1/0", 2),
            ("0 && 1/0", 0),
            ("1 || 1/0", 1),
            ("!5", 0),
            ("9223372036854775807+1", i64::MIN),
            ("-9223372036854775808/-1", i64::MIN),
            ("5>3", 1),
            ("5<=3", 0),
            ("6&3", 2),
            ("6|3", 7),
            ("6^3", 5),
            ("1!=1", 0),
            ("3 - -1", 4),
            ("3--1", 4),
            ("", 0),
            ("  ", 0),
        ] {
            assert_eq!(eval(e), Ok(v), "{}", e);
        }
    }

    #[test]
    fn variables() {
        let mut s = Session::new();
        s.set(b"X", b"5".to_vec());
        assert_eq!(arith(b"X+=2", &mut s), Ok(7));
        assert_eq!(arith(b"X++ + ++X", &mut s), Ok(16));
        assert_eq!(s.get(b"X"), Some(&b"9"[..]));
        s.set(b"A", b"B".to_vec());
        s.set(b"B", b"3*2".to_vec());
        assert_eq!(arith(b"A+1", &mut s), Ok(7));
        assert_eq!(arith(b"unset_one + 1", &mut s), Ok(1));
        assert_eq!(arith(b"0 && (X=1)", &mut s), Ok(0));
        assert_eq!(s.get(b"X"), Some(&b"9"[..]));
    }

    #[test]
    fn errors_are_bashs() {
        for (e, m) in [
            ("1/0", "1/0: division by 0 (error token is \"0\")"),
            ("1/0+5", "1/0+5: division by 0 (error token is \"0+5\")"),
            (" 3 + ", "3 + : syntax error: operand expected (error token is \"+ \")"),
            ("3 4", "3 4: syntax error in expression (error token is \"4\")"),
            ("1=2", "1=2: attempted assignment to non-variable (error token is \"=2\")"),
            ("2**-1", "2**-1: exponent less than 0 (error token is \"1\")"),
            ("08", "08: value too great for base (error token is \"08\")"),
        ] {
            assert_eq!(eval(e), Err(m.to_string()), "{}", e);
        }
    }

    #[test]
    fn paths() {
        assert_eq!(absolute(b"/", b"a/b"), b"/a/b");
        assert_eq!(absolute(b"/a/b", b"../c/./d//"), b"/a/c/d");
        assert_eq!(absolute(b"/a", b"/x"), b"/x");
        assert_eq!(absolute(b"/a", b"../../.."), b"/");
    }
}
