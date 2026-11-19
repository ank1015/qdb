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

  kill        the kill test, with --spawn DATA: --clients connections
              append numbered lines to --filesystems filesystems, and the
              server is killed with SIGKILL at a random moment, --count
              times; after each kill every filesystem is opened and
              checked, and must hold every unit that was answered, whole

  isolation   two accounts with marker text in their filesystems; --count
              requests from --clients connections with each key against
              its own and the other's IDs, guessed IDs and odd paths, and
              commands that reach for what lies outside a filesystem. No
              answer may hold the other account's markers or IDs, and a
              request for the other's filesystem must be answered as one
              for a filesystem that does not exist

  long        the long run, with --spawn DATA: --clients connections, for
              --seconds, on up to --filesystems filesystems that are made,
              used (mostly a few hot ones, so the rest go idle and are
              closed), and deleted; some units are stopped at a deadline,
              some streamed, some in sessions. The server's memory and
              open files are sampled each 10 s; at the end every
              filesystem left must take a write at once

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
    if o.rest.first().map(String::as_str) == Some("kill") {
        std::process::exit(kill_run(&o));
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
                        drop(_child);
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
        "long" => {
            let pid = _child.as_ref().map(|c| c.0.id()).unwrap_or_else(|| die("long needs --spawn DATA"));
            let r = long_run(&addr, &o, pid);
            drop(_child);
            std::process::exit(r);
        }
        "isolation" => {
            let r = isolation(&addr, &o.admin, o.count, o.clients, o.seed);
            drop(_child);
            std::process::exit(r);
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
    let read = |files: &[usize]| files.iter().map(|f| format!("echo '== {f}'; cat f{f}.txt 2>/dev/null || true; ")).collect::<String>();
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
        let line: String = (0..FILES).map(|f| format!("echo '== {f}'; cat f{f}.txt 2>/dev/null || true; ")).collect();
        let (_, j) = unit(&mut c, key, fs, &line);
        parse_reads(j.get("stdout").and_then(Json::as_str).unwrap_or(""))?
    };
    let mut order: Vec<Vec<usize>> = vec![Vec::new(); FILES];
    for (f, toks) in finals {
        order[f] = toks;
    }
    let n = units.len();
    // To see the checker catch a stale read: one read is made to look as
    // if it was sent after an append it did not see had been answered.
    if std::env::var_os("QDB_ORDER_MUTATE").is_some() {
        let pick = units.iter().position(|u| u.writes.is_empty() && u.stopped.is_none() && u.reads.iter().any(|f| {
            let seen = parse_reads(&u.out).ok().and_then(|g| g.iter().find(|x| x.0 == *f).map(|x| x.1.len())).unwrap_or(0);
            order[*f].len() > seen
        }));
        if let Some(i) = pick {
            let got = parse_reads(&units[i].out).unwrap();
            let (f, toks) = got.iter().find(|(f, t)| order[*f].len() > t.len()).unwrap();
            let unseen = order[*f][toks.len()];
            units[i].start = units[unseen].end + std::time::Duration::from_nanos(1);
            units[i].end = units[i].end.max(units[i].start);
            eprintln!("mutated: unit {} now sent after unit {} was answered", i, unseen);
        }
    }
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

// ------------------------------------------------------------ the kill test

fn kill_run(o: &Opts) -> i32 {
    let data = o.spawn.clone().unwrap_or_else(|| die("kill needs --spawn DATA"));
    let _ = std::fs::remove_dir_all(&data);
    let kills = o.count.min(100_000);
    let nfs = o.filesystems.max(1);
    let clients = o.clients.max(1);
    let mut rng = Rng(o.seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let (child, addr) = spawn(&data, &o.admin);
    let mut child = Some(child);
    let mut c = Client::new(&addr);
    let key = account(&mut c, &o.admin);
    let fss: Vec<String> = (0..nfs).map(|_| new_fs(&mut c, &key)).collect();
    // For each client: the next number it sends, and the last one
    // answered.
    let mut next = vec![0u64; clients];
    let mut answered_total = 0u64;
    let mut units_total = 0u64;
    let mut addr = addr;
    let t0 = Instant::now();
    for k in 0..kills {
        let stop = AtomicBool::new(false);
        let answered: Vec<AtomicU64> = (0..clients).map(|_| AtomicU64::new(u64::MAX)).collect();
        let sent: Vec<AtomicU64> = (0..clients).map(|_| AtomicU64::new(0)).collect();
        let units = AtomicU64::new(0);
        let wait = Duration::from_millis(50 + rng.below(400) as u64);
        std::thread::scope(|s| {
            for (i, start) in next.iter().enumerate() {
                let (stop, answered, sent, units, fss, key, addr) = (&stop, &answered, &sent, &units, &fss, &key, &addr);
                let start = *start;
                s.spawn(move || {
                    let mut c = Client::new(addr);
                    let fs = &fss[i % fss.len()];
                    let mut n = start;
                    let mut reads = 0u64;
                    while !stop.load(Relaxed) {
                        // A read now and then, to load the server.
                        let line = if reads % 4 == 3 { format!("tail -n 1 c{i}.txt 2>/dev/null; true") } else { format!("echo {n} >> c{i}.txt; echo {n} >> d{i}.txt") };
                        let body = Json::obj().with("command", line.as_str()).to_string();
                        let writing = reads % 4 != 3;
                        reads += 1;
                        if writing {
                            sent[i].store(n, Relaxed);
                        }
                        match c.request("POST", &format!("/v1/filesystems/{}/run", fs), Some(key), body.as_bytes()) {
                            Ok(r) if r.status == 200 => {
                                units.fetch_add(1, Relaxed);
                                if writing {
                                    let j = Json::parse(&r.body).unwrap();
                                    if j.get("exit_status").and_then(Json::as_i64) == Some(0) {
                                        answered[i].store(n, Relaxed);
                                        n += 1;
                                    }
                                }
                            }
                            _ => return,
                        }
                    }
                });
            }
            std::thread::sleep(wait);
            // SIGKILL: nothing is flushed, nothing is closed.
            if let Some(mut ch) = child.take() {
                unsafe { libc::kill(ch.0.id() as i32, libc::SIGKILL) };
                let _ = ch.0.wait();
                std::mem::forget(ch);
            }
            stop.store(true, Relaxed);
        });
        units_total += units.load(Relaxed);
        // Every filesystem opened as the server would, and checked.
        for (f, fs) in fss.iter().enumerate() {
            let dir = std::path::Path::new(&data).join("fs").join(&fs[..2]).join(fs);
            let (db, _) = match qdb::Db::open_dir(&dir, qdb::Config::default()) {
                Ok(r) => r,
                Err(e) => {
                    println!("FAIL  kill {}: filesystem {} does not open: {}", k + 1, f, e);
                    return 1;
                }
            };
            if let Err(e) = db.check() {
                println!("FAIL  kill {}: filesystem {}: check: {}", k + 1, f, e);
                return 1;
            }
            for i in (0..clients).filter(|i| i % nfs == f) {
                let read = |name: String| -> Vec<u64> {
                    match db.resolve(name.as_bytes()) {
                        Ok(id) => String::from_utf8_lossy(&db.content(id)).lines().map(|l| l.parse().unwrap_or(u64::MAX)).collect(),
                        Err(_) => Vec::new(),
                    }
                };
                let (cs, ds) = (read(format!("c{i}.txt")), read(format!("d{i}.txt")));
                let want: Vec<u64> = (0..cs.len() as u64).collect();
                if cs != want || ds != cs {
                    println!("FAIL  kill {}: client {}: the files are not the whole units 0 to {} in order: c has {:?}..., d has {} lines", k + 1, i, cs.len(), &cs[..cs.len().min(5)], ds.len());
                    return 1;
                }
                let got = cs.len() as u64;
                let a = answered[i].load(Relaxed);
                if a != u64::MAX && got < a + 1 {
                    println!("FAIL  kill {}: client {}: unit {} was answered, but the filesystem holds only units up to {}", k + 1, i, a, got as i64 - 1);
                    return 1;
                }
                if got > sent[i].load(Relaxed).max(next[i]) + 1 {
                    println!("FAIL  kill {}: client {}: the filesystem holds units that were never sent", k + 1, i);
                    return 1;
                }
                if a != u64::MAX {
                    answered_total += a + 1 - next[i];
                }
                next[i] = got;
            }
            db.close().ok();
        }
        let (ch, a) = spawn(&data, &o.admin);
        child = Some(ch);
        addr = a;
        // The account and its key outlived the kill.
        let mut c = Client::new(&addr);
        let r = c.request("GET", "/v1/filesystems", Some(&key), b"").unwrap_or_else(|e| die(&format!("after kill {}: {}", k + 1, e)));
        if r.status != 200 {
            println!("FAIL  kill {}: the account's key no longer works: HTTP {}", k + 1, r.status);
            return 1;
        }
    }
    drop(child);
    println!(
        "kill test: {} kills with SIGKILL under load from {} connections on {} filesystems, seed {}: {} units run, {} answered writes; after every kill each filesystem opened, passed its check and held every answered unit, whole and in order ({:.0} s)",
        kills,
        clients,
        nfs,
        o.seed,
        units_total,
        answered_total,
        t0.elapsed().as_secs_f64()
    );
    0
}

// ------------------------------------------------------- the isolation test

fn isolation(addr: &str, admin: &str, count: usize, clients: usize, seed: u64) -> i32 {
    let mut c = Client::new(addr);
    struct Acc {
        key: String,
        id: String,
        fss: Vec<String>,
        marker: String,
        session: String,
    }
    let mut accs = Vec::new();
    for name in ["A", "B"] {
        let j = call(&mut c, "POST", "/v1/accounts", admin, r#"{"limits":{"units_at_once":1000}}"#);
        let key = j.get("key").and_then(Json::as_str).unwrap().to_string();
        let id = j.get("account").and_then(Json::as_str).unwrap().to_string();
        let marker = format!("MARKER-{}-{}", name, &qdb::host::new_id()[..12]);
        let fss: Vec<String> = (0..3).map(|_| new_fs(&mut c, &key)).collect();
        for f in &fss {
            unit(&mut c, &key, f, &format!("mkdir -p a/b .hidden; echo {m} > m.txt; echo {m} > a/b/deep.txt; echo {m} > .hidden/x; echo {m} > 'odd name %2e.txt'", m = marker));
        }
        let session = call(&mut c, "POST", &format!("/v1/filesystems/{}/sessions", fss[0]), &key, "").get("session").and_then(Json::as_str).unwrap().to_string();
        accs.push(Acc { key, id, fss, marker, session });
    }
    let accs = &accs;
    const LINES: &[&str] = &[
        "cat m.txt",
        "cat /m.txt /../m.txt ../../m.txt",
        "ls -la / .. ../.. /..",
        "find / -name '*'",
        "grep -r MARKER /",
        "rg MARKER /",
        "cat ../../../../server/registry.log",
        "cat /../../../fs/*/*/m.txt",
        "ls ../../../../fs",
        "cd ..; cd ..; pwd; ls",
        "cat \"$HOME/m.txt\"; echo $HOME $PWD",
        "find .. -maxdepth 3",
        "cat /proc/self/environ /etc/passwd",
        "ls -R /",
        "xargs cat < m.txt",
        "cat a/b/../../m.txt",
    ];
    let failures = AtomicU64::new(0);
    let done = AtomicU64::new(0);
    let first_failure: Mutex<Option<String>> = Mutex::new(None);
    // A self-test of the checker: send every 100th request with the other
    // account's key, which must be caught.
    let mutate = std::env::var_os("QDB_ISOLATION_MUTATE").is_some();
    let t0 = Instant::now();
    std::thread::scope(|s| {
        for t in 0..clients.max(1) {
            let (failures, done, first_failure) = (&failures, &done, &first_failure);
            s.spawn(move || {
                let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(t as u64 * 104729) | 1);
                let mut c = Client::new(addr);
                let mut none = Client::new(addr);
                while (done.fetch_add(1, Relaxed) as usize) < count {
                    let me = rng.below(2);
                    let swap = mutate && rng.below(100) == 0;
                    let (mine, other) = (&accs[me], &accs[1 - me]);
                    // Whose filesystem the request names.
                    let target_other = rng.below(2) == 0;
                    let fs: String = match rng.below(8) {
                        0..=3 => (if target_other { &other.fss } else { &mine.fss })[rng.below(3)].clone(),
                        4 => qdb::host::new_id(),
                        5 => other.fss[0].to_uppercase(),
                        6 => format!("{}/../{}", mine.fss[0], other.fss[0]),
                        _ => {
                            let mut f = other.fss[rng.below(3)].clone().into_bytes();
                            let i = rng.below(32);
                            f[i] = if f[i] == b'0' { b'1' } else { b'0' };
                            String::from_utf8(f).unwrap()
                        }
                    };
                    let (method, path, body) = match rng.below(8) {
                        0 => ("GET", format!("/v1/filesystems/{}", fs), String::new()),
                        1 => ("DELETE", format!("/v1/filesystems/{}/sessions/{}", fs, other.session), String::new()),
                        2 => ("POST", format!("/v1/filesystems/{}/run", fs), Json::obj().with("command", "pwd").with("session", other.session.as_str()).to_string()),
                        3 => ("GET", format!("/v1/accounts/{}", other.id), String::new()),
                        4 => ("POST", format!("/v1/filesystems/{}/sessions", fs), String::new()),
                        _ => ("POST", format!("/v1/filesystems/{}/run", fs), Json::obj().with("command", LINES[rng.below(LINES.len())]).to_string()),
                    };
                    let key = if swap { &other.key } else { &mine.key };
                    let r = match c.request(method, &path, Some(key), body.as_bytes()) {
                        Ok(r) => r,
                        Err(e) => die(&format!("{} {}: {}", method, path, e)),
                    };
                    let text = String::from_utf8_lossy(&r.body);
                    let mut bad = None;
                    if text.contains(&other.marker) {
                        bad = Some("holds the other account's marker");
                    } else if other.fss.iter().any(|f| text.contains(f.as_str())) || text.contains(&other.id) || text.contains(&other.key) || text.contains(&other.session) {
                        bad = Some("names one of the other account's IDs");
                    } else if fs.len() == 32 && other.fss.contains(&fs) {
                        // The same answer as for a filesystem that is not there.
                        let fake = qdb::host::new_id();
                        let r2 = none.request(method, &path.replace(&fs, &fake), Some(&mine.key), body.replace(&fs, &fake).as_bytes()).unwrap_or_else(|e| die(&e.to_string()));
                        if r2.status != r.status || r2.body != r.body {
                            bad = Some("is not the answer for a filesystem that does not exist");
                        }
                    }
                    if let Some(why) = bad {
                        failures.fetch_add(1, Relaxed);
                        let mut f = first_failure.lock().unwrap();
                        if f.is_none() {
                            *f = Some(format!("{} {} {} with the key of account {}: the answer {}: HTTP {} {}", method, path, body, me, why, r.status, text.chars().take(300).collect::<String>()));
                        }
                    }
                }
            });
        }
    });
    let n = failures.load(Relaxed);
    if n > 0 {
        println!("FAIL  isolation: {} answers leaked; the first: {}", n, first_failure.into_inner().unwrap().unwrap());
        return 1;
    }
    // Each still sees its own.
    for a in accs.iter() {
        let (_, j) = unit(&mut c, &a.key, &a.fss[0], "cat m.txt");
        if j.get("stdout").and_then(Json::as_str) != Some(&format!("{}\n", a.marker)) {
            println!("FAIL  isolation: an account no longer reads its own file");
            return 1;
        }
    }
    println!("isolation: {} requests from {} connections with two accounts' keys, against their own and each other's filesystems, guessed IDs and odd paths, seed {}: no answer held the other account's markers or IDs, and every request for the other's filesystem was answered as for one that does not exist ({:.0} s)", count, clients, seed, t0.elapsed().as_secs_f64());
    0
}

// ------------------------------------------------------------ the long run

/// The server's resident memory in KB, and its open files.
fn sample(pid: u32) -> (u64, usize) {
    let status = std::fs::read_to_string(format!("/proc/{}/status", pid)).unwrap_or_default();
    let rss = status.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let fds = std::fs::read_dir(format!("/proc/{}/fd", pid)).map(|d| d.count()).unwrap_or(0);
    (rss, fds)
}

fn long_run(addr: &str, o: &Opts, pid: u32) -> i32 {
    let mut c = Client::new(addr);
    let key = account(&mut c, &o.admin);
    let most = o.filesystems.max(16);
    let pool: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let (made, deleted, units, stopped, streamed, sessions) = (AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0));
    let failure: Mutex<Option<String>> = Mutex::new(None);
    let stop = AtomicBool::new(false);
    let samples: Mutex<Vec<(f64, u64, usize)>> = Mutex::new(Vec::new());
    const SETUP: &str = "mkdir -p src docs; for i in 1 2 3 4 5 6 7 8; do echo \"line $i of a small file with some words\" > src/f$i.txt; done; seq 1 300 > docs/n.txt";
    const READS: &[&str] = &["cat src/f3.txt", "grep -rn words src", "wc -l docs/n.txt", "ls -la src", "head -n 5 docs/n.txt", "find . -name '*.txt'", "rg -c 1 docs", "sort -r docs/n.txt | head -n 3", "awk '{s+=$1} END {print s}' docs/n.txt", "sed -n 2p src/f1.txt"];
    for _ in 0..16 {
        let f = new_fs(&mut c, &key);
        unit(&mut c, &key, &f, SETUP);
        pool.lock().unwrap().push(f);
    }
    let t0 = Instant::now();
    std::thread::scope(|s| {
        s.spawn(|| {
            while !stop.load(Relaxed) {
                let (rss, fds) = sample(pid);
                samples.lock().unwrap().push((t0.elapsed().as_secs_f64(), rss, fds));
                for _ in 0..100 {
                    if stop.load(Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        });
        for t in 0..o.clients.max(1) {
            let (pool, made, deleted, units, stopped, streamed, sessions, failure, stop, key) = (&pool, &made, &deleted, &units, &stopped, &streamed, &sessions, &failure, &stop, &key);
            s.spawn(move || {
                let mut rng = Rng(o.seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(t as u64 * 7919) | 1);
                let mut c = Client::new(addr);
                let fail = |m: String| {
                    let mut f = failure.lock().unwrap();
                    if f.is_none() {
                        *f = Some(m);
                    }
                    stop.store(true, Relaxed);
                };
                let mut k = 0usize;
                while !stop.load(Relaxed) && t0.elapsed().as_secs_f64() < o.seconds {
                    k += 1;
                    let r = rng.below(1000);
                    let n = pool.lock().unwrap().len();
                    if r < 8 && n < most {
                        let f = new_fs(&mut c, key);
                        unit(&mut c, key, &f, SETUP);
                        pool.lock().unwrap().push(f);
                        made.fetch_add(1, Relaxed);
                        continue;
                    }
                    if r < 12 && n > most / 2 {
                        let f = {
                            let mut p = pool.lock().unwrap();
                            let i = 16 + rng.below(p.len() - 16);
                            p.swap_remove(i)
                        };
                        call(&mut c, "DELETE", &format!("/v1/filesystems/{}", f), key, "");
                        deleted.fetch_add(1, Relaxed);
                        continue;
                    }
                    // Most units go to the first few filesystems; the rest
                    // are touched now and then and go idle between.
                    let f = {
                        let p = pool.lock().unwrap();
                        if rng.below(10) < 8 { p[rng.below(16)].clone() } else { p[rng.below(p.len())].clone() }
                    };
                    let mut body = Json::obj();
                    let line = match rng.below(100) {
                        0..=4 => format!("echo {} > src/f{}.txt", k, k % 8 + 1),
                        5 => format!("seq 1 {} > docs/n.txt", 100 + k % 400),
                        6 => {
                            body.set("limits", Json::obj().with("time_ms", 5u64));
                            "while true; do echo x >> docs/spin.txt; done".to_string()
                        }
                        7 => {
                            body.set("stream", true);
                            "seq 1 20000".to_string()
                        }
                        _ => READS[rng.below(READS.len())].to_string(),
                    };
                    body.set("command", line.as_str());
                    let session = if rng.below(50) == 0 {
                        let r = c.request("POST", &format!("/v1/filesystems/{}/sessions", f), Some(key), b"");
                        match r {
                            Ok(r) if r.status == 201 => Json::parse(&r.body).ok().and_then(|j| j.get("session").and_then(Json::as_str).map(String::from)),
                            Ok(r) if r.status == 404 => continue, // deleted meanwhile
                            Ok(r) => {
                                fail(format!("a session: HTTP {}: {}", r.status, String::from_utf8_lossy(&r.body)));
                                break;
                            }
                            Err(e) => {
                                fail(format!("a session: {}", e));
                                break;
                            }
                        }
                    } else {
                        None
                    };
                    if let Some(sid) = &session {
                        body.set("session", sid.as_str());
                    }
                    let r = c.request("POST", &format!("/v1/filesystems/{}/run", f), Some(key), body.to_string().as_bytes());
                    match r {
                        Ok(r) if r.status == 200 => {
                            units.fetch_add(1, Relaxed);
                            if line.starts_with("while") {
                                let j = Json::parse(&r.body).unwrap_or(Json::Null);
                                if j.get("stopped").and_then(Json::as_str) == Some("deadline") {
                                    stopped.fetch_add(1, Relaxed);
                                } else {
                                    fail(format!("a unit with a deadline was not stopped by it: {}", String::from_utf8_lossy(&r.body)));
                                }
                                // Leave the filesystem as small as it was.
                            } else if body.get("stream").is_some() {
                                let last = r.body.split(|&b| b == b'\n').filter(|l| !l.is_empty()).last().unwrap_or(b"");
                                if !String::from_utf8_lossy(last).contains("\"exit_status\":0") {
                                    fail(format!("a streamed unit ended badly: {}", String::from_utf8_lossy(last)));
                                }
                                streamed.fetch_add(1, Relaxed);
                            }
                        }
                        Ok(r) if r.status == 404 => {}
                        Ok(r) => fail(format!("{:?}: HTTP {}: {}", line, r.status, String::from_utf8_lossy(&r.body).chars().take(300).collect::<String>())),
                        Err(e) => fail(format!("{:?}: {}", line, e)),
                    }
                    if let Some(sid) = session {
                        let _ = c.request("DELETE", &format!("/v1/filesystems/{}/sessions/{}", f, sid), Some(key), b"");
                        sessions.fetch_add(1, Relaxed);
                    }
                }
            });
        }
        while !stop.load(Relaxed) && t0.elapsed().as_secs_f64() < o.seconds {
            std::thread::sleep(Duration::from_millis(200));
        }
        stop.store(true, Relaxed);
    });
    if let Some(f) = failure.into_inner().unwrap() {
        println!("FAIL  long run after {:.0} s: {}", t0.elapsed().as_secs_f64(), f);
        return 1;
    }
    // No filesystem left locked: each takes a write at once.
    let left = pool.into_inner().unwrap();
    let mut slowest = 0u64;
    for f in &left {
        let mut body = Json::obj().with("command", "echo last > last.txt; cat last.txt");
        body.set("limits", Json::obj().with("time_ms", 2000u64));
        let t = Instant::now();
        let r = c.request("POST", &format!("/v1/filesystems/{}/run", f), Some(&key), body.to_string().as_bytes()).unwrap_or_else(|e| die(&e.to_string()));
        slowest = slowest.max(t.elapsed().as_micros() as u64);
        if r.status != 200 || !String::from_utf8_lossy(&r.body).contains("\"stdout\":\"last\\n\"") {
            println!("FAIL  long run: filesystem {} did not take a write at the end: HTTP {}: {}", f, r.status, String::from_utf8_lossy(&r.body));
            return 1;
        }
    }
    let samples = samples.into_inner().unwrap();
    let mut sheet = String::from("seconds\trss_kb\topen_files\n");
    for (t, r, f) in &samples {
        sheet.push_str(&format!("{:.0}\t{}\t{}\n", t, r, f));
    }
    let _ = std::fs::write(format!("{}/long-samples.tsv", o.spawn.as_deref().unwrap()), &sheet);
    // Level: after the first fifth of the run, the most is within a margin
    // of the level reached by the end of that fifth.
    let settle = o.seconds / 5.0;
    let early: Vec<_> = samples.iter().filter(|s| s.0 <= settle).collect();
    let late: Vec<_> = samples.iter().filter(|s| s.0 > settle).collect();
    let max = |v: &[&(f64, u64, usize)], i: usize| v.iter().map(|s| if i == 0 { s.1 } else { s.2 as u64 }).max().unwrap_or(0);
    let (rss_early, rss_late, fd_early, fd_late) = (max(&early, 0), max(&late, 0), max(&early, 1), max(&late, 1));
    let last = samples.last().copied().unwrap_or((0.0, 0, 0));
    let level = rss_late as f64 <= rss_early as f64 * 1.25 + 8192.0 && fd_late <= fd_early + 16;
    println!(
        "{}long run: {:.0} s from {} connections; {} units ({} stopped at a deadline, {} streamed, {} in sessions); {} filesystems made and {} deleted, {} left; memory most {} MB in the first {:.0} s and {} MB after, {} MB at the end; open files most {} then {}, {} at the end; every filesystem left took a write at once (slowest {:.1} ms)",
        if level { "" } else { "FAIL  " },
        t0.elapsed().as_secs_f64(),
        o.clients,
        units.load(Relaxed),
        stopped.load(Relaxed),
        streamed.load(Relaxed),
        sessions.load(Relaxed),
        made.load(Relaxed) + 16,
        deleted.load(Relaxed),
        left.len(),
        rss_early / 1024,
        settle,
        rss_late / 1024,
        last.1 / 1024,
        fd_early,
        fd_late,
        last.2,
        slowest as f64 / 1000.0
    );
    if level { 0 } else { 1 }
}
