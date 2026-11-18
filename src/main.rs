//! The qdb command line: open or load a tree, run command lines against it.

use qdb::disk::Damage;
use qdb::{execute, store, Config, Db, SyncMode};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[global_allocator]
static ALLOC: qdb::mem::Counting = qdb::mem::Counting;

const USAGE: &str = "\
Usage: qdb [--db PATH] [--load DIR | --fixture FILE] [OPTIONS] [-c COMMAND]...

Runs bash command lines against a tree of text files.

  --db PATH        keep the tree in the database directory PATH, which is
                   made if it is not there. Without --db the tree is in
                   memory and is gone when qdb ends.
  --load DIR       start from a copy of the directory DIR; symbolic links
                   and files that are not valid text are skipped and counted;
                   what was loaded and how long it took is reported on
                   standard error. With --db the database must be empty.
  --fixture FILE   start from a fixture file (see tests/fixtures)
  --sync MODE      with --db: `always` (the default) has every command
                   line's changes on disk before the line returns; `batch`
                   syncs once per 64 command lines or 4 MB written, so a
                   power cut can lose up to that many of the last lines
                   (never an earlier line while keeping a later one)
  --batch-lines N  with --sync batch: sync once N command lines have changed
                   something since the last sync (default 64)
  --batch-bytes N  with --sync batch: or once N bytes have been written
                   since the last sync (default 4194304)
  --piece BYTES    cut file content into pieces of at most BYTES (default
                   65536); applies to what is written from now on
  --compress       store what is written from now on compressed (LZ4, a
                   piece at a time); what is there already is read
                   whichever way it was stored
  --index WHEN     `write` (the default) indexes content as it is written;
                   `later` indexes it in bulk, once 16 MB are waiting, when
                   a data file is finished and when the database is closed;
                   until then searches read what waits
  --no-index       answer every search by scanning, without the search index;
                   output is the same either way, only speed differs
  --check          check every checksum and every rule the database files
                   obey, and that the stored index is what a fresh build
                   would be; exit status 1 if anything is wrong
  --stats          print what is on disk, in bytes, by what it is
  --compact        do all outstanding upkeep now: rewrite content so that
                   no space is wasted, save the index and the tree
  --dump DIR       when done, write the tree into the empty directory DIR
  --cost           after each unit, print what it used on standard error,
                   on one line: `cost` and then NAME=NUMBER fields. The
                   fields up to `memory` are counts and repeat exactly;
                   `cpu_us` and `elapsed_us` are measured; `status` is the
                   unit's exit status
  -c COMMAND       run COMMAND; can be given more than once

Each -c is one unit: all of what it changes is there after a crash, or
none of it. With no -c (and none of --check, --stats, --compact), units
are read from standard input: one line, or as many lines as it takes to
close a quote, a here-document or a line ending in |, && or ||. The exit status is that of the
last command, or 2 if the database cannot be used.";

