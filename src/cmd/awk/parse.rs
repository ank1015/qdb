//! The awk program text: tokens, then a tree.
//!
//! The grammar follows gawk's: concatenation binds less than `+` and `-`,
//! comparisons do not chain, `in` binds less than `~`, and a `>` in a
//! `print` that is not inside parentheses would be a redirection. What
//! the engine does not run is refused by name (`Stop::Unsupported`); what
//! gawk itself rejects is a syntax error with gawk's two lines.

use std::rc::Rc;

/// Slots every program has, in this order.
#[allow(dead_code)]
pub const NF: usize = 0;
pub const NR: usize = 1;
pub const FNR: usize = 2;
pub const FILENAME: usize = 3;
pub const FS: usize = 4;
pub const OFS: usize = 5;
pub const ORS: usize = 6;
pub const RSTART: usize = 7;
pub const RLENGTH: usize = 8;
const SPECIAL: [&str; 9] = ["NF", "NR", "FNR", "FILENAME", "FS", "OFS", "ORS", "RSTART", "RLENGTH"];

/// Variables of gawk that this awk has no meaning for.
const REFUSED_VARS: [&str; 19] = [
    "RS", "SUBSEP", "CONVFMT", "OFMT", "ENVIRON", "ARGC", "ARGV", "ARGIND", "IGNORECASE", "FIELDWIDTHS", "FPAT", "RT", "PROCINFO", "ERRNO", "BINMODE", "LINT",
    "TEXTDOMAIN", "SYMTAB", "FUNCTAB",
];

