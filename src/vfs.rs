//! The one place the engine touches files.
//!
//! Everything the storage layer does to a disk goes through `Vfs` (a
//! directory) and `VFile` (a file in it): create, write at an offset,
//! truncate, sync, rename, remove, and read through a mapping. There are
//! two implementations:
//!
//! - `RealFs`: a directory on the machine. Reads go through `mmap`, so the
//!   pages a search touches are the operating system's file cache and not
//!   memory of the engine.
//! - `MemFs`: files in memory. With `MemFs::volatile` it is what a
//!   database without `--db` lives on. With `MemFs::tracked` it also
//!   remembers what has been synced and what has only been written, and
//!   can produce the directory as a power cut could leave it: that is the
//!   crash test.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub trait VFile {
    fn len(&self) -> u64;
    /// Write at an offset, growing the file if the write ends past its
    /// end. A gap is filled with zeros.
    fn write_at(&mut self, off: u64, data: &[u8]) -> io::Result<()>;
    fn truncate(&mut self, len: u64) -> io::Result<()>;
    /// Returns once everything written so far is on the disk.
    fn sync(&mut self) -> io::Result<()>;
    /// The start of a read-only view of the file. Bytes below `len()` can
    /// be read through it until the next call that changes the file.
    fn base(&self) -> *const u8;
}

pub trait Vfs {
    /// A new, empty file, replacing any file of that name. At most
    /// `reserve` bytes will ever be written to it through this object.
    fn create(&self, name: &str, reserve: u64) -> io::Result<Box<dyn VFile>>;
    /// An existing file. It can grow to `reserve` bytes, or stay as it is
    /// if that is less than its length.
    fn open(&self, name: &str, reserve: u64) -> io::Result<Box<dyn VFile>>;
    fn list(&self) -> io::Result<Vec<String>>;
    fn remove(&self, name: &str) -> io::Result<()>;
    /// Replaces `to` if it exists, in one step.
    fn rename(&self, from: &str, to: &str) -> io::Result<()>;
    /// Returns once creations, removals and renames so far are on disk.
    fn sync_dir(&self) -> io::Result<()>;
    /// Take the directory for this process alone. False if another
    /// process has it.
    fn lock(&self) -> io::Result<bool>;
    /// True if nothing here outlives the process, so that syncing and
    /// checking what was read back would be wasted work.
    fn volatile(&self) -> bool;
    /// For messages: where a file is.
    fn describe(&self, name: &str) -> String;
}

// ------------------------------------------------------------ real files

pub struct RealFs {
    dir: PathBuf,
    lock: RefCell<Option<std::fs::File>>,
}

impl RealFs {
    pub fn new(dir: &Path) -> RealFs {
        RealFs { dir: dir.to_path_buf(), lock: RefCell::new(None) }
    }
}

struct RealFile {
    file: std::fs::File,
    base: *mut libc::c_void,
    reserve: usize,
    len: u64,
}

impl RealFile {
    fn new(file: std::fs::File, reserve: u64) -> io::Result<RealFile> {
        use std::os::unix::io::AsRawFd;
        let len = file.metadata()?.len();
        // A file opened for more writing always has room for a little
        // more, whatever size it was made with: see `Segment::with`.
        let reserve = if reserve > 0 { reserve.max(len + 4096) } else { len } as usize;
        let mut base = std::ptr::null_mut();
        if reserve > 0 {
            // The mapping may be longer than the file: the pages past the
            // end become readable as the file grows, and are never read
            // before that.
            // SAFETY: a fresh shared read-only mapping of a file we hold open.
            base = unsafe { libc::mmap(std::ptr::null_mut(), reserve, libc::PROT_READ, libc::MAP_SHARED, file.as_raw_fd(), 0) };
            if base == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(RealFile { file, base, reserve, len })
    }
}

impl Drop for RealFile {
    fn drop(&mut self) {
        if self.reserve > 0 {
            // SAFETY: the mapping made in `new`, unmapped once.
            unsafe { libc::munmap(self.base, self.reserve) };
        }
    }
}

impl VFile for RealFile {
    fn len(&self) -> u64 {
        self.len
    }

