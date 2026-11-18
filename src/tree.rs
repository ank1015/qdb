//! The data model: one strict tree of folders and files.
//!
//! ```text
//! Database = one tree with a root folder
//! Node     = Folder | File
//! Folder   = { id, name, parent, children: name -> Node }
//! File     = { id, name, parent, content: bytes of UTF-8 text }
//! ```
//!
//! Every node has an internal id that never changes; name and parent are
//! properties, so a move keeps the node. No metadata is stored: size and line
//! count are derived from content.

use crate::index::Index;
pub use crate::index::{IndexStats, IndexWork};
use crate::query::Query;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

pub type NodeId = u64;

#[derive(Debug, Clone)]
pub enum NodeKind {
    /// Children are keyed by name. A BTreeMap keeps them in byte order, which
    /// is the "sorted by name" order of the specification (LC_ALL=C).
    Folder(BTreeMap<Vec<u8>, NodeId>),
    File(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub name: Vec<u8>,
    pub parent: Option<NodeId>,
    pub kind: NodeKind,
    /// Bookkeeping, not part of the data model. For a folder, how many
    /// files lie anywhere beneath it. For a file, the number its content
    /// has in the index (0 for none).
    aux: u32,
}

/// Node ids are handed out in sequence, so one multiplication spreads
/// them well enough for a hash table.
#[derive(Default, Clone, Copy)]
pub struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ *b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

type NodeMap = HashMap<NodeId, Node, BuildHasherDefault<IdHasher>>;

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
    /// turn out not to match; it never lacks one that does.
    Files(Vec<Candidate>),
    /// A good share of them: walk the folder and ask `Db::passes` of each
    /// file. A file that does not pass cannot match.
    Among(Shortlist),
}

/// A set of files that may contain a match. Only `Db::passes` reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortlist {
    /// Document numbers of the index, ascending.
    docs: Vec<u32>,
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

#[derive(Debug, Clone)]
pub struct Db {
    nodes: NodeMap,
    root: NodeId,
    next_id: NodeId,
    /// Derived from file content and kept current by every change to it.
    /// None when indexing is switched off.
    index: Option<Index>,
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

/// Content must be valid UTF-8 with no NUL bytes.
pub fn is_valid_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

/// Names follow POSIX: anything except `/` and NUL, and not empty.
pub fn is_valid_name(name: &[u8]) -> bool {
    !name.is_empty() && name != b"." && name != b".." && !name.contains(&b'/') && !name.contains(&0)
}

impl Db {
    /// An empty database, with the search index on.
    pub fn new() -> Db {
        let mut nodes = NodeMap::default();
        nodes.insert(
            0,
            Node { id: 0, name: Vec::new(), parent: None, kind: NodeKind::Folder(BTreeMap::new()), aux: 0 },
        );
        Db { nodes, root: 0, next_id: 1, index: Some(Index::new()) }
    }

    /// An empty database that answers every search by scanning.
    pub fn without_index() -> Db {
        let mut db = Db::new();
        db.index = None;
        db
    }

    // ------------------------------------------------------ the index

    pub fn has_index(&self) -> bool {
        self.index.is_some()
    }

    /// Switch the search index on or off. Switching it on builds it from
    /// the files; switching it off throws it away. Results never depend
    /// on it, only speed.
    pub fn set_index(&mut self, on: bool) {
        if on {
            self.rebuild_index();
        } else if self.index.take().is_some() {
            for node in self.nodes.values_mut() {
                if matches!(node.kind, NodeKind::File(_)) {
                    node.aux = 0;
                }
            }
        }
    }

    /// Build the index again from the files, in the order a walk visits
    /// them so that neighbouring files get neighbouring numbers.
    pub fn rebuild_index(&mut self) {
        let mut order = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            match &self.nodes[&id].kind {
                NodeKind::File(_) => order.push(id),
                NodeKind::Folder(children) => stack.extend(children.values().rev().copied()),
            }
        }
        let mut index = Index::new();
        index.begin_bulk();
        for id in order {
            let node = self.nodes.get_mut(&id).unwrap();
            if let NodeKind::File(content) = &node.kind {
                node.aux = index.add(id, content);
            }
        }
        index.end_bulk();
        // The totals of work carry on; building the index for the first
        // time is not a rebuild.
        if let Some(old) = &self.index {
            let mut work = old.work();
            work.rebuilds += 1;
            index.set_work(work);
        }
        self.index = Some(index);
    }

    /// Facts about the index, or None when it is off.
    pub fn index_stats(&self) -> Option<IndexStats> {
        self.index.as_ref().map(|i| i.stats())
    }

    /// Running totals of index work, or None when it is off.
    pub fn index_work(&self) -> Option<IndexWork> {
        self.index.as_ref().map(|i| i.work())
    }

