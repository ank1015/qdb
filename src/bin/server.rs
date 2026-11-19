//! qdb-server: the service over HTTP/1.1 and JSON. See docs/API.md and
//! docs/OPERATIONS.md.

use qdb::net::json::Json;
use qdb::service::{self, Service, ServiceConfig};
use qdb::SyncMode;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static TERM: AtomicBool = AtomicBool::new(false);

extern "C" fn on_term(_: libc::c_int) {
    TERM.store(true, Ordering::SeqCst);
}

const USAGE: &str = "usage: qdb-server --data DIR [--listen ADDR] [--config FILE] [settings]

The admin key comes from --admin-key-file FILE, or the QDB_ADMIN_KEY
environment variable, or --admin-key KEY (seen by other users of the
machine; avoid it). It must be at least 16 characters.

Settings, each also a field of the --config JSON object (without the
dashes, with _ for -); a flag wins over the file:

  --listen ADDR              127.0.0.1:7878
  --sync always|batch        always
  --group-commit on|off      on: under sync always, writers share syncs
  --workers N                units running at once: 2 per core
  --queue N                  units waiting for a worker: 256
  --max-connections N        1024
  --max-request-bytes N      16777216
  --output-cap-bytes N       1048576 (each of stdout and stderr)
  --session-idle-secs N      900
  --max-open N               filesystems open at once: 1024
  --max-open-bytes N         memory of open filesystems: 2147483648
  --idle-close-secs N        60
  Defaults for new accounts:
  --text-bytes N             10737418240 per filesystem
  --nodes N                  1000000 per filesystem
  --disk-bytes N             21474836480 per filesystem
  --filesystems N            1000 per account
  --time-ms N                30000 per unit
  --commands N               1000000 per unit
  --scratch-bytes N          1073741824 per unit
  --memory-bytes N           1073741824 per unit
  --units-at-once N          16 per account
";

