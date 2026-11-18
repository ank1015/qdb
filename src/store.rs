//! Getting whole trees in and out of a database: real directories, and
//! the small text format the tests use for fixtures.
//!
//! This is import and export, not the engine's own storage: that is in
//! `tree.rs` and `disk.rs`.

use crate::tree::{TextCheck, is_valid_text, Db, NodeId};
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


/// What `load_dir` did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoadReport {
    pub files: usize,
    pub folders: usize,
    /// Bytes of file content loaded.
    pub bytes: usize,
    pub skipped_symlinks: usize,
    /// Regular files that are not valid UTF-8 text, or contain a NUL.
    pub skipped_not_text: usize,
    /// Devices, sockets and other things that are neither file nor folder.
    pub skipped_other: usize,
    /// Time spent reading the tree, in microseconds.
    pub read_micros: u64,
    /// Time spent building the search index, or None when it is off.
    pub index_micros: Option<u64>,
}

impl LoadReport {
    pub fn skipped(&self) -> usize {
        self.skipped_symlinks + self.skipped_not_text + self.skipped_other
    }
}

/// Import a directory tree. Symbolic links, files that are not valid text
/// and anything that is neither a file nor a folder are skipped and
/// counted; the data model has no place for them.
pub fn load_dir(path: &Path) -> io::Result<(Db, LoadReport)> {
    load_dir_with(path, true)
}

/// `load_dir`, with the search index built (`index` true) or left off.
/// The index is built after the files are read, and timed on its own.
pub fn load_dir_with(path: &Path, index: bool) -> io::Result<(Db, LoadReport)> {
    let mut db = Db::without_index();
    let mut report = load_into_db(&mut db, path)?;
    if index {
        let start = std::time::Instant::now();
        db.set_index(true);
        report.index_micros = Some(start.elapsed().as_micros() as u64);
    }
    Ok((db, report))
}

/// Import a directory tree into a database that is empty, with whatever
/// settings it has. The import is logged a few megabytes at a time, so a
/// crash part of the way leaves part of the tree.
pub fn load_into_db(db: &mut Db, path: &Path) -> io::Result<LoadReport> {
    let mut report = LoadReport::default();
    let root = db.root();
    let start = std::time::Instant::now();
    let mut pending = 0;
    load_into(db, root, path, &mut report, &mut pending)?;
    db.set_mtime(root, mtime_of(&fs::metadata(path)?));
    db.commit();
    if let Some(e) = db.failure() {
        return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
    }
    report.read_micros = start.elapsed().as_micros() as u64;
    Ok(report)
}

fn load_into(db: &mut Db, parent: NodeId, path: &Path, report: &mut LoadReport, pending: &mut usize) -> io::Result<()> {
    // Sorted, so that a load always builds the same database.
    let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| name_bytes(&e.file_name()));
    for entry in entries {
        let name = name_bytes(&entry.file_name());
        let kind = entry.file_type()?;
        let child = entry.path();
        if kind.is_symlink() {
            report.skipped_symlinks += 1;
        } else if kind.is_dir() {
            let id = db.create_folder(parent, &name);
            report.folders += 1;
            load_into(db, id, &child, report, pending)?;
            // After its entries, which each set its time on the way in.
            db.set_mtime(id, mtime_of(&entry.metadata()?));
        } else if kind.is_file() {
            let meta = entry.metadata()?;
            let size = meta.len() as usize;
            if size > LOAD_WHOLE {
                // A large file is read and stored a stretch at a time.
                match load_large(db, parent, &name, &child, report)? {
                    Some(id) => db.set_mtime(id, mtime_of(&meta)),
                    None => report.skipped_not_text += 1,
                }
                *pending = usize::MAX;
            } else {
                let content = fs::read(&child)?;
                if !is_valid_text(&content) {
                    report.skipped_not_text += 1;
                    continue;
                }
                report.files += 1;
                report.bytes += content.len();
                let id = db.create_file(parent, &name, &content);
                db.set_mtime(id, mtime_of(&meta));
                *pending = pending.saturating_add(content.len() + 64);
            }
            if *pending > 4 << 20 {
                *pending = 0;
                db.commit();
                if let Some(e) = db.failure() {
                    return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
                }
            }
        } else {
            report.skipped_other += 1;
        }
    }
    Ok(())
}

/// A file's modification time as the database keeps it: nanoseconds
/// since 1970, UTC. The import keeps the times the files have.
pub fn mtime_of(meta: &fs::Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    meta.mtime().saturating_mul(1_000_000_000).saturating_add(meta.mtime_nsec())
}

/// Files larger than this are loaded a stretch at a time.
const LOAD_WHOLE: usize = 8 << 20;