    /// What asking `candidates` would cost, as the number of bytes of file
    /// content that could be searched in the same time; None when the
    /// answer would be "any of them" and costs nothing. A command that may
    /// be stopped early can read that much before it asks.
    pub fn candidates_cost(&self, query: &Query) -> Option<usize> {
        // Reading a byte of a list takes about as long as searching
        // sixteen bytes of text.
        Some(self.index.as_ref()?.cost(query)?.saturating_mul(16))
    }

    /// Which files under `folder` can match `query`? This is the one
    /// question commands ask of the index; how it is answered stays in
    /// here.
    pub fn candidates(&self, folder: NodeId, query: &Query) -> Candidates {
        let index = match &self.index {
            Some(i) => i,
            None => return Candidates::All,
        };
        let docs = match index.docs(query) {
            Some(d) => d,
            None => return Candidates::All,
        };
        let below = self.nodes[&folder].aux as usize;
        if docs.len() * 8 >= below {
            // Many candidates for the size of this folder: a list of them
            // would cost a walk of the folder to put in order, so leave
            // the walk to the caller.
            return Candidates::Among(Shortlist { docs });
        }
        // Few candidates: place each one by following its parents up,
        // without looking at the rest of the folder.
        let mut placed: Vec<(Vec<&[u8]>, NodeId)> = Vec::new();
        for id in docs.into_iter().filter_map(|d| index.file_of(d)) {
            let mut names = Vec::new();
            let mut cur = id;
            let inside = loop {
                if cur == folder {
                    break true;
                }
                let node = &self.nodes[&cur];
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
        Candidates::Files(placed.into_iter().map(|(names, id)| Candidate { id, path: names.join(&b'/') }).collect())
    }

    /// For checks: every file the index holds as able to satisfy the
    /// query, ascending, wherever it is in the tree. None when the index
    /// is off or the query says nothing.
    pub fn index_answer(&self, query: &Query) -> Option<Vec<NodeId>> {
        self.index.as_ref()?.files(query)
    }

    /// Whether a file is in the set a `Candidates::Among` stands for.
    pub fn passes(&self, filter: &Shortlist, file: NodeId) -> bool {
        // A file's `aux` is its document number, 0 if it has none.
        let doc = self.nodes[&file].aux;
        doc != 0 && filter.docs.binary_search(&doc).is_ok()
    }

    /// The path of a file from a folder above it, or None when the file
    /// is not under that folder.
    pub fn path_below(&self, folder: NodeId, id: NodeId) -> Option<Vec<u8>> {
        let mut names = Vec::new();
        let mut cur = id;
        while cur != folder {
            let node = self.nodes.get(&cur)?;
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
        self.nodes[&folder].aux as usize
    }

    /// Add to the file count of a folder and of every folder above it.
    fn count_files(&mut self, mut folder: Option<NodeId>, delta: i64) {
        while let Some(id) = folder {
            let node = self.nodes.get_mut(&id).unwrap();
            node.aux = (node.aux as i64 + delta) as u32;
            folder = node.parent;
        }
    }

    /// After a change: start the index afresh if it has used up too many
    /// document numbers.
    fn index_upkeep(&mut self) {
        if self.index.as_ref().is_some_and(|i| i.wants_rebuild()) {
            self.rebuild_index();
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[&id]
    }

    pub fn exists(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn is_dir(&self, id: NodeId) -> bool {
        matches!(self.nodes[&id].kind, NodeKind::Folder(_))
    }

    pub fn name(&self, id: NodeId) -> &[u8] {
        &self.nodes[&id].name
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[&id].parent
    }

    /// Children of a folder in byte order. Panics on a file.
    pub fn children(&self, id: NodeId) -> &BTreeMap<Vec<u8>, NodeId> {
        match &self.nodes[&id].kind {
            NodeKind::Folder(c) => c,
            NodeKind::File(_) => panic!("children() on a file"),
        }
    }

    pub fn child(&self, id: NodeId, name: &[u8]) -> Option<NodeId> {
        match &self.nodes[&id].kind {
            NodeKind::Folder(c) => c.get(name).copied(),
            NodeKind::File(_) => None,
        }
    }

    /// Content of a file. Panics on a folder.
    pub fn content(&self, id: NodeId) -> &[u8] {
        match &self.nodes[&id].kind {
            NodeKind::File(c) => c,
            NodeKind::Folder(_) => panic!("content() on a folder"),
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total bytes of file content.
    pub fn content_bytes(&self) -> usize {
        self.nodes
            .values()
            .map(|n| match &n.kind {
                NodeKind::File(c) => c.len(),
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
            cur = self.nodes[&c].parent;
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
        let mut cur = self.root;
        for comp in path.split(|b| *b == b'/') {
            let children = match &self.nodes[&cur].kind {
                NodeKind::Folder(c) => c,
                NodeKind::File(_) => return Err(Errno::NotDir),
            };
            if comp.is_empty() || comp == b"." {
                continue;
            }
            if comp == b".." {
                cur = self.nodes[&cur].parent.unwrap_or(self.root);
                continue;
            }
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

    fn alloc(&mut self, name: &[u8], parent: NodeId, kind: NodeKind) -> NodeId {
        assert!(is_valid_name(name), "invalid node name");
        let id = self.next_id;
        self.next_id += 1;
        let is_file = matches!(kind, NodeKind::File(_));
        let aux = match (&kind, &mut self.index) {
            (NodeKind::File(content), Some(index)) => index.add(id, content),
            _ => 0,
        };
        self.nodes.insert(id, Node { id, name: name.to_vec(), parent: Some(parent), kind, aux });
        match &mut self.nodes.get_mut(&parent).unwrap().kind {
            NodeKind::Folder(c) => {
                let old = c.insert(name.to_vec(), id);
                assert!(old.is_none(), "duplicate name in folder");
            }
            NodeKind::File(_) => panic!("parent is a file"),
        }
        if is_file {
            self.count_files(Some(parent), 1);
            self.index_upkeep();
        }
        id
    }

    pub fn create_folder(&mut self, parent: NodeId, name: &[u8]) -> NodeId {
        self.alloc(name, parent, NodeKind::Folder(BTreeMap::new()))
    }

    /// The caller must have checked the content with `is_valid_text`.
    pub fn create_file(&mut self, parent: NodeId, name: &[u8], content: Vec<u8>) -> NodeId {
        debug_assert!(is_valid_text(&content));
        self.alloc(name, parent, NodeKind::File(content))
    }

    /// The caller must have checked the content with `is_valid_text`.
    pub fn set_content(&mut self, id: NodeId, content: Vec<u8>) {
        debug_assert!(is_valid_text(&content));
        let node = self.nodes.get_mut(&id).unwrap();
        match &mut node.kind {
            NodeKind::File(c) => {
                if let Some(index) = &mut self.index {
                    index.retire(node.aux);
                    node.aux = index.add(id, &content);
                }
                *c = content;
            }
            NodeKind::Folder(_) => panic!("set_content() on a folder"),
        }
        self.index_upkeep();
    }

    /// Remove a node and everything under it. The root cannot be removed.
    pub fn remove(&mut self, id: NodeId) {
        assert!(id != self.root, "cannot remove the root");
        let (parent, name) = {
            let n = &self.nodes[&id];
            (n.parent.unwrap(), n.name.clone())
        };
        if let NodeKind::Folder(c) = &mut self.nodes.get_mut(&parent).unwrap().kind {
            c.remove(&name);
        }
        let gone = match &self.nodes[&id].kind {
            NodeKind::File(_) => 1,
            NodeKind::Folder(_) => self.nodes[&id].aux as i64,
        };
        self.count_files(Some(parent), -gone);
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            if let Some(node) = self.nodes.remove(&n) {
                match node.kind {
                    NodeKind::Folder(c) => stack.extend(c.values().copied()),
                    NodeKind::File(_) => {
                        if let Some(index) = &mut self.index {
                            index.retire(node.aux);
                        }
                    }
                }
            }
        }
        self.index_upkeep();
    }

    /// Move a node to a new parent and name, keeping its id. The destination
    /// name must be free, and a folder cannot move into its own descendant.
    pub fn rename(&mut self, id: NodeId, new_parent: NodeId, new_name: &[u8]) -> Result<(), Errno> {
        assert!(id != self.root);
        assert!(is_valid_name(new_name));
        if self.is_same_or_descendant(new_parent, id) {
            return Err(Errno::Inval);
        }
        if self.child(new_parent, new_name).is_some() {
            return Err(Errno::Exist);
        }
        let (old_parent, old_name) = {
            let n = &self.nodes[&id];
            (n.parent.unwrap(), n.name.clone())
        };
        if let NodeKind::Folder(c) = &mut self.nodes.get_mut(&old_parent).unwrap().kind {
            c.remove(&old_name);
        }
        if let NodeKind::Folder(c) = &mut self.nodes.get_mut(&new_parent).unwrap().kind {
            c.insert(new_name.to_vec(), id);
        }
        let n = self.nodes.get_mut(&id).unwrap();
        n.parent = Some(new_parent);
        n.name = new_name.to_vec();
        // The index is keyed by node id, so a move costs it nothing; only
        // the file counts of the folders on both sides change.
        let moved = match n.kind {
            NodeKind::File(_) => 1,
            NodeKind::Folder(_) => n.aux as i64,
        };
        self.count_files(Some(old_parent), -moved);
        self.count_files(Some(new_parent), moved);
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
            match &self.nodes[child].kind {
                NodeKind::File(c) => out.push((prefix.clone(), Some(c.clone()))),
                NodeKind::Folder(_) => {
                    out.push((prefix.clone(), None));
                    self.snapshot_into(*child, prefix, out);
                }
            }
            prefix.truncate(len);
        }
    }
}
