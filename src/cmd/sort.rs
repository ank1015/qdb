//! sort. Lines are gathered into runs; a run that has grown past what
//! may be held is sorted and written to a scratch file, and the runs are
//! merged at the end, so input of any size takes a fixed amount of memory.

use super::text::fails;
use super::{options_long, LineBuf};
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use crate::tree::Scratch;
use crate::util::{quote, quote_if_needed, LongOpt};
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
        self.by_keys_from(a, b, 0)
    }

    fn by_keys_from(&self, a: &[u8], b: &[u8], from: usize) -> Ordering {
        for k in &self.keys[from..] {
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

impl Opts {
    /// Whether what is compared first is compared as bytes, so that its
    /// first eight can stand in for it: the whole line, or a first key
    /// with no ordering letters.
    fn bytes_first(&self) -> bool {
        self.keys.first().is_none_or(|k| !k.o.numeric && !k.o.human && !k.o.fold)
    }

    /// What is compared first in a line: its place in the line and its
    /// first bytes.
    fn head(&self, line: &[u8]) -> (u64, u32, u32) {
        match self.keys.first() {
            None => (prefix(line), 0, line.len() as u32),
            // A whole number of up to eighteen digits is kept as a
            // number (marked by the 1); anything else is read again at
            // every comparison.
            Some(k) if k.o.numeric && !k.o.human => {
                let (neg, int, frac) = number(self.field(line, k));
                if frac.is_empty() && int.len() <= 18 {
                    let v = int.iter().fold(0i64, |v, d| v * 10 + i64::from(d - b'0'));
                    ((if neg { -v } else { v }) as u64 ^ (1 << 63), 1, 0)
                } else {
                    (0, 0, 0)
                }
            }
            Some(k) if self.bytes_first() => {
                let f = self.field(line, k);
                let off = f.as_ptr() as usize - line.as_ptr() as usize;
                (prefix(f), off as u32, f.len() as u32)
            }
            Some(_) => (0, 0, 0),
        }
    }

    /// The order of two lines whose heads are known.
    #[inline]
    fn compare_headed(&self, a: &[u8], ha: (u64, u32, u32), b: &[u8], hb: (u64, u32, u32)) -> Ordering {
        if self.keys.is_empty() {
            let ord = ha.0.cmp(&hb.0).then_with(|| a.cmp(b));
            return if self.reverse { ord.reverse() } else { ord };
        }
        let k = &self.keys[0];
        let ord = if !self.bytes_first() {
            if !(k.o.numeric && !k.o.human && ha.1 == 1 && hb.1 == 1) {
                return self.compare(a, b);
            }
            ha.0.cmp(&hb.0)
        } else {
            ha.0.cmp(&hb.0).then_with(|| a[ha.1 as usize..(ha.1 + ha.2) as usize].cmp(&b[hb.1 as usize..(hb.1 + hb.2) as usize]))
        };
        if ord != Ordering::Equal {
            return if k.o.reverse { ord.reverse() } else { ord };
        }
        let ord = self.by_keys_from(a, b, 1);
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

/// One line of the run being gathered: where it is, and the first bytes
/// of what is compared first, to settle most comparisons without looking
/// at the line.
#[derive(Clone, Copy)]
struct Ent {
    prefix: u64,
    off: u32,
    len: u32,
    koff: u32,
    klen: u32,
}

fn prefix(line: &[u8]) -> u64 {
    let mut b = [0u8; 8];
    let n = line.len().min(8);
    b[..n].copy_from_slice(&line[..n]);
    u64::from_be_bytes(b)
}

/// Sort by a string of bytes in each line: the whole line, or its first
/// key. Lines that begin alike are common (indented text, numbered
/// lines, a key with few values), and a comparison sort reads the shared
/// start again at every comparison. This one looks at each string eight
/// bytes at a time: it splits a group three ways on those eight bytes,
/// and only the strings equal there go on to their next eight.
///
/// On entry every `prefix` holds the first eight bytes of its string.
/// `tie` orders two lines whose strings are equal.
fn by_bytes(ents: &mut [Ent], buf: &[u8], whole: bool, tie: &dyn Fn(&Ent, &Ent) -> Ordering) {
    let text = |e: &Ent| -> &[u8] {
        let from = e.off as usize + if whole { 0 } else { e.koff as usize };
        &buf[from..from + if whole { e.len } else { e.klen } as usize]
    };
    let size = |e: &Ent| if whole { e.len } else { e.klen } as usize;
    // Two strings equal in their first `depth` bytes, and all that follows.
    let full = |a: &Ent, b: &Ent, depth: usize| {
        a.prefix.cmp(&b.prefix).then_with(|| {
            let (x, y) = (text(a), text(b));
            x[depth.min(x.len())..].cmp(&y[depth.min(y.len())..]).then_with(|| tie(a, b))
        })
    };
    // (start, end, bytes already known to be equal within the group)
    let mut todo: Vec<(usize, usize, usize)> = vec![(0, ents.len(), 0)];
    let mut pops = 0u32;
    while let Some((start, end, depth)) = todo.pop() {
        let part = &mut ents[start..end];
        let n = part.len();
        // A long sort looks now and then for a request to stop: before
        // every large group, and every so many small ones.
        pops = pops.wrapping_add(1);
        if n > 4096 || pops & 255 == 0 {
            crate::unit::poll();
        }
        if n <= 32 {
            let mut n = 0u32;
            part.sort_unstable_by(|a, b| {
                n = n.wrapping_add(1);
                if n & 0xffff == 0 {
                    crate::unit::poll();
                }
                full(a, b, depth)
            });
            continue;
        }
        let pivot = {
            let (a, b, c) = (part[n / 4].prefix, part[n / 2].prefix, part[n - 1 - n / 4].prefix);
            a.max(b).min(a.min(b).max(c))
        };
        let (mut less, mut i, mut more) = (0, 0, n);
        while i < more {
            match part[i].prefix.cmp(&pivot) {
                Ordering::Less => {
                    part.swap(less, i);
                    less += 1;
                    i += 1;
                }
                Ordering::Greater => {
                    more -= 1;
                    part.swap(i, more);
                }
                Ordering::Equal => i += 1,
            }
        }
        if less > 1 {
            todo.push((start, start + less, depth));
        }
        if n - more > 1 {
            todo.push((start + more, end, depth));
        }
        // The strings equal on these eight bytes. Those that end within
        // them come first, the shorter before the longer; the others go
        // on to their next eight.
        let same = &mut part[less..more];
        let mut ended = 0;
        for i in 0..same.len() {
            if size(&same[i]) <= depth + 8 {
                same.swap(i, ended);
                ended += 1;
            }
        }
        same[..ended].sort_unstable_by(|a, b| size(a).cmp(&size(b)).then_with(|| tie(a, b)));
        for e in &mut same[ended..] {
            let from = e.off as usize + if whole { 0 } else { e.koff as usize };
            let to = from + if whole { e.len } else { e.klen } as usize;
            e.prefix = prefix(&buf[from + depth + 8..to]);
        }
        if same.len() - ended > 1 {
            todo.push((start + less + ended, start + more, depth + 8));
        }
    }
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
    /// No line of the run in hand has come before one it belongs after.
    in_order: bool,
}

/// With somewhere to write runs, a run is this many times what output
/// bound for a file may hold in memory: 96 MB as the engine is set up.
const RUN_SPOOLS: usize = 12;

impl Sorter {
    fn new(o: Opts) -> Sorter {
        Sorter { o, buf: Vec::new(), ents: Vec::new(), lines: LineBuf::default(), runs: Vec::new(), file: None, scratch: None, written: 0, failed: None, in_order: true }
    }

    fn add(&mut self, line: &[u8]) {
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        if self.buf.len() + text.len() > u32::MAX as usize || self.scratch.as_ref().is_some_and(|s| s.vfs.is_some() && self.buf.len() + self.ents.len() * std::mem::size_of::<Ent>() >= s.limit.saturating_mul(RUN_SPOOLS)) {
            self.spill();
        }
        let (prefix, koff, klen) = self.o.head(text);
        if self.in_order {
            if let Some(last) = self.ents.last() {
                let before = &self.buf[last.off as usize..last.off as usize + last.len as usize];
                self.in_order = self.o.compare_headed(before, (last.prefix, last.koff, last.klen), text, (prefix, koff, klen)) != Ordering::Greater;
            }
        }
        self.ents.push(Ent { prefix, off: self.buf.len() as u32, len: text.len() as u32, koff, klen });
        self.buf.extend_from_slice(text);
    }

    fn sort_run(&mut self) {
        // Input that came in order is common (a file sorted before, or
        // numbered lines), and was seen to be as it came.
        if self.in_order {
            return;
        }
        let buf = &self.buf;
        let o = &self.o;
        let text = |e: &Ent| &buf[e.off as usize..e.off as usize + e.len as usize];
        if o.keys.is_empty() {
            by_bytes(&mut self.ents, buf, true, &|_, _| Ordering::Equal);
            if o.reverse {
                self.ents.reverse();
            }
        } else if o.bytes_first() && !o.keys[0].o.reverse {
            // Lines with the same first key: by the other keys, then as
            // they came (for -s and -u) or by the whole line.
            let tie = |a: &Ent, b: &Ent| {
                let (x, y) = (text(a), text(b));
                let ord = o.by_keys_from(x, y, 1);
                if ord != Ordering::Equal {
                    ord
                } else if o.unique || o.stable {
                    // An empty line starts where the line after it does,
                    // and came first.
                    a.off.cmp(&b.off).then(a.len.cmp(&b.len))
                } else if o.reverse {
                    y.cmp(x)
                } else {
                    x.cmp(y)
                }
            };
            by_bytes(&mut self.ents, buf, false, &tie);
        } else {
            // A long sort looks now and then for a request to stop.
            let mut n = 0u32;
            self.ents.sort_by(|a, b| {
                n = n.wrapping_add(1);
                if n & 0xffff == 0 {
                    crate::unit::poll();
                }
                o.compare_headed(text(a), (a.prefix, a.koff, a.klen), text(b), (b.prefix, b.koff, b.klen))
            });
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
        if self.scratch.is_some() {
            crate::tree::count_scratch(self.written - start);
        }
        self.runs.push((start, self.written));
        self.buf.clear();
        self.ents.clear();
        self.in_order = true;
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
        let o = self.o.clone();
        // Safety: as above; the slices do not outlive this call.
        let all: &'static [u8] = unsafe { std::mem::transmute::<&[u8], &'static [u8]>(all) };
        // Runs that follow one another in order are one run: input that
        // came in order is passed on as it was written.
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for (s, e) in self.runs.iter().map(|(s, e)| (*s as usize, *e as usize)).filter(|(s, e)| e > s) {
            if let Some(prev) = runs.last_mut() {
                let body = &all[prev.0..prev.1 - 1];
                let last_line = &body[memchr::memrchr(b'\n', body).map_or(0, |i| i + 1)..];
                let first_line = next_line(&all[s..e]).map_or(&b""[..], |l| l.0);
                if o.compare(last_line, first_line) != Ordering::Greater {
                    prev.1 = e;
                    continue;
                }
            }
            runs.push((s, e));
        }
        if runs.len() == 1 && !o.unique {
            for chunk in all[runs[0].0..runs[0].1].chunks(1 << 16) {
                io.put(chunk);
                if io.stopped() {
                    break;
                }
            }
            return 0;
        }
        // The next line of each run, and the lines after it.
        type Head = Option<(&'static [u8], &'static [u8], (u64, u32, u32))>;
        let headed = |h: Option<(&'static [u8], &'static [u8])>| -> Head { h.map(|(line, rest)| (line, rest, o.head(line))) };
        let mut heads: Vec<Head> = runs.iter().map(|(s, e)| headed(next_line(&all[*s..*e]))).collect();
        // A heap of the runs by their next line; of equal lines, the one
        // of the earliest run first.
        let before = |heads: &[Head], i: usize, j: usize| -> bool {
            let ((a, _, ha), (b, _, hb)) = (heads[i].unwrap(), heads[j].unwrap());
            match o.compare_headed(a, ha, b, hb) {
                Ordering::Less => true,
                Ordering::Greater => false,
                Ordering::Equal => i < j,
            }
        };
        let sift = |heap: &mut [usize], heads: &[Head], mut at: usize| loop {
            let left = 2 * at + 1;
            if left >= heap.len() {
                break;
            }
            let mut least = left;
            if left + 1 < heap.len() && before(heads, heap[left + 1], heap[left]) {
                least = left + 1;
            }
            if !before(heads, heap[least], heap[at]) {
                break;
            }
            heap.swap(least, at);
            at = least;
        };
        let mut heap: Vec<usize> = (0..heads.len()).filter(|i| heads[*i].is_some()).collect();
        for at in (0..heap.len() / 2).rev() {
            sift(&mut heap, &heads, at);
        }
        let mut last: Vec<u8> = Vec::new();
        let mut any = false;
        while let Some(&b) = heap.first() {
            let (line, rest, _) = heads[b].unwrap();
            heads[b] = headed(next_line(rest));
            if heads[b].is_none() {
                let moved = heap.pop().unwrap();
                if !heap.is_empty() {
                    heap[0] = moved;
                }
            }
            sift(&mut heap, &heads, 0);
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
                    io.err_line(format!("sort: open failed: {}: {}", quote_if_needed(&c.name), e.msg()));
                    2
                }
                Ok(id) if db.is_dir(id) => {
                    io.err_line(format!("sort: read failed: {}: Is a directory", quote_if_needed(&c.name)));
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
        return Ok(Prepared { run, input, writes: false });
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
                    io.err_line(format!("sort: cannot read: {}: {}", quote_if_needed(f), e.msg()));
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
                    io.err_line(format!("sort: read failed: {}: Is a directory", quote_if_needed(f)));
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
    Ok(Prepared { run, input, writes: false })
}
