//! The service: accounts and keys, filesystems by account, units through
//! HTTP and JSON, sessions, and usage totals. `qdb-server` is a thin
//! program around `serve`. See docs/API.md for the requests and answers,
//! and docs/OPERATIONS.md for running it.
//!
//! What it keeps on disk, besides the filesystems (`host`):
//!
//! ```text
//! DATA/server/registry.log   accounts, key hashes, settings, who owns
//!                            which filesystem: one JSON line per change,
//!                            synced before the change is answered
//! DATA/server/usage.log      usage totals by filesystem and account:
//!                            the totals that changed, appended every
//!                            second and at shutdown
//! ```
//! Both are read at start, the last line for each thing winning, and
//! written again whole.

use crate::engine::{Ending, Sink, UnitEnd};
use crate::host::{self, Host, HostConfig, HostError, Limits, Quotas, Request};
use crate::net::codec::{base64, hex, same_secret, sha256, unbase64};
use crate::net::http::{self, ReadError};
use crate::net::json::Json;
use crate::session::Session;
use crate::tree::Cost;
use crate::unit::{Reason, Stop};
use crate::vfs::lock;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::{self, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Condvar, Mutex, RwLock, Weak};
use std::time::{Duration, Instant};

/// What an account may use. Every field has a default from the server's
/// settings and can be set per account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub quotas: Quotas,
    /// Filesystems the account may have.
    pub max_fs: u64,
    pub limits: Limits,
    /// Units of the account running at once.
    pub max_running: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { quotas: Quotas::default(), max_fs: 1000, limits: Limits::default(), max_running: 16 }
    }
}

impl Settings {
    fn to_json(self) -> Json {
        Json::obj()
            .with(
                "quotas",
                Json::obj().with("text_bytes", self.quotas.text).with("nodes", self.quotas.nodes).with("disk_bytes", self.quotas.disk).with("filesystems", self.max_fs),
            )
            .with(
                "limits",
                Json::obj()
                    .with("time_ms", self.limits.time.as_millis() as u64)
                    .with("commands", self.limits.commands)
                    .with("scratch_bytes", self.limits.scratch)
                    .with("memory_bytes", self.limits.memory as u64)
                    .with("units_at_once", self.max_running as u64),
            )
    }

    /// Apply what a JSON object sets. An unknown field or a bad value is
    /// an error, and nothing is changed.
    fn apply(&mut self, j: &Json) -> Result<(), String> {
        let mut s = *self;
        for (k, v) in j.fields() {
            match k.as_str() {
                "quotas" | "limits" => {
                    if !v.is_obj() {
                        return Err(format!("{} must be an object", k));
                    }
                    for (f, n) in v.fields() {
                        let n = n.as_u64().filter(|n| *n > 0).ok_or_else(|| format!("{}.{} must be a positive whole number", k, f))?;
                        match (k.as_str(), f.as_str()) {
                            ("quotas", "text_bytes") => s.quotas.text = n,
                            ("quotas", "nodes") => s.quotas.nodes = n,
                            ("quotas", "disk_bytes") => s.quotas.disk = n,
                            ("quotas", "filesystems") => s.max_fs = n,
                            ("limits", "time_ms") => s.limits.time = Duration::from_millis(n),
                            ("limits", "commands") => s.limits.commands = n,
                            ("limits", "scratch_bytes") => s.limits.scratch = n,
                            ("limits", "memory_bytes") => s.limits.memory = n as usize,
                            ("limits", "units_at_once") => s.max_running = n.min(u32::MAX as u64) as u32,
                            _ => return Err(format!("unknown setting {}.{}", k, f)),
                        }
                    }
                }
                _ => return Err(format!("unknown field {}", k)),
            }
        }
        *self = s;
        Ok(())
    }
}

/// How the service runs.
#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub data: PathBuf,
    pub admin_key: String,
    pub host: HostConfig,
    /// Units running at once, over all accounts.
    pub workers: usize,
    /// Units that may wait for a worker; more are refused with 429.
    pub queue: usize,
    pub max_connections: usize,
    /// The largest request body.
    pub max_request: usize,
    /// The most bytes of standard output, and of standard error, in an
    /// answer that is not streamed.
    pub output_cap: usize,
    /// A session no unit has used for this long ends.
    pub session_idle: Duration,
    /// What a new account may use.
    pub defaults: Settings,
}

impl ServiceConfig {
    pub fn new(data: &Path, admin_key: &str) -> ServiceConfig {
        let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
        ServiceConfig {
            data: data.to_path_buf(),
            admin_key: admin_key.to_string(),
            host: HostConfig::default(),
            workers: cores * 2,
            queue: 256,
            max_connections: 1024,
            max_request: 16 << 20,
            output_cap: 1 << 20,
            session_idle: Duration::from_secs(15 * 60),
            defaults: Settings::default(),
        }
    }
}

struct Account {
    key_hash: String,
    settings: Settings,
    fss: BTreeSet<String>,
    running: Arc<AtomicU32>,
}

#[derive(Default)]
struct State {
    accounts: HashMap<String, Account>,
    by_key: HashMap<String, String>,
    owner: HashMap<String, String>,
}

/// Usage of one filesystem or account: units run, units stopped, and the
/// counters of every unit added up.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Totals {
    pub units: u64,
    pub stopped: u64,
    pub cost: Cost,
}

impl Totals {
    fn add(&mut self, o: &Totals) {
        self.units += o.units;
        self.stopped += o.stopped;
        let m = self.cost.memory.max(o.cost.memory);
        self.cost.add(&o.cost);
        self.cost.memory = m;
    }

    fn to_json(self) -> Json {
        let mut c = Json::obj();
        for (k, v) in self.cost.fields() {
            // Memory is a high-water mark of one unit, not a total.
            if k == "memory" {
                c.set("memory_max", v);
            } else {
                c.set(k, v);
            }
        }
        Json::obj().with("units", self.units).with("stopped", self.stopped).with("counters", c)
    }

