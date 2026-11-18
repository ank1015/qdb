//! POSIX regular expressions (basic and extended, with the GNU extensions),
//! matched on bytes as in the C locale.
//!
//! Matching is leftmost-longest, as POSIX requires: among the matches that
//! start earliest, the longest one wins, whatever order the alternatives were
//! written in. `echo ab | sed -E 's/a|ab/X/'` prints `X`, not `Xb`.
//!
//! Two matchers share one syntax tree:
//!
//! * a Thompson NFA simulation that finds the leftmost-longest span in time
//!   proportional to pattern size times input size, used whenever the
//!   pattern has no back-references;
//! * a backtracking matcher that explores every path, used for
//!   back-references and to recover the text of `\(...\)` groups.

use crate::glob::class_by_name;
use std::cell::RefCell;

mod dfa;
pub use dfa::Dfa;

pub const DUP_MAX: u32 = 32767;
const MAX_PROGRAM: usize = 2_000_000;
const STEP_LIMIT: usize = 20_000_000;

/// GNU grep and GNU sed read a few corner cases differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Grep,
    Sed,
}

#[derive(Debug, Clone, Copy)]
pub struct Syntax {
    pub extended: bool,
    pub icase: bool,
    pub flavor: Flavor,
}

type Set = Box<[bool; 256]>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assert {
    LineStart,
    LineEnd,
    WordBoundary,
    NotWordBoundary,
    WordStart,
    WordEnd,
}

#[derive(Debug, Clone)]
enum Node {
    Empty,
    Set(Set),
    Cat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat { sub: Box<Node>, min: u32, max: Option<u32> },
    Group(Box<Node>, usize),
    Backref(usize),
    Assert(Assert),
}

#[derive(Debug, Clone)]
enum Inst {
    Set(usize),
    Split(usize, usize),
    Jmp(usize),
    Assert(Assert),
    Match,
}

#[derive(Debug, Clone)]
pub struct Regex {
    ast: Node,
    ngroups: usize,
    has_backref: bool,
    icase: bool,
    prog: Vec<Inst>,
    sets: Vec<Set>,
    /// Work lists of the NFA simulation, kept between calls.
    scratch: RefCell<Scratch>,
    /// Warnings GNU grep prints for this pattern, without the program name.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct Scratch {
    clist: Vec<(usize, usize)>,
    nlist: Vec<(usize, usize)>,
    /// `mark[pc]` is `epoch + pos` for the last position pc was added at.
    mark: Vec<u64>,
    stack: Vec<usize>,
    /// Raised past every mark of a search when it ends, so the marks never
    /// need clearing.
    epoch: u64,
}

/// Fixed strings found in a pattern. With `fold`, the strings are stored
/// in lower case and stand for either case of each ASCII letter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literals {
    pub strings: Vec<Vec<u8>>,
    pub fold: bool,
}

/// More alternatives or longer strings than this and a pattern is not
/// treated as a set of fixed strings.
const MAX_LITERALS: usize = 16;
const MAX_LITERAL_LEN: usize = 256;

/// Group spans: index 0 is the whole match.
pub type Caps = Vec<Option<(usize, usize)>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchError {
    /// The backtracking matcher gave up. Only patterns with back-references
    /// or groups used in a replacement can hit this.
    TooComplex,
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn check_assert(a: Assert, hay: &[u8], pos: usize) -> bool {
    let prev = pos > 0 && is_word(hay[pos - 1]);
    let next = pos < hay.len() && is_word(hay[pos]);
    match a {
        Assert::LineStart => pos == 0,
        Assert::LineEnd => pos == hay.len(),
        Assert::WordBoundary => prev != next,
        Assert::NotWordBoundary => prev == next,
        Assert::WordStart => !prev && next,
        Assert::WordEnd => prev && !next,
    }
}

fn has_assert(n: &Node) -> bool {
    match n {
        Node::Assert(_) => true,
        Node::Cat(v) | Node::Alt(v) => v.iter().any(has_assert),
        Node::Repeat { sub, .. } | Node::Group(sub, _) => has_assert(sub),
        Node::Empty | Node::Set(_) | Node::Backref(_) => false,
    }
}

/// Every string of `a` followed by every string of `b`, unless that is
/// too many or too long.
fn cross(a: &[Vec<u8>], b: &[Vec<u8>]) -> Option<Vec<Vec<u8>>> {
    if a.len() * b.len() > MAX_LITERALS {
        return None;
    }
    let mut out = Vec::with_capacity(a.len() * b.len());
    for x in a {
        for y in b {
            if x.len() + y.len() > MAX_LITERAL_LEN {
                return None;
            }
            let joined = [x.as_slice(), y.as_slice()].concat();
            if !out.contains(&joined) {
                out.push(joined);
            }
        }
    }
    Some(out)
}

// ---------------------------------------------------------------- parsing

struct Parser<'a> {
    p: &'a [u8],
    i: usize,
    syn: Syntax,
    ngroups: usize,
    closed: Vec<bool>,
    has_backref: bool,
    warnings: Vec<String>,
}

type PResult<T> = Result<T, String>;

