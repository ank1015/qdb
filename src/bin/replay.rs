//! qdb-replay: runs a file of units against a database and says where
//! the work went: per kind of unit, its share of the units, processor
//! time, text read, bytes written and syncs. The figures are the ones
//! the engine counts for every unit (`qdb --cost` prints the same).

use qdb::{Config, Cost, Db, SyncMode};
use std::fmt::Write as _;

#[global_allocator]
static ALLOC: qdb::mem::Counting = qdb::mem::Counting;

const USAGE: &str = "\
Usage: qdb-replay --db DIR [OPTIONS] UNITS_FILE

Runs every unit of UNITS_FILE, in order, against the database in DIR,
and prints two tables in Markdown: the units grouped by their first
command, and grouped by what they did. The database is changed by the
units that write: run it on a copy to keep the original.

UNITS_FILE holds one unit per line. Empty lines and lines starting with
`#` are skipped; the sign ⏎ inside a line stands for a newline in the
unit.

What a unit did is decided from its counters, in this order:
  wrote a lot                 it added 64 KiB or more of content and records
  wrote a little              it added less
  searched with the index     it ran grep or rg and asked the index
  searched without the index  it ran grep or rg and did not
  read a lot                  it read 64 KiB or more of text
  read a little               anything else

  --sync MODE     always (the default) or batch
  --no-index      answer searches without the search index
  --title TEXT    a heading for the report
  --per-unit FILE also write one line per unit to FILE: the counters as
                  `qdb --cost` prints them, the exit status, then the unit
  --limit N       stop after N units";

const LOT: u64 = 64 * 1024;

const KINDS: [&str; 6] = ["read a little", "read a lot", "searched with the index", "searched without the index", "wrote a little", "wrote a lot"];

/// The first command word of a unit: assignments before it are skipped.
fn first_command(unit: &str) -> String {
    for word in unit.split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '<' | '>' | '(' | ')')) {
        if word.is_empty() || word == "!" || word == "{" {
            continue;
        }
        let name_end = word.find('=').unwrap_or(0);
        let is_assignment = name_end > 0 && word[..name_end].bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') && !word.as_bytes()[0].is_ascii_digit();
        if is_assignment {
            continue;
        }
        return word.to_string();
    }
    "(nothing)".to_string()
}

fn searches(unit: &str) -> bool {
    unit.split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')')).any(|w| matches!(w, "grep" | "rg" | "egrep" | "fgrep"))
}

fn what_it_did(unit: &str, c: &Cost) -> usize {
    let wrote = c.content_written + c.records_written;
    if wrote >= LOT {
        5
    } else if wrote > 0 {
        4
    } else if searches(unit) {
        if c.index_lookups > 0 {
            2
        } else {
            3
        }
    } else if c.text_bytes >= LOT {
        1
    } else {
        0
    }
}

#[derive(Default, Clone)]
struct Group {
    units: u64,
    cost: Cost,
}

fn ms(ns: u64) -> String {
    let v = ns as f64 / 1e6;
    if v >= 100.0 {
        format!("{:.0}", v)
    } else if v >= 10.0 {
        format!("{:.1}", v)
    } else if v >= 1.0 {
        format!("{:.2}", v)
    } else {
        format!("{:.3}", v)
    }
}

fn mb(bytes: u64) -> String {
    let v = bytes as f64 / 1e6;
    if bytes == 0 {
        "0".into()
    } else if v >= 100.0 {
        format!("{:.0}", v)
    } else if v >= 1.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.3}", v)
    }
}

