//! Generated tests: for each family of commands, a tree and command
//! lines made of random inputs and random combinations of the supported
//! flags, to be run on the real tools and on the engine and compared.

/// A generator of numbers that gives the same run for the same seed.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    pub fn pick<'a, T: ?Sized>(&mut self, of: &[&'a T]) -> &'a T {
        of[self.below(of.len())]
    }

    pub fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

/// What one family of generated tests runs on and runs.
pub struct Generated {
    /// The tree, in the form of a fixture file.
    pub fixture: String,
    /// Paths whose time is set: seconds before the time of the run.
    pub ages: Vec<(String, i64)>,
    pub commands: Vec<String>,
}

pub const FAMILIES: &[&str] = &["shell", "grep", "sed", "find", "rg", "text", "ls", "xargs", "scripts", "diff"];

const WORDS: &[&str] = &[
    "alpha", "beta", "gamma", "delta", "Alpha", "BETA", "needle", "Needle", "hay", "stack", "foo", "bar", "baz", "x1", "y22", "z333", "the", "a", "of", "to", "naïve", "café", "πr", "日本", "Ünï", "end",
];

fn word(rng: &mut Rng) -> &'static str {
    WORDS[rng.below(WORDS.len())]
}

fn ascii_word(rng: &mut Rng) -> &'static str {
    WORDS[rng.below(20)]
}

fn line(rng: &mut Rng) -> String {
    let n = rng.below(6);
    let mut s = String::new();
    for i in 0..n {
        if i > 0 {
            s.push_str(rng.pick(&[" ", " ", "  ", ":", ",", "\t", " = ", "."]));
        }
        s.push_str(word(rng));
        if rng.chance(8) {
            s.push_str(&rng.below(3000).to_string());
        }
    }
    s
}

fn text(rng: &mut Rng, lines: usize) -> Vec<String> {
    (0..lines).map(|_| line(rng)).collect()
}

fn fixture_file(out: &mut String, path: &str, lines: &[String]) {
    out.push_str(&format!("F {}\n", path));
    for l in lines {
        out.push_str(&format!("|{}\n", l));
    }
}

/// A tree of text files a few folders deep. Returns the files and the
/// folders.
fn tree(rng: &mut Rng, out: &mut String) -> (Vec<String>, Vec<String>) {
    let dirs = ["", "src/", "src/lib/", "docs/", "docs/old/", "data/"];
    let names = ["a.txt", "b.txt", "main.rs", "util.rs", "notes.md", "x.log", "Makefile", "data.csv", "c.TXT", "long name.txt"];
    let mut files = Vec::new();
    for d in dirs {
        for n in names {
            if rng.chance(3) {
                let path = format!("{}{}", d, n);
                let count = if rng.chance(6) { 0 } else { 1 + rng.below(12) };
                fixture_file(out, &path, &text(rng, count));
                files.push(path);
            }
        }
    }
    // Always something at the top to name.
    for n in ["one.txt", "two.txt"] {
        fixture_file(out, n, &text(rng, 8));
        files.push(n.to_string());
    }
    out.push_str("D empty\n");
    let folders = dirs.iter().filter(|d| !d.is_empty() && files.iter().any(|f| f.starts_with(*d))).map(|d| d.trim_end_matches('/').to_string()).collect();
    (files, folders)
}

fn q(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn some<'a>(rng: &mut Rng, of: &'a [String]) -> &'a str {
    &of[rng.below(of.len())]
}

/// A pattern for grep and sed: a word, a piece of one, or a small
/// expression in basic or extended syntax.
fn pattern(rng: &mut Rng, extended: bool) -> String {
    let w = ascii_word(rng);
    match rng.below(9) {
        0 => w.to_string(),
        1 => w[..w.len().min(2)].to_string(),
        2 => format!("^{}", w),
        3 => format!("{}$", w),
        4 => format!("{}.*{}", &w[..1], ascii_word(rng)),
        5 => "[0-9][0-9]*".to_string(),
        6 if extended => format!("({}|{})", w, ascii_word(rng)),
        6 => format!("\\({}\\|{}\\)", w, ascii_word(rng)),
        7 if extended => format!("{}+", &w[..1]),
        7 => format!("[{}]", &w[..w.len().min(3)]),
        _ => "^$".to_string(),
    }
}

fn shell(rng: &mut Rng, files: &[String], n: usize) -> Vec<String> {
    let simple = |rng: &mut Rng| -> String {
        let f = some(rng, files).to_string();
        match rng.below(14) {
            0 => format!("echo {} {}", ascii_word(rng), ascii_word(rng)),
            1 => format!("cat {}", q(&f)),
            2 => format!("grep -c {} {}", ascii_word(rng), q(&f)),
            3 => format!("wc -l {}", q(&f)),
            4 => "true".to_string(),
            5 => "false".to_string(),
            6 => format!("cat nope{}", rng.below(3)),
            7 => format!("echo {{{},{}}}{{1..{}}}", ascii_word(rng), ascii_word(rng), 1 + rng.below(3)),
            8 => format!("echo x{{a,{{b,c}}}}y {{{}..{}..{}}} {{0{}..10}}", rng.below(5), 5 + rng.below(9), 1 + rng.below(3), 1 + rng.below(8)),
            9 => format!("echo $(wc -l < {}) `echo {}`", q(&f), ascii_word(rng)),
            10 => format!("echo \"$(head -n 2 {})\"", q(&f)),
            11 => format!("printf '%s\\n' {{a..{}}}", ["c", "f", "k"][rng.below(3)]),
            12 => format!("test -f {}", q(&f)),
            _ => format!("head -n {} {}", 1 + rng.below(4), q(&f)),
        }
    };
    let redirect = |rng: &mut Rng| -> &'static str {
        *rng.pick(&[
            &"", &"", &"", &" > out", &" >> out", &" 2> err", &" 2>> err", &" 2>&1", &" > out 2>&1", &" 2>&1 > out", &" &> both", &" &>> both", &" >&2", &" > /dev/null", &" 2> /dev/null", &" < one.txt",
            &" < /dev/null",
        ])
    };
    let mut out = Vec::new();
    for _ in 0..n {
        let mut s = String::new();
        let parts = 1 + rng.below(4);
        let joiners: Vec<&str> = (1..parts).map(|_| *rng.pick(&[&" ; ", &" && ", &" || ", &" | ", &"\n"])).collect();
        for i in 0..parts {
            if i > 0 {
                s.push_str(joiners[i - 1]);
            }
            // Two commands of one pipeline writing one file, or both
            // writing errors, would race: only the last one may.
            let piped = i > 0 && joiners[i - 1] == " | ";
            let feeds = joiners.get(i) == Some(&" | ");
            if rng.chance(9) {
                // A here-document.
                let (mark, body) = match rng.below(4) {
                    0 => ("<<EOF", "plain $(echo sub) line\n\ttabbed\n"),
                    1 => ("<<'EOF'", "kept $(echo sub) `x`\n"),
                    2 => ("<<\"EOF\"", "kept too $(echo sub)\n"),
                    _ => ("<<-EOF", "\tstripped\n\t\ttwice\n"),
                };
                let cmd = *rng.pick(&[&"cat", &"wc -l", &"grep -c e", &"cat -n"]);
                let to = if rng.chance(3) && !piped { " > out" } else { "" };
                s.push_str(&format!("{} {}{}\n{}EOF\n", cmd, mark, to, body));
                s.push_str("true");
                continue;
            }
            let cmd = simple(rng);
            let r = redirect(rng);
            if feeds {
                let quiet = if cmd.starts_with("cat nope") { *rng.pick(&[&" 2>&1", &" 2> /dev/null"]) } else { "" };
                s.push_str(&cmd);
                s.push_str(if quiet.is_empty() && matches!(r, " 2>&1" | " 2> /dev/null" | " < one.txt" | " < /dev/null") { r } else { quiet });
            } else {
                s.push_str(&cmd);
                if !piped || !(r.contains("out") || r.contains("err") || r.contains("both")) {
                    s.push_str(r);
                }
            }
        }
        if rng.chance(12) {
            s = format!("echo one \\\n two; {}", s);
        }
        out.push(s);
    }
    out
}

