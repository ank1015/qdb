//! echo, cat, head, tail, wc.

use super::{msg, options, LineBuf};
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use std::collections::VecDeque;
use crate::tree::{Db, Errno, NodeId};
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
    /// A file: read it a piece at a time with `Db::pieces`.
    File(NodeId),
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
        Ok(id) => Source::File(id),
    }
}

fn operands_or_stdin(operands: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    if operands.is_empty() {
        vec![b"-".to_vec()]
    } else {
        operands
    }
}

/// What cat does to its input besides copying it.
#[derive(Clone, Copy, Default)]
struct CatOpts {
    /// -n
    number: bool,
    /// -b: number lines that are not empty; overrides -n.
    number_nonblank: bool,
    /// -E: `$` at the end of each line.
    ends: bool,
    /// -s: runs of empty lines become one.
    squeeze: bool,
    /// -T: tabs as `^I`.
    tabs: bool,
    /// -v: control characters as `^X`, bytes above 127 as `M-`.
    nonprinting: bool,
}

impl CatOpts {
    fn plain(&self) -> bool {
        !(self.number || self.number_nonblank || self.ends || self.squeeze || self.tabs || self.nonprinting)
    }
}

/// cat's state, which runs on across files: the line number, whether the
/// last byte ended a line, and whether the last line was empty.
struct CatLines {
    opts: CatOpts,
    line_no: u64,
    mid_line: bool,
    after_blank: bool,
    out: Vec<u8>,
}

impl CatLines {
    fn new(opts: CatOpts) -> CatLines {
        CatLines { opts, line_no: 0, mid_line: false, after_blank: false, out: Vec::new() }
    }

    fn put(&mut self, data: &[u8], io: &mut Io) {
        if self.opts.plain() {
            io.put(data);
            return;
        }
        let o = self.opts;
        for line in crate::util::lines(data) {
            let ended = line.ends_with(b"\n");
            if !self.mid_line {
                let blank = line == b"\n";
                if blank && o.squeeze && self.after_blank {
                    continue;
                }
                self.after_blank = blank;
                if o.number_nonblank && !blank || o.number && !o.number_nonblank {
                    self.line_no += 1;
                    self.out.extend_from_slice(format!("{:>6}\t", self.line_no).as_bytes());
                }
            }
            let mut body = if ended { &line[..line.len() - 1] } else { line };
            // A carriage return before the end of the line shows with -E.
            let cr = ended && o.ends && body.ends_with(b"\r");
            if cr {
                body = &body[..body.len() - 1];
            }
            if o.tabs || o.nonprinting {
                for &c in body {
                    match c {
                        b'\t' if o.tabs => self.out.extend_from_slice(b"^I"),
                        b'\t' => self.out.push(c),
                        _ if !o.nonprinting => self.out.push(c),
                        0..=31 => self.out.extend_from_slice(&[b'^', c + 64]),
                        127 => self.out.extend_from_slice(b"^?"),
                        128..=255 => {
                            self.out.extend_from_slice(b"M-");
                            match c - 128 {
                                d @ 0..=31 => self.out.extend_from_slice(&[b'^', d + 64]),
                                127 => self.out.extend_from_slice(b"^?"),
                                d => self.out.push(d),
                            }
                        }
                        _ => self.out.push(c),
                    }
                }
            } else {
                self.out.extend_from_slice(body);
            }
            if cr {
                self.out.extend_from_slice(b"^M");
            }
            if ended {
                if o.ends {
                    self.out.push(b'$');
                }
                self.out.push(b'\n');
            }
            self.mid_line = !ended;
            if self.out.len() > 8192 {
                io.put(&self.out);
                self.out.clear();
            }
        }
        io.put(&self.out);
        self.out.clear();
    }
}

struct CatFilter {
    lines: CatLines,
}

impl Filter for CatFilter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        self.lines.put(data, io);
        true
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }
}

