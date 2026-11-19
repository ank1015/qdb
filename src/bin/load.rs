//! qdb-load: load on a qdb-server from many connections, for the stage 6
//! targets and the capacity sheet. See bench/README.md.

use qdb::net::http::Client;
use qdb::net::json::Json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const USAGE: &str = "\
Usage: qdb-load [--server ADDR | --spawn DATA] --admin-key KEY WORKLOAD [OPTIONS]
       qdb-load import --data DATA --admin-key KEY --tree DIR

--server talks to a running server; --spawn starts the qdb-server beside
this program on the data directory DATA, with default settings and any
flags in the environment variable QDB_SERVER_ARGS, and stops it at the
end.

Workloads (each prints one line of results):
  true        `true` from --clients connections, for --seconds
  writers     --clients writers on one filesystem, each appending a line
              to a file of its own, for --seconds
  units FILE  the command lines of FILE (one a line, as qdb-stress units
              prints them) on the filesystem --fs of the account --key,
              from --clients connections, --count lines in all
  mixed       --filesystems small filesystems, one client each, a
              generated mix of 95 reads to 5 writes, for --seconds
  interfere   small reads on a filesystem of their own, alone and then
              while another connection runs --search on --fs of --key,
              for --seconds each
  order       the units-at-once checker: --clients connections send
              generated reads and appends to one filesystem, --count
              units a run, --runs runs (each on a new filesystem, with
              --seed); then an order of the units, one at a time and
              consistent with when each was answered, must explain every
              output. Some units are given a deadline so short that they
              are stopped; they must leave nothing

import makes an account and a filesystem in DATA (the server must not
be running), loads DIR into it, and prints the account, key and
filesystem as JSON.

  --clients N --seconds N --count N --filesystems N --fs ID --key KEY
  --search COMMAND (default: grep -rn define .)";

fn die(msg: &str) -> ! {
    eprintln!("qdb-load: {}", msg);
    std::process::exit(2);
}

struct Opts {
    server: Option<String>,
    spawn: Option<String>,
    admin: String,
    clients: usize,
    seconds: f64,
    count: usize,
    filesystems: usize,
    fs: Option<String>,
    key: Option<String>,
    search: String,
    runs: usize,
    seed: u64,
    data: Option<String>,
    tree: Option<String>,
    rest: Vec<String>,
}

fn parse() -> Opts {
    let mut o = Opts {
        server: None,
        spawn: None,
        admin: std::env::var("QDB_ADMIN_KEY").unwrap_or_default(),
        clients: 1,
        seconds: 10.0,
        count: 10_000,
        filesystems: 64,
        fs: None,
        key: None,
        search: "grep -rn define .".into(),
        runs: 1,
        seed: 1,
        data: None,
        tree: None,
        rest: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = |n: &str| args.next().unwrap_or_else(|| die(&format!("{} needs a value", n)));
        match a.as_str() {
            "--server" => o.server = Some(v(&a)),
            "--spawn" => o.spawn = Some(v(&a)),
            "--admin-key" => o.admin = v(&a),
            "--clients" => o.clients = v(&a).parse().unwrap_or_else(|_| die("--clients needs a number")),
            "--seconds" => o.seconds = v(&a).parse().unwrap_or_else(|_| die("--seconds needs a number")),
            "--count" => o.count = v(&a).parse().unwrap_or_else(|_| die("--count needs a number")),
            "--filesystems" => o.filesystems = v(&a).parse().unwrap_or_else(|_| die("--filesystems needs a number")),
            "--fs" => o.fs = Some(v(&a)),
            "--key" => o.key = Some(v(&a)),
            "--search" => o.search = v(&a),
            "--runs" => o.runs = v(&a).parse().unwrap_or_else(|_| die("--runs needs a number")),
            "--seed" => o.seed = v(&a).parse().unwrap_or_else(|_| die("--seed needs a number")),
            "--data" => o.data = Some(v(&a)),
            "--tree" => o.tree = Some(v(&a)),
            "-h" | "--help" => {
                println!("{}", USAGE);
                std::process::exit(0);
            }
            _ if a.starts_with("--") => die(&format!("unknown option {}\n{}", a, USAGE)),
            _ => o.rest.push(a),
        }
    }
    o
}

/// A server started by this program.
struct Spawned(std::process::Child);

impl Drop for Spawned {
    fn drop(&mut self) {
        unsafe { libc::kill(self.0.id() as i32, libc::SIGTERM) };
        let _ = self.0.wait();
    }
}

