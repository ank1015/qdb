//! ls. Output is always the piped form: one name per line, sorted in byte
//! order, no columns and no colours.

use super::options;
use crate::engine::Runner;
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
        let mut out = Vec::new();
        for f in &files {
            out.extend_from_slice(f);
            out.push(b'\n');
        }
        let headers = operands.len() > 1 || flags.recursive;
        let mut first = files.is_empty();
        for (name, id) in dirs {
            list_dir(db, &flags, name, id, headers, &mut first, &mut out);
        }
        io.out = out;
        status
    }))
}

fn list_dir(db: &Db, flags: &Flags, path: &[u8], id: NodeId, header: bool, first: &mut bool, out: &mut Vec<u8>) {
    if !*first {
        out.push(b'\n');
    }
    *first = false;
    if header {
        out.extend_from_slice(path);
        out.extend_from_slice(b":\n");
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
        out.extend_from_slice(name);
        if flags.slash && *is_dir {
            out.push(b'/');
        }
        out.push(b'\n');
    }
    if flags.recursive {
        for (name, child) in db.children(id) {
            if db.is_dir(*child) && (flags.all || name[0] != b'.') {
                list_dir(db, flags, &join(path, name), *child, true, first, out);
            }
        }
    }
}
