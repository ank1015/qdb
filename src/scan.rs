//! Finding the lines of a buffer that match a pattern, fast.
//!
//! The reference for what matches is the matcher in `regex.rs`. Everything
//! here is a faster way to the same answer, chosen from the pattern:
//!
//! * a pattern that is one fixed string, or a few, is found with a
//!   substring search over the whole buffer, and only then is the line
//!   around the hit worked out;
//! * a pattern with a fixed string every match must contain is searched
//!   for that string first, and only lines that have it are checked;
//! * anything else without a back-reference runs through a DFA built as
//!   it goes, one table lookup per byte;
//! * a pattern with a back-reference is checked line by line with the
//!   backtracking matcher, after the fixed-string filter if there is one.
//!
//! Nothing is split into lines or allocated until there is a candidate.

use crate::bytes::{fold_all, memchr, memrchr, Finder};
use crate::query::Query;
use crate::regex::{Dfa, Literals, MatchError, Regex};

/// One or more fixed strings, searched for together.
#[derive(Clone)]
struct Strings {
    finders: Vec<Finder>,
    /// For each string, its next occurrence at or after `valid_from` in
    /// the buffer `owner`, once looked for.
    next: Vec<Option<Option<usize>>>,
    owner: (usize, usize),
}

impl Strings {
    fn new(lits: &Literals) -> Strings {
        let finders: Vec<Finder> = lits.strings.iter().map(|s| Finder::new(s, lits.fold)).collect();
        Strings { next: vec![None; finders.len()], finders, owner: (0, 0) }
    }

    /// The first occurrence of any of the strings at or after `from`.
    fn find(&mut self, hay: &[u8], from: usize) -> Option<usize> {
        if self.finders.len() == 1 {
            return self.finders[0].find(hay, from);
        }
        // Each string is searched for separately and its next position
        // remembered, so every string passes over the buffer only once
        // however many matches there are.
        let owner = (hay.as_ptr() as usize, hay.len());
        if owner != self.owner || from == 0 {
            self.owner = owner;
            self.next.iter_mut().for_each(|n| *n = None);
        }
        let mut best: Option<usize> = None;
        for (finder, next) in self.finders.iter().zip(self.next.iter_mut()) {
            let stale = match next {
                None => true,
                Some(Some(at)) => *at < from,
                Some(None) => false,
            };
            if stale {
                *next = Some(finder.find(hay, from));
            }
            if let Some(Some(at)) = next {
                if best.map_or(true, |b| *at < b) {
                    best = Some(*at);
                }
            }
        }
        best
    }
}

#[derive(Clone)]
enum Kind {
    /// Every line matches.
    All,
    /// A line matches when it contains one of the strings.
    Strings(Strings),
    Regex {
        /// Strings of which a matching line has at least one.
        filter: Option<Strings>,
        dfa: Option<Dfa>,
        /// Used when there is no DFA: patterns with back-references.
        regexes: Vec<Regex>,
        /// How well the filter is doing: lines it let through, and bytes
        /// searched to find them.
        candidates: usize,
        searched: usize,
    },
}

/// A compiled search for the lines that match. A clone starts with what
/// the original has learned, and goes on learning on its own.
#[derive(Clone)]
pub struct Searcher {
    kind: Kind,
    /// What a file must contain to have a matching line.
    query: Query,
}

/// A filter that lets through more than one line in this many bytes, on
/// average, costs more than it saves.
const FILTER_MIN_SPACING: usize = 256;

fn line_around(hay: &[u8], at: usize) -> (usize, usize) {
    let start = memrchr(b'\n', &hay[..at]).map_or(0, |i| i + 1);
    let end = memchr(b'\n', &hay[at..]).map_or(hay.len(), |i| at + i);
    (start, end)
}

impl Searcher {
    /// Lines that contain any of the fixed strings.
    pub fn fixed(strings: Vec<Vec<u8>>, ignore_case: bool) -> Searcher {
        let strings: Vec<Vec<u8>> = if ignore_case { strings.iter().map(|s| fold_all(s)).collect() } else { strings };
        if strings.iter().any(|s| s.is_empty()) {
            return Searcher { kind: Kind::All, query: Query::All };
        }
        let query = Query::any_string(&strings);
        if strings.len() <= 8 {
            return Searcher { kind: Kind::Strings(Strings::new(&Literals { strings, fold: ignore_case })), query };
        }
        // Many strings: one pass of a DFA beats one pass per string.
        match Regex::from_strings(&strings, ignore_case) {
            Some(re) => Searcher { query, ..Searcher::regex(vec![re]) },
            None => Searcher { kind: Kind::Strings(Strings::new(&Literals { strings, fold: ignore_case })), query },
        }
    }

