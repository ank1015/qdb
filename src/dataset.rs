//! Generated datasets: trees shaped like a source repository, the same
//! every time for the same name. The benchmark and the counter baseline
//! of the test harness run on them.

use crate::Db;
use std::fmt::Write as _;

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

pub struct Spec {
    pub name: &'static str,
    pub files: usize,
    /// Average lines per file.
    pub lines: usize,
}

pub const SPECS: &[Spec] = &[
    Spec { name: "small", files: 200, lines: 120 },
    Spec { name: "medium", files: 2_000, lines: 150 },
    Spec { name: "large", files: 12_000, lines: 160 },
];

pub const SYLLABLES: &[&str] = &[
    "al", "be", "co", "da", "en", "fi", "go", "ha", "in", "jo", "ka", "le", "mo", "ne", "or", "pa", "qu", "re", "si", "to", "un", "va",
    "we", "xi", "yo", "ze", "ar", "bu", "cy", "di", "el", "fo",
];

pub const COMMON: &[&str] = &[
    "the", "of", "and", "to", "in", "is", "for", "with", "value", "result", "data", "index", "count", "name", "list", "node", "file",
    "path", "error", "return", "if", "else", "int", "char", "const", "static", "void", "alpha_one", "beta_two", "handle", "buffer",
];

pub fn vocabulary(rng: &mut Rng) -> Vec<String> {
    let mut words: Vec<String> = COMMON.iter().map(|w| w.to_string()).collect();
    for _ in 0..3000 {
        let n = 2 + rng.below(3);
        let mut w = String::new();
        for _ in 0..n {
            w.push_str(SYLLABLES[rng.below(SYLLABLES.len())]);
        }
        words.push(w);
    }
    words
}

/// Pick a word, favouring the start of the list (a rough Zipf shape, as in
/// real text: a few words are very common, most are rare).
pub fn word<'a>(rng: &mut Rng, vocab: &'a [String]) -> &'a str {
    let r = rng.below(1000);
    let idx = if r < 500 { rng.below(COMMON.len()) } else if r < 850 { rng.below(300) } else { rng.below(vocab.len()) };
    &vocab[idx]
}

pub fn file_content(rng: &mut Rng, vocab: &[String], ext: &str, lines: usize) -> Vec<u8> {
    let mut out = String::new();
    let n = lines / 2 + rng.below(lines + 1);
    match ext {
        "c" | "h" => {
            for _ in 0..1 + rng.below(4) {
                let _ = writeln!(out, "#include \"{}.h\"", word(rng, vocab));
            }
        }
        "py" => {
            for _ in 0..1 + rng.below(4) {
                let _ = writeln!(out, "import {}", word(rng, vocab));
            }
        }
        "md" => {
            let _ = writeln!(out, "# {} {}", word(rng, vocab), word(rng, vocab));
        }
        _ => {}
    }
    for _ in 0..n {
        if rng.chance(12) {
            out.push('\n');
            continue;
        }
        let code = matches!(ext, "c" | "h" | "py");
        if code {
            for _ in 0..rng.below(4) {
                out.push_str("    ");
            }
        }
        if rng.chance(150) {
            let _ = writeln!(out, "// TODO: {} {}", word(rng, vocab), word(rng, vocab));
            continue;
        }
        if rng.chance(900) {
            let _ = writeln!(out, "// FIXME {}", word(rng, vocab));
            continue;
        }
        if code && rng.chance(40) {
            let _ = writeln!(out, "return {};", word(rng, vocab));
            continue;
        }
        if code && rng.chance(60) {
            let _ = writeln!(out, "handle_request({}, {});", word(rng, vocab), rng.below(100_000));
            continue;
        }
        if rng.chance(70) {
            let _ = writeln!(out, "error: {} {}", word(rng, vocab), rng.below(10_000));
            continue;
        }
        let words = 2 + rng.below(10);
        for i in 0..words {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(word(rng, vocab));
            if code && rng.chance(8) {
                let _ = write!(out, "_{}(", word(rng, vocab));
            }
            if rng.chance(25) {
                let _ = write!(out, " {}", rng.below(1_000_000));
            }
        }
        if code && rng.chance(3) {
            out.push(';');
        }
        out.push('\n');
    }
    // Some files have no final newline, as in real repositories.
    if rng.chance(20) && out.ends_with('\n') {
        out.pop();
    }
    out.into_bytes()
}

/// Build a dataset: a tree shaped like a source repository. The same name
/// always gives the same tree.
pub fn generate(spec: &Spec) -> Db {
    let mut rng = Rng(0x5EED_0000 + spec.files as u64);
    let vocab = vocabulary(&mut rng);
    let mut db = Db::new();
    let put = |db: &mut Db, path: &str, content: Vec<u8>| {
        let mut cur = db.root();
        let comps: Vec<&str> = path.split('/').collect();
        for (i, comp) in comps.iter().enumerate() {
            cur = match db.child(cur, comp.as_bytes()) {
                Some(c) => c,
                None if i + 1 == comps.len() => db.create_file(cur, comp.as_bytes(), &content),
                None => db.create_folder(cur, comp.as_bytes()),
            };
        }
    };
    // Paths the command list relies on.
    for (path, ext, lines) in [
        ("README.md", "md", 60),
        ("src/main.c", "c", 400),
        ("docs/guide.md", "md", 200),
        ("config/settings.json", "json", 40),
        (".gitignore", "txt", 5),
    ] {
        let c = file_content(&mut rng, &vocab, ext, lines);
        put(&mut db, path, c);
    }
    const TOPS: &[(&str, &[&str])] = &[
        ("src", &["c", "c", "c", "h"]),
        ("lib", &["c", "h", "py"]),
        ("tests", &["py", "c", "txt"]),
        ("docs", &["md", "md", "txt"]),
        ("scripts", &["py", "txt"]),
        ("config", &["json", "txt"]),
    ];
    for i in 0..spec.files {
        let (top, exts) = TOPS[match rng.below(100) {
            0..=44 => 0,
            45..=64 => 1,
            65..=79 => 2,
            80..=91 => 3,
            92..=96 => 4,
            _ => 5,
        }];
        let ext = exts[rng.below(exts.len())];
        let mut path = top.to_string();
        // Up to four levels of sub-folders; fewer distinct folders near
        // the top.
        let depth = rng.below(5);
        for level in 0..depth {
            let fanout = 2 + (spec.files / 400).min(10) + level;
            let _ = write!(path, "/mod_{:02}", rng.below(fanout));
        }
        let hidden = if rng.chance(60) { "." } else { "" };
        let _ = write!(path, "/{}{}_{}.{}", hidden, word(&mut rng, &vocab), i, ext);
        let c = file_content(&mut rng, &vocab, ext, spec.lines);
        put(&mut db, &path, c);
    }
    db
}