fn die(msg: &str) -> ! {
    eprintln!("qdb-server: {}", msg);
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut flags: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-h" || a == "--help" {
            print!("{}", USAGE);
            return;
        }
        let Some(name) = a.strip_prefix("--") else { die(&format!("unexpected argument {}\n{}", a, USAGE)) };
        let (name, value) = match name.split_once('=') {
            Some((n, v)) => (n.to_string(), v.to_string()),
            None => {
                i += 1;
                let Some(v) = args.get(i) else { die(&format!("--{} needs a value", name)) };
                (name.to_string(), v.clone())
            }
        };
        flags.push((name.replace('-', "_"), value));
        i += 1;
    }
    // The config file first, then the flags over it.
    let mut settings: Vec<(String, String)> = Vec::new();
    if let Some((_, path)) = flags.iter().find(|(n, _)| n == "config") {
        let text = std::fs::read(path).unwrap_or_else(|e| die(&format!("{}: {}", path, e)));
        let j = Json::parse(&text).unwrap_or_else(|e| die(&format!("{}: {}", path, e)));
        if !j.is_obj() {
            die(&format!("{}: must be a JSON object", path));
        }
        for (k, v) in j.fields() {
            let v = match v {
                Json::Str(s) => s.clone(),
                Json::Int(n) => n.to_string(),
                Json::Bool(b) => b.to_string(),
                _ => die(&format!("{}: {} must be a string or a number", path, k)),
            };
            settings.push((k.clone(), v));
        }
    }
    settings.extend(flags.into_iter().filter(|(n, _)| n != "config"));
    let get = |name: &str| settings.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v.clone());
    let num = |name: &str| -> Option<u64> { get(name).map(|v| v.parse::<u64>().ok().filter(|n| *n > 0).unwrap_or_else(|| die(&format!("{} must be a positive whole number", name)))) };
    const KNOWN: &[&str] = &[
        "data", "listen", "admin_key", "admin_key_file", "sync", "group_commit", "workers", "queue", "max_connections", "max_request_bytes", "output_cap_bytes", "session_idle_secs", "max_open",
        "max_open_bytes", "idle_close_secs", "text_bytes", "nodes", "disk_bytes", "filesystems", "time_ms", "commands", "scratch_bytes", "memory_bytes", "units_at_once",
    ];
    for (n, _) in &settings {
        if !KNOWN.contains(&n.as_str()) {
            die(&format!("unknown setting {}\n{}", n, USAGE));
        }
    }
    let Some(data) = get("data") else { die(&format!("--data is needed\n{}", USAGE)) };
    let key = if let Some(f) = get("admin_key_file") {
        let t = std::fs::read_to_string(&f).unwrap_or_else(|e| die(&format!("{}: {}", f, e)));
        t.trim().to_string()
    } else if let Ok(k) = std::env::var("QDB_ADMIN_KEY") {
        k.trim().to_string()
    } else if let Some(k) = get("admin_key") {
        k
    } else {
        die("no admin key: give --admin-key-file, QDB_ADMIN_KEY or --admin-key")
    };
    let mut cfg = ServiceConfig::new(&PathBuf::from(&data), &key);
    match get("sync").as_deref() {
        None | Some("always") => cfg.host.db.sync = SyncMode::Always,
        Some("batch") => cfg.host.db.sync = SyncMode::Batch,
        Some(o) => die(&format!("--sync must be always or batch, not {}", o)),
    }
    match get("group_commit").as_deref() {
        None | Some("on") | Some("true") => cfg.host.group_commit = true,
        Some("off") | Some("false") => cfg.host.group_commit = false,
        Some(o) => die(&format!("--group-commit must be on or off, not {}", o)),
    }
    if let Some(n) = num("workers") {
        cfg.workers = n as usize;
    }
    if let Some(n) = num("queue") {
        cfg.queue = n as usize;
    }
    if let Some(n) = num("max_connections") {
        cfg.max_connections = n as usize;
    }
    if let Some(n) = num("max_request_bytes") {
        cfg.max_request = n as usize;
    }
    if let Some(n) = num("output_cap_bytes") {
        cfg.output_cap = n as usize;
    }
    if let Some(n) = num("session_idle_secs") {
        cfg.session_idle = Duration::from_secs(n);
    }
    if let Some(n) = num("max_open") {
        cfg.host.max_open = n as usize;
    }
    if let Some(n) = num("max_open_bytes") {
        cfg.host.max_open_bytes = n;
    }
    if let Some(n) = num("idle_close_secs") {
        cfg.host.idle_close = Duration::from_secs(n);
    }
    let d = &mut cfg.defaults;
    if let Some(n) = num("text_bytes") {
        d.quotas.text = n;
    }
    if let Some(n) = num("nodes") {
        d.quotas.nodes = n;
    }
    if let Some(n) = num("disk_bytes") {
        d.quotas.disk = n;
    }
    if let Some(n) = num("filesystems") {
        d.max_fs = n;
    }
    if let Some(n) = num("time_ms") {
        d.limits.time = Duration::from_millis(n);
    }
    if let Some(n) = num("commands") {
        d.limits.commands = n;
    }
    if let Some(n) = num("scratch_bytes") {
        d.limits.scratch = n;
    }
    if let Some(n) = num("memory_bytes") {
        d.limits.memory = n as usize;
    }
    if let Some(n) = num("units_at_once") {
        d.max_running = n.min(u32::MAX as u64) as u32;
    }
    let listen = get("listen").unwrap_or_else(|| "127.0.0.1:7878".into());
    unsafe {
        libc::signal(libc::SIGTERM, on_term as extern "C" fn(libc::c_int) as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_term as extern "C" fn(libc::c_int) as *const () as libc::sighandler_t);
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    let svc = Service::open(cfg).unwrap_or_else(|e| die(&e));
    let listener = TcpListener::bind(&listen).unwrap_or_else(|e| die(&format!("{}: {}", listen, e)));
    let addr = listener.local_addr().map(|a| a.to_string()).unwrap_or(listen);
    service::log(&format!("listening on {}", addr));
    // The address on standard output, for whoever started the server.
    println!("{}", addr);
    let svc2 = svc.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(20));
        if TERM.load(Ordering::SeqCst) {
            service::log("stopping");
            svc2.shutdown();
            return;
        }
    });
    service::serve(svc.clone(), listener);
    svc.shutdown();
    service::log("stopped");
}
