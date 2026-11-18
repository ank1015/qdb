//! find: tests on names, paths, kinds, sizes and times, joined by the
//! operators, with the actions -print, -print0, -prune and -exec. Results
//! come out depth first with each folder's children in byte order.

use crate::engine::{self, Io, Runner};
use crate::glob::{self, Tok};
use crate::tree::{Db, NodeId};
use crate::util::{basename, join, lossy, quote};

#[derive(Clone, Copy)]
enum Cmp {
    Less,
    Equal,
    More,
}

enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Name(Vec<Tok>),
    Path(Vec<Tok>),
    Type(u8),
    Empty,
    /// Size in units of `unit` bytes, rounded up.
    Size(Cmp, u64, u64),
    /// Modified after this file was; resolved when find starts.
    Newer(Vec<u8>),
    /// Modified so many minutes ago.
    Mmin(Cmp, f64),
    /// Modified so many days ago.
    Mtime(Cmp, f64),
    Print,
    Print0,
    Prune,
    /// `-exec ... ;`: the index of the command in `Find::execs`.
    Exec(usize),
    /// `-exec ... {} +`
    ExecPlus(usize),
}

struct Find {
    expr: Expr,
    maxdepth: Option<usize>,
    mindepth: usize,
    execs: Vec<Vec<Vec<u8>>>,
    has_exec: bool,
}

struct Parser<'a> {
    args: &'a [Vec<u8>],
    i: usize,
    maxdepth: Option<usize>,
    mindepth: usize,
    execs: Vec<Vec<Vec<u8>>>,
    /// True once an action that prints or runs something was given: the
    /// default -print is then not added.
    action: bool,
    /// An error GNU find reports itself.
    usage: Option<String>,
}

enum Stop {
    Unsupported(String),
    Usage(String),
}

