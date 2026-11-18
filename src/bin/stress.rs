//! Measurements that the command benchmark does not make: one very large
//! file, and a long run of writes.
//!
//! See bench/README.md.

use qdb::store;
use qdb::testing::{helper, timed_real};
use qdb::Db;
use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

const USAGE: &str = "\
Usage: qdb-stress bigfile [OPTIONS]
       qdb-stress writes --dataset NAME [OPTIONS]

bigfile: five commands on one generated file, on the real tools and on
the engine: append a line, search for a rare term, wc -l, tail -n 5, and
a sed -i that changes one line.

writes: a long run of small writes mixed with occasional large ones on a
loaded dataset, engine only, recording the time of every write.

  --size MB         bigfile: size of the file (default 1024)
  --dataset NAME    writes: a dataset listed in bench/datasets.txt
  --count N         writes: how many writes (default 100000)
  --seed N          writes: which sequence (default 1)
  --data DIR        where the real datasets are unpacked (default ../qdb-data/src)
  --label NAME      what to call this run in the results (default dev)
  --no-index        run the engine with the search index switched off
  --out DIR         where to write results (default bench/results)
  --scratch DIR     where the real tools work (default: the system temporary directory)";

// ------------------------------------------------------ memory accounting

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            let live = LIVE.load(Relaxed) + layout.size();
            LIVE.store(live, Relaxed);
            if live > PEAK.load(Relaxed) {
                PEAK.store(live, Relaxed);
            }
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.store(LIVE.load(Relaxed) - layout.size(), Relaxed);
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, layout, new_size);
        if !p.is_null() {
            let live = LIVE.load(Relaxed) - layout.size() + new_size;
            LIVE.store(live, Relaxed);
            if live > PEAK.load(Relaxed) {
                PEAK.store(live, Relaxed);
            }
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const SYLLABLES: &[&str] = &[
    "ba", "ne", "ti", "lo", "mu", "ra", "se", "ki", "do", "vu", "pa", "le", "ni", "to", "gu", "fa", "re", "mi", "so", "ju", "an", "er", "is", "on",
    "ul", "st", "tr", "ch", "ing", "ent", "ous", "ate",
];

fn vocabulary(rng: &mut Rng, n: usize) -> Vec<String> {
    (0..n)
        .map(|_| {
            let mut w = String::new();
            for _ in 0..2 + rng.below(3) {
                w.push_str(SYLLABLES[rng.below(SYLLABLES.len())]);
            }
            w
        })
        .collect()
}

fn sentence(rng: &mut Rng, vocab: &[String], least: usize, spread: usize) -> String {
    let words = least + rng.below(spread);
    let mut s = String::new();
    for i in 0..words {
        if i > 0 {
            s.push(' ');
        }
        // A few words are much more common than the rest, as in real text.
        let w = if rng.below(3) == 0 { &vocab[rng.below(40)] } else { &vocab[rng.below(vocab.len())] };
        s.push_str(w);
    }
    s
}

struct Options {
    out: PathBuf,
    scratch: PathBuf,
    data: PathBuf,
    label: String,
    index: bool,
    size_mb: usize,
    dataset: String,
    count: usize,
    seed: u64,
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

fn show_ms(v: f64) -> String {
    if v >= 100.0 {
        format!("{:.0}", v)
    } else if v >= 10.0 {
        format!("{:.1}", v)
    } else if v >= 1.0 {
        format!("{:.2}", v)
    } else {
        format!("{:.3}", v)
    }
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

#[derive(Default)]
struct Keep(Vec<u8>);

impl qdb::Sink for Keep {
    fn write(&mut self, data: &[u8]) -> bool {
        self.0.extend_from_slice(data);
        true
    }
}

/// Run one command on the engine: time, most heap held above what was
/// held before, exit status, output.
fn engine(db: &mut Db, command: &str) -> (Duration, usize, i32, Vec<u8>) {
    let mut out = Keep::default();
    let mut err: Vec<u8> = Vec::new();
    let base = LIVE.load(Relaxed);
    PEAK.store(base, Relaxed);
    let t = Instant::now();
    let code = qdb::execute_to(db, command.as_bytes(), &mut out, &mut err);
    let time = t.elapsed();
    let peak = PEAK.load(Relaxed).saturating_sub(base + out.0.capacity());
    out.0.extend_from_slice(&err);
    (time, peak, code, out.0)
}

// ------------------------------------------------------------- one big file

const RARE: &str = "qdbrareterm";
const UNIQUE: &str = "QDBUNIQUELINE";

/// The text of the big file: numbered lines of generated words, a rare
/// term on seven of them, and one line that is like no other.
fn big_text(bytes: usize) -> Vec<u8> {
    let mut rng = Rng(0xB16F11E);
    let vocab = vocabulary(&mut rng, 20_000);
    let mut text = Vec::with_capacity(bytes + 256);
    let mut line = 0usize;
    let rare_at: Vec<usize> = (1..=7).map(|i| bytes / 8 * i).collect();
    let mut next_rare = 0;
    let unique_at = bytes / 5 * 3;
    let mut unique_done = false;
    while text.len() < bytes {
        line += 1;
        let mut s = format!("{:09} {}", line, sentence(&mut rng, &vocab, 6, 10));
        if next_rare < rare_at.len() && text.len() >= rare_at[next_rare] {
            next_rare += 1;
            let _ = write!(s, " {} {}", RARE, next_rare);
        }
        if !unique_done && text.len() >= unique_at {
            unique_done = true;
            let _ = write!(s, " {} here", UNIQUE);
        }
        text.extend_from_slice(s.as_bytes());
        text.push(b'\n');
    }
    text
}

fn bigfile(opts: &Options) -> Result<bool, String> {
    let root = opts.scratch.join("qdb-stress-bigfile");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    // Start the helper for the real tools while this process is small.
    timed_real(&root, "true");
    eprintln!("bigfile: generating {} MB", opts.size_mb);
    let text = big_text(opts.size_mb << 20);
    let bytes = text.len();
    std::fs::write(root.join("big.txt"), &text).map_err(|e| e.to_string())?;
    let mut db = if opts.index { Db::new() } else { Db::without_index() };
    let top = db.root();
    let t = Instant::now();
    db.create_file(top, b"big.txt", text);
    let create = t.elapsed();
    eprintln!("bigfile: {} bytes, stored in the engine in {} ms", bytes, show_ms(ms(create)));

    // (command, changes data). Reads are timed five times, changes once
    // each; the three appends give a median.
    let cases: Vec<(String, bool)> = vec![
        (format!("grep -n {} big.txt", RARE), false),
        (format!("grep -c {} big.txt", RARE), false),
        ("wc -l big.txt".into(), false),
        ("tail -n 5 big.txt".into(), false),
        ("echo 'one more line at the end' >> big.txt".into(), true),
        ("echo 'and a second one' >> big.txt".into(), true),
        ("echo 'and a third' >> big.txt".into(), true),
        (format!("sed -i 's/{}/QDBCHANGEDLINE/' big.txt", UNIQUE), true),
        ("grep -n QDBCHANGEDLINE big.txt".into(), false),
        ("tail -n 4 big.txt".into(), false),
        ("wc big.txt".into(), false),
    ];
    let mut rows = String::from("command,kind,gnu_ms,engine_ms,engine_peak_bytes,agree\n");
    let mut all_agree = true;
    println!("{:<52} {:>10} {:>10} {:>12}  result", "command", "GNU ms", "engine ms", "peak heap");
    for (command, changes) in &cases {
        let reps = if *changes { 1 } else { 5 };
        let mut gnu = Vec::new();
        let mut eng = Vec::new();
        let mut agree = true;
        let mut peak = 0;
        for _ in 0..reps {
            let (gt, real) = timed_real(&root, command);
            let (et, p, code, out) = engine(&mut db, command);
            let mut real_out = real.stdout.clone();
            real_out.extend_from_slice(&real.stderr);
            agree &= code == real.code && out == real_out;
            peak = peak.max(p);
            gnu.push(gt);
            eng.push(et);
        }
        all_agree &= agree;
        let (g, e) = (ms(median(gnu)), ms(median(eng)));
        println!("{:<52} {:>10} {:>10} {:>12}  {}", command, show_ms(g), show_ms(e), peak, if agree { "same" } else { "DIFFERENT" });
        let _ = writeln!(rows, "\"{}\",{},{:.4},{:.4},{},{}", command.replace('"', "\"\""), if *changes { "write" } else { "read" }, g, e, peak, agree);
    }
    // The whole file, at the end.
    let real = std::fs::read(root.join("big.txt")).map_err(|e| e.to_string())?;
    let id = db.resolve(b"big.txt").map_err(|e| e.msg().to_string())?;
    let same = *db.content(id) == real[..];
    println!("content at the end: {}", if same { "same" } else { "DIFFERENT" });
    all_agree &= same;
    let dir = opts.out.join("bigfile");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.csv", opts.label)), rows).map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join(format!("{}.meta", opts.label)),
        format!("size_bytes={}\nindex={}\ncreate_ms={:.1}\nagree={}\n", bytes, opts.index, ms(create), all_agree),
    )
    .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&root);
    Ok(all_agree)
}

// --------------------------------------------------------- a run of writes

/// The kinds of write, and whether each counts as small.
const KINDS: &[(&str, bool)] = &[
    ("append", true),
    ("create", true),
    ("overwrite", true),
    ("sed", true),
    ("rm", true),
    ("mv", true),
    ("same", true),
    ("cp file", false),
    ("append to a dataset file", false),
    ("sed over a folder", false),
    ("cp -r", false),
    ("cat into one", false),
    ("sed on a large file", false),
    ("rm -r", false),
];

fn dataset_files(db: &Db) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack: Vec<(qdb::tree::NodeId, String)> = vec![(db.root(), String::new())];
    while let Some((id, path)) = stack.pop() {
        for (name, child) in db.children(id) {
            let name = String::from_utf8_lossy(name);
            // Only names that need no quoting.
            if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b)) || name.starts_with('-') {
                continue;
            }
            let p = if path.is_empty() { name.into_owned() } else { format!("{}/{}", path, name) };
            if db.is_dir(*child) {
                stack.push((*child, p));
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn writes(opts: &Options) -> Result<bool, String> {
    let folder = std::fs::read_to_string("bench/datasets.txt")
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut p = l.splitn(3, '|').map(|p| p.trim().to_string());
            Some((p.next()?, p.next()?))
        })
        .find(|(n, _)| *n == opts.dataset)
        .ok_or(format!("unknown dataset {}", opts.dataset))?
        .1;
    let (mut db, report) = store::load_dir_with(&opts.data.join(&folder), opts.index).map_err(|e| e.to_string())?;
    eprintln!("writes: {} loaded, {} files, {:.1} MB", opts.dataset, report.files, report.bytes as f64 / 1e6);
    let files = dataset_files(&db);
    let mut rng = Rng(opts.seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let vocab = vocabulary(&mut rng, 5_000);
    // Folders with a fair number of files, for cp -r.
    let mut dirs: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for f in &files {
        if let Some((d, _)) = f.rsplit_once('/') {
            *dirs.entry(d).or_default() += 1;
        }
    }
    let dirs: Vec<&str> = dirs.into_iter().filter(|(_, n)| (20..=400).contains(n)).map(|(d, _)| d).collect();
    if dirs.is_empty() || files.len() < 100 {
        return Err("the dataset is too small for this run".into());
    }

    for c in ["mkdir -p stress/small stress/big stress/copies", "mkdir stress/d0 stress/d1 stress/d2 stress/d3 stress/d4 stress/d5 stress/d6 stress/d7"] {
        qdb::execute(&mut db, c.as_bytes());
    }
    // The run's own files: path and how many lines were written to it.
    let mut own: Vec<String> = Vec::new();
    let mut next = 0usize;
    let mut copies: Vec<String> = Vec::new();
    let mut bigs: Vec<String> = Vec::new();
    let mut times: Vec<(u8, u64)> = Vec::with_capacity(opts.count);
    let mut samples = String::from("write,index_documents_ever,index_live_documents,index_stale_postings,index_bytes,bytes_swept,rebuilds,heap_bytes\n");
    let mut slowest: Vec<(u64, usize, String)> = Vec::new();
    let started = Instant::now();
    for n in 0..opts.count {
        let roll = rng.below(1000);
        let pick = |rng: &mut Rng, own: &Vec<String>| own[rng.below(own.len())].clone();
        let (kind, command): (usize, String) = if own.len() < 50 || (roll < 200 && own.len() < 20_000) {
            next += 1;
            let p = format!("stress/d{}/f{}.txt", rng.below(8), next);
            own.push(p.clone());
            (1, format!("echo '{}' > {}", sentence(&mut rng, &vocab, 4, 40), p))
        } else if roll < 600 {
            (0, format!("echo '{}' >> {}", sentence(&mut rng, &vocab, 4, 12), pick(&mut rng, &own)))
        } else if roll < 700 {
            (2, format!("echo '{}' > {}", sentence(&mut rng, &vocab, 4, 40), pick(&mut rng, &own)))
        } else if roll < 800 {
            let w = &vocab[rng.below(40)];
            (3, format!("sed -i 's/{}/{}/' {}", w, vocab[rng.below(vocab.len())], pick(&mut rng, &own)))
        } else if roll < 850 {
            let at = rng.below(own.len());
            (4, format!("rm {}", own.swap_remove(at)))
        } else if roll < 890 {
            let at = rng.below(own.len());
            next += 1;
            let to = format!("stress/d{}/m{}.txt", rng.below(8), next);
            let from = std::mem::replace(&mut own[at], to.clone());
            (5, format!("mv {} {}", from, to))
        } else if roll < 910 {
            // A write that changes nothing.
            (6, format!("sed -i 's/qdbnosuchword/x/' {}", pick(&mut rng, &own)))
        } else if roll < 960 {
            next += 1;
            let p = format!("stress/small/c{}", next);
            (7, format!("cp {} {}", files[rng.below(files.len())], p))
        } else if roll < 985 {
            (8, format!("echo '{}' >> {}", sentence(&mut rng, &vocab, 8, 1), files[rng.below(files.len())]))
        } else if roll < 990 {
            (9, format!("sed -i 's/{}/{}/' stress/d{}/*.txt", vocab[rng.below(40)], vocab[rng.below(vocab.len())], rng.below(8)))
        } else if roll < 993 || (roll < 996 && copies.is_empty()) {
            next += 1;
            let p = format!("stress/copies/k{}", next);
            copies.push(p.clone());
            (10, format!("cp -r {} {}", dirs[rng.below(dirs.len())], p))
        } else if roll < 995 || (roll < 998 && bigs.is_empty()) {
            let p = format!("stress/big/b{}.txt", rng.below(4));
            if !bigs.contains(&p) {
                bigs.push(p.clone());
            }
            let parts: Vec<&str> = (0..24).map(|_| files[rng.below(files.len())].as_str()).collect();
            (11, format!("cat {} > {}", parts.join(" "), p))
        } else if roll < 998 {
            (12, format!("sed -i 's/{}/{}/' {}", ["e", "th", "a"][rng.below(3)], vocab[rng.below(vocab.len())], bigs[rng.below(bigs.len())]))
        } else {
            let at = rng.below(copies.len());
            (13, format!("rm -r {}", copies.swap_remove(at)))
        };
        let t = Instant::now();
        let out = qdb::execute(&mut db, command.as_bytes());
        let nanos = t.elapsed().as_nanos() as u64;
        if out.code != 0 && kind != 9 {
            return Err(format!("write {} failed: {}\n{}", n + 1, command, String::from_utf8_lossy(&out.stderr)));
        }
        times.push((kind as u8, nanos));
        if KINDS[kind].1 && slowest.last().map_or(true, |s| nanos > s.0 || slowest.len() < 10) {
            slowest.push((nanos, n + 1, command.chars().take(100).collect()));
            slowest.sort_by(|a, b| b.0.cmp(&a.0));
            slowest.truncate(10);
        }
        if (n + 1) % 1000 == 0 {
            let (s, w) = (db.index_stats(), db.index_work());
            let _ = writeln!(
                samples,
                "{},{},{},{},{},{},{},{}",
                n + 1,
                s.as_ref().map_or(0, |s| s.documents_ever),
                s.as_ref().map_or(0, |s| s.live_documents),
                s.as_ref().map_or(0, |s| s.stale_postings),
                s.as_ref().map_or(0, |s| s.total_bytes()),
                w.map_or(0, |w| w.bytes_swept),
                w.map_or(0, |w| w.rebuilds),
                LIVE.load(Relaxed)
            );
        }
    }
    let total = started.elapsed();

    let mut md = String::new();
    let _ = writeln!(md, "# {} writes on {}, run `{}`\n", opts.count, opts.dataset, opts.label);
    let _ = writeln!(md, "Seed {}, search index {}. The whole run took {:.1} s.\n", opts.seed, if opts.index { "on" } else { "off" }, total.as_secs_f64());
    let _ = writeln!(md, "Times in milliseconds.\n");
    let _ = writeln!(md, "| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |");
    let _ = writeln!(md, "|---|---:|---:|---:|---:|---:|---:|");
    let mut line = |name: &str, mut v: Vec<u64>| {
        v.sort_unstable();
        let f = |n: u64| show_ms(n as f64 / 1e6);
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} |",
            name,
            v.len(),
            f(percentile(&v, 0.5)),
            f(percentile(&v, 0.9)),
            f(percentile(&v, 0.99)),
            f(percentile(&v, 0.999)),
            f(v.last().copied().unwrap_or(0))
        );
    };
    line("**every small write**", times.iter().filter(|t| KINDS[t.0 as usize].1).map(|t| t.1).collect());
    line("**every large write**", times.iter().filter(|t| !KINDS[t.0 as usize].1).map(|t| t.1).collect());
    for (k, (name, _)) in KINDS.iter().enumerate() {
        line(name, times.iter().filter(|t| t.0 as usize == k).map(|t| t.1).collect());
    }
    let _ = writeln!(md, "\nThe ten slowest small writes:\n");
    let _ = writeln!(md, "| ms | Write number | Command |");
    let _ = writeln!(md, "|---:|---:|---|");
    for (nanos, n, command) in &slowest {
        let _ = writeln!(md, "| {} | {} | `{}` |", show_ms(*nanos as f64 / 1e6), n, command.replace('|', "\\|"));
    }
    if let Some(w) = db.index_work() {
        let _ = writeln!(
            md,
            "\nIndex work over the run: {} documents added, {} retired, {} bytes of lists rewritten by sweeps, {} rebuilds.",
            w.documents_added, w.documents_retired, w.bytes_swept, w.rebuilds
        );
    }
    print!("{}", md);
    let dir = opts.out.join("writes");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let base = format!("{}-{}", opts.dataset, opts.label);
    std::fs::write(dir.join(format!("{}.md", base)), md).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.samples.csv", base)), samples).map_err(|e| e.to_string())?;
    // Every write: its kind (a letter, in the order of the table) and its
    // time in microseconds.
    let mut every = String::with_capacity(times.len() * 8);
    for (k, nanos) in &times {
        let _ = writeln!(every, "{},{}", (b'a' + k) as char, (nanos + 500) / 1000);
    }
    std::fs::write(dir.join(format!("{}.times.csv", base)), every).map_err(|e| e.to_string())?;
    Ok(true)
}