pub fn cat(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let p = match options("cat", &args, b"nbEsTvAetu", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let mut opts = CatOpts::default();
    for (f, _) in &p.flags {
        match f {
            b'n' => opts.number = true,
            b'b' => opts.number_nonblank = true,
            b'E' => opts.ends = true,
            b's' => opts.squeeze = true,
            b'T' => opts.tabs = true,
            b'v' => opts.nonprinting = true,
            b'A' => (opts.nonprinting, opts.ends, opts.tabs) = (true, true, true),
            b'e' => (opts.nonprinting, opts.ends) = (true, true),
            b't' => (opts.nonprinting, opts.tabs) = (true, true),
            // -u: output is never held back here.
            _ => {}
        }
    }
    let input = match p.operands.as_slice() {
        [] => Input::Stream(Box::new(CatFilter { lines: CatLines::new(opts) })),
        ops if ops.iter().any(|f| f == b"-") => Input::Whole,
        _ => Input::Unused,
    };
    let files = operands_or_stdin(p.operands);
    let run: Runner = Box::new(move |db, io| {
        let mut status = 0;
        let mut lines = CatLines::new(opts);
        for f in &files {
            if io.stopped() {
                break;
            }
            if f != b"-" {
                // GNU cat refuses to copy a non-empty file onto itself.
                if let (Ok(id), Some(o)) = (db.resolve(f), io.out_file) {
                    if id == o && !db.is_dir(id) && db.size(id) > 0 {
                        io.err_line(format!("cat: {}: input file is output file", quote_if_needed(f)));
                        status = 1;
                        continue;
                    }
                }
            }
            match open(db, if f == b"-" { io.take_stdin() } else { b"" }, f) {
                Source::Data(d) => lines.put(d, io),
                Source::File(id) => {
                    for p in db.pieces(id) {
                        if io.stopped() {
                            break;
                        }
                        lines.put(&db.text_of(p), io);
                    }
                }
                Source::Missing(e) => {
                    io.err_line(format!("cat: {}: {}", quote_if_needed(f), e.msg()));
                    status = 1;
                }
                Source::Directory => {
                    io.err_line(format!("cat: {}: Is a directory", quote_if_needed(f)));
                    status = 1;
                }
            }
        }
        status
    });
    Ok(Prepared { run, input })
}

/// A count argument for head or tail: an optional sign, digits, and an
/// optional multiplier such as `K` or `MiB`.
#[derive(Clone, Copy)]
struct Count {
    sign: Option<u8>,
    n: usize,
}

/// The multiplier a size suffix stands for: `b` is 512, `K` and `KiB` are
/// 1,024, `KB` is 1,000, and so on up.
pub(crate) fn size_suffix(suffix: &[u8]) -> Option<u64> {
    if suffix.is_empty() {
        return Some(1);
    }
    if suffix == b"b" {
        return Some(512);
    }
    let power = 1 + b"KMGTPE".iter().position(|c| *c == suffix[0] || suffix[0] == b'k' && *c == b'K')? as u32;
    match &suffix[1..] {
        b"" | b"iB" => 1024u64.checked_pow(power),
        b"B" => 1000u64.checked_pow(power),
        _ => None,
    }
}

fn parse_count(cmd: &'static str, what: &'static str, arg: Option<&[u8]>) -> Result<Count, Runner> {
    let arg = match arg {
        None => return Ok(Count { sign: None, n: 10 }),
        Some(a) => a.to_vec(),
    };
    let (sign, rest) = match arg.first() {
        Some(c @ (b'+' | b'-')) => (Some(*c), &arg[1..]),
        _ => (None, &arg[..]),
    };
    let shown: Vec<u8> = if sign == Some(b'-') { rest.to_vec() } else { arg.clone() };
    let split = rest.iter().position(|c| !c.is_ascii_digit()).unwrap_or(rest.len());
    let (digits, suffix) = rest.split_at(split);
    if let (false, Some(mult)) = (digits.is_empty(), size_suffix(suffix)) {
        let n = digits.iter().try_fold(0u64, |n, c| n.checked_mul(10)?.checked_add((c - b'0') as u64)).and_then(|n| n.checked_mul(mult));
        return match n {
            Some(n) => Ok(Count { sign, n: usize::try_from(n).unwrap_or(usize::MAX) }),
            None => Err(Box::new(move |_, io: &mut Io| {
                io.err_line(format!("{}: invalid number of {}: {}: Value too large for defined data type", cmd, what, quote(&shown)));
                1
            })),
        };
    }
    Err(Box::new(move |_, io: &mut Io| {
        io.err_line(format!("{}: invalid number of {}: {}", cmd, what, quote(&shown)));
        1
    }))
}

/// `head -c` or `tail -c` of a file: whole pieces are skipped unread.
fn file_bytes(db: &Db, io: &mut Io, id: NodeId, head: bool, sign: Option<u8>, n: usize) {
    let size = db.size(id) as usize;
    let range = match (head, sign) {
        (true, Some(b'-')) => 0..size.saturating_sub(n),
        (true, _) => 0..n.min(size),
        (false, Some(b'+')) => n.saturating_sub(1).min(size)..size,
        (false, _) => size.saturating_sub(n)..size,
    };
    let mut at = 0;
    for p in db.pieces(id) {
        let end = at + p.len as usize;
        if at >= range.end || io.stopped() {
            break;
        }
        if end > range.start {
            let text = db.text_of(p);
            io.put(&text[range.start.saturating_sub(at)..range.end.min(end) - at]);
        }
        at = end;
    }
}

/// `head` or `tail` of a file, a piece at a time: pieces end on line
/// boundaries and say how many lines they hold, so only the pieces that
/// hold the edge of what is wanted are looked into.
fn file_lines(db: &Db, io: &mut Io, id: NodeId, head: bool, sign: Option<u8>, n: usize) {
    let pieces = db.pieces(id);
    // A final line counts whether or not it ends with a newline.
    let open_end = |db: &Db| pieces.last().is_some_and(|p| !db.text_of(p).ends_with(b"\n"));
    let units = |i: usize, db: &Db| pieces[i].lines as usize + usize::from(i + 1 == pieces.len() && open_end(db));
    if head {
        let mut left = match sign {
            Some(b'-') => (db.line_count(id) as usize + usize::from(open_end(db))).saturating_sub(n),
            _ => n,
        };
        for p in pieces {
            if left == 0 || io.stopped() {
                break;
            }
            let text = db.text_of(p);
            if left > p.lines as usize {
                io.put(&text);
                left -= p.lines as usize;
            } else {
                io.put(&text[..first_lines(&text, left)]);
                left = 0;
            }
        }
        return;
    }
    let (mut start, mut off) = (0, 0);
    if sign == Some(b'+') {
        let mut skip = n.saturating_sub(1);
        while start < pieces.len() && skip > pieces[start].lines as usize {
            skip -= pieces[start].lines as usize;
            start += 1;
        }
        if start < pieces.len() {
            off = first_lines(&db.text_of(&pieces[start]), skip);
        }
    } else {
        let mut after = 0usize;
        start = pieces.len();
        while start > 0 {
            if after >= n {
                break;
            }
            start -= 1;
            let here = units(start, db);
            if after + here >= n {
                off = last_lines(&db.text_of(&pieces[start]), n - after);
                break;
            }
            after += here;
        }
        if n == 0 {
            start = pieces.len();
        }
    }
    for (i, p) in pieces.iter().enumerate().skip(start) {
        if io.stopped() {
            break;
        }
        let text = db.text_of(p);
        io.put(&text[if i == start { off } else { 0 }..]);
    }
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

/// `head -c N` on a stream.
struct HeadBytes {
    left: usize,
}

impl Filter for HeadBytes {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        let n = self.left.min(data.len());
        io.put(&data[..n]);
        self.left -= n;
        self.left > 0
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }
}

/// `tail -c +N` on a stream.
struct SkipBytes {
    skip: usize,
}

impl Filter for SkipBytes {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        let n = self.skip.min(data.len());
        self.skip -= n;
        io.put(&data[n..]);
        true
    }

    fn finish(self: Box<Self>, _io: &mut Io) -> i32 {
        0
    }
}

