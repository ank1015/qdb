//! grep.
//!
//! This file is the command: options, which files, and what to print. The
//! search itself, finding the matching lines of a buffer, is `scan.rs`.

use super::{options_long, LineBuf};
use crate::bytes::{count_newlines, memchr, memrchr, push_decimal};
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use crate::glob::{self, Tok};
use crate::regex::{Flavor, MatchError, Regex, Syntax};
use crate::scan::Searcher;
use crate::tree::{Candidate, Candidates, Db, NodeId, Shortlist};
use crate::util::lossy;
use std::rc::Rc;

#[derive(Clone, Copy)]
struct Opts {
    recursive: bool,
    line_numbers: bool,
    /// -l
    list: bool,
    /// -L: name the files with no selected line.
    without: bool,
    invert: bool,
    count: bool,
    quiet: bool,
    /// -s: say nothing about files that cannot be read.
    no_messages: bool,
    /// -o: print the matches, not the lines.
    only: bool,
    /// -m: stop reading a file after this many selected lines.
    max: Option<usize>,
    before: usize,
    after: usize,
    /// Context was asked for, if only of no lines: groups of lines that
    /// are not next to each other get `--` between them.
    grouped: bool,
}

impl Opts {
    /// True if lines are not printed.
    fn silent(&self) -> bool {
        self.list || self.without || self.count || self.quiet
    }

    fn context(&self) -> bool {
        self.grouped && !self.silent()
    }
}

/// --include, --exclude and --exclude-dir.
#[derive(Default)]
struct Select {
    /// In the order given; true for --include.
    files: Vec<(bool, Vec<Tok>)>,
    dirs: Vec<Vec<Tok>>,
}

impl Select {
    fn is_empty(&self) -> bool {
        self.files.is_empty() && self.dirs.is_empty()
    }

    /// A name met in a walk is matched whole; one given on the command
    /// line also by any part of it that follows a slash.
    fn matches(toks: &[Tok], name: &[u8], command_line: bool) -> bool {
        if glob::matches(toks, name) {
            return true;
        }
        command_line && (1..name.len()).any(|i| name[i - 1] == b'/' && name[i] != b'/' && glob::matches(toks, &name[i..]))
    }

    /// The last option that matches decides; if none does, the file is
    /// left out when the first option is an --include.
    fn skip_file(&self, name: &[u8], command_line: bool) -> bool {
        match self.files.iter().rev().find(|(_, toks)| Select::matches(toks, name, command_line)) {
            Some((include, _)) => !include,
            None => self.files.first().is_some_and(|(include, _)| *include),
        }
    }

    fn skip_dir(&self, name: &[u8], command_line: bool) -> bool {
        self.dirs.iter().any(|toks| Select::matches(toks, name, command_line))
    }

    /// The same for a path below the folder being searched.
    fn skip_path(&self, path: &[u8]) -> bool {
        let mut parts: Vec<&[u8]> = path.split(|c| *c == b'/').collect();
        let file = parts.pop().unwrap_or(b"");
        parts.iter().any(|d| self.skip_dir(d, false)) || self.skip_file(file, false)
    }
}

/// What the command line asked for, before the pattern is compiled.
/// Pattern errors are grep's own, reported when the command runs.
#[derive(Clone)]
struct Plan {
    opts: Opts,
    icase: bool,
    conflicting: bool,
    mode: Option<u8>,
    pattern: Option<Vec<u8>>,
    /// -w, or -x which goes before it.
    words: bool,
    lines: bool,
    /// A bad number for -m or the context flags: grep's message.
    bad_number: Option<String>,
    /// -H or -h, whichever came last.
    filename: Option<bool>,
    select: Rc<Select>,
}

/// The compiled search: the lines that match, and for -o the patterns
/// that say where in a line.
struct Compiled {
    searcher: Searcher,
    finders: Vec<Regex>,
}

