//! sort. Lines are gathered into runs; a run that has grown past what
//! may be held is sorted and written to a scratch file, and the runs are
//! merged at the end, so input of any size takes a fixed amount of memory.

use super::text::fails;
use super::{options_long, LineBuf};
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use crate::tree::Scratch;
use crate::util::{quote, LongOpt};
use crate::vfs::VFile;
use std::cmp::Ordering;

#[derive(Clone, Copy, Default, Debug)]
struct Order {
    skip_start: bool,
    skip_end: bool,
    fold: bool,
    numeric: bool,
    human: bool,
    reverse: bool,
}

impl Order {
    fn is_default(&self) -> bool {
        !(self.skip_start || self.skip_end || self.fold || self.numeric || self.human)
    }
}

#[derive(Clone, Copy, Debug)]
struct Key {
    /// Fields and characters to pass over to reach the start.
    sword: usize,
    schar: usize,
    /// The end: fields to pass over, then characters; `None` is the end
    /// of the line.
    end: Option<(usize, usize)>,
    o: Order,
}

#[derive(Clone)]
struct Opts {
    keys: Vec<Key>,
    tab: Option<u8>,
    unique: bool,
    stable: bool,
    reverse: bool,
}

fn blank(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

/// A number as sort -n reads it: sign, digits before the point without
/// leading zeros, digits after it without trailing zeros.
fn number(s: &[u8]) -> (bool, &[u8], &[u8]) {
    let mut i = 0;
    while i < s.len() && blank(s[i]) {
        i += 1;
    }
    let neg = s.get(i) == Some(&b'-');
    if neg {
        i += 1;
    }
    let start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    let mut int = &s[start..i];
    while int.first() == Some(&b'0') {
        int = &int[1..];
    }
    let mut frac: &[u8] = b"";
    if s.get(i) == Some(&b'.') {
        let from = i + 1;
        let mut j = from;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        frac = &s[from..j];
        while frac.last() == Some(&b'0') {
            frac = &frac[..frac.len() - 1];
        }
    }
    (neg && !(int.is_empty() && frac.is_empty()), int, frac)
}

fn numeric(a: &[u8], b: &[u8]) -> Ordering {
    let (an, ai, af) = number(a);
    let (bn, bi, bf) = number(b);
    match (an, bn) {
        (false, true) => return Ordering::Greater,
        (true, false) => return Ordering::Less,
        _ => {}
    }
    let size = ai.len().cmp(&bi.len()).then_with(|| ai.cmp(bi)).then_with(|| af.cmp(bf));
    if an {
        size.reverse()
    } else {
        size
    }
}

/// The order of the unit after a number: K is 1, M is 2 and so on.
fn unit(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && blank(s[i]) {
        i += 1;
    }
    let neg = s.get(i) == Some(&b'-');
    if neg {
        i += 1;
    }
    let mut nonzero = false;
    loop {
        while i < s.len() && s[i].is_ascii_digit() {
            nonzero |= s[i] != b'0';
            i += 1;
        }
        if s.get(i) == Some(&b'.') {
            i += 1;
        } else {
            break;
        }
    }
    if !nonzero {
        return 0;
    }
    let order = match s.get(i) {
        Some(b'K' | b'k') => 1,
        Some(b'M') => 2,
        Some(b'G') => 3,
        Some(b'T') => 4,
        Some(b'P') => 5,
        Some(b'E') => 6,
        Some(b'Z') => 7,
        Some(b'Y') => 8,
        Some(b'R') => 9,
        Some(b'Q') => 10,
        _ => 0,
    };
    if neg {
        -order
    } else {
        order
    }
}

impl Opts {
    fn field<'a>(&self, line: &'a [u8], k: &Key) -> &'a [u8] {
        let lim = line.len();
        // The start.
        let mut p = 0;
        let mut n = k.sword;
        match self.tab {
            Some(t) => {
                while p < lim && n > 0 {
                    n -= 1;
                    while p < lim && line[p] != t {
                        p += 1;
                    }
                    if p < lim {
                        p += 1;
                    }
                }
            }
            None => {
                while p < lim && n > 0 {
                    n -= 1;
                    while p < lim && blank(line[p]) {
                        p += 1;
                    }
                    while p < lim && !blank(line[p]) {
                        p += 1;
                    }
                }
            }
        }
        if k.o.skip_start {
            while p < lim && blank(line[p]) {
                p += 1;
            }
        }
        let start = p.saturating_add(k.schar).min(lim);
        // The end.
        let end = match k.end {
            None => lim,
            Some((eword, echar)) => {
                let mut p = 0;
                let mut n = if echar == 0 { eword + 1 } else { eword };
                match self.tab {
                    Some(t) => {
                        while p < lim && n > 0 {
                            n -= 1;
                            while p < lim && line[p] != t {
                                p += 1;
                            }
                            if p < lim && (n > 0 || echar > 0) {
                                p += 1;
                            }
                        }
                    }
                    None => {
                        while p < lim && n > 0 {
                            n -= 1;
                            while p < lim && blank(line[p]) {
                                p += 1;
                            }
                            while p < lim && !blank(line[p]) {
                                p += 1;
                            }
                        }
                    }
                }
                if echar > 0 {
                    if k.o.skip_end {
                        while p < lim && blank(line[p]) {
                            p += 1;
                        }
                    }
                    p = p.saturating_add(echar).min(lim);
                }
                p
            }
        };
        &line[start..end.max(start)]
    }

    /// The order of two lines by the keys alone.
    fn by_keys(&self, a: &[u8], b: &[u8]) -> Ordering {
        for k in &self.keys {
            let (x, y) = (self.field(a, k), self.field(b, k));
            let ord = if k.o.human {
                unit(x).cmp(&unit(y)).then_with(|| numeric(x, y))
            } else if k.o.numeric {
                numeric(x, y)
            } else if k.o.fold {
                x.iter().map(|c| c.to_ascii_uppercase()).cmp(y.iter().map(|c| c.to_ascii_uppercase()))
            } else {
                x.cmp(y)
            };
            if ord != Ordering::Equal {
                return if k.o.reverse { ord.reverse() } else { ord };
            }
        }
        Ordering::Equal
    }

    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        if self.keys.is_empty() {
            let ord = a.cmp(b);
            return if self.reverse { ord.reverse() } else { ord };
        }
        let ord = self.by_keys(a, b);
        if ord != Ordering::Equal || self.unique || self.stable {
            return ord;
        }
        let ord = a.cmp(b);
        if self.reverse {
            ord.reverse()
        } else {
            ord
        }
    }
}

