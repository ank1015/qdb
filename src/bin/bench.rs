//! The benchmark: runs a list of commands on the real GNU tools and on the
//! engine against the same data, checks that they agree, and records time
//! and memory for each.
//!
//! See bench/README.md.

use qdb::shell;
use qdb::store;
use qdb::testing::{check_safe, lines_digest, order_is_unspecified, sorted_lines, timed_real_to, Mapped};
use qdb::{Config, Db, SyncMode};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};

const USAGE: &str = "\
Usage: qdb-bench [OPTIONS]

Times a list of commands on the real GNU tools and on the engine.

  --dataset NAME    a generated dataset (small, medium, large) or one of
                    the real ones listed in bench/datasets.txt
  --dir PATH        use this directory as the dataset instead
  --data DIR        where the real datasets are unpacked
                    (default ../qdb-data/src, see bench/fetch.sh)
  --label NAME      what to call this run in the results (default dev);
                    one file per label is kept under bench/results/DATASET/
  --no-index        run the engine with the trigram index switched off
  --db DIR          run the engine on a database directory instead of in
                    memory: DIR is emptied, the dataset is imported into it,
                    it is compacted, closed and opened again, and then the
                    commands run. After the whole list the commands that
                    only read are timed a second time, on the database as
                    the writes left it; that run is stored as LABEL-after.
  --sync MODE       with --db: always (default) or batch
  --piece BYTES     the engine's piece size (default 65536)
  --paired          instead of the usual run: time each read-only command on
                    the engine alone, with searches using the index and
                    ignoring it by turns in one process
  --commands FILE   the command list (default bench/commands.txt)
  --reps N          timed runs per read-only command (default 5; fewer
                    for commands that take longer than 0.3 s)
  --filter TEXT     only run commands containing TEXT
  --out DIR         where to write results (default bench/results)
  --scratch DIR     where to put the real copy of the dataset
                    (default: the system temporary directory)
  --export DIR      only generate the dataset into DIR and stop
  --report          only rebuild the tables from the stored results";

// ------------------------------------------------------ memory accounting

/// Counts live heap bytes and their high-water mark, so that the peak
/// memory of one command can be read off exactly. The engine runs on one
/// thread, so plain loads and stores are enough.
use qdb::mem::{Counting, LIVE, PEAK};

#[global_allocator]
static ALLOC: Counting = Counting;

/// Resident memory of this process, from /proc.
fn resident_bytes() -> usize {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1).and_then(|p| p.parse::<usize>().ok()))
        .map_or(0, |pages| pages * 4096)
}

// ------------------------------------------------------------- datasets

use qdb::dataset::{file_content, generate, vocabulary, Rng, SPECS};

// ------------------------------------------------------------ the engine

/// One run of a command on the engine.
struct EngineRun {
    time: Duration,
    /// Most heap held during the run, above what was held before it.
    peak: usize,
    code: i32,
    out_bytes: usize,
    stdout: Bytes,
    stderr: Vec<u8>,
}

/// Output held in memory, or under `--cap` left in a file and read
/// through a mapping, so that holding it is not memory of the engine.
enum Bytes {
    Mem(Vec<u8>),
    Map(Mapped),
}

impl std::ops::Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Bytes::Mem(v) => v,
            Bytes::Map(m) => m,
        }
    }
}

static CAPPED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn read_output(path: &Path) -> Bytes {
    let out = if CAPPED.load(Relaxed) { Bytes::Map(Mapped::open(path).expect("reading output back")) } else { Bytes::Mem(std::fs::read(path).expect("reading output back")) };
    let _ = std::fs::remove_file(path);
    out
}

/// Run a command on the real tools. Under `--cap` its output goes to a
/// file.
fn real_run(root: &Path, command: &str) -> (Duration, i32, Bytes, Vec<u8>) {
    let command = &command.replace('⏎', "\n");
    if !CAPPED.load(Relaxed) {
        let (t, real) = run_real(root, command);
        return (t, real.code, Bytes::Mem(real.stdout), real.stderr);
    }
    let path = std::env::temp_dir().join(format!("qdb-bench-real-{}", std::process::id()));
    let (t, real) = timed_real_to(root, command, Some(&path));
    (t, real.code, read_output(&path), real.stderr)
}

/// Load the dataset into the engine, building the search index unless
/// it is switched off. Returns the time to read the files and the time
/// to read them and build the index.
///
/// With `--db` the dataset is imported into a database directory (sync
/// in batches), compacted, closed and opened again; both times are then
/// the import, and what opening took is among the facts returned.
fn engine_load(root: &Path, opts: &Options) -> Result<(Db, store::LoadReport, Duration, Duration, Vec<(String, String)>), String> {
    let cfg = Config { index: opts.index, piece: opts.piece, sync: opts.sync, ..Config::default() };
    let mut facts = Vec::new();
    let Some(dir) = &opts.db else {
        let mut db = Db::in_memory(Config { index: false, ..cfg });
        let mut report = store::load_into_db(&mut db, root).map_err(|e| e.to_string())?;
        if opts.index {
            let start = Instant::now();
            db.set_index(true);
            report.index_micros = Some(start.elapsed().as_micros() as u64);
        }
        let read = Duration::from_micros(report.read_micros);
        let total = read + Duration::from_micros(report.index_micros.unwrap_or(0));
        return Ok((db, report, read, total, facts));
    };
    let _ = std::fs::remove_dir_all(dir);
    let start = Instant::now();
    let (mut db, _) = Db::open_dir(dir, Config { sync: SyncMode::Batch, ..cfg.clone() })?;
    let report = store::load_into_db(&mut db, root).map_err(|e| e.to_string())?;
    db.sync();
    let load = start.elapsed();
    let start = Instant::now();
    db.compact()?;
    let compact = start.elapsed();
    let start = Instant::now();
    db.close()?;
    let close = start.elapsed();
    let start = Instant::now();
    let (db, opened) = Db::open_dir(dir, cfg)?;
    let open = start.elapsed();
    if !opened.messages().is_empty() {
        return Err(format!("opening after a clean close had work to do: {:?}", opened.messages()));
    }
    facts.push(("store".to_string(), format!("on disk, sync {}", if opts.sync == SyncMode::Always { "always" } else { "batch" })));
    facts.push(("compact_ms".to_string(), format!("{:.1}", millis(compact))));
    facts.push(("close_ms".to_string(), format!("{:.1}", millis(close))));
    facts.push(("open_ms".to_string(), format!("{:.2}", millis(open))));
    facts.extend(disk_facts(&db, "settled"));
    Ok((db, report, load, load, facts))
}

/// What is on disk, by what it is.
fn disk_facts(db: &Db, when: &str) -> Vec<(String, String)> {
    let s = db.disk_stats();
    let io = db.io_stats();
    [
        ("text", s.text),
        ("lists", s.lists),
        ("tables", s.tables),
        ("tree", s.tree),
        ("log", s.log),
        ("framing", s.framing),
        ("unreclaimed", s.unreclaimed),
        ("total", s.total()),
        ("raw_text", s.raw_text),
        ("data_files", s.data_files as u64),
        ("written_bytes", io.bytes()),
        ("syncs", io.syncs),
        ("commits", io.commits),
        ("checkpoints", io.checkpoints),
    ]
    .iter()
    .map(|(k, v)| (format!("disk_{}_{}", when, k), v.to_string()))
    .collect()
}