    fn from_json(j: &Json) -> Totals {
        let mut t = Totals { units: j.get("units").and_then(Json::as_u64).unwrap_or(0), stopped: j.get("stopped").and_then(Json::as_u64).unwrap_or(0), ..Totals::default() };
        if let Some(c) = j.get("counters") {
            let g = |k: &str| c.get(k).and_then(Json::as_u64).unwrap_or(0);
            t.cost = Cost {
                text_bytes: g("text_bytes"),
                text_pieces: g("text_pieces"),
                index_bytes: g("index_bytes"),
                index_lookups: g("index_lookups"),
                tree_nodes: g("tree_nodes"),
                content_written: g("content_written"),
                records_written: g("records_written"),
                upkeep_written: g("upkeep_written"),
                syncs: g("syncs"),
                scratch_bytes: g("scratch_bytes"),
                memory: g("memory_max"),
                cpu_ns: g("cpu_us") * 1000,
                elapsed_ns: g("elapsed_us") * 1000,
            };
        }
        t
    }
}

#[derive(Default)]
struct Usage {
    /// By filesystem: its account and its totals.
    fs: HashMap<String, (String, Totals)>,
    /// By account, of its filesystems that have been deleted.
    gone: HashMap<String, Totals>,
    dirty_fs: HashSet<String>,
    dirty_gone: HashSet<String>,
}

struct SessionSlot {
    fs: String,
    account: String,
    sess: Mutex<Session>,
    used: AtomicU64,
}

/// Units running at once, with a bounded queue behind.
struct Slots {
    state: Mutex<(usize, usize)>,
    cv: Condvar,
    max: usize,
    queue: usize,
}

impl Slots {
    /// A slot, waiting in the queue if need be; None if the queue is full
    /// or `give_up` says so while waiting.
    fn take(&self, give_up: &dyn Fn() -> bool) -> Option<SlotGuard<'_>> {
        let mut s = lock(&self.state);
        if s.0 < self.max {
            s.0 += 1;
            return Some(SlotGuard(self));
        }
        if s.1 >= self.queue {
            return None;
        }
        s.1 += 1;
        loop {
            s = self.cv.wait_timeout(s, Duration::from_millis(5)).unwrap_or_else(|e| e.into_inner()).0;
            if s.0 < self.max {
                s.1 -= 1;
                s.0 += 1;
                return Some(SlotGuard(self));
            }
            if give_up() {
                s.1 -= 1;
                return None;
            }
        }
    }

    /// Slots no unit is using: workers a search may borrow.
    pub fn idle(&self) -> usize {
        let s = lock(&self.state);
        self.max.saturating_sub(s.0 + s.1)
    }
}

/// Idle workers are lent to searches (`par`), and taken back as soon as
/// a unit waits for one.
impl crate::par::Lender for Slots {
    fn borrow(&self, want: usize) -> usize {
        let mut s = lock(&self.state);
        if s.1 > 0 {
            return 0;
        }
        let n = want.min(self.max.saturating_sub(s.0));
        s.0 += n;
        n
    }

    fn give_back(&self, n: usize) {
        lock(&self.state).0 -= n;
        self.cv.notify_all();
    }

    fn wanted(&self) -> bool {
        lock(&self.state).1 > 0
    }
}

struct SlotGuard<'a>(&'a Slots);

impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        lock(&self.0.state).0 -= 1;
        self.0.cv.notify_one();
    }
}

/// Watches the connections of running units, and stops a unit whose
/// caller has gone away.
struct Watcher {
    conns: Mutex<HashMap<u64, (i32, Weak<Stop>)>>,
    next: AtomicU64,
}

impl Watcher {
    fn start(stopping: Arc<AtomicBool>) -> Arc<Watcher> {
        let w = Arc::new(Watcher { conns: Mutex::new(HashMap::new()), next: AtomicU64::new(0) });
        let w2 = Arc::downgrade(&w);
        std::thread::Builder::new()
            .name("qdb-callers".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(5));
                let Some(w) = w2.upgrade() else { return };
                if stopping.load(Relaxed) && lock(&w.conns).is_empty() {
                    return;
                }
                w.check();
            })
            .expect("spawn the caller watcher");
        w
    }

    fn add(&self, fd: i32, stop: &Arc<Stop>) -> u64 {
        let n = self.next.fetch_add(1, Relaxed);
        lock(&self.conns).insert(n, (fd, Arc::downgrade(stop)));
        n
    }

    fn remove(&self, n: u64) {
        lock(&self.conns).remove(&n);
    }

    fn check(&self) {
        let list: Vec<(u64, i32)> = lock(&self.conns).iter().map(|(n, (fd, _))| (*n, *fd)).collect();
        if list.is_empty() {
            return;
        }
        let mut fds: Vec<libc::pollfd> = list.iter().map(|(_, fd)| libc::pollfd { fd: *fd, events: libc::POLLRDHUP, revents: 0 }).collect();
        // SAFETY: poll on descriptors the connections hold open while
        // they are listed here.
        unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 0) };
        let conns = lock(&self.conns);
        for ((n, _), p) in list.iter().zip(&fds) {
            if p.revents & (libc::POLLRDHUP | libc::POLLHUP | libc::POLLERR) != 0 {
                if let Some(stop) = conns.get(n).and_then(|(_, w)| w.upgrade()) {
                    stop.request(Reason::Gone);
                }
            }
        }
    }
}

pub struct Service {
    cfg: ServiceConfig,
    host: Arc<Host>,
    state: RwLock<State>,
    registry: Mutex<std::fs::File>,
    usage: Mutex<Usage>,
    usage_file: Mutex<std::fs::File>,
    sessions: Mutex<HashMap<String, Arc<SessionSlot>>>,
    slots: Arc<Slots>,
    watcher: Arc<Watcher>,
    start: Instant,
    stopping: Arc<AtomicBool>,
    connections: AtomicUsize,
    /// Open connections, to be cut at shutdown.
    conns: Mutex<HashMap<u64, TcpStream>>,
    next_conn: AtomicU64,
}

