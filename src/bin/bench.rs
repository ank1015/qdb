//! The benchmark: generates a dataset, runs a list of commands on the real
//! GNU tools and on the engine, checks that they agree, and reports how
//! long each took.
//!
//! See bench/README.md.

use qdb::store;
use qdb::testing::{check_safe, order_is_unspecified, run_real, same_output};
use qdb::{execute, Db};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const USAGE: &str = "\
Usage: qdb-bench [OPTIONS]

Times a list of commands on the real GNU tools and on the engine.

  --dataset NAME    small, medium or large (default small); or all
  --dir PATH        use an existing directory as the dataset instead
  --commands FILE   the command list (default bench/commands.txt)
  --reps N          timed runs per read-only command (default 5)
  --out DIR         where to write results (default bench/results)
  --scratch DIR     where to put the real copy of the dataset
                    (default: the system temporary directory)
  --export DIR      only generate the dataset into DIR and stop";

// ------------------------------------------------------------- datasets

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

    fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

struct Spec {
    name: &'static str,
    files: usize,
    /// Average lines per file.
    lines: usize,
}

const SPECS: &[Spec] = &[
    Spec { name: "small", files: 200, lines: 120 },
    Spec { name: "medium", files: 2_000, lines: 150 },
    Spec { name: "large", files: 12_000, lines: 160 },
];

const SYLLABLES: &[&str] = &[
    "al", "be", "co", "da", "en", "fi", "go", "ha", "in", "jo", "ka", "le", "mo", "ne", "or", "pa", "qu", "re", "si", "to", "un", "va",
    "we", "xi", "yo", "ze", "ar", "bu", "cy", "di", "el", "fo",
];

const COMMON: &[&str] = &[
    "the", "of", "and", "to", "in", "is", "for", "with", "value", "result", "data", "index", "count", "name", "list", "node", "file",
    "path", "error", "return", "if", "else", "int", "char", "const", "static", "void", "alpha_one", "beta_two", "handle", "buffer",
];

fn vocabulary(rng: &mut Rng) -> Vec<String> {
    let mut words: Vec<String> = COMMON.iter().map(|w| w.to_string()).collect();
    for _ in 0..3000 {
        let n = 2 + rng.below(3);
        let mut w = String::new();
        for _ in 0..n {
            w.push_str(SYLLABLES[rng.below(SYLLABLES.len())]);
        }
        words.push(w);
    }
    words
}

/// Pick a word, favouring the start of the list (a rough Zipf shape, as in
/// real text: a few words are very common, most are rare).
fn word<'a>(rng: &mut Rng, vocab: &'a [String]) -> &'a str {
    let r = rng.below(1000);
    let idx = if r < 500 { rng.below(COMMON.len()) } else if r < 850 { rng.below(300) } else { rng.below(vocab.len()) };
    &vocab[idx]
}

fn file_content(rng: &mut Rng, vocab: &[String], ext: &str, lines: usize) -> Vec<u8> {
    let mut out = String::new();
    let n = lines / 2 + rng.below(lines + 1);
    match ext {
        "c" | "h" => {
            for _ in 0..1 + rng.below(4) {
                let _ = writeln!(out, "#include \"{}.h\"", word(rng, vocab));
            }
        }
        "py" => {
            for _ in 0..1 + rng.below(4) {
                let _ = writeln!(out, "import {}", word(rng, vocab));
            }
        }
        "md" => {
            let _ = writeln!(out, "# {} {}", word(rng, vocab), word(rng, vocab));
        }
        _ => {}
    }
    for _ in 0..n {
        if rng.chance(12) {
            out.push('\n');
            continue;
        }
        let code = matches!(ext, "c" | "h" | "py");
        if code {
            for _ in 0..rng.below(4) {
                out.push_str("    ");
            }
        }
        if rng.chance(150) {
            let _ = writeln!(out, "// TODO: {} {}", word(rng, vocab), word(rng, vocab));
            continue;
        }
        if rng.chance(900) {
            let _ = writeln!(out, "// FIXME {}", word(rng, vocab));
            continue;
        }
        if code && rng.chance(40) {
            let _ = writeln!(out, "return {};", word(rng, vocab));
            continue;
        }
        if code && rng.chance(60) {
            let _ = writeln!(out, "handle_request({}, {});", word(rng, vocab), rng.below(100_000));
            continue;
        }
        if rng.chance(70) {
            let _ = writeln!(out, "error: {} {}", word(rng, vocab), rng.below(10_000));
            continue;
        }
        let words = 2 + rng.below(10);
        for i in 0..words {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(word(rng, vocab));
            if code && rng.chance(8) {
                let _ = write!(out, "_{}(", word(rng, vocab));
            }
            if rng.chance(25) {
                let _ = write!(out, " {}", rng.below(1_000_000));
            }
        }
        if code && rng.chance(3) {
            out.push(';');
        }
        out.push('\n');
    }
    // Some files have no final newline, as in real repositories.
    if rng.chance(20) && out.ends_with('\n') {
        out.pop();
    }
    out.into_bytes()
}

