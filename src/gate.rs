//! A fair lock for many readers or one writer.
//!
//! Callers are served in the order they arrive, with readers that arrive
//! together let in together: a reader that finds readers inside and no
//! one waiting goes straight in; anyone who finds someone waiting waits
//! behind them. So a waiting writer is never overtaken by readers that
//! came after it, and readers are not held up by a writer that came after
//! them.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

#[derive(Default)]
struct State {
    readers: usize,
    writer: bool,
    /// Waiting, in order: (ticket, wants to write).
    queue: VecDeque<(u64, bool)>,
    next: u64,
    /// Tickets let in but not yet awake to see it.
    granted: Vec<u64>,
}

#[derive(Default)]
pub struct Gate {
    state: Mutex<State>,
    wake: Condvar,
}

/// Held while inside. Dropping it lets the next ones in.
pub struct Pass<'a> {
    gate: &'a Gate,
    write: bool,
}

impl Gate {
    pub fn new() -> Gate {
        Gate::default()
    }

    pub fn read(&self) -> Pass<'_> {
        self.enter(false)
    }

    pub fn write(&self) -> Pass<'_> {
        self.enter(true)
    }

    /// A writer's pass if no one is inside or waiting; None otherwise.
    pub fn try_write(&self) -> Option<Pass<'_>> {
        let mut s = crate::vfs::lock(&self.state);
        if s.readers == 0 && !s.writer && s.queue.is_empty() {
            s.writer = true;
            return Some(Pass { gate: self, write: true });
        }
        None
    }

    /// How many are inside or waiting.
    pub fn busy(&self) -> usize {
        let s = crate::vfs::lock(&self.state);
        s.readers + s.writer as usize + s.queue.len() + s.granted.len()
    }

    /// Wait for a turn, but give up (None) once `give_up` says so: it is
    /// asked every few milliseconds while waiting.
    pub fn enter_until(&self, write: bool, give_up: impl Fn() -> bool) -> Option<Pass<'_>> {
        let mut s = crate::vfs::lock(&self.state);
        let free = if write { s.readers == 0 && !s.writer } else { !s.writer };
        if free && s.queue.is_empty() {
            if write {
                s.writer = true;
            } else {
                s.readers += 1;
            }
            return Some(Pass { gate: self, write });
        }
        let ticket = s.next;
        s.next += 1;
        s.queue.push_back((ticket, write));
        loop {
            s = self.wake.wait_timeout(s, std::time::Duration::from_millis(5)).unwrap_or_else(|e| e.into_inner()).0;
            if let Some(i) = s.granted.iter().position(|t| *t == ticket) {
                s.granted.swap_remove(i);
                let pass = Pass { gate: self, write };
                if give_up() {
                    drop(s);
                    drop(pass);
                    return None;
                }
                return Some(pass);
            }
            if give_up() {
                if let Some(i) = s.queue.iter().position(|(t, _)| *t == ticket) {
                    s.queue.remove(i);
                }
                // Leaving the queue may let those behind in.
                if s.admit() {
                    self.wake.notify_all();
                }
                return None;
            }
        }
    }

    fn enter(&self, write: bool) -> Pass<'_> {
        let mut s = crate::vfs::lock(&self.state);
        let free = if write { s.readers == 0 && !s.writer } else { !s.writer };
        if free && s.queue.is_empty() {
            if write {
                s.writer = true;
            } else {
                s.readers += 1;
            }
            return Pass { gate: self, write };
        }
        let ticket = s.next;
        s.next += 1;
        s.queue.push_back((ticket, write));
        loop {
            s = self.wake.wait(s).unwrap_or_else(|e| e.into_inner());
            if let Some(i) = s.granted.iter().position(|t| *t == ticket) {
                s.granted.swap_remove(i);
                return Pass { gate: self, write };
            }
        }
    }
}

impl State {
    /// Let in whoever can go in now, from the head of the queue.
    fn admit(&mut self) -> bool {
        let mut any = false;
        while let Some(&(ticket, write)) = self.queue.front() {
            if write {
                if self.readers == 0 && !self.writer {
                    self.writer = true;
                } else {
                    break;
                }
            } else if !self.writer {
                self.readers += 1;
            } else {
                break;
            }
            self.queue.pop_front();
            self.granted.push(ticket);
            any = true;
            if write {
                break;
            }
        }
        any
    }
}

impl Drop for Pass<'_> {
    fn drop(&mut self) {
        let mut s = crate::vfs::lock(&self.gate.state);
        if self.write {
            s.writer = false;
        } else {
            s.readers -= 1;
        }
        if s.admit() {
            self.gate.wake.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn one_writer_or_many_readers() {
        let gate = Arc::new(Gate::new());
        let inside = Arc::new(AtomicUsize::new(0));
        let writing = Arc::new(AtomicUsize::new(0));
        let mut hs = Vec::new();
        for t in 0..8 {
            let (gate, inside, writing) = (gate.clone(), inside.clone(), writing.clone());
            hs.push(std::thread::spawn(move || {
                for i in 0..2000 {
                    if (i + t) % 5 == 0 {
                        let _p = gate.write();
                        assert_eq!(writing.fetch_add(1, Ordering::SeqCst), 0);
                        assert_eq!(inside.load(Ordering::SeqCst), 0);
                        writing.fetch_sub(1, Ordering::SeqCst);
                    } else {
                        let _p = gate.read();
                        inside.fetch_add(1, Ordering::SeqCst);
                        assert_eq!(writing.load(Ordering::SeqCst), 0);
                        inside.fetch_sub(1, Ordering::SeqCst);
                    }
                }
            }));
        }
        for h in hs {
            h.join().unwrap();
        }
    }

    #[test]
    fn a_waiting_writer_is_not_overtaken() {
        let gate = Arc::new(Gate::new());
        let first = gate.read();
        let order = Arc::new(Mutex::new(Vec::new()));
        let (g, o) = (gate.clone(), order.clone());
        let w = std::thread::spawn(move || {
            let _p = g.write();
            o.lock().unwrap().push("writer");
        });
        while gate.busy() < 2 {
            std::thread::yield_now();
        }
        let (g, o) = (gate.clone(), order.clone());
        let r = std::thread::spawn(move || {
            let _p = g.read();
            o.lock().unwrap().push("reader");
        });
        while gate.busy() < 3 {
            std::thread::yield_now();
        }
        drop(first);
        w.join().unwrap();
        r.join().unwrap();
        assert_eq!(*order.lock().unwrap(), vec!["writer", "reader"]);
    }
}