/// Load one large file without ever holding all of it. None if it is
/// not valid text, in which case no file is made.
fn load_large(db: &mut Db, parent: NodeId, name: &[u8], path: &Path, report: &mut LoadReport) -> io::Result<Option<NodeId>> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut buf = vec![0u8; 4 << 20];
    // Checked first, as a whole: text that is not valid is not stored.
    let mut check = TextCheck::default();
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if !check.feed(&buf[..n]) {
            return Ok(None);
        }
    }
    if !check.finish() {
        return Ok(None);
    }
    use std::io::Seek;
    file.seek(io::SeekFrom::Start(0))?;
    let id = db.create_file(parent, name, b"");
    let mut total = 0;
    let mut failed = None;
    db.set_content_stream(id, &mut |_, out| match file.read(&mut buf) {
        Ok(0) => false,
        Ok(n) => {
            out.extend_from_slice(&buf[..n]);
            total += n;
            true
        }
        Err(e) => {
            failed = Some(e);
            false
        }
    });
    if let Some(e) = failed {
        return Err(e);
    }
    report.files += 1;
    report.bytes += total;
    Ok(Some(id))
}

/// True if a database holds exactly the folders and files of a real
/// directory, with the same content. Nothing is held whole: files are
/// compared a piece at a time. On a difference, says where.
pub fn same_as_dir(db: &Db, path: &Path) -> io::Result<Result<(), String>> {
    fn walk(db: &Db, node: NodeId, path: &Path) -> io::Result<Result<(), String>> {
        use std::io::Read;
        let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| name_bytes(&e.file_name()));
        let children = db.children(node);
        if entries.len() != children.len() {
            return Ok(Err(format!("{}: {} entries, the database has {}", path.display(), entries.len(), children.len())));
        }
        let mut buf = Vec::new();
        for entry in entries {
            let child = entry.path();
            let Some(&id) = children.get(&name_bytes(&entry.file_name())) else {
                return Ok(Err(format!("{}: not in the database", child.display())));
            };
            let is_dir = entry.file_type()?.is_dir();
            if is_dir != db.is_dir(id) {
                return Ok(Err(format!("{}: a folder on one side and a file on the other", child.display())));
            }
            if is_dir {
                if let Err(e) = walk(db, id, &child)? {
                    return Ok(Err(e));
                }
                continue;
            }
            if entry.metadata()?.len() != db.size(id) {
                return Ok(Err(format!("{}: {} bytes, the database has {}", child.display(), entry.metadata()?.len(), db.size(id))));
            }
            let mut file = fs::File::open(&child)?;
            for p in db.pieces(id) {
                let text = db.text_of(p);
                buf.resize(text.len(), 0);
                file.read_exact(&mut buf)?;
                if buf[..] != text[..] {
                    return Ok(Err(format!("{}: content differs", child.display())));
                }
            }
        }
        Ok(Ok(()))
    }
    walk(db, db.root(), path)
}

/// Copy every folder and file of one database into another, which is
/// empty. Node ids are not kept.
pub fn copy_into(from: &Db, to: &mut Db) -> Result<(), String> {
    let mut stack = vec![(from.root(), to.root())];
    let mut pending = 0;
    while let Some((src, dst)) = stack.pop() {
        for (name, child) in from.children(src) {
            if from.is_dir(*child) {
                let made = to.create_folder(dst, name);
                stack.push((*child, made));
            } else {
                let content = from.content(*child);
                to.create_file(dst, name, &content);
                pending += content.len() + 64;
                if pending > 4 << 20 {
                    pending = 0;
                    to.commit();
                }
            }
        }
    }
    to.commit();
    match to.failure() {
        Some(e) => Err(e.to_string()),
        None => Ok(()),
    }
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
    parse_fixture_into(&mut db, text)?;
    Ok(db)
}

/// `parse_fixture` into a database that is empty.
pub fn parse_fixture_into(db: &mut Db, text: &[u8]) -> Result<(), String> {
    let mut current: Option<(NodeId, Vec<u8>)> = None;
    fn flush(db: &mut Db, current: &mut Option<(NodeId, Vec<u8>)>) -> Result<(), String> {
        if let Some((id, content)) = current.take() {
            if !is_valid_text(&content) {
                return Err("fixture file content is not valid UTF-8 text".into());
            }
            db.set_content(id, &content);
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
                None if last && file => db.create_file(cur, comp, &[]),
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
                flush(db, &mut current)?;
                make(db, &line[2..], false)?;
            }
            Some(b'F') if line.starts_with(b"F ") => {
                flush(db, &mut current)?;
                let id = make(db, &line[2..], true)?;
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
    flush(db, &mut current)?;
    db.commit();
    match db.failure() {
        Some(e) => Err(e.to_string()),
        None => Ok(()),
    }
}
