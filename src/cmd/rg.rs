//! rg (ripgrep 14.1.0): a recursive search that skips hidden files and
//! what the tree's ignore files name, with rg's own pattern syntax.

use crate::engine::{Input, Io, Prepared, Runner};
use crate::regex::hir::{rg_pattern, RgPattern, RgSyntax};
use crate::regex::MatchError;
use crate::scan::Searcher;
use crate::tree::{Candidates, Db, Errno, NodeId, Shortlist};
use crate::util::lossy;
use memchr::{memchr, memrchr};
use std::collections::HashMap;
use std::rc::Rc;

// ------------------------------------------------------------------- globs

/// A glob as the globset crate reads one, with `*` and `?` stopping at a
/// slash.
#[derive(Debug, Clone)]
enum Tok {
    Lit(u8),
    Any,
    Star,
    /// `**/` at the start.
    Prefix,
    /// `/**` at the end.
    Suffix,
    /// `/**/`
    Middle,
    Class(Box<[bool; 256]>),
    Alt(Vec<Vec<Tok>>),
}

struct GlobParser<'a> {
    g: &'a [u8],
    i: usize,
    prev: Option<u8>,
    /// The branches being built: one frame for the glob, one more for
    /// each open `{`.
    stack: Vec<Vec<Vec<Tok>>>,
}

impl<'a> GlobParser<'a> {
    fn cur(&mut self) -> &mut Vec<Tok> {
        self.stack.last_mut().unwrap().last_mut().unwrap()
    }

    fn star(&mut self) {
        let prev = self.prev;
        if self.g.get(self.i) != Some(&b'*') {
            self.cur().push(Tok::Star);
            return;
        }
        self.i += 1;
        let two = |p: &mut Self| {
            p.cur().push(Tok::Star);
            p.cur().push(Tok::Star);
        };
        if self.cur().is_empty() {
            if self.g.get(self.i).is_some_and(|c| *c != b'/') {
                two(self);
            } else {
                self.cur().push(Tok::Prefix);
                if self.i < self.g.len() {
                    self.i += 1;
                }
            }
            return;
        }
        if prev != Some(b'/') && (self.stack.len() <= 1 || (prev != Some(b',') && prev != Some(b'{'))) {
            two(self);
            return;
        }
        let is_suffix = match self.g.get(self.i) {
            None => true,
            Some(b',' | b'}') if self.stack.len() >= 2 => true,
            Some(b'/') => {
                self.i += 1;
                false
            }
            _ => {
                two(self);
                return;
            }
        };
        match self.cur().pop() {
            Some(Tok::Prefix) => self.cur().push(Tok::Prefix),
            Some(Tok::Suffix) => self.cur().push(Tok::Suffix),
            _ => self.cur().push(if is_suffix { Tok::Suffix } else { Tok::Middle }),
        }
    }

    fn class(&mut self) -> Result<(), String> {
        let g = self.g;
        let unclosed = || "unclosed character class; missing ']'".to_string();
        let mut negated = false;
        if matches!(g.get(self.i), Some(b'!' | b'^')) {
            negated = true;
            self.i += 1;
        }
        let mut set = Box::new([false; 256]);
        let mut first = true;
        let mut pending: Option<u8> = None;
        let mut in_range = false;
        loop {
            let c = *g.get(self.i).ok_or_else(unclosed)?;
            self.i += 1;
            match c {
                b']' if !first => break,
                b'-' if !first && pending.is_some() && !in_range && g.get(self.i).is_some_and(|n| *n != b']') => in_range = true,
                _ => {
                    if in_range {
                        let lo = pending.take().unwrap();
                        if c < lo {
                            return Err(format!("invalid range; '{}' > '{}'", char::from(lo), char::from(c)));
                        }
                        (lo..=c).for_each(|b| set[b as usize] = true);
                        in_range = false;
                    } else {
                        if let Some(p) = pending {
                            set[p as usize] = true;
                        }
                        pending = Some(c);
                    }
                }
            }
            first = false;
        }
        if let Some(p) = pending {
            set[p as usize] = true;
        }
        if in_range {
            set[b'-' as usize] = true;
        }
        if negated {
            set.iter_mut().for_each(|b| *b = !*b);
        }
        self.cur().push(Tok::Class(set));
        Ok(())
    }

    fn parse(mut self) -> Result<Vec<Tok>, String> {
        while self.i < self.g.len() {
            let c = self.g[self.i];
            self.i += 1;
            match c {
                b'?' => self.cur().push(Tok::Any),
                b'*' => self.star(),
                b'[' => self.class()?,
                b'{' => self.stack.push(vec![Vec::new()]),
                b'}' => {
                    // A `}` that closes nothing is passed over, as
                    // ripgrep 14.1.0 does.
                    if self.stack.len() >= 2 {
                        let alts = self.stack.pop().unwrap();
                        self.cur().push(Tok::Alt(alts));
                    }
                }
                b',' if self.stack.len() > 1 => self.stack.last_mut().unwrap().push(Vec::new()),
                b'\\' => match self.g.get(self.i) {
                    None => return Err("dangling '\\'".into()),
                    Some(n) => {
                        let n = *n;
                        self.i += 1;
                        self.cur().push(Tok::Lit(n));
                    }
                },
                _ => self.cur().push(Tok::Lit(c)),
            }
            self.prev = Some(c);
        }
        if self.stack.len() > 1 {
            return Err("unclosed alternate group; missing '}' (maybe escape '{' with '[{]'?)".into());
        }
        Ok(self.stack.pop().unwrap().pop().unwrap())
    }
}