fn grep(rng: &mut Rng, files: &[String], folders: &[String], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        let syntax = rng.below(4);
        let extended = syntax == 1;
        let mut flags: Vec<String> = Vec::new();
        match syntax {
            1 => flags.push("-E".into()),
            2 => flags.push("-F".into()),
            3 if rng.chance(3) => flags.push("-G".into()),
            _ => {}
        }
        for f in ["-i", "-v", "-n", "-o", "-w", "-x", "-c", "-l", "-L", "-h", "-H", "-q", "-s"] {
            if rng.chance(6) {
                flags.push(f.into());
            }
        }
        if rng.chance(4) {
            flags.push(format!("-{} {}", rng.pick(&["A", "B", "C"]), rng.below(4)));
        }
        if rng.chance(12) {
            flags.push(format!("-{}", 1 + rng.below(3)));
        }
        if rng.chance(6) {
            flags.push(format!("-m {}", 1 + rng.below(3)));
        }
        let pat = |rng: &mut Rng| if syntax == 2 { ascii_word(rng).to_string() } else { pattern(rng, extended) };
        let mut patterns = String::new();
        if rng.chance(4) {
            for _ in 0..1 + rng.below(3) {
                patterns.push_str(&format!(" -e {}", q(&pat(rng))));
            }
        } else {
            patterns = format!(" {}", q(&pat(rng)));
        }
        let targets = if rng.chance(3) {
            let mut t = format!(" -r{}", if rng.chance(4) { " --include='*.txt'" } else if rng.chance(5) { " --exclude='*.rs'" } else if rng.chance(6) { " --exclude-dir=lib" } else { "" });
            t.push_str(&format!(" {}", if rng.chance(2) || folders.is_empty() { ".".to_string() } else { some(rng, folders).to_string() }));
            t
        } else {
            (0..1 + rng.below(3)).map(|_| format!(" {}", q(some(rng, files)))).collect()
        };
        out.push(format!("grep {}{}{}", flags.join(" "), patterns, targets));
    }
    out
}

fn sed(rng: &mut Rng, files: &[String], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        let extended = rng.chance(3);
        let re = |rng: &mut Rng| pattern(rng, extended).replace('/', ".");
        let addr = |rng: &mut Rng| -> String {
            let one = |rng: &mut Rng| match rng.below(4) {
                0 => (1 + rng.below(6)).to_string(),
                1 => "$".to_string(),
                2 => format!("/{}/", re(rng)),
                _ => format!("\\%{}%", re(rng).replace('%', ".")),
            };
            match rng.below(9) {
                0 | 1 => String::new(),
                2 | 3 => one(rng),
                4 => format!("{},{}", one(rng), one(rng)),
                5 => format!("{},+{}", one(rng), rng.below(3)),
                6 => format!("{}~{}", rng.below(3), 1 + rng.below(3)),
                7 => format!("0,/{}/", re(rng)),
                _ => format!("{}!", one(rng)),
            }
        };
        let command = |rng: &mut Rng| -> String {
            match rng.below(13) {
                0 => "p".into(),
                1 => "d".into(),
                2 => format!("s/{}/<&>/{}", re(rng), rng.pick(&["", "g", "p", "2", "i", "I", "gI"])),
                3 if extended => "s/(a)([a-z])/\\2\\1[\\0]/g".into(),
                3 => "s/\\(a\\)\\([a-z]\\)/\\2\\1[\\0]/g".into(),
                4 => "q".into(),
                5 => "Q".into(),
                6 => "=".into(),
                7 => "a\\\nadded".into(),
                8 => "i\\\ninserted".into(),
                9 => "c\\\nchanged".into(),
                10 => "y/abc/xyz/".into(),
                11 => format!("s/{}//", re(rng)),
                _ => "p".into(),
            }
        };
        let one = |rng: &mut Rng| -> String {
            let a = addr(rng);
            if rng.chance(7) {
                format!("{}{{{};{}\n}}", a, command(rng), command(rng))
            } else {
                format!("{}{}", a, command(rng))
            }
        };
        let mut script = one(rng);
        let mut flags = String::new();
        if rng.chance(3) {
            flags.push_str(" -n");
        }
        if extended {
            flags.push_str(rng.pick(&[" -E", " -r"]));
        }
        let mut scripts = String::new();
        if rng.chance(4) {
            scripts = format!(" -e {} -e {}", q(&script), q(&one(rng)));
        } else {
            if rng.chance(4) {
                script = format!("{}\n{}", script, one(rng));
            }
            scripts.push_str(&format!(" {}", q(&script)));
        }
        let many = rng.chance(3);
        if many && rng.chance(2) {
            flags.push_str(" -s");
        }
        if rng.chance(8) {
            flags.push_str(rng.pick(&[" -i", " -i.bak"]));
        }
        let mut targets = format!(" {}", q(some(rng, files)));
        if many {
            targets.push_str(&format!(" {}", q(some(rng, files))));
        }
        out.push(format!("sed{}{}{}", flags, scripts, targets));
    }
    out
}

