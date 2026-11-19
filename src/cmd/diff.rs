//! diff, as GNU diffutils 3.10 prints it: the default format and the
//! unified one, over two files or two trees.
//!
//! The edit script is GNU's: the same lines set aside before the
//! comparison, the same search for a middle, the same sliding of each
//! change afterwards. What is the engine's own is how little is read.
//! Two files that are the same pieces are equal without a byte of them
//! read; pieces two files share at their start and at their end are
//! stepped over, their lines counted from what is kept about each piece;
//! only what lies between is read and compared.

use crate::disk::Piece;
use crate::engine::{Input, Io, Prepared, Runner};
use crate::glob;
use crate::tree::{Db, NodeId};
use crate::util::lossy;
use crate::when;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;

/// How much may lie between the equal ends of two files: the text of
/// both, and what the comparison keeps for each line.
const MIDDLE_LIMIT: u64 = 256 << 20;
/// What the comparison keeps for a line, roughly, in bytes.
const PER_LINE: u64 = 96;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
enum Space {
    Kept,
    /// `-b`: a run of white space is one space, and none at the end.
    Change,
    /// `-w`: no white space at all.
    All,
}

struct Opts {
    /// Lines of context, for the unified format.
    unified: Option<usize>,
    recursive: bool,
    new_file: bool,
    excludes: Vec<Vec<glob::Tok>>,
    brief: bool,
    report_same: bool,
    icase: bool,
    space: Space,
    blank: bool,
    labels: Vec<Vec<u8>>,
    /// The options as they were written, for the line printed before
    /// each pair of files of two trees.
    switches: Vec<u8>,
}

fn usage(message: String) -> Prepared {
    let run: Runner = Box::new(move |_, io| {
        io.err_line(format!("diff: {}", message));
        io.err_line("diff: Try 'diff --help' for more information.");
        2
    });
    run.into()
}

/// An argument as the shell would need it written.
fn shell_quoted(arg: &[u8]) -> Vec<u8> {
    let needs = arg.is_empty()
        || arg.iter().enumerate().any(|(i, c)| match c {
            b' ' | b'\t' | b'\n' | b'!' | b'"' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b';' | b'<' | b'=' | b'>' | b'[' | b'\\' | b'^' | b'`' | b'|' | b'?' => true,
            b'#' | b'~' => i == 0,
            b'{' | b'}' => arg.len() == 1,
            _ => false,
        });
    if !needs {
        return arg.to_vec();
    }
    let mut out = Vec::new();
    if arg.contains(&b'\'') && !arg.iter().any(|c| matches!(c, b'"' | b'$' | b'`' | b'\\' | b'!')) {
        out.push(b'"');
        out.extend_from_slice(arg);
        out.push(b'"');
        return out;
    }
    out.push(b'\'');
    for c in arg {
        if *c == b'\'' {
            out.extend_from_slice(b"'\\''");
        } else {
            out.push(*c);
        }
    }
    out.push(b'\'');
    out
}

/// A file name as diff prints it in a header: in double quotes, with C
/// escapes, when it holds a space or anything that needs one.
fn c_quoted(name: &[u8]) -> Vec<u8> {
    if !name.iter().any(|c| *c <= b' ' || *c >= 0x7f || *c == b'"' || *c == b'\\') {
        return name.to_vec();
    }
    let mut out = vec![b'"'];
    for c in name {
        match c {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\t' => out.extend_from_slice(b"\\t"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            7 => out.extend_from_slice(b"\\a"),
            8 => out.extend_from_slice(b"\\b"),
            12 => out.extend_from_slice(b"\\f"),
            11 => out.extend_from_slice(b"\\v"),
            b' '..=b'~' => out.push(*c),
            _ => out.extend_from_slice(format!("\\{:03o}", c).as_bytes()),
        }
    }
    out.push(b'"');
    out
}

pub fn diff(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let mut o = Opts {
        unified: None,
        recursive: false,
        new_file: false,
        excludes: Vec::new(),
        brief: false,
        report_same: false,
        icase: false,
        space: Space::Kept,
        blank: false,
        labels: Vec::new(),
        switches: Vec::new(),
    };
    let mut operands: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;
    // The largest context asked for wins, as in GNU diff.
    let context = |o: &mut Opts, text: Option<&[u8]>| -> Result<(), Prepared> {
        let n = match text {
            None => 3,
            Some(t) => match std::str::from_utf8(t).ok().filter(|t| !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit())) {
                Some(t) => t.parse::<usize>().unwrap_or(usize::MAX / 4).min(usize::MAX / 4),
                None => return Err(usage(format!("invalid context length '{}'", lossy(t)))),
            },
        };
        o.unified = Some(o.unified.map_or(n, |c| c.max(n)));
        Ok(())
    };
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == b"--" {
            operands.extend(args[i..].iter().cloned());
            break;
        }
        if a == b"-" || !a.starts_with(b"-") {
            operands.push(a.clone());
            continue;
        }
        o.switches.push(b' ');
        o.switches.extend(shell_quoted(a));
        // An option's argument, when it is the next word.
        let mut next = |o: &mut Opts, what: &str| -> Result<Vec<u8>, Prepared> {
            if i == args.len() {
                return Err(usage(format!("option {} requires an argument", what)));
            }
            i += 1;
            o.switches.push(b' ');
            o.switches.extend(shell_quoted(&args[i - 1]));
            Ok(args[i - 1].clone())
        };
        if let Some(long) = a.strip_prefix(b"--") {
            let (name, value) = match long.iter().position(|c| *c == b'=') {
                Some(eq) => (&long[..eq], Some(long[eq + 1..].to_vec())),
                None => (long, None),
            };
            let refused = || format!("flag: diff {} (long options other than the ones listed in the README are not supported)", lossy(a));
            match (name, value) {
                (b"unified", v) => {
                    if let Err(p) = context(&mut o, v.as_deref()) {
                        return Ok(p);
                    }
                }
                (b"recursive", None) => o.recursive = true,
                (b"new-file", None) => o.new_file = true,
                (b"brief", None) => o.brief = true,
                (b"report-identical-files", None) => o.report_same = true,
                (b"ignore-case", None) => o.icase = true,
                (b"ignore-all-space", None) => o.space = Space::All,
                (b"ignore-space-change", None) => o.space = if o.space == Space::All { Space::All } else { Space::Change },
                (b"ignore-blank-lines", None) => o.blank = true,
                (b"text", None) => {}
                (b"exclude", v) | (b"label", v) => {
                    let v = match v {
                        Some(v) => v,
                        None => match next(&mut o, &format!("'--{}'", lossy(name))) {
                            Ok(v) => v,
                            Err(p) => return Ok(p),
                        },
                    };
                    if name == b"label" {
                        o.labels.push(v);
                    } else {
                        o.excludes.push(glob::compile(&glob::fnmatch_pattern(&v)));
                    }
                }
                _ => return Err(refused()),
            }
            continue;
        }
        let mut j = 1;
        while j < a.len() {
            let c = a[j];
            j += 1;
            match c {
                b'u' => {
                    let _ = context(&mut o, None);
                }
                b'r' => o.recursive = true,
                b'N' => o.new_file = true,
                b'q' => o.brief = true,
                b's' => o.report_same = true,
                b'i' => o.icase = true,
                b'w' => o.space = Space::All,
                b'b' => o.space = if o.space == Space::All { Space::All } else { Space::Change },
                b'B' => o.blank = true,
                b'a' => {}
                b'U' | b'x' | b'L' => {
                    let v = if j < a.len() {
                        a[j..].to_vec()
                    } else {
                        match next(&mut o, &format!("requires an argument -- '{}'", c as char)) {
                            Ok(v) => v,
                            Err(_) => return Ok(usage(format!("option requires an argument -- '{}'", c as char))),
                        }
                    };
                    j = a.len();
                    match c {
                        b'U' => {
                            if let Err(p) = context(&mut o, Some(&v)) {
                                return Ok(p);
                            }
                        }
                        b'x' => o.excludes.push(glob::compile(&glob::fnmatch_pattern(&v))),
                        _ => o.labels.push(v),
                    }
                }
                _ => return Err(format!("flag: diff -{}", c as char)),
            }
        }
    }
    if o.labels.len() > 2 {
        let run: Runner = Box::new(|_, io| {
            io.err_line("diff: too many file label options");
            2
        });
        return Ok(run.into());
    }
    match operands.len() {
        0 | 1 => {
            let last = args.last().map(|a| lossy(a).into_owned()).unwrap_or_else(|| "diff".to_string());
            return Ok(usage(format!("missing operand after '{}'", last)));
        }
        2 => {}
        _ => return Ok(usage(format!("extra operand '{}'", lossy(&operands[2])))),
    }
    let stdin = operands.iter().any(|a| a == b"-");
    let run: Runner = Box::new(move |db, io| {
        let input: &[u8] = if stdin { io.take_stdin() } else { &[] };
        let mut d = Diff { db, io, o: &o, input, status: 0 };
        d.top(&operands[0], &operands[1]);
        d.status
    });
    Ok(Prepared { run, input: if stdin { Input::Whole } else { Input::Unused } })
}

