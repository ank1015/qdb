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
//! Without it the figure is 0 and everything else works. The engine runs
//! on one thread, so plain loads and stores are enough.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

pub struct Counting;

/// Bytes allocated and not yet freed.
pub static LIVE: AtomicUsize = AtomicUsize::new(0);
/// The most `LIVE` has been since it was last set.
pub static PEAK: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn grew(by: usize) {
    let live = LIVE.load(Relaxed).wrapping_add(by);
    LIVE.store(live, Relaxed);
    if live > PEAK.load(Relaxed) {
        PEAK.store(live, Relaxed);
    }
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
        LIVE.store(LIVE.load(Relaxed).wrapping_sub(layout.size()), Relaxed);
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
            LIVE.store(LIVE.load(Relaxed).wrapping_sub(layout.size()), Relaxed);
            grew(new_size);
        }
        p
    }
}

/// Start measuring a rise: the peak is set to what is live now, which is
/// returned.
pub fn mark() -> usize {
    let live = LIVE.load(Relaxed);
    PEAK.store(live, Relaxed);
    live
}

/// How far above `base` the live bytes have been since `mark`.
pub fn rise(base: usize) -> usize {
    PEAK.load(Relaxed).saturating_sub(base)
}

/// Processor time this thread has used, in nanoseconds.
pub fn cpu_ns() -> u64 {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // Safety: a plain system call that fills in the struct.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t) };
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
