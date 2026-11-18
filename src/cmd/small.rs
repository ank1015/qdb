//! The small commands: printf, test and [, true, false, :, sleep, seq,
//! basename, dirname, and ln, which is refused.

use super::{msg, usage};
use crate::engine::{Io, Runner};
use crate::tree::Db;
use crate::util::{lossy, quote};

// ------------------------------------------------------------------ printf

/// A number written the way C's strtol reads it, and how much of the
/// text that was. A leading quote gives the code of the next byte.
fn parse_int(a: &[u8]) -> (i128, bool) {
    if let [b'\'' | b'"', c, ..] = a {
        return (*c as i128, true);
    }
    let mut i = 0;
    while i < a.len() && a[i].is_ascii_whitespace() {
        i += 1;
    }
    let neg = a.get(i) == Some(&b'-');
    if matches!(a.get(i), Some(b'-' | b'+')) {
        i += 1;
    }
    let (radix, from) = if a[i..].starts_with(b"0x") || a[i..].starts_with(b"0X") {
        (16, i + 2)
    } else if a.get(i) == Some(&b'0') {
        (8, i)
    } else {
        (10, i)
    };
    let mut v: i128 = 0;
    let mut j = from;
    while let Some(d) = a.get(j).and_then(|c| char::from(*c).to_digit(radix)) {
        v = v.saturating_mul(radix as i128).saturating_add(d as i128).min(i128::MAX / 32);
        j += 1;
    }
    let whole = j == a.len() && (j > from || a.is_empty() || radix == 8);
    (if neg { -v } else { v }, whole || a.is_empty())
}

fn parse_float(a: &[u8]) -> (f64, bool) {
    if let [b'\'' | b'"', c, ..] = a {
        return (*c as f64, true);
    }
    let t = lossy(a);
    let t = t.trim_start();
    if t.is_empty() {
        return (0.0, true);
    }
    let low = t.to_ascii_lowercase();
    let body = low.trim_start_matches(['+', '-']);
    let sign = if low.starts_with('-') { -1.0 } else { 1.0 };
    if body == "inf" || body == "infinity" {
        return (sign * f64::INFINITY, true);
    }
    if body == "nan" {
        return (f64::NAN, true);
    }
    if body.starts_with("0x") {
        let (v, ok) = parse_int(a);
        return (v as f64, ok);
    }
    // The longest prefix that is a number.
    let b = t.as_bytes();
    let mut end = 0;
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let digits = |i: &mut usize| {
        let from = *i;
        while b.get(*i).is_some_and(|c| c.is_ascii_digit()) {
            *i += 1;
        }
        *i - from
    };
    let mut n = digits(&mut i);
    if b.get(i) == Some(&b'.') {
        i += 1;
        n += digits(&mut i);
    }
    if n > 0 {
        end = i;
        if matches!(b.get(i), Some(b'e' | b'E')) {
            let mut k = i + 1;
            if matches!(b.get(k), Some(b'+' | b'-')) {
                k += 1;
            }
            if digits(&mut k) > 0 {
                end = k;
            }
        }
    }
    let v = t[..end].trim_start_matches('+').parse::<f64>().unwrap_or(0.0);
    (v, end == t.len())
}