fn run() -> Result<bool, String> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default();
    if mode == "--helper" {
        helper();
        return Ok(true);
    }
    let mut opts = Options {
        out: "bench/results".into(),
        scratch: std::env::temp_dir(),
        data: PathBuf::from(std::env::var("QDB_BENCH_DATA").unwrap_or_else(|_| "../qdb-data/src".into())),
        label: "dev".into(),
        index: true,
        size_mb: 1024,
        dataset: String::new(),
        count: 100_000,
        seed: 1,
    };
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--size" => opts.size_mb = value("--size")?.parse().map_err(|_| "--size needs a number")?,
            "--dataset" => opts.dataset = value("--dataset")?,
            "--count" => opts.count = value("--count")?.parse().map_err(|_| "--count needs a number")?,
            "--seed" => opts.seed = value("--seed")?.parse().map_err(|_| "--seed needs a number")?,
            "--data" => opts.data = value("--data")?.into(),
            "--label" => opts.label = value("--label")?,
            "--no-index" => opts.index = false,
            "--out" => opts.out = value("--out")?.into(),
            "--scratch" => opts.scratch = value("--scratch")?.into(),
            other => return Err(format!("unknown argument {}\n{}", other, USAGE)),
        }
    }
    match mode.as_str() {
        "bigfile" => bigfile(&opts),
        "writes" => writes(&opts),
        _ => {
            println!("{}", USAGE);
            Ok(mode == "--help" || mode == "-h")
        }
    }
}

fn main() -> std::process::ExitCode {
    let worker = std::thread::Builder::new().stack_size(512 << 20).spawn(run).expect("spawn");
    match worker.join().expect("qdb-stress panicked") {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(msg) => {
            eprintln!("qdb-stress: {}", msg);
            std::process::ExitCode::from(2)
        }
    }
}

#[allow(dead_code)]
fn unused(_: &Path) {}
