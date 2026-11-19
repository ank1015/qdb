//! awk, as GNU awk 5.2.1 runs the part of the language listed in the
//! README: patterns and actions, fields, the ten string functions,
//! printf, and arrays of one subscript. No user functions, no getline,
//! no output to anything but standard output. Input is bytes, as in the C
//! locale: `length`, `substr`, `index`, `toupper` and `tolower` count and
//! change bytes, and only ASCII letters have a case.

mod parse;
mod run;

use std::rc::Rc;

use crate::bytes::memchr;
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use crate::regex::Regex;
use crate::util::lossy;
use parse::{Parser, Program, Stop};
use run::{Rt, Value};

const USAGE: &str = include_str!("usage.txt");

struct Setup {
    prog: Rc<Program>,
    res: Rc<Vec<Regex>>,
    fs: Option<Vec<u8>>,
    assigns: Vec<(String, Vec<u8>)>,
}

/// The escapes of a string given on the command line.
fn unescape(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i] == b'\\' && i + 1 < text.len() {
            match run::byte_escape(text, i + 1) {
                Some((b, end)) => {
                    out.push(b);
                    i = end;
                }
                None => {
                    out.push(text[i + 1]);
                    i += 2;
                }
            }
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    out
}

fn fails(lines: Vec<String>, usage: bool, status: i32) -> Prepared {
    let run: Runner = Box::new(move |_, io: &mut Io| {
        for l in &lines {
            io.err_line(l);
        }
        if usage {
            io.err.extend_from_slice(USAGE.as_bytes());
        }
        status
    });
    Prepared::from(run)
}

fn is_name(n: &[u8]) -> bool {
    !n.is_empty() && !n[0].is_ascii_digit() && n.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

struct Machine {
    rt: Rt,
    setup: Rc<Setup>,
    /// The start of a line whose end has not come yet.
    partial: Vec<u8>,
    begun: bool,
}

impl Machine {
    fn new(setup: Rc<Setup>) -> Machine {
        Machine { rt: Rt::new(setup.prog.clone(), setup.res.clone()), setup, partial: Vec::new(), begun: false }
    }

    fn begin(&mut self, io: &mut Io) {
        if self.begun {
            return;
        }
        self.begun = true;
        let setup = Rc::clone(&self.setup);
        for w in &setup.prog.warnings {
            io.err_line(w);
        }
        if let Some(i) = setup.prog.uses.iter().position(|u| u.0 && u.1) {
            let name = &setup.prog.names[i];
            let text = if setup.prog.array_first[i] {
                format!("awk: cmd. line:1: fatal: attempt to use array `{}' in a scalar context", name)
            } else {
                format!("awk: cmd. line:1: fatal: attempt to use scalar `{}' as an array", name)
            };
            self.rt.fatal = Some(text.into_bytes());
            return;
        }
        if let Some(fs) = &setup.fs {
            self.rt.set_var("FS", Value::Str(Rc::from(&fs[..])));
        }
        for (name, value) in &setup.assigns {
            self.rt.set_var(name, Value::StrNum(Rc::from(&value[..])));
        }
        for body in &setup.prog.begin {
            if self.rt.run(body, io) == run::Flow::Stop {
                break;
            }
        }
    }

    fn lines(&mut self, data: &[u8], io: &mut Io) -> bool {
        let mut rest = data;
        if !self.partial.is_empty() {
            match memchr(b'\n', rest) {
                None => {
                    self.partial.extend_from_slice(rest);
                    return true;
                }
                Some(i) => {
                    self.partial.extend_from_slice(&rest[..i]);
                    let mut line = std::mem::take(&mut self.partial);
                    let more = self.rt.record(&line, io);
                    line.clear();
                    self.partial = line;
                    if !more {
                        return false;
                    }
                    rest = &rest[i + 1..];
                }
            }
        }
        while let Some(i) = memchr(b'\n', rest) {
            if !self.rt.record(&rest[..i], io) {
                return false;
            }
            rest = &rest[i + 1..];
        }
        self.partial.extend_from_slice(rest);
        true
    }

    fn feed(&mut self, data: &[u8], io: &mut Io) -> bool {
        if self.rt.dead() {
            return false;
        }
        self.lines(data, io)
    }

    /// The end of a file ends its last line.
    fn file_end(&mut self, io: &mut Io) {
        if !self.partial.is_empty() && !self.rt.dead() {
            let mut line = std::mem::take(&mut self.partial);
            self.rt.record(&line, io);
            line.clear();
            self.partial = line;
        }
        self.partial.clear();
    }

    fn end(&mut self, io: &mut Io) -> i32 {
        if !self.rt.exiting {
            self.file_end(io);
        }
        // After `exit` in BEGIN or in a rule the END rules still run; an
        // `exit` in one of them ends the program there.
        if !self.rt.stopped && self.rt.fatal.is_none() {
            self.rt.exiting = false;
            let setup = Rc::clone(&self.setup);
            for body in &setup.prog.end {
                if self.rt.run(body, io) == run::Flow::Stop {
                    break;
                }
            }
        }
        self.rt.finish_output(io);
        match self.rt.fatal.take() {
            Some(m) => {
                io.err_line(m);
                2
            }
            None => self.rt.exit_code,
        }
    }
}

impl Filter for Machine {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        if !self.begun {
            self.begin(io);
            self.rt.file_starts(b"-");
        }
        let more = self.feed(data, io);
        self.rt.finish_output(io);
        more && !self.rt.dead()
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        if !self.begun {
            self.begin(io);
            self.rt.file_starts(b"-");
        }
        self.end(io)
    }
}

