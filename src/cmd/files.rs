//! mkdir, touch, cp, mv, rm, rmdir.

use super::{options, usage};
use crate::engine::{Io, Runner};
use crate::tree::{Db, Errno, NodeId};
use crate::util::{basename, join, quote};
use crate::when;
use std::collections::HashSet;

/// True if the path's last component is `.` or `..`.
fn ends_in_dot(path: &[u8]) -> bool {
    matches!(basename(path), b"." | b"..")
}

pub fn mkdir(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("mkdir", &args, b"p", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let parents = p.has(b'p');
    Ok(Box::new(move |db, io| {
        if p.operands.is_empty() {
            return usage(io, "mkdir", "missing operand");
        }
        let mut status = 0;
        for path in &p.operands {
            let result = if parents { mkdir_parents(db, path) } else { mkdir_one(db, path).map_err(|e| (path.clone(), e)) };
            if let Err((shown, e)) = result {
                io.err_line(format!("mkdir: cannot create directory {}: {}", quote(&shown), e.msg()));
                status = 1;
            }
        }
        status
    }))
}

fn mkdir_one(db: &mut Db, path: &[u8]) -> Result<(), Errno> {
    if db.resolve(path).is_ok() {
        return Err(Errno::Exist);
    }
    let (parent, name) = db.resolve_parent(path)?;
    if db.child(parent, &name).is_some() {
        // A file named with a trailing slash.
        return Err(Errno::Exist);
    }
    db.create_folder(parent, &name);
    Ok(())
}

/// mkdir -p: create every missing folder along the path. The error carries
/// the part of the path GNU names in its message.
fn mkdir_parents(db: &mut Db, path: &[u8]) -> Result<(), (Vec<u8>, Errno)> {
    if path.is_empty() {
        return Err((Vec::new(), Errno::NoEnt));
    }
    let mut cur = db.root();
    let mut pos = 0;
    let trimmed_len = path.len() - path.iter().rev().take_while(|c| **c == b'/').count();
    for comp in path.split(|c| *c == b'/') {
        let end = pos + comp.len();
        let prefix = &path[..end];
        pos = end + 1;
        if comp.is_empty() || comp == b"." {
            continue;
        }
        if comp == b".." {
            cur = db.parent(cur).unwrap_or(db.root());
            continue;
        }
        cur = match db.child(cur, comp) {
            Some(c) if db.is_dir(c) => c,
            Some(_) if end >= trimmed_len => return Err((path.to_vec(), Errno::Exist)),
            Some(_) => return Err((prefix.to_vec(), Errno::NotDir)),
            None => db.create_folder(cur, comp),
        };
    }
    Ok(())
}

/// When `touch -d` or `-t` says.
enum Stamp {
    Now,
    At(i64),
    /// `-t` with no year: the year is the current one.
    ThisYear(Vec<u8>),
    Invalid(Vec<u8>),
    /// Both `-d` and `-t`.
    TwoSources,
}

pub fn touch(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("touch", &args, b"c", b"dt", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let no_create = p.has(b'c');
    let stamp = match (p.arg(b'd'), p.arg(b't')) {
        (Some(_), Some(_)) => Stamp::TwoSources,
        (Some(d), None) => match when::parse_date(d) {
            Some(t) => Stamp::At(t),
            // What looks like one of the supported forms but names no
            // real moment is an invalid date; anything else is a form
            // this version does not read.
            None if !d.is_empty() && d.iter().all(|c| c.is_ascii_digit() || b"-:.,T @".contains(c)) => Stamp::Invalid(d.to_vec()),
            None => {
                return Err(format!(
                    "flag: touch -d {} (the dates supported are @SECONDS, YYYY-MM-DD and YYYY-MM-DD HH:MM[:SS[.frac]])",
                    quote(d)
                ))
            }
        },
        (None, Some(t)) => match t.iter().position(|c| *c == b'.').unwrap_or(t.len()) {
            8 => Stamp::ThisYear(t.to_vec()),
            _ => when::parse_stamp(t, 0).map_or(Stamp::Invalid(t.to_vec()), Stamp::At),
        },
        (None, None) => Stamp::Now,
    };
    Ok(Box::new(move |db, io| {
        let time = match &stamp {
            Stamp::Now => None,
            Stamp::At(t) => Some(*t),
            Stamp::ThisYear(t) => match when::parse_stamp(t, when::civil(db.now()).year) {
                Some(t) => Some(t),
                None => {
                    io.err_line(format!("touch: invalid date format {}", quote(t)));
                    return 1;
                }
            },
            Stamp::Invalid(t) => {
                io.err_line(format!("touch: invalid date format {}", quote(t)));
                return 1;
            }
            Stamp::TwoSources => return usage(io, "touch", "cannot specify times from more than one source"),
        };
        if p.operands.is_empty() {
            return usage(io, "touch", "missing file operand");
        }
        let mut status = 0;
        for path in &p.operands {
            let err = match db.resolve(path) {
                Ok(id) => {
                    let t = time.unwrap_or_else(|| db.now());
                    db.set_mtime(id, t);
                    continue;
                }
                Err(e) => e,
            };
            if no_create && err == Errno::NoEnt {
                continue;
            }
            let result = if path.ends_with(b"/") {
                Err(format!("touch: setting times of {}: {}", quote(path), err.msg()))
            } else {
                match db.resolve_parent(path) {
                    Ok((parent, name)) => {
                        let id = db.create_file(parent, &name, &[]);
                        if let Some(t) = time {
                            db.set_mtime(id, t);
                        }
                        Ok(())
                    }
                    Err(e) => Err(format!("touch: cannot touch {}: {}", quote(path), e.msg())),
                }
            };
            if let Err(m) = result {
                io.err_line(m);
                status = 1;
            }
        }
        status
    }))
}

/// `removed 'x'` for everything under a node, children first, as rm -v
/// prints while it works.
fn removed_lines(db: &Db, id: NodeId, path: &[u8], io: &mut Io) {
    if db.is_dir(id) {
        for (name, child) in db.children(id) {
            removed_lines(db, *child, &join(path, name), io);
        }
        io.put(format!("removed directory {}\n", quote(path)).as_bytes());
    } else {
        io.put(format!("removed {}\n", quote(path)).as_bytes());
    }
}

pub fn rm(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("rm", &args, b"rRfdv", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    // The later of -f and a (hypothetical) -i wins in GNU rm; with only -f
    // supported, its presence is enough.
    let (recursive, force, empty_dirs, verbose) = (p.has(b'r') || p.has(b'R'), p.has(b'f'), p.has(b'd'), p.has(b'v'));
    Ok(Box::new(move |db, io| {
        if p.operands.is_empty() {
            return if force { 0 } else { usage(io, "rm", "missing operand") };
        }
        let mut status = 0;
        for path in &p.operands {
            let q = quote(path);
            if ends_in_dot(path) && recursive {
                io.err_line(format!("rm: refusing to remove '.' or '..' directory: skipping {}", q));
                status = 1;
                continue;
            }
            let id = match db.resolve(path) {
                Ok(id) => id,
                Err(e) => {
                    if !force || e != Errno::NoEnt && e != Errno::NotDir {
                        io.err_line(format!("rm: cannot remove {}: {}", q, e.msg()));
                        status = 1;
                    }
                    continue;
                }
            };
            if id == db.root() {
                if recursive {
                    io.err_line(format!("rm: it is dangerous to operate recursively on {}", q));
                    io.err_line("rm: use --no-preserve-root to override this failsafe");
                } else if empty_dirs {
                    io.err_line(format!("rm: cannot remove {}: Directory not empty", q));
                } else {
                    io.err_line(format!("rm: cannot remove {}: Is a directory", q));
                }
                status = 1;
                continue;
            }
            if db.is_dir(id) && !recursive {
                if !empty_dirs {
                    io.err_line(format!("rm: cannot remove {}: Is a directory", q));
                    status = 1;
                    continue;
                }
                if !db.children(id).is_empty() {
                    io.err_line(format!("rm: cannot remove {}: Directory not empty", q));
                    status = 1;
                    continue;
                }
            }
            if verbose {
                removed_lines(db, id, path, io);
            }
            db.remove(id);
        }
        status
    }))
}

/// Remove one empty folder, or say why not.
fn rmdir_one(db: &mut Db, path: &[u8]) -> Result<(), &'static str> {
    match db.resolve(path) {
        Err(e) => Err(e.msg()),
        Ok(id) if !db.is_dir(id) => Err(Errno::NotDir.msg()),
        Ok(_) if basename(path) == b"." => Err(Errno::Inval.msg()),
        Ok(id) if id == db.root() && basename(path) != b".." => Err("Device or resource busy"),
        Ok(id) if !db.children(id).is_empty() || id == db.root() => Err(Errno::NotEmpty.msg()),
        Ok(id) => {
            db.remove(id);
            Ok(())
        }
    }
}

pub fn rmdir(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("rmdir", &args, b"p", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let parents = p.has(b'p');
    Ok(Box::new(move |db, io| {
        if p.operands.is_empty() {
            return usage(io, "rmdir", "missing operand");
        }
        let mut status = 0;
        for path in &p.operands {
            if let Err(m) = rmdir_one(db, path) {
                io.err_line(format!("rmdir: failed to remove {}: {}", quote(path), m));
                status = 1;
                continue;
            }
            if !parents {
                continue;
            }
            // Then each folder the path names on the way down, last first.
            let mut dir: &[u8] = path;
            loop {
                while dir.len() > 1 && dir.ends_with(b"/") {
                    dir = &dir[..dir.len() - 1];
                }
                let Some(slash) = dir.iter().rposition(|c| *c == b'/') else { break };
                dir = &dir[..slash];
                while dir.len() > 1 && dir.ends_with(b"/") {
                    dir = &dir[..dir.len() - 1];
                }
                if dir.is_empty() {
                    break;
                }
                if let Err(m) = rmdir_one(db, dir) {
                    io.err_line(format!("rmdir: failed to remove directory {}: {}", quote(dir), m));
                    status = 1;
                    break;
                }
            }
        }
        status
    }))
}

/// The shared front end of cp and mv: split the operands into sources and a
/// destination, and work out the target path of each source.
fn sources_and_targets(db: &Db, io: &mut Io, cmd: &str, operands: &[Vec<u8>], as_file: bool) -> Result<Vec<(Vec<u8>, Vec<u8>)>, i32> {
    let (dest, sources) = match operands.split_last() {
        None => return Err(usage(io, cmd, "missing file operand")),
        Some((only, [])) => {
            return Err(usage(io, cmd, &format!("missing destination file operand after {}", quote(only))))
        }
        Some(x) => x,
    };
    if as_file {
        // -T: the destination is the target itself, never a folder to
        // copy into.
        if operands.len() > 2 {
            return Err(usage(io, cmd, &format!("extra operand {}", quote(&operands[2]))));
        }
        return Ok(vec![(sources[0].clone(), dest.clone())]);
    }
    let dest_node = db.resolve(dest);
    let dest_is_dir = dest_node.is_ok_and(|id| db.is_dir(id));
    if sources.len() > 1 && !dest_is_dir {
        let why = match dest_node {
            Ok(_) => Errno::NotDir,
            Err(e) => e,
        };
        io.err_line(format!("{}: target {}: {}", cmd, quote(dest), why.msg()));
        return Err(1);
    }
    Ok(sources
        .iter()
        .map(|s| (s.clone(), if dest_is_dir { join(dest, basename(s)) } else { dest.clone() }))
        .collect())
}

pub fn cp(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("cp", &args, b"rRapfnTv", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let archive = p.has(b'a');
    let recursive = p.has(b'r') || p.has(b'R') || archive;
    let (preserve, no_clobber, as_file, verbose) = (p.has(b'p') || archive, p.has(b'n'), p.has(b'T'), p.has(b'v'));
    Ok(Box::new(move |db, io| {
        if no_clobber {
            io.err_line("cp: warning: behavior of -n is non-portable and may change in future; use --update=none instead");
        }
        let pairs = match sources_and_targets(db, io, "cp", &p.operands, as_file) {
            Ok(p) => p,
            Err(status) => return status,
        };
        let mut copy = Copy { recursive, preserve, no_clobber, verbose, created: HashSet::new(), into_self: false };
        let mut status = 0;
        for (src, dst) in &pairs {
            copy.into_self = false;
            if !copy.copy(db, io, src, dst, src, dst) {
                status = 1;
            }
        }
        status
    }))
}

struct Copy {
    recursive: bool,
    /// -p, -a: the copy keeps the source's modification time.
    preserve: bool,
    /// -n: a file that is there already is left alone.
    no_clobber: bool,
    verbose: bool,
    /// Folders this cp created. Meeting one of them as a source means the
    /// copy is descending into its own output.
    created: HashSet<NodeId>,
    into_self: bool,
}

impl Copy {
    /// Copy `src` to the exact path `dst`. Returns false on any failure.
    fn copy(&mut self, db: &mut Db, io: &mut Io, src: &[u8], dst: &[u8], top_src: &[u8], top_dst: &[u8]) -> bool {
        let src_id = match db.resolve(src) {
            Ok(id) => id,
            Err(e) => {
                io.err_line(format!("cp: cannot stat {}: {}", quote(src), e.msg()));
                return false;
            }
        };
        if db.is_dir(src_id) {
            if !self.recursive {
                io.err_line(format!("cp: -r not specified; omitting directory {}", quote(src)));
                return false;
            }
            return self.copy_dir(db, io, src_id, src, dst, top_src, top_dst);
        }
        match db.resolve(dst) {
            Ok(d) if d == src_id => {
                io.err_line(format!("cp: {} and {} are the same file", quote(src), quote(dst)));
                false
            }
            Ok(d) if db.is_dir(d) => {
                io.err_line(format!("cp: cannot overwrite directory {} with non-directory", quote(dst)));
                false
            }
            Ok(_) if self.no_clobber => true,
            Ok(d) => {
                // Nothing is copied: the two files share their content
                // until one of them changes.
                db.copy_content(src_id, d);
                self.copied(db, io, src_id, d, src, dst);
                true
            }
            Err(Errno::NoEnt) => {
                let made = if dst.ends_with(b"/") {
                    Err(Errno::NotDir)
                } else {
                    db.resolve_parent(dst).map(|(parent, name)| {
                        let made = db.create_file(parent, &name, &[]);
                        db.copy_content(src_id, made);
                        made
                    })
                };
                match made {
                    Ok(made) => self.copied(db, io, src_id, made, src, dst),
                    Err(e) => io.err_line(format!("cp: cannot create regular file {}: {}", quote(dst), e.msg())),
                }
                made.is_ok()
            }
            Err(e) => {
                io.err_line(format!("cp: cannot stat {}: {}", quote(dst), e.msg()));
                false
            }
        }
    }

    /// A file has been copied: the line -v prints and the time -p keeps.
    fn copied(&self, db: &mut Db, io: &mut Io, from: NodeId, to: NodeId, src: &[u8], dst: &[u8]) {
        if self.verbose {
            io.put(format!("{} -> {}\n", quote(src), quote(dst)).as_bytes());
        }
        if self.preserve {
            let t = db.mtime(from);
            db.set_mtime(to, t);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_dir(&mut self, db: &mut Db, io: &mut Io, src_id: NodeId, src: &[u8], dst: &[u8], top_src: &[u8], top_dst: &[u8]) -> bool {
        if self.created.contains(&src_id) {
            io.err_line(format!("cp: cannot copy a directory, {}, into itself, {}", quote(top_src), quote(top_dst)));
            self.into_self = true;
            return false;
        }
        let dst_id = match db.resolve(dst) {
            Ok(d) if !db.is_dir(d) => {
                io.err_line(format!("cp: cannot overwrite non-directory {} with directory {}", quote(dst), quote(src)));
                return false;
            }
            Ok(d) => d,
            Err(Errno::NoEnt) => match db.resolve_parent(dst) {
                Ok((parent, name)) => {
                    let id = db.create_folder(parent, &name);
                    self.created.insert(id);
                    if self.verbose {
                        io.put(format!("{} -> {}\n", quote(src), quote(dst)).as_bytes());
                    }
                    id
                }
                Err(e) => {
                    io.err_line(format!("cp: cannot create directory {}: {}", quote(dst), e.msg()));
                    return false;
                }
            },
            Err(e) => {
                io.err_line(format!("cp: cannot stat {}: {}", quote(dst), e.msg()));
                return false;
            }
        };
        // List the source after creating the destination, as a real
        // directory read would see it.
        let names: Vec<Vec<u8>> = db.children(src_id).keys().cloned().collect();
        let mut ok = true;
        for name in names {
            ok &= self.copy(db, io, &join(src, &name), &join(dst, &name), top_src, top_dst);
            if self.into_self {
                break;
            }
        }
        if self.preserve && db.exists(dst_id) && db.exists(src_id) {
            // After its entries, which changed its time as they arrived.
            let t = db.mtime(src_id);
            db.set_mtime(dst_id, t);
        }
        ok
    }
}

pub fn mv(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("mv", &args, b"fnv", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    // Of -f and -n the later one decides.
    let no_clobber = p.flags.iter().rev().find(|(f, _)| matches!(f, b'f' | b'n')).is_some_and(|(f, _)| *f == b'n');
    let verbose = p.has(b'v');
    Ok(Box::new(move |db, io| {
        let pairs = match sources_and_targets(db, io, "mv", &p.operands, false) {
            Ok(p) => p,
            Err(status) => return status,
        };
        let mut status = 0;
        for (src, dst) in &pairs {
            if no_clobber && db.resolve(src).is_ok() && db.resolve(dst).is_ok() {
                io.err_line(format!("mv: not replacing {}", quote(dst)));
                status = 1;
                continue;
            }
            match move_one(db, src, dst) {
                Ok(()) if verbose => io.put(format!("renamed {} -> {}\n", quote(src), quote(dst)).as_bytes()),
                Ok(()) => {}
                Err(m) => {
                    io.err_line(format!("mv: {}", m));
                    status = 1;
                }
            }
        }
        status
    }))
}

/// Move `src` to the exact path `dst`. The node keeps its id.
fn move_one(db: &mut Db, src: &[u8], dst: &[u8]) -> Result<(), String> {
    let (qs, qd) = (quote(src), quote(dst));
    let cannot = |e: &str| format!("cannot move {} to {}: {}", qs, qd, e);
    let into_itself = || format!("cannot move {} to a subdirectory of itself, {}", qs, qd);
    let id = db.resolve(src).map_err(|e| format!("cannot stat {}: {}", qs, e.msg()))?;
    let src_is_dir = db.is_dir(id);
    if dst.is_empty() && !src_is_dir {
        return Err("cannot overwrite directory '' with non-directory".into());
    }
    match db.resolve(dst) {
        Ok(d) if d == id => Err(format!("{} and {} are the same file", qs, qd)),
        _ if ends_in_dot(src) || id == db.root() => Err(cannot("Device or resource busy")),
        Ok(d) => {
            if src_is_dir && db.is_same_or_descendant(d, id) {
                return Err(into_itself());
            }
            match (src_is_dir, db.is_dir(d)) {
                (true, false) => return Err(format!("cannot overwrite non-directory {} with directory {}", qd, qs)),
                (false, true) => return Err(format!("cannot overwrite directory {} with non-directory", qd)),
                (true, true) if !db.children(d).is_empty() || d == db.root() => {
                    return Err(format!("cannot overwrite {}: Directory not empty", qd))
                }
                _ => {}
            }
            // Replace the destination: an empty folder or a file.
            let parent = db.parent(d).unwrap();
            let name = db.name(d).to_vec();
            db.remove(d);
            db.rename(id, parent, &name).map_err(|e| cannot(e.msg()))
        }
        Err(Errno::NoEnt) => {
            if dst.ends_with(b"/") && !src_is_dir {
                return Err(cannot(Errno::NotDir.msg()));
            }
            let (parent, name) = db.resolve_parent(dst).map_err(|e| cannot(e.msg()))?;
            if src_is_dir && db.is_same_or_descendant(parent, id) {
                return Err(into_itself());
            }
            db.rename(id, parent, &name).map_err(|e| cannot(e.msg()))
        }
        Err(e) => Err(format!("cannot stat {}: {}", qd, e.msg())),
    }
}
