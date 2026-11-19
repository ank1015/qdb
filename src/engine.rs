//! Runs one command line against the database.
//!
//! One call is one unit: it is parsed whole, every command in it is
//! checked for unsupported flags, and only then does anything run. A unit
//! that uses something unsupported therefore fails before it changes any
//! data, and what a unit changes is logged as one record.

use crate::cmd;
use crate::glob;
use crate::session::{self, Session};
use crate::shell::{self, Command, CompoundKind, Connector, ParamOp, ParseError, Part, Pipeline, Redirect, Script, SimpleCommand, Word};
use std::cell::{Cell, RefCell};
use crate::tree::{is_valid_text, Cost, Db, Errno, NodeId, Scratch, TextCheck};
use crate::vfs::VFile;
use crate::util::lossy;

/// What a command line produced. Two outcomes are equal when output,
/// errors and status are: what the unit used is not part of the answer.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: i32,
    /// What the unit used.
    pub cost: Cost,
}

impl PartialEq for Outcome {
    fn eq(&self, o: &Outcome) -> bool {
        self.stdout == o.stdout && self.stderr == o.stderr && self.code == o.code
    }
}

impl Eq for Outcome {}

/// Exit status for anything unsupported or unparsable.
pub const EXIT_UNSUPPORTED: i32 = 2;

/// Exit status of a unit that was stopped because it ran more commands
/// than a unit may.
pub const EXIT_LIMIT: i32 = 3;

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
    /// Whatever arrives is kept: this is input held for a command, not
    /// output for a file.
    raw: bool,
    error: Option<std::io::Error>,
}

/// With a scratch file, this much is gathered between writes to it.
const SPOOL_WRITE: usize = 1 << 20;

static SPOOLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl Spool {
    pub fn new(scratch: Scratch) -> Spool {
        Spool { scratch, mem: Vec::new(), file: None, len: 0, check: TextCheck::default(), raw: false, error: None }
    }

    /// A place for all of a command's standard input to wait until the
    /// command runs: in memory up to the scratch limit, in a scratch file
    /// beyond it.
    pub fn held(scratch: Scratch) -> Spool {
        let mut s = Spool::new(scratch);
        s.raw = true;
        s
    }

    /// Everything held, once `end` has been called.
    pub fn all(&self) -> &[u8] {
        self.read(0, usize::MAX)
    }

    fn nowhere() -> Spool {
        Spool::new(Scratch::none())
    }

    fn to_file(&mut self) {
        if self.file.is_none() {
            let vfs = self.scratch.vfs.as_ref().expect("a place for scratch files");
            let name = format!("spool-{}.tmp", SPOOLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
            match vfs.create(&name, 1 << 38) {
                Ok(f) => self.file = Some((name, f)),
                Err(e) if self.raw => {
                    // Held input stays in memory if it cannot go to disk.
                    let _ = e;
                    self.scratch.vfs = None;
                    return;
                }
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            }
        }
        let (_, f) = self.file.as_mut().unwrap();
        crate::tree::count_scratch(self.mem.len() as u64);
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
        self.raw || self.check.finish()
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
        if !self.raw && (!self.check.feed(data) || self.error.is_some()) {
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
        self.err_bytes(msg.as_ref());
        self.err.push(b'\n');
    }

    /// Write to standard error.
    fn err_bytes(&mut self, msg: &[u8]) {
        if self.merged && self.exact {
            self.flush();
        }
        self.err.extend_from_slice(msg);
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

/// Why a list stopped before its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// The unit cannot go on: something unsupported was reached after it
    /// began, or it ran more commands than a unit may.
    Stop(i32),
    /// An expansion failed. bash gives up the rest of the line it is on
    /// and goes on with the next.
    Line(i32),
    /// `break N`
    Break(u32),
    /// `continue N`
    Continue(u32),
    /// Whoever read the output of this stage of a pipeline needs no more
    /// of it: the stage ends, as a process ends that writes to a closed
    /// pipe.
    Gone,
}

impl Flow {
    fn code(self) -> i32 {
        match self {
            Flow::Stop(c) | Flow::Line(c) => c,
            Flow::Gone => 141,
            Flow::Break(_) | Flow::Continue(_) => 0,
        }
    }
}

/// What all the commands of a unit share while it runs.
struct Ctx {
    /// A unit may run this many commands.
    limit: u64,
    ran: Cell<u64>,
    /// How many loops are running around the command.
    loops: Cell<u32>,
    /// Set by a command that ends more than itself: `break`, `continue`,
    /// or something unsupported that was only seen when it was reached.
    flow: Cell<Option<Flow>>,
    /// The status of the last command substitution in the words of the
    /// command being put together: the status of a command that is
    /// nothing but assignments.
    subst: Cell<Option<i32>>,
}

impl Ctx {
    fn new(limit: u64) -> Ctx {
        Ctx { limit, ran: Cell::new(0), loops: Cell::new(0), flow: Cell::new(None), subst: Cell::new(None) }
    }

    /// One more command is about to run.
    fn count(&self, err: &mut dyn Sink) -> Result<(), Flow> {
        let n = self.ran.get() + 1;
        self.ran.set(n);
        if n > self.limit {
            err.write(format!("qdb: the unit was stopped after {} commands, the most one unit may run; what it changed before that stays\n", self.limit).as_bytes());
            return Err(Flow::Stop(EXIT_LIMIT));
        }
        Ok(())
    }
}

/// Where the commands of a list read and write.
struct Run<'r, 's> {
    ctx: &'r Ctx,
    out: &'r mut dyn Sink,
    err: &'r mut dyn Sink,
    /// What is left of the standard input of the loop or `if` the list
    /// is in: `read` takes a line of it, any other command that reads
    /// its standard input takes the rest. `None` outside one.
    stdin: &'r mut Option<&'s [u8]>,
    /// Set when the reader of `out` wants no more.
    gone: &'r Cell<bool>,
    /// `err` goes where `out` goes (`2>&1` on the construct the list is
    /// in), so what the commands write to the two keeps its order.
    merged: bool,
}

/// The commands that act on the session, and so are not among the
/// commands of `cmd`: `xargs` and `find -exec` cannot run them.
const BUILTINS: &[&str] = &["cd", "export", "unset", "read", "break", "continue"];

/// True for a command only the shell itself can run.
pub fn is_builtin(name: &[u8]) -> bool {
    BUILTINS.iter().any(|b| b.as_bytes() == name)
}

/// Check the arguments of a builtin. The error names what is unsupported.
fn check_builtin(name: &[u8], args: &[Vec<u8>]) -> Result<(), String> {
    let flag = |a: &&Vec<u8>| a.len() > 1 && a[0] == b'-';
    match name {
        b"cd" => {
            if let Some(a) = args.iter().find(flag) {
                return Err(format!("flag: cd {}", lossy(a)));
            }
        }
        b"export" => {
            if let Some(a) = args.iter().find(flag) {
                return Err(format!("flag: export {}", lossy(a)));
            }
            if args.is_empty() {
                return Err("export without a name (it would list the exported variables)".into());
            }
        }
        b"unset" => {
            if let Some(a) = args.iter().find(|a| flag(a) && a.as_slice() != b"-v") {
                return Err(format!("flag: unset {}", lossy(a)));
            }
        }
        b"read" => {
            if let Some(a) = args.iter().find(|a| flag(a) && a.as_slice() != b"-r") {
                return Err(format!("flag: read {}", lossy(a)));
            }
        }
        _ => {}
    }
    Ok(())
}

/// A word being split into fields.
#[derive(Default)]
struct Fields {
    out: Vec<Vec<(u8, bool)>>,
    cur: Vec<(u8, bool)>,
    /// There is a field under way, even if it is empty so far.
    open: bool,
    /// The last field was ended by blanks, and nothing came since: a
    /// separator that is not a blank belongs to the same gap.
    after_blank: bool,
}

impl Fields {
    fn lit(&mut self, text: &[u8]) {
        if !text.is_empty() {
            self.cur.extend(text.iter().map(|c| (*c, false)));
            self.open = true;
            self.after_blank = false;
        }
    }

    fn quoted(&mut self, text: &[u8]) {
        self.cur.extend(text.iter().map(|c| (*c, true)));
        self.open = true;
        self.after_blank = false;
    }

    /// The result of an unquoted expansion: split on the characters of
    /// `ifs`. Runs of blanks among them are one separator and none at
    /// the ends; every other one separates by itself.
    fn split(&mut self, text: &[u8], ifs: &[u8]) {
        for &c in text {
            if !ifs.contains(&c) {
                // A backslash from an expansion is a plain character.
                self.cur.push((c, c == b'\\'));
                self.open = true;
                self.after_blank = false;
            } else if matches!(c, b' ' | b'\t' | b'\n') {
                if self.open {
                    self.out.push(std::mem::take(&mut self.cur));
                    self.open = false;
                    self.after_blank = true;
                }
            } else if self.after_blank {
                self.after_blank = false;
            } else {
                self.out.push(std::mem::take(&mut self.cur));
                self.open = false;
            }
        }
    }
}

/// What expansion needs: the tree for globs and for the commands of a
/// substitution, and the session for everything with a dollar.
struct Ex<'a> {
    db: &'a mut Db,
    sess: &'a mut Session,
    ctx: &'a Ctx,
    err: &'a mut dyn Sink,
}

impl Ex<'_> {
    /// Expand one word into arguments, in bash's order: braces, tilde,
    /// parameters with arithmetic and command substitution, field
    /// splitting, globs, then quote removal.
    fn expand(&mut self, word: &Word) -> Result<Vec<Vec<u8>>, Flow> {
        // The usual word: nothing to expand.
        if let [Part::Lit(b)] = word.parts.as_slice() {
            if !b.contains(&b'{') && !glob::has_glob_bytes(b) {
                return Ok(vec![b.clone()]);
            }
        }
        let mut out = Vec::new();
        for w in shell::brace_expand(word) {
            let mut f = Fields::default();
            self.parts(&w.parts, &mut f, false)?;
            if f.open {
                f.out.push(f.cur);
            }
            for field in f.out {
                if glob::has_glob(&field) {
                    let found = glob::expand_path(self.db, &field);
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

    fn put(&mut self, f: &mut Fields, text: &[u8], quoted: bool) {
        if quoted {
            f.quoted(text);
        } else {
            f.split(text, self.sess.ifs());
        }
    }

    /// Add the parts of a word to its fields. `value` is true for the
    /// word of `${NAME:-word}` outside quotes: its plain text is split
    /// like the value it stands in for.
    fn parts(&mut self, parts: &[Part], f: &mut Fields, value: bool) -> Result<(), Flow> {
        for p in parts {
            match p {
                Part::Lit(b) if value => f.split(b, self.sess.ifs()),
                Part::Lit(b) => f.lit(b),
                Part::Quoted(b) => f.quoted(b),
                Part::Tilde => {
                    let home = self.sess.get(b"HOME").unwrap_or(b"/").to_vec();
                    f.quoted(&home);
                }
                Part::Subst { script, quoted } => {
                    let text = self.substitute(script)?;
                    self.put(f, &text, *quoted);
                }
                Part::Arith { expr, quoted } => {
                    let text = self.arith(expr)?;
                    self.put(f, &text, *quoted);
                }
                Part::Param { name, op, quoted } => match op {
                    ParamOp::Value | ParamOp::Length => {
                        let text = self.param(name, op);
                        self.put(f, &text, *quoted);
                    }
                    ParamOp::Default(word) | ParamOp::Assign(word) => {
                        let now = self.param(name, &ParamOp::Value);
                        if !now.is_empty() {
                            self.put(f, &now, *quoted);
                        } else if matches!(op, ParamOp::Assign(_)) {
                            let text = self.value(&word.parts)?;
                            self.sess.set(name, text.clone());
                            self.put(f, &text, *quoted);
                        } else if *quoted {
                            let text = self.value(&word.parts)?;
                            f.quoted(&text);
                        } else {
                            self.parts(&word.parts, f, true)?;
                        }
                    }
                },
            }
        }
        Ok(())
    }

    /// The value of `$NAME` or `${#NAME}`. An unset variable is empty.
    fn param(&self, name: &[u8], op: &ParamOp) -> Vec<u8> {
        let value = if name == b"?" { self.sess.status.to_string().into_bytes() } else { self.sess.get(name).unwrap_or(b"").to_vec() };
        match op {
            ParamOp::Length => value.len().to_string().into_bytes(),
            _ => value,
        }
    }

    /// Parts as one string, with no splitting into fields and no globs:
    /// the value of an assignment, the text of a here-document, the
    /// expression of `$((...))`.
    fn value(&mut self, parts: &[Part]) -> Result<Vec<u8>, Flow> {
        let mut out = Vec::new();
        for p in parts {
            match p {
                Part::Lit(b) | Part::Quoted(b) => out.extend_from_slice(b),
                Part::Tilde => out.extend_from_slice(self.sess.get(b"HOME").unwrap_or(b"/")),
                Part::Subst { script, .. } => out.extend(self.substitute(script)?),
                Part::Arith { expr, .. } => out.extend(self.arith(expr)?),
                Part::Param { name, op, .. } => match op {
                    ParamOp::Value | ParamOp::Length => out.extend(self.param(name, op)),
                    ParamOp::Default(word) | ParamOp::Assign(word) => {
                        let now = self.param(name, &ParamOp::Value);
                        if !now.is_empty() {
                            out.extend(now);
                        } else {
                            let text = self.value(&word.parts)?;
                            if matches!(op, ParamOp::Assign(_)) {
                                self.sess.set(name, text.clone());
                            }
                            out.extend(text);
                        }
                    }
                },
            }
        }
        Ok(out)
    }

    fn arith(&mut self, expr: &[Part]) -> Result<Vec<u8>, Flow> {
        let text = self.value(expr)?;
        match session::arith(&text, self.sess) {
            Ok(v) => Ok(v.to_string().into_bytes()),
            Err(msg) => {
                self.err.write(format!("qdb: {}\n", msg).as_bytes());
                Err(Flow::Line(1))
            }
        }
    }

    /// Run a command substitution: its standard output without the
    /// newlines at its end. It runs on a copy of the session: what it
    /// sets, and where it goes, is gone when it ends.
    fn substitute(&mut self, script: &Script) -> Result<Vec<u8>, Flow> {
        let mut text = Vec::new();
        let mut copy = self.sess.clone();
        let mut none = None;
        let gone = Cell::new(false);
        let result = run_list(self.db, &mut copy, &script.items, &mut Run { ctx: self.ctx, out: &mut text, err: &mut *self.err, stdin: &mut none, gone: &gone, merged: false });
        self.db.set_cwd(self.sess.cwd());
        let code = match result {
            Ok(code) => code,
            Err(Flow::Stop(code)) => return Err(Flow::Stop(code)),
            Err(other) => other.code(),
        };
        self.ctx.subst.set(Some(code));
        // `$?` is the substitution's status from here on, in the same
        // command too.
        self.sess.status = code;
        while text.last() == Some(&b'\n') {
            text.pop();
        }
        Ok(text)
    }

    fn redirect(&mut self, r: &Redirect) -> Result<Redir, Flow> {
        let one = |ex: &mut Ex, target: &Word| -> Result<Result<Vec<u8>, Vec<u8>>, Flow> {
            let mut targets = ex.expand(target)?;
            Ok(if targets.len() == 1 { Ok(targets.pop().unwrap()) } else { Err(target.raw.clone()) })
        };
        Ok(match r {
            Redirect::Out { fd, append, target } => Redir::Out { fd: *fd, append: *append, path: one(self, target)? },
            Redirect::Both { append, target } => Redir::Both { append: *append, path: one(self, target)? },
            Redirect::Dup { fd, to } => Redir::Dup { fd: *fd, to: *to },
            Redirect::In { target } => Redir::In { path: one(self, target)? },
            Redirect::Here { body } => {
                let body = body.borrow().clone();
                Redir::Here(self.value(&body)?)
            }
        })
    }
}

/// Expand a word in which nothing depends on the session: braces, globs
/// and quote removal. Used to check a command before anything runs.
fn expand_static(db: &Db, word: &Word) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for w in shell::brace_expand(word) {
        // Nothing at all, not even empty quotes, is no argument.
        if w.parts.iter().all(|p| matches!(p, Part::Lit(b) if b.is_empty())) {
            continue;
        }
        let field = w.bytes();
        if glob::has_glob(&field) {
            let found = glob::expand_path(db, &field);
            if !found.is_empty() {
                out.extend(found);
                continue;
            }
        }
        out.push(field.into_iter().map(|(c, _)| c).collect());
    }
    out
}

/// Run a command line in a session of its own and collect what it
/// prints. Nothing is kept from one call to the next: for variables and
/// a current directory that last, keep a `Session` and use `execute_in`.
pub fn execute(db: &mut Db, line: &[u8]) -> Outcome {
    execute_in(db, &mut Session::new(), line)
}

/// Run a command line in the caller's session and collect what it prints.
pub fn execute_in(db: &mut Db, sess: &mut Session, line: &[u8]) -> Outcome {
    let mut out = Outcome::default();
    out.code = execute_to_in(db, sess, line, &mut out.stdout, &mut out.stderr);
    out.cost = db.last_cost();
    out
}

/// Run one unit: everything in `line`, however many commands that is.
/// Standard output is handed to `out` as it is produced and standard
/// error to `err`. Returns the exit status.
///
/// What the unit changes becomes one record of the log, so after a crash
/// all of it is there or none of it. If anything its text shows is
/// unsupported, none of it runs.
///
/// Nothing is held back beyond a small buffer, so a command that can
/// stream (`cat`, `grep`, `find`, ...) uses the same memory whatever the
/// size of its output. If `out` returns false the commands that only read
/// stop early; commands that change data always run to completion.
pub fn execute_to(db: &mut Db, line: &[u8], out: &mut dyn Sink, err: &mut dyn Sink) -> i32 {
    execute_to_in(db, &mut Session::new(), line, out, err)
}

/// As `execute_to`, in the caller's session: the unit sees its variables
/// and its current directory, and what it does to them stays.
pub fn execute_to_in(db: &mut Db, sess: &mut Session, line: &[u8], out: &mut dyn Sink, err: &mut dyn Sink) -> i32 {
    db.begin_unit();
    db.set_cwd(sess.cwd());
    let code = execute_unit(db, sess, line, out, err);
    db.set_cwd(b"/");
    db.end_unit();
    sess.status = code;
    code
}

fn execute_unit(db: &mut Db, sess: &mut Session, line: &[u8], out: &mut dyn Sink, err: &mut dyn Sink) -> i32 {
    if let Some(e) = db.failure() {
        err.write(format!("qdb: {}\n", e).as_bytes());
        return 2;
    }
    // Reads cannot return an error, so damage found while reading is
    // thrown (`disk::Damage`). It ends the unit and the use of this
    // database: what the unit had changed so far is not logged.
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let code = run_line(db, sess, line, out, err);
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
/// terminal or a script: no quote, substitution, here-document, loop or
/// `if` is left open and the last line does not end in `|`, `&&`, `||`
/// or a backslash.
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

fn run_line(db: &mut Db, sess: &mut Session, line: &[u8], out: &mut dyn Sink, err: &mut dyn Sink) -> i32 {
    let script = match shell::parse(line) {
        Ok(s) => s,
        Err(ParseError::Unsupported(what)) => return fail(format!("unsupported syntax: {}", what), err),
        Err(ParseError::Syntax(what)) | Err(ParseError::Incomplete(what)) => return fail(what, err),
    };
    // Check everything the text shows before running anything.
    let mut ready = Vec::new();
    if let Err(what) = check_list(db, &script.items, Some(&mut ready)) {
        return fail(format!("unsupported {}", what), err);
    }
    for w in &script.warnings {
        err.write(format!("qdb: {}\n", w).as_bytes());
    }
    let ctx = Ctx::new(db.config().command_limit);
    let mut none = None;
    let gone = Cell::new(false);
    let mut r = Run { ctx: &ctx, out, err, stdin: &mut none, gone: &gone, merged: false };
    // An expansion that fails gives up the rest of its line; the unit
    // goes on with the next line.
    let mut code = 0;
    let mut skipping = false;
    let mut ready = Some(ready);
    for item in &script.items {
        if skipping {
            skipping = !item.ends_line;
            continue;
        }
        match run_and_or(db, sess, item, &mut r, ready.take()) {
            Ok(c) => code = c,
            Err(Flow::Stop(c)) => return c,
            Err(flow) => {
                code = flow.code();
                sess.status = code;
                skipping = matches!(flow, Flow::Line(_)) && !item.ends_line;
            }
        }
    }
    code
}

/// Look through a list for anything unsupported without running it:
/// syntax has been checked by the parser; this checks the names of
/// commands and the flags that are written out. A command none of whose
/// words holds an expansion is expanded against the tree as it is now
/// and its flags are checked; of one that does, only the name can be
/// checked, and the rest when it is reached. The commands of the first
/// pipeline are kept in `ready` when they will run exactly as checked.
fn check_list(db: &mut Db, items: &[shell::AndOr], mut ready: Option<&mut Vec<Option<Prepared>>>) -> Result<(), String> {
    let mut first = true;
    for item in items {
        for pipeline in item.pipelines() {
            let mut kept = Vec::new();
            for command in &pipeline.commands {
                for r in command.redirects() {
                    match r {
                        Redirect::Out { target, .. } | Redirect::Both { target, .. } | Redirect::In { target } => {
                            check_parts(db, &target.parts)?;
                        }
                        Redirect::Here { body } => {
                            check_parts(db, &body.borrow())?;
                        }
                        Redirect::Dup { .. } => {}
                    }
                }
                match command {
                    Command::Simple(c) => kept.push(check_simple(db, c)?),
                    Command::Compound(c) => {
                        kept.push(None);
                        match &c.kind {
                            CompoundKind::For { words, body, .. } => {
                                for w in words {
                                    check_parts(db, &w.parts)?;
                                }
                                check_list(db, body, None)?;
                            }
                            CompoundKind::If { arms, otherwise } => {
                                for (cond, body) in arms {
                                    check_list(db, cond, None)?;
                                    check_list(db, body, None)?;
                                }
                                if let Some(body) = otherwise {
                                    check_list(db, body, None)?;
                                }
                            }
                            CompoundKind::While { cond, body, .. } => {
                                check_list(db, cond, None)?;
                                check_list(db, body, None)?;
                            }
                        }
                    }
                }
            }
            if first {
                if let Some(r) = ready.as_mut() {
                    **r = kept;
                }
            }
            first = false;
        }
    }
    Ok(())
}

/// Check what is inside the parts of a word. Returns true if the word's
/// value is only known when it runs.
fn check_parts(db: &mut Db, parts: &[Part]) -> Result<bool, String> {
    let mut any = false;
    for p in parts {
        match p {
            Part::Lit(_) | Part::Quoted(_) => continue,
            Part::Subst { script, .. } => check_list(db, &script.items, None)?,
            Part::Arith { expr, .. } => {
                check_parts(db, expr)?;
            }
            Part::Param { op: ParamOp::Default(w) | ParamOp::Assign(w), .. } => {
                check_parts(db, &w.parts)?;
            }
            Part::Param { .. } | Part::Tilde => {}
        }
        any = true;
    }
    Ok(any)
}

fn check_simple(db: &mut Db, c: &SimpleCommand) -> Result<Option<Prepared>, String> {
    let mut fixed = true;
    for w in &c.words {
        fixed &= !check_parts(db, &w.parts)?;
    }
    for a in &c.assigns {
        check_parts(db, &a.value.parts)?;
    }
    if fixed {
        let mut argv = Vec::new();
        for w in &c.words {
            argv.extend(expand_static(db, w));
        }
        if let Some((name, args)) = argv.split_first() {
            if is_builtin(name) {
                check_builtin(name, args)?;
                return Ok(None);
            }
            let prepared = cmd::prepare(name, args)?;
            return Ok(c.assigns.is_empty().then_some(prepared));
        }
    } else if let Some(w) = c.words.first() {
        if !w.has_subst() && !glob::has_glob(&w.bytes()) {
            let name = w.text();
            if !cmd::known(&name) && !is_builtin(&name) {
                return Err(format!("command: {}", lossy(&name)));
            }
        }
    }
    Ok(None)
}

fn run_list(db: &mut Db, sess: &mut Session, items: &[shell::AndOr], r: &mut Run) -> Result<i32, Flow> {
    let mut code = 0;
    for item in items {
        code = run_and_or(db, sess, item, r, None)?;
        if r.gone.get() {
            return Err(Flow::Gone);
        }
    }
    Ok(code)
}

fn run_and_or(db: &mut Db, sess: &mut Session, item: &shell::AndOr, r: &mut Run, ready: Option<Vec<Option<Prepared>>>) -> Result<i32, Flow> {
    let mut code = run_pipeline(db, sess, &item.first, r, ready)?;
    sess.status = code;
    for (op, pipeline) in &item.rest {
        if (*op == Connector::And) == (code == 0) {
            code = run_pipeline(db, sess, pipeline, r, None)?;
            sess.status = code;
        }
    }
    Ok(code)
}

/// What one stage of a pipeline runs.
struct Work<'a> {
    run: Box<dyn FnOnce(&mut Db, &mut Io) -> i32 + 'a>,
    input: Input,
}

/// A variable as it was before `NAME=value command` set it.
type Before = (Vec<u8>, Option<Vec<u8>>);

fn run_pipeline(db: &mut Db, sess: &mut Session, pipeline: &Pipeline, r: &mut Run, ready: Option<Vec<Option<Prepared>>>) -> Result<i32, Flow> {
    let code = run_pipeline_plain(db, sess, pipeline, r, ready)?;
    Ok(if pipeline.negated { (code == 0) as i32 } else { code })
}

fn run_pipeline_plain(db: &mut Db, sess: &mut Session, pipeline: &Pipeline, r: &mut Run, ready: Option<Vec<Option<Prepared>>>) -> Result<i32, Flow> {
    db.set_cwd(sess.cwd());
    let n = pipeline.commands.len();
    // A loop or an `if` that stands alone, with no redirection, runs
    // right here, in this session and on these streams.
    if let [Command::Compound(c)] = pipeline.commands.as_slice() {
        if c.redirects.is_empty() {
            return run_compound(db, sess, &c.kind, r);
        }
    }
    let ctx = r.ctx;
    let mut ready = ready.unwrap_or_default().into_iter();
    let mut before: Vec<Before> = Vec::new();
    // Each stage of a pipeline of several gets its own copy of the
    // variables and the current directory.
    let here = sess.cwd().to_vec();
    let mut copies: Vec<Session> = if n > 1 { (0..n).map(|_| sess.clone()).collect() } else { Vec::new() };
    let (code, flow) = {
        let sessions: Vec<&mut Session> = if n > 1 { copies.iter_mut().collect() } else { vec![&mut *sess] };
        let mut stages: Vec<Stage> = Vec::with_capacity(n);
        for (i, (command, s)) in pipeline.commands.iter().zip(sessions).enumerate() {
            let (work, redirs) = match command {
                Command::Simple(c) => prepare_simple(db, s, ctx, c, &mut *r.err, ready.next().flatten(), &mut before)?,
                Command::Compound(c) => {
                    ready.next();
                    let mut redirs = Vec::new();
                    let mut ex = Ex { db: &mut *db, sess: &mut *s, ctx, err: &mut *r.err };
                    for redirect in &c.redirects {
                        redirs.push(ex.redirect(redirect)?);
                    }
                    let kind = &c.kind;
                    // As a stage of a pipeline of several it is a shell of
                    // its own in bash, which counts loops from none:
                    // `continue 2` in its one loop is `continue`.
                    let own_shell = n > 1;
                    let run: Box<dyn FnOnce(&mut Db, &mut Io) -> i32 + '_> = Box::new(move |db, io| {
                        let around = ctx.loops.get();
                        if own_shell {
                            ctx.loops.set(0);
                        }
                        let code = compound_in_io(db, s, ctx, kind, io);
                        ctx.loops.set(around);
                        code
                    });
                    (Work { run, input: Input::Whole }, redirs)
                }
            };
            let mut stage = Stage::new(work, redirs, db.scratch(), i > 0);
            stage.merge_down = r.merged && i == n - 1;
            stages.push(stage);
        }

        // The first command drives the pipeline: what it writes is pushed
        // through the later ones as it appears. When one of them needs no
        // more input, the write is refused and the commands before it stop.
        let (first, later) = stages.split_first_mut().expect("a pipeline has a command");
        let inherited = *r.stdin;
        let mut code = first.end(db, &mut Rest { stages: later, last: &mut *r.out }, &mut *r.err, inherited);
        if let (Some(rest), Some(left)) = (r.stdin.as_mut(), first.inherited_left) {
            *rest = &rest[rest.len() - left.min(rest.len())..];
        }
        let mut flow = ctx.flow.take();
        for i in 0..later.len() {
            db.set_cwd(&here);
            let (stage, rest) = later[i..].split_first_mut().unwrap();
            code = stage.end(db, &mut Rest { stages: rest, last: &mut *r.out }, &mut *r.err, None);
            // Only the end of the unit reaches out of a stage.
            flow = match (flow, ctx.flow.take()) {
                (Some(Flow::Stop(c)), _) | (_, Some(Flow::Stop(c))) => Some(Flow::Stop(c)),
                _ => None,
            };
        }
        if n > 1 && !matches!(flow, Some(Flow::Stop(_))) {
            flow = None;
        }
        (code, flow)
    };
    for (name, old) in before.into_iter().rev() {
        match old {
            Some(v) => sess.set(&name, v),
            None => sess.unset(&name),
        }
    }
    db.set_cwd(sess.cwd());
    // A reader that wants no more ends the loop when the shell itself
    // was writing: a builtin of bash's, or a construct. A program bash
    // would start is ended alone, and the loop goes on to its next
    // command, as it does in bash.
    if r.gone.get() {
        if let Some(Command::Simple(c)) = pipeline.commands.last() {
            let name = c.words.first().map(|w| w.text()).unwrap_or_default();
            if !matches!(&name[..], b"echo" | b"printf" | b"pwd") {
                r.gone.set(false);
            }
        }
    }
    match flow {
        Some(flow) => {
            sess.status = code;
            Err(flow)
        }
        None => Ok(code),
    }
}

/// Put a simple command together: expand its words, its assignments and
/// its redirections, and find what it runs.
fn prepare_simple<'a>(
    db: &mut Db,
    sess: &'a mut Session,
    ctx: &'a Ctx,
    c: &SimpleCommand,
    err: &mut dyn Sink,
    ready: Option<Prepared>,
    before: &mut Vec<Before>,
) -> Result<(Work<'a>, Vec<Redir>), Flow> {
    ctx.count(err)?;
    ctx.subst.set(None);
    let mut ex = Ex { db, sess: &mut *sess, ctx, err: &mut *err };
    if let Some(p) = ready {
        let mut redirs = Vec::new();
        for r in &c.redirects {
            redirs.push(ex.redirect(r)?);
        }
        return Ok((Work { run: p.run, input: p.input }, redirs));
    }
    let export = matches!(c.words.first().map(|w| w.parts.as_slice()), Some([Part::Lit(b)]) if b == b"export");
    let mut argv = Vec::new();
    for (i, w) in c.words.iter().enumerate() {
        // The value of `export NAME=value` is an assignment's: one
        // string, whatever is in it.
        let assigns = export && i > 0 && matches!(w.parts.first(), Some(Part::Lit(b)) if b.contains(&b'='));
        if assigns {
            argv.push(ex.value(&w.parts)?);
        } else {
            argv.extend(ex.expand(w)?);
        }
    }
    let mut redirs = Vec::new();
    for r in &c.redirects {
        redirs.push(ex.redirect(r)?);
    }
    // Assignments are made from left to right, each one seeing the ones
    // before it. Alone they stay in the session; before a command they
    // last as long as it runs, which matters to the builtins only: no
    // other command reads a variable.
    let mark = before.len();
    for a in &c.assigns {
        let value = ex.value(&a.value.parts)?;
        before.push((a.name.clone(), ex.sess.get(&a.name).map(|v| v.to_vec())));
        ex.sess.set(&a.name, value);
    }
    let Some((name, args)) = argv.split_first() else {
        before.truncate(mark);
        let code = ctx.subst.get().unwrap_or(0);
        return Ok((Work { run: Box::new(move |_, _| code), input: Input::Unused }, redirs));
    };
    if is_builtin(name) {
        if let Err(what) = check_builtin(name, args) {
            return Err(Flow::Stop(fail(format!("unsupported {}", what), err)));
        }
        let input = if name == b"read" { Input::Whole } else { Input::Unused };
        let (name, args) = (name.clone(), args.to_vec());
        return Ok((Work { run: Box::new(move |db, io| builtin(db, sess, ctx, &name, &args, io)), input }, redirs));
    }
    for (name, old) in before.drain(mark..).rev() {
        match old {
            Some(v) => sess.set(&name, v),
            None => sess.unset(&name),
        }
    }
    match cmd::prepare(name, args) {
        Ok(p) => Ok((Work { run: p.run, input: p.input }, redirs)),
        Err(what) => Err(Flow::Stop(fail(format!("unsupported {}", what), err))),
    }
}

/// Standard output of the commands inside a loop or an `if` that is a
/// stage of a pipeline or has redirections: it goes where the output of
/// the whole construct goes.
struct IoOut<'x, 'y, 'z> {
    io: &'x RefCell<&'y mut Io<'z>>,
    gone: &'x Cell<bool>,
}

