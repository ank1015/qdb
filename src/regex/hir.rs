//! rg's pattern syntax. The regex-syntax crate parses a pattern and says
//! what it means, with its Unicode tables; here that meaning is turned
//! into this engine's own pattern tree, over bytes, and matched by the
//! engine's own matchers.

use super::{set_of, Assert, MatchError, Node, Regex};
use regex_syntax::ast::{self, Ast};
use regex_syntax::hir::{Class, Hir, HirKind, Look};
use regex_syntax::utf8::Utf8Sequences;

/// How rg was asked to read its pattern.
#[derive(Debug, Clone, Copy, Default)]
pub struct RgSyntax {
    pub icase: bool,
    /// -S: ignore case unless the pattern has an upper case letter.
    pub smart: bool,
    pub fixed: bool,
    pub word: bool,
    pub line: bool,
}

/// A compiled rg pattern. `ascii` is exact for a line of ASCII bytes;
/// `exact`, there when the pattern asks about Unicode word boundaries,
/// is for the lines that hold other bytes.
#[derive(Clone)]
pub struct RgPattern {
    pub ascii: Regex,
    pub exact: Option<Regex>,
}

pub const NEWLINE: &str = "the literal \"\\n\" is not allowed in a regex\n\nConsider enabling multiline mode with the --multiline flag (or -U for short).\nWhen multiline mode is enabled, new line characters can be matched.";

/// Whether a pattern has a literal, and whether one is upper case.
fn literals(a: &Ast, any: &mut bool, upper: &mut bool) {
    let mut note = |c: char| {
        *any = true;
        *upper |= c.is_uppercase();
    };
    fn class(set: &ast::ClassSet, note: &mut dyn FnMut(char)) {
        match set {
            ast::ClassSet::Item(item) => item_of(item, note),
            ast::ClassSet::BinaryOp(op) => {
                class(&op.lhs, note);
                class(&op.rhs, note);
            }
        }
    }
    fn item_of(item: &ast::ClassSetItem, note: &mut dyn FnMut(char)) {
        match item {
            ast::ClassSetItem::Literal(l) => note(l.c),
            ast::ClassSetItem::Range(r) => {
                note(r.start.c);
                note(r.end.c);
            }
            ast::ClassSetItem::Bracketed(b) => class(&b.kind, note),
            ast::ClassSetItem::Union(u) => u.items.iter().for_each(|i| item_of(i, note)),
            _ => {}
        }
    }
    match a {
        Ast::Literal(l) => note(l.c),
        Ast::ClassBracketed(b) => class(&b.kind, &mut note),
        Ast::Repetition(r) => literals(&r.ast, any, upper),
        Ast::Group(g) => literals(&g.ast, any, upper),
        Ast::Alternation(x) => x.asts.iter().for_each(|a| literals(a, any, upper)),
        Ast::Concat(x) => x.asts.iter().for_each(|a| literals(a, any, upper)),
        _ => {}
    }
}

struct Build {
    /// Give Unicode word assertions their own kind, to be decided by
    /// decoding characters; otherwise they are the ASCII ones.
    unicode: bool,
    saw_unicode: bool,
}

impl Build {
    fn node(&mut self, h: &Hir) -> Result<Node, String> {
        Ok(match h.kind() {
            HirKind::Empty => Node::Empty,
            HirKind::Literal(l) => {
                if l.0.contains(&b'\n') {
                    return Err(NEWLINE.into());
                }
                Node::Cat(l.0.iter().map(|b| Node::Set(set_of(|c| c == *b))).collect())
            }
            HirKind::Class(Class::Bytes(c)) => {
                let ranges: Vec<(u8, u8)> = c.ranges().iter().map(|r| (r.start(), r.end())).collect();
                Node::Set(set_of(|b| ranges.iter().any(|(lo, hi)| *lo <= b && b <= *hi)))
            }
            HirKind::Class(Class::Unicode(c)) => {
                let mut seqs: Vec<Vec<(u8, u8)>> = Vec::new();
                for r in c.ranges() {
                    for s in Utf8Sequences::new(r.start(), r.end()) {
                        seqs.push(s.as_slice().iter().map(|b| (b.start, b.end)).collect());
                    }
                }
                let refs: Vec<&[(u8, u8)]> = seqs.iter().map(|s| &s[..]).collect();
                trie(&refs)
            }
            HirKind::Look(l) => Node::Assert(self.look(*l)),
            HirKind::Repetition(r) => Node::Repeat { sub: Box::new(self.node(&r.sub)?), min: r.min, max: r.max },
            HirKind::Capture(c) => self.node(&c.sub)?,
            HirKind::Concat(v) => Node::Cat(v.iter().map(|h| self.node(h)).collect::<Result<_, _>>()?),
            HirKind::Alternation(v) => Node::Alt(v.iter().map(|h| self.node(h)).collect::<Result<_, _>>()?),
        })
    }

