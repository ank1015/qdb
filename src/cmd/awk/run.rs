//! Running an awk program: values, records and fields, and the tree walk.

use std::collections::HashMap;
use std::rc::Rc;

use super::parse::*;
use crate::bytes::memchr;
use crate::cmd::small::{float_digits, pad};
use crate::engine::Io;
use crate::regex::{Flavor, Regex, Syntax};
use crate::scan::Searcher;

/// A string may not grow past this; a loop that doubles one would take
/// all the memory there is otherwise.
const MAX_STRING: usize = 256 << 20;
/// Turns of loops one run may take, so that a loop without an end stops.
pub const MAX_TURNS: u64 = 1_000_000_000;

#[derive(Clone, Debug)]
pub enum Value {
    /// Never assigned: an empty string and zero at once.
    Uninit,
    Num(f64),
    Str(Rc<[u8]>),
    /// Text from the input: a number if it looks like one.
    StrNum(Rc<[u8]>),
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 11 | 12 | b'\r')
}

/// The number a string starts with, and where it ends (0 for no number).
pub fn scan_number(s: &[u8]) -> (f64, usize) {
    let mut i = 0;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    let start = i;
    let neg = match s.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let signed = i > start;
    let d0 = i;
    let mut v: u64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        if i - d0 < 18 {
            v = v * 10 + (s[i] - b'0') as u64;
        }
        i += 1;
    }
    let whole = i - d0;
    let next = s.get(i).copied();
    if whole > 0 && whole <= 15 && next != Some(b'.') && !matches!(next, Some(b'e' | b'E')) {
        let f = v as f64;
        return (if neg { -f } else { f }, i);
    }
    let mut j = i;
    let mut frac = 0;
    if s.get(j) == Some(&b'.') {
        j += 1;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
            frac += 1;
        }
    }
    if whole + frac == 0 {
        if signed && s.len() >= i + 3 {
            let w = [s[i].to_ascii_lowercase(), s[i + 1].to_ascii_lowercase(), s[i + 2].to_ascii_lowercase()];
            if &w == b"inf" {
                return (if neg { f64::NEG_INFINITY } else { f64::INFINITY }, i + 3);
            }
            if &w == b"nan" {
                return (if neg { -f64::NAN } else { f64::NAN }, i + 3);
            }
        }
        return (0.0, 0);
    }
    let mut end = if frac == 0 && s.get(i) == Some(&b'.') { i + 1 } else { j };
    if matches!(s.get(end), Some(b'e' | b'E')) {
        let mut k = end + 1;
        if matches!(s.get(k), Some(b'+' | b'-')) {
            k += 1;
        }
        if k < s.len() && s[k].is_ascii_digit() {
            while k < s.len() && s[k].is_ascii_digit() {
                k += 1;
            }
            end = k;
        }
    }
    let text = std::str::from_utf8(&s[start..end]).unwrap_or("0");
    (text.parse().unwrap_or(0.0), end)
}

/// The number a whole string is, blanks around it aside.
pub fn looks_numeric(s: &[u8]) -> Option<f64> {
    let (v, end) = scan_number(s);
    if end == 0 || !s[end..].iter().all(|c| is_space(*c)) {
        return None;
    }
    Some(v)
}

/// A number as awk prints it: whole numbers in full, the rest by %.6g.
pub fn push_num(x: f64, out: &mut Vec<u8>) {
    if x.is_nan() {
        out.extend_from_slice(if x.is_sign_negative() { b"-nan" } else { b"+nan" });
    } else if x.is_infinite() {
        out.extend_from_slice(if x < 0.0 { b"-inf" } else { b"+inf" });
    } else if x == x.trunc() {
        if x.abs() < 1e16 {
            let n = x as i64;
            if n < 0 {
                out.push(b'-');
            }
            crate::bytes::push_decimal(out, n.unsigned_abs() as usize);
        } else {
            out.extend_from_slice(format!("{:.0}", x).as_bytes());
        }
    } else {
        if x < 0.0 {
            out.push(b'-');
        }
        out.extend_from_slice(float_digits(x.abs(), b'g', Some(6), false).as_bytes());
    }
}

impl Value {
    pub fn num(&self) -> f64 {
        match self {
            Value::Uninit => 0.0,
            Value::Num(n) => *n,
            Value::Str(s) | Value::StrNum(s) => scan_number(s).0,
        }
    }

    pub fn push(&self, out: &mut Vec<u8>) {
        match self {
            Value::Uninit => {}
            Value::Num(n) => push_num(*n, out),
            Value::Str(s) | Value::StrNum(s) => out.extend_from_slice(s),
        }
    }

    pub fn text(&self) -> Rc<[u8]> {
        match self {
            Value::Str(s) | Value::StrNum(s) => s.clone(),
            other => {
                let mut b = Vec::new();
                other.push(&mut b);
                Rc::from(b)
            }
        }
    }

    /// The number this value compares as, if it compares as one.
    fn numeric(&self) -> Option<f64> {
        match self {
            Value::Uninit => Some(0.0),
            Value::Num(n) => Some(*n),
            Value::Str(_) => None,
            Value::StrNum(s) => looks_numeric(s),
        }
    }

