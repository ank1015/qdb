//! The test harness: runs each command on the real GNU tools and on the
//! engine, and compares standard output, standard error, exit status and
//! the resulting tree.
//!
//! See tests/README.md for the case file format.

use qdb::shell::{self, Pipeline};
use qdb::store;
use qdb::{execute, Db, Outcome};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const USAGE: &str = "\
Usage: qdb-harness [OPTIONS] [CASE_FILE]...

Compares the engine with the real GNU tools, case by case.
With no case files, runs every tests/cases/*.case.

  --fixtures DIR    where fixtures live (default tests/fixtures)
  --filter TEXT     only run cases whose command contains TEXT
  --fuzz N          also run N generated regex cases against grep and sed
  --seed N          seed for --fuzz (default 1)
  --list-failures   print one line per failure instead of full details
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

/// Real find and grep -r list a folder in whatever order the filesystem
/// returns, so their output is compared as a sorted set of lines.
fn order_is_unspecified(command: &str) -> bool {
    let pipeline: Pipeline = match shell::parse(command.as_bytes()) {
        Ok(Some(p)) => p,
        _ => return false,
    };
    pipeline.commands.iter().any(|c| {
        let words: Vec<Vec<u8>> = c.words.iter().map(|w| w.text()).collect();
        match words.first().map(|w| w.as_slice()) {
            Some(b"find") => true,
            Some(b"grep") => words[1..]
                .iter()
                .take_while(|w| w.as_slice() != b"--")
                .any(|w| w.starts_with(b"-") && !w.starts_with(b"--") && w.contains(&b'r')),
            _ => false,
        }
    })
}

fn sorted_lines(bytes: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<&[u8]> = bytes.split_inclusive(|c| *c == b'\n').collect();
    lines.sort();
    lines
}

fn replace(hay: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return hay.to_vec();
    }
    let mut out = Vec::with_capacity(hay.len());
    let mut i = 0;
    while i < hay.len() {
        if hay[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(hay[i]);
            i += 1;
        }
    }
    out
}

/// Refuse to hand the real tools a command that could touch anything
/// outside the scratch directory.
fn check_safe(command: &str, root: &Path) -> Result<(), String> {
    let pipeline = match shell::parse(command.as_bytes()) {
        Ok(Some(p)) => p,
        _ => return Ok(()),
    };
    let root = root.to_string_lossy().into_owned().into_bytes();
    for c in &pipeline.commands {
        let words = c.words.iter().map(|w| w.text());
        let targets = c.redirects.iter().map(|r| r.target.text());
        for w in words.chain(targets) {
            // Follow the path: it must never climb above where it starts,
            // and an absolute path must start inside the scratch directory.
            let rel: &[u8] = if w.starts_with(b"/") {
                match w.strip_prefix(&root[..]) {
                    Some(rest) => rest,
                    None => return Err(format!("refusing to run on the real tools (absolute path): {}", command)),
                }
            } else {
                &w
            };
            let mut depth = 0i64;
            for comp in rel.split(|c| *c == b'/') {
                match comp {
                    b"" | b"." => {}
                    b".." => depth -= 1,
                    _ => depth += 1,
                }
                if depth < 0 {
                    return Err(format!("refusing to run on the real tools (path leaves the scratch directory): {}", command));
                }
            }
        }
    }
    Ok(())
}

fn run_real(root: &Path, command: &str) -> Outcome {
    let out = Command::new("bash")
        .args(["--noprofile", "--norc", "-c", command])
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .expect("cannot run bash");
    let root_bytes = root.to_string_lossy().into_owned().into_bytes();
    let stdout = replace(&out.stdout, &root_bytes, b"");
    let stderr = replace(&out.stderr, &root_bytes, b"");
    // bash reports its own errors (a failed redirection) with a prefix the
    // engine replaces with its own name.
    let stderr = replace(&stderr, b"bash: line 1: ", b"qdb: ");
    Outcome { stdout, stderr, code: out.status.code().unwrap_or(-1) }
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
fn run_case(case: &Case, fixture: &Db, scratch: &mut Scratch) -> Result<(), String> {
    let mut db = fixture.clone();
    let needs_real = case.steps.iter().any(|s| matches!(s.kind, Kind::Compare { .. }));
    let root = if needs_real { Some(scratch.materialise(fixture)) } else { None };
    let mut unordered_seen = false;
    let result = (|| {
        for step in &case.steps {
            let fail = |what: String| Err(format!("{}:{}: {}\n{}", case.file, step.line, step.command, what));
            let before = db.snapshot();
            match &step.kind {
                Kind::Setup => {
                    execute(&mut db, step.command.as_bytes());
                }
                Kind::Rejected(expect) => {
                    let got = execute(&mut db, step.command.as_bytes());
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
                    if db.snapshot() != before {
                        return fail("    a rejected command changed the tree\n".into());
                    }
                }
                Kind::Expect { stdout, stderr, code } => {
                    let got = execute(&mut db, step.command.as_bytes());
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
                    let got = execute(&mut db, engine_command.as_bytes());
                    // Once a case has stored unordered output, later steps
                    // that read it back are unordered too.
                    unordered_seen |= order_is_unspecified(&engine_command);
                    let unordered = unordered_seen;
                    let same = |a: &[u8], b: &[u8]| if unordered { sorted_lines(a) == sorted_lines(b) } else { a == b };
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
                    if let Some(d) = tree_diff(&real_tree, &normalise(db.snapshot())) {
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
    result
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
    db.create_file(root, b"f", content);

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

// --------------------------------------------------------------------- main

fn run() -> Result<bool, String> {
    let mut fixtures_dir = PathBuf::from("tests/fixtures");
    let mut files: Vec<PathBuf> = Vec::new();
    let mut filter: Option<String> = None;
    let mut fuzz = 0usize;
    let mut seed = 1u64;
    let mut verbose = false;
    let mut brief = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--fixtures" => fixtures_dir = value("--fixtures")?.into(),
            "--filter" => filter = Some(value("--filter")?),
            "--fuzz" => fuzz = value("--fuzz")?.parse().map_err(|_| "--fuzz needs a number")?,
            "--seed" => seed = value("--seed")?.parse().map_err(|_| "--seed needs a number")?,
            "--list-failures" => brief = true,
            "-v" => verbose = true,
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(true);
            }
            _ if a.starts_with('-') => return Err(format!("unknown option {}\n{}", a, USAGE)),
            _ => files.push(a.into()),
        }
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
                let db = store::parse_fixture(&text).map_err(|e| format!("{}: {}", path.display(), e))?;
                fixtures.insert(case.fixture.clone(), db);
            }
            let result = run_case(case, &fixtures[&case.fixture], &mut scratch);
            report(case, result);
        }
    }
    if fuzz > 0 {
        let (db, cases) = fuzz_cases(fuzz, seed);
        for case in &cases {
            let result = run_case(case, &db, &mut scratch);
            report(case, result);
        }
    }
    println!("{} cases ({} steps): {} passed, {} failed", passed + failed, steps, passed, failed);
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