impl Plan {
    /// Compile the pattern. The error is the exit status, with the message
    /// already written.
    fn compile(&self, io: &mut Io) -> Result<Compiled, i32> {
        let pattern = match &self.pattern {
            Some(p) => p,
            None => {
                io.err_line("Usage: grep [OPTION]... PATTERNS [FILE]...");
                io.err_line("Try 'grep --help' for more information.");
                return Err(2);
            }
        };
        if self.conflicting {
            io.err_line("grep: conflicting matchers specified");
            return Err(2);
        }
        if let Some(m) = &self.bad_number {
            io.err_line(m);
            return Err(2);
        }
        // A newline in the pattern separates alternative patterns.
        let parts: Vec<&[u8]> = pattern.split(|c| *c == b'\n').collect();
        let bounded = self.words || self.lines;
        let fixed = self.mode == Some(b'F');
        let mut res = Vec::new();
        if !fixed || bounded || self.opts.only {
            let syn = Syntax { extended: self.mode == Some(b'E'), icase: self.icase, flavor: Flavor::Grep };
            for part in &parts {
                let re = if fixed {
                    Regex::from_strings(&[part.to_vec()], self.icase).ok_or_else(|| "pattern too long".to_string())
                } else {
                    Regex::new(part, syn)
                };
                let re = re.and_then(|re| if bounded { re.bounded(self.lines) } else { Ok(re) });
                match re {
                    Ok(re) => {
                        for w in &re.warnings {
                            io.err_line(format!("grep: {}", w));
                        }
                        res.push(re);
                    }
                    Err(e) => {
                        io.err_line(format!("grep: {}", e));
                        return Err(2);
                    }
                }
            }
        }
        let searcher = if fixed && !bounded {
            Searcher::fixed(parts.iter().map(|p| p.to_vec()).collect(), self.icase)
        } else {
            Searcher::regex(res.clone())
        };
        // GNU grep sees that nothing can be selected and stops at once.
        if self.opts.invert && pattern.is_empty() && !bounded || self.opts.max == Some(0) {
            return Err(1);
        }
        Ok(Compiled { searcher, finders: if self.opts.only { res } else { Vec::new() } })
    }
}

/// The leftmost match of any of the patterns at or after `from`, the
/// longest there.
fn next_match(finders: &[Regex], line: &[u8], from: usize) -> Result<Option<(usize, usize)>, MatchError> {
    let mut best: Option<(usize, usize)> = None;
    for re in finders {
        if let Some(caps) = re.find_at(line, from, false)? {
            let (s, e) = caps[0].expect("the whole match");
            if best.map_or(true, |(bs, be)| s < bs || s == bs && e > be) {
                best = Some((s, e));
            }
        }
    }
    Ok(best)
}

const TOO_COMPLEX: &str = "pattern too complex for the backtracking matcher";

/// grep reading its standard input in pieces.
struct GrepFilter {
    plan: Plan,
    /// Compiled when the first input arrives.
    compiled: Option<Result<Compiled, i32>>,
    lines: LineBuf,
    line_no: usize,
    count: usize,
    error: bool,
    done: bool,
    out: Vec<u8>,
}

impl GrepFilter {
    fn start(&mut self, io: &mut Io) -> bool {
        if self.compiled.is_none() {
            self.compiled = Some(self.plan.compile(io));
        }
        matches!(self.compiled, Some(Ok(_)))
    }

    /// One line, with or without its newline. Returns false when no more
    /// input is needed.
    fn line(&mut self, line: &[u8], io: &mut Io) -> bool {
        let o = self.plan.opts;
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        self.line_no += 1;
        let compiled = self.compiled.as_mut().and_then(|s| s.as_mut().ok()).expect("started");
        let hit = match compiled.searcher.is_match(text) {
            Ok(h) => h,
            Err(_) => {
                io.err_line(format!("qdb: grep: (standard input): {}", TOO_COMPLEX));
                self.error = true;
                return false;
            }
        };
        if hit == o.invert {
            return true;
        }
        self.count += 1;
        if o.list || o.without || o.quiet {
            self.done = true;
            return false;
        }
        if !o.count {
            self.out.clear();
            if self.plan.filename == Some(true) {
                self.out.extend_from_slice(b"(standard input):");
            }
            if o.line_numbers {
                push_decimal(&mut self.out, self.line_no);
                self.out.push(b':');
            }
            if o.only {
                let mut at = 0;
                while at <= text.len() && !o.invert {
                    match next_match(&compiled.finders, text, at) {
                        Ok(Some((s, e))) if e > s => {
                            io.put(&self.out);
                            io.put(&text[s..e]);
                            io.put(b"\n");
                            at = e;
                        }
                        Ok(Some((_, e))) => at = e + 1,
                        _ => break,
                    }
                }
            } else {
                io.put(&self.out);
                io.put(text);
                io.put(b"\n");
            }
        }
        if o.max.is_some_and(|m| self.count >= m) {
            self.done = true;
            return false;
        }
        true
    }
}