/// Every way through a glob's `{a,b}` groups, as globs without any.
fn expand(toks: &[Tok]) -> Vec<Vec<Tok>> {
    let mut out: Vec<Vec<Tok>> = vec![Vec::new()];
    for t in toks {
        match t {
            Tok::Alt(branches) => {
                let each: Vec<Vec<Tok>> = branches.iter().flat_map(|b| expand(b)).collect();
                let mut next = Vec::with_capacity(out.len() * each.len());
                for o in &out {
                    for e in &each {
                        next.push([o.as_slice(), e.as_slice()].concat());
                    }
                }
                out = next;
            }
            other => out.iter_mut().for_each(|o| o.push(other.clone())),
        }
    }
    out
}

fn glob_match(t: &[Tok], s: &[u8]) -> bool {
    let Some(first) = t.first() else {
        return s.is_empty();
    };
    let rest = &t[1..];
    match first {
        Tok::Lit(c) => s.first() == Some(c) && glob_match(rest, &s[1..]),
        Tok::Any => s.first().is_some_and(|c| *c != b'/') && glob_match(rest, &s[1..]),
        Tok::Class(set) => s.first().is_some_and(|c| set[*c as usize]) && glob_match(rest, &s[1..]),
        Tok::Star => {
            if rest.is_empty() {
                return !s.contains(&b'/');
            }
            let mut k = 0;
            loop {
                if glob_match(rest, &s[k..]) {
                    return true;
                }
                if k >= s.len() || s[k] == b'/' {
                    return false;
                }
                k += 1;
            }
        }
        // `**` alone is every path.
        Tok::Prefix if rest.is_empty() => true,
        Tok::Prefix => glob_match(rest, s) || (0..s.len()).any(|i| s[i] == b'/' && glob_match(rest, &s[i + 1..])),
        Tok::Suffix => s.first() == Some(&b'/') && (rest.is_empty() || (1..=s.len()).any(|k| glob_match(rest, &s[k..]))),
        Tok::Middle => s.first() == Some(&b'/') && (glob_match(rest, &s[1..]) || (1..s.len()).any(|j| s[j] == b'/' && glob_match(rest, &s[j + 1..]))),
        Tok::Alt(_) => false,
    }
}

/// A compiled glob, with the two commonest shapes told apart.
#[derive(Debug, Clone)]
enum Glob {
    /// `**/name`: the last component is `name`.
    Base(Vec<u8>),
    /// `**/*tail`: the last component ends with `tail`.
    Tail(Vec<u8>),
    Any(Vec<Vec<Tok>>),
}

impl Glob {
    fn new(text: &[u8]) -> Result<Glob, String> {
        let toks = GlobParser { g: text, i: 0, prev: None, stack: vec![vec![Vec::new()]] }.parse()?;
        let lits = |t: &[Tok]| -> Option<Vec<u8>> {
            t.iter().map(|x| if let Tok::Lit(c) = x { (*c != b'/').then_some(*c) } else { None }).collect()
        };
        Ok(match toks.as_slice() {
            [Tok::Prefix, rest @ ..] if !rest.is_empty() && lits(rest).is_some() => Glob::Base(lits(rest).unwrap()),
            [Tok::Prefix, Tok::Star, rest @ ..] if lits(rest).is_some() => Glob::Tail(lits(rest).unwrap()),
            _ => Glob::Any(expand(&toks)),
        })
    }

    fn matches(&self, path: &[u8]) -> bool {
        let base = || &path[memrchr(b'/', path).map_or(0, |i| i + 1)..];
        match self {
            Glob::Base(name) => base() == &name[..],
            Glob::Tail(tail) => base().ends_with(tail),
            Glob::Any(ways) => ways.iter().any(|w| glob_match(w, path)),
        }
    }
}

// ------------------------------------------------------------ ignore files

struct Rule {
    glob: Glob,
    whitelist: bool,
    only_dir: bool,
}

/// The rules of one ignore file, or of the -g options.
#[derive(Default)]
struct Rules {
    rules: Vec<Rule>,
    /// Rules that are not whitelists.
    ignores: usize,
}

impl Rules {
    /// One line of a gitignore file. A line that is not a valid glob is
    /// an error for -g and passed over in a file.
    fn add_line(&mut self, line: &[u8]) -> Result<(), String> {
        let mut line = line;
        if line.starts_with(b"#") {
            return Ok(());
        }
        if !line.ends_with(b"\\ ") {
            while line.last().is_some_and(|c| c.is_ascii_whitespace()) {
                line = &line[..line.len() - 1];
            }
        }
        if line.is_empty() {
            return Ok(());
        }
        let mut whitelist = false;
        let mut absolute = false;
        if line.starts_with(b"\\!") || line.starts_with(b"\\#") {
            line = &line[1..];
            absolute = line.first() == Some(&b'/');
        } else {
            if line.starts_with(b"!") {
                whitelist = true;
                line = &line[1..];
            }
            if line.starts_with(b"/") {
                line = &line[1..];
                absolute = true;
            }
        }
        let mut only_dir = false;
        if line.last() == Some(&b'/') {
            only_dir = true;
            line = &line[..line.len() - 1];
            if line.last() == Some(&b'\\') {
                line = &line[..line.len() - 1];
            }
        }
        let mut actual = line.to_vec();
        if !absolute && !line.contains(&b'/') && !(actual.starts_with(b"**/") || actual == b"**") {
            actual = [b"**/", &actual[..]].concat();
        }
        if actual.ends_with(b"/**") {
            actual.extend_from_slice(b"/*");
        }
        let glob = Glob::new(&actual)?;
        self.ignores += usize::from(!whitelist);
        self.rules.push(Rule { glob, whitelist, only_dir });
        Ok(())
    }

    fn parse(text: &[u8]) -> Rules {
        let mut r = Rules::default();
        for line in text.split(|c| *c == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            let _ = r.add_line(line);
        }
        r
    }

