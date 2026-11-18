//! The files of a database and what is in them. `docs/FORMAT.md` is the
//! reference for the layout; this file is the code that reads and writes
//! it. Nothing here knows about folders or commands: `tree.rs` decides
//! what to store, this file stores it.
//!
//! There are four kinds of file, all in one directory:
//!
//! - **Data files** (`NNNNNNNN.seg`), written only at their end. A data
//!   file is a row of records: pieces of file content, and one record per
//!   command line that says what the command changed in the tree. Read in
//!   order, those records are the log.
//! - **Index files** (`NNNNNNNN.idx`), one per data file: the posting
//!   lists for the pieces in it, end to end. Derived; can be rebuilt.
//! - **A checkpoint** (`tree-NNNNNNNN.ckp`): the folder tree as of some
//!   position in the log.
//! - **`MANIFEST`**: which checkpoint is current. Replaced in one step.

use crate::index::{Index, Lists};
use crate::query::Trigram;
use crate::tree::NodeId;
use crate::vfs::{VFile, Vfs};
use std::cell::RefCell;
use std::io;

pub const FORMAT_VERSION: u32 = 1;
/// How content was folded for the index: 1 means ASCII letters lowered
/// and nothing else (`bytes::fold`).
pub const FOLD_ASCII: u32 = 1;

pub const SEG_MAGIC: &[u8; 8] = b"QDBSEG1\n";
pub const IDX_MAGIC: &[u8; 8] = b"QDBIDX1\n";
pub const IDX_END: &[u8; 8] = b"QDBIDXE\n";
pub const TREE_MAGIC: &[u8; 8] = b"QDBTREE\n";
pub const TREE_END: &[u8; 8] = b"QDBTREND";
pub const MANIFEST_MAGIC: &[u8; 8] = b"QDBMANI\n";

pub const SEG_HEADER: u64 = 32;
pub const REC_HEADER: u64 = 12;
pub const REC_CONTENT: u8 = 1;
pub const REC_META: u8 = 2;
pub const REC_NEXT: u8 = 3;

/// No file owns this piece any more.
/// Bytes one piece takes in the table at the start of a content record.
pub const TABLE_ENTRY: u64 = 20;

/// How a piece's bytes are stored.
pub const ENC_PLAIN: u8 = 0;
/// An LZ4 block that unpacks to the piece's `len` bytes of text.
pub const ENC_LZ4: u8 = 1;

pub const DEAD: NodeId = NodeId::MAX;

pub fn seg_name(id: u32) -> String {
    format!("{:08}.seg", id)
}

pub fn idx_name(id: u32) -> String {
    format!("{:08}.idx", id)
}

pub fn ckp_name(n: u32) -> String {
    format!("tree-{:08}.ckp", n)
}

pub fn crc(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

/// A failure that means the files cannot be trusted: a checksum that does
/// not match, a file that is missing or cut short. Raised as a panic
/// payload from reads, which have no other way to fail; `main` and the
/// tests catch it.
///
/// The second field is set when the damaged file is the index file of
/// that data file: an index can be built again, so this is not the end
/// of the database. See `Db::after_damage`.
#[derive(Debug, Clone)]
pub struct Damage(pub String, pub Option<u32>);

pub fn damage(msg: String) -> ! {
    std::panic::panic_any(Damage(msg, None))
}

pub fn bad(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

// ---------------------------------------------------------------- varints

pub fn put_u64(out: &mut Vec<u8>, mut n: u64) {
    while n >= 0x80 {
        out.push(n as u8 | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}

pub fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    put_u64(out, b.len() as u64);
    out.extend_from_slice(b);
}

/// Reads varints and byte strings from a buffer. An attempt to read past
/// the end sets `bad` and gives zeros, so a caller can decode a whole
/// entry and check once.
pub struct Reader<'a> {
    pub buf: &'a [u8],
    pub at: usize,
    pub bad: bool,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Reader<'a> {
        Reader { buf, at: 0, bad: false }
    }

    pub fn done(&self) -> bool {
        self.at >= self.buf.len()
    }

    pub fn u64(&mut self) -> u64 {
        let mut n = 0u64;
        let mut shift = 0;
        loop {
            let Some(&b) = self.buf.get(self.at) else {
                self.bad = true;
                return 0;
            };
            self.at += 1;
            if shift < 64 {
                n |= ((b & 0x7f) as u64) << shift;
            }
            if b < 0x80 {
                return n;
            }
            shift += 7;
        }
    }

    pub fn u32(&mut self) -> u32 {
        let n = self.u64();
        if n > u32::MAX as u64 {
            self.bad = true;
        }
        n as u32
    }

    pub fn bytes(&mut self) -> &'a [u8] {
        let len = self.u64() as usize;
        if self.bad || len > self.buf.len() - self.at {
            self.bad = true;
            return &[];
        }
        let b = &self.buf[self.at..self.at + len];
        self.at += len;
        b
    }
}

