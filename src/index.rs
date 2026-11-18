//! The trigram index in memory: for every three-byte sequence, the
//! pieces of content that contain it.
//!
//! This is derived data. It belongs to storage (`tree.rs` keeps it current
//! on every content change) and nothing else reads it; losing it loses
//! nothing a rebuild cannot restore. One of these covers the pieces most
//! recently written to a data file; when the file is closed the lists are
//! written out end to end (`disk::Packed`) and this structure is dropped.
//!
//! # Layout
//!
//! Each stored piece of content is a *document* with a number, one more
//! than its position in its data file. Numbers only go up, so a posting
//! list only ever grows at its end, and a write costs time in proportion
//! to what is written, however large the database is.
//!
//! A posting list is the ascending document numbers for one trigram,
//! stored as the differences between neighbours in a variable-length
//! code (seven bits per byte, low bits first). A difference of zero
//! cannot occur, and is used to mean "start counting from zero again":
//! that lets two lists be joined by putting a zero byte between them.
//! Lists of up to eight bytes live inside the hash table entry; longer
//! ones are on the heap.
//!
//! Retired documents stay in the lists until a sweep removes them. Stale
//! entries cost space and a little time; they never change an answer,
//! because a retired document maps to no file. The sweep runs a little at
//! a time, paid for by the writes that make it necessary.

use crate::bytes::fold;
use crate::query::{Query, Trigram};

/// Set in a document's entry when its content is no longer in any file.
const RETIRED: u32 = 1 << 31;

/// How many posting lists a document was added to, and `RETIRED`.
#[derive(Debug, Clone, Copy)]
struct Doc(u32);

impl Doc {
    fn postings(self) -> u32 {
        self.0 & !RETIRED
    }

    fn retired(self) -> bool {
        self.0 & RETIRED != 0
    }
}

/// Posting lists by trigram: what a query is answered from. The index in
/// memory is one source of them, a packed index file another.
pub trait Lists {
    /// The list for a trigram, encoded; empty if there is none.
    fn list(&self, t: Trigram) -> &[u8];
}

/// One slot of the trigram table, sixteen bytes.
#[derive(Debug, Clone, Copy, Default)]
struct Entry {
    /// The trigram in the low 24 bits. The top byte is 0 for an empty
    /// slot, 1 + length for a list stored in `data`, or `ON_HEAP`.
    key: u32,
    /// The last document in the list, so the next can be appended.
    last: u32,
    /// The list itself, or the number of its heap block (low byte first).
    data: [u8; 8],
}

const ON_HEAP: u32 = 255;
const INLINE_MAX: usize = 8;

impl Entry {
    fn tag(&self) -> u32 {
        self.key >> 24
    }

    fn trigram(&self) -> Trigram {
        self.key & 0x00ff_ffff
    }

    fn block(&self) -> usize {
        u64::from_le_bytes(self.data) as usize
    }
}

pub fn put_varint(out: &mut Vec<u8>, mut n: u32) {
    while n >= 0x80 {
        out.push(n as u8 | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}

/// Decodes a posting list into ascending document numbers.
pub struct Postings<'a> {
    bytes: &'a [u8],
    doc: u32,
}

impl<'a> Postings<'a> {
    pub fn new(bytes: &'a [u8]) -> Postings<'a> {
        Postings { bytes, doc: 0 }
    }
}

impl Iterator for Postings<'_> {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<u32> {
        loop {
            let (&first, mut rest) = self.bytes.split_first()?;
            let mut delta = (first & 0x7f) as u32;
            if first >= 0x80 {
                let mut shift = 7;
                loop {
                    let (&b, r) = rest.split_first()?;
                    rest = r;
                    delta |= ((b & 0x7f) as u32) << shift;
                    if b < 0x80 {
                        break;
                    }
                    shift += 7;
                }
            }
            self.bytes = rest;
            if delta == 0 {
                // Two lists joined: the second counts from zero again.
                self.doc = 0;
                continue;
            }
            self.doc += delta;
            return Some(self.doc);
        }
    }
}

/// A set of documents: the answer to a query before it is turned into
/// files.
enum Docs {
    All,
    Some(Vec<u32>),
}

