//! echo, cat, head, tail, wc.

use super::{msg, options};
use crate::engine::{Io, Runner};
use crate::tree::{Db, Errno};
use crate::util::quote;

pub fn echo(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    // bash's echo: only leading words made of option letters are options.
    let mut newline = true;
    let mut first = 0;
    for a in &args {
        if a.len() < 2 || a[0] != b'-' || !a[1..].iter().all(|c| b"neE".contains(c)) {
            break;
        }
        if let Some(c) = a[1..].iter().find(|c| **c != b'n') {
            return Err(format!("flag: echo -{}", char::from(*c)));
        }
        newline = false;
        first += 1;
    }
    Ok(Box::new(move |_, io| {
        io.out = args[first..].join(&b' ');
        if newline {
            io.out.push(b'\n');
        }
        0
    }))
}

/// Where a read command gets one input from.
enum Input<'a> {
    Data(&'a [u8]),
    Missing(Errno),
    Directory,
}

fn open<'a>(db: &'a Db, io_stdin: &'a [u8], name: &[u8]) -> Input<'a> {
    if name == b"-" {
        return Input::Data(io_stdin);
    }
    match db.resolve(name) {
        Err(e) => Input::Missing(e),
        Ok(id) if db.is_dir(id) => Input::Directory,
        Ok(id) => Input::Data(db.content(id)),
    }
}

fn operands_or_stdin(operands: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    if operands.is_empty() {
        vec![b"-".to_vec()]
    } else {
        operands
    }
}

pub fn cat(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("cat", &args, b"n", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let number = p.has(b'n');
    let files = operands_or_stdin(p.operands);
    Ok(Box::new(move |db, io| {
        let mut status = 0;
        let mut line_no = 0u64;
        let mut at_line_start = true;
        let mut out = Vec::new();
        for f in &files {
            if f != b"-" {
                // GNU cat refuses to copy a non-empty file onto itself.
                if let (Ok(id), Some(o)) = (db.resolve(f), io.out_file) {
                    if id == o && !db.is_dir(id) && !db.content(id).is_empty() {
                        io.err.extend(msg(&[b"cat: ", f, b": input file is output file\n"]));
                        status = 1;
                        continue;
                    }
                }
            }
            let data = match open(db, io.stdin, f) {
                Input::Data(d) => d,
                Input::Missing(e) => {
                    io.err.extend(msg(&[b"cat: ", f, b": ", e.msg().as_bytes(), b"\n"]));
                    status = 1;
                    continue;
                }
                Input::Directory => {
                    io.err.extend(msg(&[b"cat: ", f, b": Is a directory\n"]));
                    status = 1;
                    continue;
                }
            };
            if !number {
                out.extend_from_slice(data);
                continue;
            }
            for line in crate::util::lines(data) {
                if at_line_start {
                    line_no += 1;
                    out.extend_from_slice(format!("{:>6}\t", line_no).as_bytes());
                }
                out.extend_from_slice(line);
                at_line_start = line.ends_with(b"\n");
            }
        }
        io.out = out;
        status
    }))
}

/// A line count argument for head or tail: an optional sign and digits.
struct Count {
    sign: Option<u8>,
    n: usize,
}

fn parse_count(cmd: &'static str, arg: Option<&[u8]>) -> Result<Result<Count, Runner>, String> {
    let arg = match arg {
        None => return Ok(Ok(Count { sign: None, n: 10 })),
        Some(a) => a.to_vec(),
    };
    let (sign, digits) = match arg.first() {
        Some(c @ (b'+' | b'-')) => (Some(*c), &arg[1..]),
        _ => (None, &arg[..]),
    };
    if !digits.is_empty() && digits.iter().all(|c| c.is_ascii_digit()) {
        let n = digits.iter().fold(0usize, |n, c| n.saturating_mul(10).saturating_add((c - b'0') as usize));
        return Ok(Ok(Count { sign, n }));
    }
    // GNU accepts size suffixes such as 1k; this version does not.
    let suffix_start = digits.iter().position(|c| !c.is_ascii_digit()).unwrap_or(0);
    if suffix_start > 0 && digits[suffix_start..].iter().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!("flag: {} -n with a size suffix ({})", cmd, String::from_utf8_lossy(&arg)));
    }
    Ok(Err(Box::new(move |_, io: &mut Io| {
        // head strips the sign it understood; tail reports the whole word.
        let shown: &[u8] = if cmd == "head" && sign == Some(b'-') { &arg[1..] } else { &arg };
        io.err_line(format!("{}: invalid number of lines: {}", cmd, quote(shown)));
        1
    })))
}

/// The byte offset just past the first `n` lines.
fn first_lines(data: &[u8], n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut seen = 0;
    for (i, c) in data.iter().enumerate() {
        if *c == b'\n' {
            seen += 1;
            if seen == n {
                return i + 1;
            }
        }
    }
    data.len()
}