fn run() -> Result<i32, String> {
    let mut source: Option<(bool, String)> = None;
    let mut cfg = Config::default();
    let mut path: Option<PathBuf> = None;
    let mut commands: Vec<String> = Vec::new();
    let mut dump: Option<PathBuf> = None;
    let (mut check, mut stats, mut compact) = (false, false, false);
    let mut cost = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--db" => path = Some(value("--db")?.into()),
            "--load" => source = Some((true, value("--load")?)),
            "--fixture" => source = Some((false, value("--fixture")?)),
            "--sync" => {
                cfg.sync = match value("--sync")?.as_str() {
                    "always" => SyncMode::Always,
                    "batch" => SyncMode::Batch,
                    other => return Err(format!("--sync: {} is not `always` or `batch`", other)),
                }
            }
            "--piece" => {
                cfg.piece = value("--piece")?.parse().ok().filter(|n| *n >= 16 && *n <= 16 << 20).ok_or("--piece needs a number of bytes from 16 to 16777216")?
            }
            // For tests: small values make new data files and saves of
            // the tree frequent.
            "--segment" => cfg.segment = value("--segment")?.parse().ok().filter(|n| *n >= 4096).ok_or("--segment needs a number of bytes, at least 4096")?,
            "--checkpoint" => cfg.checkpoint = value("--checkpoint")?.parse().map_err(|_| "--checkpoint needs a number of bytes")?,
            "--batch-lines" => cfg.batch_lines = value("--batch-lines")?.parse().ok().filter(|n| *n >= 1).ok_or("--batch-lines needs a number of command lines, at least 1")?,
            "--batch-bytes" => cfg.batch_bytes = value("--batch-bytes")?.parse().ok().filter(|n| *n >= 1).ok_or("--batch-bytes needs a number of bytes, at least 1")?,
            "--no-index" => cfg.index = false,
            "--compress" => cfg.compress = true,
            "--index" => {
                cfg.index_later = match value("--index")?.as_str() {
                    "write" => false,
                    "later" => true,
                    other => return Err(format!("--index: {} is not `write` or `later`", other)),
                }
            }
            "--cost" => cost = true,
            "--check" => check = true,
            "--stats" => stats = true,
            "--compact" => compact = true,
            "--dump" => dump = Some(value("--dump")?.into()),
            "-c" => commands.push(value("-c")?),
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(0);
            }
            other => return Err(format!("unknown argument {}\n{}", other, USAGE)),
        }
    }
    let index = cfg.index;
    let on_disk = path.is_some();
    let mut db = match &path {
        None => Db::in_memory(Config { index: source.is_none() && index, ..cfg }),
        Some(p) => {
            let (db, report) = Db::open_dir(p, cfg).map_err(|e| format!("{}: {}", p.display(), e))?;
            for m in report.messages() {
                eprintln!("qdb: {}: {}", p.display(), m);
            }
            db
        }
    };
    if source.is_some() && db.node_count() > 1 {
        return Err("--load and --fixture need an empty database, and this one holds files already".into());
    }
    match source {
        None => {}
        Some((true, dir)) => {
            let report = store::load_into_db(&mut db, dir.as_ref()).map_err(|e| format!("{}: {}", dir, e))?;
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
            if on_disk {
                if index {
                    eprintln!("qdb: {}: search index built as the files were stored", dir);
                }
            } else if index {
                // In memory the index is built once everything is read.
                let start = std::time::Instant::now();
                db.set_index(true);
                let stats = db.index_details().expect("index on");
                eprintln!(
                    "qdb: {}: search index built in {:.2} s ({:.1} MB, {} lists)",
                    dir,
                    start.elapsed().as_secs_f64(),
                    stats.total_bytes() as f64 / 1e6,
                    stats.trigrams
                );
            }
            if !index {
                eprintln!("qdb: {}: search index off", dir);
            }
        }
        Some((false, file)) => {
            let text = std::fs::read(&file).map_err(|e| format!("{}: {}", file, e))?;
            store::parse_fixture_into(&mut db, &text)?;
            if !on_disk && index {
                db.set_index(true);
            }
        }
    }
    let mut code = 0;
    let mut run_line = |db: &mut Db, line: &[u8]| {
        let out = execute(db, line);
        std::io::stdout().write_all(&out.stdout).ok();
        std::io::stdout().flush().ok();
        std::io::stderr().write_all(&out.stderr).ok();
        if cost {
            eprintln!("cost {} status={}", out.cost, out.code);
        }
        code = out.code;
    };
    if !commands.is_empty() {
        for c in &commands {
            run_line(&mut db, c.as_bytes());
        }
    } else if !(check || stats || compact) {
        // A unit is one complete command as bash would read it: lines are
        // gathered while a quote, a substitution or a here-document is
        // open, or the last one ends in `|`, `&&`, `||` or a backslash.
        let mut unit: Vec<u8> = Vec::new();
        for line in std::io::stdin().lock().split(b'\n') {
            let line = line.map_err(|e| e.to_string())?;
            if !unit.is_empty() {
                unit.push(b'\n');
            }
            unit.extend_from_slice(&line);
            if qdb::engine::is_complete(&unit) {
                run_line(&mut db, &unit);
                unit.clear();
            }
        }
        if !unit.is_empty() {
            run_line(&mut db, &unit);
        }
    }
    if db.failure().is_some() {
        // Already reported with the command that met it. The database is
        // left as it is: the next open recovers.
        return Ok(2);
    }
    if compact {
        db.compact()?;
    }
    if check {
        match db.check() {
            Ok(what) => println!("ok: {}", what),
            Err(what) => {
                println!("NOT OK: {}", what);
                code = 1;
            }
        }
    }
    if stats {
        let s = db.disk_stats();
        let line = |name: &str, n: u64| println!("{:<28}{:>16}", name, n);
        line("text", s.text);
        line("index lists", s.lists);
        line("index tables", s.tables);
        line("tree", s.tree);
        line("log", s.log);
        line("record and file headers", s.framing);
        line("space not yet reclaimed", s.unreclaimed);
        line("total", s.total());
        line("data files", s.data_files as u64);
        line("raw text of all files", s.raw_text);
        if s.raw_text > 0 {
            println!("{:<28}{:>16.3}", "total / raw text", s.total() as f64 / s.raw_text as f64);
        }
    }
    if let Some(dir) = dump {
        store::dump_dir(&db, &dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    }
    db.close()?;
    Ok(code)
}

fn main() -> ExitCode {
    // Damage found while reading is thrown as a panic that carries its
    // message; it is reported below like any other error.
    let usual = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if info.payload().downcast_ref::<Damage>().is_none() {
            usual(info);
        }
    }));
    // Deep folder trees and long lines recurse; give the work a large stack.
    let worker = std::thread::Builder::new().stack_size(64 << 20).spawn(run).expect("spawn");
    match worker.join() {
        Ok(Ok(code)) => ExitCode::from(code as u8),
        Ok(Err(msg)) => {
            eprintln!("qdb: {}", msg);
            ExitCode::from(2)
        }
        Err(payload) => match payload.downcast::<Damage>() {
            Ok(d) => {
                eprintln!("qdb: {}", d.0);
                ExitCode::from(2)
            }
            Err(other) => std::panic::resume_unwind(other),
        },
    }
}