/// Facts about the engine's state for the results, as key and value.
fn engine_facts(db: &Db) -> Vec<(String, String)> {
    let s = match db.index_stats() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let lengths: Vec<String> = s
        .lists_by_length
        .iter()
        .enumerate()
        .filter(|(_, n)| **n > 0)
        .map(|(i, n)| if i < 2 { format!("{}:{}", i + 1, n) } else { format!("{}-{}:{}", (1usize << (i - 1)) + 1, 1usize << i, n) })
        .collect();
    let fact = |k: &str, v: String| (format!("index_{}", k), v);
    vec![
        fact("bytes", s.total_bytes().to_string()),
        fact("table_bytes", s.table_bytes.to_string()),
        fact("list_bytes", s.list_bytes.to_string()),
        fact("document_bytes", s.document_bytes.to_string()),
        fact("posting_bytes", s.posting_bytes.to_string()),
        fact("trigrams", s.trigrams.to_string()),
        fact("postings", s.postings.to_string()),
        fact("stale_postings", s.stale_postings.to_string()),
        fact("live_documents", s.live_documents.to_string()),
        fact("documents_ever", s.documents_ever.to_string()),
        fact("lists_by_length", lengths.join(" ")),
        fact("top100_posting_bytes", s.top_list_bytes[0].to_string()),
        fact("top1000_posting_bytes", s.top_list_bytes[1].to_string()),
        fact("top10000_posting_bytes", s.top_list_bytes[2].to_string()),
    ]
}

/// The index's running totals of work: documents added, documents
/// retired, postings added, bytes of lists rewritten, rebuilds.
fn engine_work(db: &Db) -> Option<[u64; 5]> {
    db.index_work().map(|w| [w.documents_added, w.documents_retired, w.postings_added, w.bytes_swept, w.rebuilds])
}

/// Counts output and throws it away.
#[derive(Default)]
struct Discard {
    bytes: usize,
}

impl qdb::Sink for Discard {
    fn write(&mut self, data: &[u8]) -> bool {
        self.bytes += data.len();
        true
    }
}

/// Keeps output in a file, so that holding it does not count as the
/// engine's memory.
struct ToFile {
    file: std::io::BufWriter<std::fs::File>,
    bytes: usize,
}

impl qdb::Sink for ToFile {
    fn write(&mut self, data: &[u8]) -> bool {
        use std::io::Write;
        self.bytes += data.len();
        self.file.write_all(data).expect("writing engine output");
        true
    }
}

/// Run a command on the engine through its streaming entry point. Output
/// is kept (for comparing with the real tools) only when asked for.
fn engine_run(db: &mut Db, command: &str, keep_output: bool) -> EngineRun {
    let command = &command.replace('⏎', "\n");
    let mut stderr: Vec<u8> = Vec::new();
    if !keep_output {
        let mut out = Discard::default();
        let base = LIVE.load(Relaxed);
        PEAK.store(base, Relaxed);
        let t = Instant::now();
        let code = qdb::execute_to(db, command.as_bytes(), &mut out, &mut stderr);
        let time = t.elapsed();
        let peak = PEAK.load(Relaxed) - base;
        return EngineRun { time, peak, code, out_bytes: out.bytes, stdout: Bytes::Mem(Vec::new()), stderr };
    }
    let path = std::env::temp_dir().join(format!("qdb-bench-out-{}", std::process::id()));
    let mut out = ToFile { file: std::io::BufWriter::new(std::fs::File::create(&path).expect("creating a file for engine output")), bytes: 0 };
    let base = LIVE.load(Relaxed);
    PEAK.store(base, Relaxed);
    let t = Instant::now();
    let code = qdb::execute_to(db, command.as_bytes(), &mut out, &mut stderr);
    let time = t.elapsed();
    let peak = PEAK.load(Relaxed) - base;
    let bytes = out.bytes;
    drop(out);
    let stdout = read_output(&path);
    EngineRun { time, peak, code, out_bytes: bytes, stdout, stderr }
}

// -------------------------------------------------------- real datasets

/// One line of bench/datasets.txt: name | folder under the data root |
/// where it came from.
struct RealDataset {
    name: String,
    folder: String,
    source: String,
}

fn real_datasets() -> Vec<RealDataset> {
    let text = std::fs::read_to_string("bench/datasets.txt").unwrap_or_default();
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut parts = l.splitn(3, '|').map(|p| p.trim().to_string());
            Some(RealDataset { name: parts.next()?, folder: parts.next()?, source: parts.next()? })
        })
        .collect()
}

use qdb::testing::{helper, timed_real as run_real};

fn shell_quote(arg: &[u8]) -> String {
    format!("'{}'", String::from_utf8_lossy(arg).replace('\'', "'\\''"))
}

/// The ripgrep command that does the same search as a recursive grep, on
/// one thread and without its ignore rules, or None if there is no direct
/// equivalent. Used for timing only: its output is never compared.
fn rg_equivalent(command: &str) -> Option<String> {
    let script = shell::parse(command.as_bytes()).ok()?;
    if script.items.len() != 1 || !script.items[0].rest.is_empty() {
        return None;
    }
    let pipeline = &script.items[0].first;
    if pipeline.commands.len() != 1 || !pipeline.commands[0].redirects.is_empty() {
        return None;
    }
    let words: Vec<Vec<u8>> = pipeline.commands[0].words.iter().map(|w| w.text()).collect();
    if words.first().map(|w| w.as_slice()) != Some(b"grep") {
        return None;
    }
    let mut flags: Vec<u8> = Vec::new();
    let mut operands: Vec<&[u8]> = Vec::new();
    for w in &words[1..] {
        if w.len() > 1 && w[0] == b'-' {
            flags.extend_from_slice(&w[1..]);
        } else {
            operands.push(w);
        }
    }
    if !flags.contains(&b'r') || operands.is_empty() {
        return None;
    }
    let pattern = operands[0];
    let fixed = flags.contains(&b'F');
    let extended = flags.contains(&b'E');
    // A basic regular expression means the same to ripgrep only if it has
    // none of the characters the two syntaxes read differently.
    if !fixed && !extended && pattern.iter().any(|c| b"\\(){}|+?".contains(c)) {
        return None;
    }
    let mut rg = String::from("rg -j 1 --no-ignore --hidden --no-config --no-heading");
    rg.push_str(if flags.contains(&b'n') { " -n" } else { " -N" });
    for (flag, arg) in [(b'i', " -i"), (b'l', " -l"), (b'c', " -c"), (b'v', " -v"), (b'F', " -F")] {
        if flags.contains(&flag) {
            rg.push_str(arg);
        }
    }
    let _ = write!(rg, " -e {}", shell_quote(pattern));
    for path in operands[1..].iter().copied().chain(if operands.len() == 1 { Some(&b"."[..]) } else { None }) {
        let _ = write!(rg, " {}", shell_quote(path));
    }
    Some(rg)
}

// ------------------------------------------------------------ parameters

/// The values that fill the {NAME} placeholders of the command list for
/// one dataset: search terms chosen by how many files contain them, and a
/// few paths. Kept in bench/terms/DATASET.txt so every run uses the same.
struct Terms {
    values: BTreeMap<String, String>,
    notes: BTreeMap<String, String>,
}