/// One side of a comparison.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Node(NodeId),
    Stdin,
    /// Not there, and taken as empty (`-N`).
    Absent,
}

struct Diff<'a, 'b, 'c> {
    db: &'a Db,
    io: &'a mut Io<'b>,
    o: &'a Opts,
    input: &'c [u8],
    status: i32,
}

fn joined(dir: &[u8], name: &[u8]) -> Vec<u8> {
    let mut p = dir.to_vec();
    if !p.ends_with(b"/") {
        p.push(b'/');
    }
    p.extend_from_slice(name);
    p
}

impl Diff<'_, '_, '_> {
    fn note(&mut self, status: i32) {
        self.status = self.status.max(status);
    }

    fn trouble(&mut self, name: &[u8], what: &str) {
        self.io.err_line(format!("diff: {}: {}", lossy(name), what));
        self.note(2);
    }

    fn is_dir(&self, s: Side) -> bool {
        matches!(s, Side::Node(id) if self.db.is_dir(id))
    }

    /// The two operands of the command.
    fn top(&mut self, a: &[u8], b: &[u8]) {
        let find = |d: &Diff, name: &[u8]| -> Result<Side, &'static str> {
            if name == b"-" {
                return Ok(Side::Stdin);
            }
            d.db.resolve(name).map(Side::Node).map_err(|e| e.msg())
        };
        let (mut names, mut sides) = ([a.to_vec(), b.to_vec()], [find(self, a), find(self, b)]);
        // A file against a folder is the file against the one of its
        // name in the folder.
        let mut in_folder = None;
        for f in 0..2 {
            if let (Ok(Side::Node(file)), Ok(dir)) = (sides[f], sides[1 - f]) {
                if !self.db.is_dir(file) && self.is_dir(dir) {
                    let base = names[f].rsplit(|c| *c == b'/').next().unwrap_or(&names[f]).to_vec();
                    names[1 - f] = joined(&names[1 - f], &base);
                    sides[1 - f] = find(self, &names[1 - f]);
                    in_folder = Some(1 - f);
                }
            }
        }
        // With -N a file that is not there is an empty one, when the
        // other is there.
        let missing = |s: &Result<Side, &'static str>| *s == Err("No such file or directory");
        if self.o.new_file && (missing(&sides[0]) != missing(&sides[1])) {
            for (f, s) in sides.iter_mut().enumerate() {
                // But not the file looked for in a folder.
                if missing(s) && in_folder != Some(f) {
                    *s = Ok(Side::Absent);
                }
            }
        }
        let mut found = [Side::Absent; 2];
        let mut failed = false;
        for f in 0..2 {
            match sides[f] {
                Ok(s) => found[f] = s,
                Err(what) => {
                    self.trouble(&names[f].clone(), what);
                    failed = true;
                }
            }
        }
        if failed {
            return;
        }
        self.compare(&names[0], &names[1], found[0], found[1], 0);
    }

    /// Two names that are both there, or one of them taken as empty.
    fn compare(&mut self, name0: &[u8], name1: &[u8], s0: Side, s1: Side, depth: usize) {
        let dir0 = self.is_dir(s0) || (s0 == Side::Absent && self.is_dir(s1));
        let dir1 = self.is_dir(s1) || (s1 == Side::Absent && self.is_dir(s0));
        // A label stands for the name where a pair of files is named.
        let (shown0, shown1) = (self.o.labels.first().map_or(name0, |l| &l[..]), self.o.labels.get(1).map_or(name1, |l| &l[..]));
        if dir0 && dir1 {
            if s0 == s1 {
                // A folder against itself: nothing is said, as in GNU.
                return;
            }
            if depth > 0 && !self.o.recursive {
                self.io.put(format!("Common subdirectories: {} and {}\n", lossy(name0), lossy(name1)).as_bytes());
                return;
            }
            return self.folders(name0, name1, s0, s1, depth);
        }
        if dir0 != dir1 {
            let kind = |d: &Diff, s: Side, dir: bool| match s {
                _ if dir => "directory",
                Side::Node(id) if d.db.size(id) == 0 => "regular empty file",
                _ => "regular file",
            };
            let line = format!("File {} is a {} while file {} is a {}\n", lossy(shown0), kind(self, s0, dir0), lossy(shown1), kind(self, s1, dir1));
            self.io.put(line.as_bytes());
            self.note(1);
            return;
        }
        self.files(name0, name1, s0, s1, depth);
    }

    fn folders(&mut self, name0: &[u8], name1: &[u8], s0: Side, s1: Side, depth: usize) {
        let db = self.db;
        let names = |s: Side| -> Vec<(&[u8], NodeId)> {
            match s {
                Side::Node(id) => db.children(id).iter().filter(|(n, _)| !self.o.excludes.iter().any(|x| glob::matches(x, n))).map(|(n, id)| (&n[..], *id)).collect(),
                _ => Vec::new(),
            }
        };
        // Both lists are in byte order, which is the order GNU diff
        // sorts names into in the C locale.
        let (l0, l1) = (names(s0), names(s1));
        let (mut i, mut j) = (0, 0);
        while i < l0.len() || j < l1.len() {
            let order = match (l0.get(i), l1.get(j)) {
                (Some(a), Some(b)) => a.0.cmp(b.0),
                (Some(_), None) => std::cmp::Ordering::Less,
                _ => std::cmp::Ordering::Greater,
            };
            let (name, c0, c1) = match order {
                std::cmp::Ordering::Less => (l0[i].0, Side::Node(l0[i].1), Side::Absent),
                std::cmp::Ordering::Greater => (l1[j].0, Side::Absent, Side::Node(l1[j].1)),
                std::cmp::Ordering::Equal => (l0[i].0, Side::Node(l0[i].1), Side::Node(l1[j].1)),
            };
            i += (order != std::cmp::Ordering::Greater) as usize;
            j += (order != std::cmp::Ordering::Less) as usize;
            if (c0 == Side::Absent || c1 == Side::Absent) && !self.o.new_file {
                let dir = if c0 == Side::Absent { name1 } else { name0 };
                self.io.put(format!("Only in {}: {}\n", lossy(dir), lossy(name)).as_bytes());
                self.note(1);
                continue;
            }
            self.compare(&joined(name0, name), &joined(name1, name), c0, c1, depth + 1);
            if self.io.stopped() {
                return;
            }
        }
    }

    fn files(&mut self, name0: &[u8], name1: &[u8], s0: Side, s1: Side, depth: usize) {
        let (db, input, o) = (self.db, self.input, self.o);
        // A label stands for the name wherever the name is printed for
        // the reader, but not in the line that names the command.
        let (shown0, shown1) = (o.labels.first().map_or(name0, |l| &l[..]), o.labels.get(1).map_or(name1, |l| &l[..]));
        let doc = |s: Side| match s {
            Side::Node(id) => Doc::of_file(db, id),
            Side::Stdin => Doc::of_bytes(db, input),
            Side::Absent => Doc::of_bytes(db, &[]),
        };
        let (a, b) = (doc(s0), doc(s1));
        let plain = !o.icase && o.space == Space::Kept && !o.blank;
        let found = if s0 == s1 || a.same_pieces(&b) {
            Ok(None)
        } else if o.brief && plain {
            // Whether they differ is all that is asked.
            let (same, _, _) = a.common_start(&b, 0);
            if same {
                Ok(None)
            } else {
                return self.differ(shown0, shown1);
            }
        } else {
            analyse(&a, &b, o).map(Some)
        };
        let found = match found {
            Ok(f) => f,
            Err(message) => {
                self.io.err_line(format!("diff: {} {}: {}", lossy(name0), lossy(name1), message));
                return self.note(2);
            }
        };
        let changes = found.as_ref().is_some_and(|f| f.differs);
        if !changes {
            if o.report_same {
                self.io.put(format!("Files {} and {} are identical\n", lossy(shown0), lossy(shown1)).as_bytes());
            }
            return;
        }
        if o.brief {
            return self.differ(shown0, shown1);
        }
        self.note(1);
        let found = found.unwrap();
        if depth > 0 {
            let mut line = b"diff".to_vec();
            line.extend_from_slice(&o.switches);
            for n in [shown0, shown1] {
                line.push(b' ');
                line.extend(c_quoted(n));
            }
            line.push(b'\n');
            self.io.put(&line);
        }
        let view = View { a: &a, b: &b, found: &found };
        match o.unified {
            None => view.normal(self.io, o),
            Some(context) => {
                let time = |s: Side| match s {
                    Side::Node(id) => db.mtime(id),
                    Side::Stdin => db.now(),
                    Side::Absent => 0,
                };
                let mut head = Vec::new();
                for (f, (mark, name, s)) in [("---", name0, s0), ("+++", name1, s1)].into_iter().enumerate() {
                    head.extend_from_slice(mark.as_bytes());
                    head.push(b' ');
                    match o.labels.get(f) {
                        Some(label) => head.extend_from_slice(label),
                        None => {
                            let t = when::civil(time(s));
                            head.extend(c_quoted(name));
                            head.extend_from_slice(format!("\t{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09} +0000", t.year, t.month, t.day, t.hour, t.min, t.sec, t.nanos).as_bytes());
                        }
                    }
                    head.push(b'\n');
                }
                self.io.put(&head);
                view.unified(self.io, o, context);
            }
        }
    }

    fn differ(&mut self, name0: &[u8], name1: &[u8]) {
        self.io.put(format!("Files {} and {} differ\n", lossy(name0), lossy(name1)).as_bytes());
        self.note(1);
    }
}