// ----------------------------------------------------------------- pieces

/// Where one piece of a file's content is: in which data file, at what
/// offset, how long. A file is a row of these; every piece but the last
/// ends with a newline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// The data file.
    pub seg: u32,
    /// The piece's position among the pieces of that data file, from 0.
    /// The index knows the piece as document `ord + 1`.
    pub ord: u32,
    /// Offset of the stored bytes in the data file.
    pub off: u32,
    /// Stored length. Equal to `len` when the piece is stored as it is.
    pub slen: u32,
    /// Length of the text.
    pub len: u32,
    /// Newlines in the text.
    pub lines: u32,
    /// Checksum of the stored bytes.
    pub crc: u32,
    /// How the bytes are stored: `ENC_PLAIN` or a codec.
    pub enc: u8,
}

impl Piece {
    pub fn encode(&self, out: &mut Vec<u8>) {
        for n in [self.seg, self.ord, self.off, self.slen, self.len, self.lines, self.crc, self.enc as u32] {
            put_u64(out, n as u64);
        }
    }

    pub fn decode(r: &mut Reader) -> Piece {
        let mut p = Piece { seg: r.u32(), ord: r.u32(), off: r.u32(), slen: r.u32(), len: r.u32(), lines: r.u32(), crc: r.u32(), enc: 0 };
        let enc = r.u32();
        if enc > u8::MAX as u32 {
            r.bad = true;
        }
        p.enc = enc as u8;
        p
    }
}

/// A piece on its way into a data file.
pub struct Stored<'a> {
    pub bytes: &'a [u8],
    /// Length of the text, which differs from `bytes.len()` if compressed.
    pub len: u32,
    pub lines: u32,
    pub enc: u8,
}

// ------------------------------------------------------------ data files

/// One record of a data file, as found by `Segment::record_at`.
pub struct Record<'a> {
    pub kind: u8,
    pub payload: &'a [u8],
    /// Offset of the record after this one.
    pub end: u64,
}

pub struct Segment {
    pub id: u32,
    name: String,
    file: Box<dyn VFile>,
    /// Bytes of valid data; the next record goes here.
    pub end: u64,
    /// How large the file may grow.
    pub cap: u64,
    /// The owner of each piece by `ord`: a file, or `DEAD`.
    pub owners: Vec<NodeId>,
    pub live_pieces: u32,
    /// Stored bytes of the pieces that have an owner.
    pub live_bytes: u64,
    /// Which pieces have had their checksum compared since the database
    /// was opened: each is checked the first time it is read.
    checked: RefCell<Vec<u64>>,
    /// False when the files are in memory and cannot have been damaged.
    check: bool,
    /// The index of the pieces below `packed.ndocs`, from the index file.
    pub packed: Option<Packed>,
    /// The index of the pieces from there up to `indexed`, in memory.
    pub dynamic: Option<Index>,
    /// Pieces at or above this are in no index and are always searched.
    pub indexed: u32,
    /// False for the one data file still being written.
    pub sealed: bool,
}

impl Segment {
    /// Start a new data file.
    pub fn create(vfs: &dyn Vfs, id: u32, cap: u64) -> io::Result<Segment> {
        let name = seg_name(id);
        let mut file = vfs.create(&name, cap)?;
        let mut h = Vec::with_capacity(SEG_HEADER as usize);
        h.extend_from_slice(SEG_MAGIC);
        h.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        h.extend_from_slice(&id.to_le_bytes());
        h.extend_from_slice(&FOLD_ASCII.to_le_bytes());
        h.extend_from_slice(&[0u8; 8]);
        let c = crc(&h);
        h.extend_from_slice(&c.to_le_bytes());
        file.write_at(0, &h)?;
        Ok(Segment::with(vfs, id, name, file, cap))
    }

    fn with(vfs: &dyn Vfs, id: u32, name: String, file: Box<dyn VFile>, cap: u64) -> Segment {
        let end = file.len();
        Segment {
            id,
            name: vfs.describe(&name),
            file,
            end,
            // Room is kept for the record that names the next data file,
            // also in a file that was made larger than usual for one big
            // record and is now full.
            cap: if cap > 0 { cap.max(end + 4096) } else { end },
            owners: Vec::new(),
            live_pieces: 0,
            live_bytes: 0,
            checked: RefCell::new(Vec::new()),
            check: !vfs.volatile(),
            packed: None,
            dynamic: None,
            indexed: 0,
            sealed: true,
        }
    }

    /// Open an existing data file and check its header.
    pub fn open(vfs: &dyn Vfs, id: u32, cap: u64) -> io::Result<Segment> {
        let name = seg_name(id);
        let file = vfs.open(&name, cap).map_err(|e| io::Error::new(e.kind(), format!("{}: {}", vfs.describe(&name), e)))?;
        let seg = Segment::with(vfs, id, name, file, cap);
        seg.check_header().map_err(bad)?;
        Ok(seg)
    }