fn find(rng: &mut Rng, files: &[String], folders: &[String], n: usize) -> Vec<String> {
    fn test(rng: &mut Rng, files: &[String], depth: usize) -> String {
        let glob = |rng: &mut Rng| *rng.pick(&[&"*.txt", &"*.rs", &"a*", &"*o*", &"?.txt", &"[a-m]*", &"*.TXT", &"M*", &"docs", &"lib", &"*"]);
        match rng.below(if depth > 2 { 12 } else { 17 }) {
            0 => format!("-name {}", q(glob(rng))),
            1 => format!("-iname {}", q(glob(rng))),
            2 => format!("-path {}", q(rng.pick(&["./src/*", "*/lib/*", "./docs*", "*.md", "*a*"]))),
            3 => format!("-ipath {}", q(rng.pick(&["./SRC/*", "*/LIB/*", "*.MD"]))),
            4 => format!("-type {}", rng.pick(&["f", "d"])),
            5 => format!("\\( -type f -size {}{}{} \\)", rng.pick(&["+", "-", ""]), rng.below(3), rng.pick(&["k", "c", ""])),
            6 => format!("-newer {}", q(some(rng, files))),
            7 => format!("-mmin {}{}", rng.pick(&["+", "-", ""]), 10 * (1 + rng.below(9))),
            8 => format!("-mtime {}{}", rng.pick(&["+", "-", ""]), rng.below(4)),
            9 => "-empty".to_string(),
            10 => format!("-name {} -prune", q(rng.pick(&["lib", "docs", "old", "src"]))),
            11 => "-print".to_string(),
            12 => format!("{} {}", rng.pick(&["!", "-not"]), test(rng, files, depth + 1)),
            13 => format!("{} {} {}", test(rng, files, depth + 1), rng.pick(&["-a", "-and", ""]), test(rng, files, depth + 1)),
            14 => format!("{} {} {}", test(rng, files, depth + 1), rng.pick(&["-o", "-or"]), test(rng, files, depth + 1)),
            _ => format!("\\( {} \\)", test(rng, files, depth + 1)),
        }
    }
    let mut out = Vec::new();
    for _ in 0..n {
        let start = if rng.chance(3) && !folders.is_empty() { some(rng, folders).to_string() } else { ".".to_string() };
        let mut s = format!("find {}", start);
        if rng.chance(5) {
            s.push_str(&format!(" -maxdepth {}", rng.below(3)));
        }
        if rng.chance(6) {
            s.push_str(&format!(" -mindepth {}", rng.below(3)));
        }
        s.push(' ');
        s.push_str(&test(rng, files, 0));
        match rng.below(10) {
            0 if !s.contains("-print") => s.push_str(" -print0"),
            1 => s.push_str(" -type f -exec wc -l {} \\;"),
            2 => s.push_str(" -type f -exec grep -c a {} +"),
            3 => s.push_str(" -exec echo found {} \\;"),
            _ => {}
        }
        out.push(s);
    }
    out
}

/// A tree for rg: ignore files at several levels, hidden names, and
/// text that is not ASCII.
fn rg_tree(rng: &mut Rng, out: &mut String) -> Vec<String> {
    let (mut files, _) = tree(rng, out);
    let pool = ["*.log", "x.log", "/data", "docs/old/", "*.rs", "!util.rs", "notes.md", "src/lib", "**/old/*.txt", "a.*", "!a.txt", "Makefile", "*.csv", "/one.txt", "lib/", "c.TXT"];
    let ignore = |rng: &mut Rng, path: &str, out: &mut String| {
        let lines: Vec<String> = (0..1 + rng.below(3)).map(|_| pool[rng.below(pool.len())].to_string()).collect();
        fixture_file(out, path, &lines);
    };
    if rng.chance(2) {
        fixture_file(out, ".git/HEAD", &["ref: refs/heads/main".to_string()]);
        ignore(rng, ".gitignore", out);
        if rng.chance(2) {
            ignore(rng, "src/.gitignore", out);
        }
        if rng.chance(3) {
            ignore(rng, ".git/info/exclude", out);
        }
    } else if rng.chance(2) {
        ignore(rng, ".gitignore", out);
    }
    for (path, one_in) in [(".ignore", 2), ("src/.ignore", 3), ("docs/.ignore", 3), (".rgignore", 3), ("docs/old/.rgignore", 4)] {
        if rng.chance(one_in) {
            ignore(rng, path, out);
        }
    }
    for hidden in [".hidden.txt", "src/.secret.rs", ".config/settings.txt"] {
        fixture_file(out, hidden, &text(rng, 4));
        files.push(hidden.to_string());
    }
    files
}

fn rg(rng: &mut Rng, files: &[String], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        let mut flags: Vec<String> = Vec::new();
        for f in ["-i", "-S", "-s", "-w", "-x", "-v", "-c", "-l", "-n", "-N", "--no-heading", "--hidden", "--no-ignore", "-u", "-uu", "-uuu", "--null", "-0", "-j 2"] {
            if rng.chance(10) {
                flags.push(f.into());
            }
        }
        if rng.chance(5) {
            flags.push(format!("-{} {}", rng.pick(&["A", "B", "C"]), rng.below(3)));
        }
        if rng.chance(8) {
            flags.push(format!("-m {}", 1 + rng.below(3)));
        }
        if rng.chance(5) {
            flags.push(format!("-g {}", q(rng.pick(&["*.txt", "!*.rs", "src/**", "*.{rs,md}", "!docs/", "/one.txt", "**/a.*", "[a-c]*"]))));
        }
        if rng.chance(6) {
            flags.push(format!("-{} {}", rng.pick(&["t", "T"]), rng.pick(&["txt", "rust", "md", "c", "py"])));
        }
        let fixed = rng.chance(5);
        if fixed {
            flags.push("-F".into());
        }
        let w = word(rng);
        let pat = if fixed {
            w.to_string()
        } else {
            match rng.below(12) {
                0 => w.to_string(),
                1 => format!("\\b{}\\b", w),
                2 => "\\w+é".to_string(),
                3 => "\\p{Greek}".to_string(),
                4 => "[[:upper:]]\\w*".to_string(),
                5 => format!("(?i){}", w),
                6 => "\\d{2,}".to_string(),
                7 => format!("{}|{}", w, word(rng)),
                8 => "^\\s*$".to_string(),
                9 => "\\p{Han}+".to_string(),
                10 => format!("{}.?{}", w.chars().next().unwrap(), ascii_word(rng)),
                _ => "[^\\x00-\\x7F]".to_string(),
            }
        };
        let sorted = rng.chance(2);
        if sorted {
            flags.push("--sort path".into());
        }
        let mut s = if rng.chance(12) { format!("rg --files {}", flags.iter().filter(|f| !f.starts_with("-c") && !f.starts_with("-l")).cloned().collect::<Vec<_>>().join(" ")) } else { format!("rg {} {}", flags.join(" "), q(&pat)) };
        match rng.below(6) {
            0 => s.push_str(&format!(" {}", q(some(rng, files)))),
            1 => s.push_str(rng.pick(&[" src", " docs", " data", " docs/old", " src/lib", " ."])),
            _ => {}
        }
        out.push(s);
    }
    out
}

