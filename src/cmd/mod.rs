//! The fifteen commands of the first version.
//!
//! Each command has a `prepare` function that checks its flags and returns
//! a closure to run. Unsupported flags are reported by `prepare`, before
//! any command on the line has run.

use crate::engine::{Io, Prepared, Runner};
use crate::util::{getopt_long, lossy, LongOpt, OptError, Parsed};

mod files;
mod find;
mod grep;
mod ls;
mod read;
mod rg;
mod sed;
mod small;
mod sort;
mod sys;
mod text;

/// Look a command up by name and check its arguments. The error names what
/// is unsupported, for example `command: awk` or `flag: grep -w`.
pub fn prepare(name: &[u8], args: &[Vec<u8>]) -> Result<Prepared, String> {
    let args = args.to_vec();
    let plain: Result<Runner, String> = match name {
        b"echo" => read::echo(args),
        b"cat" => return read::cat(args),
        b"head" => return read::head(args),
        b"tail" => return read::tail(args),
        b"wc" => return read::wc(args),
        b"ls" => ls::ls(args),
        b"find" => find::find(args),
        b"grep" => return grep::grep(args),
        b"sed" => return sed::sed(args),
        b"rg" => return rg::rg(args),
        b"mkdir" => files::mkdir(args),
        b"touch" => files::touch(args),
        b"cp" => files::cp(args),
        b"mv" => files::mv(args),
        b"rm" => files::rm(args),
        b"rmdir" => files::rmdir(args),
        b"printf" => small::printf(args),
        b"test" => small::test("test", args),
        b"[" => small::test("[", args),
        b"true" | b":" => small::status(0),
        b"false" => small::status(1),
        b"sleep" => small::sleep(args),
        b"ln" => small::ln(args),
        b"basename" => small::basename(args),
        b"dirname" => small::dirname(args),
        b"seq" => small::seq(args),
        b"pwd" => small::pwd(args),
        b"du" => sys::du(args),
        b"date" => sys::date(args),
        b"xargs" => return sys::xargs(args),
        b"sort" => return sort::sort(args),
        b"cut" => return text::cut(args),
        b"tr" => return text::tr(args),
        b"uniq" => return text::uniq(args),
        b"nl" => return text::nl(args),
        b"tee" => return text::tee(args),
        _ => Err(format!("command: {}", lossy(name))),
    };
    plain.map(Prepared::from)
}

/// True if there is a command of this name.
pub fn known(name: &[u8]) -> bool {
    NAMES.iter().any(|n| n.as_bytes() == name)
}

/// Every command, for the check of a name that is all that is known of a
/// command before it runs.
pub const NAMES: &[&str] = &[
    "echo", "cat", "head", "tail", "wc", "ls", "find", "grep", "sed", "mkdir", "touch", "cp", "mv", "rm", "rmdir", "printf", "test", "[", "true", ":", "false",
    "sleep", "ln", "basename", "dirname", "seq", "rg", "du", "date", "xargs", "sort", "cut", "tr", "uniq", "nl", "tee", "pwd",
];

/// Gathers input into whole lines for a command that reads its standard
/// input in pieces.
#[derive(Default)]
pub(crate) struct LineBuf {
    partial: Vec<u8>,
}

impl LineBuf {
    /// Call `f` with each complete line in the input so far, newline
    /// included. Stops early, returning false, when `f` returns false.
    pub fn feed(&mut self, mut data: &[u8], mut f: impl FnMut(&[u8]) -> bool) -> bool {
        if !self.partial.is_empty() {
            match data.iter().position(|c| *c == b'\n') {
                None => {
                    self.partial.extend_from_slice(data);
                    return true;
                }
                Some(i) => {
                    self.partial.extend_from_slice(&data[..=i]);
                    data = &data[i + 1..];
                    let line = std::mem::take(&mut self.partial);
                    if !f(&line) {
                        return false;
                    }
                }
            }
        }
        while let Some(i) = data.iter().position(|c| *c == b'\n') {
            if !f(&data[..=i]) {
                return false;
            }
            data = &data[i + 1..];
        }
        self.partial.extend_from_slice(data);
        true
    }

    /// The last line, if the input did not end with a newline.
    pub fn rest(&mut self) -> Option<Vec<u8>> {
        if self.partial.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.partial))
        }
    }
}

/// Parse options, turning a missing option argument into GNU's message.
/// `status` is the exit status the real tool uses for usage errors.
fn options(cmd: &'static str, args: &[Vec<u8>], bools: &[u8], with_arg: &[u8], status: i32) -> Result<Result<Parsed, Runner>, String> {
    options_long(cmd, args, bools, with_arg, &[], status)
}

/// `options` for a command with long options.
fn options_long(
    cmd: &'static str,
    args: &[Vec<u8>],
    bools: &[u8],
    with_arg: &[u8],
    longs: &[LongOpt],
    status: i32,
) -> Result<Result<Parsed, Runner>, String> {
    match getopt_long(cmd, args, bools, with_arg, longs) {
        Ok(p) => Ok(Ok(p)),
        Err(OptError::Unsupported(what)) => Err(what),
        Err(OptError::MissingLongArg(name)) => Ok(Err(Box::new(move |_, io: &mut Io| {
            io.err_line(format!("{}: option '--{}' requires an argument", cmd, name));
            usage_line(cmd, io);
            io.err_line(format!("Try '{} --help' for more information.", cmd));
            status
        }))),
        Err(OptError::MissingArg(c)) => Ok(Err(Box::new(move |_, io: &mut Io| {
            io.err_line(format!("{}: option requires an argument -- '{}'", cmd, char::from(c)));
            usage_line(cmd, io);
            io.err_line(format!("Try '{} --help' for more information.", cmd));
            status
        }))),
    }
}

/// The line some tools put before "Try 'cmd --help'".
fn usage_line(cmd: &str, io: &mut Io) {
    if cmd == "grep" {
        io.err_line("Usage: grep [OPTION]... PATTERNS [FILE]...".to_string());
    }
}

/// "cmd: message" plus the "Try 'cmd --help'" line, as GNU prints for
/// usage errors.
fn usage(io: &mut Io, cmd: &str, msg: &str) -> i32 {
    io.err_line(format!("{}: {}", cmd, msg));
    io.err_line(format!("Try '{} --help' for more information.", cmd));
    1
}

/// Build an error line from byte pieces; names are not always valid UTF-8
/// once quoted.
fn msg(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}
