//! The test harness: runs each command on the real GNU tools and on the
//! engine, and compares standard output, standard error, exit status and
//! the resulting tree.
//!
//! See tests/README.md for the case file format.

use qdb::store;
use qdb::testing::{check_safe, order_is_unspecified, run_real, same_output, sorted_lines};
use qdb::vfs::{MemFs, Vfs};
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
  --gen FAMILY      instead of the cases: generated tests against the real
                    tools for one family of commands (shell, grep, sed,
                    find, rg, text, ls, xargs, scripts, diff, awk, or all), with --count cases
                    (default 10000) and --seed
  --coverage        print the coverage table without running the cases
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
  --compress        store content compressed
  --index-later     index content in bulk some time after it is written
  --diff N          instead of the cases: run N generated steps, searches
                    mixed with writes, on two engines, one with the index
                    and one without, and require the same results; then
                    check the maintained index against a freshly built one
  --crash N         instead of the cases: the crash test. Cut the power at N
                    points of a generated run (with --seed and --sync) on
                    a simulated disk that loses what was not synced, and
                    require that every one recovers to the result of a
                    whole number of command lines
  --kill N          instead of the cases: the kill test. Feed a real qdb
                    process generated command lines and kill it with
                    SIGKILL, N times, each time opening and checking what
                    it left and going on from there
  --counters        instead of the cases: only the counter check that a
                    default run ends with. The benchmark's command list
                    (tests/baseline/units.txt) runs twice on a generated
                    tree on a simulated disk; every count of what a unit
                    read, wrote and synced must be the same both times,
                    and none may be above tests/baseline/counters.tsv
  --update-baseline write tests/baseline/counters.tsv anew from this build.
                    Say why in the commit that changes it
  --failures FILE   write every failure in full to FILE, after a line
                    with the time and the command line of the run
                    (default target/harness-failures.txt; runs add to it)
  -v                print every case as it runs";

#[global_allocator]
static ALLOC: qdb::mem::Counting = qdb::mem::Counting;

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

/// Where every failure is written in full, with what ran it, so that a
/// failure seen once can be looked at afterwards: `--failures FILE`, by
/// default `target/harness-failures.txt`. Each run adds to the file.
static FAILURES: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn log_failure(text: &str) {
    use std::io::Write;
    let path = FAILURES.lock().unwrap().clone().unwrap_or_else(|| "target/harness-failures.txt".to_string());
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let args: Vec<String> = std::env::args().collect();
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = write!(f, "=== {} (unix time {}), pid {}: {}\n{}\n\n", String::from_utf8_lossy(&qdb::when::format(now as i64 * 1_000_000_000, b"%Y-%m-%d %H:%M:%S UTC").unwrap_or_default()), now, std::process::id(), args.join(" "), text.trim_end());
    }
}