/// One line of the run being gathered: where it is, and its first bytes
/// to settle most comparisons without looking at it.
#[derive(Clone, Copy)]
struct Ent {
    prefix: u64,
    off: u32,
    len: u32,
}

fn prefix(line: &[u8]) -> u64 {
    let mut b = [0u8; 8];
    let n = line.len().min(8);
    b[..n].copy_from_slice(&line[..n]);
    u64::from_be_bytes(b)
}

static RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct Sorter {
    o: Opts,
    buf: Vec<u8>,
    ents: Vec<Ent>,
    lines: LineBuf,
    /// Where each run written out begins and ends in the scratch file.
    runs: Vec<(u64, u64)>,
    file: Option<(String, Box<dyn VFile>)>,
    scratch: Option<Scratch>,
    written: u64,
    failed: Option<std::io::Error>,
}

/// With somewhere to write runs, a run is this many bytes of lines.
const RUN_BYTES: usize = 96 << 20;

impl Sorter {
    fn new(o: Opts) -> Sorter {
        Sorter { o, buf: Vec::new(), ents: Vec::new(), lines: LineBuf::default(), runs: Vec::new(), file: None, scratch: None, written: 0, failed: None }
    }

    fn add(&mut self, line: &[u8]) {
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        if self.buf.len() + text.len() > u32::MAX as usize || (self.buf.len() + self.ents.len() * 16 >= RUN_BYTES && self.scratch.as_ref().is_some_and(|s| s.vfs.is_some())) {
            self.spill();
        }
        self.ents.push(Ent { prefix: if self.o.keys.is_empty() { prefix(text) } else { 0 }, off: self.buf.len() as u32, len: text.len() as u32 });
        self.buf.extend_from_slice(text);
    }