/// A line for the log: the time, then the text.
pub fn log(text: &str) {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i64).unwrap_or(0);
    let t = crate::when::format(now, b"%Y-%m-%dT%H:%M:%SZ").unwrap_or_default();
    eprintln!("{} {}", String::from_utf8_lossy(&t), text);
}

fn append_line(f: &Mutex<std::fs::File>, j: &Json, sync: bool) -> io::Result<()> {
    let mut line = j.to_string();
    line.push('\n');
    let mut f = lock(f);
    f.write_all(line.as_bytes())?;
    if sync {
        f.sync_data()?;
    }
    Ok(())
}

fn read_lines(path: &Path) -> Vec<Json> {
    let Ok(text) = std::fs::read(path) else { return Vec::new() };
    let mut out = Vec::new();
    for line in text.split(|c| *c == b'\n') {
        if line.is_empty() {
            continue;
        }
        // A line cut short by a crash is the last one, and is dropped.
        if let Ok(j) = Json::parse(line) {
            out.push(j);
        }
    }
    out
}

/// Write a file whole: under a temporary name, synced, then renamed.
fn write_whole(path: &Path, lines: &[Json]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::File::create(&tmp)?;
    let mut text = String::new();
    for j in lines {
        j.write(&mut text);
        text.push('\n');
    }
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(&tmp, path)?;
    if let Ok(d) = std::fs::File::open(path.parent().unwrap()) {
        let _ = d.sync_all();
    }
    Ok(())
}

fn open_append(path: &Path) -> io::Result<std::fs::File> {
    std::fs::OpenOptions::new().create(true).append(true).open(path)
}

fn key_hash(key: &str) -> String {
    hex(&sha256(key.as_bytes()))
}