    /// The last rule that matches says: Some(true) to ignore, Some(false)
    /// to keep whatever an earlier rule said.
    fn matched(&self, path: &[u8], is_dir: bool) -> Option<bool> {
        self.rules.iter().rev().find(|r| (!r.only_dir || is_dir) && r.glob.matches(path)).map(|r| !r.whitelist)
    }
}

/// The ignore files of one folder.
#[derive(Default)]
struct DirRules {
    rgignore: Option<Rules>,
    ignore: Option<Rules>,
    gitignore: Option<Rules>,
    exclude: Option<Rules>,
    has_git: bool,
}

const TYPES: &[(&str, &[&str])] = &[
    ("c", &["*.[chH]", "*.[chH].in", "*.cats"]),
    ("cpp", &["*.[ChH]", "*.[ChH].in", "*.[ch]pp", "*.[ch]pp.in", "*.[ch]xx", "*.[ch]xx.in", "*.cc", "*.cc.in", "*.hh", "*.hh.in", "*.inl"]),
    ("css", &["*.css", "*.scss"]),
    ("go", &["*.go"]),
    ("html", &["*.ejs", "*.htm", "*.html"]),
    ("js", &["*.cjs", "*.js", "*.jsx", "*.mjs", "*.vue"]),
    ("json", &["*.json", "*.sarif", "composer.lock"]),
    ("md", &["*.markdown", "*.md", "*.mdown", "*.mdwn", "*.mdx", "*.mkd", "*.mkdn"]),
    ("py", &["*.py", "*.pyi"]),
    ("rust", &["*.rs"]),
    (
        "sh",
        &[
            "*.bash", "*.bashrc", "*.csh", "*.cshrc", "*.ksh", "*.kshrc", "*.sh", "*.tcsh", "*.zsh", ".bash_login", ".bash_logout", ".bash_profile", ".bashrc", ".cshrc", ".kshrc", ".login", ".logout",
            ".profile", ".tcshrc", ".zlogin", ".zlogout", ".zprofile", ".zshenv", ".zshrc", "bash_login", "bash_logout", "bash_profile", "bashrc", "profile", "zlogin", "zlogout", "zprofile", "zshenv",
            "zshrc",
        ],
    ),
    ("toml", &["*.toml", "Cargo.lock"]),
    ("ts", &["*.cts", "*.mts", "*.ts", "*.tsx"]),
    ("txt", &["*.txt"]),
    ("yaml", &["*.yaml", "*.yml"]),
];

/// The names ripgrep has for types this version does not know: asked
/// for, they are refused and not reported as unrecognised.
const OTHER_TYPES: &[&str] = &["all", "markdown", "python", "typescript", "java", "ruby", "php", "lua", "swift", "kotlin", "make", "cmake", "xml", "sql", "tex", "docker", "config", "asm", "awk", "perl", "vim", "zig", "haskell", "elixir", "erlang", "scala", "dart", "csharp", "objc", "protobuf", "license", "readme", "log", "csv", "diff", "h", "cs", "r", "m4", "nix", "ocaml", "clojure", "lisp", "julia", "elm", "fish", "zsh", "jinja", "sass", "less", "svg", "pdf", "gradle", "groovy", "tf", "thrift", "d", "ada", "fortran", "pascal", "cobol", "verilog", "vhdl", "wiki", "org", "rst", "man", "yacc", "lex", "rdoc", "pod", "msbuild", "bazel", "jl", "ps", "qml", "qmake", "sv", "tcl", "vala", "webidl", "xz", "gzip", "bzip2", "lz4", "zstd", "brotli", "lzma", "jupyter", "kconfig", "dts", "devicetree", "mk", "meson", "ninja", "po", "puppet", "racket", "raku", "red", "robot", "rpc", "soy", "spark", "spec", "stylus", "systemd", "taskpaper", "toml", "twig", "typoscript", "usd", "v", "vb", "vcl", "vimscript", "wgsl", "yang", "z", "motoko", "move", "seed7", "ets", "hare", "hbs", "hs", "ungrammar", "gn", "gap", "fidl", "fennel", "elisp", "edn", "dvc", "dita", "dhall", "cython", "cabal", "crystal", "creole", "coffeescript", "candid", "bitbake", "avro", "asciidoc", "aidl", "alire", "agda", "ack", "fsharp", "fut", "flatbuffers", "graphql", "gprbuild", "html", "idris", "janet", "jsonl", "lean", "lilypond", "llvm", "lock", "minified", "mako", "matlab", "nim", "pants", "prolog", "purs", "cml", "postscript", "smarty", "solidity", "sml", "slim", "starlark", "svelte", "thrift", "vue", "xml"];

struct TypeSel {
    negate: bool,
    globs: Vec<Glob>,
}

// ------------------------------------------------------------------ options

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Lines,
    Count,
    List,
    Files,
}

struct Opts {
    mode: Mode,
    invert: bool,
    numbers: bool,
    null: bool,
    before: usize,
    after: usize,
    max: Option<u64>,
    hidden: bool,
    no_ignore: bool,
    overrides: Rules,
    types: Vec<TypeSel>,
    /// A -t was given: a file of no selected type is left out.
    selected: bool,
}

/// What stops rg before it searches: its own message, with status 2.
type Fatal = String;

struct Parsed {
    opts: Opts,
    syn: RgSyntax,
    pattern: Option<Vec<u8>>,
    paths: Vec<Vec<u8>>,
    fatal: Option<Fatal>,
}

