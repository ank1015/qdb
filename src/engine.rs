//! Runs one command line against the database.
//!
//! A line is parsed, every word is expanded against the current tree, every
//! command is checked for unsupported flags, and only then does anything
//! run. A line that uses something unsupported therefore fails before it
//! changes any data.

use crate::cmd;
use crate::glob;
use crate::shell::{self, ParseError, RedirOp, Word};
use crate::tree::{is_valid_text, Db, Errno, NodeId, Scratch, TextCheck};
use crate::vfs::VFile;
use crate::util::lossy;

/// What a command line produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: i32,
}

/// Exit status for anything unsupported or unparsable.
pub const EXIT_UNSUPPORTED: i32 = 2;

/// Somewhere output goes as it is produced.
pub trait Sink {
    /// Take the next piece of output. Returns false when nothing more is
    /// wanted: whoever is producing the output can stop working.
    fn write(&mut self, data: &[u8]) -> bool;
}

impl Sink for Vec<u8> {
    fn write(&mut self, data: &[u8]) -> bool {
        self.extend_from_slice(data);
        true
    }
}

/// Output on its way to a file. It is checked as it arrives (a file holds
/// valid text only) and stored when the command has ended, so that a
/// command whose output is refused changes nothing. Up to the scratch
/// limit it waits in memory, beyond that in a scratch file, so that output
/// of any size takes a fixed amount of memory.
pub struct Spool {
    scratch: Scratch,
    /// All of the output, or with a file the part not yet written to it.
    mem: Vec<u8>,
    file: Option<(String, Box<dyn VFile>)>,
    len: u64,
    check: TextCheck,
    error: Option<std::io::Error>,
}

/// With a scratch file, this much is gathered between writes to it.
const SPOOL_WRITE: usize = 1 << 20;

static SPOOLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl Spool {
    pub fn new(scratch: Scratch) -> Spool {
        Spool { scratch, mem: Vec::new(), file: None, len: 0, check: TextCheck::default(), error: None }
    }

    fn nowhere() -> Spool {
        Spool::new(Scratch { vfs: None, limit: 0 })
    }

    fn to_file(&mut self) {
        if self.file.is_none() {
            let vfs = self.scratch.vfs.as_ref().expect("a place for scratch files");
            let name = format!("spool-{}.tmp", SPOOLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
            match vfs.create(&name, 1 << 38) {
                Ok(f) => self.file = Some((name, f)),
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            }
        }
        let (_, f) = self.file.as_mut().unwrap();
        if let Err(e) = f.write_at(self.len - self.mem.len() as u64, &self.mem) {
            self.error = Some(e);
        }
        self.mem.clear();
    }

    /// The output has ended. True if it is valid text.
    fn end(&mut self) -> bool {
        if self.file.is_some() && !self.mem.is_empty() && self.error.is_none() {
            self.to_file();
        }
        self.check.finish()
    }

    /// Up to `max` bytes of the output from offset `at`. After `end`.
    fn read(&self, at: u64, max: usize) -> &[u8] {
        let n = ((self.len - at) as usize).min(max);
        match &self.file {
            None => &self.mem[at as usize..at as usize + n],
            // Safety: the file holds `len` bytes and is not changed again.
            Some((_, f)) => unsafe { std::slice::from_raw_parts(f.base().add(at as usize), n) },
        }
    }
}

impl Sink for Spool {
    fn write(&mut self, data: &[u8]) -> bool {
        if !self.check.feed(data) || self.error.is_some() {
            // Nothing will be stored; the command runs on as it would.
            self.mem = Vec::new();
            return true;
        }
        self.mem.extend_from_slice(data);
        self.len += data.len() as u64;
        let limit = if self.file.is_some() { SPOOL_WRITE } else { self.scratch.limit };
        if self.mem.len() > limit && self.scratch.vfs.is_some() {
            self.to_file();
        }
        true
    }
}

impl Drop for Spool {
    fn drop(&mut self) {
        if let Some((name, f)) = self.file.take() {
            drop(f);
            if let Some(vfs) = &self.scratch.vfs {
                let _ = vfs.remove(&name);
            }
        }
    }
}

/// Output smaller than this is gathered before it is handed on.
const OUT_BUFFER: usize = 16 * 1024;

/// The streams of one command in a pipeline.
pub struct Io<'a> {
    stdin: &'a [u8],
    out: &'a mut dyn Sink,
    buf: &'a mut Vec<u8>,
    stopped: bool,
    pub err: &'a mut Vec<u8>,
    /// The file standard output is redirected to, if any. GNU cat and grep
    /// refuse to read the file they are writing to.
    pub out_file: Option<NodeId>,
}

