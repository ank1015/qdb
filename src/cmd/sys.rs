//! du, date and xargs.

use super::ls::blocks_of;
use super::text::fails;
use crate::engine::{run_argv, Input, Prepared, Runner};
use crate::glob;
use crate::tree::{Db, NodeId};
use crate::util::{quote, quote_if_needed};
use crate::when::{self, SECOND};
use std::collections::HashSet;

// ---------------------------------------------------------------------- du

#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Size(u64),
    Human,
}

struct Du {
    summarize: bool,
    all: bool,
    total: bool,
    depth: Option<usize>,
    apparent: bool,
    unit: Unit,
    excludes: Vec<Vec<glob::Tok>>,
}

impl Du {
    fn excluded(&self, path: &[u8]) -> bool {
        self.excludes.iter().any(|g| {
            // The pattern may match the whole path or what follows any
            // of its slashes.
            glob::matches(g, path) || (0..path.len()).any(|i| path[i] == b'/' && glob::matches(g, &path[i + 1..]))
        })
    }

    fn show(&self, out: &mut Vec<u8>, bytes: u64, path: &[u8]) {
        let n = match self.unit {
            Unit::Human => when::human(bytes),
            Unit::Size(u) => bytes.div_ceil(u).to_string(),
        };
        out.extend_from_slice(n.as_bytes());
        out.push(b'\t');
        out.extend_from_slice(path);
        out.push(b'\n');
    }

    fn own(&self, db: &Db, id: NodeId) -> u64 {
        if self.apparent {
            if db.is_dir(id) {
                0
            } else {
                db.size(id)
            }
        } else {
            blocks_of(db, id) * 1024
        }
    }

    /// The size of everything at and under a node, printing what is to
    /// be shown on the way.
    fn walk(&self, db: &Db, io: &mut crate::engine::Io, id: NodeId, path: &mut Vec<u8>, depth: usize, seen: &mut Option<HashSet<NodeId>>, out: &mut Vec<u8>) -> u64 {
        if let Some(seen) = seen {
            seen.insert(id);
        }
        let limit = if self.summarize { 0 } else { self.depth.unwrap_or(usize::MAX) };
        let mut sum = self.own(db, id);
        if db.is_dir(id) {
            // Nothing below is shown and nothing left out: the sizes
            // alone are wanted.
            let quiet = depth >= limit && self.excludes.is_empty() && seen.is_none();
            if quiet {
                sum += self.below(db, id);
            } else {
                let base = path.len();
                for (name, child) in db.children(id) {
                    if path.last() != Some(&b'/') {
                        path.push(b'/');
                    }
                    path.extend_from_slice(name);
                    // What an earlier operand counted is not counted again.
                    if !self.excluded(path) && !seen.as_ref().is_some_and(|s| s.contains(child)) {
                        sum += self.walk(db, io, *child, path, depth + 1, seen, out);
                    }
                    path.truncate(base);
                }
            }
            if depth <= limit {
                self.show(out, sum, path);
            }
        } else if depth == 0 || (self.all && depth <= limit) {
            self.show(out, sum, path);
        }
        if out.len() >= 1 << 15 {
            io.put(out);
            out.clear();
        }
        sum
    }

    fn below(&self, db: &Db, dir: NodeId) -> u64 {
        let mut sum = 0;
        for child in db.children(dir).values() {
            sum += self.own(db, *child);
            if db.is_dir(*child) {
                sum += self.below(db, *child);
            }
        }
        sum
    }
}