/// `tail -c N` (pass the last N bytes when the input ends) and
/// `head -c -N` (pass everything but the last N bytes, as it arrives) on
/// a stream: the last N bytes seen are all that is kept.
struct LastBytes {
    keep: usize,
    held: VecDeque<u8>,
    /// True for head: what falls out of the window is passed on.
    pass: bool,
}

impl Filter for LastBytes {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        self.held.extend(data);
        if self.held.len() > self.keep {
            let out = self.held.len() - self.keep;
            if self.pass {
                let (a, b) = self.held.as_slices();
                io.put(&a[..out.min(a.len())]);
                if out > a.len() {
                    io.put(&b[..out - a.len()]);
                }
            }
            self.held.drain(..out);
        }
        true
    }

    fn finish(self: Box<Self>, io: &mut Io) -> i32 {
        if !self.pass {
            let (a, b) = self.held.as_slices();
            io.put(a);
            io.put(b);
        }
        0
    }
}

/// `head -n -N` on a stream: a line is passed on once N more have come.
struct DropLastLines {
    keep: usize,
    lines: VecDeque<Vec<u8>>,
    buf: LineBuf,
}

impl Filter for DropLastLines {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        let (keep, lines) = (self.keep, &mut self.lines);
        self.buf.feed(data, |line| {
            lines.push_back(line.to_vec());
            if lines.len() > keep {
                io.put(&lines.pop_front().unwrap());
            }
            true
        })
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if let Some(last) = self.buf.rest() {
            self.lines.push_back(last);
        }
        while self.lines.len() > self.keep {
            io.put(&self.lines.pop_front().unwrap());
        }
        0
    }
}

