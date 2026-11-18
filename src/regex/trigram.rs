//! From a pattern to a trigram query: a condition on three-byte sequences
//! that every file with a matching line must satisfy.
//!
//! The method is the one in Russ Cox, "Regular Expression Matching with a
//! Trigram Index" (2012). For each part of the pattern five things are
//! worked out: whether it can match the empty string, the exact set of
//! strings it matches if that set is small, a set of prefixes and a set of
//! suffixes of everything it matches, and a query. Parts are combined
//! bottom-up, and whenever a set grows too large its trigrams are moved
//! into the query and the set is cut down.
//!
//! All strings are folded (ASCII letters lowered), because the index is
//! built over folded content. The query is therefore a little weaker than
//! it could be for a case-sensitive pattern, never wrong.

use super::{Node, Regex};
use crate::bytes::fold;
use crate::query::Query;

/// A set of strings: sorted, no duplicates.
type Set = Vec<Vec<u8>>;

const MAX_EXACT: usize = 7;
const MAX_SET: usize = 20;
/// A character class with more members than this is treated as "any
/// character".
const MAX_CLASS: usize = 8;

struct Info {
    can_empty: bool,
    /// Every string this can match, if known.
    exact: Option<Set>,
    /// When `exact` is None: every match starts with one of `prefix` and
    /// ends with one of `suffix`.
    prefix: Set,
    suffix: Set,
    query: Query,
}

fn clean(mut set: Set) -> Set {
    set.sort();
    set.dedup();
    set
}

fn min_len(set: &Set) -> usize {
    set.iter().map(|s| s.len()).min().unwrap_or(0)
}

fn cross(a: &Set, b: &Set) -> Set {
    let mut out = Vec::with_capacity(a.len() * b.len());
    for x in a {
        for y in b {
            out.push([x.as_slice(), y.as_slice()].concat());
        }
    }
    clean(out)
}

fn union(a: &Set, b: &Set) -> Set {
    clean(a.iter().chain(b.iter()).cloned().collect())
}

/// The query "contains one of these strings", or All if one of them is
/// too short to say anything.
fn any_of(set: &Set) -> Query {
    if set.is_empty() || min_len(set) < 3 {
        return Query::All;
    }
    Query::any_string(set)
}

impl Info {
    fn empty_string() -> Info {
        Info { can_empty: true, exact: Some(vec![Vec::new()]), prefix: Vec::new(), suffix: Vec::new(), query: Query::All }
    }

    /// Matches some single character, nothing more known.
    fn any_char() -> Info {
        Info { can_empty: false, exact: None, prefix: vec![Vec::new()], suffix: vec![Vec::new()], query: Query::All }
    }

    /// Matches anything at all, the empty string included.
    fn anything() -> Info {
        Info { can_empty: true, exact: None, prefix: vec![Vec::new()], suffix: vec![Vec::new()], query: Query::All }
    }

    /// Move what the exact set says into the query and into the prefix
    /// and suffix sets, and forget the exact set.
    fn drop_exact(&mut self) {
        if let Some(exact) = self.exact.take() {
            self.query = Query::and(vec![std::mem::replace(&mut self.query, Query::All), any_of(&exact)]);
            for s in &exact {
                let n = s.len();
                if n < 3 {
                    self.prefix.push(s.clone());
                    self.suffix.push(s.clone());
                } else {
                    self.prefix.push(s[..2].to_vec());
                    self.suffix.push(s[n - 2..].to_vec());
                }
            }
            self.prefix = clean(std::mem::take(&mut self.prefix));
            self.suffix = clean(std::mem::take(&mut self.suffix));
        }
    }

    /// Keep the sets small. With `force`, an exact set whose strings are
    /// long enough to give trigrams is always moved into the query.
    fn simplify(&mut self, force: bool) {
        if let Some(exact) = &self.exact {
            let shortest = min_len(exact);
            if exact.len() > MAX_EXACT || (shortest >= 3 && force) || shortest >= 4 {
                self.drop_exact();
            }
        }
        if self.exact.is_none() {
            self.query = Query::and(vec![std::mem::replace(&mut self.query, Query::All), any_of(&self.prefix), any_of(&self.suffix)]);
            self.prefix = shorten(std::mem::take(&mut self.prefix), true);
            self.suffix = shorten(std::mem::take(&mut self.suffix), false);
        }
    }
}

/// Cut the strings of a prefix or suffix set down to two bytes, and
/// further while there are too many of them. A shorter prefix is a weaker
/// claim, so this is always allowed.
fn shorten(mut set: Set, is_prefix: bool) -> Set {
    let mut n = 3;
    while n == 3 || set.len() > MAX_SET {
        if n == 0 {
            break;
        }
        for s in set.iter_mut() {
            if s.len() >= n {
                if is_prefix {
                    s.truncate(n - 1);
                } else {
                    s.drain(..s.len() - (n - 1));
                }
            }
        }
        set = clean(set);
        n -= 1;
    }
    set
}

