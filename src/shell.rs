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
                Part::Subst { .. } => {}
            }
        }
        out
    }

    /// The word after quote removal, without any expansion.
    pub fn text(&self) -> Vec<u8> {
        self.bytes().into_iter().map(|(c, _)| c).collect()
    }

    /// True if the word holds a command substitution.
    pub fn has_subst(&self) -> bool {
        self.parts.iter().any(|p| matches!(p, Part::Subst { .. }))
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SimpleCommand {
    pub words: Vec<Word>,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
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
    /// substitutions and here-documents included.
    pub fn all_commands(&self) -> Vec<SimpleCommand> {
        fn parts(parts: &[Part], out: &mut Vec<SimpleCommand>) {
            for p in parts {
                if let Part::Subst { script, .. } = p {
                    walk(script, out);
                }
            }
        }
        fn walk(script: &Script, out: &mut Vec<SimpleCommand>) {
            for item in &script.items {
                for pipeline in std::iter::once(&item.first).chain(item.rest.iter().map(|(_, p)| p)) {
                    for c in &pipeline.commands {
                        out.push(c.clone());
                        for w in &c.words {
                            parts(&w.parts, out);
                        }
                        for r in &c.redirects {
                            match r {
                                Redirect::Out { target, .. } | Redirect::Both { target, .. } | Redirect::In { target } => {
                                    parts(&target.parts, out)
                                }
                                Redirect::Here { body } => parts(&body.borrow(), out),
                                Redirect::Dup { .. } => {}
                            }
                        }
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(self, &mut out);
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
    let mut p = Parser { s: input, i: 0, pending: Vec::new(), warnings: Vec::new(), open_here: false, here_error: None };
    let items = p.parse_list(None)?;
    p.read_heres();
    if let Some(e) = p.here_error {
        return Err(e);
    }
    Ok(Script { items, warnings: p.warnings, open_here: p.open_here })
}

const RESERVED: &[(&str, &str)] = &[
    ("if", "if statement"),
    ("then", "if statement"),
    ("else", "if statement"),
    ("elif", "if statement"),
    ("fi", "if statement"),
    ("for", "for loop"),
    ("while", "while loop"),
    ("until", "until loop"),
    ("do", "loop"),
    ("done", "loop"),
    ("case", "case statement"),
    ("esac", "case statement"),
    ("select", "select statement"),
    ("function", "function definition"),
    ("time", "time keyword"),
    ("coproc", "coprocess"),
    ("{", "group command { ...; }"),
    ("}", "group command { ...; }"),
    ("!", "pipeline negation (!)"),
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
}

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
                let mut sub = Parser { s: &text, i: 0, pending: Vec::new(), warnings: Vec::new(), open_here: false, here_error: None };
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
    fn parse_list(&mut self, end: Option<u8>) -> Result<Vec<AndOr>, ParseError> {
        let mut items = Vec::new();
        loop {
            self.skip_blanks_and_newlines();
            match self.peek() {
                None => break,
                Some(c) if Some(c) == end => break,
                _ => {}
            }
            items.push(self.parse_and_or()?);
            self.skip_blanks();
            match self.peek() {
                None | Some(b'\n') => {}
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
        Ok(AndOr { first, rest })
    }

    // pipeline := ['!'] command ( '|' command )*
    fn parse_pipeline(&mut self) -> Result<Pipeline, ParseError> {
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
        Ok(Pipeline { commands })
    }

    // command := simple_command | compound_command | function_def
    fn parse_command(&mut self) -> Result<SimpleCommand, ParseError> {
        self.skip_blanks();
        if self.starts_with(b"((") {
            return unsupported("arithmetic command ((...))");
        }
        if self.peek() == Some(b'(') {
            return unsupported("subshell ( ... )");
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
                return Err(self.unexpected());
            }
            if let Some(r) = self.parse_redirect()? {
                cmd.redirects.push(r);
                continue;
            }
            let word = self.parse_word()?;
            if cmd.words.is_empty() {
                check_command_position(&word)?;
            }
            cmd.words.push(word);
        }
        if cmd.words.is_empty() && cmd.redirects.is_empty() {
            return Err(self.unexpected());
        }
        Ok(cmd)
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
        let items = self.parse_list(Some(b')'))?;
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
                Some(b'$') => {
                    if self.dollar(true)? {
                        let script = self.dollar_paren()?;
                        parts.push(Part::Subst { script, quoted: true });
                    } else {
                        push_quoted(&mut parts, b'$');
                        self.i += 1;
                    }
                }
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
                b'$' => {
                    if self.dollar(false)? {
                        let script = self.dollar_paren()?;
                        parts.push(Part::Subst { script, quoted: false });
                    } else {
                        push_lit(&mut parts, b'$');
                        self.i += 1;
                    }
                }
                _ => {
                    push_lit(&mut parts, c);
                    self.i += 1;
                }
            }
        }
        let word = Word { parts, raw: self.s[start..self.i].to_vec() };
        check_word(&word)?;
        Ok(word)
    }

    /// Called at a `$`. Returns true for the start of a command
    /// substitution, false if the dollar is a literal character, and an
    /// error naming the expansion otherwise.
    fn dollar(&self, in_double_quotes: bool) -> Result<bool, ParseError> {
        let next = match self.peek_at(1) {
            None => return Ok(false),
            Some(n) => n,
        };
        match next {
            b'(' if self.peek_at(2) == Some(b'(') => unsupported("arithmetic expansion $((...))"),
            b'(' => Ok(true),
            b'{' => unsupported("parameter expansion ${...}"),
            b'[' => unsupported("arithmetic expansion $[...]"),
            b'\'' if !in_double_quotes => unsupported("ANSI-C quoting $'...'"),
            b'"' if !in_double_quotes => unsupported("locale translation $\"...\""),
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let mut j = self.i + 1;
                while j < self.s.len() && (self.s[j].is_ascii_alphanumeric() || self.s[j] == b'_') {
                    j += 1;
                }
                unsupported(format!(
                    "variable expansion (${})",
                    String::from_utf8_lossy(&self.s[self.i + 1..j])
                ))
            }
            c if c.is_ascii_digit() || matches!(c, b'?' | b'#' | b'@' | b'*' | b'$' | b'!' | b'-') => {
                unsupported(format!("special parameter (${})", c as char))
            }
            _ => Ok(false),
        }
    }
}

/// Checks on the first word of a command: reserved words and assignments.
fn check_command_position(word: &Word) -> Result<(), ParseError> {
    if let Some(lit) = word.unquoted_literal() {
        if let Some((_, what)) = RESERVED.iter().find(|(w, _)| w.as_bytes() == lit) {
            return unsupported(*what);
        }
    }
    // NAME=value, NAME+=value, NAME[i]=value
    if let Some(Part::Lit(b)) = word.parts.first() {
        let mut j = 0;
        if !b.is_empty() && (b[0].is_ascii_alphabetic() || b[0] == b'_') {
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                j += 1;
            }
            let rest = &b[j..];
            if rest.starts_with(b"=") || rest.starts_with(b"+=") || rest.starts_with(b"[") && rest.contains(&b'=') {
                return unsupported("variable assignment");
            }
        }
    }
    Ok(())
}

/// Checks on every word: tilde expansion, which bash performs on unquoted
/// text and this version does not.
fn check_word(word: &Word) -> Result<(), ParseError> {
    if let Some(Part::Lit(b)) = word.parts.first() {
        if b.starts_with(b"~") {
            return unsupported("tilde expansion (~)");
        }
    }
    Ok(())
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
    /// A command substitution: the index of its part in the word.
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
            Part::Subst { .. } => atoms.push(Atom::S(i)),
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
        parse(s.as_bytes()).unwrap().items[0].first.commands[0].clone()
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
        assert!(matches!(&p.commands[1].redirects[0], Redirect::Out { fd: 1, append: true, .. }));
        assert!(matches!(&p.commands[2].redirects[0], Redirect::Out { fd: 1, append: false, target } if target.text() == b"n"));
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
        let body = |i: usize| match &c.redirects[i] {
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
        assert!(unsupported_name("echo $HOME").contains("variable expansion"));
        assert!(unsupported_name("ls &").contains("background"));
        assert!(unsupported_name("for f in a; do ls; done").contains("for loop"));
        assert!(unsupported_name("if ls; then ls; fi").contains("if statement"));
        assert!(unsupported_name("cat <<< a").contains("here-strings"));
        assert!(unsupported_name("ls 3> e").contains("file descriptor"));
        assert!(unsupported_name("ls 2>&3").contains("duplication"));
        assert!(unsupported_name("X=1 ls").contains("assignment"));
        assert!(unsupported_name("ls ~").contains("tilde"));
        assert!(unsupported_name("(ls)").contains("subshell"));
        assert!(unsupported_name("f() { ls; }").contains("function"));
        assert!(unsupported_name("echo $((1+2))").contains("arithmetic"));
        assert!(unsupported_name("diff <(ls) a").contains("process substitution"));
        assert!(unsupported_name("ls; echo $(echo $x)").contains("variable expansion"));
        assert!(unsupported_name("cat <<E\n$x\nE").contains("variable expansion"));
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