impl Sink for IoOut<'_, '_, '_> {
    fn write(&mut self, data: &[u8]) -> bool {
        let mut io = self.io.borrow_mut();
        // Handed on at once, as bash writes: a reader that has had
        // enough is noticed at the next write.
        io.put(data);
        io.flush();
        if io.stopped() {
            self.gone.set(true);
        }
        !io.stopped()
    }
}

/// And their standard error.
struct IoErr<'x, 'y, 'z> {
    io: &'x RefCell<&'y mut Io<'z>>,
}

impl Sink for IoErr<'_, '_, '_> {
    fn write(&mut self, data: &[u8]) -> bool {
        self.io.borrow_mut().err_bytes(data);
        true
    }
}

/// Run a loop or an `if` with the streams of a stage: its standard input
/// is what `read` and the commands inside take from.
fn compound_in_io(db: &mut Db, sess: &mut Session, ctx: &Ctx, kind: &CompoundKind, io: &mut Io) -> i32 {
    let mut stdin = if io.has_stdin() { Some(io.take_stdin()) } else { None };
    let gone = Cell::new(false);
    let merged = io.merged;
    let cell = RefCell::new(io);
    let result = {
        let mut out = IoOut { io: &cell, gone: &gone };
        let mut err = IoErr { io: &cell };
        run_compound(db, sess, kind, &mut Run { ctx, out: &mut out, err: &mut err, stdin: &mut stdin, gone: &gone, merged })
    };
    cell.into_inner().stdin = stdin.unwrap_or(&[]);
    match result {
        Ok(code) => code,
        Err(Flow::Gone) => Flow::Gone.code(),
        Err(flow) => {
            ctx.flow.set(Some(flow));
            match flow {
                Flow::Break(_) | Flow::Continue(_) => sess.status,
                other => other.code(),
            }
        }
    }
}