// ------------------------------------------------------------- the text

/// The text of one side, a chunk at a time: the pieces of a file, or
/// bytes held in memory cut at line ends.
struct Doc<'a> {
    db: &'a Db,
    pieces: &'a [Piece],
    mem: &'a [u8],
    /// Where each chunk begins, and where the last one ends.
    offs: Vec<u64>,
    /// Newlines before each chunk, and in all.
    nls: Vec<u64>,
    tail: OnceCell<bool>,
}

impl<'a> Doc<'a> {
    fn of_file(db: &'a Db, id: NodeId) -> Doc<'a> {
        let pieces = db.pieces(id);
        let (mut offs, mut nls) = (Vec::with_capacity(pieces.len() + 1), Vec::with_capacity(pieces.len() + 1));
        let (mut off, mut nl) = (0u64, 0u64);
        for p in pieces {
            offs.push(off);
            nls.push(nl);
            off += p.len as u64;
            nl += p.lines as u64;
        }
        offs.push(off);
        nls.push(nl);
        Doc { db, pieces, mem: &[], offs, nls, tail: OnceCell::new() }
    }

    fn of_bytes(db: &'a Db, mem: &'a [u8]) -> Doc<'a> {
        let (mut offs, mut nls) = (Vec::new(), Vec::new());
        let (mut off, mut nl) = (0usize, 0u64);
        while off < mem.len() {
            offs.push(off as u64);
            nls.push(nl);
            let want = (off + 64 * 1024).min(mem.len());
            let end = if want == mem.len() { want } else { memchr::memchr(b'\n', &mem[want..]).map_or(mem.len(), |i| want + i + 1) };
            nl += memchr::memchr_iter(b'\n', &mem[off..end]).count() as u64;
            off = end;
        }
        offs.push(off as u64);
        nls.push(nl);
        Doc { db, pieces: &[], mem, offs, nls, tail: OnceCell::new() }
    }

