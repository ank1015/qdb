//! The unit running on this thread, as far as stopping it goes.
//!
//! A server must be able to stop a unit: at its deadline, when the caller
//! goes away, at shutdown, when it uses more memory than it may. It does
//! so by setting the unit's `Stop`. The engine looks at it often (`poll`):
//! at every command, at every piece of content and every folder read, at
//! every piece of output, in the loops of awk and sed. When it is set,
//! `poll` throws `Stopped`, which unwinds to whoever ran the unit; that
//! code takes back everything the unit changed (`Db::rollback`).
//!
//! `poll` is never reached from inside a change to the tree, so a unit is
//! always stopped between two whole changes, and taking them back is
//! exact.

use std::cell::Cell;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};
use std::sync::Arc;

/// Why a unit was stopped from outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reason {
    /// It ran past its time limit.
    Deadline = 1,
    /// The caller went away before the answer.
    Gone = 2,
    /// The server is shutting down.
    Shutdown = 3,
    /// It used more memory than one unit may.
    Memory = 4,
    /// Its filesystem is being deleted.
    Deleted = 5,
    /// The engine failed inside it (a test plants this: `poll` then
    /// panics as a bug would).
    Fault = 6,
    /// It wrote more to scratch files than one unit may.
    Scratch = 7,
}

impl Reason {
    fn from_u8(v: u8) -> Reason {
        match v {
            1 => Reason::Deadline,
            2 => Reason::Gone,
            3 => Reason::Shutdown,
            4 => Reason::Memory,
            5 => Reason::Deleted,
            7 => Reason::Scratch,
            _ => Reason::Fault,
        }
    }

    /// The name an answer gives the reason.
    pub fn name(self) -> &'static str {
        match self {
            Reason::Deadline => "deadline",
            Reason::Gone => "caller_gone",
            Reason::Shutdown => "shutdown",
            Reason::Memory => "memory",
            Reason::Deleted => "deleted",
            Reason::Fault => "fault",
            Reason::Scratch => "scratch",
        }
    }

    /// What the unit's standard error says.
    pub fn message(self) -> &'static str {
        match self {
            Reason::Deadline => "the unit was stopped at its time limit; nothing it changed was kept",
            Reason::Gone => "the unit was stopped because its caller went away; nothing it changed was kept",
            Reason::Shutdown => "the unit was stopped because the server is shutting down; nothing it changed was kept",
            Reason::Memory => "the unit was stopped because it used more memory than one unit may; nothing it changed was kept",
            Reason::Deleted => "the unit was stopped because its filesystem is being deleted",
            Reason::Fault => "the unit was stopped by a fault inside the engine; nothing it changed was kept",
            Reason::Scratch => "the unit was stopped because it used more scratch space than one unit may; nothing it changed was kept",
        }
    }
}

/// A request to stop one unit. Shared by the unit's threads and whoever
/// may stop it.
#[derive(Debug, Default)]
pub struct Stop {
    reason: AtomicU8,
    /// The most bytes the unit may have allocated at once, 0 for no limit.
    memory: std::sync::atomic::AtomicUsize,
    /// The most commands it may run, 0 for the database's own limit.
    commands: std::sync::atomic::AtomicU64,
    /// The most bytes it may write to scratch files, 0 for no limit.
    scratch: std::sync::atomic::AtomicU64,
    /// Scratch bytes written so far.
    scratch_used: std::sync::atomic::AtomicU64,
    /// Memory the unit's threads hold, each as it last told at a `poll`.
    live: std::sync::atomic::AtomicIsize,
}

impl Stop {
    pub fn new() -> Arc<Stop> {
        Arc::new(Stop::default())
    }

    /// Ask the unit to stop. The first reason given is the one kept.
    pub fn request(&self, why: Reason) {
        let _ = self.reason.compare_exchange(0, why as u8, Relaxed, Relaxed);
    }

    pub fn requested(&self) -> Option<Reason> {
        match self.reason.load(Relaxed) {
            0 => None,
            v => Some(Reason::from_u8(v)),
        }
    }

    /// Limit the memory the unit may hold, counted on the threads it runs
    /// on (with `mem::Counting` installed; otherwise nothing is counted).
    pub fn set_memory_limit(&self, bytes: usize) {
        self.memory.store(bytes, Relaxed);
    }

    /// Limit the commands the unit may run, below the database's limit.
    pub fn set_command_limit(&self, n: u64) {
        self.commands.store(n, Relaxed);
    }

    /// Limit what the unit may write to scratch files.
    pub fn set_scratch_limit(&self, bytes: u64) {
        self.scratch.store(bytes, Relaxed);
    }

    /// Scratch bytes the unit has written.
    pub fn scratch_used(&self) -> u64 {
        self.scratch_used.load(Relaxed)
    }
}

