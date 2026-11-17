//! The qdb command line: load a tree, run command lines against it.

use qdb::{execute, store, Db};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Usage: qdb [--load DIR | --fixture FILE] [--dump DIR] [-c COMMAND]...

Runs bash command lines against an in-memory tree of text files.

  --load DIR       start from a copy of the directory DIR
  --fixture FILE   start from a fixture file (see tests/fixtures)
  --dump DIR       when done, write the tree into the empty directory DIR
  -c COMMAND       run COMMAND; can be given more than once

With no -c, command lines are read from standard input, one per line.
The exit status is that of the last command.";

fn run() -> Result<i32, String> {
    let mut db = Db::new();
    let mut commands: Vec<String> = Vec::new();
    let mut dump: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--load" => {
                let dir = value("--load")?;
                db = store::load_dir(dir.as_ref()).map_err(|e| format!("{}: {}", dir, e))?;
            }
            "--fixture" => {
                let file = value("--fixture")?;
                let text = std::fs::read(&file).map_err(|e| format!("{}: {}", file, e))?;
                db = store::parse_fixture(&text)?;
            }
            "--dump" => dump = Some(value("--dump")?.into()),
            "-c" => commands.push(value("-c")?),
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(0);
            }
            other => return Err(format!("unknown argument {}\n{}", other, USAGE)),
        }
    }
    let mut code = 0;
    let mut run_line = |db: &mut Db, line: &[u8]| {
        let out = execute(db, line);
        std::io::stdout().write_all(&out.stdout).ok();
        std::io::stdout().flush().ok();
        std::io::stderr().write_all(&out.stderr).ok();
        code = out.code;
    };
    if commands.is_empty() {
        for line in std::io::stdin().lock().split(b'\n') {
            let line = line.map_err(|e| e.to_string())?;
            run_line(&mut db, &line);
        }
    } else {
        for c in &commands {
            run_line(&mut db, c.as_bytes());
        }
    }
    if let Some(dir) = dump {
        store::dump_dir(&db, &dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    }
    Ok(code)
}

fn main() -> ExitCode {
    // Deep folder trees and long lines recurse; give the work a large stack.
    let worker = std::thread::Builder::new().stack_size(512 << 20).spawn(run).expect("spawn");
    match worker.join().expect("worker panicked") {
        Ok(code) => ExitCode::from(code as u8),
        Err(msg) => {
            eprintln!("qdb: {}", msg);
            ExitCode::from(2)
        }
    }
}
