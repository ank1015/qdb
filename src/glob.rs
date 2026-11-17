//! Shell pattern matching: `*`, `?` and `[...]`, on bytes (the C locale).
//!
//! Used for pathname expansion (bash rules) and for `find -name` (fnmatch
//! rules). The two differ only in how a leading `.` is treated, which the
//! caller decides.

use crate::tree::{Db, NodeId};

#[derive(Debug, Clone)]
pub enum Tok {
    Lit(u8),
    AnyOne,
    Star,
    Class(Box<[bool; 256]>),
}

pub fn class_by_name(name: &[u8]) -> Option<fn(u8) -> bool> {
    Some(match name {
        b"alpha" => |c: u8| c.is_ascii_alphabetic(),
        b"digit" => |c: u8| c.is_ascii_digit(),
        b"alnum" => |c: u8| c.is_ascii_alphanumeric(),
        b"upper" => |c: u8| c.is_ascii_uppercase(),
        b"lower" => |c: u8| c.is_ascii_lowercase(),
        b"space" => |c: u8| matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'),
        b"blank" => |c: u8| c == b' ' || c == b'\t',
        b"punct" => |c: u8| c.is_ascii_punctuation(),
        b"print" => |c: u8| (0x20..0x7f).contains(&c),
        b"graph" => |c: u8| (0x21..0x7f).contains(&c),
        b"cntrl" => |c: u8| c < 0x20 || c == 0x7f,
        b"xdigit" => |c: u8| c.is_ascii_hexdigit(),
        _ => return None,
    })
}

/// True if the pattern contains an unquoted `*`, `?` or `[`.
pub fn has_glob(pat: &[(u8, bool)]) -> bool {
    pat.iter().any(|(c, q)| !q && matches!(c, b'*' | b'?' | b'['))
}

/// Compile one pattern (no `/` handling). Each byte carries a flag saying
/// whether it was quoted; quoted bytes are always literal.
pub fn compile(pat: &[(u8, bool)]) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pat.len() {
        let (c, quoted) = pat[i];
        if quoted {
            out.push(Tok::Lit(c));
            i += 1;
            continue;
        }
        match c {
            b'*' => {
                if !matches!(out.last(), Some(Tok::Star)) {
                    out.push(Tok::Star);
                }
                i += 1;
            }
            b'?' => {
                out.push(Tok::AnyOne);
                i += 1;
            }
            b'[' => match parse_class(pat, i) {
                Some((set, next)) => {
                    out.push(Tok::Class(set));
                    i = next;
                }
                None => {
                    out.push(Tok::Lit(b'['));
                    i += 1;
                }
            },
            _ => {
                out.push(Tok::Lit(c));
                i += 1;
            }
        }
    }
    out
}

/// Parse a bracket expression starting at `pat[start] == '['`. Returns the
/// set and the index after the closing `]`, or None if there is no closing
/// bracket (the `[` is then an ordinary character).
fn parse_class(pat: &[(u8, bool)], start: usize) -> Option<(Box<[bool; 256]>, usize)> {
    let mut set = Box::new([false; 256]);
    let mut i = start + 1;
    let mut negate = false;
    if i < pat.len() && !pat[i].1 && (pat[i].0 == b'!' || pat[i].0 == b'^') {
        negate = true;
        i += 1;
    }
    let mut first = true;
    loop {
        let (c, quoted) = *pat.get(i)?;
        if c == b']' && !quoted && !first {
            i += 1;
            break;
        }
        first = false;
        if c == b'[' && !quoted && i + 1 < pat.len() && pat[i + 1] == (b':', false) {
            let close = (i + 2..pat.len().saturating_sub(1)).find(|&j| pat[j].0 == b':' && pat[j + 1].0 == b']');
            if let Some(close) = close {
                let name: Vec<u8> = pat[i + 2..close].iter().map(|p| p.0).collect();
                if let Some(f) = class_by_name(&name) {
                    for b in 0..=255u8 {
                        if f(b) {
                            set[b as usize] = true;
                        }
                    }
                    i = close + 2;
                    continue;
                }
            }
        }
        // A range, unless the '-' is the last character before ']'.
        if i + 2 < pat.len() && pat[i + 1] == (b'-', false) && !(pat[i + 2].0 == b']' && !pat[i + 2].1) {
            let hi = pat[i + 2].0;
            if c <= hi {
                for b in c..=hi {
                    set[b as usize] = true;
                }
            }
            i += 3;
            continue;
        }
        set[c as usize] = true;
        i += 1;
    }
    if negate {
        for b in set.iter_mut() {
            *b = !*b;
        }
    }
    Some((set, i))
}