impl<'a> Io<'a> {
    /// Read standard input. It is a stream: the first reader gets all of
    /// it and later readers get nothing, as `cat - -` shows.
    pub fn take_stdin(&mut self) -> &'a [u8] {
        std::mem::take(&mut self.stdin)
    }

    pub fn err_line(&mut self, msg: impl AsRef<[u8]>) {
        self.err.extend_from_slice(msg.as_ref());
        self.err.push(b'\n');
    }

    /// Write to standard output.
    pub fn put(&mut self, data: &[u8]) {
        if self.stopped {
            return;
        }
        if self.buf.len() + data.len() > OUT_BUFFER {
            self.flush();
            if data.len() >= OUT_BUFFER {
                if !self.stopped && !self.out.write(data) {
                    self.stopped = true;
                }
                return;
            }
        }
        self.buf.extend_from_slice(data);
    }

    /// Hand on what has been written so far.
    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            if !self.stopped && !self.out.write(self.buf) {
                self.stopped = true;
            }
            self.buf.clear();
        }
    }

    /// True once the reader of this output wants no more of it. A command
    /// that only reads can stop; one that changes data carries on.
    pub fn stopped(&self) -> bool {
        self.stopped
    }
}

/// A command with its flags already checked, ready to run.
pub type Runner = Box<dyn FnOnce(&mut Db, &mut Io) -> i32>;

/// A command reading its standard input in pieces, as the stage before it
/// produces them.
pub trait Filter {
    /// The next piece of input. Returns false when no more is needed.
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool;
    /// The input has ended. Returns the exit status.
    fn finish(self: Box<Self>, io: &mut Io) -> i32;
}

/// How a command uses its standard input.
pub enum Input {
    /// Not at all.
    Unused,
    /// It needs all of it at once (`cat a - b`, `head -n -3`).
    Whole,
    /// It can take it in pieces and needs nothing else.
    Stream(Box<dyn Filter>),
}

/// A command with its flags already checked. `run` is the whole command,
/// reading any standard input it uses from `Io`; `input` says how it can
/// be fed when it is not the first command of a pipeline.
pub struct Prepared {
    pub run: Runner,
    pub input: Input,
}

impl From<Runner> for Prepared {
    fn from(run: Runner) -> Prepared {
        Prepared { run, input: Input::Unused }
    }
}

struct Stage {
    prepared: Prepared,
    redirects: Vec<(RedirOp, Result<Vec<u8>, Vec<u8>>)>,
}

fn fail(msg: String, err: &mut dyn Sink) -> i32 {
    err.write(format!("qdb: {}\n", msg).as_bytes());
    EXIT_UNSUPPORTED
}

/// Expand one word into arguments: pathname expansion, then quote removal.
fn expand(db: &Db, word: &Word) -> Vec<Vec<u8>> {
    let bytes = word.bytes();
    if glob::has_glob(&bytes) {
        let found = glob::expand_path(db, &bytes);
        if !found.is_empty() {
            return found;
        }
    }
    vec![word.text()]
}

/// Run a command line and collect what it prints.
pub fn execute(db: &mut Db, line: &[u8]) -> Outcome {
    let mut out = Outcome::default();
    out.code = execute_to(db, line, &mut out.stdout, &mut out.stderr);
    out
}

/// Run a command line, handing standard output to `out` as it is produced
/// and standard error to `err`. Returns the exit status.
///
/// Nothing is held back beyond a small buffer, so a command that can
/// stream (`cat`, `grep`, `find`, ...) uses the same memory whatever the
/// size of its output. If `out` returns false the commands that only read
/// stop early; commands that change data always run to completion.
pub fn execute_to(db: &mut Db, line: &[u8], out: &mut dyn Sink, err: &mut dyn Sink) -> i32 {
    if let Some(e) = db.failure() {
        err.write(format!("qdb: {}\n", e).as_bytes());
        return 2;
    }
    // Reads cannot return an error, so damage found while reading is
    // thrown (`disk::Damage`). It ends the command line and the use of
    // this database: what the line had changed so far is not logged.
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let code = run_line(db, line, out, err);
        // The line's changes become one record of the log; a file that
        // `>` emptied and nothing wrote to is now empty for good.
        db.commit();
        code
    }));
    let code = match ran {
        Ok(code) => code,
        Err(payload) => match payload.downcast::<crate::disk::Damage>() {
            Ok(d) => {
                db.after_damage(&d);
                2
            }
            Err(other) => std::panic::resume_unwind(other),
        },
    };
    if let Some(e) = db.failure() {
        err.write(format!("qdb: {}\n", e).as_bytes());
        return 2;
    }
    code
}

