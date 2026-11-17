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

/// The streams of one command in a pipeline.
pub struct Io<'a> {
    pub stdin: &'a [u8],
    pub out: Vec<u8>,
    pub err: Vec<u8>,
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
}

/// A command with its flags already checked, ready to run.
pub type Runner = Box<dyn FnOnce(&mut Db, &mut Io) -> i32>;

struct Stage {
    runner: Runner,
    redirects: Vec<(RedirOp, Result<Vec<u8>, Vec<u8>>)>,
}

fn fail(msg: String) -> Outcome {
    Outcome { stdout: Vec::new(), stderr: format!("qdb: {}\n", msg).into_bytes(), code: EXIT_UNSUPPORTED }
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

pub fn execute(db: &mut Db, line: &[u8]) -> Outcome {
    let pipeline = match shell::parse(line) {
        Ok(None) => return Outcome::default(),
        Ok(Some(p)) => p,
        Err(ParseError::Unsupported(what)) => return fail(format!("unsupported syntax: {}", what)),
        Err(ParseError::Syntax(what)) => return fail(what),
    };

    // Expand and check everything before running anything.
    let mut stages = Vec::new();
    for command in &pipeline.commands {
        let argv: Vec<Vec<u8>> = command.words.iter().flat_map(|w| expand(db, w)).collect();
        let runner: Runner = match argv.split_first() {
            None => Box::new(|_, _| 0),
            Some((name, args)) => match cmd::prepare(name, args) {
                Ok(r) => r,
                Err(what) => return fail(format!("unsupported {}", what)),
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
        stages.push(Stage { runner, redirects });
    }

    let mut stderr = Vec::new();
    let mut piped: Vec<u8> = Vec::new();
    let mut code = 0;
    for stage in stages {
        let (out, c) = run_stage(db, stage, &piped, &mut stderr);
        piped = out;
        code = c;
    }
    Outcome { stdout: piped, stderr, code }
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

/// Run one command of a pipeline. Returns what goes down the pipe and the
/// exit status.
fn run_stage(db: &mut Db, stage: Stage, stdin: &[u8], stderr: &mut Vec<u8>) -> (Vec<u8>, i32) {
    // Redirections are opened left to right before the command runs, as in
    // bash: `>` creates or empties its file even if a later one takes the
    // output.
    let mut undo = Vec::new();
    let mut target: Option<(RedirOp, NodeId, Vec<u8>)> = None;
    for (op, path) in &stage.redirects {
        let opened = match path {
            Err(word) => Err(format!("qdb: {}: ambiguous redirect\n", lossy(word))),
            Ok(path) => open_redirect(db, *op, path, &mut undo)
                .map(|id| (id, path.clone()))
                .map_err(|e| format!("qdb: {}: {}\n", lossy(path), e.msg())),
        };
        match opened {
            Ok((id, path)) => target = Some((*op, id, path)),
            Err(msg) => {
                stderr.extend_from_slice(msg.as_bytes());
                return (Vec::new(), 1);
            }
        }
    }

    let mut io = Io { stdin, out: Vec::new(), err: Vec::new(), out_file: target.as_ref().map(|t| t.1) };
    let mut code = (stage.runner)(db, &mut io);
    stderr.extend_from_slice(&io.err);

    let (op, id, path) = match target {
        None => return (io.out, code),
        Some(t) => t,
    };
    if !is_valid_text(&io.out) {
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
        return (Vec::new(), 1);
    }
    // The command may have removed or replaced the file it was writing to;
    // the output then goes nowhere, as it would with an unlinked file.
    if db.exists(id) && !db.is_dir(id) && !io.out.is_empty() {
        let mut content = db.content(id).to_vec();
        match op {
            RedirOp::Append => content.extend_from_slice(&io.out),
            RedirOp::Write => {
                // Writing starts at offset 0 of the emptied file.
                if io.out.len() >= content.len() {
                    content = io.out;
                } else {
                    content[..io.out.len()].copy_from_slice(&io.out);
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
    (Vec::new(), code)
}
