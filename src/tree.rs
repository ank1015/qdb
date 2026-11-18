//! The data model, and the storage layer under it.
//!
//! ```text
//! Database = one tree with a root folder
//! Node     = Folder | File
//! Folder   = { id, name, parent, children: name -> Node }
//! File     = { id, name, parent, content: bytes of UTF-8 text }
//! ```
//!
//! Every node has an internal id that never changes; name and parent are
//! properties, so a move keeps the node. The one piece of metadata is a
//! modification time per node, in nanoseconds since 1970 UTC; size and line
//! count are derived from content. Two files may use the same pieces (after
//! `cp`): that is storage detail and never shows.
//!
//! # Where things are kept
//!
//! The tree itself is in memory: every node, and for every file a list of
//! *pieces*. A piece says where a stretch of the file's content is in a
//! data file. Content is never held here; it is read through `disk.rs`
//! when a command asks for it. A file's pieces each end on a line
//! boundary, so a command that works by lines can take them one at a
//! time, and a change to one line of a large file replaces one piece.
//!
//! Every change to the tree made by one command line is written to the
//! data file as a single record when the line ends (`commit`). After a
//! crash, `open` reads the last saved copy of the tree (a checkpoint) and
//! applies the records written after it; a command line whose record is
//! not whole is as if it had never run.
//!
//! A database made with `Db::new` keeps its files in memory
//! (`vfs::MemFs`) and writes no log; everything else is the same code.

use crate::bytes::count_newlines;
use crate::disk::{self, damage, Damage, Packed, PackedWriter, Piece, Reader, Segment, Stored, DEAD, REC_CONTENT, REC_HEADER, REC_META, REC_NEXT, SEG_HEADER, SHARED};
use crate::index::{self, Index, Lists, Postings};
pub use crate::index::{IndexStats, IndexWork};
use crate::query::{Query, Trigram};
use std::cell::Cell;
use std::rc::Rc;
use crate::vfs::{MemFs, RealFs, Vfs};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io;

pub type NodeId = u64;

/// `/dev/null`: the one path that is not in the database. It resolves to
/// this id, which reads as an empty file and ignores what is written.
pub const NULL: NodeId = NodeId::MAX - 2;

static NULL_NODE: std::sync::OnceLock<Node> = std::sync::OnceLock::new();

#[derive(Debug)]
pub enum NodeKind {
    /// Children are keyed by name. A BTreeMap keeps them in byte order, which
    /// is the "sorted by name" order of the specification (LC_ALL=C).
    Folder(BTreeMap<Vec<u8>, NodeId>),
    File(Text),
}

/// A file's content: where its pieces are.
#[derive(Debug, Default)]
pub struct Text {
    pieces: Vec<Piece>,
    /// Bytes of text in all pieces.
    len: u64,
    /// Goes up by one with every change to `pieces`. The log names the
    /// version each change produces, so that applying a change twice
    /// can be told from applying it once.
    ver: u32,
    /// Reads as empty until the command line ends. See `Db::veil`.
    veiled: bool,
}

#[derive(Debug)]
pub struct Node {
    pub id: NodeId,
    pub name: Vec<u8>,
    pub parent: Option<NodeId>,
    pub kind: NodeKind,
    /// Bookkeeping, not part of the data model: for a folder, how many
    /// files lie anywhere beneath it.
    aux: u32,
    /// When the node was last modified: nanoseconds since 1970, UTC.
    pub mtime: i64,
}

/// Every node by id. Ids are handed out in sequence and never reused, so
/// this is an array in pages; a page with no node left is given back.
#[derive(Default)]
struct Nodes {
    pages: Vec<Option<Box<Page>>>,
    len: usize,
}

const PAGE: usize = 1024;

struct Page {
    slots: [Option<Box<Node>>; PAGE],
    used: usize,
}

impl Nodes {
    #[inline]
    fn get(&self, id: NodeId) -> Option<&Node> {
        if id == NULL {
            return Some(NULL_NODE.get_or_init(|| Node {
                id: NULL,
                name: b"null".to_vec(),
                parent: None,
                kind: NodeKind::File(Text::default()),
                aux: 0,
                mtime: 0,
            }));
        }
        self.pages.get(id as usize / PAGE)?.as_ref()?.slots[id as usize % PAGE].as_deref()
    }

    #[inline]
    fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.pages.get_mut(id as usize / PAGE)?.as_mut()?.slots[id as usize % PAGE].as_deref_mut()
    }

    fn insert(&mut self, node: Node) {
        let (p, s) = (node.id as usize / PAGE, node.id as usize % PAGE);
        if self.pages.len() <= p {
            self.pages.resize_with(p + 1, || None);
        }
        let page = self.pages[p].get_or_insert_with(|| Box::new(Page { slots: std::array::from_fn(|_| None), used: 0 }));
        if page.slots[s].replace(Box::new(node)).is_none() {
            page.used += 1;
            self.len += 1;
        }
    }

    fn remove(&mut self, id: NodeId) -> Option<Node> {
        let p = id as usize / PAGE;
        let page = self.pages.get_mut(p)?.as_mut()?;
        let node = page.slots[id as usize % PAGE].take()?;
        page.used -= 1;
        self.len -= 1;
        if page.used == 0 {
            self.pages[p] = None;
        }
        Some(*node)
    }

    fn iter(&self) -> impl Iterator<Item = &Node> {
        self.pages.iter().flatten().flat_map(|p| p.slots.iter().flatten().map(|n| &**n))
    }

    /// Up to `max` ids at or after `from`, ascending.
    fn ids_from(&self, from: NodeId, max: usize) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut p = from as usize / PAGE;
        let mut s = from as usize % PAGE;
        while p < self.pages.len() && out.len() < max {
            if let Some(page) = &self.pages[p] {
                while s < PAGE && out.len() < max {
                    if page.slots[s].is_some() {
                        out.push((p * PAGE + s) as NodeId);
                    }
                    s += 1;
                }
                if s < PAGE {
                    break;
                }
            }
            p += 1;
            s = 0;
        }
        out
    }
}

impl std::ops::Index<NodeId> for Nodes {
    type Output = Node;

    #[inline]
    fn index(&self, id: NodeId) -> &Node {
        self.get(id).expect("no such node")
    }
}

/// A file that may contain a match, with its path below the folder that
/// was asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: NodeId,
    pub path: Vec<u8>,
}

/// The answer to "which files under this folder can match this query?"
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Candidates {
    /// Any of them: search every file under the folder.
    All,
    /// Only these, listed in the order a walk of the folder visits them
    /// (depth first, names in byte order). The list may hold files that
    /// turn out not to match; it never lacks one that does. The shortlist
    /// says which pieces of them can match.
    Files(Vec<Candidate>, Shortlist),
    /// A good share of them: walk the folder and ask `Db::passes` of each
    /// file. A file that does not pass cannot match.
    Among(Shortlist),
}

/// A set of pieces that may contain a match. `Db::passes` and
/// `Db::piece_passes` read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortlist {
    /// By data file: positions of pieces, ascending.
    by_seg: Vec<Vec<u32>>,
}

/// The errors a tree operation can produce, named after the errno a
/// filesystem would give so that commands can print GNU's wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Errno {
    NoEnt,
    NotDir,
    IsDir,
    Exist,
    NotEmpty,
    Inval,
}

impl Errno {
    pub fn msg(self) -> &'static str {
        match self {
            Errno::NoEnt => "No such file or directory",
            Errno::NotDir => "Not a directory",
            Errno::IsDir => "Is a directory",
            Errno::Exist => "File exists",
            Errno::NotEmpty => "Directory not empty",
            Errno::Inval => "Invalid argument",
        }
    }
}

/// When a command line's changes are forced to the disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Before the command line returns.
    Always,
    /// In groups: see `Config::batch_lines`.
    Batch,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// The most text one piece holds, unless a single line is longer.
    pub piece: usize,
    /// How large a data file grows before the next is started.
    pub segment: u64,
    pub sync: SyncMode,
    /// With `SyncMode::Batch`: sync once this many command lines have
    /// been logged since the last sync, or `batch_bytes` written.
    pub batch_lines: u32,
    pub batch_bytes: u64,
    /// Keep the search index.
    pub index: bool,
    /// Save the tree once the log written since the last save is longer
    /// than this, and longer than twice the saved tree.
    pub checkpoint: u64,
    /// Output on its way to a file is held in memory up to this many
    /// bytes; more than that waits in a scratch file.
    pub spool: usize,
    /// Move content out of data files that are mostly unused, as writes
    /// go on. Off only to measure what it costs.
    pub upkeep: bool,
    /// Store each piece compressed (LZ4) when that makes it smaller.
    /// Applies to what is written from now on; what is there is read
    /// whichever way it was stored.
    pub compress: bool,
    /// Do not index content as it is written, but later and in bulk:
    /// once this many megabytes are waiting, when a data file is
    /// finished, and at close. Until then searches read what waits.
    pub index_later: bool,
}

/// With `index_later`: how much text may wait to be indexed.
const INDEX_WAIT: u64 = 16 << 20;

impl Default for Config {
    fn default() -> Config {
        Config { piece: 64 * 1024, segment: 128 << 20, sync: SyncMode::Always, batch_lines: 64, batch_bytes: 4 << 20, index: true, checkpoint: 1 << 20, spool: 8 << 20, upkeep: true, compress: false, index_later: false }
    }
}

/// A place for scratch files, and how much may be held in memory before
/// one is used. Without a place, everything is held in memory.
#[derive(Clone)]
pub struct Scratch {
    pub vfs: Option<Rc<dyn Vfs>>,
    pub limit: usize,
    /// Bytes written to scratch files, for the cost report.
    pub used: Rc<Cell<u64>>,
}

impl Scratch {
    /// No place: everything is held in memory.
    pub fn none() -> Scratch {
        Scratch { vfs: None, limit: 0, used: Rc::default() }
    }
}

/// What one unit of work used. Everything but the two times is a count,
/// and is the same every time the same unit runs on the same database.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    /// Bytes of file content looked at, and in how many pieces.
    pub text_bytes: u64,
    pub text_pieces: u64,
    /// Bytes of index lists read, and for how many trigrams.
    pub index_bytes: u64,
    pub index_lookups: u64,
    /// Files and folders reached: every step of a path that was looked
    /// up, and every entry of a folder that was listed.
    pub tree_nodes: u64,
    /// Bytes added to the data files as content.
    pub content_written: u64,
    /// Bytes added to the data files as records of what changed.
    pub records_written: u64,
    /// Bytes written by upkeep that fell due in this unit: index files,
    /// saves of the tree, content moved to free space.
    pub upkeep_written: u64,
    /// Calls that waited for the disk.
    pub syncs: u64,
    /// Bytes that went to scratch files.
    pub scratch_bytes: u64,
    /// The most the engine's own memory rose above where it stood when
    /// the unit began. 0 unless the program counts allocations (`mem`).
    pub memory: u64,
    /// Measured, so different from run to run: processor time and
    /// elapsed time, in nanoseconds.
    pub cpu_ns: u64,
    pub elapsed_ns: u64,
}

impl Cost {
    /// The counts without the two times: what must repeat exactly.
    pub fn counts(&self) -> Cost {
        Cost { cpu_ns: 0, elapsed_ns: 0, ..*self }
    }

    /// Add another unit's cost; memory is the larger of the two.
    pub fn add(&mut self, o: &Cost) {
        self.text_bytes += o.text_bytes;
        self.text_pieces += o.text_pieces;
        self.index_bytes += o.index_bytes;
        self.index_lookups += o.index_lookups;
        self.tree_nodes += o.tree_nodes;
        self.content_written += o.content_written;
        self.records_written += o.records_written;
        self.upkeep_written += o.upkeep_written;
        self.syncs += o.syncs;
        self.scratch_bytes += o.scratch_bytes;
        self.memory = self.memory.max(o.memory);
        self.cpu_ns += o.cpu_ns;
        self.elapsed_ns += o.elapsed_ns;
    }

    /// Every name and value, in the order `--cost` prints them.
    pub fn fields(&self) -> [(&'static str, u64); 13] {
        [
            ("text_bytes", self.text_bytes),
            ("text_pieces", self.text_pieces),
            ("index_bytes", self.index_bytes),
            ("index_lookups", self.index_lookups),
            ("tree_nodes", self.tree_nodes),
            ("content_written", self.content_written),
            ("records_written", self.records_written),
            ("upkeep_written", self.upkeep_written),
            ("syncs", self.syncs),
            ("scratch_bytes", self.scratch_bytes),
            ("memory", self.memory),
            ("cpu_us", self.cpu_ns / 1000),
            ("elapsed_us", self.elapsed_ns / 1000),
        ]
    }
}

impl std::fmt::Display for Cost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (name, value)) in self.fields().iter().enumerate() {
            write!(f, "{}{}={}", if i == 0 { "" } else { " " }, name, value)?;
        }
        Ok(())
    }
}

/// Running counts of what was read, behind `Cost`.
#[derive(Default)]
struct Tally {
    text_bytes: Cell<u64>,
    text_pieces: Cell<u64>,
    index_bytes: Cell<u64>,
    index_lookups: Cell<u64>,
    tree_nodes: Cell<u64>,
    scratch: Rc<Cell<u64>>,
}

/// Where the counts stood when a unit began.
struct UnitStart {
    reads: [u64; 6],
    io: IoStats,
    live: usize,
    cpu: u64,
    at: std::time::Instant,
}

#[inline]
fn bump(c: &Cell<u64>, by: u64) {
    c.set(c.get() + by);
}

/// What opening a database found and did.
#[derive(Debug, Default, Clone)]
pub struct OpenReport {
    /// The directory was empty and a new database was made in it.
    pub created: bool,
    /// Command lines applied from the log after the saved tree.
    pub replayed: u64,
    /// Bytes at the end of the log that were not a whole command line and
    /// were cut off.
    pub discarded: u64,
    /// Data files whose index was missing, damaged or incomplete and was
    /// built again, and how much text that took reading.
    pub rebuilt: Vec<String>,
    pub rebuilt_bytes: u64,
    /// Files left behind by something unfinished, now removed.
    pub removed: Vec<String>,
    pub nodes: usize,
    pub data_files: usize,
}

impl OpenReport {
    /// What to tell the user: nothing after a clean close.
    pub fn messages(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.discarded > 0 {
            out.push(format!("recovery: cut off {} bytes at the end of the log that were not a whole command line", self.discarded));
        }
        if !self.rebuilt.is_empty() {
            out.push(format!("recovery: built the search index again for {} ({:.1} MB of text read)", self.rebuilt.join(", "), self.rebuilt_bytes as f64 / 1e6));
        }
        if !self.removed.is_empty() {
            out.push(format!("recovery: removed leftover files: {}", self.removed.join(", ")));
        }
        if !out.is_empty() || self.replayed > 0 && self.discarded > 0 {
            out.insert(0, format!("recovery: the database was not closed; {} command lines applied from the log", self.replayed));
        }
        out
    }
}