    pub fn check_header(&self) -> Result<(), String> {
        if self.end < SEG_HEADER {
            return Err(format!("{}: too short to be a data file", self.name));
        }
        let h = self.bytes(0, SEG_HEADER as usize);
        if &h[..8] != SEG_MAGIC || crc(&h[..28]) != u32_at(h, 28) {
            return Err(format!("{}: not a qdb data file, or its header is damaged", self.name));
        }
        if u32_at(h, 8) != FORMAT_VERSION {
            return Err(format!("{}: written by a newer version of qdb (format {})", self.name, u32_at(h, 8)));
        }
        if u32_at(h, 12) != self.id {
            return Err(format!("{}: is data file {} under another name", self.name, u32_at(h, 12)));
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Bytes of the file, without any check.
    #[inline]
    pub fn bytes(&self, off: u64, len: usize) -> &[u8] {
        if off + len as u64 > self.end {
            damage(format!("{}: is cut short: {} bytes at offset {} are wanted and it has {}", self.name, len, off, self.end));
        }
        // SAFETY: the range lies inside the file, the view stays where it
        // is until the file is changed, and changing it takes `&mut self`.
        unsafe { std::slice::from_raw_parts(self.file.base().add(off as usize), len) }
    }

    /// The stored bytes of a piece. The first read of a piece compares
    /// its checksum.
    #[inline]
    pub fn piece(&self, p: &Piece) -> &[u8] {
        let bytes = self.bytes(p.off as u64, p.slen as usize);
        if self.check {
            let (word, bit) = (p.ord as usize / 64, 1u64 << (p.ord % 64));
            let mut checked = self.checked.borrow_mut();
            if checked.len() <= word {
                checked.resize(word + 1, 0);
            }
            if checked[word] & bit == 0 {
                if crc(bytes) != p.crc {
                    damage(format!("{}: damaged: the {} bytes at offset {} do not match their checksum", self.name, p.slen, p.off));
                }
                checked[word] |= bit;
            }
        }
        bytes
    }

    /// Whether a record with this much payload still fits, leaving room
    /// for the record that names the next data file.
    pub fn fits(&self, payload: u64) -> bool {
        self.end + REC_HEADER + payload + REC_HEADER + 4 <= self.cap
    }

    fn append(&mut self, kind: u8, covered: &[u8], rest: &[&[u8]]) -> io::Result<()> {
        let len: usize = covered.len() + rest.iter().map(|r| r.len()).sum::<usize>();
        let mut head = Vec::with_capacity(REC_HEADER as usize + covered.len());
        head.extend_from_slice(&(len as u32).to_le_bytes());
        head.extend_from_slice(&[kind, 0, 0, 0]);
        let mut h = crc32fast::Hasher::new();
        h.update(&head);
        h.update(covered);
        head.extend_from_slice(&h.finalize().to_le_bytes());
        head.extend_from_slice(covered);
        // One write for a small record; a large one goes piece by piece.
        let total = head.len() + len - covered.len();
        if total <= 256 * 1024 {
            for r in rest {
                head.extend_from_slice(r);
            }
            self.file.write_at(self.end, &head)?;
        } else {
            self.file.write_at(self.end, &head)?;
            let mut at = self.end + head.len() as u64;
            for r in rest {
                self.file.write_at(at, r)?;
                at += r.len() as u64;
            }
        }
        self.end += total as u64;
        Ok(())
    }

    /// Append pieces of content as one record. `owner` is the file they
    /// belong to. Returns where each went.
    pub fn append_content(&mut self, items: &[Stored], owner: NodeId) -> io::Result<Vec<Piece>> {
        let first = self.owners.len() as u32;
        let mut table = Vec::with_capacity(8 + items.len() * TABLE_ENTRY as usize);
        table.extend_from_slice(&first.to_le_bytes());
        table.extend_from_slice(&(items.len() as u32).to_le_bytes());
        let mut off = self.end + REC_HEADER + 8 + items.len() as u64 * TABLE_ENTRY;
        let mut pieces = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let c = crc(item.bytes);
            for n in [item.enc as u32, item.bytes.len() as u32, item.len, item.lines, c] {
                table.extend_from_slice(&n.to_le_bytes());
            }
            pieces.push(Piece { seg: self.id, ord: first + i as u32, off: off as u32, slen: item.bytes.len() as u32, len: item.len, lines: item.lines, crc: c, enc: item.enc });
            off += item.bytes.len() as u64;
        }
        let rest: Vec<&[u8]> = items.iter().map(|i| i.bytes).collect();
        self.append(REC_CONTENT, &table, &rest)?;
        for p in &pieces {
            self.owners.push(owner);
            self.live_pieces += 1;
            self.live_bytes += p.slen as u64;
        }
        // What this process wrote it need not check on reading back.
        if self.check {
            let mut checked = self.checked.borrow_mut();
            checked.resize(self.owners.len() / 64 + 1, 0);
            for p in &pieces {
                checked[p.ord as usize / 64] |= 1u64 << (p.ord % 64);
            }
        }
        Ok(pieces)
    }

    /// Append the record of one command line: its number, how much of
    /// this file was known to be on disk when it was written, and what
    /// the command changed.
    pub fn append_meta(&mut self, seq: u64, synced: u64, ops: &[u8]) -> io::Result<()> {
        let mut payload = Vec::with_capacity(16 + ops.len());
        payload.extend_from_slice(&seq.to_le_bytes());
        payload.extend_from_slice(&synced.to_le_bytes());
        payload.extend_from_slice(ops);
        self.append(REC_META, &payload, &[])
    }

    /// Append the record that ends this file and names the one the log
    /// goes on in.
    pub fn append_next(&mut self, next: u32) -> io::Result<()> {
        self.append(REC_NEXT, &next.to_le_bytes(), &[])
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.file.sync()
    }

    pub fn truncate(&mut self, len: u64) -> io::Result<()> {
        self.file.truncate(len)?;
        self.end = len;
        Ok(())
    }

    /// The record at an offset, if a whole one with a matching checksum
    /// is there. The content of a content record is not looked at.
    pub fn record_at(&self, off: u64) -> Option<Record<'_>> {
        if off + REC_HEADER > self.end {
            return None;
        }
        let head = self.bytes(off, REC_HEADER as usize);
        let len = u32_at(head, 0) as u64;
        let kind = head[4];
        if off + REC_HEADER + len > self.end || head[5..8] != [0, 0, 0] {
            return None;
        }
        let payload = self.bytes(off + REC_HEADER, len as usize);
        let covered = match kind {
            REC_CONTENT => {
                if len < 8 {
                    return None;
                }
                let n = u32_at(payload, 4) as u64;
                if 8 + n * TABLE_ENTRY > len {
                    return None;
                }
                (8 + n * TABLE_ENTRY) as usize
            }
            REC_META if len >= 16 => len as usize,
            REC_NEXT if len == 4 => 4,
            _ => return None,
        };
        let mut h = crc32fast::Hasher::new();
        h.update(&head[..8]);
        h.update(&payload[..covered]);
        if h.finalize() != u32_at(head, 8) {
            return None;
        }
        Some(Record { kind, payload, end: off + REC_HEADER + len })
    }

