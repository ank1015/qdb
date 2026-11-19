//! Many filesystems in one process.
//!
//! A host keeps filesystems under one data directory, each a database
//! directory of its own, named by an ID the host makes: 32 random hex
//! digits. It creates, opens, closes and deletes them, and runs units on
//! them from any number of threads:
//!
//! - Units that only read run together on one filesystem; a unit that
//!   may write runs alone on it (`engine::classify`). Callers are let in
//!   fairly, in the order they came (`gate::Gate`).
//! - A filesystem is opened when a unit needs it and closed when it has
//!   been idle for a while, or when too many are open or they hold too
//!   much memory, the least recently used first.
//! - A unit can be stopped (its `unit::Stop`): at its deadline, which the
//!   host watches, when its caller goes away, at shutdown, when its
//!   filesystem is deleted. A stopped unit leaves nothing.
//! - A unit in which the engine fails is stopped; its filesystem is
//!   closed and opened again from disk, which recovers it, and nothing
//!   else notices.
//!
//! ```text
//! DATA/LOCK                      held while a host has the directory
//! DATA/fs/ab/ab12...ef/          one filesystem: a database directory
//! DATA/trash/                    filesystems being deleted
//! ```

use crate::engine::{self, classify, run_unit_with, Access, Ending, Kind, Sink, UnitEnd};
use crate::gate::{Gate, Pass};
use crate::session::Session;
use crate::tree::{Config, Db};
use crate::unit::{self, Reason, Stop};
use crate::vfs::lock;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Condvar, Mutex, RwLock, Weak};
use std::time::{Duration, Instant};

/// How a host runs.
#[derive(Clone, Debug)]
pub struct HostConfig {
    /// The engine's settings for every filesystem.
    pub db: Config,
    /// The most filesystems open at once.
    pub max_open: usize,
    /// The most memory open filesystems may hold together, about.
    pub max_open_bytes: u64,
    /// A filesystem no unit has used for this long is closed.
    pub idle_close: Duration,
    /// Under sync always, let units share syncs (`group`).
    pub group_commit: bool,
}

impl Default for HostConfig {
    fn default() -> HostConfig {
        HostConfig { db: Config::default(), max_open: 1024, max_open_bytes: 2 << 30, idle_close: Duration::from_secs(60), group_commit: true }
    }
}

/// What one filesystem may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quotas {
    pub text: u64,
    pub nodes: u64,
    pub disk: u64,
}

impl Default for Quotas {
    fn default() -> Quotas {
        Quotas { text: 10 << 30, nodes: 1_000_000, disk: 20 << 30 }
    }
}

/// What one unit may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub time: Duration,
    pub commands: u64,
    pub scratch: u64,
    /// Bytes of memory, 0 for no limit.
    pub memory: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits { time: Duration::from_secs(30), commands: 1_000_000, scratch: 1 << 30, memory: 1 << 30 }
    }
}

/// Why a host could not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    /// No filesystem has that ID (or the ID is not one the host makes).
    NotFound,
    /// The unit was stopped while it waited for its turn.
    Stopped(Reason),
    /// The host is shutting down.
    ShuttingDown,
    /// Storage failed, or a filesystem could not be opened.
    Failed(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HostError::NotFound => write!(f, "no such filesystem"),
            HostError::Stopped(r) => write!(f, "{}", r.message()),
            HostError::ShuttingDown => write!(f, "the server is shutting down"),
            HostError::Failed(m) => write!(f, "{}", m),
        }
    }
}

/// What a filesystem holds, for an answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FsInfo {
    pub text_bytes: u64,
    pub files: u64,
    pub folders: u64,
    pub disk_bytes: u64,
    pub open: bool,
}

/// One unit to run.
pub struct Request<'a> {
    pub line: &'a [u8],
    /// Standard input for the unit.
    pub stdin: Option<&'a [u8]>,
    pub quotas: Quotas,
    pub limits: Limits,
    /// Set by whoever may stop the unit: the caller going away, shutdown.
    pub stop: Arc<Stop>,
}