fn text_tools(rng: &mut Rng, files: &[String], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        let f = q(some(rng, files));
        let g = q(some(rng, files));
        let s = match rng.below(9) {
            0 | 1 => {
                let mut flags: Vec<String> = Vec::new();
                for x in ["-u", "-r", "-f", "-b", "-s"] {
                    if rng.chance(5) {
                        flags.push(x.into());
                    }
                }
                if rng.chance(4) {
                    flags.push(rng.pick(&["-n", "-h"]).to_string());
                }
                if rng.chance(3) {
                    flags.push(format!("-t {}", q(rng.pick(&[":", ",", " ", "."]))));
                }
                for _ in 0..rng.below(3) {
                    let a = 1 + rng.below(3);
                    let mut k = format!("-k {}", a);
                    if rng.chance(3) {
                        k.push_str(&format!(".{}", 1 + rng.below(3)));
                    }
                    if rng.chance(3) {
                        k.push_str(rng.pick(&["n", "r", "f", "b", "nr", "bf"]));
                    }
                    if rng.chance(2) {
                        k.push_str(&format!(",{}", a + rng.below(2)));
                        if rng.chance(4) {
                            k.push_str(&format!(".{}", rng.below(4)));
                        }
                        if rng.chance(5) {
                            k.push_str(rng.pick(&["n", "r", "b"]));
                        }
                    }
                    flags.push(k);
                }
                if rng.chance(12) {
                    flags.push("-c".into());
                    format!("sort {} {}", flags.join(" "), f)
                } else if rng.chance(3) {
                    format!("sort {} {} {}", flags.join(" "), f, g)
                } else {
                    format!("sort {} {}", flags.join(" "), f)
                }
            }
            2 => {
                let mut flags: Vec<String> = Vec::new();
                for x in ["-c", "-d", "-u", "-i"] {
                    if rng.chance(4) {
                        flags.push(x.into());
                    }
                }
                if rng.chance(4) {
                    flags.push(format!("-f {}", rng.below(3)));
                }
                if rng.chance(4) {
                    flags.push(format!("-s {}", rng.below(4)));
                }
                if rng.chance(2) {
                    format!("sort {} | uniq {}", f, flags.join(" "))
                } else {
                    format!("cut -c 1-{} {} | uniq {}", 1 + rng.below(3), f, flags.join(" "))
                }
            }
            3 => {
                let list = |rng: &mut Rng| -> String {
                    (0..1 + rng.below(3))
                        .map(|_| {
                            let a = 1 + rng.below(5);
                            match rng.below(4) {
                                0 => a.to_string(),
                                1 => format!("{}-{}", a, a + rng.below(4)),
                                2 => format!("{}-", a),
                                _ => format!("-{}", a),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                };
                let mut s = match rng.below(3) {
                    0 => format!("cut -b {}", list(rng)),
                    1 => format!("cut -c {}", list(rng)),
                    _ => format!("cut -f {} -d {}{}", list(rng), q(rng.pick(&[" ", ":", ",", "a", "."])), if rng.chance(3) { " -s" } else { "" }),
                };
                if rng.chance(4) {
                    s.push_str(" --complement");
                }
                format!("{} {}", s, f)
            }
            4 => {
                let set = |rng: &mut Rng| *rng.pick(&[&"a-z", &"A-Z", &"abc", &"[:lower:]", &"[:upper:]", &"[:digit:]", &"[:space:]", &"aeiou", &" ", &"\\n", &"a-f0-9", &"[:punct:]", &"[:alpha:]"]);
                match rng.below(7) {
                    0 => format!("tr {} {} < {}", q(rng.pick(&["a-z", "abc", "[:lower:]", "aeiou", " ", "a-f"])), q(rng.pick(&["A-Z", "xyz", "[:upper:]", "_", "X", "A-F", "12"])), f),
                    1 => format!("tr -d {} < {}", q(set(rng)), f),
                    2 => format!("tr -s {} < {}", q(set(rng)), f),
                    3 => format!("tr -cd {} < {}", q(&format!("{}\\n", set(rng))), f),
                    4 => format!("tr -c {} {} < {}", q(&format!("{}\\n", set(rng))), q(rng.pick(&["_", "x", "."])), f),
                    5 => format!("tr -t {} {} < {}", q("abcdef"), q(rng.pick(&["xy", "12345", "Q"])), f),
                    _ => format!("tr -ds {} {} < {}", q(set(rng)), q(set(rng)), f),
                }
            }
            5 => {
                let mut flags: Vec<String> = Vec::new();
                if rng.chance(2) {
                    flags.push(format!("-b {}", rng.pick(&["a", "t"])));
                }
                if rng.chance(3) {
                    flags.push(format!("-v {}", rng.below(200) as i64 - 20));
                }
                if rng.chance(3) {
                    flags.push(format!("-w {}", 1 + rng.below(8)));
                }
                if rng.chance(3) {
                    flags.push(format!("-s {}", q(rng.pick(&[" ", ": ", "", "\t", "--"]))));
                }
                if rng.chance(3) {
                    flags.push(format!("-n {}", rng.pick(&["ln", "rn", "rz"])));
                }
                format!("nl {} {}", flags.join(" "), f)
            }
            6 => {
                let mut flags: Vec<String> = Vec::new();
                if rng.chance(4) {
                    flags.push(format!("-s {}", q(rng.pick(&[",", " ", ""]))));
                }
                if rng.chance(4) {
                    flags.push("-w".into());
                } else if rng.chance(5) {
                    flags.push(format!("-f {}", q(rng.pick(&["%g", "%03g", "%.2f", "%e", "n%gx"]))));
                }
                let num = |rng: &mut Rng| -> String {
                    match rng.below(4) {
                        0 => format!("-{}", rng.below(12)),
                        1 => format!("{}.{}", rng.below(12), rng.below(10)),
                        _ => rng.below(25).to_string(),
                    }
                };
                match rng.below(3) {
                    0 => format!("seq {} {}", flags.join(" "), rng.below(15)),
                    1 => format!("seq {} {} {}", flags.join(" "), num(rng), num(rng)),
                    _ => format!("seq {} {} {} {}", flags.join(" "), num(rng), *rng.pick(&[&"1", &"2", &"-1", &"-3", &"0.5", &"-0.25", &"3"]), num(rng)),
                }
            }
            7 => {
                let conv = |rng: &mut Rng| -> String {
                    let flags = *rng.pick(&[&"", &"", &"-", &"0", &"+", &" ", &"#"]);
                    let width = if rng.chance(2) { rng.below(9).to_string() } else { String::new() };
                    let prec = if rng.chance(3) { format!(".{}", rng.below(5)) } else { String::new() };
                    let c = *rng.pick(&[&"s", &"d", &"i", &"u", &"f", &"e", &"g", &"x", &"X", &"o", &"c", &"b", &"E", &"G"]);
                    format!("%{}{}{}{}", flags, width, prec, c)
                };
                let format: String = (0..1 + rng.below(3)).map(|_| format!("{}{}", conv(rng), rng.pick(&[" ", "|", "\\n", "\\t", "%%", ":"]))).collect();
                let args: String = (0..rng.below(5))
                    .map(|_| format!(" {}", q(rng.pick(&["12", "-7", "3.75", "abc", "0x1f", "010", "a\\tb", "", "1e3", "'A", "9999999999", "x y"]))))
                    .collect();
                format!("printf {}{}", q(&format!("{}\\n", format)), args)
            }
            _ => format!(
                "grep -oh {} {} {} | sort | uniq -c | sort {} | head -n {}",
                q(rng.pick(&["[a-z]*", "[A-Za-z]\\+", "[0-9]\\+"])),
                f,
                g,
                rng.pick(&["-rn", "-k1,1nr -k2", "-n"]),
                1 + rng.below(6)
            ),
        };
        out.push(s);
    }
    out
}

fn ls_du(rng: &mut Rng, files: &[String], folders: &[String], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        let target = |rng: &mut Rng| -> String {
            match rng.below(4) {
                0 => String::new(),
                1 if !folders.is_empty() => format!(" {}", some(rng, folders)),
                2 => format!(" {}", q(some(rng, files))),
                _ => " .".to_string(),
            }
        };
        let s = if rng.chance(2) {
            let mut flags = String::new();
            for x in ["a", "A", "d", "r", "S", "t", "l", "h", "R", "1"] {
                if rng.chance(4) {
                    flags.push_str(x);
                }
            }
            // The folder above the root is not in the tree: its time and
            // size are whatever the real tools find there.
            if flags.contains('a') && flags.contains(['t', 'l', 'S']) {
                flags = flags.replace('a', "A");
            }
            let flags = if flags.is_empty() { String::new() } else { format!(" -{}", flags) };
            format!("ls{}{}{}", flags, target(rng), if rng.chance(4) { target(rng) } else { String::new() })
        } else {
            let mut flags: Vec<String> = Vec::new();
            match rng.below(4) {
                0 => flags.push("-s".into()),
                1 => flags.push("-a".into()),
                2 => flags.push(format!("-d {}", rng.below(3))),
                _ => {}
            }
            for x in ["-h", "-c", "-b", "-k", "-m", "-x", "--apparent-size"] {
                if rng.chance(6) {
                    flags.push(x.into());
                }
            }
            if rng.chance(6) {
                flags.push(format!("--exclude={}", q(rng.pick(&["*.txt", "lib", "docs/old", "*.rs", "a*"]))));
            }
            format!("du {}{}{}", flags.join(" "), target(rng), if rng.chance(5) { target(rng) } else { String::new() })
        };
        out.push(s);
    }
    out
}

fn xargs(rng: &mut Rng, files: &[String], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        let names: Vec<&str> = (0..1 + rng.below(4)).map(|_| some(rng, files)).filter(|f| !f.contains(' ')).collect();
        let source = match rng.below(6) {
            0 => format!("printf '%s\\n' {}", names.join(" ")),
            1 => format!("echo {}", names.join(" ")),
            2 => "find . -name '*.txt' -type f | sort".to_string(),
            3 => format!("printf '%s\\n' {} {} {}", q(&line(rng)), q(&line(rng)), q(ascii_word(rng))),
            4 => "seq 1 12".to_string(),
            _ => "find . -type f -print0 | sort -z".to_string(),
        };
        let lists_files = matches!(source.as_bytes()[0], b'p' | b'e' | b'f') && !source.contains("' '") && !source.starts_with("printf '%s\\n' '");
        let null = source.ends_with("-z");
        let source = if null { format!("printf '%s\\0' {}", names.join(" ")) } else { source };
        let mut flags: Vec<String> = Vec::new();
        if null {
            flags.push(rng.pick(&["-0", "--null"]).to_string());
        } else if rng.chance(8) {
            flags.push("-d '\\n'".into());
        }
        let mut replace = false;
        match rng.below(7) {
            0 => flags.push(format!("-n {}", 1 + rng.below(4))),
            1 => flags.push(format!("-L {}", 1 + rng.below(3))),
            2 if !null => {
                flags.push(rng.pick(&["-I {}", "-i"]).to_string());
                replace = true;
            }
            3 => flags.push("-l".into()),
            4 => flags.push(format!("--max-args={}", 1 + rng.below(3))),
            _ => {}
        }
        for x in ["-r", "-t", "-x", "-P 1"] {
            if rng.chance(6) {
                flags.push(x.into());
            }
        }
        if rng.chance(10) && !replace {
            flags.push(format!("-s {}", 60 + rng.below(100)));
        }
        let command = if replace {
            rng.pick(&["echo '<{}>'", "echo {} and {}", "printf '%s;' {}"]).to_string()
        } else if lists_files || null {
            rng.pick(&["cat", "wc -l", "grep -c a", "ls -d", "head -n 1", "echo", "du -b", "sort -u", "basename -a"]).to_string()
        } else {
            rng.pick(&["echo", "echo pre", "printf '%s|'", "", "basename -a"]).to_string()
        };
        out.push(format!("{} | xargs {} {}", source, flags.join(" "), command));
    }
    out
}

