//! Getting data in and out of the in-memory database: real directories on
//! disk, and the small text format the tests use for fixtures.
//!
//! This is not on-disk storage for the engine (that comes in a later
//! stage); it only imports and exports whole trees.

use crate::tree::{is_valid_text, Db, NodeId};
use std::fs;
use std::io;
use std::path::Path;

#[cfg(unix)]
fn name_bytes(name: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    name.as_bytes().to_vec()
}

#[cfg(unix)]
fn name_os(name: &[u8]) -> std::ffi::OsString {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(name).to_os_string()
}

fn invalid(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// Import a directory tree. Anything that is not a folder or a UTF-8 text
/// file (symlinks, binary files, devices) is an error.
pub fn load_dir(path: &Path) -> io::Result<Db> {
    let mut db = Db::new();
    let root = db.root();
    load_into(&mut db, root, path)?;
    Ok(db)
}

fn load_into(db: &mut Db, parent: NodeId, path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = name_bytes(&entry.file_name());
        let kind = entry.file_type()?;
        let child = entry.path();
        if kind.is_dir() {
            let id = db.create_folder(parent, &name);
            load_into(db, id, &child)?;
        } else if kind.is_file() {
            let content = fs::read(&child)?;
            if !is_valid_text(&content) {
                return Err(invalid(format!("{}: not UTF-8 text", child.display())));
            }
            db.create_file(parent, &name, content);
        } else {
            return Err(invalid(format!("{}: not a regular file or folder", child.display())));
        }
    }
    Ok(())
}

/// Export the whole tree into an existing, empty directory.
pub fn dump_dir(db: &Db, path: &Path) -> io::Result<()> {
    for (rel, content) in db.snapshot() {
        let mut target = path.to_path_buf();
        for comp in rel.split(|c| *c == b'/') {
            target.push(name_os(comp));
        }
        match content {
            None => fs::create_dir(&target)?,
            Some(c) => fs::write(&target, c)?,
        }
    }
    Ok(())
}

/// Read a real directory into the same shape `Db::snapshot` produces, so
/// the two can be compared. Unlike `load_dir` this accepts any bytes.
pub fn snapshot_dir(path: &Path) -> io::Result<Vec<(Vec<u8>, Option<Vec<u8>>)>> {
    fn walk(path: &Path, prefix: &[u8], out: &mut Vec<(Vec<u8>, Option<Vec<u8>>)>) -> io::Result<()> {
        let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| name_bytes(&e.file_name()));
        for entry in entries {
            let mut rel = prefix.to_vec();
            if !rel.is_empty() {
                rel.push(b'/');
            }
            rel.extend_from_slice(&name_bytes(&entry.file_name()));
            if entry.file_type()?.is_dir() {
                out.push((rel.clone(), None));
                walk(&entry.path(), &rel, out)?;
            } else {
                out.push((rel, Some(fs::read(entry.path())?)));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(path, b"", &mut out)?;
    Ok(out)
}

/// Parse a fixture: a tree written as one text file.
///
/// ```text
/// # a comment
/// D path/to/empty-folder
/// F path/to/file.txt
/// |first line
/// |second line
/// F no-final-newline.txt
/// |a line
/// :last line, with no newline after it
/// ```
///
/// `D` creates a folder and `F` a file; parent folders are created as
/// needed. Each `|` line is a line of content. A `:` line is content with
/// no newline after it, so it can only be the last line of a file.
pub fn parse_fixture(text: &[u8]) -> Result<Db, String> {
    let mut db = Db::new();
    let mut current: Option<(NodeId, Vec<u8>)> = None;
    fn flush(db: &mut Db, current: &mut Option<(NodeId, Vec<u8>)>) -> Result<(), String> {
        if let Some((id, content)) = current.take() {
            if !is_valid_text(&content) {
                return Err("fixture file content is not valid UTF-8 text".into());
            }
            db.set_content(id, content);
        }
        Ok(())
    }
    fn make(db: &mut Db, path: &[u8], file: bool) -> Result<NodeId, String> {
        let mut cur = db.root();
        let comps: Vec<&[u8]> = path.split(|c| *c == b'/').filter(|c| !c.is_empty()).collect();
        for (i, comp) in comps.iter().enumerate() {
            let last = i + 1 == comps.len();
            cur = match db.child(cur, comp) {
                Some(c) => c,
                None if last && file => db.create_file(cur, comp, Vec::new()),
                None => db.create_folder(cur, comp),
            };
        }
        Ok(cur)
    }
    for (n, raw) in text.split(|c| *c == b'\n').enumerate() {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        match line.first() {
            None | Some(b'#') => {}
            Some(b'D') if line.starts_with(b"D ") => {
                flush(&mut db, &mut current)?;
                make(&mut db, &line[2..], false)?;
            }
            Some(b'F') if line.starts_with(b"F ") => {
                flush(&mut db, &mut current)?;
                let id = make(&mut db, &line[2..], true)?;
                current = Some((id, Vec::new()));
            }
            Some(c @ (b'|' | b':')) => {
                let (_, content) = current.as_mut().ok_or(format!("fixture line {}: content before any F line", n + 1))?;
                content.extend_from_slice(&line[1..]);
                if *c == b'|' {
                    content.push(b'\n');
                }
            }
            _ => return Err(format!("fixture line {}: expected D, F, | or :", n + 1)),
        }
    }
    flush(&mut db, &mut current)?;
    Ok(db)
}