/// The digits of a float in one of C's styles, without sign.
fn float_digits(v: f64, conv: u8, prec: Option<usize>, alt: bool) -> String {
    let upper = conv.is_ascii_uppercase();
    if v.is_nan() || v.is_infinite() {
        let s = if v.is_nan() { "nan" } else { "inf" };
        return if upper { s.to_uppercase() } else { s.to_string() };
    }
    let a = v.abs();
    let e_style = |p: usize| -> (String, i32) {
        let s = format!("{:.*e}", p, a);
        let (m, e) = s.split_once('e').unwrap();
        (m.to_string(), e.parse().unwrap())
    };
    let with_exp = |m: String, e: i32| format!("{}{}{}{:02}", m, if upper { 'E' } else { 'e' }, if e < 0 { '-' } else { '+' }, e.abs());
    match conv.to_ascii_lowercase() {
        b'f' => {
            let p = prec.unwrap_or(6);
            let mut s = format!("{:.*}", p, a);
            if alt && p == 0 {
                s.push('.');
            }
            s
        }
        b'e' => {
            let p = prec.unwrap_or(6);
            let (mut m, e) = e_style(p);
            if alt && p == 0 {
                m.push('.');
            }
            with_exp(m, e)
        }
        _ => {
            let p = prec.unwrap_or(6).max(1);
            let (m, e) = e_style(p - 1);
            let strip = |s: String| -> String {
                if alt {
                    if s.contains('.') { s } else { s + "." }
                } else if !s.contains('.') {
                    s
                } else {
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            };
            if e >= -4 && (e as i64) < p as i64 {
                strip(format!("{:.*}", (p as i64 - 1 - e as i64) as usize, a))
            } else {
                with_exp(strip(m), e)
            }
        }
    }
}

/// Backslash escapes of the format (`in_arg` false) or of a %b argument.
/// Returns false when `\c` says to stop.
fn unescape(s: &[u8], in_arg: bool, out: &mut Vec<u8>) -> bool {
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        i += 1;
        if c != b'\\' || i >= s.len() {
            out.push(c);
            continue;
        }
        let n = s[i];
        i += 1;
        let number = |i: &mut usize, radix: u32, max: usize| -> Option<u32> {
            let from = *i;
            let mut v = 0u32;
            while *i - from < max {
                match s.get(*i).and_then(|c| char::from(*c).to_digit(radix)) {
                    Some(d) => v = v * radix + d,
                    None => break,
                }
                *i += 1;
            }
            (*i > from).then_some(v)
        };
        match n {
            b'a' => out.push(7),
            b'b' => out.push(8),
            b'e' | b'E' => out.push(27),
            b'f' => out.push(12),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(11),
            b'\\' => out.push(b'\\'),
            b'\'' | b'"' | b'?' if !in_arg => out.push(n),
            b'c' if in_arg => return false,
            b'0'..=b'7' => {
                i -= 1;
                // In an argument a leading 0 is not one of the three digits.
                if in_arg && n == b'0' {
                    i += 1;
                    out.push(number(&mut i, 8, 3).unwrap_or(0) as u8);
                } else {
                    out.push(number(&mut i, 8, 3).unwrap_or(0) as u8);
                }
            }
            b'x' => match number(&mut i, 16, 2) {
                Some(v) => out.push(v as u8),
                None => out.extend_from_slice(b"\\x"),
            },
            b'u' | b'U' => match number(&mut i, 16, if n == b'u' { 4 } else { 8 }) {
                Some(v) => {
                    let ch = char::from_u32(v).unwrap_or('\u{fffd}');
                    out.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
                }
                None => out.extend_from_slice(&[b'\\', n]),
            },
            _ => out.extend_from_slice(&[b'\\', n]),
        }
    }
    true
}

fn pad(out: &mut Vec<u8>, prefix: &str, body: &[u8], width: usize, left: bool, zero: bool) {
    let len = prefix.len() + body.len();
    let fill = width.saturating_sub(len);
    if left {
        out.extend_from_slice(prefix.as_bytes());
        out.extend_from_slice(body);
        out.resize(out.len() + fill, b' ');
    } else if zero {
        out.extend_from_slice(prefix.as_bytes());
        out.resize(out.len() + fill, b'0');
        out.extend_from_slice(body);
    } else {
        out.resize(out.len() + fill, b' ');
        out.extend_from_slice(prefix.as_bytes());
        out.extend_from_slice(body);
    }
}

/// Formats printf does not have here, found before anything runs.
fn printf_check(fmt: &[u8]) -> Result<(), String> {
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] != b'%' {
            i += 1;
            continue;
        }
        i += 1;
        while i < fmt.len() && (b"-+ #0123456789.*".contains(&fmt[i]) || b"hlLjtzq".contains(&fmt[i]) && fmt[i] != b'q') {
            i += 1;
        }
        match fmt.get(i) {
            Some(b'q' | b'Q' | b'n' | b'(' | b'a' | b'A') => return Err(format!("flag: printf %{}", char::from(fmt[i]))),
            _ => i += 1,
        }
    }
    Ok(())
}

