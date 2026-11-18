//! Commands that pass text through: cut, tr, uniq, nl and tee. Each
//! takes its input a piece at a time and holds no more than a line.

use super::{options_long, LineBuf};
use crate::engine::{store_output, Filter, Input, Io, Prepared, Runner, Spool};
use crate::tree::{Db, NodeId};
use crate::util::{quote, quote_if_needed, LongOpt, Parsed};

/// A command that fails before it reads anything, with the tool's own
/// message.
pub(super) fn fails(cmd: &'static str, message: String, try_line: bool, code: i32) -> Prepared {
    let run: Runner = Box::new(move |_, io| {
        io.err_line(format!("{}: {}", cmd, message));
        if try_line {
            io.err_line(format!("Try '{} --help' for more information.", cmd));
        }
        code
    });
    run.into()
}

/// How a command names a folder it was asked to read.
type DirMessage = fn(&[u8]) -> String;

/// A command that passes its files in order, or its standard input,
/// through one filter. A file that does not end with a newline is read as
/// if it did.
pub(super) fn over_files(cmd: &'static str, files: Vec<Vec<u8>>, make: impl Fn() -> Box<dyn Filter> + 'static, dir: DirMessage) -> Prepared {
    if files.is_empty() {
        let input = Input::Stream(make());
        let run: Runner = Box::new(move |_, io| {
            let mut f = make();
            let stdin = io.take_stdin();
            f.push(stdin, io);
            f.finish(io)
        });
        return Prepared { run, input };
    }
    let input = if files.iter().any(|f| f == b"-") { Input::Whole } else { Input::Unused };
    let run: Runner = Box::new(move |db, io| {
        let mut f = make();
        let mut status = 0;
        let mut more = true;
        for name in &files {
            if !more {
                break;
            }
            let mut last = b'\n';
            if name == b"-" {
                let stdin = io.take_stdin();
                more = f.push(stdin, io);
                last = stdin.last().copied().unwrap_or(b'\n');
            } else {
                match db.resolve(name) {
                    Err(e) => {
                        io.err_line(format!("{}: {}: {}", cmd, quote_if_needed(name), e.msg()));
                        status = 1;
                    }
                    Ok(id) if db.is_dir(id) => {
                        io.err_line(dir(name));
                        status = 1;
                    }
                    Ok(id) => {
                        for p in db.pieces(id) {
                            let text = db.text_of(p);
                            last = text.last().copied().unwrap_or(last);
                            more = f.push(&text, io);
                            if !more {
                                break;
                            }
                        }
                    }
                }
            }
            if more && last != b'\n' {
                more = f.push(b"\n", io);
            }
        }
        f.finish(io).max(status)
    });
    Prepared { run, input }
}

fn parsed(cmd: &'static str, args: &[Vec<u8>], bools: &[u8], with_arg: &[u8], longs: &[LongOpt]) -> Result<Result<Parsed, Prepared>, String> {
    Ok(options_long(cmd, args, bools, with_arg, longs, 1)?.map_err(Prepared::from))
}

// --------------------------------------------------------------------- cut

/// Positions counted from 1, both ends included, in order and apart.
type Ranges = Vec<(usize, usize)>;

