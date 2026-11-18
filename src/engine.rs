//! Runs one command line against the database.
//!
//! One call is one unit: it is parsed whole, every command in it is
//! checked for unsupported flags, and only then does anything run. A unit
//! that uses something unsupported therefore fails before it changes any
//! data, and what a unit changes is logged as one record.

use crate::cmd;
use crate::glob;
use crate::shell::{self, Connector, ParseError, Part, Pipeline, Redirect, Script, Word};
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

    /// How many bytes were written.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// The output has ended. True if it is valid text.
    pub fn end(&mut self) -> bool {
        if self.file.is_some() && !self.mem.is_empty() && self.error.is_none() {
            self.to_file();
        }
        self.check.finish()
    }

    /// Up to `max` bytes of the output from offset `at`. After `end`.
    pub fn read(&self, at: u64, max: usize) -> &[u8] {
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
    /// Standard error goes where standard output goes (`2>&1`, `&>`).
    merged: bool,
    /// The command writes without a buffer of its own, as cat does, so
    /// merged output and errors appear in the order they were written.
    exact: bool,
    stdin_size: Option<u64>,
    piped: bool,
    scratch: Scratch,
}

impl<'a> Io<'a> {
    /// Where a command may keep what does not fit in memory.
    pub fn scratch(&self) -> Scratch {
        self.scratch.clone()
    }

    /// Say that the command's errors overtake its output. The GNU tools
    /// write out what they have buffered before they report an error, so
    /// with `2>&1` output and errors appear in the order they were
    /// written; that is what happens here unless a command says
    /// otherwise. sed, rg and the shell's own commands do not: their
    /// errors come out ahead of the output still in the buffer.
    pub fn errors_first(&mut self) {
        self.exact = false;
    }

    /// The size of the file standard input was redirected from, if it was.
    pub fn stdin_size(&self) -> Option<u64> {
        self.stdin_size
    }

    /// Whether standard input is a pipe or a file. rg searches it then,
    /// and the tree when it is neither.
    pub fn has_stdin(&self) -> bool {
        self.piped
    }

    /// Standard input from here on is empty and not a pipe, as it is for
    /// the commands xargs runs.
    pub fn close_stdin(&mut self) {
        self.stdin = &[];
        self.piped = false;
    }

    /// Read standard input. It is a stream: the first reader gets all of
    /// it and later readers get nothing, as `cat - -` shows.
    pub fn take_stdin(&mut self) -> &'a [u8] {
        std::mem::take(&mut self.stdin)
    }

    pub fn err_line(&mut self, msg: impl AsRef<[u8]>) {
        if self.merged && self.exact {
            self.flush();
        }
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
        if self.merged && !self.err.is_empty() {
            // Errors are not buffered: they go ahead of the output that
            // still is.
            if !self.stopped && !self.out.write(self.err) {
                self.stopped = true;
            }
            self.err.clear();
        }
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
    /// The same, for a filter that then changes data, as tee does.
    fn finish_db(self: Box<Self>, _db: &mut Db, io: &mut Io) -> i32 {
        self.finish(io)
    }
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

/// A redirection with its word expanded. A path is `Err` with the word
/// as written when it expanded to no name or to several.
enum Redir {
    Out { fd: u8, append: bool, path: Result<Vec<u8>, Vec<u8>> },
    Both { append: bool, path: Result<Vec<u8>, Vec<u8>> },
    Dup { fd: u8, to: u8 },
    In { path: Result<Vec<u8>, Vec<u8>> },
    Here(Vec<u8>),
}

/// Where a stream of a command goes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dest {
    /// To the next command of the pipeline, or out of the unit.
    Down,
    /// To the standard error of the unit.
    Err,
    /// To the file of the redirection with this index.
    File(usize),
    /// Nowhere.
    Null,
}

/// What a command's redirections come to, read left to right.
#[derive(Clone, Copy)]
struct Plan {
    d1: Dest,
    d2: Dest,
    /// The redirection standard input comes from.
    input: Option<usize>,
}

