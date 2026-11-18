//! echo, cat, head, tail, wc.

use super::{msg, options, LineBuf};
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use std::collections::VecDeque;
use crate::tree::{Db, Errno};
use crate::util::{quote, quote_if_needed};

pub fn echo(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    // bash's echo: only leading words made of option letters are options.
    let mut newline = true;
    let mut first = 0;
    for a in &args {
        if a.len() < 2 || a[0] != b'-' || !a[1..].iter().all(|c| b"neE".contains(c)) {
            break;
        }
        if let Some(c) = a[1..].iter().find(|c| **c != b'n') {
            return Err(format!("flag: echo -{}", char::from(*c)));
        }
        newline = false;
        first += 1;
    }
    Ok(Box::new(move |_, io| {
        io.put(&args[first..].join(&b' '));
        if newline {
            io.put(b"\n");
        }
        0
    }))
}

/// Where a read command gets one input from.
enum Source<'a> {
    Data(&'a [u8]),
    Missing(Errno),
    Directory,
}

fn open<'a>(db: &'a Db, stdin: &'a [u8], name: &[u8]) -> Source<'a> {
    if name == b"-" {
        return Source::Data(stdin);
    }
    match db.resolve(name) {
        Err(e) => Source::Missing(e),
        Ok(id) if db.is_dir(id) => Source::Directory,
        Ok(id) => Source::Data(db.content(id)),
    }
}

fn operands_or_stdin(operands: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    if operands.is_empty() {
        vec![b"-".to_vec()]
    } else {
        operands
    }
}

/// `cat -n` numbering, which runs on across files.
#[derive(Default)]
struct Numbering {
    line_no: u64,
    mid_line: bool,
}

impl Numbering {
    fn put(&mut self, data: &[u8], io: &mut Io) {
        for line in crate::util::lines(data) {
            if !self.mid_line {
                self.line_no += 1;
                io.put(format!("{:>6}\t", self.line_no).as_bytes());
            }
            io.put(line);
            self.mid_line = !line.ends_with(b"\n");
        }
    }
}

struct CatFilter {
    numbering: Option<Numbering>,
}

impl Filter for CatFilter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        match &mut self.numbering {
            None => io.put(data),
            Some(n) => n.put(data, io),
        }
        true
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }
}

pub fn cat(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let p = match options("cat", &args, b"n", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let number = p.has(b'n');
    let input = match p.operands.as_slice() {
        [] => Input::Stream(Box::new(CatFilter { numbering: number.then(Numbering::default) })),
        ops if ops.iter().any(|f| f == b"-") => Input::Whole,
        _ => Input::Unused,
    };
    let files = operands_or_stdin(p.operands);
    let run: Runner = Box::new(move |db, io| {
        let mut status = 0;
        let mut numbering = Numbering::default();
        for f in &files {
            if io.stopped() {
                break;
            }
            if f != b"-" {
                // GNU cat refuses to copy a non-empty file onto itself.
                if let (Ok(id), Some(o)) = (db.resolve(f), io.out_file) {
                    if id == o && !db.is_dir(id) && !db.content(id).is_empty() {
                        io.err_line(format!("cat: {}: input file is output file", quote_if_needed(f)));
                        status = 1;
                        continue;
                    }
                }
            }
            let data = match open(db, if f == b"-" { io.take_stdin() } else { b"" }, f) {
                Source::Data(d) => d,
                Source::Missing(e) => {
                    io.err_line(format!("cat: {}: {}", quote_if_needed(f), e.msg()));
                    status = 1;
                    continue;
                }
                Source::Directory => {
                    io.err_line(format!("cat: {}: Is a directory", quote_if_needed(f)));
                    status = 1;
                    continue;
                }
            };
            if number {
                numbering.put(data, io);
            } else {
                io.put(data);
            }
        }
        status
    });
    Ok(Prepared { run, input })
}

/// A line count argument for head or tail: an optional sign and digits.
struct Count {
    sign: Option<u8>,
    n: usize,
}

fn parse_count(cmd: &'static str, arg: Option<&[u8]>) -> Result<Result<Count, Runner>, String> {
    let arg = match arg {
        None => return Ok(Ok(Count { sign: None, n: 10 })),
        Some(a) => a.to_vec(),
    };
    let (sign, digits) = match arg.first() {
        Some(c @ (b'+' | b'-')) => (Some(*c), &arg[1..]),
        _ => (None, &arg[..]),
    };
    let shown: Vec<u8> = if sign == Some(b'-') { digits.to_vec() } else { arg.clone() };
    if !digits.is_empty() && digits.iter().all(|c| c.is_ascii_digit()) {
        let n = digits.iter().try_fold(0u64, |n, c| n.checked_mul(10)?.checked_add((c - b'0') as u64));
        return Ok(match n {
            Some(n) => Ok(Count { sign, n: usize::try_from(n).unwrap_or(usize::MAX) }),
            None => Err(Box::new(move |_, io: &mut Io| {
                io.err_line(format!("{}: invalid number of lines: {}: Value too large for defined data type", cmd, quote(&shown)));
                1
            })),
        });
    }
    // GNU accepts size suffixes such as 1k; this version does not.
    let split = digits.iter().position(|c| !c.is_ascii_digit()).unwrap_or(0);
    let suffix = &digits[split..];
    let is_size_suffix = suffix == b"b" || (!suffix.is_empty() && b"kKMGTPEZYRQ".contains(&suffix[0]) && matches!(&suffix[1..], b"" | b"B" | b"iB"));
    if split > 0 && is_size_suffix {
        return Err(format!("flag: {} -n with a size suffix ({})", cmd, String::from_utf8_lossy(&arg)));
    }
    Ok(Err(Box::new(move |_, io: &mut Io| {
        io.err_line(format!("{}: invalid number of lines: {}", cmd, quote(&shown)));
        1
    })))
}

/// The byte offset just past the first `n` lines.
fn first_lines(data: &[u8], n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut seen = 0;
    for (i, c) in data.iter().enumerate() {
        if *c == b'\n' {
            seen += 1;
            if seen == n {
                return i + 1;
            }
        }
    }
    data.len()
}

/// The byte offset where the last `n` lines start.
fn last_lines(data: &[u8], n: usize) -> usize {
    if n == 0 {
        return data.len();
    }
    // A final line counts whether or not it ends with a newline.
    let body = data.strip_suffix(b"\n").unwrap_or(data);
    let mut seen = 0;
    for i in (0..body.len()).rev() {
        if body[i] == b'\n' {
            seen += 1;
            if seen == n {
                return i + 1;
            }
        }
    }
    0
}

fn line_count(data: &[u8]) -> usize {
    let newlines = data.iter().filter(|c| **c == b'\n').count();
    newlines + usize::from(!data.is_empty() && !data.ends_with(b"\n"))
}

/// `head -n N` on a stream: pass N lines on, then need no more.
struct HeadFilter {
    left: usize,
}

impl Filter for HeadFilter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        if self.left == 0 {
            return false;
        }
        let mut end = data.len();
        for (i, c) in data.iter().enumerate() {
            if *c == b'\n' {
                self.left -= 1;
                if self.left == 0 {
                    end = i + 1;
                    break;
                }
            }
        }
        io.put(&data[..end]);
        self.left > 0
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }
}