fn cut_list(text: &[u8], fields: bool) -> Result<Ranges, String> {
    let zero = || if fields { "fields are numbered from 1".to_string() } else { "byte/character positions are numbered from 1".to_string() };
    let number = |t: &[u8]| -> Result<usize, String> {
        if t.is_empty() || !t.iter().all(|c| c.is_ascii_digit()) {
            let what = if fields { "invalid field value" } else { "invalid byte/character position" };
            return Err(format!("{} {}", what, quote(t)));
        }
        let n = std::str::from_utf8(t).unwrap().parse::<usize>().unwrap_or(usize::MAX);
        if n == 0 {
            return Err(zero());
        }
        Ok(n)
    };
    let mut out: Ranges = Vec::new();
    for item in text.split(|c| *c == b',' || *c == b' ' || *c == b'\t') {
        if item.is_empty() {
            return Err(zero());
        }
        let range = match item.iter().position(|c| *c == b'-') {
            None => {
                let n = number(item)?;
                (n, n)
            }
            Some(dash) => {
                let (lo, hi) = (&item[..dash], &item[dash + 1..]);
                if lo.is_empty() && hi.is_empty() {
                    return Err("invalid range with no endpoint: -".into());
                }
                let lo = if lo.is_empty() { 1 } else { number(lo)? };
                let hi = if hi.is_empty() { usize::MAX } else { number(hi)? };
                if hi < lo {
                    return Err("invalid decreasing range".into());
                }
                (lo, hi)
            }
        };
        out.push(range);
    }
    out.sort();
    let mut merged: Ranges = Vec::new();
    for (lo, hi) in out {
        match merged.last_mut() {
            Some(last) if lo <= last.1 => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    Ok(merged)
}

fn complement(r: &Ranges) -> Ranges {
    let mut out = Vec::new();
    let mut next = 1;
    for (lo, hi) in r {
        if *lo > next {
            out.push((next, lo - 1));
        }
        if *hi == usize::MAX {
            return out;
        }
        next = hi + 1;
    }
    out.push((next, usize::MAX));
    out
}

struct Cut {
    fields: bool,
    ranges: Ranges,
    delim: u8,
    only_delimited: bool,
    out_delim: Option<Vec<u8>>,
    lines: LineBuf,
    out: Vec<u8>,
}

impl Cut {
    fn line(&mut self, line: &[u8]) {
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        let out = &mut self.out;
        if !self.fields {
            let mut first = true;
            for (lo, hi) in &self.ranges {
                if *lo > text.len() {
                    break;
                }
                if !first {
                    if let Some(d) = &self.out_delim {
                        out.extend_from_slice(d);
                    }
                }
                first = false;
                out.extend_from_slice(&text[lo - 1..(*hi).min(text.len())]);
            }
            out.push(b'\n');
            return;
        }
        let Some(mut end) = memchr::memchr(self.delim, text) else {
            if !self.only_delimited {
                out.extend_from_slice(text);
                out.push(b'\n');
            }
            return;
        };
        // Walk the fields, with the ranges beside them.
        let mut start = 0;
        let mut field = 1;
        let mut r = 0;
        let mut first = true;
        loop {
            while r < self.ranges.len() && self.ranges[r].1 < field {
                r += 1;
            }
            if r == self.ranges.len() {
                break;
            }
            if self.ranges[r].0 <= field {
                if !first {
                    match &self.out_delim {
                        Some(d) => out.extend_from_slice(d),
                        None => out.push(self.delim),
                    }
                }
                first = false;
                if self.ranges[r].1 == usize::MAX && self.out_delim.is_none() {
                    // The rest of the line, as it is.
                    out.extend_from_slice(&text[start..]);
                    break;
                }
                out.extend_from_slice(&text[start..end]);
            }
            if end == text.len() {
                break;
            }
            start = end + 1;
            end = memchr::memchr(self.delim, &text[start..]).map_or(text.len(), |i| start + i);
            field += 1;
        }
        out.push(b'\n');
    }
}

impl Filter for Cut {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        let mut lines = std::mem::take(&mut self.lines);
        lines.feed(data, |l| {
            self.line(l);
            if self.out.len() >= 1 << 15 {
                io.put(&self.out);
                self.out.clear();
            }
            true
        });
        self.lines = lines;
        io.put(&self.out);
        self.out.clear();
        !io.stopped()
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if let Some(rest) = self.lines.rest() {
            self.line(&rest);
            io.put(&self.out);
        }
        0
    }
}

pub fn cut(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let longs: [LongOpt; 7] = [
        ("bytes", true, b'b'),
        ("characters", true, b'c'),
        ("fields", true, b'f'),
        ("delimiter", true, b'd'),
        ("only-delimited", false, b's'),
        ("complement", false, 1),
        ("output-delimiter", true, 2),
    ];
    let p = match parsed("cut", &args, b"sn", b"bcfd", &longs)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let bad = |m: &str| Ok(fails("cut", m.to_string(), true, 1));
    let lists: Vec<&(u8, Option<Vec<u8>>)> = p.flags.iter().filter(|(c, _)| b"bcf".contains(c)).collect();
    if lists.is_empty() {
        return bad("you must specify a list of bytes, characters, or fields");
    }
    if lists.len() > 1 {
        return bad("only one list may be specified");
    }
    let fields = lists[0].0 == b'f';
    let delim = match p.arg(b'd') {
        None => b'\t',
        Some(_) if !fields => return bad("an input delimiter may be specified only when operating on fields"),
        Some(d) if d.len() > 1 => return bad("the delimiter must be a single character"),
        Some(d) => d.first().copied().unwrap_or(0),
    };
    if p.has(b's') && !fields {
        return bad("suppressing non-delimited lines makes sense\n\tonly when operating on fields");
    }
    let mut ranges = match cut_list(lists[0].1.as_deref().unwrap_or(b""), fields) {
        Ok(r) => r,
        Err(m) => return bad(&m),
    };
    if p.has(1) {
        ranges = complement(&ranges);
    }
    let only_delimited = p.has(b's');
    let out_delim = p.arg(2).map(|d| d.to_vec());
    let make = move || -> Box<dyn Filter> {
        Box::new(Cut { fields, ranges: ranges.clone(), delim, only_delimited, out_delim: out_delim.clone(), lines: LineBuf::default(), out: Vec::new() })
    };
    Ok(over_files("cut", p.operands, make, |n| format!("cut: {}: Is a directory", quote_if_needed(n))))
}

// ---------------------------------------------------------------------- tr

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Alnum,
    Alpha,
    Blank,
    Cntrl,
    Digit,
    Graph,
    Lower,
    Print,
    Punct,
    Space,
    Upper,
    Xdigit,
}

