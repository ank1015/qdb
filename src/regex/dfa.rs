//! A DFA built as it runs, for patterns without back-references.
//!
//! It answers one question: which lines of a buffer contain a match. Each
//! DFA state is a set of positions in the NFA program of `regex.rs`, so it
//! accepts exactly what the NFA simulation there accepts; the fuzzer in the
//! test harness compares the two. States and their transitions are worked
//! out the first time they are needed and remembered.
//!
//! Assertions (`^`, `$`, `\b`, `\<`, `\>`, `\B`) depend on the bytes on
//! both sides of a position. A state therefore records what came before
//! (start of line, or whether the last byte was a word byte), and an
//! assertion is decided when the next byte, or the end of the line, is
//! known.

use super::{is_word, Assert, Inst, Regex};
use crate::bytes::memchr;
use std::collections::HashMap;

/// Table entries below `FIRST_STATE` are not states.
const UNKNOWN: u32 = 0;
/// The line matches.
const MATCH: u32 = 1;
/// Nothing on the rest of this line can match.
const DEAD: u32 = 2;
/// The line ended without a match.
const NEXT_LINE: u32 = 3;
const FIRST_STATE: u32 = 4;

/// Above this many states the cache is thrown away at the next line and
/// built again.
const MAX_STATES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    /// Program positions, sorted. Assertions are in the set unresolved.
    pcs: Vec<u32>,
    at_line_start: bool,
    after_word: bool,
}

#[derive(Debug, Clone)]
pub struct Dfa {
    prog: Vec<Inst>,
    sets: Vec<Box<[bool; 256]>>,
    /// Byte to byte class; bytes in one class behave alike everywhere.
    class: [u8; 256],
    /// One byte of each class, and the class of the end of a line.
    representative: Vec<u8>,
    stride: usize,
    /// `table[state * stride + class]`; states are stored premultiplied.
    table: Vec<u32>,
    keys: Vec<Key>,
    ids: HashMap<Key, u32>,
    start: u32,
    uses_word: bool,
    /// For a state that often leads back to itself: the bytes that do.
    /// Runs of them are skipped eight at a time. Built on first need.
    stay: Vec<Option<Box<[bool; 256]>>>,
    // Scratch for building states.
    stack: Vec<u32>,
    seen: Vec<u32>,
    stamp: u32,
}

impl Dfa {
    /// None if the pattern needs the backtracking matcher.
    pub fn new(re: &Regex) -> Option<Dfa> {
        if re.has_backref {
            return None;
        }
        let prog = re.prog.clone();
        let sets = re.sets.clone();
        // Two bytes are in one class when every set treats them alike and
        // they are both word bytes or both not. Newline is always alone.
        let uses_word = prog.iter().any(|i| matches!(i, Inst::Assert(a) if !matches!(a, Assert::LineStart | Assert::LineEnd)));
        let mut signature: HashMap<Vec<bool>, u8> = HashMap::new();
        let mut class = [0u8; 256];
        let mut representative = Vec::new();
        for b in 0..=255u8 {
            let mut sig: Vec<bool> = sets.iter().map(|s| s[b as usize]).collect();
            sig.push(uses_word && is_word(b));
            sig.push(b == b'\n');
            let next = signature.len() as u8;
            let c = *signature.entry(sig).or_insert_with(|| {
                representative.push(b);
                next
            });
            class[b as usize] = c;
        }
        let stride = representative.len() + 1;
        let n = prog.len();
        let mut dfa = Dfa {
            prog,
            sets,
            class,
            representative,
            stride,
            table: Vec::new(),
            keys: Vec::new(),
            ids: HashMap::new(),
            start: 0,
            uses_word,
            stay: Vec::new(),
            stack: Vec::new(),
            seen: vec![0; n],
            stamp: 0,
        };
        dfa.reset();
        Some(dfa)
    }

    fn reset(&mut self) {
        self.table.clear();
        self.table.resize(FIRST_STATE as usize * self.stride, UNKNOWN);
        self.keys.clear();
        self.keys.resize(FIRST_STATE as usize, Key { pcs: Vec::new(), at_line_start: false, after_word: false });
        self.ids.clear();
        self.stay.clear();
        let mut pcs = Vec::new();
        self.new_stamp();
        self.closure(0, None, &mut pcs);
        pcs.sort_unstable();
        self.start = self.intern(Key { pcs, at_line_start: true, after_word: false });
    }