    /// The pieces of a content record: (ord, offset, stored length, text
    /// length, lines, checksum) each. `at` is the record's offset.
    pub fn content_table(&self, at: u64, payload: &[u8]) -> Vec<Piece> {
        let first = u32_at(payload, 0);
        let n = u32_at(payload, 4) as usize;
        let mut off = at + REC_HEADER + 8 + n as u64 * TABLE_ENTRY;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let e = &payload[8 + i * 20..28 + i * 20];
            let p = Piece { seg: self.id, ord: first + i as u32, off: off as u32, slen: u32_at(e, 4), len: u32_at(e, 8), lines: u32_at(e, 12), crc: u32_at(e, 16), enc: e[0] };
            off += p.slen as u64;
            out.push(p);
        }
        out
    }

    /// True if the stored bytes of every piece of a content record match
    /// their checksums and lie inside the record.
    pub fn content_intact(&self, at: u64, rec: &Record) -> bool {
        let table = self.content_table(at, rec.payload);
        let bytes: u64 = table.iter().map(|p| p.slen as u64).sum();
        if at + REC_HEADER + 8 + table.len() as u64 * TABLE_ENTRY + bytes != rec.end {
            return false;
        }
        table.iter().all(|p| crc(self.bytes(p.off as u64, p.slen as usize)) == p.crc)
    }

    /// Make room in `owners` for pieces up to `ords`.
    pub fn grow_owners(&mut self, ords: u32) {
        if self.owners.len() < ords as usize {
            self.owners.resize(ords as usize, DEAD);
        }
    }

    /// Note that a piece is owned by a file. Used when the tree is read
    /// at open.
    pub fn adopt(&mut self, p: &Piece, owner: NodeId) {
        self.grow_owners(p.ord + 1);
        self.owners[p.ord as usize] = owner;
        self.live_pieces += 1;
        self.live_bytes += p.slen as u64;
    }

    /// The piece's content is no longer in any file.
    pub fn retire(&mut self, p: &Piece) {
        let o = &mut self.owners[p.ord as usize];
        if *o == DEAD {
            return;
        }
        *o = DEAD;
        self.live_pieces -= 1;
        self.live_bytes -= p.slen as u64;
        if let Some(index) = &mut self.dynamic {
            let doc = p.ord + 1;
            if doc < index.next_doc() && p.ord >= self.packed.as_ref().map_or(0, |k| k.ndocs) {
                index.retire(doc);
            }
        }
    }

    /// Lists for a trigram from both parts of the index.
    pub fn lists(&self, t: Trigram) -> (&[u8], &[u8]) {
        (self.packed.as_ref().map_or(&[][..], |p| p.list(t)), self.dynamic.as_ref().map_or(&[][..], |d| d.list(t)))
    }
}