/// Facts about the index, for reports.
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    /// Trigrams that have a posting list.
    pub trigrams: usize,
    /// Entries over all posting lists, retired documents included.
    pub postings: u64,
    /// Entries that belong to retired documents (an upper estimate).
    pub stale_postings: u64,
    pub live_documents: usize,
    pub documents_ever: usize,
    /// How many separate indexes these figures are summed over: one per
    /// data file, and one in memory for what was written last.
    pub parts: usize,
    /// Memory held: the table, the lists on the heap, the document table.
    pub table_bytes: usize,
    pub list_bytes: usize,
    pub document_bytes: usize,
    /// Bytes of posting data in use, inside the table or on the heap.
    pub posting_bytes: u64,
    /// How many lists have 1, 2, 3-4, 5-8, ... documents, by powers of two.
    pub lists_by_length: Vec<usize>,
    /// Posting bytes held by the 100, 1,000 and 10,000 longest lists.
    pub top_list_bytes: [u64; 3],
}

impl IndexStats {
    pub fn total_bytes(&self) -> usize {
        self.table_bytes + self.list_bytes + self.document_bytes
    }
}

/// Running totals of the work the index has done, for reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexWork {
    pub documents_added: u64,
    pub documents_retired: u64,
    pub postings_added: u64,
    /// Bytes of posting lists rewritten by sweeps.
    pub bytes_swept: u64,
    /// Always 0 since stage 3: the index is no longer built again from
    /// all files.
    pub rebuilds: u64,
}

impl IndexWork {
    pub fn add(&mut self, o: &IndexWork) {
        self.documents_added += o.documents_added;
        self.documents_retired += o.documents_retired;
        self.postings_added += o.postings_added;
        self.bytes_swept += o.bytes_swept;
    }
}

#[derive(Debug, Clone)]
pub struct Index {
    slots: Vec<Entry>,
    used: usize,
    heap: Vec<Vec<u8>>,
    free_heap: Vec<u32>,
    /// The documents here are numbered from `base + 1`.
    base: u32,
    /// Indexed by document number less `base`; entry 0 is never used.
    docs: Vec<Doc>,
    live_docs: usize,
    live_postings: u64,
    stale_postings: u64,
    /// A sweep in progress: the next slot, and the stale count when it
    /// started.
    sweep: Option<(usize, u64)>,
    work: IndexWork,
    // Scratch for taking the distinct trigrams of one file.
    seen_bits: Bitmap,
    seen_small: Vec<u32>,
    found: Vec<Trigram>,
    scratch: Vec<u8>,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

/// Below this, distinct trigrams are collected in a small hash set
/// instead of a bitmap of all 2^24.
const SMALL_CONTENT: usize = 4 * 1024;

/// Scratch for `collect`: a bit per possible trigram, two megabytes, made
/// at the first write of more than a little content and then kept. A copy
/// of the index starts without it.
#[derive(Debug, Default)]
struct Bitmap(Option<Box<[u64]>>);

impl Clone for Bitmap {
    fn clone(&self) -> Bitmap {
        Bitmap(None)
    }
}

impl Index {
    pub fn new() -> Index {
        Index::starting_after(0)
    }