pub fn matches(toks: &[Tok], name: &[u8]) -> bool {
    // Iterative matcher with single-star backtracking.
    let (mut t, mut n) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    loop {
        if t < toks.len() {
            match &toks[t] {
                Tok::Star => {
                    star = Some((t, n));
                    t += 1;
                    continue;
                }
                tok if n < name.len() => {
                    let ok = match tok {
                        Tok::Lit(c) => *c == name[n],
                        Tok::AnyOne => true,
                        Tok::Class(set) => set[name[n] as usize],
                        Tok::Star => unreachable!(),
                    };
                    if ok {
                        t += 1;
                        n += 1;
                        continue;
                    }
                }
                _ => {}
            }
        } else if n == name.len() {
            return true;
        }
        match star {
            Some((st, sn)) if sn < name.len() => {
                star = Some((st, sn + 1));
                t = st + 1;
                n = sn + 1;
            }
            _ => return false,
        }
    }
}

/// Convert an fnmatch pattern (as `find -name` receives it) to the
/// (byte, quoted) form: a backslash quotes the next byte.
pub fn fnmatch_pattern(pat: &[u8]) -> Vec<(u8, bool)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pat.len() {
        if pat[i] == b'\\' && i + 1 < pat.len() {
            out.push((pat[i + 1], true));
            i += 2;
        } else {
            out.push((pat[i], false));
            i += 1;
        }
    }
    out
}

/// Pathname expansion with bash defaults: `*` and `?` do not match a leading
/// `.`, `.` and `..` are never matched, there is no `**`, and results are
/// sorted in byte order. Returns an empty list when nothing matches; the
/// caller then passes the word through unchanged.
pub fn expand_path(db: &Db, word: &[(u8, bool)]) -> Vec<Vec<u8>> {
    let comps: Vec<&[(u8, bool)]> = word.split(|(c, _)| *c == b'/').collect();
    let mut states: Vec<(Vec<u8>, NodeId)> = vec![(Vec::new(), db.root())];
    for (idx, comp) in comps.iter().enumerate() {
        let sep: &[u8] = if idx > 0 { b"/" } else { b"" };
        let mut next = Vec::new();
        for (prefix, node) in states {
            if !db.is_dir(node) {
                continue;
            }
            let join = |name: &[u8]| {
                let mut p = prefix.clone();
                p.extend_from_slice(sep);
                p.extend_from_slice(name);
                p
            };
            if comp.is_empty() {
                next.push((join(b""), node));
            } else if !has_glob(comp) {
                let name: Vec<u8> = comp.iter().map(|p| p.0).collect();
                let child = match name.as_slice() {
                    b"." => Some(node),
                    b".." => Some(db.parent(node).unwrap_or(db.root())),
                    n => db.child(node, n),
                };
                if let Some(child) = child {
                    next.push((join(&name), child));
                }
            } else {
                let toks = compile(comp);
                let dot_ok = comp[0].0 == b'.';
                for (name, child) in db.children(node) {
                    if name[0] == b'.' && !dot_ok {
                        continue;
                    }
                    if matches(&toks, name) {
                        next.push((join(name), *child));
                    }
                }
            }
        }
        states = next;
    }
    let mut out: Vec<Vec<u8>> = states.into_iter().map(|s| s.0).collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pat: &str, name: &str) -> bool {
        matches(&compile(&fnmatch_pattern(pat.as_bytes())), name.as_bytes())
    }

    #[test]
    fn patterns() {
        assert!(m("*.txt", "a.txt"));
        assert!(!m("*.txt", "a.txt.bak"));
        assert!(m("a?c", "abc"));
        assert!(m("[a-c]x", "bx"));
        assert!(!m("[!a-c]x", "bx"));
        assert!(m("[]]", "]"));
        assert!(m("[[:digit:]]*", "9lives"));
        assert!(m("a[", "a["));
        assert!(m("\\*", "*"));
        assert!(!m("\\*", "a"));
        assert!(m("*a*b*", "xxaxxbxx"));
        assert!(m("[a-]", "-"));
    }
}