/// Units that hold variables, arithmetic, loops and tests. Everything
/// they write goes to `out1.txt`..`out3.txt` in the folder they are in;
/// every path is one of the tree's, or one of those. The commands
/// inside a construct write to those and the construct as a
/// whole to `tN.out`, N its depth: what a construct sends to a file reaches it
/// when the construct ends, which a command inside that wrote to the
/// same file would show.
fn scripts(rng: &mut Rng, files: &[String], folders: &[String], n: usize) -> Vec<String> {
    const VARS: [&str; 5] = ["X", "Y", "N", "W", "_v1"];
    let plain: Vec<String> = files.iter().filter(|f| !f.contains(' ')).cloned().collect();
    let var = |rng: &mut Rng| VARS[rng.below(VARS.len())];
    let number = |rng: &mut Rng| match rng.below(6) {
        0 => "0".to_string(),
        1 => format!("0x{:x}", rng.below(300)),
        2 => format!("0{:o}", rng.below(70)),
        _ => rng.below(40).to_string(),
    };
    // An arithmetic expression over numbers and the variables.
    fn expr(rng: &mut Rng, depth: usize, number: &dyn Fn(&mut Rng) -> String, var: &dyn Fn(&mut Rng) -> &'static str) -> String {
        if depth == 0 || rng.chance(3) {
            return match rng.below(5) {
                0 => var(rng).to_string(),
                1 => format!("${}", var(rng)),
                _ => number(rng),
            };
        }
        let a = expr(rng, depth - 1, number, var);
        let b = expr(rng, depth - 1, number, var);
        match rng.below(16) {
            0 => format!("({})", a),
            1 => format!("-{}", a),
            2 => format!("!{}", a),
            3 => format!("~{}", a),
            4 => format!("{} ? {} : {}", a, b, number(rng)),
            5 => format!("{} ** {}", a, rng.below(5)),
            6 => format!("{} = {}", var(rng), b),
            7 => format!("{} {}= {}", var(rng), rng.pick(&["+", "-", "*", "|", "&", "^"]), b),
            8 => format!("{}{}", var(rng), rng.pick(&["++", "--"])),
            9 => format!("{}{}", rng.pick(&["++", "--"]), var(rng)),
            _ => format!("{} {} {}", a, rng.pick(&["+", "-", "*", "/", "%", "<", "<=", ">", ">=", "==", "!=", "&&", "||", "&", "|", "^", "<<", ">>", ",", "+", "*"]), b),
        }
    }
    let arith = |rng: &mut Rng| {
        let depth = 1 + rng.below(3);
        expr(rng, depth, &number, &var)
    };
    // A word for a list or an argument: literal, variable, or a form.
    let word_of = |rng: &mut Rng| -> String {
        match rng.below(16) {
            0 => format!("${}", var(rng)),
            1 => format!("\"${}\"", var(rng)),
            2 => format!("${{#{}}}", var(rng)),
            3 => format!("${{{}:-{}}}", var(rng), ascii_word(rng)),
            4 => format!("\"${{{}:=d{}}}\"", var(rng), rng.below(9)),
            5 => format!("$(({}))", arith(rng)),
            6 => format!("x${{{}}}y", var(rng)),
            7 => "$?".to_string(),
            8 => some(rng, &plain).to_string(),
            9 => format!("'{} {}'", ascii_word(rng), ascii_word(rng)),
            10 => "*.txt".to_string(),
            11 => format!("{{{},{}}}", ascii_word(rng), rng.below(9)),
            12 => "$(echo a  b)".to_string(),
            _ => ascii_word(rng).to_string(),
        }
    };
    let assign = |rng: &mut Rng| -> String {
        let v = var(rng);
        match rng.below(10) {
            0 => format!("{}=", v),
            1 => format!("{}={}", v, number(rng)),
            2 => format!("{}=$(({}))", v, arith(rng)),
            3 => format!("{}='{}  {}'", v, ascii_word(rng), ascii_word(rng)),
            4 => format!("{}={}", v, some(rng, &plain)),
            5 => format!("{}=\"${} {}\"", v, var(rng), ascii_word(rng)),
            6 => format!("{}=$(wc -l < {})", v, some(rng, &plain)),
            7 => format!("{}={}:{}:{}", v, ascii_word(rng), ascii_word(rng), rng.below(9)),
            8 => format!("unset {}", v),
            _ => format!("{}={}", v, ascii_word(rng)),
        }
    };
    let condition = |rng: &mut Rng, loop_var: &str| -> String {
        let f = some(rng, &plain).to_string();
        match rng.below(12) {
            0 => format!("[ \"${}\" = {} ]", loop_var, word_of(rng)),
            1 => format!("[ $(({})) -gt {} ]", arith(rng), rng.below(20)),
            2 => format!("grep -q {} {}", ascii_word(rng), f),
            3 => format!("[ -f \"${}\" ]", loop_var),
            4 => format!("test -d {}", if folders.is_empty() { "empty" } else { some(rng, folders) }),
            5 => format!("[ -z \"${}\" ]", var(rng)),
            6 => format!("! grep -q {} {}", ascii_word(rng), f),
            7 => "true".to_string(),
            8 => "false".to_string(),
            9 => format!("[ ${} = {} ]", var(rng), ascii_word(rng)),
            10 => format!("[ ${{#{}}} -ge 3 ] && [ -n \"${}\" ]", loop_var, var(rng)),
            _ => format!("[ \"${}\" != \"${}\" ] || false", var(rng), loop_var),
        }
    };
    let simple = |rng: &mut Rng, loop_var: &str| -> String {
        match rng.below(14) {
            0 => format!("echo \"${} ${}\"", loop_var, var(rng)),
            1 => format!("echo ${} {}", loop_var, word_of(rng)),
            2 => format!("echo {} >> out{}.txt", word_of(rng), 1 + rng.below(3)),
            3 => format!("wc -l < {}", some(rng, &plain)),
            4 => format!("grep -c {} \"./${}\"", ascii_word(rng), loop_var),
            5 => format!("head -n 1 \"./${}\"", loop_var),
            6 => assign(rng),
            7 => format!("echo $(({}))", arith(rng)),
            8 => format!("printf '%s|' {} {}; echo", word_of(rng), word_of(rng)),
            9 => "cat out1.txt".to_string(),
            10 => format!("echo \"${{#{}}}\"", loop_var),
            11 => format!("{}=$(({} + 1))", var(rng), var(rng)),
            12 => format!("{}={} echo ${}", var(rng), ascii_word(rng), var(rng)),
            _ => format!("echo {}", word_of(rng)),
        }
    };
    fn statement(
        rng: &mut Rng,
        depth: usize,
        loop_var: &str,
        in_loop: bool,
        plain: &[String],
        simple: &dyn Fn(&mut Rng, &str) -> String,
        condition: &dyn Fn(&mut Rng, &str) -> String,
        word_of: &dyn Fn(&mut Rng) -> String,
    ) -> String {
        let body = |rng: &mut Rng, lv: &str, in_loop: bool| -> String {
            let k = 1 + rng.below(3);
            (0..k).map(|_| statement(rng, depth.saturating_sub(1), lv, in_loop, plain, simple, condition, word_of)).collect::<Vec<_>>().join("; ")
        };
        if depth == 0 {
            return simple(rng, loop_var);
        }
        // A construct's own file is named by its depth, so that no
        // construct inside it writes to the same one.
        let tail = |rng: &mut Rng| -> String {
            match rng.below(11) {
                0 => " | wc -l".to_string(),
                // Not `sort`, which would put bash's messages (`bash:`)
                // and the engine's (`qdb:`) in different places; and no
                // filter that writes as it reads, whose output mixes
                // with the loop's errors as the moment has it.
                1 => " | tail -n 4".to_string(),
                2 => " | tail -n 2".to_string(),
                3 => format!(" > t{}.out", depth),
                4 => format!(" >> t{}.out", depth),
                // Not `head`: how far a loop gets before bash notices
                // that its reader has gone depends on the moment.
                5 => " 2>&1 | tail -n 3".to_string(),
                6 => " < one.txt".to_string(),
                _ => String::new(),
            }
        };
        match rng.below(16) {
            0 | 1 => {
                let v = rng.pick(&["f", "i", "X"]);
                let list = match rng.below(6) {
                    0 => "*.txt".to_string(),
                    1 => format!("$(seq {})", 1 + rng.below(6)),
                    2 => "$(ls | grep -v out)".to_string(),
                    3 => String::new(),
                    _ => (0..1 + rng.below(4)).map(|_| word_of(rng)).collect::<Vec<_>>().join(" "),
                };
                format!("for {} in {}; do {}; done{}", v, list, body(rng, v, true), tail(rng))
            }
            2 => {
                let fi = if rng.chance(2) { format!("; else {}", body(rng, loop_var, in_loop)) } else { String::new() };
                let elif = if rng.chance(5) { format!("; elif {}; then {}", condition(rng, loop_var), body(rng, loop_var, in_loop)) } else { String::new() };
                format!("if {}; then {}{}{}; fi", condition(rng, loop_var), body(rng, loop_var, in_loop), elif, fi)
            }
            // The counters of these two are named by depth and stepped
            // first, so that nothing in a body can keep a loop going.
            3 => {
                let n = rng.below(5);
                let c = format!("c{}", depth);
                format!("{0}=0; while [ ${0} -lt {1} ]; do {0}=$(({0}+1)); {2}; done{3}", c, n, body(rng, &c, true), tail(rng))
            }
            4 => {
                let n = 1 + rng.below(4);
                let c = format!("u{}", depth);
                format!("{0}={1}; until [ ${0} -le 0 ]; do {0}=$(({0}-1)); {2}; done", c, n, body(rng, &c, true))
            }
            5 => {
                let source = match rng.below(5) {
                    0 => format!("cat {}", &plain[rng.below(plain.len())]),
                    1 => "ls | grep -v out".to_string(),
                    2 => format!("head -n 3 {}", &plain[rng.below(plain.len())]),
                    3 => "printf 'a b c\\nd  e\\n\\\\x y\\nlast'".to_string(),
                    _ => format!("grep -n a {}", &plain[rng.below(plain.len())]),
                };
                let names = rng.pick(&["f", "f", "-r f", "a b", "-r a b c", "a b"]);
                let first = names.split(' ').rfind(|w| !w.starts_with('-')).unwrap();
                let ifs = if rng.chance(4) { "IFS=: " } else { "" };
                // No `< file` here: when the file is missing, which of the
                // two stages complains first is a race in bash.
                let end = Some(tail(rng)).filter(|t| !t.starts_with(" <")).unwrap_or_default();
                format!("{} | while {}read {}; do {}; done{}", source, ifs, names, body(rng, first, true), end)
            }
            6 if in_loop => format!("if {}; then {}; fi", condition(rng, loop_var), rng.pick(&["break", "continue", "break 2", "continue 2", "break", "continue"])),
            7 => format!("while read f; do {}; done < {}", body(rng, "f", true), &plain[rng.below(plain.len())]),
            8 => format!("IFS=:; for f in ${}; do echo \"<$f>\"; done; unset IFS", loop_var),
            9 => format!("{} && {} || {}", condition(rng, loop_var), simple(rng, loop_var), simple(rng, loop_var)),
            _ => simple(rng, loop_var),
        }
    }
    let mut out = Vec::new();
    for _ in 0..n {
        let mut parts: Vec<String> = Vec::new();
        if rng.chance(5) && !folders.is_empty() {
            parts.push(format!("cd {}", some(rng, folders)));
        }
        for _ in 0..rng.below(3) {
            parts.push(assign(rng));
        }
        for _ in 0..1 + rng.below(3) {
            let v = var(rng);
            let depth = 1 + rng.below(3);
            parts.push(statement(rng, depth, v, false, &plain, &simple, &condition, &word_of));
        }
        if rng.chance(4) {
            parts.push(format!("echo \"$? $X $Y $N\"; pwd"));
        }
        out.push(parts.join("; "));
    }
    out
}