/// What a loop does after one run of its body.
enum Step {
    Next(i32),
    Leave(i32),
}

fn step(result: Result<i32, Flow>, ctx: &Ctx, sess: &Session) -> Result<Step, Flow> {
    // `ctx.loops` counts this loop: above 1 there is one around it.
    match result {
        Ok(code) => Ok(Step::Next(code)),
        Err(Flow::Break(n)) if n > 1 && ctx.loops.get() > 1 => Err(Flow::Break(n - 1)),
        Err(Flow::Continue(n)) if n > 1 && ctx.loops.get() > 1 => Err(Flow::Continue(n - 1)),
        Err(Flow::Break(_)) => Ok(Step::Leave(sess.status)),
        Err(Flow::Continue(_)) => Ok(Step::Next(sess.status)),
        Err(other) => Err(other),
    }
}

fn run_compound(db: &mut Db, sess: &mut Session, kind: &CompoundKind, r: &mut Run) -> Result<i32, Flow> {
    let ctx = r.ctx;
    match kind {
        CompoundKind::If { arms, otherwise } => {
            for (cond, body) in arms {
                if run_list(db, sess, cond, r)? == 0 {
                    return run_list(db, sess, body, r);
                }
            }
            match otherwise {
                Some(body) => run_list(db, sess, body, r),
                None => Ok(0),
            }
        }
        CompoundKind::For { name, words, body } => {
            let mut list = Vec::new();
            {
                let mut ex = Ex { db: &mut *db, sess: &mut *sess, ctx, err: &mut *r.err };
                for w in words {
                    list.extend(ex.expand(w)?);
                }
            }
            ctx.loops.set(ctx.loops.get() + 1);
            let mut code = 0;
            let mut result = Ok(());
            for item in list {
                sess.set(name, item);
                match step(run_list(db, sess, body, r), ctx, sess) {
                    Ok(Step::Next(c)) => code = c,
                    Ok(Step::Leave(c)) => {
                        code = c;
                        break;
                    }
                    Err(flow) => {
                        result = Err(flow);
                        break;
                    }
                }
            }
            ctx.loops.set(ctx.loops.get() - 1);
            result.map(|_| code)
        }
        CompoundKind::While { until, cond, body } => {
            ctx.loops.set(ctx.loops.get() + 1);
            let mut code = 0;
            let mut result = Ok(());
            loop {
                match step(run_list(db, sess, cond, r), ctx, sess) {
                    Ok(Step::Next(c)) => {
                        if (c == 0) == *until {
                            break;
                        }
                    }
                    Ok(Step::Leave(_)) => break,
                    Err(flow) => {
                        result = Err(flow);
                        break;
                    }
                }
                match step(run_list(db, sess, body, r), ctx, sess) {
                    Ok(Step::Next(c)) => code = c,
                    Ok(Step::Leave(c)) => {
                        code = c;
                        break;
                    }
                    Err(flow) => {
                        result = Err(flow);
                        break;
                    }
                }
            }
            ctx.loops.set(ctx.loops.get() - 1);
            result.map(|_| code)
        }
    }
}