const BAD_REPEAT: &str = "Invalid preceding regular expression";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    Close,
    Comma,
    End,
}

fn set_of(f: impl Fn(u8) -> bool) -> Set {
    let mut s = Box::new([false; 256]);
    for b in 0..=255u8 {
        s[b as usize] = f(b);
    }
    s
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.p.get(self.i).copied()
    }

    fn looking_at(&self, t: &[u8]) -> bool {
        self.p[self.i..].starts_with(t)
    }

    fn ere(&self) -> bool {
        self.syn.extended
    }

    fn at_alt(&self) -> bool {
        if self.ere() {
            self.looking_at(b"|")
        } else {
            self.looking_at(b"\\|")
        }
    }

    fn at_close(&self) -> bool {
        if self.ere() {
            self.looking_at(b")")
        } else {
            self.looking_at(b"\\)")
        }
    }

    fn lit(&self, c: u8) -> Node {
        if self.syn.icase && c.is_ascii_alphabetic() {
            Node::Set(set_of(|b| b.eq_ignore_ascii_case(&c)))
        } else {
            Node::Set(set_of(|b| b == c))
        }
    }

    fn parse_alt(&mut self, depth: usize) -> PResult<Node> {
        let mut branches = vec![self.parse_branch(depth)?];
        while self.at_alt() {
            self.i += if self.ere() { 1 } else { 2 };
            branches.push(self.parse_branch(depth)?);
        }
        Ok(if branches.len() == 1 { branches.pop().unwrap() } else { Node::Alt(branches) })
    }

    fn parse_branch(&mut self, depth: usize) -> PResult<Node> {
        let ere = self.ere();
        let grep = self.syn.flavor == Flavor::Grep;
        let mut items: Vec<Node> = Vec::new();
        // `fresh`: nothing but anchors so far in this branch.
        // `after_anchor`: the previous item was an anchor.
        let mut fresh = true;
        let mut after_anchor = false;
        loop {
            if self.peek().is_none() || self.at_alt() {
                break;
            }
            if self.at_close() {
                if depth > 0 {
                    break;
                }
                if ere && grep {
                    // GNU grep -E treats an unmatched ) as a literal.
                    self.i += 1;
                    let l = self.lit(b')');
                    items.push(self.more_dups(l)?);
                    fresh = false;
                    after_anchor = false;
                    continue;
                }
                return Err("Unmatched ) or \\)".into());
            }
            // A repetition operator with no ordinary expression before it.
            // POSIX leaves most of this undefined; GNU grep and GNU sed
            // each do something different.
            if items.is_empty() || after_anchor {
                if let Some(op) = self.dup_token() {
                    let literal = if ere {
                        if !grep {
                            return Err(BAD_REPEAT.into());
                        }
                        false
                    } else if grep {
                        fresh
                    } else if op == b'{' {
                        return Err(BAD_REPEAT.into());
                    } else {
                        true
                    };
                    if literal {
                        self.i += if ere || op == b'*' { 1 } else { 2 };
                        let l = self.lit(op);
                        items.push(self.more_dups(l)?);
                        fresh = false;
                        after_anchor = false;
                        continue;
                    }
                    // grep: the operator applies to the anchor before it,
                    // or to the empty expression. Here glibc has already
                    // accepted the pattern, so a bad interval is either a
                    // literal (-E) or an error in dfa.c's wording.
                    let save = self.i;
                    let parsed = match self.parse_dup() {
                        Ok(p) => p,
                        Err(_) if ere => None,
                        Err(_) => return Err("invalid content of \\{\\}".into()),
                    };
                    if let Some((min, max)) = parsed {
                        if fresh && ere {
                            let shown = if op == b'{' { "{...}".to_string() } else { char::from(op).to_string() };
                            self.warnings.push(format!("warning: {} at start of expression", shown));
                        }
                        let sub = items.pop().unwrap_or(Node::Empty);
                        items.push(Node::Repeat { sub: Box::new(sub), min, max });
                        if op == b'{' {
                            fresh = false;
                        }
                        // A further operator applies to this item too.
                        after_anchor = true;
                        continue;
                    }
                    self.i = save;
                }
            }
            let caret_anchor = ere || items.is_empty();
            let (atom, is_anchor) = self.parse_atom(caret_anchor, depth)?;
            if is_anchor {
                items.push(atom);
                after_anchor = true;
                continue;
            }
            items.push(self.more_dups(atom)?);
            fresh = false;
            after_anchor = false;
        }
        Ok(match items.len() {
            0 => Node::Empty,
            1 => items.pop().unwrap(),
            _ => Node::Cat(items),
        })
    }

    /// Wrap a node in every repetition operator that follows it.
    fn more_dups(&mut self, mut node: Node) -> PResult<Node> {
        let mut repeated = matches!(node, Node::Repeat { .. });
        loop {
            // glibc's basic syntax (sed) does not allow * or \{ straight
            // after another repetition.
            if repeated && !self.ere() && self.syn.flavor == Flavor::Sed && matches!(self.dup_token(), Some(b'*') | Some(b'{')) {
                return Err(BAD_REPEAT.into());
            }
            match self.parse_dup()? {
                Some((min, max)) => node = Node::Repeat { sub: Box::new(node), min, max },
                None => return Ok(node),
            }
            repeated = true;
        }
    }

    /// The repetition operator at the cursor, as its character.
    fn dup_token(&self) -> Option<u8> {
        let c = self.peek()?;
        if c == b'*' {
            return Some(b'*');
        }
        if self.ere() {
            return if matches!(c, b'+' | b'?' | b'{') { Some(c) } else { None };
        }
        if c == b'\\' {
            return match self.p.get(self.i + 1) {
                Some(n @ (b'+' | b'?' | b'{')) => Some(*n),
                _ => None,
            };
        }
        None
    }

    /// Parse a repetition operator at the cursor, if there is one.
    fn parse_dup(&mut self) -> PResult<Option<(u32, Option<u32>)>> {
        let width = if self.ere() { 1 } else { 2 };
        match self.dup_token() {
            None => Ok(None),
            Some(b'*') => {
                self.i += 1;
                Ok(Some((0, None)))
            }
            Some(b'+') => {
                self.i += width;
                Ok(Some((1, None)))
            }
            Some(b'?') => {
                self.i += width;
                Ok(Some((0, Some(1))))
            }
            Some(_) => {
                let save = self.i;
                self.i += width;
                match self.parse_interval()? {
                    Some(r) => Ok(Some(r)),
                    None => {
                        self.i = save;
                        Ok(None)
                    }
                }
            }
        }
    }

    /// Read a number the way glibc does inside an interval: scan to the
    /// closing brace, a comma or the end of the pattern. Returns -1 for no
    /// characters and -2 for anything that is not a number, and says what
    /// stopped the scan.
    fn interval_number(&mut self) -> (i64, Stop) {
        let close: &[u8] = if self.ere() { b"}" } else { b"\\}" };
        let mut num: i64 = -1;
        loop {
            if self.peek().is_none() {
                return (-2, Stop::End);
            }
            if self.looking_at(close) {
                self.i += close.len();
                return (num, Stop::Close);
            }
            let c = self.p[self.i];
            if c == b',' {
                self.i += 1;
                return (num, Stop::Comma);
            }
            if c == b'\\' && self.i + 1 < self.p.len() {
                self.i += 2;
                num = -2;
                continue;
            }
            self.i += 1;
            num = if !c.is_ascii_digit() || num == -2 {
                -2
            } else if num == -1 {
                (c - b'0') as i64
            } else {
                (num * 10 + (c - b'0') as i64).min(DUP_MAX as i64 + 1)
            };
        }
    }

    /// Parse the inside of an interval, after the opening brace. Ok(None)
    /// means the brace is not an interval and GNU grep -E takes it as a
    /// literal character.
    fn parse_interval(&mut self) -> PResult<Option<(u32, Option<u32>)>> {
        let literal_ok = self.ere() && self.syn.flavor == Flavor::Grep;
        let bad_content = || "Invalid content of \\{\\}".to_string();
        let (mut start, mut stop) = self.interval_number();
        if start == -1 {
            if stop != Stop::Comma {
                return Err(bad_content());
            }
            start = 0;
        }
        let mut end = -2;
        if start != -2 {
            end = match stop {
                Stop::Close => start,
                Stop::Comma => {
                    let (n, s) = self.interval_number();
                    stop = s;
                    n
                }
                Stop::End => -2,
            };
        }
        if start == -2 || end == -2 {
            if literal_ok {
                return Ok(None);
            }
            return Err(if stop == Stop::End { "Unmatched \\{".to_string() } else { bad_content() });
        }
        if (end != -1 && start > end) || stop != Stop::Close {
            return Err(bad_content());
        }
        if (if end == -1 { start } else { end }) > DUP_MAX as i64 {
            return Err("Regular expression too big".into());
        }
        Ok(Some((start as u32, if end == -1 { None } else { Some(end as u32) })))
    }

    /// Parse one atom. The flag in the result says the atom was an anchor.
    /// `caret_anchor` says whether a `^` here is an anchor.
    fn parse_atom(&mut self, caret_anchor: bool, depth: usize) -> PResult<(Node, bool)> {
        let ere = self.ere();
        let c = self.peek().unwrap();
        // Groups.
        if ere && c == b'(' || !ere && self.looking_at(b"\\(") {
            self.i += if ere { 1 } else { 2 };
            self.ngroups += 1;
            let idx = self.ngroups;
            self.closed.push(false);
            let inner = self.parse_alt(depth + 1)?;
            if !self.at_close() {
                return Err("Unmatched ( or \\(".into());
            }
            self.i += if ere { 1 } else { 2 };
            self.closed[idx - 1] = true;
            return Ok((Node::Group(Box::new(inner), idx), false));
        }
        match c {
            b'^' => {
                self.i += 1;
                if caret_anchor {
                    return Ok((Node::Assert(Assert::LineStart), true));
                }
                Ok((self.lit(b'^'), false))
            }
            b'$' => {
                self.i += 1;
                let anchor = ere || self.peek().is_none() || self.looking_at(b"\\)") || self.looking_at(b"\\|");
                if anchor {
                    return Ok((Node::Assert(Assert::LineEnd), true));
                }
                Ok((self.lit(b'$'), false))
            }
            b'.' => {
                self.i += 1;
                Ok((Node::Set(set_of(|b| b != b'\n')), false))
            }
            b'[' => {
                self.i += 1;
                Ok((Node::Set(self.parse_bracket()?), false))
            }
            b'\\' => {
                self.i += 1;
                let e = match self.peek() {
                    None => return Err("Trailing backslash".into()),
                    Some(e) => e,
                };
                self.i += 1;
                let assert = |a| Ok((Node::Assert(a), true));
                match e {
                    b'1'..=b'9' => {
                        let n = (e - b'0') as usize;
                        if n > self.ngroups || !self.closed[n - 1] {
                            return Err("Invalid back reference".into());
                        }
                        self.has_backref = true;
                        Ok((Node::Backref(n), false))
                    }
                    b'w' => Ok((Node::Set(set_of(is_word)), false)),
                    b'W' => Ok((Node::Set(set_of(|b| !is_word(b) && b != b'\n')), false)),
                    b's' => Ok((Node::Set(set_of(|b| class_by_name(b"space").unwrap()(b))), false)),
                    b'S' => Ok((Node::Set(set_of(|b| !class_by_name(b"space").unwrap()(b))), false)),
                    b'b' => assert(Assert::WordBoundary),
                    b'B' => assert(Assert::NotWordBoundary),
                    b'<' => assert(Assert::WordStart),
                    b'>' => assert(Assert::WordEnd),
                    b'`' => assert(Assert::LineStart),
                    b'\'' => assert(Assert::LineEnd),
                    _ => Ok((self.lit(e), false)),
                }
            }
            _ => {
                self.i += 1;
                Ok((self.lit(c), false))
            }
        }
    }

    /// Parse a bracket expression; the cursor is just past the `[`.
    fn parse_bracket(&mut self) -> PResult<Set> {
        let unmatched = || "Unmatched [, [^, [:, [., or [=".to_string();
        let mut set = Box::new([false; 256]);
        let mut negate = false;
        if self.peek() == Some(b'^') {
            negate = true;
            self.i += 1;
        }
        if self.peek().is_none() {
            return Err("Invalid regular expression".into());
        }
        let content_start = self.i;
        let mut first = true;
        loop {
            let c = self.peek().ok_or_else(unmatched)?;
            if c == b']' && !first {
                break;
            }
            first = false;
            // One element: a class, or a single character that may start a range.
            let lo: u8;
            if c == b'[' && matches!(self.p.get(self.i + 1), Some(b':') | Some(b'.') | Some(b'=')) {
                let kind = self.p[self.i + 1];
                let body_start = self.i + 2;
                let close = (body_start..self.p.len().saturating_sub(1))
                    .find(|&j| self.p[j] == kind && self.p[j + 1] == b']')
                    .ok_or_else(unmatched)?;
                let body = &self.p[body_start..close];
                self.i = close + 2;
                if kind == b':' {
                    let f = class_by_name(body).ok_or("Invalid character class name")?;
                    for b in 0..=255u8 {
                        if f(b) {
                            set[b as usize] = true;
                        }
                    }
                    continue;
                }
                if body.len() != 1 {
                    return Err("Invalid collation character".into());
                }
                lo = body[0];
            } else {
                lo = c;
                self.i += 1;
            }
            if self.peek() == Some(b'-') && self.p.get(self.i + 1).is_some_and(|n| *n != b']') {
                self.i += 1;
                let mut hi = self.peek().unwrap();
                if hi == b'[' && matches!(self.p.get(self.i + 1), Some(b'.') | Some(b'=')) {
                    let kind = self.p[self.i + 1];
                    let close = (self.i + 2..self.p.len().saturating_sub(1))
                        .find(|&j| self.p[j] == kind && self.p[j + 1] == b']')
                        .ok_or_else(unmatched)?;
                    if close != self.i + 3 {
                        return Err("Invalid collation character".into());
                    }
                    hi = self.p[self.i + 2];
                    self.i = close + 2;
                } else {
                    self.i += 1;
                }
                if hi < lo {
                    return Err("Invalid range end".into());
                }
                for b in lo..=hi {
                    set[b as usize] = true;
                }
            } else {
                set[lo as usize] = true;
            }
        }
        let content = &self.p[content_start..self.i];
        self.i += 1;
        if self.syn.flavor == Flavor::Grep && !negate && content.len() >= 3 && content[0] == b':' && content[content.len() - 1] == b':' {
            return Err("character class syntax is [[:space:]], not [:space:]".into());
        }
        if self.syn.icase {
            for b in b'a'..=b'z' {
                let u = b.to_ascii_uppercase();
                let either = set[b as usize] || set[u as usize];
                set[b as usize] = either;
                set[u as usize] = either;
            }
        }
        if negate {
            for b in set.iter_mut() {
                *b = !*b;
            }
            set[b'\n' as usize] = false;
        }
        Ok(set)
    }
}