fn arguments(args: &[Vec<u8>]) -> Result<Parsed, String> {
    let mut o = Opts { mode: Mode::Lines, invert: false, numbers: false, null: false, before: 0, after: 0, max: None, hidden: false, no_ignore: false, overrides: Rules::default(), types: Vec::new(), selected: false };
    let mut syn = RgSyntax::default();
    let mut positional: Vec<Vec<u8>> = Vec::new();
    let mut fatal: Option<Fatal> = None;
    let mut unrestricted = 0;
    let (mut around, mut before, mut after) = (None, None, None);
    let longs: &[(&str, u8)] = &[
        ("ignore-case", b'i'),
        ("smart-case", b'S'),
        ("case-sensitive", b's'),
        ("fixed-strings", b'F'),
        ("word-regexp", b'w'),
        ("line-regexp", b'x'),
        ("invert-match", b'v'),
        ("count", b'c'),
        ("files-with-matches", b'l'),
        ("line-number", b'n'),
        ("no-line-number", b'N'),
        ("null", b'0'),
        ("after-context", b'A'),
        ("before-context", b'B'),
        ("context", b'C'),
        ("max-count", b'm'),
        ("glob", b'g'),
        ("type", b't'),
        ("type-not", b'T'),
        ("unrestricted", b'u'),
        ("threads", b'j'),
        ("no-heading", 1),
        ("files", 2),
        ("hidden", 3),
        ("no-ignore", 4),
        ("sort", 5),
    ];
    let takes_value = |f: u8| b"ABCmgtTj".contains(&f) || f == 5;
    let mut i = 0;
    let mut flags: Vec<(u8, String, Option<Vec<u8>>)> = Vec::new();
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == b"--" {
            positional.extend(args[i..].iter().cloned());
            break;
        }
        if a.starts_with(b"--") {
            let (name, value) = match a.iter().position(|c| *c == b'=') {
                Some(eq) => (&a[2..eq], Some(a[eq + 1..].to_vec())),
                None => (&a[2..], None),
            };
            let Some((_, f)) = longs.iter().find(|(n, _)| n.as_bytes() == name) else {
                return Err(format!("flag: rg --{}", lossy(name)));
            };
            let shown = format!("--{}", lossy(name));
            if takes_value(*f) {
                let value = match value {
                    Some(v) => Some(v),
                    None if i < args.len() => {
                        i += 1;
                        Some(args[i - 1].clone())
                    }
                    None => None,
                };
                if value.is_none() {
                    fatal.get_or_insert(format!("rg: missing value for flag {}: missing argument for option '{}'", shown, shown));
                }
                flags.push((*f, shown, value));
            } else {
                flags.push((*f, shown, None));
            }
            continue;
        }
        if a.len() < 2 || a[0] != b'-' {
            positional.push(a.clone());
            continue;
        }
        let mut j = 1;
        while j < a.len() {
            let f = a[j];
            j += 1;
            let shown = format!("-{}", char::from(f));
            if takes_value(f) && f != 5 {
                let value = if j < a.len() {
                    Some(a[j..].to_vec())
                } else if i < args.len() {
                    i += 1;
                    Some(args[i - 1].clone())
                } else {
                    None
                };
                if value.is_none() {
                    fatal.get_or_insert(format!("rg: missing value for flag {}: missing argument for option '{}'", shown, shown));
                }
                flags.push((f, shown, value));
                break;
            }
            if !b"iSsFwxvclnN0u".contains(&f) {
                return Err(format!("flag: rg -{}", char::from(f)));
            }
            flags.push((f, shown, None));
        }
    }
    for (f, shown, value) in flags {
        let number = |fatal: &mut Option<Fatal>| -> u64 {
            let v = value.clone().unwrap_or_default();
            let t = lossy(&v);
            match t.parse::<u64>() {
                Ok(n) => n,
                Err(_) => {
                    let why = if t.is_empty() { "cannot parse integer from empty string" } else if t.bytes().all(|c| c.is_ascii_digit()) { "number too large to fit in target type" } else { "invalid digit found in string" };
                    if value.is_some() {
                        fatal.get_or_insert(format!("rg: error parsing flag {}: value is not a valid number: {}", shown, why));
                    }
                    0
                }
            }
        };
        match f {
            b'i' => (syn.icase, syn.smart) = (true, false),
            b'S' => (syn.icase, syn.smart) = (false, true),
            b's' => (syn.icase, syn.smart) = (false, false),
            b'F' => syn.fixed = true,
            b'w' => syn.word = true,
            b'x' => syn.line = true,
            b'v' => o.invert = true,
            b'c' => o.mode = Mode::Count,
            b'l' => o.mode = Mode::List,
            b'n' => o.numbers = true,
            b'N' => o.numbers = false,
            b'0' => o.null = true,
            b'A' => after = Some(number(&mut fatal) as usize),
            b'B' => before = Some(number(&mut fatal) as usize),
            b'C' => around = Some(number(&mut fatal) as usize),
            b'm' => o.max = Some(number(&mut fatal)),
            b'j' => {
                number(&mut fatal);
            }
            b'g' => {
                if let Some(g) = &value {
                    if let Err(e) = o.overrides.add_line(g) {
                        fatal.get_or_insert(format!("rg: error parsing glob '{}': {}", lossy(g), e));
                    }
                }
            }
            b't' | b'T' => {
                if let Some(name) = &value {
                    match TYPES.iter().find(|(n, _)| n.as_bytes() == &name[..]) {
                        Some((_, globs)) => {
                            o.types.push(TypeSel { negate: f == b'T', globs: globs.iter().map(|g| Glob::new(g.as_bytes()).expect("type glob")).collect() });
                            o.selected |= f == b't';
                        }
                        None if OTHER_TYPES.contains(&&*lossy(name)) => return Err(format!("flag: rg {} {} (a file type this version does not have)", shown, lossy(name))),
                        None => {
                            fatal.get_or_insert(format!("rg: unrecognized file type: {}", lossy(name)));
                        }
                    }
                }
            }
            b'u' => {
                unrestricted += 1;
                if unrestricted > 3 {
                    fatal.get_or_insert("rg: error parsing flag -u: flag can only be repeated up to 3 times".to_string());
                }
            }
            1 => {}
            2 => o.mode = Mode::Files,
            3 => o.hidden = true,
            4 => o.no_ignore = true,
            5 => match value.as_deref() {
                Some(b"path") | Some(b"none") | None => {}
                Some(v @ (b"modified" | b"accessed" | b"created")) => return Err(format!("flag: rg --sort {}", lossy(v))),
                Some(v) => {
                    fatal.get_or_insert(format!("rg: error parsing flag --sort: choice '{}' is unrecognized", lossy(v)));
                }
            },
            _ => {}
        }
    }
    o.no_ignore |= unrestricted >= 1;
    o.hidden |= unrestricted >= 2;
    o.before = before.or(around).unwrap_or(0);
    o.after = after.or(around).unwrap_or(0);
    let pattern = if o.mode == Mode::Files || positional.is_empty() { None } else { Some(positional.remove(0)) };
    Ok(Parsed { opts: o, syn, pattern, paths: positional, fatal })
}