/// Run one of the commands that act on the session.
fn builtin(db: &mut Db, sess: &mut Session, ctx: &Ctx, name: &[u8], args: &[Vec<u8>], io: &mut Io) -> i32 {
    io.errors_first();
    match name {
        b"cd" => {
            let mut back = false;
            let dir = match args {
                [] => match sess.get(b"HOME") {
                    Some(home) => home.to_vec(),
                    None => {
                        io.err_line("qdb: cd: HOME not set");
                        return 1;
                    }
                },
                // `cd -`: back to OLDPWD, printing where that is.
                [dash] if dash == b"-" => match sess.get(b"OLDPWD") {
                    Some(old) if old.is_empty() => {
                        io.put(b"\n");
                        return 0;
                    }
                    Some(old) => {
                        back = true;
                        old.to_vec()
                    }
                    None => {
                        io.err_line("qdb: cd: OLDPWD not set");
                        return 1;
                    }
                },
                [dir] => dir.clone(),
                _ => {
                    io.err_line("qdb: cd: too many arguments");
                    return 1;
                }
            };
            if dir.is_empty() {
                return 0;
            }
            match db.resolve(&dir) {
                Ok(id) if db.is_dir(id) => {
                    sess.chdir(&dir);
                    db.set_cwd(sess.cwd());
                    if back {
                        let mut line = sess.get(b"PWD").unwrap_or(b"/").to_vec();
                        line.push(b'\n');
                        io.put(&line);
                    }
                    0
                }
                Ok(_) => {
                    io.err_line(format!("qdb: cd: {}: Not a directory", lossy(&dir)));
                    1
                }
                Err(e) => {
                    io.err_line(format!("qdb: cd: {}: {}", lossy(&dir), e.msg()));
                    1
                }
            }
        }
        b"export" => {
            let mut code = 0;
            for a in args {
                let (name, value) = match a.iter().position(|c| *c == b'=') {
                    Some(at) => (&a[..at], Some(a[at + 1..].to_vec())),
                    None => (&a[..], None),
                };
                if !session::is_name(name) {
                    io.err_line(format!("qdb: export: `{}': not a valid identifier", lossy(a)));
                    code = 1;
                    continue;
                }
                if let Some(value) = value {
                    sess.set(name, value);
                }
                sess.export(name);
            }
            code
        }
        b"unset" => {
            // A word that cannot be a name is passed over without a
            // word, as bash does.
            for a in args.iter().filter(|a| a.as_slice() != b"-v") {
                sess.unset(a);
            }
            0
        }
        b"read" => read_line(sess, args, io),
        _ => {
            let which = lossy(name);
            if ctx.loops.get() == 0 {
                io.err_line(format!("qdb: {}: only meaningful in a `for', `while', or `until' loop", which));
                return 0;
            }
            let mut code = 0;
            let n = match args.first() {
                None => 1,
                Some(a) => match std::str::from_utf8(a).ok().and_then(|t| t.parse::<i64>().ok()) {
                    Some(n) if n > 0 => n.min(u32::MAX as i64) as u32,
                    Some(_) => {
                        io.err_line(format!("qdb: {}: {}: loop count out of range", which, lossy(a)));
                        code = 1;
                        u32::MAX
                    }
                    None => {
                        // bash ends the whole shell here.
                        io.err_line(format!("qdb: {}: {}: numeric argument required", which, lossy(a)));
                        ctx.flow.set(Some(Flow::Stop(128)));
                        return 128;
                    }
                },
            };
            ctx.flow.set(Some(if name == b"break" || code != 0 { Flow::Break(n) } else { Flow::Continue(n) }));
            code
        }
    }
}

