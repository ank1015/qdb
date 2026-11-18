//! Measurements that the command benchmark does not make: one very large
//! file, and a long run of writes.
//!
//! See bench/README.md.

use qdb::store;
use qdb::testing::{helper, timed_real};
use qdb::{Config, Db, SyncMode};
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
  --scratch DIR     where the real tools work (default: the system temporary directory)
  --db DIR          keep the engine's data in a database under DIR (made
                    anew) instead of in memory
  --sync MODE       with --db: always or batch (default batch)
  --piece BYTES     piece size (default 65536)
  --no-upkeep       writes: do not move content to free space as the run
                    goes (to measure what that costs)
  --cap MB          limit the engine's own memory to MB: the run dies if
                    it asks for more";

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
    db: Option<PathBuf>,
    sync: SyncMode,
    piece: usize,
    cap_mb: Option<usize>,
    upkeep: bool,
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
/// term on seven of them, and one line that is like no other. It is
/// handed over in stretches of whole lines of about 4 MB, so that no one
/// ever holds all of it.
fn big_text(bytes: usize, each: &mut dyn FnMut(&[u8])) -> usize {
    let mut rng = Rng(0xB16F11E);
    let vocab = vocabulary(&mut rng, 20_000);
    let mut text = Vec::with_capacity((4 << 20) + 256);
    let mut done = 0usize;
    let mut line = 0usize;
    let rare_at: Vec<usize> = (1..=7).map(|i| bytes / 8 * i).collect();
    let mut next_rare = 0;
    let unique_at = bytes / 5 * 3;
    let mut unique_done = false;
    while done + text.len() < bytes {
        line += 1;
        let at = done + text.len();
        let mut s = format!("{:09} {}", line, sentence(&mut rng, &vocab, 6, 10));
        if next_rare < rare_at.len() && at >= rare_at[next_rare] {
            next_rare += 1;
            let _ = write!(s, " {} {}", RARE, next_rare);
        }
        if !unique_done && at >= unique_at {
            unique_done = true;
            let _ = write!(s, " {} here", UNIQUE);
        }
        text.extend_from_slice(s.as_bytes());
        text.push(b'\n');
        if text.len() >= 4 << 20 {
            each(&text);
            done += text.len();
            text.clear();
        }
    }
    each(&text);
    done + text.len()
}