fn head_or_tail(cmd: &'static str, mut args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    // GNU tail still reads a leading +N as the obsolete form of -n +N.
    if cmd == "tail" && args.first().is_some_and(|a| a.len() > 1 && a[0] == b'+' && a[1..].iter().all(|c| c.is_ascii_digit())) {
        return Err("flag: tail +N (write tail -n +N)".into());
    }
    // -NUM is -n NUM.
    let mut old_form = None;
    if let Some(first) = args.first_mut() {
        if first.len() > 1 && first[0] == b'-' && first[1..].iter().all(|c| c.is_ascii_digit()) {
            old_form = Some(first[1]);
            let mut n = b"-n".to_vec();
            n.extend_from_slice(&first[1..]);
            *first = n;
        }
    }
    let p = match options(cmd, &args, b"qv", b"nc", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    if let (Some(digit), false, true) = (old_form, cmd == "head", p.operands.len() > 1) {
        // tail takes the old form with one file at most.
        return Ok(Prepared::from(Box::new(move |_: &mut Db, io: &mut Io| {
            io.err_line(format!("tail: option used in invalid context -- {}", char::from(digit)));
            1
        }) as Runner));
    }
    // Of -n and -c, and of -q and -v, the later one decides.
    let unit = p.flags.iter().rev().find(|(f, _)| matches!(f, b'n' | b'c'));
    let bytes = unit.is_some_and(|(f, _)| *f == b'c');
    let count = match parse_count(cmd, if bytes { "bytes" } else { "lines" }, unit.and_then(|(_, a)| a.as_deref())) {
        Ok(c) => c,
        Err(r) => return Ok(r.into()),
    };
    let headers = p.flags.iter().rev().find(|(f, _)| matches!(f, b'q' | b'v')).map(|(f, _)| *f == b'v');
    let head = cmd == "head";
    let input = if !p.operands.is_empty() {
        if p.operands.iter().any(|f| f == b"-") {
            Input::Whole
        } else {
            Input::Unused
        }
    } else if headers == Some(true) {
        Input::Whole
    } else {
        let f: Box<dyn Filter> = match (head, bytes, count.sign) {
            (true, false, Some(b'-')) => Box::new(DropLastLines { keep: count.n, lines: VecDeque::new(), buf: LineBuf::default() }),
            (true, false, _) => Box::new(HeadFilter { left: count.n }),
            (true, true, Some(b'-')) => Box::new(LastBytes { keep: count.n, held: VecDeque::new(), pass: true }),
            (true, true, _) => Box::new(HeadBytes { left: count.n }),
            (false, false, Some(b'+')) => Box::new(TailFromFilter { skip: count.n.saturating_sub(1) }),
            (false, false, _) => Box::new(TailFilter { keep: count.n, lines: VecDeque::new(), buf: LineBuf::default() }),
            (false, true, Some(b'+')) => Box::new(SkipBytes { skip: count.n.saturating_sub(1) }),
            (false, true, _) => Box::new(LastBytes { keep: count.n, held: VecDeque::new(), pass: false }),
        };
        Input::Stream(f)
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
            if headers.unwrap_or(files.len() > 1) {
                if !first_header {
                    io.put(b"\n");
                }
                first_header = false;
                let shown: &[u8] = if f == b"-" { b"standard input" } else { f };
                io.put(&msg(&[b"==> ", shown, b" <==\n"]));
            }
            let data = match input {
                Source::Data(d) => d,
                Source::File(id) => {
                    if bytes {
                        file_bytes(db, io, id, head, count.sign, count.n);
                    } else {
                        file_lines(db, io, id, head, count.sign, count.n);
                    }
                    continue;
                }
                _ => {
                    io.err_line(format!("{}: error reading {}: Is a directory", cmd, quote(f)));
                    status = 1;
                    continue;
                }
            };
            let range = match (head, bytes, count.sign) {
                // head -n N: the first N lines; head -n -N: all but the last N.
                (true, false, Some(b'-')) => 0..first_lines(data, line_count(data).saturating_sub(count.n)),
                (true, false, _) => 0..first_lines(data, count.n),
                // tail -n +N: from line N on; tail -n N: the last N lines.
                (false, false, Some(b'+')) => first_lines(data, count.n.saturating_sub(1))..data.len(),
                (false, false, _) => last_lines(data, count.n)..data.len(),
                (true, true, Some(b'-')) => 0..data.len().saturating_sub(count.n),
                (true, true, _) => 0..count.n.min(data.len()),
                (false, true, Some(b'+')) => count.n.saturating_sub(1).min(data.len())..data.len(),
                (false, true, _) => data.len().saturating_sub(count.n)..data.len(),
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

/// One line of counts. The columns are lines, words, characters and
/// bytes; in the C locale a character is a byte.
fn wc_line(io: &mut Io, show: [bool; 4], width: usize, counts: [usize; 3], name: Option<&[u8]>) {
    let cols: Vec<String> = (0..4).filter(|i| show[*i]).map(|i| format!("{:>w$}", counts[i.min(2)], w = width)).collect();
    io.put(cols.join(" ").as_bytes());
    if let Some(n) = name {
        io.put(b" ");
        io.put(n);
    }
    io.put(b"\n");
}

/// `wc` on a stream: only the counts are kept.
struct WcFilter {
    show: [bool; 4],
    width: usize,
    counter: Counter,
}

impl Filter for WcFilter {
    fn push(&mut self, data: &[u8], _io: &mut Io) -> bool {
        self.counter.add(data);
        true
    }

    fn finish(self: Box<Self>, io: &mut Io) -> i32 {
        // Read from a file with `<`, the columns are as wide as its size
        // needs, as for a file that is named.
        let width = match io.stdin_size() {
            Some(size) if self.width > 1 => size.to_string().len(),
            _ => self.width,
        };
        wc_line(io, self.show, width, self.counter.counts, None);
        0
    }
}

pub fn wc(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let p = match options("wc", &args, b"lwcm", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let all = p.flags.is_empty();
    let show = [all || p.has(b'l'), all || p.has(b'w'), p.has(b'm'), all || p.has(b'c')];
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
                    Ok(id) => regular_total += db.size(id) as usize,
                    Err(_) => {}
                }
            }
            width = regular_total.to_string().len().max(minimum);
        }
        let mut status = 0;
        let mut totals = [0usize; 3];
        for f in &files {
            if f.is_empty() {
                io.err_line("wc: invalid zero-length file name");
                status = 1;
                continue;
            }
            let mut counter = Counter::default();
            let data: &[u8] = match open(db, if f == b"-" { io.take_stdin() } else { b"" }, f) {
                Source::Data(d) => d,
                Source::File(id) => {
                    if show[1] {
                        for p in db.pieces(id) {
                            counter.add(&db.text_of(p));
                        }
                    } else {
                        // Lines and bytes are known without reading.
                        counter.counts[0] = db.line_count(id) as usize;
                        counter.counts[2] = db.size(id) as usize;
                    }
                    b""
                }
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