fn run_line(db: &mut Db, line: &[u8], out: &mut dyn Sink, err: &mut dyn Sink) -> i32 {
    let pipeline = match shell::parse(line) {
        Ok(None) => return 0,
        Ok(Some(p)) => p,
        Err(ParseError::Unsupported(what)) => return fail(format!("unsupported syntax: {}", what), err),
        Err(ParseError::Syntax(what)) => return fail(what, err),
    };

    // Expand and check everything before running anything.
    let mut stages = Vec::new();
    for command in &pipeline.commands {
        let argv: Vec<Vec<u8>> = command.words.iter().flat_map(|w| expand(db, w)).collect();
        let prepared: Prepared = match argv.split_first() {
            None => {
                let nothing: Runner = Box::new(|_, _| 0);
                nothing.into()
            }
            Some((name, args)) => match cmd::prepare(name, args) {
                Ok(p) => p,
                Err(what) => return fail(format!("unsupported {}", what), err),
            },
        };
        let redirects = command
            .redirects
            .iter()
            .map(|r| {
                let mut targets = expand(db, &r.target);
                let target = if targets.len() == 1 { Ok(targets.pop().unwrap()) } else { Err(r.target.text()) };
                (r.op, target)
            })
            .collect();
        stages.push(Stage { prepared, redirects });
    }

    // The first command drives the pipeline: what it writes is pushed
    // through the later ones as it appears. When one of them needs no more
    // input, the write is refused and the commands before it stop.
    let mut stages = stages.into_iter();
    let first = stages.next().expect("a pipeline has a command");
    let mut later: Vec<Later> = stages.map(|s| Later::new(s, db.scratch())).collect();
    let mut stderr = Vec::new();
    let mut buf = Vec::new();
    let mut code = run_whole(db, first.prepared.run, &first.redirects, &[], &mut Rest { stages: &mut later, last: out }, &mut buf, &mut stderr);
    err.write(&stderr);
    for i in 0..later.len() {
        let (stage, rest) = later[i..].split_first_mut().unwrap();
        stderr.clear();
        code = stage.finish(db, &mut Rest { stages: rest, last: out }, &mut stderr);
        err.write(&stderr);
    }
    code
}

/// A command after the first one in a pipeline.
struct Later {
    state: State,
    redirects: Vec<(RedirOp, Result<Vec<u8>, Vec<u8>>)>,
    run: Option<Runner>,
    /// Output on its way to the next stage.
    buf: Vec<u8>,
    /// Output on its way to a file, when standard output is redirected.
    to_file: Spool,
    err: Vec<u8>,
}

enum State {
    /// Taking input in pieces.
    Streaming(Box<dyn Filter>),
    /// Gathering all the input, to run on it when it ends.
    Gathering(Vec<u8>),
    /// Needs no input, or no more of it.
    Closed(Option<Box<dyn Filter>>),
}

impl Later {
    fn new(stage: Stage, scratch: Scratch) -> Later {
        let state = match stage.prepared.input {
            Input::Stream(f) => State::Streaming(f),
            Input::Whole => State::Gathering(Vec::new()),
            Input::Unused => State::Closed(None),
        };
        Later { state, redirects: stage.redirects, run: Some(stage.prepared.run), buf: Vec::new(), to_file: Spool::new(scratch), err: Vec::new() }
    }

    fn push(&mut self, data: &[u8], down: &mut dyn Sink) -> bool {
        match &mut self.state {
            State::Closed(_) => false,
            State::Gathering(all) => {
                all.extend_from_slice(data);
                true
            }
            State::Streaming(filter) => {
                let redirected = !self.redirects.is_empty();
                let out: &mut dyn Sink = if redirected { &mut self.to_file } else { down };
                let mut io = Io { stdin: &[], out, buf: &mut self.buf, stopped: false, err: &mut self.err, out_file: None };
                let more = filter.push(data, &mut io);
                io.flush();
                // If the next stage wants no more, neither does this one.
                let more = more && !io.stopped;
                if !more {
                    if let State::Streaming(f) = std::mem::replace(&mut self.state, State::Closed(None)) {
                        self.state = State::Closed(Some(f));
                    }
                }
                more
            }
        }
    }