/// Build a dataset: a tree shaped like a source repository. The same name
/// always gives the same tree.
fn generate(spec: &Spec) -> Db {
    let mut rng = Rng(0x5EED_0000 + spec.files as u64);
    let vocab = vocabulary(&mut rng);
    let mut db = Db::new();
    let put = |db: &mut Db, path: &str, content: Vec<u8>| {
        let mut cur = db.root();
        let comps: Vec<&str> = path.split('/').collect();
        for (i, comp) in comps.iter().enumerate() {
            cur = match db.child(cur, comp.as_bytes()) {
                Some(c) => c,
                None if i + 1 == comps.len() => db.create_file(cur, comp.as_bytes(), content.clone()),
                None => db.create_folder(cur, comp.as_bytes()),
            };
        }
    };
    // Paths the command list relies on.
    for (path, ext, lines) in [
        ("README.md", "md", 60),
        ("src/main.c", "c", 400),
        ("docs/guide.md", "md", 200),
        ("config/settings.json", "json", 40),
        (".gitignore", "txt", 5),
    ] {
        let c = file_content(&mut rng, &vocab, ext, lines);
        put(&mut db, path, c);
    }
    const TOPS: &[(&str, &[&str])] = &[
        ("src", &["c", "c", "c", "h"]),
        ("lib", &["c", "h", "py"]),
        ("tests", &["py", "c", "txt"]),
        ("docs", &["md", "md", "txt"]),
        ("scripts", &["py", "txt"]),
        ("config", &["json", "txt"]),
    ];
    for i in 0..spec.files {
        let (top, exts) = TOPS[match rng.below(100) {
            0..=44 => 0,
            45..=64 => 1,
            65..=79 => 2,
            80..=91 => 3,
            92..=96 => 4,
            _ => 5,
        }];
        let ext = exts[rng.below(exts.len())];
        let mut path = top.to_string();
        // Up to four levels of sub-folders; fewer distinct folders near
        // the top.
        let depth = rng.below(5);
        for level in 0..depth {
            let fanout = 2 + (spec.files / 400).min(10) + level;
            let _ = write!(path, "/mod_{:02}", rng.below(fanout));
        }
        let hidden = if rng.chance(60) { "." } else { "" };
        let _ = write!(path, "/{}{}_{}.{}", hidden, word(&mut rng, &vocab), i, ext);
        let c = file_content(&mut rng, &vocab, ext, spec.lines);
        put(&mut db, &path, c);
    }
    db
}

// ------------------------------------------------------------ measuring

struct Row {
    command: String,
    mutating: bool,
    real: Duration,
    engine: Duration,
    out_bytes: usize,
    agree: bool,
    note: String,
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn ms(d: Duration) -> String {
    let m = d.as_secs_f64() * 1000.0;
    if m < 10.0 {
        format!("{:.3}", m)
    } else if m < 100.0 {
        format!("{:.2}", m)
    } else {
        format!("{:.1}", m)
    }
}

fn tool_version(tool: &str) -> String {
    std::process::Command::new(tool)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().next().map(|l| l.to_string()))
        .unwrap_or_else(|| "unknown".into())
}