fn concat(mut x: Info, mut y: Info) -> Info {
    let mut out = Info { can_empty: x.can_empty && y.can_empty, exact: None, prefix: Vec::new(), suffix: Vec::new(), query: Query::All };
    match (&x.exact, &y.exact) {
        (Some(a), Some(b)) => out.exact = Some(cross(a, b)),
        _ => {
            out.prefix = match &x.exact {
                Some(a) => cross(a, &y.prefix),
                None if x.can_empty => union(&x.prefix, &y.prefix),
                None => x.prefix.clone(),
            };
            out.suffix = match &y.exact {
                Some(b) => cross(&x.suffix, b),
                None if y.can_empty => union(&y.suffix, &x.suffix),
                None => y.suffix.clone(),
            };
            // If both sides are open, a trigram can still straddle the
            // join: the end of x followed by the start of y.
            if x.exact.is_none() && y.exact.is_none() && x.suffix.len() <= MAX_SET && y.prefix.len() <= MAX_SET && min_len(&x.suffix) + min_len(&y.prefix) >= 3 {
                out.query = any_of(&cross(&x.suffix, &y.prefix));
            }
            // When only one side is exact, its strings are part of every
            // match, so they belong in the query too.
            if x.exact.is_some() {
                x.drop_exact();
            }
            if y.exact.is_some() {
                y.drop_exact();
            }
        }
    }
    out.query = Query::and(vec![out.query, x.query, y.query]);
    out.simplify(false);
    out
}

fn alternate(mut x: Info, mut y: Info) -> Info {
    let mut out = Info { can_empty: x.can_empty || y.can_empty, exact: None, prefix: Vec::new(), suffix: Vec::new(), query: Query::All };
    match (&x.exact, &y.exact) {
        (Some(a), Some(b)) => out.exact = Some(union(a, b)),
        _ => {
            x.drop_exact();
            y.drop_exact();
            out.prefix = union(&x.prefix, &y.prefix);
            out.suffix = union(&x.suffix, &y.suffix);
        }
    }
    out.query = Query::or(vec![x.query, y.query]);
    out.simplify(false);
    out
}

fn analyse(n: &Node) -> Info {
    match n {
        Node::Empty | Node::Assert(_) => Info::empty_string(),
        // A back-reference repeats text that is already accounted for, or
        // matches nothing; either way it adds no condition.
        Node::Backref(_) => Info::anything(),
        Node::Set(set) => {
            let mut members: Vec<u8> = (0..=255u8).filter(|b| set[*b as usize] && *b != b'\n').map(fold).collect();
            members.sort_unstable();
            members.dedup();
            if members.is_empty() || members.len() > MAX_CLASS {
                return Info::any_char();
            }
            let mut info = Info { can_empty: false, exact: Some(members.into_iter().map(|b| vec![b]).collect()), prefix: Vec::new(), suffix: Vec::new(), query: Query::All };
            info.simplify(false);
            info
        }
        Node::Group(sub, _) => analyse(sub),
        Node::Cat(parts) => parts.iter().map(analyse).reduce(concat).unwrap_or_else(Info::empty_string),
        Node::Alt(parts) => parts.iter().map(analyse).reduce(alternate).unwrap_or_else(Info::empty_string),
        Node::Repeat { sub, min, max } => {
            if *min == 0 {
                if *max == Some(0) {
                    return Info::empty_string();
                }
                if *max == Some(1) {
                    // x? is x or nothing.
                    return alternate(analyse(sub), Info::empty_string());
                }
                return Info::anything();
            }
            // At least `min` copies in a row; a few of them are enough to
            // reason about.
            let copies = (*min).min(4);
            let mut info = (0..copies).map(|_| analyse(sub)).reduce(concat).expect("at least one copy");
            if *max == Some(*min) && *min <= 4 {
                return info;
            }
            // More copies may follow: what is known is how a match starts
            // and ends, not all of it.
            if let Some(exact) = info.exact.clone() {
                info.drop_exact();
                info.prefix = exact.clone();
                info.suffix = exact;
            }
            info.simplify(false);
            info
        }
    }
}

impl Regex {
    /// A condition on trigrams that the content of any file with a
    /// matching line satisfies.
    pub fn trigram_query(&self) -> Query {
        let mut info = analyse(&self.ast);
        info.simplify(true);
        if info.exact.is_some() {
            // Short exact strings: all that is left to say.
            info.drop_exact();
        }
        info.query
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Flavor, Syntax};
    use super::*;

    fn query(p: &str, icase: bool) -> Query {
        Regex::new(p.as_bytes(), Syntax { extended: true, icase, flavor: Flavor::Grep }).unwrap().trigram_query()
    }

    #[test]
    fn literals() {
        assert_eq!(query("abc", false), Query::string(b"abc"));
        assert_eq!(query("Hello", false), Query::string(b"hello"));
        assert_eq!(query("Hello", true), Query::string(b"hello"));
        assert_eq!(query("ab", false), Query::All);
        assert_eq!(query("", false), Query::All);
        assert_eq!(query(".*", false), Query::All);
    }

    #[test]
    fn structure() {
        assert_eq!(query("abc|xyz", false), Query::or(vec![Query::string(b"abc"), Query::string(b"xyz")]));
        assert_eq!(query("^#include", false), Query::string(b"#include"));
        assert_eq!(query("abc.*def", false), Query::and(vec![Query::string(b"abc"), Query::string(b"def")]));
        assert_eq!(query("abc[0-9]+def", false), Query::and(vec![Query::string(b"abc"), Query::string(b"def")]));
        // The class is small enough to enumerate: abx or aby must appear.
        let q = query("ab[xy]", false);
        assert!(q.accepts(b"..abx..") && q.accepts(b"ABY") && !q.accepts(b"abz"));
        // One side optional: only the certain part counts.
        let q = query("(abc)?def", false);
        assert!(q.accepts(b"def") && q.accepts(b"xabcdef") && !q.accepts(b"abc"));
        assert_eq!(query("(abc)+", false), Query::string(b"abc"));
        assert_eq!(query("(abc)*", false), Query::All);
        assert_eq!(query("(abc){2}", false), Query::string(b"abcabc"));
        assert_eq!(query("(abc)\\1", false), Query::string(b"abc"));
    }
}