// ----------------------------------------------------------- index files

const IDX_HEADER: u64 = 16;
const IDX_FOOTER: usize = 72;
const BLOCK: usize = 4096;
const OFFSET_MASK: u64 = (1 << 40) - 1;

/// An index file, read in place.
///
/// ```text
/// header   magic, format version, data file number
/// lists    every posting list, end to end, in trigram order
/// table    one 8-byte entry per trigram, ascending: trigram << 40 | offset
///          of its list within `lists`; then one more entry, for the end
/// buckets  65,537 4-byte numbers: for each value of a trigram's first two
///          bytes, the position in `table` of the first trigram at or
///          above it
/// sums     a checksum for every 4,096 bytes of all of the above
/// footer   where each part is, how many pieces the lists cover, which
///          folding rule built them, checksums of `sums` and of itself
/// ```
pub struct Packed {
    seg: u32,
    name: String,
    file: Box<dyn VFile>,
    len: u64,
    /// The lists cover the pieces with `ord` below this.
    pub ndocs: u32,
    pub ntrigrams: u32,
    lists_off: u64,
    pub lists_len: u64,
    table_off: u64,
    buckets_off: u64,
    sums_off: u64,
    checked: RefCell<Vec<u64>>,
    check: bool,
}

impl Packed {
    fn raw(&self, off: u64, len: usize) -> &[u8] {
        debug_assert!(off + len as u64 <= self.len);
        // SAFETY: inside the file, which is never changed once written.
        unsafe { std::slice::from_raw_parts(self.file.base().add(off as usize), len) }
    }

    /// Compare the checksums of the blocks that hold a range, once each.
    fn verify(&self, off: u64, len: usize) {
        if !self.verified(off, len) {
            std::panic::panic_any(Damage(format!("{}: damaged: a block of it does not match its checksum", self.name), Some(self.seg)));
        }
    }

    fn verified(&self, off: u64, len: usize) -> bool {
        if !self.check || len == 0 {
            return true;
        }
        let mut checked = self.checked.borrow_mut();
        for block in off as usize / BLOCK..=(off as usize + len - 1) / BLOCK {
            if checked[block / 64] & (1 << (block % 64)) != 0 {
                continue;
            }
            let start = block * BLOCK;
            let end = (start + BLOCK).min(self.sums_off as usize);
            let want = u32_at(self.raw(self.sums_off + block as u64 * 4, 4), 0);
            if crc(self.raw(start as u64, end - start)) != want {
                return false;
            }
            checked[block / 64] |= 1 << (block % 64);
        }
        true
    }

    /// Open an index file. None if it is not there; an error if it is
    /// there and cannot be used.
    pub fn open(vfs: &dyn Vfs, seg: u32) -> Result<Option<Packed>, String> {
        let name = idx_name(seg);
        let file = match vfs.open(&name, 0) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {}", vfs.describe(&name), e)),
        };
        let len = file.len();
        let name = vfs.describe(&name);
        let mut p = Packed { seg, name, file, len, ndocs: 0, ntrigrams: 0, lists_off: IDX_HEADER, lists_len: 0, table_off: 0, buckets_off: 0, sums_off: 0, checked: RefCell::new(Vec::new()), check: !vfs.volatile() };
        if len < IDX_HEADER + IDX_FOOTER as u64 {
            return Err(format!("{}: too short to be an index file", p.name));
        }
        let head = &p.raw(0, IDX_HEADER as usize).to_vec()[..];
        let foot = &p.raw(len - IDX_FOOTER as u64, IDX_FOOTER).to_vec()[..];
        if &head[..8] != IDX_MAGIC || &foot[..8] != IDX_END || crc(&foot[..68]) != u32_at(foot, 68) {
            return Err(format!("{}: not a complete index file", p.name));
        }
        if u32_at(head, 8) != FORMAT_VERSION {
            return Err(format!("{}: written by a newer version of qdb (format {})", p.name, u32_at(head, 8)));
        }
        if u32_at(foot, 56) != FOLD_ASCII {
            return Err(format!("{}: built with folding rule {}, which this version does not know", p.name, u32_at(foot, 56)));
        }
        p.ndocs = u32_at(foot, 8);
        p.ntrigrams = u32_at(foot, 12);
        p.lists_off = u64_at(foot, 16);
        p.lists_len = u64_at(foot, 24);
        p.table_off = u64_at(foot, 32);
        p.buckets_off = u64_at(foot, 40);
        p.sums_off = u64_at(foot, 48);
        let blocks = (p.sums_off as usize + BLOCK - 1) / BLOCK;
        let sound = u32_at(head, 12) == seg
            && p.lists_off == IDX_HEADER
            && p.table_off == p.lists_off + p.lists_len
            && p.buckets_off == p.table_off + (p.ntrigrams as u64 + 1) * 8
            && p.sums_off == p.buckets_off + 65537 * 4
            && len == p.sums_off + blocks as u64 * 4 + IDX_FOOTER as u64
            && crc(p.raw(p.sums_off, blocks * 4)) == u32_at(foot, 60);
        if !sound {
            return Err(format!("{}: damaged: its parts are not where its footer says", p.name));
        }
        *p.checked.borrow_mut() = vec![0u64; blocks / 64 + 1];
        // The table and the buckets are read by every search: check them
        // now. The lists are checked as they are read.
        if !p.verified(0, IDX_HEADER as usize) || !p.verified(p.table_off, (p.sums_off - p.table_off) as usize) {
            return Err(format!("{}: damaged: its table does not match its checksums", p.name));
        }
        Ok(Some(p))
    }

    fn entry(&self, i: u32) -> u64 {
        u64_at(self.raw(self.table_off + i as u64 * 8, 8), 0)
    }

    /// The trigram at a position of the table.
    pub fn trigram_at(&self, i: u32) -> Trigram {
        (self.entry(i) >> 40) as Trigram
    }

    /// The list at a position of the table.
    pub fn list_at(&self, i: u32) -> &[u8] {
        let (a, b) = (self.entry(i) & OFFSET_MASK, self.entry(i + 1) & OFFSET_MASK);
        self.verify(self.lists_off + a, (b - a) as usize);
        self.raw(self.lists_off + a, (b - a) as usize)
    }

    /// Bytes of the file by part: (lists, table and buckets, checksums
    /// and the rest).
    pub fn sizes(&self) -> (u64, u64, u64) {
        (self.lists_len, self.sums_off - self.table_off, self.len - self.sums_off + IDX_HEADER)
    }

    /// Compare every checksum in the file.
    pub fn verify_all(&self) {
        self.verify(0, self.sums_off as usize);
    }
}