/// `read [-r] [NAME...]`: take one line of standard input and split it
/// among the names; with no name it goes to `REPLY` whole. The status is
/// 1 when the input ended before a newline.
fn read_line(sess: &mut Session, args: &[Vec<u8>], io: &mut Io) -> i32 {
    let raw = args.iter().any(|a| a == b"-r");
    let names: Vec<&Vec<u8>> = args.iter().filter(|a| a.as_slice() != b"-r" && a.as_slice() != b"--").collect();
    if let Some(bad) = names.iter().find(|n| !session::is_name(n)) {
        io.err_line(format!("qdb: read: `{}': not a valid identifier", lossy(bad)));
        return 1;
    }
    let data = io.stdin;
    // The line, with what a backslash protected marked.
    let mut line: Vec<(u8, bool)> = Vec::new();
    let mut i = 0;
    let mut ended = false;
    while i < data.len() {
        let c = data[i];
        i += 1;
        if c == b'\\' && !raw {
            if i < data.len() {
                if data[i] != b'\n' {
                    line.push((data[i], true));
                }
                i += 1;
            }
            continue;
        }
        if c == b'\n' {
            ended = true;
            break;
        }
        line.push((c, false));
    }
    io.stdin = &data[i..];
    let code = if ended { 0 } else { 1 };
    let text = |part: &[(u8, bool)]| part.iter().map(|(c, _)| *c).collect::<Vec<u8>>();
    if names.is_empty() {
        sess.set(b"REPLY", text(&line));
        return code;
    }
    let ifs = sess.ifs().to_vec();
    let sep = |p: &(u8, bool)| !p.1 && ifs.contains(&p.0);
    let blank = |p: &(u8, bool)| sep(p) && matches!(p.0, b' ' | b'\t' | b'\n');
    let mut rest = &line[..];
    while rest.first().is_some_and(blank) {
        rest = &rest[1..];
    }
    for (n, name) in names.iter().enumerate() {
        if n + 1 == names.len() {
            // The last name takes all that is left, without the blanks
            // at its end, and without a lone separator there.
            let mut end = rest.len();
            while end > 0 && blank(&rest[end - 1]) {
                end -= 1;
            }
            let mut last = &rest[..end];
            if let Some((tail, body)) = last.split_last() {
                if sep(tail) && !body.iter().any(|p| sep(p) && !blank(p)) {
                    last = body;
                    while last.last().is_some_and(blank) {
                        last = &last[..last.len() - 1];
                    }
                }
            }
            sess.set(name, text(last));
            break;
        }
        let end = rest.iter().position(sep).unwrap_or(rest.len());
        sess.set(name, text(&rest[..end]));
        rest = &rest[end..];
        // One separator: blanks, at most one that is not a blank, blanks.
        while rest.first().is_some_and(blank) {
            rest = &rest[1..];
        }
        if rest.first().is_some_and(|p| sep(p) && !blank(p)) {
            rest = &rest[1..];
            while rest.first().is_some_and(blank) {
                rest = &rest[1..];
            }
        }
    }
    code
}