pub fn awk(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let mut fs: Option<Vec<u8>> = None;
    let mut assigns: Vec<(String, Vec<u8>)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == b"--" {
            i += 1;
            break;
        }
        let (flag, value): (u8, Option<Vec<u8>>) = if a.starts_with(b"--") {
            let (name, value) = match a.iter().position(|c| *c == b'=') {
                Some(k) => (&a[2..k], Some(a[k + 1..].to_vec())),
                None => (&a[2..], None),
            };
            match name {
                b"field-separator" => (b'F', value),
                b"assign" => (b'v', value),
                _ => return Err(format!("flag: awk --{}", lossy(name))),
            }
        } else if a.len() >= 2 && a[0] == b'-' {
            if a[1] != b'F' && a[1] != b'v' {
                return Err(format!("flag: awk -{}", a[1] as char));
            }
            (a[1], if a.len() > 2 { Some(a[2..].to_vec()) } else { None })
        } else {
            break;
        };
        i += 1;
        let value = match value {
            Some(v) => v,
            None => {
                if i >= args.len() {
                    return Ok(fails(vec![format!("awk: option requires an argument -- {}", flag as char)], true, 1));
                }
                i += 1;
                args[i - 1].clone()
            }
        };
        if flag == b'F' {
            fs = Some(unescape(&value));
            continue;
        }
        let eq = match value.iter().position(|c| *c == b'=') {
            Some(k) => k,
            None => return Ok(fails(vec![format!("awk: `{}' argument to `-v' not in `var=value' form", lossy(&value)), String::new()], true, 1)),
        };
        if !is_name(&value[..eq]) {
            return Ok(fails(vec![format!("awk: fatal: `{}' is not a legal variable name", lossy(&value[..eq]))], false, 2));
        }
        assigns.push((lossy(&value[..eq]).into_owned(), unescape(&value[eq + 1..])));
    }
    if i >= args.len() {
        return Ok(fails(Vec::new(), true, 1));
    }
    let text = args[i].clone();
    let files: Vec<Vec<u8>> = args[i + 1..].to_vec();
    for f in &files {
        if let Some(k) = f.iter().position(|c| *c == b'=') {
            if is_name(&f[..k]) {
                return Err(format!("awk feature: an assignment among the file names ({})", lossy(f)));
            }
        }
    }
    let prog = match Parser::new(&text).program() {
        Ok(p) => p,
        Err(Stop::Unsupported(what)) => return Err(what),
        Err(Stop::Syntax(lines, status)) => return Ok(fails(lines, false, status)),
    };
    for (name, _) in &assigns {
        if parse::refused_variable(name) {
            return Err(format!("awk variable: {}", name));
        }
    }
    let mut res = Vec::with_capacity(prog.regexes.len());
    for (re, line) in &prog.regexes {
        match run::compile(re) {
            Ok(r) => res.push(r),
            Err(m) => return Ok(fails(vec![format!("awk: cmd. line:{}: error: {}: /{}/", line, m, lossy(re))], false, 1)),
        }
    }
    let reads = !prog.rules.is_empty() || prog.has_end;
    let setup = Rc::new(Setup { prog: Rc::new(prog), res: Rc::new(res), fs, assigns });
    if !reads {
        let run: Runner = Box::new(move |_, io: &mut Io| {
            let mut m = Machine::new(setup);
            m.begin(io);
            m.end(io)
        });
        return Ok(Prepared::from(run));
    }
    if files.is_empty() {
        let again = Rc::clone(&setup);
        let run: Runner = Box::new(move |_, io: &mut Io| {
            let mut m = Box::new(Machine::new(again));
            let stdin = io.take_stdin();
            m.push(stdin, io);
            m.finish(io)
        });
        return Ok(Prepared { run, input: Input::Stream(Box::new(Machine::new(setup))), writes: false });
    }
    let input = if files.iter().any(|f| f == b"-") { Input::Whole } else { Input::Unused };
    let run: Runner = Box::new(move |db, io: &mut Io| {
        let mut m = Machine::new(setup);
        m.begin(io);
        for name in &files {
            if m.rt.dead() {
                break;
            }
            if name == b"-" {
                m.rt.file_starts(b"-");
                let stdin = io.take_stdin();
                m.feed(stdin, io);
                m.file_end(io);
                continue;
            }
            match db.resolve(name) {
                Err(e) => {
                    let mut t = if m.rt.ran { format!("awk: cmd. line:{}: fatal: cannot open file `", m.rt.line()).into_bytes() } else { b"awk: fatal: cannot open file `".to_vec() };
                    t.extend_from_slice(name);
                    t.extend_from_slice(format!("' for reading: {}", e.msg()).as_bytes());
                    m.rt.fatal = Some(t);
                }
                Ok(id) if db.is_dir(id) => {
                    m.rt.finish_output(io);
                    let mut t = b"awk: warning: command line argument `".to_vec();
                    t.extend_from_slice(name);
                    t.extend_from_slice(b"' is a directory: skipped");
                    io.err_line(t);
                }
                Ok(id) => {
                    m.rt.file_starts(name);
                    for p in db.pieces(id) {
                        let text = db.text_of(p);
                        if !m.feed(&text, io) {
                            break;
                        }
                    }
                    m.file_end(io);
                }
            }
        }
        m.end(io)
    });
    Ok(Prepared { run, input, writes: false })
}
