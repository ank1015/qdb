//! The question a search asks storage: which files can contain a match?
//!
//! A `Query` is a condition on the three-byte sequences (trigrams) of a
//! file's content, built from AND and OR. Commands build one from their
//! pattern and hand it to `Db::candidates`; they never see how storage
//! answers it. Today the answer comes from an in-memory index; a later
//! stage can answer the same question from disk.
//!
//! Trigrams are taken over content with ASCII letters lowered (see
//! `bytes::fold`), so one query serves searches that respect case and
//! searches that ignore it. A trigram never contains a newline, because no
//! line can.

use crate::bytes::fold;

/// Three bytes of folded content, packed big-endian into the low 24 bits.
pub type Trigram = u32;

pub fn trigram(a: u8, b: u8, c: u8) -> Trigram {
    (fold(a) as u32) << 16 | (fold(b) as u32) << 8 | fold(c) as u32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// Every file can match: the pattern says nothing useful.
    All,
    /// No file can match.
    Nothing,
    /// The file's content contains this trigram.
    Has(Trigram),
    And(Vec<Query>),
    Or(Vec<Query>),
}

impl Query {
    /// How many trigrams the query names.
    pub fn trigrams(&self) -> usize {
        match self {
            Query::All | Query::Nothing => 0,
            Query::Has(_) => 1,
            Query::And(parts) | Query::Or(parts) => parts.iter().map(|p| p.trigrams()).sum(),
        }
    }

    /// Files containing the string: all of its trigrams. A string shorter
    /// than three bytes says nothing.
    pub fn string(s: &[u8]) -> Query {
        if s.len() < 3 {
            return Query::All;
        }
        let mut parts: Vec<Query> = Vec::new();
        for w in s.windows(3) {
            let q = if w.contains(&b'\n') { Query::Nothing } else { Query::Has(trigram(w[0], w[1], w[2])) };
            if !parts.contains(&q) {
                parts.push(q);
            }
        }
        Query::and(parts)
    }

    /// Files containing at least one of the strings.
    pub fn any_string<S: AsRef<[u8]>>(strings: &[S]) -> Query {
        Query::or(strings.iter().map(|s| Query::string(s.as_ref())).collect())
    }

    pub fn and(parts: Vec<Query>) -> Query {
        let mut out: Vec<Query> = Vec::new();
        for p in parts {
            match p {
                Query::All => {}
                Query::Nothing => return Query::Nothing,
                Query::And(inner) => {
                    for q in inner {
                        if !out.contains(&q) {
                            out.push(q);
                        }
                    }
                }
                q => {
                    if !out.contains(&q) {
                        out.push(q);
                    }
                }
            }
        }
        match out.len() {
            0 => Query::All,
            1 => out.pop().unwrap(),
            _ => Query::And(out),
        }
    }

    pub fn or(parts: Vec<Query>) -> Query {
        let mut out: Vec<Query> = Vec::new();
        for p in parts {
            match p {
                Query::Nothing => {}
                Query::All => return Query::All,
                Query::Or(inner) => {
                    for q in inner {
                        if !out.contains(&q) {
                            out.push(q);
                        }
                    }
                }
                q => {
                    if !out.contains(&q) {
                        out.push(q);
                    }
                }
            }
        }
        match out.len() {
            0 => Query::Nothing,
            1 => out.pop().unwrap(),
            _ => Query::Or(out),
        }
    }

    /// Whether content satisfies the query. This is the definition the
    /// index is tested against; it is not used to answer searches.
    pub fn accepts(&self, content: &[u8]) -> bool {
        match self {
            Query::All => true,
            Query::Nothing => false,
            Query::Has(t) => content.windows(3).any(|w| !w.contains(&b'\n') && trigram(w[0], w[1], w[2]) == *t),
            Query::And(parts) => parts.iter().all(|p| p.accepts(content)),
            Query::Or(parts) => parts.iter().any(|p| p.accepts(content)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings() {
        assert_eq!(Query::string(b"ab"), Query::All);
        assert_eq!(Query::string(b"aBc"), Query::Has(trigram(b'a', b'b', b'c')));
        assert!(Query::string(b"Hello").accepts(b"say HELLO there"));
        assert!(!Query::string(b"Hello").accepts(b"hel\nlo"));
        assert_eq!(Query::any_string(&[b"abc".to_vec(), b"x".to_vec()]), Query::All);
        assert_eq!(Query::and(vec![Query::All, Query::Nothing]), Query::Nothing);
        assert_eq!(Query::or(vec![Query::Nothing, Query::Nothing]), Query::Nothing);
    }
}