/// Bytes on disk by what they are.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DiskStats {
    /// Content that belongs to a file.
    pub text: u64,
    /// Content no file uses any more, and other space in data files that
    /// a compaction would give back.
    pub unreclaimed: u64,
    /// Headers of data files and of the records in them.
    pub framing: u64,
    /// The log: one record per command line.
    pub log: u64,
    /// Posting lists.
    pub lists: u64,
    /// The tables that find a list by trigram, and checksums of the
    /// index files.
    pub tables: u64,
    /// The saved tree and MANIFEST.
    pub tree: u64,
    pub data_files: usize,
    /// Bytes of text as the files hold it (before any compression).
    pub raw_text: u64,
}

impl DiskStats {
    pub fn total(&self) -> u64 {
        self.text + self.unreclaimed + self.framing + self.log + self.lists + self.tables + self.tree
    }
}

/// Running totals of what was written and synced.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IoStats {
    pub syncs: u64,
    pub dir_syncs: u64,
    pub content_bytes: u64,
    pub log_bytes: u64,
    pub index_bytes: u64,
    pub tree_bytes: u64,
    pub checkpoints: u64,
    pub commits: u64,
    /// Of `content_bytes`: content written again to free the space
    /// around it.
    pub moved_bytes: u64,
}

impl IoStats {
    pub fn bytes(&self) -> u64 {
        self.content_bytes + self.log_bytes + self.index_bytes + self.tree_bytes
    }
}

const OP_MKDIR: u64 = 1;
const OP_MKFILE: u64 = 2;
const OP_SPLICE: u64 = 3;
const OP_REMOVE: u64 = 4;
const OP_RENAME: u64 = 5;
const OP_MTIME: u64 = 6;

/// The most content one record holds. A file up to this size is stored
/// in one stretch and can be read without joining its pieces.
const RECORD_MAX: usize = 64 << 20;

pub struct Db {
    nodes: Nodes,
    root: NodeId,
    next_id: NodeId,
    vfs: Rc<dyn Vfs>,
    /// The files are in memory and there is no log.
    volatile: bool,
    cfg: Config,
    /// Data files by number; number 0 is never used.
    segs: Vec<Option<Box<Segment>>>,
    /// The data file being written.
    active: u32,
    /// False while searches are told to read every file although the
    /// index is there and kept current. See `set_index_consulted`.
    consult: bool,
    /// What the command line now running has changed in the tree, as it
    /// will go into the log.
    ops: Vec<u8>,
    dirty: bool,
    /// Counts every change to the tree or to content; what is remembered
    /// about the tree is good for one value of it.
    epoch: u64,
    /// Totals a command worked out over whole folders, with the epoch
    /// they are good for.
    totals: std::cell::RefCell<(u64, std::collections::HashMap<(NodeId, u8), u64>)>,
    /// Pieces it has taken out of files. Their content is forgotten when
    /// the command line ends.
    retired: Vec<(NodeId, Piece)>,
    veils: Vec<NodeId>,
    /// Nodes whose modification time it has set. The times go into the
    /// log once, when the command line ends.
    timed: std::collections::HashSet<NodeId>,
    /// The time, when a test has set it; otherwise the system clock is
    /// read.
    clock: Option<i64>,
    /// Number of the last command line logged.
    seq: u64,
    /// How much of the active data file is known to be on disk.
    synced: u64,
    unsynced_lines: u32,
    /// Bytes logged since the tree was last saved.
    log_since: u64,
    /// The current checkpoint, the data file the log continues in from
    /// there, and its size.
    ckp: u32,
    ckp_seg: u32,
    ckp_size: u64,
    live_pieces: u64,
    failed: Option<String>,
    /// Work done by indexes that are no longer in memory.
    work: IndexWork,
    io: IoStats,
    tally: Tally,
    unit: Option<UnitStart>,
    last_cost: Cost,
    /// Upkeep under way, done a step at a time as command lines end: an
    /// index file being written, the tree being saved, and the data file
    /// whose content is being moved out so that it can be removed (and
    /// the position reached in it).
    idx_job: Option<IdxJob>,
    ckp_job: Option<CkpJob>,
    clean: Option<(u32, u32)>,
    /// Bytes of content that may be moved now: earned by writing.
    clean_credit: u64,
    /// Content and log bytes written when the last command line ended.
    mark: u64,
    /// Bytes of text written and not yet indexed (`index_later`).
    waiting: u64,
    /// Files that are no longer part of the database and are being
    /// removed: cut shorter a step at a time, since removing a large
    /// file in one go takes the operating system a long moment.
    dying: Vec<(String, Box<dyn crate::vfs::VFile>)>,
}

/// How much shorter a file being removed gets per step.
const DYING_STEP: u64 = 1 << 20;

struct IdxJob {
    seg: u32,
    w: PackedWriter,
}

struct CkpJob {
    w: disk::CheckpointWriter,
    /// Nodes below this id are written.
    cursor: NodeId,
    /// The data file the log went on in when the save began.
    seg: u32,
    /// `log_since` when the save began.
    log_before: u64,
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Db {
    /// A copy in memory with the same nodes, ids and content.
    fn clone(&self) -> Db {
        let mut cfg = self.cfg.clone();
        cfg.index = self.has_index();
        let mut db = Db::in_memory(cfg);
        db.next_id = self.next_id;
        db.clock = self.clock;
        db.nodes.get_mut(0).unwrap().mtime = self.nodes[self.root].mtime;
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id];
            match &node.kind {
                NodeKind::Folder(children) => {
                    if id != self.root {
                        db.nodes.insert(Node { id, name: node.name.clone(), parent: node.parent, kind: NodeKind::Folder(BTreeMap::new()), aux: 0, mtime: node.mtime });
                        db.link(id);
                    }
                    stack.extend(children.values().rev().copied());
                }
                NodeKind::File(_) => {
                    db.nodes.insert(Node { id, name: node.name.clone(), parent: node.parent, kind: NodeKind::File(Text::default()), aux: 0, mtime: node.mtime });
                    db.link(id);
                    db.count_files(node.parent, 1);
                    let content = self.content(id);
                    let pieces = db.store(id, &content);
                    db.splice(id, 0, 0, pieces);
                }
            }
        }
        db.commit();
        db
    }
}

/// Content must be valid UTF-8 with no NUL bytes.
pub fn is_valid_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

/// `is_valid_text` for text that arrives in parts.
pub struct TextCheck {
    valid: bool,
    /// The start of a character whose other bytes have not arrived.
    carry: Vec<u8>,
}

impl Default for TextCheck {
    fn default() -> TextCheck {
        TextCheck { valid: true, carry: Vec::new() }
    }
}

impl TextCheck {
    /// The next part. False once the text cannot be valid.
    pub fn feed(&mut self, mut data: &[u8]) -> bool {
        if !self.valid {
            return false;
        }
        if memchr::memchr(0, data).is_some() {
            self.valid = false;
            return false;
        }
        if !self.carry.is_empty() {
            let whole = match self.carry[0] {
                0xC2..=0xDF => 2,
                0xE0..=0xEF => 3,
                _ => 4,
            };
            let take = (whole - self.carry.len()).min(data.len());
            self.carry.extend_from_slice(&data[..take]);
            data = &data[take..];
            match std::str::from_utf8(&self.carry) {
                Ok(_) => self.carry.clear(),
                Err(e) if e.error_len().is_none() => return true,
                Err(_) => {
                    self.valid = false;
                    return false;
                }
            }
        }
        if let Err(e) = std::str::from_utf8(data) {
            if e.error_len().is_none() {
                self.carry.extend_from_slice(&data[e.valid_up_to()..]);
            } else {
                self.valid = false;
            }
        }
        self.valid
    }

    /// The text has ended. True if all of it was valid text.
    pub fn finish(&self) -> bool {
        self.valid && self.carry.is_empty()
    }
}

/// Names follow POSIX: anything except `/` and NUL, and not empty.
pub fn is_valid_name(name: &[u8]) -> bool {
    !name.is_empty() && name != b"." && name != b".." && !name.contains(&b'/') && !name.contains(&0)
}

/// Where to cut content into pieces: each at most `limit` bytes and
/// ending after a newline, except that a line longer than `limit` is a
/// piece of its own and the last piece ends where the content does.
fn cut(data: &[u8], limit: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while data.len() - at > limit {
        let end = match memchr::memrchr(b'\n', &data[at..at + limit]) {
            Some(i) => at + i + 1,
            None => memchr::memchr(b'\n', &data[at + limit..]).map_or(data.len(), |i| at + limit + i + 1),
        };
        out.push((at, end));
        at = end;
    }
    if at < data.len() {
        out.push((at, data.len()));
    }
    out
}

fn io_err(e: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e)
}

impl Db {
    /// An empty database in memory, with the search index on.
    pub fn new() -> Db {
        Db::in_memory(Config::default())
    }

    /// An empty database in memory that answers every search by scanning.
    pub fn without_index() -> Db {
        Db::in_memory(Config { index: false, ..Config::default() })
    }

    pub fn in_memory(cfg: Config) -> Db {
        Db::open(Box::new(MemFs::volatile()), cfg).expect("a database in memory").0
    }

    /// Open the database in a directory, making the directory and a new
    /// database if there is none.
    pub fn open_dir(path: &std::path::Path, cfg: Config) -> Result<(Db, OpenReport), String> {
        std::fs::create_dir_all(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        Db::open(Box::new(RealFs::new(path)), cfg)
    }

    fn blank(vfs: Box<dyn Vfs>, cfg: Config) -> Db {
        let vfs: Rc<dyn Vfs> = Rc::from(vfs);
        let mut nodes = Nodes::default();
        nodes.insert(Node { id: 0, name: Vec::new(), parent: None, kind: NodeKind::Folder(BTreeMap::new()), aux: 0, mtime: 0 });
        let volatile = vfs.volatile();
        Db {
            nodes,
            root: 0,
            next_id: 1,
            vfs,
            volatile,
            cfg,
            segs: vec![None],
            active: 0,
            consult: true,
            ops: Vec::new(),
            dirty: false,
            epoch: 0,
            totals: Default::default(),
            retired: Vec::new(),
            veils: Vec::new(),
            timed: Default::default(),
            clock: None,
            seq: 0,
            synced: SEG_HEADER,
            unsynced_lines: 0,
            log_since: 0,
            ckp: 0,
            ckp_seg: 1,
            ckp_size: 0,
            live_pieces: 0,
            failed: None,
            work: IndexWork::default(),
            io: IoStats::default(),
            tally: Tally::default(),
            unit: None,
            last_cost: Cost::default(),
            idx_job: None,
            ckp_job: None,
            clean: None,
            clean_credit: 0,
            mark: 0,
            waiting: 0,
            dying: Vec::new(),
        }
    }

    // ------------------------------------------------------- open, close

    /// Open a database on a directory, or make one if the directory is
    /// empty. After a crash this is where recovery happens: the saved
    /// tree is read, the command lines logged after it are applied, and
    /// an unfinished one at the very end is cut off. Damage anywhere else
    /// is an error that names the file.
    pub fn open(vfs: Box<dyn Vfs>, cfg: Config) -> Result<(Db, OpenReport), String> {
        // Reads cannot return an error, so damage found while reading is
        // thrown; here it becomes the error it is.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Db::open_inner(vfs, cfg))) {
            Ok(r) => r,
            Err(payload) => match payload.downcast::<Damage>() {
                Ok(d) => Err(d.0),
                Err(other) => std::panic::resume_unwind(other),
            },
        }
    }