// ---------------------------------------------------------------- the search

/// Finds the lines a pattern matches in a stretch of whole lines.
struct Matcher {
    searcher: Searcher,
    pattern: RgPattern,
}

/// The first byte of `hay` at or after `from` that is not ASCII.
fn first_non_ascii(hay: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 8 <= hay.len() {
        let word = u64::from_ne_bytes(hay[i..i + 8].try_into().unwrap());
        if word & 0x8080_8080_8080_8080 != 0 {
            break;
        }
        i += 8;
    }
    (i..hay.len()).find(|k| hay[*k] >= 0x80)
}

impl Matcher {
    fn next_line(&mut self, hay: &[u8], from: usize) -> Result<Option<(usize, usize)>, MatchError> {
        let Some(exact) = &self.pattern.exact else {
            return self.searcher.next_line(hay, from);
        };
        // The fast matcher is right for lines of ASCII; a line with other
        // bytes is given to the one that knows what a character is.
        let mut from = from;
        while from < hay.len() {
            let Some(at) = first_non_ascii(hay, from) else {
                return self.searcher.next_line(hay, from);
            };
            let start = memrchr(b'\n', &hay[..at]).map_or(0, |i| i + 1);
            let end = memchr(b'\n', &hay[at..]).map_or(hay.len(), |i| at + i);
            if start > from {
                if let Some(found) = self.searcher.next_line(&hay[..start], from)? {
                    return Ok(Some(found));
                }
            }
            if exact.is_match(&hay[start..end])? {
                return Ok(Some((start, end)));
            }
            from = end + 1;
        }
        Ok(None)
    }
}

/// How a file is named in front of its lines.
struct Label<'a>(Option<&'a [u8]>);

struct Search<'a> {
    o: &'a Opts,
    m: Option<Matcher>,
    /// A line has been printed: with context, the next group of lines is
    /// set off by `--`.
    printed: bool,
    matched: bool,
    failed: bool,
    /// Files looked at, or that would have been but for the index.
    searched: u64,
    rules: HashMap<NodeId, Rc<DirRules>>,
    skipped: HashMap<NodeId, bool>,
    /// The folder being searched.
    root: NodeId,
    /// For each folder above one that was searched, the searched folder
    /// it was first met from: ripgrep reads the ignore files of such a
    /// folder as if every entry sat directly in that searched folder.
    met_from: HashMap<NodeId, NodeId>,
    base: NodeId,
}

/// The lines of a file that have been dealt with so far.
#[derive(Default)]
struct Progress {
    count: u64,
    /// Lines before `counted_to`, for line numbers.
    numbered: u64,
    counted_to: usize,
}

impl<'a> Search<'a> {
    fn line(&self, io: &mut Io, label: &Label, number: u64, text: &[u8], sep: u8) {
        if let Some(l) = label.0 {
            io.put(l);
            io.put(if self.o.null { b"\0" } else { std::slice::from_ref(&sep) });
        }
        if self.o.numbers {
            io.put(number.to_string().as_bytes());
            io.put(std::slice::from_ref(&sep));
        }
        io.put(text);
        io.put(b"\n");
    }

    /// Search a stretch of whole lines with no context asked for.
    /// Returns false when the file needs no more looking at.
    fn plain(&mut self, io: &mut Io, label: &Label, hay: &[u8], p: &mut Progress) -> Result<bool, MatchError> {
        let o = self.o;
        let full = |p: &Progress| o.max.is_some_and(|m| p.count >= m);
        if full(p) {
            return Ok(false);
        }
        p.counted_to = 0;
        let mut pos = 0;
        let select = |this: &mut Self, io: &mut Io, p: &mut Progress, s: usize, e: usize| -> bool {
            p.count += 1;
            match o.mode {
                Mode::List => return false,
                Mode::Count | Mode::Files => {}
                Mode::Lines => {
                    if o.numbers {
                        p.numbered += memchr::memchr_iter(b'\n', &hay[p.counted_to..s]).count() as u64;
                        p.counted_to = s;
                    }
                    this.line(io, label, p.numbered + 1, &hay[s..e], b':');
                    this.printed = true;
                }
            }
            !full(p)
        };
        while pos < hay.len() {
            let found = {
                let m = self.m.as_mut().unwrap();
                m.next_line(hay, pos)?
            };
            if !o.invert {
                match found {
                    None => break,
                    Some((s, e)) => {
                        if !select(self, io, p, s, e) {
                            return Ok(false);
                        }
                        pos = e + 1;
                    }
                }
            } else {
                let stop = found.map_or(hay.len(), |(s, _)| s);
                while pos < stop {
                    let e = memchr(b'\n', &hay[pos..stop]).map_or(stop, |i| pos + i);
                    if !select(self, io, p, pos, e) {
                        return Ok(false);
                    }
                    pos = e + 1;
                }
                match found {
                    Some((_, e)) => pos = e + 1,
                    None => break,
                }
            }
        }
        if o.numbers {
            p.numbered += memchr::memchr_iter(b'\n', &hay[p.counted_to.min(hay.len())..]).count() as u64;
        }
        Ok(!io.stopped())
    }