impl Lists for Packed {
    #[inline]
    fn list(&self, t: Trigram) -> &[u8] {
        let b = (t >> 8) as u64;
        let range = self.raw(self.buckets_off + b * 4, 8);
        let (mut lo, hi) = (u32_at(range, 0), u32_at(range, 4));
        while lo < hi {
            let e = self.entry(lo);
            let found = (e >> 40) as Trigram;
            if found == t {
                return self.list_at(lo);
            }
            if found > t {
                break;
            }
            lo += 1;
        }
        &[]
    }
}

/// Writes an index file a part at a time: the lists of an existing index
/// file, if there is one, each followed by the list the in-memory index
/// has for the same trigram.
pub struct PackedWriter {
    seg: u32,
    file: Box<dyn VFile>,
    /// Bytes not yet handed to the file, and the offset they start at.
    buf: Vec<u8>,
    flushed: u64,
    sums: Vec<u32>,
    /// Every trigram of either source, ascending, and how far we are.
    keys: Vec<Trigram>,
    next: usize,
    old_at: u32,
    table: Vec<u64>,
}

pub fn idx_tmp(seg: u32) -> String {
    format!("{}.tmp", idx_name(seg))
}

impl PackedWriter {
    pub fn new(vfs: &dyn Vfs, seg: u32, old: Option<&Packed>, dynamic: Option<&Index>) -> io::Result<PackedWriter> {
        let mut keys: Vec<Trigram> = dynamic.map_or(Vec::new(), |d| d.trigrams());
        if let Some(old) = old {
            let fresh = std::mem::take(&mut keys);
            // Both ascending: merge.
            let (mut i, mut j) = (0u32, 0usize);
            while i < old.ntrigrams || j < fresh.len() {
                let a = if i < old.ntrigrams { old.trigram_at(i) } else { Trigram::MAX };
                let b = fresh.get(j).copied().unwrap_or(Trigram::MAX);
                keys.push(a.min(b));
                i += (a <= b) as u32;
                j += (b <= a) as usize;
            }
        }
        let file = vfs.create(&idx_tmp(seg), 0)?;
        let mut w = PackedWriter { seg, file, buf: Vec::new(), flushed: 0, sums: Vec::new(), keys, next: 0, old_at: 0, table: Vec::new() };
        w.buf.extend_from_slice(IDX_MAGIC);
        w.buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        w.buf.extend_from_slice(&seg.to_le_bytes());
        Ok(w)
    }

    fn pos(&self) -> u64 {
        self.flushed + self.buf.len() as u64
    }

    /// Hand whole blocks to the file, all of the buffer if `all`.
    fn flush(&mut self, all: bool) -> io::Result<()> {
        let whole = if all { self.buf.len() } else { self.buf.len() / BLOCK * BLOCK };
        if whole == 0 {
            return Ok(());
        }
        for block in self.buf[..whole].chunks(BLOCK) {
            self.sums.push(crc(block));
        }
        self.file.write_at(self.flushed, &self.buf[..whole])?;
        self.flushed += whole as u64;
        self.buf.drain(..whole);
        Ok(())
    }