impl Terms {
    fn read(path: &Path) -> Option<Terms> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut t = Terms { values: BTreeMap::new(), notes: BTreeMap::new() };
        for line in text.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let (key, rest) = line.split_once('=')?;
            let (value, note) = match rest.split_once("  # ") {
                Some((v, n)) => (v, n),
                None => (rest, ""),
            };
            t.values.insert(key.trim().to_string(), value.to_string());
            t.notes.insert(key.trim().to_string(), note.to_string());
        }
        Some(t)
    }

    fn write(&self, path: &Path, dataset: &str) -> Result<(), String> {
        let mut text = format!(
            "# Values for the placeholders in bench/commands.txt, for the {} dataset.\n\
             # Chosen by qdb-bench from measured frequencies; delete this file to choose again.\n",
            dataset
        );
        for (k, v) in &self.values {
            let _ = writeln!(text, "{}={}  # {}", k, v, self.notes.get(k).map(|s| s.as_str()).unwrap_or(""));
        }
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    /// Fill the placeholders of one command. None if one has no value.
    fn fill(&self, template: &str) -> Option<String> {
        let mut out = String::new();
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            match after.find('}') {
                Some(close) if close > 0 && after[..close].bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) => {
                    out.push_str(self.values.get(&after[..close])?);
                    rest = &after[close + 1..];
                }
                _ => {
                    out.push('{');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        Some(out)
    }
}

fn plain_name(name: &[u8]) -> bool {
    !name.is_empty() && name.iter().all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(c)) && name[0] != b'-'
}

/// How many files the real grep finds for a pattern.
fn real_file_count(root: &Path, flags: &str, pattern: &str) -> usize {
    let (_, out) = run_real(root, &format!("grep {} -e {} . | wc -l", flags, shell_quote(pattern.as_bytes())));
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0)
}

fn pick_terms(db: &Db, root: &Path) -> Terms {
    let snapshot = db.snapshot();
    let files: Vec<(&[u8], &[u8])> = snapshot.iter().filter_map(|(p, c)| c.as_ref().map(|c| (p.as_slice(), c.as_slice()))).collect();
    let n = files.len().max(1);
    let mut terms = Terms { values: BTreeMap::new(), notes: BTreeMap::new() };
    let set = |terms: &mut Terms, key: &str, value: String, note: String| {
        terms.values.insert(key.into(), value);
        terms.notes.insert(key.into(), note);
    };

    // In how many files does each word occur, and each word prefix that is
    // followed by an underscore?
    let mut word_files: HashMap<&[u8], u32> = HashMap::new();
    let mut prefix_files: HashMap<&[u8], u32> = HashMap::new();
    for (_, content) in &files {
        let mut seen: HashSet<&[u8]> = HashSet::new();
        let mut seen_prefix: HashSet<&[u8]> = HashSet::new();
        let mut i = 0;
        while i < content.len() {
            if !(content[i].is_ascii_alphabetic() || content[i] == b'_') {
                i += 1;
                continue;
            }
            let start = i;
            while i < content.len() && (content[i].is_ascii_alphanumeric() || content[i] == b'_') {
                i += 1;
            }
            let word = &content[start..i];
            if !(3..=20).contains(&word.len()) || !word.iter().all(|c| c.is_ascii_lowercase() || *c == b'_') || word[0] == b'_' {
                continue;
            }
            if seen.insert(word) {
                *word_files.entry(word).or_insert(0) += 1;
            }
            if let Some(us) = word.iter().position(|c| *c == b'_') {
                if us >= 3 && us + 1 < word.len() && seen_prefix.insert(&word[..us]) {
                    *prefix_files.entry(&word[..us]).or_insert(0) += 1;
                }
            }
        }
    }
    let mut words: Vec<(&[u8], u32)> = word_files.into_iter().collect();
    words.sort();

    // A word counted in k files can still be found by grep in more, as part
    // of a longer word. So take the nearest few and ask the real grep.
    let pct = |count: usize| format!("{} of {} files, {:.2}%", count, n, 100.0 * count as f64 / n as f64);
    let choose = |terms: &mut Terms, key: &str, target: usize, ok: &dyn Fn(usize) -> bool, min_len: usize| {
        let mut near: Vec<&(&[u8], u32)> = words.iter().filter(|(w, _)| w.len() >= min_len && !w.contains(&b'_')).collect();
        near.sort_by_key(|(w, c)| ((*c as i64 - target as i64).abs(), std::cmp::Reverse(w.len()), *w));
        let mut best: Option<(String, usize)> = None;
        for (w, _) in near.into_iter().take(8) {
            let word = String::from_utf8_lossy(w).into_owned();
            let count = real_file_count(root, "-rlF", &word);
            if !ok(count) {
                continue;
            }
            let better = best.as_ref().map_or(true, |(_, c)| (count as i64 - target as i64).abs() < (*c as i64 - target as i64).abs());
            if better {
                best = Some((word, count));
            }
        }
        if let Some((word, count)) = best {
            set(terms, key, word, pct(count));
        }
    };
    let share = |per_thousand: usize| (n * per_thousand).div_ceil(1000).max(2);
    choose(&mut terms, "ONE", 1, &|c| c == 1, 6);
    choose(&mut terms, "RARE", share(1), &|c| c >= 2, 6);
    choose(&mut terms, "FEW", share(10), &|c| c >= 2, 6);
    choose(&mut terms, "SOME", share(100), &|c| c >= 2, 6);
    choose(&mut terms, "COMMON", share(600), &|c| 2 * c > n, 5);
    if !terms.values.contains_key("COMMON") {
        choose(&mut terms, "COMMON", share(600), &|c| 2 * c > n, 3);
    }
    let absent = "qdbzzabsentterm";
    set(&mut terms, "ABSENT", absent.into(), pct(real_file_count(root, "-rlF", absent)));
    if let Some(rare) = terms.values.get("RARE").cloned() {
        set(&mut terms, "RAREUP", rare.to_ascii_uppercase(), "the RARE term in capitals, for -i".into());
    }

    // A prefix for the pattern PREFIX_[a-z]+\( .
    let mut prefixes: Vec<(&[u8], u32)> = prefix_files.into_iter().collect();
    prefixes.sort_by_key(|(w, c)| ((*c as i64 - share(50) as i64).abs(), *w));
    for (w, _) in prefixes.into_iter().take(8) {
        let prefix = String::from_utf8_lossy(w).into_owned();
        let count = real_file_count(root, "-rlE", &format!("{}_[a-z]+\\(", prefix));
        if count > 0 {
            set(&mut terms, "PREFIX", prefix, format!("PREFIX_[a-z]+\\( matches in {}", pct(count)));
            break;
        }
    }

    // Folders by the share of all files underneath them.
    let mut under: BTreeMap<&[u8], usize> = BTreeMap::new();
    let mut direct: BTreeMap<(&[u8], &[u8]), Vec<(&[u8], usize)>> = BTreeMap::new();
    let mut by_ext: BTreeMap<&[u8], usize> = BTreeMap::new();
    for (path, content) in &files {
        let mut ok = true;
        for (i, c) in path.iter().enumerate() {
            if *c == b'/' {
                ok &= plain_name(path[..i].rsplit(|c| *c == b'/').next().unwrap());
                if ok && path[..i].iter().filter(|c| **c == b'/').count() < 3 {
                    *under.entry(&path[..i]).or_insert(0) += 1;
                }
            }
        }
        let (dir, name) = match path.iter().rposition(|c| *c == b'/') {
            Some(i) => (&path[..i], &path[i + 1..]),
            None => (&b""[..], *path),
        };
        if !ok || !plain_name(name) {
            continue;
        }
        if let Some(dot) = name.iter().rposition(|c| *c == b'.') {
            let ext = &name[dot + 1..];
            if dot > 0 && !ext.is_empty() && ext.iter().all(|c| c.is_ascii_alphanumeric()) {
                direct.entry((dir, ext)).or_default().push((path, content.len()));
                *by_ext.entry(ext).or_insert(0) += 1;
            }
        }
    }
    let nearest = |target: usize, lo: usize, hi: usize| {
        under.iter().filter(|(_, c)| **c >= lo && **c <= hi).min_by_key(|(p, c)| ((**c as i64 - target as i64).abs(), **p)).map(|(p, c)| (*p, *c))
    };
    if let Some((dir, count)) = nearest(n * 15 / 100, n * 3 / 100, n * 40 / 100) {
        set(&mut terms, "DIR", String::from_utf8_lossy(dir).into_owned(), format!("holds {}", pct(count)));
    }
    if let Some((dir, count)) = nearest((n / 100).max(3), 2, n * 3 / 100 + 3) {
        set(&mut terms, "SMALLDIR", String::from_utf8_lossy(dir).into_owned(), format!("holds {}", pct(count)));
    }
    // The folder with the most files of one extension directly inside it.
    if let Some(((dir, ext), group)) = direct.iter().filter(|(_, g)| g.len() <= 1500).max_by_key(|((d, e), g)| (g.len(), std::cmp::Reverse((*d, *e)))) {
        let dir_s = String::from_utf8_lossy(dir).into_owned();
        let ext_s = String::from_utf8_lossy(ext).into_owned();
        let glob = if dir.is_empty() { format!("*.{}", ext_s) } else { format!("{}/*.{}", dir_s, ext_s) };
        set(&mut terms, "GLOB", glob, format!("{} files", group.len()));
        if !dir.is_empty() {
            set(&mut terms, "GLOBDIR", dir_s, "the folder of GLOB".into());
        }
        set(&mut terms, "EXT", ext_s, format!("{} files in the whole dataset", by_ext[ext]));
        if let Some((path, size)) = group.iter().min_by_key(|(p, size)| ((*size as i64 - 20_000).abs(), *p)) {
            set(&mut terms, "FILE", String::from_utf8_lossy(path).into_owned(), format!("{} bytes", size));
        }
        if let Some((ext2, count)) = by_ext.iter().filter(|(e, _)| *e != ext).max_by_key(|(e, c)| (**c, std::cmp::Reverse(**e))) {
            set(&mut terms, "EXT2", String::from_utf8_lossy(ext2).into_owned(), format!("{} files in the whole dataset", count));
        }
    }
    terms
}