/// Functions of gawk that are not in this awk.
const REFUSED_FUNCS: [&str; 33] = [
    "atan2", "cos", "sin", "exp", "log", "sqrt", "int", "rand", "srand", "system", "close", "fflush", "systime", "strftime", "mktime", "gensub", "patsplit",
    "strtonum", "asort", "asorti", "and", "or", "xor", "compl", "lshift", "rshift", "isarray", "typeof", "bindtextdomain", "dcgettext", "dcngettext", "mkbool",
    "getline",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Length,
    Substr,
    Index,
    Split,
    Sub,
    Gsub,
    Match,
    Tolower,
    Toupper,
    Sprintf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rel {
    Lt,
    Le,
    Eq,
    Ne,
    Gt,
    Ge,
}

/// A regular expression operand: one written between slashes, or a value
/// whose text is compiled when it is used.
#[derive(Debug)]
pub enum Re {
    Fixed(usize),
    Value(Box<Expr>),
}

#[derive(Debug)]
pub enum Lv {
    Var(usize),
    Nf,
    Field(Box<Expr>),
    Elem(usize, Box<Expr>),
}

#[derive(Debug)]
pub enum Expr {
    Num(f64),
    Str(Rc<[u8]>),
    /// `/re/` standing alone: does the record match.
    Regex(usize),
    Var(usize),
    Nf,
    /// `$N` with N written as a whole number.
    FieldN(usize),
    Field(Box<Expr>),
    Elem(usize, Box<Expr>),
    In(Box<Expr>, usize),
    Group(Box<Expr>),
    Assign(Lv, Box<Expr>),
    /// `lv op= e`; the line is for the message of a division by zero.
    Update(Lv, Arith, Box<Expr>, u32),
    /// `++lv`, `lv--`: the step, and whether the old value is the result.
    Step(Lv, f64, bool),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Plus(Box<Expr>),
    Arith(Arith, Box<Expr>, Box<Expr>, u32),
    Concat(Vec<Expr>),
    Cmp(Rel, Box<Expr>, Box<Expr>),
    Match(bool, Box<Expr>, Re),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    Length(Option<Box<Expr>>),
    LengthOf(usize),
    Substr(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Index(Box<Expr>, Box<Expr>),
    Split(Box<Expr>, usize, Option<Re>),
    Subst(bool, Re, Box<Expr>, Option<Lv>),
    MatchFn(Box<Expr>, Re),
    Lower(Box<Expr>),
    Upper(Box<Expr>),
    Sprintf(Vec<Expr>, u32),
}

#[derive(Debug)]
pub enum Stmt {
    Expr(Expr, u32),
    Print(Vec<Expr>, u32),
    Printf(Vec<Expr>, u32),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>, u32),
    While(Expr, Box<Stmt>, u32),
    For(Option<Expr>, Option<Expr>, Option<Expr>, Box<Stmt>, u32),
    ForIn(Lv, usize, Box<Stmt>),
    Block(Vec<Stmt>),
    Next,
    Break,
    Continue,
}

#[derive(Debug)]
pub enum Pattern {
    Always,
    Expr(Expr),
    /// From a record matching the first to one matching the second; the
    /// number is the slot that remembers being inside.
    Range(Expr, Expr, usize),
}

#[derive(Debug)]
pub struct Rule {
    pub line: u32,
    pub pattern: Pattern,
    /// None is `{ print }`.
    pub action: Option<Vec<Stmt>>,
}

#[derive(Debug, Default)]
pub struct Program {
    pub begin: Vec<Vec<Stmt>>,
    pub rules: Vec<Rule>,
    pub end: Vec<Vec<Stmt>>,
    pub has_end: bool,
    pub names: Vec<String>,
    /// For each name: used as a plain variable, used as an array.
    pub uses: Vec<(bool, bool)>,
    /// For each name: its first use was as an array.
    pub array_first: Vec<bool>,
    /// The regular expressions between slashes, as written.
    pub regexes: Vec<(Vec<u8>, u32)>,
    pub ranges: usize,
    /// Lines gawk prints about the text before it runs it.
    pub warnings: Vec<String>,
}

pub enum Stop {
    /// Not in this awk: the words go after "unsupported ".
    Unsupported(String),
    /// What gawk prints for a program it cannot read, and its exit status.
    Syntax(Vec<String>, i32),
}

pub fn refused_variable(name: &str) -> bool {
    REFUSED_VARS.contains(&name)
}

type PResult<T> = Result<T, Stop>;

fn refuse<T>(what: impl Into<String>) -> PResult<T> {
    Err(Stop::Unsupported(format!("awk feature: {}", what.into())))
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(Vec<u8>),
    Name(String),
    Func(Func),
    /// Punctuation and keywords, by their text.
    P(&'static str),
    Newline,
    Eof,
}

const KEYWORDS: [&str; 12] = ["BEGIN", "END", "if", "else", "while", "for", "in", "next", "break", "continue", "print", "printf"];
const REFUSED_WORDS: [(&str, &str); 13] = [
    ("getline", "getline"),
    ("function", "user-defined functions"),
    ("func", "user-defined functions"),
    ("return", "user-defined functions"),
    ("delete", "delete"),
    ("exit", "exit"),
    ("do", "do ... while"),
    ("nextfile", "nextfile"),
    ("switch", "switch"),
    ("case", "switch"),
    ("default", "switch"),
    ("BEGINFILE", "BEGINFILE"),
    ("ENDFILE", "ENDFILE"),
];
const PUNCT: [&str; 43] = [
    "**=", "&&", "||", "++", "--", "+=", "-=", "*=", "/=", "%=", "^=", "**", "==", "!=", "<=", ">=", "!~", ">>", "|&", "+", "-", "*", "/", "%", "^", "!", ">",
    "<", "|", "?", ":", "~", "$", "=", ",", ";", "(", ")", "[", "]", "{", "}", "@",
];

#[derive(Clone)]
struct Mark {
    pos: usize,
    line: u32,
    tok: Tok,
    tok_start: usize,
    tok_line: u32,
    warnings: usize,
    regexes: usize,
}

pub struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    tok: Tok,
    tok_start: usize,
    tok_line: u32,
    /// Where the token before this one ended.
    prev_end: usize,
    prog: Program,
    /// Inside the list of a `print` and outside every parenthesis: `>`
    /// is not a comparison there.
    no_gt: bool,
    loops: u32,
    in_begin_end: Option<&'static str>,
}

fn name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a [u8]) -> Parser<'a> {
        let mut prog = Program::default();
        for n in SPECIAL {
            prog.names.push(n.to_string());
            prog.uses.push((true, false));
            prog.array_first.push(false);
        }
        Parser { src, pos: 0, line: 1, tok: Tok::Eof, tok_start: 0, tok_line: 1, prev_end: 0, prog, no_gt: false, loops: 0, in_begin_end: None }
    }

    // ------------------------------------------------------------ tokens

    fn mark(&self) -> Mark {
        Mark {
            pos: self.pos,
            line: self.line,
            tok: self.tok.clone(),
            tok_start: self.tok_start,
            tok_line: self.tok_line,
            warnings: self.prog.warnings.len(),
            regexes: self.prog.regexes.len(),
        }
    }

    fn reset(&mut self, m: Mark) {
        self.pos = m.pos;
        self.line = m.line;
        self.tok = m.tok;
        self.tok_start = m.tok_start;
        self.tok_line = m.tok_line;
        self.prog.warnings.truncate(m.warnings);
        self.prog.regexes.truncate(m.regexes);
    }

    fn warn(&mut self, text: String) {
        let line = self.line;
        self.prog.warnings.push(format!("awk: cmd. line:{}: warning: {}", line, text));
    }

    /// The line of the text that holds `at`, and gawk's two lines for a
    /// complaint about that place: the line, and a mark under column `col`.
    fn marked<T>(&self, line: u32, at: usize, col: Option<usize>, what: &str) -> PResult<T> {
        let at = at.min(self.src.len());
        let start = self.src[..at].iter().rposition(|c| *c == b'\n').map_or(0, |i| i + 1);
        let end = self.src[at..].iter().position(|c| *c == b'\n').map_or(self.src.len(), |i| at + i);
        let col = col.unwrap_or(at - start);
        let text = String::from_utf8_lossy(&self.src[start..end]).into_owned();
        // A tab in the text is a tab under it, so the mark stays in place.
        let pad: String = self.src[start..end].iter().chain(std::iter::repeat(&b' ')).take(col).map(|c| if *c == b'\t' { '\t' } else { ' ' }).collect();
        Err(Stop::Syntax(vec![format!("awk: cmd. line:{}: {}", line, text), format!("awk: cmd. line:{}: {}^ {}", line, pad, what)], 1))
    }

    /// gawk's two lines for a token it did not expect.
    fn syntax<T>(&self) -> PResult<T> {
        match self.tok {
            // The line has been counted by the time its end is refused.
            Tok::Newline => self.marked(self.tok_line + 1, self.tok_start, None, "unexpected newline or end of string"),
            Tok::Eof => {
                let last = if self.src.ends_with(b"\n") { self.src.len() - 1 } else { self.src.len() };
                let start = self.src[..last].iter().rposition(|c| *c == b'\n').map_or(0, |i| i + 1);
                self.marked(self.line, last, Some(last - start), "unexpected newline or end of string")
            }
            _ => self.marked(self.tok_line, self.tok_start, None, "syntax error"),
        }
    }

    fn error<T>(&self, text: &str, twice: bool) -> PResult<T> {
        let l = format!("awk: cmd. line:{}: error: {}", self.tok_line, text);
        Err(Stop::Syntax(if twice { vec![l.clone(), l] } else { vec![l] }, 1))
    }

    fn skip_blank(&mut self) {
        loop {
            match self.src.get(self.pos) {
                Some(b' ' | b'\t' | b'\r') => self.pos += 1,
                Some(b'\\') if self.src.get(self.pos + 1) == Some(&b'\n') => {
                    self.pos += 2;
                    self.line += 1;
                }
                Some(b'#') => {
                    while self.src.get(self.pos).is_some_and(|c| *c != b'\n') {
                        self.pos += 1;
                    }
                }
                _ => return,
            }
        }
    }

    fn advance(&mut self) -> PResult<()> {
        self.prev_end = self.pos;
        self.skip_blank();
        self.tok_start = self.pos;
        self.tok_line = self.line;
        let c = match self.src.get(self.pos) {
            None => {
                // The text ends as if with a newline.
                self.tok = if self.tok == Tok::Newline || self.tok == Tok::Eof || self.pos == 0 { Tok::Eof } else { Tok::Newline };
                return Ok(());
            }
            Some(c) => *c,
        };
        if c == b'\n' {
            self.pos += 1;
            self.line += 1;
            self.tok = Tok::Newline;
            return Ok(());
        }
        if c.is_ascii_digit() || c == b'.' && self.src.get(self.pos + 1).is_some_and(|d| d.is_ascii_digit()) {
            self.tok = Tok::Num(self.number());
            return Ok(());
        }
        if c == b'"' {
            self.tok = Tok::Str(self.string()?);
            return Ok(());
        }
        if name_start(c) {
            let from = self.pos;
            while self.src.get(self.pos).is_some_and(|c| name_start(*c) || c.is_ascii_digit()) {
                self.pos += 1;
            }
            let word = std::str::from_utf8(&self.src[from..self.pos]).unwrap();
            if let Some(k) = KEYWORDS.iter().find(|k| **k == word) {
                self.tok = Tok::P(k);
                return Ok(());
            }
            if let Some((_, what)) = REFUSED_WORDS.iter().find(|(k, _)| *k == word) {
                return refuse(*what);
            }
            let func = match word {
                "length" => Some(Func::Length),
                "substr" => Some(Func::Substr),
                "index" => Some(Func::Index),
                "split" => Some(Func::Split),
                "sub" => Some(Func::Sub),
                "gsub" => Some(Func::Gsub),
                "match" => Some(Func::Match),
                "tolower" => Some(Func::Tolower),
                "toupper" => Some(Func::Toupper),
                "sprintf" => Some(Func::Sprintf),
                _ => None,
            };
            if let Some(f) = func {
                self.tok = Tok::Func(f);
                return Ok(());
            }
            if REFUSED_FUNCS.contains(&word) {
                return Err(Stop::Unsupported(format!("awk function: {}", word)));
            }
            if REFUSED_VARS.contains(&word) {
                return Err(Stop::Unsupported(format!("awk variable: {}", word)));
            }
            if self.src.get(self.pos) == Some(&b'(') {
                return refuse(format!("a call of {} (user-defined functions)", word));
            }
            self.tok = Tok::Name(word.to_string());
            return Ok(());
        }
        for p in PUNCT {
            if self.src[self.pos..].starts_with(p.as_bytes()) {
                self.pos += p.len();
                self.tok = Tok::P(p);
                return match p {
                    "|" | "|&" => refuse("a pipe (|)"),
                    ">>" => refuse("output redirection (>>)"),
                    "@" => refuse("@ directives"),
                    _ => Ok(()),
                };
            }
        }
        self.pos += 1;
        self.tok = Tok::P("?bad");
        self.syntax()
    }

    fn number(&mut self) -> f64 {
        let s = self.src;
        let from = self.pos;
        if s[from] == b'0' && matches!(s.get(from + 1), Some(b'x' | b'X')) && s.get(from + 2).is_some_and(|c| c.is_ascii_hexdigit()) {
            self.pos += 2;
            let mut v = 0f64;
            while let Some(d) = s.get(self.pos).and_then(|c| (*c as char).to_digit(16)) {
                v = v * 16.0 + d as f64;
                self.pos += 1;
            }
            return v;
        }
        let digits = |p: &mut usize| {
            while s.get(*p).is_some_and(|c| c.is_ascii_digit()) {
                *p += 1;
            }
        };
        digits(&mut self.pos);
        let mut plain = true;
        if s.get(self.pos) == Some(&b'.') {
            plain = false;
            self.pos += 1;
            digits(&mut self.pos);
        }
        if matches!(s.get(self.pos), Some(b'e' | b'E')) {
            let mut k = self.pos + 1;
            if matches!(s.get(k), Some(b'+' | b'-')) {
                k += 1;
            }
            if s.get(k).is_some_and(|c| c.is_ascii_digit()) {
                plain = false;
                digits(&mut k);
                self.pos = k;
            }
        }
        let text = &s[from..self.pos];
        if plain && text.len() > 1 && text[0] == b'0' && text.iter().all(|c| (b'0'..=b'7').contains(c)) {
            return text.iter().fold(0f64, |v, c| v * 8.0 + (c - b'0') as f64);
        }
        std::str::from_utf8(text).unwrap().parse().unwrap_or(0.0)
    }

    fn string(&mut self) -> PResult<Vec<u8>> {
        let mut out = Vec::new();
        self.pos += 1;
        loop {
            let c = match self.src.get(self.pos) {
                None | Some(b'\n') => {
                    return self.marked(self.tok_line, self.tok_start, None, "unterminated string");
                }
                Some(c) => *c,
            };
            self.pos += 1;
            if c == b'"' {
                return Ok(out);
            }
            if c != b'\\' {
                out.push(c);
                continue;
            }
            let e = match self.src.get(self.pos) {
                None => {
                    out.push(b'\\');
                    continue;
                }
                Some(e) => *e,
            };
            self.pos += 1;
            match e {
                b'\n' => self.line += 1,
                b'"' | b'\\' => out.push(e),
                b'/' => {
                    self.warn("escape sequence `\\/' treated as plain `/'".to_string());
                    out.push(b'/');
                }
                b'a' => out.push(7),
                b'b' => out.push(8),
                b'f' => out.push(12),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'v' => out.push(11),
                b'0'..=b'7' => {
                    let mut v = (e - b'0') as u32;
                    for _ in 0..2 {
                        match self.src.get(self.pos) {
                            Some(d @ b'0'..=b'7') => {
                                v = v * 8 + (d - b'0') as u32;
                                self.pos += 1;
                            }
                            _ => break,
                        }
                    }
                    out.push(v as u8);
                }
                b'x' if self.src.get(self.pos).is_some_and(|c| c.is_ascii_hexdigit()) => {
                    let mut v = 0u32;
                    for _ in 0..2 {
                        match self.src.get(self.pos).and_then(|c| (*c as char).to_digit(16)) {
                            Some(d) => {
                                v = v * 16 + d;
                                self.pos += 1;
                            }
                            None => break,
                        }
                    }
                    out.push(v as u8);
                }
                _ => {
                    self.warn(format!("escape sequence `\\{}' treated as plain `{}'", e as char, e as char));
                    out.push(e);
                }
            }
        }
    }

    /// The current token is a `/` or `/=` where a value must start: read
    /// from its slash to the closing one as a regular expression.
    fn regex_here(&mut self) -> PResult<usize> {
        let mut i = self.tok_start + 1;
        let mut in_set = false;
        loop {
            match self.src.get(i) {
                None | Some(b'\n') => {
                    return self.marked(self.tok_line, self.tok_start + 1, None, "unterminated regexp");
                }
                Some(b'\\') => i += 1,
                Some(b'[') if !in_set => {
                    in_set = true;
                    // A `]` right after the opening is a member.
                    if self.src.get(i + 1) == Some(&b'^') {
                        i += 1;
                    }
                    if self.src.get(i + 1) == Some(&b']') {
                        i += 1;
                    }
                }
                Some(b'[') if matches!(self.src.get(i + 1), Some(b':' | b'.' | b'=')) => {
                    let kind = self.src[i + 1];
                    let mut j = i + 2;
                    while j + 1 < self.src.len() && !(self.src[j] == kind && self.src[j + 1] == b']') && self.src[j] != b'\n' {
                        j += 1;
                    }
                    if j + 1 < self.src.len() && self.src[j] == kind {
                        i = j + 1;
                    }
                }
                Some(b']') if in_set => in_set = false,
                Some(b'/') if !in_set => break,
                _ => {}
            }
            i += 1;
        }
        let text = self.src[self.tok_start + 1..i].to_vec();
        self.pos = i + 1;
        for e in super::run::translate(&text).1 {
            self.warn(format!("regexp escape sequence `\\{}' is not a known regexp operator", e as char));
        }
        self.prog.regexes.push((text, self.tok_line));
        self.advance()?;
        Ok(self.prog.regexes.len() - 1)
    }

    fn is(&self, p: &str) -> bool {
        matches!(&self.tok, Tok::P(q) if *q == p)
    }

    fn eat(&mut self, p: &str) -> PResult<bool> {
        if self.is(p) {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect(&mut self, p: &str) -> PResult<()> {
        if !self.eat(p)? {
            return self.syntax();
        }
        Ok(())
    }

    fn newlines(&mut self) -> PResult<()> {
        while self.tok == Tok::Newline {
            self.advance()?;
        }
        Ok(())
    }

    fn intern(&mut self, name: &str, array: bool) -> usize {
        let id = match self.prog.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                self.prog.names.push(name.to_string());
                self.prog.uses.push((false, false));
                self.prog.array_first.push(false);
                self.prog.names.len() - 1
            }
        };
        if self.prog.uses[id] == (false, false) {
            self.prog.array_first[id] = array;
        }
        if array {
            self.prog.uses[id].1 = true;
        } else {
            self.prog.uses[id].0 = true;
        }
        id
    }

    // ----------------------------------------------------------- program

    pub fn program(mut self) -> PResult<Program> {
        self.advance()?;
        loop {
            while self.tok == Tok::Newline || self.is(";") {
                self.advance()?;
            }
            if self.tok == Tok::Eof {
                break;
            }
            if self.is("BEGIN") || self.is("END") {
                let begin = self.is("BEGIN");
                self.advance()?;
                if !self.is("{") {
                    let which = if begin { "BEGIN" } else { "END" };
                    return Err(Stop::Syntax(vec![format!("awk: cmd. line:{}: {} blocks must have an action part", self.tok_line, which)], 1));
                }
                self.in_begin_end = Some(if begin { "BEGIN" } else { "END" });
                let body = self.block()?;
                self.in_begin_end = None;
                if begin {
                    self.prog.begin.push(body);
                } else {
                    self.prog.has_end = true;
                    self.prog.end.push(body);
                }
                continue;
            }
            let line = self.tok_line;
            let pattern = if self.is("{") {
                Pattern::Always
            } else {
                let first = self.expr()?;
                if self.eat(",")? {
                    self.newlines()?;
                    let second = self.expr()?;
                    self.prog.ranges += 1;
                    Pattern::Range(first, second, self.prog.ranges - 1)
                } else {
                    Pattern::Expr(first)
                }
            };
            let action = if self.is("{") {
                Some(self.block()?)
            } else {
                if !(self.tok == Tok::Newline || self.tok == Tok::Eof || self.is(";")) {
                    return self.syntax();
                }
                None
            };
            self.prog.rules.push(Rule { line, pattern, action });
        }
        Ok(self.prog)
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect("{")?;
        let mut body = Vec::new();
        loop {
            while self.tok == Tok::Newline || self.is(";") {
                self.advance()?;
            }
            if self.eat("}")? {
                return Ok(body);
            }
            if self.tok == Tok::Eof {
                return self.syntax();
            }
            body.push(self.statement()?);
        }
    }

    /// The end of a simple statement: `;`, a newline, or the `}` or the
    /// end that follows it.
    fn terminator(&mut self) -> PResult<()> {
        if self.is(";") || self.tok == Tok::Newline {
            return self.advance();
        }
        if self.is("}") || self.tok == Tok::Eof {
            return Ok(());
        }
        self.syntax()
    }

    fn body(&mut self) -> PResult<Stmt> {
        self.newlines()?;
        if self.eat(";")? {
            return Ok(Stmt::Block(Vec::new()));
        }
        self.statement()
    }

    fn statement(&mut self) -> PResult<Stmt> {
        if self.is("{") {
            return Ok(Stmt::Block(self.block()?));
        }
        let at = self.tok_line;
        if self.eat("if")? {
            self.expect("(")?;
            let cond = self.grouped()?;
            self.expect(")")?;
            let then = self.body()?;
            // An `else` may follow newlines; the `;` of a simple
            // statement went with the statement.
            let m = self.mark();
            self.newlines()?;
            if self.eat("else")? {
                let other = self.body()?;
                return Ok(Stmt::If(cond, Box::new(then), Some(Box::new(other)), at));
            }
            self.reset(m);
            return Ok(Stmt::If(cond, Box::new(then), None, at));
        }
        if self.eat("while")? {
            self.expect("(")?;
            let cond = self.grouped()?;
            self.expect(")")?;
            self.loops += 1;
            let body = self.body();
            self.loops -= 1;
            return Ok(Stmt::While(cond, Box::new(body?), at));
        }
        if self.eat("for")? {
            self.expect("(")?;
            // `for (k in a)`
            let m = self.mark();
            if let Tok::Name(k) = self.tok.clone() {
                self.advance()?;
                if self.eat("in")? {
                    if let Tok::Name(a) = self.tok.clone() {
                        self.advance()?;
                        if self.eat(")")? {
                            let var = if k == "NF" { Lv::Nf } else { Lv::Var(self.intern(&k, false)) };
                            let arr = self.intern(&a, true);
                            self.loops += 1;
                            let body = self.body();
                            self.loops -= 1;
                            return Ok(Stmt::ForIn(var, arr, Box::new(body?)));
                        }
                    }
                }
            }
            self.reset(m);
            let init = if self.is(";") { None } else { Some(self.grouped()?) };
            self.expect(";")?;
            self.newlines()?;
            let cond = if self.is(";") { None } else { Some(self.grouped()?) };
            self.expect(";")?;
            self.newlines()?;
            let step = if self.is(")") { None } else { Some(self.grouped()?) };
            self.expect(")")?;
            self.loops += 1;
            let body = self.body();
            self.loops -= 1;
            return Ok(Stmt::For(init, cond, step, Box::new(body?), at));
        }
        let s = if self.is("next") {
            if let Some(which) = self.in_begin_end {
                return self.error(&format!("`next' used in {} action", which), false);
            }
            self.advance()?;
            Stmt::Next
        } else if self.is("break") || self.is("continue") {
            let brk = self.is("break");
            if self.loops == 0 {
                return self.error(if brk { "`break' is not allowed outside a loop or switch" } else { "`continue' is not allowed outside a loop" }, true);
            }
            self.advance()?;
            if brk { Stmt::Break } else { Stmt::Continue }
        } else if self.is("print") || self.is("printf") {
            let formatted = self.is("printf");
            let line = self.tok_line;
            self.advance()?;
            let list = self.print_list()?;
            if formatted {
                Stmt::Printf(list, line)
            } else {
                Stmt::Print(list, at)
            }
        } else {
            Stmt::Expr(self.expr()?, at)
        };
        self.terminator()?;
        Ok(s)
    }

    fn ends_statement(&self) -> bool {
        self.is(";") || self.is("}") || self.tok == Tok::Newline || self.tok == Tok::Eof
    }

    fn print_list(&mut self) -> PResult<Vec<Expr>> {
        if self.ends_statement() {
            return Ok(Vec::new());
        }
        if self.is(">") {
            return refuse("output redirection (>)");
        }
        // `print (a, b)`: a list in parentheses, if the statement ends there.
        if self.is("(") {
            let m = self.mark();
            self.advance()?;
            let mut list = Vec::new();
            let ok = loop {
                match self.grouped() {
                    Ok(e) => list.push(e),
                    Err(Stop::Unsupported(u)) => return Err(Stop::Unsupported(u)),
                    Err(_) => break false,
                }
                if self.is(",") {
                    self.advance()?;
                    self.newlines()?;
                    continue;
                }
                break self.is(")");
            };
            if ok {
                self.advance()?;
                if self.is(">") {
                    return refuse("output redirection (>)");
                }
                if self.ends_statement() && list.len() > 1 {
                    return Ok(list);
                }
            }
            self.reset(m);
        }
        let mut list = Vec::new();
        let was = std::mem::replace(&mut self.no_gt, true);
        let r = loop {
            match self.expr() {
                Ok(e) => list.push(e),
                Err(e) => break Err(e),
            }
            if self.is(",") {
                if let Err(e) = self.advance().and_then(|_| self.newlines()) {
                    break Err(e);
                }
                continue;
            }
            break Ok(());
        };
        self.no_gt = was;
        r?;
        if self.is(">") {
            return refuse("output redirection (>)");
        }
        Ok(list)
    }

    // ------------------------------------------------------- expressions

    /// An expression inside parentheses or brackets, where `>` compares.
    fn grouped(&mut self) -> PResult<Expr> {
        let was = std::mem::replace(&mut self.no_gt, false);
        let r = self.expr();
        self.no_gt = was;
        r
    }

    fn lvalue(e: Expr) -> Result<Lv, Expr> {
        match e {
            Expr::Var(i) => Ok(Lv::Var(i)),
            Expr::Nf => Ok(Lv::Nf),
            Expr::FieldN(n) => Ok(Lv::Field(Box::new(Expr::Num(n as f64)))),
            Expr::Field(i) => Ok(Lv::Field(i)),
            Expr::Elem(a, k) => Ok(Lv::Elem(a, k)),
            other => Err(other),
        }
    }

    fn is_lvalue(e: &Expr) -> bool {
        matches!(e, Expr::Var(_) | Expr::Nf | Expr::FieldN(_) | Expr::Field(_) | Expr::Elem(..))
    }

    pub fn expr(&mut self) -> PResult<Expr> {
        let left = self.ternary()?;
        let op = match &self.tok {
            Tok::P("=") => None,
            Tok::P("+=") => Some(Arith::Add),
            Tok::P("-=") => Some(Arith::Sub),
            Tok::P("*=") => Some(Arith::Mul),
            Tok::P("/=") => Some(Arith::Div),
            Tok::P("%=") => Some(Arith::Mod),
            Tok::P("^=") | Tok::P("**=") => Some(Arith::Pow),
            _ => return Ok(left),
        };
        let lv = match Self::lvalue(left) {
            Ok(lv) => lv,
            Err(_) => return self.syntax(),
        };
        let line = self.tok_line;
        self.advance()?;
        let right = Box::new(self.expr()?);
        Ok(match op {
            None => Expr::Assign(lv, right),
            Some(op) => Expr::Update(lv, op, right, line),
        })
    }

    fn ternary(&mut self) -> PResult<Expr> {
        let cond = self.or()?;
        if !self.eat("?")? {
            return Ok(cond);
        }
        self.newlines()?;
        let a = self.expr()?;
        self.newlines()?;
        self.expect(":")?;
        self.newlines()?;
        let b = self.expr()?;
        Ok(Expr::Cond(Box::new(cond), Box::new(a), Box::new(b)))
    }

    fn or(&mut self) -> PResult<Expr> {
        let mut left = self.and()?;
        while self.eat("||")? {
            self.newlines()?;
            let right = self.and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and(&mut self) -> PResult<Expr> {
        let mut left = self.membership()?;
        while self.eat("&&")? {
            self.newlines()?;
            let right = self.membership()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn membership(&mut self) -> PResult<Expr> {
        let mut left = self.matching()?;
        while self.is("in") {
            self.advance()?;
            let name = match self.tok.clone() {
                Tok::Name(n) => n,
                _ => return self.syntax(),
            };
            self.advance()?;
            let arr = self.intern(&name, true);
            left = Expr::In(Box::new(left), arr);
        }
        Ok(left)
    }

    fn re_operand(e: Expr) -> Re {
        match e {
            Expr::Regex(i) => Re::Fixed(i),
            other => Re::Value(Box::new(other)),
        }
    }

    fn matching(&mut self) -> PResult<Expr> {
        let mut left = self.relation()?;
        while self.is("~") || self.is("!~") {
            let negated = self.is("!~");
            self.advance()?;
            let right = self.relation()?;
            left = Expr::Match(negated, Box::new(left), Self::re_operand(right));
        }
        Ok(left)
    }

    fn relop(&self) -> Option<Rel> {
        Some(match &self.tok {
            Tok::P("<") => Rel::Lt,
            Tok::P("<=") => Rel::Le,
            Tok::P("==") => Rel::Eq,
            Tok::P("!=") => Rel::Ne,
            Tok::P(">=") => Rel::Ge,
            Tok::P(">") if !self.no_gt => Rel::Gt,
            _ => return None,
        })
    }

    fn relation(&mut self) -> PResult<Expr> {
        let left = self.concat()?;
        let op = match self.relop() {
            None => return Ok(left),
            Some(op) => op,
        };
        self.advance()?;
        let right = self.concat()?;
        if self.relop().is_some() {
            return self.syntax();
        }
        Ok(Expr::Cmp(op, Box::new(left), Box::new(right)))
    }

    fn starts_value(&self) -> bool {
        match &self.tok {
            Tok::Num(_) | Tok::Str(_) | Tok::Name(_) | Tok::Func(_) => true,
            Tok::P(p) => matches!(*p, "$" | "!" | "(" | "++" | "--"),
            _ => false,
        }
    }

    fn concat(&mut self) -> PResult<Expr> {
        let first = self.additive()?;
        if !self.starts_value() {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.starts_value() {
            parts.push(self.additive()?);
        }
        Ok(Expr::Concat(parts))
    }

    fn additive(&mut self) -> PResult<Expr> {
        let mut left = self.multiplicative()?;
        loop {
            let op = if self.is("+") {
                Arith::Add
            } else if self.is("-") {
                Arith::Sub
            } else {
                return Ok(left);
            };
            let line = self.tok_line;
            self.advance()?;
            let right = self.multiplicative()?;
            left = Expr::Arith(op, Box::new(left), Box::new(right), line);
        }
    }

    fn multiplicative(&mut self) -> PResult<Expr> {
        let mut left = self.unary()?;
        loop {
            let op = if self.is("*") {
                Arith::Mul
            } else if self.is("/") {
                Arith::Div
            } else if self.is("%") {
                Arith::Mod
            } else {
                return Ok(left);
            };
            let line = self.tok_line;
            self.advance()?;
            let right = self.unary()?;
            // gawk folds constants and reports this before running.
            if matches!((&left, &right), (Expr::Num(_), Expr::Num(z)) if *z == 0.0) && op != Arith::Mul {
                let text = if op == Arith::Div { "division by zero attempted" } else { "division by zero attempted in `%'" };
                return Err(Stop::Syntax(vec![format!("awk: cmd. line:{}: error: {}", line, text)], 1));
            }
            left = Expr::Arith(op, Box::new(left), Box::new(right), line);
        }
    }

    fn unary(&mut self) -> PResult<Expr> {
        if self.eat("!")? {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        if self.eat("-")? {
            let e = self.unary()?;
            return Ok(match e {
                Expr::Num(n) => Expr::Num(-n),
                e => Expr::Neg(Box::new(e)),
            });
        }
        if self.eat("+")? {
            return Ok(Expr::Plus(Box::new(self.unary()?)));
        }
        self.power()
    }

    fn power(&mut self) -> PResult<Expr> {
        let base = self.postfix()?;
        if !(self.is("^") || self.is("**")) {
            return Ok(base);
        }
        let line = self.tok_line;
        self.advance()?;
        let exp = self.power_operand()?;
        Ok(Expr::Arith(Arith::Pow, Box::new(base), Box::new(exp), line))
    }

    fn power_operand(&mut self) -> PResult<Expr> {
        if self.eat("-")? {
            return Ok(Expr::Neg(Box::new(self.power_operand()?)));
        }
        if self.eat("+")? {
            return Ok(Expr::Plus(Box::new(self.power_operand()?)));
        }
        if self.eat("!")? {
            return Ok(Expr::Not(Box::new(self.power_operand()?)));
        }
        self.power()
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let e = self.primary()?;
        if (self.is("++") || self.is("--")) && Self::is_lvalue(&e) {
            let step = if self.is("++") { 1.0 } else { -1.0 };
            self.advance()?;
            let lv = Self::lvalue(e).ok().unwrap();
            return Ok(Expr::Step(lv, step, true));
        }
        Ok(e)
    }

    fn name(&mut self, n: String) -> PResult<Expr> {
        self.advance()?;
        if self.is("[") {
            self.advance()?;
            let key = self.grouped()?;
            if self.is(",") {
                return refuse("an array with more than one subscript");
            }
            self.expect("]")?;
            let arr = self.intern(&n, true);
            return Ok(Expr::Elem(arr, Box::new(key)));
        }
        if n == "NF" {
            return Ok(Expr::Nf);
        }
        Ok(Expr::Var(self.intern(&n, false)))
    }

    fn primary(&mut self) -> PResult<Expr> {
        match self.tok.clone() {
            Tok::Num(n) => {
                self.advance()?;
                Ok(Expr::Num(n))
            }
            Tok::Str(s) => {
                self.advance()?;
                Ok(Expr::Str(Rc::from(s)))
            }
            Tok::Name(n) => self.name(n),
            Tok::Func(f) => self.call(f),
            Tok::P("/") | Tok::P("/=") => Ok(Expr::Regex(self.regex_here()?)),
            Tok::P("$") => {
                self.advance()?;
                let operand = if self.is("-") || self.is("+") || self.is("!") {
                    self.unary()?
                } else if self.is("++") || self.is("--") {
                    self.primary()?
                } else {
                    self.primary()?
                };
                Ok(match operand {
                    Expr::Num(n) if n >= 0.0 && n.fract() == 0.0 && n < 1e9 => Expr::FieldN(n as usize),
                    other => Expr::Field(Box::new(other)),
                })
            }
            Tok::P("++") | Tok::P("--") => {
                let step = if self.is("++") { 1.0 } else { -1.0 };
                self.advance()?;
                let target = self.primary()?;
                match Self::lvalue(target) {
                    Ok(lv) => Ok(Expr::Step(lv, step, false)),
                    Err(_) => self.syntax(),
                }
            }
            Tok::P("(") => {
                self.advance()?;
                let inner = self.grouped()?;
                if self.is(",") {
                    return refuse("(i, j) in array: an array with more than one subscript");
                }
                self.expect(")")?;
                Ok(Expr::Group(Box::new(inner)))
            }
            Tok::P("-") | Tok::P("+") | Tok::P("!") => self.unary(),
            _ => self.syntax(),
        }
    }

    fn call(&mut self, f: Func) -> PResult<Expr> {
        let line = self.tok_line;
        self.advance()?;
        if f == Func::Length && !self.is("(") {
            return Ok(Expr::Length(None));
        }
        self.expect("(")?;
        let was = std::mem::replace(&mut self.no_gt, false);
        let r = self.arguments(f, line);
        self.no_gt = was;
        r
    }

    fn arguments(&mut self, f: Func, line: u32) -> PResult<Expr> {
        let mut args: Vec<Expr> = Vec::new();
        self.newlines()?;
        // The array of `split` and of `length` is a name, not a value.
        let mut array: Option<usize> = None;
        while !self.is(")") {
            if f == Func::Length && args.is_empty() && matches!(self.tok, Tok::Name(_)) {
                // `length(a)` counts an array; which the name is shows elsewhere.
                let m = self.mark();
                if let Tok::Name(n) = self.tok.clone() {
                    self.advance()?;
                    if self.is(")") && n != "NF" {
                        self.advance()?;
                        let id = match self.prog.names.iter().position(|x| *x == n) {
                            Some(i) => i,
                            None => {
                                self.prog.names.push(n);
                                self.prog.uses.push((false, false));
                self.prog.array_first.push(false);
                                self.prog.names.len() - 1
                            }
                        };
                        return Ok(Expr::LengthOf(id));
                    }
                }
                self.reset(m);
                args.push(self.expr()?);
            } else if f == Func::Split && args.len() == 1 && array.is_none() {
                match self.tok.clone() {
                    Tok::Name(n) => {
                        self.advance()?;
                        array = Some(self.intern(&n, true));
                    }
                    _ => return self.syntax(),
                }
            } else {
                args.push(self.expr()?);
            }
            self.newlines()?;
            if self.is(",") {
                self.advance()?;
                self.newlines()?;
                if self.is(")") {
                    return self.syntax();
                }
            } else if !self.is(")") {
                return self.syntax();
            }
        }
        let close = self.tok_start;
        self.advance()?;
        let wrong = |p: &Parser, n: usize| -> PResult<Expr> {
            let name = match f {
                Func::Length => "length",
                Func::Substr => "substr",
                Func::Index => "index",
                Func::Split => "split",
                Func::Sub => "sub",
                Func::Gsub => "gsub",
                Func::Match => "match",
                Func::Tolower => "tolower",
                Func::Toupper => "toupper",
                Func::Sprintf => "sprintf",
            };
            p.marked(p.tok_line, close, None, &format!("{} is invalid as number of arguments for {}", n, name))
        };
        let n = args.len() + usize::from(array.is_some());
        let mut it = args.into_iter();
        let mut next = || Box::new(it.next().unwrap());
        Ok(match f {
            Func::Length => match n {
                0 => Expr::Length(None),
                1 => Expr::Length(Some(next())),
                _ => return wrong(self, n),
            },
            Func::Substr => match n {
                2 => Expr::Substr(next(), next(), None),
                3 => Expr::Substr(next(), next(), Some(next())),
                _ => return wrong(self, n),
            },
            Func::Index if n == 2 => Expr::Index(next(), next()),
            Func::Tolower if n == 1 => Expr::Lower(next()),
            Func::Toupper if n == 1 => Expr::Upper(next()),
            Func::Sprintf if n >= 1 => {
                drop(next);
                Expr::Sprintf(it.collect(), line)
            }
            Func::Match => match n {
                2 => {
                    let s = next();
                    Expr::MatchFn(s, Self::re_operand(*next()))
                }
                3 => return refuse("match with an array for the groups"),
                _ => return wrong(self, n),
            },
            Func::Split => match (n, array) {
                (2, Some(a)) => Expr::Split(next(), a, None),
                (3, Some(a)) => {
                    let s = next();
                    Expr::Split(s, a, Some(Self::re_operand(*next())))
                }
                (4, _) => return refuse("split with an array for the separators"),
                _ => return wrong(self, n),
            },
            Func::Sub | Func::Gsub => {
                if n != 2 && n != 3 {
                    return wrong(self, n);
                }
                let re = Self::re_operand(*next());
                let with = next();
                let target = if n == 3 {
                    match Self::lvalue(*next()) {
                        Ok(lv) => Some(lv),
                        Err(_) => return refuse("sub or gsub on a value that cannot be assigned to"),
                    }
                } else {
                    None
                };
                Expr::Subst(f == Func::Gsub, re, with, target)
            }
            _ => return wrong(self, n),
        })
    }
}