    /// Write lists until about `budget` bytes have been written. Returns
    /// true when every list is written; `finish` is then due.
    pub fn step(&mut self, old: Option<&Packed>, dynamic: Option<&Index>, budget: usize) -> io::Result<bool> {
        let start = self.pos();
        while self.next < self.keys.len() {
            if (self.pos() - start) as usize >= budget {
                return Ok(false);
            }
            let t = self.keys[self.next];
            self.next += 1;
            self.table.push((t as u64) << 40 | (self.pos() - IDX_HEADER));
            let mut wrote = false;
            if let Some(old) = old {
                if self.old_at < old.ntrigrams && old.trigram_at(self.old_at) == t {
                    self.buf.extend_from_slice(old.list_at(self.old_at));
                    self.old_at += 1;
                    wrote = true;
                }
            }
            if let Some(d) = dynamic {
                let list = d.list(t);
                if !list.is_empty() {
                    if wrote {
                        // The second list counts documents from zero again.
                        self.buf.push(0);
                    }
                    self.buf.extend_from_slice(list);
                }
            }
            if self.buf.len() >= 1 << 20 {
                self.flush(false)?;
            }
        }
        Ok(true)
    }

    /// Write the rest of the file, sync it and give it its name. `ndocs`
    /// is how many pieces the lists cover.
    pub fn finish(mut self, vfs: &dyn Vfs, ndocs: u32) -> io::Result<()> {
        let lists_len = self.pos() - IDX_HEADER;
        let table_off = self.pos();
        self.table.push((Trigram::MAX as u64 & 0xff_ffff) << 40 | lists_len);
        let table = std::mem::take(&mut self.table);
        for e in &table {
            self.buf.extend_from_slice(&e.to_le_bytes());
        }
        let buckets_off = self.pos();
        let mut at = 0usize;
        for b in 0..=65536u64 {
            while at < self.keys.len() && ((self.keys[at] >> 8) as u64) < b {
                at += 1;
            }
            self.buf.extend_from_slice(&(at as u32).to_le_bytes());
        }
        let sums_off = self.pos();
        self.flush(true)?;
        let mut tail = Vec::with_capacity(self.sums.len() * 4 + IDX_FOOTER);
        for s in &self.sums {
            tail.extend_from_slice(&s.to_le_bytes());
        }
        let sums_crc = crc(&tail);
        let foot = tail.len();
        tail.extend_from_slice(IDX_END);
        tail.extend_from_slice(&ndocs.to_le_bytes());
        tail.extend_from_slice(&(self.keys.len() as u32).to_le_bytes());
        for n in [IDX_HEADER, lists_len, table_off, buckets_off, sums_off] {
            tail.extend_from_slice(&n.to_le_bytes());
        }
        tail.extend_from_slice(&FOLD_ASCII.to_le_bytes());
        tail.extend_from_slice(&sums_crc.to_le_bytes());
        tail.extend_from_slice(&[0u8; 4]);
        let c = crc(&tail[foot..]);
        tail.extend_from_slice(&c.to_le_bytes());
        self.file.write_at(sums_off, &tail)?;
        self.file.sync()?;
        drop(self.file);
        vfs.rename(&idx_tmp(self.seg), &idx_name(self.seg))?;
        vfs.sync_dir()
    }
}

// ------------------------------------------------------------ checkpoints

/// Writes a checkpoint file: a header, then blocks of entries, each block
/// with its length and checksum, then an end mark with the number of
/// blocks.
pub struct CheckpointWriter {
    file: Box<dyn VFile>,
    at: u64,
    block: Vec<u8>,
    blocks: u64,
    pub number: u32,
}

const CKP_BLOCK: usize = 64 * 1024;

pub fn ckp_tmp(n: u32) -> String {
    format!("{}.tmp", ckp_name(n))
}

impl CheckpointWriter {
    pub fn new(vfs: &dyn Vfs, number: u32) -> io::Result<CheckpointWriter> {
        let mut file = vfs.create(&ckp_tmp(number), 0)?;
        let mut h = Vec::new();
        h.extend_from_slice(TREE_MAGIC);
        h.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        h.extend_from_slice(&number.to_le_bytes());
        let c = crc(&h);
        h.extend_from_slice(&c.to_le_bytes());
        file.write_at(0, &h)?;
        Ok(CheckpointWriter { file, at: h.len() as u64, block: Vec::new(), blocks: 0, number })
    }

    /// Where the next entry goes. End the entry with `entry_done`.
    pub fn entry(&mut self) -> &mut Vec<u8> {
        &mut self.block
    }

    pub fn entry_done(&mut self) -> io::Result<()> {
        if self.block.len() >= CKP_BLOCK {
            self.end_block()?;
        }
        Ok(())
    }

