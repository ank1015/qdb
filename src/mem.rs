//! Counting what the engine allocates.
//!
//! A program that wants the memory figure of the cost report installs
//! `Counting` as its allocator:
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOC: qdb::mem::Counting = qdb::mem::Counting;
//! ```
//!
//! Without it the figure is 0 and everything else works. Counts are kept
//! per thread, since units run on several at once, and for the process
//! (`LIVE`, to within 64 KB a thread).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

pub struct Counting;

/// Bytes allocated and not yet freed, by the whole process.
pub static LIVE: AtomicUsize = AtomicUsize::new(0);
/// The most `LIVE` has been since it was last set.
pub static PEAK: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Bytes this thread allocated less what it freed. Memory freed on
    /// another thread than the one that allocated it moves the two
    /// counts apart, so only differences over a stretch of one thread's
    /// work mean anything: what a unit allocated while it ran.
    static MINE: Cell<isize> = const { Cell::new(0) };
    /// The most `MINE` has been since `mark`.
    static MY_PEAK: Cell<isize> = const { Cell::new(0) };
    /// What this thread allocated, less what it freed, not yet added to
    /// `LIVE`: the process count is changed in steps of `STEP` or more,
    /// so that threads allocating at once do not all write one counter.
    static OWED: Cell<isize> = const { Cell::new(0) };
}

/// `LIVE` lags the truth by less than this much a thread.
const STEP: isize = 64 << 10;

#[inline]
fn settle(by: isize) {
    let owed = OWED.try_with(|o| {
        let v = o.get() + by;
        if v.abs() >= STEP {
            o.set(0);
            v
        } else {
            o.set(v);
            0
        }
    });
    // A thread being torn down has no counts of its own: straight to the
    // total.
    let v = owed.unwrap_or(by);
    if v > 0 {
        let live = LIVE.fetch_add(v as usize, Relaxed) + v as usize;
        if live > PEAK.load(Relaxed) {
            PEAK.store(live, Relaxed);
        }
    } else if v < 0 {
        LIVE.fetch_sub((-v) as usize, Relaxed);
    }
}

#[inline]
fn grew(by: usize) {
    settle(by as isize);
    let _ = MINE.try_with(|m| {
        let live = m.get().wrapping_add(by as isize);
        m.set(live);
        let _ = MY_PEAK.try_with(|p| {
            if live > p.get() {
                p.set(live);
            }
        });
    });
}

#[inline]
fn shrank(by: usize) {
    settle(-(by as isize));
    let _ = MINE.try_with(|m| m.set(m.get().wrapping_sub(by as isize)));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            grew(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        shrank(layout.size());
    }

    // Without these the default versions would be used, which allocate
    // afresh and copy: slower than the allocator underneath.
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            grew(layout.size());
        }
        p
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, layout, new_size);
        if !p.is_null() {
            shrank(layout.size());
            grew(new_size);
        }
        p
    }
}

/// What this thread holds by its own count (see `MINE`), as a number
/// that only differences of are used.
pub fn thread_live() -> usize {
    MINE.try_with(|m| m.get()).unwrap_or(0) as usize
}

/// Start measuring a rise on this thread: its peak is set to what it
/// holds now, which is returned.
pub fn mark() -> usize {
    let live = MINE.try_with(|m| m.get()).unwrap_or(0);
    let _ = MY_PEAK.try_with(|p| p.set(live));
    live as usize
}

/// How far above `base` this thread's count has been since `mark`.
pub fn rise(base: usize) -> usize {
    let peak = MY_PEAK.try_with(|p| p.get()).unwrap_or(0);
    (peak.wrapping_sub(base as isize)).max(0) as usize
}

/// Processor time this thread has used, in nanoseconds.
pub fn cpu_ns() -> u64 {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // Safety: a plain system call that fills in the struct.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t) };
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
