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
//! Only part of that language is supported in the first version: one pipeline
//! of simple commands, `>` and `>>`, and quoting. Everything else is
//! recognised where the grammar puts it and rejected by name, so that it can
//! be added later without rewriting the parser.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Valid bash that this version does not support. The string names it.
    Unsupported(String),
    /// Not valid bash.
    Syntax(String),
}

/// One piece of a word. Quoting matters after parsing: quoted bytes are
/// never glob characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Lit(Vec<u8>),
    Quoted(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Word {
    pub parts: Vec<Part>,
}

impl Word {
    /// The word as (byte, quoted) pairs.
    pub fn bytes(&self) -> Vec<(u8, bool)> {
        let mut out = Vec::new();
        for p in &self.parts {
            match p {
                Part::Lit(b) => out.extend(b.iter().map(|c| (*c, false))),
                Part::Quoted(b) => out.extend(b.iter().map(|c| (*c, true))),
            }
        }
        out
    }

    /// The word after quote removal, without any expansion.
    pub fn text(&self) -> Vec<u8> {
        self.bytes().into_iter().map(|(c, _)| c).collect()
    }

    fn unquoted_literal(&self) -> Option<&[u8]> {
        match self.parts.as_slice() {
            [Part::Lit(b)] => Some(b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirOp {
    /// `>`
    Write,
    /// `>>`
    Append,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    pub op: RedirOp,
    pub target: Word,
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

/// A parsed command line. `None` means the line was empty or only a comment.
pub type Script = Option<Pipeline>;

pub fn parse(input: &[u8]) -> Result<Script, ParseError> {
    let mut p = Parser { s: input, i: 0 };
    p.parse_list()
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

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError::Unsupported(what.into()))
}

fn is_meta(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'|' | b'&' | b';' | b'(' | b')' | b'<' | b'>')
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
            } else {
                break;
            }
        }
    }

    fn unexpected(&self) -> ParseError {
        let tok = match self.peek() {
            None | Some(b'\n') => "newline".to_string(),
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
    fn parse_list(&mut self) -> Result<Script, ParseError> {
        self.skip_blanks_and_newlines();
        if self.peek().is_none() {
            return Ok(None);
        }
        let first = self.parse_and_or()?;
        self.skip_blanks();
        match self.peek() {
            None => {}
            Some(b'\n') => {
                self.skip_blanks_and_newlines();
                if self.peek().is_some() {
                    return unsupported("more than one command (newline-separated list)");
                }
            }
            Some(b';') => {
                if self.peek_at(1) == Some(b';') {
                    return Err(self.unexpected());
                }
                return unsupported("command lists (;)");
            }
            Some(b'&') => return unsupported("background jobs (&)"),
            Some(_) => return Err(self.unexpected()),
        }
        Ok(Some(first))
    }

    // and_or := pipeline ( ('&&' | '||') pipeline )*
    fn parse_and_or(&mut self) -> Result<Pipeline, ParseError> {
        let first = self.parse_pipeline()?;
        self.skip_blanks();
        if self.starts_with(b"&&") {
            return unsupported("&& lists");
        }
        if self.starts_with(b"||") {
            return unsupported("|| lists");
        }
        Ok(first)
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
            if c == b'&' && self.peek_at(1) == Some(b'>') {
                return unsupported("&> (redirect standard output and standard error)");
            }
            if matches!(c, b'\n' | b'|' | b';' | b'&') {
                break;
            }
            if c == b')' {
                return Err(self.unexpected());
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

    // redirect := [io_number] operator word
    fn parse_redirect(&mut self) -> Result<Option<Redirect>, ParseError> {
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
            (b"<<", Some("here-documents (<<)")),
            (b"<(", Some("process substitution <(...)")),
            (b"<&", Some("file descriptor duplication (<&)")),
            (b"<>", Some("read-write redirection (<>)")),
            (b"<", Some("input redirection (<)")),
            (b">(", Some("process substitution >(...)")),
            (b">&", Some("file descriptor duplication (>&)")),
            (b">|", Some("noclobber override (>|)")),
            (b">>", None),
            (b">", None),
        ];
        let (op, problem) = OPS.iter().find(|(t, _)| rest.starts_with(t)).unwrap();
        if let Some(fd) = fd {
            if fd != b"1" || problem.is_some() {
                let fd = String::from_utf8_lossy(fd);
                let op = String::from_utf8_lossy(op);
                return match fd.as_ref() {
                    "2" => unsupported(format!("standard error redirection ({}{})", fd, op)),
                    _ => unsupported(format!("file descriptor redirection ({}{})", fd, op)),
                };
            }
        }
        if let Some(p) = problem {
            return unsupported(*p);
        }
        self.i = j + op.len();
        self.skip_blanks();
        if self.starts_with(b">(") || self.starts_with(b"<(") {
            return unsupported("process substitution");
        }
        match self.peek() {
            None => return Err(self.unexpected()),
            Some(c) if is_meta(c) => return Err(self.unexpected()),
            _ => {}
        }
        let target = self.parse_word()?;
        let op = if *op == b">>" { RedirOp::Append } else { RedirOp::Write };
        Ok(Some(Redirect { op, target }))
    }

    fn parse_word(&mut self) -> Result<Word, ParseError> {
        let mut parts: Vec<Part> = Vec::new();
        fn lit(parts: &mut Vec<Part>, c: u8) {
            if let Some(Part::Lit(b)) = parts.last_mut() {
                b.push(c);
            } else {
                parts.push(Part::Lit(vec![c]));
            }
        }
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
                        lit(&mut parts, b'\\');
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
                        return Err(ParseError::Syntax(
                            "unexpected end of input while looking for matching `''".into(),
                        ));
                    }
                    parts.push(Part::Quoted(self.s[start..self.i].to_vec()));
                    self.i += 1;
                }
                b'"' => {
                    self.i += 1;
                    let mut buf = Vec::new();
                    loop {
                        match self.peek() {
                            None => {
                                return Err(ParseError::Syntax(
                                    "unexpected end of input while looking for matching `\"'".into(),
                                ))
                            }
                            Some(b'"') => {
                                self.i += 1;
                                break;
                            }
                            Some(b'\\') => match self.peek_at(1) {
                                Some(b'\n') => self.i += 2,
                                Some(n) if matches!(n, b'$' | b'`' | b'"' | b'\\') => {
                                    buf.push(n);
                                    self.i += 2;
                                }
                                _ => {
                                    buf.push(b'\\');
                                    self.i += 1;
                                }
                            },
                            Some(b'`') => return unsupported("command substitution (`...`)"),
                            Some(b'$') => {
                                self.dollar(true)?;
                                buf.push(b'$');
                                self.i += 1;
                            }
                            Some(c) => {
                                buf.push(c);
                                self.i += 1;
                            }
                        }
                    }
                    parts.push(Part::Quoted(buf));
                }
                b'`' => return unsupported("command substitution (`...`)"),
                b'$' => {
                    self.dollar(false)?;
                    lit(&mut parts, b'$');
                    self.i += 1;
                }
                _ => {
                    lit(&mut parts, c);
                    self.i += 1;
                }
            }
        }
        let word = Word { parts };
        check_word(&word)?;
        Ok(word)
    }

    /// Called at a `$`. Returns Ok if the dollar is a literal character and
    /// an error naming the expansion otherwise.
    fn dollar(&self, in_double_quotes: bool) -> Result<(), ParseError> {
        let next = match self.peek_at(1) {
            None => return Ok(()),
            Some(n) => n,
        };
        match next {
            b'(' if self.peek_at(2) == Some(b'(') => unsupported("arithmetic expansion $((...))"),
            b'(' => unsupported("command substitution $(...)"),
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
            _ => Ok(()),
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

/// Checks on every word: tilde and brace expansion, which bash performs on
/// unquoted text and this version does not.
fn check_word(word: &Word) -> Result<(), ParseError> {
    if let Some(Part::Lit(b)) = word.parts.first() {
        if b.starts_with(b"~") {
            return unsupported("tilde expansion (~)");
        }
    }
    let bytes = word.bytes();
    let n = bytes.len();
    for start in 0..n {
        if bytes[start] != (b'{', false) {
            continue;
        }
        let mut depth = 0usize;
        let mut has_sep = false;
        for j in start..n {
            let (c, quoted) = bytes[j];
            if quoted {
                continue;
            }
            match c {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if has_sep {
                            return unsupported("brace expansion ({a,b} or {1..3})");
                        }
                        break;
                    }
                }
                b',' if depth == 1 => has_sep = true,
                b'.' if depth == 1 && j + 1 < n && bytes[j + 1] == (b'.', false) => has_sep = true,
                _ => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<Vec<u8>> {
        let p = parse(s.as_bytes()).unwrap().unwrap();
        p.commands[0].words.iter().map(|w| w.text()).collect()
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
        let p = parse(b"cat a | grep x >> out |\n wc -l > n").unwrap().unwrap();
        assert_eq!(p.commands.len(), 3);
        assert_eq!(p.commands[1].redirects[0].op, RedirOp::Append);
        assert_eq!(p.commands[2].redirects[0].target.text(), b"n");
    }

    #[test]
    fn rejected_by_name() {
        assert!(unsupported_name("echo $HOME").contains("variable expansion"));
        assert!(unsupported_name("echo \"$(ls)\"").contains("command substitution"));
        assert!(unsupported_name("echo `ls`").contains("command substitution"));
        assert!(unsupported_name("ls && ls").contains("&&"));
        assert!(unsupported_name("ls || ls").contains("||"));
        assert!(unsupported_name("ls; ls").contains(";"));
        assert!(unsupported_name("ls &").contains("background"));
        assert!(unsupported_name("for f in a; do ls; done").contains("for loop"));
        assert!(unsupported_name("if ls; then ls; fi").contains("if statement"));
        assert!(unsupported_name("cat < a").contains("input redirection"));
        assert!(unsupported_name("cat <<EOF").contains("here-documents"));
        assert!(unsupported_name("ls 2> e").contains("standard error"));
        assert!(unsupported_name("ls 2>&1").contains("standard error"));
        assert!(unsupported_name("X=1 ls").contains("assignment"));
        assert!(unsupported_name("echo {a,b}").contains("brace"));
        assert!(unsupported_name("ls ~").contains("tilde"));
        assert!(unsupported_name("(ls)").contains("subshell"));
        assert!(unsupported_name("f() { ls; }").contains("function"));
        assert!(unsupported_name("echo $((1+2))").contains("arithmetic"));
        assert!(unsupported_name("diff <(ls) a").contains("process substitution"));
    }

    #[test]
    fn syntax_errors() {
        for s in ["ls |", "| ls", "echo 'a", "echo \"a", "ls >", "ls )"] {
            assert!(matches!(parse(s.as_bytes()), Err(ParseError::Syntax(_))), "{}", s);
        }
        assert_eq!(parse(b"  # just a comment").unwrap(), None);
        assert_eq!(words("echo {} a{b}c"), vec![&b"echo"[..], b"{}", b"a{b}c"]);
    }
}