    fn chunks(&self) -> usize {
        self.offs.len() - 1
    }

    fn size(&self) -> u64 {
        *self.offs.last().unwrap()
    }

    fn chunk(&self, i: usize) -> Cow<'a, [u8]> {
        if self.pieces.is_empty() {
            Cow::Borrowed(&self.mem[self.offs[i] as usize..self.offs[i + 1] as usize])
        } else {
            self.db.text_of(&self.pieces[i])
        }
    }

    /// Chunk `i` here and chunk `j` there are one stored piece.
    fn shared(&self, i: usize, other: &Doc, j: usize) -> bool {
        match (self.pieces.get(i), other.pieces.get(j)) {
            (Some(p), Some(q)) => p.seg == q.seg && p.off == q.off && p.len == q.len,
            _ => false,
        }
    }

    fn same_pieces(&self, other: &Doc) -> bool {
        !self.pieces.is_empty() && self.pieces.len() == other.pieces.len() && (0..self.pieces.len()).all(|i| self.shared(i, other, i)) || (self.size() == 0 && other.size() == 0)
    }

    /// The chunk that holds byte `off`, which is not the end.
    fn chunk_of(&self, off: u64) -> usize {
        self.offs.partition_point(|o| *o <= off) - 1
    }

    fn byte(&self, off: u64) -> u8 {
        let c = self.chunk_of(off);
        self.chunk(c)[(off - self.offs[c]) as usize]
    }

    /// The text does not end with a newline.
    fn open_tail(&self) -> bool {
        *self.tail.get_or_init(|| self.size() > 0 && self.byte(self.size() - 1) != b'\n')
    }

    /// Lines in all, one that is not ended among them.
    fn lines(&self) -> u64 {
        self.nls.last().unwrap() + self.open_tail() as u64
    }

    fn bytes(&self, from: u64, to: u64) -> Vec<u8> {
        let mut out = Vec::with_capacity((to - from) as usize);
        let mut at = from;
        while at < to {
            let c = self.chunk_of(at);
            let text = self.chunk(c);
            let lo = (at - self.offs[c]) as usize;
            let hi = ((to - self.offs[c]) as usize).min(text.len());
            out.extend_from_slice(&text[lo..hi]);
            at = self.offs[c] + hi as u64;
        }
        out
    }

    /// Line `n`, from 0, with its newline if it has one.
    fn line(&self, n: u64) -> Vec<u8> {
        let c = (self.nls.partition_point(|l| *l <= n) - 1).min(self.chunks() - 1);
        let text = self.chunk(c);
        let mut start = 0;
        for _ in self.nls[c]..n {
            start += memchr::memchr(b'\n', &text[start..]).map_or(text.len() - start, |i| i + 1);
        }
        let end = memchr::memchr(b'\n', &text[start..]).map_or(text.len(), |i| start + i + 1);
        text[start..end].to_vec()
    }

    /// Where line `n` begins: the end, if there is no such line.
    fn line_offset(&self, n: u64) -> u64 {
        if n >= *self.nls.last().unwrap() + 1 || self.chunks() == 0 {
            return self.size();
        }
        let c = (self.nls.partition_point(|l| *l <= n) - 1).min(self.chunks() - 1);
        let text = self.chunk(c);
        let mut start = 0;
        for _ in self.nls[c]..n {
            start += memchr::memchr(b'\n', &text[start..]).map_or(text.len() - start, |i| i + 1);
        }
        self.offs[c] + start as u64
    }

    /// The offset `count` lines on from `at`, or the end.
    fn skip_lines(&self, mut at: u64, mut count: u64) -> u64 {
        while count > 0 && at < self.size() {
            let c = self.chunk_of(at);
            let text = self.chunk(c);
            let mut lo = (at - self.offs[c]) as usize;
            while count > 0 && lo < text.len() {
                lo += memchr::memchr(b'\n', &text[lo..]).map_or(text.len() - lo, |i| i + 1);
                count -= 1;
            }
            at = self.offs[c] + lo as u64;
        }
        at
    }

    /// How far this text and the other are the same from byte `from` of
    /// each on (the same offset in both: what is before is known to be
    /// equal). Returns whether they are the same to the end of both,
    /// the offset after the last newline of the equal part, and the
    /// newlines in the equal part.
    fn common_start(&self, other: &Doc, from: u64) -> (bool, u64, u64) {
        let (mut i, mut j) = (0, 0);
        let (mut at, mut line_end, mut lines) = (from, from, 0u64);
        if from == 0 {
            // Pieces the two share are equal without being read.
            while self.shared(i, other, j) {
                i += 1;
                j += 1;
            }
            at = self.offs[i];
            lines = self.nls[i];
            if i > 0 && (i < self.chunks() || j < other.chunks() || !self.open_tail()) {
                line_end = at;
            } else if i > 0 {
                // The shared pieces are all there is of both, and the
                // last line is not ended.
                return (true, at, lines);
            }
        } else {
            i = if from < self.size() { self.chunk_of(from) } else { self.chunks() };
            j = if from < other.size() { other.chunk_of(from) } else { other.chunks() };
        }
        let (mut ta, mut tb): (Cow<[u8]>, Cow<[u8]>) = (Cow::Borrowed(&[]), Cow::Borrowed(&[]));
        let (mut pa, mut pb) = (0usize, 0usize);
        if i < self.chunks() {
            ta = self.chunk(i);
            pa = (at - self.offs[i]) as usize;
        }
        if j < other.chunks() {
            tb = other.chunk(j);
            pb = (at - other.offs[j]) as usize;
        }
        loop {
            if pa == ta.len() {
                i += (i < self.chunks()) as usize;
                if i < self.chunks() {
                    ta = self.chunk(i);
                    pa = 0;
                }
            }
            if pb == tb.len() {
                j += (j < other.chunks()) as usize;
                if j < other.chunks() {
                    tb = other.chunk(j);
                    pb = 0;
                }
            }
            let (end_a, end_b) = (i >= self.chunks(), j >= other.chunks());
            if end_a || end_b {
                return (end_a && end_b, if end_a && end_b { at } else { line_end }, lines);
            }
            let n = (ta.len() - pa).min(tb.len() - pb);
            let (x, y) = (&ta[pa..pa + n], &tb[pb..pb + n]);
            let same = if x == y { n } else { x.iter().zip(y).position(|(p, q)| p != q).unwrap() };
            if let Some(nl) = memchr::memrchr(b'\n', &x[..same]) {
                line_end = at + nl as u64 + 1;
                lines += memchr::memchr_iter(b'\n', &x[..same]).count() as u64;
            }
            at += same as u64;
            pa += same;
            pb += same;
            if same < n {
                return (false, line_end, lines);
            }
        }
    }

    /// How far back from their ends the two are the same, not going
    /// before `floor_a` here and `floor_b` there, which are line starts.
    /// Returns where the equal end begins in each, at a line start of
    /// both.
    fn common_end(&self, other: &Doc, floor_a: u64, floor_b: u64) -> (u64, u64) {
        let (mut ea, mut eb) = (self.size(), other.size());
        let (mut i, mut j) = (self.chunks(), other.chunks());
        // Shared pieces first, unread: each begins at a line start.
        while i > 0 && j > 0 && self.offs[i - 1] >= floor_a && other.offs[j - 1] >= floor_b && self.shared(i - 1, other, j - 1) {
            i -= 1;
            j -= 1;
            ea = self.offs[i];
            eb = other.offs[j];
        }
        let (whole_a, whole_b) = (ea, eb);
        let (mut ta, mut tb): (Cow<[u8]>, Cow<[u8]>) = (Cow::Borrowed(&[]), Cow::Borrowed(&[]));
        // What is left of each chunk, counted from its start.
        let (mut ra, mut rb) = (0usize, 0usize);
        let mut differ = false;
        while ea > floor_a && eb > floor_b {
            if ra == 0 {
                i -= 1;
                ta = self.chunk(i);
                ra = ta.len();
            }
            if rb == 0 {
                j -= 1;
                tb = other.chunk(j);
                rb = tb.len();
            }
            let n = ra.min(rb).min((ea - floor_a) as usize).min((eb - floor_b) as usize);
            let (x, y) = (&ta[ra - n..ra], &tb[rb - n..rb]);
            let same = if x == y { n } else { x.iter().rev().zip(y.iter().rev()).position(|(p, q)| p != q).unwrap() };
            ea -= same as u64;
            eb -= same as u64;
            ra -= same;
            rb -= same;
            if same < n {
                differ = true;
                break;
            }
        }
        // The equal end must begin a line in both. It does where the
        // comparison ran into a floor on both sides, or stopped at the
        // shared pieces; otherwise look at the byte before.
        let starts_line = |d: &Doc, at: u64, floor: u64, whole: u64| at == floor || at == 0 || (at == whole && at < d.size() && !differ) || d.byte(at - 1) == b'\n';
        if !(starts_line(self, ea, floor_a, whole_a) && starts_line(other, eb, floor_b, whole_b)) {
            // Step over the rest of the line it begins in.
            let mut at = ea;
            let mut found = None;
            while at < self.size() && found.is_none() {
                let c = self.chunk_of(at);
                let text = self.chunk(c);
                let lo = (at - self.offs[c]) as usize;
                match memchr::memchr(b'\n', &text[lo..]) {
                    Some(k) => found = Some(at + k as u64 + 1),
                    None => at = self.offs[c + 1],
                }
            }
            let next = found.unwrap_or(self.size());
            eb += next - ea;
            ea = next;
        }
        (ea, eb)
    }
}