pub fn printf(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let mut args = args;
    if args.first().is_some_and(|a| a == b"--") {
        args.remove(0);
    } else if let Some(a) = args.first().filter(|a| a.len() > 1 && a[0] == b'-') {
        if a == b"-v" {
            return Err("flag: printf -v".into());
        }
        let bad = a[..2].to_vec();
        return Ok(Box::new(move |_, io| {
            io.errors_first();
            io.err_line(msg(&[b"qdb: printf: ", &bad, b": invalid option"]));
            io.err_line("printf: usage: printf [-v var] format [arguments]");
            2
        }));
    }
    if args.is_empty() {
        return Ok(Box::new(|_, io| {
            io.errors_first();
            io.err_line("printf: usage: printf [-v var] format [arguments]");
            2
        }));
    }
    printf_check(&args[0])?;
    Ok(Box::new(move |_, io| {
        io.errors_first();
        let fmt = &args[0];
        let mut rest = args[1..].iter();
        let mut status = 0;
        let mut out = Vec::new();
        'again: loop {
            let before = rest.len();
            let mut i = 0;
            while i < fmt.len() {
                // Literal text up to the next directive.
                let stop = fmt[i..].iter().position(|c| *c == b'%').map_or(fmt.len(), |p| i + p);
                if stop > i {
                    if !unescape(&fmt[i..stop], false, &mut out) {
                        break 'again;
                    }
                    i = stop;
                    continue;
                }
                let start = i;
                i += 1;
                if fmt.get(i) == Some(&b'%') {
                    out.push(b'%');
                    i += 1;
                    continue;
                }
                let (mut left, mut plus, mut space, mut alt, mut zero) = (false, false, false, false, false);
                while let Some(c) = fmt.get(i) {
                    match c {
                        b'-' => left = true,
                        b'+' => plus = true,
                        b' ' => space = true,
                        b'#' => alt = true,
                        b'0' => zero = true,
                        _ => break,
                    }
                    i += 1;
                }
                let star = |rest: &mut std::slice::Iter<Vec<u8>>, status: &mut i32, io: &mut Io| -> i64 {
                    match rest.next() {
                        None => 0,
                        Some(a) => {
                            let (v, ok) = parse_int(a);
                            if !ok {
                                io.err_line(msg(&[b"qdb: printf: ", a, b": invalid number"]));
                                *status = 1;
                            }
                            v.clamp(i32::MIN as i128, i32::MAX as i128) as i64
                        }
                    }
                };
                let mut width = 0usize;
                if fmt.get(i) == Some(&b'*') {
                    i += 1;
                    let w = star(&mut rest, &mut status, io);
                    left |= w < 0;
                    width = w.unsigned_abs() as usize;
                } else {
                    while let Some(d) = fmt.get(i).filter(|c| c.is_ascii_digit()) {
                        width = width.saturating_mul(10).saturating_add((d - b'0') as usize);
                        i += 1;
                    }
                }
                let mut prec = None;
                if fmt.get(i) == Some(&b'.') {
                    i += 1;
                    if fmt.get(i) == Some(&b'*') {
                        i += 1;
                        let p = star(&mut rest, &mut status, io);
                        prec = (p >= 0).then_some(p as usize);
                    } else {
                        let mut p = 0usize;
                        while let Some(d) = fmt.get(i).filter(|c| c.is_ascii_digit()) {
                            p = p.saturating_mul(10).saturating_add((d - b'0') as usize);
                            i += 1;
                        }
                        prec = Some(p);
                    }
                }
                let width = width.min(1 << 20);
                let prec = prec.map(|p| p.min(1 << 16));
                while fmt.get(i).is_some_and(|c| b"hlLjtz".contains(c)) {
                    i += 1;
                }
                let Some(&conv) = fmt.get(i) else {
                    io.err_line(msg(&[b"qdb: printf: `", &fmt[start..], b"': missing format character"]));
                    status = 1;
                    break 'again;
                };
                i += 1;
                let arg = match conv {
                    b's' | b'b' | b'c' | b'd' | b'i' | b'u' | b'x' | b'X' | b'o' | b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => rest.next(),
                    _ => {
                        io.err_line(msg(&[b"qdb: printf: `", &[conv], b"': invalid format character"]));
                        status = 1;
                        break 'again;
                    }
                };
                let empty = Vec::new();
                let text = arg.unwrap_or(&empty);
                let invalid = |io: &mut Io, status: &mut i32| {
                    io.err_line(msg(&[b"qdb: printf: ", text, b": invalid number"]));
                    *status = 1;
                };
                match conv {
                    b's' => {
                        let shown = &text[..prec.map_or(text.len(), |p| p.min(text.len()))];
                        pad(&mut out, "", shown, width, left, false);
                    }
                    b'b' => {
                        let mut shown = Vec::new();
                        let more = unescape(text, true, &mut shown);
                        if let Some(p) = prec {
                            shown.truncate(p);
                        }
                        pad(&mut out, "", &shown, width, left, false);
                        if !more {
                            break 'again;
                        }
                    }
                    b'c' => pad(&mut out, "", &text[..text.len().min(1)].iter().copied().chain(text.is_empty().then_some(0)).collect::<Vec<u8>>(), width, left, false),
                    b'd' | b'i' => {
                        let (v, ok) = parse_int(text);
                        if !ok {
                            invalid(io, &mut status);
                        }
                        let clamped = v.clamp(i64::MIN as i128, i64::MAX as i128);
                        if clamped != v {
                            io.err_line(msg(&[b"qdb: printf: warning: ", text, b": Numerical result out of range"]));
                        }
                        let mut digits = clamped.unsigned_abs().to_string();
                        if let Some(p) = prec {
                            if p == 0 && clamped == 0 {
                                digits.clear();
                            }
                            while digits.len() < p {
                                digits.insert(0, '0');
                            }
                        }
                        let sign = if clamped < 0 {
                            "-"
                        } else if plus {
                            "+"
                        } else if space {
                            " "
                        } else {
                            ""
                        };
                        pad(&mut out, sign, digits.as_bytes(), width, left, zero && prec.is_none());
                    }
                    b'u' | b'x' | b'X' | b'o' => {
                        let (v, ok) = parse_int(text);
                        if !ok {
                            invalid(io, &mut status);
                        }
                        let u = if v < 0 {
                            (v.max(i64::MIN as i128) as i64) as u64
                        } else {
                            if v > u64::MAX as i128 {
                                io.err_line(msg(&[b"qdb: printf: warning: ", text, b": Numerical result out of range"]));
                            }
                            v.min(u64::MAX as i128) as u64
                        };
                        let mut digits = match conv {
                            b'u' => u.to_string(),
                            b'x' => format!("{:x}", u),
                            b'X' => format!("{:X}", u),
                            _ => format!("{:o}", u),
                        };
                        if let Some(p) = prec {
                            if p == 0 && u == 0 {
                                digits.clear();
                            }
                            while digits.len() < p {
                                digits.insert(0, '0');
                            }
                        }
                        let mut prefix = "";
                        if alt {
                            match conv {
                                b'x' if u != 0 => prefix = "0x",
                                b'X' if u != 0 => prefix = "0X",
                                b'o' if !digits.starts_with('0') => digits.insert(0, '0'),
                                _ => {}
                            }
                        }
                        pad(&mut out, prefix, digits.as_bytes(), width, left, zero && prec.is_none());
                    }
                    _ => {
                        let (v, ok) = parse_float(text);
                        if !ok {
                            invalid(io, &mut status);
                        }
                        let digits = float_digits(v, conv, prec, alt);
                        let sign = if v.is_sign_negative() && !v.is_nan() {
                            "-"
                        } else if plus {
                            "+"
                        } else if space {
                            " "
                        } else {
                            ""
                        };
                        pad(&mut out, sign, digits.as_bytes(), width, left, zero && v.is_finite());
                    }
                }
            }
            // The format is used again while arguments remain, if it
            // took any.
            if rest.len() == 0 || rest.len() == before {
                break;
            }
        }
        io.put(&out);
        status
    }))
}