struct Fs {
    path: PathBuf,
    gate: Gate,
    db: RwLock<Option<Db>>,
    /// When a unit last used it, in host ticks (milliseconds).
    used: AtomicU64,
    /// Memory it holds while open, about.
    mem: AtomicU64,
    deleted: AtomicBool,
    running: Mutex<Vec<Arc<Stop>>>,
}

pub struct Host {
    dir: PathBuf,
    cfg: HostConfig,
    /// The data directory's lock, let go at shutdown.
    dir_lock: Mutex<Option<std::fs::File>>,
    fss: Mutex<HashMap<String, Arc<Fs>>>,
    start: Instant,
    closing: AtomicBool,
    watch: Arc<Watchdog>,
    janitor: Mutex<Option<std::thread::JoinHandle<()>>>,
    /// The janitor sleeps on this; shutdown wakes it.
    quit: Arc<(Mutex<bool>, Condvar)>,
}

/// True for an ID the host could have made: 32 lowercase hex digits.
pub fn is_fs_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// Random bytes from the operating system.
pub fn random_bytes(out: &mut [u8]) {
    let mut done = 0;
    while done < out.len() {
        // SAFETY: getrandom writes at most the length given into the buffer.
        let n = unsafe { libc::getrandom(out[done..].as_mut_ptr() as *mut libc::c_void, out.len() - done, 0) };
        if n > 0 {
            done += n as usize;
        } else if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            panic!("no randomness from the operating system: {}", std::io::Error::last_os_error());
        }
    }
}