impl Filter for GrepFilter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        if !self.start(io) || self.error || self.done {
            return false;
        }
        let mut lines = std::mem::take(&mut self.lines);
        let more = lines.feed(data, |line| self.line(line, io));
        self.lines = lines;
        more
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if !self.start(io) {
            return self.compiled.and_then(|s| s.err()).unwrap_or(2);
        }
        if !self.error && !self.done {
            if let Some(last) = self.lines.rest() {
                self.line(&last, io);
            }
        }
        let o = self.plan.opts;
        if o.quiet {
        } else if o.list || o.without {
            if (self.count > 0) == o.list {
                io.put(b"(standard input)\n");
            }
        } else if o.count {
            if self.plan.filename == Some(true) {
                io.put(b"(standard input):");
            }
            io.put(format!("{}\n", self.count).as_bytes());
        }
        if self.error && !(o.quiet && self.count > 0) {
            2
        } else if self.count > 0 {
            0
        } else {
            1
        }
    }
}

const INCLUDE: u8 = 0x80;
const EXCLUDE: u8 = 0x81;
const EXCLUDE_DIR: u8 = 0x82;

pub fn grep(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let longs = [("include", true, INCLUDE), ("exclude", true, EXCLUDE), ("exclude-dir", true, EXCLUDE_DIR)];
    let p = match options_long("grep", &args, b"rRinlLvcEFGoqswxhHIa0123456789", b"eABCm", &longs, 2)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let mut opts = Opts {
        recursive: p.has(b'r') || p.has(b'R'),
        line_numbers: p.has(b'n'),
        list: false,
        without: false,
        invert: p.has(b'v'),
        count: p.has(b'c'),
        quiet: p.has(b'q'),
        no_messages: p.has(b's'),
        only: p.has(b'o'),
        max: None,
        before: 0,
        after: 0,
        grouped: false,
    };
    let icase = p.has(b'i');
    let mut modes: Vec<u8> = p.flags.iter().map(|f| f.0).filter(|c| matches!(c, b'E' | b'F' | b'G')).collect();
    modes.dedup();
    let conflicting = modes.iter().any(|m| *m != modes[0]);
    let mode = modes.first().copied();
    let mut bad_number = None;
    let mut patterns: Option<Vec<u8>> = None;
    let mut select = Select::default();
    let mut filename = None;
    // -NUM is -C NUM; -A and -B, given at all, go before either.
    let (mut around, mut before, mut after) = (None, None, None);
    let mut digits: Option<usize> = None;
    let number = |arg: &Option<Vec<u8>>| -> Result<usize, ()> {
        let a = arg.as_deref().unwrap_or(b"");
        if a.is_empty() || !a.iter().all(|c| c.is_ascii_digit()) {
            return Err(());
        }
        Ok(lossy(a).parse().unwrap_or(usize::MAX))
    };
    for (f, arg) in &p.flags {
        if f.is_ascii_digit() {
            digits = Some(digits.unwrap_or(0).saturating_mul(10).saturating_add((f - b'0') as usize));
            around = digits;
            continue;
        }
        digits = None;
        match *f {
            b'l' => (opts.list, opts.without) = (true, false),
            b'L' => (opts.list, opts.without) = (false, true),
            b'H' => filename = Some(true),
            b'h' => filename = Some(false),
            b'e' => {
                let pat = arg.clone().unwrap_or_default();
                match &mut patterns {
                    Some(all) => {
                        all.push(b'\n');
                        all.extend(pat);
                    }
                    None => patterns = Some(pat),
                }
            }
            b'A' | b'B' | b'C' => match number(arg) {
                Ok(n) => match *f {
                    b'A' => after = Some(n),
                    b'B' => before = Some(n),
                    _ => around = Some(n),
                },
                Err(()) => {
                    bad_number.get_or_insert(format!("grep: {}: invalid context length argument", lossy(arg.as_deref().unwrap_or(b""))));
                }
            },
            // A negative count is no limit.
            b'm' if arg.as_deref().is_some_and(|a| a.len() > 1 && a[0] == b'-' && a[1..].iter().all(|c| c.is_ascii_digit())) => opts.max = None,
            b'm' => match number(arg) {
                Ok(n) => opts.max = Some(n),
                Err(()) => {
                    bad_number.get_or_insert("grep: invalid max count".to_string());
                }
            },
            INCLUDE | EXCLUDE => {
                let toks = glob::compile(&glob::fnmatch_pattern(arg.as_deref().unwrap_or(b"")));
                select.files.push((*f == INCLUDE, toks));
            }
            EXCLUDE_DIR => {
                let mut pat = arg.clone().unwrap_or_default();
                while pat.len() > 1 && pat.ends_with(b"/") {
                    pat.pop();
                }
                select.dirs.push(glob::compile(&glob::fnmatch_pattern(&pat)));
            }
            _ => {}
        }
    }
    opts.grouped = before.or(after).or(around).is_some();
    opts.before = before.or(around).unwrap_or(0);
    opts.after = after.or(around).unwrap_or(0);
    let (words, lines) = (p.has(b'w'), p.has(b'x'));
    let mut operands = p.operands.into_iter();
    let pattern = patterns.or_else(|| operands.next());
    let files: Vec<Vec<u8>> = operands.collect();
    let plan = Plan {
        opts,
        icase,
        conflicting,
        mode: mode.filter(|m| *m != b'G'),
        pattern,
        words,
        lines,
        bad_number,
        filename,
        select: Rc::new(select),
    };

    let input = if files.is_empty() && !opts.recursive {
        if opts.context() {
            // Lines before a match have gone by when the match is seen.
            Input::Whole
        } else {
            Input::Stream(Box::new(GrepFilter {
                plan: plan.clone(),
                compiled: None,
                lines: LineBuf::default(),
                line_no: 0,
                count: 0,
                error: false,
                done: false,
                out: Vec::new(),
            }))
        }
    } else if files.iter().any(|f| f == b"-") {
        Input::Whole
    } else {
        Input::Unused
    };

    let run: Runner = Box::new(move |db, io| {
        let Compiled { searcher, finders } = match plan.compile(io) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let implicit_dot = files.is_empty() && opts.recursive;
        let single_dir = files.len() == 1 && db.resolve(&files[0]).is_ok_and(|id| db.is_dir(id));
        let with_filename = plan.filename.unwrap_or(files.len() > 1 || opts.recursive && (implicit_dot || single_dir));
        let mut run = Run {
            opts,
            searcher,
            finders,
            select: plan.select.clone(),
            with_filename,
            selected: false,
            error: false,
            done: false,
            printed: false,
            line: Vec::new(),
            only: None,
            probe: None,
            top: (db.root(), 0),
            rest: None,
            list_probe: None,
            list_only: None,
            pieces: None,
        };
        // Several files named: storage may know which of them can match.
        // (Or one large file: storage may know which pieces of it can.)
        if !files.is_empty() && !opts.invert {
            run.list_probe = db.candidates_cost(run.searcher.query());
        }
        if implicit_dot {
            let mut path = Vec::new();
            run.tree(db, io, db.root(), &mut path);
        } else if files.is_empty() {
            let data = io.take_stdin();
            run.search(io, b"(standard input)", data, None);
        }
        for f in &files {
            if io.stopped() || run.done {
                break;
            }
            if f == b"-" {
                let data = io.take_stdin();
                run.search(io, b"(standard input)", data, None);
                continue;
            }
            match db.resolve(f) {
                Err(e) => {
                    if !opts.no_messages {
                        io.err_line(format!("grep: {}: {}", lossy(f), e.msg()));
                    }
                    run.error = true;
                }
                Ok(id) if db.is_dir(id) => {
                    if !opts.recursive {
                        if !opts.no_messages {
                            io.err_line(format!("grep: {}: Is a directory", lossy(f)));
                        }
                        run.error = true;
                        // GNU grep still reports a count for it.
                        if opts.count && !opts.list && !opts.without {
                            run.search(io, f, b"", None);
                        }
                        continue;
                    }
                    if run.select.skip_dir(f, true) {
                        continue;
                    }
                    // Trailing slashes are dropped before names are joined.
                    let mut base = f.clone();
                    while base.len() > 1 && base.ends_with(b"/") {
                        base.pop();
                    }
                    if base != b"/" {
                        base.push(b'/');
                    }
                    run.tree(db, io, id, &mut base);
                }
                Ok(_) if run.select.skip_file(f, true) => {}
                Ok(id) => run.listed(db, io, f, id),
            }
        }
        if run.error && !(opts.quiet && run.selected) {
            2
        } else if run.selected {
            0
        } else {
            1
        }
    });
    Ok(Prepared { run, input })
}