pub fn du(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let mut d = Du { summarize: false, all: false, total: false, depth: None, apparent: false, unit: Unit::Size(1024), excludes: Vec::new() };
    let mut files: Vec<Vec<u8>> = Vec::new();
    let mut depth_text: Option<Vec<u8>> = None;
    let mut i = 0;
    let stop = |m: String| -> Result<Runner, String> { Ok(fails("du", m, true, 1).run) };
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == b"--" {
            files.extend(args[i..].iter().cloned());
            break;
        }
        if let Some(long) = a.strip_prefix(b"--") {
            let (name, value) = match long.iter().position(|c| *c == b'=') {
                Some(eq) => (&long[..eq], Some(long[eq + 1..].to_vec())),
                None => (long, None),
            };
            let arg = |i: &mut usize| -> Result<Vec<u8>, Result<Runner, String>> {
                if let Some(v) = &value {
                    return Ok(v.clone());
                }
                if *i < args.len() {
                    *i += 1;
                    return Ok(args[*i - 1].clone());
                }
                Err(stop(format!("option '--{}' requires an argument", String::from_utf8_lossy(name))))
            };
            match name {
                b"summarize" => d.summarize = true,
                b"human-readable" => d.unit = Unit::Human,
                b"all" => d.all = true,
                b"total" => d.total = true,
                b"bytes" => {
                    d.apparent = true;
                    d.unit = Unit::Size(1);
                }
                b"apparent-size" => d.apparent = true,
                b"one-file-system" => {}
                b"max-depth" => match arg(&mut i) {
                    Ok(v) => depth_text = Some(v),
                    Err(r) => return r,
                },
                b"exclude" => match arg(&mut i) {
                    Ok(v) => d.excludes.push(glob::compile(&glob::fnmatch_pattern(&v))),
                    Err(r) => return r,
                },
                _ => return Err(format!("flag: du {} (long options other than the ones listed in the README are not supported)", String::from_utf8_lossy(a))),
            }
            continue;
        }
        if a.len() < 2 || a[0] != b'-' {
            files.push(a.clone());
            continue;
        }
        let mut j = 1;
        while j < a.len() {
            let c = a[j];
            j += 1;
            match c {
                b's' => d.summarize = true,
                b'h' => d.unit = Unit::Human,
                b'a' => d.all = true,
                b'c' => d.total = true,
                b'b' => {
                    d.apparent = true;
                    d.unit = Unit::Size(1);
                }
                b'k' => d.unit = Unit::Size(1024),
                b'm' => d.unit = Unit::Size(1 << 20),
                b'x' => {}
                b'd' => {
                    if j < a.len() {
                        depth_text = Some(a[j..].to_vec());
                    } else if i < args.len() {
                        depth_text = Some(args[i].clone());
                        i += 1;
                    } else {
                        return stop("option requires an argument -- 'd'".into());
                    }
                    break;
                }
                _ => return Err(format!("flag: du -{}", char::from(c))),
            }
        }
    }
    if let Some(t) = &depth_text {
        match std::str::from_utf8(t).ok().and_then(|s| s.parse::<usize>().ok()) {
            Some(n) => d.depth = Some(n),
            None => return stop(format!("invalid maximum depth {}", quote(t))),
        }
    }
    if d.all && d.summarize {
        return stop("cannot both summarize and show all entries".into());
    }
    let mut warning = None;
    if d.summarize {
        match d.depth {
            Some(0) => warning = Some("du: warning: summarizing is the same as using --max-depth=0".to_string()),
            Some(n) => return stop(format!("warning: summarizing conflicts with --max-depth={}", n)),
            None => {}
        }
    }
    if files.is_empty() {
        files.push(b".".to_vec());
    }
    Ok(Box::new(move |db, io| {
        if let Some(w) = &warning {
            io.err_line(w);
        }
        let mut status = 0;
        let mut total = 0;
        let mut seen = if files.len() > 1 { Some(HashSet::new()) } else { None };
        let mut out = Vec::new();
        for f in &files {
            if f.is_empty() {
                io.put(&out);
                out.clear();
                io.err_line("du: invalid zero-length file name");
                status = 1;
                continue;
            }
            match db.resolve(f) {
                Err(e) => {
                    io.put(&out);
                    out.clear();
                    io.err_line(format!("du: cannot access {}: {}", quote(f), e.msg()));
                    status = 1;
                }
                Ok(id) => {
                    if seen.as_ref().is_some_and(|s| s.contains(&id)) || d.excluded(f) {
                        continue;
                    }
                    let mut path = f.clone();
                    total += d.walk(db, io, id, &mut path, 0, &mut seen, &mut out);
                }
            }
        }
        if d.total {
            d.show(&mut out, total, b"total");
        }
        io.put(&out);
        status
    }))
}

// -------------------------------------------------------------------- date

/// Words of dates that GNU date reads and this does not.
const OTHER_WORDS: &[&str] = &[
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec", "january", "february", "march", "april", "june", "july", "august", "september", "october", "november",
    "december", "sun", "mon", "tue", "tues", "wed", "thu", "thur", "thurs", "fri", "sat", "sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "next", "last", "this", "am", "pm",
    "noon", "midnight", "first", "third", "fourth", "gmt", "est", "pst", "cet", "tz",
];