impl Parser<'_> {
    fn peek(&self) -> Option<&[u8]> {
        self.args.get(self.i).map(|a| a.as_slice())
    }

    fn or(&mut self) -> Result<Expr, Stop> {
        let mut left = self.and()?;
        while matches!(self.peek(), Some(b"-o") | Some(b"-or")) {
            self.i += 1;
            if matches!(self.peek(), None | Some(b")")) {
                return Err(Stop::Usage("find: expected an expression after '-o'".into()));
            }
            let right = self.and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, Stop> {
        let mut left = self.not()?;
        loop {
            match self.peek() {
                None | Some(b")") | Some(b"-o") | Some(b"-or") => break,
                Some(b"-a") | Some(b"-and") => {
                    self.i += 1;
                    if matches!(self.peek(), None | Some(b")")) {
                        return Err(Stop::Usage("find: expected an expression after '-a'".into()));
                    }
                }
                _ => {}
            }
            let right = self.not()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn not(&mut self) -> Result<Expr, Stop> {
        if matches!(self.peek(), Some(b"!") | Some(b"-not")) {
            self.i += 1;
            if matches!(self.peek(), None | Some(b")")) {
                return Err(Stop::Usage("find: expected an expression after '!'".into()));
            }
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.primary()
    }

    fn arg(&mut self, of: &[u8]) -> Result<&[u8], Stop> {
        match self.args.get(self.i) {
            Some(a) => {
                self.i += 1;
                Ok(a)
            }
            None => Err(Stop::Usage(format!("find: missing argument to `{}'", lossy(of)))),
        }
    }

    fn primary(&mut self) -> Result<Expr, Stop> {
        let tok = self.args[self.i].clone();
        let tok = tok.as_slice();
        self.i += 1;
        Ok(match tok {
            b"(" => {
                if self.peek() == Some(b")") {
                    return Err(Stop::Usage("find: invalid expression; empty parentheses are not allowed.".into()));
                }
                if self.peek().is_none() {
                    return Err(Stop::Usage(
                        "find: invalid expression; expected to find a ')' but didn't see one. Perhaps you need an extra predicate after '('".into(),
                    ));
                }
                let inner = self.or()?;
                if self.peek() != Some(b")") {
                    return Err(Stop::Usage(
                        "find: invalid expression; I was expecting to find a ')' somewhere but did not see one.".into(),
                    ));
                }
                self.i += 1;
                inner
            }
            b")" => return Err(Stop::Usage("find: you have too many ')'".into())),
            b"-o" | b"-or" | b"-a" | b"-and" => {
                let shown = if tok.starts_with(b"-o") { "-o" } else { "-a" };
                return Err(Stop::Usage(format!(
                    "find: invalid expression; you have used a binary operator '{}' with nothing before it.",
                    shown
                )));
            }
            b"-name" | b"-iname" | b"-path" | b"-ipath" => {
                let mut toks = glob::compile(&glob::fnmatch_pattern(self.arg(tok)?));
                if tok[1] == b'i' {
                    glob::fold(&mut toks);
                }
                if tok.ends_with(b"name") {
                    Expr::Name(toks)
                } else {
                    Expr::Path(toks)
                }
            }
            b"-type" => {
                let arg = self.arg(tok)?;
                match arg {
                    b"f" | b"d" => Expr::Type(arg[0]),
                    [c] if b"bcpls".contains(c) => return Err(Stop::Unsupported(format!("flag: find -type {}", char::from(*c)))),
                    _ if arg.contains(&b',') => return Err(Stop::Unsupported("flag: find -type with a list of types".into())),
                    _ => return Err(Stop::Usage(format!("find: Unknown argument to -type: {}", lossy(arg)))),
                }
            }
            b"-maxdepth" | b"-mindepth" => {
                let arg = self.arg(tok)?;
                if arg.is_empty() || !arg.iter().all(|c| c.is_ascii_digit()) {
                    return Err(Stop::Usage(format!(
                        "find: Expected a positive decimal integer argument to {}, but got {}",
                        lossy(tok),
                        quote(arg)
                    )));
                }
                let n = arg.iter().fold(0usize, |n, c| n.saturating_mul(10).saturating_add((c - b'0') as usize));
                if tok == b"-maxdepth" {
                    self.maxdepth = Some(n);
                } else {
                    self.mindepth = n;
                }
                // An option: always true.
                Expr::Not(Box::new(Expr::Type(0)))
            }
            b"-empty" => Expr::Empty,
            b"-size" => {
                let arg = self.arg(tok)?;
                if arg.is_empty() {
                    return Err(Stop::Usage("find: invalid null argument to -size".into()));
                }
                let invalid = || Stop::Usage(format!("find: Invalid argument `{}' to -size", lossy(arg)));
                let (number, unit) = match arg[arg.len() - 1] {
                    b'b' => (&arg[..arg.len() - 1], 512),
                    b'c' => (&arg[..arg.len() - 1], 1),
                    b'w' => (&arg[..arg.len() - 1], 2),
                    b'k' => (&arg[..arg.len() - 1], 1024),
                    b'M' => (&arg[..arg.len() - 1], 1 << 20),
                    b'G' => (&arg[..arg.len() - 1], 1 << 30),
                    c if c.is_ascii_digit() => (arg, 512),
                    c => return Err(Stop::Usage(format!("find: invalid -size type `{}'", char::from(c)))),
                };
                let (cmp, rest) = compare(number);
                if rest.is_empty() || !rest.iter().all(|c| c.is_ascii_digit()) {
                    return Err(invalid());
                }
                let n: u64 = lossy(rest).parse().map_err(|_| invalid())?;
                Expr::Size(cmp, n, unit)
            }
            b"-newer" => Expr::Newer(self.arg(tok)?.to_vec()),
            b"-mmin" | b"-mtime" => {
                let arg = self.arg(tok)?;
                let (cmp, rest) = compare(arg);
                let n: f64 = match lossy(rest).parse() {
                    Ok(n) if rest.first().is_some_and(|c| c.is_ascii_digit() || *c == b'.') => n,
                    _ => return Err(Stop::Usage(format!("find: invalid argument `{}' to `{}'", lossy(arg), lossy(tok)))),
                };
                // GNU find reads `-mtime -0` as it reads `-mtime -1`:
                // changed within the last day.
                let n = if tok == b"-mtime" && n == 0.0 && arg.starts_with(b"-") { 1.0 } else { n };
                if tok == b"-mmin" {
                    Expr::Mmin(cmp, n)
                } else {
                    Expr::Mtime(cmp, n)
                }
            }
            b"-print" => {
                self.action = true;
                Expr::Print
            }
            b"-print0" => {
                self.action = true;
                Expr::Print0
            }
            b"-prune" => Expr::Prune,
            b"-exec" => {
                self.action = true;
                // `+` ends the command only straight after `{}`.
                let mut j = self.i;
                let mut found = None;
                while j < self.args.len() {
                    if self.args[j] == b";" {
                        found = Some((j, false));
                        break;
                    }
                    if self.args[j] == b"+" && j > self.i && self.args[j - 1] == b"{}" {
                        found = Some((j, true));
                        break;
                    }
                    j += 1;
                }
                let (end, plus) = match found {
                    Some((end, false)) if end == self.i => {
                        return Err(Stop::Usage("find: invalid argument `;' to `-exec'".into()))
                    }
                    Some(f) if f.0 > self.i => f,
                    _ => return Err(Stop::Usage("find: missing argument to `-exec'".into())),
                };
                let command: Vec<Vec<u8>> = self.args[self.i..end].to_vec();
                self.i = end + 1;
                if plus && command[..command.len() - 1].iter().any(|a| a.windows(2).any(|w| w == b"{}")) {
                    return Err(Stop::Usage("find: Only one instance of {} is supported with -exec ... +".into()));
                }
                // Refuse now what the command could not run.
                let probe: Vec<Vec<u8>> = command.iter().map(|a| crate::testing::replace(a, b"{}", b"x")).collect();
                if probe[0] == b":" {
                    return Err(Stop::Unsupported("command: : (it is part of the shell)".into()));
                }
                if let Err(what) = super::prepare(&probe[0], &probe[1..]) {
                    return Err(Stop::Unsupported(what));
                }
                self.execs.push(command);
                if plus {
                    Expr::ExecPlus(self.execs.len() - 1)
                } else {
                    Expr::Exec(self.execs.len() - 1)
                }
            }
            _ if tok.starts_with(b"-") && tok.len() > 1 => return Err(Stop::Unsupported(format!("flag: find {}", lossy(tok)))),
            _ => return Err(Stop::Usage(format!("find: paths must precede expression: `{}'", lossy(tok)))),
        })
    }
}

/// `+N`, `-N` or `N`.
fn compare(arg: &[u8]) -> (Cmp, &[u8]) {
    match arg.first() {
        Some(b'+') => (Cmp::More, &arg[1..]),
        Some(b'-') => (Cmp::Less, &arg[1..]),
        _ => (Cmp::Equal, arg),
    }
}

pub fn find(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let split = args
        .iter()
        .position(|a| a.starts_with(b"-") && a.len() > 1 || matches!(a.as_slice(), b"(" | b")" | b"!" | b","))
        .unwrap_or(args.len());
    let mut starts: Vec<Vec<u8>> = args[..split].to_vec();
    if starts.is_empty() {
        starts.push(b".".to_vec());
    }
    if args[split..].iter().any(|a| a == b",") {
        return Err("flag: find , (the comma operator)".into());
    }
    let mut p = Parser { args: &args[split..], i: 0, maxdepth: None, mindepth: 0, execs: Vec::new(), action: false, usage: None };
    let mut expr = None;
    if !p.args.is_empty() {
        match p.or() {
            Ok(e) => {
                if p.peek() == Some(b")") {
                    p.usage = Some("find: you have too many ')'".into());
                }
                expr = Some(e);
            }
            Err(Stop::Unsupported(what)) => return Err(what),
            Err(Stop::Usage(msg)) => {
                // A flag this version does not know, later on the line,
                // is still refused rather than reported as find would.
                p.usage = Some(msg);
            }
        }
    }
    if p.usage.is_some() {
        if let Some(bad) = args[split..].iter().find(|a| a.starts_with(b"-") && a.len() > 1 && !KNOWN.contains(&a.as_slice())) {
            // Only where it stands as a test, not as the argument of one.
            let at = args[split..].iter().position(|a| a == bad).unwrap();
            let after_arg = at > 0 && TAKES_ARG.contains(&args[split + at - 1].as_slice());
            if !after_arg && !args[split..].iter().any(|a| a == b"-exec") {
                return Err(format!("flag: find {}", lossy(bad)));
            }
        }
    }
    let expr = match (expr, p.action) {
        (None, _) => Expr::Print,
        (Some(e), true) => e,
        (Some(e), false) => Expr::And(Box::new(e), Box::new(Expr::Print)),
    };
    let usage = p.usage.take();
    let has_exec = !p.execs.is_empty();
    let find = Find { expr, maxdepth: p.maxdepth, mindepth: p.mindepth, execs: p.execs, has_exec };
    Ok(Box::new(move |db, io| {
        if let Some(e) = usage {
            io.err_line(e);
            return 1;
        }
        let mut run = Run { find: &find, now: db.now(), newer: Vec::new(), batches: find.execs.iter().map(|_| Vec::new()).collect(), status: 0 };
        // -newer names a file that must be there.
        let mut refs = Vec::new();
        collect_newer(&find.expr, &mut refs);
        for r in refs {
            match db.resolve(r) {
                Ok(id) => run.newer.push((r.to_vec(), db.mtime(id))),
                Err(e) => {
                    io.err_line(format!("find: {}: {}", quote(r), e.msg()));
                    return 1;
                }
            }
        }
        for start in &starts {
            match db.resolve(start) {
                Err(e) => {
                    io.err_line(format!("find: {}: {}", quote(start), e.msg()));
                    run.status = 1;
                }
                Ok(id) => {
                    if find.has_exec {
                        run.visit_rw(db, start, id, 0, io);
                    } else {
                        run.visit(db, start, basename(start), id, 0, io);
                    }
                }
            }
        }
        for i in 0..find.execs.len() {
            run.flush(db, i, io);
        }
        run.status
    }))
}

const KNOWN: &[&[u8]] = &[
    b"-name", b"-iname", b"-path", b"-ipath", b"-type", b"-maxdepth", b"-mindepth", b"-empty", b"-size", b"-newer", b"-mmin",
    b"-mtime", b"-print", b"-print0", b"-prune", b"-exec", b"-o", b"-or", b"-a", b"-and", b"-not",
];
const TAKES_ARG: &[&[u8]] =
    &[b"-name", b"-iname", b"-path", b"-ipath", b"-type", b"-maxdepth", b"-mindepth", b"-size", b"-newer", b"-mmin", b"-mtime"];

fn collect_newer<'a>(e: &'a Expr, out: &mut Vec<&'a [u8]>) {
    match e {
        Expr::And(a, b) | Expr::Or(a, b) => {
            collect_newer(a, out);
            collect_newer(b, out);
        }
        Expr::Not(a) => collect_newer(a, out),
        Expr::Newer(f) => out.push(f),
        _ => {}
    }
}

/// The most a batch of `-exec ... +` holds, in bytes of arguments: the
/// 128 KiB findutils allows itself, less its headroom.
const BATCH_BYTES: usize = 128 * 1024 - 4096;

struct Run<'a> {
    find: &'a Find,
    now: i64,
    newer: Vec<(Vec<u8>, i64)>,
    /// Paths waiting for each `-exec ... +`.
    batches: Vec<Vec<Vec<u8>>>,
    status: i32,
}

/// What evaluating the expression can do besides answer.
enum Act<'a> {
    /// Nothing that changes data: the tree is only read.
    Read(&'a Db),
    Write(&'a mut Db),
}

impl Act<'_> {
    fn db(&self) -> &Db {
        match self {
            Act::Read(db) => db,
            Act::Write(db) => db,
        }
    }
}

impl Run<'_> {
    /// The walk when nothing is run: children are read in place.
    fn visit(&mut self, db: &Db, path: &[u8], name: &[u8], id: NodeId, depth: usize, io: &mut Io) {
        if io.stopped() {
            return;
        }
        let mut prune = false;
        if depth >= self.find.mindepth {
            self.eval(&self.find.expr, &mut Act::Read(db), path, name, id, &mut prune, io);
        }
        if !prune && db.is_dir(id) && self.find.maxdepth.map_or(true, |m| depth < m) {
            for (child_name, child) in db.children(id) {
                self.visit(db, &join(path, child_name), child_name, *child, depth + 1, io);
            }
        }
    }

    /// The walk when commands run: they may change the tree, so each
    /// folder's entries are noted before any of them is visited.
    fn visit_rw(&mut self, db: &mut Db, path: &[u8], id: NodeId, depth: usize, io: &mut Io) {
        let mut prune = false;
        let was_dir = db.is_dir(id);
        if depth >= self.find.mindepth {
            let name = basename(path).to_vec();
            self.eval(&self.find.expr, &mut Act::Write(db), path, &name, id, &mut prune, io);
        }
        if !db.exists(id) {
            // A command removed it. find reports a folder it can no
            // longer read.
            if was_dir && !prune {
                io.err_line(format!("find: {}: No such file or directory", quote(path)));
                self.status = 1;
            }
            return;
        }
        if !prune && db.is_dir(id) && self.find.maxdepth.map_or(true, |m| depth < m) {
            let children: Vec<(Vec<u8>, NodeId)> = db.children(id).iter().map(|(n, c)| (n.clone(), *c)).collect();
            for (child_name, child) in children {
                if db.exists(child) {
                    self.visit_rw(db, &join(path, &child_name), child, depth + 1, io);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn eval(&mut self, e: &Expr, act: &mut Act, path: &[u8], name: &[u8], id: NodeId, prune: &mut bool, io: &mut Io) -> bool {
        let db = act.db();
        match e {
            Expr::And(a, b) => self.eval(a, act, path, name, id, prune, io) && self.eval(b, act, path, name, id, prune, io),
            Expr::Or(a, b) => self.eval(a, act, path, name, id, prune, io) || self.eval(b, act, path, name, id, prune, io),
            Expr::Not(a) => !self.eval(a, act, path, name, id, prune, io),
            Expr::Name(toks) => glob::matches(toks, name),
            Expr::Path(toks) => glob::matches(toks, path),
            Expr::Type(t) => *t == if db.is_dir(id) { b'd' } else { b'f' },
            Expr::Empty => {
                if db.is_dir(id) {
                    db.children(id).is_empty()
                } else {
                    db.size(id) == 0
                }
            }
            Expr::Size(cmp, n, unit) => {
                if db.is_dir(id) {
                    return false;
                }
                let size = db.size(id).div_ceil(*unit);
                match cmp {
                    Cmp::Less => size < *n,
                    Cmp::Equal => size == *n,
                    Cmp::More => size > *n,
                }
            }
            Expr::Newer(f) => {
                let than = self.newer.iter().find(|(name, _)| name == f).map_or(0, |(_, t)| *t);
                db.mtime(id) > than
            }
            Expr::Mmin(cmp, n) => {
                let t = db.mtime(id) as f64;
                let origin = self.now as f64 - n * 60e9;
                match cmp {
                    Cmp::Less => t > origin,
                    Cmp::More => t < origin,
                    Cmp::Equal => t > origin && t - origin <= 60e9,
                }
            }
            Expr::Mtime(cmp, n) => {
                let t = db.mtime(id) as f64;
                const DAY: f64 = 86400e9;
                match cmp {
                    // Less than n days old.
                    Cmp::Less => t > self.now as f64 - 1.0 - n * DAY,
                    // More than n whole days old: at least n + 1 days.
                    Cmp::More => t < self.now as f64 - DAY - n * DAY,
                    Cmp::Equal => {
                        let origin = self.now as f64 - DAY - n * DAY;
                        t > origin && t - origin <= DAY
                    }
                }
            }
            Expr::Print => {
                io.put(path);
                io.put(b"\n");
                true
            }
            Expr::Print0 => {
                io.put(path);
                io.put(b"\0");
                true
            }
            Expr::Prune => {
                *prune = true;
                true
            }
            Expr::Exec(i) => {
                let argv: Vec<Vec<u8>> = self.find.execs[*i].iter().map(|a| crate::testing::replace(a, b"{}", path)).collect();
                match act {
                    Act::Write(db) => {
                        io.flush();
                        engine::run_argv(db, &argv, io) == 0
                    }
                    Act::Read(_) => unreachable!("commands run on the walk that can write"),
                }
            }
            Expr::ExecPlus(i) => {
                let fixed: usize = self.find.execs[*i].iter().map(|a| a.len() + 1).sum();
                let waiting: usize = self.batches[*i].iter().map(|a| a.len() + 1).sum();
                if fixed + waiting + path.len() + 1 > BATCH_BYTES {
                    if let Act::Write(db) = act {
                        self.flush(db, *i, io);
                    }
                }
                self.batches[*i].push(path.to_vec());
                true
            }
        }
    }

    /// Run an `-exec ... +` on the paths gathered for it.
    fn flush(&mut self, db: &mut Db, i: usize, io: &mut Io) {
        let paths = std::mem::take(&mut self.batches[i]);
        if paths.is_empty() {
            return;
        }
        let command = &self.find.execs[i];
        let mut argv: Vec<Vec<u8>> = command[..command.len() - 1].to_vec();
        argv.extend(paths);
        if engine::run_argv(db, &argv, io) != 0 {
            self.status = 1;
        }
    }
}