    fn truthy(&self) -> bool {
        match self {
            Value::Uninit => false,
            Value::Num(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::StrNum(s) => match looks_numeric(s) {
                Some(n) => n != 0.0,
                None => !s.is_empty(),
            },
        }
    }
}

fn flag(b: bool) -> Value {
    Value::Num(if b { 1.0 } else { 0.0 })
}

// ------------------------------------------------------------------ regex

fn is_meta(c: u8) -> bool {
    b".[]()*+?{}|^$\\".contains(&c)
}

fn push_literal(out: &mut Vec<u8>, c: u8) {
    if is_meta(c) {
        out.push(b'\\');
    }
    out.push(c);
}

/// One escape after a backslash at `i`: the byte it stands for and where
/// it ends, for the escapes that stand for a byte.
pub fn byte_escape(src: &[u8], i: usize) -> Option<(u8, usize)> {
    let e = *src.get(i)?;
    Some(match e {
        b'a' => (7, i + 1),
        b'b' => (8, i + 1),
        b'f' => (12, i + 1),
        b'n' => (b'\n', i + 1),
        b'r' => (b'\r', i + 1),
        b't' => (b'\t', i + 1),
        b'v' => (11, i + 1),
        b'/' | b'"' | b'&' => (e, i + 1),
        b'0'..=b'7' => {
            let mut v = 0u32;
            let mut j = i;
            while j < src.len() && j < i + 3 && (b'0'..=b'7').contains(&src[j]) {
                v = v * 8 + (src[j] - b'0') as u32;
                j += 1;
            }
            (v as u8, j)
        }
        b'x' if src.get(i + 1).is_some_and(|c| c.is_ascii_hexdigit()) => {
            let mut v = 0u32;
            let mut j = i + 1;
            while j < src.len() && j < i + 3 && src[j].is_ascii_hexdigit() {
                v = v * 16 + (src[j] as char).to_digit(16).unwrap();
                j += 1;
            }
            (v as u8, j)
        }
        _ => return None,
    })
}

/// An awk regular expression in the engine's extended syntax. gawk reads
/// backslash escapes everywhere, inside brackets too; the engine's
/// brackets take a backslash for itself, so a set is written out again.
/// The second result is the escapes gawk warns about.
pub fn translate(src: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut out = Vec::with_capacity(src.len() + 8);
    let mut unknown = Vec::new();
    let mut i = 0;
    while i < src.len() {
        let c = src[i];
        if c == b'\\' {
            let e = match src.get(i + 1) {
                None => {
                    out.push(b'\\');
                    break;
                }
                Some(e) => *e,
            };
            if let Some((b, end)) = byte_escape(src, i + 1) {
                push_literal(&mut out, b);
                i = end;
                continue;
            }
            i += 2;
            match e {
                b'y' => out.extend_from_slice(b"\\b"),
                b'B' | b'<' | b'>' | b'w' | b'W' | b's' | b'S' | b'`' | b'\'' => out.extend_from_slice(&[b'\\', e]),
                _ if is_meta(e) => out.extend_from_slice(&[b'\\', e]),
                _ => {
                    if !e.is_ascii_punctuation() {
                        unknown.push(e);
                    }
                    push_literal(&mut out, e);
                }
            }
            continue;
        }
        if c != b'[' {
            out.push(c);
            i += 1;
            continue;
        }
        // A bracket expression: gather what is in it, then write it in
        // the order POSIX needs (`]` first, `-` last, `^` not first).
        let mut j = i + 1;
        let negated = src.get(j) == Some(&b'^');
        if negated {
            j += 1;
        }
        let mut singles: Vec<u8> = Vec::new();
        let mut rest: Vec<u8> = Vec::new();
        let mut first = true;
        let mut closed = false;
        while j < src.len() {
            let c = src[j];
            if c == b']' && !first {
                closed = true;
                j += 1;
                break;
            }
            first = false;
            if c == b'[' && matches!(src.get(j + 1), Some(b':' | b'.' | b'=')) {
                let kind = src[j + 1];
                if let Some(k) = (j + 2..src.len().saturating_sub(1)).find(|&k| src[k] == kind && src[k + 1] == b']') {
                    rest.extend_from_slice(&src[j..k + 2]);
                    j = k + 2;
                    continue;
                }
            }
            // One member, perhaps the start of a range.
            let member = |at: usize| -> (u8, usize) {
                if src[at] == b'\\' && at + 1 < src.len() {
                    match byte_escape(src, at + 1) {
                        Some((b, end)) => (b, end),
                        None => (src[at + 1], at + 2),
                    }
                } else {
                    (src[at], at + 1)
                }
            };
            let (lo, after) = member(j);
            if src.get(after) == Some(&b'-') && src.get(after + 1).is_some_and(|c| *c != b']') {
                let (hi, end) = member(after + 1);
                if lo == b']' || lo == b'^' || lo == b'-' || hi == b']' || hi == b'-' || lo == b'[' {
                    // Ends POSIX has no place for: spell the range out.
                    for b in lo..=hi {
                        singles.push(b);
                    }
                } else {
                    rest.extend_from_slice(&[lo, b'-', hi]);
                }
                j = end;
            } else {
                singles.push(lo);
                j = after;
            }
        }
        if !closed {
            // Not a set after all: the engine gives the message.
            out.extend_from_slice(&src[i..]);
            break;
        }
        let has = |b: u8| singles.contains(&b);
        let others: Vec<u8> = singles.iter().copied().filter(|b| !matches!(*b, b']' | b'^' | b'-' | b'[')).collect();
        if !negated && rest.is_empty() && others.is_empty() && has(b'^') && !has(b']') && !has(b'-') && !has(b'[') {
            out.extend_from_slice(b"\\^");
        } else {
            let open = out.len();
            out.push(b'[');
            if negated {
                out.push(b'^');
            }
            let members = out.len();
            if has(b']') {
                out.push(b']');
            }
            out.extend_from_slice(&others);
            out.extend_from_slice(&rest);
            if has(b'[') {
                out.push(b'[');
            }
            let mut dash = has(b'-');
            if has(b'^') {
                if out.len() == members && !negated {
                    // `[^` would negate; only a `-` can be with it here.
                    out.extend_from_slice(b"-^");
                    dash = false;
                } else {
                    out.push(b'^');
                }
            }
            if dash {
                out.push(b'-');
            }
            out.push(b']');
            let _ = open;
        }
        i = j;
    }
    (out, unknown)
}

pub fn compile(src: &[u8]) -> Result<Regex, String> {
    let (text, _) = translate(src);
    Regex::new(&text, Syntax { extended: true, icase: false, flavor: Flavor::Awk })
}

// ----------------------------------------------------------------- arrays

#[derive(Default)]
struct Array {
    map: HashMap<Rc<[u8]>, usize>,
    /// In the order the keys were first set. A deleted element stays as
    /// a gap (`gone`) until there are many, so that deleting is quick.
    items: Vec<(Rc<[u8]>, Value)>,
    gone: Vec<bool>,
    dead: usize,
}

impl Array {
    fn slot(&mut self, key: &[u8]) -> usize {
        if let Some(i) = self.map.get(key) {
            return *i;
        }
        let k: Rc<[u8]> = Rc::from(key);
        self.items.push((k.clone(), Value::Uninit));
        self.gone.push(false);
        self.map.insert(k, self.items.len() - 1);
        self.items.len() - 1
    }

    fn clear(&mut self) {
        self.map.clear();
        self.items.clear();
        self.gone.clear();
        self.dead = 0;
    }

    fn len(&self) -> usize {
        self.items.len() - self.dead
    }

    fn keys(&self) -> Vec<Rc<[u8]>> {
        self.items.iter().zip(&self.gone).filter(|(_, g)| !**g).map(|((k, _), _)| k.clone()).collect()
    }