const DEV_NULL: &[u8] = b"/dev/null";

fn plan(redirs: &[Redir]) -> Plan {
    let mut p = Plan { d1: Dest::Down, d2: Dest::Err, input: None };
    let file = |i: usize, path: &Result<Vec<u8>, Vec<u8>>| match path {
        Ok(p) if p == DEV_NULL => Dest::Null,
        _ => Dest::File(i),
    };
    for (i, r) in redirs.iter().enumerate() {
        match r {
            Redir::Out { fd: 2, path, .. } => p.d2 = file(i, path),
            Redir::Out { path, .. } => p.d1 = file(i, path),
            Redir::Both { path, .. } => {
                p.d1 = file(i, path);
                p.d2 = p.d1;
            }
            Redir::Dup { fd: 1, to: 2 } => p.d1 = p.d2,
            Redir::Dup { fd: 2, to: 1 } => p.d2 = p.d1,
            Redir::Dup { .. } => {}
            Redir::In { .. } | Redir::Here(_) => p.input = Some(i),
        }
    }
    p
}

struct Nowhere;

impl Sink for Nowhere {
    fn write(&mut self, _: &[u8]) -> bool {
        true
    }
}

fn fail(msg: String, err: &mut dyn Sink) -> i32 {
    err.write(format!("qdb: {}\n", msg).as_bytes());
    EXIT_UNSUPPORTED
}

/// The unit cannot go on: something unsupported was met after it began.
struct Stop(i32);

/// Expand one word into arguments: brace expansion, command substitution
/// with field splitting, pathname expansion, then quote removal.
fn expand(db: &mut Db, word: &Word, err: &mut dyn Sink) -> Result<Vec<Vec<u8>>, Stop> {
    // The usual word: no braces, no substitution.
    if let [Part::Lit(b)] = word.parts.as_slice() {
        if !b.contains(&b'{') && !glob::has_glob_bytes(b) {
            return Ok(vec![b.clone()]);
        }
    }
    let mut out = Vec::new();
    for w in shell::brace_expand(word) {
        for field in fields(db, &w, err)? {
            if glob::has_glob(&field) {
                let found = glob::expand_path(db, &field);
                if !found.is_empty() {
                    out.extend(found);
                    continue;
                }
            }
            out.push(field.into_iter().map(|(c, _)| c).collect());
        }
    }
    Ok(out)
}

/// Run a command substitution: its standard output without the newlines
/// at its end.
fn substitute(db: &mut Db, script: &Script, err: &mut dyn Sink) -> Result<Vec<u8>, Stop> {
    let mut text = Vec::new();
    run_script(db, script, &mut text, err, None)?;
    while text.last() == Some(&b'\n') {
        text.pop();
    }
    Ok(text)
}

/// The fields of a word: one, unless an unquoted substitution's output
/// holds spaces, tabs or newlines, which split it; none, if nothing but
/// such a substitution with empty output makes up the word.
fn fields(db: &mut Db, word: &Word, err: &mut dyn Sink) -> Result<Vec<Vec<(u8, bool)>>, Stop> {
    let mut out = Vec::new();
    let mut cur: Vec<(u8, bool)> = Vec::new();
    let mut open = false;
    for p in &word.parts {
        match p {
            Part::Lit(b) => {
                cur.extend(b.iter().map(|c| (*c, false)));
                open |= !b.is_empty();
            }
            Part::Quoted(b) => {
                cur.extend(b.iter().map(|c| (*c, true)));
                open = true;
            }
            Part::Subst { script, quoted: true } => {
                cur.extend(substitute(db, script, err)?.into_iter().map(|c| (c, true)));
                open = true;
            }
            Part::Subst { script, quoted: false } => {
                let text = substitute(db, script, err)?;
                let blank = |c: u8| matches!(c, b' ' | b'\t' | b'\n');
                let mut i = 0;
                while i < text.len() {
                    if blank(text[i]) {
                        if open {
                            out.push(std::mem::take(&mut cur));
                            open = false;
                        }
                        i += 1;
                        continue;
                    }
                    // A backslash from a substitution is a plain character.
                    cur.push((text[i], text[i] == b'\\'));
                    open = true;
                    i += 1;
                }
            }
        }
    }
    if open {
        out.push(cur);
    }
    Ok(out)
}