/// A tree for diff: groups of files that are versions of one another,
/// over small sets of lines so that many lines are equal, and two
/// folders `ta` and `tb` that hold some of the same names.
fn diff_tree(rng: &mut Rng, out: &mut String) -> (Vec<String>, Vec<String>) {
    fn alphabet(rng: &mut Rng) -> Vec<String> {
        match rng.below(6) {
            0 => ["a", "b", "c", ""].iter().map(|s| s.to_string()).collect(),
            1 => ["x", "y", "{", "}", "", "  foo", "foo", "bar ", "Bar", "BAR", " a  b", "a b", "\ta b ", "a\tb"].iter().map(|s| s.to_string()).collect(),
            2 => (0..40).map(|i| format!("line{}", i)).chain(["".to_string(), "}".to_string()]).collect(),
            3 => (0..400).map(|i| format!("w{}", i)).collect(),
            4 => ["", " ", "\t", "a", "A", "a ", " a"].iter().map(|s| s.to_string()).collect(),
            _ => (0..8).map(|i| format!("k{}", i)).chain(["".to_string(), "".to_string(), "".to_string()]).collect(),
        }
    }
    fn lines(rng: &mut Rng, k: usize, alpha: &[String]) -> Vec<String> {
        (0..k).map(|_| alpha[rng.below(alpha.len())].clone()).collect()
    }
    fn mutate(rng: &mut Rng, from: &[String], alpha: &[String]) -> Vec<String> {
        let mut out = from.to_vec();
        for _ in 0..[0, 1, 1, 2, 3, 5, 10, 30][rng.below(8)] {
            let pos = rng.below(out.len() + 1);
            let at = pos.min(out.len().saturating_sub(1));
            match rng.below(6) {
                0 if !out.is_empty() => {
                    out.remove(at);
                }
                1 => out.insert(pos, alpha[rng.below(alpha.len())].clone()),
                2 if !out.is_empty() => out[at] = alpha[rng.below(alpha.len())].clone(),
                3 if !out.is_empty() => {
                    let end = (at + 1 + rng.below(7)).min(out.len());
                    out.drain(at..end);
                }
                4 => {
                    let k = 1 + rng.below(7);
                    let new = lines(rng, k, alpha);
                    out.splice(pos..pos, new);
                }
                5 if !out.is_empty() => {
                    let end = (at + 1 + rng.below(5)).min(out.len());
                    let block: Vec<String> = out.drain(at..end).collect();
                    let to = rng.below(out.len() + 1);
                    out.splice(to..to, block);
                }
                _ => {}
            }
        }
        out
    }
    // A file whose last line has no newline, one time in six.
    fn file(rng: &mut Rng, out: &mut String, path: &str, text: &[String]) {
        out.push_str(&format!("F {}\n", path));
        let open = !text.is_empty() && rng.chance(6);
        for (i, l) in text.iter().enumerate() {
            out.push_str(&format!("{}{}\n", if open && i + 1 == text.len() { ':' } else { '|' }, l));
        }
    }
    let mut files = Vec::new();
    for g in 0..10 {
        let alpha = alphabet(rng);
        let k = [0, 1, 2, 3, 5, 8, 20, 60, 200, 800][rng.below(10)];
        let base = lines(rng, k, &alpha);
        for v in 0..4 {
            let text = if v == 0 {
                base.clone()
            } else if rng.chance(8) {
                let k = [0, 1, 5, 30][rng.below(4)];
                lines(rng, k, &alpha)
            } else {
                mutate(rng, &base, &alpha)
            };
            let path = format!("g{}/v{}.txt", g, v);
            file(rng, out, &path, &text);
            files.push(path);
        }
    }
    // Two folders: names in one, in the other, in both; equal, changed.
    // Their files are returned with the folders: every path returned
    // is given a time, which the headers of a unified diff print.
    let mut timed = vec!["ta".to_string(), "tb".to_string()];
    let alpha = alphabet(rng);
    for name in ["1.txt", "2.txt", "3.c", "4.o", "B", "a b", "s/5.txt", "s/6.o", "s/t/7", "u/8", "Z.txt", "_x"] {
        let k = [0, 1, 4, 12, 40][rng.below(5)];
        let base = lines(rng, k, &alpha);
        let where_ = rng.below(8);
        if where_ != 0 {
            file(rng, out, &format!("ta/{}", name), &base);
            timed.push(format!("ta/{}", name));
        }
        if where_ != 1 {
            let text = if rng.chance(2) { base.clone() } else { mutate(rng, &base, &alpha) };
            file(rng, out, &format!("tb/{}", name), &text);
            timed.push(format!("tb/{}", name));
        }
    }
    // One name that is a file on one side and a folder on the other.
    out.push_str("F ta/mixed\n|x\nF tb/mixed/inner\n|y\nD ta/hollow\n");
    (files, timed)
}