    fn remove(&mut self, key: &[u8]) {
        let Some(i) = self.map.remove(key) else { return };
        self.gone[i] = true;
        self.items[i].1 = Value::Uninit;
        self.dead += 1;
        if self.dead > 64 && self.dead * 2 > self.items.len() {
            let items = std::mem::take(&mut self.items);
            let gone = std::mem::take(&mut self.gone);
            self.clear();
            for ((k, v), g) in items.into_iter().zip(gone) {
                if !g {
                    self.map.insert(k.clone(), self.items.len());
                    self.items.push((k, v));
                    self.gone.push(false);
                }
            }
        }
    }
}

// ----------------------------------------------------------------- fields

#[derive(Clone)]
enum Sep {
    /// Runs of blanks, none at the ends.
    Blank,
    Byte(u8),
    Re(Rc<Regex>),
    /// Every character is a field.
    Chars,
}

fn is_blank(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n'
}

fn split_all(text: &[u8], sep: &Sep, out: &mut Vec<(usize, usize)>) {
    if text.is_empty() {
        return;
    }
    match sep {
        Sep::Blank => {
            let mut i = 0;
            loop {
                while i < text.len() && is_blank(text[i]) {
                    i += 1;
                }
                if i >= text.len() {
                    break;
                }
                let s = i;
                while i < text.len() && !is_blank(text[i]) {
                    i += 1;
                }
                out.push((s, i));
            }
        }
        Sep::Byte(c) => {
            let mut i = 0;
            while let Some(k) = memchr(*c, &text[i..]) {
                out.push((i, i + k));
                i += k + 1;
            }
            out.push((i, text.len()));
        }
        Sep::Re(re) => {
            let mut start = 0;
            let mut pos = 0;
            while pos <= text.len() {
                match re.find_span(text, pos).unwrap_or(None) {
                    Some((s, e)) if e > s => {
                        out.push((start, s));
                        start = e;
                        pos = e;
                    }
                    Some((s, _)) => pos = s + 1,
                    None => break,
                }
            }
            out.push((start, text.len()));
        }
        Sep::Chars => {
            for i in 0..text.len() {
                out.push((i, i + 1));
            }
        }
    }
}

enum Place {
    Var(usize),
    Nf,
    Field(usize),
    Elem(usize, usize),
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Flow {
    Normal,
    Next,
    Break,
    Continue,
    /// A fatal error, `exit`, or nobody reads the output any more.
    Stop,
}

pub struct Rt {
    prog: Rc<Program>,
    res: Rc<Vec<Regex>>,
    vars: Vec<Value>,
    arrays: Vec<Array>,
    rec: Vec<u8>,
    fields: Vec<(usize, usize)>,
    /// Where the splitting of the record has come to.
    scan: usize,
    split_done: bool,
    /// Fields that were assigned a string (1), which then is no number,
    /// or a value never set (2), which stays one.
    text_fields: Vec<u8>,
    /// The record was assigned a string.
    rec_text: u8,
    /// Some field or the record holds a value never set.
    any_unset: bool,
    fs: Sep,
    ofs: Vec<u8>,
    ors: Vec<u8>,
    ranges: Vec<bool>,
    pub out: Vec<u8>,
    pub fatal: Option<Vec<u8>>,
    pub stopped: bool,
    /// `exit` has run: no more input is read, and the END rules run
    /// unless it was in one of them.
    pub exiting: bool,
    /// The status `exit` gave, kept by an `exit` without one.
    pub exit_code: i32,
    /// For each name the program uses as an array: whether it has been
    /// used as one yet, and whether `length` met it before that, which in
    /// gawk makes it a scalar for good.
    typed: Vec<bool>,
    scalar: Vec<bool>,
    dynamic: HashMap<Vec<u8>, Rc<Regex>>,
    /// For `/re/` asked of the whole record: the search grep uses.
    searchers: Vec<Searcher>,
    turns: u64,
    line: u32,
    /// Something of the program has run: messages then name its line.
    pub ran: bool,
}

impl Rt {
    pub fn new(prog: Rc<Program>, res: Rc<Vec<Regex>>) -> Rt {
        let n = prog.names.len();
        let mut vars = vec![Value::Uninit; n];
        vars[NR] = Value::Num(0.0);
        vars[FNR] = Value::Num(0.0);
        vars[FILENAME] = Value::Str(Rc::from(&b""[..]));
        vars[FS] = Value::Str(Rc::from(&b" "[..]));
        vars[OFS] = Value::Str(Rc::from(&b" "[..]));
        vars[ORS] = Value::Str(Rc::from(&b"\n"[..]));
        vars[RSTART] = Value::Num(0.0);
        vars[RLENGTH] = Value::Num(-1.0);
        let ranges = vec![false; prog.ranges];
        let searchers = res.iter().map(|r| Searcher::regex(vec![r.clone()])).collect();
        Rt {
            prog,
            res,
            vars,
            arrays: (0..n).map(|_| Array::default()).collect(),
            rec: Vec::new(),
            fields: Vec::new(),
            scan: 0,
            split_done: true,
            text_fields: Vec::new(),
            rec_text: 0,
            any_unset: false,
            fs: Sep::Blank,
            ofs: b" ".to_vec(),
            ors: b"\n".to_vec(),
            ranges,
            out: Vec::new(),
            fatal: None,
            stopped: false,
            exiting: false,
            exit_code: 0,
            typed: vec![false; n],
            scalar: vec![false; n],
            dynamic: HashMap::new(),
            searchers,
            turns: 0,
            line: 1,
            ran: false,
        }
    }

    pub fn dead(&self) -> bool {
        self.stopped || self.fatal.is_some() || self.exiting
    }

    pub fn fail(&mut self, msg: &str) {
        if self.fatal.is_some() {
            return;
        }
        let mut m = format!("awk: cmd. line:{}: ", self.line).into_bytes();
        if self.vars[FNR].num() > 0.0 {
            m.extend_from_slice(b"(FILENAME=");
            self.vars[FILENAME].push(&mut m);
            m.extend_from_slice(b" FNR=");
            self.vars[FNR].push(&mut m);
            m.extend_from_slice(b") ");
        }
        m.extend_from_slice(b"fatal: ");
        m.extend_from_slice(msg.as_bytes());
        self.fatal = Some(m);
    }

    /// Set a variable from outside the program (`-v`, `-F`, FILENAME).
    pub fn set_var(&mut self, name: &str, v: Value) {
        if let Some(id) = self.prog.names.iter().position(|n| n == name) {
            self.store(&Place::Var(id), v);
        }
    }