// -------------------------------------------------------------- test and [

const UNARY: &[&[u8]] = &[b"-f", b"-d", b"-e", b"-s", b"-z", b"-n"];
const BINARY: &[&[u8]] = &[b"=", b"==", b"!=", b"-eq", b"-ne", b"-lt", b"-le", b"-gt", b"-ge"];

struct Test<'a> {
    db: &'a Db,
    args: &'a [Vec<u8>],
    at: usize,
}

/// What stops a test: a message, with status 2.
type Stop = Vec<u8>;

fn test_int(a: &[u8]) -> Result<i128, Stop> {
    let t = lossy(a);
    let t = t.trim();
    let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return Err(msg(&[a, b": integer expression expected"]));
    }
    Ok(t.strip_prefix('+').unwrap_or(t).parse::<i128>().unwrap_or(if t.starts_with('-') { i128::MIN } else { i128::MAX }))
}

impl<'a> Test<'a> {
    fn unary(&self, op: &[u8], a: &[u8]) -> bool {
        let node = || self.db.resolve(a).ok();
        match op {
            b"-z" => a.is_empty(),
            b"-n" => !a.is_empty(),
            b"-e" => node().is_some(),
            b"-d" => node().is_some_and(|id| self.db.is_dir(id)),
            b"-f" => node().is_some_and(|id| !self.db.is_dir(id)),
            _ => node().is_some_and(|id| self.db.is_dir(id) || self.db.size(id) > 0),
        }
    }

    fn binary(&self, a: &[u8], op: &[u8], b: &[u8]) -> Result<bool, Stop> {
        Ok(match op {
            b"=" | b"==" => a == b,
            b"!=" => a != b,
            _ => {
                let (x, y) = (test_int(a)?, test_int(b)?);
                match op {
                    b"-eq" => x == y,
                    b"-ne" => x != y,
                    b"-lt" => x < y,
                    b"-le" => x <= y,
                    b"-gt" => x > y,
                    _ => x >= y,
                }
            }
        })
    }

    fn one(&self, a: &[u8]) -> bool {
        !a.is_empty()
    }

    fn two(&self, a: &[u8], b: &[u8]) -> Result<bool, Stop> {
        if a == b"!" {
            Ok(!self.one(b))
        } else if UNARY.contains(&a) {
            Ok(self.unary(a, b))
        } else {
            Err(msg(&[a, b": unary operator expected"]))
        }
    }

    fn three(&self, a: &[u8], b: &[u8], c: &[u8]) -> Result<bool, Stop> {
        if BINARY.contains(&b) {
            self.binary(a, b, c)
        } else if b == b"-a" {
            Ok(self.one(a) && self.one(c))
        } else if b == b"-o" {
            Ok(self.one(a) || self.one(c))
        } else if a == b"!" {
            Ok(!self.two(b, c)?)
        } else if a == b"(" && c == b")" {
            Ok(self.one(b))
        } else {
            Err(msg(&[b, b": binary operator expected"]))
        }
    }