fn share(part: u64, whole: u64) -> String {
    if whole == 0 {
        "0%".into()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

fn table(md: &mut String, heading: &str, first: &str, rows: &[(String, Group)], total: &Group) {
    let _ = writeln!(md, "\n### {}\n", heading);
    let _ = writeln!(
        md,
        "| {} | Units | Share of units | CPU, ms | Share of CPU | Elapsed, ms | Not on the CPU, ms | Text read, MB | Index read, MB | Tree entries | Content written, MB | Records written, MB | Upkeep written, MB | Syncs | Scratch, MB |",
        first
    );
    let _ = writeln!(md, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    let mut line = |name: &str, g: &Group| {
        let c = &g.cost;
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            name,
            g.units,
            share(g.units, total.units),
            ms(c.cpu_ns),
            share(c.cpu_ns, total.cost.cpu_ns),
            ms(c.elapsed_ns),
            ms(c.elapsed_ns.saturating_sub(c.cpu_ns)),
            mb(c.text_bytes),
            mb(c.index_bytes),
            c.tree_nodes,
            mb(c.content_written),
            mb(c.records_written),
            mb(c.upkeep_written),
            c.syncs,
            mb(c.scratch_bytes)
        );
    };
    for (name, g) in rows {
        if g.units > 0 {
            line(name, g);
        }
    }
    line("**all**", total);
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut dir, mut file, mut title, mut per_unit) = (None, None, None, None);
    let mut cfg = Config { sync: SyncMode::Always, ..Config::default() };
    let mut limit = usize::MAX;
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{} needs a value", name));
        match a.as_str() {
            "--db" => dir = Some(value("--db")?),
            "--sync" => {
                cfg.sync = match value("--sync")?.as_str() {
                    "always" => SyncMode::Always,
                    "batch" => SyncMode::Batch,
                    other => return Err(format!("--sync: {} is not `always` or `batch`", other)),
                }
            }
            "--no-index" => cfg.index = false,
            "--title" => title = Some(value("--title")?),
            "--per-unit" => per_unit = Some(value("--per-unit")?),
            "--limit" => limit = value("--limit")?.parse().map_err(|_| "--limit needs a number")?,
            "--help" | "-h" => {
                println!("{}", USAGE);
                return Ok(());
            }
            other if other.starts_with("--") => return Err(format!("unknown argument {}\n{}", other, USAGE)),
            other => file = Some(other.to_string()),
        }
    }
    let dir = dir.ok_or(format!("--db is needed\n{}", USAGE))?;
    let file = file.ok_or(format!("a file of units is needed\n{}", USAGE))?;
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {}", file, e))?;
    let (sync, index) = (cfg.sync, cfg.index);
    let (mut db, report) = Db::open_dir(std::path::Path::new(&dir), cfg).map_err(|e| format!("{}: {}", dir, e))?;
    for m in report.messages() {
        eprintln!("qdb-replay: {}: {}", dir, m);
    }
    let mut by_command: Vec<(String, Group)> = Vec::new();
    let mut by_work: Vec<(String, Group)> = KINDS.iter().map(|k| (k.to_string(), Group::default())).collect();
    let mut total = Group::default();
    let mut lines = String::new();
    let mut failed = 0u64;
    struct Null;
    impl qdb::Sink for Null {
        fn write(&mut self, _: &[u8]) -> bool {
            true
        }
    }
    let started = std::time::Instant::now();
    for raw in text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).take(limit) {
        let unit = raw.replace('⏎', "\n");
        let code = qdb::execute_to(&mut db, unit.as_bytes(), &mut Null, &mut Null);
        let cost = db.last_cost();
        if code > 1 {
            failed += 1;
        }
        if per_unit.is_some() {
            let _ = writeln!(lines, "{} status={} {}", cost, code, raw);
        }
        let name = first_command(&unit);
        let at = match by_command.iter().position(|(n, _)| *n == name) {
            Some(i) => i,
            None => {
                by_command.push((name, Group::default()));
                by_command.len() - 1
            }
        };
        for g in [&mut by_command[at].1, &mut by_work[what_it_did(&unit, &cost)].1, &mut total] {
            g.units += 1;
            g.cost.add(&cost);
        }
    }
    let wall = started.elapsed();
    if let Some(e) = db.failure() {
        return Err(e.to_string());
    }
    db.close()?;
    by_command.sort_by(|a, b| b.1.cost.cpu_ns.cmp(&a.1.cost.cpu_ns).then(a.0.cmp(&b.0)));
    let mut md = String::new();
    let _ = writeln!(md, "## {}\n", title.unwrap_or_else(|| format!("Units of `{}`", file)));
    let _ = writeln!(
        md,
        "{} units in {:.2} s ({:.0} a second), sync {}, index {}. {} ended with a status above 1. The most memory any one unit added: {} MB. Times are measured and differ from run to run; every other column is counted and is the same on every run over the same database.",
        total.units,
        wall.as_secs_f64(),
        total.units as f64 / wall.as_secs_f64().max(1e-9),
        if sync == SyncMode::Always { "always" } else { "in batches" },
        if index { "on" } else { "off" },
        failed,
        mb(total.cost.memory)
    );
    table(&mut md, "By what the unit did", "What it did", &by_work, &total);
    for (name, _) in by_command.iter_mut() {
        *name = format!("`{}`", name);
    }
    table(&mut md, "By first command", "First command", &by_command, &total);
    print!("{}", md);
    if let Some(p) = per_unit {
        std::fs::write(&p, lines).map_err(|e| format!("{}: {}", p, e))?;
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qdb-replay: {}", e);
            std::process::ExitCode::from(2)
        }
    }
}