// ------------------------------------------------------- the comparison

/// A run of lines taken out of the first text and a run put in from the
/// second, at the same place. Lines count from 0 over the whole text.
#[derive(Clone, Copy, Debug)]
struct Change {
    line0: u64,
    line1: u64,
    deleted: u64,
    inserted: u64,
    /// All its lines are blank, and blank lines are not to count.
    ignore: bool,
}

struct Found {
    script: Vec<Change>,
    /// Whether any change counts.
    differs: bool,
    /// Lines in the equal start of both.
    prefix: u64,
    /// The text between the equal ends, and where its lines begin.
    mid: [Vec<u8>; 2],
    starts: [Vec<u32>; 2],
}

/// The text of a line as the comparison sees it under the options, or
/// `None` when it sees the line as it is.
fn folded(line: &[u8], o: &Opts) -> Option<Vec<u8>> {
    if !o.icase && o.space == Space::Kept {
        return None;
    }
    let ended = line.ends_with(b"\n");
    let body = &line[..line.len() - ended as usize];
    let space = |c: u8| matches!(c, b' ' | b'\t' | 0x0b | 0x0c | b'\r' | b'\n');
    let mut out = Vec::with_capacity(body.len() + 1);
    let mut k = 0;
    while k < body.len() {
        let c = body[k];
        k += 1;
        if space(c) && o.space != Space::Kept {
            if o.space == Space::Change {
                while k < body.len() && space(body[k]) {
                    k += 1;
                }
                if k < body.len() {
                    out.push(b' ');
                }
            }
            continue;
        }
        out.push(if o.icase { c.to_ascii_lowercase() } else { c });
    }
    // A last line with no newline is equal only to another such, unless
    // white space at the end of a line is not to count.
    if ended || o.space != Space::Kept {
        out.push(b'\n');
    }
    Some(out)
}

fn line_starts(text: &[u8]) -> Vec<u32> {
    let mut starts = vec![0u32];
    starts.extend(memchr::memchr_iter(b'\n', text).map(|i| i as u32 + 1));
    if starts.last() == Some(&(text.len() as u32)) {
        starts.pop();
    }
    starts.push(text.len() as u32);
    starts
}