/// Have the operating system forget the file content it holds in memory.
fn drop_caches() -> Result<(), String> {
    unsafe { libc::sync() };
    std::fs::write("/proc/sys/vm/drop_caches", "3").map_err(|e| format!("dropping the file cache: {}", e))
}

/// `ls -l` output with what the engine has no fact for taken out: owner
/// and group, the size of a folder (the real one grows with its history)
/// and the `total` lines that add those sizes up.
fn long_listing_normal(out: &[u8]) -> Vec<u8> {
    let mut to = Vec::with_capacity(out.len());
    for line in out.split_inclusive(|c| *c == b'\n') {
        if line.starts_with(b"total ") {
            to.extend_from_slice(b"total\n");
            continue;
        }
        let entry = line.len() > 11 && matches!(line[0], b'-' | b'd') && line[1..10].iter().all(|c| b"rwx-".contains(c)) && line[10] == b' ';
        if !entry {
            to.extend_from_slice(line);
            continue;
        }
        let mut at = 0;
        for field in 0..5 {
            while line.get(at) == Some(&b' ') {
                at += 1;
            }
            let start = at;
            while line.get(at).is_some_and(|c| *c != b' ' && *c != b'\n') {
                at += 1;
            }
            match field {
                0 | 1 => to.extend_from_slice(&line[start..at]),
                4 if line[0] != b'd' => to.extend_from_slice(&line[start..at]),
                _ => to.push(b'-'),
            }
            to.push(b' ');
        }
        to.extend_from_slice(&line[at.min(line.len())..]);
    }
    to
}

/// The same lines, in any order.
fn same_lines(a: &[u8], b: &[u8]) -> bool {
    if CAPPED.load(Relaxed) {
        // Sorting would take memory in step with the output.
        lines_digest(a) == lines_digest(b)
    } else {
        sorted_lines(a) == sorted_lines(b)
    }
}

// ------------------------------------------------------------ measuring

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn ms(d: f64) -> String {
    if d < 0.0 {
        String::new()
    } else if d < 10.0 {
        format!("{:.3}", d)
    } else if d < 100.0 {
        format!("{:.2}", d)
    } else {
        format!("{:.1}", d)
    }
}

fn millis(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn size(bytes: usize) -> String {
    if bytes < 10_000 {
        format!("{} B", bytes)
    } else if bytes < 10_000_000 {
        format!("{:.1} KB", bytes as f64 / 1e3)
    } else {
        format!("{:.1} MB", bytes as f64 / 1e6)
    }
}

fn tool_version(tool: &str) -> String {
    Command::new(tool)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().next().map(|l| l.to_string()))
        .unwrap_or_else(|| "not installed".into())
}

struct Options {
    commands: PathBuf,
    reps: usize,
    out: PathBuf,
    scratch: PathBuf,
    label: String,
    index: bool,
    paired: bool,
    filter: Option<String>,
    db: Option<PathBuf>,
    sync: SyncMode,
    piece: usize,
    cap_mb: Option<usize>,
    /// Before every read-only command: close the database, have the
    /// operating system forget every file it holds in memory, open again.
    cold: bool,
}

/// How many more timed runs a command gets after its first one.
fn more_reps(first: Duration, reps: usize) -> usize {
    if first > Duration::from_secs(2) {
        0
    } else if first > Duration::from_millis(300) {
        reps.min(3) - 1
    } else {
        reps - 1
    }
}

/// The same for the engine. Its runs cost little, so a quick command gets
/// many more of them: a difference of a tenth between two runs of the
/// benchmark should mean something even for a command that takes
/// microseconds.
fn more_engine_reps(first: Duration, reps: usize) -> usize {
    if first < Duration::from_millis(2) {
        reps * 10
    } else if first < Duration::from_millis(50) {
        reps * 3
    } else {
        more_reps(first, reps)
    }
}

/// Whether a later stage keeps only part of what an unordered stage
/// printed, so that the real result depends on directory order.
fn order_dependent(command: &str) -> bool {
    let script = match shell::parse(command.as_bytes()) {
        Ok(s) => s,
        _ => return false,
    };
    order_is_unspecified(command)
        && script.all_commands().iter().skip(1).any(|c| matches!(c.words.first().map(|w| w.text()).as_deref(), Some(b"head") | Some(b"tail")))
}