fn diff(rng: &mut Rng, files: &[String], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        let mut flags: Vec<String> = Vec::new();
        match rng.below(10) {
            0..=3 => flags.push("-u".into()),
            4 => flags.push(format!("-U {}", [0, 1, 2, 5, 10][rng.below(5)])),
            5 => flags.push(format!("--unified={}", rng.below(4))),
            _ => {}
        }
        for f in ["-i", "-w", "-b", "-B", "-a"] {
            if rng.chance(8) {
                flags.push(f.into());
            }
        }
        if rng.chance(15) {
            flags.push(rng.pick(&["-q", "--brief"]).to_string());
        }
        if rng.chance(15) {
            flags.push(rng.pick(&["-s", "--report-identical-files"]).to_string());
        }
        // Standard input has no time to print in a header: the forms
        // that read it always name the two sides.
        let form = rng.below(12);
        if rng.chance(6) || form == 3 || form == 4 {
            flags.push("--label one -L two".into());
        }
        let x = some(rng, files).to_string();
        // Mostly a version of the same file.
        let y = if rng.chance(6) { some(rng, files).to_string() } else { format!("{}/v{}.txt", x.split('/').next().unwrap(), rng.below(4)) };
        let flags = flags.join(" ");
        out.push(match form {
            0 | 1 => {
                let mut more = String::new();
                if rng.chance(2) {
                    more.push_str(" -N");
                }
                if rng.chance(3) {
                    more.push_str(rng.pick(&[" -x '*.o'", " --exclude='*.txt'", " -x s", " -x t --exclude 4.o", " -x '[0-9]*'"]));
                }
                let (a, b) = if rng.chance(8) { ("tb", "ta") } else { ("ta", "tb") };
                format!("diff -r{} {} {} {}", more, flags, a, b)
            }
            2 => format!("diff {} ta tb", flags),
            3 => format!("cat {} | diff {} - {}", x, flags, y),
            4 => format!("diff {} {} - < {}", flags, x, y),
            5 => format!("diff {} {} ta/s", flags, rng.pick(&["tb/s/5.txt", "ta/s/6.o", "g0/v0.txt"])),
            6 => format!("diff {} -N {} {}", flags, x, rng.pick(&["nowhere", "ta/gone"])),
            _ => format!("diff {} {} {}", flags, x, y),
        });
    }
    out
}