/// The command limit of the unit on this thread, if one was set.
pub fn command_limit() -> Option<u64> {
    with_current(|s| s.commands.load(Relaxed)).filter(|n| *n != 0)
}

/// The unit on this thread wrote `bytes` more to scratch files. Stops it
/// if that passes its limit.
pub fn add_scratch(bytes: u64) {
    let over = with_current(|s| {
        let used = s.scratch_used.fetch_add(bytes, Relaxed) + bytes;
        let limit = s.scratch.load(Relaxed);
        if limit != 0 && used > limit {
            s.request(Reason::Scratch);
            true
        } else {
            false
        }
    });
    if over == Some(true) {
        poll();
    }
}

fn with_current<T>(f: impl FnOnce(&Stop) -> T) -> Option<T> {
    CURRENT.with(|c| {
        let p = c.get();
        // SAFETY: as in `current`.
        (!p.is_null()).then(|| f(unsafe { &*p }))
    })
}

/// Thrown by `poll` when the unit has been asked to stop.
#[derive(Debug, Clone, Copy)]
pub struct Stopped(pub Reason);

thread_local! {
    static CURRENT: Cell<*const Stop> = const { Cell::new(std::ptr::null()) };
    /// What this thread had allocated when the unit began.
    static BASE: Cell<usize> = const { Cell::new(0) };
    /// What this thread last added to its unit's `Stop::live`.
    static TOLD: Cell<isize> = const { Cell::new(0) };
}

/// While it lives, this thread runs the unit `stop` belongs to.
pub struct Entered {
    prev: *const Stop,
    prev_base: usize,
    prev_told: isize,
    _keep: Arc<Stop>,
}

/// Run the rest of this thread's work, until the guard is dropped, as
/// part of the unit that `stop` stops.
pub fn enter(stop: &Arc<Stop>) -> Entered {
    let prev = CURRENT.with(|c| c.replace(Arc::as_ptr(stop)));
    let prev_base = BASE.with(|b| b.replace(crate::mem::thread_live()));
    let prev_told = TOLD.with(|t| t.replace(0));
    Entered { prev, prev_base, prev_told, _keep: stop.clone() }
}

impl Drop for Entered {
    fn drop(&mut self) {
        CURRENT.with(|c| c.set(self.prev));
        BASE.with(|b| b.set(self.prev_base));
        TOLD.with(|t| t.set(self.prev_told));
    }
}

/// The stop request of the unit on this thread, if any.
pub fn current() -> Option<Arc<Stop>> {
    CURRENT.with(|c| {
        let p = c.get();
        if p.is_null() {
            return None;
        }
        // SAFETY: the pointer is kept alive by the `Entered` that set it.
        unsafe {
            Arc::increment_strong_count(p);
            Some(Arc::from_raw(p))
        }
    })
}

/// Stop here if the unit on this thread has been asked to stop.
#[inline]
pub fn poll() {
    CURRENT.with(|c| {
        let p = c.get();
        if p.is_null() {
            return;
        }
        // SAFETY: as in `current`.
        let stop = unsafe { &*p };
        let why = stop.reason.load(Relaxed);
        if why != 0 {
            stopped(why);
        }
        let limit = stop.memory.load(Relaxed);
        if limit != 0 {
            // What all of the unit's threads hold: each adds what it
            // holds to the unit's total as it passes here. Memory handed
            // from one thread to another is counted where it was
            // allocated until it is freed where it went.
            let mine = (crate::mem::thread_live() as isize).wrapping_sub(BASE.with(|b| b.get()) as isize);
            let told = TOLD.with(|t| t.replace(mine));
            let total = stop.live.fetch_add(mine - told, Relaxed) + (mine - told);
            if total > limit as isize {
                stop.request(Reason::Memory);
                stopped(Reason::Memory as u8);
            }
        }
    })
}

/// What a planted fault panics with.
const PLANTED: &str = "a fault planted by a test";

#[cold]
#[inline(never)]
fn stopped(why: u8) -> ! {
    let why = Reason::from_u8(why);
    if why == Reason::Fault {
        std::panic::panic_any(PLANTED);
    }
    std::panic::panic_any(Stopped(why))
}

/// Install a panic hook that keeps quiet about the panics the engine
/// throws on purpose (damage found, a unit stopped, a reader that tried
/// to write) and reports the others as before.
pub fn quiet_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(install_quiet_hook);
}

fn install_quiet_hook() {
    let usual = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let p = info.payload();
        let planted = p.downcast_ref::<&str>().is_some_and(|m| *m == PLANTED);
        let meant = planted || p.is::<crate::disk::Damage>() || p.is::<Stopped>() || p.is::<crate::engine::ReadOnly>() || p.is::<crate::tree::QuotaReached>();
        if !meant {
            usual(info);
        }
    }));
}