const UNITS: &[(&str, i64, i64)] = &[
    ("sec", 1, 0),
    ("second", 1, 0),
    ("min", 60, 0),
    ("minute", 60, 0),
    ("hour", 3600, 0),
    ("day", 86400, 0),
    ("week", 7 * 86400, 0),
    ("fortnight", 14 * 86400, 0),
    ("month", 0, 1),
    ("year", 0, 12),
];

/// A date as `date -d` takes it: the forms of `when::parse_date`, with a
/// `Z` or `UTC` after them, the words now, today, yesterday and tomorrow,
/// and steps such as `+2 days`, `3 weeks ago` and `1 month`. `Err(true)`
/// is a date GNU date may read and this does not.
fn parse_when(text: &[u8], now: i64) -> Result<i64, bool> {
    if let Some(t) = when::parse_date(text) {
        return Ok(t);
    }
    let Ok(text) = std::str::from_utf8(text) else { return Err(false) };
    if text.trim().is_empty() {
        // No date is the start of today.
        return Ok(now - now.rem_euclid(86400 * SECOND));
    }
    let lower = text.to_ascii_lowercase();
    // A sign may stand apart from its number.
    let lower = lower.replace("+ ", "+").replace("- ", "-");
    let mut words: Vec<&str> = lower.split_ascii_whitespace().collect();
    let other = words.iter().any(|w| OTHER_WORDS.contains(&w.trim_matches(|c: char| !c.is_ascii_alphabetic())));
    let fail = Err(other);
    // The date the steps start from.
    let mut base = now;
    let mut used = 0;
    if !words.is_empty() {
        let two = words.len() >= 2;
        let joined = if two { format!("{} {}", words[0], words[1]) } else { String::new() };
        let strip = |w: &str| w.strip_suffix('z').map(|s| s.to_string());
        if two && when::parse_date(joined.as_bytes()).is_some() {
            base = when::parse_date(joined.as_bytes()).unwrap();
            used = 2;
        } else if two && strip(words[1]).is_some_and(|t| when::parse_date(format!("{} {}", words[0], t).as_bytes()).is_some()) {
            base = when::parse_date(format!("{} {}", words[0], strip(words[1]).unwrap()).as_bytes()).unwrap();
            used = 2;
        } else if let Some(t) = when::parse_date(words[0].as_bytes()) {
            base = t;
            used = 1;
        } else if let Some(t) = strip(words[0]).and_then(|w| when::parse_date(w.to_ascii_uppercase().as_bytes())) {
            base = t;
            used = 1;
        } else {
            match words[0] {
                "now" | "today" => used = 1,
                "yesterday" => {
                    base -= 86400 * SECOND;
                    used = 1;
                }
                "tomorrow" => {
                    base += 86400 * SECOND;
                    used = 1;
                }
                _ => {}
            }
        }
    }
    words.drain(..used);
    if words.first() == Some(&"utc") {
        words.remove(0);
    }
    // After a time of day, a signed number is a time zone to GNU date,
    // and one of those it does not accept.
    if used == 2 && words.first().is_some_and(|w| w.starts_with('+') || w.starts_with('-')) {
        return Err(false);
    }
    let (mut secs, mut months) = (0i64, 0i64);
    let mut i = 0;
    let mut any = used > 0;
    while i < words.len() {
        let w = words[i];
        i += 1;
        // A count, with the unit attached or in the next word.
        let digits_end = w.char_indices().find(|(k, c)| !(c.is_ascii_digit() || (*k == 0 && (*c == '+' || *c == '-')))).map_or(w.len(), |(k, _)| k);
        let (count, mut unit) = (&w[..digits_end], &w[digits_end..]);
        let n: i64 = match count {
            "" => 1,
            "+" | "-" => return fail,
            c => match c.parse() {
                Ok(n) => n,
                Err(_) => return fail,
            },
        };
        if unit.is_empty() {
            if count.is_empty() || i == words.len() {
                return fail;
            }
            unit = words[i];
            i += 1;
        }
        let unit = unit.strip_suffix('s').unwrap_or(unit);
        let Some((_, s, m)) = UNITS.iter().find(|(name, _, _)| *name == unit) else { return fail };
        let mut n = n;
        if words.get(i) == Some(&"ago") {
            n = -n;
            i += 1;
        }
        secs = secs.checked_add(n.checked_mul(*s).ok_or(false)?).ok_or(false)?;
        months = months.checked_add(n.checked_mul(*m).ok_or(false)?).ok_or(false)?;
        any = true;
    }
    if !any {
        return fail;
    }
    let mut t = base;
    if months != 0 {
        let c = when::civil(t);
        let m0 = c.year * 12 + (c.month as i64 - 1) + months;
        let (y, m) = (m0.div_euclid(12), m0.rem_euclid(12) as u32 + 1);
        if !(-9999..=99999).contains(&y) {
            return Err(false);
        }
        t = when::seconds(y, m, c.day, c.hour, c.min, c.sec) * SECOND + c.nanos as i64;
    }
    secs.checked_mul(SECOND).and_then(|s| t.checked_add(s)).ok_or(false)
}

