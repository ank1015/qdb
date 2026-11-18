//! The qdb command line: load a tree, run command lines against it.

use qdb::{execute, store, Db};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Usage: qdb [--load DIR | --fixture FILE] [--no-index] [--dump DIR] [-c COMMAND]...

Runs bash command lines against an in-memory tree of text files.

  --load DIR       start from a copy of the directory DIR; symbolic links
                   and files that are not valid text are skipped and counted;
                   what was loaded and how long it took is reported on
                   standard error
  --fixture FILE   start from a fixture file (see tests/fixtures)
  --no-index       answer every search by scanning, without the search index;
                   output is the same either way, only speed differs
  --dump DIR       when done, write the tree into the empty directory DIR
  -c COMMAND       run COMMAND; can be given more than once

With no -c, command lines are read from standard input, one per line.
The exit status is that of the last command.";

fn run() -> Result<i32, String> {
    let mut source: Option<(bool, String)> = None;
    let mut index = true;
    let mut commands: Vec<String> = Vec::new();
    let mut dump: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--load" => source = Some((true, value("--load")?)),
            "--fixture" => source = Some((false, value("--fixture")?)),
            "--no-index" => index = false,
            "--dump" => dump = Some(value("--dump")?.into()),
            "-c" => commands.push(value("-c")?),
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(0);
            }
            other => return Err(format!("unknown argument {}\n{}", other, USAGE)),
        }
    }
    let mut db = match source {
        None => Db::new(),
        Some((true, dir)) => {
            let (db, report) = store::load_dir_with(dir.as_ref(), index).map_err(|e| format!("{}: {}", dir, e))?;
            let seconds = |micros: u64| micros as f64 / 1e6;
            eprintln!(
                "qdb: {}: loaded {} files ({:.1} MB) in {:.2} s; skipped {} ({} symbolic links, {} not valid text, {} other)",
                dir,
                report.files,
                report.bytes as f64 / 1e6,
                seconds(report.read_micros),
                report.skipped(),
                report.skipped_symlinks,
                report.skipped_not_text,
                report.skipped_other
            );
            match (report.index_micros, db.index_stats()) {
                (Some(micros), Some(stats)) => eprintln!(
                    "qdb: {}: search index built in {:.2} s ({:.1} MB, {} trigrams)",
                    dir,
                    seconds(micros),
                    stats.total_bytes() as f64 / 1e6,
                    stats.trigrams
                ),
                _ => eprintln!("qdb: {}: search index off", dir),
            }
            db
        }
        Some((false, file)) => {
            let text = std::fs::read(&file).map_err(|e| format!("{}: {}", file, e))?;
            store::parse_fixture(&text)?
        }
    };
    if !index {
        db.set_index(false);
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