impl Class {
    fn named(name: &[u8]) -> Option<Class> {
        Some(match name {
            b"alnum" => Class::Alnum,
            b"alpha" => Class::Alpha,
            b"blank" => Class::Blank,
            b"cntrl" => Class::Cntrl,
            b"digit" => Class::Digit,
            b"graph" => Class::Graph,
            b"lower" => Class::Lower,
            b"print" => Class::Print,
            b"punct" => Class::Punct,
            b"space" => Class::Space,
            b"upper" => Class::Upper,
            b"xdigit" => Class::Xdigit,
            _ => return None,
        })
    }

    fn has(self, c: u8) -> bool {
        match self {
            Class::Alnum => c.is_ascii_alphanumeric(),
            Class::Alpha => c.is_ascii_alphabetic(),
            Class::Blank => c == b' ' || c == b'\t',
            Class::Cntrl => c.is_ascii_control(),
            Class::Digit => c.is_ascii_digit(),
            Class::Graph => c.is_ascii_graphic(),
            Class::Lower => c.is_ascii_lowercase(),
            Class::Print => c.is_ascii_graphic() || c == b' ',
            Class::Punct => c.is_ascii_punctuation(),
            Class::Space => matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'),
            Class::Upper => c.is_ascii_uppercase(),
            Class::Xdigit => c.is_ascii_hexdigit(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Elem {
    Ch(u8),
    Range(u8, u8),
    Class(Class),
    Equiv(u8),
    /// `[c*n]`; a count of 0 is as many as are needed.
    Repeat(u8, usize),
}

/// A character of a tr set, and whether it was written with a backslash.
fn tr_chars(text: &[u8], warn: &mut bool) -> Vec<(u8, bool)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        i += 1;
        if c != b'\\' {
            out.push((c, false));
            continue;
        }
        let Some(n) = text.get(i).copied() else {
            *warn = true;
            out.push((b'\\', true));
            break;
        };
        i += 1;
        let v = match n {
            b'a' => 7,
            b'b' => 8,
            b'f' => 12,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => 11,
            b'0'..=b'7' => {
                let mut v = (n - b'0') as u32;
                let mut k = 1;
                while k < 3 && text.get(i).is_some_and(|d| (b'0'..=b'7').contains(d)) {
                    let next = v * 8 + (text[i] - b'0') as u32;
                    if next > 255 {
                        break;
                    }
                    v = next;
                    i += 1;
                    k += 1;
                }
                v as u8
            }
            other => other,
        };
        out.push((v, true));
    }
    out
}

fn tr_set(text: &[u8], warn: &mut bool) -> Result<Vec<Elem>, String> {
    let cs = tr_chars(text, warn);
    let is = |i: usize, c: u8| cs.get(i).is_some_and(|x| *x == (c, false));
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if is(i, b'[') {
            // [:class:] and [=c=]
            if is(i + 1, b':') || is(i + 1, b'=') {
                let mark = cs[i + 1].0;
                let close = (i + 2..cs.len()).find(|j| is(*j, mark) && is(j + 1, b']'));
                if let Some(j) = close {
                    let inner: Vec<u8> = cs[i + 2..j].iter().map(|x| x.0).collect();
                    if mark == b':' {
                        match Class::named(&inner) {
                            Some(c) => out.push(Elem::Class(c)),
                            None => return Err(format!("invalid character class {}", quote(&inner))),
                        }
                        i = j + 2;
                        continue;
                    }
                    if inner.len() == 1 {
                        out.push(Elem::Equiv(inner[0]));
                        i = j + 2;
                        continue;
                    }
                    if !inner.is_empty() {
                        return Err(format!("{}: equivalence class operand must be a single character", String::from_utf8_lossy(&inner)));
                    }
                }
            }
            // [c*n]
            if i + 2 < cs.len() && is(i + 2, b'*') {
                if let Some(j) = (i + 3..cs.len()).find(|j| is(*j, b']')) {
                    let digits: Vec<u8> = cs[i + 3..j].iter().map(|x| x.0).collect();
                    if cs[i + 3..j].iter().all(|x| !x.1 && x.0.is_ascii_digit()) {
                        let text = std::str::from_utf8(&digits).unwrap();
                        let n = if text.starts_with('0') { usize::from_str_radix(text, 8).ok() } else if text.is_empty() { Some(0) } else { text.parse().ok() };
                        match n {
                            Some(n) => {
                                out.push(Elem::Repeat(cs[i + 1].0, n));
                                i = j + 1;
                                continue;
                            }
                            None => return Err(format!("invalid repeat count {} in [c*n] construct", quote(&digits))),
                        }
                    }
                }
            }
        }
        // A range, or one character.
        if is(i + 1, b'-') && i + 2 < cs.len() {
            let (lo, hi) = (cs[i].0, cs[i + 2].0);
            if hi < lo {
                let show = |c: u8| if c.is_ascii_graphic() { char::from(c).to_string() } else { format!("\\{:03o}", c) };
                return Err(format!("range-endpoints of '{}-{}' are in reverse collating sequence order", show(lo), show(hi)));
            }
            out.push(Elem::Range(lo, hi));
            i += 3;
            continue;
        }
        out.push(Elem::Ch(cs[i].0));
        i += 1;
    }
    Ok(out)
}

/// The characters of a set in order; beside each, the class it came from
/// when it is the first of one.
fn expand(set: &[Elem], fill: usize) -> Vec<(u8, Option<Class>)> {
    let mut out = Vec::new();
    for e in set {
        match *e {
            Elem::Ch(c) | Elem::Equiv(c) => out.push((c, None)),
            Elem::Range(lo, hi) => out.extend((lo..=hi).map(|c| (c, None))),
            Elem::Class(class) => {
                let at = out.len();
                out.extend((0..=255u8).filter(|c| class.has(*c)).map(|c| (c, None)));
                if let Some(first) = out.get_mut(at) {
                    first.1 = Some(class);
                }
            }
            Elem::Repeat(c, n) => out.extend(std::iter::repeat((c, None)).take(if n == 0 { fill } else { n })),
        }
    }
    out
}

struct Tr {
    map: [u8; 256],
    delete: [bool; 256],
    squeeze: [bool; 256],
    plain: bool,
    last: Option<u8>,
    out: Vec<u8>,
}

impl Filter for Tr {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        for chunk in data.chunks(1 << 15) {
            self.out.clear();
            if self.plain {
                self.out.extend(chunk.iter().map(|c| self.map[*c as usize]));
            } else {
                for c in chunk {
                    if self.delete[*c as usize] {
                        continue;
                    }
                    let c = self.map[*c as usize];
                    if self.squeeze[c as usize] && self.last == Some(c) {
                        continue;
                    }
                    self.last = Some(c);
                    self.out.push(c);
                }
            }
            io.put(&self.out);
        }
        !io.stopped()
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }
}