    pub fn file_starts(&mut self, name: &[u8]) {
        self.vars[FILENAME] = Value::Str(Rc::from(name));
        self.vars[FNR] = Value::Num(0.0);
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    // ---------------------------------------------------- record and fields

    fn sep_of(&mut self, text: &[u8]) -> Sep {
        if text.is_empty() {
            Sep::Chars
        } else if text == b" " {
            Sep::Blank
        } else if text.len() == 1 {
            Sep::Byte(text[0])
        } else {
            match self.dynamic_re(text) {
                Some(re) => Sep::Re(re),
                None => Sep::Blank,
            }
        }
    }

    fn set_record(&mut self, text: &[u8]) {
        self.rec.clear();
        self.rec.extend_from_slice(text);
        self.fields.clear();
        self.text_fields.clear();
        self.rec_text = 0;
        self.any_unset = false;
        self.scan = 0;
        self.split_done = text.is_empty();
    }

    /// Split on until there are `want` fields or the record ends.
    fn more_fields(&mut self, want: usize) {
        let r = &self.rec;
        match &self.fs {
            Sep::Blank => {
                let mut i = self.scan;
                while self.fields.len() < want {
                    while i < r.len() && is_blank(r[i]) {
                        i += 1;
                    }
                    if i >= r.len() {
                        self.split_done = true;
                        break;
                    }
                    let s = i;
                    while i < r.len() && !is_blank(r[i]) {
                        i += 1;
                    }
                    self.fields.push((s, i));
                }
                self.scan = i;
            }
            Sep::Byte(c) => {
                let mut i = self.scan;
                while self.fields.len() < want {
                    match memchr(*c, &r[i..]) {
                        Some(k) => {
                            self.fields.push((i, i + k));
                            i += k + 1;
                        }
                        None => {
                            self.fields.push((i, r.len()));
                            self.split_done = true;
                            break;
                        }
                    }
                }
                self.scan = i;
            }
            sep => {
                split_all(r, sep, &mut self.fields);
                self.split_done = true;
            }
        }
    }

    fn nf(&mut self) -> usize {
        if !self.split_done {
            self.more_fields(usize::MAX);
        }
        self.fields.len()
    }

    /// The bytes of field `i`, counted from 1; empty past the last.
    fn field(&mut self, i: usize) -> (usize, usize) {
        if i == 0 {
            return (0, self.rec.len());
        }
        if self.fields.len() < i && !self.split_done {
            self.more_fields(i);
        }
        self.fields.get(i - 1).copied().unwrap_or((0, 0))
    }

    fn kind(&self, i: usize) -> u8 {
        if i == 0 {
            self.rec_text
        } else {
            self.text_fields.get(i - 1).copied().unwrap_or(0)
        }
    }

    fn field_value(&mut self, i: usize) -> Value {
        let (s, e) = self.field(i);
        let text: Rc<[u8]> = Rc::from(&self.rec[s..e]);
        match self.kind(i) {
            0 => Value::StrNum(text),
            1 => Value::Str(text),
            _ => Value::Uninit,
        }
    }

    fn field_index(&mut self, e: &Expr) -> Option<usize> {
        let n = self.num(e);
        if n < 0.0 {
            self.fail(&format!("attempt to access field {}", n.trunc() as i64));
            return None;
        }
        Some(n as usize)
    }

    /// Put the record together again from `count` fields, field `set`
    /// being new.
    fn rebuild(&mut self, count: usize, set: Option<(usize, &[u8])>) {
        let have = self.nf();
        let mut rec = Vec::with_capacity(self.rec.len() + 16);
        let mut fields = Vec::with_capacity(count);
        for j in 1..=count {
            if j > 1 {
                rec.extend_from_slice(&self.ofs);
            }
            let s = rec.len();
            match set {
                Some((i, text)) if i == j => rec.extend_from_slice(text),
                _ if j <= have => {
                    let (a, b) = self.fields[j - 1];
                    rec.extend_from_slice(&self.rec[a..b]);
                }
                _ => {}
            }
            fields.push((s, rec.len()));
        }
        self.rec = rec;
        self.fields = fields;
        self.split_done = true;
        self.text_fields.resize(count, 0);
        // A record put together from its fields is a string, as in gawk.
        self.rec_text = 1;
    }

    // ------------------------------------------------------------ regexes

    fn dynamic_re(&mut self, text: &[u8]) -> Option<Rc<Regex>> {
        if let Some(re) = self.dynamic.get(text) {
            return Some(re.clone());
        }
        match compile(text) {
            Ok(re) => {
                if self.dynamic.len() > 500 {
                    self.dynamic.clear();
                }
                let re = Rc::new(re);
                self.dynamic.insert(text.to_vec(), re.clone());
                Some(re)
            }
            Err(m) => {
                let mut t = format!("invalid regexp: {}: /", m);
                t.push_str(&String::from_utf8_lossy(text));
                t.push('/');
                self.fail(&t);
                None
            }
        }
    }

    /// Where `re` first matches in `text` from `from` on.
    fn find(&mut self, re: &Re, text: &[u8], from: usize) -> Option<(usize, usize)> {
        match re {
            Re::Fixed(i) => {
                let res = Rc::clone(&self.res);
                res[*i].find_span(text, from).unwrap_or(None)
            }
            Re::Value(e) => {
                let pattern = self.eval(e).text();
                let re = self.dynamic_re(&pattern)?;
                re.find_span(text, from).unwrap_or(None)
            }
        }
    }

    /// The matcher itself, for uses that search many times.
    fn matcher(&mut self, re: &Re) -> Option<Rc<Regex>> {
        match re {
            Re::Fixed(i) => {
                // The fixed ones live in one shared list; hand out a view.
                let pattern = self.prog.regexes[*i].0.clone();
                self.dynamic_re(&pattern)
            }
            Re::Value(e) => {
                let pattern = self.eval(e).text();
                self.dynamic_re(&pattern)
            }
        }
    }

    // -------------------------------------------------------- expressions

    /// About to use a name as an array. False, with gawk's fatal error,
    /// if `length` has made it a scalar.
    fn as_array(&mut self, a: usize) -> bool {
        if self.scalar[a] {
            let msg = format!("attempt to use scalar `{}' as an array", self.prog.names[a]);
            self.fail(&msg);
            return false;
        }
        self.typed[a] = true;
        true
    }

    fn key(&mut self, e: &Expr) -> Rc<[u8]> {
        match e {
            Expr::Str(s) => s.clone(),
            _ => self.eval(e).text(),
        }
    }

    fn place(&mut self, lv: &Lv) -> Option<Place> {
        Some(match lv {
            Lv::Var(i) => Place::Var(*i),
            Lv::Nf => Place::Nf,
            Lv::Field(e) => Place::Field(self.field_index(e)?),
            Lv::Elem(a, k) => {
                let key = self.key(k);
                if !self.as_array(*a) {
                    return None;
                }
                Place::Elem(*a, self.arrays[*a].slot(&key))
            }
        })
    }

    fn load(&mut self, p: &Place) -> Value {
        match p {
            Place::Var(i) => self.vars[*i].clone(),
            Place::Nf => Value::Num(self.nf() as f64),
            Place::Field(i) => self.field_value(*i),
            Place::Elem(a, i) => self.arrays[*a].items[*i].1.clone(),
        }
    }

    fn store(&mut self, p: &Place, v: Value) {
        match p {
            Place::Var(i) => {
                if *i == FS {
                    // The record in hand was read under the old separator.
                    self.nf();
                    let text = v.text();
                    self.fs = self.sep_of(&text);
                } else if *i == OFS {
                    self.ofs = v.text().to_vec();
                } else if *i == ORS {
                    self.ors = v.text().to_vec();
                }
                self.vars[*i] = v;
            }
            Place::Nf => {
                let n = v.num();
                if n < 0.0 {
                    self.fail("NF set to negative value");
                    return;
                }
                self.rebuild(n as usize, None);
            }
            Place::Field(0) => {
                let text = v.text();
                self.set_record(&text);
                self.rec_text = Self::kind_of(&v);
                self.any_unset = self.rec_text == 2;
            }
            Place::Field(i) => {
                let text = v.text();
                let count = self.nf().max(*i);
                self.rebuild(count, Some((*i, &text)));
                self.text_fields[*i - 1] = Self::kind_of(&v);
                self.any_unset |= self.text_fields[*i - 1] == 2;
            }
            Place::Elem(a, i) => self.arrays[*a].items[*i].1 = v,
        }
    }

    fn arith(&mut self, op: Arith, x: f64, y: f64, line: u32, assign: bool) -> f64 {
        match op {
            Arith::Add => x + y,
            Arith::Sub => x - y,
            Arith::Mul => x * y,
            Arith::Div => {
                if y == 0.0 {
                    self.line = line;
                    self.fail(if assign { "division by zero attempted in `/='" } else { "division by zero attempted" });
                    return 0.0;
                }
                x / y
            }
            Arith::Mod => {
                if y == 0.0 {
                    self.line = line;
                    self.fail(if assign { "division by zero attempted in `%='" } else { "division by zero attempted in `%'" });
                    return 0.0;
                }
                x % y
            }
            Arith::Pow => {
                // As gawk: a whole exponent by repeated squaring.
                if y == y.trunc() && y.abs() < 9e18 {
                    let n = y as i64;
                    if n == 0 {
                        return 1.0;
                    }
                    let mut k = n.unsigned_abs();
                    let mut base = x;
                    let mut mult = 1.0;
                    while k > 1 {
                        if k & 1 == 1 {
                            mult *= base;
                        }
                        base *= base;
                        k /= 2;
                    }
                    let r = mult * base;
                    if n > 0 { r } else { 1.0 / r }
                } else {
                    x.powf(y)
                }
            }
        }
    }

    pub fn num(&mut self, e: &Expr) -> f64 {
        match e {
            Expr::Num(n) => *n,
            Expr::Var(i) => self.vars[*i].num(),
            Expr::FieldN(n) => {
                let (s, e) = self.field(*n);
                scan_number(&self.rec[s..e]).0
            }
            Expr::Nf => self.nf() as f64,
            Expr::Group(x) | Expr::Plus(x) => self.num(x),
            Expr::Neg(x) => -self.num(x),
            Expr::Arith(op, a, b, line) => {
                let x = self.num(a);
                let y = self.num(b);
                self.arith(*op, x, y, *line, false)
            }
            _ => self.eval(e).num(),
        }
    }

    /// A field or a constant whose value can be looked at where it is.
    fn simple(e: &Expr) -> bool {
        match e {
            Expr::Num(_) | Expr::Str(_) | Expr::FieldN(_) => true,
            Expr::Field(i) => matches!(**i, Expr::Var(_)),
            _ => false,
        }
    }

    /// Where a simple operand that is a field lies in the record, and
    /// whether it was assigned a string.
    fn kind_of(v: &Value) -> u8 {
        match v {
            Value::Str(_) => 1,
            Value::Uninit => 2,
            _ => 0,
        }
    }

    fn locate(&mut self, e: &Expr) -> Option<(usize, usize, bool)> {
        let i = match e {
            Expr::FieldN(n) => *n,
            Expr::Field(x) => self.field_index(x).unwrap_or(0),
            _ => return None,
        };
        let (a, b) = self.field(i);
        Some((a, b, self.kind(i) == 1))
    }

    /// The text of a simple operand, and the number it compares as.
    fn view<'a>(rec: &'a [u8], e: &'a Expr, at: Option<(usize, usize, bool)>, scratch: &'a mut Vec<u8>) -> (&'a [u8], Option<f64>) {
        match (e, at) {
            (_, Some((a, b, stringy))) => {
                let text = &rec[a..b];
                (text, if stringy { None } else { looks_numeric(text) })
            }
            (Expr::Num(n), _) => {
                push_num(*n, scratch);
                (&scratch[..], Some(*n))
            }
            (Expr::Str(s), _) => (&s[..], None),
            _ => (&[], None),
        }
    }