pub fn date(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let mut when_text: Option<Vec<u8>> = None;
    let mut formats: Vec<Vec<u8>> = Vec::new();
    let mut operands: Vec<Vec<u8>> = Vec::new();
    let mut bad_iso: Option<Vec<u8>> = None;
    let iso = |spec: &[u8]| -> Option<&'static [u8]> {
        Some(match spec {
            b"" | b"date" => b"%Y-%m-%d",
            b"hours" => b"%Y-%m-%dT%H%:z",
            b"minutes" => b"%Y-%m-%dT%H:%M%:z",
            b"seconds" => b"%Y-%m-%dT%H:%M:%S%:z",
            b"ns" => b"%Y-%m-%dT%H:%M:%S,%N%:z",
            _ => return None,
        })
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == b"--" {
            operands.extend(args[i..].iter().cloned());
            break;
        }
        if let Some(long) = a.strip_prefix(b"--") {
            let (name, value) = match long.iter().position(|c| *c == b'=') {
                Some(eq) => (&long[..eq], Some(&long[eq + 1..])),
                None => (long, None),
            };
            match (name, value) {
                (b"utc" | b"universal", None) => {}
                (b"date", v) => {
                    when_text = Some(match v {
                        Some(v) => v.to_vec(),
                        None if i < args.len() => {
                            i += 1;
                            args[i - 1].clone()
                        }
                        None => return Ok(fails("date", "option '--date' requires an argument".into(), true, 1).run),
                    })
                }
                (b"iso-8601", v) => match iso(v.unwrap_or(b"")) {
                    Some(f) => formats.push(f.to_vec()),
                    None => bad_iso = Some(v.unwrap_or(b"").to_vec()),
                },
                (b"rfc-email" | b"rfc-2822" | b"rfc-822", None) => formats.push(b"%a, %d %b %Y %H:%M:%S %z".to_vec()),
                (b"rfc-3339", Some(v)) => formats.push(match v {
                    b"date" => b"%Y-%m-%d".to_vec(),
                    b"seconds" => b"%Y-%m-%d %H:%M:%S%:z".to_vec(),
                    b"ns" => b"%Y-%m-%d %H:%M:%S.%N%:z".to_vec(),
                    _ => return Err("flag: date --rfc-3339 with this argument".into()),
                }),
                _ => return Err(format!("flag: date {} (long options other than the ones listed in the README are not supported)", String::from_utf8_lossy(a))),
            }
            continue;
        }
        if a.len() < 2 || a[0] != b'-' {
            operands.push(a.clone());
            continue;
        }
        let mut j = 1;
        while j < a.len() {
            let c = a[j];
            j += 1;
            match c {
                b'u' => {}
                b'R' => formats.push(b"%a, %d %b %Y %H:%M:%S %z".to_vec()),
                b'I' => {
                    match iso(&a[j..]) {
                        Some(f) => formats.push(f.to_vec()),
                        None => bad_iso = Some(a[j..].to_vec()),
                    }
                    break;
                }
                b'd' => {
                    if j < a.len() {
                        when_text = Some(a[j..].to_vec());
                    } else if i < args.len() {
                        when_text = Some(args[i].clone());
                        i += 1;
                    } else {
                        return Ok(fails("date", "option requires an argument -- 'd'".into(), true, 1).run);
                    }
                    break;
                }
                _ => return Err(format!("flag: date -{}", char::from(c))),
            }
        }
    }
    if let Some(spec) = bad_iso {
        let m = format!("invalid argument {} for '--iso-8601'\nValid arguments are:\n  - 'hours'\n  - 'minutes'\n  - 'date'\n  - 'seconds'\n  - 'ns'", quote(&spec));
        return Ok(fails("date", m, true, 1).run);
    }
    let mut rest = operands.iter();
    if let Some(first) = rest.next() {
        match first.strip_prefix(b"+") {
            Some(f) => formats.push(f.to_vec()),
            None if when_text.is_some() => {
                let m = format!("the argument {} lacks a leading '+';\nwhen using an option to specify date(s), any non-option\nargument must be a format string beginning with '+'", quote(first));
                return Ok(fails("date", m, true, 1).run);
            }
            None => return Err("flag: date with a time to set (the clock cannot be set)".into()),
        }
        if let Some(extra) = rest.next() {
            return Ok(fails("date", format!("extra operand {}", quote(extra)), true, 1).run);
        }
    }
    if formats.len() > 1 {
        return Ok(fails("date", "multiple output formats specified".into(), false, 1).run);
    }
    let format = formats.pop().unwrap_or_else(|| b"%a %b %e %H:%M:%S %Z %Y".to_vec());
    // What cannot be read or written is known now, whatever the time is.
    if let Err(what) = when::format(0, &format) {
        return Err(format!("flag: date +FORMAT with {}", what));
    }
    if let Some(t) = &when_text {
        match parse_when(t, 0) {
            Ok(_) => {}
            Err(true) => return Err("flag: date -d with a date in words (dates are read as YYYY-MM-DD [HH:MM[:SS]], @SECONDS, now, today, yesterday, tomorrow, and steps such as '+2 days' or '3 weeks ago')".into()),
            Err(false) => return Ok(fails("date", format!("invalid date {}", quote(t)), false, 1).run),
        }
    }
    Ok(Box::new(move |db, io| {
        let now = db.now();
        let t = match &when_text {
            Some(text) => match parse_when(text, now) {
                Ok(t) => t,
                Err(_) => {
                    io.err_line(format!("date: invalid date {}", quote(text)));
                    return 1;
                }
            },
            None => now,
        };
        let mut line = when::format(t, &format).unwrap_or_default();
        line.push(b'\n');
        io.put(&line);
        0
    }))
}