    fn open_inner(vfs: Box<dyn Vfs>, cfg: Config) -> Result<(Db, OpenReport), String> {
        if !vfs.lock().map_err(|e| e.to_string())? {
            return Err("the database is in use by another process".into());
        }
        let mut db = Db::blank(vfs, cfg);
        let mut report = OpenReport::default();
        let io = |e: io::Error| e.to_string();
        let names = db.vfs.list().map_err(io)?;
        let number = match disk::read_manifest(&*db.vfs)? {
            Some(n) => n,
            None => {
                if let Some(other) = names.iter().find(|n| *n != "LOCK" && !n.ends_with(".tmp")) {
                    return Err(format!("{}: there are files here but no MANIFEST: this is not a qdb database, or its MANIFEST is lost", db.vfs.describe(other)));
                }
                let mut seg = Segment::create(&*db.vfs, 1, db.cfg.segment).map_err(io)?;
                seg.sealed = false;
                if db.cfg.index {
                    seg.dynamic = Some(Index::new());
                }
                db.segs.push(Some(Box::new(seg)));
                db.active = 1;
                if !db.volatile {
                    db.checkpoint().map_err(io)?;
                }
                report.created = true;
                report.nodes = 1;
                report.data_files = 1;
                return Ok((db, report));
            }
        };

        // The saved tree.
        let (entries, size) = disk::read_checkpoint(&*db.vfs, number)?;
        let ckp_file = db.vfs.describe(&disk::ckp_name(number));
        db.ckp = number;
        db.ckp_size = size;
        db.nodes = Nodes::default();
        let mut r = Reader::new(&entries);
        let (mut log_seg, mut log_off, mut log_ords) = (0u32, 0u64, 0u32);
        while !r.done() {
            match r.u64() {
                0 => {
                    db.seq = r.u64();
                    log_seg = r.u32();
                    log_off = r.u64();
                    log_ords = r.u32();
                    db.next_id = r.u64();
                }
                tag @ (1 | 2) => {
                    let id = r.u64();
                    let parent = r.u64().checked_sub(1);
                    let name = r.bytes().to_vec();
                    let mtime = r.u64() as i64;
                    let kind = if tag == 1 {
                        NodeKind::Folder(BTreeMap::new())
                    } else {
                        let ver = r.u32();
                        let n = r.u64() as usize;
                        let mut pieces = Vec::with_capacity(n.min(1 << 20));
                        let mut len = 0u64;
                        for _ in 0..n {
                            let p = Piece::decode(&mut r);
                            len += p.len as u64;
                            pieces.push(p);
                        }
                        NodeKind::File(Text { pieces, len, ver, veiled: false })
                    };
                    db.nodes.insert(Node { id, name, parent, kind, aux: 0, mtime });
                }
                _ => r.bad = true,
            }
            if r.bad {
                return Err(format!("{}: damaged: an entry cannot be read", ckp_file));
            }
        }
        if log_seg == 0 || db.nodes.get(0).is_none() {
            return Err(format!("{}: damaged: it does not say where the log goes on, or has no root", ckp_file));
        }
        db.ckp_seg = log_seg;

        // The log from there: one data file after another.
        let (mut cur, mut off, mut ords) = (log_seg, log_off, log_ords);
        loop {
            let mut seg = match Segment::open(&*db.vfs, cur, db.cfg.segment) {
                Ok(s) => s,
                Err(_) if cur != log_seg => {
                    // The file before this one named it, and then the
                    // power went before it was made, or before its first
                    // bytes were on disk. Nothing can have been logged in
                    // it: make it now.
                    let mut s = Segment::create(&*db.vfs, cur, db.cfg.segment).map_err(io)?;
                    s.sync().map_err(io)?;
                    db.vfs.sync_dir().map_err(io)?;
                    report.removed.push(format!("{} (made again)", disk::seg_name(cur)));
                    s
                }
                Err(e) => return Err(e.to_string()),
            };
            if off > seg.end {
                return Err(format!("{}: is cut short: the saved tree says the log goes on at offset {} and the file has {} bytes", seg.name(), off, seg.end));
            }
            let start = off;
            let mut at = off;
            let mut known = start;
            let mut next = None;
            // (offset, end) of content records; (end, pieces so far, ops) of
            // command lines.
            let mut contents: Vec<(u64, u64)> = Vec::new();
            let mut lines: Vec<(u64, u32, Vec<u8>)> = Vec::new();
            let mut seq = db.seq;
            while let Some(rec) = seg.record_at(at) {
                match rec.kind {
                    REC_CONTENT => {
                        let first = u32::from_le_bytes(rec.payload[0..4].try_into().unwrap());
                        if first != ords {
                            break;
                        }
                        ords += u32::from_le_bytes(rec.payload[4..8].try_into().unwrap());
                        contents.push((at, rec.end));
                    }
                    REC_META => {
                        if u64::from_le_bytes(rec.payload[0..8].try_into().unwrap()) != seq + 1 {
                            break;
                        }
                        seq += 1;
                        let s = u64::from_le_bytes(rec.payload[8..16].try_into().unwrap());
                        if s > known && s <= at {
                            known = s;
                        }
                        lines.push((rec.end, ords, rec.payload[16..].to_vec()));
                    }
                    REC_NEXT => {
                        next = Some(u32::from_le_bytes(rec.payload[0..4].try_into().unwrap()));
                        break;
                    }
                    _ => break,
                }
                at = rec.end;
            }
            // A data file is synced before the next is made. If it names
            // a next one that is not there (or is not whole), the power
            // went in between, and this file's end may not have been
            // synced either: it is looked at like a last file.
            let next_missing = next.is_some_and(|n| Segment::open(&*db.vfs, n, db.cfg.segment).is_err());
            let last_file = next.is_none() || next_missing;
            let mut limit = at;
            if last_file {
                // What was written after the last sync we know of may be
                // there only in part: look at the content itself.
                for &(c_at, _) in contents.iter().filter(|c| c.0 >= known) {
                    let rec = seg.record_at(c_at).expect("read a moment ago");
                    if !seg.content_intact(c_at, &rec) {
                        limit = c_at;
                        break;
                    }
                }
            }
            if next_missing && limit < at {
                // Not all of it is there: the log ends in this file.
                next = None;
            }
            let (mut good_end, mut good_ords) = (start, if cur == log_seg { log_ords } else { 0 });
            for (end, ords_then, ops) in &lines {
                if *end > limit {
                    break;
                }
                db.replay(ops).map_err(|e| format!("{}: damaged: {}", seg.name(), e))?;
                // What was replayed is log that the next save of the
                // tree will make unnecessary.
                db.log_since += ops.len() as u64 + 16 + REC_HEADER;
                db.seq += 1;
                report.replayed += 1;
                good_end = *end;
                good_ords = *ords_then;
            }
            while db.segs.len() <= cur as usize {
                db.segs.push(None);
            }
            if let Some(n) = next {
                if n <= cur {
                    return Err(format!("{}: damaged: it names data file {} as the one after it", seg.name(), n));
                }
                db.segs[cur as usize] = Some(Box::new(seg));
                cur = n;
                off = SEG_HEADER;
                ords = 0;
                continue;
            }
            if good_end < seg.end {
                report.discarded = seg.end - good_end;
                seg.truncate(good_end).map_err(io)?;
                seg.sync().map_err(io)?;
            } else if report.replayed > 0 {
                // The log that was read may never have been synced. What
                // recovery writes from here on (an index built again) is
                // made from it, so it must not be able to go away in a
                // second power cut.
                seg.sync().map_err(io)?;
            }
            seg.sealed = false;
            seg.grow_owners(good_ords);
            db.synced = good_end;
            db.active = cur;
            db.segs[cur as usize] = Some(Box::new(seg));
            break;
        }

        // The tree is whole again: put every node in its folder, count
        // files, and note which pieces of which data file are in use.
        let ids: Vec<NodeId> = db.nodes.iter().map(|n| n.id).collect();
        for &id in &ids {
            if id == 0 {
                continue;
            }
            let (parent, name) = (db.nodes[id].parent, db.nodes[id].name.clone());
            let sound = is_valid_name(&name)
                && match parent.and_then(|p| db.nodes.get_mut(p)) {
                    Some(Node { kind: NodeKind::Folder(c), .. }) => c.insert(name, id).is_none(),
                    _ => false,
                };
            if !sound {
                return Err(format!("{}: damaged: node {} has no folder to be in, or a name another node has", ckp_file, id));
            }
        }
        for &id in &ids {
            let pieces = match &db.nodes[id].kind {
                NodeKind::File(t) => t.pieces.clone(),
                NodeKind::Folder(_) => continue,
            };
            db.count_files(db.nodes[id].parent, 1);
            for p in &pieces {
                if p.seg == 0 || p.seg > db.active {
                    return Err(format!("{}: damaged: a file has content in data file {}, which is after the last", ckp_file, p.seg));
                }
                if db.segs[p.seg as usize].is_none() {
                    let seg = Segment::open(&*db.vfs, p.seg, 0).map_err(io)?;
                    db.segs[p.seg as usize] = Some(Box::new(seg));
                }
                let seg = db.segs[p.seg as usize].as_mut().unwrap();
                if p.off as u64 + p.slen as u64 > seg.end {
                    return Err(format!("{}: is cut short: a file has {} bytes at offset {} and the data file has {}", seg.name(), p.slen, p.off, seg.end));
                }
                if seg.add_user(p, id) {
                    db.live_pieces += 1;
                }
            }
        }
        db.next_id = db.next_id.max(ids.iter().max().map_or(1, |m| m + 1));
        report.nodes = ids.len();
        report.data_files = db.segs.iter().flatten().count();

        // Files that belong to nothing: what an unfinished step left.
        for name in &names {
            let number = |suffix: &str| name.strip_suffix(suffix).and_then(|n| n.parse::<u32>().ok());
            let stray = if name.ends_with(".tmp") {
                true
            } else if let Some(n) = number(".seg").or(number(".idx")) {
                db.segs.get(n as usize).map_or(true, |s| s.is_none())
            } else if name.starts_with("tree-") {
                *name != disk::ckp_name(db.ckp)
            } else {
                false
            };
            if stray {
                db.vfs.remove(name).map_err(io)?;
                report.removed.push(name.clone());
            }
        }

        // The index: read what is saved, build what is missing.
        if db.cfg.index {
            for id in 1..db.segs.len() as u32 {
                if db.segs[id as usize].is_none() {
                    continue;
                }
                let packed = match Packed::open(&*db.vfs, id) {
                    Ok(p) => p,
                    Err(why) => {
                        report.removed.push(format!("{} (unusable: {})", disk::idx_name(id), why));
                        None
                    }
                };
                let active = db.active;
                let seg = db.segs[id as usize].as_mut().unwrap();
                let mut packed = packed;
                if packed.as_ref().is_some_and(|p| id == active && p.ndocs as usize > seg.owners.len()) {
                    // It covers pieces the data file no longer has: the
                    // file was cut back below what the index was built
                    // from. New pieces would take those numbers, so it
                    // must go before anything is written.
                    packed = None;
                    db.vfs.remove(&disk::idx_name(id)).map_err(io)?;
                    db.vfs.sync_dir().map_err(io)?;
                    report.removed.push(format!("{} (covers more than its data file holds)", disk::idx_name(id)));
                }
                if let Some(p) = &packed {
                    seg.grow_owners(p.ndocs);
                    seg.indexed = p.ndocs;
                }
                seg.packed = packed;
            }
            let (rebuilt, bytes) = db.index_backlog().map_err(io)?;
            report.rebuilt = rebuilt.iter().map(|id| disk::seg_name(*id)).collect();
            report.rebuilt_bytes = bytes;
        }
        Ok((db, report))
    }

    /// Apply the changes one command line logged. Applying a change that
    /// the tree already has does nothing, so a tree saved while the log
    /// went on comes out right.
    fn replay(&mut self, ops: &[u8]) -> Result<(), String> {
        let mut r = Reader::new(ops);
        while !r.done() {
            match r.u64() {
                op @ (OP_MKDIR | OP_MKFILE) => {
                    let (id, parent, name) = (r.u64(), r.u64(), r.bytes().to_vec());
                    if !r.bad && self.nodes.get(id).is_none() {
                        let kind = if op == OP_MKDIR { NodeKind::Folder(BTreeMap::new()) } else { NodeKind::File(Text::default()) };
                        self.nodes.insert(Node { id, name, parent: Some(parent), kind, aux: 0, mtime: 0 });
                    }
                    self.next_id = self.next_id.max(id + 1);
                }
                OP_SPLICE => {
                    let (id, ver, at, del, n) = (r.u64(), r.u32(), r.u64() as usize, r.u64() as usize, r.u64() as usize);
                    let mut new = Vec::with_capacity(n.min(1 << 20));
                    for _ in 0..n {
                        new.push(Piece::decode(&mut r));
                    }
                    if r.bad {
                        break;
                    }
                    if let Some(Node { kind: NodeKind::File(t), .. }) = self.nodes.get_mut(id) {
                        if ver == t.ver + 1 {
                            if at + del > t.pieces.len() {
                                return Err(format!("the log changes pieces {}..{} of a file that has {}", at, at + del, t.pieces.len()));
                            }
                            t.pieces.splice(at..at + del, new);
                            t.len = t.pieces.iter().map(|p| p.len as u64).sum();
                            t.ver = ver;
                        } else if ver > t.ver {
                            return Err(format!("the log skips from version {} of a file to version {}", t.ver, ver));
                        }
                    }
                }
                OP_REMOVE => {
                    let id = r.u64();
                    if id != 0 {
                        self.nodes.remove(id);
                    }
                }
                OP_RENAME => {
                    let (id, parent, name) = (r.u64(), r.u64(), r.bytes().to_vec());
                    if let Some(node) = self.nodes.get_mut(id).filter(|_| !r.bad && id != 0) {
                        node.parent = Some(parent);
                        node.name = name;
                    }
                }
                OP_MTIME => {
                    let (id, t) = (r.u64(), r.u64() as i64);
                    if let Some(node) = self.nodes.get_mut(id).filter(|_| !r.bad) {
                        node.mtime = t;
                    }
                }
                other => return Err(format!("the log has a change of unknown kind {}", other)),
            }
        }
        if r.bad {
            return Err("a logged change ends in the middle".into());
        }
        Ok(())
    }

    /// Index every piece that is in no index yet, reading its content.
    /// For a data file that is no longer written the index file is then
    /// written too. Returns the data files it did this for and the bytes
    /// of text read.
    fn index_backlog(&mut self) -> io::Result<(Vec<u32>, u64)> {
        self.idx_finish()?;
        self.waiting = 0;
        let behind: Vec<u32> = self.segs.iter().flatten().filter(|s| (s.indexed as usize) < s.owners.len() || (s.sealed && s.dynamic.is_some())).map(|s| s.id).collect();
        for seg in self.segs.iter_mut().flatten() {
            if seg.dynamic.is_none() && (!seg.sealed || behind.contains(&seg.id)) {
                seg.dynamic = Some(Index::starting_after(seg.indexed));
            }
        }
        if behind.is_empty() {
            return Ok((behind, 0));
        }
        // The pieces to index, by data file and position.
        let mut todo: Vec<Piece> = Vec::new();
        for node in self.nodes.iter() {
            if let NodeKind::File(t) = &node.kind {
                todo.extend(t.pieces.iter().filter(|p| p.ord >= self.segs[p.seg as usize].as_ref().unwrap().indexed));
            }
        }
        // And those the command line now running has taken out of files:
        // until its record is in the log they are still in use as far as
        // the disk goes, and an index file must have them.
        todo.extend(self.retired.iter().map(|(_, p)| p).filter(|p| self.segs[p.seg as usize].as_ref().is_some_and(|s| p.ord >= s.indexed && s.owners[p.ord as usize] != DEAD)).cloned());
        todo.sort_unstable_by_key(|p| (p.seg, p.ord));
        todo.dedup_by_key(|p| (p.seg, p.ord));
        let mut bytes = 0u64;
        let mut at = 0;
        for &id in &behind {
            let mut index = self.segs[id as usize].as_mut().unwrap().dynamic.take().unwrap();
            let total = self.segs[id as usize].as_ref().unwrap().owners.len() as u32;
            for ord in index.next_doc() - 1..total {
                // A position no file uses still takes a number.
                if todo.get(at).is_some_and(|p| p.seg == id && p.ord == ord) {
                    let text = self.text_of(&todo[at]);
                    bytes += text.len() as u64;
                    index.add(&text);
                    at += 1;
                } else {
                    let doc = index.add(b"");
                    index.retire(doc);
                }
            }
            let seg = self.segs[id as usize].as_mut().unwrap();
            seg.indexed = total;
            seg.dynamic = Some(index);
            if seg.sealed {
                self.save_index(id)?;
            }
        }
        Ok((behind, bytes))
    }

    /// Write the index file of a data file: what its index file holds
    /// now, if there is one, joined with what is indexed in memory. The
    /// part in memory is then dropped, or started afresh if the data file
    /// is still being written.
    fn save_index(&mut self, id: u32) -> io::Result<()> {
        self.idx_finish()?;
        self.idx_start(id)?;
        self.idx_finish()
    }

    /// Begin writing the index file of a data file, if what is indexed
    /// in memory has anything to add to it. `idx_step` does the writing.
    fn idx_start(&mut self, id: u32) -> io::Result<()> {
        debug_assert!(self.idx_job.is_none());
        let seg = self.segs[id as usize].as_mut().unwrap();
        let fresh = seg.dynamic.as_ref().is_some_and(|d| d.next_doc() - 1 > seg.packed.as_ref().map_or(0, |p| p.ndocs));
        if fresh {
            let w = PackedWriter::new(&*self.vfs, id, seg.packed.as_ref(), seg.dynamic.as_ref())?;
            self.idx_job = Some(IdxJob { seg: id, w });
        } else if seg.sealed {
            seg.dynamic = None;
        } else if seg.dynamic.is_none() {
            seg.dynamic = Some(Index::starting_after(seg.indexed));
        }
        Ok(())
    }

