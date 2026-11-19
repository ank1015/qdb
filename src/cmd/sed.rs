//! sed, a line at a time: addresses, blocks and the commands p d s q Q =
//! a i c y. The hold space, multi-line commands, branches and reading or
//! writing other files are not supported.

use super::LineBuf;
use crate::engine::{Filter, Input, Io, Prepared, Runner};
use crate::regex::{Flavor, MatchError, Regex, Syntax};
use crate::scan::Searcher;
use crate::tree::{is_valid_text, Db};
use crate::util::{basename, lossy};
use std::borrow::Cow;
use std::rc::Rc;

// ------------------------------------------------------------- the script

#[derive(Debug)]
enum Rep {
    Lit(Vec<u8>),
    Group(usize),
}

struct Subst {
    /// The pattern, as an index into the program's patterns; None is the
    /// empty pattern, which stands for the last one used.
    re: Option<usize>,
    rep: Vec<Rep>,
    global: bool,
    /// Replace from this match on (1 unless a number flag was given).
    nth: usize,
    print: bool,
    want_groups: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Addr {
    Line(u64),
    Last,
    Re(Option<usize>),
    /// first~step
    Step(u64, u64),
    /// addr1,+N
    Plus(u64),
    /// The 0 of 0,/re/.
    Zero,
}

enum Kind {
    /// `{`: where to carry on when the block is not entered.
    Open(usize),
    Close,
    Print,
    Delete,
    Subst(Subst),
    Quit { code: i32, print: bool },
    LineNo,
    Append(Vec<u8>),
    Insert(Vec<u8>),
    Change(Vec<u8>),
    Trans(Box<[u8; 256]>),
}

struct Cmd {
    a1: Option<Addr>,
    a2: Option<Addr>,
    not: bool,
    kind: Kind,
    /// Which -e it came from, counted from 1.
    expr: usize,
}

struct Program {
    cmds: Vec<Cmd>,
    res: Vec<Regex>,
    quiet: bool,
    /// Some command asks whether a line is the last one.
    uses_last: bool,
}

enum ScriptError {
    /// Valid sed that this version does not support.
    Unsupported(String),
    /// An error GNU sed reports: where in the script, and the message.
    Gnu(usize, String),
    /// The same with the position given as an expression number, char 0.
    GnuAt(usize, String),
}

use ScriptError::{Gnu, GnuAt, Unsupported};

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

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    extended: bool,
    /// Where each -e starts in `s`.
    starts: &'a [usize],
    res: Vec<Regex>,
    cmds: Vec<Cmd>,
    open: Vec<usize>,
    uses_last: bool,
}

impl<'a> Parser<'a> {
    fn expr_of(&self, at: usize) -> usize {
        self.starts.iter().rposition(|s| *s <= at).unwrap_or(0) + 1
    }

    fn spaces(&mut self) {
        while matches!(self.s.get(self.i), Some(b' ' | b'\t' | b'\r')) {
            self.i += 1;
        }
    }

    fn number(&mut self) -> u64 {
        let mut n: u64 = 0;
        while let Some(c) = self.s.get(self.i).filter(|c| c.is_ascii_digit()) {
            n = n.saturating_mul(10).saturating_add((c - b'0') as u64);
            self.i += 1;
        }
        n
    }