/// The text of a here-document after its substitutions have run.
fn here_text(db: &mut Db, body: &[Part], err: &mut dyn Sink) -> Result<Vec<u8>, Stop> {
    let mut text = Vec::new();
    for p in body {
        match p {
            Part::Lit(b) | Part::Quoted(b) => text.extend_from_slice(b),
            Part::Subst { script, .. } => text.extend(substitute(db, script, err)?),
        }
    }
    Ok(text)
}

fn expand_redirect(db: &mut Db, r: &Redirect, err: &mut dyn Sink) -> Result<Redir, Stop> {
    let mut one = |db: &mut Db, target: &Word| -> Result<Result<Vec<u8>, Vec<u8>>, Stop> {
        let mut targets = expand(db, target, err)?;
        Ok(if targets.len() == 1 { Ok(targets.pop().unwrap()) } else { Err(target.raw.clone()) })
    };
    Ok(match r {
        Redirect::Out { fd, append, target } => Redir::Out { fd: *fd, append: *append, path: one(db, target)? },
        Redirect::Both { append, target } => Redir::Both { append: *append, path: one(db, target)? },
        Redirect::Dup { fd, to } => Redir::Dup { fd: *fd, to: *to },
        Redirect::In { target } => Redir::In { path: one(db, target)? },
        Redirect::Here { body } => {
            let body = body.borrow().clone();
            Redir::Here(here_text(db, &body, err)?)
        }
    })
}

/// Run a command line and collect what it prints.
pub fn execute(db: &mut Db, line: &[u8]) -> Outcome {
    let mut out = Outcome::default();
    out.code = execute_to(db, line, &mut out.stdout, &mut out.stderr);
    out
}

/// Run one unit: everything in `line`, however many commands that is.
/// Standard output is handed to `out` as it is produced and standard
/// error to `err`. Returns the exit status.
///
/// What the unit changes becomes one record of the log, so after a crash
/// all of it is there or none of it. If any part of it is unsupported,
/// none of it runs.
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
    // thrown (`disk::Damage`). It ends the unit and the use of this
    // database: what the unit had changed so far is not logged.
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let code = run_line(db, line, out, err);
        // The unit's changes become one record of the log; a file that
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

/// True if `text` is a complete unit as bash would read it from a
/// terminal or a script: no quote, substitution or here-document is left
/// open and the last line does not end in `|`, `&&`, `||` or a backslash.
pub fn is_complete(text: &[u8]) -> bool {
    if text.ends_with(b"\\") && !text.ends_with(b"\\\\") {
        return false;
    }
    match shell::parse(text) {
        Ok(script) => !script.open_here,
        Err(ParseError::Incomplete(_)) => false,
        Err(_) => true,
    }
}

fn run_line(db: &mut Db, line: &[u8], out: &mut dyn Sink, err: &mut dyn Sink) -> i32 {
    let script = match shell::parse(line) {
        Ok(s) => s,
        Err(ParseError::Unsupported(what)) => return fail(format!("unsupported syntax: {}", what), err),
        Err(ParseError::Syntax(what)) | Err(ParseError::Incomplete(what)) => return fail(what, err),
    };
    // Check everything before running anything.
    let mut ready = Vec::new();
    if let Err(what) = check_script(db, &script, Some(&mut ready)) {
        return fail(format!("unsupported {}", what), err);
    }
    for w in &script.warnings {
        err.write(format!("qdb: {}\n", w).as_bytes());
    }
    match run_script(db, &script, out, err, Some(ready)) {
        Ok(code) | Err(Stop(code)) => code,
    }
}