    /// An empty index whose first document will have number `base + 1`.
    pub fn starting_after(base: u32) -> Index {
        Index {
            base,
            slots: vec![Entry::default(); 16],
            used: 0,
            heap: Vec::new(),
            free_heap: Vec::new(),
            docs: vec![Doc(RETIRED)],
            live_docs: 0,
            live_postings: 0,
            stale_postings: 0,
            sweep: None,
            work: IndexWork::default(),
            seen_bits: Bitmap(None),
            seen_small: Vec::new(),
            found: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// Called before and after many files are added in a row, as when
    /// loading: the scratch space is given back afterwards.
    pub fn begin_bulk(&mut self) {}

    pub fn end_bulk(&mut self) {
        self.found = Vec::new();
        self.seen_small = Vec::new();
        self.scratch = Vec::new();
    }

    pub fn work(&self) -> IndexWork {
        self.work
    }

    /// Carry the totals of work over to a freshly built index. Building
    /// it is not counted as work done for a write.
    pub fn set_work(&mut self, work: IndexWork) {
        self.work = work;
    }

    /// The number the next document will get.
    pub fn next_doc(&self) -> u32 {
        self.base + self.docs.len() as u32
    }

    pub fn live_docs(&self) -> usize {
        self.live_docs
    }

    /// Every trigram that has a list, ascending.
    pub fn trigrams(&self) -> Vec<Trigram> {
        let mut all: Vec<Trigram> = self.slots.iter().filter(|e| e.tag() > 1).map(|e| e.trigram()).collect();
        all.sort_unstable();
        all
    }

    // ------------------------------------------------------- the table

    fn slot_of(&self, t: Trigram) -> usize {
        let mask = self.slots.len() - 1;
        let mut i = (t.wrapping_mul(0x9E37_79B1) >> 8) as usize & mask;
        loop {
            let e = &self.slots[i];
            if e.tag() == 0 || e.trigram() == t {
                return i;
            }
            i = (i + 1) & mask;
        }
    }

    fn grow(&mut self) {
        let old = std::mem::replace(&mut self.slots, vec![Entry::default(); 0]);
        self.slots = vec![Entry::default(); old.len() * 2];
        for e in old {
            if e.tag() != 0 {
                let i = self.slot_of(e.trigram());
                self.slots[i] = e;
            }
        }
        // Slots have moved: a sweep in progress starts over.
        if let Some((slot, _)) = &mut self.sweep {
            *slot = 0;
        }
    }

    fn list_bytes<'a>(&'a self, e: &'a Entry) -> &'a [u8] {
        match e.tag() {
            0 => &[],
            ON_HEAP => &self.heap[e.block()],
            tag => &e.data[..tag as usize - 1],
        }
    }


    fn append(&mut self, t: Trigram, doc: u32) {
        if (self.used + 1) * 4 > self.slots.len() * 3 {
            self.grow();
        }
        let i = self.slot_of(t);
        let mut e = self.slots[i];
        if e.tag() == 0 {
            e = Entry { key: t | 1 << 24, last: 0, data: [0; 8] };
            self.used += 1;
        }
        debug_assert!(doc > e.last);
        let mut code = [0u8; 5];
        let mut n = 0;
        let mut delta = doc - e.last;
        while delta >= 0x80 {
            code[n] = delta as u8 | 0x80;
            delta >>= 7;
            n += 1;
        }
        code[n] = delta as u8;
        n += 1;
        e.last = doc;
        if e.tag() == ON_HEAP {
            push_tight(&mut self.heap[e.block()], &code[..n]);
        } else {
            let len = e.tag() as usize - 1;
            if len + n <= INLINE_MAX {
                e.data[len..len + n].copy_from_slice(&code[..n]);
                e.key = t | ((len + n + 1) as u32) << 24;
            } else {
                let mut list = Vec::with_capacity(len + n + 8);
                list.extend_from_slice(&e.data[..len]);
                list.extend_from_slice(&code[..n]);
                e.data = (self.heap_put(list) as u64).to_le_bytes();
                e.key = t | ON_HEAP << 24;
            }
        }
        self.slots[i] = e;
    }

    fn heap_put(&mut self, list: Vec<u8>) -> u32 {
        match self.free_heap.pop() {
            Some(i) => {
                self.heap[i as usize] = list;
                i
            }
            None => {
                self.heap.push(list);
                self.heap.len() as u32 - 1
            }
        }
    }

    // --------------------------------------------------------- writing

    /// The distinct trigrams of `content`, left in `self.found`.
    fn collect(&mut self, content: &[u8]) {
        self.found.clear();
        if content.len() < 3 {
            return;
        }
        // `clean` counts the bytes since the last newline: a trigram is
        // only taken when none of its three bytes is a newline.
        let mut clean = 0u32;
        let mut t: u32 = 0;
        if self.seen_bits.0.is_some() || content.len() >= SMALL_CONTENT {
            // One bit per possible trigram. The loop has no branch that
            // depends on the text: whether a trigram is new, and whether
            // it is whole, are worked out as numbers, because branches on
            // them would be guessed wrong too often.
            let bits = self.seen_bits.0.get_or_insert_with(|| vec![0u64; 1 << 18].into_boxed_slice());
            for block in content.chunks(1 << 16) {
                self.found.reserve(block.len());
                let mut len = self.found.len();
                let out = self.found.as_mut_ptr();
                for &b in block {
                    t = (t << 8 | fold(b) as u32) & 0x00ff_ffff;
                    clean = if b == b'\n' { 0 } else { (clean + 1).min(3) };
                    let whole = (clean >= 3) as u64;
                    let word = &mut bits[(t >> 6) as usize];
                    let new = !(*word >> (t & 63)) & whole;
                    *word |= whole << (t & 63);
                    // SAFETY: room for one entry per byte of the block
                    // was reserved above.
                    unsafe { *out.add(len) = t };
                    len += new as usize;
                }
                // SAFETY: the first `len` entries have been written.
                unsafe { self.found.set_len(len) };
            }
            for &t in &self.found {
                bits[(t >> 6) as usize] = 0;
            }
        } else {
            // A hash set that starts small and doubles as it fills, so
            // that for an ordinary file it stays in the fastest cache.
            // EMPTY is not a trigram.
            const EMPTY: u32 = u32::MAX;
            let mut size = 1024usize;
            self.seen_small.clear();
            self.seen_small.resize(size, EMPTY);
            let mut mask = size - 1;
            for &b in content {
                if b == b'\n' {
                    clean = 0;
                    continue;
                }
                t = (t << 8 | fold(b) as u32) & 0x00ff_ffff;
                clean += 1;
                if clean >= 3 {
                    let mut i = (t.wrapping_mul(0x9E37_79B1) >> 8) as usize & mask;
                    loop {
                        let s = self.seen_small[i];
                        if s == t {
                            break;
                        }
                        if s == EMPTY {
                            self.seen_small[i] = t;
                            self.found.push(t);
                            break;
                        }
                        i = (i + 1) & mask;
                    }
                    if self.found.len() * 2 > size {
                        size *= 4;
                        mask = size - 1;
                        self.seen_small.clear();
                        self.seen_small.resize(size, EMPTY);
                        for &t in &self.found {
                            let mut i = (t.wrapping_mul(0x9E37_79B1) >> 8) as usize & mask;
                            while self.seen_small[i] != EMPTY {
                                i = (i + 1) & mask;
                            }
                            self.seen_small[i] = t;
                        }
                    }
                }
            }
        }
    }

    /// Ask the processor to fetch the memory that appending to a
    /// trigram's list will touch. `stage` 0 is the table slot, 1 the
    /// list's header, 2 the end of the list. Appending is a chain of
    /// three reads of memory that is rarely in a cache; asking for each a
    /// few trigrams ahead lets them overlap instead of queueing.
    #[inline]
    fn prefetch(&self, t: Trigram, stage: u8) {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
            let mask = self.slots.len() - 1;
            let home = (t.wrapping_mul(0x9E37_79B1) >> 8) as usize & mask;
            if stage == 0 {
                // SAFETY: a prefetch reads nothing and cannot fault.
                unsafe { _mm_prefetch(self.slots.as_ptr().add(home) as *const i8, _MM_HINT_T0) };
                return;
            }
            let e = &self.slots[self.slot_of(t)];
            if e.tag() != ON_HEAP {
                return;
            }
            let list = &self.heap[e.block()];
            let at = if stage == 1 { list as *const Vec<u8> as *const i8 } else { list.as_ptr().wrapping_add(list.len()) as *const i8 };
            // SAFETY: as above.
            unsafe { _mm_prefetch(at, _MM_HINT_T0) };
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (t, stage);
        }
    }

    /// Index a new piece of content. Returns its document number, which
    /// is one more than the last.
    pub fn add(&mut self, content: &[u8]) -> u32 {
        self.collect(content);
        let doc = self.base + self.docs.len() as u32;
        let found = std::mem::take(&mut self.found);
        for (i, &t) in found.iter().enumerate() {
            if let Some(&ahead) = found.get(i + 12) {
                self.prefetch(ahead, 0);
            }
            if let Some(&ahead) = found.get(i + 8) {
                self.prefetch(ahead, 1);
            }
            if let Some(&ahead) = found.get(i + 4) {
                self.prefetch(ahead, 2);
            }
            self.append(t, doc);
        }
        self.docs.push(Doc(found.len() as u32));
        self.live_docs += 1;
        self.live_postings += found.len() as u64;
        self.work.documents_added += 1;
        self.work.postings_added += found.len() as u64;
        let budget = found.len() * 8 + 64;
        self.found = found;
        self.maintain(budget);
        doc
    }

    /// The content this document stood for is gone.
    pub fn retire(&mut self, doc: u32) {
        let d = &mut self.docs[(doc - self.base) as usize];
        debug_assert!(!d.retired());
        d.0 |= RETIRED;
        let postings = d.postings() as u64;
        self.live_docs -= 1;
        self.live_postings -= postings;
        self.stale_postings += postings;
        self.work.documents_retired += 1;
        self.maintain(postings as usize * 2 + 64);
    }

    /// Remove retired documents from the lists, a little at a time: at
    /// most about `budget` bytes of lists are rewritten per call. A sweep
    /// starts when stale entries pass a quarter of the live ones.
    fn maintain(&mut self, mut budget: usize) {
        if self.sweep.is_none() {
            if self.stale_postings <= self.live_postings / 4 + 1024 {
                return;
            }
            self.sweep = Some((0, self.stale_postings));
        }
        while let Some((slot, stale_at_start)) = self.sweep {
            if slot >= self.slots.len() {
                // Everything retired before the sweep began is now gone.
                self.stale_postings -= stale_at_start;
                self.sweep = None;
                return;
            }
            let cost = self.sweep_slot(slot);
            self.sweep = Some((slot + 1, stale_at_start));
            self.work.bytes_swept += cost as u64;
            if cost + 2 > budget {
                return;
            }
            budget -= cost + 2;
        }
    }

    /// Rewrite one list without its retired documents. Returns the size
    /// of the list before.
    fn sweep_slot(&mut self, slot: usize) -> usize {
        let e = self.slots[slot];
        if e.tag() == 0 {
            return 0;
        }
        let mut out = std::mem::take(&mut self.scratch);
        out.clear();
        let before;
        let mut last = 0u32;
        {
            let bytes = self.list_bytes(&e);
            before = bytes.len();
            for doc in (Postings { bytes, doc: 0 }) {
                if !self.docs[(doc - self.base) as usize].retired() {
                    put_varint(&mut out, doc - last);
                    last = doc;
                }
            }
        }
        if out.len() != before {
            let mut e = e;
            if e.tag() == ON_HEAP {
                let block = e.block();
                if out.len() <= INLINE_MAX {
                    self.heap[block] = Vec::new();
                    self.free_heap.push(block as u32);
                    e.key = e.trigram();
                } else {
                    let list = &mut self.heap[block];
                    list.clear();
                    list.extend_from_slice(&out);
                    if list.capacity() > out.len() * 2 {
                        list.shrink_to(out.len() + out.len() / 4);
                    }
                }
            }
            if e.tag() != ON_HEAP {
                e.data = [0; 8];
                e.data[..out.len()].copy_from_slice(&out);
                e.key = e.trigram() | (out.len() as u32 + 1) << 24;
            }
            e.last = last;
            self.slots[slot] = e;
        }
        self.scratch = out;
        before
    }

    // ---------------------------------------------------------- reports

    /// Bytes of memory held: (table, lists, documents). Reads no list.
    pub fn sizes(&self) -> (usize, usize, usize) {
        let lists = self.heap.capacity() * std::mem::size_of::<Vec<u8>>() + self.free_heap.capacity() * 4 + self.heap.iter().map(|b| b.capacity()).sum::<usize>();
        (self.slots.capacity() * std::mem::size_of::<Entry>(), lists, self.docs.capacity() * std::mem::size_of::<Doc>())
    }

    pub fn stats(&self) -> IndexStats {
        let mut s = IndexStats {
            live_documents: self.live_docs,
            documents_ever: self.docs.len() - 1,
            parts: 1,
            stale_postings: self.stale_postings,
            table_bytes: self.slots.capacity() * std::mem::size_of::<Entry>(),
            document_bytes: self.docs.capacity() * std::mem::size_of::<Doc>(),
            ..IndexStats::default()
        };
        s.list_bytes = self.heap.capacity() * std::mem::size_of::<Vec<u8>>() + self.free_heap.capacity() * 4;
        let mut sizes: Vec<u64> = Vec::new();
        for e in &self.slots {
            if e.tag() == 0 {
                continue;
            }
            let bytes = self.list_bytes(e);
            if e.tag() == ON_HEAP {
                s.list_bytes += self.heap[e.block()].capacity();
            }
            let count = Postings { bytes, doc: 0 }.count();
            if count == 0 {
                continue;
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
        }
        sizes.sort_unstable_by(|a, b| b.cmp(a));
        for (i, top) in [100usize, 1000, 10000].into_iter().enumerate() {
            s.top_list_bytes[i] = sizes.iter().take(top).sum();
        }
        s
    }
}

impl Lists for Index {
    fn list(&self, t: Trigram) -> &[u8] {
        let e = &self.slots[self.slot_of(t)];
        self.list_bytes(e)
    }
}

// ------------------------------------------------------------ answering

/// An upper bound on how many documents satisfy the query, found without
/// reading a list: a list has at least a byte per document.
pub fn estimate<L: Lists + ?Sized>(lists: &L, query: &Query) -> usize {
    match query {
        Query::All => usize::MAX,
        Query::Nothing => 0,
        Query::Has(t) => lists.list(*t).len(),
        Query::And(parts) => parts.iter().map(|p| estimate(lists, p)).min().unwrap_or(usize::MAX),
        Query::Or(parts) => parts.iter().fold(0usize, |sum, p| sum.saturating_add(estimate(lists, p))),
    }
}

/// Roughly how many bytes of lists `eval` would read for the query. This
/// follows what `eval` reads: of the lists of an AND, the shortest, and
/// then only those not too long to be worth it.
pub fn cost<L: Lists + ?Sized>(lists: &L, query: &Query) -> usize {
    match query {
        Query::All | Query::Nothing => 0,
        Query::Has(t) => lists.list(*t).len(),
        Query::Or(parts) => parts.iter().map(|p| cost(lists, p)).sum(),
        Query::And(parts) => {
            let mut lens: Vec<usize> = Vec::new();
            let mut others = 0;
            for p in parts {
                match p {
                    Query::Has(t) => lens.push(lists.list(*t).len()),
                    other => others += cost(lists, other),
                }
            }
            lens.sort_unstable();
            match lens.first() {
                None => others,
                Some(0) => 0,
                Some(&least) => others + lens.iter().take_while(|len| **len <= least * 512 + 64).sum::<usize>(),
            }
        }
    }
}

/// The documents whose lists satisfy the query, ascending, or None if the
/// query says nothing. Documents whose content is gone may be among them.
pub fn eval<L: Lists + ?Sized>(lists: &L, query: &Query) -> Option<Vec<u32>> {
    match eval_docs(lists, query) {
        Docs::All => None,
        Docs::Some(docs) => Some(docs),
    }
}

fn eval_docs<L: Lists + ?Sized>(lists: &L, query: &Query) -> Docs {
    match query {
        Query::All => Docs::All,
        Query::Nothing => Docs::Some(Vec::new()),
        Query::Has(t) => Docs::Some(Postings::new(lists.list(*t)).collect()),
        Query::Or(parts) => {
            let mut all: Vec<u32> = Vec::new();
            for p in parts {
                match eval_docs(lists, p) {
                    Docs::All => return Docs::All,
                    Docs::Some(docs) => all.extend(docs),
                }
            }
            all.sort_unstable();
            all.dedup();
            Docs::Some(all)
        }
        Query::And(parts) => {
            // Shortest lists first: the set shrinks fastest, and an
            // absent trigram ends the work at once.
            let mut found: Vec<(usize, &[u8])> = Vec::new();
            let mut others: Vec<&Query> = Vec::new();
            for p in parts {
                match p {
                    Query::Has(t) => {
                        let list = lists.list(*t);
                        if list.is_empty() {
                            return Docs::Some(Vec::new());
                        }
                        found.push((list.len(), list));
                    }
                    Query::All => {}
                    other => others.push(other),
                }
            }
            found.sort_unstable_by_key(|f| f.0);
            let mut current: Option<Vec<u32>> = None;
            for (len, list) in found {
                match &mut current {
                    None => current = Some(Postings::new(list).collect()),
                    Some(cur) => {
                        if cur.is_empty() {
                            break;
                        }
                        // Reading a long list to rule out a few files
                        // costs more than scanning those files.
                        if len > cur.len() * 512 + 64 {
                            break;
                        }
                        intersect(cur, Postings::new(list));
                    }
                }
            }
            for q in others {
                if current.as_ref().is_some_and(|c| c.len() <= 8) {
                    break;
                }
                match eval_docs(lists, q) {
                    Docs::All => {}
                    Docs::Some(docs) => match &mut current {
                        None => current = Some(docs),
                        Some(cur) => intersect(cur, docs.into_iter()),
                    },
                }
            }
            match current {
                None => Docs::All,
                Some(docs) => Docs::Some(docs),
            }
        }
    }
}

/// Keep in `cur` only the documents that are also in `other`. Both are
/// ascending.
fn intersect(cur: &mut Vec<u32>, mut other: impl Iterator<Item = u32>) {
    let mut kept = 0;
    let mut next = other.next();
    for i in 0..cur.len() {
        let doc = cur[i];
        while let Some(o) = next {
            if o >= doc {
                break;
            }
            next = other.next();
        }
        match next {
            None => break,
            Some(o) if o == doc => {
                cur[kept] = doc;
                kept += 1;
            }
            Some(_) => {}
        }
    }
    cur.truncate(kept);
}

/// Append to a list on the heap without the doubling a `Vec` does by
/// default: with hundreds of thousands of lists the slack would be a
/// large share of the index.
fn push_tight(list: &mut Vec<u8>, bytes: &[u8]) {
    if list.len() + bytes.len() > list.capacity() {
        list.reserve_exact((list.len() / 4).max(8));
    }
    list.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs(index: &Index, s: &str) -> Vec<u32> {
        eval(index, &Query::string(s.as_bytes())).unwrap().into_iter().filter(|d| !index.docs[(*d - index.base) as usize].retired()).collect()
    }

    #[test]
    fn add_query_retire() {
        let mut index = Index::starting_after(7);
        let a = index.add(b"hello world\n");
        let b = index.add(b"Hello there\nworld\n");
        let c = index.add(b"hi");
        assert_eq!((a, b, c), (8, 9, 10));
        assert_eq!(docs(&index, "hello"), vec![8, 9]);
        assert_eq!(docs(&index, "WORLD"), vec![8, 9]);
        assert_eq!(docs(&index, "o w"), vec![8]);
        assert_eq!(docs(&index, "there"), vec![9]);
        assert_eq!(docs(&index, "absent"), Vec::<u32>::new());
        // No trigram spans a line break.
        assert_eq!(eval(&index, &Query::Has(crate::query::trigram(b'e', b'\n', b'w'))).unwrap(), Vec::<u32>::new());
        index.retire(a);
        assert_eq!(docs(&index, "hello"), vec![9]);
        index.retire(b);
        assert_eq!(docs(&index, "hello"), Vec::<u32>::new());
        assert!(eval(&index, &Query::All).is_none());
    }

    #[test]
    fn joined_lists_restart() {
        let mut bytes = Vec::new();
        for d in [3u32, 2, 200] {
            put_varint(&mut bytes, d);
        }
        bytes.push(0);
        for d in [300u32, 1] {
            put_varint(&mut bytes, d);
        }
        assert_eq!(Postings::new(&bytes).collect::<Vec<u32>>(), vec![3, 5, 205, 300, 301]);
    }

    #[test]
    fn long_lists_and_sweeps() {
        let mut index = Index::new();
        let mut all = Vec::new();
        for i in 0..5000u64 {
            all.push(index.add(format!("common text number {} zz{}zz\n", i, i % 7).as_bytes()));
        }
        assert_eq!(docs(&index, "common").len(), 5000);
        assert_eq!(docs(&index, "zz3zz").len(), 5000 / 7 + usize::from(5000 % 7 > 3));
        // Retire most: sweeps must run and answers stay right.
        for i in 0..5000usize {
            if i % 10 != 0 {
                index.retire(all[i]);
            }
        }
        assert!(index.work().bytes_swept > 0);
        let left = docs(&index, "common");
        assert_eq!(left.len(), 500);
        assert!(left.iter().all(|d| (d - 1) % 10 == 0));
        let stats = index.stats();
        assert_eq!(stats.live_documents, 500);
        assert!(stats.postings < 5000 * 40, "sweeps did not shrink the lists: {} entries", stats.postings);
        // Appending after a sweep still works.
        index.add(b"common text again\n");
        assert_eq!(docs(&index, "common").len(), 501);
    }
}
