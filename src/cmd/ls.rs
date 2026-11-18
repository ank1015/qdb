//! ls. Output is always the piped form: one name per line, sorted in byte
//! order, no columns and no colours.

use super::options;
use crate::engine::{Io, Runner};
use crate::tree::{Db, NodeId};
use crate::util::{join, quote};

struct Flags {
    all: bool,
    recursive: bool,
    slash: bool,
}

pub fn ls(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("ls", &args, b"aR1p", b"", 2)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let flags = Flags { all: p.has(b'a'), recursive: p.has(b'R'), slash: p.has(b'p') };
    let operands = if p.operands.is_empty() { vec![b".".to_vec()] } else { p.operands };
    Ok(Box::new(move |db, io| {
        let mut status = 0;
        let mut files: Vec<&[u8]> = Vec::new();
        let mut dirs: Vec<(&[u8], NodeId)> = Vec::new();
        for name in &operands {
            match db.resolve(name) {
                Err(e) => {
                    io.err_line(format!("ls: cannot access {}: {}", quote(name), e.msg()));
                    status = 2;
                }
                Ok(id) if db.is_dir(id) => dirs.push((name, id)),
                Ok(_) => files.push(name),
            }
        }
        files.sort();
        dirs.sort();
        for f in &files {
            io.put(f);
            io.put(b"\n");
        }
        let headers = operands.len() > 1 || flags.recursive;
        let mut first = files.is_empty();
        for (name, id) in dirs {
            list_dir(db, &flags, name, id, headers, &mut first, io);
        }
        status
    }))
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
    let mut entries: Vec<(&[u8], bool)> = db
        .children(id)
        .iter()
        .filter(|(name, _)| flags.all || name[0] != b'.')
        .map(|(name, child)| (name.as_slice(), db.is_dir(*child)))
        .collect();
    if flags.all {
        entries.push((b".", true));
        entries.push((b"..", true));
        entries.sort();
    }
    for (name, is_dir) in &entries {
        io.put(name);
        if flags.slash && *is_dir {
            io.put(b"/");
        }
        io.put(b"\n");
    }
    if flags.recursive {
        for (name, child) in db.children(id) {
            if db.is_dir(*child) && (flags.all || name[0] != b'.') {
                list_dir(db, flags, &join(path, name), *child, true, first, io);
            }
        }
    }
}