    fn look(&mut self, l: Look) -> Assert {
        let mut uni = |ascii: Assert, unicode: Assert| {
            self.saw_unicode = true;
            if self.unicode {
                unicode
            } else {
                ascii
            }
        };
        match l {
            // A line is searched on its own, so the text is the line.
            Look::Start | Look::StartLF | Look::StartCRLF => Assert::LineStart,
            Look::End | Look::EndLF | Look::EndCRLF => Assert::LineEnd,
            Look::WordAscii => Assert::WordBoundary,
            Look::WordAsciiNegate => Assert::NotWordBoundary,
            Look::WordStartAscii => Assert::WordStart,
            Look::WordEndAscii => Assert::WordEnd,
            Look::WordStartHalfAscii => Assert::AfterNonWord,
            Look::WordEndHalfAscii => Assert::BeforeNonWord,
            Look::WordUnicode => uni(Assert::WordBoundary, Assert::UWordBoundary),
            Look::WordUnicodeNegate => uni(Assert::NotWordBoundary, Assert::UNotWordBoundary),
            Look::WordStartUnicode => uni(Assert::WordStart, Assert::UWordStart),
            Look::WordEndUnicode => uni(Assert::WordEnd, Assert::UWordEnd),
            Look::WordStartHalfUnicode => uni(Assert::AfterNonWord, Assert::UAfterNonWord),
            Look::WordEndHalfUnicode => uni(Assert::BeforeNonWord, Assert::UBeforeNonWord),
        }
    }
}

/// Byte sequences, in order, as a tree that shares their first bytes.
fn trie(seqs: &[&[(u8, u8)]]) -> Node {
    let mut alts: Vec<Node> = Vec::new();
    // Every sequence of one byte goes into one set.
    let singles: Vec<(u8, u8)> = seqs.iter().filter(|s| s.len() == 1).map(|s| s[0]).collect();
    if !singles.is_empty() {
        alts.push(Node::Set(set_of(|b| singles.iter().any(|(lo, hi)| *lo <= b && b <= *hi))));
    }
    let longer: Vec<&[(u8, u8)]> = seqs.iter().copied().filter(|s| s.len() > 1).collect();
    let mut i = 0;
    while i < longer.len() {
        let first = longer[i][0];
        let mut j = i;
        while j < longer.len() && longer[j][0] == first {
            j += 1;
        }
        let rest: Vec<&[(u8, u8)]> = longer[i..j].iter().map(|s| &s[1..]).collect();
        alts.push(Node::Cat(vec![Node::Set(set_of(|b| first.0 <= b && b <= first.1)), trie(&rest)]));
        i = j;
    }
    if alts.len() == 1 {
        alts.pop().unwrap()
    } else {
        Node::Alt(alts)
    }
}

/// Compile a pattern as rg reads it. The error is rg's message.
pub fn rg_pattern(pattern: &[u8], syn: RgSyntax) -> Result<RgPattern, String> {
    let text = std::str::from_utf8(pattern).map_err(|_| "pattern is not valid UTF-8".to_string())?;
    let text = if syn.fixed { regex_syntax::escape(text) } else { text.to_string() };
    let wrapped = format!("(?:{})", text);
    let parsed = ast::parse::Parser::new().parse(&wrapped).map_err(|e| e.to_string())?;
    let (mut any, mut upper) = (false, false);
    literals(&parsed, &mut any, &mut upper);
    let icase = syn.icase || (syn.smart && any && !upper);
    let full = if syn.line {
        format!("(?m:^)(?:{})(?m:$)", text)
    } else if syn.word {
        format!("\\b{{start-half}}(?:{})\\b{{end-half}}", text)
    } else {
        wrapped
    };
    let hir = regex_syntax::ParserBuilder::new()
        .case_insensitive(icase)
        .multi_line(true)
        .utf8(false)
        .build()
        .parse(&full)
        .map_err(|e| e.to_string())?;
    let too_big = |_| "compiled regex exceeds size limit".to_string();
    let mut b = Build { unicode: false, saw_unicode: false };
    let ascii = Regex::from_ast(b.node(&hir)?, false).ok_or(()).map_err(too_big)?;
    let exact = if b.saw_unicode {
        b.unicode = true;
        Some(Regex::from_ast(b.node(&hir)?, false).ok_or(()).map_err(too_big)?)
    } else {
        None
    };
    Ok(RgPattern { ascii, exact })
}

impl RgPattern {
    /// Whether one line, given without its newline, matches.
    pub fn is_match(&self, line: &[u8]) -> Result<bool, MatchError> {
        match &self.exact {
            Some(exact) if !line.is_ascii() => exact.is_match(line),
            _ => self.ascii.is_match(line),
        }
    }
}