    fn compare(&mut self, op: Rel, a: &Expr, b: &Expr) -> bool {
        if Self::simple(a) && Self::simple(b) && !self.any_unset {
            let (la, lb) = (self.locate(a), self.locate(b));
            let (mut s1, mut s2) = (Vec::new(), Vec::new());
            let (ta, na) = Self::view(&self.rec, a, la, &mut s1);
            let (tb, nb) = Self::view(&self.rec, b, lb, &mut s2);
            let ord = match (na, nb) {
                (Some(x), Some(y)) => x.partial_cmp(&y),
                _ => Some(ta.cmp(tb)),
            };
            return Self::holds(op, ord);
        }
        let va = self.eval(a);
        let vb = self.eval(b);
        let ord = match (va.numeric(), vb.numeric()) {
            (Some(x), Some(y)) if !matches!((&va, &vb), (Value::Uninit, Value::Str(_)) | (Value::Str(_), Value::Uninit)) => x.partial_cmp(&y),
            _ => {
                let mut x = Vec::new();
                let mut y = Vec::new();
                va.push(&mut x);
                vb.push(&mut y);
                Some(x.cmp(&y))
            }
        };
        Self::holds(op, ord)
    }

    fn holds(op: Rel, ord: Option<std::cmp::Ordering>) -> bool {
        use std::cmp::Ordering;
        match ord {
            None => op == Rel::Ne,
            Some(o) => match op {
                Rel::Lt => o == Ordering::Less,
                Rel::Le => o != Ordering::Greater,
                Rel::Eq => o == Ordering::Equal,
                Rel::Ne => o != Ordering::Equal,
                Rel::Gt => o == Ordering::Greater,
                Rel::Ge => o != Ordering::Less,
            },
        }
    }

    fn matches_record(&mut self, i: usize) -> bool {
        match self.searchers[i].is_match(&self.rec) {
            Ok(hit) => hit,
            Err(_) => self.res[i].find_span(&self.rec, 0).unwrap_or(None).is_some(),
        }
    }

    pub fn truth(&mut self, e: &Expr) -> bool {
        match e {
            Expr::Num(n) => *n != 0.0,
            Expr::Regex(i) => self.matches_record(*i),
            Expr::Group(x) => self.truth(x),
            Expr::Not(x) => !self.truth(x),
            Expr::And(a, b) => self.truth(a) && self.truth(b),
            Expr::Or(a, b) => self.truth(a) || self.truth(b),
            Expr::Cmp(op, a, b) => self.compare(*op, a, b),
            Expr::Match(negated, l, re) => {
                let found = match &**l {
                    Expr::FieldN(n) => {
                        let (s, e) = self.field(*n);
                        let text = std::mem::take(&mut self.rec);
                        let f = self.find(re, &text[s..e], 0);
                        self.rec = text;
                        f
                    }
                    _ => {
                        let text = self.eval(l).text();
                        self.find(re, &text, 0)
                    }
                };
                found.is_some() != *negated
            }
            Expr::In(k, a) => {
                let key = self.key(k);
                self.as_array(*a) && self.arrays[*a].map.contains_key(&key[..])
            }
            _ => self.eval(e).truthy(),
        }
    }

    /// The text of a value, straight into `out`.
    pub fn emit(&mut self, e: &Expr, out: &mut Vec<u8>) {
        match e {
            Expr::Str(s) => out.extend_from_slice(s),
            Expr::Num(n) => push_num(*n, out),
            Expr::FieldN(n) => {
                let (s, e) = self.field(*n);
                out.extend_from_slice(&self.rec[s..e]);
            }
            Expr::Var(i) => self.vars[*i].push(out),
            Expr::Group(x) => self.emit(x, out),
            Expr::Concat(parts) => {
                for p in parts {
                    self.emit(p, out);
                }
            }
            _ => self.eval(e).push(out),
        }
    }

    fn string(&mut self, e: &Expr) -> Vec<u8> {
        let mut b = Vec::new();
        self.emit(e, &mut b);
        b
    }