struct Run {
    opts: Opts,
    searcher: Searcher,
    /// For -o: the patterns, to find where in a line they match.
    finders: Vec<Regex>,
    select: Rc<Select>,
    with_filename: bool,
    selected: bool,
    error: bool,
    /// Nothing more is wanted: -q has its answer.
    done: bool,
    /// With context: something has been printed, so the next group of
    /// lines is set off by `--`.
    printed: bool,
    /// Scratch for putting an output line together.
    line: Vec<u8>,
    /// During a walk: which of the files can have a match.
    only: Option<Only>,
    /// During a walk that has not asked storage yet: how many more bytes
    /// to read before asking.
    probe: Option<usize>,
    /// The folder being searched, and how much of the path names it.
    top: (NodeId, usize),
    /// Set to end a walk: the files still to read, from the given place.
    rest: Option<(Vec<Candidate>, usize)>,
    /// For files named on the command line: how many more bytes to read
    /// before asking storage, and what it answered.
    list_probe: Option<usize>,
    list_only: Option<Shortlist>,
    /// During a walk, once storage has been asked: the pieces that can
    /// have a match.
    pieces: Option<Shortlist>,
}

enum Only {
    /// These, in walk order; and how many of them the walk has reached.
    Listed(Vec<Candidate>, usize),
    /// Those with a piece in `Run::pieces`.
    Passing,
}