    /// The stage before has ended. Returns the exit status.
    fn finish(&mut self, db: &mut Db, down: &mut dyn Sink, stderr: &mut Vec<u8>) -> i32 {
        let filter = match std::mem::replace(&mut self.state, State::Closed(None)) {
            State::Streaming(f) | State::Closed(Some(f)) => f,
            State::Gathering(all) => {
                return run_whole(db, self.run.take().unwrap(), &self.redirects, &all, down, &mut self.buf, stderr);
            }
            State::Closed(None) => {
                return run_whole(db, self.run.take().unwrap(), &self.redirects, &[], down, &mut self.buf, stderr);
            }
        };
        if self.redirects.is_empty() {
            let mut io = Io { stdin: &[], out: down, buf: &mut self.buf, stopped: false, err: &mut self.err, out_file: None };
            let code = filter.finish(&mut io);
            io.flush();
            stderr.extend_from_slice(&self.err);
            return code;
        }
        // The file is opened now, after the stages before have finished,
        // which is when a command run on the whole input would open it.
        let mut undo = Vec::new();
        let target = match open_redirects(db, &self.redirects, &mut undo, stderr) {
            Ok(t) => t.expect("there is a redirection"),
            Err(code) => return code,
        };
        let mut io = Io { stdin: &[], out: &mut self.to_file, buf: &mut self.buf, stopped: false, err: &mut self.err, out_file: None };
        let code = filter.finish(&mut io);
        io.flush();
        stderr.extend_from_slice(&self.err);
        commit_redirect(db, target, undo, std::mem::replace(&mut self.to_file, Spool::nowhere()), code, stderr)
    }
}

/// The stages after the current one, as the place its output goes.
struct Rest<'a> {
    stages: &'a mut [Later],
    last: &'a mut dyn Sink,
}

impl Sink for Rest<'_> {
    fn write(&mut self, data: &[u8]) -> bool {
        match self.stages.split_first_mut() {
            None => self.last.write(data),
            Some((next, rest)) => next.push(data, &mut Rest { stages: rest, last: &mut *self.last }),
        }
    }
}

/// What a redirection did before the command ran, so it can be undone.
enum Undo {
    Created(NodeId),
    /// And the modification time it had.
    Truncated(NodeId, i64),
}

fn open_redirect(db: &mut Db, op: RedirOp, path: &[u8], undo: &mut Vec<Undo>) -> Result<NodeId, Errno> {
    match db.resolve(path) {
        Ok(id) if db.is_dir(id) => Err(Errno::IsDir),
        Ok(id) => {
            if op == RedirOp::Write {
                // Emptied for the command to see, but nothing is stored
                // until the command's output is known: if that is what
                // the file held, nothing is written at all.
                let before = db.mtime(id);
                db.veil(id);
                // Opening a file to write it anew sets its time, whatever
                // is then written.
                db.touch(id);
                undo.push(Undo::Truncated(id, before));
            }
            Ok(id)
        }
        Err(Errno::NoEnt) => {
            if path.ends_with(b"/") {
                return Err(if path.iter().all(|c| *c == b'/') { Errno::NoEnt } else { Errno::IsDir });
            }
            let (parent, name) = db.resolve_parent(path)?;
            let id = db.create_file(parent, &name, &[]);
            undo.push(Undo::Created(id));
            Ok(id)
        }
        Err(e) => Err(e),
    }
}

type Target = (RedirOp, NodeId, Vec<u8>);

/// Open a command's redirections left to right, as bash does before the
/// command runs: `>` creates or empties its file even if a later one
/// takes the output. Returns the file that receives standard output.
fn open_redirects(
    db: &mut Db,
    redirects: &[(RedirOp, Result<Vec<u8>, Vec<u8>>)],
    undo: &mut Vec<Undo>,
    stderr: &mut Vec<u8>,
) -> Result<Option<Target>, i32> {
    let mut target = None;
    for (op, path) in redirects {
        let opened = match path {
            Err(word) => Err(format!("qdb: {}: ambiguous redirect\n", lossy(word))),
            Ok(path) => open_redirect(db, *op, path, undo)
                .map(|id| (id, path.clone()))
                .map_err(|e| format!("qdb: {}: {}\n", lossy(path), e.msg())),
        };
        match opened {
            Ok((id, path)) => target = Some((*op, id, path)),
            Err(msg) => {
                stderr.extend_from_slice(msg.as_bytes());
                return Err(1);
            }
        }
    }
    Ok(target)
}