struct Options {
    commands: PathBuf,
    reps: usize,
    out: PathBuf,
    scratch: PathBuf,
}

fn bench(name: &str, db: Db, opts: &Options) -> Result<bool, String> {
    let text = std::fs::read_to_string(&opts.commands).map_err(|e| format!("{}: {}", opts.commands.display(), e))?;
    let commands: Vec<(String, bool)> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.strip_prefix("mut: ") {
            Some(c) => (c.to_string(), true),
            None => (l.to_string(), false),
        })
        .collect();

    let root = opts.scratch.join(format!("qdb-bench-{}-{}", std::process::id(), name)).join("root");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    store::dump_dir(&db, &root).map_err(|e| format!("writing the dataset: {}", e))?;

    // How long the engine takes to read the dataset from disk.
    let t = Instant::now();
    let mut db = store::load_dir(&root).map_err(|e| e.to_string())?;
    let load = t.elapsed();
    let before = db.snapshot();
    let files = before.iter().filter(|(_, c)| c.is_some()).count();
    let folders = before.len() - files;
    let bytes = db.content_bytes();
    eprintln!("{}: {} files, {} folders, {:.1} MB", name, files, folders, bytes as f64 / 1e6);

    // What starting a process costs the real tools, for reference.
    let startup = median((0..opts.reps.max(3)).map(|_| {
        let t = Instant::now();
        run_real(&root, "echo -n");
        t.elapsed()
    }).collect());

    let mut rows: Vec<Row> = Vec::new();
    for (command, mutating) in &commands {
        check_safe(command, &root)?;
        let unordered = order_is_unspecified(command);
        let reps = if *mutating { 1 } else { opts.reps };
        let mut real_times = Vec::new();
        let mut engine_times = Vec::new();
        let mut real_out = None;
        let mut engine_out = None;
        for _ in 0..reps {
            let t = Instant::now();
            let r = run_real(&root, command);
            real_times.push(t.elapsed());
            real_out = Some(r);
            let t = Instant::now();
            let e = execute(&mut db, command.as_bytes());
            engine_times.push(t.elapsed());
            engine_out = Some(e);
        }
        let (r, e) = (real_out.unwrap(), engine_out.unwrap());
        let mut note = String::new();
        if r.code != e.code {
            note = format!("exit status: real {}, engine {}", r.code, e.code);
        } else if !same_output(&r.stdout, &e.stdout, unordered) {
            note = "standard output differs".into();
        } else if !same_output(&r.stderr, &e.stderr, unordered) {
            note = "standard error differs".into();
        }
        eprintln!("  {:>10} ms real {:>10} ms engine  {}", ms(median(real_times.clone())), ms(median(engine_times.clone())), command);
        rows.push(Row {
            command: command.clone(),
            mutating: *mutating,
            real: median(real_times),
            engine: median(engine_times),
            out_bytes: e.stdout.len(),
            agree: note.is_empty(),
            note,
        });
    }
    // After the whole list, both sides must hold the same tree.
    let real_tree = store::snapshot_dir(&root).map_err(|e| e.to_string())?;
    let trees_agree = real_tree == db.snapshot();
    let _ = std::fs::remove_dir_all(root.parent().unwrap());

    // Report.
    let all_agree = trees_agree && rows.iter().all(|r| r.agree);
    let mut md = String::new();
    let _ = writeln!(md, "# Benchmark: {} dataset\n", name);
    let _ = writeln!(md, "- Dataset: {} files in {} folders, {:.1} MB of text", files, folders, bytes as f64 / 1e6);
    let _ = writeln!(md, "- Engine load time (reading the dataset from disk into memory): {} ms", ms(load));
    let _ = writeln!(md, "- Real tools: {}; {}; {}; {}; {}", tool_version("bash"), tool_version("grep"), tool_version("sed"), tool_version("ls"), tool_version("find"));
    let _ = writeln!(md, "- Process start-up cost of the real tools (`bash -c 'echo -n'`): {} ms. Every real time below includes it; engine times are measured in-process and include parsing the command.", ms(startup));
    let _ = writeln!(md, "- Read-only commands: median of {} runs. Commands that change data: one run.", opts.reps);
    let _ = writeln!(md, "- Results agree with the real tools: {}\n", if all_agree { "yes, for every command and for the final tree" } else { "NO, see the last column" });
    let _ = writeln!(md, "| Command | Real (ms) | Engine (ms) | Engine / real | Output (bytes) | Agree |");
    let _ = writeln!(md, "|---|---:|---:|---:|---:|---|");
    let mut csv = String::from("dataset,command,kind,real_ms,engine_ms,ratio,output_bytes,agree\n");
    for r in &rows {
        let ratio = r.engine.as_secs_f64() / r.real.as_secs_f64().max(1e-9);
        let _ = writeln!(
            md,
            "| `{}` | {} | {} | {:.2}x | {} | {} |",
            r.command.replace('|', "\\|"),
            ms(r.real),
            ms(r.engine),
            ratio,
            r.out_bytes,
            if r.agree { "yes".to_string() } else { format!("NO: {}", r.note) }
        );
        let _ = writeln!(
            csv,
            "{},\"{}\",{},{:.4},{:.4},{:.3},{},{}",
            name,
            r.command.replace('"', "\"\""),
            if r.mutating { "write" } else { "read" },
            r.real.as_secs_f64() * 1000.0,
            r.engine.as_secs_f64() * 1000.0,
            ratio,
            r.out_bytes,
            r.agree
        );
    }
    if !trees_agree {
        let _ = writeln!(md, "\nThe trees differ after the command list.");
    }
    let slower: Vec<&Row> = rows.iter().filter(|r| r.engine > r.real).collect();
    let _ = writeln!(md, "\n{} of {} commands were slower on the engine than on the real tools.", slower.len(), rows.len());
    std::fs::create_dir_all(&opts.out).map_err(|e| e.to_string())?;
    std::fs::write(opts.out.join(format!("{}.md", name)), &md).map_err(|e| e.to_string())?;
    std::fs::write(opts.out.join(format!("{}.csv", name)), &csv).map_err(|e| e.to_string())?;
    println!("{}", md);
    Ok(all_agree)
}

