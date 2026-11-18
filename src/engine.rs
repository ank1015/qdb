//! Runs one command line against the database.
//!
//! A line is parsed, every word is expanded against the current tree, every
//! command is checked for unsupported flags, and only then does anything
//! run. A line that uses something unsupported therefore fails before it
//! changes any data.

use crate::cmd;
use crate::glob;
use crate::shell::{self, ParseError, RedirOp, Word};
use crate::tree::{is_valid_text, Db, Errno, NodeId};
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
    let mut later: Vec<Later> = stages.map(Later::new).collect();
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
    to_file: Vec<u8>,
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
    fn new(stage: Stage) -> Later {
        let state = match stage.prepared.input {
            Input::Stream(f) => State::Streaming(f),
            Input::Whole => State::Gathering(Vec::new()),
            Input::Unused => State::Closed(None),
        };
        Later { state, redirects: stage.redirects, run: Some(stage.prepared.run), buf: Vec::new(), to_file: Vec::new(), err: Vec::new() }
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
        commit_redirect(db, target, undo, std::mem::take(&mut self.to_file), code, stderr)
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
    Truncated(NodeId, Vec<u8>),
}

fn open_redirect(db: &mut Db, op: RedirOp, path: &[u8], undo: &mut Vec<Undo>) -> Result<NodeId, Errno> {
    match db.resolve(path) {
        Ok(id) if db.is_dir(id) => Err(Errno::IsDir),
        Ok(id) => {
            if op == RedirOp::Write {
                let old = db.content(id).to_vec();
                db.set_content(id, Vec::new());
                undo.push(Undo::Truncated(id, old));
            }
            Ok(id)
        }
        Err(Errno::NoEnt) => {
            if path.ends_with(b"/") {
                return Err(if path.iter().all(|c| *c == b'/') { Errno::NoEnt } else { Errno::IsDir });
            }
            let (parent, name) = db.resolve_parent(path)?;
            let id = db.create_file(parent, &name, Vec::new());
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
            let mut gathered = Vec::new();
            let mut io = Io { stdin, out: &mut gathered, buf, stopped: false, err: stderr, out_file: Some(target.1) };
            let code = run(db, &mut io);
            io.flush();
            commit_redirect(db, target, undo, gathered, code, stderr)
        }
    }
}

/// Store redirected output in its file, or undo the redirection if the
/// result would not be valid text.
fn commit_redirect(db: &mut Db, target: Target, undo: Vec<Undo>, out: Vec<u8>, mut code: i32, stderr: &mut Vec<u8>) -> i32 {
    let (op, id, path) = target;
    if !is_valid_text(&out) {
        // Text only: reject the write and leave the data as it was.
        for u in undo.into_iter().rev() {
            match u {
                Undo::Created(id) if db.exists(id) => db.remove(id),
                Undo::Truncated(id, old) if db.exists(id) => db.set_content(id, old),
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
    if db.exists(id) && !db.is_dir(id) && !out.is_empty() {
        let mut content = db.content(id).to_vec();
        match op {
            RedirOp::Append => content.extend_from_slice(&out),
            RedirOp::Write => {
                // Writing starts at offset 0 of the emptied file.
                if out.len() >= content.len() {
                    content = out;
                } else {
                    content[..out.len()].copy_from_slice(&out);
                }
            }
        }
        if is_valid_text(&content) {
            db.set_content(id, content);
        } else {
            stderr.extend_from_slice(
                format!("qdb: {}: output is not valid UTF-8 text, nothing was written\n", lossy(&path)).as_bytes(),
            );
            code = 1;
        }
    }
    code
}