    fn sort_run(&mut self) {
        let buf = &self.buf;
        let o = &self.o;
        let text = |e: &Ent| &buf[e.off as usize..e.off as usize + e.len as usize];
        if o.keys.is_empty() {
            if o.reverse {
                self.ents.sort_unstable_by(|a, b| b.prefix.cmp(&a.prefix).then_with(|| text(b).cmp(text(a))));
            } else {
                self.ents.sort_unstable_by(|a, b| a.prefix.cmp(&b.prefix).then_with(|| text(a).cmp(text(b))));
            }
        } else {
            self.ents.sort_by(|a, b| o.compare(text(a), text(b)));
        }
    }

    /// Sort the run in hand and write it to the scratch file.
    fn spill(&mut self) {
        self.sort_run();
        if self.file.is_none() {
            let vfs = self.scratch.as_ref().and_then(|s| s.vfs.clone()).expect("a place for scratch files");
            let name = format!("sort-{}.tmp", RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
            match vfs.create(&name, 1 << 40) {
                Ok(f) => self.file = Some((name, f)),
                Err(e) => {
                    self.failed = Some(e);
                    self.buf.clear();
                    self.ents.clear();
                    return;
                }
            }
        }
        let start = self.written;
        let (_, f) = self.file.as_mut().unwrap();
        let mut out: Vec<u8> = Vec::with_capacity(1 << 20);
        for e in &self.ents {
            out.extend_from_slice(&self.buf[e.off as usize..e.off as usize + e.len as usize]);
            out.push(b'\n');
            if out.len() >= 1 << 20 {
                if let Err(e) = f.write_at(self.written, &out) {
                    self.failed = Some(e);
                }
                self.written += out.len() as u64;
                out.clear();
            }
        }
        if let Err(e) = f.write_at(self.written, &out) {
            self.failed = Some(e);
        }
        self.written += out.len() as u64;
        self.runs.push((start, self.written));
        self.buf.clear();
        self.ents.clear();
    }

    fn feed(&mut self, data: &[u8], io: &Io) {
        if self.scratch.is_none() {
            self.scratch = Some(io.scratch());
        }
        let mut lines = std::mem::take(&mut self.lines);
        lines.feed(data, |l| {
            self.add(l);
            true
        });
        self.lines = lines;
    }

    /// Write everything out in order.
    fn output(mut self, io: &mut Io) -> i32 {
        if let Some(rest) = self.lines.rest() {
            self.add(&rest);
        }
        let mut out: Vec<u8> = Vec::with_capacity(1 << 16);
        if self.runs.is_empty() {
            self.sort_run();
            let mut last: Option<&[u8]> = None;
            for e in &self.ents {
                let text = &self.buf[e.off as usize..e.off as usize + e.len as usize];
                if self.o.unique {
                    if last.is_some_and(|l| self.o.compare(l, text) == Ordering::Equal) {
                        continue;
                    }
                    last = Some(text);
                }
                out.extend_from_slice(text);
                out.push(b'\n');
                if out.len() >= 1 << 16 {
                    io.put(&out);
                    out.clear();
                    if io.stopped() {
                        return 0;
                    }
                }
            }
            io.put(&out);
            return 0;
        }
        if !self.ents.is_empty() {
            self.spill();
        }
        if let Some(e) = self.failed.take() {
            io.err_line(format!("sort: write failed: {}", e));
            return 2;
        }
        let (_, f) = self.file.as_ref().unwrap();
        // Safety: the file holds `written` bytes and is not changed again
        // while `all` is in use.
        let all: &[u8] = unsafe { std::slice::from_raw_parts(f.base(), self.written as usize) };
        // The next line of each run, and the lines after it.
        let mut heads: Vec<Option<(&[u8], &[u8])>> = self
            .runs
            .iter()
            .map(|(s, e)| {
                let run = &all[*s as usize..*e as usize];
                next_line(run)
            })
            .collect();
        let mut last: Vec<u8> = Vec::new();
        let mut any = false;
        loop {
            // The least head; of equal ones, that of the earliest run.
            let mut best: Option<usize> = None;
            for (i, h) in heads.iter().enumerate() {
                if let Some((line, _)) = h {
                    match best {
                        Some(b) if self.o.compare(heads[b].unwrap().0, line) != Ordering::Greater => {}
                        _ => best = Some(i),
                    }
                }
            }
            let Some(b) = best else { break };
            let (line, rest) = heads[b].unwrap();
            heads[b] = next_line(rest);
            if self.o.unique {
                if any && self.o.compare(&last, line) == Ordering::Equal {
                    continue;
                }
                last.clear();
                last.extend_from_slice(line);
                any = true;
            }
            out.extend_from_slice(line);
            out.push(b'\n');
            if out.len() >= 1 << 16 {
                io.put(&out);
                out.clear();
                if io.stopped() {
                    return 0;
                }
            }
        }
        io.put(&out);
        0
    }
}

fn next_line(run: &[u8]) -> Option<(&[u8], &[u8])> {
    if run.is_empty() {
        return None;
    }
    let nl = memchr::memchr(b'\n', run).unwrap_or(run.len());
    Some((&run[..nl], &run[(nl + 1).min(run.len())..]))
}

impl Drop for Sorter {
    fn drop(&mut self) {
        if let Some((name, f)) = self.file.take() {
            drop(f);
            if let Some(vfs) = self.scratch.as_ref().and_then(|s| s.vfs.as_ref()) {
                let _ = vfs.remove(&name);
            }
        }
    }
}

impl Filter for Sorter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        self.feed(data, io);
        true
    }

    fn finish(self: Box<Self>, io: &mut Io) -> i32 {
        self.output(io)
    }
}