pub fn tr(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let longs: [LongOpt; 4] = [("delete", false, b'd'), ("squeeze-repeats", false, b's'), ("complement", false, b'c'), ("truncate-set1", false, b't')];
    // A lone `-` is a set, and so is anything after the first set.
    let p = match parsed("tr", &args, b"dscCt", b"", &longs)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let (delete, squeeze, comp, truncate) = (p.has(b'd'), p.has(b's'), p.has(b'c') || p.has(b'C'), p.has(b't'));
    let ops = &p.operands;
    let translating = !delete && !(squeeze && ops.len() == 1);
    let bad = |m: String| Ok(fails("tr", m, true, 1));
    let wanted = if delete == squeeze { 2 } else { 1 };
    let most = if delete && !squeeze { 1 } else { 2 };
    if ops.is_empty() {
        return bad("missing operand".into());
    }
    if ops.len() < wanted && !(squeeze && !delete) {
        let why = if delete { "Two strings must be given when both deleting and squeezing repeats." } else { "Two strings must be given when translating." };
        return bad(format!("missing operand after {}\n{}", quote(&ops[ops.len() - 1]), why));
    }
    if ops.len() > most {
        let mut m = format!("extra operand {}", quote(&ops[most]));
        if most == 1 {
            m.push_str("\nOnly one string may be given when deleting without squeezing repeats.");
        }
        return bad(m);
    }
    let stop = |m: String| Ok(fails("tr", m, false, 1));
    let mut warn = false;
    let set1 = match tr_set(&ops[0], &mut warn) {
        Ok(s) => s,
        Err(m) => return stop(m),
    };
    let set2 = match ops.get(1).map(|o| tr_set(o, &mut warn)) {
        Some(Ok(s)) => Some(s),
        Some(Err(m)) => return stop(m),
        None => None,
    };
    if set1.iter().any(|e| matches!(e, Elem::Repeat(_, 0))) {
        return stop("the [c*] repeat construct may not appear in string1".into());
    }
    let mut s1: Vec<(u8, Option<Class>)> = expand(&set1, 0);
    if comp {
        let mut member = [false; 256];
        s1.iter().for_each(|(c, _)| member[*c as usize] = true);
        s1 = (0..=255u8).filter(|c| !member[*c as usize]).map(|c| (c, None)).collect();
    }
    let mut t = Tr { map: std::array::from_fn(|i| i as u8), delete: [false; 256], squeeze: [false; 256], plain: false, last: None, out: Vec::new() };
    if let Some(set2) = &set2 {
        let fills = set2.iter().filter(|e| matches!(e, Elem::Repeat(_, 0))).count();
        if fills > 1 {
            return stop("only one [c*] repeat construct may appear in string2".into());
        }
        if translating {
            if set2.iter().any(|e| matches!(e, Elem::Equiv(_))) {
                return stop("[=c=] expressions may not appear in string2 when translating".into());
            }
            if set2.iter().any(|e| matches!(e, Elem::Class(c) if *c != Class::Upper && *c != Class::Lower)) {
                return stop("when translating, the only character classes that may appear in\nstring2 are 'upper' and 'lower'".into());
            }
        } else if fills > 0 || set2.iter().any(|e| matches!(e, Elem::Repeat(..))) {
            return stop("the [c*] construct may appear in string2 only when translating".into());
        }
        let fixed = expand(set2, 0).len();
        let mut s2 = expand(set2, s1.len().saturating_sub(fixed));
        if translating {
            for (i, (_, class)) in s2.iter().enumerate() {
                if class.is_some() && !matches!(s1.get(i), Some((_, Some(Class::Upper | Class::Lower)))) {
                    return stop("misaligned [:upper:] and/or [:lower:] construct".into());
                }
            }
            if s2.len() < s1.len() {
                if truncate {
                    s1.truncate(s2.len());
                } else {
                    if s2.is_empty() {
                        return stop("when not truncating set1, string2 must be non-empty".into());
                    }
                    if matches!(set2.last(), Some(Elem::Class(_))) {
                        return stop("when translating with string1 longer than string2,\nthe latter string must not end with a character class".into());
                    }
                    let last = (s2[s2.len() - 1].0, None);
                    s2.resize(s1.len(), last);
                }
            }
            for ((from, _), (to, _)) in s1.iter().zip(&s2) {
                t.map[*from as usize] = *to;
            }
            if squeeze {
                s2.iter().for_each(|(c, _)| t.squeeze[*c as usize] = true);
            }
        } else {
            // Deleting and squeezing: the second set is what is squeezed.
            s1.iter().for_each(|(c, _)| t.delete[*c as usize] = true);
            s2.iter().for_each(|(c, _)| t.squeeze[*c as usize] = true);
        }
    } else if delete {
        s1.iter().for_each(|(c, _)| t.delete[*c as usize] = true);
    } else {
        s1.iter().for_each(|(c, _)| t.squeeze[*c as usize] = true);
    }
    t.plain = !t.delete.iter().any(|d| *d) && !t.squeeze.iter().any(|s| *s);
    let (map, del, sq, plain) = (t.map, t.delete, t.squeeze, t.plain);
    let make = move || Box::new(Tr { map, delete: del, squeeze: sq, plain, last: None, out: Vec::new() });
    let input = Input::Stream(make());
    let run: Runner = Box::new(move |_, io| {
        if warn {
            io.err_line("tr: warning: an unescaped backslash at end of string is not portable");
        }
        let mut f = make();
        let stdin = io.take_stdin();
        f.push(stdin, io);
        0
    });
    if warn {
        // The warning comes before any output: take the input whole.
        return Ok(Prepared { run, input: Input::Whole });
    }
    Ok(Prepared { run, input })
}