    /// A pattern up to the next unescaped `delim` outside a bracket
    /// expression. `what` names the thing in "unterminated ...".
    fn pattern(&mut self, delim: u8, what: &str) -> Result<Vec<u8>, ScriptError> {
        let s = self.s;
        let len = s.len();
        let unterminated = |at: usize| Gnu(at, format!("unterminated {}", what));
        let bad_escape = |c: u8| Unsupported(format!("sed script: the \\{} escape", char::from(c)));
        let mut pat = Vec::new();
        let mut i = self.i;
        loop {
            let c = *s.get(i).ok_or_else(|| unterminated(len))?;
            if c == b'\n' {
                return Err(unterminated(i));
            }
            i += 1;
            if c == delim {
                break;
            }
            if c == b'\\' {
                let n = *s.get(i).ok_or_else(|| unterminated(len))?;
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
                let c = *s.get(i).ok_or_else(|| unterminated(len))?;
                if c == b'\n' {
                    return Err(unterminated(i));
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
        self.i = i;
        Ok(pat)
    }

    fn compile(&mut self, pat: &[u8], icase: bool, at: usize) -> Result<Option<usize>, ScriptError> {
        if pat.is_empty() {
            return Ok(None);
        }
        let re = Regex::new(pat, Syntax { extended: self.extended, icase, flavor: Flavor::Sed }).map_err(|m| Gnu(at, m))?;
        self.res.push(re);
        Ok(Some(self.res.len() - 1))
    }

    /// An address, if one starts here. `second` is the address after a comma.
    fn addr(&mut self, second: bool) -> Result<Option<Addr>, ScriptError> {
        let c = match self.s.get(self.i) {
            Some(c) => *c,
            None => return Ok(None),
        };
        Ok(Some(match c {
            b'0'..=b'9' => {
                let n = self.number();
                if !second && self.s.get(self.i) == Some(&b'~') {
                    self.i += 1;
                    Addr::Step(n, self.number())
                } else if n == 0 && !second {
                    Addr::Zero
                } else {
                    Addr::Line(n)
                }
            }
            b'+' | b'~' if !second => {
                self.i += 1;
                self.number();
                return Err(Gnu(self.i, "invalid usage of +N or ~N as first address".into()));
            }
            b'+' => {
                self.i += 1;
                Addr::Plus(self.number())
            }
            b'~' => return Err(Unsupported("sed script: an addr1,~N address".into())),
            b'$' => {
                self.i += 1;
                self.uses_last = true;
                Addr::Last
            }
            b'/' | b'\\' => {
                self.i += 1;
                let delim = if c == b'/' {
                    b'/'
                } else {
                    let d = *self.s.get(self.i).ok_or_else(|| Gnu(self.s.len(), "unexpected `,'".into()))?;
                    self.i += 1;
                    d
                };
                if delim == b'\n' || delim == b'\\' {
                    return Err(Unsupported("sed script: newline or backslash as a regex delimiter".into()));
                }
                let pat = self.pattern(delim, "address regex")?;
                let mut icase = false;
                loop {
                    self.spaces();
                    match self.s.get(self.i) {
                        Some(b'I') => {
                            icase = true;
                            self.i += 1;
                        }
                        Some(b'M') => return Err(Unsupported("sed script: the M flag of an address".into())),
                        _ => break,
                    }
                }
                Addr::Re(self.compile(&pat, icase, self.i)?)
            }
            _ => return Ok(None),
        }))
    }

    /// What may follow a command: nothing but white space up to a `;`, a
    /// newline, a comment or a `}`.
    fn end(&mut self) -> Result<(), ScriptError> {
        self.spaces();
        match self.s.get(self.i) {
            None | Some(b'\n' | b';' | b'#' | b'}') => Ok(()),
            Some(_) => Err(Gnu(self.i + 1, "extra characters after command".into())),
        }
    }

    /// The text of a, i or c.
    fn text(&mut self) -> Result<Vec<u8>, ScriptError> {
        let s = self.s;
        self.spaces();
        match s.get(self.i) {
            None | Some(b'\n') => return Err(Gnu(self.i, "expected \\ after `a', `c' or `i'".into())),
            Some(b'\\') => {
                self.i += 1;
                if s.get(self.i) == Some(&b'\n') {
                    self.i += 1;
                }
            }
            Some(_) => {}
        }
        let mut text = Vec::new();
        let from = self.i;
        while let Some(&c) = s.get(self.i) {
            self.i += 1;
            if c == b'\n' {
                break;
            }
            if c != b'\\' {
                text.push(c);
                continue;
            }
            match s.get(self.i) {
                // A backslash that ends the script is dropped.
                None => break,
                Some(&n) => {
                    self.i += 1;
                    text.push(escape_char(n).unwrap_or(n));
                }
            }
        }
        // Text there was, even a backslash that came to nothing, is a line.
        if self.i > from {
            text.push(b'\n');
        }
        Ok(text)
    }

    fn subst(&mut self) -> Result<Subst, ScriptError> {
        let s = self.s;
        let len = s.len();
        let unterminated = |at: usize| Gnu(at, "unterminated `s' command".to_string());
        let delim = *s.get(self.i).ok_or_else(|| unterminated(len))?;
        if delim == b'\n' || delim == b'\\' {
            return Err(Unsupported("sed script: newline or backslash as the s delimiter".into()));
        }
        self.i += 1;
        let pat = self.pattern(delim, "`s' command")?;
        let mut i = self.i;
        let mut rep: Vec<Rep> = Vec::new();
        let mut lit: Vec<u8> = Vec::new();
        let mut max_ref = 0usize;
        loop {
            let c = *s.get(i).ok_or_else(|| unterminated(len))?;
            if c == b'\n' {
                return Err(unterminated(i));
            }
            i += 1;
            if c == delim {
                break;
            }
            let group = if c == b'&' {
                Some(0)
            } else if c == b'\\' {
                let n = *s.get(i).ok_or_else(|| unterminated(len))?;
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
                    return Err(Unsupported(format!("sed script: the \\{} escape", char::from(n))));
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
        let (mut global, mut print, mut icase, mut nth) = (false, false, false, None);
        while let Some(&c) = s.get(i) {
            match c {
                b'g' => {
                    i += 1;
                    if global {
                        return Err(Gnu(i, "multiple `g' options to `s' command".into()));
                    }
                    global = true;
                }
                b'p' => {
                    i += 1;
                    if print {
                        return Err(Gnu(i, "multiple `p' options to `s' command".into()));
                    }
                    print = true;
                }
                b'i' | b'I' => {
                    i += 1;
                    icase = true;
                }
                b'0'..=b'9' => {
                    self.i = i;
                    let n = self.number();
                    i = self.i;
                    if nth.is_some() {
                        return Err(Gnu(i, "multiple number options to `s' command".into()));
                    }
                    if n == 0 {
                        return Err(Gnu(i, "number option to `s' command may not be zero".into()));
                    }
                    nth = Some(n as usize);
                }
                b' ' | b'\t' | b'\r' => i += 1,
                b';' | b'\n' | b'#' | b'}' => break,
                b'm' | b'M' | b'e' | b'w' => return Err(Unsupported(format!("sed script: the `{}` flag of s", char::from(c)))),
                _ => return Err(Gnu(i + 1, "unknown option to `s'".into())),
            }
        }
        self.i = i;
        let re = self.compile(&pat, icase, i)?;
        if let Some(r) = re {
            if max_ref > self.res[r].group_count() {
                return Err(Gnu(i, format!("invalid reference \\{} on `s' command's RHS", max_ref)));
            }
        }
        Ok(Subst { re, rep, global, nth: nth.unwrap_or(1), print, want_groups: max_ref > 0 })
    }

    fn trans(&mut self) -> Result<Box<[u8; 256]>, ScriptError> {
        let s = self.s;
        let unterminated = |at: usize| Gnu(at, "unterminated `y' command".to_string());
        let delim = *s.get(self.i).ok_or_else(|| unterminated(s.len()))?;
        if delim == b'\n' || delim == b'\\' {
            return Err(Unsupported("sed script: newline or backslash as the y delimiter".into()));
        }
        self.i += 1;
        let mut parts: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        for part in &mut parts {
            loop {
                let c = *s.get(self.i).ok_or_else(|| unterminated(s.len()))?;
                if c == b'\n' {
                    return Err(unterminated(self.i));
                }
                self.i += 1;
                if c == delim {
                    break;
                }
                if c == b'\\' {
                    let n = *s.get(self.i).ok_or_else(|| unterminated(s.len()))?;
                    self.i += 1;
                    part.push(if n == b'n' { b'\n' } else { n });
                } else {
                    part.push(c);
                }
            }
        }
        if parts[0].len() != parts[1].len() {
            return Err(Gnu(self.i, "strings for `y' command are different lengths".into()));
        }
        let mut map = Box::new([0u8; 256]);
        for (i, m) in map.iter_mut().enumerate() {
            *m = i as u8;
        }
        // The last of two entries for one byte wins.
        for (from, to) in parts[0].iter().zip(&parts[1]) {
            map[*from as usize] = *to;
        }
        Ok(map)
    }

    fn parse(&mut self) -> Result<(), ScriptError> {
        let s = self.s;
        loop {
            while matches!(s.get(self.i), Some(b' ' | b'\t' | b'\r' | b'\n' | b';')) {
                self.i += 1;
            }
            if self.i >= s.len() {
                break;
            }
            if s[self.i] == b'#' {
                while self.i < s.len() && s[self.i] != b'\n' {
                    self.i += 1;
                }
                continue;
            }
            let expr = self.expr_of(self.i);
            let a1 = self.addr(false)?;
            let mut a2 = None;
            if a1.is_some() {
                self.spaces();
                if s.get(self.i) == Some(&b',') {
                    self.i += 1;
                    self.spaces();
                    a2 = self.addr(true)?;
                    if a2.is_none() {
                        return Err(Gnu((self.i + 1).min(s.len()), "unexpected `,'".into()));
                    }
                }
            }
            self.spaces();
            let mut not = false;
            if s.get(self.i) == Some(&b'!') {
                not = true;
                self.i += 1;
                self.spaces();
                if s.get(self.i) == Some(&b'!') {
                    return Err(Gnu(self.i + 1, "multiple `!'s".into()));
                }
            }
            let c = *s.get(self.i).ok_or_else(|| Gnu(s.len(), "missing command".into()))?;
            self.i += 1;
            let at = self.i;
            if a1 == Some(Addr::Zero) && !matches!(a2, Some(Addr::Re(_))) {
                return Err(Gnu(at, "invalid usage of line address 0".into()));
            }
            let refuse = |what: String| Err(Unsupported(format!("sed script: {}", what)));
            let kind = match c {
                b'{' => {
                    self.open.push(self.cmds.len());
                    Kind::Open(0)
                }
                b'}' => {
                    let Some(open) = self.open.pop() else {
                        return Err(Gnu(at, "unexpected `}'".into()));
                    };
                    if a1.is_some() {
                        return Err(Gnu(at, "`}' doesn't want any addresses".into()));
                    }
                    self.cmds[open].kind = Kind::Open(self.cmds.len() + 1);
                    self.end()?;
                    Kind::Close
                }
                b'#' => return Err(Gnu(at, "comments don't accept any addresses".into())),
                b'p' | b'd' | b'=' => {
                    self.end()?;
                    match c {
                        b'p' => Kind::Print,
                        b'd' => Kind::Delete,
                        _ => Kind::LineNo,
                    }
                }
                b'q' | b'Q' => {
                    if a2.is_some() {
                        return Err(Gnu(at, "command only uses one address".into()));
                    }
                    self.spaces();
                    let code = self.number();
                    self.end()?;
                    Kind::Quit { code: (code & 0xff) as i32, print: c == b'q' }
                }
                b's' => Kind::Subst(self.subst()?),
                b'y' => {
                    let map = self.trans()?;
                    self.end()?;
                    Kind::Trans(map)
                }
                b'a' => Kind::Append(self.text()?),
                b'i' => Kind::Insert(self.text()?),
                b'c' => Kind::Change(self.text()?),
                b'h' | b'H' | b'g' | b'G' | b'x' => return refuse(format!("the `{}` command (the hold space)", char::from(c))),
                b'N' | b'D' | b'P' | b'n' => return refuse(format!("the `{}` command (lines are edited one at a time)", char::from(c))),
                b':' | b'b' | b't' | b'T' => return refuse(format!("the `{}` command (labels and branches)", char::from(c))),
                b'r' | b'R' | b'w' | b'W' | b'e' => return refuse(format!("the `{}` command (reading or writing other files, running commands)", char::from(c))),
                b'l' | b'z' | b'F' | b'v' => return refuse(format!("the `{}` command", char::from(c))),
                _ => return Err(Gnu(at, format!("unknown command: `{}'", char::from(c)))),
            };
            self.cmds.push(Cmd { a1, a2, not, kind, expr });
        }
        if let Some(open) = self.open.last() {
            return Err(GnuAt(self.cmds[*open].expr, "unmatched `{'".into()));
        }
        Ok(())
    }
}

fn parse_script(pieces: &[Vec<u8>], extended: bool) -> Result<Program, (usize, usize, String)> {
    let mut starts = Vec::new();
    let mut s = Vec::new();
    for (k, p) in pieces.iter().enumerate() {
        if k > 0 {
            s.push(b'\n');
        }
        starts.push(s.len());
        s.extend_from_slice(p);
    }
    // A script that starts with `#n` is run as with -n.
    let quiet = s.starts_with(b"#n");
    let mut p = Parser { s: &s, i: 0, extended, starts: &starts, res: Vec::new(), cmds: Vec::new(), open: Vec::new(), uses_last: false };
    match p.parse() {
        Ok(()) => Ok(Program { cmds: p.cmds, res: p.res, quiet, uses_last: p.uses_last }),
        Err(Unsupported(what)) => Err((0, 0, what)),
        Err(Gnu(at, m)) => {
            let at = at.min(s.len());
            // A position on the newline that joins two -e belongs to the first.
            let expr = starts.iter().rposition(|st| *st <= at && (at == 0 || *st < at || *st == 0)).unwrap_or(0);
            Err((expr + 1, at - starts[expr], m))
        }
        Err(GnuAt(expr, m)) => Err((expr, 0, m)),
    }
}

// ------------------------------------------------------------ running it

enum Fail {
    /// The empty pattern with no pattern used before it; the -e it is in.
    NoPrevious(usize),
    TooComplex,
}

impl From<MatchError> for Fail {
    fn from(_: MatchError) -> Fail {
        Fail::TooComplex
    }
}

const TOO_COMPLEX: &str = "pattern too complex for the backtracking matcher";

impl Subst {
    /// Apply to one line (without its newline). None if nothing matched.
    fn apply(&self, re: &Regex, line: &[u8]) -> Result<Option<Vec<u8>>, MatchError> {
        let mut out = Vec::new();
        let mut start = 0;
        let mut prev_end: Option<usize> = None;
        let mut count = 0;
        let mut replaced = false;
        while start <= line.len() {
            let caps = match re.find_at(line, start, self.want_groups)? {
                None => break,
                Some(c) => c,
            };
            let (s, e) = caps[0].unwrap();
            if out.capacity() == 0 {
                out.reserve(line.len() + 16);
            }
            out.extend_from_slice(&line[start..s]);
            // An empty match right after the previous match is not a match.
            let skip = s == e && prev_end == Some(s);
            let mut here = false;
            if !skip {
                count += 1;
                if count >= self.nth {
                    for r in &self.rep {
                        match r {
                            Rep::Lit(l) => out.extend_from_slice(l),
                            Rep::Group(g) => {
                                if let Some((a, b)) = caps.get(*g).copied().flatten() {
                                    out.extend_from_slice(&line[a..b]);
                                }
                            }
                        }
                    }
                    replaced = true;
                    here = true;
                } else {
                    out.extend_from_slice(&line[s..e]);
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
            if here && !self.global {
                break;
            }
        }
        if !replaced {
            return Ok(None);
        }
        if start < line.len() {
            out.extend_from_slice(&line[start..]);
        }
        Ok(Some(out))
    }
}

type Out<'o> = &'o mut dyn FnMut(&[u8]);

/// A program being run over lines.
struct Exec {
    prog: Rc<Program>,
    line_no: u64,
    /// For each command with two addresses: None outside its range,
    /// Some(end) inside, where end is the line number that ends it if a
    /// number does.
    ranges: Vec<Option<u64>>,
    /// Ranges that start at a line number and have ended in this file.
    closed: Vec<bool>,
    last_re: Option<usize>,
    /// Texts of `a` waiting for the end of the cycle, by command.
    appends: Vec<usize>,
    /// Something was written that did not end with a newline; the next
    /// thing written is put on a line of its own.
    open: bool,
}

impl Exec {
    fn new(prog: Rc<Program>) -> Exec {
        let mut e = Exec { prog, line_no: 0, ranges: Vec::new(), closed: Vec::new(), last_re: None, appends: Vec::new(), open: false };
        e.restart();
        e
    }

    /// Start counting lines again, for the next file of -s or -i.
    fn restart(&mut self) {
        self.line_no = 0;
        self.ranges = self.prog.cmds.iter().map(|c| (c.a1 == Some(Addr::Zero)).then_some(u64::MAX)).collect();
        self.closed = vec![false; self.prog.cmds.len()];
    }

    fn emit(&mut self, text: &[u8], nl: bool, out: Out) {
        if self.open {
            out(b"\n");
            self.open = false;
        }
        out(text);
        if nl {
            out(b"\n");
        } else if !text.is_empty() {
            self.open = true;
        }
    }

    /// Text that ends with a newline of its own, or is empty.
    fn text(&mut self, text: &[u8], out: Out) {
        if self.open {
            out(b"\n");
            self.open = false;
        }
        out(text);
    }

    fn regex(&mut self, re: Option<usize>, expr: usize) -> Result<usize, Fail> {
        let r = re.or(self.last_re).ok_or(Fail::NoPrevious(expr))?;
        self.last_re = Some(r);
        Ok(r)
    }

    fn one(&mut self, a: Addr, k: usize, line: &[u8], last: bool) -> Result<bool, Fail> {
        Ok(match a {
            Addr::Line(n) => self.line_no == n,
            Addr::Last => last,
            Addr::Step(first, step) => {
                if step == 0 {
                    self.line_no == first
                } else {
                    self.line_no >= first && (self.line_no - first) % step == 0
                }
            }
            Addr::Re(re) => {
                let r = self.regex(re, self.prog.cmds[k].expr)?;
                self.prog.res[r].is_match(line)?
            }
            Addr::Plus(_) | Addr::Zero => false,
        })
    }

    fn selects(&mut self, k: usize, line: &[u8], last: bool) -> Result<bool, Fail> {
        let prog = self.prog.clone();
        let c = &prog.cmds[k];
        let hit = match (c.a1, c.a2) {
            (None, _) => true,
            (Some(a), None) => self.one(a, k, line, last)?,
            (Some(a1), Some(a2)) => match self.ranges[k] {
                None => {
                    // A range that starts at a line number starts at the
                    // first line from there on that reaches the command,
                    // and only once in a file.
                    let starts = match a1 {
                        Addr::Line(n) => !self.closed[k] && self.line_no >= n,
                        _ => self.one(a1, k, line, last)?,
                    };
                    if !starts {
                        false
                    } else if let (Addr::Line(n), Addr::Line(m)) = (a1, a2) {
                        if m <= self.line_no {
                            // The end is already behind: one line, and
                            // none when the start was missed as well.
                            self.closed[k] = true;
                            self.line_no == n || self.line_no == m
                        } else {
                            self.ranges[k] = Some(m);
                            true
                        }
                    } else {
                        // A range whose end is already behind is one line.
                        self.ranges[k] = match a2 {
                            Addr::Line(n) => (self.line_no < n).then_some(n),
                            Addr::Plus(n) => (n > 0).then_some(self.line_no.saturating_add(n)),
                            Addr::Last => (!last).then_some(u64::MAX),
                            _ => Some(u64::MAX),
                        };
                        if self.ranges[k].is_none() && matches!(a1, Addr::Line(_)) {
                            self.closed[k] = true;
                        }
                        true
                    }
                }
                Some(end) => {
                    let ends = match a2 {
                        Addr::Line(_) | Addr::Plus(_) => self.line_no >= end,
                        Addr::Last => last,
                        other => self.one(other, k, line, last)?,
                    };
                    if ends {
                        self.ranges[k] = None;
                        if matches!(a1, Addr::Line(_)) {
                            self.closed[k] = true;
                        }
                    }
                    // A line past a numbered end, which never reached
                    // the command itself, is not part of the range.
                    !(matches!(a2, Addr::Line(_)) && self.line_no > end)
                }
            },
        };
        Ok(hit != c.not)
    }

    /// One line through the program. Some(status) when it says to quit.
    fn cycle(&mut self, line: &[u8], nl: bool, last: bool, out: Out) -> Result<Option<i32>, Fail> {
        let prog = self.prog.clone();
        self.line_no += 1;
        let mut space: Cow<[u8]> = Cow::Borrowed(line);
        let mut print = !prog.quiet;
        let mut quit = None;
        let mut pc = 0;
        let mut steps = 0u32;
        while pc < prog.cmds.len() {
            steps = steps.wrapping_add(1);
            if steps & 4095 == 0 {
                crate::unit::poll();
            }
            let c = &prog.cmds[pc];
            if matches!(c.kind, Kind::Close) {
                pc += 1;
                continue;
            }
            if !self.selects(pc, &space, last)? {
                pc = if let Kind::Open(end) = c.kind { end } else { pc + 1 };
                continue;
            }
            match &c.kind {
                Kind::Open(_) | Kind::Close => {}
                Kind::Print => self.emit(&space, nl, out),
                Kind::Delete => {
                    print = false;
                    break;
                }
                Kind::LineNo => {
                    let n = self.line_no.to_string();
                    self.emit(n.as_bytes(), true, out);
                }
                Kind::Insert(text) => self.text(text, out),
                Kind::Append(_) => self.appends.push(pc),
                Kind::Change(text) => {
                    // For a range the text is written once, at its end.
                    if c.a2.is_none() || self.ranges[pc].is_none() {
                        self.text(text, out);
                    }
                    print = false;
                    break;
                }
                Kind::Subst(s) => {
                    let r = self.regex(s.re, c.expr)?;
                    if let Some(new) = s.apply(&prog.res[r], &space)? {
                        space = Cow::Owned(new);
                        if s.print {
                            self.emit(&space, nl, out);
                        }
                    }
                }
                Kind::Trans(map) => {
                    if space.iter().any(|b| map[*b as usize] != *b) {
                        space = Cow::Owned(space.iter().map(|b| map[*b as usize]).collect());
                    }
                }
                Kind::Quit { code, print: shown } => {
                    quit = Some(*code);
                    if !*shown {
                        print = false;
                        self.appends.clear();
                    }
                    break;
                }
            }
            pc += 1;
        }
        if print {
            self.emit(&space, nl, out);
        }
        if quit.is_some() && self.open {
            out(b"\n");
            self.open = false;
        }
        for k in std::mem::take(&mut self.appends) {
            if let Kind::Append(text) = &prog.cmds[k].kind {
                self.text(text, out);
            }
        }
        Ok(quit)
    }

    /// Whole lines through the program; only the last line of `data` may
    /// lack its newline. `ends` says that the last line here is the last
    /// one there is. With `finder`, a program that leaves alone every line
    /// its pattern does not match: only the lines that do are looked at.
    fn chunk(&mut self, data: &[u8], ends: bool, finder: Option<&mut Searcher>, out: Out) -> Result<Option<i32>, Fail> {
        let mut at = 0;
        if let Some(finder) = finder {
            let quiet = self.prog.quiet;
            while at < data.len() {
                let (s, e) = match finder.next_line(data, at)? {
                    Some(found) => found,
                    None => (data.len(), data.len()),
                };
                if s > at && !quiet {
                    let skipped = &data[at..s];
                    let nl = skipped.ends_with(b"\n");
                    self.emit(&skipped[..skipped.len() - usize::from(nl)], nl, out);
                }
                if s >= data.len() {
                    break;
                }
                let nl = e < data.len();
                at = e + usize::from(nl);
                if let Some(code) = self.cycle(&data[s..e], nl, ends && at >= data.len(), out)? {
                    return Ok(Some(code));
                }
            }
            return Ok(None);
        }
        while at < data.len() {
            let e = memchr::memchr(b'\n', &data[at..]).map_or(data.len(), |i| at + i);
            let nl = e < data.len();
            let next = e + usize::from(nl);
            if let Some(code) = self.cycle(&data[at..e], nl, ends && next >= data.len(), out)? {
                return Ok(Some(code));
            }
            at = next;
        }
        Ok(None)
    }
}

impl Program {
    /// If every line this pattern does not match goes through the program
    /// untouched and unnoticed (printed as it is, or not at all with -n),
    /// the pattern.
    fn selector(&self) -> Option<&Regex> {
        let [c] = self.cmds.as_slice() else { return None };
        if c.not || c.a2.is_some() {
            return None;
        }
        let re = match (&c.a1, &c.kind) {
            (Some(Addr::Re(Some(r))), Kind::Print | Kind::Delete | Kind::Trans(_) | Kind::Append(_) | Kind::Insert(_) | Kind::Change(_)) => *r,
            (Some(Addr::Re(Some(r))), Kind::Subst(s)) if s.re.is_some() => *r,
            (None, Kind::Subst(Subst { re: Some(r), .. })) => *r,
            _ => return None,
        };
        Some(&self.res[re])
    }
}

fn fail_message(fail: &Fail, name: &[u8]) -> String {
    match fail {
        Fail::NoPrevious(expr) => format!("sed: -e expression #{}, char 0: no previous regular expression", expr),
        Fail::TooComplex => format!("qdb: sed: {}: {}", lossy(name), TOO_COMPLEX),
    }
}

/// sed reading its standard input in pieces.
struct SedFilter {
    exec: Exec,
    lines: LineBuf,
    /// The line before the one being read, kept until it is known
    /// whether it is the last, when the program asks.
    held: Option<Vec<u8>>,
    done: Option<i32>,
}

impl SedFilter {
    fn line(&mut self, line: &[u8], last: bool, io: &mut Io) -> bool {
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        match self.exec.cycle(text, text.len() < line.len(), last, &mut |b| io.put(b)) {
            Ok(None) => true,
            Ok(Some(code)) => {
                self.done = Some(code);
                false
            }
            Err(fail) => {
                io.err_line(fail_message(&fail, b"-"));
                self.done = Some(1);
                false
            }
        }
    }
}

impl Filter for SedFilter {
    fn push(&mut self, data: &[u8], io: &mut Io) -> bool {
        if self.done.is_some() {
            return false;
        }
        let hold = self.exec.prog.uses_last;
        let mut lines = std::mem::take(&mut self.lines);
        let more = lines.feed(data, |line| {
            if !hold {
                return self.line(line, false, io);
            }
            match self.held.replace(line.to_vec()) {
                Some(before) => self.line(&before, false, io),
                None => true,
            }
        });
        self.lines = lines;
        more && !io.stopped()
    }

    fn finish(mut self: Box<Self>, io: &mut Io) -> i32 {
        io.errors_first();
        if self.done.is_none() {
            let rest = self.lines.rest();
            if let Some(held) = self.held.take() {
                if self.line(&held, rest.is_none(), io) {
                    if let Some(rest) = rest {
                        self.line(&rest, true, io);
                    }
                }
            } else if let Some(rest) = rest {
                self.line(&rest, true, io);
            }
        }
        self.done.unwrap_or(0)
    }
}

const USAGE: &str = "Usage: sed [OPTION]... {script-only-if-no-other-script} [input-file]...

  -n, --quiet, --silent
                 suppress automatic printing of pattern space
      --debug
                 annotate program execution
  -e script, --expression=script
                 add the script to the commands to be executed
  -f script-file, --file=script-file
                 add the contents of script-file to the commands to be executed
  --follow-symlinks
                 follow symlinks when processing in place
  -i[SUFFIX], --in-place[=SUFFIX]
                 edit files in place (makes backup if SUFFIX supplied)
  -l N, --line-length=N
                 specify the desired line-wrap length for the `l' command
  --posix
                 disable all GNU extensions.
  -E, -r, --regexp-extended
                 use extended regular expressions in the script
                 (for portability use POSIX -E).
  -s, --separate
                 consider files as separate rather than as a single,
                 continuous long stream.
      --sandbox
                 operate in sandbox mode (disable e/r/w commands).
  -u, --unbuffered
                 load minimal amounts of data from the input files and flush
                 the output buffers more often
  -z, --null-data
                 separate lines by NUL characters
      --help     display this help and exit
      --version  output version information and exit

If no -e, --expression, -f, or --file option is given, then the first
non-option argument is taken as the sed script to interpret.  All
remaining arguments are names of input files; if no input files are
specified, then the standard input is read.

GNU sed home page: <https://www.gnu.org/software/sed/>.
General help using GNU software: <https://www.gnu.org/gethelp/>.";

struct Args {
    quiet: bool,
    extended: bool,
    separate: bool,
    /// -i, with its backup suffix.
    in_place: Option<Vec<u8>>,
    scripts: Vec<Vec<u8>>,
    files: Vec<Vec<u8>>,
    /// -e was last on the line.
    missing: bool,
}

fn arguments(args: &[Vec<u8>]) -> Result<Args, String> {
    let mut a = Args { quiet: false, extended: false, separate: false, in_place: None, scripts: Vec::new(), files: Vec::new(), missing: false };
    let mut explicit = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if arg == b"--" {
            a.files.extend(args[i..].iter().cloned());
            break;
        }
        if arg.starts_with(b"--") {
            return Err(format!("flag: sed {} (long options are not supported)", lossy(arg)));
        }
        if arg.len() < 2 || arg[0] != b'-' {
            a.files.push(arg.clone());
            continue;
        }
        let mut j = 1;
        while j < arg.len() {
            let c = arg[j];
            j += 1;
            match c {
                b'n' => a.quiet = true,
                b'r' | b'E' => a.extended = true,
                b's' => a.separate = true,
                b'i' => {
                    // What is attached to -i is the backup suffix.
                    let suffix = arg[j..].to_vec();
                    if suffix.contains(&b'*') || suffix.contains(&b'/') {
                        return Err("flag: sed -i with * or / in the backup suffix".into());
                    }
                    a.in_place = Some(suffix);
                    break;
                }
                b'e' => {
                    explicit = true;
                    if j < arg.len() {
                        a.scripts.push(arg[j..].to_vec());
                    } else if i < args.len() {
                        a.scripts.push(args[i].clone());
                        i += 1;
                    } else {
                        a.missing = true;
                    }
                    break;
                }
                _ => return Err(format!("flag: sed -{}", char::from(c))),
            }
        }
    }
    if !explicit && !a.files.is_empty() {
        a.scripts.push(a.files.remove(0));
    }
    Ok(a)
}

/// Edit one file where it is. Err is a status that ends the whole command.
fn edit(db: &mut Db, io: &mut Io, exec: &mut Exec, name: &[u8], id: crate::tree::NodeId, finder: &mut Option<(Searcher, Option<usize>, Option<crate::tree::Shortlist>)>) -> Result<i32, i32> {
    // A piece at a time: pieces hold whole lines, so each is replaced by
    // what the program makes of it, and one that comes out as it went in
    // stays where it is.
    let (mut searcher, only) = match finder {
        Some((searcher, probe, list)) => {
            // A file with no line the pattern matches comes out as it went
            // in, so it is neither rewritten nor, where storage can tell
            // without reading it, read. The first files are simply read;
            // storage is asked once that has cost what its answer costs.
            if let Some(budget) = *probe {
                let cost = db.size(id) as usize + 256;
                if budget >= cost {
                    *probe = Some(budget - cost);
                } else {
                    *probe = None;
                    *list = db.shortlist(searcher.query());
                }
            }
            if list.as_ref().is_some_and(|set| !db.passes(set, id)) {
                return Ok(0);
            }
            (Some(searcher), list.as_ref())
        }
        None => (None, None),
    };
    let saved = db.save_pieces(id);
    let mut at = 0;
    let mut changed = false;
    let mut failed = None;
    let mut quit = None;
    let mut out = Vec::new();
    for (n, p) in saved.iter().enumerate() {
        if only.is_some_and(|set| !db.piece_passes(set, p)) {
            at += 1;
            continue;
        }
        out.clear();
        let rewrite = {
            let text = db.text_of(p);
            let ends = n + 1 == saved.len();
            let skip = match &mut searcher {
                Some(searcher) => matches!(searcher.next_line(&text, 0), Ok(None)),
                None => false,
            };
            if skip {
                false
            } else {
                match exec.chunk(&text, ends, None, &mut |b| out.extend_from_slice(b)) {
                    Err(fail) => {
                        failed = Some(fail_message(&fail, name));
                        break;
                    }
                    Ok(q) => quit = q,
                }
                if !is_valid_text(&out) {
                    failed = Some(format!("qdb: sed: {}: result is not valid UTF-8 text, file left unchanged", lossy(name)));
                    break;
                }
                quit.is_some() || out != *text
            }
        };
        if rewrite {
            if quit.is_some() {
                // What was not read is not written.
                db.replace_pieces(id, at, saved.len() - n, &out);
            } else {
                at += db.replace_pieces(id, at, 1, &out);
            }
            changed = true;
        } else {
            at += 1;
        }
        if quit.is_some() {
            break;
        }
    }
    if let Some(message) = failed {
        if changed {
            db.restore_pieces(id, saved);
        }
        io.err_line(message);
        return Ok(1);
    }
    match quit {
        Some(code) => Err(code),
        None => Ok(0),
    }
}

pub fn sed(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let a = arguments(&args)?;
    if a.missing {
        let bad: Runner = Box::new(|_, io| {
            io.err_line("sed: option requires an argument -- 'e'");
            io.err_line(USAGE);
            1
        });
        return Ok(bad.into());
    }
    if a.scripts.is_empty() {
        let usage: Runner = Box::new(|_, io| {
            io.err_line(USAGE);
            1
        });
        return Ok(usage.into());
    }
    let mut prog = match parse_script(&a.scripts, a.extended) {
        Ok(p) => p,
        Err((0, _, what)) => return Err(what),
        Err((expr, pos, m)) => {
            let bad: Runner = Box::new(move |_, io| {
                io.err_line(format!("sed: -e expression #{}, char {}: {}", expr, pos, m));
                1
            });
            return Ok(bad.into());
        }
    };
    prog.quiet |= a.quiet;
    let prog = Rc::new(prog);
    let files = a.files;
    let in_place = a.in_place;
    let writes = in_place.is_some();
    let separate = a.separate || in_place.is_some();
    let input = if in_place.is_some() {
        Input::Unused
    } else if files.is_empty() {
        Input::Stream(Box::new(SedFilter { exec: Exec::new(prog.clone()), lines: LineBuf::default(), held: None, done: None }))
    } else if files.iter().any(|f| f == b"-") {
        Input::Whole
    } else {
        Input::Unused
    };
    let run: Runner = Box::new(move |db, io| {
        io.errors_first();
        let mut status = 0;
        let mut exec = Exec::new(prog.clone());
        let selector = prog.selector().cloned();
        if let Some(suffix) = in_place {
            if files.is_empty() {
                io.err_line("sed: no input files");
                return 4;
            }
            // With -n a line the pattern does not match is dropped, which
            // changes the file, so every piece is read.
            let mut finder = selector.filter(|_| !prog.quiet).map(|re| {
                let s = Searcher::regex(vec![re]);
                let probe = db.candidates_cost(s.query());
                (s, probe, None)
            });
            for f in &files {
                let id = match db.resolve(f) {
                    Err(e) => {
                        io.err_line(format!("sed: can't read {}: {}", lossy(f), e.msg()));
                        status = status.max(2);
                        continue;
                    }
                    Ok(id) if db.is_dir(id) => {
                        // Fatal in GNU sed: later files are not touched.
                        io.err_line(format!("sed: couldn't edit {}: not a regular file", lossy(f)));
                        return 4;
                    }
                    Ok(id) => id,
                };
                let parent = db.parent(id);
                if !suffix.is_empty() {
                    if let Some(parent) = parent {
                        let name = [basename(f), &suffix[..]].concat();
                        let copy = match db.child(parent, &name) {
                            Some(there) if db.is_dir(there) => {
                                io.err_line(format!("sed: cannot rename {}: Is a directory", lossy(f)));
                                return 4;
                            }
                            Some(there) => there,
                            None => db.create_file(parent, &name, &[]),
                        };
                        db.copy_content(id, copy);
                    }
                }
                // The real command writes a new file in the folder and
                // gives it the old name, whether or not a line changed.
                db.touch(id);
                if let Some(parent) = parent {
                    db.touch(parent);
                }
                exec.restart();
                exec.open = false;
                match edit(db, io, &mut exec, f, id, &mut finder) {
                    Ok(s) => status = status.max(s),
                    Err(code) => return code,
                }
            }
            return status;
        }
        let inputs = if files.is_empty() { vec![b"-".to_vec()] } else { files };
        let stdin = io.take_stdin();
        // The last line there is, for `$`, is the last line of the last
        // input that has one.
        let sized = |f: &Vec<u8>| -> bool {
            if f == b"-" {
                !stdin.is_empty()
            } else {
                db.resolve(f).is_ok_and(|id| db.is_dir(id) || db.size(id) > 0)
            }
        };
        let last_input = inputs.iter().rposition(sized);
        let mut stdin = Some(stdin);
        // In a pipeline with -n only matching lines matter: storage is
        // asked which files and pieces can hold one.
        let mut finder = selector.map(|re| Searcher::regex(vec![re]));
        let mut probe = match &finder {
            Some(f) if prog.quiet => db.candidates_cost(f.query()),
            _ => None,
        };
        let mut only = None;
        for (n, f) in inputs.iter().enumerate() {
            if io.stopped() {
                break;
            }
            let ends = separate || Some(n) == last_input;
            if separate && n > 0 {
                exec.restart();
            }
            let result = if f == b"-" {
                let data = stdin.take().unwrap_or(b"");
                exec.chunk(data, ends, finder.as_mut(), &mut |b| io.put(b))
            } else {
                match db.resolve(f) {
                    Err(e) => {
                        io.err_line(format!("sed: can't read {}: {}", lossy(f), e.msg()));
                        status = status.max(2);
                        continue;
                    }
                    Ok(id) if db.is_dir(id) => {
                        // Fatal in GNU sed: later files are not read.
                        io.err_line(format!("sed: read error on {}: Is a directory", lossy(f)));
                        return 4;
                    }
                    Ok(id) => {
                        if let (Some(budget), Some(finder)) = (probe, &finder) {
                            let cost = db.size(id) as usize + 256;
                            if budget >= cost {
                                probe = Some(budget - cost);
                            } else {
                                probe = None;
                                only = db.shortlist(finder.query());
                            }
                        }
                        if only.as_ref().is_some_and(|set| !db.passes(set, id)) {
                            continue;
                        }
                        let pieces = db.pieces(id);
                        let mut result = Ok(None);
                        for (k, p) in pieces.iter().enumerate() {
                            if only.as_ref().is_some_and(|set| !db.piece_passes(set, p)) {
                                continue;
                            }
                            result = exec.chunk(&db.text_of(p), ends && k + 1 == pieces.len(), finder.as_mut(), &mut |b| io.put(b));
                            if !matches!(result, Ok(None)) || io.stopped() {
                                break;
                            }
                        }
                        result
                    }
                }
            };
            match result {
                Ok(None) => {}
                Ok(Some(code)) => return code,
                Err(fail) => {
                    io.err_line(fail_message(&fail, f));
                    return 1;
                }
            }
        }
        status
    });
    Ok(Prepared { run, input, writes })
}