struct Row {
    command: String,
    kind: &'static str,
    gnu_ms: f64,
    rg_ms: f64,
    engine_ms: f64,
    out_bytes: usize,
    peak_bytes: usize,
    agree: String,
    /// For a command that changes data with the index on: what the index
    /// did for it.
    index_work: String,
}

fn csv_field(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    fields.push(cur);
    fields
}

/// Where the real tools' copy of a dataset lives.
fn scratch_root(name: &str, opts: &Options) -> (PathBuf, PathBuf) {
    let base = opts.scratch.join(format!("qdb-bench-{}", name));
    let root = base.join("root");
    (base, root)
}

/// True if the copy for the real tools is already there. It is made once
/// per dataset; `source` is recorded beside it and must still match.
fn prepared(name: &str, source: &str, opts: &Options) -> bool {
    let (base, _) = scratch_root(name, opts);
    std::fs::read_to_string(base.join("stamp")).ok().as_deref() == Some(source)
}

/// Write the dataset out for the real tools: exactly what the engine
/// holds, with anything the loader skips already gone.
fn prepare(name: &str, source: &str, db: &Db, opts: &Options) -> Result<(), String> {
    let (base, root) = scratch_root(name, opts);
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    store::dump_dir(db, &root).map_err(|e| format!("writing the dataset: {}", e))?;
    std::fs::write(base.join("stamp"), source).map_err(|e| e.to_string())
}

/// Run this program again with the same arguments. Measuring starts in a
/// fresh process so that memory used to prepare the data is not counted.
fn run_again() -> Result<bool, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = Command::new(exe).args(std::env::args().skip(1)).status().map_err(|e| e.to_string())?;
    Ok(status.success())
}