fn run() -> Result<bool, String> {
    let mut dataset = String::from("small");
    let mut dir: Option<PathBuf> = None;
    let mut export: Option<PathBuf> = None;
    let mut opts = Options {
        commands: "bench/commands.txt".into(),
        reps: 5,
        out: "bench/results".into(),
        scratch: std::env::temp_dir(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--dataset" => dataset = value("--dataset")?,
            "--dir" => dir = Some(value("--dir")?.into()),
            "--commands" => opts.commands = value("--commands")?.into(),
            "--reps" => opts.reps = value("--reps")?.parse().map_err(|_| "--reps needs a number")?,
            "--out" => opts.out = value("--out")?.into(),
            "--scratch" => opts.scratch = value("--scratch")?.into(),
            "--export" => export = Some(value("--export")?.into()),
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(true);
            }
            other => return Err(format!("unknown argument {}\n{}", other, USAGE)),
        }
    }
    if opts.reps == 0 {
        return Err("--reps must be at least 1".into());
    }
    if let Some(dir) = dir {
        let db = store::load_dir(&dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
        let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "dir".into());
        return bench(&name, db, &opts);
    }
    let specs: Vec<&Spec> = if dataset == "all" {
        SPECS.iter().collect()
    } else {
        vec![SPECS.iter().find(|s| s.name == dataset).ok_or(format!("unknown dataset {}", dataset))?]
    };
    let mut ok = true;
    for spec in specs {
        let db = generate(spec);
        if let Some(dir) = &export {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            store::dump_dir(&db, dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
            continue;
        }
        ok &= bench(spec.name, db, &opts)?;
    }
    Ok(ok)
}

fn main() -> std::process::ExitCode {
    let worker = std::thread::Builder::new().stack_size(512 << 20).spawn(run).expect("spawn");
    match worker.join().expect("benchmark panicked") {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(msg) => {
            eprintln!("qdb-bench: {}", msg);
            std::process::ExitCode::from(2)
        }
    }
}