// ------------------------------------------------------------------- xargs

/// The most a command line may hold, each word counted with one byte
/// more, as xargs 4.9.0 allows by default.
const LINE_MAX: usize = 131_067;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Split {
    /// Blanks and newlines, with quotes and backslashes.
    Words,
    Byte(u8),
}

struct Xargs {
    max_args: Option<usize>,
    max_lines: Option<usize>,
    replace: Option<Vec<u8>>,
    split: Split,
    no_run_if_empty: bool,
    trace: bool,
    max_chars: usize,
    exit_if_long: bool,
    command: Vec<Vec<u8>>,
    warnings: Vec<String>,
}

/// The input as lines of words, read as xargs reads it without -0 or -d.
/// A line that ends with a blank runs on into the next. With `whole`,
/// each line is one word, without the blanks in front of it. An unmatched
/// quote gives the words before it and which quote it was.
fn words(input: &[u8], whole: bool) -> (Vec<Vec<Vec<u8>>>, Option<&'static str>) {
    let mut lines: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut line: Vec<Vec<u8>> = Vec::new();
    let mut word: Option<Vec<u8>> = None;
    let mut last_blank = false;
    let mut i = 0;
    let blank = |c: u8| c == b' ' || c == b'\t';
    while i < input.len() {
        let c = input[i];
        i += 1;
        match c {
            b'\n' => {
                if let Some(w) = word.take() {
                    line.push(w);
                }
                if !(last_blank && !whole) && !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                last_blank = false;
            }
            _ if blank(c) && (!whole || word.is_none()) => {
                if let Some(w) = word.take() {
                    line.push(w);
                }
                last_blank = true;
            }
            b'\'' | b'"' => {
                let w = word.get_or_insert_with(Vec::new);
                let end = input[i..].iter().position(|x| *x == c || *x == b'\n');
                match end {
                    Some(e) if input[i + e] == c => {
                        w.extend_from_slice(&input[i..i + e]);
                        i += e + 1;
                    }
                    _ => {
                        if !line.is_empty() {
                            lines.push(line);
                        }
                        return (lines, Some(if c == b'\'' { "single" } else { "double" }));
                    }
                }
                last_blank = false;
            }
            b'\\' => {
                let w = word.get_or_insert_with(Vec::new);
                if i < input.len() {
                    w.push(input[i]);
                    i += 1;
                }
                last_blank = false;
            }
            _ => {
                word.get_or_insert_with(Vec::new).push(c);
                last_blank = false;
            }
        }
    }
    if let Some(w) = word.take() {
        line.push(w);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    (lines, None)
}

fn replace_all(word: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return word.to_vec();
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < word.len() {
        if word[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(word[i]);
            i += 1;
        }
    }
    out
}

pub fn xargs(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let mut x = Xargs {
        max_args: None,
        max_lines: None,
        replace: None,
        split: Split::Words,
        no_run_if_empty: false,
        trace: false,
        max_chars: LINE_MAX,
        exit_if_long: false,
        command: Vec::new(),
        warnings: Vec::new(),
    };
    let stop = |m: String, try_line: bool| Ok(fails("xargs", m, try_line, 1));
    let number = |text: &[u8], flag: &str| -> Result<usize, String> {
        match std::str::from_utf8(text).ok().and_then(|s| s.parse::<i64>().ok()) {
            None => Err(format!("invalid number \"{}\" for -{} option", String::from_utf8_lossy(text), flag)),
            Some(n) if n < 1 => Err(format!("value {} for -{} option should be >= 1", n, flag)),
            Some(n) => Ok(n as usize),
        }
    };
    let delimiter = |text: &[u8]| -> Result<u8, String> {
        let bad = || format!("Invalid input delimiter specification {}: the delimiter must be either a single character or an escape sequence starting with \\.", String::from_utf8_lossy(text));
        match text {
            [c] => Ok(*c),
            [b'\\', rest @ ..] if !rest.is_empty() => match rest {
                b"a" => Ok(7),
                b"b" => Ok(8),
                b"f" => Ok(12),
                b"n" => Ok(b'\n'),
                b"r" => Ok(b'\r'),
                b"t" => Ok(b'\t'),
                b"v" => Ok(11),
                b"\\" => Ok(b'\\'),
                [b'x', hex @ ..] if !hex.is_empty() => u8::from_str_radix(std::str::from_utf8(hex).unwrap_or("-"), 16).map_err(|_| bad()),
                oct if oct.iter().all(|c| (b'0'..=b'7').contains(c)) => u8::from_str_radix(std::str::from_utf8(oct).unwrap(), 8).map_err(|_| bad()),
                _ => Err(bad()),
            },
            _ => Err(bad()),
        }
    };
    // What an option does, by its short name.
    enum Opt {
        N(Vec<u8>),
        L(Vec<u8>),
        I(Vec<u8>),
        D(Vec<u8>),
        S(Vec<u8>),
        P(Vec<u8>),
        Flag(u8),
    }
    let mut opts: Vec<Opt> = Vec::new();
    let mut i = 0;
    let missing = |c: &str| Ok(fails("xargs", format!("option requires an argument -- '{}'", c), true, 1));
    while i < args.len() {
        let a = &args[i];
        if a == b"--" {
            i += 1;
            break;
        }
        if a.len() < 2 || a[0] != b'-' {
            break;
        }
        i += 1;
        if let Some(long) = a.strip_prefix(b"--") {
            let (name, value) = match long.iter().position(|c| *c == b'=') {
                Some(eq) => (&long[..eq], Some(long[eq + 1..].to_vec())),
                None => (long, None),
            };
            let need = |i: &mut usize| -> Option<Vec<u8>> {
                if value.is_some() {
                    return value.clone();
                }
                if *i < args.len() {
                    *i += 1;
                    return Some(args[*i - 1].clone());
                }
                None
            };
            let long_missing = || Ok(fails("xargs", format!("option '--{}' requires an argument", String::from_utf8_lossy(name)), true, 1));
            match name {
                b"max-args" => match need(&mut i) {
                    Some(v) => opts.push(Opt::N(v)),
                    None => return long_missing(),
                },
                b"max-chars" => match need(&mut i) {
                    Some(v) => opts.push(Opt::S(v)),
                    None => return long_missing(),
                },
                b"max-procs" => match need(&mut i) {
                    Some(v) => opts.push(Opt::P(v)),
                    None => return long_missing(),
                },
                b"delimiter" => match need(&mut i) {
                    Some(v) => opts.push(Opt::D(v)),
                    None => return long_missing(),
                },
                b"replace" => opts.push(Opt::I(value.unwrap_or_else(|| b"{}".to_vec()))),
                b"max-lines" => opts.push(Opt::L(value.unwrap_or_else(|| b"1".to_vec()))),
                b"null" => opts.push(Opt::Flag(b'0')),
                b"no-run-if-empty" => opts.push(Opt::Flag(b'r')),
                b"verbose" => opts.push(Opt::Flag(b't')),
                b"exit" => opts.push(Opt::Flag(b'x')),
                _ => return Err(format!("flag: xargs {} (long options other than the ones listed in the README are not supported)", String::from_utf8_lossy(a))),
            }
            continue;
        }
        let mut j = 1;
        while j < a.len() {
            let c = a[j];
            j += 1;
            let attached = a[j..].to_vec();
            match c {
                b'0' | b'r' | b't' | b'x' => opts.push(Opt::Flag(c)),
                b'i' => {
                    opts.push(Opt::I(if attached.is_empty() { b"{}".to_vec() } else { attached }));
                    break;
                }
                b'l' => {
                    opts.push(Opt::L(if attached.is_empty() { b"1".to_vec() } else { attached }));
                    break;
                }
                b'n' | b'L' | b'I' | b'd' | b's' | b'P' => {
                    let v = if !attached.is_empty() {
                        attached
                    } else if i < args.len() {
                        i += 1;
                        args[i - 1].clone()
                    } else {
                        return missing(&char::from(c).to_string());
                    };
                    opts.push(match c {
                        b'n' => Opt::N(v),
                        b'L' => Opt::L(v),
                        b'I' => Opt::I(v),
                        b'd' => Opt::D(v),
                        b's' => Opt::S(v),
                        _ => Opt::P(v),
                    });
                    break;
                }
                _ => return Err(format!("flag: xargs -{}", char::from(c))),
            }
        }
    }
    x.command = args[i..].to_vec();
    for o in opts {
        let exclusive = |a: &str, b: &str| format!("xargs: warning: options {} and {} are mutually exclusive, ignoring previous {} value", a, b, a);
        match o {
            Opt::Flag(b'0') => x.split = Split::Byte(0),
            Opt::Flag(b'r') => x.no_run_if_empty = true,
            Opt::Flag(b't') => x.trace = true,
            Opt::Flag(_) => x.exit_if_long = true,
            Opt::N(v) => match number(&v, "n") {
                Ok(n) => {
                    if x.max_lines.take().is_some() {
                        x.warnings.push(exclusive("--max-lines", "--max-args/-n"));
                    }
                    if x.replace.is_some() && n == 1 {
                        continue;
                    }
                    if x.replace.take().is_some() {
                        x.warnings.push(exclusive("--replace", "--max-args/-n"));
                    }
                    x.max_args = Some(n);
                }
                Err(m) => return stop(m, true),
            },
            Opt::L(v) => match number(&v, "L") {
                Ok(n) => {
                    if x.max_args.take().is_some() {
                        x.warnings.push(exclusive("--max-args", "-L"));
                    }
                    if x.replace.take().is_some() {
                        x.warnings.push(exclusive("--replace", "-L"));
                    }
                    x.max_lines = Some(n);
                }
                Err(m) => return stop(m, true),
            },
            Opt::I(v) => {
                if x.max_args.take().is_some() {
                    x.warnings.push(exclusive("--max-args", "--replace/-I/-i"));
                }
                if x.max_lines.take().is_some() {
                    x.warnings.push(exclusive("--max-lines", "--replace/-I/-i"));
                }
                x.replace = Some(v);
            }
            Opt::D(v) => match delimiter(&v) {
                Ok(d) => x.split = Split::Byte(d),
                Err(m) => return stop(m, false),
            },
            Opt::S(v) => match number(&v, "s") {
                Ok(n) => x.max_chars = n.min(LINE_MAX),
                Err(m) => return stop(m, true),
            },
            Opt::P(v) => {
                if v != b"1" {
                    return Err("flag: xargs -P with more than one process (commands run one at a time)".into());
                }
            }
        }
    }
    if x.command.is_empty() {
        x.command.push(b"echo".to_vec());
    }
    if !super::known(&x.command[0]) {
        return Err(format!("command: {}", String::from_utf8_lossy(&x.command[0])));
    }
    let run: Runner = Box::new(move |db, io| xargs_run(&x, db, io));
    Ok(Prepared { run, input: Input::Whole })
}

fn xargs_run(x: &Xargs, db: &mut Db, io: &mut crate::engine::Io) -> i32 {
    for w in &x.warnings {
        io.err_line(w);
    }
    let input = io.take_stdin();
    io.close_stdin();
    let mut status = 0;
    // Run one command. False when xargs must stop.
    let exec = |argv: &[Vec<u8>], db: &mut Db, io: &mut crate::engine::Io, status: &mut i32| -> bool {
        if x.trace {
            let shown: Vec<String> = argv
                .iter()
                .map(|w| {
                    // As the shell would need it: a colon is no reason to quote.
                    let plain: Vec<u8> = w.iter().map(|c| if *c == b':' { b'a' } else { *c }).collect();
                    if quote_if_needed(&plain).as_bytes() == plain.as_slice() {
                        String::from_utf8_lossy(w).into_owned()
                    } else {
                        quote_if_needed(w)
                    }
                })
                .collect();
            io.err_line(shown.join(" "));
        }
        match run_argv(db, argv, io) {
            0 => true,
            255 => {
                io.err_line(format!("xargs: {}: exited with status 255; aborting", String::from_utf8_lossy(&argv[0])));
                *status = 124;
                false
            }
            _ => {
                *status = 123;
                true
            }
        }
    };
    let size = |argv: &[Vec<u8>]| argv.iter().map(|w| w.len() + 1).sum::<usize>();
    let base = size(&x.command);
    // The input as lines of words.
    let (lines, unmatched): (Vec<Vec<Vec<u8>>>, Option<&str>) = match x.split {
        Split::Words => words(input, x.replace.is_some()),
        Split::Byte(d) => {
            let mut items: Vec<Vec<u8>> = input.split(|c| *c == d).map(|s| s.to_vec()).collect();
            if items.last().is_some_and(|l| l.is_empty()) {
                items.pop();
            }
            (items.into_iter().map(|w| vec![w]).collect(), None)
        }
    };
    if let Some(which) = unmatched {
        io.err_line(format!("xargs: unmatched {} quote; by default quotes are special to xargs unless you use the -0 option", which));
        status = 1;
    }
    if let Some(pattern) = &x.replace {
        for line in &lines {
            let item: &[u8] = &line[0];
            if x.split == Split::Words && item.is_empty() {
                continue;
            }
            let argv: Vec<Vec<u8>> = x.command.iter().map(|w| replace_all(w, pattern, item)).collect();
            if size(&argv) > x.max_chars {
                io.err_line("xargs: argument list too long");
                return 1;
            }
            if !exec(&argv, db, io, &mut status) {
                return status;
            }
        }
        return status;
    }
    // Batches: so many lines, so many words, and no longer than allowed.
    let mut argv = x.command.clone();
    let mut used = base;
    let mut lines_in = 0;
    let mut ran = false;
    for line in &lines {
        for word in line {
            if used + word.len() + 1 > x.max_chars {
                if argv.len() == x.command.len() {
                    io.err_line("xargs: argument line too long");
                    return 1;
                }
                if x.exit_if_long && (x.max_args.is_some() || x.max_lines.is_some()) {
                    io.err_line("xargs: argument list too long");
                    return 1;
                }
                ran = true;
                if !exec(&argv, db, io, &mut status) {
                    return status;
                }
                argv.truncate(x.command.len());
                used = base;
                lines_in = 0;
                if used + word.len() + 1 > x.max_chars {
                    io.err_line("xargs: argument line too long");
                    return 1;
                }
            }
            used += word.len() + 1;
            argv.push(word.clone());
            if x.max_args.is_some_and(|n| argv.len() - x.command.len() >= n) {
                ran = true;
                if !exec(&argv, db, io, &mut status) {
                    return status;
                }
                argv.truncate(x.command.len());
                used = base;
                lines_in = 0;
            }
        }
        lines_in += 1;
        if x.max_lines.is_some_and(|n| lines_in >= n) && argv.len() > x.command.len() {
            ran = true;
            if !exec(&argv, db, io, &mut status) {
                return status;
            }
            argv.truncate(x.command.len());
            used = base;
            lines_in = 0;
        }
    }
    if argv.len() > x.command.len() || (!ran && !x.no_run_if_empty && unmatched.is_none()) {
        exec(&argv, db, io, &mut status);
    }
    status
}
