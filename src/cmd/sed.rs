//! sed with one substitution command: `s/old/new/` with the optional `g`
//! flag, and the options -i and -E.

use super::options;
use crate::engine::Runner;
use crate::regex::{Flavor, MatchError, Regex, Syntax};
use crate::tree::is_valid_text;
use crate::util::{lines, lossy};

#[derive(Debug)]
enum Rep {
    Lit(Vec<u8>),
    Group(usize),
}

struct Subst {
    re: Regex,
    rep: Vec<Rep>,
    global: bool,
    want_groups: bool,
}

enum ScriptError {
    /// Valid sed that this version does not support.
    Unsupported(String),
    /// An error GNU sed reports: the character position and the message.
    Gnu(usize, String),
}

use ScriptError::{Gnu, Unsupported};

const UNTERMINATED: &str = "unterminated `s' command";

fn escape_char(c: u8) -> Option<u8> {
    Some(match c {
        b'n' => b'\n',
        b't' => b'\t',
        b'f' => 0x0c,
        b'v' => 0x0b,
        b'a' => 0x07,
        _ => return None,
    })
}

fn parse_script(s: &[u8], extended: bool) -> Result<Subst, ScriptError> {
    let len = s.len();
    let mut i = 0;
    while i < len && (s[i].is_ascii_whitespace() || s[i] == b';') {
        i += 1;
    }
    match s.get(i) {
        None => return Err(Unsupported("sed script: an empty script".into())),
        Some(b's') => {}
        Some(c) if c.is_ascii_digit() || matches!(c, b'/' | b'$' | b'\\') => {
            return Err(Unsupported("sed script: addresses (line numbers, ranges and /regex/ selectors)".into()))
        }
        Some(b'{') => return Err(Unsupported("sed script: command blocks { }".into())),
        Some(b'#') => return Err(Unsupported("sed script: a script with no command".into())),
        Some(c) => {
            return Err(Unsupported(format!(
                "sed script: the `{}` command (only s/old/new/ is supported)",
                char::from(*c)
            )))
        }
    }
    i += 1;
    let delim = *s.get(i).ok_or_else(|| Gnu(len, UNTERMINATED.into()))?;
    if delim == b'\n' || delim == b'\\' {
        return Err(Unsupported("sed script: newline or backslash as the s delimiter".into()));
    }
    i += 1;
    let bad_escape = |c: u8| Unsupported(format!("sed script: the \\{} escape", char::from(c)));

    // The pattern, up to the next unescaped delimiter outside a bracket
    // expression.
    let mut pat = Vec::new();
    loop {
        let c = *s.get(i).ok_or_else(|| Gnu(len, UNTERMINATED.into()))?;
        if c == b'\n' {
            return Err(Gnu(i + 1, UNTERMINATED.into()));
        }
        i += 1;
        if c == delim {
            break;
        }
        if c == b'\\' {
            let n = *s.get(i).ok_or_else(|| Gnu(len, UNTERMINATED.into()))?;
            i += 1;
            if n == delim || n == b'\n' {
                pat.push(n);
            } else if let Some(e) = escape_char(n) {
                pat.push(e);
            } else if matches!(n, b'c' | b'd' | b'o' | b'x') {
                return Err(bad_escape(n));
            } else {
                pat.extend_from_slice(&[b'\\', n]);
            }
            continue;
        }
        pat.push(c);
        if c != b'[' {
            continue;
        }
        for lead in [b'^', b']'] {
            if s.get(i) == Some(&lead) {
                pat.push(lead);
                i += 1;
            }
        }
        loop {
            let c = *s.get(i).ok_or_else(|| Gnu(len, UNTERMINATED.into()))?;
            if c == b'\n' {
                return Err(Gnu(i + 1, UNTERMINATED.into()));
            }
            i += 1;
            if c == b'\\' {
                match s.get(i).and_then(|n| escape_char(*n)) {
                    Some(e) => {
                        pat.push(e);
                        i += 1;
                    }
                    None => pat.push(b'\\'),
                }
                continue;
            }
            pat.push(c);
            if c == b']' {
                break;
            }
            if c == b'[' && matches!(s.get(i), Some(b':') | Some(b'.') | Some(b'=')) {
                let kind = s[i];
                if let Some(close) = (i + 1..len.saturating_sub(1)).find(|&j| s[j] == kind && s[j + 1] == b']') {
                    pat.extend_from_slice(&s[i..close + 2]);
                    i = close + 2;
                }
            }
        }
    }

    // The replacement.
    let mut rep: Vec<Rep> = Vec::new();
    let mut lit: Vec<u8> = Vec::new();
    let mut max_ref = 0usize;
    loop {
        let c = *s.get(i).ok_or_else(|| Gnu(len, UNTERMINATED.into()))?;
        if c == b'\n' {
            return Err(Gnu(i + 1, UNTERMINATED.into()));
        }
        i += 1;
        if c == delim {
            break;
        }
        let group = if c == b'&' {
            Some(0)
        } else if c == b'\\' {
            let n = *s.get(i).ok_or_else(|| Gnu(len, UNTERMINATED.into()))?;
            i += 1;
            if n == delim {
                lit.push(n);
                None
            } else if n.is_ascii_digit() {
                Some((n - b'0') as usize)
            } else if let Some(e) = escape_char(n) {
                lit.push(e);
                None
            } else if matches!(n, b'L' | b'U' | b'l' | b'u' | b'E') {
                return Err(Unsupported("sed script: case conversion (\\U, \\L, \\u, \\l, \\E) in the replacement".into()));
            } else if matches!(n, b'c' | b'd' | b'o' | b'x') {
                return Err(bad_escape(n));
            } else {
                lit.push(n);
                None
            }
        } else {
            lit.push(c);
            None
        };
        if let Some(g) = group {
            if !lit.is_empty() {
                rep.push(Rep::Lit(std::mem::take(&mut lit)));
            }
            max_ref = max_ref.max(g);
            rep.push(Rep::Group(g));
        }
    }
    if !lit.is_empty() {
        rep.push(Rep::Lit(lit));
    }

    // Flags.
    let mut global = false;
    while let Some(&c) = s.get(i) {
        match c {
            b'g' => {
                if global {
                    return Err(Gnu(i + 1, "multiple `g' options to `s' command".into()));
                }
                global = true;
                i += 1;
            }
            b' ' | b'\t' | b'\r' => i += 1,
            b';' | b'\n' => {
                i += 1;
                break;
            }
            b'#' => break,
            b'}' => return Err(Gnu(i + 1, "unexpected `}'".into())),
            b'p' | b'i' | b'I' | b'm' | b'M' | b'e' | b'w' | b'0'..=b'9' => {
                return Err(Unsupported(format!(
                    "sed script: the `{}` flag of s (only g is supported)",
                    char::from(c)
                )))
            }
            _ => return Err(Gnu(i + 1, "unknown option to `s'".into())),
        }
    }
    let end = i;
    // Nothing but white space and comments may follow.
    while i < len {
        match s[i] {
            b'#' => {
                while i < len && s[i] != b'\n' {
                    i += 1;
                }
            }
            c if c.is_ascii_whitespace() || c == b';' => i += 1,
            _ => return Err(Unsupported("sed script: more than one command".into())),
        }
    }
    if pat.is_empty() {
        return Err(Gnu(0, "no previous regular expression".into()));
    }
    let re = Regex::new(&pat, Syntax { extended, icase: false, flavor: Flavor::Sed }).map_err(|m| Gnu(end, m))?;
    if max_ref > re.group_count() {
        return Err(Gnu(end, format!("invalid reference \\{} on `s' command's RHS", max_ref)));
    }
    Ok(Subst { re, rep, global, want_groups: max_ref > 0 })
}