    pub fn eval(&mut self, e: &Expr) -> Value {
        match e {
            Expr::Num(n) => Value::Num(*n),
            Expr::Str(s) => Value::Str(s.clone()),
            Expr::Var(i) => self.vars[*i].clone(),
            Expr::Nf => Value::Num(self.nf() as f64),
            Expr::FieldN(n) => self.field_value(*n),
            Expr::Field(x) => match self.field_index(x) {
                Some(i) => self.field_value(i),
                None => Value::Uninit,
            },
            Expr::Elem(a, k) => {
                let key = self.key(k);
                if !self.as_array(*a) {
                    return Value::Uninit;
                }
                let i = self.arrays[*a].slot(&key);
                self.arrays[*a].items[i].1.clone()
            }
            Expr::Group(x) => self.eval(x),
            Expr::Assign(lv, r) => {
                let v = self.eval(r);
                if let Some(p) = self.place(lv) {
                    self.store(&p, v.clone());
                }
                v
            }
            Expr::Update(lv, op, r, line) => {
                let p = match self.place(lv) {
                    Some(p) => p,
                    None => return Value::Uninit,
                };
                let y = self.num(r);
                let x = self.load(&p).num();
                let v = Value::Num(self.arith(*op, x, y, *line, true));
                self.store(&p, v.clone());
                v
            }
            Expr::Step(lv, step, old) => {
                let p = match self.place(lv) {
                    Some(p) => p,
                    None => return Value::Uninit,
                };
                let x = self.load(&p).num();
                self.store(&p, Value::Num(x + step));
                Value::Num(if *old { x } else { x + step })
            }
            Expr::Neg(_) | Expr::Plus(_) | Expr::Arith(..) => Value::Num(self.num(e)),
            Expr::Concat(_) => {
                let b = self.string(e);
                if b.len() > MAX_STRING {
                    self.fail("a string grew past 256 MB, the most this awk keeps in one string");
                    return Value::Uninit;
                }
                Value::Str(Rc::from(b))
            }
            Expr::Regex(_) | Expr::Not(_) | Expr::And(..) | Expr::Or(..) | Expr::Cmp(..) | Expr::Match(..) | Expr::In(..) => flag(self.truth(e)),
            Expr::Cond(c, a, b) => {
                if self.truth(c) {
                    self.eval(a)
                } else {
                    self.eval(b)
                }
            }
            Expr::Length(None) => Value::Num(self.rec.len() as f64),
            Expr::Length(Some(x)) => {
                let n = match &**x {
                    Expr::FieldN(n) => {
                        let (s, e) = self.field(*n);
                        e - s
                    }
                    _ => self.string(x).len(),
                };
                Value::Num(n as f64)
            }
            Expr::LengthOf(id) => {
                if self.prog.uses[*id].1 && !self.scalar[*id] {
                    if !self.typed[*id] {
                        // gawk: an untyped name given to length becomes a
                        // scalar.
                        self.scalar[*id] = true;
                        return Value::Num(0.0);
                    }
                    Value::Num(self.arrays[*id].len() as f64)
                } else {
                    Value::Num(self.vars[*id].text().len() as f64)
                }
            }
            Expr::Substr(s, m, n) => {
                let text = self.string(s);
                let m = self.num(m);
                let n = n.as_ref().map(|n| self.num(n));
                let start = if m.is_nan() { return Value::Str(Rc::from(&b""[..])) } else { m.trunc().max(1.0) };
                if start > text.len() as f64 {
                    return Value::Str(Rc::from(&b""[..]));
                }
                let start = start as usize - 1;
                let len = match n {
                    None => text.len() - start,
                    Some(n) if n.is_nan() || n.trunc() <= 0.0 => 0,
                    Some(n) => (n.trunc().min(text.len() as f64) as usize).min(text.len() - start),
                };
                Value::Str(Rc::from(&text[start..start + len]))
            }
            Expr::Index(s, t) => {
                let text = self.string(s);
                let what = self.string(t);
                let at = if what.is_empty() {
                    Some(0)
                } else if what.len() > text.len() {
                    None
                } else {
                    memchr::memmem::find(&text, &what)
                };
                Value::Num(at.map_or(0.0, |i| (i + 1) as f64))
            }
            Expr::Lower(x) => {
                let mut b = self.string(x);
                b.make_ascii_lowercase();
                Value::Str(Rc::from(b))
            }
            Expr::Upper(x) => {
                let mut b = self.string(x);
                b.make_ascii_uppercase();
                Value::Str(Rc::from(b))
            }
            Expr::MatchFn(s, re) => {
                let text = self.string(s);
                let found = self.find(re, &text, 0);
                let (start, len) = match found {
                    Some((a, b)) => ((a + 1) as f64, (b - a) as f64),
                    None => (0.0, -1.0),
                };
                self.vars[RSTART] = Value::Num(start);
                self.vars[RLENGTH] = Value::Num(len);
                Value::Num(start)
            }
            Expr::Split(s, arr, sep) => {
                let text = self.string(s);
                let sep = match sep {
                    None => self.fs.clone(),
                    Some(Re::Fixed(i)) => {
                        let pattern = self.prog.regexes[*i].0.clone();
                        match self.dynamic_re(&pattern) {
                            Some(re) => Sep::Re(re),
                            None => return Value::Uninit,
                        }
                    }
                    Some(Re::Value(e)) => {
                        let t = self.eval(e).text();
                        self.sep_of(&t)
                    }
                };
                let mut parts = Vec::new();
                split_all(&text, &sep, &mut parts);
                if self.scalar[*arr] {
                    self.fail("split: second argument is not an array");
                    return Value::Uninit;
                }
                self.typed[*arr] = true;
                let a = &mut self.arrays[*arr];
                a.clear();
                let mut key = Vec::new();
                for (n, (s, e)) in parts.iter().enumerate() {
                    key.clear();
                    crate::bytes::push_decimal(&mut key, n + 1);
                    let i = a.slot(&key);
                    a.items[i].1 = Value::StrNum(Rc::from(&text[*s..*e]));
                }
                Value::Num(parts.len() as f64)
            }
            Expr::Subst(global, re, with, target) => {
                let re = match self.matcher(re) {
                    Some(re) => re,
                    None => return Value::Uninit,
                };
                let with = self.string(with);
                let place = match target {
                    None => Place::Field(0),
                    Some(lv) => match self.place(lv) {
                        Some(p) => p,
                        None => return Value::Uninit,
                    },
                };
                // Naming a field past the last one as the target makes
                // it exist, as in gawk, whether or not anything changes.
                if let Place::Field(i) = place {
                    if i > self.nf() {
                        self.fields.resize(i, (0, 0));
                        self.text_fields.resize(i, 0);
                    }
                }
                let text = self.load(&place).text();
                let (count, new) = substitute(&re, &text, &with, *global);
                if count > 0 {
                    if new.len() > MAX_STRING {
                        self.fail("a string grew past 256 MB, the most this awk keeps in one string");
                        return Value::Uninit;
                    }
                    self.store(&place, Value::Str(Rc::from(new)));
                }
                Value::Num(count as f64)
            }
            Expr::Sprintf(args, line) => match self.format(args, *line) {
                Some(b) => Value::Str(Rc::from(b)),
                None => Value::Uninit,
            },
        }
    }

    fn format(&mut self, args: &[Expr], line: u32) -> Option<Vec<u8>> {
        let mut vals: Vec<Value> = Vec::with_capacity(args.len());
        for a in args {
            vals.push(self.eval(a));
        }
        if self.fatal.is_some() {
            return None;
        }
        let fmt = vals[0].text();
        let mut out = Vec::new();
        match format_into(&fmt, &vals[1..], &mut out) {
            Ok(()) if out.len() <= MAX_STRING => Some(out),
            Ok(()) => {
                self.fail("a string grew past 256 MB, the most this awk keeps in one string");
                None
            }
            Err(at) => {
                self.line = line;
                let mut m = String::from("not enough arguments to satisfy format string\n\t`");
                m.push_str(&String::from_utf8_lossy(&fmt));
                m.push_str("'\n\t");
                m.push_str(&" ".repeat(at));
                m.push_str("^ ran out for this one");
                self.fail(&m);
                None
            }
        }
    }

    // --------------------------------------------------------- statements

    fn turn(&mut self) -> bool {
        self.turns += 1;
        if self.turns > MAX_TURNS {
            self.fail(&format!("stopped after {} turns of its loops, the most one run of awk may take", MAX_TURNS));
        }
        !self.dead()
    }

    fn flush(&mut self, io: &mut Io) {
        if !self.out.is_empty() {
            io.put(&self.out);
            self.out.clear();
        }
        if io.stopped() {
            self.stopped = true;
        }
    }

