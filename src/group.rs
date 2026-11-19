//! Syncs shared by several units: group commit.
//!
//! Under `SyncMode::Always` a writer is answered only once its unit is on
//! disk. Done by each writer for itself, every unit waits for a sync of
//! its own, one after another, while the filesystem is held. With a
//! `GroupLog`, a writer logs its unit and lets the filesystem go before
//! the sync; then it waits for a sync that covers its unit. One writer
//! (the first to find no sync running) syncs for everyone logged by
//! then; units logged while that sync runs are covered by the next one.
//!
//! The log is one stream of records in order, so a sync covers a prefix
//! of it: a power cut loses only whole units at the end, in order, and
//! never one that was answered. A reader that saw a unit's changes waits
//! for the same sync before it is answered, so nothing it says can be
//! lost either.

use crate::vfs::{lock, Syncer};
use std::sync::{Arc, Condvar, Mutex};

struct State {
    /// The last unit logged, by its sequence number, and a handle on the
    /// data file it is in.
    logged: u64,
    tail: Option<Arc<dyn Syncer>>,
    /// Every unit up to this one is on disk.
    synced: u64,
    syncing: bool,
    /// A sync failed: the filesystem must be opened again.
    failed: Option<String>,
    /// Syncs made, and units they covered.
    syncs: u64,
    covered: u64,
}

pub struct GroupLog {
    state: Mutex<State>,
    cv: Condvar,
}

/// How waiting for a sync went, for the unit's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Waited {
    /// 1 if this unit made the sync that covered it.
    pub synced: u64,
}

impl GroupLog {
    /// For a database whose log is on disk up to unit `seq`.
    pub fn new(seq: u64) -> Arc<GroupLog> {
        Arc::new(GroupLog { state: Mutex::new(State { logged: seq, tail: None, synced: seq, syncing: false, failed: None, syncs: 0, covered: 0 }), cv: Condvar::new() })
    }

    /// Unit `seq` was logged to the data file `syncer` syncs. Called by
    /// the database while it is held by the writer, so in order.
    pub fn logged(&self, seq: u64, syncer: Arc<dyn Syncer>) {
        let mut s = lock(&self.state);
        s.logged = seq;
        s.tail = Some(syncer);
    }

    /// The database synced its log itself, up to unit `seq`.
    pub fn synced_to(&self, seq: u64) {
        let mut s = lock(&self.state);
        if seq > s.synced {
            s.synced = seq;
            self.cv.notify_all();
        }
    }

    /// Return once unit `seq` is on disk, syncing if no one else is.
    pub fn wait(&self, seq: u64) -> Result<Waited, String> {
        let mut s = lock(&self.state);
        let mut w = Waited::default();
        loop {
            if s.synced >= seq {
                return Ok(w);
            }
            if let Some(e) = &s.failed {
                return Err(e.clone());
            }
            if s.syncing {
                s = self.cv.wait(s).unwrap_or_else(|e| e.into_inner());
                continue;
            }
            // This unit syncs, for every unit logged so far.
            let (upto, tail) = (s.logged, s.tail.clone());
            s.syncing = true;
            drop(s);
            let r = match tail {
                Some(t) => t.sync(),
                None => Ok(()),
            };
            s = lock(&self.state);
            s.syncing = false;
            match r {
                Ok(()) => {
                    if upto > s.synced {
                        s.covered += upto - s.synced;
                        s.synced = upto;
                    }
                    s.syncs += 1;
                    w.synced = 1;
                }
                Err(e) => s.failed = Some(format!("a sync failed: {}", e)),
            }
            self.cv.notify_all();
        }
    }

    /// Syncs made, and units they covered.
    pub fn counts(&self) -> (u64, u64) {
        let s = lock(&self.state);
        (s.syncs, s.covered)
    }
}