/// Run a command given as words, as `xargs` and `find -exec` do, with the
/// streams of the command that runs it.
pub fn run_argv(db: &mut Db, argv: &[Vec<u8>], io: &mut Io) -> i32 {
    let Some((name, args)) = argv.split_first() else { return 0 };
    // `:` is part of the shell, like cd and read, which `cmd` does not
    // know at all: none of them can be run from here.
    let prepared = if name == b":" { Err("command: : (it is part of the shell)".to_string()) } else { cmd::prepare(name, args) };
    match prepared {
        Ok(p) => (p.run)(db, io),
        Err(what) => {
            io.err_line(format!("qdb: unsupported {}", what));
            EXIT_UNSUPPORTED
        }
    }
}

/// One command of a pipeline.
struct Stage<'a> {
    run: Option<Box<dyn FnOnce(&mut Db, &mut Io) -> i32 + 'a>>,
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
    /// How much of the standard input the command was given it left
    /// unread.
    left: usize,
    /// The same, when that input was the enclosing loop's: what the
    /// loop's next command starts from.
    inherited_left: Option<usize>,
    /// The stage is the last of its pipeline, in a construct whose
    /// errors go where its output goes.
    merge_down: bool,
}

enum State {
    /// Taking input in pieces.
    Streaming(Box<dyn Filter>),
    /// Gathering all the input, to run on it when it ends.
    Gathering(Spool),
    /// Needs no input, or no more of it.
    Closed(Option<Box<dyn Filter>>),
}