// -------------------------------------------------------------------- uniq

#[derive(Clone, Copy)]
struct UniqOpts {
    count: bool,
    repeated: bool,
    unique: bool,
    icase: bool,
    skip_fields: usize,
    skip_chars: usize,
    width: usize,
}

struct Uniq {
    o: UniqOpts,
    held: Option<Vec<u8>>,
    n: u64,
    lines: LineBuf,
    out: Vec<u8>,
}

impl Uniq {
    fn key<'a>(&self, line: &'a [u8]) -> &'a [u8] {
        let mut i = 0;
        for _ in 0..self.o.skip_fields {
            if i == line.len() {
                break;
            }
            while i < line.len() && (line[i] == b' ' || line[i] == b'\t') {
                i += 1;
            }
            while i < line.len() && !(line[i] == b' ' || line[i] == b'\t') {
                i += 1;
            }
        }
        i = (i.saturating_add(self.o.skip_chars)).min(line.len());
        let rest = &line[i..];
        &rest[..rest.len().min(self.o.width)]
    }

    fn same(&self, a: &[u8], b: &[u8]) -> bool {
        let (a, b) = (self.key(a), self.key(b));
        if self.o.icase {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    }

    fn emit(&mut self) {
        let Some(line) = self.held.take() else { return };
        let show = match (self.o.repeated, self.o.unique) {
            (true, true) => false,
            (true, false) => self.n > 1,
            (false, true) => self.n == 1,
            (false, false) => true,
        };
        if show {
            if self.o.count {
                self.out.extend_from_slice(format!("{:>7} ", self.n).as_bytes());
            }
            self.out.extend_from_slice(&line);
            self.out.push(b'\n');
        }
    }

    fn line(&mut self, line: &[u8]) {
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        if self.held.as_ref().is_some_and(|h| self.same(h, text)) {
            self.n += 1;
            return;
        }
        self.emit();
        self.held = Some(text.to_vec());
        self.n = 1;
    }
}