impl Run {
    /// Search the files under a folder; the output is what reading every
    /// file in order would give.
    ///
    /// Storage can say which files can match, and that answer costs
    /// something to work out. So the search starts by simply reading
    /// files, which is all a command like `grep -rn x . | head -n 1` may
    /// ever need, and asks storage once it has read about as much as the
    /// answer costs. From there on only the files that can match are read.
    fn tree(&mut self, db: &Db, io: &mut Io, dir: NodeId, path: &mut Vec<u8>) {
        // With -v a file is selected for the lines that do not match, so
        // what it must contain says nothing.
        self.probe = if self.opts.invert { None } else { db.candidates_cost(self.searcher.query()) };
        self.top = (dir, path.len());
        self.pieces = None;
        self.walk(db, io, dir, path);
        self.probe = None;
        self.only = None;
        // The walk stops early when what is left is a short list of files.
        if let Some((files, from)) = self.rest.take() {
            let base = path.len();
            for file in &files[from..] {
                if io.stopped() || self.done {
                    return;
                }
                path.extend_from_slice(&file.path);
                let pieces = self.pieces.take();
                self.search_file(db, io, path, file.id, pieces.as_ref());
                self.pieces = pieces;
                path.truncate(base);
            }
        }
        self.pieces = None;
    }

    /// Search a file that was named on the command line. As in a walk,
    /// the first files are simply read, and storage is asked which files
    /// can match once that much reading has cost what the answer costs.
    fn listed(&mut self, db: &Db, io: &mut Io, label: &[u8], id: NodeId) {
        if let Some(budget) = self.list_probe {
            let cost = db.size(id) as usize + 256;
            if budget >= cost {
                self.list_probe = Some(budget - cost);
            } else {
                self.list_probe = None;
                self.list_only = db.shortlist(self.searcher.query());
            }
        }
        let o = self.opts;
        if self.list_only.as_ref().map_or(true, |set| db.passes(set, id)) {
            let only = self.list_only.take();
            self.search_file(db, io, label, id, only.as_ref());
            self.list_only = only;
        } else if o.count && !o.list || o.without {
            // No line of it matches, and it still gets its count.
            self.search(io, label, b"", Some(id));
        } else if io.out_file == Some(id) && !o.list {
            // Writing into a file that is being searched is an error
            // whatever the file holds.
            self.is_output(io, label, Some(id));
        }
    }

    /// Ask storage which files can match, part of the way through a walk:
    /// `path` is the file the walk has reached and not yet read.
    fn consult(&mut self, db: &Db, io: &mut Io, path: &[u8]) {
        let o = self.opts;
        let (dir, prefix) = self.top;
        let here = &path[prefix..];
        let order = |a: &[u8], b: &[u8]| a.split(|c| *c == b'/').cmp(b.split(|c| *c == b'/'));
        let mut files = match db.candidates(dir, self.searcher.query()) {
            Candidates::All => return,
            Candidates::Among(set) => {
                self.only = Some(Only::Passing);
                self.pieces = Some(set);
                return;
            }
            Candidates::Files(mut files, set) => {
                self.pieces = Some(set);
                if !self.select.is_empty() {
                    files.retain(|c| !self.select.skip_path(&c.path));
                }
                files
            }
        };
        let counting = o.count && !o.list || o.without;
        // Writing into a file that is being searched is an error whatever
        // the file holds, so that file is always looked at.
        if let (Some(out), false) = (io.out_file, counting) {
            if let Some(rel) = db.path_below(dir, out) {
                if let Err(at) = files.binary_search_by(|c| order(&c.path, &rel)) {
                    files.insert(at, Candidate { id: out, path: rel });
                }
            }
        }
        // The candidates the walk has not passed yet.
        let from = files.partition_point(|c| order(&c.path, here).is_lt());
        if counting {
            // Every file gets a line, so every file is still visited, but
            // only the candidates are read.
            self.only = Some(Only::Listed(files, from));
        } else {
            self.rest = Some((files, from));
        }
    }