fn bench(name: &str, source: &str, opts: &Options) -> Result<bool, String> {
    let text = std::fs::read_to_string(&opts.commands).map_err(|e| format!("{}: {}", opts.commands.display(), e))?;
    let templates: Vec<(String, bool)> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.strip_prefix("mut: ") {
            Some(c) => (c.to_string(), true),
            None => (l.strip_prefix("nocmp: ").unwrap_or(l).to_string(), false),
        })
        .collect();
    // Timed on both sides, but the output is known to differ (see the
    // command list for why) and is not compared.
    let nocmp: std::collections::HashSet<String> = text.lines().filter_map(|l| l.trim().strip_prefix("nocmp: ")).map(|l| l.to_string()).collect();

    let (_, root) = scratch_root(name, opts);
    let _ = std::fs::remove_dir_all(root.join("bench_out"));

    let terms_path = PathBuf::from("bench/terms").join(format!("{}.txt", name));
    let live_before = LIVE.load(Relaxed);
    // Start the helper that runs the real tools while this process is
    // still small.
    run_real(&root, "true");
    if let Some(mb) = opts.cap_mb {
        CAPPED.store(true, Relaxed);
        // From here on the engine's own memory (everything that is not a
        // file) is limited: asking for more ends the run.
        let lim = libc::rlimit { rlim_cur: (mb as u64) << 20, rlim_max: (mb as u64) << 20 };
        if unsafe { libc::setrlimit(libc::RLIMIT_DATA, &lim) } != 0 {
            return Err(format!("setting the memory limit: {}", std::io::Error::last_os_error()));
        }
    }
    let (mut db, report, load, load_with_index, load_facts) = engine_load(&root, opts)?;
    let heap_after_load = LIVE.load(Relaxed) - live_before;
    // Taken now, before any command has changed the data.
    let mut facts = engine_facts(&db);
    facts.extend(load_facts);
    let rss_after_load = resident_bytes();
    eprintln!("{}: {} files, {} folders, {:.1} MB, loaded in {} ms", name, report.files, report.folders, report.bytes as f64 / 1e6, ms(millis(load_with_index)));

    let terms = match Terms::read(&terms_path) {
        Some(t) => t,
        None => {
            eprintln!("{}: choosing search terms", name);
            let t = pick_terms(&db, &root);
            t.write(&terms_path, name)?;
            t
        }
    };

    if opts.paired {
        return paired(name, &mut db, &terms, &templates, opts);
    }

    // What starting a process costs the real tools, for reference.
    let startup = median((0..5).map(|_| run_real(&root, "echo -n").0).collect());

    // The same write on every dataset: a new file with the same 64 KB of
    // generated text, to show what a write costs as the data grows.
    let fixed_write = {
        let mut rng = Rng(0xF1BED);
        let vocab = vocabulary(&mut rng);
        let mut texts: Vec<Vec<u8>> = Vec::new();
        for _ in 0..9 {
            let mut text = Vec::new();
            while text.len() < 64 * 1024 {
                text.extend_from_slice(&file_content(&mut rng, &vocab, "c", 200));
                if !text.ends_with(b"\n") {
                    text.push(b'\n');
                }
            }
            text.truncate(64 * 1024);
            texts.push(text);
        }
        let top = db.root();
        let folder = db.create_folder(top, b"qdb_bench_fixed");
        let mut times = Vec::new();
        for (i, text) in texts.into_iter().enumerate() {
            let t = Instant::now();
            db.create_file(folder, format!("{}.c", i).as_bytes(), &text);
            times.push(t.elapsed());
        }
        db.remove(folder);
        median(times)
    };

    let mut all_agree = true;
    let mut trees_agree = true;
    let dir = opts.out.join(name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // On disk there is a second pass: the commands that only read, on the
    // database as the writes of the first pass left it.
    let passes = if opts.db.is_some() && opts.filter.is_none() && !opts.cold { 2 } else { 1 };
    for pass in 0..passes {
    let label = if pass == 0 { opts.label.clone() } else { format!("{}-after", opts.label) };
    if pass == 1 {
        eprintln!("{}: the commands that only read, again, after the writes", name);
        facts.retain(|(k, _)| !k.starts_with("disk_"));
        facts.extend(disk_facts(&db, "after"));
    }
    let mut rows: Vec<Row> = Vec::new();
    for (template, mutating) in &templates {
        if pass == 1 && *mutating {
            continue;
        }
        let command = match terms.fill(template) {
            Some(c) => c,
            None => {
                eprintln!("  skipped, no value for a placeholder: {}", template);
                continue;
            }
        };
        if opts.filter.as_ref().is_some_and(|f| !command.contains(f.as_str())) && !mutating {
            continue;
        }
        check_safe(&command.replace('⏎', "\n"), &root)?;
        // A sort after the walk puts its lines in one order again.
        let sorted_later = command.contains("| sort");
        let unordered = order_is_unspecified(&command) && !sorted_later;
        let not_compared = nocmp.contains(template);
        let long_listing = command.split(['|', ';', '&']).any(|c| {
            let mut w = c.split_whitespace();
            w.next() == Some("ls") && w.any(|f| f.starts_with('-') && !f.starts_with("--") && f.contains('l'))
        });

        if opts.cold && !mutating {
            drop_caches()?;
        }
        // The real tools.
        let (first, real_code, real_stdout, real_stderr) = real_run(&root, &command);
        let mut real_times = vec![first];
        if !mutating {
            for _ in 0..more_reps(first, opts.reps) {
                real_times.push(real_run(&root, &command).0);
            }
        }
        let rg_ms = match rg_equivalent(&command).filter(|_| !mutating && have_rg()) {
            None => -1.0,
            Some(rg) => {
                let first = real_run(&root, &rg).0;
                let mut times = vec![first];
                for _ in 0..more_reps(first, opts.reps) {
                    times.push(real_run(&root, &rg).0);
                }
                millis(median(times))
            }
        };

        // The engine.
        if let Some(dir) = opts.db.as_ref().filter(|_| opts.cold && !mutating) {
            // Closed and opened again, so that nothing of the files is
            // in this process either.
            db.close()?;
            drop_caches()?;
            db = Db::open_dir(dir, Config { index: opts.index, piece: opts.piece, sync: opts.sync, ..Config::default() })?.0;
        }
        let work_before = engine_work(&db);
        let first = engine_run(&mut db, &command, true);
        let index_work = match (work_before, engine_work(&db)) {
            (Some(b), Some(a)) if *mutating => format!(
                "documents +{} -{}; postings +{}; list bytes rewritten {}; rebuilds {}",
                a[0].wrapping_sub(b[0]),
                a[1].wrapping_sub(b[1]),
                a[2].wrapping_sub(b[2]),
                a[3].wrapping_sub(b[3]),
                a[4] - b[4]
            ),
            _ => String::new(),
        };
        let mut engine_times = vec![first.time];
        let mut peak = first.peak;
        if !mutating {
            for _ in 0..more_engine_reps(first.time, opts.reps) {
                let again = engine_run(&mut db, &command, false);
                engine_times.push(again.time);
                peak = again.peak;
            }
        }

        // Under the memory limit only a listing of moderate size is
        // brought into memory to be masked.
        let (real_stdout, engine_stdout) = if long_listing && (!CAPPED.load(Relaxed) || real_stdout.len().max(first.stdout.len()) < 64 << 20) {
            (Bytes::Mem(long_listing_normal(&real_stdout)), Bytes::Mem(long_listing_normal(&first.stdout)))
        } else {
            (real_stdout, first.stdout)
        };
        let agree = if not_compared {
            "not compared".to_string()
        } else if real_code != first.code {
            format!("NO: exit status, real {}, engine {}", real_code, first.code)
        } else if order_dependent(&command) && !sorted_later {
            let lines = |b: &[u8]| b.iter().filter(|c| **c == b'\n').count();
            if lines(&real_stdout) == lines(&engine_stdout) {
                "order differs".to_string()
            } else {
                "NO: number of lines differs".to_string()
            }
        } else if (unordered && !same_lines(&real_stdout, &engine_stdout)) || (!unordered && real_stdout[..] != engine_stdout[..]) {
            "NO: standard output differs".to_string()
        } else if real_stderr != first.stderr {
            "NO: standard error differs".to_string()
        } else {
            "yes".to_string()
        };
        all_agree &= !agree.starts_with("NO");
        let row = Row {
            command: command.clone(),
            kind: if *mutating { "write" } else { "read" },
            gnu_ms: millis(median(real_times)),
            rg_ms,
            engine_ms: millis(median(engine_times)),
            out_bytes: first.out_bytes,
            peak_bytes: peak,
            agree,
            index_work,
        };
        eprintln!("  {:>10} gnu {:>10} rg {:>10} engine {:>10} peak  {}  {}", ms(row.gnu_ms), ms(row.rg_ms), ms(row.engine_ms), size(row.peak_bytes), row.command, if row.agree == "yes" { "" } else { &row.agree });
        rows.push(row);
    }

    // After the whole list, both sides must hold the same tree.
    if pass == 0 {
        trees_agree = opts.filter.is_some()
            || match store::same_as_dir(&db, &root).map_err(|e| e.to_string())? {
                Ok(()) => true,
                Err(what) => {
                    eprintln!("{}: {}", name, what);
                    false
                }
            };
        all_agree &= trees_agree;
        if opts.db.is_some() {
            if let Err(e) = db.check() {
                eprintln!("{}: check after the command list: {}", name, e);
                all_agree = false;
            }
        }
    }
    let mut csv = String::from("command,kind,gnu_ms,rg_ms,engine_ms,output_bytes,peak_bytes,agree,index_work\n");
    for r in &rows {
        let rg = if r.rg_ms < 0.0 { String::new() } else { format!("{:.4}", r.rg_ms) };
        let _ = writeln!(csv, "{},{},{:.4},{},{:.4},{},{},{},{}", csv_field(&r.command), r.kind, r.gnu_ms, rg, r.engine_ms, r.out_bytes, r.peak_bytes, csv_field(&r.agree), csv_field(&r.index_work));
    }
    let mut meta = String::new();
    let _ = writeln!(meta, "source={}", source);
    let _ = writeln!(meta, "files={}", report.files);
    let _ = writeln!(meta, "folders={}", report.folders);
    let _ = writeln!(meta, "bytes={}", report.bytes);
    let _ = writeln!(meta, "index={}", if opts.index { "on" } else { "off" });
    let _ = writeln!(meta, "load_ms={:.1}", millis(load));
    let _ = writeln!(meta, "load_with_index_ms={:.1}", millis(load_with_index));
    let _ = writeln!(meta, "index_build_ms={:.1}", millis(load_with_index - load));
    let _ = writeln!(meta, "heap_after_load={}", heap_after_load);
    let _ = writeln!(meta, "rss_after_load={}", rss_after_load);
    let _ = writeln!(meta, "startup_ms={:.3}", millis(startup));
    let _ = writeln!(meta, "fixed_write_ms={:.4}", millis(fixed_write));
    let _ = writeln!(meta, "trees_agree={}", trees_agree);
    let _ = writeln!(meta, "all_agree={}", all_agree);
    let _ = writeln!(meta, "tools={}; {}; {}; {}; {}; {}", tool_version("bash"), tool_version("grep"), tool_version("sed"), tool_version("ls"), tool_version("find"), tool_version("rg"));
    for (k, v) in &facts {
        let _ = writeln!(meta, "{}={}", k, v);
    }
    if opts.filter.is_none() {
        std::fs::write(dir.join(format!("{}.csv", label)), csv).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{}.meta", label)), meta).map_err(|e| e.to_string())?;
    }
    }
    if opts.filter.is_none() {
        report_dataset(name, opts)?;
    }
    if opts.db.is_some() {
        db.close()?;
    }
    if !trees_agree {
        eprintln!("{}: the trees differ after the command list", name);
    }
    Ok(all_agree)
}