impl Filter for Uniq {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        let mut lines = std::mem::take(&mut self.lines);
        lines.feed(data, |l| {
            self.line(l);
            if self.out.len() >= 1 << 15 {
                io.put(&self.out);
                self.out.clear();
            }
            true
        });
        self.lines = lines;
        io.put(&self.out);
        self.out.clear();
        !io.stopped()
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if let Some(rest) = self.lines.rest() {
            self.line(&rest);
        }
        self.emit();
        io.put(&self.out);
        0
    }
}

pub fn uniq(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let longs: [LongOpt; 7] = [
        ("count", false, b'c'),
        ("repeated", false, b'd'),
        ("unique", false, b'u'),
        ("ignore-case", false, b'i'),
        ("skip-fields", true, b'f'),
        ("skip-chars", true, b's'),
        ("check-chars", true, b'w'),
    ];
    let p = match parsed("uniq", &args, b"cdui", b"fsw", &longs)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    if p.operands.len() > 2 {
        return Ok(fails("uniq", format!("extra operand {}", quote(&p.operands[2])), true, 1));
    }
    if p.operands.len() == 2 {
        return Err("flag: uniq with an output file (write the output with > instead)".into());
    }
    let number = |flag: u8, what: &str| -> Result<Option<usize>, Prepared> {
        match p.arg(flag) {
            None => Ok(None),
            Some(a) if !a.is_empty() && a.iter().all(|c| c.is_ascii_digit()) => Ok(Some(std::str::from_utf8(a).unwrap().parse().unwrap_or(usize::MAX))),
            Some(a) => Err(fails("uniq", format!("{}: invalid number of {}", String::from_utf8_lossy(a), what), false, 1)),
        }
    };
    let skip_fields = match number(b'f', "fields to skip") {
        Ok(n) => n.unwrap_or(0),
        Err(r) => return Ok(r),
    };
    let skip_chars = match number(b's', "bytes to skip") {
        Ok(n) => n.unwrap_or(0),
        Err(r) => return Ok(r),
    };
    let width = match number(b'w', "bytes to compare") {
        Ok(n) => n.unwrap_or(usize::MAX),
        Err(r) => return Ok(r),
    };
    let o = UniqOpts { count: p.has(b'c'), repeated: p.has(b'd'), unique: p.has(b'u'), icase: p.has(b'i'), skip_fields, skip_chars, width };
    let make = move || -> Box<dyn Filter> { Box::new(Uniq { o, held: None, n: 0, lines: LineBuf::default(), out: Vec::new() }) };
    let files = if p.operands.first().is_some_and(|f| f == b"-") { Vec::new() } else { p.operands };
    Ok(over_files("uniq", files, make, |n| format!("uniq: error reading {}: Is a directory", quote(n))))
}