/// `tail -n +N` on a stream: drop N-1 lines, pass the rest on.
struct TailFromFilter {
    skip: usize,
}

impl Filter for TailFromFilter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        let mut start = 0;
        while self.skip > 0 {
            match data[start..].iter().position(|c| *c == b'\n') {
                Some(i) => {
                    start += i + 1;
                    self.skip -= 1;
                }
                None => return true,
            }
        }
        io.put(&data[start..]);
        true
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }
}

/// `tail -n N` on a stream: keep only the last N lines seen.
struct TailFilter {
    keep: usize,
    lines: VecDeque<Vec<u8>>,
    buf: LineBuf,
}

impl Filter for TailFilter {
    fn push(&mut self, data: &[u8], _io: &mut Io) -> bool {
        if self.keep == 0 {
            return false;
        }
        let (keep, lines) = (self.keep, &mut self.lines);
        self.buf.feed(data, |line| {
            if lines.len() == keep {
                lines.pop_front();
            }
            lines.push_back(line.to_vec());
            true
        })
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if let Some(last) = self.buf.rest() {
            if self.lines.len() == self.keep {
                self.lines.pop_front();
            }
            self.lines.push_back(last);
        }
        for line in self.lines.iter().take(self.keep) {
            io.put(line);
        }
        0
    }
}

