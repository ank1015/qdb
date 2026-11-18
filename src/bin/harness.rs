//! The test harness: runs each command on the real GNU tools and on the
//! engine, and compares standard output, standard error, exit status and
//! the resulting tree.
//!
//! See tests/README.md for the case file format.

use qdb::store;
use qdb::testing::{check_safe, order_is_unspecified, run_real, same_output, sorted_lines};
use qdb::{execute, Config, Db, Outcome, SyncMode};
use std::path::{Path, PathBuf};

const USAGE: &str = "\
Usage: qdb-harness [OPTIONS] [CASE_FILE]...

Compares the engine with the real GNU tools, case by case.
With no case files, runs every tests/cases/*.case.

  --fixtures DIR    where fixtures live (default tests/fixtures)
  --filter TEXT     only run cases whose command contains TEXT
  --fuzz N          also run N generated regex cases against grep and sed
  --seed N          seed for --fuzz (default 1)
  --list-failures   print one line per failure instead of full details
  --no-index        run the engine with its search index switched off
  --store MODE      where the engine keeps its data: memory (default), disk
                    (a database directory, checked with --check's checks
                    when each case ends), or reopen (the same, closed and
                    opened again after every step, and checked each time)
  --sync MODE       with disk or reopen: always (default) or batch
  --piece BYTES     cut file content into pieces of at most this many bytes
                    (default 65536); a small value makes small test files
                    span many pieces
  --segment BYTES   start a new data file after this many bytes
  --diff N          instead of the cases: run N generated steps, searches
                    mixed with writes, on two engines, one with the index
                    and one without, and require the same results; then
                    check the maintained index against a freshly built one
  -v                print every case as it runs";

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    /// Run on both sides and compare. The flag says whether to compare
    /// standard error too.
    Compare { stderr: bool },
    /// Engine only: must be rejected as unsupported or as a syntax error,
    /// with a message containing the text, and must change nothing.
    Rejected(String),
    /// Engine only, with the expected result written in the case file.
    Expect { stdout: Vec<u8>, stderr: Vec<u8>, code: i32 },
    /// Engine only, no checks: a setup step of an engine-only case.
    Setup,
}

#[derive(Debug, Clone)]
struct Step {
    command: String,
    kind: Kind,
    line: usize,
}

#[derive(Debug, Clone)]
struct Case {
    file: String,
    fixture: String,
    steps: Vec<Step>,
}

fn parse_cases(file: &str, text: &str) -> Result<Vec<Case>, String> {
    let mut cases: Vec<Case> = Vec::new();
    let mut fixture = String::from("empty");
    for (n, raw) in text.lines().enumerate() {
        let line_no = n + 1;
        let err = |m: &str| format!("{}:{}: {}", file, line_no, m);
        if raw.trim().is_empty() || raw.starts_with('#') {
            continue;
        }
        if let Some(name) = raw.strip_prefix("@fixture ") {
            fixture = name.trim().to_string();
            continue;
        }
        // Expectation lines attach to the last step.
        let expectation = [("> ", 0), (">: ", 1), ("2> ", 2), ("? ", 3)].iter().find(|(p, _)| raw.starts_with(p) || raw == p.trim_end());
        if let Some((prefix, which)) = expectation {
            let rest = raw.get(prefix.len()..).unwrap_or("");
            let step = cases.last_mut().and_then(|c| c.steps.last_mut()).ok_or_else(|| err("expectation before any case"))?;
            match &mut step.kind {
                Kind::Expect { stdout, stderr, code } => match which {
                    0 => {
                        stdout.extend_from_slice(rest.as_bytes());
                        stdout.push(b'\n');
                    }
                    1 => stdout.extend_from_slice(rest.as_bytes()),
                    2 => {
                        stderr.extend_from_slice(rest.as_bytes());
                        stderr.push(b'\n');
                    }
                    _ => *code = rest.trim().parse().map_err(|_| err("bad exit status"))?,
                },
                _ => return Err(err("expectation lines only follow an = step")),
            }
            continue;
        }
        let (continued, body) = match raw.strip_prefix('+') {
            Some(b) => (true, b),
            None => (false, raw),
        };
        let (marker, command) = match body.split_once(' ') {
            Some(x) => x,
            None => return Err(err("expected a marker and a command")),
        };
        let command = command.replace('\u{23ce}', "\n");
        let (kind, command) = match marker {
            "$" | "" => (Kind::Compare { stderr: true }, command),
            "$~" => (Kind::Compare { stderr: false }, command),
            "!" => match command.split_once(" ## ") {
                Some((c, expect)) => (Kind::Rejected(expect.trim().to_string()), c.to_string()),
                None => (Kind::Rejected(String::new()), command),
            },
            "=" => (Kind::Expect { stdout: Vec::new(), stderr: Vec::new(), code: 0 }, command),
            _ => return Err(err("unknown marker")),
        };
        let step = Step { command, kind, line: line_no };
        if continued {
            let case = cases.last_mut().ok_or_else(|| err("+ before any case"))?;
            let engine_only = matches!(case.steps[0].kind, Kind::Expect { .. } | Kind::Setup);
            let mut step = step;
            if engine_only && matches!(step.kind, Kind::Compare { .. }) {
                step.kind = Kind::Setup;
            }
            if !engine_only && matches!(step.kind, Kind::Expect { .. }) {
                return Err(err("an = step can only continue a case that starts with ="));
            }
            case.steps.push(step);
        } else {
            cases.push(Case { file: file.to_string(), fixture: fixture.clone(), steps: vec![step] });
        }
    }
    Ok(cases)
}

struct Scratch {
    base: PathBuf,
    next: usize,
}

impl Scratch {
    fn new() -> Scratch {
        let base = std::env::temp_dir().join(format!("qdb-harness-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("cannot create scratch directory");
        Scratch { base, next: 0 }
    }

    /// A fresh directory holding a copy of the tree.
    fn materialise(&mut self, db: &Db) -> PathBuf {
        self.next += 1;
        let dir = self.base.join(format!("c{}", self.next)).join("root");
        std::fs::create_dir_all(&dir).expect("cannot create case directory");
        store::dump_dir(db, &dir).expect("cannot write fixture");
        dir
    }

    fn discard(&self, dir: &Path) {
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn show(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.is_empty() {
        return "      (empty)\n".into();
    }
    let mut out = String::new();
    for l in s.split_inclusive('\n') {
        out.push_str("      ");
        out.push_str(&format!("{:?}", l));
        out.push('\n');
    }
    out
}

/// Describe how two trees differ, or None if they are the same.
fn tree_diff(real: &[(Vec<u8>, Option<Vec<u8>>)], engine: &[(Vec<u8>, Option<Vec<u8>>)]) -> Option<String> {
    if real == engine {
        return None;
    }
    let mut out = String::new();
    let describe = |c: &Option<Vec<u8>>| match c {
        None => "folder".to_string(),
        Some(c) => format!("file {:?}", String::from_utf8_lossy(c)),
    };
    for (path, content) in real {
        match engine.iter().find(|(p, _)| p == path) {
            None => out.push_str(&format!("      only real:   {} ({})\n", String::from_utf8_lossy(path), describe(content))),
            Some((_, c)) if c != content => out.push_str(&format!(
                "      differs:     {}\n        real:   {}\n        engine: {}\n",
                String::from_utf8_lossy(path),
                describe(content),
                describe(c)
            )),
            _ => {}
        }
    }
    for (path, content) in engine {
        if !real.iter().any(|(p, _)| p == path) {
            out.push_str(&format!("      only engine: {} ({})\n", String::from_utf8_lossy(path), describe(content)));
        }
    }
    Some(out)
}

/// Run one case. Returns a description of the first failing step.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Memory,
    Disk,
    Reopen,
}

#[derive(Debug, Clone)]
struct Setup {
    mode: Mode,
    cfg: Config,
}

impl Setup {
    fn describe(&self) -> String {
        let store = match self.mode {
            Mode::Memory => "in memory".to_string(),
            Mode::Disk => format!("on disk, sync {}", if self.cfg.sync == SyncMode::Always { "always" } else { "batch" }),
            Mode::Reopen => format!("on disk, closed and reopened after every step, sync {}", if self.cfg.sync == SyncMode::Always { "always" } else { "batch" }),
        };
        format!("{}, pieces of {} bytes, index {}", store, self.cfg.piece, if self.cfg.index { "on" } else { "off" })
    }
}

/// The engine under test, in memory or in a database directory.
struct Engine {
    db: Option<Db>,
    dir: Option<PathBuf>,
    setup: Setup,
    reopens: usize,
}

impl Engine {
    /// A database holding a copy of `start`.
    fn new(start: &Db, setup: &Setup, scratch: &mut Scratch) -> Result<Engine, String> {
        let (mut db, dir) = match setup.mode {
            Mode::Memory => (Db::in_memory(setup.cfg.clone()), None),
            _ => {
                scratch.next += 1;
                let dir = scratch.base.join(format!("db{}", scratch.next));
                (Db::open_dir(&dir, setup.cfg.clone())?.0, Some(dir))
            }
        };
        store::copy_into(start, &mut db)?;
        Ok(Engine { db: Some(db), dir, setup: setup.clone(), reopens: 0 })
    }

    fn db(&self) -> &Db {
        self.db.as_ref().unwrap()
    }

    fn execute(&mut self, command: &[u8]) -> Result<Outcome, String> {
        let out = execute(self.db.as_mut().unwrap(), command);
        if self.setup.mode == Mode::Reopen {
            self.reopen()?;
        }
        Ok(out)
    }

    /// Close the database and open it again; it must come back clean and
    /// pass every check.
    fn reopen(&mut self) -> Result<(), String> {
        let dir = self.dir.clone().ok_or("not on disk")?;
        let before = self.db().snapshot();
        self.db.take().unwrap().close().map_err(|e| format!("close: {}", e))?;
        let (db, report) = Db::open_dir(&dir, self.setup.cfg.clone()).map_err(|e| format!("open again: {}", e))?;
        if report.replayed != 0 || !report.messages().is_empty() {
            return Err(format!("opening after a clean close had work to do: {:?}", report));
        }
        db.check().map_err(|e| format!("check after reopening: {}", e))?;
        if db.snapshot() != before {
            return Err("the tree is not the same after closing and opening again".into());
        }
        self.db = Some(db);
        self.reopens += 1;
        Ok(())
    }

    /// The checks of `qdb --check`, and for a database on disk a last
    /// close and reopen.
    fn finish(mut self) -> Result<(), String> {
        self.db().check().map_err(|e| format!("check: {}", e))?;
        if self.dir.is_some() {
            self.reopen()?;
        }
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.db = None;
        if let Some(dir) = &self.dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn run_case(case: &Case, fixture: &Db, scratch: &mut Scratch, setup: &Setup) -> Result<(), String> {
    let mut engine = Engine::new(fixture, setup, scratch)?;
    let db = &mut engine;
    let needs_real = case.steps.iter().any(|s| matches!(s.kind, Kind::Compare { .. }));
    let root = if needs_real { Some(scratch.materialise(fixture)) } else { None };
    let mut unordered_seen = false;
    let result = (|| {
        for step in &case.steps {
            let fail = |what: String| Err(format!("{}:{}: {}\n{}", case.file, step.line, step.command, what));
            let before = db.db().snapshot();
            match &step.kind {
                Kind::Setup => {
                    db.execute(step.command.as_bytes())?;
                }
                Kind::Rejected(expect) => {
                    let got = db.execute(step.command.as_bytes())?;
                    let stderr = String::from_utf8_lossy(&got.stderr).into_owned();
                    if got.code != 2 || !stderr.starts_with("qdb: ") || !stderr.contains(expect.as_str()) || !got.stdout.is_empty() {
                        return fail(format!(
                            "    expected to be rejected with exit status 2 and a message containing {:?}\n    exit status: {}\n    stdout:\n{}    stderr:\n{}",
                            expect,
                            got.code,
                            show(&got.stdout),
                            show(&got.stderr)
                        ));
                    }
                    if db.db().snapshot() != before {
                        return fail("    a rejected command changed the tree\n".into());
                    }
                }
                Kind::Expect { stdout, stderr, code } => {
                    let got = db.execute(step.command.as_bytes())?;
                    if &got.stdout != stdout || &got.stderr != stderr || got.code != *code {
                        return fail(format!(
                            "    expected exit status {}, got {}\n    expected stdout:\n{}    engine stdout:\n{}    expected stderr:\n{}    engine stderr:\n{}",
                            code,
                            got.code,
                            show(stdout),
                            show(&got.stdout),
                            show(stderr),
                            show(&got.stderr)
                        ));
                    }
                }
                Kind::Compare { stderr: check_stderr } => {
                    let root = root.as_ref().unwrap();
                    let root_text = root.to_string_lossy();
                    let real_command = step.command.replace("{ROOT}", &root_text);
                    let engine_command = step.command.replace("{ROOT}", "");
                    check_safe(&real_command, root)?;
                    let real = run_real(root, &real_command);
                    let got = db.execute(engine_command.as_bytes())?;
                    // Once a case has stored unordered output, later steps
                    // that read it back are unordered too.
                    unordered_seen |= order_is_unspecified(&engine_command);
                    let unordered = unordered_seen;
                    let same = |a: &[u8], b: &[u8]| same_output(a, b, unordered);
                    let mut what = String::new();
                    if real.code != got.code {
                        what.push_str(&format!("    exit status: real {}, engine {}\n", real.code, got.code));
                    }
                    if !same(&real.stdout, &got.stdout) {
                        what.push_str(&format!("    stdout, real:\n{}    stdout, engine:\n{}", show(&real.stdout), show(&got.stdout)));
                    }
                    if *check_stderr && !same(&real.stderr, &got.stderr) {
                        what.push_str(&format!("    stderr, real:\n{}    stderr, engine:\n{}", show(&real.stderr), show(&got.stderr)));
                    }
                    // A redirected find or grep -r stores its lines in
                    // whatever order it found them.
                    let normalise = |tree: Vec<(Vec<u8>, Option<Vec<u8>>)>| -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
                        if !unordered {
                            return tree;
                        }
                        tree.into_iter().map(|(p, c)| (p, c.map(|c| sorted_lines(&c).concat()))).collect()
                    };
                    let real_tree = normalise(store::snapshot_dir(root).map_err(|e| e.to_string())?);
                    if let Some(d) = tree_diff(&real_tree, &normalise(db.db().snapshot())) {
                        what.push_str(&format!("    tree after the command:\n{}", d));
                    }
                    if !what.is_empty() {
                        return fail(what);
                    }
                }
            }
        }
        Ok(())
    })();
    if let Some(root) = root {
        scratch.discard(&root);
    }
    result?;
    engine.finish().map_err(|e| format!("{}:{}: {}\n    {}", case.file, case.steps[0].line, case.steps[0].command, e))
}

// ------------------------------------------------------------------ fuzzing

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

/// Generate regex cases: random patterns over a small alphabet, run through
/// grep and sed in both syntaxes against a file of random short lines.
fn fuzz_cases(n: usize, seed: u64) -> (Db, Vec<Case>) {
    let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut content = Vec::new();
    for _ in 0..40 {
        let len = rng.below(9);
        for _ in 0..len {
            content.push(*b"aaabbbc ._A".get(rng.below(11)).unwrap());
        }
        content.push(b'\n');
    }
    content.extend_from_slice(b"abcabc\naXb a.b\n(a) {b} [c] a|b a+b a*b a?b ^a$ a\\b\nAbC abc ABC\n");
    let mut db = Db::new();
    let root = db.root();
    db.create_file(root, b"f", &content);

    const BRE: &[&str] = &[
        "a", "b", "c", "A", ".", "*", "\\+", "\\?", "\\|", "\\(", "\\)", "\\{1,2\\}", "\\{2\\}", "\\{0,\\}", "\\{,2\\}", "[ab]", "[^a]",
        "[[:upper:]]", "[a-c]", "^", "$", "\\1", "\\b", "\\<", "\\>", "\\w", "\\W", "\\s", "\\.", "\\*", " ", "+", "?", "|", "(", ")",
        "{", "}", "\\B", "[]a]", "[a-]", "\\{1", "\\}", "x*", "\\(a\\)", "\\(b*\\)", "a\\|ab", "[.]",
    ];
    const ERE: &[&str] = &[
        "a", "b", "c", "A", ".", "*", "+", "?", "|", "(", ")", "{1,2}", "{2}", "{0,}", "{,2}", "[ab]", "[^a]", "[[:upper:]]", "[a-c]",
        "^", "$", "\\1", "\\b", "\\<", "\\>", "\\w", "\\W", "\\s", "\\.", "\\*", " ", "\\+", "\\?", "\\|", "\\(", "\\)", "\\{", "{",
        "}", "\\B", "[]a]", "{1", "x*", "(a)", "(b*)", "(a|ab)", "(a|b)*", "[.]", "()",
    ];
    let mut cases = Vec::new();
    for i in 0..n {
        let extended = rng.below(2) == 0;
        let atoms = if extended { ERE } else { BRE };
        let len = 1 + rng.below(6);
        let pattern: String = (0..len).map(|_| rng.pick(atoms)).collect();
        let e = if extended { "E" } else { "" };
        let command = match rng.below(6) {
            0 => format!("grep -n{} '{}' f", e, pattern),
            1 => format!("grep -c{}i '{}' f", e, pattern),
            2 => format!("grep -v{} '{}' f", e, pattern),
            3 => format!("sed {} 's/{}/<&>/' f", if extended { "-E" } else { "" }, pattern),
            4 => format!("sed {} 's/{}/<&>/g' f", if extended { "-E" } else { "" }, pattern),
            _ => format!("sed {} 's/{}/<\\1>/g' f", if extended { "-E" } else { "" }, pattern),
        };
        cases.push(Case {
            file: format!("fuzz seed {}", seed),
            fixture: String::new(),
            steps: vec![Step { command, kind: Kind::Compare { stderr: true }, line: i + 1 }],
        });
    }
    (db, cases)
}

// --------------------------------------------------- index on against off

const SYLLABLES: &[&str] = &["al", "be", "co", "de", "er", "fi", "go", "ha", "in", "jo", "ke", "lo", "ma", "ne", "or", "pa"];

/// Words that share many trigrams with each other, a few in capitals and
/// a few with characters outside ASCII.
fn diff_words(rng: &mut Rng) -> Vec<String> {
    let mut words = Vec::new();
    for i in 0..48 {
        let mut w = String::new();
        for _ in 0..2 + rng.below(3) {
            w.push_str(SYLLABLES[rng.below(SYLLABLES.len())]);
        }
        match i % 12 {
            0 => w = w.to_uppercase(),
            1 => w = format!("{}{}", w[..1].to_uppercase(), &w[1..]),
            2 => w.push_str("_x"),
            3 => w.push('é'),
            4 => w = format!("π{}", w),
            _ => {}
        }
        words.push(w);
    }
    words
}

fn diff_line(rng: &mut Rng, words: &[String]) -> String {
    let n = 1 + rng.below(6);
    let mut line = String::new();
    for i in 0..n {
        if i > 0 {
            line.push_str([" ", " ", "  ", "(", ", ", " = ", "->", "."][rng.below(8)]);
        }
        line.push_str(&words[rng.below(words.len())]);
        if rng.below(12) == 0 {
            line.push_str(&format!("{}", rng.below(5000)));
        }
    }
    line
}

/// A pattern and the grep letter for its syntax ("" for basic).
fn diff_pattern(rng: &mut Rng, words: &[String]) -> (String, &'static str) {
    let w = |rng: &mut Rng| words[rng.below(words.len())].clone();
    // A piece of a word, cut on character boundaries.
    let piece = |rng: &mut Rng| {
        let word = w(rng);
        let chars: Vec<char> = word.chars().collect();
        let from = rng.below(chars.len());
        let len = 1 + rng.below(chars.len() - from);
        chars[from..from + len].iter().collect::<String>()
    };
    match rng.below(22) {
        0 | 1 | 2 => (w(rng), ""),
        3 | 4 => (piece(rng), ""),
        5 => (w(rng), "F"),
        6 => (format!("{}\n{}", w(rng), piece(rng)), "F"),
        7 => (format!("{}.*{}", piece(rng), piece(rng)), ""),
        8 => (format!("{}|{}", w(rng), w(rng)), "E"),
        9 => (format!("({}|{})[ (]+{}", piece(rng), piece(rng), piece(rng)), "E"),
        10 => (format!("{}[0-9]+", piece(rng)), "E"),
        11 => (format!("^{}", w(rng)), ""),
        12 => (format!("{}$", piece(rng)), ""),
        13 => (format!("\\({}\\).*\\1", piece(rng)), ""),
        14 => (format!("{}[a-f]{}", piece(rng), piece(rng)), ""),
        15 => (format!("{}.?{}", piece(rng), piece(rng)), "E"),
        16 => (format!("({})+{}?", piece(rng), piece(rng)), "E"),
        17 => (format!("\\<{}\\>", w(rng)), ""),
        18 => ("zzqqxx".to_string() + &piece(rng), ""),
        19 => (format!("[[:upper:]]{}", piece(rng)), ""),
        20 => (format!("{}\\|{}\\|{}", piece(rng), w(rng), piece(rng)), ""),
        _ => (format!("{}{{2,}}", piece(rng)), "E"),
    }
}

/// The differential test: the same random steps on an engine with the
/// index and one without. Returns false at the first difference.
fn diff_run(steps: usize, seed: u64, setup: &Setup, scratch: &mut Scratch) -> Result<bool, String> {
    let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let words = diff_words(&mut rng);
    let mut on = Engine::new(&Db::new(), setup, scratch)?;
    let mut off = Db::without_index();
    assert!(on.db().has_index() && !off.has_index());
    let (mut searches, mut writes, mut selected, mut refused) = (0usize, 0usize, 0usize, 0usize);
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut next_name = 0usize;

    for step in 0..steps {
        let tree = on.db().snapshot();
        let files: Vec<String> = tree.iter().filter(|(_, c)| c.is_some()).map(|(p, _)| String::from_utf8_lossy(p).into_owned()).collect();
        let dirs: Vec<String> = tree.iter().filter(|(_, c)| c.is_none()).map(|(p, _)| String::from_utf8_lossy(p).into_owned()).collect();
        let mut fresh = |rng: &mut Rng| {
            next_name += 1;
            let dir = match rng.below(4) {
                0 => String::new(),
                1 if !dirs.is_empty() => format!("{}/", dirs[rng.below(dirs.len())]),
                _ => format!("d{}/", rng.below(6)),
            };
            let sub = if rng.below(3) == 0 { format!("s{}/", rng.below(3)) } else { String::new() };
            format!("{}{}f{}.{}", dir, sub, next_name, ["c", "h", "txt"][rng.below(3)])
        };
        // Commands that give a file new content, a line at a time.
        let write = |rng: &mut Rng, path: &str, c: &mut Vec<String>| {
            for i in 0..1 + rng.below(8) {
                c.push(format!("echo '{}' {} {}", diff_line(rng, &words), if i == 0 { ">" } else { ">>" }, path));
            }
        };
        // Which kind of step. Searches are about half; a tree that has
        // grown large is cut back, and an empty one is filled.
        let roll = rng.below(100);
        let (kind, commands): (&str, Vec<String>) = if files.len() < 8 {
            let path = fresh(&mut rng);
            let dir = path.rsplit_once('/').map(|(d, _)| d.to_string());
            let mut c = Vec::new();
            if let Some(d) = dir {
                c.push(format!("mkdir -p {}", d));
            }
            write(&mut rng, &path, &mut c);
            ("create", c)
        } else if files.len() > 160 && !dirs.is_empty() {
            ("rm -r", vec![format!("rm -r {}", dirs[rng.below(dirs.len())].split('/').next().unwrap())])
        } else if tree.iter().map(|(_, c)| c.as_ref().map_or(0, |c| c.len())).sum::<usize>() > 1 << 20 {
            // Too much text: remove the top-level name that holds most.
            let mut held: std::collections::BTreeMap<&[u8], usize> = std::collections::BTreeMap::new();
            for (path, content) in &tree {
                *held.entry(path.split(|b| *b == b'/').next().unwrap()).or_default() += content.as_ref().map_or(0, |c| c.len());
            }
            let most = held.iter().max_by_key(|(_, n)| **n).unwrap().0;
            ("rm -r", vec![format!("rm -r {}", String::from_utf8_lossy(most))])
        } else if roll < 50 {
            let (pattern, syntax) = diff_pattern(&mut rng, &words);
            let flags = ["rn", "rn", "rl", "rc", "rin", "ril", "rnv", "rlv", "rci", "r", "rcv"][rng.below(11)];
            let target = match rng.below(6) {
                0 if !dirs.is_empty() => dirs[rng.below(dirs.len())].clone(),
                1 if !dirs.is_empty() => format!("{} {}", dirs[rng.below(dirs.len())], files[rng.below(files.len())]),
                // Lists of files, which are searched with the index too.
                2 => "*.c *.txt d1/* d2/*.h d3/s1/*".to_string(),
                3 => "* d0/* d4/*".to_string(),
                _ => ".".to_string(),
            };
            let tail = match rng.below(12) {
                0 => " | head -n 3".to_string(),
                1 => " | wc -l".to_string(),
                2 => format!(" > {}", files[rng.below(files.len())]),
                3 => " >> d0/found.txt".to_string(),
                _ => String::new(),
            };
            let pattern = if syntax == "F" { pattern.replace("\\n", "\n") } else { pattern };
            ("search", vec![format!("grep -{}{} '{}' {}{}", flags, syntax, pattern, target, tail)])
        } else if roll < 60 {
            let path = fresh(&mut rng);
            let mut c = Vec::new();
            if let Some((d, _)) = path.rsplit_once('/') {
                c.push(format!("mkdir -p {}", d));
            }
            write(&mut rng, &path, &mut c);
            ("create", c)
        } else if roll < 67 {
            let mut c = Vec::new();
            let path = &files[rng.below(files.len())];
            write(&mut rng, path, &mut c);
            ("overwrite", c)
        } else if roll < 70 {
            // Writes that change nothing: they must not reach the index.
            let others: Vec<&String> = files.iter().filter(|f| *f != "same.txt").collect();
            let f = others[rng.below(others.len())];
            let c = vec![
                format!("cp {} same.txt", f),
                format!("cp {} same.txt", f),
                format!("cat {} > same.txt", f),
                "sed -i 's/qqqq/y/' same.txt".to_string(),
                "sed -i 's/qqqq/y/' *.c d1/*.txt same.txt".to_string(),
                "grep -c '' same.txt > count.txt".to_string(),
                "grep -c '' same.txt > count.txt".to_string(),
            ];
            ("same", c)
        } else if roll < 75 {
            ("append", vec![format!("echo '{}' >> {}", diff_line(&mut rng, &words), files[rng.below(files.len())])])
        } else if roll < 83 {
            let from = &words[rng.below(words.len())];
            let to = &words[rng.below(words.len())];
            let target = match rng.below(3) {
                0 => files[rng.below(files.len())].clone(),
                1 => format!("{} {}", files[rng.below(files.len())], files[rng.below(files.len())]),
                _ => "*.c d1/*.txt".to_string(),
            };
            ("sed -i", vec![format!("sed -i 's/{}/{}/g' {}", &from[..from.len().min(4)].trim_end_matches(|c: char| !c.is_ascii()), to, target)])
        } else if roll < 88 && !dirs.is_empty() {
            next_name += 1;
            ("cp -r", vec![format!("cp -r {} c{}", dirs[rng.below(dirs.len())], next_name % 7)])
        } else if roll < 92 && !dirs.is_empty() {
            ("rm -r", vec![format!("rm -r {}", dirs[rng.below(dirs.len())])])
        } else if roll < 97 {
            next_name += 1;
            let what = if rng.below(2) == 0 || dirs.is_empty() { files[rng.below(files.len())].clone() } else { dirs[rng.below(dirs.len())].clone() };
            let to = match rng.below(3) {
                0 => format!("m{}", next_name),
                1 if !dirs.is_empty() => dirs[rng.below(dirs.len())].clone(),
                _ => format!("d{}/m{}", rng.below(6), next_name),
            };
            ("mv", vec![format!("mv {} {}", what, to)])
        } else {
            // Eight copies of a file, of itself included, so that some
            // files grow large enough to be indexed the way big files are.
            let small: Vec<&(Vec<u8>, Option<Vec<u8>>)> = tree.iter().filter(|(_, c)| c.as_ref().is_some_and(|c| c.len() < 64 * 1024)).collect();
            let f = String::from_utf8_lossy(&small[rng.below(small.len())].0).into_owned();
            ("grow", vec![format!("cat {0} {0} {0} {0} {0} {0} {0} {0} > big{1}.txt", f, rng.below(3))])
        };
        *counts.entry(kind).or_default() += 1;
        if kind == "search" {
            searches += 1;
        } else {
            writes += 1;
        }
        // The step itself, then a search over everything, so that every
        // write is followed by a comparison of what the index answers.
        let mut all = commands;
        if kind != "search" {
            let (pattern, syntax) = diff_pattern(&mut rng, &words);
            let pattern = if syntax == "F" { pattern.replace("\\n", "\n") } else { pattern };
            all.push(format!("grep -r{}{} '{}' .", ["n", "l", "c", "in"][rng.below(4)], syntax, pattern));
        }
        for (nth, command) in all.iter().enumerate() {
            let before = (on.db().index_work(), on.db().io_stats().bytes());
            let a = execute(on.db.as_mut().unwrap(), command.as_bytes());
            if kind == "same" && matches!(nth, 1..=4 | 6) && (on.db().index_work(), on.db().io_stats().bytes()) != before {
                println!("FAIL  seed {} step {}: a write that changes nothing reached the index or the disk: {}", seed, step + 1, command);
                return Ok(false);
            }
            if let Some(e) = on.db().failure() {
                return Err(e.to_string());
            }
            if setup.mode == Mode::Reopen {
                on.reopen().map_err(|e| format!("seed {} step {}: {}: {}", seed, step + 1, command, e))?;
            }
            let b = execute(&mut off, command.as_bytes());
            if a != b {
                println!("FAIL  seed {} step {}: {}", seed, step + 1, command);
                println!("    exit status: index on {}, index off {}", a.code, b.code);
                println!("    stdout, index on:\n{}    stdout, index off:\n{}", show(&a.stdout), show(&b.stdout));
                println!("    stderr, index on:\n{}    stderr, index off:\n{}", show(&a.stderr), show(&b.stderr));
                return Ok(false);
            }
            if command.starts_with("grep") && a.code == 0 {
                selected += 1;
            }
            if a.stderr.starts_with(b"qdb: ") {
                refused += 1;
                if std::env::var_os("QDB_DIFF_VERBOSE").is_some() {
                    println!("refused: {}\n{}", command, show(&a.stderr));
                }
            }
        }
        if ((step + 1) % 50 == 0 || step + 1 == steps) && on.db().snapshot() != off.snapshot() {
            println!("FAIL  seed {} step {}: the trees differ after {:?}", seed, step + 1, all);
            return Ok(false);
        }
        if (step + 1) % 1000 == 0 || step + 1 == steps {
            if let Some(what) = rebuild_check(on.db()) {
                println!("FAIL  seed {} step {}: {}", seed, step + 1, what);
                return Ok(false);
            }
        }
    }
    let work = on.db().index_work().expect("index on");
    let io = on.db().io_stats();
    let reopens = on.reopens;
    on.finish().map_err(|e| format!("seed {}: at the end: {}", seed, e))?;
    let kinds: Vec<String> = counts.iter().map(|(k, n)| format!("{} {}", k, n)).collect();
    println!(
        "seed {}: {} steps ({} searches, {} writes), same results from the engine under test ({}) and one in memory with no index; {} greps selected something, {} commands stopped by the shell layer (a missing folder for a redirection)",
        seed, steps, searches, writes, setup.describe(), selected, refused
    );
    if setup.mode != Mode::Memory {
        println!("    on disk: {} command lines logged, {} syncs, {} saves of the tree, {} bytes written, closed and reopened {} times", io.commits, io.syncs, io.checkpoints, io.bytes(), reopens);
    }
    println!("    steps by kind: {}", kinds.join(", "));
    println!(
        "    index work: {} documents added, {} retired, {} bytes of lists rewritten by sweeps, {} rebuilds; at every check the maintained index equals one built afresh, list by list",
        work.documents_added, work.documents_retired, work.bytes_swept, work.rebuilds
    );
    Ok(true)
}

/// The maintained index must say of every piece of every file exactly
/// what an index built now from that piece's content would say, list by
/// list. `Db::check` does that, with every other check of `qdb --check`.
fn rebuild_check(db: &Db) -> Option<String> {
    db.check().err().map(|e| format!("check: {}", e))
}

// --------------------------------------------------------------------- main

fn run() -> Result<bool, String> {
    let mut fixtures_dir = PathBuf::from("tests/fixtures");
    let mut files: Vec<PathBuf> = Vec::new();
    let mut filter: Option<String> = None;
    let mut fuzz = 0usize;
    let mut seed = 1u64;
    let mut verbose = false;
    let mut brief = false;
    let mut index = true;
    let mut diff = 0usize;
    let mut setup = Setup { mode: Mode::Memory, cfg: Config::default() };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--fixtures" => fixtures_dir = value("--fixtures")?.into(),
            "--filter" => filter = Some(value("--filter")?),
            "--fuzz" => fuzz = value("--fuzz")?.parse().map_err(|_| "--fuzz needs a number")?,
            "--seed" => seed = value("--seed")?.parse().map_err(|_| "--seed needs a number")?,
            "--list-failures" => brief = true,
            "--no-index" => index = false,
            "--store" => {
                setup.mode = match value("--store")?.as_str() {
                    "memory" => Mode::Memory,
                    "disk" => Mode::Disk,
                    "reopen" => Mode::Reopen,
                    other => return Err(format!("--store: {} is not memory, disk or reopen", other)),
                }
            }
            "--sync" => {
                setup.cfg.sync = match value("--sync")?.as_str() {
                    "always" => SyncMode::Always,
                    "batch" => SyncMode::Batch,
                    other => return Err(format!("--sync: {} is not always or batch", other)),
                }
            }
            "--piece" => setup.cfg.piece = value("--piece")?.parse().map_err(|_| "--piece needs a number")?,
            "--segment" => setup.cfg.segment = value("--segment")?.parse().map_err(|_| "--segment needs a number")?,
            "--diff" => diff = value("--diff")?.parse().map_err(|_| "--diff needs a number")?,
            "-v" => verbose = true,
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(true);
            }
            _ if a.starts_with('-') => return Err(format!("unknown option {}\n{}", a, USAGE)),
            _ => files.push(a.into()),
        }
    }
    setup.cfg.index = index;
    if diff > 0 {
        let mut scratch = Scratch::new();
        return diff_run(diff, seed, &setup, &mut scratch);
    }
    let default_run = files.is_empty() && fuzz == 0;
    if default_run {
        let mut found: Vec<PathBuf> = std::fs::read_dir("tests/cases")
            .map_err(|e| format!("tests/cases: {}", e))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "case"))
            .collect();
        found.sort();
        files = found;
    }

    let mut scratch = Scratch::new();
    let mut fixtures: std::collections::HashMap<String, Db> = std::collections::HashMap::new();
    let (mut passed, mut failed, mut steps) = (0usize, 0usize, 0usize);
    let mut report = |case: &Case, result: Result<(), String>| {
        steps += case.steps.len();
        match result {
            Ok(()) => {
                passed += 1;
                if verbose {
                    println!("ok    {}:{}: {}", case.file, case.steps[0].line, case.steps[0].command);
                }
            }
            Err(what) => {
                failed += 1;
                if brief {
                    println!("FAIL  {}", what.lines().next().unwrap_or(""));
                } else {
                    println!("FAIL  {}", what);
                }
            }
        }
    };

    for file in &files {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {}", file.display(), e))?;
        let cases = parse_cases(&file.to_string_lossy(), &text)?;
        for case in &cases {
            if let Some(f) = &filter {
                if !case.steps.iter().any(|s| s.command.contains(f.as_str())) {
                    continue;
                }
            }
            if !fixtures.contains_key(&case.fixture) {
                let path = fixtures_dir.join(format!("{}.fx", case.fixture));
                let text = std::fs::read(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
                let mut db = store::parse_fixture(&text).map_err(|e| format!("{}: {}", path.display(), e))?;
                db.set_index(index);
                fixtures.insert(case.fixture.clone(), db);
            }
            let result = run_case(case, &fixtures[&case.fixture], &mut scratch, &setup);
            report(case, result);
        }
    }
    if fuzz > 0 {
        let (mut db, cases) = fuzz_cases(fuzz, seed);
        db.set_index(index);
        for case in &cases {
            let result = run_case(case, &db, &mut scratch, &setup);
            report(case, result);
        }
    }
    println!("{} cases ({} steps), {}: {} passed, {} failed", passed + failed, steps, setup.describe(), passed, failed);
    Ok(failed == 0)
}

fn main() -> std::process::ExitCode {
    let worker = std::thread::Builder::new().stack_size(512 << 20).spawn(run).expect("spawn");
    match worker.join().expect("harness panicked") {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(msg) => {
            eprintln!("qdb-harness: {}", msg);
            std::process::ExitCode::from(2)
        }
    }
}
