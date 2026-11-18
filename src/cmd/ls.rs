//! ls. Output is always the piped form: one entry per line, no columns
//! and no colours. What the data model has no fact for is fixed: the mode
//! is `-rw-r--r--` or `drwxr-xr-x`, the owner and group are `qdb`, a
//! folder is 4,096 bytes, and a file takes its content rounded up to
//! 4 KB blocks.

use super::options;
use crate::engine::{Io, Runner};
use crate::tree::{Db, NodeId};
use crate::util::{join, quote};
use crate::when;

#[derive(Clone, Copy, PartialEq)]
enum Hidden {
    /// Names starting with a dot are left out.
    No,
    /// `-A`: all but `.` and `..`.
    Almost,
    /// `-a`
    All,
}

#[derive(Clone, Copy, PartialEq)]
enum Order {
    Name,
    /// `-S`: largest first.
    Size,
    /// `-t`: newest first.
    Time,
}

struct Flags {
    hidden: Hidden,
    recursive: bool,
    slash: bool,
    /// `-d`: folders are listed as themselves.
    dirs: bool,
    reverse: bool,
    order: Order,
    long: bool,
    human: bool,
}

/// The size a folder is given, as on the reference filesystem.
pub const FOLDER_BYTES: u64 = 4096;

/// Bytes of a node as ls and du report them.
pub fn bytes_of(db: &Db, id: NodeId) -> u64 {
    if db.is_dir(id) {
        FOLDER_BYTES
    } else {
        db.size(id)
    }
}

/// Disk blocks of a node, in units of 1 KB: content rounded up to 4 KB.
pub fn blocks_of(db: &Db, id: NodeId) -> u64 {
    bytes_of(db, id).div_ceil(4096) * 4
}

/// The link count: 1 for a file, 2 plus the number of subfolders for a
/// folder.
pub fn links_of(db: &Db, id: NodeId) -> u64 {
    if db.is_dir(id) {
        2 + db.children(id).values().filter(|c| db.is_dir(**c)).count() as u64
    } else {
        1
    }
}

pub fn ls(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("ls", &args, b"aAR1pdrStlh", b"", 2)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let mut flags = Flags {
        hidden: Hidden::No,
        recursive: p.has(b'R'),
        slash: p.has(b'p'),
        dirs: p.has(b'd'),
        reverse: p.has(b'r'),
        order: Order::Name,
        long: false,
        human: p.has(b'h'),
    };
    // Where two flags disagree the later one wins.
    for (f, _) in &p.flags {
        match f {
            b'a' => flags.hidden = Hidden::All,
            b'A' => flags.hidden = Hidden::Almost,
            b'S' => flags.order = Order::Size,
            b't' => flags.order = Order::Time,
            b'l' => flags.long = true,
            _ => {}
        }
    }
    let operands = if p.operands.is_empty() { vec![b".".to_vec()] } else { p.operands };
    Ok(Box::new(move |db, io| {
        let mut status = 0;
        let mut files: Vec<Entry> = Vec::new();
        let mut dirs: Vec<Entry> = Vec::new();
        for name in &operands {
            match db.resolve(name) {
                Err(e) => {
                    io.err_line(format!("ls: cannot access {}: {}", quote(name), e.msg()));
                    status = 2;
                }
                Ok(id) if db.is_dir(id) && !flags.dirs => dirs.push(Entry { name: name.clone(), id }),
                Ok(id) => files.push(Entry { name: name.clone(), id }),
            }
        }
        sort(db, &flags, &mut files);
        sort(db, &flags, &mut dirs);
        // Column widths take in the folders named beside the files.
        let all: Vec<NodeId> = files.iter().chain(&dirs).map(|e| e.id).collect();
        print(db, &flags, &files, &all, io);
        let headers = operands.len() > 1 || flags.recursive;
        let mut first = files.is_empty();
        for d in dirs {
            list_dir(db, &flags, &d.name, d.id, headers, &mut first, io);
        }
        status
    }))
}

struct Entry {
    name: Vec<u8>,
    id: NodeId,
}

fn sort(db: &Db, flags: &Flags, entries: &mut [Entry]) {
    match flags.order {
        Order::Name => entries.sort_by(|a, b| a.name.cmp(&b.name)),
        Order::Size => entries.sort_by(|a, b| bytes_of(db, b.id).cmp(&bytes_of(db, a.id)).then_with(|| a.name.cmp(&b.name))),
        Order::Time => entries.sort_by(|a, b| db.mtime(b.id).cmp(&db.mtime(a.id)).then_with(|| a.name.cmp(&b.name))),
    }
    if flags.reverse {
        entries.reverse();
    }
}

fn print(db: &Db, flags: &Flags, entries: &[Entry], widths_of: &[NodeId], io: &mut Io) {
    if !flags.long {
        for e in entries {
            io.put(&e.name);
            if flags.slash && db.is_dir(e.id) {
                io.put(b"/");
            }
            io.put(b"\n");
        }
        return;
    }
    let size = |id: NodeId| {
        let n = bytes_of(db, id);
        if flags.human {
            when::human(n)
        } else {
            n.to_string()
        }
    };
    let links_width = widths_of.iter().map(|id| links_of(db, *id).to_string().len()).max().unwrap_or(0);
    let size_width = widths_of.iter().map(|id| size(*id).len()).max().unwrap_or(0);
    let now = db.now();
    for e in entries {
        let is_dir = db.is_dir(e.id);
        let line = format!(
            "{} {:>lw$} qdb qdb {:>sw$} {} ",
            if is_dir { "drwxr-xr-x" } else { "-rw-r--r--" },
            links_of(db, e.id),
            size(e.id),
            when::ls_date(db.mtime(e.id), now),
            lw = links_width,
            sw = size_width
        );
        io.put(line.as_bytes());
        io.put(&e.name);
        if flags.slash && is_dir {
            io.put(b"/");
        }
        io.put(b"\n");
    }
}

fn list_dir(db: &Db, flags: &Flags, path: &[u8], id: NodeId, header: bool, first: &mut bool, io: &mut Io) {
    if io.stopped() {
        return;
    }
    if !*first {
        io.put(b"\n");
    }
    *first = false;
    if header {
        io.put(path);
        io.put(b":\n");
    }
    let mut entries: Vec<Entry> = db
        .children(id)
        .iter()
        .filter(|(name, _)| flags.hidden != Hidden::No || name[0] != b'.')
        .map(|(name, child)| Entry { name: name.clone(), id: *child })
        .collect();
    if flags.hidden == Hidden::All {
        entries.push(Entry { name: b".".to_vec(), id });
        entries.push(Entry { name: b"..".to_vec(), id: db.parent(id).unwrap_or(id) });
    }
    sort(db, flags, &mut entries);
    if flags.long {
        let blocks: u64 = entries.iter().map(|e| blocks_of(db, e.id)).sum();
        let total = if flags.human { when::human(blocks * 1024) } else { blocks.to_string() };
        io.put(format!("total {}\n", total).as_bytes());
    }
    let ids: Vec<NodeId> = entries.iter().map(|e| e.id).collect();
    print(db, flags, &entries, &ids, io);
    if flags.recursive {
        for e in &entries {
            if db.is_dir(e.id) && e.name != b"." && e.name != b".." {
                list_dir(db, flags, &join(path, &e.name), e.id, true, first, io);
            }
        }
    }
}
