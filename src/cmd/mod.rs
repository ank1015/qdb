//! The fifteen commands of the first version.
//!
//! Each command has a `prepare` function that checks its flags and returns
//! a closure to run. Unsupported flags are reported by `prepare`, before
//! any command on the line has run.

use crate::engine::{Io, Runner};
use crate::util::{getopt, lossy, OptError, Parsed};

mod files;
mod find;
mod grep;
mod ls;
mod read;
mod sed;

/// Look a command up by name and check its arguments. The error names what
/// is unsupported, for example `command: awk` or `flag: grep -w`.
pub fn prepare(name: &[u8], args: &[Vec<u8>]) -> Result<Runner, String> {
    let args = args.to_vec();
    match name {
        b"echo" => read::echo(args),
        b"cat" => read::cat(args),
        b"head" => read::head(args),
        b"tail" => read::tail(args),
        b"wc" => read::wc(args),
        b"ls" => ls::ls(args),
        b"find" => find::find(args),
        b"grep" => grep::grep(args),
        b"sed" => sed::sed(args),
        b"mkdir" => files::mkdir(args),
        b"touch" => files::touch(args),
        b"cp" => files::cp(args),
        b"mv" => files::mv(args),
        b"rm" => files::rm(args),
        b"rmdir" => files::rmdir(args),
        _ => Err(format!("command: {}", lossy(name))),
    }
}

/// Parse options, turning a missing option argument into GNU's message.
/// `status` is the exit status the real tool uses for usage errors.
fn options(cmd: &'static str, args: &[Vec<u8>], bools: &[u8], with_arg: &[u8], status: i32) -> Result<Result<Parsed, Runner>, String> {
    match getopt(cmd, args, bools, with_arg) {
        Ok(p) => Ok(Ok(p)),
        Err(OptError::Unsupported(what)) => Err(what),
        Err(OptError::MissingArg(c)) => Ok(Err(Box::new(move |_, io: &mut Io| {
            io.err_line(format!("{}: option requires an argument -- '{}'", cmd, char::from(c)));
            io.err_line(format!("Try '{} --help' for more information.", cmd));
            status
        }))),
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