/// Index used against index ignored, for commands that only read, in one
/// process and by turns. Two separate runs of the benchmark differ by more
/// than a tenth for reasons that have nothing to do with the index (where
/// memory happens to land, mostly), so a difference that small can only be
/// seen this way.
fn paired(name: &str, db: &mut Db, terms: &Terms, templates: &[(String, bool)], opts: &Options) -> Result<bool, String> {
    let mut csv = String::from("command,ignored_ms,used_ms,ratio\n");
    for (template, mutating) in templates {
        let command = match terms.fill(template) {
            Some(c) if !mutating => c,
            _ => continue,
        };
        if opts.filter.as_ref().is_some_and(|f| !command.contains(f.as_str())) {
            continue;
        }
        let first = engine_run(db, &command, false).time;
        let pairs = if first > Duration::from_millis(300) {
            3
        } else if first > Duration::from_millis(20) {
            9
        } else {
            41
        };
        let (mut used, mut ignored) = (Vec::new(), Vec::new());
        for i in 0..pairs * 2 {
            // Alternate which goes first.
            let consult = (i % 2 == 0) == (i % 4 < 2);
            db.set_index_consulted(consult);
            let t = engine_run(db, &command, false).time;
            if consult { used.push(t) } else { ignored.push(t) }
        }
        db.set_index_consulted(true);
        let (u, i) = (millis(median(used)), millis(median(ignored)));
        eprintln!("  {:>10} ignored {:>10} used  {:>5.2}  {}", ms(i), ms(u), u / i.max(1e-9), command);
        let _ = writeln!(csv, "{},{:.5},{:.5},{:.3}", csv_field(&command), i, u, u / i.max(1e-9));
    }
    if opts.filter.is_none() {
        let dir = opts.out.join(name);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{}.csv", opts.label)), csv).map_err(|e| e.to_string())?;
    }
    Ok(true)
}

fn have_rg() -> bool {
    static HAVE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *HAVE.get_or_init(|| Command::new("rg").arg("--version").output().is_ok_and(|o| o.status.success()))
}

// -------------------------------------------------------------- reporting

/// The runs a table shows, in order: label, column heading.
const LABELS: &[(&str, &str)] = &[
    ("part4", "Stage 2, in memory"),
    ("s3-disk", "Stage 3, on disk, settled"),
    ("s4-mem", "Stage 4, in memory"),
    ("s4-disk", "Stage 4, on disk, settled"),
    ("s4-disk-after", "Stage 4, on disk, after the writes"),
    ("s4-disk-cap512", "Stage 4, on disk, 512 MB limit"),
    ("s4-disk-small", "Stage 4, on disk, 1 GB for everything, file cache included"),
];

struct Stored {
    rows: Vec<Vec<String>>,
    meta: BTreeMap<String, String>,
}

fn read_stored(dir: &Path, label: &str) -> Option<Stored> {
    let csv = std::fs::read_to_string(dir.join(format!("{}.csv", label))).ok()?;
    let meta = std::fs::read_to_string(dir.join(format!("{}.meta", label))).unwrap_or_default();
    Some(Stored {
        rows: csv.lines().skip(1).map(parse_csv_line).collect(),
        meta: meta.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    })
}

/// Rebuild bench/results/DATASET.md from every stored run of a dataset.
fn report_dataset(name: &str, opts: &Options) -> Result<(), String> {
    let dir = opts.out.join(name);
    let mut labels: Vec<(String, String)> = LABELS.iter().map(|(l, h)| (l.to_string(), h.to_string())).collect();
    if !labels.iter().any(|(l, _)| *l == opts.label) {
        labels.push((opts.label.clone(), opts.label.clone()));
    }
    let runs: Vec<(String, String, Stored)> = labels.into_iter().filter_map(|(l, h)| read_stored(&dir, &l).map(|s| (l, h, s))).collect();
    // The real tools' times, and the facts about the index, come from the
    // settled run on disk when there is one: the runs after it in the list
    // were made under limits.
    let last = match runs.iter().find(|r| r.0 == "s4-disk").or(runs.last()) {
        Some(r) => r,
        None => return Ok(()),
    };
    let get = |s: &Stored, k: &str| s.meta.get(k).cloned().unwrap_or_default();
    let num = |s: &Stored, k: &str| get(s, k).parse::<f64>().unwrap_or(0.0);
    let mut md = String::new();
    let _ = writeln!(md, "# Benchmark: {}\n", name);
    let _ = writeln!(md, "- Data: {}", get(&last.2, "source"));
    let _ = writeln!(md, "- Size: {} files in {} folders, {:.1} MB of text", get(&last.2, "files"), get(&last.2, "folders"), num(&last.2, "bytes") / 1e6);
    let _ = writeln!(md, "- Tools: {}", get(&last.2, "tools"));
    let _ = writeln!(md, "- GNU and rg times are from the latest run ({}), and include starting bash and the tool ({} ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.", last.1, get(&last.2, "startup_ms"));
    let _ = writeln!(md, "- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.");
    let _ = writeln!(md, "- Peak memory is the most heap the engine held during the command, above what it held before it.\n");

    let _ = writeln!(md, "## Loading and memory\n");
    let _ = writeln!(md, "| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |");
    let _ = writeln!(md, "|---|---|---:|---:|---:|---:|---:|---|");
    for (label, heading, s) in &runs {
        // The engine had no index before part 4.
        let index = if matches!(label.as_str(), "before" | "part2" | "part3") { "none".to_string() } else { get(s, "index") };
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            heading,
            index,
            ms(num(s, "load_ms")),
            ms(num(s, "load_with_index_ms")),
            size(num(s, "heap_after_load") as usize),
            size(num(s, "rss_after_load") as usize),
            if s.meta.contains_key("fixed_write_ms") { format!("{:.3}", num(s, "fixed_write_ms")) } else { String::new() },
            if get(s, "all_agree") == "true" { "yes" } else { "NO" }
        );
    }
    for (_, heading, s) in &runs {
        if !s.meta.contains_key("index_trigrams") {
            continue;
        }
        let text = num(s, "bytes");
        let posting = num(s, "index_posting_bytes");
        let share = |k: &str| 100.0 * num(s, k) / posting.max(1.0);
        let _ = writeln!(md, "\n## The index ({})\n", heading);
        let _ = writeln!(md, "- Built in {} ms while loading.", ms(num(s, "index_build_ms")));
        let _ = writeln!(
            md,
            "- Size: {} in memory, {:.1}% of the text ({} table, {} lists, {} document table). The posting data itself is {}.",
            size(num(s, "index_bytes") as usize),
            100.0 * num(s, "index_bytes") / text.max(1.0),
            size(num(s, "index_table_bytes") as usize),
            size(num(s, "index_list_bytes") as usize),
            size(num(s, "index_document_bytes") as usize),
            size(posting as usize)
        );
        let _ = writeln!(md, "- {} distinct trigrams, {} postings, {} documents.", get(s, "index_trigrams"), get(s, "index_postings"), get(s, "index_live_documents"));
        let _ = writeln!(
            md,
            "- Share of posting data in the longest lists: top 100 trigrams {:.1}%, top 1,000 {:.1}%, top 10,000 {:.1}%.",
            share("index_top100_posting_bytes"),
            share("index_top1000_posting_bytes"),
            share("index_top10000_posting_bytes")
        );
        let _ = writeln!(md, "- These figures are taken straight after loading.");
        let _ = writeln!(md, "\n| Files in a trigram's list | Trigrams |\n|---:|---:|");
        for part in get(s, "index_lists_by_length").split_whitespace() {
            if let Some((range, n)) = part.split_once(':') {
                let _ = writeln!(md, "| {} | {} |", range, n);
            }
        }
        let work: Vec<&Vec<String>> = s.rows.iter().filter(|r| r.len() > 8 && !r[8].is_empty()).collect();
        if !work.is_empty() {
            let _ = writeln!(md, "\nIndex work per command that changes data:\n\n| Command | Index work |\n|---|---|");
            for r in work {
                let _ = writeln!(md, "| `{}` | {} |", r[0].replace('|', "\\|"), r[8]);
            }
        }
    }

    let _ = writeln!(md, "\n## Time per command, in milliseconds\n");
    let mut head = String::from("| Command | GNU | rg |");
    let mut rule = String::from("|---|---:|---:|");
    for (_, heading, _) in &runs {
        let _ = write!(head, " {} |", heading);
        rule.push_str("---:|");
    }
    let _ = writeln!(md, "{} Agree |\n{}---|", head, rule);
    let by_command: Vec<HashMap<&str, &Vec<String>>> = runs.iter().map(|(_, _, s)| s.rows.iter().map(|r| (r[0].as_str(), r)).collect()).collect();
    for row in &last.2.rows {
        let mut line = format!("| `{}` | {} | {} |", row[0].replace('|', "\\|"), ms(row[2].parse().unwrap_or(-1.0)), ms(row[3].parse().unwrap_or(-1.0)));
        let mut agree = "yes".to_string();
        for map in &by_command {
            match map.get(row[0].as_str()) {
                Some(r) => {
                    let _ = write!(line, " {} |", ms(r[4].parse().unwrap_or(-1.0)));
                    if r[7] != "yes" {
                        agree = r[7].clone();
                    }
                }
                None => line.push_str(" |"),
            }
        }
        let _ = writeln!(md, "{} {} |", line, agree);
    }

    let _ = writeln!(md, "\n## Peak memory per command\n");
    let mut head = String::from("| Command | Output |");
    let mut rule = String::from("|---|---:|");
    for (_, heading, _) in &runs {
        let _ = write!(head, " {} |", heading);
        rule.push_str("---:|");
    }
    let _ = writeln!(md, "{}\n{}", head, rule);
    for row in &last.2.rows {
        let mut line = format!("| `{}` | {} |", row[0].replace('|', "\\|"), size(row[5].parse().unwrap_or(0)));
        for map in &by_command {
            match map.get(row[0].as_str()) {
                Some(r) => {
                    let _ = write!(line, " {} |", size(r[6].parse().unwrap_or(0)));
                }
                None => line.push_str(" |"),
            }
        }
        let _ = writeln!(md, "{}", line);
    }

    if let Some(terms) = Terms::read(&PathBuf::from("bench/terms").join(format!("{}.txt", name))) {
        let _ = writeln!(md, "\n## Search terms and paths\n\n| Placeholder | Value | Measured |\n|---|---|---|");
        for (k, v) in &terms.values {
            let _ = writeln!(md, "| {} | `{}` | {} |", k, v, terms.notes.get(k).map(|s| s.as_str()).unwrap_or(""));
        }
    }
    std::fs::write(opts.out.join(format!("{}.md", name)), md).map_err(|e| e.to_string())
}

