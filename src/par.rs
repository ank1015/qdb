//! One search on several threads.
//!
//! A recursive grep or rg that reads much of a tree hands files to helper
//! threads and prints their results in the order one thread would have:
//! the walk stays on the unit's own thread, which numbers each file it
//! would have searched, gives it to a helper, and writes the results in
//! that order as they come in. The output is the same bytes; the counts
//! of what was read are the same, since every file is read once either
//! way (helpers hand theirs back, `tree::add_reads`). A search that is
//! stopped early by a reader of its output (`| head`) may have read a few
//! files ahead.
//!
//! Helpers are lent by whoever runs the unit (`Lender`): a server lends
//! the workers it has idle, and takes them back when other units arrive
//! (a helper looks before each file). Without a lender a search stays on
//! one thread. A search takes helpers only once it has read `threshold`
//! bytes on its own thread, so that small searches never do.

use crate::vfs::lock;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Condvar, Mutex, RwLock};

/// Lends threads to a unit's search.
pub trait Lender: Send + Sync {
    /// Up to `want` threads, fewer or none if they are not idle.
    fn borrow(&self, want: usize) -> usize;
    /// Give back threads that were borrowed.
    fn give_back(&self, n: usize);
    /// Whether other work is waiting for a thread: a helper then stops.
    fn wanted(&self) -> bool;
}

/// Lends up to so many threads, always: for tests and the command line.
pub struct Fixed(pub usize);

impl Lender for Fixed {
    fn borrow(&self, want: usize) -> usize {
        want.min(self.0)
    }
    fn give_back(&self, _: usize) {}
    fn wanted(&self) -> bool {
        false
    }
}

thread_local! {
    static LENDER: RefCell<Option<Arc<dyn Lender>>> = const { RefCell::new(None) };
    /// The most threads the unit's search may use (rg -j); 0 for no
    /// limit of its own.
    static MAX: Cell<usize> = const { Cell::new(0) };
}

/// For a process with no lender of its own per unit (the command line, a
/// test): the lender every thread uses.
static DEFAULT: RwLock<Option<Arc<dyn Lender>>> = RwLock::new(None);
/// Bytes a search reads on its own thread before it asks for helpers.
static THRESHOLD: AtomicUsize = AtomicUsize::new(4 << 20);

pub fn set_default_lender(l: Option<Arc<dyn Lender>>) {
    *DEFAULT.write().unwrap_or_else(|e| e.into_inner()) = l;
}

pub fn set_threshold(bytes: usize) {
    THRESHOLD.store(bytes, Relaxed);
}

pub fn threshold() -> usize {
    THRESHOLD.load(Relaxed)
}

/// While it lives, searches on this thread borrow from `l`.
pub struct Lending(Option<Arc<dyn Lender>>);

pub fn lend(l: Arc<dyn Lender>) -> Lending {
    Lending(LENDER.with(|c| c.replace(Some(l))))
}

impl Drop for Lending {
    fn drop(&mut self) {
        LENDER.with(|c| *c.borrow_mut() = self.0.take());
    }
}

/// The most threads, the unit's own included, that searches on this
/// thread may use: `rg -j N`. 0 for no limit of the unit's own.
pub fn set_max_threads(n: usize) -> usize {
    MAX.with(|m| m.replace(n))
}

fn lender() -> Option<Arc<dyn Lender>> {
    LENDER.with(|c| c.borrow().clone()).or_else(|| DEFAULT.read().unwrap_or_else(|e| e.into_inner()).clone())
}

fn cores() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

type Panic = Box<dyn std::any::Any + Send>;

struct Shared<J, R> {
    jobs: Mutex<(VecDeque<(u64, J)>, bool)>,
    job_cv: Condvar,
    done: Mutex<HashMap<u64, Result<(R, [u64; 6]), Panic>>>,
    done_cv: Condvar,
    /// Helpers still running.
    running: Mutex<usize>,
    running_cv: Condvar,
    closed: AtomicBool,
}