impl Service {
    /// Open the data directory: take its lock, read the registry and the
    /// usage totals, and remove filesystems that belong to no account
    /// (left by a crash in the middle of a create or a delete).
    pub fn open(cfg: ServiceConfig) -> Result<Arc<Service>, String> {
        if cfg.admin_key.len() < 16 {
            return Err("the admin key must be at least 16 characters".into());
        }
        let host = Host::open(&cfg.data, cfg.host.clone())?;
        let dir = cfg.data.join("server");
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
        let reg_path = dir.join("registry.log");
        let mut st = State::default();
        for j in read_lines(&reg_path) {
            let id = j.get("id").and_then(Json::as_str).unwrap_or("").to_string();
            match j.get("op").and_then(Json::as_str) {
                Some("account") => {
                    let mut settings = cfg.defaults;
                    if let Some(s) = j.get("settings") {
                        let _ = settings.apply(s);
                    }
                    let key_hash = j.get("key_hash").and_then(Json::as_str).unwrap_or("").to_string();
                    st.by_key.insert(key_hash.clone(), id.clone());
                    st.accounts.insert(id, Account { key_hash, settings, fss: BTreeSet::new(), running: Arc::new(AtomicU32::new(0)) });
                }
                Some("settings") => {
                    if let (Some(a), Some(s)) = (st.accounts.get_mut(&id), j.get("settings")) {
                        let mut settings = cfg.defaults;
                        let _ = settings.apply(s);
                        a.settings = settings;
                    }
                }
                Some("drop_account") => {
                    if let Some(a) = st.accounts.remove(&id) {
                        st.by_key.remove(&a.key_hash);
                        for f in a.fss {
                            st.owner.remove(&f);
                        }
                    }
                }
                Some("fs") => {
                    let acc = j.get("account").and_then(Json::as_str).unwrap_or("").to_string();
                    if let Some(a) = st.accounts.get_mut(&acc) {
                        a.fss.insert(id.clone());
                        st.owner.insert(id, acc);
                    }
                }
                Some("drop_fs") => {
                    if let Some(acc) = st.owner.remove(&id) {
                        if let Some(a) = st.accounts.get_mut(&acc) {
                            a.fss.remove(&id);
                        }
                    }
                }
                _ => {}
            }
        }
        // Written again whole: one line per account and filesystem.
        let mut lines = Vec::new();
        let mut ids: Vec<&String> = st.accounts.keys().collect();
        ids.sort();
        for id in ids {
            let a = &st.accounts[id];
            lines.push(Json::obj().with("op", "account").with("id", id.as_str()).with("key_hash", a.key_hash.as_str()).with("settings", a.settings.to_json()));
            for f in &a.fss {
                lines.push(Json::obj().with("op", "fs").with("id", f.as_str()).with("account", id.as_str()));
            }
        }
        write_whole(&reg_path, &lines).map_err(|e| format!("{}: {}", reg_path.display(), e))?;
        // Filesystems no account owns.
        for id in host.list() {
            if !st.owner.contains_key(&id) {
                log(&format!("removing filesystem {}, which no account owns", id));
                let _ = host.delete(&id);
            }
        }
        // Usage.
        let usage_path = dir.join("usage.log");
        let mut usage = Usage::default();
        for j in read_lines(&usage_path) {
            let t = j.get("totals").map(Totals::from_json).unwrap_or_default();
            if let Some(f) = j.get("fs").and_then(Json::as_str) {
                let acc = j.get("account").and_then(Json::as_str).unwrap_or("").to_string();
                usage.fs.insert(f.to_string(), (acc, t));
            } else if let Some(a) = j.get("gone_of").and_then(Json::as_str) {
                usage.gone.insert(a.to_string(), t);
            }
        }
        // Totals of filesystems deleted while the usage was not yet
        // written go to their accounts.
        let lost: Vec<String> = usage.fs.keys().filter(|f| !st.owner.contains_key(*f)).cloned().collect();
        for f in lost {
            let (acc, t) = usage.fs.remove(&f).unwrap();
            if st.accounts.contains_key(&acc) {
                usage.gone.entry(acc).or_default().add(&t);
            }
        }
        usage.gone.retain(|a, _| st.accounts.contains_key(a));
        let mut lines = Vec::new();
        for (f, (a, t)) in &usage.fs {
            lines.push(Json::obj().with("fs", f.as_str()).with("account", a.as_str()).with("totals", t.to_json()));
        }
        for (a, t) in &usage.gone {
            lines.push(Json::obj().with("gone_of", a.as_str()).with("totals", t.to_json()));
        }
        write_whole(&usage_path, &lines).map_err(|e| format!("{}: {}", usage_path.display(), e))?;
        let stopping = Arc::new(AtomicBool::new(false));
        let svc = Arc::new(Service {
            registry: Mutex::new(open_append(&reg_path).map_err(|e| e.to_string())?),
            usage_file: Mutex::new(open_append(&usage_path).map_err(|e| e.to_string())?),
            slots: Arc::new(Slots { state: Mutex::new((0, 0)), cv: Condvar::new(), max: cfg.workers.max(1), queue: cfg.queue }),
            watcher: Watcher::start(stopping.clone()),
            cfg,
            host,
            state: RwLock::new(st),
            usage: Mutex::new(usage),
            sessions: Mutex::new(HashMap::new()),
            start: Instant::now(),
            stopping,
            connections: AtomicUsize::new(0),
            conns: Mutex::new(HashMap::new()),
            next_conn: AtomicU64::new(0),
        });
        let weak = Arc::downgrade(&svc);
        std::thread::Builder::new()
            .name("qdb-housekeeping".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(1));
                let Some(svc) = weak.upgrade() else { return };
                if svc.stopping.load(Relaxed) {
                    return;
                }
                svc.housekeeping();
            })
            .expect("spawn housekeeping");
        Ok(svc)
    }

    pub fn host(&self) -> &Arc<Host> {
        &self.host
    }

    /// Workers no unit is using.
    pub fn idle_workers(&self) -> usize {
        self.slots.idle()
    }

    fn tick(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    fn housekeeping(&self) {
        let now = self.tick();
        let idle = self.cfg.session_idle.as_millis() as u64;
        lock(&self.sessions).retain(|_, s| now.saturating_sub(s.used.load(Relaxed)) < idle);
        if let Err(e) = self.flush_usage() {
            log(&format!("usage could not be written: {}", e));
        }
    }

    /// Append the usage totals that changed.
    pub fn flush_usage(&self) -> io::Result<()> {
        let lines = {
            let mut u = lock(&self.usage);
            let mut lines = Vec::new();
            for f in std::mem::take(&mut u.dirty_fs) {
                if let Some((a, t)) = u.fs.get(&f) {
                    lines.push(Json::obj().with("fs", f.as_str()).with("account", a.as_str()).with("totals", t.to_json()));
                }
            }
            for a in std::mem::take(&mut u.dirty_gone) {
                if let Some(t) = u.gone.get(&a) {
                    lines.push(Json::obj().with("gone_of", a.as_str()).with("totals", t.to_json()));
                }
            }
            lines
        };
        if lines.is_empty() {
            return Ok(());
        }
        let mut text = String::new();
        for j in &lines {
            j.write(&mut text);
            text.push('\n');
        }
        let mut f = lock(&self.usage_file);
        f.write_all(text.as_bytes())?;
        f.sync_data()
    }

    fn note_usage(&self, fs: &str, account: &str, end: &UnitEnd) {
        let mut u = lock(&self.usage);
        let e = u.fs.entry(fs.to_string()).or_insert_with(|| (account.to_string(), Totals::default()));
        e.1.add(&Totals { units: 1, stopped: (end.ending != Ending::Ran) as u64, cost: end.cost });
        u.dirty_fs.insert(fs.to_string());
    }

    fn usage_of_fs(&self, fs: &str) -> Totals {
        lock(&self.usage).fs.get(fs).map(|e| e.1).unwrap_or_default()
    }

    fn usage_of_account(&self, account: &str, fss: &BTreeSet<String>) -> Totals {
        let u = lock(&self.usage);
        let mut t = u.gone.get(account).copied().unwrap_or_default();
        for f in fss {
            if let Some(e) = u.fs.get(f) {
                t.add(&e.1);
            }
        }
        t
    }

    /// Stop taking requests, stop every unit, close every filesystem and
    /// write the usage.
    pub fn shutdown(&self) {
        if self.stopping.swap(true, Relaxed) {
            return;
        }
        self.host.shutdown();
        if let Err(e) = self.flush_usage() {
            log(&format!("usage could not be written: {}", e));
        }
        for (_, c) in lock(&self.conns).drain() {
            let _ = c.shutdown(std::net::Shutdown::Both);
        }
    }

    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Relaxed)
    }
}

// --------------------------------------------------------------- serving