    fn write_at(&mut self, off: u64, data: &[u8]) -> io::Result<()> {
        use std::os::unix::fs::FileExt;
        // A file made with nothing reserved is written and never read
        // through this handle.
        if self.reserve > 0 && off + data.len() as u64 > self.reserve as u64 {
            return Err(io::Error::new(io::ErrorKind::Other, "write past the space reserved for the file"));
        }
        self.file.write_all_at(data, off)?;
        self.len = self.len.max(off + data.len() as u64);
        Ok(())
    }

    fn truncate(&mut self, len: u64) -> io::Result<()> {
        self.file.set_len(len)?;
        self.len = len;
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        self.file.sync_data()
    }

    fn base(&self) -> *const u8 {
        self.base as *const u8
    }
}

impl Vfs for RealFs {
    fn create(&self, name: &str, reserve: u64) -> io::Result<Box<dyn VFile>> {
        let file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(self.dir.join(name))?;
        Ok(Box::new(RealFile::new(file, reserve)?))
    }

    fn open(&self, name: &str, reserve: u64) -> io::Result<Box<dyn VFile>> {
        let file = std::fs::OpenOptions::new().read(true).write(true).open(self.dir.join(name))?;
        Ok(Box::new(RealFile::new(file, reserve)?))
    }

    fn list(&self) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            names.push(entry?.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        Ok(names)
    }

    fn remove(&self, name: &str) -> io::Result<()> {
        std::fs::remove_file(self.dir.join(name))
    }

    fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        std::fs::rename(self.dir.join(from), self.dir.join(to))
    }

    fn sync_dir(&self) -> io::Result<()> {
        std::fs::File::open(&self.dir)?.sync_all()
    }

    fn lock(&self) -> io::Result<bool> {
        use std::os::unix::io::AsRawFd;
        let file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(self.dir.join("LOCK"))?;
        // SAFETY: flock on a descriptor we own.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::WouldBlock { Ok(false) } else { Err(e) };
        }
        *self.lock.borrow_mut() = Some(file);
        Ok(true)
    }

    fn volatile(&self) -> bool {
        false
    }

    fn describe(&self, name: &str) -> String {
        self.dir.join(name).display().to_string()
    }
}

// ------------------------------------------------------- files in memory

#[derive(Default)]
struct MemData {
    /// What reading the file gives now.
    data: Vec<u8>,
    /// What the disk is known to hold: the file as of the last sync.
    durable: Vec<u8>,
    /// Ranges written since the last sync.
    dirty: Vec<(u64, u64)>,
}

/// What a crash test wants to know about the file operations: how many
/// there have been, by kind.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IoCounts {
    pub writes: u64,
    pub bytes_written: u64,
    pub syncs: u64,
    pub dir_syncs: u64,
    pub other: u64,
}

impl IoCounts {
    pub fn ops(&self) -> u64 {
        self.writes + self.syncs + self.dir_syncs + self.other
    }
}

type Hook = Box<dyn FnMut(&MemFs, u64)>;

#[derive(Default)]
struct MemState {
    names: BTreeMap<String, Rc<RefCell<MemData>>>,
    /// The directory as of the last `sync_dir`.
    durable_names: BTreeMap<String, Rc<RefCell<MemData>>>,
    counts: IoCounts,
    /// Called after every operation that changes anything, with the
    /// number of operations so far.
    hook: Option<Hook>,
    /// The operation with this number fails, and every one after it.
    fail_from: Option<u64>,
}

#[derive(Clone)]
pub struct MemFs {
    state: Rc<RefCell<MemState>>,
    tracked: bool,
}

impl MemFs {
    /// Files that live and die with the process.
    pub fn volatile() -> MemFs {
        MemFs { state: Rc::default(), tracked: false }
    }

    /// Files in memory that behave like a disk: what has not been synced
    /// can be lost. See `crash`.
    pub fn tracked() -> MemFs {
        MemFs { state: Rc::default(), tracked: true }
    }