    /// Search a whole file with lines of context around what is found.
    fn with_context(&mut self, io: &mut Io, label: &Label, hay: &[u8], p: &mut Progress) -> Result<(), MatchError> {
        let o = self.o;
        // Where the next line not yet printed starts, and how many lines
        // after the last selected one are still owed.
        let mut shown_to = 0usize;
        let mut owed = 0usize;
        let line_no = |p: &mut Progress, at: usize| -> u64 {
            p.numbered += memchr::memchr_iter(b'\n', &hay[p.counted_to..at]).count() as u64;
            p.counted_to = at;
            p.numbered + 1
        };
        let line_end = |at: usize| memchr(b'\n', &hay[at..]).map_or(hay.len(), |i| at + i);
        let mut pos = 0;
        let mut next_match: Option<(usize, usize)> = None;
        let mut looked_to = 0;
        loop {
            // The next selected line at or after pos.
            let selected = if p.count >= o.max.unwrap_or(u64::MAX) || pos >= hay.len() {
                None
            } else if !o.invert {
                self.m.as_mut().unwrap().next_line(hay, pos)?
            } else {
                loop {
                    if pos >= hay.len() {
                        break None;
                    }
                    if looked_to <= pos {
                        next_match = self.m.as_mut().unwrap().next_line(hay, pos)?;
                        looked_to = next_match.map_or(hay.len(), |(_, e)| e + 1);
                    }
                    match next_match {
                        Some((s, e)) if s == pos => pos = e + 1,
                        _ => break Some((pos, line_end(pos))),
                    }
                }
            };
            // Lines owed after the last selected line, up to this one.
            let limit = selected.map_or(hay.len(), |(s, _)| s);
            while owed > 0 && shown_to < limit {
                let e = line_end(shown_to);
                let n = line_no(p, shown_to);
                self.line(io, label, n, &hay[shown_to..e], b'-');
                shown_to = e + 1;
                owed -= 1;
            }
            let Some((s, e)) = selected else { break };
            // Lines before this one, not printed already.
            let mut from = s;
            for _ in 0..o.before {
                if from <= shown_to {
                    break;
                }
                from = memrchr(b'\n', &hay[..from - 1]).map_or(0, |i| i + 1);
            }
            let from = from.max(shown_to.min(s));
            if o.mode == Mode::Lines {
                if self.printed && (from > shown_to || shown_to == 0) {
                    io.put(b"--\n");
                }
                let mut at = from;
                while at < s {
                    let le = line_end(at);
                    let n = line_no(p, at);
                    self.line(io, label, n, &hay[at..le], b'-');
                    at = le + 1;
                }
                let n = line_no(p, s);
                self.line(io, label, n, &hay[s..e], b':');
                self.printed = true;
            }
            p.count += 1;
            shown_to = e + 1;
            owed = if o.mode == Mode::Lines { o.after } else { 0 };
            pos = e + 1;
            if o.mode == Mode::List || io.stopped() {
                break;
            }
        }
        Ok(())
    }

    /// What is printed for a file once it has been searched.
    fn finish(&mut self, io: &mut Io, label: &Label, shown: &[u8], count: u64) {
        if count == 0 {
            return;
        }
        self.matched = true;
        match self.o.mode {
            Mode::Lines | Mode::Files => {}
            Mode::List => {
                io.put(shown);
                io.put(if self.o.null { b"\0" } else { b"\n" });
            }
            Mode::Count => {
                if let Some(l) = label.0 {
                    io.put(l);
                    io.put(if self.o.null { b"\0" } else { b":" });
                }
                io.put(count.to_string().as_bytes());
                io.put(b"\n");
            }
        }
    }

    fn too_complex(&mut self, io: &mut Io, shown: &[u8]) {
        io.err_line(format!("qdb: rg: {}: pattern too complex for the backtracking matcher", lossy(shown)));
        self.failed = true;
    }

    fn text(&mut self, io: &mut Io, label: &Label, shown: &[u8], hay: &[u8]) {
        let mut p = Progress::default();
        let done = if self.o.before > 0 || self.o.after > 0 { self.with_context(io, label, hay, &mut p) } else { self.plain(io, label, hay, &mut p).map(|_| ()) };
        match done {
            Ok(()) => self.finish(io, label, shown, p.count),
            Err(_) => self.too_complex(io, shown),
        }
    }

    fn file(&mut self, db: &Db, io: &mut Io, label: &Label, shown: &[u8], id: NodeId, pieces: Option<&Shortlist>) {
        self.searched += 1;
        if self.o.mode == Mode::Files {
            io.put(shown);
            io.put(if self.o.null { b"\0" } else { b"\n" });
            self.matched = true;
            return;
        }
        if self.o.before > 0 || self.o.after > 0 {
            // Lines around a match can lie in pieces that hold none.
            return self.text(io, label, shown, &db.content(id));
        }
        let mut p = Progress::default();
        for piece in db.pieces(id) {
            if pieces.is_some_and(|set| !db.piece_passes(set, piece)) {
                p.numbered += piece.lines as u64;
                continue;
            }
            match self.plain(io, label, &db.text_of(piece), &mut p) {
                Ok(true) => {}
                Ok(false) => break,
                Err(_) => return self.too_complex(io, shown),
            }
        }
        self.finish(io, label, shown, p.count);
    }

    // ------------------------------------------------- which files to search