/// Print a one-line failure and log it.
macro_rules! fail_line {
    ($($t:tt)*) => {{
        let text = format!($($t)*);
        log_failure(&text);
        println!("{}", text);
    }};
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
        // The root lies several folders deep, all of them made here and
        // empty: a command that climbs out of the root with `..` (the
        // check before each real run cannot see through a variable)
        // lands in one of them and nowhere else.
        let dir = self.base.join(format!("c{}", self.next)).join("p/p/p/p/p/p/root");
        std::fs::create_dir_all(&dir).expect("cannot create case directory");
        store::dump_dir(db, &dir).expect("cannot write fixture");
        dir
    }

    fn discard(&self, dir: &Path) {
        if let Some(case) = dir.ancestors().nth(7) {
            let _ = std::fs::remove_dir_all(case);
        }
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
        format!(
            "{}, pieces of {} bytes, index {}{}{}",
            store,
            self.cfg.piece,
            if self.cfg.index { "on" } else { "off" },
            if self.cfg.index_later { " (built later)" } else { "" },
            if self.cfg.compress { ", compressed" } else { "" }
        )
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

/// Give every file and folder under `root` the modification time the
/// engine has for it, so that commands that sort or select by time see
/// the same on both sides.
fn sync_times(db: &Db, root: &Path) {
    fn walk(db: &Db, id: qdb::tree::NodeId, path: &Path) {
        if db.is_dir(id) {
            for (name, child) in db.children(id) {
                use std::os::unix::ffi::OsStrExt;
                walk(db, *child, &path.join(std::ffi::OsStr::from_bytes(name)));
            }
        }
        use std::os::unix::ffi::OsStrExt;
        let t = db.mtime(id);
        let ts = libc::timespec { tv_sec: t.div_euclid(1_000_000_000) as libc::time_t, tv_nsec: t.rem_euclid(1_000_000_000) as _ };
        if let Ok(c) = std::ffi::CString::new(path.as_os_str().as_bytes()) {
            // Safety: a path and two times, as utimensat expects.
            unsafe {
                libc::utimensat(libc::AT_FDCWD, c.as_ptr(), [ts, ts].as_ptr(), libc::AT_SYMLINK_NOFOLLOW);
            }
        }
    }
    walk(db, db.root(), root);
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
                    // Both sides start the step with the same file times.
                    sync_times(db.db(), root);
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
                    // bash's own messages, sent to a file, begin as they do
                    // on the screen.
                    let real_tree: Vec<(Vec<u8>, Option<Vec<u8>>)> =
                        store::snapshot_dir(root).map_err(|e| e.to_string())?.into_iter().map(|(p, c)| (p, c.map(|c| qdb::testing::bash_prefix(&c)))).collect();
                    let real_tree = normalise(real_tree);
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

/// The generator of the differential test: random steps, searches mixed
/// with writes of every kind, each step one or more command lines. Used
/// by the differential test and by the crash test.
struct Gen {
    rng: Rng,
    words: Vec<String>,
    next_name: usize,
}

impl Gen {
    fn new(seed: u64) -> Gen {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let words = diff_words(&mut rng);
        Gen { rng, words, next_name: 0 }
    }

    /// The next step for a database that holds `tree`: what kind it is
    /// and its command lines. `most` is how many files and how many bytes
    /// of text the tree may grow to before the step is a removal.
    fn step(&mut self, tree: &[(Vec<u8>, Option<Vec<u8>>)], most: (usize, usize)) -> (&'static str, Vec<String>) {
        let rng = &mut self.rng;
        let words = &self.words[..];
        let next_name = &mut self.next_name;
        let files: Vec<String> = tree.iter().filter(|(_, c)| c.is_some()).map(|(p, _)| String::from_utf8_lossy(p).into_owned()).collect();
        let dirs: Vec<String> = tree.iter().filter(|(_, c)| c.is_none()).map(|(p, _)| String::from_utf8_lossy(p).into_owned()).collect();
        let mut fresh = |rng: &mut Rng| {
            (*next_name) += 1;
            let dir = match rng.below(4) {
                0 => String::new(),
                1 if !dirs.is_empty() => format!("{}/", dirs[rng.below(dirs.len())]),
                _ => format!("d{}/", rng.below(6)),
            };
            let sub = if rng.below(3) == 0 { format!("s{}/", rng.below(3)) } else { String::new() };
            format!("{}{}f{}.{}", dir, sub, (*next_name), ["c", "h", "txt"][rng.below(3)])
        };
        // Commands that give a file new content, a line at a time.
        let write = |rng: &mut Rng, path: &str, c: &mut Vec<String>| {
            for i in 0..1 + rng.below(8) {
                c.push(format!("echo '{}' {} {}", diff_line(rng, words), if i == 0 { ">" } else { ">>" }, path));
            }
        };
        // Which kind of step. Searches are about half; a tree that has
        // grown large is cut back, and an empty one is filled.
        let roll = rng.below(100);
        let (kind, commands): (&str, Vec<String>) = if files.len() < 8 {
            let path = fresh(rng);
            let dir = path.rsplit_once('/').map(|(d, _)| d.to_string());
            let mut c = Vec::new();
            if let Some(d) = dir {
                c.push(format!("mkdir -p {}", d));
            }
            write(rng, &path, &mut c);
            ("create", c)
        } else if files.len() > most.0 && !dirs.is_empty() {
            ("rm -r", vec![format!("rm -r {}", dirs[rng.below(dirs.len())].split('/').next().unwrap())])
        } else if tree.iter().map(|(_, c)| c.as_ref().map_or(0, |c| c.len())).sum::<usize>() > most.1 {
            // Too much text: remove the top-level name that holds most.
            let mut held: std::collections::BTreeMap<&[u8], usize> = std::collections::BTreeMap::new();
            for (path, content) in tree {
                *held.entry(path.split(|b| *b == b'/').next().unwrap()).or_default() += content.as_ref().map_or(0, |c| c.len());
            }
            let most = held.iter().max_by_key(|(_, n)| **n).unwrap().0;
            ("rm -r", vec![format!("rm -r {}", String::from_utf8_lossy(most))])
        } else if roll >= 45 && roll < 50 {
            // One unit that holds a loop: several writes that are kept
            // or lost together.
            let f = |rng: &mut Rng| files[rng.below(files.len())].clone();
            let word = |rng: &mut Rng| words[rng.below(words.len())].chars().filter(|c| c.is_ascii_alphanumeric()).take(4).collect::<String>();
            let unit = match rng.below(6) {
                0 => format!("for f in {} {} {}; do echo '{}' >> $f; done", f(rng), f(rng), f(rng), diff_line(rng, words)),
                1 => {
                    let file = f(rng);
                    format!("for i in 1 2 3 4; do echo \"$i {}\" >> {}; done; N=$(grep -c '' {}); echo \"$N lines\" >> count.txt", word(rng), file, file)
                }
                2 if !dirs.is_empty() => {
                    let d = &dirs[rng.below(dirs.len())];
                    format!("ls {0} | while read f; do sed -i 's/{1}/{2}/' \"{0}/$f\"; done", d, word(rng), word(rng))
                }
                3 if !dirs.is_empty() => {
                    let d = &dirs[rng.below(dirs.len())];
                    format!("cd {}; for f in *.c *.txt; do if grep -q '{}' \"$f\"; then echo '{}' >> \"$f\"; else cp \"$f\" kept.h; fi; done; cd /", d, word(rng), diff_line(rng, words))
                }
                4 => {
                    let (file, w) = (f(rng), word(rng));
                    format!("until grep -q 'zz{1}' {0}; do echo 'zz{1}' >> {0}; done; X={0}; echo \"${{#X}} $X\" > last.txt", file, w)
                }
                _ => {
                    let file = f(rng);
                    format!("for a in x y; do for b in 1 2 3; do if [ $b = 2 ]; then continue; fi; echo \"$a$b $((b * 7))\"; done; done > {}", file)
                }
            };
            ("loop", vec![unit])
        } else if roll < 50 {
            let (pattern, syntax) = diff_pattern(rng, words);
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
            let path = fresh(rng);
            let mut c = Vec::new();
            if let Some((d, _)) = path.rsplit_once('/') {
                c.push(format!("mkdir -p {}", d));
            }
            write(rng, &path, &mut c);
            ("create", c)
        } else if roll < 67 {
            let mut c = Vec::new();
            let path = &files[rng.below(files.len())];
            write(rng, path, &mut c);
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
            ("append", vec![format!("echo '{}' >> {}", diff_line(rng, words), files[rng.below(files.len())])])
        } else if roll < 83 {
            let from = &words[rng.below(words.len())];
            let to = &words[rng.below(words.len())];
            let target = match rng.below(3) {
                0 => files[rng.below(files.len())].clone(),
                1 => format!("{} {}", files[rng.below(files.len())], files[rng.below(files.len())]),
                _ => "*.c d1/*.txt".to_string(),
            };
            ("sed -i", vec![format!("sed -i 's/{}/{}/g' {}", &from[..from.len().min(4)].trim_end_matches(|c: char| !c.is_ascii()), to, target)])
        } else if roll < 85 {
            // A copy, then changes to either side: the two share their
            // content until then and must not after.
            (*next_name) += 1;
            let src = files[rng.below(files.len())].clone();
            let copy = format!("copy{}.txt", (*next_name) % 5);
            let mut c = vec![format!("cp {} {}", src, copy)];
            for _ in 0..1 + rng.below(4) {
                let side = if rng.below(2) == 0 { &src } else { &copy };
                c.push(match rng.below(5) {
                    0 => format!("echo '{}' >> {}", diff_line(rng, words), side),
                    1 => format!("sed -i 's/{}/{}/' {}", &words[rng.below(words.len())].chars().take(3).collect::<String>(), words[rng.below(words.len())], side),
                    2 => format!("echo '{}' > {}", diff_line(rng, words), side),
                    3 => format!("cp {} copy{}.txt", side, rng.below(5)),
                    _ => format!("rm {}", side),
                });
                if c.last().unwrap().starts_with("rm ") {
                    break;
                }
            }
            c.push(format!("cat {} {}", src, copy));
            ("cp, edit", c)
        } else if roll < 88 && !dirs.is_empty() {
            (*next_name) += 1;
            ("cp -r", vec![format!("cp -r {} c{}", dirs[rng.below(dirs.len())], (*next_name) % 7)])
        } else if roll < 92 && !dirs.is_empty() {
            ("rm -r", vec![format!("rm -r {}", dirs[rng.below(dirs.len())])])
        } else if roll < 97 {
            (*next_name) += 1;
            let what = if rng.below(2) == 0 || dirs.is_empty() { files[rng.below(files.len())].clone() } else { dirs[rng.below(dirs.len())].clone() };
            let to = match rng.below(3) {
                0 => format!("m{}", (*next_name)),
                1 if !dirs.is_empty() => dirs[rng.below(dirs.len())].clone(),
                _ => format!("d{}/m{}", rng.below(6), (*next_name)),
            };
            ("mv", vec![format!("mv {} {}", what, to)])
        } else {
            // Eight copies of a file, of itself included, so that some
            // files grow large enough to be indexed the way big files are.
            let small: Vec<&(Vec<u8>, Option<Vec<u8>>)> = tree.iter().filter(|(_, c)| c.as_ref().is_some_and(|c| c.len() < 64 * 1024)).collect();
            let f = String::from_utf8_lossy(&small[rng.below(small.len())].0).into_owned();
            ("grow", vec![format!("cat {0} {0} {0} {0} {0} {0} {0} {0} > big{1}.txt", f, rng.below(3))])
        };
        // The step itself, then a search over everything, so that every
        // write is followed by a comparison of what the index answers.
        let mut all = commands;
        if kind != "search" {
            let (pattern, syntax) = diff_pattern(rng, words);
            let pattern = if syntax == "F" { pattern.replace("\\n", "\n") } else { pattern };
            all.push(format!("grep -r{}{} '{}' .", ["n", "l", "c", "in"][rng.below(4)], syntax, pattern));
        }
        (kind, all)
    }
}

/// The differential test: the same random steps on an engine with the
/// index and one without. Returns false at the first difference.
fn diff_run(steps: usize, seed: u64, setup: &Setup, scratch: &mut Scratch) -> Result<bool, String> {
    let mut gen = Gen::new(seed);
    let mut on = Engine::new(&Db::new(), setup, scratch)?;
    let mut off = Db::without_index();
    assert!(on.db().has_index() && !off.has_index());
    let (mut searches, mut writes, mut selected, mut refused) = (0usize, 0usize, 0usize, 0usize);
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for step in 0..steps {
        let tree = on.db().snapshot();
        let (kind, all) = gen.step(&tree, (160, 1 << 20));
        *counts.entry(kind).or_default() += 1;
        if kind == "search" {
            searches += 1;
        } else {
            writes += 1;
        }
        for (nth, command) in all.iter().enumerate() {
            // One clock for both, moving on a second with every line.
            let now = CLOCK_BASE + (step * 16 + nth) as i64 * 1_000_000_000;
            on.db.as_mut().unwrap().set_clock(Some(now));
            off.set_clock(Some(now));
            let before = (on.db().index_work(), on.db().io_stats().content_bytes);
            let moved = on.db().io_stats().moved_bytes;
            let a = execute(on.db.as_mut().unwrap(), command.as_bytes());
            // A write of the same content still sets the file's time, as
            // the real tools do, so upkeep may take a step after it: only
            // a line where upkeep moved nothing can be held to this.
            let upkeep = on.db().io_stats().moved_bytes != moved;
            if kind == "same" && matches!(nth, 1..=4 | 6) && !upkeep && (on.db().index_work(), on.db().io_stats().content_bytes) != before {
                fail_line!("FAIL  seed {} step {}: a write that changes nothing reached the index or wrote content: {}", seed, step + 1, command);
                println!("    index work and content bytes before: {:?}", before);
                println!("    and after: {:?}", (on.db().index_work(), on.db().io_stats().content_bytes));
                println!("    all lines of the step: {:?}; the line printed: {}", all, show(&a.stderr));
                if std::env::var_os("QDB_DIFF_VERBOSE").is_some() {
                    let f = command.split(' ').nth(1).unwrap_or("");
                    for c in [format!("wc {} same.txt", f), format!("tail -c 80 {}", f), "tail -c 80 same.txt".to_string()] {
                        println!("    {}: {}", c, show(&execute(on.db.as_mut().unwrap(), c.as_bytes()).stdout));
                    }
                    if let Ok(id) = on.db().resolve(f.as_bytes()) {
                        println!("    pieces of {}: {:?}", f, on.db().pieces(id).iter().map(|p| on.db().text_of(p).len()).collect::<Vec<_>>());
                    }
                    if let Ok(id) = on.db().resolve(b"same.txt") {
                        println!("    pieces of same.txt: {:?}", on.db().pieces(id).iter().map(|p| on.db().text_of(p).len()).collect::<Vec<_>>());
                    }
                }
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
                fail_line!("FAIL  seed {} step {}: {}", seed, step + 1, command);
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
            fail_line!("FAIL  seed {} step {}: the trees differ after {:?}", seed, step + 1, all);
            return Ok(false);
        }
        let every: usize = std::env::var("QDB_DIFF_CHECK_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1000);
        if (step + 1) % every == 0 || step + 1 == steps {
            if let Some(what) = rebuild_check(on.db()) {
                println!("    the step: {:?}", all);
                fail_line!("FAIL  seed {} step {}: {}", seed, step + 1, what);
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

// ------------------------------------------------------------ crash tests

/// A number that stands for a whole tree: every path and every byte.
/// A generated case whose difference from the real tool is known and
/// listed in tests/README.md: the real tool's output is a defect that the
/// engine does not copy.
///
/// GNU grep 3.11 with -o, -w and -x together takes the newline as part of
/// the match for some patterns and prints an empty line after it.
fn known_difference(family: &str, command: &str) -> bool {
    let has = |f: &str| command.split(' ').any(|w| w == f);
    family == "grep" && has("-o") && has("-w") && has("-x")
}

/// Where the clock of the generated tests starts: 2023-11-14 22:13:20 UTC.
const CLOCK_BASE: i64 = 1_700_000_000_000_000_000;

fn digest(db: &Db) -> u64 {
    digest_of(db, true)
}

/// The same without the modification times, for the kill test: there
/// the tree to compare with is made by another process, a moment
/// earlier or later by the clock.
fn digest_untimed(db: &Db) -> u64 {
    digest_of(db, false)
}

fn digest_of(db: &Db, times: bool) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    if times {
        h.write_i64(db.mtime(db.root()));
    }
    for (path, content) in db.snapshot() {
        h.write_usize(path.len());
        h.write(&path);
        // Modification times are part of what must survive.
        if times {
            h.write_i64(db.mtime(db.resolve(&[b"/", &path[..]].concat()).expect("a path of the snapshot")));
        }
        match content {
            None => h.write_u8(0),
            Some(c) => {
                h.write_u8(1);
                h.write_usize(c.len());
                h.write(&c);
            }
        }
    }
    h.finish()
}

/// What the crash test knows about the run so far.
struct History {
    /// After each command line of this stretch (entry 0: before the
    /// first): the tree's digest and the number of the last command line
    /// logged.
    states: Vec<(u64, u64)>,
    /// How many command lines have returned.
    returned: usize,
    /// A command line is running.
    running: bool,
    /// A compaction is running: it logs records of its own, which move
    /// content and leave the tree as it is.
    upkeep: bool,
}

impl History {
    /// Whether a recovered database is one this history allows: its tree
    /// is the tree after some whole number of command lines, no line that
    /// returned is missing beyond what the sync setting allows, and
    /// nothing is there that had not been asked for.
    fn allows(&self, cfg: &Config, seq: u64, tree: u64) -> Result<(), String> {
        let done = self.states[self.returned].1;
        if self.upkeep && seq >= done {
            // Its own records move content and leave the tree as it is.
            return if tree == self.states[self.returned].0 { Ok(()) } else { Err(format!("during a compaction the recovered database is at record {} (it was at {}) and its tree has changed", seq, done)) };
        }
        // The line now running may or may not have been logged. If it
        // changes nothing it has no number of its own.
        let last = if self.running { self.states.get(self.returned + 1).map_or(done + 1, |s| s.1) } else { done };
        let lose = if cfg.sync == SyncMode::Always { 0 } else { cfg.batch_lines as u64 - 1 };
        if seq + lose < done || seq > last {
            return Err(format!(
                "the recovered database holds command lines up to number {}; {} had returned{} and at most {} may be lost",
                seq,
                done,
                if self.running { " and one more was running" } else { "" },
                lose
            ));
        }
        let want = self.states.iter().find(|s| s.1 == seq).map(|s| s.0);
        // The running line's result is not in `states` yet if it is the
        // first with its number; then it is checked when the line ends.
        match want {
            Some(w) if w != tree => Err(format!("the recovered tree is not the tree after command line {}", seq)),
            None if !(self.running && seq == done + 1) => Err(format!("no state with number {} is known", seq)),
            _ => Ok(()),
        }
    }
}

#[derive(Default)]
struct CrashCounts {
    points: usize,
    during_merge: usize,
    during_tree_save: usize,
    during_new_data_file: usize,
    during_recovery: usize,
    in_a_command: usize,
    lost_lines: u64,
    cut_tails: usize,
    rebuilt_indexes: usize,
    went_on: usize,
    disk_failures: usize,
    closes: usize,
    compactions: usize,
}

/// Open what a crash left, and check it. Returns the open database.
fn recover(image: &MemFs, cfg: &Config, history: &History, counts: &mut CrashCounts) -> Result<Db, String> {
    let (db, report) = Db::open(Box::new(image.clone()), cfg.clone()).map_err(|e| format!("recovery failed: {}", e))?;
    db.check().map_err(|e| format!("after recovery, check: {}", e))?;
    let (seq, tree) = (db.commit_seq(), digest(&db));
    history.allows(cfg, seq, tree)?;
    counts.lost_lines += history.states[history.returned].1.saturating_sub(seq);
    counts.cut_tails += (report.discarded > 0) as usize;
    counts.rebuilt_indexes += !report.rebuilt.is_empty() as usize;
    Ok(db)
}

/// One simulated power cut: `image` is what was left on disk. Recovery
/// must succeed and give an allowed state. One time in four the power is
/// cut again at some point during the recovery, and that must recover
/// too. One time in eight the recovered database is then used: it must
/// take writes, pass the checks, and close and open cleanly.
fn crash_point(image: MemFs, cfg: &Config, history: &History, rng: &mut Rng, counts: &mut CrashCounts) -> Result<(), String> {
    counts.points += 1;
    if rng.below(4) == 0 {
        // How many operations does recovery make? Then cut at one of them.
        let trial = image.crash(0);
        drop(Db::open(Box::new(trial.clone()), cfg.clone()));
        let ops = trial.counts().ops();
        if ops > 0 {
            let at = 1 + rng.below(ops as usize) as u64;
            let seed = rng.next();
            let again = image.crash(0);
            let second: std::sync::Arc<std::sync::Mutex<Option<MemFs>>> = Default::default();
            let slot = second.clone();
            again.set_hook(Some(Box::new(move |fs, n| {
                if n == at {
                    *slot.lock().unwrap() = Some(fs.crash(seed));
                }
            })));
            drop(Db::open(Box::new(again.clone()), cfg.clone()));
            again.set_hook(None);
            let left = second.lock().unwrap().take();
            if let Some(left) = left {
                counts.during_recovery += 1;
                counts.points += 1;
                recover(&left, cfg, history, counts).map_err(|e| format!("power cut again at operation {} of {} of the recovery: {}", at, ops, e))?;
            }
        }
    }
    let mut db = recover(&image, cfg, history, counts)?;
    if rng.below(8) == 0 {
        counts.went_on += 1;
        for i in 0..3 {
            let out = execute(&mut db, format!("echo 'after the crash {}' >> after_crash_{}.txt", rng.below(1000), i).as_bytes());
            if out.code != 0 {
                return Err(format!("a write after recovery failed: {}", String::from_utf8_lossy(&out.stderr)));
            }
        }
        db.check().map_err(|e| format!("check after writing to the recovered database: {}", e))?;
        let tree = digest(&db);
        db.close().map_err(|e| format!("closing the recovered database: {}", e))?;
        let (db, report) = Db::open(Box::new(image.clone()), cfg.clone()).map_err(|e| format!("opening the recovered database again: {}", e))?;
        if !report.messages().is_empty() || digest(&db) != tree {
            return Err(format!("the recovered database did not close cleanly: {:?}", report.messages()));
        }
    }
    Ok(())
}

/// The crash test. The differential test's generator drives a database
/// whose files are in memory behind the engine's one I/O layer, which
/// tracks what has been written and what has been synced. After file
/// operations chosen at random (and after every operation while an index
/// file or the tree is being written) the power is cut: an image of the
/// disk is made in which any part of what was not synced may be missing,
/// and that image must recover (`crash_point`). Small pieces, data files
/// and thresholds make merges, new data files and tree saves frequent.
/// Every few hundred command lines the run itself goes on from a crash
/// image, or from a disk that failed.
fn crash_run(points: usize, seed: u64, sync: SyncMode, index: bool, compress: bool, index_later: bool) -> Result<bool, String> {
    use std::sync::{Arc, Mutex};
    let cfg = Config { piece: 512, segment: 96 << 10, sync, batch_lines: 8, batch_bytes: 24 << 10, index, checkpoint: 12 << 10, spool: 300, upkeep: true, compress, index_later, ..Config::default() };
    let mut gen = Gen::new(seed);
    let mut rng = Rng(seed.wrapping_mul(0xD1B54A32D192ED03) | 1);
    let counts: Arc<Mutex<CrashCounts>> = Default::default();
    let failure: Arc<Mutex<Option<String>>> = Default::default();
    let history = Arc::new(Mutex::new(History { states: Vec::new(), returned: 0, running: false, upkeep: false }));
    let mut fs = MemFs::tracked();
    let mut db = Db::open(Box::new(fs.clone()), cfg.clone())?.0;
    let mut shadow = Db::without_index();
    let (mut lines, mut epochs) = (0usize, 0usize);
    let mut last_line = String::new();
    // One operation in `spread` is a crash point, to spread the points
    // over a run of a few thousand command lines.
    let spread = 3;
    let hook_for = |fs: &MemFs, hook_seed: u64| {
        let (counts, failure, history, cfg) = (counts.clone(), failure.clone(), history.clone(), cfg.clone());
        let mut rng = Rng(hook_seed | 1);
        fs.set_hook(Some(Box::new(move |fs, _| {
            if failure.lock().unwrap().is_some() || counts.lock().unwrap().points >= points {
                return;
            }
            let names = fs.list().unwrap_or_default();
            let merge = names.iter().any(|n| n.ends_with(".idx.tmp"));
            let tree_save = names.iter().any(|n| n.ends_with(".ckp.tmp") || n == "MANIFEST.tmp");
            if !(merge || tree_save) && rng.below(spread) != 0 {
                return;
            }
            let history = history.lock().unwrap();
            {
                let mut c = counts.lock().unwrap();
                c.during_merge += merge as usize;
                c.during_tree_save += tree_save as usize;
                c.in_a_command += history.running as usize;
            }
            let image = fs.crash(rng.next());
            let mut c = counts.lock().unwrap();
            if let Err(e) = crash_point(image, &cfg, &history, &mut rng, &mut c) {
                *failure.lock().unwrap() = Some(e);
            }
        })));
    };
    history.lock().unwrap().states.push((digest(&shadow), db.commit_seq()));
    hook_for(&fs, rng.next());
    while counts.lock().unwrap().points < points {
        let tree = shadow.snapshot();
        let (_, commands) = gen.step(&tree, (60, 160 << 10));
        for command in &commands {
            // The reference first, so that the state the line leads to is
            // known while the line runs.
            let now = CLOCK_BASE + lines as i64 * 1_000_000_000;
            shadow.set_clock(Some(now));
            db.set_clock(Some(now));
            let want = execute(&mut shadow, command.as_bytes());
            {
                let mut h = history.lock().unwrap();
                let before = h.states[h.returned];
                let after = digest(&shadow);
                h.states.push((after, before.1 + (after != before.0) as u64));
                h.running = true;
            }
            let got = execute(&mut db, command.as_bytes());
            last_line = command.clone();
            lines += 1;
            if let Some(e) = failure.lock().unwrap().as_ref() {
                fail_line!("FAIL  seed {} sync {:?}: at a power cut during command line {} ({}): {}", seed, sync, lines, command, e);
                return Ok(false);
            }
            let mut h = history.lock().unwrap();
            h.running = false;
            h.returned += 1;
            let known = h.states[h.returned];
            if got != want || db.commit_seq() != known.1 {
                fail_line!("FAIL  seed {} sync {:?}: command line {} ({}): the database on the tracked disk and the reference differ (logged line number {}, expected {})", seed, sync, lines, command, db.commit_seq(), known.1);
                return Ok(false);
            }
        }
        // Now and then: close and open again, or compact, with power cuts
        // during it like at any other time; or go on from a crash, or
        // from a disk that failed.
        let roll = rng.below(100);
        if (3..7).contains(&roll) {
            let before = (digest(&db), db.commit_seq());
            if roll < 5 {
                counts.lock().unwrap().closes += 1;
                let closing = std::mem::replace(&mut db, Db::new());
                closing.close().map_err(|e| format!("seed {}: close after command line {}: {}", seed, lines, e))?;
                let (again, report) = Db::open(Box::new(fs.clone()), cfg.clone()).map_err(|e| format!("seed {}: open after a clean close, after command line {}: {}", seed, lines, e))?;
                if !report.messages().is_empty() || report.replayed != 0 {
                    fail_line!("FAIL  seed {} sync {:?}: opening after a clean close (after command line {}) had work to do: {:?}", seed, sync, lines, report);
                    return Ok(false);
                }
                db = again;
            } else {
                counts.lock().unwrap().compactions += 1;
                history.lock().unwrap().upkeep = true;
                db.compact().map_err(|e| format!("seed {}: compact after command line {}: {}", seed, lines, e))?;
                // The records the compaction logged are behind us: count
                // from here.
                let mut h = history.lock().unwrap();
                h.upkeep = false;
                h.states = vec![(before.0, db.commit_seq())];
                h.returned = 0;
            }
            if let Some(e) = failure.lock().unwrap().as_ref() {
                fail_line!("FAIL  seed {} sync {:?}: at a power cut during a {} after command line {} ({}): {}", seed, sync, if roll < 5 { "close or the open after it" } else { "compaction" }, lines, last_line, e);
                return Ok(false);
            }
            if digest(&db) != before.0 || (roll < 5 && db.commit_seq() != before.1) || db.check().is_err() {
                fail_line!("FAIL  seed {} sync {:?}: a {} after command line {} changed the tree, or left something a check finds: {:?}", seed, sync, if roll < 5 { "close and open" } else { "compaction" }, lines, db.check().err());
                return Ok(false);
            }
        }
        if roll < 3 {
            epochs += 1;
            fs.set_hook(None);
            let image = if roll == 0 {
                // The disk goes away part of the way through the next
                // few operations: everything fails from there on.
                counts.lock().unwrap().disk_failures += 1;
                let at = fs.counts().ops() + 1 + rng.below(12) as u64;
                fs.fail_from(Some(at));
                let mut failed = false;
                for i in 0..12 {
                    let line = format!("echo 'the disk is going {}' >> going_{}.txt", rng.below(1000), i % 3);
                    if failed {
                        if execute(&mut db, line.as_bytes()).code != 2 {
                            fail_line!("FAIL  seed {} sync {:?}: a command ran on a database whose disk had failed: {}", seed, sync, line);
                            return Ok(false);
                        }
                        continue;
                    }
                    execute(&mut shadow, line.as_bytes());
                    {
                        let mut h = history.lock().unwrap();
                        let before = h.states[h.returned];
                        let after = digest(&shadow);
                        h.states.push((after, before.1 + (after != before.0) as u64));
                        h.running = true;
                    }
                    // A line that fails may still have been logged whole
                    // before the failure was met: it stays "running".
                    if execute(&mut db, line.as_bytes()).code == 2 {
                        failed = true;
                    } else {
                        let mut h = history.lock().unwrap();
                        h.running = false;
                        h.returned += 1;
                    }
                }
                if !failed {
                    fail_line!("FAIL  seed {} sync {:?}: the disk failed and no command noticed", seed, sync);
                    return Ok(false);
                }
                fs.crash(rng.next())
            } else {
                fs.crash(rng.next())
            };
            let recovered = {
                let mut c = counts.lock().unwrap();
                c.points += 1;
                recover(&image, &cfg, &history.lock().unwrap(), &mut c)
            };
            db = match recovered {
                Ok(db) => db,
                Err(e) => {
                    fail_line!("FAIL  seed {} sync {:?}: going on from a crash after command line {} ({}): {}", seed, sync, lines, last_line, e);
                    return Ok(false);
                }
            };
            fs = image;
            // The reference becomes what was recovered.
            shadow = db.clone();
            shadow.set_index(false);
            let mut h = history.lock().unwrap();
            h.states = vec![(digest(&shadow), db.commit_seq())];
            h.returned = 0;
            h.running = false;
            drop(h);
            hook_for(&fs, rng.next());
        }
    }
    fs.set_hook(None);
    let c = counts.lock().unwrap();
    println!(
        "seed {} sync {}{}: {} power cuts over {} command lines, every one recovered to the tree after a whole number of command lines and passed every check",
        seed,
        if sync == SyncMode::Always { "always" } else { "batch (8 lines)" },
        if index { "" } else { ", index off" },
        c.points,
        lines
    );
    println!(
        "    of them: {} while a command line was running, {} while an index file was being written, {} while the tree was being saved, {} during recovery itself; {} times the run went on from a crash image, {} of those after the disk failed",
        c.in_a_command, c.during_merge, c.during_tree_save, c.during_recovery, epochs, c.disk_failures
    );
    println!(
        "    recovery cut an unfinished tail off the log {} times and built an index again {} times; {} command lines that had returned were lost in all ({}); {} recovered databases were written to, checked, closed and reopened",
        c.cut_tails,
        c.rebuilt_indexes,
        c.lost_lines,
        if sync == SyncMode::Always { "none may be" } else { "at most 7 per power cut may be" },
        c.went_on
    );
    println!("    the run also closed and reopened the database {} times and compacted it {} times, with power cuts during those too", c.closes, c.compactions);
    let _ = c.during_new_data_file;
    Ok(true)
}

/// The kill test: a real `qdb --db` process is fed command lines from the
/// generator and killed with SIGKILL, at a random moment while a line is
/// running or between lines. The directory it leaves must open, pass
/// every check, hold the tree after a whole number of command lines, and
/// lack no line that was answered. The next process then goes on from
/// there.
fn kill_run(kills: usize, seed: u64, sync: SyncMode, scratch: &mut Scratch) -> Result<bool, String> {
    use std::io::{BufRead, Write};
    use std::process::{Command, Stdio};
    let qdb = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("qdb");
    if !qdb.exists() {
        return Err(format!("{}: not built", qdb.display()));
    }
    let cfg = Config { piece: 512, segment: 256 << 10, checkpoint: 16 << 10, sync, ..Config::default() };
    let dir = scratch.base.join("kill-db");
    let started = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i64).unwrap_or(0);
    let mut gen = Gen::new(seed);
    let mut rng = Rng(seed.wrapping_mul(0xD1B54A32D192ED03) | 1);
    let mut shadow = Db::without_index();
    let mut history = History { states: vec![(digest_untimed(&shadow), 0)], returned: 0, running: false, upkeep: false };
    let mut counts = CrashCounts::default();
    let (mut lines, mut mid_line, mut replayed) = (0usize, 0usize, 0u64);
    for kill in 0..kills {
        let mut child = Command::new(&qdb)
            .arg("--db")
            .arg(&dir)
            .args(["--sync", if sync == SyncMode::Always { "always" } else { "batch" }])
            .args(["--piece", "512", "--segment", "262144", "--checkpoint", "16384"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut to = child.stdin.take().unwrap();
        let mut from = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut left = 1 + rng.below(40);
        'life: loop {
            let (_, commands) = gen.step(&shadow.snapshot(), (120, 512 << 10));
            for command in &commands {
                // A newline inside quotes would be two lines to qdb's
                // standard input; the generator makes a few.
                if command.contains('\n') {
                    continue;
                }
                // Whether the line changed anything, a time included,
                // decides whether it was logged.
                let timed = digest(&shadow);
                execute(&mut shadow, command.as_bytes());
                let before = history.states[history.returned];
                let after = digest_untimed(&shadow);
                history.states.push((after, before.1 + (digest(&shadow) != timed) as u64));
                history.running = true;
                lines += 1;
                let marker = format!("QDB-ANSWERED-{}", lines);
                to.write_all(format!("{}\necho {}\n", command, marker).as_bytes()).map_err(|e| format!("the qdb process went away: {}", e))?;
                to.flush().ok();
                left -= 1;
                // The last line of this life: kill without waiting for
                // its answer, two times in three.
                if left == 0 && rng.below(3) != 0 {
                    let spin = rng.below(400);
                    let start = std::time::Instant::now();
                    while start.elapsed() < std::time::Duration::from_micros(spin as u64) {}
                    mid_line += 1;
                    break 'life;
                }
                let mut line = Vec::new();
                loop {
                    line.clear();
                    if from.read_until(b'\n', &mut line).map_err(|e| e.to_string())? == 0 {
                        return Err(format!("the qdb process ended by itself at command line {}: {}", lines, command));
                    }
                    if line.strip_suffix(b"\n") == Some(marker.as_bytes()) {
                        break;
                    }
                }
                history.running = false;
                history.returned += 1;
                if left == 0 {
                    break 'life;
                }
            }
        }
        child.kill().map_err(|e| e.to_string())?;
        child.wait().map_err(|e| e.to_string())?;
        drop((to, from));
        let opened = Db::open_dir(&dir, cfg.clone()).map_err(|e| format!("recovery failed: {}", e)).and_then(|(db, report)| {
            db.check().map_err(|e| format!("after recovery, check: {}", e))?;
            history.allows(&cfg, db.commit_seq(), digest_untimed(&db))?;
            // The times cannot be compared with the shadow's, but every
            // one must be a moment of this test.
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i64).unwrap_or(0);
            for (path, _) in db.snapshot() {
                let t = db.mtime(db.resolve(&[b"/", &path[..]].concat()).expect("a path of the snapshot"));
                if t < started || t > now {
                    return Err(format!("after recovery, {} has a modification time outside the test ({} ns)", String::from_utf8_lossy(&path), t));
                }
            }
            Ok((db, report))
        });
        let (db, report) = match opened {
            Ok(x) => x,
            Err(e) => {
                fail_line!("FAIL  seed {} sync {:?}: kill {} at command line {}: {}", seed, sync, kill + 1, lines, e);
                // Keep the directory for a look.
                let keep = std::env::temp_dir().join(format!("qdb-kill-failure-{}", std::process::id()));
                let _ = std::fs::rename(&dir, &keep);
                println!("    the database directory is kept at {}", keep.display());
                return Ok(false);
            }
        };
        counts.points += 1;
        counts.cut_tails += (report.discarded > 0) as usize;
        counts.rebuilt_indexes += !report.rebuilt.is_empty() as usize;
        counts.lost_lines += history.states[history.returned].1.saturating_sub(db.commit_seq());
        replayed += report.replayed;
        shadow = db.clone();
        shadow.set_index(false);
        history = History { states: vec![(digest_untimed(&shadow), db.commit_seq())], returned: 0, running: false, upkeep: false };
        // Half the time the checking process closes the database
        // properly; otherwise the next process finds it as the kill left
        // it and recovers by itself.
        if rng.below(2) == 0 {
            db.close()?;
        }
    }
    println!(
        "seed {} sync {}: {} kills (SIGKILL) of a qdb process over {} command lines, {} of them without waiting for the last line's answer; every one recovered to the tree after a whole number of command lines, with every answered line present, and passed every check",
        seed,
        if sync == SyncMode::Always { "always" } else { "batch" },
        counts.points,
        lines,
        mid_line
    );
    println!(
        "    recovery applied {} command lines from the log in all, cut an unfinished tail off {} times, built an index again {} times; {} answered lines lost",
        replayed, counts.cut_tails, counts.rebuilt_indexes, counts.lost_lines
    );
    Ok(true)
}

// --------------------------------------------------------------------- main

/// Count, for each row of tests/coverage.txt, the cases that use it, and
/// print the table. False if a row has no case.
fn coverage(table: &str, commands: &[String]) -> Result<bool, String> {
    use qdb::regex::{Flavor, Regex, Syntax};
    // The words of every simple command of every case, from each word
    // that could be a command name on.
    let words: Vec<Vec<Vec<u8>>> = commands
        .iter()
        .map(|c| match qdb::shell::parse(c.as_bytes()) {
            Ok(script) => script.all_commands().iter().map(|c| c.words.iter().map(|w| w.text()).collect::<Vec<Vec<u8>>>()).collect::<Vec<_>>().concat(),
            Err(_) => c.split_whitespace().map(|w| w.as_bytes().to_vec()).collect(),
        })
        .collect();
    let mut ok = true;
    println!("coverage: cases for each command and flag");
    for row in table.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let cols: Vec<&str> = row.split('\t').collect();
        if cols.len() != 3 {
            return Err(format!("tests/coverage.txt: not three columns: {}", row));
        }
        let (cmd, item, pat) = (cols[0], cols[1], cols[2].replace("\\n", "\n"));
        let after = |ws: &'_ [Vec<u8>]| -> Vec<usize> { ws.iter().enumerate().filter(|(_, w)| w.as_slice() == cmd.as_bytes()).map(|(i, _)| i + 1).collect() };
        let count = if let Some(re) = pat.strip_prefix("re:") {
            let re = Regex::new(re.as_bytes(), Syntax { extended: true, icase: false, flavor: Flavor::Grep }).map_err(|e| format!("tests/coverage.txt: {}: {}", row, e))?;
            commands.iter().filter(|c| re.is_match(c.as_bytes()).unwrap_or(false)).count()
        } else {
            let hit = |w: &[u8]| -> bool {
                if let Some(word) = pat.strip_prefix("w:") {
                    w == word.as_bytes()
                } else if pat == "n:" {
                    w.len() > 1 && w[0] == b'-' && w[1..].iter().all(|c| c.is_ascii_digit())
                } else if pat.starts_with("--") {
                    w == pat.as_bytes() || (w.starts_with(pat.as_bytes()) && w.get(pat.len()) == Some(&b'='))
                } else {
                    let flag = pat.as_bytes()[1];
                    w.len() > 1 && w[0] == b'-' && w[1] != b'-' && w[1..].iter().take_while(|c| c.is_ascii_alphanumeric()).any(|c| *c == flag)
                }
            };
            words.iter().filter(|ws| after(ws).iter().any(|from| ws[*from..].iter().any(|w| hit(w)))).count()
        };
        println!("  {:<10} {:<28} {:>5}{}", cmd, item, count, if count == 0 { "   NO CASE" } else { "" });
        ok &= count > 0;
    }
    Ok(ok)
}

// ------------------------------------------------------------- counters

const UNITS: &str = "tests/baseline/units.txt";
const BASELINE: &str = "tests/baseline/counters.tsv";

/// The counts a unit may not exceed: everything it read, wrote and
/// synced. Memory is recorded beside them and not enforced.
const ENFORCED: usize = 10;

fn counter_units() -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(UNITS).map_err(|e| format!("{}: {}", UNITS, e))?;
    Ok(text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.to_string()).collect())
}

/// Run the units on a fresh database on a simulated disk, with the
/// default settings, and return what each one used.
fn counter_run(units: &[String]) -> Result<Vec<[u64; 11]>, String> {
    let spec = qdb::dataset::SPECS.iter().find(|s| s.name == "small").expect("the small dataset");
    let source = qdb::dataset::generate(spec);
    let mut db = Db::open(Box::new(MemFs::tracked()), Config::default())?.0;
    store::copy_into(&source, &mut db)?;
    db.compact()?;
    // 2026-01-01 00:00:00 UTC, so that nothing depends on the day.
    db.set_clock(Some(1_767_225_600_000_000_000));
    let mut out = Vec::new();
    for unit in units {
        let cost = execute(&mut db, unit.replace('⏎', "\n").as_bytes()).cost;
        let f = cost.fields();
        let mut row = [0u64; 11];
        for (i, slot) in row.iter_mut().enumerate() {
            *slot = f[i].1;
        }
        out.push(row);
    }
    if let Some(e) = db.failure() {
        return Err(format!("the counter run failed: {}", e));
    }
    db.check().map_err(|e| format!("the counter run left a database that does not check: {}", e))?;
    Ok(out)
}

fn counters(update: bool) -> Result<bool, String> {
    let units = counter_units()?;
    let names: Vec<&str> = qdb::Cost::default().fields().iter().take(11).map(|f| f.0).collect();
    let first = counter_run(&units)?;
    let second = counter_run(&units)?;
    let mut ok = true;
    let mut differing = 0;
    for (i, unit) in units.iter().enumerate() {
        if first[i] != second[i] {
            differing += 1;
            ok = false;
            for k in 0..11 {
                if first[i][k] != second[i][k] {
                    fail_line!("FAIL  counters: {} was {} in one run and {} in the next: {}", names[k], first[i][k], second[i][k], unit);
                }
            }
        }
    }
    if update {
        let mut text = String::from("# What each unit of tests/baseline/units.txt used, on the generated dataset\n# `small` on a simulated disk with the default settings. A harness run fails\n# when a unit reads, writes or syncs more than this (every column but\n# memory). Written by `qdb-harness --update-baseline`; a commit that changes\n# this file says why.\n");
        text.push_str(&names.join("\t"));
        text.push_str("\tunit\n");
        for (i, unit) in units.iter().enumerate() {
            for v in first[i] {
                text.push_str(&v.to_string());
                text.push('\t');
            }
            text.push_str(unit);
            text.push('\n');
        }
        std::fs::write(BASELINE, text).map_err(|e| format!("{}: {}", BASELINE, e))?;
        println!("counters: {} written for {} units", BASELINE, units.len());
        return Ok(ok);
    }
    let stored = std::fs::read_to_string(BASELINE).map_err(|e| format!("{}: {}", BASELINE, e))?;
    let rows: Vec<&str> = stored.lines().filter(|l| !l.starts_with('#')).skip(1).collect();
    if rows.len() != units.len() {
        fail_line!("FAIL  counters: {} has {} units and {} has {}", BASELINE, rows.len(), UNITS, units.len());
        return Ok(false);
    }
    let (mut above, mut below) = (0, 0);
    for (i, unit) in units.iter().enumerate() {
        let cols: Vec<&str> = rows[i].splitn(12, '\t').collect();
        if cols.len() != 12 || cols[11] != unit {
            fail_line!("FAIL  counters: line {} of {} is not for the unit: {}", i + 1, BASELINE, unit);
            return Ok(false);
        }
        let mut lower = false;
        for k in 0..ENFORCED {
            let base: u64 = cols[k].parse().map_err(|_| format!("{}: not a number: {}", BASELINE, cols[k]))?;
            if first[i][k] > base {
                above += 1;
                fail_line!("FAIL  counters: {} is {}, above its baseline of {}: {}", names[k], first[i][k], base, unit);
            }
            lower |= first[i][k] < base;
        }
        below += lower as usize;
    }
    ok &= above == 0;
    println!(
        "counters: {} units run twice on the same database, {} with different counts; {} counts above the baseline{}",
        units.len(),
        differing,
        above,
        if below > 0 { format!("; {} units now use less than their baseline, which can be lowered with --update-baseline", below) } else { String::new() }
    );
    Ok(ok)
}

fn run() -> Result<bool, String> {
    let mut fixtures_dir = PathBuf::from("tests/fixtures");
    let mut files: Vec<PathBuf> = Vec::new();
    let mut filter: Option<String> = None;
    let mut fuzz = 0usize;
    let mut seed = 1u64;
    let mut verbose = false;
    let mut brief = false;
    let mut coverage_only = false;
    let (mut counters_only, mut update_baseline) = (false, false);
    let mut generate: Option<String> = None;
    let mut count = 10_000usize;
    let mut index = true;
    let mut diff = 0usize;
    let mut crash = 0usize;
    let mut kill = 0usize;
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
            "--coverage" => coverage_only = true,
            "--counters" => counters_only = true,
            "--update-baseline" => update_baseline = true,
            "--gen" => generate = Some(value("--gen")?),
            "--count" => count = value("--count")?.parse().map_err(|_| "--count needs a number")?,
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
            "--piece" => {
                setup.cfg.piece = value("--piece")?.parse().map_err(|_| "--piece needs a number")?;
                // Small pieces come with a small limit on redirected
                // output held in memory, so that scratch files are used.
                if setup.cfg.piece < 4096 {
                    setup.cfg.spool = setup.cfg.piece * 4;
                }
            }
            "--segment" => setup.cfg.segment = value("--segment")?.parse().map_err(|_| "--segment needs a number")?,
            "--compress" => setup.cfg.compress = true,
            "--index-later" => setup.cfg.index_later = true,
            "--diff" => diff = value("--diff")?.parse().map_err(|_| "--diff needs a number")?,
            "--crash" => crash = value("--crash")?.parse().map_err(|_| "--crash needs a number")?,
            "--kill" => kill = value("--kill")?.parse().map_err(|_| "--kill needs a number")?,
            "-v" => verbose = true,
            "--failures" => *FAILURES.lock().unwrap() = Some(value("--failures")?),
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(true);
            }
            _ if a.starts_with('-') => return Err(format!("unknown option {}\n{}", a, USAGE)),
            _ => files.push(a.into()),
        }
    }
    setup.cfg.index = index;
    if crash > 0 {
        return crash_run(crash, seed, setup.cfg.sync, index, setup.cfg.compress, setup.cfg.index_later);
    }
    if kill > 0 {
        let mut scratch = Scratch::new();
        return kill_run(kill, seed, setup.cfg.sync, &mut scratch);
    }
    if diff > 0 {
        let mut scratch = Scratch::new();
        return diff_run(diff, seed, &setup, &mut scratch);
    }
    if let Some(family) = generate {
        let families: Vec<&str> = if family == "all" { qdb::testgen::FAMILIES.to_vec() } else { vec![family.as_str()] };
        let mut scratch = Scratch::new();
        let mut all_ok = true;
        for family in families {
            let g = qdb::testgen::generate(family, count, seed).ok_or(format!("--gen: no family {} (there are: {})", family, qdb::testgen::FAMILIES.join(", ")))?;
            let mut db = store::parse_fixture(g.fixture.as_bytes()).map_err(|e| format!("generated tree: {}", e))?;
            db.set_index(index);
            let (mut passed, mut failed, mut known) = (0usize, 0usize, 0usize);
            for (i, command) in g.commands.iter().enumerate() {
                if let Some(f) = &filter {
                    if !command.contains(f.as_str()) {
                        continue;
                    }
                }
                // File times are set afresh for every case, the same on
                // both sides, each so long before now.
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                for (path, age) in &g.ages {
                    if let Ok(id) = db.resolve(path.as_bytes()) {
                        db.set_mtime(id, (now - age) * 1_000_000_000);
                    }
                }
                let case = Case {
                    file: format!("generated {} seed {}", family, seed),
                    fixture: String::new(),
                    steps: vec![Step { command: command.clone(), kind: Kind::Compare { stderr: true }, line: i + 1 }],
                };
                match run_case(&case, &db, &mut scratch, &setup) {
                    Ok(()) => passed += 1,
                    Err(_) if known_difference(family, command) => known += 1,
                    Err(what) => {
                        failed += 1;
                        if brief {
                            log_failure(&what);
                            println!("FAIL  {}", what.lines().next().unwrap_or(""));
                        } else {
                            log_failure(&what);
                            println!("FAIL  {}", what);
                        }
                    }
                }
            }
            println!(
                "generated {}: {} cases, seed {}, {}: {} passed, {} known differences, {} failed",
                family,
                passed + failed + known,
                seed,
                setup.describe(),
                passed,
                known,
                failed
            );
            all_ok &= failed == 0;
        }
        return Ok(all_ok);
    }
    if counters_only || update_baseline {
        return counters(update_baseline);
    }
    let default_run = files.is_empty() && fuzz == 0;
    if coverage_only {
        // Every case is read and none is run.
        filter = Some("\u{0}".into());
    }
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
    let mut every_command: Vec<String> = Vec::new();
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
                    log_failure(&what);
                            println!("FAIL  {}", what.lines().next().unwrap_or(""));
                } else {
                    log_failure(&what);
                    println!("FAIL  {}", what);
                }
            }
        }
    };

    for file in &files {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {}", file.display(), e))?;
        let cases = parse_cases(&file.to_string_lossy(), &text)?;
        for case in &cases {
            every_command.extend(case.steps.iter().map(|s| s.command.clone()));
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
    let mut covered = true;
    if default_run || coverage_only {
        let table = std::fs::read_to_string("tests/coverage.txt").map_err(|e| format!("tests/coverage.txt: {}", e))?;
        covered = coverage(&table, &every_command)?;
        if !covered {
            fail_line!("FAIL  coverage: a command or flag has no case");
        }
    }
    println!("{} cases ({} steps), {}: {} passed, {} failed", passed + failed, steps, setup.describe(), passed, failed);
    // A whole run of the cases ends with the counter check.
    let counted = if default_run && filter.is_none() && setup.mode == Mode::Memory { counters(false)? } else { true };
    Ok(failed == 0 && covered && counted)
}

fn main() -> std::process::ExitCode {
    let worker = std::thread::Builder::new().stack_size(64 << 20).spawn(run).expect("spawn");
    match worker.join().expect("harness panicked") {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(msg) => {
            eprintln!("qdb-harness: {}", msg);
            std::process::ExitCode::from(2)
        }
    }
}