/// Accept connections until the service is stopped, each on a thread of
/// its own.
pub fn serve(svc: Arc<Service>, listener: TcpListener) {
    listener.set_nonblocking(true).expect("a listening socket");
    loop {
        if svc.is_stopping() {
            return;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_nodelay(true);
                if svc.connections.load(Relaxed) >= svc.cfg.max_connections {
                    let mut s = stream;
                    let _ = respond(&mut s, 503, &error_body("too many connections"), false);
                    continue;
                }
                svc.connections.fetch_add(1, Relaxed);
                let svc2 = svc.clone();
                let spawned = std::thread::Builder::new().name("qdb-conn".into()).stack_size(64 << 20).spawn(move || {
                    connection(&svc2, stream);
                    svc2.connections.fetch_sub(1, Relaxed);
                });
                if spawned.is_err() {
                    svc.connections.fetch_sub(1, Relaxed);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(2)),
            Err(e) => {
                log(&format!("accept failed: {}", e));
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn error_body(msg: &str) -> Vec<u8> {
    Json::obj().with("error", msg).to_string().into_bytes()
}

fn respond(w: &mut TcpStream, status: u16, body: &[u8], keep_alive: bool) -> io::Result<()> {
    http::write_response(w, status, "application/json", body, keep_alive)
}

fn connection(svc: &Arc<Service>, stream: TcpStream) {
    let Ok(read_half) = stream.try_clone() else { return };
    let n = svc.next_conn.fetch_add(1, Relaxed);
    if let Ok(c) = stream.try_clone() {
        lock(&svc.conns).insert(n, c);
    }
    struct Forget<'a>(&'a Service, u64);
    impl Drop for Forget<'_> {
        fn drop(&mut self) {
            lock(&self.0.conns).remove(&self.1);
        }
    }
    let _forget = Forget(svc, n);
    if svc.is_stopping() {
        return;
    }
    let mut r = BufReader::with_capacity(64 << 10, read_half);
    let mut w = stream;
    // A connection that says nothing for this long is closed.
    let _ = w.set_read_timeout(Some(Duration::from_secs(300)));
    loop {
        let req = match http::read_request(&mut r, svc.cfg.max_request) {
            Ok(req) => req,
            Err(ReadError::Closed) | Err(ReadError::Io(_)) => return,
            Err(ReadError::Malformed(m)) => {
                let _ = respond(&mut w, 400, &error_body(&m), false);
                return;
            }
            Err(ReadError::TooLarge(n)) => {
                let _ = respond(&mut w, 413, &error_body(&format!("the request body is {} bytes; the most is {}", n, svc.cfg.max_request)), false);
                return;
            }
        };
        let keep = req.keep_alive && !svc.is_stopping();
        let result = handle(svc, &req, &mut w, keep);
        match result {
            Ok(true) if keep => {}
            _ => return,
        }
    }
}

/// An answer that is not streamed.
struct Answer {
    status: u16,
    body: Json,
}

fn ok(body: Json) -> Answer {
    Answer { status: 200, body }
}

fn fail(status: u16, msg: &str) -> Answer {
    Answer { status, body: Json::obj().with("error", msg) }
}

fn not_found() -> Answer {
    fail(404, "no such filesystem")
}

/// Handle one request. Ok(true) if the connection can take another.
fn handle(svc: &Arc<Service>, req: &http::Request, w: &mut TcpStream, keep: bool) -> io::Result<bool> {
    let segs: Vec<&str> = req.path.trim_end_matches('/').split('/').skip(1).collect();
    let answer = route(svc, req, &segs, w, keep);
    match answer {
        Some(a) => {
            respond(w, a.status, a.body.to_string().as_bytes(), keep)?;
            Ok(true)
        }
        // Streamed: already written.
        None => Ok(true),
    }
}

enum Who {
    Admin,
    Account(String),
}

fn who(svc: &Service, req: &http::Request) -> Option<Who> {
    let auth = req.header("authorization")?;
    let key = auth.strip_prefix("Bearer ").or_else(|| auth.strip_prefix("bearer "))?.trim();
    if same_secret(key.as_bytes(), svc.cfg.admin_key.as_bytes()) {
        return Some(Who::Admin);
    }
    let h = key_hash(key);
    svc.state.read().unwrap_or_else(|e| e.into_inner()).by_key.get(&h).cloned().map(Who::Account)
}

fn body_json(req: &http::Request) -> Result<Json, Answer> {
    if req.body.iter().all(|c| c.is_ascii_whitespace()) {
        return Ok(Json::obj());
    }
    match Json::parse(&req.body) {
        Ok(j) if j.is_obj() => Ok(j),
        Ok(_) => Err(fail(400, "the body must be a JSON object")),
        Err(e) => Err(fail(400, &format!("the body is not JSON: {}", e))),
    }
}

fn route(svc: &Arc<Service>, req: &http::Request, segs: &[&str], w: &mut TcpStream, keep: bool) -> Option<Answer> {
    let m = req.method.as_str();
    if segs.first() != Some(&"v1") {
        return Some(fail(404, "no such request"));
    }
    if segs.get(1) == Some(&"health") && segs.len() == 2 {
        return Some(if m == "GET" { ok(Json::obj().with("ok", true).with("stopping", svc.is_stopping())) } else { fail(405, "use GET") });
    }
    if svc.is_stopping() {
        return Some(fail(503, "the server is shutting down"));
    }
    let Some(who) = who(svc, req) else { return Some(fail(401, "no valid key: send Authorization: Bearer KEY")) };
    match (segs.get(1).copied(), &who) {
        (Some("accounts"), Who::Admin) => Some(accounts(svc, req, m, &segs[2..])),
        (Some("accounts"), Who::Account(_)) => Some(fail(401, "this request needs the admin key")),
        (Some("filesystems"), Who::Account(acc)) => filesystems(svc, req, m, &segs[2..], acc, w, keep),
        (Some("filesystems"), Who::Admin) => Some(fail(401, "this request needs an account key")),
        _ => Some(fail(404, "no such request")),
    }
}

fn accounts(svc: &Service, req: &http::Request, m: &str, rest: &[&str]) -> Answer {
    let body = match body_json(req) {
        Ok(b) => b,
        Err(a) => return a,
    };
    match (m, rest) {
        ("POST", []) => {
            let mut settings = svc.cfg.defaults;
            if let Err(e) = settings.apply(&body) {
                return fail(400, &e);
            }
            let id = host::new_id();
            let mut kb = [0u8; 32];
            host::random_bytes(&mut kb);
            let key = format!("qdbk_{}", hex(&kb));
            let h = key_hash(&key);
            let line = Json::obj().with("op", "account").with("id", id.as_str()).with("key_hash", h.as_str()).with("settings", settings.to_json());
            if let Err(e) = append_line(&svc.registry, &line, true) {
                return fail(500, &format!("the account could not be stored: {}", e));
            }
            let mut st = svc.state.write().unwrap_or_else(|e| e.into_inner());
            st.by_key.insert(h.clone(), id.clone());
            st.accounts.insert(id.clone(), Account { key_hash: h, settings, fss: BTreeSet::new(), running: Arc::new(AtomicU32::new(0)) });
            Answer { status: 201, body: Json::obj().with("account", id).with("key", key).with("settings", settings.to_json()) }
        }
        ("PATCH", [id]) => {
            let mut st = svc.state.write().unwrap_or_else(|e| e.into_inner());
            let Some(a) = st.accounts.get_mut(*id) else { return fail(404, "no such account") };
            let mut settings = a.settings;
            if let Err(e) = settings.apply(&body) {
                return fail(400, &e);
            }
            let line = Json::obj().with("op", "settings").with("id", *id).with("settings", settings.to_json());
            if let Err(e) = append_line(&svc.registry, &line, true) {
                return fail(500, &format!("the settings could not be stored: {}", e));
            }
            a.settings = settings;
            ok(Json::obj().with("account", *id).with("settings", settings.to_json()))
        }
        ("GET", [id]) => {
            let st = svc.state.read().unwrap_or_else(|e| e.into_inner());
            let Some(a) = st.accounts.get(*id) else { return fail(404, "no such account") };
            let usage = svc.usage_of_account(id, &a.fss);
            ok(Json::obj().with("account", *id).with("filesystems", a.fss.len()).with("settings", a.settings.to_json()).with("usage", usage.to_json()))
        }
        ("DELETE", [id]) => {
            let fss = {
                let st = svc.state.read().unwrap_or_else(|e| e.into_inner());
                let Some(a) = st.accounts.get(*id) else { return fail(404, "no such account") };
                a.fss.clone()
            };
            let line = Json::obj().with("op", "drop_account").with("id", *id);
            if let Err(e) = append_line(&svc.registry, &line, true) {
                return fail(500, &format!("the account could not be deleted: {}", e));
            }
            {
                let mut st = svc.state.write().unwrap_or_else(|e| e.into_inner());
                if let Some(a) = st.accounts.remove(*id) {
                    st.by_key.remove(&a.key_hash);
                    for f in &a.fss {
                        st.owner.remove(f);
                    }
                }
            }
            lock(&svc.sessions).retain(|_, s| s.account != *id);
            for f in &fss {
                let _ = svc.host.delete(f);
            }
            {
                let mut u = lock(&svc.usage);
                for f in &fss {
                    u.fs.remove(f);
                }
                u.gone.remove(*id);
            }
            ok(Json::obj().with("account", *id).with("deleted_filesystems", fss.len()))
        }
        _ => fail(405, "no such request"),
    }
}

/// The account's own filesystem, or None: another account's is answered
/// as one that does not exist.
fn owned(svc: &Service, acc: &str, fs: &str) -> bool {
    svc.state.read().unwrap_or_else(|e| e.into_inner()).owner.get(fs).is_some_and(|a| a == acc)
}

fn filesystems(svc: &Arc<Service>, req: &http::Request, m: &str, rest: &[&str], acc: &str, w: &mut TcpStream, keep: bool) -> Option<Answer> {
    let answer = match (m, rest) {
        ("POST", []) => {
            {
                let st = svc.state.read().unwrap_or_else(|e| e.into_inner());
                let a = &st.accounts[acc];
                if a.fss.len() as u64 >= a.settings.max_fs {
                    return Some(fail(507, &format!("the account has {} filesystems, its quota", a.settings.max_fs)));
                }
            }
            let id = match svc.host.create() {
                Ok(id) => id,
                Err(e) => return Some(fail(500, &e.to_string())),
            };
            let line = Json::obj().with("op", "fs").with("id", id.as_str()).with("account", acc);
            if let Err(e) = append_line(&svc.registry, &line, true) {
                let _ = svc.host.delete(&id);
                return Some(fail(500, &format!("the filesystem could not be stored: {}", e)));
            }
            let mut st = svc.state.write().unwrap_or_else(|e| e.into_inner());
            st.owner.insert(id.clone(), acc.to_string());
            if let Some(a) = st.accounts.get_mut(acc) {
                a.fss.insert(id.clone());
            }
            Answer { status: 201, body: Json::obj().with("filesystem", id) }
        }
        ("GET", []) => {
            let st = svc.state.read().unwrap_or_else(|e| e.into_inner());
            let a = &st.accounts[acc];
            let usage = svc.usage_of_account(acc, &a.fss);
            ok(Json::obj().with("filesystems", a.fss.iter().map(|f| Json::from(f.as_str())).collect::<Vec<_>>()).with("usage", usage.to_json()).with("settings", a.settings.to_json()))
        }
        ("GET", [fs]) => {
            if !owned(svc, acc, fs) {
                return Some(not_found());
            }
            match svc.host.info(fs) {
                Ok(i) => ok(Json::obj()
                    .with("filesystem", *fs)
                    .with("text_bytes", i.text_bytes)
                    .with("files", i.files)
                    .with("folders", i.folders)
                    .with("disk_bytes", i.disk_bytes)
                    .with("usage", svc.usage_of_fs(fs).to_json())),
                Err(HostError::NotFound) => not_found(),
                Err(e) => fail(500, &e.to_string()),
            }
        }
        ("DELETE", [fs]) => {
            if !owned(svc, acc, fs) {
                return Some(not_found());
            }
            let line = Json::obj().with("op", "drop_fs").with("id", *fs);
            if let Err(e) = append_line(&svc.registry, &line, true) {
                return Some(fail(500, &format!("the filesystem could not be deleted: {}", e)));
            }
            {
                let mut st = svc.state.write().unwrap_or_else(|e| e.into_inner());
                st.owner.remove(*fs);
                if let Some(a) = st.accounts.get_mut(acc) {
                    a.fss.remove(*fs);
                }
            }
            lock(&svc.sessions).retain(|_, s| s.fs != *fs);
            let _ = svc.host.delete(fs);
            let mut u = lock(&svc.usage);
            if let Some((a, t)) = u.fs.remove(*fs) {
                u.gone.entry(a.clone()).or_default().add(&t);
                u.dirty_gone.insert(a);
            }
            ok(Json::obj().with("filesystem", *fs).with("deleted", true))
        }
        ("POST", [fs, "sessions"]) => {
            if !owned(svc, acc, fs) {
                return Some(not_found());
            }
            let id = format!("s_{}", host::new_id());
            let slot = Arc::new(SessionSlot { fs: fs.to_string(), account: acc.to_string(), sess: Mutex::new(Session::new()), used: AtomicU64::new(svc.tick()) });
            lock(&svc.sessions).insert(id.clone(), slot);
            Answer { status: 201, body: Json::obj().with("session", id).with("idle_timeout_ms", svc.cfg.session_idle.as_millis() as u64) }
        }
        ("DELETE", [fs, "sessions", sid]) => {
            let mut map = lock(&svc.sessions);
            match map.get(*sid) {
                Some(s) if s.fs == *fs && s.account == acc => {
                    map.remove(*sid);
                    ok(Json::obj().with("session", *sid).with("ended", true))
                }
                _ => fail(404, "no such session"),
            }
        }
        ("POST", [fs, "run"]) => return run(svc, req, acc, fs, w, keep),
        _ => fail(405, "no such request"),
    };
    Some(answer)
}

/// Standard output or error of a unit that is not streamed: kept up to
/// the cap. Past it, standard output asks the unit to stop reading, as
/// head does.
struct Capped {
    buf: Vec<u8>,
    cap: usize,
    cut: bool,
}

impl Sink for Capped {
    fn write(&mut self, data: &[u8]) -> bool {
        let room = self.cap - self.buf.len();
        if data.len() > room {
            self.buf.extend_from_slice(&data[..room]);
            self.cut = true;
            return false;
        }
        self.buf.extend_from_slice(data);
        true
    }
}

/// Put bytes in a JSON object: as a string when they are UTF-8, as
/// base64 under `NAME_base64` when not.
fn put_bytes(j: &mut Json, name: &str, bytes: &[u8]) {
    match std::str::from_utf8(bytes) {
        Ok(s) => j.set(name, s),
        Err(_) => j.set(&format!("{}_base64", name), base64(bytes)),
    }
}

fn ending_name(e: &Ending) -> Option<&'static str> {
    match e {
        Ending::Ran => None,
        Ending::Limit => Some("command_limit"),
        Ending::Stopped(r) => Some(r.name()),
        Ending::Quota(_) => Some("quota"),
        Ending::Damaged(..) => Some("damage"),
        Ending::Fault(_) => Some("fault"),
    }
}

fn cost_json(c: &Cost) -> Json {
    let mut j = Json::obj();
    for (k, v) in c.fields() {
        j.set(k, v);
    }
    j
}

/// Output streamed as it is produced: lines of JSON in chunks, each
/// `{"stdout": ...}` or `{"stderr": ...}` (or `..._base64`), then a last
/// line with the exit status.
struct Streamed<'a> {
    w: &'a mut TcpStream,
    name: &'static str,
    held: Vec<u8>,
    failed: bool,
    stop: Arc<Stop>,
}