/// Run a whole command on input that is already complete. Returns the
/// exit status.
fn run_whole(
    db: &mut Db,
    run: Runner,
    redirects: &[(RedirOp, Result<Vec<u8>, Vec<u8>>)],
    stdin: &[u8],
    down: &mut dyn Sink,
    buf: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
) -> i32 {
    let mut undo = Vec::new();
    let target = match open_redirects(db, redirects, &mut undo, stderr) {
        Ok(t) => t,
        Err(code) => return code,
    };
    match target {
        None => {
            let mut io = Io { stdin, out: down, buf, stopped: false, err: stderr, out_file: None };
            let code = run(db, &mut io);
            io.flush();
            code
        }
        Some(target) => {
            // Output bound for a file is checked as a whole before any of
            // it is stored, so it is gathered first.
            let mut gathered = Spool::new(db.scratch());
            let mut io = Io { stdin, out: &mut gathered, buf, stopped: false, err: stderr, out_file: Some(target.1) };
            let code = run(db, &mut io);
            io.flush();
            commit_redirect(db, target, undo, gathered, code, stderr)
        }
    }
}

/// Store redirected output in its file, or undo the redirection if the
/// result would not be valid text.
fn commit_redirect(db: &mut Db, target: Target, undo: Vec<Undo>, mut out: Spool, mut code: i32, stderr: &mut Vec<u8>) -> i32 {
    let (op, id, path) = target;
    let valid = out.end();
    if let Some(e) = out.error.take() {
        // Reported when the command line ends, like any failure to store.
        db.scratch_failed(e);
        return code;
    }
    if !valid {
        // Text only: reject the write and leave the data as it was.
        for u in undo.into_iter().rev() {
            match u {
                Undo::Created(id) if db.exists(id) => db.remove(id),
                Undo::Truncated(id, before) if db.exists(id) => {
                    db.unveil(id);
                    db.set_mtime(id, before);
                }
                _ => {}
            }
        }
        stderr.extend_from_slice(
            format!("qdb: {}: output is not valid UTF-8 text, nothing was written\n", lossy(&path)).as_bytes(),
        );
        return 1;
    }
    // The command may have removed or replaced the file it was writing to;
    // the output then goes nowhere, as it would with an unlinked file.
    if db.exists(id) && !db.is_dir(id) && out.len > 0 {
        // How much of the output is handed to storage at a time: 4 MB
        // with pieces of the usual size.
        let step = db.config().piece * 64;
        if op == RedirOp::Append || db.size(id) == 0 {
            // Valid text added to valid text is valid text.
            let mut at = 0;
            let mut first = op == RedirOp::Write;
            if out.len as usize <= step {
                let all = out.read(0, step);
                if first {
                    db.set_content(id, all);
                } else {
                    db.append(id, all);
                }
                return code;
            }
            if first {
                // What the file held is still known: pieces at its start
                // that the output repeats are kept.
                db.set_content_stream(id, &mut |_, buf| {
                    let part = out.read(at, step / 4);
                    buf.extend_from_slice(part);
                    at += part.len() as u64;
                    !part.is_empty()
                });
                first = false;
            }
            while !first && at < out.len {
                // In whole lines where there are any.
                let mut part = out.read(at, step);
                if at + (part.len() as u64) < out.len {
                    if let Some(nl) = memchr::memrchr(b'\n', part) {
                        part = &part[..nl + 1];
                    }
                }
                db.append(id, part);
                at += part.len() as u64;
            }
            return code;
        }
        // The command itself wrote to the file its output goes to. The
        // output starts at offset 0 of what is there now.
        let mut content = db.content(id).into_owned();
        let out = out.read(0, usize::MAX);
        if out.len() >= content.len() {
            content = out.to_vec();
        } else {
            content[..out.len()].copy_from_slice(out);
        }
        if is_valid_text(&content) {
            db.set_content(id, &content);
        } else {
            stderr.extend_from_slice(
                format!("qdb: {}: output is not valid UTF-8 text, nothing was written\n", lossy(&path)).as_bytes(),
            );
            code = 1;
        }
    }
    code
}
