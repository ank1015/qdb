//! grep with -r -i -n -l -v -c -E -F.

use super::{options, LineBuf};
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use crate::regex::{Flavor, Regex, Syntax};
use crate::tree::{Db, NodeId};
use crate::util::{lines, lossy};

enum Matcher {
    Regex(Vec<Regex>),
    /// Fixed strings, lowercased when matching without regard to case.
    Fixed(Vec<Vec<u8>>, bool),
}

impl Matcher {
    fn is_match(&self, line: &[u8]) -> Result<bool, ()> {
        match self {
            Matcher::Regex(res) => {
                for re in res {
                    if re.is_match(line).map_err(|_| ())? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Matcher::Fixed(needles, icase) => {
                let folded;
                let hay = if *icase {
                    folded = line.to_ascii_lowercase();
                    &folded[..]
                } else {
                    line
                };
                Ok(needles.iter().any(|n| contains(hay, n)))
            }
        }
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > hay.len() {
        return false;
    }
    let first = needle[0];
    (0..=hay.len() - needle.len()).any(|i| hay[i] == first && &hay[i..i + needle.len()] == needle)
}

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
    fn matcher(&self, io: &mut Io) -> Result<Matcher, i32> {
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
        let matcher = if self.mode == Some(b'F') {
            let needles = parts.iter().map(|p| if self.icase { p.to_ascii_lowercase() } else { p.to_vec() }).collect();
            Matcher::Fixed(needles, self.icase)
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
            Matcher::Regex(res)
        };
        // GNU grep sees that nothing can be selected and stops at once.
        if self.opts.invert && pattern.is_empty() {
            return Err(1);
        }
        Ok(matcher)
    }
}

struct Run<'a> {
    opts: &'a Opts,
    matcher: &'a Matcher,
    with_filename: bool,
    selected: bool,
    error: bool,
}

/// grep reading its standard input in pieces.
struct GrepFilter {
    plan: Plan,
    /// Compiled when the first input arrives.
    matcher: Option<Result<Matcher, i32>>,
    lines: LineBuf,
    line_no: usize,
    count: usize,
    error: bool,
}

impl GrepFilter {
    fn start(&mut self, io: &mut Io) -> bool {
        if self.matcher.is_none() {
            self.matcher = Some(self.plan.matcher(io));
        }
        matches!(self.matcher, Some(Ok(_)))
    }

    /// One line, with or without its newline. Returns false when no more
    /// input is needed.
    fn line(&mut self, line: &[u8], io: &mut Io) -> bool {
        let o = self.plan.opts;
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        self.line_no += 1;
        let hit = match self.matcher.as_ref().unwrap().as_ref().ok().unwrap().is_match(text) {
            Ok(h) => h,
            Err(()) => {
                io.err_line("qdb: grep: (standard input): pattern too complex for the backtracking matcher");
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
            return self.matcher.unwrap().err().unwrap();
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
        Input::Stream(Box::new(GrepFilter { plan: plan.clone(), matcher: None, lines: LineBuf::default(), line_no: 0, count: 0, error: false }))
    } else if files.iter().any(|f| f == b"-") {
        Input::Whole
    } else {
        Input::Unused
    };

    let run: Runner = Box::new(move |db, io| {
        let matcher = match plan.matcher(io) {
            Ok(m) => m,
            Err(code) => return code,
        };
        let implicit_dot = files.is_empty() && opts.recursive;
        let single_dir = files.len() == 1 && db.resolve(&files[0]).is_ok_and(|id| db.is_dir(id));
        let with_filename = files.len() > 1 || opts.recursive && (implicit_dot || single_dir);
        let mut run = Run { opts: &opts, matcher: &matcher, with_filename, selected: false, error: false };
        if implicit_dot {
            run.walk(db, io, db.root(), b"");
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
                    run.walk(db, io, id, &base);
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

impl Run<'_> {
    /// Search every file under a folder, depth first in byte order.
    /// `prefix` is what goes in front of each child's name.
    fn walk(&mut self, db: &Db, io: &mut Io, dir: NodeId, prefix: &[u8]) {
        for (name, child) in db.children(dir) {
            if io.stopped() {
                return;
            }
            let mut path = prefix.to_vec();
            path.extend_from_slice(name);
            if db.is_dir(*child) {
                path.push(b'/');
                self.walk(db, io, *child, &path);
            } else {
                self.search(io, &path, db.content(*child), Some(*child));
            }
        }
    }

    fn search(&mut self, io: &mut Io, label: &[u8], content: &[u8], id: Option<NodeId>) {
        let o = self.opts;
        if id.is_some() && id == io.out_file && !o.count && !o.list {
            io.err_line(format!("grep: {}: input file is also the output", lossy(label)));
            self.error = true;
            return;
        }
        let mut count = 0usize;
        for (idx, line) in lines(content).enumerate() {
            let text = line.strip_suffix(b"\n").unwrap_or(line);
            let hit = match self.matcher.is_match(text) {
                Ok(h) => h,
                Err(()) => {
                    io.err_line(format!("qdb: grep: {}: pattern too complex for the backtracking matcher", lossy(label)));
                    self.error = true;
                    return;
                }
            };
            if hit == o.invert {
                continue;
            }
            count += 1;
            if o.list {
                break;
            }
            if o.count {
                continue;
            }
            if self.with_filename {
                io.put(label);
                io.put(b":");
            }
            if o.line_numbers {
                io.put(format!("{}:", idx + 1).as_bytes());
            }
            io.put(text);
            io.put(b"\n");
        }
        if count > 0 {
            self.selected = true;
            // Hand the lines on now: a reader that wants only a few
            // can then stop the search after this file.
            io.flush();
        }
        if o.list {
            if count > 0 {
                io.put(label);
                io.put(b"\n");
            }
        } else if o.count {
            if self.with_filename {
                io.put(label);
                io.put(b":");
            }
            io.put(format!("{}\n", count).as_bytes());
        }
    }
}