/// Look through a script for anything unsupported without running it. A
/// command none of whose words holds a substitution is expanded against
/// the tree as it is now and its flags are checked; of one that does,
/// only the name can be checked. The commands of the first pipeline are
/// kept in `ready` when they will run exactly as checked.
fn check_script(db: &mut Db, script: &Script, mut ready: Option<&mut Vec<Option<Prepared>>>) -> Result<(), String> {
    let mut first = true;
    for item in &script.items {
        for pipeline in std::iter::once(&item.first).chain(item.rest.iter().map(|(_, p)| p)) {
            for command in &pipeline.commands {
                let mut inner = |parts: &[Part]| -> Result<bool, String> {
                    let mut any = false;
                    for p in parts {
                        if let Part::Subst { script, .. } = p {
                            check_script(db, script, None)?;
                            any = true;
                        }
                    }
                    Ok(any)
                };
                let mut fixed = true;
                for w in &command.words {
                    fixed &= !inner(&w.parts)?;
                }
                for r in &command.redirects {
                    match r {
                        Redirect::Out { target, .. } | Redirect::Both { target, .. } | Redirect::In { target } => {
                            inner(&target.parts)?;
                        }
                        Redirect::Here { body } => {
                            inner(&body.borrow())?;
                        }
                        Redirect::Dup { .. } => {}
                    }
                }
                let mut kept = None;
                if fixed {
                    let mut argv = Vec::new();
                    for w in &command.words {
                        match expand(db, w, &mut Nowhere) {
                            Ok(a) => argv.extend(a),
                            Err(_) => unreachable!("nothing runs in a word without a substitution"),
                        }
                    }
                    if let Some((name, args)) = argv.split_first() {
                        kept = Some(cmd::prepare(name, args)?);
                    }
                } else if let Some(w) = command.words.first() {
                    if !w.has_subst() && !glob::has_glob(&w.bytes()) && !cmd::known(&w.text()) {
                        return Err(format!("command: {}", lossy(&w.text())));
                    }
                }
                if first {
                    if let Some(r) = ready.as_mut() {
                        r.push(kept);
                    }
                }
            }
            first = false;
        }
    }
    Ok(())
}

fn run_script(
    db: &mut Db,
    script: &Script,
    out: &mut dyn Sink,
    err: &mut dyn Sink,
    mut ready: Option<Vec<Option<Prepared>>>,
) -> Result<i32, Stop> {
    let mut code = 0;
    for item in &script.items {
        code = run_pipeline(db, &item.first, out, err, ready.take())?;
        for (op, pipeline) in &item.rest {
            if (*op == Connector::And) == (code == 0) {
                code = run_pipeline(db, pipeline, out, err, None)?;
            }
        }
    }
    Ok(code)
}

fn run_pipeline(
    db: &mut Db,
    pipeline: &Pipeline,
    out: &mut dyn Sink,
    err: &mut dyn Sink,
    ready: Option<Vec<Option<Prepared>>>,
) -> Result<i32, Stop> {
    let mut ready = ready.unwrap_or_default().into_iter();
    let mut stages = Vec::new();
    for (n, command) in pipeline.commands.iter().enumerate() {
        let prepared: Prepared = match ready.next().flatten() {
            Some(p) => p,
            None => {
                let mut argv = Vec::new();
                for w in &command.words {
                    argv.extend(expand(db, w, err)?);
                }
                match argv.split_first() {
                    None => {
                        let nothing: Runner = Box::new(|_, _| 0);
                        nothing.into()
                    }
                    Some((name, args)) => match cmd::prepare(name, args) {
                        Ok(p) => p,
                        Err(what) => return Err(Stop(fail(format!("unsupported {}", what), err))),
                    },
                }
            }
        };
        let mut redirs = Vec::new();
        for r in &command.redirects {
            redirs.push(expand_redirect(db, r, err)?);
        }
        stages.push(Stage::new(prepared, redirs, db.scratch(), n > 0));
    }

    // The first command drives the pipeline: what it writes is pushed
    // through the later ones as it appears. When one of them needs no more
    // input, the write is refused and the commands before it stop.
    let (first, later) = stages.split_first_mut().expect("a pipeline has a command");
    let mut code = first.end(db, &mut Rest { stages: later, last: out }, err);
    for i in 0..later.len() {
        let (stage, rest) = later[i..].split_first_mut().unwrap();
        code = stage.end(db, &mut Rest { stages: rest, last: out }, err);
    }
    Ok(code)
}