/// A new random ID of 128 bits, as hex.
pub fn new_id() -> String {
    let mut b = [0u8; 16];
    random_bytes(&mut b);
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

impl Host {
    /// Take the data directory, making it if need be. Refused if another
    /// host (in this process or another) has it. Stopping a unit unwinds
    /// it, so the panic hook is set to say nothing of those unwinds.
    pub fn open(dir: &Path, cfg: HostConfig) -> Result<Arc<Host>, String> {
        use std::os::unix::io::AsRawFd;
        unit::quiet_panics();
        std::fs::create_dir_all(dir.join("fs")).map_err(|e| format!("{}: {}", dir.display(), e))?;
        std::fs::create_dir_all(dir.join("trash")).map_err(|e| format!("{}: {}", dir.display(), e))?;
        let lockfile = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(dir.join("LOCK")).map_err(|e| format!("{}: {}", dir.join("LOCK").display(), e))?;
        // SAFETY: flock on a descriptor we own.
        if unsafe { libc::flock(lockfile.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(format!("{}: another server is using this data directory", dir.display()));
        }
        // What a delete left half done.
        if let Ok(rd) = std::fs::read_dir(dir.join("trash")) {
            for e in rd.flatten() {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
        let quit = Arc::new((Mutex::new(false), Condvar::new()));
        let host = Arc::new(Host {
            dir: dir.to_path_buf(),
            cfg,
            dir_lock: Mutex::new(Some(lockfile)),
            fss: Mutex::new(HashMap::new()),
            start: Instant::now(),
            closing: AtomicBool::new(false),
            watch: Watchdog::start(),
            janitor: Mutex::new(None),
            quit: quit.clone(),
        });
        let weak = Arc::downgrade(&host);
        let every = (host.cfg.idle_close / 4).clamp(Duration::from_millis(10), Duration::from_secs(1));
        let j = std::thread::Builder::new()
            .name("qdb-janitor".into())
            .spawn(move || loop {
                {
                    let (m, c) = &*quit;
                    let g = lock(m);
                    let (g, _) = c.wait_timeout(g, every).unwrap_or_else(|e| e.into_inner());
                    if *g {
                        return;
                    }
                }
                match weak.upgrade() {
                    Some(h) => h.close_idle(),
                    None => return,
                }
            })
            .expect("spawn the janitor");
        *lock(&host.janitor) = Some(j);
        Ok(host)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn config(&self) -> &HostConfig {
        &self.cfg
    }

    fn path_of(&self, id: &str) -> PathBuf {
        self.dir.join("fs").join(&id[..2]).join(id)
    }

    fn tick(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// True if a filesystem with this ID is there.
    pub fn exists(&self, id: &str) -> bool {
        is_fs_id(id) && self.path_of(id).join("MANIFEST").exists()
    }

    /// Every filesystem in the data directory.
    pub fn list(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(self.dir.join("fs")) {
            for shard in rd.flatten() {
                if let Ok(rd) = std::fs::read_dir(shard.path()) {
                    for e in rd.flatten() {
                        if let Some(name) = e.file_name().to_str() {
                            if is_fs_id(name) && e.path().join("MANIFEST").exists() {
                                out.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// Make a new, empty filesystem. Returns its ID.
    pub fn create(&self) -> Result<String, HostError> {
        if self.closing.load(Relaxed) {
            return Err(HostError::ShuttingDown);
        }
        let id = new_id();
        let path = self.path_of(&id);
        std::fs::create_dir_all(&path).map_err(|e| HostError::Failed(format!("{}: {}", path.display(), e)))?;
        let (mut db, _) = Db::open_dir(&path, self.cfg.db.clone()).map_err(HostError::Failed)?;
        self.prepare(&mut db);
        // The new directory's name must survive a power cut too.
        sync_dir(path.parent().unwrap());
        let fs = Arc::new(Fs::new(path));
        fs.mem.store(db.memory_estimate(), Relaxed);
        *fs.db.write().unwrap_or_else(|e| e.into_inner()) = Some(db);
        fs.used.store(self.tick(), Relaxed);
        lock(&self.fss).insert(id.clone(), fs);
        self.make_room();
        Ok(id)
    }

    fn get(&self, id: &str) -> Result<Arc<Fs>, HostError> {
        if !is_fs_id(id) {
            return Err(HostError::NotFound);
        }
        let mut map = lock(&self.fss);
        if let Some(fs) = map.get(id) {
            if fs.deleted.load(Relaxed) {
                return Err(HostError::NotFound);
            }
            return Ok(fs.clone());
        }
        let path = self.path_of(id);
        if !path.join("MANIFEST").exists() {
            return Err(HostError::NotFound);
        }
        let fs = Arc::new(Fs::new(path));
        map.insert(id.to_string(), fs.clone());
        Ok(fs)
    }

    /// Run one unit on a filesystem, in the caller's session.
    pub fn run(&self, id: &str, req: &Request, sess: &mut Session, out: &mut dyn Sink, err: &mut dyn Sink) -> Result<UnitEnd, HostError> {
        if self.closing.load(Relaxed) {
            return Err(HostError::ShuttingDown);
        }
        let fs = self.get(id)?;
        let stop = &req.stop;
        stop.set_command_limit(req.limits.commands);
        stop.set_scratch_limit(req.limits.scratch);
        stop.set_memory_limit(req.limits.memory);
        let deadline = Instant::now() + req.limits.time;
        let watched = self.watch.add(deadline, stop);
        lock(&fs.running).push(stop.clone());
        let result = self.run_on(&fs, req, sess, out, err);
        lock(&fs.running).retain(|s| !Arc::ptr_eq(s, stop));
        self.watch.cancel(watched);
        fs.used.store(self.tick(), Relaxed);
        self.make_room();
        result
    }

    fn run_on(&self, fs: &Arc<Fs>, req: &Request, sess: &mut Session, out: &mut dyn Sink, err: &mut dyn Sink) -> Result<UnitEnd, HostError> {
        let kind = classify(req.line);
        let pass = fs.enter(kind == Kind::Writer, &req.stop).ok_or_else(|| HostError::Stopped(req.stop.requested().unwrap_or(Reason::Gone)))?;
        if fs.deleted.load(Relaxed) {
            return Err(HostError::NotFound);
        }
        self.ensure_open(fs)?;
        let (mut end, seq, group) = {
            let _in = unit::enter(&req.stop);
            match kind {
                Kind::Reader => {
                    let g = fs.db.read().unwrap_or_else(|e| e.into_inner());
                    let db = g.as_ref().unwrap();
                    let end = run_unit_with(Access::Read(db), sess, req.line, req.stdin, out, err);
                    (end, db.commit_seq(), db.group())
                }
                Kind::Writer => {
                    let mut g = fs.db.write().unwrap_or_else(|e| e.into_inner());
                    let db = g.as_mut().unwrap();
                    db.set_quotas(req.quotas.text, req.quotas.nodes, req.quotas.disk);
                    let end = run_unit_with(Access::Write(db), sess, req.line, req.stdin, out, err);
                    (end, db.commit_seq(), db.group())
                }
            }
        };
        if matches!(end.ending, Ending::Damaged(..) | Ending::Fault(_)) {
            // The filesystem is opened again from disk, which recovers it.
            drop(pass);
            let _alone = fs.gate.write();
            let mut g = fs.db.write().unwrap_or_else(|e| e.into_inner());
            if let Some(db) = g.as_mut() {
                engine::after_unit(db, &end, err);
            }
            *g = None;
            drop(g);
            let _ = self.ensure_open(fs);
            return Ok(end);
        }
        let mem = fs.db.read().unwrap_or_else(|e| e.into_inner()).as_ref().map_or(0, |db| db.memory_estimate());
        fs.mem.store(mem, Relaxed);
        drop(pass);
        // What the unit wrote, or saw, is on disk before it is answered;
        // the next unit has the filesystem meanwhile.
        if let Some(group) = group {
            match group.wait(seq) {
                Ok(w) => end.cost.syncs += w.synced,
                Err(e) => {
                    let _ = err.write(format!("qdb: storage failed: {}; the filesystem is opened again from disk, and what this unit did may or may not be kept\n", e).as_bytes());
                    end.ending = Ending::Fault(e);
                    end.code = 2;
                    let _alone = fs.gate.write();
                    *fs.db.write().unwrap_or_else(|e| e.into_inner()) = None;
                }
            }
        }
        Ok(end)
    }

    fn prepare(&self, db: &mut Db) {
        if self.cfg.group_commit && self.cfg.db.sync == crate::tree::SyncMode::Always {
            db.group_log();
        }
    }

    fn ensure_open(&self, fs: &Fs) -> Result<(), HostError> {
        if fs.db.read().unwrap_or_else(|e| e.into_inner()).is_some() {
            return Ok(());
        }
        let mut g = fs.db.write().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            let (mut db, _) = Db::open_dir(&fs.path, self.cfg.db.clone()).map_err(|e| HostError::Failed(format!("the filesystem could not be opened: {}", e)))?;
            self.prepare(&mut db);
            fs.mem.store(db.memory_estimate(), Relaxed);
            *g = Some(db);
        }
        Ok(())
    }

    /// Look at a filesystem's database directly, as a reader would. For
    /// tools and tests (the harness compares trees through it); nothing
    /// on the API reaches it.
    pub fn with_db<R>(&self, id: &str, f: impl FnOnce(&Db) -> R) -> Result<R, HostError> {
        let fs = self.get(id)?;
        let stop = Stop::new();
        let _pass = fs.enter(false, &stop).ok_or(HostError::ShuttingDown)?;
        self.ensure_open(&fs)?;
        let g = fs.db.read().unwrap_or_else(|e| e.into_inner());
        Ok(f(g.as_ref().unwrap()))
    }

    /// Change a filesystem's database directly, alone on it, as a writer
    /// would. For tools and tests: loading a test's starting tree.
    pub fn with_db_mut<R>(&self, id: &str, f: impl FnOnce(&mut Db) -> R) -> Result<R, HostError> {
        let fs = self.get(id)?;
        let stop = Stop::new();
        let _pass = fs.enter(true, &stop).ok_or(HostError::ShuttingDown)?;
        self.ensure_open(&fs)?;
        let mut g = fs.db.write().unwrap_or_else(|e| e.into_inner());
        let r = f(g.as_mut().unwrap());
        fs.mem.store(g.as_ref().unwrap().memory_estimate(), Relaxed);
        Ok(r)
    }

    /// Sizes and counts of a filesystem.
    pub fn info(&self, id: &str) -> Result<FsInfo, HostError> {
        let fs = self.get(id)?;
        let stop = Stop::new();
        let _pass = fs.enter(false, &stop).ok_or(HostError::ShuttingDown)?;
        self.ensure_open(&fs)?;
        let g = fs.db.read().unwrap_or_else(|e| e.into_inner());
        let db = g.as_ref().unwrap();
        let files = db.file_count();
        Ok(FsInfo { text_bytes: db.text_bytes(), files, folders: db.node_count() as u64 - files, disk_bytes: dir_bytes(&fs.path), open: true })
    }

    /// Delete a filesystem: its running units are stopped, and once they
    /// have ended its directory is removed. Final.
    pub fn delete(&self, id: &str) -> Result<(), HostError> {
        let fs = self.get(id)?;
        fs.deleted.store(true, Relaxed);
        for s in lock(&fs.running).iter() {
            s.request(Reason::Deleted);
        }
        let _alone = fs.gate.write();
        if let Some(db) = fs.db.write().unwrap_or_else(|e| e.into_inner()).take() {
            drop(db);
        }
        // Moved out of the way in one step, then removed: a crash in
        // between leaves it in the trash, which the next start empties.
        let trash = self.dir.join("trash").join(format!("{}-{}", id, new_id()));
        std::fs::rename(&fs.path, &trash).map_err(|e| HostError::Failed(format!("{}: {}", fs.path.display(), e)))?;
        sync_dir(fs.path.parent().unwrap());
        let _ = std::fs::remove_dir_all(&trash);
        lock(&self.fss).remove(id);
        Ok(())
    }

    /// How many filesystems are open, and the memory they hold, about.
    pub fn open_counts(&self) -> (usize, u64) {
        let map = lock(&self.fss);
        let mut n = 0;
        let mut mem = 0;
        for fs in map.values() {
            if fs.is_open() {
                n += 1;
                mem += fs.mem.load(Relaxed);
            }
        }
        (n, mem)
    }

    /// Close the least recently used filesystems while too many are open
    /// or they hold too much memory.
    fn make_room(&self) {
        loop {
            let victim = {
                let map = lock(&self.fss);
                let (mut n, mut mem) = (0usize, 0u64);
                let mut oldest: Option<(u64, String)> = None;
                for (id, fs) in map.iter() {
                    if fs.is_open() {
                        n += 1;
                        mem += fs.mem.load(Relaxed);
                        let used = fs.used.load(Relaxed);
                        if fs.gate.busy() == 0 && oldest.as_ref().is_none_or(|(u, _)| used < *u) {
                            oldest = Some((used, id.clone()));
                        }
                    }
                }
                if n <= self.cfg.max_open && mem <= self.cfg.max_open_bytes {
                    return;
                }
                match oldest {
                    Some((_, id)) => map.get(&id).cloned().map(|fs| (id, fs)),
                    None => return,
                }
            };
            match victim {
                Some((id, fs)) => {
                    if !self.close_one(&id, &fs) {
                        return;
                    }
                }
                None => return,
            }
        }
    }

    /// Close one filesystem if no unit is using it. True if it was closed.
    fn close_one(&self, id: &str, fs: &Arc<Fs>) -> bool {
        let Some(_alone) = fs.gate.try_write() else { return false };
        if let Some(db) = fs.db.write().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = db.close();
        }
        let mut map = lock(&self.fss);
        if map.get(id).is_some_and(|f| Arc::ptr_eq(f, fs)) && Arc::strong_count(fs) <= 2 {
            map.remove(id);
        }
        true
    }

    /// Close every filesystem that has been idle for the set time.
    pub fn close_idle(&self) {
        let now = self.tick();
        let idle = self.cfg.idle_close.as_millis() as u64;
        let list: Vec<(String, Arc<Fs>)> = lock(&self.fss).iter().filter(|(_, fs)| now.saturating_sub(fs.used.load(Relaxed)) >= idle).map(|(id, fs)| (id.clone(), fs.clone())).collect();
        for (id, fs) in list {
            if fs.gate.busy() == 0 {
                self.close_one(&id, &fs);
            }
        }
    }

    /// Stop every unit, wait for them, and close every filesystem.
    pub fn shutdown(&self) {
        self.closing.store(true, Relaxed);
        let list: Vec<Arc<Fs>> = lock(&self.fss).values().cloned().collect();
        for fs in &list {
            for s in lock(&fs.running).iter() {
                s.request(Reason::Shutdown);
            }
        }
        for fs in &list {
            let _alone = fs.gate.write();
            if let Some(db) = fs.db.write().unwrap_or_else(|e| e.into_inner()).take() {
                let _ = db.close();
            }
        }
        lock(&self.fss).clear();
        {
            let (m, c) = &*self.quit;
            *lock(m) = true;
            c.notify_all();
        }
        if let Some(j) = lock(&self.janitor).take() {
            let _ = j.join();
        }
        self.watch.stop();
        lock(&self.dir_lock).take();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if !self.closing.load(Relaxed) {
            self.shutdown();
        }
    }
}

impl Fs {
    fn new(path: PathBuf) -> Fs {
        Fs { path, gate: Gate::new(), db: RwLock::new(None), used: AtomicU64::new(0), mem: AtomicU64::new(0), deleted: AtomicBool::new(false), running: Mutex::new(Vec::new()) }
    }

    fn is_open(&self) -> bool {
        self.db.try_read().map_or(true, |g| g.is_some())
    }

    /// Wait for a turn, as a writer or a reader. None if the unit is
    /// stopped while it waits.
    fn enter(&self, write: bool, stop: &Stop) -> Option<Pass<'_>> {
        self.gate.enter_until(write, || stop.requested().is_some())
    }
}

/// Bytes the files of a directory take, by their sizes.
pub fn dir_bytes(path: &Path) -> u64 {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                n += m.len();
            }
        }
    }
    n
}

fn sync_dir(path: &Path) {
    if let Ok(f) = std::fs::File::open(path) {
        let _ = f.sync_all();
    }
}

// ------------------------------------------------------------ deadlines

/// Asks units to stop at their deadlines.
struct Watchdog {
    state: Mutex<WatchState>,
    wake: Condvar,
}

#[derive(Default)]
struct WatchState {
    heap: BinaryHeap<std::cmp::Reverse<(Instant, u64)>>,
    stops: HashMap<u64, Weak<Stop>>,
    next: u64,
    done: bool,
}

impl Watchdog {
    fn start() -> Arc<Watchdog> {
        let w = Arc::new(Watchdog { state: Mutex::new(WatchState::default()), wake: Condvar::new() });
        let w2 = w.clone();
        std::thread::Builder::new().name("qdb-deadlines".into()).spawn(move || w2.run()).expect("spawn the deadline thread");
        w
    }

    fn cancel(&self, n: u64) {
        lock(&self.state).stops.remove(&n);
    }

    fn add(&self, at: Instant, stop: &Arc<Stop>) -> u64 {
        let mut s = lock(&self.state);
        let n = s.next;
        s.next += 1;
        let earliest = s.heap.peek().is_none_or(|e| at < e.0 .0);
        s.heap.push(std::cmp::Reverse((at, n)));
        s.stops.insert(n, Arc::downgrade(stop));
        if earliest {
            self.wake.notify_one();
        }
        n
    }

    fn stop(&self) {
        lock(&self.state).done = true;
        self.wake.notify_one();
    }

    fn run(&self) {
        let mut s = lock(&self.state);
        loop {
            if s.done {
                return;
            }
            let now = Instant::now();
            while let Some(std::cmp::Reverse((at, n))) = s.heap.peek().copied() {
                if at > now {
                    break;
                }
                s.heap.pop();
                if let Some(stop) = s.stops.remove(&n).and_then(|w| w.upgrade()) {
                    stop.request(Reason::Deadline);
                }
            }
            // Units that ended long before their deadline leave dead
            // entries; they are dropped when their time comes.
            let wait = s.heap.peek().map_or(Duration::from_secs(3600), |e| e.0 .0.saturating_duration_since(now));
            s = self.wake.wait_timeout(s, wait).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}