    fn peek(&self) -> Option<&'a [u8]> {
        self.args.get(self.at).map(|a| &a[..])
    }

    fn or(&mut self) -> Result<bool, Stop> {
        let mut v = self.and()?;
        while self.peek() == Some(b"-o") {
            self.at += 1;
            let r = self.and()?;
            v = v || r;
        }
        Ok(v)
    }

    fn and(&mut self) -> Result<bool, Stop> {
        let mut v = self.term()?;
        while self.peek() == Some(b"-a") {
            self.at += 1;
            let r = self.term()?;
            v = v && r;
        }
        Ok(v)
    }

    fn term(&mut self) -> Result<bool, Stop> {
        let Some(a) = self.peek() else {
            return Err(b"argument expected".to_vec());
        };
        if a == b"!" && self.at + 1 < self.args.len() {
            self.at += 1;
            return Ok(!self.term()?);
        }
        if a == b"(" && self.at + 1 < self.args.len() {
            self.at += 1;
            let v = self.or()?;
            if self.peek() != Some(b")") {
                return Err(match self.peek() {
                    Some(_) => b"`)' expected".to_vec(),
                    None => b"`)' expected".to_vec(),
                });
            }
            self.at += 1;
            return Ok(v);
        }
        if let (Some(op), Some(b)) = (self.args.get(self.at + 1), self.args.get(self.at + 2)) {
            if BINARY.contains(&&op[..]) {
                self.at += 3;
                return self.binary(a, op, b);
            }
        }
        if UNARY.contains(&a) {
            return match self.args.get(self.at + 1) {
                Some(b) => {
                    self.at += 2;
                    Ok(self.unary(a, b))
                }
                None => Err(b"argument expected".to_vec()),
            };
        }
        self.at += 1;
        Ok(self.one(a))
    }

    fn run(&mut self) -> Result<bool, Stop> {
        let a = self.args;
        match a.len() {
            0 => Ok(false),
            1 => Ok(self.one(&a[0])),
            2 => self.two(&a[0], &a[1]),
            3 => self.three(&a[0], &a[1], &a[2]),
            4 if a[0] == b"!" => Ok(!self.three(&a[1], &a[2], &a[3])?),
            4 if a[0] == b"(" && a[3] == b")" => self.two(&a[1], &a[2]),
            _ => {
                let v = self.or()?;
                if self.at < a.len() {
                    return Err(b"too many arguments".to_vec());
                }
                Ok(v)
            }
        }
    }
}

pub fn test(name: &'static str, args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let mut args = args;
    let unclosed = name == "[" && args.last().is_none_or(|a| a != b"]");
    if name == "[" && !unclosed {
        args.pop();
    }
    // A file test this version does not have is refused where it stands
    // as an operator: before one operand, or before the last of several.
    for (i, a) in args.iter().enumerate() {
        let looks = a.len() == 2 && a[0] == b'-' && a[1].is_ascii_alphabetic() && !UNARY.contains(&&a[..]) && a != b"-a" && a != b"-o";
        let long = a.len() == 3 && a[0] == b'-' && !BINARY.contains(&&a[..]) && matches!(&a[..], b"-nt" | b"-ot" | b"-ef");
        let as_op = i + 2 == args.len() || (i + 1 < args.len() && args.len() > 3);
        if (looks && as_op && (i == 0 || matches!(&args[i - 1][..], b"!" | b"(" | b"-a" | b"-o"))) || (long && i > 0 && i + 1 < args.len()) {
            return Err(format!("flag: {} {}", if name == "[" { "[" } else { "test" }, lossy(a)));
        }
    }
    Ok(Box::new(move |db, io| {
        io.errors_first();
        if unclosed {
            io.err_line("qdb: [: missing `]'");
            return 2;
        }
        match (Test { db, args: &args, at: 0 }).run() {
            Ok(true) => 0,
            Ok(false) => 1,
            Err(stop) => {
                io.err_line(msg(&[b"qdb: ", name.as_bytes(), b": ", &stop]));
                2
            }
        }
    }))
}

// ------------------------------------------------- true, false, :, sleep, ln

pub fn status(code: i32) -> Result<Runner, String> {
    Ok(Box::new(move |_, _| code))
}

pub fn ln(_args: Vec<Vec<u8>>) -> Result<Runner, String> {
    Err("command: ln (the database has no links, hard or symbolic)".into())
}

pub fn sleep(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    if let Some(a) = args.iter().find(|a| a.starts_with(b"--") && a.len() > 2) {
        return Err(format!("flag: sleep {} (long options are not supported)", lossy(a)));
    }
    let args: Vec<Vec<u8>> = args.into_iter().filter(|a| a != b"--").collect();
    Ok(Box::new(move |_, io| {
        if args.is_empty() {
            return usage(io, "sleep", "missing operand");
        }
        let mut bad = false;
        if let Some(a) = args.iter().find(|a| a.len() > 1 && a[0] == b'-') {
            return usage(io, "sleep", &format!("invalid option -- '{}'", char::from(a[1])));
        }
        for a in &args {
            let t = lossy(a);
            let number = t.strip_suffix(['s', 'm', 'h', 'd']).unwrap_or(&t);
            let ok = !number.is_empty()
                && number.bytes().all(|c| c.is_ascii_digit() || c == b'.')
                && number.bytes().filter(|c| *c == b'.').count() <= 1
                && number != ".";
            let ok = ok || matches!(number.to_ascii_lowercase().as_str(), "inf" | "infinity") || {
                let (_, whole) = parse_float(number.as_bytes());
                whole && !number.starts_with('-') && !number.is_empty() && number.bytes().next().is_some_and(|c| c.is_ascii_digit() || c == b'.' || c == b'+')
            };
            if !ok {
                io.err_line(format!("sleep: invalid time interval {}", quote_gnu(a)));
                bad = true;
            }
        }
        if bad {
            io.err_line("Try 'sleep --help' for more information.");
            return 1;
        }
        // The time is not waited out: the database has nothing to wait for.
        0
    }))
}