/// Run a command given as words, as `xargs` and `find -exec` do, with the
/// streams of the command that runs it.
pub fn run_argv(db: &mut Db, argv: &[Vec<u8>], io: &mut Io) -> i32 {
    let Some((name, args)) = argv.split_first() else { return 0 };
    match cmd::prepare(name, args) {
        Ok(p) => (p.run)(db, io),
        Err(what) => {
            io.err_line(format!("qdb: unsupported {}", what));
            EXIT_UNSUPPORTED
        }
    }
}

/// One command of a pipeline.
struct Stage {
    run: Option<Runner>,
    state: State,
    /// The command as a filter, when its standard input is a file it can
    /// take a piece at a time.
    stream: Option<Box<dyn Filter>>,
    redirs: Vec<Redir>,
    plan: Plan,
    /// Output on its way to a file: one for each redirection.
    spools: Vec<Spool>,
    /// Output on its way to the next stage.
    buf: Vec<u8>,
    err: Vec<u8>,
    /// Standard output on its way to standard error (`>&2`).
    to_err: Vec<u8>,
    /// The size of the file standard input comes from, once it is open.
    stdin_size: Option<u64>,
    /// Standard input is a pipe or a file.
    piped: bool,
    scratch: Scratch,
}

enum State {
    /// Taking input in pieces.
    Streaming(Box<dyn Filter>),
    /// Gathering all the input, to run on it when it ends.
    Gathering(Vec<u8>),
    /// Needs no input, or no more of it.
    Closed(Option<Box<dyn Filter>>),
}

enum Stdin {
    Bytes(Vec<u8>),
    File(NodeId),
}

impl Stage {
    fn new(prepared: Prepared, redirs: Vec<Redir>, scratch: Scratch, piped: bool) -> Stage {
        let plan = plan(&redirs);
        let mut stream = None;
        let state = match prepared.input {
            Input::Stream(f) if plan.input.is_some() || !piped => {
                if plan.input.is_some() {
                    stream = Some(f);
                }
                State::Closed(None)
            }
            _ if plan.input.is_some() || !piped => State::Closed(None),
            Input::Stream(f) => State::Streaming(f),
            Input::Whole => State::Gathering(Vec::new()),
            Input::Unused => State::Closed(None),
        };
        let spools = redirs.iter().map(|_| Spool::new(scratch.clone())).collect();
        Stage { run: Some(prepared.run), state, stream, redirs, plan, spools, buf: Vec::new(), err: Vec::new(), to_err: Vec::new(), stdin_size: None, piped: false, scratch }
    }

    /// Give `f` the streams of this command. Returns what `f` returned and
    /// whether the reader of the output wants no more.
    fn with_io<R>(&mut self, stdin: &[u8], down: &mut dyn Sink, out_file: Option<NodeId>, f: impl FnOnce(&mut Io) -> R) -> (R, bool) {
        let Stage { plan, spools, buf, err, to_err, stdin_size, piped, scratch, .. } = self;
        let mut nowhere = Nowhere;
        let out: &mut dyn Sink = match plan.d1 {
            Dest::Down => down,
            Dest::Err => to_err,
            Dest::File(i) => &mut spools[i],
            Dest::Null => &mut nowhere,
        };
        let mut io = Io { stdin, out, buf, stopped: false, err, out_file, merged: plan.d1 == plan.d2, exact: true, stdin_size: *stdin_size, piped: *piped, scratch: scratch.clone() };
        let r = f(&mut io);
        io.flush();
        (r, io.stopped)
    }