/// What a helper thread runs: made once per thread from the factory.
pub type Work<'a, J, R> = Box<dyn FnMut(J) -> R + 'a>;

/// Files handed to helpers, with results taken back in order. `J` is a
/// file to search, `R` what searching it gave.
pub struct Ahead<J: Send + 'static, R: Send + 'static> {
    shared: Arc<Shared<J, R>>,
    /// The search run on this thread, for when it helps.
    own: Work<'static, J, R>,
    make: Arc<dyn Fn() -> Work<'static, J, R> + Send + Sync>,
    lender: Arc<dyn Lender>,
    stop: Option<Arc<crate::unit::Stop>>,
    helpers: usize,
    most: usize,
    next: u64,
    taken: u64,
    window: usize,
    /// When to ask for helpers again: after this many more files.
    ask_again: usize,
}

impl<J: Send + 'static, R: Send + 'static> Ahead<J, R> {
    /// Start, if helpers can be had. `make` makes the search a thread
    /// runs; it may borrow from the caller, since every helper has ended
    /// before this is dropped.
    pub fn start<'a>(make: impl Fn() -> Work<'a, J, R> + Send + Sync + 'a) -> Option<Ahead<J, R>> {
        let lender = lender()?;
        let max = MAX.with(|m| m.get());
        let most = if max == 0 { cores() } else { max }.saturating_sub(1);
        if most == 0 {
            return None;
        }
        let make: Arc<dyn Fn() -> Work<'a, J, R> + Send + Sync + 'a> = Arc::new(make);
        // SAFETY: the helpers that call `make`, and the searches it made,
        // are all finished before `Drop` returns, and `Ahead` cannot
        // outlive 'a, since nothing it holds is handed out.
        let make: Arc<dyn Fn() -> Work<'static, J, R> + Send + Sync + 'static> = unsafe { std::mem::transmute(make) };
        let shared = Arc::new(Shared {
            jobs: Mutex::new((VecDeque::new(), false)),
            job_cv: Condvar::new(),
            done: Mutex::new(HashMap::new()),
            done_cv: Condvar::new(),
            running: Mutex::new(0),
            running_cv: Condvar::new(),
            closed: AtomicBool::new(false),
        });
        let own = make();
        let mut a = Ahead { shared, own, make, lender, stop: crate::unit::current(), helpers: 0, most, next: 0, taken: 0, window: 0, ask_again: 0 };
        a.more_helpers();
        if a.helpers == 0 {
            return None;
        }
        Some(a)
    }

    fn more_helpers(&mut self) {
        let want = self.most.saturating_sub(self.helpers);
        if want == 0 {
            return;
        }
        let got = self.lender.borrow(want);
        for _ in 0..got {
            self.spawn();
        }
        self.window = 4 * (self.helpers + 1);
        self.ask_again = 64;
    }

    fn spawn(&mut self) {
        let (shared, make, lender, stop) = (self.shared.clone(), self.make.clone(), self.lender.clone(), self.stop.clone());
        *lock(&shared.running) += 1;
        let spawned = std::thread::Builder::new().name("qdb-search".into()).stack_size(64 << 20).spawn(move || {
            helper(&shared, &*make, &*lender, stop.as_ref());
            lender.give_back(1);
            let mut n = lock(&shared.running);
            *n -= 1;
            shared.running_cv.notify_all();
        });
        match spawned {
            Ok(_) => self.helpers += 1,
            Err(_) => {
                *lock(&self.shared.running) -= 1;
                self.lender.give_back(1);
            }
        }
    }

    /// Hand on one file; give `take` every result that is due, in order.
    pub fn submit(&mut self, job: J, take: &mut dyn FnMut(R)) {
        let seq = self.next;
        self.next += 1;
        {
            let mut q = lock(&self.shared.jobs);
            q.0.push_back((seq, job));
        }
        self.shared.job_cv.notify_one();
        self.ask_again = self.ask_again.saturating_sub(1);
        if self.ask_again == 0 {
            self.more_helpers();
        }
        self.collect(take, false);
        // Too far ahead: help, or wait, until the oldest is in.
        while (self.next - self.taken) as usize > self.window.max(1) {
            if !self.help_one() {
                self.wait_oldest();
            }
            self.collect(take, false);
        }
    }

    /// Every result still to come, in order.
    pub fn finish(&mut self, take: &mut dyn FnMut(R)) {
        while self.taken < self.next {
            self.collect(take, false);
            if self.taken == self.next {
                break;
            }
            if !self.help_one() {
                self.wait_oldest();
            }
        }
    }

    /// Run the next file nobody has taken yet, here. False if there is
    /// none.
    fn help_one(&mut self) -> bool {
        let job = lock(&self.shared.jobs).0.pop_front();
        let Some((seq, job)) = job else { return false };
        // Counted on this thread as it reads.
        let r = (self.own)(job);
        lock(&self.shared.done).insert(seq, Ok((r, [0; 6])));
        true
    }

    fn wait_oldest(&mut self) {
        let mut d = lock(&self.shared.done);
        while !d.contains_key(&self.taken) {
            if let Some(s) = &self.stop {
                if s.requested().is_some() {
                    drop(d);
                    crate::unit::poll();
                    d = lock(&self.shared.done);
                    continue;
                }
            }
            let (g, _) = self.shared.done_cv.wait_timeout(d, std::time::Duration::from_millis(5)).unwrap_or_else(|e| e.into_inner());
            d = g;
            // A helper that left (other work wanted it) leaves its files
            // to whoever is still here.
            if !d.contains_key(&self.taken) && *lock(&self.shared.running) == 0 {
                drop(d);
                self.help_one();
                d = lock(&self.shared.done);
            }
        }
    }

    fn collect(&mut self, take: &mut dyn FnMut(R), _wait: bool) {
        loop {
            let r = lock(&self.shared.done).remove(&self.taken);
            match r {
                None => return,
                Some(Ok((r, reads))) => {
                    self.taken += 1;
                    crate::tree::add_reads(&reads);
                    take(r);
                }
                Some(Err(p)) => std::panic::resume_unwind(p),
            }
        }
    }

    /// Helpers in use now.
    pub fn helpers(&self) -> usize {
        *lock(&self.shared.running)
    }
}

