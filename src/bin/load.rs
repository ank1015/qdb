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
        _ => die(&format!("no workload {}\n{}", workload, USAGE)),
    }
    let _ = Arc::new(());
}