    fn push(&mut self, data: &[u8], down: &mut dyn Sink) -> bool {
        match std::mem::replace(&mut self.state, State::Closed(None)) {
            State::Closed(f) => {
                self.state = State::Closed(f);
                false
            }
            State::Gathering(mut all) => {
                all.extend_from_slice(data);
                self.state = State::Gathering(all);
                true
            }
            State::Streaming(mut filter) => {
                let (more, stopped) = self.with_io(&[], down, None, |io| filter.push(data, io));
                // If the next stage wants no more, neither does this one.
                let more = more && !stopped;
                self.state = if more { State::Streaming(filter) } else { State::Closed(Some(filter)) };
                more
            }
        }
    }

    /// The stage before has ended, or there is none: open the
    /// redirections, run the command (or finish it, if it has been taking
    /// its input in pieces), and store what went to files. Returns the
    /// exit status.
    fn end(&mut self, db: &mut Db, down: &mut dyn Sink, uerr: &mut dyn Sink) -> i32 {
        let state = std::mem::replace(&mut self.state, State::Closed(None));
        let mut undo = Vec::new();
        let (opened, stdin, failed) = open_redirects(db, &self.redirs, &mut undo);
        let mut code;
        if let Some((upto, msg)) = failed {
            // The message goes where standard error went at that point,
            // and the command does not run.
            self.plan = plan(&self.redirs[..upto]);
            self.err.extend_from_slice(msg.as_bytes());
            code = 1;
        } else {
            let out_file = match self.plan.d1 {
                Dest::File(i) => opened[i].as_ref().map(|t| t.1),
                _ => None,
            };
            let run = self.run.take();
            if let Some(Stdin::File(id)) = &stdin {
                self.stdin_size = Some(db.size(*id));
            }
            self.piped = stdin.is_some() || matches!(state, State::Gathering(_));
            code = match (state, stdin) {
                (State::Streaming(f), _) | (State::Closed(Some(f)), _) => self.with_io(&[], down, None, |io| f.finish_db(db, io)).0,
                (State::Gathering(all), None) => self.with_io(&all, down, out_file, |io| run.unwrap()(db, io)).0,
                (_, None) => self.with_io(&[], down, out_file, |io| run.unwrap()(db, io)).0,
                (_, Some(stdin)) => match (self.stream.take(), stdin) {
                    (Some(mut f), Stdin::File(id)) => {
                        self.with_io(&[], down, None, |io| {
                            for p in db.pieces(id).to_vec() {
                                if !f.push(&db.text_of(&p), io) {
                                    break;
                                }
                            }
                            f.finish_db(db, io)
                        })
                        .0
                    }
                    (Some(mut f), Stdin::Bytes(text)) => {
                        self.with_io(&[], down, None, |io| {
                            f.push(&text, io);
                            f.finish_db(db, io)
                        })
                        .0
                    }
                    (None, Stdin::File(id)) => {
                        let text = db.content(id).into_owned();
                        self.with_io(&text, down, out_file, |io| run.unwrap()(db, io)).0
                    }
                    (None, Stdin::Bytes(text)) => self.with_io(&text, down, out_file, |io| run.unwrap()(db, io)).0,
                },
            };
        }
        // Standard error, where it was not merged into the output already.
        match self.plan.d2 {
            Dest::Down => {
                down.write(&self.err);
            }
            Dest::Err => {
                uerr.write(&self.err);
            }
            Dest::File(i) => {
                self.spools[i].write(&self.err);
            }
            Dest::Null => {}
        }
        self.err.clear();
        uerr.write(&self.to_err);
        self.to_err.clear();
        // What went to files is stored now, if all of it is valid text.
        let mut bad = None;
        for (i, t) in opened.iter().enumerate() {
            if let Some(t) = t {
                if !self.spools[i].end() && bad.is_none() {
                    bad = Some(t.2.clone());
                }
            }
        }
        if let Some(path) = bad {
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
            uerr.write(format!("qdb: {}: output is not valid UTF-8 text, nothing was written\n", lossy(&path)).as_bytes());
            return 1;
        }
        let mut late = Vec::new();
        for (i, t) in opened.into_iter().enumerate() {
            if let Some(t) = t {
                let mut spool = std::mem::replace(&mut self.spools[i], Spool::nowhere());
                code = store_redirect(db, t, &mut spool, code, &mut late);
            }
        }
        uerr.write(&late);
        code
    }
}