fn run() -> Result<bool, String> {
    if std::env::args().nth(1).as_deref() == Some("--helper") {
        helper();
        return Ok(true);
    }
    let mut dataset = String::from("small");
    let mut dir: Option<PathBuf> = None;
    let mut export: Option<PathBuf> = None;
    let mut data = PathBuf::from(std::env::var("QDB_BENCH_DATA").unwrap_or_else(|_| "../qdb-data/src".into()));
    let mut report_only = false;
    let mut opts = Options {
        commands: "bench/commands.txt".into(),
        reps: 5,
        out: "bench/results".into(),
        scratch: std::env::temp_dir(),
        label: "dev".into(),
        index: true,
        paired: false,
        filter: None,
        db: None,
        sync: SyncMode::Always,
        piece: Config::default().piece,
        cap_mb: None,
        cold: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--dataset" => dataset = value("--dataset")?,
            "--dir" => dir = Some(value("--dir")?.into()),
            "--data" => data = value("--data")?.into(),
            "--label" => opts.label = value("--label")?,
            "--no-index" => opts.index = false,
            "--commands" => opts.commands = value("--commands")?.into(),
            "--reps" => opts.reps = value("--reps")?.parse().map_err(|_| "--reps needs a number")?,
            "--filter" => opts.filter = Some(value("--filter")?),
            "--out" => opts.out = value("--out")?.into(),
            "--scratch" => opts.scratch = value("--scratch")?.into(),
            "--export" => export = Some(value("--export")?.into()),
            "--report" => report_only = true,
            "--paired" => opts.paired = true,
            "--db" => opts.db = Some(value("--db")?.into()),
            "--sync" => {
                opts.sync = match value("--sync")?.as_str() {
                    "always" => SyncMode::Always,
                    "batch" => SyncMode::Batch,
                    other => return Err(format!("--sync: {} is not always or batch", other)),
                }
            }
            "--piece" => opts.piece = value("--piece")?.parse().map_err(|_| "--piece needs a number")?,
            "--cold" => opts.cold = true,
            "--cap" => opts.cap_mb = Some(value("--cap")?.parse().map_err(|_| "--cap needs a number of MB")?),
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
    if report_only {
        for entry in std::fs::read_dir(&opts.out).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                report_dataset(&entry.file_name().to_string_lossy(), &opts)?;
            }
        }
        return Ok(true);
    }
    if let Some(dir) = dir {
        let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "dir".into());
        let source = dir.display().to_string();
        if !prepared(&name, &source, &opts) {
            let (db, _) = store::load_dir_with(&dir, false).map_err(|e| format!("{}: {}", dir.display(), e))?;
            prepare(&name, &source, &db, &opts)?;
            return run_again();
        }
        return bench(&name, &source, &opts);
    }
    if let Some(real) = real_datasets().into_iter().find(|d| d.name == dataset) {
        let path = data.join(&real.folder);
        let skipped_note = PathBuf::from("bench/terms").join(format!("{}.skipped", real.name));
        if !prepared(&real.name, &real.source, &opts) {
            let (db, report) = store::load_dir_with(&path, false).map_err(|e| format!("{}: {} (see bench/fetch.sh)", path.display(), e))?;
            let note = format!(
                "Skipped on import: {} symbolic links, {} files that are not valid text, {} other",
                report.skipped_symlinks, report.skipped_not_text, report.skipped_other
            );
            std::fs::create_dir_all("bench/terms").map_err(|e| e.to_string())?;
            std::fs::write(&skipped_note, note).map_err(|e| e.to_string())?;
            prepare(&real.name, &real.source, &db, &opts)?;
            return run_again();
        }
        let source = format!("{}. {}", real.source, std::fs::read_to_string(&skipped_note).unwrap_or_default().trim());
        return bench(&real.name, &source, &opts);
    }
    let spec = SPECS.iter().find(|s| s.name == dataset).ok_or(format!("unknown dataset {}", dataset))?;
    let source = "generated by qdb-bench from a fixed seed";
    if let Some(dir) = &export {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        store::dump_dir(&generate(spec), dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
        return Ok(true);
    }
    if !prepared(spec.name, source, &opts) {
        prepare(spec.name, source, &generate(spec), &opts)?;
        return run_again();
    }
    bench(spec.name, source, &opts)
}

fn main() -> std::process::ExitCode {
    let worker = std::thread::Builder::new().stack_size(64 << 20).spawn(run).expect("spawn");
    match worker.join().expect("benchmark panicked") {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(msg) => {
            eprintln!("qdb-bench: {}", msg);
            std::process::ExitCode::from(2)
        }
    }
}