    fn dir_rules(&mut self, db: &Db, dir: NodeId) -> Rc<DirRules> {
        if let Some(r) = self.rules.get(&dir) {
            return r.clone();
        }
        let read = |name: &[u8]| db.child(dir, name).filter(|id| !db.is_dir(*id)).map(|id| Rules::parse(&db.content(id)));
        let git = db.child(dir, b".git");
        let exclude = git.filter(|g| db.is_dir(*g)).and_then(|g| db.child(g, b"info")).filter(|i| db.is_dir(*i)).and_then(|i| db.child(i, b"exclude")).filter(|e| !db.is_dir(*e)).map(|e| Rules::parse(&db.content(e)));
        let r = Rc::new(DirRules { rgignore: read(b".rgignore"), ignore: read(b".ignore"), gitignore: read(b".gitignore"), exclude, has_git: git.is_some() });
        self.rules.insert(dir, r.clone());
        r
    }

    /// Whether the walk passes over an entry, as the ignore crate decides
    /// it: the -g globs first, then the ignore files from the entry's
    /// folder up to the root, then the types, then whether it is hidden.
    fn skip(&mut self, db: &Db, id: NodeId, is_dir: bool) -> bool {
        let o = self.o;
        let name = db.name(id);
        // The folders above, nearest first.
        let mut above = Vec::new();
        let mut cur = id;
        while let Some(parent) = db.parent(cur) {
            above.push(parent);
            cur = parent;
        }
        let simple = o.overrides.rules.is_empty() && o.types.is_empty();
        let rules: Vec<Rc<DirRules>> = if o.no_ignore { Vec::new() } else { above.iter().map(|d| self.dir_rules(db, *d)).collect() };
        if simple && rules.iter().all(|r| r.rgignore.is_none() && r.ignore.is_none() && r.gitignore.is_none() && r.exclude.is_none()) {
            return !o.hidden && name.starts_with(b".");
        }
        // The path from each folder above down to the entry.
        let mut paths: Vec<Vec<u8>> = Vec::with_capacity(above.len());
        let mut path = name.to_vec();
        let mut cur = id;
        for _ in &above {
            paths.push(path.clone());
            cur = db.parent(cur).unwrap();
            let mut longer = db.name(cur).to_vec();
            longer.push(b'/');
            longer.extend_from_slice(&path);
            path = longer;
        }
        let full = paths.last().cloned().unwrap_or_default();
        // Above the searched folder, the entry is taken to sit directly
        // in the folder those ignore files were first read for.
        if let Some(at) = above.iter().position(|d| *d == self.root) {
            if at + 1 < above.len() {
                let mut rel = name.to_vec();
                let mut cur = self.base;
                for (i, d) in above.iter().enumerate().skip(at + 1) {
                    while cur != *d {
                        let mut longer = db.name(cur).to_vec();
                        longer.push(b'/');
                        longer.extend_from_slice(&rel);
                        rel = longer;
                        cur = db.parent(cur).unwrap();
                    }
                    paths[i] = rel.clone();
                }
            }
        }
        if !o.overrides.rules.is_empty() {
            // A -g glob that matches keeps the entry, one with `!` leaves
            // it out; a file no glob matches is left out when there is a
            // glob without `!`.
            match o.overrides.matched(&full, is_dir) {
                Some(ignore) => return !ignore,
                None if o.overrides.ignores > 0 && !is_dir => return true,
                None => {}
            }
        }
        let mut kept = false;
        if !o.no_ignore {
            let any_git = rules.iter().any(|r| r.has_git);
            let (mut custom, mut ignore, mut gi, mut exclude) = (None, None, None, None);
            let mut saw_git = false;
            for (r, rel) in rules.iter().zip(&paths) {
                let ask = |rules: &Option<Rules>| rules.as_ref().and_then(|x| x.matched(rel, is_dir));
                if custom.is_none() {
                    custom = ask(&r.rgignore);
                }
                if ignore.is_none() {
                    ignore = ask(&r.ignore);
                }
                if any_git && !saw_git && gi.is_none() {
                    gi = ask(&r.gitignore);
                }
                if any_git && !saw_git && exclude.is_none() {
                    exclude = ask(&r.exclude);
                }
                saw_git |= r.has_git;
            }
            match custom.or(ignore).or(gi).or(exclude) {
                Some(true) => return true,
                Some(false) => kept = true,
                None => {}
            }
        }
        if !is_dir && !o.types.is_empty() {
            let last = o.types.iter().rev().find(|t| t.globs.iter().any(|g| g.matches(name)));
            match last {
                Some(t) if t.negate => return true,
                Some(_) => kept = true,
                None if o.selected => return true,
                None => {}
            }
        }
        !kept && !o.hidden && name.starts_with(b".")
    }

    fn skip_dir(&mut self, db: &Db, dir: NodeId) -> bool {
        if let Some(s) = self.skipped.get(&dir) {
            return *s;
        }
        let s = self.skip(db, dir, true);
        self.skipped.insert(dir, s);
        s
    }

    fn walk(&mut self, db: &Db, io: &mut Io, dir: NodeId, path: &mut Vec<u8>, pieces: Option<&Shortlist>, probe_only: bool) -> bool {
        let base = path.len();
        for (name, child) in db.children(dir) {
            if io.stopped() {
                return false;
            }
            let is_dir = db.is_dir(*child);
            if if is_dir { self.skip_dir(db, *child) } else { self.skip(db, *child, false) } {
                continue;
            }
            path.extend_from_slice(name);
            if is_dir {
                path.push(b'/');
                if self.walk(db, io, *child, path, pieces, probe_only) && probe_only {
                    path.truncate(base);
                    return true;
                }
            } else if probe_only {
                path.truncate(base);
                return true;
            } else if pieces.is_some_and(|set| !db.passes(set, *child)) {
                self.searched += 1;
            } else {
                let shown = path.clone();
                self.file(db, io, &Label(Some(&shown)), &shown, *child, pieces);
            }
            path.truncate(base);
        }
        false
    }

