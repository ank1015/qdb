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

use std::collections::{BTreeMap, HashMap};

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
    nodes: HashMap<NodeId, Node>,
    root: NodeId,
    next_id: NodeId,
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
    pub fn new() -> Db {
        let mut nodes = HashMap::new();
        nodes.insert(
            0,
            Node { id: 0, name: Vec::new(), parent: None, kind: NodeKind::Folder(BTreeMap::new()) },
        );
        Db { nodes, root: 0, next_id: 1 }
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
        self.nodes.insert(id, Node { id, name: name.to_vec(), parent: Some(parent), kind });
        match &mut self.nodes.get_mut(&parent).unwrap().kind {
            NodeKind::Folder(c) => {
                let old = c.insert(name.to_vec(), id);
                assert!(old.is_none(), "duplicate name in folder");
            }
            NodeKind::File(_) => panic!("parent is a file"),
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
        match &mut self.nodes.get_mut(&id).unwrap().kind {
            NodeKind::File(c) => *c = content,
            NodeKind::Folder(_) => panic!("set_content() on a folder"),
        }
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
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            if let Some(node) = self.nodes.remove(&n) {
                if let NodeKind::Folder(c) = node.kind {
                    stack.extend(c.values().copied());
                }
            }
        }
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