fn analyse(a: &Doc, b: &Doc, o: &Opts) -> Result<Found, String> {
    // The equal start, then the equal end of what is left: GNU's order.
    let (all, _, prefix) = a.common_start(b, 0);
    if all {
        return Ok(Found { script: Vec::new(), differs: false, prefix, mid: [Vec::new(), Vec::new()], starts: [vec![0], vec![0]] });
    }
    // GNU keeps as many lines of each equal end in the comparison as
    // there are lines of context, so that a change at the edge of the
    // part that differs can slide into them.
    let horizon = o.unified.unwrap_or(0) as u64;
    let prefix = prefix.saturating_sub(horizon);
    let start = a.line_offset(prefix);
    let (end_a, end_b) = a.common_end(b, start, start);
    let kept = a.skip_lines(end_a, horizon);
    let (end_a, end_b) = (kept, end_b + (kept - end_a));
    let between = (end_a - start) + (end_b - start);
    let too_much = |n: u64| format!("too much differs to compare in memory: {} MB lie between the parts that are equal, and the most that is compared is {} MB", n >> 20, MIDDLE_LIMIT >> 20);
    if between > MIDDLE_LIMIT || end_a - start > u32::MAX as u64 / 2 || end_b - start > u32::MAX as u64 / 2 {
        return Err(too_much(between));
    }
    let mid = [a.bytes(start, end_a), b.bytes(start, end_b)];
    let starts = [line_starts(&mid[0]), line_starts(&mid[1])];
    let count = [starts[0].len() - 1, starts[1].len() - 1];
    let need = between + (count[0] + count[1]) as u64 * PER_LINE;
    if need > MIDDLE_LIMIT {
        return Err(too_much(need));
    }

    // Each line's class: lines of one class are equal.
    let mut classes: HashMap<Cow<[u8]>, u32> = HashMap::new();
    let mut equivs: [Vec<u32>; 2] = [Vec::with_capacity(count[0]), Vec::with_capacity(count[1])];
    for f in 0..2 {
        for k in 0..count[f] {
            let line = &mid[f][starts[f][k] as usize..starts[f][k + 1] as usize];
            let key = match folded(line, o) {
                Some(v) => Cow::Owned(v),
                None => Cow::Borrowed(line),
            };
            let next = classes.len() as u32 + 1;
            equivs[f].push(*classes.entry(key).or_insert(next));
        }
    }
    let class_count = classes.len() + 1;
    drop(classes);

    // `changed[f][k + 1]` says line k of text f is not in the other.
    let mut changed: [Vec<bool>; 2] = [vec![false; count[0] + 2], vec![false; count[1] + 2]];
    let (kept, real) = discard_confusing_lines(&equivs, class_count, &mut changed);
    compare_sequences(&kept, &real, &mut changed);
    shift_boundaries(&equivs, &mut changed);

    let mut script = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < count[0] || j < count[1] {
        if changed[0][i + 1] || changed[1][j + 1] {
            let (from0, from1) = (i, j);
            while changed[0][i + 1] {
                i += 1;
            }
            while changed[1][j + 1] {
                j += 1;
            }
            script.push(Change { line0: prefix + from0 as u64, line1: prefix + from1 as u64, deleted: (i - from0) as u64, inserted: (j - from1) as u64, ignore: false });
        } else {
            i += 1;
            j += 1;
        }
    }
    let mut found = Found { script, differs: false, prefix, mid, starts };
    if o.blank {
        for k in 0..found.script.len() {
            found.script[k].ignore = found.blank_only(&found.script[k], o);
        }
    }
    found.differs = found.script.iter().any(|c| !c.ignore);
    Ok(found)
}

impl Found {
    fn mid_line(&self, f: usize, n: u64) -> &[u8] {
        let k = (n - self.prefix) as usize;
        &self.mid[f][self.starts[f][k] as usize..self.starts[f][k + 1] as usize]
    }

    /// Every line the change takes out or puts in is blank, as `-B`
    /// counts blank: empty, or white space alone when that is not to
    /// count either.
    fn blank_only(&self, c: &Change, o: &Opts) -> bool {
        let blank = |line: &[u8]| {
            let body = line.strip_suffix(b"\n").unwrap_or(line);
            body.is_empty() || (o.space != Space::Kept && body.iter().all(|c| matches!(c, b' ' | b'\t' | 0x0b | 0x0c | b'\r')))
        };
        (c.line0..c.line0 + c.deleted).all(|n| blank(self.mid_line(0, n))) && (c.line1..c.line1 + c.inserted).all(|n| blank(self.mid_line(1, n)))
    }
}

/// Set aside the lines of each text that no line of the other equals:
/// they are changes whatever else is found. A line that equals very
/// many is set aside with them when it stands among such lines. What is
/// left is compared. Returns, for each text, the classes of the lines
/// left and which lines they are.
fn discard_confusing_lines(equivs: &[Vec<u32>; 2], classes: usize, changed: &mut [Vec<bool>; 2]) -> ([Vec<u32>; 2], [Vec<u32>; 2]) {
    let mut counts = [vec![0u32; classes], vec![0u32; classes]];
    for f in 0..2 {
        for e in &equivs[f] {
            counts[f][*e as usize] += 1;
        }
    }
    let mut discards: [Vec<u8>; 2] = [vec![0; equivs[0].len()], vec![0; equivs[1].len()]];
    for f in 0..2 {
        let end = equivs[f].len();
        let mut many = 5usize;
        let mut tem = end / 64;
        // MANY grows with the square root of the number of lines.
        loop {
            tem >>= 2;
            if tem == 0 {
                break;
            }
            many *= 2;
        }
        for i in 0..end {
            let nmatch = counts[1 - f][equivs[f][i] as usize] as usize;
            if nmatch == 0 {
                discards[f][i] = 1;
            } else if nmatch > many {
                discards[f][i] = 2;
            }
        }
    }
    // A provisional discard stands only inside a run of discards that
    // begins and ends with lines that match nothing.
    for d in discards.iter_mut() {
        let end = d.len();
        let mut i = 0;
        while i < end {
            if d[i] == 2 {
                d[i] = 0;
            } else if d[i] != 0 {
                let mut j = i;
                let mut provisional = 0usize;
                while j < end {
                    if d[j] == 0 {
                        break;
                    }
                    if d[j] == 2 {
                        provisional += 1;
                    }
                    j += 1;
                }
                while j > i && d[j - 1] == 2 {
                    j -= 1;
                    d[j] = 0;
                    provisional -= 1;
                }
                let length = j - i;
                if provisional * 4 > length {
                    while j > i {
                        j -= 1;
                        if d[j] == 2 {
                            d[j] = 0;
                        }
                    }
                } else {
                    let mut minimum = 1usize;
                    let mut tem = length >> 2;
                    loop {
                        tem >>= 2;
                        if tem == 0 {
                            break;
                        }
                        minimum <<= 1;
                    }
                    minimum += 1;
                    // Cancel any run of MINIMUM or more provisionals.
                    let (mut j, mut consec) = (0isize, 0usize);
                    while (j as usize) < length {
                        let at = i + j as usize;
                        if d[at] != 2 {
                            consec = 0;
                        } else {
                            consec += 1;
                            if minimum == consec {
                                // Back to the start of the run.
                                j -= consec as isize;
                            } else if minimum < consec {
                                d[at] = 0;
                            }
                        }
                        j += 1;
                    }
                    // From the start of the run, until three lines in a
                    // row match nothing or the first such line eight
                    // lines in, cancel the provisionals.
                    let mut consec = 0;
                    for j in 0..length {
                        if j >= 8 && d[i + j] == 1 {
                            break;
                        }
                        if d[i + j] == 2 {
                            consec = 0;
                            d[i + j] = 0;
                        } else if d[i + j] == 0 {
                            consec = 0;
                        } else {
                            consec += 1;
                        }
                        if consec == 3 {
                            break;
                        }
                    }
                    i += length - 1;
                    // The same from the end.
                    let mut consec = 0;
                    for j in 0..length {
                        if j >= 8 && d[i - j] == 1 {
                            break;
                        }
                        if d[i - j] == 2 {
                            consec = 0;
                            d[i - j] = 0;
                        } else if d[i - j] == 0 {
                            consec = 0;
                        } else {
                            consec += 1;
                        }
                        if consec == 3 {
                            break;
                        }
                    }
                }
            }
            i += 1;
        }
    }
    let mut kept: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    let mut real: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    for f in 0..2 {
        for i in 0..equivs[f].len() {
            if discards[f][i] == 0 {
                kept[f].push(equivs[f][i]);
                real[f].push(i as u32);
            } else {
                changed[f][i + 1] = true;
            }
        }
    }
    (kept, real)
}