    /// Lines that match any of the patterns.
    pub fn regex(regexes: Vec<Regex>) -> Searcher {
        let combined = if regexes.len() == 1 { Some(regexes[0].clone()) } else { Regex::any_of(&regexes) };
        let combined = match combined {
            Some(c) => c,
            // Several patterns, at least one with a back-reference.
            None => {
                let kind = Kind::Regex { filter: None, dfa: None, regexes, candidates: 0, searched: 0 };
                return Searcher { kind, query: Query::All };
            }
        };
        if let Some(lits) = combined.as_literals() {
            if lits.strings.iter().any(|s| s.is_empty()) {
                return Searcher { kind: Kind::All, query: Query::All };
            }
            if lits.strings.len() <= 8 {
                let query = Query::any_string(&lits.strings);
                return Searcher { kind: Kind::Strings(Strings::new(&lits)), query };
            }
        }
        let required = combined.required_literals().filter(|l| l.strings.len() <= 8 && l.strings.iter().all(|s| s.len() >= 2));
        let dfa = Dfa::new(&combined);
        let query = match (&dfa, &required) {
            (Some(_), _) => Query::or(regexes.iter().map(|re| re.trigram_query()).collect()),
            // The backtracking matcher can give up on a line. So that a
            // search reports that for the same files with and without an
            // index, the index may rule out only the files the filter
            // below would pass over without trying a line.
            (None, Some(lits)) => Query::any_string(&lits.strings),
            (None, None) => Query::All,
        };
        let filter = required.map(|l| Strings::new(&l));
        Searcher { kind: Kind::Regex { filter, dfa, regexes: vec![combined], candidates: 0, searched: 0 }, query }
    }

    /// What a file must contain to have a matching line: the question to
    /// put to `Db::candidates`.
    pub fn query(&self) -> &Query {
        &self.query
    }

    /// The first matching line that starts at or after `from`, as (start,
    /// end) without its newline. `from` must be the start of a line.
    pub fn next_line(&mut self, hay: &[u8], from: usize) -> Result<Option<(usize, usize)>, MatchError> {
        if from >= hay.len() {
            return Ok(None);
        }
        match &mut self.kind {
            Kind::All => Ok(Some((from, memchr(b'\n', &hay[from..]).map_or(hay.len(), |i| from + i)))),
            Kind::Strings(strings) => Ok(strings.find(hay, from).map(|at| line_around(hay, at))),
            Kind::Regex { filter, dfa, regexes, candidates, searched } => {
                let mut from = from;
                if let Some(strings) = filter {
                    loop {
                        let at = match strings.find(hay, from) {
                            Some(at) => at,
                            None => {
                                *searched += hay.len() - from;
                                return Ok(None);
                            }
                        };
                        let (start, end) = line_around(hay, at);
                        *candidates += 1;
                        *searched += end - from;
                        let line = &hay[start..end];
                        let hit = match dfa {
                            Some(dfa) => dfa.is_match(line),
                            None => any_match(regexes, line)?,
                        };
                        if hit {
                            return Ok(Some((start, end)));
                        }
                        from = end + 1;
                        if from >= hay.len() {
                            return Ok(None);
                        }
                        // With a DFA to fall back on, give the filter up
                        // once it is letting too many lines through.
                        if dfa.is_some() && *candidates >= 64 && *searched < *candidates * FILTER_MIN_SPACING {
                            *filter = None;
                            break;
                        }
                    }
                }
                match dfa {
                    Some(dfa) => Ok(dfa.next_line(hay, from)),
                    None => {
                        while from < hay.len() {
                            let end = memchr(b'\n', &hay[from..]).map_or(hay.len(), |i| from + i);
                            if any_match(regexes, &hay[from..end])? {
                                return Ok(Some((from, end)));
                            }
                            from = end + 1;
                        }
                        Ok(None)
                    }
                }
            }
        }
    }

    /// Whether one line, given without its newline, matches.
    pub fn is_match(&mut self, line: &[u8]) -> Result<bool, MatchError> {
        if line.is_empty() {
            return match &mut self.kind {
                Kind::All => Ok(true),
                Kind::Strings(_) => Ok(false),
                Kind::Regex { dfa: Some(dfa), .. } => Ok(dfa.is_match(line)),
                Kind::Regex { regexes, .. } => any_match(regexes, line),
            };
        }
        Ok(self.next_line(line, 0)?.is_some())
    }
}

fn any_match(regexes: &[Regex], line: &[u8]) -> Result<bool, MatchError> {
    for re in regexes {
        if re.is_match(line)? {
            return Ok(true);
        }
    }
    Ok(false)
}
