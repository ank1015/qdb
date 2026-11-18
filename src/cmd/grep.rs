//! grep with -r -i -n -l -v -c -E -F.
//!
//! This file is the command: options, which files, and what to print. The
//! search itself, finding the matching lines of a buffer, is `scan.rs`.

use super::{options, LineBuf};
use crate::bytes::{count_newlines, memchr, push_decimal};
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use crate::regex::{Flavor, Regex, Syntax};
use crate::scan::Searcher;
use crate::tree::{Db, NodeId};
use crate::util::lossy;

#[derive(Clone, Copy)]
struct Opts {
    recursive: bool,
    line_numbers: bool,
    list: bool,
    invert: bool,
    count: bool,
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
}

impl Plan {
    /// Compile the pattern. The error is the exit status, with the message
    /// already written.
    fn searcher(&self, io: &mut Io) -> Result<Searcher, i32> {
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
        // A newline in the pattern separates alternative patterns.
        let parts: Vec<&[u8]> = pattern.split(|c| *c == b'\n').collect();
        let searcher = if self.mode == Some(b'F') {
            Searcher::fixed(parts.iter().map(|p| p.to_vec()).collect(), self.icase)
        } else {
            let syn = Syntax { extended: self.mode == Some(b'E'), icase: self.icase, flavor: Flavor::Grep };
            let mut res = Vec::new();
            for part in &parts {
                match Regex::new(part, syn) {
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
            Searcher::regex(res)
        };
        // GNU grep sees that nothing can be selected and stops at once.
        if self.opts.invert && pattern.is_empty() {
            return Err(1);
        }
        Ok(searcher)
    }
}

const TOO_COMPLEX: &str = "pattern too complex for the backtracking matcher";

/// grep reading its standard input in pieces.
struct GrepFilter {
    plan: Plan,
    /// Compiled when the first input arrives.
    searcher: Option<Result<Searcher, i32>>,
    lines: LineBuf,
    line_no: usize,
    count: usize,
    error: bool,
}

impl GrepFilter {
    fn start(&mut self, io: &mut Io) -> bool {
        if self.searcher.is_none() {
            self.searcher = Some(self.plan.searcher(io));
        }
        matches!(self.searcher, Some(Ok(_)))
    }

    /// One line, with or without its newline. Returns false when no more
    /// input is needed.
    fn line(&mut self, line: &[u8], io: &mut Io) -> bool {
        let o = self.plan.opts;
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        self.line_no += 1;
        let searcher = self.searcher.as_mut().and_then(|s| s.as_mut().ok()).expect("started");
        let hit = match searcher.is_match(text) {
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
        if o.list {
            return false;
        }
        if !o.count {
            if o.line_numbers {
                io.put(format!("{}:", self.line_no).as_bytes());
            }
            io.put(text);
            io.put(b"\n");
        }
        true
    }
}

impl Filter for GrepFilter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        if !self.start(io) || self.error {
            return false;
        }
        let mut lines = std::mem::take(&mut self.lines);
        let more = lines.feed(data, |line| self.line(line, io));
        self.lines = lines;
        more
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if !self.start(io) {
            return self.searcher.and_then(|s| s.err()).unwrap_or(2);
        }
        if !self.error && !(self.plan.opts.list && self.count > 0) {
            if let Some(last) = self.lines.rest() {
                self.line(&last, io);
            }
        }
        let o = self.plan.opts;
        if o.list {
            if self.count > 0 {
                io.put(b"(standard input)\n");
            }
        } else if o.count {
            io.put(format!("{}\n", self.count).as_bytes());
        }
        if self.error {
            2
        } else if self.count > 0 {
            0
        } else {
            1
        }
    }
}

pub fn grep(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let p = match options("grep", &args, b"rinlvcEF", b"", 2)? {
        Ok(p) => p,
        Err(r) => return Ok(r.into()),
    };
    let opts = Opts {
        recursive: p.has(b'r'),
        line_numbers: p.has(b'n'),
        list: p.has(b'l'),
        invert: p.has(b'v'),
        count: p.has(b'c'),
    };
    let icase = p.has(b'i');
    let conflicting = p.has(b'E') && p.has(b'F');
    let mode = p.flags.iter().map(|f| f.0).find(|c| *c == b'E' || *c == b'F');
    let mut operands = p.operands.into_iter();
    let pattern = operands.next();
    let files: Vec<Vec<u8>> = operands.collect();
    let plan = Plan { opts, icase, conflicting, mode, pattern };

    let input = if files.is_empty() && !opts.recursive {
        Input::Stream(Box::new(GrepFilter { plan: plan.clone(), searcher: None, lines: LineBuf::default(), line_no: 0, count: 0, error: false }))
    } else if files.iter().any(|f| f == b"-") {
        Input::Whole
    } else {
        Input::Unused
    };

    let run: Runner = Box::new(move |db, io| {
        let searcher = match plan.searcher(io) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let implicit_dot = files.is_empty() && opts.recursive;
        let single_dir = files.len() == 1 && db.resolve(&files[0]).is_ok_and(|id| db.is_dir(id));
        let with_filename = files.len() > 1 || opts.recursive && (implicit_dot || single_dir);
        let mut run = Run { opts, searcher, with_filename, selected: false, error: false, line: Vec::new() };
        if implicit_dot {
            let mut path = Vec::new();
            run.walk(db, io, db.root(), &mut path);
        } else if files.is_empty() {
            let data = io.take_stdin();
            run.search(io, b"(standard input)", data, None);
        }
        for f in &files {
            if io.stopped() {
                break;
            }
            if f == b"-" {
                let data = io.take_stdin();
                run.search(io, b"(standard input)", data, None);
                continue;
            }
            match db.resolve(f) {
                Err(e) => {
                    io.err_line(format!("grep: {}: {}", lossy(f), e.msg()));
                    run.error = true;
                }
                Ok(id) if db.is_dir(id) => {
                    if !opts.recursive {
                        io.err_line(format!("grep: {}: Is a directory", lossy(f)));
                        run.error = true;
                        // GNU grep still reports a count for it.
                        if opts.count && !opts.list {
                            run.search(io, f, b"", None);
                        }
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
                    run.walk(db, io, id, &mut base);
                }
                Ok(id) => run.search(io, f, db.content(id), Some(id)),
            }
        }
        if run.error {
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
    with_filename: bool,
    selected: bool,
    error: bool,
    /// Scratch for putting an output line together.
    line: Vec<u8>,
}

impl Run {
    /// Search every file under a folder, depth first in byte order.
    /// `path` is what goes in front of each child's name; it is restored
    /// before returning.
    fn walk(&mut self, db: &Db, io: &mut Io, dir: NodeId, path: &mut Vec<u8>) {
        let base = path.len();
        for (name, child) in db.children(dir) {
            if io.stopped() {
                return;
            }
            path.extend_from_slice(name);
            if db.is_dir(*child) {
                path.push(b'/');
                self.walk(db, io, *child, path);
            } else {
                self.search(io, path, db.content(*child), Some(*child));
            }
            path.truncate(base);
        }
    }

    /// Print one selected line: `label:number:text`.
    fn emit(&mut self, io: &mut Io, label: &[u8], number: usize, text: &[u8]) {
        self.line.clear();
        if self.with_filename {
            self.line.extend_from_slice(label);
            self.line.push(b':');
        }
        if self.opts.line_numbers {
            push_decimal(&mut self.line, number);
            self.line.push(b':');
        }
        io.put(&self.line);
        io.put(text);
        io.put(b"\n");
    }

    fn search(&mut self, io: &mut Io, label: &[u8], content: &[u8], id: Option<NodeId>) {
        let o = self.opts;
        if id.is_some() && id == io.out_file && !o.count && !o.list {
            io.err_line(format!("grep: {}: input file is also the output", lossy(label)));
            self.error = true;
            return;
        }
        let quiet = o.list || o.count;
        let mut count = 0usize;
        // Line numbers are counted only up to the lines that are printed:
        // `numbered` lines lie before offset `counted_to`.
        let mut numbered = 0usize;
        let mut counted_to = 0usize;
        let mut pos = 0usize;
        while pos < content.len() {
            let found = match self.searcher.next_line(content, pos) {
                Ok(f) => f,
                Err(_) => {
                    io.err_line(format!("qdb: grep: {}: {}", lossy(label), TOO_COMPLEX));
                    self.error = true;
                    return;
                }
            };
            if !o.invert {
                let (start, end) = match found {
                    Some(line) => line,
                    None => break,
                };
                count += 1;
                if o.list {
                    break;
                }
                if !quiet {
                    if o.line_numbers {
                        numbered += count_newlines(&content[counted_to..start]) + 1;
                        counted_to = (end + 1).min(content.len());
                    }
                    self.emit(io, label, numbered, &content[start..end]);
                    if io.stopped() {
                        return;
                    }
                }
                pos = end + 1;
            } else {
                // Every line before the next match is selected.
                let gap_end = found.map_or(content.len(), |(start, _)| start);
                let mut at = pos;
                while at < gap_end {
                    let end = memchr(b'\n', &content[at..gap_end]).map_or(gap_end, |i| at + i);
                    count += 1;
                    if o.list {
                        break;
                    }
                    if !quiet {
                        numbered += 1;
                        self.emit(io, label, numbered, &content[at..end]);
                    }
                    at = end + 1;
                }
                if (o.list && count > 0) || io.stopped() {
                    break;
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
        if count > 0 {
            self.selected = true;
            // Hand the lines on now: a reader that wants only a few can
            // then stop the search after this file.
            io.flush();
        }
        if o.list {
            if count > 0 {
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