// -------------------------------------------------------------- compiling

struct Compiler {
    prog: Vec<Inst>,
    sets: Vec<Set>,
}

impl Compiler {
    fn emit(&mut self, i: Inst) -> PResult<usize> {
        if self.prog.len() >= MAX_PROGRAM {
            return Err("Regular expression too big".into());
        }
        self.prog.push(i);
        Ok(self.prog.len() - 1)
    }

    fn compile(&mut self, n: &Node) -> PResult<()> {
        match n {
            Node::Empty => {}
            Node::Set(s) => {
                self.sets.push(s.clone());
                let idx = self.sets.len() - 1;
                self.emit(Inst::Set(idx))?;
            }
            Node::Cat(v) => {
                for x in v {
                    self.compile(x)?;
                }
            }
            Node::Alt(v) => {
                let mut jumps = Vec::new();
                for (k, x) in v.iter().enumerate() {
                    if k + 1 < v.len() {
                        let split = self.emit(Inst::Split(0, 0))?;
                        self.compile(x)?;
                        jumps.push(self.emit(Inst::Jmp(0))?);
                        let next = self.prog.len();
                        self.prog[split] = Inst::Split(split + 1, next);
                    } else {
                        self.compile(x)?;
                    }
                }
                let end = self.prog.len();
                for j in jumps {
                    self.prog[j] = Inst::Jmp(end);
                }
            }
            Node::Group(sub, _) => self.compile(sub)?,
            Node::Backref(_) => unreachable!("back-references use the backtracking matcher"),
            Node::Assert(a) => {
                self.emit(Inst::Assert(*a))?;
            }
            Node::Repeat { sub, min, max } => {
                for _ in 0..*min {
                    self.compile(sub)?;
                }
                match max {
                    None => {
                        // L: split(body, end); body; jmp L
                        let split = self.emit(Inst::Split(0, 0))?;
                        self.compile(sub)?;
                        self.emit(Inst::Jmp(split))?;
                        let end = self.prog.len();
                        self.prog[split] = Inst::Split(split + 1, end);
                    }
                    Some(mx) => {
                        let mut splits = Vec::new();
                        for _ in *min..*mx {
                            splits.push(self.emit(Inst::Split(0, 0))?);
                            self.compile(sub)?;
                        }
                        let end = self.prog.len();
                        for s in splits {
                            self.prog[s] = Inst::Split(s + 1, end);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Regex {
    /// Compile a pattern. The error is GNU's message for it.
    pub fn new(pattern: &[u8], syn: Syntax) -> Result<Regex, String> {
        let mut p = Parser { p: pattern, i: 0, syn, ngroups: 0, closed: Vec::new(), has_backref: false, warnings: Vec::new() };
        let ast = p.parse_alt(0)?;
        debug_assert!(p.i == pattern.len());
        let mut c = Compiler { prog: Vec::new(), sets: Vec::new() };
        if !p.has_backref {
            c.compile(&ast)?;
            c.emit(Inst::Match)?;
        }
        Ok(Regex {
            ast,
            ngroups: p.ngroups,
            has_backref: p.has_backref,
            icase: syn.icase,
            prog: c.prog,
            sets: c.sets,
            scratch: RefCell::default(),
            warnings: p.warnings,
        })
    }

    fn from_ast(ast: Node, icase: bool) -> Option<Regex> {
        let mut c = Compiler { prog: Vec::new(), sets: Vec::new() };
        c.compile(&ast).ok()?;
        c.emit(Inst::Match).ok()?;
        Some(Regex { ast, ngroups: 0, has_backref: false, icase, prog: c.prog, sets: c.sets, scratch: RefCell::default(), warnings: Vec::new() })
    }

    /// One pattern that matches wherever any of `parts` does. None if one
    /// of them has a back-reference. Groups are not kept.
    pub fn any_of(parts: &[Regex]) -> Option<Regex> {
        if parts.iter().any(|r| r.has_backref) {
            return None;
        }
        let icase = parts.iter().any(|r| r.icase);
        Regex::from_ast(Node::Alt(parts.iter().map(|r| r.ast.clone()).collect()), icase)
    }

    /// A pattern that matches any of the given fixed strings.
    pub fn from_strings(strings: &[Vec<u8>], icase: bool) -> Option<Regex> {
        let lit = |c: u8| {
            if icase && c.is_ascii_alphabetic() {
                Node::Set(set_of(|b| b.eq_ignore_ascii_case(&c)))
            } else {
                Node::Set(set_of(|b| b == c))
            }
        };
        let alt = strings.iter().map(|s| Node::Cat(s.iter().map(|c| lit(*c)).collect())).collect();
        Regex::from_ast(Node::Alt(alt), icase)
    }

    pub fn has_backref(&self) -> bool {
        self.has_backref
    }

    /// If a line matches exactly when it contains one of a few fixed
    /// strings, those strings. An empty string among them means every line
    /// matches.
    pub fn as_literals(&self) -> Option<Literals> {
        if self.has_backref || has_assert(&self.ast) {
            return None;
        }
        Some(Literals { strings: self.exact(&self.ast)?, fold: self.icase })
    }

    /// Fixed strings of which every match contains at least one, chosen
    /// to be as long as the pattern allows. A line without any of them
    /// cannot match.
    pub fn required_literals(&self) -> Option<Literals> {
        Some(Literals { strings: self.required(&self.ast)?, fold: self.icase })
    }

    /// The one byte a set stands for, if it is a single byte or, in a
    /// pattern that ignores case, the two cases of one letter (given in
    /// lower case).
    fn single(&self, set: &Set) -> Option<u8> {
        let mut members = set.iter().enumerate().filter(|(_, m)| **m).map(|(b, _)| b as u8);
        let first = members.next()?;
        match (members.next(), members.next()) {
            (None, _) if !(self.icase && first.is_ascii_alphabetic()) => Some(first),
            (Some(second), None) if self.icase && first.is_ascii_uppercase() && second == first.to_ascii_lowercase() => Some(second),
            _ => None,
        }
    }

    /// Every string the node can match, if there are only a few.
    fn exact(&self, n: &Node) -> Option<Vec<Vec<u8>>> {
        match n {
            Node::Empty | Node::Assert(_) => Some(vec![Vec::new()]),
            Node::Set(s) => Some(vec![vec![self.single(s)?]]),
            Node::Cat(v) => {
                let mut all = vec![Vec::new()];
                for x in v {
                    all = cross(&all, &self.exact(x)?)?;
                }
                Some(all)
            }
            Node::Alt(v) => {
                let mut all: Vec<Vec<u8>> = Vec::new();
                for x in v {
                    for s in self.exact(x)? {
                        if !all.contains(&s) {
                            all.push(s);
                        }
                    }
                }
                (all.len() <= MAX_LITERALS).then_some(all)
            }
            Node::Repeat { sub, min, max } if Some(*min) == *max && *min <= 8 => {
                let one = self.exact(sub)?;
                let mut all = vec![Vec::new()];
                for _ in 0..*min {
                    all = cross(&all, &one)?;
                }
                Some(all)
            }
            Node::Group(sub, _) => self.exact(sub),
            Node::Repeat { .. } | Node::Backref(_) => None,
        }
    }

    fn required(&self, n: &Node) -> Option<Vec<Vec<u8>>> {
        let usable = |set: Vec<Vec<u8>>| if set.iter().all(|s| !s.is_empty()) { Some(set) } else { None };
        if let Some(all) = self.exact(n) {
            return usable(all);
        }
        match n {
            Node::Cat(v) => {
                // Neighbours that are fixed strings join into longer ones;
                // the best of those runs and of what the other parts
                // require is kept.
                let mut best: Option<Vec<Vec<u8>>> = None;
                let mut consider = |set: Option<Vec<Vec<u8>>>| {
                    if let Some(set) = set.and_then(usable) {
                        let score = |s: &Vec<Vec<u8>>| (s.iter().map(|x| x.len()).min().unwrap_or(0), std::cmp::Reverse(s.len()));
                        if best.as_ref().map_or(true, |b| score(&set) > score(b)) {
                            best = Some(set);
                        }
                    }
                };
                let mut run = vec![Vec::new()];
                for x in v {
                    match self.exact(x) {
                        Some(e) => match cross(&run, &e) {
                            Some(joined) => run = joined,
                            None => {
                                consider(Some(std::mem::replace(&mut run, e)));
                            }
                        },
                        None => {
                            consider(Some(std::mem::replace(&mut run, vec![Vec::new()])));
                            consider(self.required(x));
                        }
                    }
                }
                consider(Some(run));
                best
            }
            Node::Alt(v) => {
                let mut all: Vec<Vec<u8>> = Vec::new();
                for x in v {
                    for s in self.required(x)? {
                        if !all.contains(&s) {
                            all.push(s);
                        }
                    }
                }
                (all.len() <= MAX_LITERALS).then_some(all)
            }
            Node::Repeat { sub, min, .. } if *min >= 1 => self.required(sub),
            Node::Group(sub, _) => self.required(sub),
            _ => None,
        }
    }

    pub fn group_count(&self) -> usize {
        self.ngroups
    }

    pub fn is_match(&self, hay: &[u8]) -> Result<bool, MatchError> {
        if self.has_backref {
            Ok(self.backtrack_search(hay, 0)?.is_some())
        } else {
            Ok(self.pike(hay, 0, true).is_some())
        }
    }

    /// The leftmost-longest match starting at or after `start`. Group spans
    /// are filled in only when `want_groups` is set.
    pub fn find_at(&self, hay: &[u8], start: usize, want_groups: bool) -> Result<Option<Caps>, MatchError> {
        if self.has_backref {
            return self.backtrack_search(hay, start);
        }
        let (s, e) = match self.pike(hay, start, false) {
            None => return Ok(None),
            Some(span) => span,
        };
        if !want_groups || self.ngroups == 0 {
            let mut caps = vec![None; self.ngroups + 1];
            caps[0] = Some((s, e));
            return Ok(Some(caps));
        }
        // Recover the groups by backtracking over the known span.
        let mut bt = Backtracker::new(hay, self);
        let mut found: Option<Caps> = None;
        bt.run(&self.ast, s, &mut |b, p| {
            if p == e {
                found = Some(b.caps.clone());
                true
            } else {
                false
            }
        });
        if bt.overflow {
            return Err(MatchError::TooComplex);
        }
        let mut caps = found.expect("the two matchers disagree");
        caps[0] = Some((s, e));
        Ok(Some(caps))
    }

    /// Thompson NFA simulation. Threads carry the position they started at;
    /// an earlier start always wins, and for one start the last match seen
    /// is the longest.
    fn pike(&self, hay: &[u8], start: usize, earliest: bool) -> Option<(usize, usize)> {
        let mut guard = self.scratch.borrow_mut();
        let scratch = &mut *guard;
        let n = self.prog.len();
        if scratch.mark.len() != n {
            scratch.mark = vec![0; n];
            scratch.epoch = 1;
        }
        let epoch = scratch.epoch;
        scratch.epoch += hay.len() as u64 + 2;
        let Scratch { clist, nlist, mark, stack, .. } = scratch;
        clist.clear();
        let mut best: Option<(usize, usize)> = None;
        let mut pos = start;
        loop {
            // The marks for position `pos` were set while building clist.
            if best.is_none() {
                self.add_thread(clist, mark, stack, 0, pos, hay, pos, epoch);
            }
            if clist.is_empty() && best.is_some() {
                break;
            }
            nlist.clear();
            for &(pc, st) in clist.iter() {
                if let Some((bs, _)) = best {
                    if st > bs {
                        continue;
                    }
                }
                match &self.prog[pc] {
                    Inst::Match => {
                        let better = match best {
                            None => true,
                            Some((bs, be)) => st < bs || (st == bs && pos > be),
                        };
                        if better {
                            best = Some((st, pos));
                            if earliest {
                                return best;
                            }
                        }
                    }
                    Inst::Set(s) => {
                        if pos < hay.len() && self.sets[*s][hay[pos] as usize] {
                            self.add_thread(nlist, mark, stack, pc + 1, st, hay, pos + 1, epoch);
                        }
                    }
                    _ => {}
                }
            }
            if pos >= hay.len() {
                break;
            }
            pos += 1;
            std::mem::swap(clist, nlist);
        }
        best
    }

    /// Add a thread and everything reachable from it without consuming
    /// input. `mark[pc]` records the last position at which pc was added.
    #[allow(clippy::too_many_arguments)]
    fn add_thread(
        &self,
        list: &mut Vec<(usize, usize)>,
        mark: &mut [u64],
        stack: &mut Vec<usize>,
        pc: usize,
        st: usize,
        hay: &[u8],
        pos: usize,
        epoch: u64,
    ) {
        let stamp = epoch + pos as u64;
        stack.push(pc);
        while let Some(pc) = stack.pop() {
            if mark[pc] == stamp {
                continue;
            }
            mark[pc] = stamp;
            match &self.prog[pc] {
                Inst::Jmp(t) => stack.push(*t),
                Inst::Split(a, b) => {
                    stack.push(*b);
                    stack.push(*a);
                }
                Inst::Assert(a) => {
                    if check_assert(*a, hay, pos) {
                        stack.push(pc + 1);
                    }
                }
                Inst::Set(_) | Inst::Match => list.push((pc, st)),
            }
        }
    }

    fn backtrack_search(&self, hay: &[u8], start: usize) -> Result<Option<Caps>, MatchError> {
        for s in start..=hay.len() {
            let mut bt = Backtracker::new(hay, self);
            let mut best: Option<(usize, Caps)> = None;
            bt.run(&self.ast, s, &mut |b, p| {
                if best.as_ref().map_or(true, |(e, _)| p > *e) {
                    best = Some((p, b.caps.clone()));
                }
                // Stop early only when nothing longer is possible.
                p == b.hay.len()
            });
            if bt.overflow {
                return Err(MatchError::TooComplex);
            }
            if let Some((e, mut caps)) = best {
                caps[0] = Some((s, e));
                return Ok(Some(caps));
            }
        }
        Ok(None)
    }
}

// ------------------------------------------------------------ backtracking

struct Backtracker<'a> {
    hay: &'a [u8],
    icase: bool,
    caps: Caps,
    steps: usize,
    overflow: bool,
}

type Cont<'k, 'a> = &'k mut dyn FnMut(&mut Backtracker<'a>, usize) -> bool;

impl<'a> Backtracker<'a> {
    fn new(hay: &'a [u8], re: &Regex) -> Backtracker<'a> {
        Backtracker { hay, icase: re.icase, caps: vec![None; re.ngroups + 1], steps: 0, overflow: false }
    }

    /// Match `n` at `pos`, calling `k` with the end of every way it can
    /// match, most-preferred first. Returns true once `k` accepts.
    fn run(&mut self, n: &Node, pos: usize, k: Cont<'_, 'a>) -> bool {
        self.steps += 1;
        if self.steps > STEP_LIMIT {
            self.overflow = true;
            return true;
        }
        match n {
            Node::Empty => k(self, pos),
            Node::Set(s) => pos < self.hay.len() && s[self.hay[pos] as usize] && k(self, pos + 1),
            Node::Assert(a) => check_assert(*a, self.hay, pos) && k(self, pos),
            Node::Cat(v) => self.cat(v, 0, pos, k),
            Node::Alt(v) => {
                for b in v {
                    if self.run(b, pos, k) {
                        return true;
                    }
                }
                false
            }
            Node::Group(sub, i) => {
                let i = *i;
                let old = self.caps[i];
                let r = self.run(sub, pos, &mut |b, p| {
                    let prev = b.caps[i];
                    b.caps[i] = Some((pos, p));
                    if k(b, p) {
                        return true;
                    }
                    b.caps[i] = prev;
                    false
                });
                if !r {
                    self.caps[i] = old;
                }
                r
            }
            Node::Backref(i) => match self.caps[*i] {
                None => false,
                Some((a, b)) => {
                    let len = b - a;
                    if pos + len > self.hay.len() {
                        return false;
                    }
                    let (x, y) = (&self.hay[a..b], &self.hay[pos..pos + len]);
                    let same = if self.icase { x.eq_ignore_ascii_case(y) } else { x == y };
                    same && k(self, pos + len)
                }
            },
            Node::Repeat { sub, min, max } => {
                if let Node::Set(s) = &**sub {
                    // A repeated single character needs no recursion.
                    let limit = max.map_or(usize::MAX, |m| m as usize);
                    let mut run = 0;
                    while run < limit && pos + run < self.hay.len() && s[self.hay[pos + run] as usize] {
                        run += 1;
                    }
                    let min = *min as usize;
                    if run < min {
                        return false;
                    }
                    for c in (min..=run).rev() {
                        if k(self, pos + c) {
                            return true;
                        }
                    }
                    return false;
                }
                self.rep(sub, *min, *max, 0, pos, k)
            }
        }
    }

    fn cat(&mut self, v: &[Node], i: usize, pos: usize, k: Cont<'_, 'a>) -> bool {
        if i == v.len() {
            return k(self, pos);
        }
        self.run(&v[i], pos, &mut |b, p| b.cat(v, i + 1, p, k))
    }

    fn rep(&mut self, sub: &Node, min: u32, max: Option<u32>, count: u32, pos: usize, k: Cont<'_, 'a>) -> bool {
        if max.map_or(true, |m| count < m) {
            let more = self.run(sub, pos, &mut |b, p| {
                // In an unbounded repetition, an iteration that consumes
                // nothing cannot help once the minimum is met, and would
                // loop forever. A bounded one may match empty, as in glibc,
                // which changes what a group inside it captures.
                if p == pos && count >= min && max.is_none() {
                    return false;
                }
                b.rep(sub, min, max, count + 1, p, k)
            });
            if more {
                return true;
            }
        }
        count >= min && k(self, pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn re(p: &str, extended: bool) -> Regex {
        Regex::new(p.as_bytes(), Syntax { extended, icase: false, flavor: Flavor::Sed }).unwrap()
    }

    fn span(p: &str, extended: bool, hay: &str) -> Option<(usize, usize)> {
        re(p, extended).find_at(hay.as_bytes(), 0, false).unwrap().map(|c| c[0].unwrap())
    }

    #[test]
    fn leftmost_longest() {
        assert_eq!(span("a|ab", true, "ab"), Some((0, 2)));
        assert_eq!(span("a\\|ab", false, "ab"), Some((0, 2)));
        assert_eq!(span("(a|ab)(c|bcd)?", true, "abcd"), Some((0, 4)));
        assert_eq!(span("b*", true, "abc"), Some((0, 0)));
        assert_eq!(span("x*$", true, "abc"), Some((3, 3)));
    }

    #[test]
    fn bre_rules() {
        assert_eq!(span("*a", false, "x*a"), Some((1, 3)));
        assert_eq!(span("a^b", false, "a^b"), Some((0, 3)));
        assert_eq!(span("a$b", false, "a$b"), Some((0, 3)));
        assert_eq!(span("a\\{2,3\\}", false, "aaaa"), Some((0, 3)));
        assert_eq!(span("\\(ab\\)\\1", false, "xababx"), Some((1, 5)));
        assert_eq!(span("a+", false, "aa+"), Some((1, 3)));
    }

    #[test]
    fn groups() {
        let r = re("(a|ab)(c|bcd)(d*)", true);
        let c = r.find_at(b"abcd", 0, true).unwrap().unwrap();
        assert_eq!(c[1], Some((0, 1)));
        assert_eq!(c[2], Some((1, 4)));
        assert_eq!(c[3], Some((4, 4)));
    }

    #[test]
    fn errors() {
        let e = |p: &str, x: bool| Regex::new(p.as_bytes(), Syntax { extended: x, icase: false, flavor: Flavor::Grep }).unwrap_err();
        assert_eq!(e("[a", false), "Unmatched [, [^, [:, [., or [=");
        assert_eq!(e("\\(", false), "Unmatched ( or \\(");
        assert_eq!(e("a\\{1", false), "Unmatched \\{");
        assert_eq!(e("a{2,1}", true), "Invalid content of \\{\\}");
        assert_eq!(e("a\\", false), "Trailing backslash");
        assert_eq!(e("[[:foo:]]", false), "Invalid character class name");
        assert_eq!(e("[b-a]", false), "Invalid range end");
    }
}