fn line_count(data: &[u8]) -> usize {
    let newlines = data.iter().filter(|c| **c == b'\n').count();
    newlines + usize::from(!data.is_empty() && !data.ends_with(b"\n"))
}

fn head_or_tail(cmd: &'static str, args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options(cmd, &args, b"", b"n", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let count = match parse_count(cmd, p.arg(b'n'))? {
        Ok(c) => c,
        Err(r) => return Ok(r),
    };
    let files = operands_or_stdin(p.operands);
    Ok(Box::new(move |db, io| {
        let mut status = 0;
        let mut out = Vec::new();
        let mut first_header = true;
        for f in &files {
            let input = open(db, io.stdin, f);
            if let Input::Missing(e) = input {
                io.err_line(format!("{}: cannot open {} for reading: {}", cmd, quote(f), e.msg()));
                status = 1;
                continue;
            }
            if files.len() > 1 {
                if !first_header {
                    out.push(b'\n');
                }
                first_header = false;
                let shown: &[u8] = if f == b"-" { b"standard input" } else { f };
                out.extend(msg(&[b"==> ", shown, b" <==\n"]));
            }
            let data = match input {
                Input::Data(d) => d,
                _ => {
                    io.err_line(format!("{}: error reading {}: Is a directory", cmd, quote(f)));
                    status = 1;
                    continue;
                }
            };
            let total = line_count(data);
            let range = match (cmd, count.sign) {
                // head -n N: the first N lines; head -n -N: all but the last N.
                ("head", Some(b'-')) => 0..first_lines(data, total.saturating_sub(count.n)),
                ("head", _) => 0..first_lines(data, count.n),
                // tail -n +N: from line N on; tail -n N: the last N lines.
                (_, Some(b'+')) => first_lines(data, count.n.saturating_sub(1))..data.len(),
                _ => first_lines(data, total.saturating_sub(count.n))..data.len(),
            };
            out.extend_from_slice(&data[range]);
        }
        io.out = out;
        status
    }))
}

pub fn head(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    head_or_tail("head", args)
}

pub fn tail(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    head_or_tail("tail", args)
}

pub fn wc(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let p = match options("wc", &args, b"lwc", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let all = p.flags.is_empty();
    let show = [all || p.has(b'l'), all || p.has(b'w'), all || p.has(b'c')];
    let fields = show.iter().filter(|s| **s).count();
    let from_stdin = p.operands.is_empty();
    let files = operands_or_stdin(p.operands);
    Ok(Box::new(move |db, io| {
        // Column width, as GNU wc computes it: wide enough for the total
        // size of the regular files, and at least 7 if any input is not a
        // regular file. One file and one count needs no padding.
        let mut width = 1;
        if !(files.len() == 1 && fields == 1) {
            let first_ok = files[0] == b"-" || db.resolve(&files[0]).is_ok();
            if first_ok {
                let mut regular_total = 0usize;
                let mut minimum = 1;
                for f in &files {
                    match open(db, io.stdin, f) {
                        Input::Data(d) if f != b"-" => regular_total += d.len(),
                        Input::Missing(_) => {}
                        _ => minimum = 7,
                    }
                }
                width = regular_total.to_string().len().max(minimum);
            }
        }
        let mut status = 0;
        let mut out = Vec::new();
        let mut totals = [0usize; 3];
        let print = |out: &mut Vec<u8>, counts: [usize; 3], name: Option<&[u8]>| {
            let cols: Vec<String> = (0..3).filter(|i| show[*i]).map(|i| format!("{:>w$}", counts[i], w = width)).collect();
            out.extend_from_slice(cols.join(" ").as_bytes());
            if let Some(n) = name {
                out.push(b' ');
                out.extend_from_slice(n);
            }
            out.push(b'\n');
        };
        for f in &files {
            let data: &[u8] = match open(db, io.stdin, f) {
                Input::Data(d) => d,
                Input::Missing(e) => {
                    io.err.extend(msg(&[b"wc: ", f, b": ", e.msg().as_bytes(), b"\n"]));
                    status = 1;
                    continue;
                }
                Input::Directory => {
                    io.err.extend(msg(&[b"wc: ", f, b": Is a directory\n"]));
                    status = 1;
                    b""
                }
            };
            let mut counts = [0usize, 0, data.len()];
            let mut in_word = false;
            for &c in data {
                if c == b'\n' {
                    counts[0] += 1;
                }
                // In the C locale a word is started by a printable
                // character and ended by white space; other bytes do
                // neither.
                if matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                    in_word = false;
                } else if (0x21..0x7f).contains(&c) && !in_word {
                    in_word = true;
                    counts[1] += 1;
                }
            }
            for i in 0..3 {
                totals[i] += counts[i];
            }
            print(&mut out, counts, if from_stdin { None } else { Some(f) });
        }
        if files.len() > 1 {
            print(&mut out, totals, Some(b"total"));
        }
        io.out = out;
        status
    }))
}