/// A name in the quotes GNU puts around an argument in an error message.
pub fn quote_gnu(a: &[u8]) -> String {
    quote(a)
}

// ----------------------------------------------------- basename and dirname

fn base_of(path: &[u8]) -> &[u8] {
    if path.is_empty() {
        return path;
    }
    crate::util::basename(path)
}

pub fn basename(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let longs: &[crate::util::LongOpt] = &[("multiple", false, b'a'), ("suffix", true, b's'), ("zero", false, b'z')];
    let p = match super::options_long("basename", &args, b"az", b"s", longs, 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    Ok(Box::new(move |_, io| {
        let end: &[u8] = if p.has(b'z') { b"\0" } else { b"\n" };
        let mut names = &p.operands[..];
        if names.is_empty() {
            return usage(io, "basename", "missing operand");
        }
        let mut suffix: &[u8] = p.arg(b's').unwrap_or(b"");
        if !p.has(b'a') && p.arg(b's').is_none() {
            if names.len() > 2 {
                return usage(io, "basename", &format!("extra operand {}", quote_gnu(&names[2])));
            }
            if names.len() == 2 {
                suffix = &names[1];
                names = &names[..1];
            }
        }
        for n in names {
            let mut b = base_of(n);
            if !suffix.is_empty() && b.len() > suffix.len() && b.ends_with(suffix) {
                b = &b[..b.len() - suffix.len()];
            }
            io.put(b);
            io.put(end);
        }
        0
    }))
}

pub fn dirname(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match super::options_long("dirname", &args, b"z", b"", &[("zero", false, b'z')], 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    Ok(Box::new(move |_, io| {
        if p.operands.is_empty() {
            return usage(io, "dirname", "missing operand");
        }
        for n in &p.operands {
            let mut end = n.len();
            while end > 1 && n[end - 1] == b'/' {
                end -= 1;
            }
            let dir: &[u8] = match n[..end].iter().rposition(|c| *c == b'/') {
                None => b".",
                Some(at) => {
                    let mut d = at;
                    while d > 0 && n[d - 1] == b'/' {
                        d -= 1;
                    }
                    if d == 0 {
                        b"/"
                    } else {
                        &n[..d]
                    }
                }
            };
            io.put(if n.is_empty() { b"." } else { dir });
            io.put(if p.has(b'z') { b"\0" } else { b"\n" });
        }
        0
    }))
}

// --------------------------------------------------------------------- seq

/// A decimal number as seq reads one: its value, the digits after the
/// point and its width as written.
struct SeqNum {
    v: f64,
    int: Option<i128>,
    decimals: usize,
    width: usize,
}

fn seq_num(a: &[u8]) -> Option<SeqNum> {
    let t = lossy(a);
    let t = t.trim_start();
    let body = t.strip_prefix(['-', '+']).unwrap_or(t);
    if body.is_empty() || !body.bytes().all(|c| c.is_ascii_digit() || c == b'.') || body.bytes().filter(|c| *c == b'.').count() > 1 || body == "." {
        // An exponent, inf or nan: a float with no fixed number of decimals.
        let (v, whole) = parse_float(t.as_bytes());
        return (whole && !t.is_empty() && !v.is_nan()).then_some(SeqNum { v, int: None, decimals: usize::MAX, width: 0 });
    }
    let decimals = body.split_once('.').map_or(0, |(_, d)| d.len());
    let v: f64 = t.trim_start_matches('+').parse().ok()?;
    let int = if decimals == 0 { t.trim_start_matches('+').parse::<i128>().ok() } else { None };
    let width = t.trim_start_matches('+').len();
    Some(SeqNum { v, int, decimals, width })
}

fn plain(io: &mut Io, m: String) -> i32 {
    io.err_line(m);
    1
}

pub fn seq(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    // Options stop at the first argument that is a number, negative or not.
    let mut sep: Vec<u8> = b"\n".to_vec();
    let mut equal = false;
    let mut format: Option<Vec<u8>> = None;
    let mut i = 0;
    let mut missing: Option<String> = None;
    while i < args.len() {
        let a = &args[i];
        if a == b"--" {
            i += 1;
            break;
        }
        if a.len() < 2 || a[0] != b'-' || a[1].is_ascii_digit() || a[1] == b'.' {
            break;
        }
        i += 1;
        let (name, attached): (u8, Option<Vec<u8>>) = if a.starts_with(b"--") {
            let (n, v) = match a.iter().position(|c| *c == b'=') {
                Some(eq) => (&a[2..eq], Some(a[eq + 1..].to_vec())),
                None => (&a[2..], None),
            };
            match n {
                b"separator" => (b's', v),
                b"format" => (b'f', v),
                b"equal-width" if v.is_none() => (b'w', None),
                _ => return Err(format!("flag: seq {} (long options other than the ones listed in the README are not supported)", lossy(a))),
            }
        } else {
            let mut name = 0;
            let mut attached = None;
            for (k, c) in a[1..].iter().enumerate() {
                match c {
                    b'w' => equal = true,
                    b's' | b'f' => {
                        name = *c;
                        if k + 2 < a.len() {
                            attached = Some(a[k + 2..].to_vec());
                        }
                        break;
                    }
                    _ => return Err(format!("flag: seq -{}", char::from(*c))),
                }
            }
            (name, attached)
        };
        if name == b'w' {
            equal = true;
            continue;
        }
        if name == 0 {
            continue;
        }
        let value = match attached {
            Some(v) => v,
            None if i < args.len() => {
                i += 1;
                args[i - 1].clone()
            }
            None => {
                missing = Some(if a.starts_with(b"--") { format!("option '{}' requires an argument", lossy(a)) } else { format!("option requires an argument -- '{}'", char::from(name)) });
                break;
            }
        };
        if name == b's' {
            sep = value;
        } else {
            format = Some(value);
        }
    }
    let operands: Vec<Vec<u8>> = args[i.min(args.len())..].to_vec();
    if let Some(f) = &format {
        // %a prints the bits of the number in hexadecimal.
        let mut k = 0;
        while k < f.len() {
            if f[k] == b'%' {
                k += 1;
                while f.get(k).is_some_and(|c| b"-+ #0'.".contains(c) || c.is_ascii_digit()) {
                    k += 1;
                }
                if matches!(f.get(k), Some(b'a') | Some(b'A')) {
                    return Err(format!("flag: seq -f with %{} (hexadecimal floating point is not supported)", char::from(f[k])));
                }
            }
            k += 1;
        }
    }
    Ok(Box::new(move |_, io| {
        if let Some(m) = missing {
            return usage(io, "seq", &m);
        }
        if operands.is_empty() {
            return usage(io, "seq", "missing operand");
        }
        if operands.len() > 3 {
            return usage(io, "seq", &format!("extra operand {}", quote_gnu(&operands[3])));
        }
        let mut nums = Vec::new();
        for o in &operands {
            match seq_num(o) {
                Some(n) => nums.push(n),
                None => return usage(io, "seq", &format!("invalid floating point argument: {}", quote_gnu(o))),
            }
        }
        let one = || SeqNum { v: 1.0, int: Some(1), decimals: 0, width: 1 };
        let last = nums.pop().unwrap();
        let (first, step) = match nums.len() {
            0 => (one(), one()),
            1 => (nums.pop().unwrap(), one()),
            _ => {
                let step = nums.pop().unwrap();
                (nums.pop().unwrap(), step)
            }
        };
        if step.v == 0.0 {
            return usage(io, "seq", &format!("invalid Zero increment value: {}", quote_gnu(&operands[1])));
        }
        // A format of the caller's: one %e, %f or %g directive.
        let mut directive = None;
        if let Some(f) = &format {
            let mut k = 0;
            let mut found: Option<(usize, usize, u8)> = None;
            while k < f.len() {
                if f[k] != b'%' {
                    k += 1;
                    continue;
                }
                if f.get(k + 1) == Some(&b'%') {
                    k += 2;
                    continue;
                }
                let start = k;
                k += 1;
                while f.get(k).is_some_and(|c| b"-+ #0'".contains(c)) {
                    k += 1;
                }
                while f.get(k).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
                    k += 1;
                }
                match f.get(k) {
                    None => return plain(io, format!("seq: format {} ends in %", quote_gnu(f))),
                    Some(c) if b"aefgAEFG".contains(c) => {
                        if found.is_some() {
                            return plain(io, format!("seq: format {} has too many % directives", quote_gnu(f)));
                        }
                        found = Some((start, k + 1, *c));
                        k += 1;
                    }
                    Some(c) => return plain(io, format!("seq: format {} has unknown %{} directive", quote_gnu(f), char::from(*c))),
                }
            }
            match found {
                None => return plain(io, format!("seq: format {} has no % directive", quote_gnu(f))),
                Some(d) => directive = Some(d),
            }
            if equal {
                return usage(io, "seq", "format string may not be specified when printing equal width strings");
            }
        }
        let mut out: Vec<u8> = Vec::new();
        let mut first_line = true;
        let flush = |out: &mut Vec<u8>, io: &mut Io| -> bool {
            io.put(out);
            out.clear();
            !io.stopped()
        };
        // Whole numbers with no format of the caller's: counted exactly.
        let minus_zero = first.v == 0.0 && first.v.is_sign_negative();
        if let (Some(a), Some(s), Some(b), None, false) = (first.int, step.int, last.int, &format, minus_zero) {
            let width = if equal { first.width.max(last.width) } else { 0 };
            let mut n = a;
            while (s > 0 && n <= b) || (s < 0 && n >= b) {
                if !first_line {
                    out.extend_from_slice(&sep);
                }
                first_line = false;
                if width > 0 {
                    let digits = n.unsigned_abs().to_string();
                    if n < 0 {
                        out.push(b'-');
                    }
                    out.resize(out.len() + width.saturating_sub(digits.len() + usize::from(n < 0)), b'0');
                    out.extend_from_slice(digits.as_bytes());
                } else {
                    out.extend_from_slice(n.to_string().as_bytes());
                }
                if out.len() >= 1 << 16 && !flush(&mut out, io) {
                    return 0;
                }
                n += s;
            }
            if !first_line {
                out.push(b'\n');
            }
            flush(&mut out, io);
            return 0;
        }
        let fixed = first.decimals != usize::MAX && step.decimals != usize::MAX;
        let decimals = if fixed { first.decimals.max(step.decimals) } else { 0 };
        let width = if equal && fixed && last.decimals != usize::MAX {
            let w = |n: &SeqNum| n.width - if n.decimals > 0 { n.decimals + 1 } else { 0 } + if decimals > 0 { decimals + 1 } else { 0 };
            w(&first).max(w(&last))
        } else {
            0
        };
        let mut k = 0u64;
        loop {
            let x = if k == 0 { first.v } else { first.v + k as f64 * step.v };
            if (step.v > 0.0 && x > last.v) || (step.v < 0.0 && x < last.v) {
                // The sum can overshoot by a rounding error: what prints
                // the same as the limit is still in.
                let same = match (&format, directive) {
                    // With a format of the caller's: what prints as the
                    // limit, and not as the number before it, is in.
                    (Some(f), Some((start, end, conv))) if k > 0 => {
                        let spec = &f[start + 1..end - 1];
                        let flags = spec.iter().take_while(|c| b"-+ #0'".contains(c)).count();
                        let p: Option<usize> = lossy(&spec[flags..]).split_once('.').map(|(_, p)| p.parse().unwrap_or(0));
                        let shown = |v: f64| format!("{}{}", if v.is_sign_negative() { "-" } else { "" }, float_digits(v, conv, p, spec[..flags].contains(&b'#')));
                        let before = first.v + (k - 1) as f64 * step.v;
                        shown(x).parse::<f64>().is_ok_and(|v| v == last.v) && shown(x) != shown(if k == 1 { first.v } else { before })
                    }
                    (Some(_), _) => false,
                    _ => fixed && format!("{:.*}", decimals, x) == format!("{:.*}", decimals, last.v) && last.decimals <= decimals,
                };
                if !same {
                    break;
                }
            }
            if !first_line {
                out.extend_from_slice(&sep);
            }
            first_line = false;
            match (&format, directive) {
                (Some(f), Some((start, end, conv))) => {
                    let mut lit = Vec::new();
                    let spec = &f[start + 1..end - 1];
                    let flags: Vec<u8> = spec.iter().copied().take_while(|c| b"-+ #0'".contains(c)).collect();
                    let rest = lossy(&spec[flags.len()..]).into_owned();
                    let (w, p) = match rest.split_once('.') {
                        Some((w, p)) => (w.parse().unwrap_or(0), Some(p.parse().unwrap_or(0))),
                        None => (rest.parse().unwrap_or(0), None),
                    };
                    let percent = |s: &[u8], lit: &mut Vec<u8>| {
                        let mut j = 0;
                        while j < s.len() {
                            lit.push(s[j]);
                            j += if s[j] == b'%' { 2 } else { 1 };
                        }
                    };
                    percent(&f[..start], &mut lit);
                    let digits = float_digits(x, conv, p, flags.contains(&b'#'));
                    let sign = if x.is_sign_negative() {
                        "-"
                    } else if flags.contains(&b'+') {
                        "+"
                    } else if flags.contains(&b' ') {
                        " "
                    } else {
                        ""
                    };
                    pad(&mut lit, sign, digits.as_bytes(), w, flags.contains(&b'-'), flags.contains(&b'0'));
                    percent(&f[end..], &mut lit);
                    out.extend_from_slice(&lit);
                }
                _ if fixed => {
                    let digits = format!("{:.*}", decimals, x.abs());
                    let neg = x.is_sign_negative() && (k == 0 || digits.bytes().any(|c| c.is_ascii_digit() && c != b'0'));
                    if neg {
                        out.push(b'-');
                    }
                    out.resize(out.len() + width.saturating_sub(digits.len() + usize::from(neg)), b'0');
                    out.extend_from_slice(digits.as_bytes());
                }
                _ => {
                    if x.is_sign_negative() {
                        out.push(b'-');
                    }
                    out.extend_from_slice(float_digits(x, b'g', None, false).as_bytes());
                }
            }
            if out.len() >= 1 << 16 && !flush(&mut out, io) {
                return 0;
            }
            if x.is_infinite() {
                break;
            }
            k += 1;
        }
        if !first_line {
            out.push(b'\n');
        }
        flush(&mut out, io);
        0
    }))
}