    /// Write about `budget` more bytes of the index file under way. When
    /// it is whole it replaces the old one, and the part of the index
    /// that was in memory is dropped, or started afresh if the data file
    /// is still being written.
    fn idx_step(&mut self, budget: usize) -> io::Result<()> {
        let Some(mut job) = self.idx_job.take() else { return Ok(()) };
        let id = job.seg;
        let Some(seg) = self.segs[id as usize].as_mut() else {
            // The data file is gone, and its index with it.
            drop(job);
            let _ = self.vfs.remove(&disk::idx_tmp(id));
            return Ok(());
        };
        let before = job.w.syncs;
        let done = job.w.step(seg.packed.as_ref(), seg.dynamic.as_ref(), budget)?;
        self.io.syncs += job.w.syncs - before;
        if !done {
            self.idx_job = Some(job);
            return Ok(());
        }
        // The lists no longer name pieces that files stopped using while
        // they were being written. If the log of that were lost in a
        // power cut, those pieces would be in use again and missing from
        // the index: so the log is synced before the index file is given
        // its name.
        self.sync_log()?;
        let seg = self.segs[id as usize].as_mut().unwrap();
        job.w.finish(&*self.vfs, seg.indexed)?;
        self.io.syncs += 1;
        self.io.dir_syncs += 1;
        let packed = Packed::open(&*self.vfs, id).map_err(io_err)?.ok_or_else(|| io_err("the index file just written is not there".into()))?;
        let sizes = packed.sizes();
        self.io.index_bytes += sizes.0 + sizes.1 + sizes.2;
        self.work.add(&seg.dynamic.as_ref().unwrap().work());
        seg.packed = Some(packed);
        seg.dynamic = if seg.sealed { None } else { Some(Index::starting_after(seg.indexed)) };
        Ok(())
    }

    /// Sync what has been logged, if anything has been since the last
    /// sync.
    fn sync_log(&mut self) -> io::Result<()> {
        if self.volatile {
            return Ok(());
        }
        let active = self.segs[self.active as usize].as_mut().unwrap();
        if active.end > self.synced {
            active.sync()?;
            self.io.syncs += 1;
            self.synced = active.end;
            self.unsynced_lines = 0;
        }
        Ok(())
    }

    /// Write the index file under way to its end.
    fn idx_finish(&mut self) -> io::Result<()> {
        while self.idx_job.is_some() {
            self.idx_step(usize::MAX)?;
        }
        Ok(())
    }

    /// Save the tree: write a new checkpoint, make it the current one,
    /// and remove what it makes unnecessary.
    fn checkpoint(&mut self) -> io::Result<()> {
        // A save under way is given up: this one is newer.
        self.ckp_job = None;
        // The checkpoint says "the log goes on from here", so everything
        // before here has to be on disk first.
        let active = self.segs[self.active as usize].as_mut().unwrap();
        active.sync()?;
        self.io.syncs += 1;
        self.synced = active.end;
        self.unsynced_lines = 0;
        let (log_off, log_ords) = (active.end, active.owners.len() as u32);
        let number = self.ckp + 1;
        let mut w = disk::CheckpointWriter::new(&*self.vfs, number)?;
        let e = w.entry();
        disk::put_u64(e, 0);
        disk::put_u64(e, self.seq);
        disk::put_u64(e, self.active as u64);
        disk::put_u64(e, log_off);
        disk::put_u64(e, log_ords as u64);
        disk::put_u64(e, self.next_id);
        w.end_block()?;
        for node in self.nodes.iter() {
            encode_node(node, w.entry());
            w.entry_done()?;
        }
        let size = w.finish(&*self.vfs)?;
        self.io.syncs += 2;
        self.io.dir_syncs += 2;
        self.io.tree_bytes += size + 20;
        self.io.checkpoints += 1;
        if self.ckp != 0 {
            self.vfs.remove(&disk::ckp_name(self.ckp))?;
        }
        self.ckp = number;
        self.ckp_seg = self.active;
        self.ckp_size = size;
        self.log_since = 0;
        self.drop_empty()
    }

    /// Begin saving the tree a step at a time. The checkpoint names the
    /// place the log has reached now; the nodes are written over the
    /// command lines that follow, each as it is when its turn comes.
    /// Applying the log from the named place to such a tree gives the
    /// right tree, because applying a change twice does nothing.
    fn ckp_start(&mut self) -> io::Result<()> {
        let active = self.segs[self.active as usize].as_ref().unwrap();
        let mut w = disk::CheckpointWriter::new(&*self.vfs, self.ckp + 1)?;
        let e = w.entry();
        disk::put_u64(e, 0);
        disk::put_u64(e, self.seq);
        disk::put_u64(e, self.active as u64);
        disk::put_u64(e, active.end);
        disk::put_u64(e, active.owners.len() as u64);
        disk::put_u64(e, self.next_id);
        w.end_block()?;
        self.ckp_job = Some(CkpJob { w, cursor: 0, seg: self.active, log_before: self.log_since });
        Ok(())
    }

    /// Write about `budget` more bytes of the tree being saved. After
    /// the last node, everything logged so far is synced (the saved tree
    /// may hold any of it) and the checkpoint becomes the current one.
    fn ckp_step(&mut self, budget: u64) -> io::Result<()> {
        let Some(mut job) = self.ckp_job.take() else { return Ok(()) };
        let start = job.w.bytes();
        let synced = job.w.syncs;
        loop {
            let ids = self.nodes.ids_from(job.cursor, 64);
            if ids.is_empty() {
                break;
            }
            for id in ids {
                encode_node(&self.nodes[id], job.w.entry());
                job.w.entry_done()?;
                job.cursor = id + 1;
            }
            if job.w.bytes() - start >= budget {
                self.io.syncs += job.w.syncs - synced;
                self.ckp_job = Some(job);
                return Ok(());
            }
        }
        let active = self.segs[self.active as usize].as_mut().unwrap();
        if active.end > self.synced {
            active.sync()?;
            self.io.syncs += 1;
            self.synced = active.end;
            self.unsynced_lines = 0;
        }
        let number = job.w.number;
        let size = job.w.finish(&*self.vfs)?;
        self.io.syncs += 2;
        self.io.dir_syncs += 2;
        self.io.tree_bytes += size + 20;
        self.io.checkpoints += 1;
        if self.ckp != 0 {
            self.vfs.remove(&disk::ckp_name(self.ckp))?;
        }
        self.ckp = number;
        self.ckp_seg = job.seg;
        self.ckp_size = size;
        self.log_since = self.log_since.saturating_sub(job.log_before);
        self.drop_empty()
    }

    /// Cut `bytes` off the files being removed; one that is empty is
    /// removed.
    fn dying_step(&mut self, mut bytes: u64) -> io::Result<()> {
        while bytes > 0 {
            let Some((name, file)) = self.dying.last_mut() else { break };
            let len = file.len();
            let cut = len.min(bytes);
            if cut > 0 {
                file.truncate(len - cut)?;
                bytes -= cut;
            }
            if len == cut {
                let name = name.clone();
                self.dying.pop();
                self.vfs.remove(&name)?;
            }
        }
        Ok(())
    }

    /// The steps of upkeep that the command line just ended pays for:
    /// `wrote` is how many bytes it wrote.
    fn upkeep(&mut self, wrote: u64) -> io::Result<()> {
        if self.idx_job.is_some() {
            self.idx_step(32 * 1024 + wrote as usize)?;
        }
        if self.waiting > INDEX_WAIT && self.cfg.index {
            self.index_backlog()?;
        }
        if self.volatile {
            return Ok(());
        }
        if !self.dying.is_empty() {
            self.dying_step(DYING_STEP)?;
        }
        if self.ckp_job.is_none() {
            let active = self.active;
            let due = self.log_since > self.cfg.checkpoint.max(2 * self.ckp_size)
                // A data file that holds nothing can be removed once a
                // saved tree is newer than the last use of it.
                || (self.log_since > 0 && self.segs.iter().flatten().any(|s| s.id < active && s.sealed && s.live_pieces == 0));
            if due {
                self.ckp_start()?;
            }
        }
        self.ckp_step(8 * 1024 + wrote / 2)
    }

    /// Move content out of the data file that wastes the most space, as
    /// much as the writing of this command line has earned. The moves go
    /// into the log with the command line's own changes.
    fn clean_step(&mut self) {
        let wrote = self.io.content_bytes + self.io.log_bytes - self.mark + self.ops.len() as u64;
        let slack = self.cfg.segment / 32;
        let (mut live, mut dead) = (0u64, 0u64);
        for s in self.segs.iter().flatten().filter(|s| s.live_pieces > 0 || !s.sealed) {
            live += s.live_bytes;
            dead += s.end - SEG_HEADER - s.live_bytes;
        }
        if dead <= live * 3 / 10 + slack {
            self.clean = None;
            self.clean_credit = 0;
            return;
        }
        let factor = if dead > live * 6 / 10 + slack { 16 } else { 4 };
        self.clean_credit = (self.clean_credit + wrote * factor).min(64 << 20);
        // What the next piece to move needs, when the credit is short.
        let mut short = 0u64;
        for _ in 0..8 {
            if self.clean.is_none() {
                // The data file with the largest share of unused bytes.
                // If that is the one being written, it is finished here
                // and a new one started, so that it can be emptied.
                let share = |s: &Segment| (s.end - SEG_HEADER - s.live_bytes) as f64 / (s.end - SEG_HEADER).max(1) as f64;
                let active = self.active;
                let best = self.segs.iter().flatten().filter(|s| s.id != active && s.live_pieces > 0).map(|s| (share(s), s.id)).fold((0.0, 0), |a, b| if b.0 > a.0 { b } else { a });
                let own = self.seg(active);
                let own_dead = own.end - SEG_HEADER - own.live_bytes;
                let victim = if own_dead > slack && share(own) > best.0 {
                    if let Err(e) = self.roll(0) {
                        self.fail(e);
                        return;
                    }
                    active
                } else {
                    best.1
                };
                if victim == 0 {
                    break;
                }
                self.clean = Some((victim, 0));
            }
            while let Some((id, mut ord)) = self.clean {
                let Some(seg) = self.segs[id as usize].as_ref() else {
                    self.clean = None;
                    break;
                };
                while (ord as usize) < seg.owners.len() && seg.owners[ord as usize] == DEAD {
                    ord += 1;
                }
                if ord as usize >= seg.owners.len() {
                    self.clean = None;
                    break;
                }
                // Every place the piece is used: usually one, more after
                // a copy. Places are taken from the back of each file so
                // that replacing one does not move the others.
                let mut users = Vec::new();
                seg.users(ord, &mut users);
                let mut found: Vec<(NodeId, usize)> = Vec::new();
                let mut piece = None;
                for &user in &users {
                    if let Some(Node { kind: NodeKind::File(t), .. }) = self.nodes.get(user) {
                        for (i, q) in t.pieces.iter().enumerate().rev() {
                            if q.seg == id && q.ord == ord {
                                found.push((user, i));
                                piece = Some(*q);
                            }
                        }
                    }
                }
                // Not found: taken out of its files by this command line.
                let Some(piece) = piece else {
                    self.clean = Some((id, ord + 1));
                    continue;
                };
                if self.clean_credit < piece.len as u64 {
                    self.clean = Some((id, ord));
                    short = piece.len as u64;
                    break;
                }
                self.clean_credit -= piece.len as u64;
                let text = self.text_of(&piece).into_owned();
                let new = self.store(found[0].0, &text);
                if self.failed.is_some() {
                    return;
                }
                self.io.moved_bytes += text.len() as u64;
                for (n, &(user, pos)) in found.iter().enumerate() {
                    if n > 0 {
                        for q in &new {
                            self.segs[q.seg as usize].as_mut().unwrap().add_user(q, user);
                        }
                    }
                    self.splice(user, pos, 1, new.clone());
                }
                self.clean = Some((id, ord + 1));
            }
            if self.clean.is_some() {
                break;
            }
        }
        // Credit is spent by the command line that earned it: what is
        // carried over is no more than the next piece takes, so that a
        // small write never pays for a large one before it.
        self.clean_credit = self.clean_credit.min(short.max(2 * self.cfg.piece as u64));
    }

    /// Remove data files that hold nothing any file uses and that the log
    /// no longer runs through.
    fn drop_empty(&mut self) -> io::Result<()> {
        let limit = if self.volatile { self.active } else { self.ckp_seg };
        for id in 1..limit.min(self.segs.len() as u32) {
            if self.segs[id as usize].as_ref().is_some_and(|s| s.live_pieces == 0 && s.sealed) {
                let seg = self.segs[id as usize].take().unwrap();
                let mut seg = seg;
                let had_index = seg.packed.take().is_some();
                seg.dynamic = None;
                if self.idx_job.as_ref().is_some_and(|j| j.seg == id) {
                    self.idx_job = None;
                    let _ = self.vfs.remove(&disk::idx_tmp(id));
                }
                if self.clean.is_some_and(|c| c.0 == id) {
                    self.clean = None;
                }
                if self.volatile {
                    drop(seg);
                    self.vfs.remove(&disk::seg_name(id))?;
                    if had_index {
                        self.vfs.remove(&disk::idx_name(id))?;
                    }
                } else {
                    self.dying.push(seg.into_file());
                    if had_index {
                        let name = disk::idx_name(id);
                        let file = self.vfs.open(&name, 0)?;
                        self.dying.push((name, file));
                    }
                }
            }
        }
        Ok(())
    }

    /// Close the database: everything is synced, the index is saved and
    /// the tree is saved, so that the next open has no log to read.
    pub fn close(mut self) -> Result<(), String> {
        self.commit();
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        if self.volatile {
            return Ok(());
        }
        // An index file must never cover content that is not on disk.
        self.sync();
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        let r = (|| {
            if self.cfg.index {
                self.index_backlog()?;
                self.save_index(self.active)?;
            }
            self.checkpoint()?;
            self.dying_step(u64::MAX)
        })();
        r.map_err(|e: io::Error| e.to_string())
    }

    /// A storage failure, if there has been one. After a failure nothing
    /// more is written, and what commands report is not to be relied on.
    /// Where output bound for a file may wait: see `engine::Spool`.
    pub fn begin_unit(&mut self) {
        self.unit = Some(UnitStart { reads: self.reads(), io: self.io, live: crate::mem::mark(), cpu: crate::mem::cpu_ns(), at: std::time::Instant::now() });
    }

    fn reads(&self) -> [u64; 6] {
        let t = &self.tally;
        [t.text_bytes.get(), t.text_pieces.get(), t.index_bytes.get(), t.index_lookups.get(), t.tree_nodes.get(), t.scratch.get()]
    }

    /// What the unit begun with `begin_unit` used. Kept as `last_cost`.
    pub fn end_unit(&mut self) -> Cost {
        let Some(start) = self.unit.take() else { return Cost::default() };
        let now = self.reads();
        let d = |i: usize| now[i] - start.reads[i];
        let (a, b) = (&start.io, &self.io);
        let moved = b.moved_bytes - a.moved_bytes;
        let cost = Cost {
            text_bytes: d(0),
            text_pieces: d(1),
            index_bytes: d(2),
            index_lookups: d(3),
            tree_nodes: d(4),
            content_written: b.content_bytes - a.content_bytes - moved,
            records_written: b.log_bytes - a.log_bytes,
            upkeep_written: b.index_bytes - a.index_bytes + b.tree_bytes - a.tree_bytes + moved,
            syncs: b.syncs - a.syncs + b.dir_syncs - a.dir_syncs,
            scratch_bytes: d(5),
            memory: crate::mem::rise(start.live) as u64,
            cpu_ns: crate::mem::cpu_ns().saturating_sub(start.cpu),
            elapsed_ns: start.at.elapsed().as_nanos() as u64,
        };
        self.last_cost = cost;
        cost
    }