    /// Search every file under a folder, depth first in byte order.
    /// `path` is what goes in front of each child's name; it is restored
    /// before returning.
    fn walk(&mut self, db: &Db, io: &mut Io, dir: NodeId, path: &mut Vec<u8>) {
        let base = path.len();
        for (name, child) in db.children(dir) {
            if io.stopped() || self.rest.is_some() || self.done {
                return;
            }
            if !self.select.is_empty() {
                let skip = if db.is_dir(*child) { self.select.skip_dir(name, false) } else { self.select.skip_file(name, false) };
                if skip {
                    continue;
                }
            }
            path.extend_from_slice(name);
            if db.is_dir(*child) {
                path.push(b'/');
                self.walk(db, io, *child, path);
            } else {
                if let Some(budget) = self.probe {
                    // A file costs its bytes, and a little for the visit.
                    let cost = db.size(*child) as usize + 256;
                    if budget >= cost {
                        self.probe = Some(budget - cost);
                    } else {
                        self.probe = None;
                        self.consult(db, io, path);
                        if self.rest.is_some() {
                            path.truncate(base);
                            return;
                        }
                    }
                }
                let o = self.opts;
                let candidate = match &mut self.only {
                    None => true,
                    Some(Only::Listed(files, reached)) => {
                        let next = files.get(*reached).is_some_and(|f| f.id == *child);
                        *reached += next as usize;
                        next
                    }
                    Some(Only::Passing) => self.pieces.as_ref().is_some_and(|set| db.passes(set, *child)),
                };
                if candidate {
                    let pieces = self.pieces.take();
                    self.search_file(db, io, path, *child, pieces.as_ref());
                    self.pieces = pieces;
                } else if o.count && !o.list || o.without {
                    // No line of it matches, and it still gets its count.
                    self.search(io, path, b"", Some(*child));
                } else if io.out_file == Some(*child) && !o.list {
                    // Writing into a file that is being searched is an
                    // error whatever the file holds.
                    self.is_output(io, path, Some(*child));
                }
            }
            path.truncate(base);
        }
    }

    /// Print one line: `label:number:text` for a selected line, with `-`
    /// in place of `:` for a line of context.
    fn emit(&mut self, io: &mut Io, label: &[u8], number: usize, text: &[u8], sep: u8) {
        self.line.clear();
        if self.with_filename {
            self.line.extend_from_slice(label);
            self.line.push(sep);
        }
        if self.opts.line_numbers {
            push_decimal(&mut self.line, number);
            self.line.push(sep);
        }
        io.put(&self.line);
        io.put(text);
        io.put(b"\n");
    }

    /// Print a selected line, or with -o each match in it. False if the
    /// matcher gave up.
    /// A line of context. With -o, what is printed of it is what matches
    /// in it, and only under -v, as GNU grep does: those are then the lines
    /// around the selected ones.
    fn emit_context(&mut self, io: &mut Io, label: &[u8], number: usize, text: &[u8]) {
        if !self.opts.only {
            self.emit(io, label, number, text, b'-');
            return;
        }
        if !self.opts.invert {
            return;
        }
        let mut at = 0;
        while at <= text.len() {
            match next_match(&self.finders, text, at) {
                Ok(Some((s, e))) if e > s => {
                    self.emit(io, label, number, &text[s..e], b'-');
                    at = e;
                }
                Ok(Some((_, e))) => at = e + 1,
                _ => break,
            }
        }
    }

    fn emit_selected(&mut self, io: &mut Io, label: &[u8], number: usize, text: &[u8]) -> bool {
        if !self.opts.only {
            self.emit(io, label, number, text, b':');
            return true;
        }
        if self.opts.invert {
            return true;
        }
        let mut at = 0;
        while at <= text.len() {
            match next_match(&self.finders, text, at) {
                Ok(Some((s, e))) if e > s => {
                    self.emit(io, label, number, &text[s..e], b':');
                    at = e;
                }
                Ok(Some((_, e))) => at = e + 1,
                Ok(None) => break,
                Err(_) => {
                    io.err_line(format!("qdb: grep: {}: {}", lossy(label), TOO_COMPLEX));
                    self.error = true;
                    return false;
                }
            }
        }
        true
    }