/// Checks that its input is in order.
struct Checker {
    o: Opts,
    name: Vec<u8>,
    quiet: bool,
    last: Option<Vec<u8>>,
    n: u64,
    lines: LineBuf,
    found: bool,
}

impl Checker {
    fn line(&mut self, line: &[u8], io: &mut Io) -> bool {
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        self.n += 1;
        if let Some(last) = &self.last {
            let ord = self.o.compare(last, text);
            if ord == Ordering::Greater || (self.o.unique && ord == Ordering::Equal) {
                if !self.quiet {
                    io.err_line([&b"sort: "[..], &self.name, format!(":{}: disorder: ", self.n).as_bytes(), text].concat());
                }
                self.found = true;
                return false;
            }
        }
        match &mut self.last {
            Some(l) => {
                l.clear();
                l.extend_from_slice(text);
            }
            None => self.last = Some(text.to_vec()),
        }
        true
    }
}

impl Filter for Checker {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        if self.found {
            return false;
        }
        let mut lines = std::mem::take(&mut self.lines);
        let more = lines.feed(data, |l| self.line(l, io));
        self.lines = lines;
        more
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if !self.found {
            if let Some(rest) = self.lines.rest() {
                self.line(&rest, io);
            }
        }
        i32::from(self.found)
    }
}

/// Read a count at the start of `s`. Returns it and what follows.
fn count<'a>(s: &'a [u8], what: &str) -> Result<(usize, &'a [u8]), String> {
    let n = s.iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 {
        return Err(format!("{}: invalid count at start of {}", what, quote(s)));
    }
    Ok((std::str::from_utf8(&s[..n]).unwrap().parse().unwrap_or(usize::MAX), &s[n..]))
}