fn spawn(data: &str, admin: &str) -> (Spawned, String) {
    use std::io::BufRead;
    let exe = std::env::current_exe().unwrap().with_file_name("qdb-server");
    let mut child = std::process::Command::new(exe)
        .args(["--data", data, "--listen", "127.0.0.1:0"])
        .args(std::env::var("QDB_SERVER_ARGS").unwrap_or_default().split_whitespace())
        .env("QDB_ADMIN_KEY", admin)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| die(&format!("qdb-server: {}", e)));
    let mut line = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    (Spawned(child), line.trim().to_string())
}

fn call(c: &mut Client, method: &str, path: &str, key: &str, body: &str) -> Json {
    let r = c.request(method, path, Some(key), body.as_bytes()).unwrap_or_else(|e| die(&format!("{} {}: {}", method, path, e)));
    let j = Json::parse(&r.body).unwrap_or_else(|e| die(&format!("{} {}: {}", method, path, e)));
    if r.status / 100 != 2 {
        die(&format!("{} {}: HTTP {}: {}", method, path, r.status, j.to_string()));
    }
    j
}

fn account(c: &mut Client, admin: &str) -> String {
    let j = call(c, "POST", "/v1/accounts", admin, r#"{"limits":{"units_at_once":100000},"quotas":{"filesystems":1000000}}"#);
    j.get("key").and_then(Json::as_str).unwrap().to_string()
}

fn new_fs(c: &mut Client, key: &str) -> String {
    call(c, "POST", "/v1/filesystems", key, "").get("filesystem").and_then(Json::as_str).unwrap().to_string()
}

/// Run a unit; its time in microseconds, and the answer.
fn unit(c: &mut Client, key: &str, fs: &str, line: &str) -> (u64, Json) {
    let body = Json::obj().with("command", line).to_string();
    let t = Instant::now();
    let r = c.request("POST", &format!("/v1/filesystems/{}/run", fs), Some(key), body.as_bytes()).unwrap_or_else(|e| die(&format!("run: {}", e)));
    let us = t.elapsed().as_micros() as u64;
    if r.status != 200 {
        die(&format!("run {:?}: HTTP {}: {}", line, r.status, String::from_utf8_lossy(&r.body)));
    }
    (us, Json::parse(&r.body).unwrap())
}

#[derive(Default)]
struct Times {
    us: Vec<u64>,
    syncs: u64,
}

fn pct(v: &mut [u64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_unstable();
    v[((v.len() as f64 - 1.0) * p).round() as usize] as f64 / 1000.0
}

fn report(name: &str, clients: usize, took: Duration, t: &mut Times) {
    let n = t.us.len();
    println!(
        "{}: {} clients, {} units in {:.2} s: {:.0} units a second; latency median {:.3} ms, 99th percentile {:.3} ms, most {:.3} ms; {} syncs",
        name,
        clients,
        n,
        took.as_secs_f64(),
        n as f64 / took.as_secs_f64(),
        pct(&mut t.us, 0.5),
        pct(&mut t.us, 0.99),
        pct(&mut t.us, 1.0),
        t.syncs
    );
}

/// Run `clients` threads for a time or a count; each calls `step` with
/// its number and a step counter until told to stop.
fn drive(clients: usize, seconds: f64, count: Option<usize>, addr: &str, step: impl Fn(usize, usize, &mut Client) -> (u64, Json) + Sync) -> (Duration, Times) {
    let stop = AtomicBool::new(false);
    let issued = AtomicU64::new(0);
    let all = Mutex::new(Times::default());
    let t0 = Instant::now();
    std::thread::scope(|s| {
        for i in 0..clients {
            let (stop, issued, all, step) = (&stop, &issued, &all, &step);
            s.spawn(move || {
                let mut c = Client::new(addr);
                let mut mine = Times::default();
                let mut k = 0;
                loop {
                    if stop.load(Relaxed) {
                        break;
                    }
                    if let Some(n) = count {
                        if issued.fetch_add(1, Relaxed) as usize >= n {
                            break;
                        }
                    }
                    let (us, j) = step(i, k, &mut c);
                    mine.us.push(us);
                    mine.syncs += j.get("cost").and_then(|c| c.get("syncs")).and_then(Json::as_u64).unwrap_or(0);
                    k += 1;
                }
                let mut a = all.lock().unwrap();
                a.us.extend(mine.us);
                a.syncs += mine.syncs;
            });
        }
        if count.is_none() {
            std::thread::sleep(Duration::from_secs_f64(seconds));
            stop.store(true, Relaxed);
        }
    });
    let took = t0.elapsed();
    (took, all.into_inner().unwrap())
}

fn import(o: &Opts) {
    let data = o.data.as_deref().unwrap_or_else(|| die("import needs --data"));
    let tree = o.tree.as_deref().unwrap_or_else(|| die("import needs --tree"));
    let svc = qdb::service::Service::open(qdb::service::ServiceConfig::new(std::path::Path::new(data), &o.admin)).unwrap_or_else(|e| die(&e));
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap().to_string();
    let s2 = svc.clone();
    std::thread::spawn(move || qdb::service::serve(s2, l));
    let mut c = Client::new(&addr);
    let key = account(&mut c, &o.admin);
    let fs = new_fs(&mut c, &key);
    let t = Instant::now();
    let rep = svc.host().with_db_mut(&fs, |db| qdb::store::load_into_db(db, std::path::Path::new(tree))).unwrap_or_else(|e| die(&e.to_string())).unwrap_or_else(|e| die(&e.to_string()));
    eprintln!("qdb-load: loaded {} files in {:.1} s", rep.files, t.elapsed().as_secs_f64());
    svc.shutdown();
    println!("{}", Json::obj().with("key", key).with("filesystem", fs).to_string());
}

fn main() {
    let o = parse();
    if o.rest.first().map(String::as_str) == Some("import") {
        return import(&o);
    }
    if o.admin.len() < 16 {
        die("an admin key of at least 16 characters is needed: --admin-key or QDB_ADMIN_KEY");
    }
    let (_child, addr) = match (&o.server, &o.spawn) {
        (Some(a), None) => (None, a.clone()),
        (None, Some(d)) => {
            let (c, a) = spawn(d, &o.admin);
            (Some(c), a)
        }
        _ => die(&format!("give --server or --spawn\n{}", USAGE)),
    };
    let mut c = Client::new(&addr);
    let workload = o.rest.first().cloned().unwrap_or_else(|| die(USAGE));
    match workload.as_str() {
        "true" => {
            let key = account(&mut c, &o.admin);
            let fs = new_fs(&mut c, &key);
            // Warm: the filesystem open, the connections made.
            for _ in 0..200 {
                unit(&mut c, &key, &fs, "true");
            }
            let (took, mut t) = drive(o.clients, o.seconds, None, &addr, |_, _, c| unit(c, &key, &fs, "true"));
            report("true", o.clients, took, &mut t);
        }
        "writers" => {
            let key = account(&mut c, &o.admin);
            let fs = new_fs(&mut c, &key);
            let (took, mut t) = drive(o.clients, o.seconds, None, &addr, |i, k, c| unit(c, &key, &fs, &format!("echo {} >> w{}.txt", k, i)));
            report("writers on one filesystem", o.clients, took, &mut t);
            let j = unit(&mut c, &key, &fs, "cat w*.txt | wc -l").1;
            println!("    lines written: {}", j.get("stdout").and_then(Json::as_str).unwrap_or("").trim());
        }
        "units" => {
            let file = o.rest.get(1).unwrap_or_else(|| die("units needs a FILE"));
            let key = o.key.clone().unwrap_or_else(|| die("units needs --key"));
            let fs = o.fs.clone().unwrap_or_else(|| die("units needs --fs"));
            let text = std::fs::read_to_string(file).unwrap_or_else(|e| die(&format!("{}: {}", file, e)));
            let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
            let next = AtomicU64::new(0);
            let (took, mut t) = drive(o.clients, o.seconds, Some(o.count), &addr, |_, _, c| {
                let i = next.fetch_add(1, Relaxed) as usize % lines.len();
                unit(c, &key, &fs, lines[i])
            });
            report(&format!("units of {}", file), o.clients, took, &mut t);
        }
        "mixed" => {
            let key = account(&mut c, &o.admin);
            let fss: Vec<String> = (0..o.filesystems).map(|_| new_fs(&mut c, &key)).collect();
            for f in &fss {
                unit(&mut c, &key, f, "mkdir -p src docs; for i in 1 2 3 4 5 6 7 8 9 10; do echo \"line $i of a small file with some words\" > src/f$i.txt; done; seq 1 200 > docs/n.txt");
            }
            const READS: &[&str] = &["cat src/f3.txt", "grep -rn words src", "wc -l docs/n.txt", "ls -la src", "head -n 5 docs/n.txt", "find . -name '*.txt'", "grep -c 1 docs/n.txt", "sort -r docs/n.txt | head -n 3"];
            let (took, mut t) = drive(fss.len(), o.seconds, None, &addr, |i, k, c| {
                let line = if k % 20 == 19 { format!("echo {} >> src/f{}.txt", k, k % 10 + 1) } else { READS[k % READS.len()].to_string() };
                unit(c, &key, &fss[i], &line)
            });
            report(&format!("mixed, {} filesystems, one client each", fss.len()), fss.len(), took, &mut t);
        }
        "interfere" => {
            let key = o.key.clone().unwrap_or_else(|| die("interfere needs --key and --fs"));
            let fs = o.fs.clone().unwrap_or_else(|| die("interfere needs --fs"));
            let mine = account(&mut c, &o.admin);
            let small = new_fs(&mut c, &mine);
            unit(&mut c, &mine, &small, "echo hello > a.txt; seq 100 > n.txt");
            let small_reads = |c: &mut Client, k: usize| unit(c, &mine, &small, if k % 2 == 0 { "cat a.txt" } else { "grep -c 1 n.txt" });
            let (took, mut alone) = drive(1, o.seconds, None, &addr, |_, k, c| small_reads(c, k));
            report("small reads alone", 1, took, &mut alone);
            let busy = AtomicBool::new(true);
            let searches = AtomicU64::new(0);
            let (took, mut with) = std::thread::scope(|s| {
                s.spawn(|| {
                    let mut c = Client::new(&addr);
                    while busy.load(Relaxed) {
                        unit(&mut c, &key, &fs, &o.search);
                        searches.fetch_add(1, Relaxed);
                    }
                });
                std::thread::sleep(Duration::from_millis(200));
                let r = drive(1, o.seconds, None, &addr, |_, k, c| small_reads(c, k));
                busy.store(false, Relaxed);
                r
            });
            report(&format!("small reads while another filesystem runs {:?} ({} runs)", o.search, searches.load(Relaxed)), 1, took, &mut with);
        }
        "order" => {
            let key = account(&mut c, &o.admin);
            let mut total = 0usize;
            let mut stopped = 0usize;
            let t0 = Instant::now();
            for run in 0..o.runs {
                let fs = new_fs(&mut c, &key);
                match order_run(&addr, &key, &fs, o.clients, o.count, o.seed.wrapping_add(run as u64)) {
                    Ok((n, s)) => {
                        total += n;
                        stopped += s;
                    }
                    Err(e) => {
                        println!("FAIL  order checker, run {} (seed {}): {}", run + 1, o.seed.wrapping_add(run as u64), e);
                        std::process::exit(1);
                    }
                }
                call(&mut c, "DELETE", &format!("/v1/filesystems/{}", fs), &key, "");
            }
            println!(
                "order checker: {} runs of {} units from {} connections at once on one filesystem, seeds {} to {}: {} units, {} of them stopped at a deadline; every output explained by one order of the units consistent with when each was answered ({:.0} s)",
                o.runs,
                o.count,
                o.clients,
                o.seed,
                o.seed + o.runs as u64 - 1,
                total,
                stopped,
                t0.elapsed().as_secs_f64()
            );
        }
        _ => die(&format!("no workload {}\n{}", workload, USAGE)),
    }
    let _ = Arc::new(());
}

// ------------------------------------------------------- the order checker

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
}

const FILES: usize = 6;

/// One unit as sent and answered.
struct Done {
    id: usize,
    line: String,
    /// The files it appends its token to, and the files it reads.
    writes: Vec<usize>,
    reads: Vec<usize>,
    start: Instant,
    end: Instant,
    out: String,
    status: i64,
    stopped: Option<String>,
}

fn make_unit(rng: &mut Rng, id: usize) -> (String, Vec<usize>, Vec<usize>, bool) {
    let pick = |rng: &mut Rng, n: usize| {
        let mut v: Vec<usize> = Vec::new();
        while v.len() < n {
            let f = rng.below(FILES);
            if !v.contains(&f) {
                v.push(f);
            }
        }
        v
    };
    let read = |files: &[usize]| files.iter().map(|f| format!("echo '== {f}'; cat f{f}.txt 2>/dev/null; ")).collect::<String>();
    let write = |files: &[usize]| files.iter().map(|f| format!("echo u{id} >> f{f}.txt; ")).collect::<String>();
    let r = rng.below(100);
    // Some units run long and are given a deadline they cannot meet.
    let doomed = rng.below(50) == 0;
    let (line, writes, reads) = if r < 55 {
        let k = 1 + rng.below(3);
        let f = pick(rng, k);
        (read(&f), Vec::new(), f)
    } else if r < 85 {
        let k = 1 + rng.below(2);
        let f = pick(rng, k);
        (write(&f), f, Vec::new())
    } else {
        // Reads, then writes, in one unit: a writer.
        let k = 1 + rng.below(2);
        let rf = pick(rng, k);
        let wf = pick(rng, 1);
        (format!("{}{}", read(&rf), write(&wf)), wf, rf)
    };
    if doomed {
        return (format!("{}while true; do :; done", line), writes, reads, true);
    }
    (line, writes, reads, false)
}

/// What a unit's output says each file it read held: its tokens.
fn parse_reads(out: &str) -> Result<Vec<(usize, Vec<usize>)>, String> {
    let mut got: Vec<(usize, Vec<usize>)> = Vec::new();
    for l in out.lines() {
        if let Some(f) = l.strip_prefix("== ") {
            got.push((f.parse().map_err(|_| format!("bad line {:?}", l))?, Vec::new()));
        } else if let Some(t) = l.strip_prefix('u') {
            let last = got.last_mut().ok_or(format!("a token before any file: {:?}", l))?;
            last.1.push(t.parse().map_err(|_| format!("bad token {:?}", l))?);
        } else {
            return Err(format!("unexpected output line {:?}", l));
        }
    }
    Ok(got)
}

fn order_run(addr: &str, key: &str, fs: &str, clients: usize, count: usize, seed: u64) -> Result<(usize, usize), String> {
    let next = AtomicU64::new(0);
    let all: Mutex<Vec<Done>> = Mutex::new(Vec::with_capacity(count));
    std::thread::scope(|s| {
        for c in 0..clients {
            let (next, all) = (&next, &all);
            s.spawn(move || {
                let mut client = Client::new(addr);
                let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(c as u64 * 7919) | 1);
                let mut mine = Vec::new();
                loop {
                    let id = next.fetch_add(1, Relaxed) as usize;
                    if id >= count {
                        break;
                    }
                    let (line, writes, reads, doomed) = make_unit(&mut rng, id);
                    let mut body = Json::obj().with("command", line.as_str());
                    if doomed {
                        body.set("limits", Json::obj().with("time_ms", 5));
                    }
                    let start = Instant::now();
                    let r = client.request("POST", &format!("/v1/filesystems/{}/run", fs), Some(key), body.to_string().as_bytes()).unwrap_or_else(|e| die(&format!("run: {}", e)));
                    let end = Instant::now();
                    let j = Json::parse(&r.body).unwrap_or_else(|e| die(&format!("run: {}", e)));
                    if r.status != 200 {
                        die(&format!("run {:?}: HTTP {}: {}", line, r.status, j.to_string()));
                    }
                    mine.push(Done {
                        id,
                        line,
                        writes,
                        reads,
                        start,
                        end,
                        out: j.get("stdout").and_then(Json::as_str).unwrap_or("").to_string(),
                        status: j.get("exit_status").and_then(Json::as_i64).unwrap_or(-1),
                        stopped: j.get("stopped").and_then(Json::as_str).map(str::to_string),
                    });
                }
                all.lock().unwrap().extend(mine);
            });
        }
    });
    let mut units = all.into_inner().unwrap();
    units.sort_by_key(|u| u.id);
    // The files as they ended, read afresh: the order of the appends to
    // each.
    let mut c = Client::new(addr);
    let finals = {
        let line: String = (0..FILES).map(|f| format!("echo '== {f}'; cat f{f}.txt 2>/dev/null; ")).collect();
        let (_, j) = unit(&mut c, key, fs, &line);
        parse_reads(j.get("stdout").and_then(Json::as_str).unwrap_or(""))?
    };
    let mut order: Vec<Vec<usize>> = vec![Vec::new(); FILES];
    for (f, toks) in finals {
        order[f] = toks;
    }
    let n = units.len();
    let mut stopped = 0;
    // Checks on each unit alone.
    let mut kept = vec![false; n];
    for u in &units {
        match u.stopped.as_deref() {
            None => {
                if u.status != 0 {
                    return Err(format!("unit {} ({:?}) ended with status {}", u.id, u.line, u.status));
                }
                kept[u.id] = !u.writes.is_empty();
            }
            Some("deadline") => stopped += 1,
            Some(other) => return Err(format!("unit {} ({:?}) was stopped: {}", u.id, u.line, other)),
        }
    }
    // Each kept write appears once in each file it wrote; a stopped one
    // in none.
    let mut pos: Vec<Vec<Option<usize>>> = vec![vec![None; FILES]; n];
    for f in 0..FILES {
        for (i, t) in order[f].iter().enumerate() {
            let u = units.get(*t).ok_or(format!("file {} holds the token of unit {}, which was never sent", f, t))?;
            if !u.writes.contains(&f) {
                return Err(format!("file {} holds the token of unit {} ({:?}), which does not write it", f, t, u.line));
            }
            if u.stopped.is_some() {
                return Err(format!("unit {} ({:?}) was stopped, yet its token is in file {}: a stopped unit left something", t, u.line, f));
            }
            if pos[*t][f].replace(i).is_some() {
                return Err(format!("file {} holds the token of unit {} twice", f, t));
            }
        }
    }
    for u in &units {
        if kept[u.id] {
            for f in &u.writes {
                if pos[u.id][*f].is_none() {
                    return Err(format!("unit {} ({:?}) was answered, but its token is not in file {}", u.id, u.line, f));
                }
            }
        }
    }
    // The graph: unit -> unit that must come after it. Kept writes are
    // ordered as in each file; a read sits after the appends it saw and
    // before the first it did not see; and a unit answered before another
    // was sent comes before it.
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
    let add = |e: &mut Vec<Vec<usize>>, a: usize, b: usize| {
        if a != b {
            e[a].push(b);
        }
    };
    for f in 0..FILES {
        for w in order[f].windows(2) {
            add(&mut edges, w[0], w[1]);
        }
    }
    for u in &units {
        if u.stopped.is_some() {
            continue;
        }
        let got = parse_reads(&u.out).map_err(|e| format!("unit {} ({:?}): {}", u.id, u.line, e))?;
        let seen: Vec<usize> = got.iter().map(|g| g.0).collect();
        if seen != u.reads {
            return Err(format!("unit {} ({:?}) printed the files {:?}", u.id, u.line, seen));
        }
        for (f, toks) in got {
            let full = &order[f];
            if toks.len() > full.len() || toks[..] != full[..toks.len()] {
                return Err(format!("unit {} ({:?}) read file {} as {:?}, which is not how that file began: {:?}", u.id, u.line, f, toks, &full[..toks.len().min(full.len())]));
            }
            if let Some(last) = toks.last() {
                add(&mut edges, *last, u.id);
            }
            if let Some(first_unseen) = full.get(toks.len()) {
                add(&mut edges, u.id, *first_unseen);
            }
        }
    }
    // Real time, with a chain of moments: each unit leads to the moment it
    // was answered; each moment to the next; and the last moment before a
    // unit was sent leads to that unit.
    let mut ends: Vec<(Instant, usize)> = units.iter().map(|u| (u.end, u.id)).collect();
    ends.sort();
    let moment = |i: usize| n + i;
    let mut edges = edges;
    edges.resize(n + ends.len(), Vec::new());
    for (i, (_, id)) in ends.iter().enumerate() {
        edges[*id].push(moment(i));
        if i + 1 < ends.len() {
            edges[moment(i)].push(moment(i + 1));
        }
    }
    for u in &units {
        let k = ends.partition_point(|(t, _)| *t < u.start);
        if k > 0 {
            edges[moment(k - 1)].push(u.id);
        }
    }
    // An order exists if the graph has no cycle.
    let total = edges.len();
    let mut indeg = vec![0usize; total];
    for e in &edges {
        for b in e {
            indeg[*b] += 1;
        }
    }
    let mut ready: Vec<usize> = (0..total).filter(|i| indeg[*i] == 0).collect();
    let mut placed = 0;
    while let Some(a) = ready.pop() {
        placed += 1;
        for b in &edges[a] {
            indeg[*b] -= 1;
            if indeg[*b] == 0 {
                ready.push(*b);
            }
        }
    }
    if placed != total {
        let stuck: Vec<String> = (0..n).filter(|i| indeg[*i] > 0).take(12).map(|i| format!("unit {} {:?} -> {:?}", i, units[i].line, units[i].out)).collect();
        return Err(format!("no order of the units explains every output; units on a cycle (or after one):\n    {}", stuck.join("\n    ")));
    }
    Ok((n, stopped))
}