/// The comparison proper, over the lines not set aside: find where a
/// shortest edit script crosses the middle, and do the same for the
/// part before and the part after.
fn compare_sequences(kept: &[Vec<u32>; 2], real: &[Vec<u32>; 2], changed: &mut [Vec<bool>; 2]) {
    let (xv, yv) = (&kept[0], &kept[1]);
    let (nx, ny) = (xv.len() as isize, yv.len() as isize);
    let diags = (nx + ny + 3) as usize;
    let mut fd = vec![0isize; diags];
    let mut bd = vec![0isize; diags];
    let base = ny + 1;
    // About the square root of the input, and 4096 at least: the cost
    // at which the search settles for a middle that may not be best.
    let mut too_expensive = 1isize;
    let mut d = diags;
    while d != 0 {
        too_expensive <<= 1;
        d >>= 2;
    }
    let too_expensive = too_expensive.max(4096);
    let mut work = vec![(0isize, nx, 0isize, ny, false)];
    while let Some((mut xoff, mut xlim, mut yoff, mut ylim, minimal)) = work.pop() {
        while xoff < xlim && yoff < ylim && xv[xoff as usize] == yv[yoff as usize] {
            xoff += 1;
            yoff += 1;
        }
        while xoff < xlim && yoff < ylim && xv[xlim as usize - 1] == yv[ylim as usize - 1] {
            xlim -= 1;
            ylim -= 1;
        }
        if xoff == xlim {
            for y in yoff..ylim {
                changed[1][real[1][y as usize] as usize + 1] = true;
            }
        } else if yoff == ylim {
            for x in xoff..xlim {
                changed[0][real[0][x as usize] as usize + 1] = true;
            }
        } else {
            let (xmid, ymid, lo_minimal, hi_minimal) = middle(xv, yv, xoff, xlim, yoff, ylim, minimal, &mut fd, &mut bd, base, too_expensive);
            work.push((xmid, xlim, ymid, ylim, hi_minimal));
            work.push((xoff, xmid, yoff, ymid, lo_minimal));
        }
    }
}

/// Where a shortest edit script between the two ranges crosses a
/// middle diagonal: the search from both corners at once that GNU diff
/// makes, giving up at the same cost for the same answer.
#[allow(clippy::too_many_arguments)]
fn middle(xv: &[u32], yv: &[u32], xoff: isize, xlim: isize, yoff: isize, ylim: isize, find_minimal: bool, fd: &mut [isize], bd: &mut [isize], base: isize, too_expensive: isize) -> (isize, isize, bool, bool) {
    let at = |d: isize| (d + base) as usize;
    let dmin = xoff - ylim;
    let dmax = xlim - yoff;
    let fmid = xoff - yoff;
    let bmid = xlim - ylim;
    let (mut fmin, mut fmax) = (fmid, fmid);
    let (mut bmin, mut bmax) = (bmid, bmid);
    let odd = (fmid - bmid) & 1 != 0;
    fd[at(fmid)] = xoff;
    bd[at(bmid)] = xlim;
    let mut c = 1isize;
    loop {
        // One more edit on every diagonal of the search from the top.
        if fmin > dmin {
            fmin -= 1;
            fd[at(fmin - 1)] = -1;
        } else {
            fmin += 1;
        }
        if fmax < dmax {
            fmax += 1;
            fd[at(fmax + 1)] = -1;
        } else {
            fmax -= 1;
        }
        let mut d = fmax;
        while d >= fmin {
            let (tlo, thi) = (fd[at(d - 1)], fd[at(d + 1)]);
            let x0 = if tlo < thi { thi } else { tlo + 1 };
            let (mut x, mut y) = (x0, x0 - d);
            while x < xlim && y < ylim && xv[x as usize] == yv[y as usize] {
                x += 1;
                y += 1;
            }
            fd[at(d)] = x;
            if odd && bmin <= d && d <= bmax && bd[at(d)] <= x {
                return (x, y, true, true);
            }
            d -= 2;
        }
        // And on every diagonal of the search from the bottom.
        if bmin > dmin {
            bmin -= 1;
            bd[at(bmin - 1)] = isize::MAX;
        } else {
            bmin += 1;
        }
        if bmax < dmax {
            bmax += 1;
            bd[at(bmax + 1)] = isize::MAX;
        } else {
            bmax -= 1;
        }
        let mut d = bmax;
        while d >= bmin {
            let (tlo, thi) = (bd[at(d - 1)], bd[at(d + 1)]);
            let x0 = if tlo < thi { tlo } else { thi - 1 };
            let (mut x, mut y) = (x0, x0 - d);
            while xoff < x && yoff < y && xv[x as usize - 1] == yv[y as usize - 1] {
                x -= 1;
                y -= 1;
            }
            bd[at(d)] = x;
            if !odd && fmin <= d && d <= fmax && x <= fd[at(d)] {
                return (x, y, true, true);
            }
            d -= 2;
        }
        if !find_minimal && c >= too_expensive {
            // Gone on long enough: the diagonal that got furthest.
            let (mut fxybest, mut fxbest) = (-1isize, 0isize);
            let mut d = fmax;
            while d >= fmin {
                let mut x = fd[at(d)].min(xlim);
                let mut y = x - d;
                if ylim < y {
                    x = ylim + d;
                    y = ylim;
                }
                if fxybest < x + y {
                    fxybest = x + y;
                    fxbest = x;
                }
                d -= 2;
            }
            let (mut bxybest, mut bxbest) = (isize::MAX, 0isize);
            let mut d = bmax;
            while d >= bmin {
                let mut x = xoff.max(bd[at(d)]);
                let mut y = x - d;
                if y < yoff {
                    x = yoff + d;
                    y = yoff;
                }
                if x + y < bxybest {
                    bxybest = x + y;
                    bxbest = x;
                }
                d -= 2;
            }
            if (xlim + ylim) - bxybest < fxybest - (xoff + yoff) {
                return (fxbest, fxybest - fxbest, true, false);
            }
            return (bxbest, bxybest - bxbest, false, true);
        }
        c += 1;
    }
}

