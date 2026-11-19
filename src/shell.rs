//! The command-line parser.
//!
//! It follows the structure of the bash grammar:
//!
//! ```text
//! list         := and_or ( (';' | '&' | newline) and_or )*
//! and_or       := pipeline ( ('&&' | '||') pipeline )*
//! pipeline     := ['!'] command ( '|' command )*
//! command      := simple_command | compound_command | function_def
//! simple       := (assignment | redirect)* (word | redirect)*
//! redirect     := [io_number] ('<' | '>' | '>>' | '<<' | ...) word
//! word         := (literal | '...' | "..." | \c | $expansion | `...`)+
//! ```
//!
//! Supported: lists with `;`, newline, `&&` and `||`; pipelines of simple
//! commands; redirections of standard input, output and error; here-
//! documents; command substitution; brace expansion; quoting. Everything
//! else is recognised where the grammar puts it and rejected by name.

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Valid bash that this version does not support. The string names it.
    Unsupported(String),
    /// Not valid bash.
    Syntax(String),
    /// Not valid bash as it stands, but more input could complete it: an
    /// open quote, a line that ends in `|`, `&&` or `||`.
    Incomplete(String),
}

/// One piece of a word. Quoting matters after parsing: quoted bytes are
/// never glob characters and are never split into fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Lit(Vec<u8>),
    Quoted(Vec<u8>),
    /// `$(...)` or backticks. `quoted` when inside double quotes.
    Subst { script: Script, quoted: bool },
    /// `$NAME`, `${NAME}`, `$?` (the name is `?`) and the forms of
    /// `ParamOp`.
    Param { name: Vec<u8>, op: ParamOp, quoted: bool },
    /// `$((...))`: the expression, with the expansions inside it.
    Arith { expr: Vec<Part>, quoted: bool },
    /// `~` at the start of a word, or of the value of an assignment: the
    /// value of `HOME`.
    Tilde,
}

/// What a parameter expansion does besides giving the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamOp {
    /// `$NAME`, `${NAME}`
    Value,
    /// `${#NAME}`
    Length,
    /// `${NAME:-word}`: the word, if the variable is unset or empty.
    Default(Word),
    /// `${NAME:=word}`: the same, and the variable is given the word.
    Assign(Word),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Word {
    pub parts: Vec<Part>,
    /// The word as it was written, for messages.
    pub raw: Vec<u8>,
}

impl Word {
    /// The word as (byte, quoted) pairs. Substitutions contribute nothing.
    pub fn bytes(&self) -> Vec<(u8, bool)> {
        let mut out = Vec::new();
        for p in &self.parts {
            match p {
                Part::Lit(b) => out.extend(b.iter().map(|c| (*c, false))),
                Part::Quoted(b) => out.extend(b.iter().map(|c| (*c, true))),
                _ => {}
            }
        }
        out
    }

    /// The word after quote removal, without any expansion.
    pub fn text(&self) -> Vec<u8> {
        self.bytes().into_iter().map(|(c, _)| c).collect()
    }

    /// True if the word holds anything whose value is only known when
    /// it runs: a command substitution, a parameter, arithmetic, `~`.
    pub fn has_subst(&self) -> bool {
        self.parts.iter().any(|p| !matches!(p, Part::Lit(_) | Part::Quoted(_)))
    }

    fn unquoted_literal(&self) -> Option<&[u8]> {
        match self.parts.as_slice() {
            [Part::Lit(b)] => Some(b),
            _ => None,
        }
    }
}

/// The text of a here-document: literal text and substitutions. It is
/// filled in when the parser reaches the lines after the command.
pub type HereBody = Rc<RefCell<Vec<Part>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirect {
    /// `>`, `>>`, `2>`, `2>>`: `fd` is 1 or 2.
    Out { fd: u8, append: bool, target: Word },
    /// `&>`, `&>>`, `>& word`: standard output and standard error.
    Both { append: bool, target: Word },
    /// `2>&1`, `>&2`: `fd` now goes where `to` goes.
    Dup { fd: u8, to: u8 },
    /// `<`
    In { target: Word },
    /// `<<`
    Here { body: HereBody },
}

/// `NAME=value` before a command, or alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assign {
    pub name: Vec<u8>,
    pub value: Word,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SimpleCommand {
    pub assigns: Vec<Assign>,
    pub words: Vec<Word>,
    pub redirects: Vec<Redirect>,
}