/// The stages after the current one, as the place its output goes.
struct Rest<'a> {
    stages: &'a mut [Stage],
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

fn open_redirect(db: &mut Db, append: bool, path: &[u8], undo: &mut Vec<Undo>) -> Result<NodeId, Errno> {
    match db.resolve(path) {
        Ok(id) if db.is_dir(id) => Err(Errno::IsDir),
        Ok(id) => {
            if !append {
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

/// (true for `>>`, the file, its path as written)
type Target = (bool, NodeId, Vec<u8>);

/// Open a command's redirections left to right, as bash does before the
/// command runs: `>` creates or empties its file even if a later one
/// takes the output. Returns the file each one opened for writing, where
/// standard input comes from, and, if one could not be opened, its index
/// and the message.
fn open_redirects(db: &mut Db, redirs: &[Redir], undo: &mut Vec<Undo>) -> (Vec<Option<Target>>, Option<Stdin>, Option<(usize, String)>) {
    let mut opened: Vec<Option<Target>> = redirs.iter().map(|_| None).collect();
    let mut stdin = None;
    for (i, r) in redirs.iter().enumerate() {
        let (path, append, input) = match r {
            Redir::Out { path, append, .. } | Redir::Both { path, append } => (path, *append, false),
            Redir::In { path } => (path, false, true),
            Redir::Here(text) => {
                stdin = Some(Stdin::Bytes(text.clone()));
                continue;
            }
            Redir::Dup { .. } => continue,
        };
        let path = match path {
            Ok(p) => p,
            Err(word) => return (opened, stdin, Some((i, format!("qdb: {}: ambiguous redirect\n", lossy(word))))),
        };
        let result = if input {
            match db.resolve(path) {
                Ok(id) if db.is_dir(id) => Err(Errno::IsDir),
                Ok(id) => {
                    stdin = Some(Stdin::File(id));
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else if path == DEV_NULL {
            Ok(())
        } else {
            open_redirect(db, append, path, undo).map(|id| opened[i] = Some((append, id, path.clone())))
        };
        if let Err(e) = result {
            return (opened, stdin, Some((i, format!("qdb: {}: {}\n", lossy(path), e.msg()))));
        }
    }
    (opened, stdin, None)
}

/// Store redirected output, already found to be valid text, in its file.
/// Store what a command gathered for a file of its own, as tee does.
/// The output must have ended and been found to be valid text.
pub fn store_output(db: &mut Db, id: NodeId, append: bool, path: &[u8], out: &mut Spool, stderr: &mut Vec<u8>) -> i32 {
    store_redirect(db, (append, id, path.to_vec()), out, 0, stderr)
}

fn store_redirect(db: &mut Db, target: Target, out: &mut Spool, mut code: i32, stderr: &mut Vec<u8>) -> i32 {
    let (append, id, path) = target;
    if let Some(e) = out.error.take() {
        // Reported when the command line ends, like any failure to store.
        db.scratch_failed(e);
        return code;
    }
    // The command may have removed or replaced the file it was writing to;
    // the output then goes nowhere, as it would with an unlinked file.
    if db.exists(id) && !db.is_dir(id) && out.len > 0 {
        // How much of the output is handed to storage at a time: 4 MB
        // with pieces of the usual size.
        let step = db.config().piece * 64;
        if append || db.size(id) == 0 {
            // Valid text added to valid text is valid text.
            let mut at = 0;
            let mut first = !append;
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
