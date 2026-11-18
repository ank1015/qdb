//! Fast searching in bytes: single bytes and substrings come from the
//! `memchr` crate, and a substring search that ignores ASCII case is
//! written here because that crate has none.
//!
//! Two definitions the whole engine shares live here, so that a change of
//! locale has one place to start from.
//!
//! One character is one byte. That is the C locale, which the engine
//! applies to everything: `.` and a bracket expression in a pattern, `?`
//! and `[...]` in a glob, and the word and character counts of `wc` all
//! work on single bytes. The code that relies on it is the byte sets in
//! `regex.rs` and `glob.rs`, the trigrams in `query.rs`, and `wc` in
//! `cmd/read.rs`; none of them decodes UTF-8.
//!
//! Case is folded one way: ASCII letters only, as in the C locale. `fold`
//! below is that definition, and `same_folded`, `fold_all` and
//! `other_case` are the only other forms of it.

pub use memchr::{memchr, memrchr};

/// The engine's only definition of case folding: ASCII letters to lower
/// case, every other byte unchanged.
#[inline]
pub fn fold(b: u8) -> u8 {
    b.to_ascii_lowercase()
}

/// Whether two bytes are the same when case is ignored.
#[inline]
pub fn same_folded(a: u8, b: u8) -> bool {
    fold(a) == fold(b)
}

/// A copy of `s` with every byte folded.
pub fn fold_all(s: &[u8]) -> Vec<u8> {
    s.iter().map(|b| fold(*b)).collect()
}

/// The byte that folds to the same as `b` and is not `b`, if there is one:
/// the capital of a small letter and the other way round.
#[inline]
pub fn other_case(b: u8) -> Option<u8> {
    (0..=255u8).find(|c| *c != b && fold(*c) == fold(b)).filter(|_| b.is_ascii_alphabetic())
}

/// How many newlines are in `hay`.
pub fn count_newlines(hay: &[u8]) -> usize {
    memchr::memchr_iter(b'\n', hay).count()
}

/// Append a number in decimal without allocating.
pub fn push_decimal(out: &mut Vec<u8>, mut n: usize) {
    let mut digits = [0u8; 20];
    let mut i = digits.len();
    loop {
        i -= 1;
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[i..]);
}

/// How common a byte is in source text, roughly: higher is more common.
/// Used to start a search from the rarest bytes of the needle.
fn commonness(b: u8) -> u8 {
    const ORDER: &[u8] = b" etaoinsrlcdu_pmhf\t(),.gb-=;v0*y/1w>x\"k:2<&{}'3[]4+q856j79#z!|%\\?$@~^`";
    match ORDER.iter().position(|c| *c == fold(b)) {
        Some(i) => 255 - i as u8,
        None => 0,
    }
}

/// Finds a fixed string, optionally without regard to ASCII case.
#[derive(Debug, Clone)]
pub struct Finder {
    needle: Vec<u8>,
    kind: Kind,
}

#[derive(Debug, Clone)]
enum Kind {
    Exact(memchr::memmem::Finder<'static>),
    /// The needle is stored folded. `first` and `second` are the offsets of
    /// its two rarest bytes, checked together before the rest.
    Folded { first: usize, second: usize },
}

impl Finder {
    pub fn new(needle: &[u8], ignore_case: bool) -> Finder {
        if !ignore_case || !needle.iter().any(|b| b.is_ascii_alphabetic()) {
            return Finder { needle: needle.to_vec(), kind: Kind::Exact(memchr::memmem::Finder::new(needle).into_owned()) };
        }
        let needle: Vec<u8> = needle.iter().map(|b| fold(*b)).collect();
        let mut by_rarity: Vec<usize> = (0..needle.len()).collect();
        by_rarity.sort_by_key(|i| (commonness(needle[*i]), *i));
        let first = by_rarity[0];
        let second = *by_rarity.get(1).unwrap_or(&first);
        Finder { needle, kind: Kind::Folded { first, second } }
    }

    pub fn needle_len(&self) -> usize {
        self.needle.len()
    }

    /// The first occurrence at or after `from`.
    pub fn find(&self, hay: &[u8], from: usize) -> Option<usize> {
        if from > hay.len() {
            return None;
        }
        match &self.kind {
            Kind::Exact(f) => f.find(&hay[from..]).map(|i| i + from),
            Kind::Folded { first, second } => {
                #[cfg(target_arch = "x86_64")]
                {
                    if std::arch::is_x86_feature_detected!("avx2") {
                        // Safe: the feature was just detected.
                        return unsafe { self.find_folded_avx2(hay, from, *first, *second) };
                    }
                }
                self.find_folded_scalar(hay, from, *first)
            }
        }
    }