/// `for`, `if`, `while` or `until`, with the redirections after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compound {
    pub kind: CompoundKind,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompoundKind {
    /// `for NAME in WORDS; do BODY; done`
    For { name: Vec<u8>, words: Vec<Word>, body: Vec<AndOr> },
    /// `if A; then B; elif C; then D; else E; fi`: (condition, body) for
    /// `if` and every `elif`, then the `else` part.
    If { arms: Vec<(Vec<AndOr>, Vec<AndOr>)>, otherwise: Option<Vec<AndOr>> },
    /// `while COND; do BODY; done`, and `until`.
    While { until: bool, cond: Vec<AndOr>, body: Vec<AndOr> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Simple(SimpleCommand),
    Compound(Compound),
}

impl Command {
    pub fn redirects(&self) -> &[Redirect] {
        match self {
            Command::Simple(c) => &c.redirects,
            Command::Compound(c) => &c.redirects,
        }
    }

    /// The simple command, if it is one.
    pub fn simple(&self) -> Option<&SimpleCommand> {
        match self {
            Command::Simple(c) => Some(c),
            Command::Compound(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pipeline {
    pub commands: Vec<Command>,
    /// `!` stood before it: its status is turned round.
    pub negated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connector {
    /// `&&`
    And,
    /// `||`
    Or,
}

/// Pipelines joined by `&&` and `||`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndOr {
    pub first: Pipeline,
    pub rest: Vec<(Connector, Pipeline)>,
    /// A newline, or the end of the text, follows it: bash reads and runs
    /// a line at a time, and an error in an expansion gives up the rest
    /// of the line it is on.
    pub ends_line: bool,
}

impl AndOr {
    pub fn pipelines(&self) -> impl Iterator<Item = &Pipeline> {
        std::iter::once(&self.first).chain(self.rest.iter().map(|(_, p)| p))
    }
}

/// A parsed unit: and-or lists that run one after the other.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Script {
    pub items: Vec<AndOr>,
    /// What bash would warn about, such as a here-document that the input
    /// ended before closing.
    pub warnings: Vec<String>,
    /// True if a here-document was still open when the input ended: more
    /// lines would have belonged to it.
    pub open_here: bool,
}

impl Script {
    /// Every simple command in the script, those inside command
    /// substitutions, here-documents and the bodies of `for`, `if`,
    /// `while` and `until` included.
    pub fn all_commands(&self) -> Vec<SimpleCommand> {
        fn parts(parts: &[Part], out: &mut Vec<SimpleCommand>) {
            for p in parts {
                match p {
                    Part::Subst { script, .. } => list(&script.items, out),
                    Part::Arith { expr, .. } => parts_of(expr, out),
                    Part::Param { op: ParamOp::Default(w) | ParamOp::Assign(w), .. } => parts_of(&w.parts, out),
                    _ => {}
                }
            }
        }
        fn parts_of(p: &[Part], out: &mut Vec<SimpleCommand>) {
            parts(p, out)
        }
        fn redirects(rs: &[Redirect], out: &mut Vec<SimpleCommand>) {
            for r in rs {
                match r {
                    Redirect::Out { target, .. } | Redirect::Both { target, .. } | Redirect::In { target } => parts(&target.parts, out),
                    Redirect::Here { body } => parts(&body.borrow(), out),
                    Redirect::Dup { .. } => {}
                }
            }
        }
        fn list(items: &[AndOr], out: &mut Vec<SimpleCommand>) {
            for item in items {
                for pipeline in item.pipelines() {
                    for c in &pipeline.commands {
                        match c {
                            Command::Simple(c) => {
                                out.push(c.clone());
                                for a in &c.assigns {
                                    parts(&a.value.parts, out);
                                }
                                for w in &c.words {
                                    parts(&w.parts, out);
                                }
                            }
                            Command::Compound(c) => match &c.kind {
                                CompoundKind::For { words, body, .. } => {
                                    for w in words {
                                        parts(&w.parts, out);
                                    }
                                    list(body, out);
                                }
                                CompoundKind::If { arms, otherwise } => {
                                    for (cond, body) in arms {
                                        list(cond, out);
                                        list(body, out);
                                    }
                                    if let Some(body) = otherwise {
                                        list(body, out);
                                    }
                                }
                                CompoundKind::While { cond, body, .. } => {
                                    list(cond, out);
                                    list(body, out);
                                }
                            },
                        }
                        redirects(c.redirects(), out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        list(&self.items, &mut out);
        out
    }
}

impl Redirect {
    /// The word naming the file, if the redirection names one.
    pub fn target(&self) -> Option<&Word> {
        match self {
            Redirect::Out { target, .. } | Redirect::Both { target, .. } | Redirect::In { target } => Some(target),
            _ => None,
        }
    }
}

pub fn parse(input: &[u8]) -> Result<Script, ParseError> {
    let mut p = Parser { s: input, i: 0, pending: Vec::new(), warnings: Vec::new(), open_here: false, here_error: None, end: None };
    let items = p.parse_list(None, &[])?;
    p.read_heres();
    if let Some(e) = p.here_error {
        return Err(e);
    }
    Ok(Script { items, warnings: p.warnings, open_here: p.open_here })
}

const RESERVED: &[(&str, &str)] = &[
    ("case", "case statement"),
    ("esac", "case statement"),
    ("select", "select statement"),
    ("function", "function definition"),
    ("time", "time keyword"),
    ("coproc", "coprocess"),
    ("{", "group command { ...; }"),
    ("}", "group command { ...; }"),
    ("!", "`!' with no command after it"),
    ("[[", "[[ conditional ]]"),
];

struct PendingHere {
    /// The line of the input the here-document began on, from 1.
    line: usize,
    delimiter: Vec<u8>,
    strip_tabs: bool,
    expand: bool,
    body: HereBody,
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    /// Here-documents whose text starts after the next newline.
    pending: Vec<PendingHere>,
    warnings: Vec<String>,
    open_here: bool,
    /// What was wrong inside the text of a here-document.
    here_error: Option<ParseError>,
    /// The character that ends the list being read: `)` inside `$(...)`.
    end: Option<u8>,
}

/// The words that open, divide and close `if`, `for`, `while` and
/// `until`. They are words of the grammar only where a command begins.
const KEYWORDS: &[&str] = &["if", "then", "elif", "else", "fi", "for", "while", "until", "do", "done"];

fn unsupported<T>(what: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError::Unsupported(what.into()))
}

fn is_meta(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'|' | b'&' | b';' | b'(' | b')' | b'<' | b'>')
}

fn push_lit(parts: &mut Vec<Part>, c: u8) {
    if let Some(Part::Lit(b)) = parts.last_mut() {
        b.push(c);
    } else {
        parts.push(Part::Lit(vec![c]));
    }
}

fn push_quoted(parts: &mut Vec<Part>, c: u8) {
    if let Some(Part::Quoted(b)) = parts.last_mut() {
        b.push(c);
    } else {
        parts.push(Part::Quoted(vec![c]));
    }
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn peek_at(&self, n: usize) -> Option<u8> {
        self.s.get(self.i + n).copied()
    }

    fn starts_with(&self, t: &[u8]) -> bool {
        self.s[self.i..].starts_with(t)
    }

    /// Skip spaces, tabs, line continuations and, at the start of a word, a
    /// comment.
    fn skip_blanks(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') => self.i += 1,
                Some(b'\\') if self.peek_at(1) == Some(b'\n') => self.i += 2,
                Some(b'#') => {
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.i += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn skip_blanks_and_newlines(&mut self) {
        loop {
            self.skip_blanks();
            if self.peek() == Some(b'\n') {
                self.i += 1;
                self.read_heres();
            } else {
                break;
            }
        }
    }

    /// Called just after a newline (or at the end of the input): the lines
    /// from here are the text of the here-documents begun on the line
    /// before, in the order they were begun.
    fn read_heres(&mut self) {
        for here in std::mem::take(&mut self.pending) {
            let mut text = Vec::new();
            let mut closed = false;
            while self.i < self.s.len() {
                let end = memchr::memchr(b'\n', &self.s[self.i..]).map(|n| self.i + n);
                let mut line = &self.s[self.i..end.unwrap_or(self.s.len())];
                self.i = end.map_or(self.s.len(), |e| e + 1);
                if here.strip_tabs {
                    while line.first() == Some(&b'\t') {
                        line = &line[1..];
                    }
                }
                if line == &here.delimiter[..] {
                    closed = true;
                    break;
                }
                text.extend_from_slice(line);
                if end.is_some() || !line.is_empty() {
                    text.push(b'\n');
                }
            }
            if !closed {
                self.open_here = true;
                self.warnings.push(format!(
                    "warning: here-document at line {} delimited by end-of-file (wanted `{}')",
                    here.line,
                    String::from_utf8_lossy(&here.delimiter)
                ));
            }
            let parts = if here.expand {
                let mut sub = Parser { s: &text, i: 0, pending: Vec::new(), warnings: Vec::new(), open_here: false, here_error: None, end: None };
                match sub.quoted_text(None) {
                    Ok(parts) => parts,
                    Err(e) => {
                        self.here_error.get_or_insert(e);
                        Vec::new()
                    }
                }
            } else {
                vec![Part::Quoted(text)]
            };
            *here.body.borrow_mut() = parts;
        }
    }

    fn unexpected(&self) -> ParseError {
        let tok = match self.peek() {
            None => return ParseError::Incomplete("syntax error: unexpected end of file".into()),
            Some(b'\n') => "newline".to_string(),
            Some(c) => {
                let two = [b"&&", b"||", b";;"];
                match two.iter().find(|t| self.starts_with(&t[..])) {
                    Some(t) => String::from_utf8_lossy(&t[..]).into_owned(),
                    None => (c as char).to_string(),
                }
            }
        };
        ParseError::Syntax(format!("syntax error near unexpected token `{}'", tok))
    }

    // list := and_or ( (';' | '&' | newline) and_or )*
    //
    // The list ends at the end of the text, at the character `end`, or
    // before one of the words `stops` where a command would begin.
    fn parse_list(&mut self, end: Option<u8>, stops: &[&str]) -> Result<Vec<AndOr>, ParseError> {
        let outer = std::mem::replace(&mut self.end, end);
        let items = self.parse_list_inner(stops);
        self.end = outer;
        items
    }

    fn parse_list_inner(&mut self, stops: &[&str]) -> Result<Vec<AndOr>, ParseError> {
        let end = self.end;
        let mut items: Vec<AndOr> = Vec::new();
        loop {
            self.skip_blanks_and_newlines();
            match self.peek() {
                None => break,
                Some(c) if Some(c) == end => break,
                _ => {}
            }
            if let Some(word) = self.keyword() {
                if stops.contains(&word) {
                    break;
                }
            }
            items.push(self.parse_and_or()?);
            self.skip_blanks();
            match self.peek() {
                None | Some(b'\n') => items.last_mut().unwrap().ends_line = true,
                Some(c) if Some(c) == end => {}
                Some(b';') => {
                    if self.peek_at(1) == Some(b';') {
                        return Err(self.unexpected());
                    }
                    self.i += 1;
                    self.skip_blanks();
                    if matches!(self.peek(), Some(b';') | Some(b'&') | Some(b'|')) {
                        return Err(self.unexpected());
                    }
                }
                Some(b'&') => return unsupported("background jobs (&)"),
                Some(_) => return Err(self.unexpected()),
            }
        }
        Ok(items)
    }

    // and_or := pipeline ( ('&&' | '||') pipeline )*
    fn parse_and_or(&mut self) -> Result<AndOr, ParseError> {
        let first = self.parse_pipeline()?;
        let mut rest = Vec::new();
        loop {
            self.skip_blanks();
            let op = if self.starts_with(b"&&") {
                Connector::And
            } else if self.starts_with(b"||") {
                Connector::Or
            } else {
                break;
            };
            self.i += 2;
            self.skip_blanks_and_newlines();
            rest.push((op, self.parse_pipeline()?));
        }
        Ok(AndOr { first, rest, ends_line: false })
    }

    // pipeline := ['!'] command ( '|' command )*
    fn parse_pipeline(&mut self) -> Result<Pipeline, ParseError> {
        let mut negated = false;
        loop {
            self.skip_blanks();
            if self.peek() == Some(b'!') && matches!(self.peek_at(1), Some(b' ' | b'\t')) {
                negated = !negated;
                self.i += 1;
            } else {
                break;
            }
        }
        if negated && matches!(self.peek(), None | Some(b'\n' | b';' | b'&' | b'|' | b')')) {
            return unsupported("`!' with no command after it");
        }
        let mut commands = vec![self.parse_command()?];
        loop {
            self.skip_blanks();
            if self.peek() != Some(b'|') || self.peek_at(1) == Some(b'|') {
                break;
            }
            if self.peek_at(1) == Some(b'&') {
                return unsupported("|& (pipe standard error)");
            }
            self.i += 1;
            self.skip_blanks_and_newlines();
            commands.push(self.parse_command()?);
        }
        Ok(Pipeline { commands, negated })
    }

    /// The keyword at this place, if the token here is one: unquoted and
    /// ended by a blank, an operator or the end of the text.
    fn keyword(&self) -> Option<&'static str> {
        let rest = &self.s[self.i..];
        KEYWORDS.iter().copied().find(|k| rest.starts_with(k.as_bytes()) && rest.get(k.len()).map_or(true, |c| is_meta(*c)))
    }

    /// Take the keyword `word`, which must be next.
    fn expect(&mut self, word: &'static str) -> Result<(), ParseError> {
        self.skip_blanks_and_newlines();
        if self.keyword() == Some(word) {
            self.i += word.len();
            return Ok(());
        }
        Err(self.unexpected())
    }

    /// A list that must hold a command, up to one of `stops`.
    fn body(&mut self, stops: &[&'static str]) -> Result<Vec<AndOr>, ParseError> {
        let items = self.parse_list_inner(stops)?;
        if items.is_empty() {
            return Err(self.unexpected());
        }
        Ok(items)
    }

    /// The redirections after `done` or `fi`.
    fn trailing_redirects(&mut self) -> Result<Vec<Redirect>, ParseError> {
        let mut redirects = Vec::new();
        loop {
            self.skip_blanks();
            match self.parse_redirect()? {
                Some(r) => redirects.push(r),
                None => return Ok(redirects),
            }
        }
    }

    // compound := for | if | while | until. Called at its first word.
    fn parse_compound(&mut self, word: &'static str) -> Result<Compound, ParseError> {
        self.i += word.len();
        let kind = match word {
            "if" => {
                let mut arms = Vec::new();
                let mut otherwise = None;
                loop {
                    let cond = self.body(&["then"])?;
                    self.expect("then")?;
                    arms.push((cond, self.body(&["elif", "else", "fi"])?));
                    self.skip_blanks_and_newlines();
                    match self.keyword() {
                        Some("elif") => self.i += 4,
                        Some("else") => {
                            self.i += 4;
                            otherwise = Some(self.body(&["fi"])?);
                            self.expect("fi")?;
                            break;
                        }
                        _ => {
                            self.expect("fi")?;
                            break;
                        }
                    }
                }
                CompoundKind::If { arms, otherwise }
            }
            "for" => {
                self.skip_blanks();
                if self.starts_with(b"((") {
                    return unsupported("arithmetic for loop: for ((...))");
                }
                let start = self.i;
                while self.peek().is_some_and(|c| !is_meta(c)) {
                    self.i += 1;
                }
                let name = self.s[start..self.i].to_vec();
                if name.is_empty() {
                    return Err(self.unexpected());
                }
                if !crate::session::is_name(&name) {
                    return Err(ParseError::Syntax(format!("`{}': not a valid identifier", String::from_utf8_lossy(&name))));
                }
                self.skip_blanks_and_newlines();
                let rest = &self.s[self.i..];
                if !(rest.starts_with(b"in") && rest.get(2).map_or(true, |c| is_meta(*c))) {
                    if self.peek().is_none() {
                        return Err(self.unexpected());
                    }
                    return unsupported("for loop without `in' (it would go over the positional parameters)");
                }
                self.i += 2;
                let mut words = Vec::new();
                loop {
                    self.skip_blanks();
                    match self.peek() {
                        None | Some(b'\n') | Some(b';') => break,
                        Some(c) if is_meta(c) => return Err(self.unexpected()),
                        _ => words.push(self.parse_word()?),
                    }
                }
                if self.peek() == Some(b';') {
                    self.i += 1;
                }
                self.expect("do")?;
                let body = self.body(&["done"])?;
                self.expect("done")?;
                CompoundKind::For { name, words, body }
            }
            _ => {
                let cond = self.body(&["do"])?;
                self.expect("do")?;
                let body = self.body(&["done"])?;
                self.expect("done")?;
                CompoundKind::While { until: word == "until", cond, body }
            }
        };
        Ok(Compound { kind, redirects: self.trailing_redirects()? })
    }

    // command := simple_command | compound_command | function_def
    fn parse_command(&mut self) -> Result<Command, ParseError> {
        self.skip_blanks();
        if self.starts_with(b"((") {
            return unsupported("arithmetic command ((...))");
        }
        if self.peek() == Some(b'(') {
            return unsupported("subshell ( ... )");
        }
        match self.keyword() {
            Some(word @ ("if" | "for" | "while" | "until")) => return Ok(Command::Compound(self.parse_compound(word)?)),
            Some(word) => return Err(ParseError::Syntax(format!("syntax error near unexpected token `{}'", word))),
            None => {}
        }
        let mut cmd = SimpleCommand::default();
        loop {
            self.skip_blanks();
            let c = match self.peek() {
                None => break,
                Some(c) => c,
            };
            let both = c == b'&' && self.peek_at(1) == Some(b'>');
            if matches!(c, b'\n' | b'|' | b';' | b')') || c == b'&' && !both {
                break;
            }
            if c == b'(' {
                if cmd.words.len() == 1 && cmd.redirects.is_empty() {
                    return unsupported("function definition");
                }
                if cmd.assigns.last().is_some_and(|a| a.value.parts.is_empty()) && self.i > 0 && self.s[self.i - 1] == b'=' {
                    return unsupported("variable assignment of an array");
                }
                return Err(self.unexpected());
            }
            if let Some(r) = self.parse_redirect()? {
                cmd.redirects.push(r);
                continue;
            }
            let word = self.parse_word()?;
            if cmd.words.is_empty() {
                if let Some(assign) = assignment(&word)? {
                    cmd.assigns.push(assign);
                    continue;
                }
                check_command_position(&word)?;
            }
            cmd.words.push(word);
        }
        if cmd.words.is_empty() && cmd.redirects.is_empty() && cmd.assigns.is_empty() {
            return Err(self.unexpected());
        }
        // `export NAME=value`: the value is an assignment's, so a `~`
        // that begins it is HOME.
        if cmd.words.first().and_then(|w| w.unquoted_literal()) == Some(b"export") {
            for w in cmd.words.iter_mut().skip(1) {
                if let Some(a) = assignment(w)? {
                    let mut parts = vec![Part::Lit([&a.name[..], b"="].concat())];
                    parts.extend(a.value.parts);
                    w.parts = parts;
                }
            }
        }
        Ok(Command::Simple(cmd))
    }

    /// The word a redirection operator applies to.
    fn redirect_word(&mut self) -> Result<Word, ParseError> {
        self.skip_blanks();
        if self.starts_with(b">(") || self.starts_with(b"<(") {
            return unsupported("process substitution");
        }
        match self.peek() {
            None => return Err(self.unexpected()),
            Some(c) if is_meta(c) => return Err(self.unexpected()),
            _ => {}
        }
        self.parse_word()
    }

    // redirect := [io_number] operator word
    fn parse_redirect(&mut self) -> Result<Option<Redirect>, ParseError> {
        if self.starts_with(b"&>") {
            let append = self.starts_with(b"&>>");
            self.i += if append { 3 } else { 2 };
            let target = self.redirect_word()?;
            return Ok(Some(Redirect::Both { append, target }));
        }
        let mut j = self.i;
        while j < self.s.len() && self.s[j].is_ascii_digit() {
            j += 1;
        }
        let fd: Option<&[u8]> = if j > self.i { Some(&self.s[self.i..j]) } else { None };
        let rest = &self.s[j..];
        if !(rest.starts_with(b"<") || rest.starts_with(b">")) {
            return Ok(None);
        }
        const OPS: &[(&[u8], Option<&str>)] = &[
            (b"<<<", Some("here-strings (<<<)")),
            (b"<<-", None),
            (b"<<", None),
            (b"<(", Some("process substitution <(...)")),
            (b"<&", Some("file descriptor duplication (<&)")),
            (b"<>", Some("read-write redirection (<>)")),
            (b"<", None),
            (b">(", Some("process substitution >(...)")),
            (b">&", None),
            (b">|", Some("noclobber override (>|)")),
            (b">>", None),
            (b">", None),
        ];
        let (op, problem) = OPS.iter().find(|(t, _)| rest.starts_with(t)).unwrap();
        let op: &[u8] = op;
        let input = op[0] == b'<';
        let fd_ok = match fd {
            None => true,
            Some(b"0") => input,
            Some(b"1") | Some(b"2") => !input,
            _ => false,
        };
        if fd.is_some() && (!fd_ok || problem.is_some()) {
            let fd = String::from_utf8_lossy(fd.unwrap());
            let op = String::from_utf8_lossy(op);
            return unsupported(format!("file descriptor redirection ({}{})", fd, op));
        }
        if let Some(p) = problem {
            return unsupported(*p);
        }
        let fd: u8 = match fd {
            Some(b"2") => 2,
            Some(b"0") => 0,
            _ if input => 0,
            _ => 1,
        };
        self.i = j + op.len();
        match op {
            b"<" => Ok(Some(Redirect::In { target: self.redirect_word()? })),
            b"<<" | b"<<-" => {
                let word = self.redirect_word()?;
                if word.has_subst() {
                    return unsupported("a command substitution as a here-document delimiter");
                }
                let expand = !word.parts.iter().any(|p| matches!(p, Part::Quoted(_)));
                let body: HereBody = Rc::new(RefCell::new(Vec::new()));
                let line = 1 + self.s[..self.i].iter().filter(|c| **c == b'\n').count();
                self.pending.push(PendingHere { line, delimiter: word.text(), strip_tabs: op == b"<<-", expand, body: body.clone() });
                Ok(Some(Redirect::Here { body }))
            }
            b">&" => {
                self.skip_blanks();
                let mut k = self.i;
                while k < self.s.len() && self.s[k].is_ascii_digit() {
                    k += 1;
                }
                let ends = k == self.s.len() || is_meta(self.s[k]);
                if k > self.i && ends {
                    let to = &self.s[self.i..k];
                    let to = match to {
                        b"1" => 1,
                        b"2" => 2,
                        _ => {
                            return unsupported(format!("file descriptor duplication (>&{})", String::from_utf8_lossy(to)))
                        }
                    };
                    self.i = k;
                    return Ok(Some(Redirect::Dup { fd, to }));
                }
                if self.peek() == Some(b'-') && (self.i + 1 == self.s.len() || is_meta(self.s[self.i + 1])) {
                    return unsupported("closing a file descriptor (>&-)");
                }
                if fd != 1 {
                    return unsupported("file descriptor duplication (2>&word)");
                }
                // `>& word` is `&> word`.
                Ok(Some(Redirect::Both { append: false, target: self.redirect_word()? }))
            }
            _ => Ok(Some(Redirect::Out { fd, append: op == b">>", target: self.redirect_word()? })),
        }
    }

    /// Called at `$(`. Parses to the matching `)`.
    fn dollar_paren(&mut self) -> Result<Script, ParseError> {
        self.i += 2;
        let outer = std::mem::take(&mut self.pending);
        let items = self.parse_list(Some(b')'), &[])?;
        if self.peek() != Some(b')') {
            return Err(ParseError::Incomplete("unexpected end of input while looking for matching `)'".into()));
        }
        // A here-document begun inside ends inside.
        self.read_heres();
        self.pending = outer;
        self.i += 1;
        Ok(Script { items, warnings: Vec::new(), open_here: false })
    }

    /// Called at a backtick. Parses to the closing one.
    fn backticks(&mut self, in_double_quotes: bool) -> Result<Script, ParseError> {
        self.i += 1;
        let mut text = Vec::new();
        loop {
            match self.peek() {
                None => {
                    return Err(ParseError::Incomplete("unexpected end of input while looking for matching ``'".into()))
                }
                Some(b'`') => {
                    self.i += 1;
                    break;
                }
                Some(b'\\') => match self.peek_at(1) {
                    Some(n) if matches!(n, b'$' | b'`' | b'\\') || n == b'"' && in_double_quotes => {
                        text.push(n);
                        self.i += 2;
                    }
                    _ => {
                        text.push(b'\\');
                        self.i += 1;
                    }
                },
                Some(c) => {
                    text.push(c);
                    self.i += 1;
                }
            }
        }
        let mut script = parse(&text)?;
        self.warnings.append(&mut script.warnings);
        Ok(script)
    }

    /// Text in which only `$`, backticks and backslash are special: the
    /// inside of double quotes (up to the closing quote) or the text of a
    /// here-document (all of it).
    fn quoted_text(&mut self, close: Option<u8>) -> Result<Vec<Part>, ParseError> {
        let dq = close.is_some();
        let mut parts = vec![Part::Quoted(Vec::new())];
        loop {
            match self.peek() {
                None => {
                    if dq {
                        return Err(ParseError::Incomplete("unexpected end of input while looking for matching `\"'".into()));
                    }
                    break;
                }
                Some(c) if Some(c) == close => {
                    self.i += 1;
                    break;
                }
                Some(b'\\') => match self.peek_at(1) {
                    Some(b'\n') => self.i += 2,
                    Some(n) if matches!(n, b'$' | b'`' | b'\\') || n == b'"' && dq => {
                        push_quoted(&mut parts, n);
                        self.i += 2;
                    }
                    _ => {
                        push_quoted(&mut parts, b'\\');
                        self.i += 1;
                    }
                },
                Some(b'`') => {
                    let script = self.backticks(dq)?;
                    parts.push(Part::Subst { script, quoted: true });
                }
                Some(b'$') => match self.dollar(true)? {
                    Some(part) => parts.push(part),
                    None => {
                        push_quoted(&mut parts, b'$');
                        self.i += 1;
                    }
                },
                Some(c) => {
                    push_quoted(&mut parts, c);
                    self.i += 1;
                }
            }
        }
        Ok(parts)
    }

    fn parse_word(&mut self) -> Result<Word, ParseError> {
        let start = self.i;
        let mut parts: Vec<Part> = Vec::new();
        while let Some(c) = self.peek() {
            if is_meta(c) {
                break;
            }
            match c {
                b'\\' => match self.peek_at(1) {
                    Some(b'\n') => self.i += 2,
                    Some(n) => {
                        parts.push(Part::Quoted(vec![n]));
                        self.i += 2;
                    }
                    None => {
                        push_lit(&mut parts, b'\\');
                        self.i += 1;
                    }
                },
                b'\'' => {
                    self.i += 1;
                    let start = self.i;
                    while self.peek().is_some_and(|c| c != b'\'') {
                        self.i += 1;
                    }
                    if self.peek().is_none() {
                        return Err(ParseError::Incomplete(
                            "unexpected end of input while looking for matching `''".into(),
                        ));
                    }
                    parts.push(Part::Quoted(self.s[start..self.i].to_vec()));
                    self.i += 1;
                }
                b'"' => {
                    self.i += 1;
                    parts.extend(self.quoted_text(Some(b'"'))?);
                }
                b'`' => {
                    let script = self.backticks(false)?;
                    parts.push(Part::Subst { script, quoted: false });
                }
                b'$' => match self.dollar(false)? {
                    Some(part) => parts.push(part),
                    None => {
                        push_lit(&mut parts, b'$');
                        self.i += 1;
                    }
                },
                b'~' if self.i == start => {
                    self.i += 1;
                    match self.peek() {
                        Some(c) if c != b'/' && !is_meta(c) => return unsupported("tilde expansion other than ~ and ~/path"),
                        _ => parts.push(Part::Tilde),
                    }
                }
                _ => {
                    push_lit(&mut parts, c);
                    self.i += 1;
                }
            }
        }
        Ok(Word { parts, raw: self.s[start..self.i].to_vec() })
    }

    /// Called at a `$`. Returns the expansion that starts here, having
    /// read all of it; `None` if the dollar is a literal character; and
    /// an error naming the expansion if it is one this version lacks.
    fn dollar(&mut self, dq: bool) -> Result<Option<Part>, ParseError> {
        let next = match self.peek_at(1) {
            None => return Ok(None),
            Some(n) => n,
        };
        match next {
            b'(' if self.peek_at(2) == Some(b'(') => {
                self.i += 3;
                let expr = self.arith_body()?;
                Ok(Some(Part::Arith { expr, quoted: dq }))
            }
            b'(' => {
                let script = self.dollar_paren()?;
                Ok(Some(Part::Subst { script, quoted: dq }))
            }
            b'{' => self.braced(dq).map(Some),
            b'[' => unsupported("arithmetic expansion $[...]"),
            b'\'' if !dq => unsupported("ANSI-C quoting $'...'"),
            b'"' if !dq => unsupported("locale translation $\"...\""),
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let mut j = self.i + 1;
                while j < self.s.len() && (self.s[j].is_ascii_alphanumeric() || self.s[j] == b'_') {
                    j += 1;
                }
                let name = self.s[self.i + 1..j].to_vec();
                self.i = j;
                Ok(Some(Part::Param { name, op: ParamOp::Value, quoted: dq }))
            }
            b'?' => {
                self.i += 2;
                Ok(Some(Part::Param { name: b"?".to_vec(), op: ParamOp::Value, quoted: dq }))
            }
            c if c.is_ascii_digit() || matches!(c, b'#' | b'@' | b'*' | b'$' | b'!' | b'-') => {
                unsupported(format!("special parameter (${})", c as char))
            }
            _ => Ok(None),
        }
    }

    /// Called at `${`.
    fn braced(&mut self, dq: bool) -> Result<Part, ParseError> {
        let refuse = |p: &Parser| -> ParseError {
            let end = p.s[p.i..].iter().position(|c| *c == b'}').map_or(p.s.len(), |n| p.i + n + 1);
            ParseError::Unsupported(format!("parameter expansion {}", String::from_utf8_lossy(&p.s[p.i..end])))
        };
        let mut j = self.i + 2;
        let length = self.s.get(j) == Some(&b'#') && self.s.get(j + 1).is_some_and(|c| *c != b'}');
        if length {
            j += 1;
        }
        let start = j;
        if self.s.get(j) == Some(&b'?') {
            j += 1;
        } else {
            while j < self.s.len() && (self.s[j].is_ascii_alphanumeric() || self.s[j] == b'_') {
                j += 1;
            }
        }
        let name = self.s[start..j].to_vec();
        if name.is_empty() || name[0].is_ascii_digit() {
            return Err(refuse(self));
        }
        if j >= self.s.len() {
            return Err(ParseError::Incomplete("unexpected end of input while looking for matching `}'".into()));
        }
        if self.s[j] == b'}' {
            self.i = j + 1;
            return Ok(Part::Param { name, op: if length { ParamOp::Length } else { ParamOp::Value }, quoted: dq });
        }
        let assign = self.s[j..].starts_with(b":=");
        if length || name == b"?" || !(assign || self.s[j..].starts_with(b":-")) {
            return Err(refuse(self));
        }
        let from = j + 2;
        self.i = from;
        let parts = self.braced_word(dq)?;
        let word = Word { parts, raw: self.s[from..self.i - 1].to_vec() };
        Ok(Part::Param { name, op: if assign { ParamOp::Assign(word) } else { ParamOp::Default(word) }, quoted: dq })
    }

    /// The word of `${NAME:-word}`, up to and with the closing brace.
    /// Outside double quotes its plain text is `Lit`, to be split into
    /// fields like the value of a variable.
    fn braced_word(&mut self, dq: bool) -> Result<Vec<Part>, ParseError> {
        let mut parts: Vec<Part> = Vec::new();
        let plain = |parts: &mut Vec<Part>, c: u8| if dq { push_quoted(parts, c) } else { push_lit(parts, c) };
        // Outside quotes a `~` that begins the word is HOME.
        if !dq && self.peek() == Some(b'~') && matches!(self.peek_at(1), Some(b'}') | Some(b'/')) {
            parts.push(Part::Tilde);
            self.i += 1;
        }
        loop {
            let c = match self.peek() {
                None => return Err(ParseError::Incomplete("unexpected end of input while looking for matching `}'".into())),
                Some(c) => c,
            };
            match c {
                b'}' => {
                    self.i += 1;
                    return Ok(parts);
                }
                b'\\' => match self.peek_at(1) {
                    Some(b'\n') => self.i += 2,
                    Some(n) if !dq || matches!(n, b'$' | b'`' | b'\\' | b'"' | b'}') => {
                        push_quoted(&mut parts, n);
                        self.i += 2;
                    }
                    _ => {
                        push_quoted(&mut parts, b'\\');
                        self.i += 1;
                    }
                },
                b'\'' if !dq => {
                    self.i += 1;
                    let start = self.i;
                    while self.peek().is_some_and(|c| c != b'\'') {
                        self.i += 1;
                    }
                    if self.peek().is_none() {
                        return Err(ParseError::Incomplete("unexpected end of input while looking for matching `''".into()));
                    }
                    parts.push(Part::Quoted(self.s[start..self.i].to_vec()));
                    self.i += 1;
                }
                b'"' => {
                    self.i += 1;
                    parts.extend(self.quoted_text(Some(b'"'))?);
                }
                b'`' => {
                    let script = self.backticks(dq)?;
                    parts.push(Part::Subst { script, quoted: dq });
                }
                b'$' => match self.dollar(dq)? {
                    Some(part) => parts.push(part),
                    None => {
                        plain(&mut parts, b'$');
                        self.i += 1;
                    }
                },
                _ => {
                    plain(&mut parts, c);
                    self.i += 1;
                }
            }
        }
    }

    /// The inside of `$((...))`, up to and with the closing `))`: the
    /// text of the expression, as quoted text, and the expansions in it.
    fn arith_body(&mut self) -> Result<Vec<Part>, ParseError> {
        let mut parts = vec![Part::Quoted(Vec::new())];
        let mut depth = 0usize;
        loop {
            let c = match self.peek() {
                None => return Err(ParseError::Incomplete("unexpected end of input while looking for matching `)'".into())),
                Some(c) => c,
            };
            match c {
                b')' if depth == 0 => {
                    if self.peek_at(1) != Some(b')') {
                        return unsupported("a command substitution that begins with a subshell: $((...) ...)");
                    }
                    self.i += 2;
                    return Ok(parts);
                }
                b'(' | b')' => {
                    depth = if c == b'(' { depth + 1 } else { depth - 1 };
                    push_quoted(&mut parts, c);
                    self.i += 1;
                }
                b'$' => match self.dollar(true)? {
                    Some(part) => parts.push(part),
                    None => {
                        push_quoted(&mut parts, b'$');
                        self.i += 1;
                    }
                },
                b'`' => {
                    let script = self.backticks(true)?;
                    parts.push(Part::Subst { script, quoted: true });
                }
                // Quotes around an operand are dropped.
                b'"' => self.i += 1,
                b'\\' if self.peek_at(1) == Some(b'\n') => self.i += 2,
                _ => {
                    push_quoted(&mut parts, c);
                    self.i += 1;
                }
            }
        }
    }
}

/// Checks on the first word of a command: the reserved words that are
/// not supported.
fn check_command_position(word: &Word) -> Result<(), ParseError> {
    if let Some(lit) = word.unquoted_literal() {
        if let Some((_, what)) = RESERVED.iter().find(|(w, _)| w.as_bytes() == lit) {
            return unsupported(*what);
        }
    }
    Ok(())
}

/// `NAME=value` where a command could begin: the assignment, or `None`
/// for any other word. `NAME+=value` and `NAME[i]=value` are refused.
fn assignment(word: &Word) -> Result<Option<Assign>, ParseError> {
    let Some(Part::Lit(b)) = word.parts.first() else { return Ok(None) };
    let mut j = 0;
    if b.is_empty() || !(b[0].is_ascii_alphabetic() || b[0] == b'_') {
        return Ok(None);
    }
    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
        j += 1;
    }
    let rest = &b[j..];
    if rest.starts_with(b"+=") {
        return unsupported("variable assignment with +=");
    }
    if rest.starts_with(b"[") && rest.contains(&b'=') {
        return unsupported("variable assignment to an array element");
    }
    if !rest.starts_with(b"=") {
        return Ok(None);
    }
    // A `~` that begins the value, or follows a colon in it, is HOME.
    let mut parts: Vec<Part> = Vec::new();
    let lit = |parts: &mut Vec<Part>, text: &[u8], mut at_start: bool, last: bool| -> Result<(), ParseError> {
        let mut i = 0;
        while i < text.len() {
            let c = text[i];
            if c == b'~' && at_start {
                match text.get(i + 1) {
                    None if !last => {}
                    None | Some(b'/') | Some(b':') => {
                        parts.push(Part::Tilde);
                        i += 1;
                        at_start = false;
                        continue;
                    }
                    Some(_) => return unsupported("tilde expansion other than ~ and ~/path"),
                }
            }
            push_lit(parts, c);
            at_start = c == b':';
            i += 1;
        }
        Ok(())
    };
    let count = word.parts.len();
    for (n, p) in word.parts.iter().enumerate() {
        match p {
            Part::Lit(text) => {
                let text = if n == 0 { &text[j + 1..] } else { &text[..] };
                let at_start = n == 0 || matches!(parts.last(), Some(Part::Lit(b)) if b.ends_with(b":"));
                lit(&mut parts, text, at_start, n + 1 == count)?;
            }
            other => parts.push(other.clone()),
        }
    }
    Ok(Some(Assign { name: b[..j].to_vec(), value: Word { parts, raw: word.raw[(j + 1).min(word.raw.len())..].to_vec() } }))
}

// ---------------------------------------------------------- brace expansion

/// One position of a word, for brace expansion: only unquoted bytes can be
/// braces, commas or dots.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Atom {
    /// An unquoted byte.
    C(u8),
    /// A quoted byte.
    Q(u8),
    /// An empty quoted string: it keeps an empty word in existence.
    Empty,
    /// A substitution or another expansion: the index of its part in
    /// the word.
    S(usize),
}

/// Brace expansion: `a{b,c}d` is `abd acd`, `{1..3}` is `1 2 3`. A word
/// with nothing to expand comes back alone.
pub fn brace_expand(word: &Word) -> Vec<Word> {
    let has_brace = word.parts.iter().any(|p| matches!(p, Part::Lit(b) if b.contains(&b'{')));
    if !has_brace {
        return vec![word.clone()];
    }
    let mut atoms = Vec::new();
    for (i, p) in word.parts.iter().enumerate() {
        match p {
            Part::Lit(b) => atoms.extend(b.iter().map(|c| Atom::C(*c))),
            Part::Quoted(b) if b.is_empty() => atoms.push(Atom::Empty),
            Part::Quoted(b) => atoms.extend(b.iter().map(|c| Atom::Q(*c))),
            _ => atoms.push(Atom::S(i)),
        }
    }
    expand_atoms(&atoms)
        .into_iter()
        .map(|atoms| {
            let mut parts = Vec::new();
            for a in atoms {
                match a {
                    Atom::C(c) => push_lit(&mut parts, c),
                    Atom::Q(c) => push_quoted(&mut parts, c),
                    Atom::Empty => parts.push(Part::Quoted(Vec::new())),
                    Atom::S(i) => parts.push(word.parts[i].clone()),
                }
            }
            Word { parts, raw: word.raw.clone() }
        })
        .collect()
}

fn expand_atoms(atoms: &[Atom]) -> Vec<Vec<Atom>> {
    let mut start = 0;
    while let Some(open) = atoms[start..].iter().position(|a| *a == Atom::C(b'{')).map(|n| n + start) {
        // `${` is not a brace expansion.
        if open > 0 && atoms[open - 1] == Atom::C(b'$') {
            start = open + 1;
            continue;
        }
        // The matching close, and the commas at the top level between.
        let mut depth = 0usize;
        let mut commas = Vec::new();
        let mut close = None;
        for (j, a) in atoms.iter().enumerate().skip(open) {
            match a {
                Atom::C(b'{') => depth += 1,
                Atom::C(b'}') => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(j);
                        break;
                    }
                }
                Atom::C(b',') if depth == 1 => commas.push(j),
                _ => {}
            }
        }
        let close = match close {
            Some(c) => c,
            None => {
                // No matching brace: this one is literal; one inside may
                // still pair up.
                start = open + 1;
                continue;
            }
        };
        let inner = &atoms[open + 1..close];
        let alternatives: Vec<Vec<Atom>> = if !commas.is_empty() {
            let mut out = Vec::new();
            let mut from = open + 1;
            for c in commas.iter().copied().chain([close]) {
                out.extend(expand_atoms(&atoms[from..c]));
                from = c + 1;
            }
            out
        } else if let Some(seq) = sequence(inner) {
            seq
        } else {
            start = open + 1;
            continue;
        };
        let tails = expand_atoms(&atoms[close + 1..]);
        let mut out = Vec::with_capacity(alternatives.len() * tails.len());
        for alt in &alternatives {
            for tail in &tails {
                let mut w = atoms[..open].to_vec();
                w.extend_from_slice(alt);
                w.extend_from_slice(tail);
                out.push(w);
            }
        }
        return out;
    }
    vec![atoms.to_vec()]
}

/// `1..5`, `a..e`, `01..10`, `1..9..2`: the words of a sequence, or `None`
/// if the text between the braces is not one.
fn sequence(inner: &[Atom]) -> Option<Vec<Vec<Atom>>> {
    let text: Vec<u8> = inner
        .iter()
        .map(|a| match a {
            Atom::C(c) => Some(*c),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let text = std::str::from_utf8(&text).ok()?;
    let fields: Vec<&str> = text.split("..").collect();
    if fields.len() < 2 || fields.len() > 3 {
        return None;
    }
    let int = |s: &str| -> Option<i64> {
        let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
        if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        s.parse().ok()
    };
    let step = match fields.get(2) {
        None => 1,
        Some(s) => int(s)?.unsigned_abs().max(1),
    };
    let mut out = Vec::new();
    if let (Some(a), Some(b)) = (int(fields[0]), int(fields[1])) {
        let padded = |s: &str| {
            let d = s.strip_prefix(['-', '+']).unwrap_or(s);
            d.len() > 1 && d.starts_with('0')
        };
        let width = if padded(fields[0]) || padded(fields[1]) { fields[0].len().max(fields[1].len()) } else { 0 };
        let count = (a.abs_diff(b) / step) as usize + 1;
        if count > 1_000_000 {
            return None;
        }
        let mut n = a;
        for _ in 0..count {
            let s = if n < 0 { format!("-{:0w$}", -n, w = width.saturating_sub(1)) } else { format!("{:0w$}", n, w = width) };
            out.push(s.bytes().map(Atom::C).collect());
            n = if a <= b { n + step as i64 } else { n - step as i64 };
        }
        return Some(out);
    }
    let letter = |s: &str| (s.len() == 1 && s.as_bytes()[0].is_ascii_alphabetic()).then(|| s.as_bytes()[0]);
    let (a, b) = (letter(fields[0])?, letter(fields[1])?);
    let count = (a.abs_diff(b) as u64 / step) as usize + 1;
    let mut c = a as i64;
    for _ in 0..count {
        // Between `Z` and `a` lie characters the shell would otherwise act
        // on; they come out as plain characters.
        let ch = c as u8;
        out.push(vec![if ch.is_ascii_alphanumeric() { Atom::C(ch) } else { Atom::Q(ch) }]);
        c = if a <= b { c + step as i64 } else { c - step as i64 };
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(s: &str) -> SimpleCommand {
        parse(s.as_bytes()).unwrap().items[0].first.commands[0].simple().unwrap().clone()
    }

    fn words(s: &str) -> Vec<Vec<u8>> {
        first(s).words.iter().flat_map(brace_expand).map(|w| w.text()).collect()
    }

    fn unsupported_name(s: &str) -> String {
        match parse(s.as_bytes()) {
            Err(ParseError::Unsupported(n)) => n,
            other => panic!("expected unsupported for {:?}, got {:?}", s, other),
        }
    }

    #[test]
    fn quoting() {
        assert_eq!(words("echo 'a b' \"c d\" e\\ f"), vec![&b"echo"[..], b"a b", b"c d", b"e f"]);
        assert_eq!(words("echo \"a\\\"b\\$c\\\\d\\e\""), vec![&b"echo"[..], b"a\"b$c\\d\\e"]);
        assert_eq!(words("echo '' \"\""), vec![&b"echo"[..], b"", b""]);
        assert_eq!(words("echo a$ '$x' # comment"), vec![&b"echo"[..], b"a$", b"$x"]);
    }

    #[test]
    fn pipes_and_redirects() {
        let s = parse(b"cat a | grep x >> out |\n wc -l > n").unwrap();
        let p = &s.items[0].first;
        assert_eq!(p.commands.len(), 3);
        assert!(matches!(&p.commands[1].redirects()[0], Redirect::Out { fd: 1, append: true, .. }));
        assert!(matches!(&p.commands[2].redirects()[0], Redirect::Out { fd: 1, append: false, target } if target.text() == b"n"));
        let c = first("ls 2>e >&2 2>&1 &>b &>>c <in 2>>f");
        assert_eq!(c.redirects.len(), 7);
        assert!(matches!(&c.redirects[0], Redirect::Out { fd: 2, append: false, .. }));
        assert!(matches!(&c.redirects[1], Redirect::Dup { fd: 1, to: 2 }));
        assert!(matches!(&c.redirects[2], Redirect::Dup { fd: 2, to: 1 }));
        assert!(matches!(&c.redirects[3], Redirect::Both { append: false, .. }));
        assert!(matches!(&c.redirects[4], Redirect::Both { append: true, .. }));
        assert!(matches!(&c.redirects[5], Redirect::In { .. }));
        assert!(matches!(&c.redirects[6], Redirect::Out { fd: 2, append: true, .. }));
    }

    #[test]
    fn lists() {
        let s = parse(b"a; b && c || d\ne | f;").unwrap();
        assert_eq!(s.items.len(), 3);
        assert_eq!(s.items[1].rest.len(), 2);
        assert_eq!(s.items[1].rest[0].0, Connector::And);
        assert_eq!(s.items[1].rest[1].0, Connector::Or);
        assert_eq!(s.items[2].first.commands.len(), 2);
        assert!(parse(b"  # just a comment").unwrap().items.is_empty());
    }

    #[test]
    fn here_documents() {
        let s = parse(b"cat <<A <<-'B'; echo x\none $(echo two)\nA\n\t$(three)\n\tB\necho y\n").unwrap();
        assert_eq!(s.items.len(), 3);
        let c = &s.items[0].first.commands[0];
        let body = |i: usize| match &c.redirects()[i] {
            Redirect::Here { body } => body.borrow().clone(),
            _ => panic!(),
        };
        assert!(matches!(&body(0)[..], [Part::Quoted(a), Part::Subst { quoted: true, .. }, Part::Quoted(b)] if a == b"one " && b == b"\n"));
        assert_eq!(body(1), vec![Part::Quoted(b"$(three)\n".to_vec())]);
        assert!(parse(b"cat <<A\nx\n").unwrap().open_here);
        assert!(!parse(b"cat <<A\nx\nA\n").unwrap().open_here);
    }

    #[test]
    fn substitution() {
        let c = first("echo a$(ls | wc -l)b \"`echo \\`x\\``\"");
        assert!(matches!(&c.words[1].parts[..], [Part::Lit(_), Part::Subst { quoted: false, .. }, Part::Lit(_)]));
        assert!(c.words[2].parts.iter().any(|p| matches!(p, Part::Subst { quoted: true, script } if script.items.len() == 1)));
        assert!(matches!(parse(b"echo $(ls"), Err(ParseError::Incomplete(_))));
    }

    #[test]
    fn braces() {
        let w = |s: &str| words(&format!("echo {}", s))[1..].iter().map(|b| String::from_utf8_lossy(b).into_owned()).collect::<Vec<_>>().join(" ");
        assert_eq!(w("{a,b}"), "a b");
        assert_eq!(w("x{a,{b,c}}y"), "xay xby xcy");
        assert_eq!(w("{1..3}{a,b}"), "1a 1b 2a 2b 3a 3b");
        assert_eq!(w("{1..7..3} {5..1..2} {a..e..2}"), "1 4 7 5 3 1 a c e");
        assert_eq!(w("{01..10..3} {-2..2}"), "01 04 07 10 -2 -1 0 1 2");
        assert_eq!(w("{} a{b}c {a,b {a..} '{a,b}' {a\\,b}"), "{} a{b}c {a,b {a..} {a,b} {a,b}");
        assert_eq!(w("{a}{b,c} {a{b,c}}"), "{a}b {a}c {ab} {ac}");
        assert_eq!(w("{,x}y"), "y xy");
    }

    #[test]
    fn rejected_by_name() {
        for (unit, name) in [
            ("echo ${X%a}", "parameter expansion"),
            ("echo $1", "special parameter"),
            ("ls &", "background"),
            ("for ((i=0;i<3;i++)); do ls; done", "arithmetic for loop"),
            ("for f; do ls; done", "for loop without"),
            ("case x in x) ls;; esac", "case statement"),
            ("cat <<< a", "here-strings"),
            ("ls 3> e", "file descriptor"),
            ("ls 2>&3", "duplication"),
            ("X+=1", "assignment"),
            ("A=(1 2)", "array"),
            ("ls ~root", "tilde"),
            ("(ls)", "subshell"),
            ("f() { ls; }", "function"),
            ("((1+2))", "arithmetic command"),
            ("diff <(ls) a", "process substitution"),
            ("ls; echo $(echo ${x/a/b})", "parameter expansion"),
        ] {
            assert!(unsupported_name(unit).contains(name), "{}", unit);
        }
    }

    #[test]
    fn syntax_errors() {
        for s in ["| ls", "ls )", "ls ;; ls", "; ls", "ls; ; ls", "ls > ; ls"] {
            assert!(matches!(parse(s.as_bytes()), Err(ParseError::Syntax(_))), "{}", s);
        }
        for s in ["ls |", "echo 'a", "echo \"a", "ls >", "ls &&", "ls ||\n"] {
            assert!(matches!(parse(s.as_bytes()), Err(ParseError::Incomplete(_))), "{}", s);
        }
    }
}