    pub fn run(&mut self, body: &[Stmt], io: &mut Io) -> Flow {
        for s in body {
            let f = self.exec(s, io);
            if f != Flow::Normal {
                return f;
            }
        }
        Flow::Normal
    }

    fn exec(&mut self, s: &Stmt, io: &mut Io) -> Flow {
        self.ran = true;
        match s {
            Stmt::Expr(e, line) => {
                self.line = *line;
                match e {
                    // The value is not wanted: skip making it.
                    Expr::Step(Lv::Var(i), step, _) if *i > RLENGTH => {
                        let x = self.vars[*i].num();
                        self.vars[*i] = Value::Num(x + step);
                    }
                    Expr::Update(Lv::Var(i), op, r, l) if *i > RLENGTH => {
                        let y = self.num(r);
                        let x = self.vars[*i].num();
                        self.vars[*i] = Value::Num(self.arith(*op, x, y, *l, true));
                    }
                    _ => {
                        self.eval(e);
                    }
                }
            }
            Stmt::Print(list, line) => {
                self.line = *line;
                let mut out = std::mem::take(&mut self.out);
                let before = out.len();
                if list.is_empty() {
                    out.extend_from_slice(&self.rec);
                } else {
                    for (i, e) in list.iter().enumerate() {
                        if i > 0 {
                            out.extend_from_slice(&self.ofs);
                        }
                        self.emit(e, &mut out);
                    }
                }
                out.extend_from_slice(&self.ors);
                if self.fatal.is_some() {
                    out.truncate(before);
                }
                self.out = out;
                if self.out.len() > 1 << 15 {
                    self.flush(io);
                }
            }
            Stmt::Printf(args, line) => {
                self.line = *line;
                if args.is_empty() {
                    self.fail("printf: no arguments");
                    return Flow::Stop;
                }
                if let Some(b) = self.format(args, *line) {
                    self.out.extend_from_slice(&b);
                    if self.out.len() > 1 << 15 {
                        self.flush(io);
                    }
                }
            }
            Stmt::If(c, then, other, line) => {
                self.line = *line;
                if self.truth(c) {
                    return self.exec(then, io);
                } else if let Some(o) = other {
                    return self.exec(o, io);
                }
            }
            Stmt::While(c, body, line) => {
                self.line = *line;
                while self.truth(c) {
                    if !self.turn() {
                        return Flow::Stop;
                    }
                    match self.exec(body, io) {
                        Flow::Break => break,
                        Flow::Normal | Flow::Continue => {}
                        f => return f,
                    }
                }
            }
            Stmt::For(init, cond, step, body, line) => {
                self.line = *line;
                if let Some(e) = init {
                    self.eval(e);
                }
                loop {
                    if let Some(c) = cond {
                        if !self.truth(c) {
                            break;
                        }
                    }
                    if !self.turn() {
                        return Flow::Stop;
                    }
                    match self.exec(body, io) {
                        Flow::Break => break,
                        Flow::Normal | Flow::Continue => {}
                        f => return f,
                    }
                    if let Some(e) = step {
                        self.eval(e);
                    }
                }
            }
            Stmt::ForIn(var, arr, body) => {
                if !self.as_array(*arr) {
                    return Flow::Stop;
                }
                let keys: Vec<Rc<[u8]>> = self.arrays[*arr].keys();
                for k in keys {
                    if !self.turn() {
                        return Flow::Stop;
                    }
                    if let Some(p) = self.place(var) {
                        self.store(&p, Value::StrNum(k));
                    }
                    match self.exec(body, io) {
                        Flow::Break => break,
                        Flow::Normal | Flow::Continue => {}
                        f => return f,
                    }
                }
            }
            Stmt::Block(body) => return self.run(body, io),
            Stmt::Do(body, c, line) => loop {
                if !self.turn() {
                    return Flow::Stop;
                }
                match self.exec(body, io) {
                    Flow::Break => break,
                    Flow::Normal | Flow::Continue => {}
                    f => return f,
                }
                self.line = *line;
                if !self.truth(c) {
                    break;
                }
            },
            Stmt::Delete(arr, key, line) => {
                self.line = *line;
                if !self.as_array(*arr) {
                    return Flow::Stop;
                }
                match key {
                    None => self.arrays[*arr].clear(),
                    Some(k) => {
                        let key = self.key(k);
                        self.arrays[*arr].remove(&key);
                    }
                }
            }
            Stmt::Exit(status, line) => {
                self.line = *line;
                if let Some(e) = status {
                    let v = self.num(e);
                    // As a process status: the low eight bits.
                    self.exit_code = (if v.is_finite() { v.trunc() as i64 } else { 0 } & 0xff) as i32;
                }
                if self.fatal.is_none() {
                    self.exiting = true;
                }
                return Flow::Stop;
            }
            Stmt::Next => return Flow::Next,
            Stmt::Break => return Flow::Break,
            Stmt::Continue => return Flow::Continue,
        }
        if self.dead() { Flow::Stop } else { Flow::Normal }
    }

    /// One record through every rule. False when the run is over.
    pub fn record(&mut self, line: &[u8], io: &mut Io) -> bool {
        self.set_record(line);
        let nr = self.vars[NR].num() + 1.0;
        self.vars[NR] = Value::Num(nr);
        let fnr = self.vars[FNR].num() + 1.0;
        self.vars[FNR] = Value::Num(fnr);
        let prog = Rc::clone(&self.prog);
        for rule in &prog.rules {
            self.line = rule.line;
            self.ran = true;
            let on = match &rule.pattern {
                Pattern::Always => true,
                Pattern::Expr(e) => self.truth(e),
                Pattern::Range(a, b, slot) => {
                    if self.ranges[*slot] {
                        if self.truth(b) {
                            self.ranges[*slot] = false;
                        }
                        true
                    } else if self.truth(a) {
                        if !self.truth(b) {
                            self.ranges[*slot] = true;
                        }
                        true
                    } else {
                        false
                    }
                }
            };
            if self.fatal.is_some() {
                return false;
            }
            if !on {
                continue;
            }
            match &rule.action {
                None => {
                    self.out.extend_from_slice(&self.rec);
                    self.out.extend_from_slice(&self.ors);
                }
                Some(body) => match self.run(body, io) {
                    Flow::Next => break,
                    Flow::Stop => return false,
                    _ => {}
                },
            }
        }
        if self.out.len() > 1 << 15 {
            self.flush(io);
        }
        !self.dead()
    }