impl Subst {
    /// Apply the substitution to one line (without its newline).
    fn apply(&self, line: &[u8]) -> Result<Vec<u8>, MatchError> {
        let mut out = Vec::with_capacity(line.len());
        let mut start = 0;
        let mut prev_end: Option<usize> = None;
        while start <= line.len() {
            let caps = match self.re.find_at(line, start, self.want_groups)? {
                None => break,
                Some(c) => c,
            };
            let (s, e) = caps[0].unwrap();
            out.extend_from_slice(&line[start..s]);
            // An empty match right after the previous match is not a match.
            let skip = s == e && prev_end == Some(s);
            if !skip {
                for r in &self.rep {
                    match r {
                        Rep::Lit(l) => out.extend_from_slice(l),
                        Rep::Group(g) => {
                            if let Some((a, b)) = caps[*g] {
                                out.extend_from_slice(&line[a..b]);
                            }
                        }
                    }
                }
                prev_end = Some(e);
            }
            if s == e {
                if s < line.len() {
                    out.push(line[s]);
                }
                start = e + 1;
            } else {
                start = e;
            }
            if !skip && !self.global {
                break;
            }
        }
        if start < line.len() {
            out.extend_from_slice(&line[start..]);
        }
        Ok(out)
    }

    /// Apply to every line of `data`, appending to `out`. A missing final
    /// newline stays missing.
    fn apply_all(&self, data: &[u8], out: &mut Vec<u8>) -> Result<(), MatchError> {
        for line in lines(data) {
            if !out.is_empty() && !out.ends_with(b"\n") {
                out.push(b'\n');
            }
            let text = line.strip_suffix(b"\n").unwrap_or(line);
            out.extend_from_slice(&self.apply(text)?);
            if line.ends_with(b"\n") {
                out.push(b'\n');
            }
        }
        Ok(())
    }
}