// ---------------------------------------------------------------------- nl

#[derive(Clone, Copy, PartialEq, Eq)]
enum Numbering {
    All,
    NonEmpty,
    None,
}

#[derive(Clone)]
struct NlOpts {
    body: Numbering,
    header: Numbering,
    footer: Numbering,
    start: i64,
    step: i64,
    width: usize,
    sep: Vec<u8>,
    /// b'l' for ln, b'r' for rn, b'z' for rz.
    format: u8,
}

struct Nl {
    o: NlOpts,
    n: i64,
    section: Numbering,
    lines: LineBuf,
    out: Vec<u8>,
}

impl Nl {
    fn line(&mut self, line: &[u8]) {
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        let section = match text {
            b"\\:\\:\\:" => Some(self.o.header),
            b"\\:\\:" => Some(self.o.body),
            b"\\:" => Some(self.o.footer),
            _ => None,
        };
        if let Some(s) = section {
            self.section = s;
            self.n = self.o.start;
            self.out.push(b'\n');
            return;
        }
        let numbered = match self.section {
            Numbering::All => true,
            Numbering::NonEmpty => !text.is_empty(),
            Numbering::None => false,
        };
        if numbered {
            let w = self.o.width;
            let s = match self.o.format {
                b'l' => format!("{:<w$}", self.n),
                b'z' if self.n < 0 => format!("-{:0>w$}", -(self.n as i128), w = w.saturating_sub(1)),
                b'z' => format!("{:0>w$}", self.n),
                _ => format!("{:>w$}", self.n),
            };
            self.out.extend_from_slice(s.as_bytes());
            self.out.extend_from_slice(&self.o.sep);
            self.n = self.n.wrapping_add(self.o.step);
        } else {
            self.out.resize(self.out.len() + self.o.width + self.o.sep.len(), b' ');
        }
        self.out.extend_from_slice(text);
        self.out.push(b'\n');
    }
}

impl Filter for Nl {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        let mut lines = std::mem::take(&mut self.lines);
        lines.feed(data, |l| {
            self.line(l);
            if self.out.len() >= 1 << 15 {
                io.put(&self.out);
                self.out.clear();
            }
            true
        });
        self.lines = lines;
        io.put(&self.out);
        self.out.clear();
        !io.stopped()
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if let Some(rest) = self.lines.rest() {
            self.line(&rest);
            io.put(&self.out);
        }
        0
    }
}