fn ordering<'a>(mut s: &'a [u8], o: &mut Order, at_end: bool, own: &mut bool) -> &'a [u8] {
    while let Some(c) = s.first() {
        match c {
            b'b' => {
                if at_end {
                    o.skip_end = true;
                } else {
                    o.skip_start = true;
                }
            }
            b'f' => o.fold = true,
            b'n' => o.numeric = true,
            b'h' => o.human = true,
            b'r' => o.reverse = true,
            _ => break,
        }
        *own = true;
        s = &s[1..];
    }
    s
}

/// Flags that change the order and are not supported, for the refusal.
const OTHER_ORDER: &[u8] = b"dgiMRV";

fn key(spec: &[u8]) -> Result<Result<(Key, bool), String>, String> {
    let bad = |why: &str| Ok(Err(format!("{}: invalid field specification {}", why, quote(spec))));
    let (sword, mut s) = match count(spec, "invalid number at field start") {
        Ok(x) => x,
        Err(m) => return Ok(Err(m)),
    };
    if sword == 0 {
        return bad("field number is zero");
    }
    let mut schar = 0;
    if s.first() == Some(&b'.') {
        let (n, rest) = match count(&s[1..], "invalid number after '.'") {
            Ok(x) => x,
            Err(m) => return Ok(Err(m)),
        };
        if n == 0 {
            return bad("character offset is zero");
        }
        schar = n - 1;
        s = rest;
    }
    let mut o = Order::default();
    let mut own = false;
    s = ordering(s, &mut o, false, &mut own);
    let mut end = None;
    if s.first() == Some(&b',') {
        let (eword, rest) = match count(&s[1..], "invalid number after ','") {
            Ok(x) => x,
            Err(m) => return Ok(Err(m)),
        };
        if eword == 0 {
            return bad("field number is zero");
        }
        s = rest;
        let mut echar = 0;
        if s.first() == Some(&b'.') {
            let (n, rest) = match count(&s[1..], "invalid number after '.'") {
                Ok(x) => x,
                Err(m) => return Ok(Err(m)),
            };
            echar = n;
            s = rest;
        }
        s = ordering(s, &mut o, true, &mut own);
        end = Some((eword - 1, echar));
    }
    if let Some(c) = s.first() {
        if OTHER_ORDER.contains(c) {
            return Err(format!("flag: sort -k with the {} modifier", char::from(*c)));
        }
        return bad("stray character in field spec");
    }
    Ok(Ok((Key { sword: sword - 1, schar, end, o }, own)))
}