    #[inline]
    fn matches_at(&self, hay: &[u8], at: usize) -> bool {
        hay[at..at + self.needle.len()].iter().zip(&self.needle).all(|(h, n)| fold(*h) == *n)
    }

    fn find_folded_scalar(&self, hay: &[u8], from: usize, first: usize) -> Option<usize> {
        let n = self.needle.len();
        if hay.len() < n {
            return None;
        }
        let last_start = hay.len() - n;
        let lower = self.needle[first];
        let upper = other_case(lower).unwrap_or(lower);
        let mut start = from;
        while start <= last_start {
            let window = &hay[start + first..last_start + first + 1];
            let hit = if lower == upper { memchr(lower, window) } else { memchr::memchr2(lower, upper, window) };
            match hit {
                None => return None,
                Some(off) => {
                    start += off;
                    if self.matches_at(hay, start) {
                        return Some(start);
                    }
                    start += 1;
                }
            }
        }
        None
    }

    /// Compare 32 positions at a time: a position is a candidate when the
    /// two rarest bytes of the needle are both there, in either case.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn find_folded_avx2(&self, hay: &[u8], from: usize, first: usize, second: usize) -> Option<usize> {
        use std::arch::x86_64::*;
        let n = self.needle.len();
        if hay.len() < n {
            return None;
        }
        let last_start = hay.len() - n;
        // OR-ing 0x20 into a byte folds a letter to lower case; it is only
        // applied where the needle has a letter, so nothing else can match.
        let bit = |b: u8| if b.is_ascii_lowercase() { 0x20u8 } else { 0 };
        let (b1, b2) = (self.needle[first], self.needle[second]);
        let v1 = _mm256_set1_epi8(b1 as i8);
        let v2 = _mm256_set1_epi8(b2 as i8);
        let m1 = _mm256_set1_epi8(bit(b1) as i8);
        let m2 = _mm256_set1_epi8(bit(b2) as i8);
        let ptr = hay.as_ptr();
        let mut start = from;
        // A block reads 32 bytes at start+first and at start+second.
        let reach = first.max(second) + 32;
        while start + reach <= hay.len() {
            let a = _mm256_loadu_si256(ptr.add(start + first) as *const __m256i);
            let b = _mm256_loadu_si256(ptr.add(start + second) as *const __m256i);
            let eq = _mm256_and_si256(_mm256_cmpeq_epi8(_mm256_or_si256(a, m1), v1), _mm256_cmpeq_epi8(_mm256_or_si256(b, m2), v2));
            let mut mask = _mm256_movemask_epi8(eq) as u32;
            while mask != 0 {
                let at = start + mask.trailing_zeros() as usize;
                if at > last_start {
                    return None;
                }
                if self.matches_at(hay, at) {
                    return Some(at);
                }
                mask &= mask - 1;
            }
            start += 32;
        }
        self.find_folded_scalar(hay, start, first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive(hay: &[u8], needle: &[u8], from: usize, icase: bool) -> Option<usize> {
        (from..=hay.len().saturating_sub(needle.len())).find(|&i| {
            i + needle.len() <= hay.len()
                && hay[i..i + needle.len()].iter().zip(needle).all(|(h, n)| if icase { fold(*h) == fold(*n) } else { h == n })
        })
    }

    #[test]
    fn finder_agrees_with_naive_search() {
        let mut seed = 0x1234_5678_9abc_def0u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let alphabet = b"abAB_x \n(";
        for round in 0..400 {
            let len = (next() % 300) as usize;
            let hay: Vec<u8> = (0..len).map(|_| alphabet[(next() % alphabet.len() as u64) as usize]).collect();
            let nlen = 1 + (next() % 5) as usize;
            let needle: Vec<u8> = (0..nlen).map(|_| alphabet[(next() % alphabet.len() as u64) as usize]).collect();
            for icase in [false, true] {
                let f = Finder::new(&needle, icase);
                let mut from = 0;
                loop {
                    let want = naive(&hay, &needle, from, icase);
                    assert_eq!(f.find(&hay, from), want, "round {} needle {:?} from {}", round, needle, from);
                    match want {
                        Some(at) => from = at + 1,
                        None => break,
                    }
                }
            }
        }
    }

    #[test]
    fn decimal() {
        let mut out = Vec::new();
        push_decimal(&mut out, 0);
        out.push(b' ');
        push_decimal(&mut out, 1234567890);
        assert_eq!(out, b"0 1234567890");
    }
}