/// Limit this process's own memory (heap, stacks, everything that is not
/// a file): an allocation beyond it fails and the run dies.
fn cap_memory(mb: usize) -> Result<(), String> {
    let lim = libc::rlimit { rlim_cur: (mb as u64) << 20, rlim_max: (mb as u64) << 20 };
    if unsafe { libc::setrlimit(libc::RLIMIT_DATA, &lim) } != 0 {
        return Err(format!("setting the memory limit: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

/// A line of /proc/self/status, in kB.
fn status_kb(name: &str) -> u64 {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    text.lines().find(|l| l.starts_with(name)).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse().ok()).unwrap_or(0)
}

fn open_engine(opts: &Options, name: &str) -> Result<Db, String> {
    let cfg = Config { index: opts.index, piece: opts.piece, sync: opts.sync, ..Config::default() };
    match &opts.db {
        None => Ok(Db::in_memory(cfg)),
        Some(dir) => {
            let dir = dir.join(name);
            let _ = std::fs::remove_dir_all(&dir);
            Ok(Db::open_dir(&dir, cfg)?.0)
        }
    }
}

fn bigfile(opts: &Options) -> Result<bool, String> {
    let root = opts.scratch.join("qdb-stress-bigfile");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    // Start the helper for the real tools before the limit is set: the
    // limit is for the engine.
    timed_real(&root, "true");
    if let Some(mb) = opts.cap_mb {
        cap_memory(mb)?;
    }
    eprintln!("bigfile: generating {} MB", opts.size_mb);
    let mut db = open_engine(opts, "bigfile")?;
    db.set_sync(SyncMode::Batch);
    let top = db.root();
    let id = db.create_file(top, b"big.txt", b"");
    let mut create = Duration::ZERO;
    let mut overall = 0usize;
    let bytes = {
        use std::io::Write as _;
        let mut real = std::io::BufWriter::new(std::fs::File::create(root.join("big.txt")).map_err(|e| e.to_string())?);
        let n = big_text(opts.size_mb << 20, &mut |part| {
            real.write_all(part).expect("writing the file for the real tools");
            let t = Instant::now();
            db.append(id, part);
            db.commit();
            create += t.elapsed();
        });
        real.flush().map_err(|e| e.to_string())?;
        n
    };
    let t = Instant::now();
    db.sync();
    if opts.db.is_some() {
        // Settled, closed and opened again: what a user of the file meets.
        db.compact()?;
        db.close()?;
        let cfg = Config { index: opts.index, piece: opts.piece, sync: opts.sync, ..Config::default() };
        db = Db::open_dir(&opts.db.as_ref().unwrap().join("bigfile"), cfg)?.0;
    }
    db.set_sync(opts.sync);
    let settle = t.elapsed();
    overall = overall.max(PEAK.load(Relaxed));
    if let Some(e) = db.failure() {
        return Err(e.to_string());
    }
    eprintln!("bigfile: {} bytes, stored in the engine in {} ms, settled in {} ms", bytes, show_ms(ms(create)), show_ms(ms(settle)));

    // (command, changes data). Reads are timed five times, changes once
    // each; the three appends give a median.
    let cases: Vec<(String, bool)> = vec![
        (format!("grep -n {} big.txt", RARE), false),
        (format!("grep -c {} big.txt", RARE), false),
        ("wc -l big.txt".into(), false),
        ("tail -n 5 big.txt".into(), false),
        ("head -n 3 big.txt".into(), false),
        ("echo 'one more line at the end' >> big.txt".into(), true),
        ("echo 'and a second one' >> big.txt".into(), true),
        ("echo 'and a third' >> big.txt".into(), true),
        (format!("sed -i 's/{}/QDBCHANGEDLINE/' big.txt", UNIQUE), true),
        ("grep -n QDBCHANGEDLINE big.txt".into(), false),
        ("tail -n 4 big.txt".into(), false),
        ("wc big.txt".into(), false),
        ("cp big.txt copy.txt".into(), true),
        ("cat big.txt > cat.txt".into(), true),
        ("grep -c e cat.txt".into(), false),
        ("tail -n 1 copy.txt".into(), false),
        ("rm copy.txt cat.txt".into(), true),
    ];
    let mut rows = String::from("command,kind,gnu_ms,engine_ms,engine_peak_bytes,disk_bytes_written,syncs,agree\n");
    let mut all_agree = true;
    println!("{:<50} {:>9} {:>10} {:>11} {:>13} {:>5}  result", "command", "GNU ms", "engine ms", "peak heap", "disk written", "syncs");
    for (command, changes) in &cases {
        let reps = if *changes { 1 } else { 5 };
        let mut gnu = Vec::new();
        let mut eng = Vec::new();
        let mut agree = true;
        let mut peak = 0;
        let before = db.io_stats();
        for _ in 0..reps {
            let (gt, real) = timed_real(&root, command);
            let (et, p, code, out) = engine(&mut db, command);
            overall = overall.max(PEAK.load(Relaxed));
            let mut real_out = real.stdout.clone();
            real_out.extend_from_slice(&real.stderr);
            agree &= code == real.code && out == real_out;
            peak = peak.max(p);
            gnu.push(gt);
            eng.push(et);
        }
        let after = db.io_stats();
        let (written, syncs) = (after.bytes() - before.bytes(), after.syncs - before.syncs);
        all_agree &= agree;
        let (g, e) = (ms(median(gnu)), ms(median(eng)));
        println!("{:<50} {:>9} {:>10} {:>11} {:>13} {:>5}  {}", command, show_ms(g), show_ms(e), peak, written, syncs, if agree { "same" } else { "DIFFERENT" });
        let _ = writeln!(
            rows,
            "\"{}\",{},{:.4},{:.4},{},{},{},{}",
            command.replace('"', "\"\""),
            if *changes { "write" } else { "read" },
            g,
            e,
            peak,
            written,
            syncs,
            agree
        );
    }
    // The whole file, at the end, piece by piece.
    let same = {
        use std::io::Read as _;
        let mut real = std::io::BufReader::new(std::fs::File::open(root.join("big.txt")).map_err(|e| e.to_string())?);
        let id = db.resolve(b"big.txt").map_err(|e| e.msg().to_string())?;
        let mut same = true;
        let mut buf = Vec::new();
        for p in db.pieces(id) {
            let text = db.text_of(p);
            buf.resize(text.len(), 0);
            same &= real.read_exact(&mut buf).is_ok() && buf[..] == text[..];
        }
        same && real.read(&mut [0u8; 1]).map_err(|e| e.to_string())? == 0
    };
    println!("content at the end: {}", if same { "same" } else { "DIFFERENT" });
    all_agree &= same;
    let own_kb = status_kb("RssAnon:");
    let data_kb = status_kb("VmData:");
    println!(
        "most heap held at any moment: {:.1} MB; memory of its own at the end: {:.1} MB resident, {:.1} MB reserved (the limit counts this){}",
        overall as f64 / 1e6,
        own_kb as f64 / 1e3,
        data_kb as f64 / 1e3,
        opts.cap_mb.map(|mb| format!("; limit {} MB", mb)).unwrap_or_default()
    );
    if let Some(e) = db.failure() {
        return Err(e.to_string());
    }
    let disk = db.disk_stats();
    db.close()?;
    let dir = opts.out.join("bigfile");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.csv", opts.label)), rows).map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join(format!("{}.meta", opts.label)),
        format!(
            "size_bytes={}\nindex={}\nstore={}\nsync={}\npiece={}\ncap_mb={}\ncreate_ms={:.1}\nsettle_ms={:.1}\npeak_heap_bytes={}\nown_resident_kb={}\nown_reserved_kb={}\ndisk_total={}\nagree={}\n",
            bytes,
            opts.index,
            if opts.db.is_some() { "disk" } else { "memory" },
            if opts.sync == SyncMode::Always { "always" } else { "batch" },
            opts.piece,
            opts.cap_mb.unwrap_or(0),
            ms(create),
            ms(settle),
            overall,
            own_kb,
            data_kb,
            disk.total(),
            all_agree
        ),
    )
    .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&root);
    if let Some(dir) = &opts.db {
        let _ = std::fs::remove_dir_all(dir.join("bigfile"));
    }
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
    let db_dir = opts.db.as_ref().map(|d| d.join(format!("writes-{}", opts.dataset)));
    let (mut db, report) = match &db_dir {
        None => store::load_dir_with(&opts.data.join(&folder), opts.index).map_err(|e| e.to_string())?,
        Some(dir) => {
            // Imported, settled, closed and opened again.
            let cfg = Config { index: opts.index, piece: opts.piece, sync: SyncMode::Batch, upkeep: opts.upkeep, ..Config::default() };
            let _ = std::fs::remove_dir_all(dir);
            let (mut db, _) = Db::open_dir(dir, cfg.clone())?;
            let report = store::load_into_db(&mut db, &opts.data.join(&folder)).map_err(|e| e.to_string())?;
            db.compact()?;
            db.close()?;
            (Db::open_dir(dir, Config { sync: opts.sync, ..cfg })?.0, report)
        }
    };
    if let Some(mb) = opts.cap_mb {
        cap_memory(mb)?;
    }
    // What the directory holds, in bytes, as a listing of it would say.
    let on_disk = |dir: &Path| -> u64 { std::fs::read_dir(dir).map(|d| d.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0) };
    let io_start = db.io_stats();
    let raw_start = db.content_bytes() as u64;
    let disk_start = db_dir.as_ref().map_or(0, |d| on_disk(d));
    // The moment the directory was largest against the text it held.
    let mut worst = (0.0f64, 0usize, 0u64, 0u64);
    let mut disk_samples = String::from("write,disk_bytes,raw_text_bytes,written_bytes,moved_bytes,syncs\n");
    // How many command lines took 0, 1, 2, 3, 4 or more syncs.
    let mut syncs_per_line = [0u64; 5];
    let mut heap_peak = 0usize;
    PEAK.store(LIVE.load(Relaxed), Relaxed);
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
        let syncs_before = db.io_stats().syncs;
        let t = Instant::now();
        let out = qdb::execute(&mut db, command.as_bytes());
        let nanos = t.elapsed().as_nanos() as u64;
        syncs_per_line[((db.io_stats().syncs - syncs_before) as usize).min(4)] += 1;
        if let Some(dir) = db_dir.as_ref().filter(|_| (n + 1) % 200 == 0) {
            let (disk, raw, io) = (on_disk(dir), db.content_bytes() as u64, db.io_stats());
            let ratio = disk as f64 / raw.max(1) as f64;
            if ratio > worst.0 {
                worst = (ratio, n + 1, disk, raw);
            }
            if (n + 1) % 1000 == 0 {
                let _ = writeln!(disk_samples, "{},{},{},{},{},{}", n + 1, disk, raw, io.bytes() - io_start.bytes(), io.moved_bytes - io_start.moved_bytes, io.syncs - io_start.syncs);
            }
            heap_peak = heap_peak.max(PEAK.load(Relaxed));
        }
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
    if let Some(dir) = &db_dir {
        let io = db.io_stats();
        let d = |f: fn(&qdb::tree::IoStats) -> u64| f(&io) - f(&io_start);
        let user = d(|i| i.content_bytes) - d(|i| i.moved_bytes) + d(|i| i.log_bytes);
        let raw_end = db.content_bytes() as u64;
        let _ = writeln!(md, "\nOn disk, sync {}{}.\n", if opts.sync == SyncMode::Always { "always" } else { "batch" }, if opts.upkeep { "" } else { ", upkeep off" });
        let _ = writeln!(md, "| Bytes written to disk over the run | |");
        let _ = writeln!(md, "|---|---:|");
        let _ = writeln!(md, "| content the commands wrote | {} |", d(|i| i.content_bytes) - d(|i| i.moved_bytes));
        let _ = writeln!(md, "| log of the command lines | {} |", d(|i| i.log_bytes));
        let _ = writeln!(md, "| content moved to free space | {} |", d(|i| i.moved_bytes));
        let _ = writeln!(md, "| index files | {} |", d(|i| i.index_bytes));
        let _ = writeln!(md, "| saves of the tree ({}) | {} |", d(|i| i.checkpoints), d(|i| i.tree_bytes));
        let _ = writeln!(md, "| **all** | {} |", d(|i| i.bytes()));
        let _ = writeln!(md, "| **write amplification**: all, over content and log the commands caused | {:.2} |", d(|i| i.bytes()) as f64 / user.max(1) as f64);
        let _ = writeln!(md, "\nSyncs: {} over {} command lines that changed something.\n", d(|i| i.syncs), d(|i| i.commits));
        let _ = writeln!(md, "| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |");
        let _ = writeln!(md, "|---|---:|---:|---:|---:|---:|");
        let _ = writeln!(md, "| command lines | {} | {} | {} | {} | {} |", syncs_per_line[0], syncs_per_line[1], syncs_per_line[2], syncs_per_line[3], syncs_per_line[4]);
        let disk_end = on_disk(dir);
        let _ = writeln!(md, "\n| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |");
        let _ = writeln!(md, "|---|---:|---:|---:|");
        let _ = writeln!(md, "| at the start (settled) | {} | {} | {:.3} |", disk_start, raw_start, disk_start as f64 / raw_start.max(1) as f64);
        let _ = writeln!(md, "| at its worst (after write {}; looked at every 200 writes) | {} | {} | {:.3} |", worst.1, worst.2, worst.3, worst.0);
        let _ = writeln!(md, "| at the end | {} | {} | {:.3} |", disk_end, raw_end, disk_end as f64 / raw_end.max(1) as f64);
        let s = db.disk_stats();
        let _ = writeln!(md, "\nAt the end, by what it is: text {}, index lists {}, index tables {}, tree {}, log {}, record and file headers {}, space not yet reclaimed {}.", s.text, s.lists, s.tables, s.tree, s.log, s.framing, s.unreclaimed);
        let _ = writeln!(md, "\nMost heap the engine held during the run: {:.1} MB{}.", heap_peak.max(PEAK.load(Relaxed)) as f64 / 1e6, opts.cap_mb.map(|mb| format!(" (limit {} MB)", mb)).unwrap_or_default());
        let checked = match db.check() {
            Ok(what) => format!("ok: {}", what),
            Err(what) => format!("NOT OK: {}", what),
        };
        let _ = writeln!(md, "\nCheck of the database after the run: {}", checked);
        let t = Instant::now();
        db.compact()?;
        let s = db.disk_stats();
        let _ = writeln!(md, "\nSettled afterwards (`--compact`, {:.1} s): {} bytes, {:.3} of the raw text.", t.elapsed().as_secs_f64(), s.total(), s.total() as f64 / s.raw_text.max(1) as f64);
        std::fs::create_dir_all(opts.out.join("writes")).map_err(|e| e.to_string())?;
        std::fs::write(opts.out.join("writes").join(format!("{}-{}.disk.csv", opts.dataset, opts.label)), &disk_samples).map_err(|e| e.to_string())?;
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
        db: None,
        sync: SyncMode::Batch,
        piece: 64 * 1024,
        cap_mb: None,
        upkeep: true,
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
            "--db" => opts.db = Some(value("--db")?.into()),
            "--sync" => {
                opts.sync = match value("--sync")?.as_str() {
                    "always" => SyncMode::Always,
                    "batch" => SyncMode::Batch,
                    other => return Err(format!("--sync: {} is not `always` or `batch`", other)),
                }
            }
            "--piece" => opts.piece = value("--piece")?.parse().map_err(|_| "--piece needs a number")?,
            "--no-upkeep" => opts.upkeep = false,
            "--cap" => opts.cap_mb = Some(value("--cap")?.parse().map_err(|_| "--cap needs a number of MB")?),
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
    let worker = std::thread::Builder::new().stack_size(64 << 20).spawn(run).expect("spawn");
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