enum Stdin {
    Bytes(Vec<u8>),
    File(NodeId),
}

impl<'a> Stage<'a> {
    fn new(prepared: Work<'a>, redirs: Vec<Redir>, scratch: Scratch, piped: bool) -> Stage<'a> {
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
            Input::Whole => State::Gathering(Spool::held(scratch.clone())),
            Input::Unused => State::Closed(None),
        };
        let spools = redirs.iter().map(|_| Spool::new(scratch.clone())).collect();
        Stage { run: Some(prepared.run), state, stream, redirs, plan, spools, buf: Vec::new(), err: Vec::new(), to_err: Vec::new(), stdin_size: None, piped: false, scratch, left: 0, inherited_left: None, merge_down: false }
    }

    /// Give `f` the streams of this command. Returns what `f` returned and
    /// whether the reader of the output wants no more.
    fn with_io<R>(&mut self, stdin: &[u8], down: &mut dyn Sink, out_file: Option<NodeId>, f: impl FnOnce(&mut Io) -> R) -> (R, bool) {
        let Stage { plan, spools, buf, err, to_err, stdin_size, piped, scratch, left, merge_down, .. } = self;
        let mut nowhere = Nowhere;
        let out: &mut dyn Sink = match plan.d1 {
            Dest::Down => down,
            Dest::Err => to_err,
            Dest::File(i) => &mut spools[i],
            Dest::Null => &mut nowhere,
        };
        let mut io = Io { stdin, out, buf, stopped: false, err, out_file, merged: plan.d1 == plan.d2 || (*merge_down && plan.d1 == Dest::Down && plan.d2 == Dest::Err), exact: true, stdin_size: *stdin_size, piped: *piped, scratch: scratch.clone() };
        let r = f(&mut io);
        io.flush();
        *left = io.stdin.len();
        (r, io.stopped)
    }

    fn push(&mut self, data: &[u8], down: &mut dyn Sink) -> bool {
        match std::mem::replace(&mut self.state, State::Closed(None)) {
            State::Closed(f) => {
                self.state = State::Closed(f);
                false
            }
            State::Gathering(mut all) => {
                all.write(data);
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
    ///
    /// `inherited` is what is left of the standard input of the loop the
    /// command is in, for the first command of a pipeline there.
    fn end(&mut self, db: &mut Db, down: &mut dyn Sink, uerr: &mut dyn Sink, inherited: Option<&[u8]>) -> i32 {
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
            self.piped = stdin.is_some() || matches!(state, State::Gathering(_)) || inherited.is_some();
            code = match (state, stdin) {
                (State::Streaming(f), _) | (State::Closed(Some(f)), _) => self.with_io(&[], down, None, |io| f.finish_db(db, io)).0,
                (State::Gathering(mut all), None) => {
                    all.end();
                    self.with_io(all.all(), down, out_file, |io| run.unwrap()(db, io)).0
                }
                (_, None) => {
                    let code = self.with_io(inherited.unwrap_or(&[]), down, out_file, |io| run.unwrap()(db, io)).0;
                    self.inherited_left = inherited.map(|_| self.left);
                    code
                }
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
                        // The command may change the file while it reads
                        // its input, so the input is a copy: in memory,
                        // or past the scratch limit in a scratch file.
                        let mut text = Spool::held(self.scratch.clone());
                        for p in db.pieces(id).to_vec() {
                            text.write(&db.text_of(&p));
                        }
                        text.end();
                        self.with_io(text.all(), down, out_file, |io| run.unwrap()(db, io)).0
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
struct Rest<'a, 'b> {
    stages: &'a mut [Stage<'b>],
    last: &'a mut dyn Sink,
}

impl Sink for Rest<'_, '_> {
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