    /// Add `pc` and everything reachable from it without consuming input.
    /// With `context` (start of line, after a word byte, before a word
    /// byte, end of line) assertions that hold are passed through;
    /// without it they stay in the set for later.
    fn closure(&mut self, pc: u32, context: Option<(bool, bool, bool, bool)>, out: &mut Vec<u32>) {
        // One closure call may be one of several building the same set;
        // the caller bumps the stamp between sets.
        self.stack.push(pc);
        while let Some(pc) = self.stack.pop() {
            if self.seen[pc as usize] == self.stamp {
                continue;
            }
            self.seen[pc as usize] = self.stamp;
            match &self.prog[pc as usize] {
                Inst::Jmp(t) => self.stack.push(*t as u32),
                Inst::Split(a, b) => {
                    self.stack.push(*b as u32);
                    self.stack.push(*a as u32);
                }
                Inst::Assert(a) => match context {
                    None => out.push(pc),
                    Some((line_start, prev, next, line_end)) => {
                        let holds = match a {
                            Assert::LineStart => line_start,
                            Assert::LineEnd => line_end,
                            Assert::WordBoundary => prev != next,
                            Assert::NotWordBoundary => prev == next,
                            Assert::WordStart => !prev && next,
                            Assert::WordEnd => prev && !next,
                        };
                        if holds {
                            self.stack.push(pc + 1);
                        }
                    }
                },
                Inst::Set(_) | Inst::Match => out.push(pc),
            }
        }
    }