fn head_or_tail(cmd: &'static str, args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    // GNU tail still reads a leading +N as the obsolete form of -n +N.
    if cmd == "tail" && args.first().is_some_and(|a| a.len() > 1 && a[0] == b'+' && a[1..].iter().all(|c| c.is_ascii_digit())) {
        return Err("flag: tail +N (write tail -n +N)".into());
    }
    let p = match options(cmd, &args, b"", b"n", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let count = match parse_count(cmd, p.arg(b'n'))? {
        Ok(c) => c,
        Err(r) => return Ok(r.into()),
    };
    let input = if !p.operands.is_empty() {
        if p.operands.iter().any(|f| f == b"-") {
            Input::Whole
        } else {
            Input::Unused
        }
    } else {
        match (cmd, count.sign) {
            // All but the last N lines: the end must be seen first.
            ("head", Some(b'-')) => Input::Whole,
            ("head", _) => Input::Stream(Box::new(HeadFilter { left: count.n })),
            (_, Some(b'+')) => Input::Stream(Box::new(TailFromFilter { skip: count.n.saturating_sub(1) })),
            _ => Input::Stream(Box::new(TailFilter { keep: count.n, lines: VecDeque::new(), buf: LineBuf::default() })),
        }
    };
    let files = operands_or_stdin(p.operands);
    let run: Runner = Box::new(move |db, io| {
        let mut status = 0;
        let mut first_header = true;
        for f in &files {
            let input = open(db, if f == b"-" { io.take_stdin() } else { b"" }, f);
            if let Source::Missing(e) = input {
                io.err_line(format!("{}: cannot open {} for reading: {}", cmd, quote(f), e.msg()));
                status = 1;
                continue;
            }
            if files.len() > 1 {
                if !first_header {
                    io.put(b"\n");
                }
                first_header = false;
                let shown: &[u8] = if f == b"-" { b"standard input" } else { f };
                io.put(&msg(&[b"==> ", shown, b" <==\n"]));
            }
            let data = match input {
                Source::Data(d) => d,
                _ => {
                    io.err_line(format!("{}: error reading {}: Is a directory", cmd, quote(f)));
                    status = 1;
                    continue;
                }
            };
            let range = match (cmd, count.sign) {
                // head -n N: the first N lines; head -n -N: all but the last N.
                ("head", Some(b'-')) => 0..first_lines(data, line_count(data).saturating_sub(count.n)),
                ("head", _) => 0..first_lines(data, count.n),
                // tail -n +N: from line N on; tail -n N: the last N lines.
                (_, Some(b'+')) => first_lines(data, count.n.saturating_sub(1))..data.len(),
                _ => last_lines(data, count.n)..data.len(),
            };
            io.put(&data[range]);
        }
        status
    });
    Ok(Prepared { run, input })
}

pub fn head(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    head_or_tail("head", args)
}

pub fn tail(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    head_or_tail("tail", args)
}

#[derive(Default)]
struct Counter {
    counts: [usize; 3],
    in_word: bool,
}

impl Counter {
    fn add(&mut self, data: &[u8]) {
        self.counts[2] += data.len();
        for &c in data {
            if c == b'\n' {
                self.counts[0] += 1;
            }
            // In the C locale a word is started by a printable character
            // and ended by white space; other bytes do neither.
            if matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                self.in_word = false;
            } else if (0x21..0x7f).contains(&c) && !self.in_word {
                self.in_word = true;
                self.counts[1] += 1;
            }
        }
    }
}

fn wc_line(io: &mut Io, show: [bool; 3], width: usize, counts: [usize; 3], name: Option<&[u8]>) {
    let cols: Vec<String> = (0..3).filter(|i| show[*i]).map(|i| format!("{:>w$}", counts[i], w = width)).collect();
    io.put(cols.join(" ").as_bytes());
    if let Some(n) = name {
        io.put(b" ");
        io.put(n);
    }
    io.put(b"\n");
}

/// `wc` on a stream: only the counts are kept.
struct WcFilter {
    show: [bool; 3],
    width: usize,
    counter: Counter,
}

impl Filter for WcFilter {
    fn push(&mut self, data: &[u8], _io: &mut Io) -> bool {
        self.counter.add(data);
        true
    }

    fn finish(self: Box<Self>, io: &mut Io) -> i32 {
        wc_line(io, self.show, self.width, self.counter.counts, None);
        0
    }
}

pub fn wc(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let p = match options("wc", &args, b"lwc", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let all = p.flags.is_empty();
    let show = [all || p.has(b'l'), all || p.has(b'w'), all || p.has(b'c')];
    let fields = show.iter().filter(|s| **s).count();
    let from_stdin = p.operands.is_empty();
    let input = if from_stdin {
        // Standard input is not a regular file, so the columns are at
        // least 7 wide unless there is only one.
        Input::Stream(Box::new(WcFilter { show, width: if fields == 1 { 1 } else { 7 }, counter: Counter::default() }))
    } else if p.operands.iter().any(|f| f == b"-") {
        Input::Whole
    } else {
        Input::Unused
    };
    let files = operands_or_stdin(p.operands);
    let run: Runner = Box::new(move |db, io| {
        // Column width, as GNU wc computes it: wide enough for the total
        // size of the regular files, and at least 7 if any input is not a
        // regular file. One file and one count needs no padding.
        let mut width = 1;
        if !(files.len() == 1 && fields == 1) {
            let mut regular_total = 0usize;
            let mut minimum = 1;
            for f in &files {
                match db.resolve(f) {
                    _ if f == b"-" => minimum = 7,
                    Ok(id) if db.is_dir(id) => minimum = 7,
                    Ok(id) => regular_total += db.content(id).len(),
                    Err(_) => {}
                }
            }
            width = regular_total.to_string().len().max(minimum);
        }
        let mut status = 0;
        let mut totals = [0usize; 3];
        for f in &files {
            let data: &[u8] = match open(db, if f == b"-" { io.take_stdin() } else { b"" }, f) {
                Source::Data(d) => d,
                Source::Missing(e) => {
                    io.err_line(format!("wc: {}: {}", quote_if_needed(f), e.msg()));
                    status = 1;
                    continue;
                }
                Source::Directory => {
                    io.err_line(format!("wc: {}: Is a directory", quote_if_needed(f)));
                    status = 1;
                    b""
                }
            };
            let mut counter = Counter::default();
            counter.add(data);
            for i in 0..3 {
                totals[i] += counter.counts[i];
            }
            wc_line(io, show, width, counter.counts, if from_stdin { None } else { Some(f) });
        }
        if files.len() > 1 {
            wc_line(io, show, width, totals, Some(b"total"));
        }
        status
    });
    Ok(Prepared { run, input })
}