pub fn nl(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let longs: [LongOpt; 8] = [
        ("body-numbering", true, b'b'),
        ("header-numbering", true, b'h'),
        ("footer-numbering", true, b'f'),
        ("starting-line-number", true, b'v'),
        ("line-increment", true, b'i'),
        ("number-width", true, b'w'),
        ("number-separator", true, b's'),
        ("number-format", true, b'n'),
    ];
    let p = match parsed("nl", &args, b"", b"bhfviwsn", &longs)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let mut o = NlOpts { body: Numbering::NonEmpty, header: Numbering::None, footer: Numbering::None, start: 1, step: 1, width: 6, sep: b"\t".to_vec(), format: b'r' };
    for (flag, arg) in &p.flags {
        let a = arg.as_deref().unwrap_or(b"");
        let int = |what: &str| -> Result<i64, Prepared> {
            std::str::from_utf8(a).ok().and_then(|s| s.parse::<i64>().ok()).ok_or_else(|| fails("nl", format!("invalid {}: {}", what, quote(a)), false, 1))
        };
        match flag {
            b'b' | b'h' | b'f' => {
                let style = match a {
                    b"a" => Numbering::All,
                    b"t" => Numbering::NonEmpty,
                    b"n" => Numbering::None,
                    _ if a.starts_with(b"p") => return Err("flag: nl -b pREGEX (numbering by pattern is not supported)".into()),
                    _ => {
                        let which = match flag {
                            b'b' => "body",
                            b'h' => "header",
                            _ => "footer",
                        };
                        return Ok(fails("nl", format!("invalid {} numbering style: {}", which, quote(a)), true, 1));
                    }
                };
                match flag {
                    b'b' => o.body = style,
                    b'h' => o.header = style,
                    _ => o.footer = style,
                }
            }
            b'v' => match int("starting line number") {
                Ok(n) => o.start = n,
                Err(r) => return Ok(r),
            },
            b'i' => match int("line number increment") {
                Ok(n) => o.step = n,
                Err(r) => return Ok(r),
            },
            b'w' => match int("line number field width") {
                Ok(n) if n >= 1 => o.width = n as usize,
                Ok(_) => return Ok(fails("nl", format!("invalid line number field width: {}: Numerical result out of range", quote(a)), false, 1)),
                Err(r) => return Ok(r),
            },
            b's' => o.sep = a.to_vec(),
            _ => {
                o.format = match a {
                    b"ln" => b'l',
                    b"rn" => b'r',
                    b"rz" => b'z',
                    _ => return Ok(fails("nl", format!("invalid line numbering format: {}", quote(a)), true, 1)),
                }
            }
        }
    }
    let make = move || -> Box<dyn Filter> { Box::new(Nl { n: o.start, section: o.body, o: o.clone(), lines: LineBuf::default(), out: Vec::new() }) };
    Ok(over_files("nl", p.operands, make, |n| format!("nl: {}: Is a directory", quote_if_needed(n))))
}

// --------------------------------------------------------------------- tee

/// Copies its input to its output and gathers it for its files, which
/// are written when the input has ended: a command whose text is not
/// valid changes nothing.
struct Tee {
    files: Vec<Vec<u8>>,
    append: bool,
    spool: Option<Spool>,
}

impl Tee {
    fn store(mut self, db: &mut Db, io: &mut Io) -> i32 {
        let mut spool = self.spool.take().unwrap_or_else(|| Spool::new(io.scratch()));
        let valid = spool.end();
        let mut status = 0;
        for name in &self.files {
            let target: Result<NodeId, &str> = match db.resolve(name) {
                Ok(id) if db.is_dir(id) => Err("Is a directory"),
                Ok(id) => Ok(id),
                Err(_) if !valid => continue,
                Err(_) => match db.resolve_parent(name) {
                    Ok((parent, base)) if !name.ends_with(b"/") => Ok(db.create_file(parent, &base, b"")),
                    Ok(_) => Err("Is a directory"),
                    Err(e) => Err(e.msg()),
                },
            };
            match target {
                Err(why) => {
                    io.err_line(format!("tee: {}: {}", quote_if_needed(name), why));
                    status = 1;
                }
                Ok(_) if !valid => {}
                Ok(id) => {
                    if !self.append && db.size(id) > 0 {
                        db.set_content(id, b"");
                    }
                    let mut late = Vec::new();
                    status = status.max(store_output(db, id, self.append, name, &mut spool, &mut late));
                    io.err.extend_from_slice(&late);
                    if spool.len() == 0 {
                        db.touch(id);
                    }
                }
            }
        }
        if !valid {
            for name in &self.files {
                io.err_line(format!("qdb: {}: output is not valid UTF-8 text, nothing was written", String::from_utf8_lossy(name)));
            }
            status = 1;
        }
        status
    }
}

impl Filter for Tee {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        use crate::engine::Sink;
        io.put(data);
        if !self.files.is_empty() {
            let scratch = io.scratch();
            self.spool.get_or_insert_with(|| Spool::new(scratch)).write(data);
        }
        // The files are still to be written: it takes all its input.
        true
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }

    fn finish_db(self: Box<Self>, db: &mut Db, io: &mut Io) -> i32 {
        self.store(db, io)
    }
}

pub fn tee(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let longs: [LongOpt; 2] = [("append", false, b'a'), ("ignore-interrupts", false, b'i')];
    let p = match parsed("tee", &args, b"ai", b"", &longs)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let append = p.has(b'a');
    let files = p.operands;
    let stream = Tee { files: files.clone(), append, spool: None };
    let run: Runner = Box::new(move |db, io| {
        let mut t = Tee { files, append, spool: None };
        let stdin = io.take_stdin();
        t.push(stdin, io);
        t.store(db, io)
    });
    Ok(Prepared { run, input: Input::Stream(Box::new(stream)) })
}