    /// The first selected line at or after `pos`, which starts a line.
    /// `ahead` remembers the next matching line between calls when the
    /// selected lines are the ones that do not match.
    fn next_selected(&mut self, content: &[u8], mut pos: usize, ahead: &mut Option<Option<(usize, usize)>>) -> Result<Option<(usize, usize)>, MatchError> {
        if !self.opts.invert {
            return self.searcher.next_line(content, pos);
        }
        while pos < content.len() {
            let stale = match ahead {
                None => true,
                Some(Some((s, _))) => *s < pos,
                Some(None) => false,
            };
            if stale {
                *ahead = Some(self.searcher.next_line(content, pos)?);
            }
            match ahead {
                Some(Some((s, e))) if *s == pos => pos = *e + 1,
                _ => return Ok(Some((pos, memchr(b'\n', &content[pos..]).map_or(content.len(), |i| pos + i)))),
            }
        }
        Ok(None)
    }

    /// Search content that is all in one place and print the selected
    /// lines with the lines around them. Returns the number selected, or
    /// None if the search failed.
    fn scan_context(&mut self, io: &mut Io, label: &[u8], content: &[u8]) -> Option<usize> {
        let o = self.opts;
        let mut count = 0;
        let mut pos = 0;
        let mut ahead = None;
        // The offset after the last line printed, selected or context.
        let mut last_end: Option<usize> = None;
        let mut after_left = 0;
        // `numbered` lines lie before offset `counted_to`.
        let (mut numbered, mut counted_to) = (0usize, 0usize);
        let mut number_at = |off: usize| {
            numbered += count_newlines(&content[counted_to..off]);
            counted_to = off;
            numbered + 1
        };
        let line_end = |at: usize| memchr(b'\n', &content[at..]).map_or(content.len(), |i| at + i);
        loop {
            let found = match self.next_selected(content, pos, &mut ahead) {
                Ok(f) => f,
                Err(_) => {
                    io.err_line(format!("qdb: grep: {}: {}", lossy(label), TOO_COMPLEX));
                    self.error = true;
                    return None;
                }
            };
            let done = found.is_none() || o.max.is_some_and(|m| count >= m);
            // The lines after the last selected line, up to the next one.
            // Once -m has stopped the search, the lines after the last
            // one selected are context whatever they hold.
            let limit = if done { content.len() } else { found.map_or(content.len(), |(s, _)| s) };
            let mut at = last_end.unwrap_or(0);
            while after_left > 0 && at < limit {
                let end = line_end(at);
                let n = number_at(at);
                self.emit_context(io, label, n, &content[at..end]);
                at = (end + 1).min(content.len());
                last_end = Some(at);
                after_left -= 1;
            }
            let (start, end) = match found {
                Some(f) if !done => f,
                _ => break,
            };
            // The lines before it, as far back as the last line printed.
            let floor = last_end.unwrap_or(0);
            let mut from = start;
            let mut back = 0;
            while back < o.before && from > floor {
                from = memrchr(b'\n', &content[..from - 1]).map_or(0, |i| i + 1);
                back += 1;
            }
            if self.printed && last_end != Some(from) {
                io.put(b"--\n");
            }
            let mut at = from;
            while at < start {
                let e = line_end(at);
                let n = number_at(at);
                self.emit_context(io, label, n, &content[at..e]);
                at = e + 1;
            }
            let n = number_at(start);
            if !self.emit_selected(io, label, n, &content[start..end]) {
                return None;
            }
            self.printed = true;
            count += 1;
            last_end = Some((end + 1).min(content.len()));
            after_left = o.after;
            pos = end + 1;
            if io.stopped() {
                return None;
            }
        }
        Some(count)
    }

    /// Writing into a file that is being searched is an error whatever
    /// the file holds. True if this file is that one.
    fn is_output(&mut self, io: &mut Io, label: &[u8], id: Option<NodeId>) -> bool {
        let o = self.opts;
        if id.is_some() && id == io.out_file && !o.silent() {
            io.err_line(format!("grep: {}: input file is also the output", lossy(label)));
            self.error = true;
            return true;
        }
        false
    }

    /// Search content that is all in one place.
    fn search(&mut self, io: &mut Io, label: &[u8], content: &[u8], id: Option<NodeId>) {
        if self.is_output(io, label, id) {
            return;
        }
        if self.opts.context() {
            if let Some(count) = self.scan_context(io, label, content) {
                self.finish(io, label, count);
            }
            return;
        }
        let mut count = 0;
        if self.scan(io, label, content, 0, &mut count) != Scan::Failed {
            self.finish(io, label, count);
        }
    }