    /// The cost of the last unit that ended.
    pub fn last_cost(&self) -> Cost {
        self.last_cost
    }

    pub fn scratch(&self) -> Scratch {
        Scratch { vfs: if self.volatile { None } else { Some(self.vfs.clone()) }, limit: self.cfg.spool, used: self.tally.scratch.clone() }
    }

    /// A scratch file could not be written: the storage has failed.
    pub fn scratch_failed(&mut self, e: io::Error) {
        self.fail(e);
    }

    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }

    /// Damage was found while a command ran (`disk::Damage`, caught by
    /// whoever ran the command). The database stops here: nothing more
    /// is written, and the next open recovers every whole command line.
    /// If what is damaged is an index file, that file is removed, so that
    /// the next open builds it again from the content.
    pub fn after_damage(&mut self, d: &Damage) -> String {
        let mut msg = d.0.clone();
        if let Some(seg) = d.1 {
            if self.vfs.remove(&disk::idx_name(seg)).is_ok() {
                msg.push_str("; this index file has been removed and will be built again when the database is next opened");
            }
        }
        if self.failed.is_none() {
            self.failed = Some(msg.clone());
        }
        msg
    }

    fn fail(&mut self, e: io::Error) {
        if self.failed.is_none() {
            self.failed = Some(format!("storage failed, nothing more will be written: {}", e));
        }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn set_sync(&mut self, sync: SyncMode) {
        self.cfg.sync = sync;
    }

    pub fn io_stats(&self) -> IoStats {
        self.io
    }

    /// Number of the last command line that changed anything.
    pub fn commit_seq(&self) -> u64 {
        self.seq
    }

    // ------------------------------------------------------ writing content

    #[inline]
    fn seg(&self, id: u32) -> &Segment {
        self.segs[id as usize].as_ref().expect("a piece in a data file that is gone")
    }

    /// Start the next data file. `need` is the size of the record that
    /// did not fit in this one.
    fn roll(&mut self, need: u64) -> io::Result<()> {
        let next = self.segs.len() as u32;
        let cap = self.cfg.segment.max(SEG_HEADER + need + 4096);
        if cap >= u32::MAX as u64 {
            return Err(io_err(format!("a single write of {} bytes is more than a data file can hold", need)));
        }
        if self.cfg.index {
            // An index file still being written is finished first, and
            // anything not yet indexed is.
            self.index_backlog()?;
        }
        let old = self.segs[self.active as usize].as_mut().unwrap();
        if !self.volatile {
            // The old file is finished and synced before the new one is
            // used, so that a crash can only ever damage the last file.
            old.append_next(next)?;
            old.sync()?;
            self.io.syncs += 1;
        }
        old.sealed = true;
        let mut seg = Segment::create(&*self.vfs, next, cap)?;
        seg.sealed = false;
        if !self.volatile {
            seg.sync()?;
            self.vfs.sync_dir()?;
            self.io.syncs += 1;
            self.io.dir_syncs += 1;
        }
        let old_id = self.active;
        self.segs.push(Some(Box::new(seg)));
        self.active = next;
        self.synced = SEG_HEADER;
        self.unsynced_lines = 0;
        if self.cfg.index {
            // The finished file's index file is written a step at a time
            // from here on; until it is whole the part in memory serves.
            self.idx_start(old_id)?;
            self.segs[next as usize].as_mut().unwrap().dynamic = Some(Index::starting_after(0));
        }
        Ok(())
    }

    /// Put content in the active data file, cut into pieces, and index
    /// it. Returns the pieces. On a storage failure the database is
    /// marked failed and what was stored so far is returned.
    fn store(&mut self, owner: NodeId, data: &[u8]) -> Vec<Piece> {
        let mut out = Vec::new();
        if data.is_empty() || self.failed.is_some() {
            return out;
        }
        let cuts = cut(data, self.cfg.piece);
        let mut i = 0;
        while i < cuts.len() {
            // As many pieces as fit in one record.
            let mut j = i + 1;
            while j < cuts.len() && cuts[j].1 - cuts[i].0 <= RECORD_MAX {
                j += 1;
            }
            // With compression: each piece by itself, kept only if it
            // comes out smaller.
            let packed: Vec<Option<Vec<u8>>> = cuts[i..j]
                .iter()
                .map(|&(a, b)| if self.cfg.compress { Some(lz4_flex::block::compress(&data[a..b])).filter(|c| c.len() < b - a) } else { None })
                .collect();
            let items: Vec<Stored> = cuts[i..j]
                .iter()
                .zip(&packed)
                .map(|(&(a, b), packed)| {
                    if b - a >= u32::MAX as usize {
                        damage(format!("a single line of {} bytes is longer than can be stored", b - a));
                    }
                    let (len, lines) = ((b - a) as u32, count_newlines(&data[a..b]) as u32);
                    match packed {
                        Some(c) => Stored { bytes: c, len, lines, enc: disk::ENC_LZ4 },
                        None => Stored { bytes: &data[a..b], len, lines, enc: disk::ENC_PLAIN },
                    }
                })
                .collect();
            let payload = 8 + items.len() as u64 * disk::TABLE_ENTRY + items.iter().map(|s| s.bytes.len() as u64).sum::<u64>();
            if !self.seg(self.active).fits(payload) {
                if let Err(e) = self.roll(payload) {
                    self.fail(e);
                    return out;
                }
            }
            let seg = self.segs[self.active as usize].as_mut().unwrap();
            match seg.append_content(&items, owner) {
                Ok(pieces) => {
                    if self.cfg.index_later {
                        self.waiting += (cuts[j - 1].1 - cuts[i].0) as u64;
                    } else if let Some(index) = &mut seg.dynamic {
                        if seg.indexed == pieces[0].ord {
                            for (p, item) in pieces.iter().zip(&items) {
                                let doc = index.add(&data[cuts[i + (p.ord - pieces[0].ord) as usize].0..][..item.len as usize]);
                                debug_assert_eq!(doc, p.ord + 1);
                            }
                            seg.indexed += pieces.len() as u32;
                        }
                    }
                    self.live_pieces += pieces.len() as u64;
                    self.io.content_bytes += payload + REC_HEADER;
                    out.extend(pieces);
                }
                Err(e) => {
                    self.fail(e);
                    return out;
                }
            }
            i = j;
        }
        out
    }

    /// Replace `del` pieces of a file, from position `at`, with others.
    fn splice(&mut self, id: NodeId, at: usize, del: usize, new: Vec<Piece>) {
        if del == 0 && new.is_empty() {
            return;
        }
        let t = match &mut self.nodes.get_mut(id).unwrap().kind {
            NodeKind::File(t) => t,
            NodeKind::Folder(_) => panic!("content of a folder"),
        };
        t.ver += 1;
        if !self.volatile {
            for n in [OP_SPLICE, id, t.ver as u64, at as u64, del as u64, new.len() as u64] {
                disk::put_u64(&mut self.ops, n);
            }
            for p in &new {
                p.encode(&mut self.ops);
            }
        }
        self.dirty = true;
        self.epoch += 1;
        let added: u64 = new.iter().map(|p| p.len as u64).sum();
        let gone: Vec<Piece> = t.pieces.splice(at..at + del, new).collect();
        t.len = t.len + added - gone.iter().map(|p| p.len as u64).sum::<u64>();
        self.retired.extend(gone.into_iter().map(|p| (id, p)));
    }

    fn text(&self, id: NodeId) -> &Text {
        match &self.nodes[id].kind {
            NodeKind::File(t) => t,
            NodeKind::Folder(_) => panic!("content of a folder"),
        }
    }

    fn piece_is(&self, p: &Piece, bytes: &[u8]) -> bool {
        p.len as usize == bytes.len() && *self.text_of(p) == *bytes
    }

    /// Store new content for a file. Returns false, having written
    /// nothing, when the file already holds exactly that. Otherwise the
    /// pieces at the start and at the end of the file that are the same
    /// in the new content are kept, and only what lies between is
    /// written.
    ///
    /// The caller must have checked the content with `is_valid_text`.
    pub fn set_content(&mut self, id: NodeId, content: &[u8]) -> bool {
        if id == NULL {
            return false;
        }
        debug_assert!(is_valid_text(content));
        let old = self.text(id).pieces.clone();
        // Pieces kept at the start.
        let (mut pre, mut at) = (0, 0usize);
        while pre < old.len() {
            let end = at + old[pre].len as usize;
            if end > content.len() || !self.piece_is(&old[pre], &content[at..end]) {
                break;
            }
            pre += 1;
            at = end;
        }
        self.touch(id);
        if pre == old.len() && at == content.len() {
            self.unveil(id);
            return false;
        }
        // Only a file's last piece may end without a newline.
        if pre > 0 && at < content.len() && content[at - 1] != b'\n' {
            pre -= 1;
            at -= old[pre].len as usize;
        }
        // Pieces kept at the end.
        let (mut suf, mut tail) = (0, content.len());
        while suf < old.len() - pre {
            let p = &old[old.len() - 1 - suf];
            if p.len as usize > tail - at || !self.piece_is(p, &content[tail - p.len as usize..tail]) {
                break;
            }
            suf += 1;
            tail -= p.len as usize;
        }
        while suf > 0 && tail > 0 && content[tail - 1] != b'\n' {
            tail += old[old.len() - suf].len as usize;
            suf -= 1;
        }
        let new = self.store(id, &content[at..tail]);
        self.splice(id, pre, old.len() - pre - suf, new);
        self.unveil(id);
        true
    }

    /// `set_content` for content that arrives in stretches: `next` fills
    /// the buffer it is given with the next stretch and returns false
    /// when there is no more. Memory in use stays at a few megabytes
    /// (or one line, if a line is longer) whatever the size of the
    /// content. As with `set_content`, the pieces at the start of the
    /// file that the new content begins with are kept, and if the content
    /// is what the file holds already, nothing is written and the answer
    /// is false.
    ///
    /// The caller must know the content to be valid text.
    pub fn set_content_stream(&mut self, id: NodeId, next: &mut dyn FnMut(&Db, &mut Vec<u8>) -> bool) -> bool {
        if id == NULL {
            return false;
        }
        let old = self.text(id).pieces.clone();
        self.touch(id);
        let mut buf: Vec<u8> = Vec::new();
        // While the new content is what the file holds: `k` whole pieces
        // and `within` bytes of the next have been matched.
        let (mut k, mut within, mut pos) = (0usize, 0usize, 0usize);
        let mut ended = false;
        loop {
            if pos == buf.len() {
                buf.clear();
                pos = 0;
                if !next(&*self, &mut buf) {
                    ended = true;
                    break;
                }
                continue;
            }
            if k == old.len() {
                break;
            }
            let text = self.text_of(&old[k]);
            let n = (text.len() - within).min(buf.len() - pos);
            if text[within..within + n] != buf[pos..pos + n] {
                break;
            }
            within += n;
            pos += n;
            if within == text.len() {
                k += 1;
                within = 0;
            }
        }
        if ended && k == old.len() {
            self.unveil(id);
            return false;
        }
        // Only a file's last piece may end without a newline: if the
        // file's last piece does and more follows it now, it is written
        // again with what follows.
        if k == old.len() && k > 0 && !self.text_of(&old[k - 1]).ends_with(b"\n") {
            k -= 1;
            within = old[k].len as usize;
        }
        // From piece `k` on the file is written anew, starting with the
        // part of that piece that was the same.
        let mut pending: Vec<u8> = Vec::new();
        if within > 0 {
            pending.extend_from_slice(&self.text_of(&old[k])[..within]);
        }
        let mut new: Vec<Piece> = Vec::new();
        const STRETCH: usize = 4 << 20;
        while !ended {
            pending.extend_from_slice(&buf[pos..]);
            if pending.len() >= STRETCH {
                // Whole lines go to the data file; the rest waits for
                // its end.
                if let Some(nl) = memchr::memrchr(b'\n', &pending) {
                    new.extend(self.store(id, &pending[..nl + 1]));
                    pending.drain(..nl + 1);
                }
            }
            buf.clear();
            pos = 0;
            ended = !next(&*self, &mut buf);
        }
        new.extend(self.store(id, &pending));
        self.splice(id, k, old.len() - k, new);
        self.unveil(id);
        true
    }

    /// Give a file the content of another. Nothing is copied: the file
    /// comes to use the same pieces, and what is written is the record
    /// that says so. False if it used exactly those pieces already.
    pub fn copy_content(&mut self, from: NodeId, to: NodeId) -> bool {
        if to == NULL {
            return false;
        }
        let pieces = self.pieces(from).to_vec();
        self.touch(to);
        let old = &self.text(to).pieces;
        let n = old.len();
        if n == pieces.len() && old.iter().zip(&pieces).all(|(a, b)| a.seg == b.seg && a.ord == b.ord) {
            self.unveil(to);
            return false;
        }
        for p in &pieces {
            if self.segs[p.seg as usize].as_mut().expect("a piece in a data file that is gone").add_user(p, to) {
                self.live_pieces += 1;
            }
        }
        self.splice(to, 0, n, pieces);
        self.unveil(to);
        true
    }

    /// Add to the end of a file. What is written is what is added, plus
    /// at most a few small pieces at the file's end that are joined with
    /// it so that a file appended to a line at a time does not end up in
    /// a piece per line.
    pub fn append(&mut self, id: NodeId, data: &[u8]) {
        if id == NULL {
            return;
        }
        if data.is_empty() {
            return;
        }
        if self.text(id).veiled {
            self.set_content(id, data);
            return;
        }
        self.touch(id);
        let pieces = &self.text(id).pieces;
        let n = pieces.len();
        let (mut take, mut size) = (0, data.len());
        while take < n {
            let p = &pieces[n - 1 - take];
            let len = p.len as usize;
            // A last piece that does not end its line has to be joined.
            let open = take == 0 && !self.text_of(p).ends_with(b"\n");
            let small = len < self.cfg.piece / 2 && len <= 2 * size && size + len <= self.cfg.piece;
            if !open && !small {
                break;
            }
            size += len;
            take += 1;
        }
        if take == 0 {
            let new = self.store(id, data);
            self.splice(id, n, 0, new);
            return;
        }
        let mut all = Vec::with_capacity(size);
        for p in &pieces[n - take..] {
            all.extend_from_slice(&self.text_of(p));
        }
        all.extend_from_slice(data);
        let new = self.store(id, &all);
        self.splice(id, n - take, take, new);
    }

    /// Replace `del` whole pieces of a file, from position `at`, with new
    /// text. Unless the replaced pieces run to the end of the file, the
    /// text must be empty or end with a newline. Returns how many pieces
    /// the text became.
    pub fn replace_pieces(&mut self, id: NodeId, at: usize, del: usize, text: &[u8]) -> usize {
        if id == NULL {
            return 0;
        }
        debug_assert!(text.is_empty() || text.ends_with(b"\n") || at + del == self.text(id).pieces.len());
        let new = self.store(id, text);
        let n = new.len();
        self.splice(id, at, del, new);
        self.touch(id);
        n
    }

    /// The pieces of a file as they are, to put back with
    /// `restore_pieces` if an edit made a piece at a time has to be
    /// abandoned. Only within one command line.
    pub fn save_pieces(&self, id: NodeId) -> Vec<Piece> {
        self.text(id).pieces.clone()
    }

    pub fn restore_pieces(&mut self, id: NodeId, saved: Vec<Piece>) {
        if id == NULL {
            return;
        }
        let now = self.text(id).pieces.len();
        let keep: std::collections::HashSet<(u32, u32)> = saved.iter().map(|p| (p.seg, p.ord)).collect();
        self.splice(id, 0, now, saved);
        self.retired.retain(|(user, p)| *user != id || !keep.contains(&(p.seg, p.ord)));
    }

    /// Make a file read as empty until the end of the command line, the
    /// way `>` empties its file before the command runs, without storing
    /// anything yet. What the command then writes is compared with what
    /// the file held, so writing the same content back changes nothing.
    /// At the end of the command line a file still veiled becomes empty.
    pub fn veil(&mut self, id: NodeId) {
        if id == NULL {
            return;
        }
        if let NodeKind::File(t) = &mut self.nodes.get_mut(id).unwrap().kind {
            if !t.veiled && !t.pieces.is_empty() {
                t.veiled = true;
                self.veils.push(id);
            }
        }
    }

    /// Take a veil off: the file reads as it did before.
    pub fn unveil(&mut self, id: NodeId) {
        if self.veils.is_empty() {
            return;
        }
        if let Some(Node { kind: NodeKind::File(t), .. }) = self.nodes.get_mut(id) {
            if t.veiled {
                t.veiled = false;
                self.veils.retain(|v| *v != id);
            }
        }
    }

    /// The command line is over. Files still veiled become empty; the
    /// content of pieces no longer in any file is forgotten; and if the
    /// tree changed, the changes are logged as one record and, under
    /// `SyncMode::Always`, synced. A command line that changed nothing
    /// writes nothing.
    pub fn commit(&mut self) {
        while let Some(id) = self.veils.pop() {
            if let Some(Node { kind: NodeKind::File(t), .. }) = self.nodes.get_mut(id) {
                if t.veiled {
                    t.veiled = false;
                    let n = t.pieces.len();
                    self.splice(id, 0, n, Vec::new());
                    self.touch(id);
                }
            }
        }
        if !self.timed.is_empty() {
            let mut timed: Vec<NodeId> = self.timed.drain().collect();
            timed.sort_unstable();
            for id in timed {
                if let Some(node) = self.nodes.get(id) {
                    disk::put_u64(&mut self.ops, OP_MTIME);
                    disk::put_u64(&mut self.ops, id);
                    disk::put_u64(&mut self.ops, node.mtime as u64);
                }
            }
        }
        if self.dirty && self.failed.is_none() && self.cfg.upkeep {
            self.clean_step();
        }
        let wrote = self.io.content_bytes + self.io.log_bytes - self.mark + self.ops.len() as u64;
        // The record goes into the log before the pieces taken out of
        // files are forgotten: writing it can finish an index file, and
        // an index file must not lack a piece that the log on disk still
        // has in use.
        let mut r = Ok(());
        let changed = self.dirty;
        if self.dirty {
            self.dirty = false;
            if self.failed.is_some() {
                self.ops.clear();
            } else if !self.volatile {
                r = self.log_commit();
            }
        }
        for (user, p) in std::mem::take(&mut self.retired) {
            if let Some(seg) = self.segs[p.seg as usize].as_mut() {
                if seg.drop_user(&p, user) {
                    self.live_pieces -= 1;
                }
            }
        }
        if self.failed.is_none() && changed {
            if self.volatile {
                r = r.and_then(|_| self.drop_empty());
            }
            r = r.and_then(|_| self.upkeep(wrote));
        }
        self.mark = self.io.content_bytes + self.io.log_bytes;
        if let Err(e) = r {
            self.fail(e);
        }
    }

    fn log_commit(&mut self) -> io::Result<()> {
        let ops = std::mem::take(&mut self.ops);
        let payload = 16 + ops.len() as u64;
        if !self.seg(self.active).fits(payload) {
            self.roll(payload)?;
        }
        self.seq += 1;
        let synced = self.synced;
        let seg = self.segs[self.active as usize].as_mut().unwrap();
        seg.append_meta(self.seq, synced, &ops)?;
        self.io.commits += 1;
        self.io.log_bytes += payload + REC_HEADER;
        self.log_since += payload + REC_HEADER;
        self.unsynced_lines += 1;
        let due = match self.cfg.sync {
            SyncMode::Always => true,
            SyncMode::Batch => self.unsynced_lines >= self.cfg.batch_lines || seg.end - self.synced >= self.cfg.batch_bytes,
        };
        if due {
            seg.sync()?;
            self.io.syncs += 1;
            self.synced = seg.end;
            self.unsynced_lines = 0;
        }
        self.ops = ops;
        self.ops.clear();
        Ok(())
    }

    /// Sync whatever has been logged and not yet synced.
    pub fn sync(&mut self) {
        if self.volatile || self.failed.is_some() {
            return;
        }
        let seg = self.segs[self.active as usize].as_mut().unwrap();
        if seg.end > self.synced {
            match seg.sync() {
                Ok(()) => {
                    self.io.syncs += 1;
                    self.synced = seg.end;
                    self.unsynced_lines = 0;
                }
                Err(e) => self.fail(e),
            }
        }
    }

    // ------------------------------------------------------ reading content

    /// The pieces of a file's content, in order. Empty while veiled.
    pub fn pieces(&self, id: NodeId) -> &[Piece] {
        let t = self.text(id);
        if t.veiled {
            &[]
        } else {
            &t.pieces
        }
    }

    /// The text of one piece.
    #[inline]
    pub fn text_of(&self, p: &Piece) -> Cow<'_, [u8]> {
        bump(&self.tally.text_bytes, p.len as u64);
        bump(&self.tally.text_pieces, 1);
        let stored = self.seg(p.seg).piece(p);
        if p.enc == disk::ENC_LZ4 {
            return match lz4_flex::block::decompress(stored, p.len as usize) {
                Ok(text) if text.len() == p.len as usize => Cow::Owned(text),
                _ => damage(format!("{}: damaged: the compressed piece at offset {} does not unpack to its {} bytes", self.seg(p.seg).name(), p.off, p.len)),
            };
        }
        if p.enc != disk::ENC_PLAIN {
            damage(format!("{}: the piece at offset {} is stored with encoding {}, which this version does not know", self.seg(p.seg).name(), p.off, p.enc));
        }
        Cow::Borrowed(stored)
    }

    /// Bytes in a file.
    pub fn size(&self, id: NodeId) -> u64 {
        let t = self.text(id);
        if t.veiled {
            0
        } else {
            t.len
        }
    }

    /// Newlines in a file, from what is kept about its pieces: the
    /// content is not read.
    pub fn line_count(&self, id: NodeId) -> u64 {
        self.pieces(id).iter().map(|p| p.lines as u64).sum()
    }

    /// Content of a file in one stretch. Panics on a folder. This borrows
    /// straight from the data file when the pieces lie one after another
    /// there, as they do for a file stored in one go; otherwise the
    /// pieces are joined in memory. A command that can work a piece at a
    /// time should use `pieces`.
    pub fn content(&self, id: NodeId) -> Cow<'_, [u8]> {
        let pieces = self.pieces(id);
        match pieces {
            [] => Cow::Borrowed(&[]),
            [one] => self.text_of(one),
            [first, ..] => {
                let seg = self.seg(first.seg);
                let mut end = first.off as u64;
                let together = pieces.iter().all(|p| {
                    let next = p.seg == first.seg && p.off as u64 == end && p.enc == disk::ENC_PLAIN;
                    end += p.slen as u64;
                    next
                });
                if together {
                    for p in pieces {
                        seg.piece(p);
                    }
                    bump(&self.tally.text_bytes, end - first.off as u64);
                    bump(&self.tally.text_pieces, pieces.len() as u64);
                    return Cow::Borrowed(seg.bytes(first.off as u64, (end - first.off as u64) as usize));
                }
                let mut all = Vec::with_capacity(self.text(id).len as usize);
                for p in pieces {
                    all.extend_from_slice(&self.text_of(p));
                }
                Cow::Owned(all)
            }
        }
    }

    // ------------------------------------------------------ the index

    pub fn has_index(&self) -> bool {
        self.cfg.index
    }

    /// Switch the search index on or off. Switching it on builds what is
    /// missing from the files; switching it off stops using and keeping
    /// it. Results never depend on it, only speed.
    pub fn set_index(&mut self, on: bool) {
        self.commit();
        self.cfg.index = on;
        if on {
            if let Err(e) = self.index_backlog() {
                self.fail(e);
            }
        } else {
            if let Some(job) = self.idx_job.take() {
                let _ = self.vfs.remove(&disk::idx_tmp(job.seg));
            }
            for seg in self.segs.iter_mut().flatten() {
                // What the index file covers stays covered; what was only
                // in memory is no longer indexed.
                seg.dynamic = None;
                seg.indexed = seg.packed.as_ref().map_or(0, |p| p.ndocs);
            }
        }
    }

    /// Have searches ignore the index (false) or use it again (true),
    /// without throwing it away: it is still kept current by writes. This
    /// is for measuring what the index does for a search, both ways in
    /// one process.
    pub fn set_index_consulted(&mut self, consulted: bool) {
        self.consult = consulted;
    }

    /// Facts about the index, or None when it is off. Sizes only; see
    /// `index_details` for what takes reading every list.
    pub fn index_stats(&self) -> Option<IndexStats> {
        if !self.cfg.index {
            return None;
        }
        let mut s = IndexStats::default();
        s.live_documents = self.live_pieces as usize;
        for seg in self.segs.iter().flatten() {
            s.documents_ever += seg.owners.len();
            if let Some(p) = &seg.packed {
                let (lists, tables, rest) = p.sizes();
                s.parts += 1;
                s.list_bytes += lists as usize;
                s.posting_bytes += lists;
                s.table_bytes += (tables + rest) as usize;
                s.trigrams = s.trigrams.max(p.ntrigrams as usize);
            }
            if let Some(d) = &seg.dynamic {
                let (table, lists, docs) = d.sizes();
                s.parts += 1;
                s.table_bytes += table;
                s.list_bytes += lists;
                s.document_bytes += docs;
            }
            s.document_bytes += seg.owners.capacity() * 8;
        }
        Some(s)
    }

    /// Facts about the index that take reading all of it: how many
    /// entries the lists have and how their lengths are spread. Figures
    /// are summed over the parts of the index, one per data file, so a
    /// trigram in several parts counts as several lists.
    pub fn index_details(&self) -> Option<IndexStats> {
        let mut s = self.index_stats()?;
        s.trigrams = 0;
        s.posting_bytes = 0;
        let mut sizes: Vec<u64> = Vec::new();
        let mut list = |bytes: &[u8], s: &mut IndexStats| {
            let count = Postings::new(bytes).count();
            if count == 0 {
                return;
            }
            s.trigrams += 1;
            s.postings += count as u64;
            s.posting_bytes += bytes.len() as u64;
            sizes.push(bytes.len() as u64);
            let bucket = (usize::BITS - (count - 1).leading_zeros()) as usize;
            if s.lists_by_length.len() <= bucket {
                s.lists_by_length.resize(bucket + 1, 0);
            }
            s.lists_by_length[bucket] += 1;
        };
        for seg in self.segs.iter().flatten() {
            if let Some(p) = &seg.packed {
                for i in 0..p.ntrigrams {
                    list(p.list_at(i), &mut s);
                }
            }
            if let Some(d) = &seg.dynamic {
                for t in d.trigrams() {
                    list(d.list(t), &mut s);
                }
                s.stale_postings += d.stats().stale_postings;
            }
        }
        sizes.sort_unstable_by(|a, b| b.cmp(a));
        for (i, top) in [100usize, 1000, 10000].into_iter().enumerate() {
            s.top_list_bytes[i] = sizes.iter().take(top).sum();
        }
        Some(s)
    }

    /// Running totals of index work, or None when it is off.
    pub fn index_work(&self) -> Option<IndexWork> {
        if !self.cfg.index {
            return None;
        }
        let mut w = self.work;
        for seg in self.segs.iter().flatten() {
            if let Some(d) = &seg.dynamic {
                w.add(&d.work());
            }
        }
        Some(w)
    }

    /// The pieces that can satisfy the query, or None for "any". With
    /// `shortcut`, a query that a large share of all pieces may satisfy
    /// is answered "any" without reading a list: such a search is faster
    /// reading the files.
    fn hits(&self, query: &Query, shortcut: bool) -> Option<Shortlist> {
        if shortcut {
            let mut most = 0usize;
            for seg in self.segs.iter().flatten() {
                let (p, d) = (seg.packed.as_ref().map_or(0, |p| index::estimate(p, query)), seg.dynamic.as_ref().map_or(0, |d| index::estimate(d, query)));
                most = most.saturating_add(p).saturating_add(d).saturating_add(seg.owners.len() - seg.indexed as usize);
            }
            if most > self.live_pieces as usize / 2 {
                return None;
            }
        }
        let mut by_seg = vec![Vec::new(); self.segs.len()];
        for seg in self.segs.iter().flatten() {
            let ords = &mut by_seg[seg.id as usize];
            // Documents are numbered from 1, pieces from 0. The index
            // file covers the lower numbers, so the result is ascending.
            if let Some(p) = &seg.packed {
                bump(&self.tally.index_bytes, index::cost(p, query) as u64);
                bump(&self.tally.index_lookups, query.trigrams() as u64);
                ords.extend(index::eval(p, query)?.into_iter().map(|d| d - 1));
            }
            if let Some(d) = &seg.dynamic {
                bump(&self.tally.index_bytes, index::cost(d, query) as u64);
                bump(&self.tally.index_lookups, query.trigrams() as u64);
                ords.extend(index::eval(d, query)?.into_iter().map(|d| d - 1));
            }
            // Pieces in no index can always match.
            ords.extend(seg.indexed..seg.owners.len() as u32);
        }
        Some(Shortlist { by_seg })
    }

    fn consulted(&self) -> bool {
        self.cfg.index && self.consult
    }

    /// What asking `candidates` would cost, as the number of bytes of file
    /// content that could be searched in the same time; None when the
    /// answer would be "any of them" and costs nothing. A command that may
    /// be stopped early can read that much before it asks.
    pub fn candidates_cost(&self, query: &Query) -> Option<usize> {
        if !self.consulted() {
            return None;
        }
        let (mut most, mut cost) = (0usize, 0usize);
        for seg in self.segs.iter().flatten() {
            for lists in [seg.packed.as_ref().map(|p| p as &dyn Lists), seg.dynamic.as_ref().map(|d| d as &dyn Lists)].into_iter().flatten() {
                most = most.saturating_add(index::estimate(lists, query));
                cost = cost.saturating_add(index::cost(lists, query));
            }
            most = most.saturating_add(seg.owners.len() - seg.indexed as usize);
        }
        if most > self.live_pieces as usize / 2 {
            return None;
        }
        // Reading a byte of a list takes about as long as searching
        // sixteen bytes of text.
        Some(cost.saturating_mul(16))
    }

    /// Which pieces, wherever they are, can match the query? None means
    /// any. For commands that are given a list of files and not a
    /// folder: ask `passes` of each file.
    pub fn shortlist(&self, query: &Query) -> Option<Shortlist> {
        if !self.consulted() {
            return None;
        }
        self.hits(query, true)
    }

    /// Which files under `folder` can match `query`? This is the one
    /// question commands ask of the index; how it is answered stays in
    /// here.
    pub fn candidates(&self, folder: NodeId, query: &Query) -> Candidates {
        let list = match self.shortlist(query) {
            Some(l) => l,
            None => return Candidates::All,
        };
        let mut files: Vec<NodeId> = Vec::new();
        for (seg, ords) in list.by_seg.iter().enumerate() {
            if let Some(s) = &self.segs[seg] {
                for o in ords {
                    s.users(*o, &mut files);
                }
            }
        }
        files.sort_unstable();
        files.dedup();
        let below = self.nodes[folder].aux as usize;
        if files.len() * 8 >= below {
            // Many candidates for the size of this folder: a list of them
            // would cost a walk of the folder to put in order, so leave
            // the walk to the caller.
            return Candidates::Among(list);
        }
        // Few candidates: place each one by following its parents up,
        // without looking at the rest of the folder.
        let mut placed: Vec<(Vec<&[u8]>, NodeId)> = Vec::new();
        for id in files {
            let mut names = Vec::new();
            let mut cur = id;
            let inside = loop {
                if cur == folder {
                    break true;
                }
                // A file made and removed again in this command line.
                let Some(node) = self.nodes.get(cur) else { break false };
                match node.parent {
                    Some(p) => {
                        names.push(node.name.as_slice());
                        cur = p;
                    }
                    None => break false,
                }
            };
            if inside {
                names.reverse();
                placed.push((names, id));
            }
        }
        // Comparing name by name gives the order of a walk.
        placed.sort_unstable();
        Candidates::Files(placed.into_iter().map(|(names, id)| Candidate { id, path: names.join(&b'/') }).collect(), list)
    }

    /// For checks: every file the index holds as able to satisfy the
    /// query, ascending, wherever it is in the tree. None when the index
    /// is off or the query says nothing.
    pub fn index_answer(&self, query: &Query) -> Option<Vec<NodeId>> {
        if !self.cfg.index {
            return None;
        }
        let list = self.hits(query, false)?;
        let mut files: Vec<NodeId> = Vec::new();
        for (seg, ords) in list.by_seg.iter().enumerate() {
            if let Some(s) = &self.segs[seg] {
                for o in ords {
                    s.users(*o, &mut files);
                }
            }
        }
        files.sort_unstable();
        files.dedup();
        Some(files)
    }

    /// Whether a piece is in the set a shortlist stands for.
    #[inline]
    pub fn piece_passes(&self, list: &Shortlist, p: &Piece) -> bool {
        list.by_seg.get(p.seg as usize).is_some_and(|ords| ords.binary_search(&p.ord).is_ok())
    }

    /// Whether any piece of a file is.
    pub fn passes(&self, list: &Shortlist, file: NodeId) -> bool {
        self.text(file).pieces.iter().any(|p| self.piece_passes(list, p))
    }

    /// The path of a file from a folder above it, or None when the file
    /// is not under that folder.
    pub fn path_below(&self, folder: NodeId, id: NodeId) -> Option<Vec<u8>> {
        let mut names = Vec::new();
        let mut cur = id;
        while cur != folder {
            let node = self.nodes.get(cur)?;
            names.push(node.name.as_slice());
            cur = node.parent?;
        }
        if names.is_empty() {
            return None;
        }
        names.reverse();
        Some(names.join(&b'/'))
    }

    /// How many files lie anywhere beneath a folder.
    pub fn files_below(&self, folder: NodeId) -> usize {
        self.nodes[folder].aux as usize
    }

    /// Add to the file count of a folder and of every folder above it.
    fn count_files(&mut self, mut folder: Option<NodeId>, delta: i64) {
        while let Some(id) = folder {
            let node = self.nodes.get_mut(id).unwrap();
            node.aux = (node.aux as i64 + delta) as u32;
            folder = node.parent;
        }
    }

    // ------------------------------------------------------------- the tree

    /// A total over everything under a folder that a command worked out
    /// since the tree last changed, by the command's own `kind` number.
    pub fn total(&self, folder: NodeId, kind: u8) -> Option<u64> {
        let t = self.totals.borrow();
        if t.0 == self.epoch {
            t.1.get(&(folder, kind)).copied()
        } else {
            None
        }
    }

    /// Remember a total until the tree next changes.
    pub fn set_total(&self, folder: NodeId, kind: u8, sum: u64) {
        let mut t = self.totals.borrow_mut();
        if t.0 != self.epoch {
            t.0 = self.epoch;
            t.1.clear();
        }
        t.1.insert((folder, kind), sum);
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn exists(&self, id: NodeId) -> bool {
        self.nodes.get(id).is_some()
    }

    pub fn is_dir(&self, id: NodeId) -> bool {
        matches!(self.nodes[id].kind, NodeKind::Folder(_))
    }

    pub fn name(&self, id: NodeId) -> &[u8] {
        &self.nodes[id].name
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id].parent
    }

    /// Children of a folder in byte order. Panics on a file.
    pub fn children(&self, id: NodeId) -> &BTreeMap<Vec<u8>, NodeId> {
        match &self.nodes[id].kind {
            NodeKind::Folder(c) => {
                bump(&self.tally.tree_nodes, c.len() as u64);
                c
            }
            NodeKind::File(_) => panic!("children() on a file"),
        }
    }

    pub fn child(&self, id: NodeId, name: &[u8]) -> Option<NodeId> {
        bump(&self.tally.tree_nodes, 1);
        match &self.nodes[id].kind {
            NodeKind::Folder(c) => c.get(name).copied(),
            NodeKind::File(_) => None,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len
    }

    /// Total bytes of file content.
    pub fn content_bytes(&self) -> usize {
        self.nodes
            .iter()
            .map(|n| match &n.kind {
                NodeKind::File(t) => t.len as usize,
                _ => 0,
            })
            .sum()
    }

    /// True if `id` is `ancestor` or lies underneath it.
    pub fn is_same_or_descendant(&self, id: NodeId, ancestor: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(c) = cur {
            if c == ancestor {
                return true;
            }
            cur = self.nodes[c].parent;
        }
        false
    }

    /// Resolve a path. There is no current directory: every path resolves
    /// from the root, so `src/a.txt`, `./src/a.txt` and `/src/a.txt` are the
    /// same file. `..` at the root stays at the root.
    pub fn resolve(&self, path: &[u8]) -> Result<NodeId, Errno> {
        if path.is_empty() {
            return Err(Errno::NoEnt);
        }
        if path == b"/dev/null" {
            return Ok(NULL);
        }
        let mut cur = self.root;
        for comp in path.split(|b| *b == b'/') {
            let children = match &self.nodes[cur].kind {
                NodeKind::Folder(c) => c,
                NodeKind::File(_) => return Err(Errno::NotDir),
            };
            if comp.is_empty() || comp == b"." {
                continue;
            }
            if comp == b".." {
                cur = self.nodes[cur].parent.unwrap_or(self.root);
                continue;
            }
            bump(&self.tally.tree_nodes, 1);
            cur = *children.get(comp).ok_or(Errno::NoEnt)?;
        }
        Ok(cur)
    }

    /// Split a path into (parent folder, final name) for an operation that
    /// may create the final component. Trailing slashes are ignored here; the
    /// caller decides what they mean. The final name can be `.` or `..`.
    pub fn resolve_parent(&self, path: &[u8]) -> Result<(NodeId, Vec<u8>), Errno> {
        if path.is_empty() {
            return Err(Errno::NoEnt);
        }
        let mut end = path.len();
        while end > 0 && path[end - 1] == b'/' {
            end -= 1;
        }
        if end == 0 {
            return Ok((self.root, b".".to_vec()));
        }
        let trimmed = &path[..end];
        let (dir, name) = match trimmed.iter().rposition(|b| *b == b'/') {
            Some(i) => (&trimmed[..i + 1], &trimmed[i + 1..]),
            None => (&b""[..], trimmed),
        };
        let parent = if dir.is_empty() { self.root } else { self.resolve(dir)? };
        if !self.is_dir(parent) {
            return Err(Errno::NotDir);
        }
        Ok((parent, name.to_vec()))
    }

    /// Put a node that is in `nodes` into its parent's children.
    fn link(&mut self, id: NodeId) {
        let (parent, name) = (self.nodes[id].parent.unwrap(), self.nodes[id].name.clone());
        match &mut self.nodes.get_mut(parent).unwrap().kind {
            NodeKind::Folder(c) => {
                let old = c.insert(name, id);
                assert!(old.is_none(), "duplicate name in folder");
            }
            NodeKind::File(_) => panic!("parent is a file"),
        }
    }

    /// The time now: the system clock, or what `set_clock` said.
    pub fn now(&self) -> i64 {
        match self.clock {
            Some(t) => t,
            None => match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(d) => d.as_nanos().min(i64::MAX as u128) as i64,
                Err(_) => 0,
            },
        }
    }

    /// Fix the time the engine sees, for tests; None goes back to the
    /// system clock.
    pub fn set_clock(&mut self, t: Option<i64>) {
        self.clock = t;
    }

    pub fn mtime(&self, id: NodeId) -> i64 {
        if id == NULL {
            return self.now();
        }
        self.nodes[id].mtime
    }

    /// Set a node's modification time. Like every change it is logged
    /// when the command line ends.
    pub fn set_mtime(&mut self, id: NodeId, t: i64) {
        if id == NULL {
            return;
        }
        self.nodes.get_mut(id).unwrap().mtime = t;
        self.dirty = true;
        self.epoch += 1;
        if !self.volatile {
            self.timed.insert(id);
        }
    }

    /// Set a node's modification time to now.
    pub fn touch(&mut self, id: NodeId) {
        let now = self.now();
        self.set_mtime(id, now);
    }

    fn log_name(&mut self, op: u64, id: NodeId, parent: NodeId, name: &[u8]) {
        self.dirty = true;
        self.epoch += 1;
        if !self.volatile {
            for n in [op, id, parent] {
                disk::put_u64(&mut self.ops, n);
            }
            disk::put_bytes(&mut self.ops, name);
        }
    }

    fn alloc(&mut self, name: &[u8], parent: NodeId, kind: NodeKind) -> NodeId {
        assert!(is_valid_name(name), "invalid node name");
        let id = self.next_id;
        self.next_id += 1;
        let is_file = matches!(kind, NodeKind::File(_));
        self.nodes.insert(Node { id, name: name.to_vec(), parent: Some(parent), kind, aux: 0, mtime: 0 });
        self.link(id);
        self.log_name(if is_file { OP_MKFILE } else { OP_MKDIR }, id, parent, name);
        let now = self.now();
        self.set_mtime(id, now);
        self.set_mtime(parent, now);
        if is_file {
            self.count_files(Some(parent), 1);
        }
        id
    }

    pub fn create_folder(&mut self, parent: NodeId, name: &[u8]) -> NodeId {
        self.alloc(name, parent, NodeKind::Folder(BTreeMap::new()))
    }

    /// The caller must have checked the content with `is_valid_text`.
    pub fn create_file(&mut self, parent: NodeId, name: &[u8], content: &[u8]) -> NodeId {
        debug_assert!(is_valid_text(content));
        let id = self.alloc(name, parent, NodeKind::File(Text::default()));
        let pieces = self.store(id, content);
        self.splice(id, 0, 0, pieces);
        id
    }

    /// Remove a node and everything under it. The root cannot be removed.
    pub fn remove(&mut self, id: NodeId) {
        if id == NULL {
            return;
        }
        assert!(id != self.root, "cannot remove the root");
        let (parent, name) = {
            let n = &self.nodes[id];
            (n.parent.unwrap(), n.name.clone())
        };
        if let NodeKind::Folder(c) = &mut self.nodes.get_mut(parent).unwrap().kind {
            c.remove(&name);
        }
        let gone = match &self.nodes[id].kind {
            NodeKind::File(_) => 1,
            NodeKind::Folder(_) => self.nodes[id].aux as i64,
        };
        self.count_files(Some(parent), -gone);
        self.dirty = true;
        self.epoch += 1;
        self.touch(parent);
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            if let Some(node) = self.nodes.remove(n) {
                if !self.volatile {
                    disk::put_u64(&mut self.ops, OP_REMOVE);
                    disk::put_u64(&mut self.ops, n);
                }
                match node.kind {
                    NodeKind::Folder(c) => stack.extend(c.values().copied()),
                    NodeKind::File(t) => {
                        if t.veiled {
                            self.veils.retain(|v| *v != n);
                        }
                        self.retired.extend(t.pieces.into_iter().map(|p| (n, p)));
                    }
                }
            }
        }
    }

    /// Move a node to a new parent and name, keeping its id. The destination
    /// name must be free, and a folder cannot move into its own descendant.
    pub fn rename(&mut self, id: NodeId, new_parent: NodeId, new_name: &[u8]) -> Result<(), Errno> {
        if id == NULL {
            return Err(Errno::Inval);
        }
        assert!(id != self.root);
        assert!(is_valid_name(new_name));
        if self.is_same_or_descendant(new_parent, id) {
            return Err(Errno::Inval);
        }
        if self.child(new_parent, new_name).is_some() {
            return Err(Errno::Exist);
        }
        let (old_parent, old_name) = {
            let n = &self.nodes[id];
            (n.parent.unwrap(), n.name.clone())
        };
        if let NodeKind::Folder(c) = &mut self.nodes.get_mut(old_parent).unwrap().kind {
            c.remove(&old_name);
        }
        if let NodeKind::Folder(c) = &mut self.nodes.get_mut(new_parent).unwrap().kind {
            c.insert(new_name.to_vec(), id);
        }
        let n = self.nodes.get_mut(id).unwrap();
        n.parent = Some(new_parent);
        n.name = new_name.to_vec();
        // Pieces know their file by id, so a move costs the index
        // nothing; only the file counts of the folders on both sides
        // change.
        let moved = match n.kind {
            NodeKind::File(_) => 1,
            NodeKind::Folder(_) => n.aux as i64,
        };
        self.count_files(Some(old_parent), -moved);
        self.count_files(Some(new_parent), moved);
        self.log_name(OP_RENAME, id, new_parent, new_name);
        let now = self.now();
        self.set_mtime(old_parent, now);
        self.set_mtime(new_parent, now);
        Ok(())
    }

    /// Every node as (path from the root, None for a folder or Some(content)),
    /// depth first in byte order. The root itself is not listed.
    pub fn snapshot(&self) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
        let mut out = Vec::new();
        self.snapshot_into(self.root, &mut Vec::new(), &mut out);
        out
    }

    fn snapshot_into(&self, id: NodeId, prefix: &mut Vec<u8>, out: &mut Vec<(Vec<u8>, Option<Vec<u8>>)>) {
        for (name, child) in self.children(id) {
            let len = prefix.len();
            if !prefix.is_empty() {
                prefix.push(b'/');
            }
            prefix.extend_from_slice(name);
            match &self.nodes[*child].kind {
                NodeKind::File(_) => out.push((prefix.clone(), Some(self.content(*child).into_owned()))),
                NodeKind::Folder(_) => {
                    out.push((prefix.clone(), None));
                    self.snapshot_into(*child, prefix, out);
                }
            }
            prefix.truncate(len);
        }
    }

    /// Every file in the order a walk visits them.
    fn files_in_order(&self) -> Vec<NodeId> {
        let mut order = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            match &self.nodes[id].kind {
                NodeKind::File(_) => order.push(id),
                NodeKind::Folder(children) => stack.extend(children.values().rev().copied()),
            }
        }
        order
    }

    // ------------------------------------------------- upkeep and checking

    /// Bring the database to its settled state: every file's content is
    /// written again, in the order of a walk and in full pieces, into new
    /// data files; the old ones are removed; the index and the tree are
    /// saved. Nothing is done if no data file holds anything unused.
    pub fn compact(&mut self) -> Result<(), String> {
        self.commit();
        // Whatever was logged before this is on disk before anything is
        // moved, so that a power cut during the work loses none of it.
        self.sync();
        self.clean = None;
        self.clean_credit = 0;
        let unused: usize = self.segs.iter().flatten().map(|s| s.owners.len() - s.live_pieces as usize).sum();
        let r = (|| -> io::Result<()> {
            if unused > 0 {
                let sync = std::mem::replace(&mut self.cfg.sync, SyncMode::Batch);
                self.roll(0)?;
                let first_new = self.active;
                // Where a piece that several files use went, so that they
                // still share it afterwards.
                let mut moved: std::collections::HashMap<(u32, u32), Vec<Piece>> = std::collections::HashMap::new();
                for id in self.files_in_order() {
                    let old = self.text(id).pieces.clone();
                    if old.is_empty() || old.iter().all(|p| p.seg >= first_new) {
                        continue;
                    }
                    let mut new = Vec::with_capacity(old.len());
                    let mut buf: Vec<u8> = Vec::new();
                    for (i, p) in old.iter().enumerate() {
                        let shared = p.seg < first_new && self.seg(p.seg).owners[p.ord as usize] == SHARED;
                        if !shared {
                            buf.extend_from_slice(&self.text_of(p));
                        }
                        if !buf.is_empty() && (shared || buf.len() >= 16 << 20 || i + 1 == old.len()) {
                            new.extend(self.store(id, &buf));
                            buf.clear();
                        }
                        if shared {
                            match moved.get(&(p.seg, p.ord)) {
                                Some(there) => {
                                    for q in there {
                                        self.segs[q.seg as usize].as_mut().unwrap().add_user(q, id);
                                    }
                                    new.extend(there.iter().copied());
                                }
                                None => {
                                    let text = self.text_of(p).into_owned();
                                    let there = self.store(id, &text);
                                    new.extend(there.iter().copied());
                                    moved.insert((p.seg, p.ord), there);
                                }
                            }
                        }
                    }
                    self.splice(id, 0, old.len(), new);
                    if self.ops.len() > 1 << 20 {
                        self.commit();
                    }
                    if let Some(e) = &self.failed {
                        return Err(io_err(e.clone()));
                    }
                }
                self.commit();
                self.cfg.sync = sync;
            }
            if self.volatile {
                return Ok(());
            }
            self.sync();
            if let Some(e) = &self.failed {
                return Err(io_err(e.clone()));
            }
            if self.cfg.index {
                self.index_backlog()?;
                self.save_index(self.active)?;
            }
            self.checkpoint()?;
            self.dying_step(u64::MAX)
        })();
        r.map_err(|e| e.to_string())?;
        match &self.failed {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    /// Bytes on disk by what they are. Reads the header of every record
    /// of every data file, not the content.
    pub fn disk_stats(&self) -> DiskStats {
        let mut s = DiskStats { tree: self.ckp_size + 20, ..DiskStats::default() };
        for seg in self.segs.iter().flatten() {
            s.data_files += 1;
            s.text += seg.live_bytes;
            let mut at = SEG_HEADER;
            let (mut framing, mut log, mut content) = (SEG_HEADER, 0u64, 0u64);
            while let Some(rec) = seg.record_at(at) {
                match rec.kind {
                    REC_CONTENT => {
                        let table = 8 + u32::from_le_bytes(rec.payload[4..8].try_into().unwrap()) as u64 * disk::TABLE_ENTRY;
                        framing += REC_HEADER + table;
                        content += rec.payload.len() as u64 - table;
                    }
                    REC_META => log += rec.end - at,
                    _ => framing += rec.end - at,
                }
                at = rec.end;
            }
            s.framing += framing;
            s.log += log;
            s.unreclaimed += content - seg.live_bytes + (seg.end - at);
            if let Some(p) = &seg.packed {
                let (lists, tables, rest) = p.sizes();
                s.lists += lists;
                s.tables += tables + rest;
            }
        }
        s.raw_text = self.content_bytes() as u64;
        s
    }

    /// Check every checksum and every rule the files are meant to obey:
    /// each record of each data file, each piece of each file, the index
    /// files block by block, and that the index says of every piece
    /// exactly what an index built now from its content would say.
    /// Returns what was checked, or the first thing found wrong.
    pub fn check(&self) -> Result<String, String> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.check_inner())) {
            Ok(r) => r,
            Err(payload) => match payload.downcast::<Damage>() {
                Ok(d) => Err(d.0),
                Err(other) => std::panic::resume_unwind(other),
            },
        }
    }

    fn check_inner(&self) -> Result<String, String> {
        // The tree.
        let (mut files, mut folders, mut pieces, mut bytes) = (0u64, 0u64, 0u64, 0u64);
        let mut by_seg: Vec<Vec<(Piece, NodeId)>> = vec![Vec::new(); self.segs.len()];
        for node in self.nodes.iter() {
            if node.id != self.root {
                let ok = is_valid_name(&node.name) && node.parent.and_then(|p| self.nodes.get(p)).is_some_and(|p| matches!(&p.kind, NodeKind::Folder(c) if c.get(&node.name) == Some(&node.id)));
                if !ok {
                    return Err(format!("the tree is not sound: node {} is not in its folder under its name", node.id));
                }
            }
            match &node.kind {
                NodeKind::Folder(c) => {
                    folders += 1;
                    let mut below = 0;
                    for child in c.values() {
                        match self.nodes.get(*child) {
                            Some(n) if n.parent == Some(node.id) => below += if matches!(n.kind, NodeKind::File(_)) { 1 } else { n.aux },
                            _ => return Err(format!("the tree is not sound: folder {} lists a child that is not its own", node.id)),
                        }
                    }
                    if below != node.aux {
                        return Err(format!("the tree is not sound: folder {} counts {} files below it and has {}", node.id, node.aux, below));
                    }
                }
                NodeKind::File(t) => {
                    files += 1;
                    let mut len = 0u64;
                    for (i, p) in t.pieces.iter().enumerate() {
                        let seg = self.segs.get(p.seg as usize).and_then(|s| s.as_ref()).ok_or(format!("file {} has content in data file {}, which is not there", node.id, p.seg))?;
                        let recorded = match seg.owners.get(p.ord as usize) {
                            Some(&SHARED) => seg.shared.get(&p.ord).is_some_and(|l| l.contains(&node.id)),
                            Some(o) => *o == node.id,
                            None => false,
                        };
                        if !recorded {
                            return Err(format!("{}: piece {} is not recorded as belonging to file {}", seg.name(), p.ord, node.id));
                        }
                        let text = self.text_of(p);
                        if disk::crc(seg.bytes(p.off as u64, p.slen as usize)) != p.crc {
                            return Err(format!("{}: damaged: the {} bytes at offset {} do not match their checksum", seg.name(), p.slen, p.off));
                        }
                        let sound = text.len() == p.len as usize && p.len > 0 && count_newlines(&text) == p.lines as usize && (i + 1 == t.pieces.len() || text.ends_with(b"\n")) && is_valid_text(&text);
                        if !sound {
                            return Err(format!("{}: the piece at offset {} is not what the tree says it is (length, lines, or a line cut in two)", seg.name(), p.off));
                        }
                        len += p.len as u64;
                        by_seg[p.seg as usize].push((*p, node.id));
                    }
                    if len != t.len {
                        return Err(format!("the tree is not sound: file {} is {} bytes and its pieces are {}", node.id, t.len, len));
                    }
                    bytes += len;
                }
            }
        }
        // A piece that several files use is one piece; the uses must be
        // the ones recorded.
        for (id, live) in by_seg.iter_mut().enumerate() {
            live.sort_by_key(|p| p.0.ord);
            let mut i = 0;
            while i < live.len() {
                let mut j = i + 1;
                while j < live.len() && live[j].0.ord == live[i].0.ord {
                    j += 1;
                }
                let seg = self.segs[id].as_ref().unwrap();
                if live[i..j].iter().any(|l| l.0 != live[i].0) || seg.uses(live[i].0.ord) != j - i {
                    return Err(format!("{}: piece {} is used {} times by files, and {} uses are recorded", seg.name(), live[i].0.ord, j - i, seg.uses(live[i].0.ord)));
                }
                i = j;
            }
            live.dedup_by_key(|p| p.0.ord);
            pieces += live.len() as u64;
        }
        if pieces != self.live_pieces {
            return Err(format!("the tree is not sound: {} pieces are counted as in use and files have {}", self.live_pieces, pieces));
        }
        // The data files, record by record.
        let mut records = 0u64;
        for seg in self.segs.iter().flatten() {
            seg.check_header()?;
            let live = &mut by_seg[seg.id as usize];
            live.sort_unstable_by_key(|p| p.0.ord);
            if seg.live_pieces as usize != live.len() || seg.owners.iter().filter(|o| **o != DEAD).count() != live.len() {
                return Err(format!("{}: {} pieces are counted as in use and files have {}", seg.name(), seg.live_pieces, live.len()));
            }
            let (mut at, mut ords, mut next_live) = (SEG_HEADER, 0u32, 0usize);
            let mut ended = false;
            while let Some(rec) = seg.record_at(at) {
                if ended {
                    return Err(format!("{}: has records after the one that ends it", seg.name()));
                }
                match rec.kind {
                    REC_CONTENT => {
                        if !seg.content_intact(at, &rec) {
                            return Err(format!("{}: damaged: the content record at offset {} does not match its checksums", seg.name(), at));
                        }
                        for p in seg.content_table(at, rec.payload) {
                            if p.ord != ords {
                                return Err(format!("{}: the content record at offset {} numbers its pieces from {} and {} was due", seg.name(), at, p.ord, ords));
                            }
                            ords += 1;
                            if live.get(next_live).is_some_and(|l| l.0.ord == p.ord) {
                                if live[next_live].0 != p {
                                    return Err(format!("{}: the tree and the data file disagree about piece {}", seg.name(), p.ord));
                                }
                                next_live += 1;
                            }
                        }
                    }
                    REC_NEXT => ended = true,
                    _ => {}
                }
                records += 1;
                at = rec.end;
            }
            if at != seg.end {
                return Err(format!("{}: damaged: no whole record at offset {}, and the file goes on to {}", seg.name(), at, seg.end));
            }
            if next_live != live.len() {
                return Err(format!("{}: a file has a piece here that no content record holds", seg.name()));
            }
            // In memory a data file that is full is not given a closing
            // record: there is no one to read it after a crash.
            if ended != seg.sealed && seg.id != self.active && !self.volatile {
                return Err(format!("{}: {} and {}", seg.name(), if ended { "has its closing record" } else { "has no closing record" }, if seg.sealed { "is counted as closed" } else { "is counted as still written to" }));
            }
            if let Some(p) = &seg.packed {
                p.verify_all();
            }
        }
        // The index against one built now.
        let mut lists = 0u64;
        if self.cfg.index {
            for seg in self.segs.iter().flatten() {
                let being_written = self.idx_job.as_ref().is_some_and(|j| j.seg == seg.id);
                if seg.sealed && !being_written && (seg.dynamic.is_some() || (seg.indexed as usize) < seg.owners.len()) {
                    return Err(format!("{}: is finished but its index file does not cover it", seg.name()));
                }
                let live = &by_seg[seg.id as usize];
                let mut fresh = Index::new();
                let mut next = 0;
                for ord in 0..seg.indexed {
                    if live.get(next).is_some_and(|l| l.0.ord == ord) {
                        fresh.add(&self.text_of(&live[next].0));
                        next += 1;
                    } else {
                        fresh.add(b"");
                    }
                }
                let alive = |d: &u32| seg.owners.get(*d as usize - 1).is_some_and(|o| *o != DEAD);
                let mut seen: Vec<Trigram> = fresh.trigrams();
                let stored: Vec<Trigram> = seg.packed.as_ref().map_or(Vec::new(), |p| (0..p.ntrigrams).map(|i| p.trigram_at(i)).collect());
                if stored.windows(2).any(|w| w[0] >= w[1]) {
                    return Err(format!("{}: its index file does not list trigrams in order", seg.name()));
                }
                seen.extend(stored);
                seen.extend(seg.dynamic.as_ref().map_or(Vec::new(), |d| d.trigrams()));
                seen.sort_unstable();
                seen.dedup();
                for t in seen {
                    let (a, b) = seg.lists(t);
                    let have: Vec<u32> = Postings::new(a).chain(Postings::new(b)).filter(alive).collect();
                    let want: Vec<u32> = Postings::new(fresh.list(t)).filter(alive).collect();
                    if have != want {
                        return Err(format!("{}: the index is wrong for trigram {:06x}: it lists pieces {:?} and the content has it in {:?}", seg.name(), t, have, want));
                    }
                    lists += 1;
                }
            }
        }
        Ok(format!(
            "{} files and {} folders, {} bytes of text in {} pieces, {} data files with {} records, {} posting lists compared with a fresh index",
            files,
            folders,
            bytes,
            pieces,
            self.segs.iter().flatten().count(),
            records,
            lists
        ))
    }
}

fn encode_node(node: &Node, out: &mut Vec<u8>) {
    match &node.kind {
        NodeKind::Folder(_) => {
            disk::put_u64(out, 1);
            disk::put_u64(out, node.id);
            disk::put_u64(out, node.parent.map_or(0, |p| p + 1));
            disk::put_bytes(out, &node.name);
            disk::put_u64(out, node.mtime as u64);
        }
        NodeKind::File(t) => {
            disk::put_u64(out, 2);
            disk::put_u64(out, node.id);
            disk::put_u64(out, node.parent.map_or(0, |p| p + 1));
            disk::put_bytes(out, &node.name);
            disk::put_u64(out, node.mtime as u64);
            disk::put_u64(out, t.ver as u64);
            disk::put_u64(out, t.pieces.len() as u64);
            for p in &t.pieces {
                p.encode(out);
            }
        }
    }
}