    pub fn counts(&self) -> IoCounts {
        self.state.borrow().counts
    }

    /// Have `hook` called after every operation that changes a file or
    /// the directory. It gets this file system and the number of such
    /// operations so far, and may call `crash` on it.
    pub fn set_hook(&self, hook: Option<Hook>) {
        self.state.borrow_mut().hook = hook;
    }

    /// Make the operation numbered `n` (counting from 1) fail, and all
    /// after it: the disk has gone away.
    pub fn fail_from(&self, n: Option<u64>) {
        self.state.borrow_mut().fail_from = n;
    }

    /// Total bytes in the files, as a disk would count them.
    pub fn disk_bytes(&self) -> u64 {
        self.state.borrow().names.values().map(|f| f.borrow().data.len() as u64).sum()
    }

    /// The directory as a power cut at this moment could leave it.
    ///
    /// Everything synced is there. Of what was written and not synced,
    /// any part may be: each 512-byte stretch of each write is kept or
    /// lost on its own, in no order. A file's length, a creation, a
    /// removal or a rename that was not synced may or may not have
    /// happened. `seed` picks one of the possible outcomes.
    pub fn crash(&self, seed: u64) -> MemFs {
        let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        let state = self.state.borrow();
        // Which names exist.
        let mut names: BTreeMap<String, Rc<RefCell<MemData>>> = BTreeMap::new();
        match next() % 3 {
            0 => names = state.durable_names.clone(),
            1 => names = state.names.clone(),
            _ => {
                for (name, file) in &state.durable_names {
                    match state.names.get(name) {
                        // Removed since: gone or not.
                        None if next() % 2 == 0 => {}
                        None => {
                            names.insert(name.clone(), file.clone());
                        }
                        // Replaced by a rename, or the same file.
                        Some(now) => {
                            names.insert(name.clone(), if next() % 2 == 0 { now.clone() } else { file.clone() });
                        }
                    }
                }
                for (name, file) in &state.names {
                    if !state.durable_names.contains_key(name) && next() % 2 == 0 {
                        names.insert(name.clone(), file.clone());
                    }
                }
            }
        }
        // How much of the unsynced writes survives: all, none, or some.
        let mode = next() % 4;
        let mut out = MemState::default();
        for (name, file) in names {
            let f = file.borrow();
            let mut data = f.durable.clone();
            if next() % 2 == 0 {
                // A change of length that reached the disk.
                data.resize(f.data.len(), 0);
            }
            for &(off, len) in &f.dirty {
                let mut at = off;
                while at < off + len {
                    let end = ((at / 512 + 1) * 512).min(off + len);
                    let keep = match mode {
                        0 => true,
                        1 => false,
                        _ => next() % 2 == 0,
                    };
                    // A write that the file has since been cut below.
                    let end_now = end.min(f.data.len() as u64);
                    if keep && at < end_now {
                        if data.len() < end_now as usize {
                            data.resize(end_now as usize, 0);
                        }
                        data[at as usize..end_now as usize].copy_from_slice(&f.data[at as usize..end_now as usize]);
                    }
                    at = end;
                }
            }
            let durable = data.clone();
            out.names.insert(name, Rc::new(RefCell::new(MemData { data, durable, dirty: Vec::new() })));
        }
        out.durable_names = out.names.clone();
        MemFs { state: Rc::new(RefCell::new(out)), tracked: true }
    }

    /// Count an operation; fail it if the disk has been taken away.
    fn begin(&self, count: impl FnOnce(&mut IoCounts)) -> io::Result<()> {
        let mut state = self.state.borrow_mut();
        if state.fail_from.is_some_and(|n| state.counts.ops() + 1 >= n) {
            return Err(io::Error::new(io::ErrorKind::Other, "injected failure"));
        }
        count(&mut state.counts);
        Ok(())
    }

    fn done(&self) {
        let hook = self.state.borrow_mut().hook.take();
        if let Some(mut hook) = hook {
            let ops = self.state.borrow().counts.ops();
            hook(self, ops);
            let mut state = self.state.borrow_mut();
            if state.hook.is_none() {
                state.hook = Some(hook);
            }
        }
    }