pub fn sed(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    // GNU reads anything attached to -i as a backup suffix.
    for a in args.iter().take_while(|a| a.as_slice() != b"--") {
        if a.len() > 2 && a[0] == b'-' && a[1] != b'-' {
            if let Some(pos) = a.iter().position(|c| *c == b'i') {
                if pos + 1 < a.len() {
                    return Err("flag: sed -i with a backup suffix".into());
                }
            }
        }
    }
    let p = match options("sed", &args, b"iE", b"", 1)? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let in_place = p.has(b'i');
    let extended = p.has(b'E');
    let mut operands = p.operands.into_iter();
    let script = match operands.next() {
        Some(s) => s,
        None => {
            return Ok(Box::new(|_, io| {
                io.err_line("Usage: sed [OPTION]... {script-only-if-no-other-script} [input-file]...");
                1
            }))
        }
    };
    let files: Vec<Vec<u8>> = operands.collect();
    let subst = match parse_script(&script, extended) {
        Ok(s) => s,
        Err(Unsupported(what)) => return Err(what),
        Err(Gnu(pos, m)) => {
            return Ok(Box::new(move |_, io| {
                io.err_line(format!("sed: -e expression #1, char {}: {}", pos, m));
                1
            }))
        }
    };
    Ok(Box::new(move |db, io| {
        let too_complex = "pattern too complex for the backtracking matcher";
        let mut status = 0;
        if in_place {
            if files.is_empty() {
                io.err_line("sed: no input files");
                return 4;
            }
            for f in &files {
                let id = match db.resolve(f) {
                    Err(e) => {
                        io.err_line(format!("sed: can't read {}: {}", lossy(f), e.msg()));
                        status = status.max(2);
                        continue;
                    }
                    Ok(id) if db.is_dir(id) => {
                        io.err_line(format!("sed: couldn't edit {}: not a regular file", lossy(f)));
                        status = status.max(4);
                        continue;
                    }
                    Ok(id) => id,
                };
                let mut out = Vec::new();
                if subst.apply_all(db.content(id), &mut out).is_err() {
                    io.err_line(format!("qdb: sed: {}: {}", lossy(f), too_complex));
                    status = status.max(1);
                } else if !is_valid_text(&out) {
                    io.err_line(format!("qdb: sed: {}: result is not valid UTF-8 text, file left unchanged", lossy(f)));
                    status = status.max(1);
                } else {
                    db.set_content(id, out);
                }
            }
            return status;
        }
        let mut out = Vec::new();
        let inputs = if files.is_empty() { vec![b"-".to_vec()] } else { files };
        for f in &inputs {
            let data: &[u8] = if f == b"-" {
                io.stdin
            } else {
                match db.resolve(f) {
                    Err(e) => {
                        io.err_line(format!("sed: can't read {}: {}", lossy(f), e.msg()));
                        status = status.max(2);
                        continue;
                    }
                    Ok(id) if db.is_dir(id) => {
                        io.err_line(format!("sed: read error on {}: Is a directory", lossy(f)));
                        status = status.max(4);
                        continue;
                    }
                    Ok(id) => db.content(id),
                }
            };
            if subst.apply_all(data, &mut out).is_err() {
                io.err_line(format!("qdb: sed: {}: {}", lossy(f), too_complex));
                status = status.max(1);
            }
        }
        io.out = out;
        status
    }))
}
