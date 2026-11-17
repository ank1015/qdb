//! grep with -r -i -n -l -v -c -E -F.

use super::options;
use crate::engine::{Io, Runner};
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

struct Opts {
    recursive: bool,
    line_numbers: bool,
    list: bool,
    invert: bool,
    count: bool,
}

struct Run<'a> {
    opts: &'a Opts,
    matcher: &'a Matcher,
    with_filename: bool,
    out: Vec<u8>,
    selected: bool,
    error: bool,
}

pub fn grep(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("grep", &args, b"rinlvcEF", b"", 2)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
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

    Ok(Box::new(move |db, io| {
        let pattern = match pattern {
            Some(p) => p,
            None => {
                io.err_line("Usage: grep [OPTION]... PATTERNS [FILE]...");
                io.err_line("Try 'grep --help' for more information.");
                return 2;
            }
        };
        if conflicting {
            io.err_line("grep: conflicting matchers specified");
            return 2;
        }
        // A newline in the pattern separates alternative patterns.
        let parts: Vec<&[u8]> = pattern.split(|c| *c == b'\n').collect();
        let matcher = if mode == Some(b'F') {
            let needles = parts.iter().map(|p| if icase { p.to_ascii_lowercase() } else { p.to_vec() }).collect();
            Matcher::Fixed(needles, icase)
        } else {
            let syn = Syntax { extended: mode == Some(b'E'), icase, flavor: Flavor::Grep };
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
                        return 2;
                    }
                }
            }
            Matcher::Regex(res)
        };
        // GNU grep sees that nothing can be selected and stops at once.
        if opts.invert && pattern.is_empty() {
            return 1;
        }

        let implicit_dot = files.is_empty() && opts.recursive;
        let single_dir = files.len() == 1 && db.resolve(&files[0]).is_ok_and(|id| db.is_dir(id));
        let with_filename = files.len() > 1 || opts.recursive && (implicit_dot || single_dir);
        let mut run = Run { opts: &opts, matcher: &matcher, with_filename, out: Vec::new(), selected: false, error: false };
        if implicit_dot {
            run.walk(db, io, db.root(), b"");
        } else if files.is_empty() {
            let data = io.take_stdin();
            run.search(io, b"(standard input)", data, None);
        }
        for f in &files {
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
        io.out = run.out;
        if run.error {
            2
        } else if run.selected {
            0
        } else {
            1
        }
    }))
}

impl Run<'_> {
    /// Search every file under a folder, depth first in byte order.
    /// `prefix` is what goes in front of each child's name.
    fn walk(&mut self, db: &Db, io: &mut Io, dir: NodeId, prefix: &[u8]) {
        for (name, child) in db.children(dir) {
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
                self.out.extend_from_slice(label);
                self.out.push(b':');
            }
            if o.line_numbers {
                self.out.extend_from_slice(format!("{}:", idx + 1).as_bytes());
            }
            self.out.extend_from_slice(text);
            self.out.push(b'\n');
        }
        if count > 0 {
            self.selected = true;
        }
        if o.list {
            if count > 0 {
                self.out.extend_from_slice(label);
                self.out.push(b'\n');
            }
        } else if o.count {
            if self.with_filename {
                self.out.extend_from_slice(label);
                self.out.push(b':');
            }
            self.out.extend_from_slice(format!("{}\n", count).as_bytes());
        }
    }
}