    /// End the block here, so that the next entry starts a new one.
    pub fn end_block(&mut self) -> io::Result<()> {
        if self.block.is_empty() {
            return Ok(());
        }
        let mut head = Vec::with_capacity(8);
        head.extend_from_slice(&(self.block.len() as u32).to_le_bytes());
        head.extend_from_slice(&crc(&self.block).to_le_bytes());
        self.file.write_at(self.at, &head)?;
        self.file.write_at(self.at + 8, &self.block)?;
        self.at += 8 + self.block.len() as u64;
        self.block.clear();
        self.blocks += 1;
        Ok(())
    }

    pub fn bytes(&self) -> u64 {
        self.at + self.block.len() as u64
    }

    /// Finish the file, sync it, give it its name and make it the current
    /// checkpoint.
    pub fn finish(mut self, vfs: &dyn Vfs) -> io::Result<u64> {
        self.end_block()?;
        let mut end = Vec::new();
        end.extend_from_slice(&0u32.to_le_bytes());
        end.extend_from_slice(TREE_END);
        end.extend_from_slice(&self.blocks.to_le_bytes());
        let c = crc(&end);
        end.extend_from_slice(&c.to_le_bytes());
        self.file.write_at(self.at, &end)?;
        self.file.sync()?;
        let size = self.at + end.len() as u64;
        drop(self.file);
        vfs.rename(&ckp_tmp(self.number), &ckp_name(self.number))?;
        // The file must be known to the directory before MANIFEST points
        // at it.
        vfs.sync_dir()?;
        let mut m = Vec::new();
        m.extend_from_slice(MANIFEST_MAGIC);
        m.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        m.extend_from_slice(&self.number.to_le_bytes());
        let c = crc(&m);
        m.extend_from_slice(&c.to_le_bytes());
        let mut f = vfs.create("MANIFEST.tmp", 0)?;
        f.write_at(0, &m)?;
        f.sync()?;
        drop(f);
        vfs.rename("MANIFEST.tmp", "MANIFEST")?;
        vfs.sync_dir()?;
        Ok(size)
    }
}

/// Which checkpoint MANIFEST names. None if there is no MANIFEST.
pub fn read_manifest(vfs: &dyn Vfs) -> Result<Option<u32>, String> {
    let file = match vfs.open("MANIFEST", 0) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {}", vfs.describe("MANIFEST"), e)),
    };
    let name = vfs.describe("MANIFEST");
    if file.len() != 20 {
        return Err(format!("{}: damaged: it is {} bytes long, not 20", name, file.len()));
    }
    // SAFETY: twenty bytes of a file that is that long.
    let m = unsafe { std::slice::from_raw_parts(file.base(), 20) };
    if &m[..8] != MANIFEST_MAGIC || crc(&m[..16]) != u32_at(m, 16) {
        return Err(format!("{}: damaged: not a qdb manifest, or its checksum does not match", name));
    }
    if u32_at(m, 8) != FORMAT_VERSION {
        return Err(format!("{}: written by a newer version of qdb (format {})", name, u32_at(m, 8)));
    }
    Ok(Some(u32_at(m, 12)))
}

/// The blocks of a checkpoint file, each checked, joined into one buffer
/// of entries. Returns the entries and the size of the file.
pub fn read_checkpoint(vfs: &dyn Vfs, number: u32) -> Result<(Vec<u8>, u64), String> {
    let name = ckp_name(number);
    let file = vfs.open(&name, 0).map_err(|e| format!("{}: {}", vfs.describe(&name), e))?;
    let name = vfs.describe(&name);
    let len = file.len() as usize;
    // SAFETY: the whole file, which nothing changes.
    let all = unsafe { std::slice::from_raw_parts(file.base(), len) };
    if len < 20 || &all[..8] != TREE_MAGIC || crc(&all[..16]) != u32_at(all, 16) {
        return Err(format!("{}: damaged: not a qdb checkpoint, or its header does not match its checksum", name));
    }
    if u32_at(all, 8) != FORMAT_VERSION {
        return Err(format!("{}: written by a newer version of qdb (format {})", name, u32_at(all, 8)));
    }
    let mut out = Vec::with_capacity(len);
    let mut at = 20usize;
    let mut blocks = 0u64;
    loop {
        if at + 8 > len {
            return Err(format!("{}: damaged: it ends in the middle of a block", name));
        }
        let n = u32_at(all, at) as usize;
        if n == 0 {
            let sound = at + 24 == len && &all[at + 4..at + 12] == TREE_END && u64_at(all, at + 12) == blocks && crc(&all[at..at + 20]) == u32_at(all, at + 20);
            if !sound {
                return Err(format!("{}: damaged: its end mark is wrong", name));
            }
            return Ok((out, len as u64));
        }
        if at + 8 + n > len || crc(&all[at + 8..at + 8 + n]) != u32_at(all, at + 4) {
            return Err(format!("{}: damaged: block {} does not match its checksum", name, blocks));
        }
        out.extend_from_slice(&all[at + 8..at + 8 + n]);
        at += 8 + n;
        blocks += 1;
    }
}