/// The tests of one family: `n` command lines over one tree.
pub fn generate(family: &str, n: usize, seed: u64) -> Option<Generated> {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03 ^ family.len() as u64);
    for _ in 0..8 {
        rng.next();
    }
    let mut fixture = String::new();
    let (files, folders) = match family {
        "rg" => (rg_tree(&mut rng, &mut fixture), Vec::new()),
        "diff" => diff_tree(&mut rng, &mut fixture),
        _ => tree(&mut rng, &mut fixture),
    };
    // Each path its own age, half a minute from any whole minute, so
    // that no test of minutes or days is near its boundary while a case
    // runs: minutes for some paths, days for others.
    let mut ages = Vec::new();
    let mut all: Vec<String> = files.clone();
    all.extend(folders.iter().cloned());
    if family != "diff" {
        all.push("empty".into());
    }
    all.push(".".into());
    let mut taken = std::collections::HashSet::new();
    for p in &all {
        let mut minutes = if rng.chance(3) { rng.below(4) as i64 * 1440 + 100 + rng.below(1200) as i64 } else { 1 + rng.below(95) as i64 };
        while !taken.insert(minutes) {
            minutes += 1;
        }
        ages.push((p.clone(), minutes * 60 + 30));
    }
    let commands = match family {
        "shell" => shell(&mut rng, &files, n),
        "grep" => grep(&mut rng, &files, &folders, n),
        "sed" => sed(&mut rng, &files, n),
        "find" => find(&mut rng, &files, &folders, n),
        "rg" => rg(&mut rng, &files, n),
        "text" => text_tools(&mut rng, &files, n),
        "ls" => ls_du(&mut rng, &files, &folders, n),
        "xargs" => xargs(&mut rng, &files, n),
        "scripts" => scripts(&mut rng, &files, &folders, n),
        "diff" => diff(&mut rng, &files, n),
        _ => return None,
    };
    Some(Generated { fixture, ages, commands })
}