pub fn sort(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let longs: [LongOpt; 12] = [
        ("unique", false, b'u'),
        ("numeric-sort", false, b'n'),
        ("reverse", false, b'r'),
        ("key", true, b'k'),
        ("field-separator", true, b't'),
        ("human-numeric-sort", false, b'h'),
        ("ignore-case", false, b'f'),
        ("ignore-leading-blanks", false, b'b'),
        ("stable", false, b's'),
        ("check", false, b'c'),
        ("parallel", true, 1),
        ("buffer-size", true, b'S'),
    ];
    let p = match options_long("sort", &args, b"unrhfbscC", b"ktS", &longs, 2)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let stop = |m: String| Ok(fails("sort", m, false, 2));
    let mut global = Order { skip_start: p.has(b'b'), skip_end: p.has(b'b'), fold: p.has(b'f'), numeric: p.has(b'n'), human: p.has(b'h'), reverse: p.has(b'r') };
    let mut tab: Option<u8> = None;
    let mut keys: Vec<(Key, bool)> = Vec::new();
    for (flag, arg) in &p.flags {
        let a = arg.as_deref().unwrap_or(b"");
        match flag {
            b't' => {
                let t = match a {
                    b"" => return stop("empty tab".into()),
                    b"\\0" => 0,
                    [c] => *c,
                    _ => return stop(format!("multi-character tab {}", quote(a))),
                };
                if tab.is_some_and(|old| old != t) {
                    return stop("incompatible tabs".into());
                }
                tab = Some(t);
            }
            b'k' => match key(a)? {
                Ok(k) => keys.push(k),
                Err(m) => return stop(m),
            },
            _ => {}
        }
    }
    let (check, quiet) = (p.has(b'c'), p.has(b'C'));
    if check && quiet {
        return stop("options '-cC' are incompatible".into());
    }
    // A key without an order of its own takes the one given for all.
    let mut list: Vec<Key> = keys
        .into_iter()
        .map(|(mut k, own)| {
            if !own {
                k.o = global;
            }
            k
        })
        .collect();
    for o in list.iter().map(|k| k.o).chain([global]) {
        if o.human && o.numeric {
            return stop("options '-hn' are incompatible".into());
        }
    }
    if list.is_empty() && !global.is_default() {
        list.push(Key { sword: 0, schar: 0, end: None, o: global });
    }
    if list.is_empty() {
        global.skip_start = false;
    }
    let o = Opts { keys: list, tab, unique: p.has(b'u'), stable: p.has(b's'), reverse: global.reverse };
    let files = p.operands;
    if check || quiet {
        if files.len() > 1 {
            return stop(format!("extra operand {} not allowed with -c", quote(&files[1])));
        }
        let name = files.first().cloned().unwrap_or_else(|| b"-".to_vec());
        let from_stdin = name == b"-";
        let make = move || Checker { o: o.clone(), name: name.clone(), quiet, last: None, n: 0, lines: LineBuf::default(), found: false };
        let input = if from_stdin { Input::Stream(Box::new(make())) } else { Input::Unused };
        let run: Runner = Box::new(move |db, io| {
            let mut c = Box::new(make());
            if from_stdin {
                let stdin = io.take_stdin();
                c.push(stdin, io);
                return c.finish(io);
            }
            match db.resolve(&c.name) {
                Err(e) => {
                    io.err_line([&b"sort: open failed: "[..], &c.name, b": ", e.msg().as_bytes()].concat());
                    2
                }
                Ok(id) if db.is_dir(id) => {
                    io.err_line([&b"sort: read failed: "[..], &c.name, b": Is a directory"].concat());
                    2
                }
                Ok(id) => {
                    for piece in db.pieces(id) {
                        if !c.push(&db.text_of(piece), io) {
                            break;
                        }
                    }
                    c.finish(io)
                }
            }
        });
        return Ok(Prepared { run, input });
    }
    let input = if files.is_empty() {
        Input::Stream(Box::new(Sorter::new(o.clone())))
    } else if files.iter().any(|f| f == b"-") {
        Input::Whole
    } else {
        Input::Unused
    };
    let run: Runner = Box::new(move |db, io| {
        let mut s = Sorter::new(o);
        if files.is_empty() {
            let stdin = io.take_stdin();
            s.feed(stdin, io);
            return s.output(io);
        }
        for f in &files {
            if f != b"-" {
                if let Err(e) = db.resolve(f) {
                    io.err_line([&b"sort: cannot read: "[..], f, b": ", e.msg().as_bytes()].concat());
                    return 2;
                }
            }
        }
        for f in &files {
            if f == b"-" {
                let stdin = io.take_stdin();
                s.feed(stdin, io);
            } else {
                let id = db.resolve(f).expect("checked");
                if db.is_dir(id) {
                    io.err_line([&b"sort: read failed: "[..], f, b": Is a directory"].concat());
                    return 2;
                }
                for piece in db.pieces(id) {
                    s.feed(&db.text_of(piece), io);
                }
            }
            // A file that does not end with a newline ends its last line.
            if let Some(rest) = s.lines.rest() {
                s.add(&rest);
            }
        }
        s.output(io)
    });
    Ok(Prepared { run, input })
}