    fn handle(&self, data: Rc<RefCell<MemData>>) -> Box<dyn VFile> {
        Box::new(MemFile { data, fs: self.clone() })
    }
}

struct MemFile {
    data: Rc<RefCell<MemData>>,
    fs: MemFs,
}

impl VFile for MemFile {
    fn len(&self) -> u64 {
        self.data.borrow().data.len() as u64
    }

    fn write_at(&mut self, off: u64, bytes: &[u8]) -> io::Result<()> {
        self.fs.begin(|c| {
            c.writes += 1;
            c.bytes_written += bytes.len() as u64;
        })?;
        {
            let mut d = self.data.borrow_mut();
            let end = off as usize + bytes.len();
            if end > d.data.capacity() {
                // Grow in modest steps: for a block this size the
                // allocator moves pages, it does not copy them.
                let more = (end - d.data.len()).max(d.data.len() / 8).max(64 * 1024);
                d.data.reserve_exact(more);
            }
            if d.data.len() < end {
                d.data.resize(end, 0);
            }
            d.data[off as usize..end].copy_from_slice(bytes);
            if self.fs.tracked {
                d.dirty.push((off, bytes.len() as u64));
            }
        }
        self.fs.done();
        Ok(())
    }

    fn truncate(&mut self, len: u64) -> io::Result<()> {
        self.fs.begin(|c| c.other += 1)?;
        {
            self.data.borrow_mut().data.resize(len as usize, 0);
        }
        self.fs.done();
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        if !self.fs.tracked {
            return Ok(());
        }
        self.fs.begin(|c| c.syncs += 1)?;
        {
            let mut d = self.data.borrow_mut();
            let d = &mut *d;
            d.durable.resize(d.data.len(), 0);
            for &(off, len) in &d.dirty {
                let end = ((off + len) as usize).min(d.data.len());
                if (off as usize) < end {
                    d.durable[off as usize..end].copy_from_slice(&d.data[off as usize..end]);
                }
            }
            d.dirty.clear();
        }
        self.fs.done();
        Ok(())
    }

    fn base(&self) -> *const u8 {
        self.data.borrow().data.as_ptr()
    }
}

impl Vfs for MemFs {
    fn create(&self, name: &str, _reserve: u64) -> io::Result<Box<dyn VFile>> {
        self.begin(|c| c.other += 1)?;
        let data = Rc::new(RefCell::new(MemData::default()));
        self.state.borrow_mut().names.insert(name.to_string(), data.clone());
        let handle = self.handle(data);
        self.done();
        Ok(handle)
    }

    fn open(&self, name: &str, _reserve: u64) -> io::Result<Box<dyn VFile>> {
        let data = self.state.borrow().names.get(name).cloned();
        match data {
            Some(d) => Ok(self.handle(d)),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no such file")),
        }
    }

    fn list(&self) -> io::Result<Vec<String>> {
        Ok(self.state.borrow().names.keys().cloned().collect())
    }

    fn remove(&self, name: &str) -> io::Result<()> {
        self.begin(|c| c.other += 1)?;
        self.state.borrow_mut().names.remove(name);
        self.done();
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        self.begin(|c| c.other += 1)?;
        {
            let mut state = self.state.borrow_mut();
            match state.names.remove(from) {
                Some(f) => state.names.insert(to.to_string(), f),
                None => return Err(io::Error::new(io::ErrorKind::NotFound, "no such file")),
            };
        }
        self.done();
        Ok(())
    }

    fn sync_dir(&self) -> io::Result<()> {
        if !self.tracked {
            return Ok(());
        }
        self.begin(|c| c.dir_syncs += 1)?;
        {
            let mut state = self.state.borrow_mut();
            state.durable_names = state.names.clone();
        }
        self.done();
        Ok(())
    }

    fn lock(&self) -> io::Result<bool> {
        Ok(true)
    }

    fn volatile(&self) -> bool {
        !self.tracked
    }

    fn describe(&self, name: &str) -> String {
        name.to_string()
    }
}