    /// Search a file a piece at a time. Pieces hold whole lines and say
    /// how many, so a piece that storage says cannot match (`filter`) is
    /// stepped over without being read and the line numbers after it are
    /// still right.
    fn search_file(&mut self, db: &Db, io: &mut Io, label: &[u8], id: NodeId, filter: Option<&Shortlist>) {
        if self.is_output(io, label, Some(id)) {
            return;
        }
        if self.opts.context() {
            // Lines around a match can lie in pieces that hold none, so
            // the file is looked at whole.
            if let Some(count) = self.scan_context(io, label, &db.content(id)) {
                self.finish(io, label, count);
            }
            return;
        }
        let mut count = 0;
        let mut lines_before = 0usize;
        for p in db.pieces(id) {
            if filter.is_some_and(|f| !db.piece_passes(f, p)) {
                lines_before += p.lines as usize;
                continue;
            }
            match self.scan(io, label, &db.text_of(p), lines_before, &mut count) {
                Scan::Failed => return,
                Scan::Enough => break,
                Scan::Done => {}
            }
            lines_before += p.lines as usize;
        }
        self.finish(io, label, count);
    }

    /// Search one stretch of whole lines. `lines_before` is how many
    /// lines of the file lie before it; `count` is added to.
    fn scan(&mut self, io: &mut Io, label: &[u8], content: &[u8], lines_before: usize, count: &mut usize) -> Scan {
        let o = self.opts;
        let quiet = o.silent();
        let first_is_enough = o.list || o.without || o.quiet;
        let full = |count: usize| o.max.is_some_and(|m| count >= m);
        if full(*count) {
            return Scan::Enough;
        }
        // Line numbers are counted only up to the lines that are printed:
        // `numbered` lines lie before offset `counted_to`.
        let mut numbered = lines_before;
        let mut counted_to = 0usize;
        let mut pos = 0usize;
        while pos < content.len() {
            let found = match self.searcher.next_line(content, pos) {
                Ok(f) => f,
                Err(_) => {
                    io.err_line(format!("qdb: grep: {}: {}", lossy(label), TOO_COMPLEX));
                    self.error = true;
                    return Scan::Failed;
                }
            };
            if !o.invert {
                let (start, end) = match found {
                    Some(line) => line,
                    None => break,
                };
                *count += 1;
                if first_is_enough {
                    return Scan::Enough;
                }
                if !quiet {
                    if o.line_numbers {
                        numbered += count_newlines(&content[counted_to..start]) + 1;
                        counted_to = (end + 1).min(content.len());
                    }
                    if !self.emit_selected(io, label, numbered, &content[start..end]) || io.stopped() {
                        return Scan::Failed;
                    }
                }
                if full(*count) {
                    return Scan::Enough;
                }
                pos = end + 1;
            } else {
                // Every line before the next match is selected.
                let gap_end = found.map_or(content.len(), |(start, _)| start);
                let mut at = pos;
                while at < gap_end {
                    let end = memchr(b'\n', &content[at..gap_end]).map_or(gap_end, |i| at + i);
                    *count += 1;
                    if first_is_enough {
                        return Scan::Enough;
                    }
                    numbered += 1;
                    if !quiet {
                        self.emit_selected(io, label, numbered, &content[at..end]);
                    }
                    if full(*count) {
                        return Scan::Enough;
                    }
                    at = end + 1;
                }
                if io.stopped() {
                    return Scan::Enough;
                }
                match found {
                    // The matching line is skipped, but still numbered.
                    Some((_, end)) => {
                        numbered += 1;
                        pos = end + 1;
                    }
                    None => break,
                }
            }
        }
        Scan::Done
    }

    /// What follows a file's lines: its name for -l or -L, its count for
    /// -c.
    fn finish(&mut self, io: &mut Io, label: &[u8], count: usize) {
        let o = self.opts;
        if count > 0 {
            self.selected = true;
            // Hand the lines on now: a reader that wants only a few can
            // then stop the search after this file.
            io.flush();
        }
        if o.quiet {
            self.done = count > 0;
        } else if o.list || o.without {
            if (count > 0) == o.list {
                io.put(label);
                io.put(b"\n");
            }
        } else if o.count {
            self.line.clear();
            if self.with_filename {
                self.line.extend_from_slice(label);
                self.line.push(b':');
            }
            push_decimal(&mut self.line, count);
            self.line.push(b'\n');
            io.put(&self.line);
        }
    }
}

/// How the search of one stretch of a file ended.
#[derive(PartialEq)]
enum Scan {
    /// Searched to its end.
    Done,
    /// Nothing more of this file is wanted.
    Enough,
    /// Stopped by an error or by the reader; nothing follows.
    Failed,
}