    pub fn finish_output(&mut self, io: &mut Io) {
        self.flush(io);
    }
}

/// `sub` and `gsub`: how many replacements, and the new text.
fn substitute(re: &Regex, text: &[u8], with: &[u8], global: bool) -> (usize, Vec<u8>) {
    let mut out = Vec::with_capacity(text.len() + with.len());
    let mut count = 0;
    let mut copied = 0;
    let mut pos = 0;
    let mut prev_end: Option<usize> = None;
    while pos <= text.len() {
        let (s, e) = match re.find_span(text, pos).unwrap_or(None) {
            None => break,
            Some(m) => m,
        };
        if s == e && prev_end == Some(s) {
            // An empty match right after a match does not count.
            if s >= text.len() {
                break;
            }
            pos = s + 1;
            continue;
        }
        out.extend_from_slice(&text[copied..s]);
        // The replacement: `&` is the match, `\&` an ampersand.
        let mut i = 0;
        while i < with.len() {
            let c = with[i];
            if c == b'\\' {
                if with[i..].starts_with(b"\\\\\\&") {
                    out.extend_from_slice(b"\\&");
                    i += 4;
                    continue;
                }
                if with[i..].starts_with(b"\\\\&") {
                    out.push(b'\\');
                    out.extend_from_slice(&text[s..e]);
                    i += 3;
                    continue;
                }
                if with.get(i + 1) == Some(&b'&') {
                    out.push(b'&');
                    i += 2;
                    continue;
                }
                out.push(c);
            } else if c == b'&' {
                out.extend_from_slice(&text[s..e]);
            } else {
                out.push(c);
            }
            i += 1;
        }
        count += 1;
        copied = e;
        prev_end = Some(e);
        if !global {
            break;
        }
        if e == s {
            if s >= text.len() {
                break;
            }
            pos = s + 1;
        } else {
            pos = e;
        }
    }
    out.extend_from_slice(&text[copied.min(text.len())..]);
    (count, out)
}

/// Infinity and not-a-number, which gawk always prints with a sign.
fn special(v: f64) -> &'static str {
    match (v.is_nan(), v.is_sign_negative()) {
        (true, true) => "-nan",
        (true, false) => "+nan",
        (false, true) => "-inf",
        (false, false) => "+inf",
    }
}

/// printf. The error is the place in the format that had no argument.
fn format_into(fmt: &[u8], args: &[Value], out: &mut Vec<u8>) -> Result<(), usize> {
    let mut ai = 0;
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] != b'%' {
            let next = memchr(b'%', &fmt[i..]).map_or(fmt.len(), |k| i + k);
            out.extend_from_slice(&fmt[i..next]);
            i = next;
            continue;
        }
        let spec = i;
        i += 1;
        if i >= fmt.len() {
            out.push(b'%');
            break;
        }
        if fmt[i] == b'%' {
            out.push(b'%');
            i += 1;
            continue;
        }
        let (mut left, mut plus, mut space, mut alt, mut zero) = (false, false, false, false, false);
        while i < fmt.len() {
            match fmt[i] {
                b'-' => left = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'#' => alt = true,
                b'0' => zero = true,
                _ => break,
            }
            i += 1;
        }
        let number = |i: &mut usize, ai: &mut usize| -> Result<Option<i64>, usize> {
            if fmt.get(*i) == Some(&b'*') {
                let v = args.get(*ai).ok_or(*i)?.num();
                *ai += 1;
                *i += 1;
                return Ok(Some(v as i64));
            }
            let from = *i;
            let mut v: i64 = 0;
            while fmt.get(*i).is_some_and(|c| c.is_ascii_digit()) {
                v = (v * 10 + (fmt[*i] - b'0') as i64).min(1 << 30);
                *i += 1;
            }
            Ok(if *i > from { Some(v) } else { None })
        };
        let mut width = number(&mut i, &mut ai)?.unwrap_or(0);
        if width < 0 {
            left = true;
            width = -width;
        }
        let width = width.min(1 << 30) as usize;
        let mut prec: Option<usize> = None;
        if fmt.get(i) == Some(&b'.') {
            i += 1;
            let p = number(&mut i, &mut ai)?.unwrap_or(0);
            prec = if p < 0 { None } else { Some(p.min(1 << 30) as usize) };
        }
        while matches!(fmt.get(i), Some(b'h' | b'l' | b'L')) {
            i += 1;
        }
        let conv = match fmt.get(i) {
            None => {
                out.extend_from_slice(&fmt[spec..]);
                break;
            }
            Some(c) => *c,
        };
        i += 1;
        if !b"diouxXeEfFgGcs".contains(&conv) {
            out.extend_from_slice(&fmt[spec..i]);
            continue;
        }
        let arg = args.get(ai).ok_or(i - 1)?;
        ai += 1;
        let sign = |neg: bool| if neg { "-" } else if plus { "+" } else if space { " " } else { "" };
        match conv {
            b'd' | b'i' => {
                let v = arg.num();
                if !v.is_finite() {
                    // The conversions written in capitals print these in capitals.
                    if conv.is_ascii_uppercase() {
                        out.extend_from_slice(special(v).to_ascii_uppercase().as_bytes());
                    } else {
                        out.extend_from_slice(special(v).as_bytes());
                    }
                    continue;
                }
                let v = v.trunc();
                let mut body = if v.abs() < 9.2e18 { (v.abs() as u64).to_string() } else { format!("{:.0}", v.abs()) };
                if let Some(p) = prec {
                    if p == 0 && v == 0.0 {
                        body.clear();
                    }
                    while body.len() < p {
                        body.insert(0, '0');
                    }
                }
                pad(out, sign(v < 0.0), body.as_bytes(), width, left, zero && prec.is_none());
            }
            b'o' | b'x' | b'X' | b'u' => {
                let v = arg.num();
                if !v.is_finite() {
                    // The conversions written in capitals print these in capitals.
                    if conv.is_ascii_uppercase() {
                        out.extend_from_slice(special(v).to_ascii_uppercase().as_bytes());
                    } else {
                        out.extend_from_slice(special(v).as_bytes());
                    }
                    continue;
                }
                let v = v.trunc();
                let n: u64 = if v < 0.0 { (v.max(-9.2e18) as i64) as u64 } else { v.min(1.8e19) as u64 };
                let mut body = match conv {
                    b'o' => format!("{:o}", n),
                    b'x' => format!("{:x}", n),
                    b'X' => format!("{:X}", n),
                    _ => n.to_string(),
                };
                if let Some(p) = prec {
                    if p == 0 && n == 0 {
                        body.clear();
                    }
                    while body.len() < p {
                        body.insert(0, '0');
                    }
                }
                let mut prefix = "";
                if alt {
                    if conv == b'o' && !body.starts_with('0') {
                        body.insert(0, '0');
                    } else if conv == b'x' && n != 0 {
                        prefix = "0x";
                    } else if conv == b'X' && n != 0 {
                        prefix = "0X";
                    }
                }
                pad(out, prefix, body.as_bytes(), width, left, zero && prec.is_none());
            }
            b'e' | b'E' | b'f' | b'F' | b'g' | b'G' => {
                let v = arg.num();
                if !v.is_finite() {
                    // The conversions written in capitals print these in capitals.
                    if conv.is_ascii_uppercase() {
                        out.extend_from_slice(special(v).to_ascii_uppercase().as_bytes());
                    } else {
                        out.extend_from_slice(special(v).as_bytes());
                    }
                    continue;
                }
                let body = float_digits(v, conv, prec, alt);
                let neg = v < 0.0 || (v == 0.0 && v.is_sign_negative()) || (v.is_nan() && v.is_sign_negative());
                pad(out, sign(neg), body.as_bytes(), width, left, zero && v.is_finite());
            }
            b'c' => {
                let one: Vec<u8> = match arg.numeric() {
                    Some(n) if !matches!(arg, Value::Uninit) => vec![n as i64 as u8],
                    _ => vec![arg.text().first().copied().unwrap_or(0)],
                };
                pad(out, "", &one, width, left, false);
            }
            _ => {
                let text = arg.text();
                let shown = match prec {
                    Some(p) if p < text.len() => &text[..p],
                    _ => &text[..],
                };
                pad(out, "", shown, width, left, false);
            }
        }
    }
    Ok(())
}