/// Slide each run of changed lines as far down as it will go, joining
/// it with runs it meets, and back up to line up with a run in the
/// other text where it can: GNU's rule, which decides which of several
/// equal lines is shown as the changed one.
fn shift_boundaries(equivs: &[Vec<u32>; 2], changed: &mut [Vec<bool>; 2]) {
    for f in 0..2 {
        let (first, second) = changed.split_at_mut(1);
        let (ch, other): (&mut Vec<bool>, &Vec<bool>) = if f == 0 { (&mut first[0], &second[0]) } else { (&mut second[0], &first[0]) };
        let eq = &equivs[f];
        // Line k is at k + 1 in both vectors; the ends are false.
        let c = |v: &[bool], k: isize| v[(k + 1) as usize];
        let i_end = eq.len() as isize;
        let (mut i, mut j) = (0isize, 0isize);
        loop {
            // The next run of changes, and the place in the other text
            // that goes with it.
            while i < i_end && !c(ch, i) {
                loop {
                    let was = c(other, j);
                    j += 1;
                    if !was {
                        break;
                    }
                }
                i += 1;
            }
            if i == i_end {
                break;
            }
            let mut start = i;
            loop {
                i += 1;
                if !c(ch, i) {
                    break;
                }
            }
            while c(other, j) {
                j += 1;
            }
            let mut corresponding;
            loop {
                let runlength = i - start;
                // Back, while the line before the run equals its last.
                while start > 0 && eq[(start - 1) as usize] == eq[(i - 1) as usize] {
                    start -= 1;
                    ch[(start + 1) as usize] = true;
                    i -= 1;
                    ch[(i + 1) as usize] = false;
                    while c(ch, start - 1) {
                        start -= 1;
                    }
                    loop {
                        j -= 1;
                        if !c(other, j) {
                            break;
                        }
                    }
                }
                // The end of the run where it last lined up with a run
                // of the other text, if it did.
                corresponding = if c(other, j - 1) { i } else { i_end };
                // Forward, while its first line equals the line after.
                while i != i_end && eq[start as usize] == eq[i as usize] {
                    ch[(start + 1) as usize] = false;
                    start += 1;
                    ch[(i + 1) as usize] = true;
                    i += 1;
                    while c(ch, i) {
                        i += 1;
                    }
                    loop {
                        j += 1;
                        if !c(other, j) {
                            break;
                        }
                        corresponding = i;
                    }
                }
                if runlength == i - start {
                    break;
                }
            }
            // Back to where it lined up.
            while corresponding < i {
                start -= 1;
                ch[(start + 1) as usize] = true;
                i -= 1;
                ch[(i + 1) as usize] = false;
                loop {
                    j -= 1;
                    if !c(other, j) {
                        break;
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- printing

struct View<'a, 'b> {
    a: &'a Doc<'b>,
    b: &'a Doc<'b>,
    found: &'a Found,
}

impl View<'_, '_> {
    /// Line `n` of text `f`: from what was compared, or read for this.
    fn line(&self, f: usize, n: u64) -> Cow<'_, [u8]> {
        let count = self.found.starts[f].len() as u64 - 1;
        if n >= self.found.prefix && n < self.found.prefix + count {
            return Cow::Borrowed(self.found.mid_line(f, n));
        }
        let doc = if f == 0 { self.a } else { self.b };
        if n < self.found.prefix {
            return Cow::Owned(doc.line(n));
        }
        // After the middle: the equal end, which begins where the
        // middle of this text stops.
        let mid_newlines = memchr::memchr_iter(b'\n', &self.found.mid[f]).count() as u64;
        Cow::Owned(doc.line(n - count + mid_newlines))
    }

    fn lines(&self, f: usize) -> u64 {
        if f == 0 {
            self.a.lines()
        } else {
            self.b.lines()
        }
    }

    fn put_line(&self, out: &mut Vec<u8>, mark: &[u8], f: usize, n: u64) {
        let line = self.line(f, n);
        out.extend_from_slice(mark);
        out.extend_from_slice(&line);
        if !line.ends_with(b"\n") {
            out.extend_from_slice(b"\n\\ No newline at end of file\n");
        }
    }

    fn normal(&self, io: &mut Io, _o: &Opts) {
        let range = |first: u64, count: u64| -> String {
            // A range of no lines is named by the line before it.
            if count > 1 {
                format!("{},{}", first + 1, first + count)
            } else {
                format!("{}", first + count)
            }
        };
        for c in &self.found.script {
            if c.ignore {
                continue;
            }
            let letter = match (c.deleted > 0, c.inserted > 0) {
                (true, true) => 'c',
                (true, false) => 'd',
                _ => 'a',
            };
            let mut out = format!("{}{}{}\n", range(c.line0, c.deleted), letter, range(c.line1, c.inserted)).into_bytes();
            for n in c.line0..c.line0 + c.deleted {
                self.put_line(&mut out, b"< ", 0, n);
            }
            if letter == 'c' {
                out.extend_from_slice(b"---\n");
            }
            for n in c.line1..c.line1 + c.inserted {
                self.put_line(&mut out, b"> ", 1, n);
            }
            io.put(&out);
            if io.stopped() {
                return;
            }
        }
    }

    fn unified(&self, io: &mut Io, _o: &Opts, context: usize) {
        let script = &self.found.script;
        let context = context as u64;
        let mut k = 0;
        while k < script.len() {
            // The changes that go in one hunk: each begins within the
            // context of the one before, counted as GNU counts it.
            let from = k;
            loop {
                let top0 = script[k].line0 + script[k].deleted;
                k += 1;
                if k == script.len() {
                    break;
                }
                let thresh = if script[k].ignore { context } else { 2 * context + 1 };
                if script[k].line0 - top0 >= thresh {
                    break;
                }
            }
            let hunk = &script[from..k];
            if hunk.iter().all(|c| c.ignore) {
                continue;
            }
            let last = hunk[hunk.len() - 1];
            let first0 = hunk[0].line0.saturating_sub(context);
            let first1 = hunk[0].line1.saturating_sub(context);
            // One past the last line shown of each.
            let end = |f: usize, after: u64| -> u64 {
                let total = self.lines(f);
                if after + context <= total {
                    after + context
                } else {
                    total
                }
            };
            let end0 = end(0, last.line0 + last.deleted);
            let end1 = end(1, last.line1 + last.inserted);
            let range = |first: u64, end: u64| -> String {
                match end - first {
                    0 => format!("{},0", first),
                    1 => format!("{}", first + 1),
                    n => format!("{},{}", first + 1, n),
                }
            };
            let mut out = format!("@@ -{} +{} @@\n", range(first0, end0), range(first1, end1)).into_bytes();
            let (mut i, mut j) = (first0, first1);
            let mut next = 0;
            while i < end0 || j < end1 {
                if next == hunk.len() || i < hunk[next].line0 {
                    self.put_line(&mut out, b" ", 0, i);
                    i += 1;
                    j += 1;
                } else {
                    for _ in 0..hunk[next].deleted {
                        self.put_line(&mut out, b"-", 0, i);
                        i += 1;
                    }
                    for _ in 0..hunk[next].inserted {
                        self.put_line(&mut out, b"+", 1, j);
                        j += 1;
                    }
                    next += 1;
                }
                if out.len() > 64 * 1024 {
                    io.put(&out);
                    out.clear();
                    if io.stopped() {
                        return;
                    }
                }
            }
            io.put(&out);
            if io.stopped() {
                return;
            }
        }
    }
}