    fn new_stamp(&mut self) {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.seen.iter_mut().for_each(|s| *s = 0);
            self.stamp = 1;
        }
    }

    fn intern(&mut self, key: Key) -> u32 {
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let id = (self.keys.len() * self.stride) as u32;
        self.keys.push(key.clone());
        self.table.resize(self.table.len() + self.stride, UNKNOWN);
        self.ids.insert(key, id);
        id
    }

    /// Work out one transition and remember it. `class` is a byte class,
    /// or `stride - 1` for the end of the line.
    fn step(&mut self, state: u32, class: usize) -> u32 {
        let key = self.keys[state as usize / self.stride].clone();
        let byte = if class == self.stride - 1 { b'\n' } else { self.representative[class] };
        // A newline in the data ends the line just as the end of data does.
        let line_end = byte == b'\n';
        let next_word = !line_end && is_word(byte);
        let context = Some((key.at_line_start, key.after_word, next_word, line_end));

        // Decide the assertions now that both sides are known.
        self.new_stamp();
        let mut here = Vec::new();
        for &pc in &key.pcs {
            match self.prog[pc as usize] {
                Inst::Assert(_) => {
                    // Re-enter the closure at the assertion itself.
                    self.seen[pc as usize] = self.stamp.wrapping_sub(1);
                    self.closure(pc, context, &mut here);
                }
                _ => {
                    if self.seen[pc as usize] != self.stamp {
                        self.seen[pc as usize] = self.stamp;
                        here.push(pc);
                    }
                }
            }
        }
        let result = if here.iter().any(|pc| matches!(self.prog[*pc as usize], Inst::Match)) {
            MATCH
        } else if line_end {
            NEXT_LINE
        } else {
            // Consume the byte, and let a new match start after it.
            self.new_stamp();
            let mut next = Vec::new();
            for &pc in &here {
                if let Inst::Set(s) = self.prog[pc as usize] {
                    if self.sets[s][byte as usize] {
                        self.closure(pc + 1, None, &mut next);
                    }
                }
            }
            self.closure(0, None, &mut next);
            if next.iter().any(|pc| matches!(self.prog[*pc as usize], Inst::Match)) {
                MATCH
            } else if !self.can_still_match(&next) {
                DEAD
            } else {
                next.sort_unstable();
                self.intern(Key { pcs: next, at_line_start: false, after_word: self.uses_word && is_word(byte) })
            }
        };
        self.table[state as usize + class] = result;
        result
    }

    /// Work out which bytes lead from a state back to itself.
    fn build_stay(&mut self, state: u32) {
        let index = state as usize / self.stride;
        let mut by_class = vec![false; self.stride - 1];
        for (class, stays) in by_class.iter_mut().enumerate() {
            let mut next = self.table[state as usize + class];
            if next == UNKNOWN {
                next = self.step(state, class);
            }
            *stays = next == state;
        }
        let mut stay = Box::new([false; 256]);
        for b in 0..256 {
            stay[b] = by_class[self.class[b] as usize];
        }
        if self.stay.len() <= index {
            self.stay.resize(index + 1, None);
        }
        self.stay[index] = Some(stay);
    }

    /// False when, past the start of a line, no position in the set can
    /// ever consume a byte or match: only `^` stands in the way.
    fn can_still_match(&mut self, pcs: &[u32]) -> bool {
        self.new_stamp();
        let mut stack: Vec<u32> = pcs.to_vec();
        while let Some(pc) = stack.pop() {
            if self.seen[pc as usize] == self.stamp {
                continue;
            }
            self.seen[pc as usize] = self.stamp;
            match &self.prog[pc as usize] {
                Inst::Set(_) | Inst::Match => return true,
                Inst::Jmp(t) => stack.push(*t as u32),
                Inst::Split(a, b) => {
                    stack.push(*a as u32);
                    stack.push(*b as u32);
                }
                Inst::Assert(Assert::LineStart) => {}
                Inst::Assert(_) => stack.push(pc + 1),
            }
        }
        false
    }

    /// The first line starting at or after `from` that contains a match,
    /// as (start, end) without its newline. `from` must be the start of a
    /// line.
    pub fn next_line(&mut self, hay: &[u8], from: usize) -> Option<(usize, usize)> {
        let len = hay.len();
        let end_class = self.stride - 1;
        let mut line = from;
        let mut i = from;
        if self.keys.len() > MAX_STATES {
            self.reset();
        }
        let mut state = self.start;
        loop {
            // The hot loop: one table lookup per byte.
            let mut next = UNKNOWN;
            {
                let table = self.table.as_slice();
                let class = &self.class;
                while i < len {
                    next = table[state as usize + class[hay[i] as usize] as usize];
                    if next < FIRST_STATE || next == state {
                        break;
                    }
                    state = next;
                    i += 1;
                }
            }
            if i < len && next == state {
                // The state leads back to itself. Most of a search is
                // spent in such a state, waiting for a byte that could
                // start a match, so skip the bytes that keep it there.
                let index = state as usize / self.stride;
                if self.stay.get(index).map_or(true, |s| s.is_none()) {
                    self.build_stay(state);
                }
                let stay = self.stay[index].as_deref().expect("just built");
                i += 1;
                while i + 8 <= len {
                    let w = &hay[i..i + 8];
                    let all = stay[w[0] as usize] & stay[w[1] as usize] & stay[w[2] as usize] & stay[w[3] as usize]
                        & stay[w[4] as usize] & stay[w[5] as usize] & stay[w[6] as usize] & stay[w[7] as usize];
                    if !all {
                        break;
                    }
                    i += 8;
                }
                while i < len && stay[hay[i] as usize] {
                    i += 1;
                }
                continue;
            }
            if i >= len {
                // The end of the data ends the last line, if there is one.
                if line >= len {
                    return None;
                }
                let mut last = self.table[state as usize + end_class];
                if last == UNKNOWN {
                    last = self.step(state, end_class);
                }
                return if last == MATCH { Some((line, len)) } else { None };
            }
            match next {
                UNKNOWN => {
                    let class = self.class[hay[i] as usize] as usize;
                    self.step(state, class);
                }
                MATCH => {
                    let end = if hay[i] == b'\n' { i } else { memchr(b'\n', &hay[i..]).map_or(len, |n| i + n) };
                    return Some((line, end));
                }
                _ => {
                    // The line is over, or nothing more on it can match.
                    if next == DEAD {
                        match memchr(b'\n', &hay[i..]) {
                            None => return None,
                            Some(n) => i += n,
                        }
                    }
                    i += 1;
                    line = i;
                    // The cache is only ever emptied between lines.
                    if self.keys.len() > MAX_STATES {
                        self.reset();
                    }
                    state = self.start;
                }
            }
        }
    }

    /// Whether one line (with no newline in it) contains a match.
    pub fn is_match(&mut self, line: &[u8]) -> bool {
        if line.is_empty() {
            // next_line sees no line at all in empty data.
            let (state, end_class) = (self.start, self.stride - 1);
            let mut r = self.table[state as usize + end_class];
            if r == UNKNOWN {
                r = self.step(state, end_class);
            }
            return r == MATCH;
        }
        self.next_line(line, 0).is_some()
    }
}