impl Streamed<'_> {
    fn send(&mut self, end: bool) {
        if self.failed || self.held.is_empty() {
            return;
        }
        // Whole characters only, unless the output ends here.
        let upto = match std::str::from_utf8(&self.held) {
            Ok(_) => self.held.len(),
            Err(e) if e.error_len().is_none() && !end => e.valid_up_to(),
            Err(_) => self.held.len(),
        };
        if upto == 0 {
            return;
        }
        let part: Vec<u8> = self.held.drain(..upto).collect();
        let mut j = Json::obj();
        put_bytes(&mut j, self.name, &part);
        let mut line = j.to_string();
        line.push('\n');
        if http::write_chunk(self.w, line.as_bytes()).is_err() {
            self.failed = true;
            self.stop.request(Reason::Gone);
        }
    }
}

impl Sink for Streamed<'_> {
    fn write(&mut self, data: &[u8]) -> bool {
        self.held.extend_from_slice(data);
        if self.held.len() >= 64 << 10 {
            self.send(false);
        }
        !self.failed
    }
}

/// Standard error while streaming: gathered, and sent after standard
/// output has been flushed at the end (and whenever output is sent).
struct ErrHeld {
    buf: Vec<u8>,
    cap: usize,
    cut: bool,
}

impl Sink for ErrHeld {
    fn write(&mut self, data: &[u8]) -> bool {
        let room = self.cap.saturating_sub(self.buf.len());
        self.buf.extend_from_slice(&data[..data.len().min(room)]);
        if data.len() > room {
            self.cut = true;
        }
        true
    }
}