    /// Search under a folder. `prefix` is what goes in front of each path.
    fn folder(&mut self, db: &Db, io: &mut Io, dir: NodeId, prefix: &[u8]) {
        let mut path = prefix.to_vec();
        self.root = dir;
        self.skipped.clear();
        let mut up = Vec::new();
        let mut cur = dir;
        while let Some(parent) = db.parent(cur) {
            up.push(parent);
            cur = parent;
        }
        for d in up.iter().rev() {
            self.met_from.entry(*d).or_insert(dir);
        }
        self.base = up.first().map_or(dir, |d| self.met_from[d]);
        let ask = self.o.mode != Mode::Files && !self.o.invert;
        let answer = match &self.m {
            Some(m) if ask => db.candidates(dir, m.searcher.query()),
            _ => Candidates::All,
        };
        match answer {
            Candidates::All => {
                self.walk(db, io, dir, &mut path, None, false);
            }
            Candidates::Among(set) => {
                self.walk(db, io, dir, &mut path, Some(&set), false);
            }
            Candidates::Files(files, set) => {
                // Each candidate is searched if the walk would have
                // reached it: no folder between here and it is passed
                // over, and nor is it.
                'files: for c in files {
                    if io.stopped() {
                        break;
                    }
                    let mut chain = Vec::new();
                    let mut cur = c.id;
                    while let Some(parent) = db.parent(cur) {
                        if parent == dir {
                            break;
                        }
                        chain.push(parent);
                        cur = parent;
                    }
                    for d in chain.iter().rev() {
                        if self.skip_dir(db, *d) {
                            continue 'files;
                        }
                    }
                    if self.skip(db, c.id, false) {
                        continue;
                    }
                    let shown = [prefix, &c.path[..]].concat();
                    self.file(db, io, &Label(Some(&shown)), &shown, c.id, Some(&set));
                }
                if self.searched == 0 {
                    // Nothing printed can still mean files were searched.
                    if self.walk(db, io, dir, &mut path, None, true) {
                        self.searched += 1;
                    }
                }
            }
        }
    }
}

pub fn rg(args: Vec<Vec<u8>>) -> Result<Prepared, String> {
    let parsed = arguments(&args)?;
    let Parsed { opts, syn, pattern, paths, fatal } = parsed;
    let fail = |m: String| -> Result<Prepared, String> {
        let r: Runner = Box::new(move |_, io| {
            io.errors_first();
            io.err_line(m);
            2
        });
        Ok(Prepared { run: r, input: Input::Unused })
    };
    if let Some(m) = fatal {
        return fail(m);
    }
    let compiled = match (&pattern, opts.mode) {
        (_, Mode::Files) => None,
        (None, _) => return fail("rg: ripgrep requires at least one pattern to execute a search".into()),
        (Some(p), _) => match rg_pattern(p, syn) {
            Ok(c) => Some(c),
            Err(e) => {
                let hint = if e.contains("backreferences are not supported") || e.contains("look-around") {
                    "\n\nConsider enabling PCRE2 with the --pcre2 flag, which can handle backreferences\nand look-around."
                } else {
                    ""
                };
                return fail(format!("rg: {}{}", e, hint));
            }
        },
    };
    let reads_stdin = opts.mode != Mode::Files && (paths.is_empty() || paths.iter().any(|p| p == b"-"));
    let run: Runner = Box::new(move |db, io| {
        io.errors_first();
        let m = compiled.map(|pattern| Matcher { searcher: Searcher::regex(vec![pattern.ascii.clone()]), pattern });
        let mut s = Search { o: &opts, m, printed: false, matched: false, failed: false, searched: 0, rules: HashMap::new(), skipped: HashMap::new(), root: 0, met_from: HashMap::new(), base: 0 };
        let mut errors = false;
        let stdin = io.take_stdin();
        if paths.is_empty() {
            if io.has_stdin() && opts.mode != Mode::Files {
                s.text(io, &Label(None), b"<stdin>", stdin);
            } else {
                s.folder(db, io, db.resolve(b"/").expect("root"), b"");
                if s.searched == 0 && opts.mode != Mode::Files {
                    io.err_line("rg: No files were searched, which means ripgrep probably applied a filter you didn't expect.");
                    io.err_line("Running with --debug will show why files are being skipped.");
                    errors = true;
                }
            }
        } else {
            let named = paths.len() > 1 || paths.iter().any(|p| db.resolve(p).is_ok_and(|id| db.is_dir(id)));
            let mut stdin = Some(stdin);
            for p in &paths {
                if io.stopped() {
                    break;
                }
                if p == b"-" && opts.mode != Mode::Files {
                    let label = Label(named.then_some(b"<stdin>"));
                    s.text(io, &label, b"<stdin>", stdin.take().unwrap_or(b""));
                    continue;
                }
                match db.resolve(p) {
                    Err(e) => {
                        let code = match e {
                            Errno::NoEnt => 2,
                            Errno::NotDir => 20,
                            _ => 5,
                        };
                        io.err_line(format!("rg: {}: {} (os error {})", lossy(p), e.msg(), code));
                        errors = true;
                    }
                    Ok(id) if db.is_dir(id) => {
                        let mut prefix = p.clone();
                        while prefix.len() > 1 && prefix.ends_with(b"/") {
                            prefix.pop();
                        }
                        if !prefix.ends_with(b"/") {
                            prefix.push(b'/');
                        }
                        s.folder(db, io, id, &prefix);
                    }
                    Ok(id) => {
                        let label = Label((named || opts.mode == Mode::Files).then_some(&p[..]));
                        s.file(db, io, &label, p, id, None);
                    }
                }
            }
        }
        if errors || s.failed {
            2
        } else if s.matched {
            0
        } else {
            1
        }
    });
    Ok(Prepared { run, input: if reads_stdin { Input::Whole } else { Input::Unused } })
}
