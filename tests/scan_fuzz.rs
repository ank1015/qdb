//! The fast scanner against the reference matcher.
//!
//! `scan::Searcher` picks a fast path from the pattern: substring search,
//! a required-string filter, a lazy DFA. Whatever it picks, it must select
//! exactly the lines that `Regex::is_match` selects one line at a time.
//! This generates random patterns and random text and compares the two.

use qdb::regex::{Flavor, Regex, Syntax};
use qdb::scan::Searcher;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

const WORDS: &[&str] = &["ab", "abc", "foo", "bar", "Foo", "a", "b", "x_y", "TODO", "todo", "f(", "1", "22", "a.b", " ", "é"];

/// A random extended regular expression, mostly made of words so that
/// the fixed-string paths are taken often.
fn pattern(rng: &mut Rng, depth: usize) -> String {
    let mut out = String::new();
    for _ in 0..1 + rng.below(3) {
        let piece = match rng.below(if depth > 2 { 8 } else { 14 }) {
            0..=4 => regex_escape(rng.pick(WORDS)),
            5 => rng.pick(&[".", "[a-c]", "[^a]", "[0-9]", "[[:alpha:]]", "[Tt]", "[ab]"]).to_string(),
            6 => rng.pick(&["^", "$", "\\b", "\\<", "\\>", "\\B"]).to_string(),
            7 => format!("{}{}", regex_escape(rng.pick(WORDS)), rng.pick(&["*", "+", "?", "{2}", "{1,3}", "{2,}"])),
            8 | 9 => format!("({})", pattern(rng, depth + 1)),
            10 => format!("({}|{})", pattern(rng, depth + 1), pattern(rng, depth + 1)),
            11 => format!("({}){}", pattern(rng, depth + 1), rng.pick(&["*", "+", "?", "{2}"])),
            12 => format!("{}|{}", regex_escape(rng.pick(WORDS)), regex_escape(rng.pick(WORDS))),
            _ => format!("({})\\1", regex_escape(rng.pick(WORDS))),
        };
        out.push_str(&piece);
    }
    out
}

fn regex_escape(word: &str) -> String {
    let mut out = String::new();
    for c in word.chars() {
        if ".()[]{}*+?|^$\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn text(rng: &mut Rng) -> Vec<u8> {
    let mut out = Vec::new();
    let lines = rng.below(12);
    for i in 0..lines {
        for _ in 0..rng.below(6) {
            match rng.below(10) {
                0..=5 => out.extend_from_slice(rng.pick(WORDS).as_bytes()),
                6 => out.extend_from_slice(rng.pick(WORDS).to_ascii_uppercase().as_bytes()),
                7 => out.push(b' '),
                8 => out.extend_from_slice(b"abab"),
                _ => out.extend_from_slice(b"fooFOO"),
            }
        }
        if i + 1 < lines || rng.below(4) > 0 {
            out.push(b'\n');
        }
    }
    out
}

/// The lines the reference matcher selects, as (start, end).
fn reference(regexes: &[Regex], hay: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < hay.len() {
        let end = hay[start..].iter().position(|c| *c == b'\n').map_or(hay.len(), |i| start + i);
        if regexes.iter().any(|re| re.is_match(&hay[start..end]).unwrap()) {
            out.push((start, end));
        }
        start = end + 1;
    }
    out
}

fn scanned(searcher: &mut Searcher, hay: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some((start, end)) = searcher.next_line(hay, from).unwrap() {
        out.push((start, end));
        from = end + 1;
    }
    out
}

fn run(seed: u64, rounds: usize) -> usize {
    let mut rng = Rng(seed);
    let mut checked = 0;
    for round in 0..rounds {
        let icase = rng.below(3) == 0;
        let count = if rng.below(8) == 0 { 2 } else { 1 };
        let fixed = rng.below(6) == 0;
        let patterns: Vec<String> = (0..count).map(|_| if fixed { rng.pick(WORDS).to_string() } else { pattern(&mut rng, 0) }).collect();
        let mut searcher;
        let regexes: Vec<Regex>;
        if fixed {
            let strings: Vec<Vec<u8>> = patterns.iter().map(|p| p.as_bytes().to_vec()).collect();
            regexes = strings.iter().map(|s| Regex::from_strings(std::slice::from_ref(s), icase).unwrap()).collect();
            searcher = Searcher::fixed(strings, icase);
        } else {
            let syn = Syntax { extended: true, icase, flavor: Flavor::Grep };
            regexes = match patterns.iter().map(|p| Regex::new(p.as_bytes(), syn)).collect() {
                Ok(r) => r,
                Err(_) => continue,
            };
            searcher = Searcher::regex(regexes.clone());
        }
        // Several buffers through one searcher: its caches carry over.
        for _ in 0..4 {
            let hay = text(&mut rng);
            let want = reference(&regexes, &hay);
            let got = scanned(&mut searcher, &hay);
            assert_eq!(
                got,
                want,
                "seed {} round {}: patterns {:?} icase {} fixed {} on {:?}",
                seed,
                round,
                patterns,
                icase,
                fixed,
                String::from_utf8_lossy(&hay)
            );
            for &(start, end) in &want {
                assert!(searcher.is_match(&hay[start..end]).unwrap(), "seed {} round {}: is_match on a selected line", seed, round);
            }
            checked += 1;
        }
    }
    checked
}

#[test]
fn fast_paths_agree_with_the_reference_matcher() {
    let mut checked = 0;
    for seed in [1, 2, 3, 0xdead_beef, 0x1234_5678_9abc] {
        checked += run(seed, 6000);
    }
    assert!(checked > 60_000, "only {} buffers were checked", checked);
}

#[test]
fn long_lines_and_many_strings() {
    // A filter that lets nearly every line through is given up for the
    // DFA part of the way through; the answer must not change.
    let hay: Vec<u8> = (0..5000).flat_map(|i| format!("return value_{};\nreturn;\nother line {}\n", i, i).into_bytes()).collect();
    let syn = Syntax { extended: true, icase: false, flavor: Flavor::Grep };
    for p in ["return [a-z_0-9]+;$", "^return;$", "line [0-9]*7$", "(value|line)_?[0-9]{4}", "^$", "x*"] {
        let re = Regex::new(p.as_bytes(), syn).unwrap();
        let want = reference(std::slice::from_ref(&re), &hay);
        assert_eq!(scanned(&mut Searcher::regex(vec![re]), &hay), want, "pattern {}", p);
    }
    // More fixed strings than are searched for one by one.
    let strings: Vec<Vec<u8>> = (0..40).map(|i| format!("value_{}7", i).into_bytes()).collect();
    let regexes: Vec<Regex> = strings.iter().map(|s| Regex::from_strings(std::slice::from_ref(s), true).unwrap()).collect();
    assert_eq!(scanned(&mut Searcher::fixed(strings, true), &hay), reference(&regexes, &hay));
}