fn run(svc: &Arc<Service>, req: &http::Request, acc: &str, fs: &str, w: &mut TcpStream, keep: bool) -> Option<Answer> {
    let body = match body_json(req) {
        Ok(b) => b,
        Err(a) => return Some(a),
    };
    if !owned(svc, acc, fs) {
        return Some(not_found());
    }
    let Some(command) = body.get("command").and_then(Json::as_str) else { return Some(fail(400, "command must be a string")) };
    let stdin: Option<Vec<u8>> = match (body.get("stdin"), body.get("stdin_base64")) {
        (Some(Json::Str(s)), None) => Some(s.as_bytes().to_vec()),
        (None, Some(Json::Str(s))) => match unbase64(s) {
            Some(b) => Some(b),
            None => return Some(fail(400, "stdin_base64 is not base64")),
        },
        (None, None) => None,
        _ => return Some(fail(400, "give stdin as a string, or stdin_base64, not both")),
    };
    let stream = body.get("stream").and_then(Json::as_bool).unwrap_or(false) || req.query("stream").is_some_and(|v| v == "1" || v == "true");
    for (k, _) in body.fields() {
        if !matches!(k.as_str(), "command" | "stdin" | "stdin_base64" | "session" | "limits" | "stream") {
            return Some(fail(400, &format!("unknown field {}", k)));
        }
    }
    let (settings, running) = {
        let st = svc.state.read().unwrap_or_else(|e| e.into_inner());
        let a = &st.accounts[acc];
        (a.settings, a.running.clone())
    };
    // Tighter limits only.
    let mut limits = settings.limits;
    if let Some(l) = body.get("limits") {
        let mut asked = settings;
        if let Err(e) = asked.apply(&Json::obj().with("limits", l.clone())) {
            return Some(fail(400, &e));
        }
        limits.time = limits.time.min(asked.limits.time);
        limits.commands = limits.commands.min(asked.limits.commands);
        limits.scratch = limits.scratch.min(asked.limits.scratch);
        limits.memory = limits.memory.min(asked.limits.memory);
    }
    let session = match body.get("session") {
        None | Some(Json::Null) => None,
        Some(Json::Str(sid)) => match lock(&svc.sessions).get(sid.as_str()) {
            Some(s) if s.fs == fs && s.account == acc => Some(s.clone()),
            _ => return Some(fail(404, "no such session")),
        },
        Some(_) => return Some(fail(400, "session must be a string")),
    };
    // At most so many units of one account at once.
    if running.fetch_add(1, Relaxed) >= settings.max_running {
        running.fetch_sub(1, Relaxed);
        return Some(fail(429, &format!("the account has {} units running, the most it may", settings.max_running)));
    }
    struct Dec(Arc<AtomicU32>);
    impl Drop for Dec {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Relaxed);
        }
    }
    let _dec = Dec(running);
    let stop = Stop::new();
    let fd = {
        use std::os::unix::io::AsRawFd;
        w.as_raw_fd()
    };
    let watched = svc.watcher.add(fd, &stop);
    struct Unwatch<'a>(&'a Watcher, u64);
    impl Drop for Unwatch<'_> {
        fn drop(&mut self) {
            self.0.remove(self.1);
        }
    }
    let _unwatch = Unwatch(&svc.watcher, watched);
    let Some(_slot) = svc.slots.take(&|| stop.requested().is_some()) else {
        return Some(fail(429, "too many units are waiting; try again"));
    };
    // A search in the unit may borrow idle workers.
    let _lending = crate::par::lend(svc.slots.clone());
    let mut fresh = Session::new();
    let mut held_session;
    let sess: &mut Session = match &session {
        Some(s) => {
            held_session = lock(&s.sess);
            &mut held_session
        }
        None => &mut fresh,
    };
    let request = Request { line: command.as_bytes(), stdin: stdin.as_deref(), quotas: settings.quotas, limits, stop: stop.clone() };
    let kind = crate::engine::classify(command.as_bytes());
    if stream {
        if http::write_chunked_head(w, 200, "application/x-ndjson", keep).is_err() {
            return None;
        }
        let mut err = ErrHeld { buf: Vec::new(), cap: svc.cfg.output_cap, cut: false };
        let w2 = w.try_clone();
        let Ok(mut w2) = w2 else { return None };
        let mut out = Streamed { w: &mut w2, name: "stdout", held: Vec::new(), failed: false, stop: stop.clone() };
        let result = svc.host.run(fs, &request, sess, &mut out, &mut err);
        out.send(true);
        if !err.buf.is_empty() {
            out.name = "stderr";
            out.held = std::mem::take(&mut err.buf);
            out.send(true);
        }
        let mut last = match &result {
            Ok(end) => end_json(end, kind, err.cut),
            Err(e) => host_error_json(e),
        };
        last.set("done", true);
        let mut line = last.to_string();
        line.push('\n');
        if let Ok(end) = &result {
            svc.note_usage(fs, acc, end);
        }
        if let Some(s) = &session {
            s.used.store(svc.tick(), Relaxed);
        }
        let _ = http::write_chunk(w, line.as_bytes()).and_then(|_| http::write_last_chunk(w));
        return None;
    }
    let mut out = Capped { buf: Vec::new(), cap: svc.cfg.output_cap, cut: false };
    let mut err = Capped { buf: Vec::new(), cap: svc.cfg.output_cap, cut: false };
    let result = svc.host.run(fs, &request, sess, &mut out, &mut err);
    if let Some(s) = &session {
        s.used.store(svc.tick(), Relaxed);
    }
    Some(match result {
        Ok(end) => {
            svc.note_usage(fs, acc, &end);
            let mut j = end_json(&end, kind, out.cut || err.cut);
            put_bytes(&mut j, "stdout", &out.buf);
            put_bytes(&mut j, "stderr", &err.buf);
            ok(j)
        }
        Err(HostError::NotFound) => not_found(),
        Err(e) => {
            let mut j = host_error_json(&e);
            put_bytes(&mut j, "stdout", b"");
            put_bytes(&mut j, "stderr", e.to_string().as_bytes());
            ok(j)
        }
    })
}

fn end_json(end: &UnitEnd, kind: crate::engine::Kind, truncated: bool) -> Json {
    Json::obj()
        .with("exit_status", end.code)
        .with("stopped", ending_name(&end.ending))
        .with("truncated", truncated)
        .with("kind", if kind == crate::engine::Kind::Reader { "reader" } else { "writer" })
        .with("cost", cost_json(&end.cost))
}

/// A unit that could not run, said as a stopped unit is: what stopped it
/// while it waited, or the server going down.
fn host_error_json(e: &HostError) -> Json {
    let reason = match e {
        HostError::Stopped(r) => r.name(),
        HostError::ShuttingDown => "shutdown",
        HostError::NotFound => "deleted",
        HostError::Failed(_) => "fault",
    };
    Json::obj().with("exit_status", 2).with("stopped", reason).with("truncated", false).with("cost", cost_json(&Cost::ZERO))
}