impl<J: Send + 'static, R: Send + 'static> Drop for Ahead<J, R> {
    fn drop(&mut self) {
        self.shared.closed.store(true, Relaxed);
        {
            let mut q = lock(&self.shared.jobs);
            q.0.clear();
            q.1 = true;
        }
        self.shared.job_cv.notify_all();
        let mut n = lock(&self.shared.running);
        while *n > 0 {
            n = self.shared.running_cv.wait(n).unwrap_or_else(|e| e.into_inner());
        }
    }
}

fn helper<J, R>(shared: &Shared<J, R>, make: &(dyn Fn() -> Work<'static, J, R> + Send + Sync), lender: &dyn Lender, stop: Option<&Arc<crate::unit::Stop>>) {
    let _in = stop.map(crate::unit::enter);
    let mut work = None;
    loop {
        if lender.wanted() || shared.closed.load(Relaxed) {
            return;
        }
        let job = {
            let mut q = lock(&shared.jobs);
            loop {
                if let Some(j) = q.0.pop_front() {
                    break Some(j);
                }
                if q.1 {
                    break None;
                }
                let (g, _) = shared.job_cv.wait_timeout(q, std::time::Duration::from_millis(20)).unwrap_or_else(|e| e.into_inner());
                q = g;
                if lender.wanted() {
                    break None;
                }
            }
        };
        let Some((seq, job)) = job else { return };
        let w = work.get_or_insert_with(make);
        let before = crate::tree::reads_now();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w(job)));
        let after = crate::tree::reads_now();
        let reads: [u64; 6] = std::array::from_fn(|i| after[i] - before[i]);
        let failed = r.is_err();
        lock(&shared.done).insert(seq, r.map(|r| (r, reads)));
        shared.done_cv.notify_all();
        if failed {
            return;
        }
    }
}
