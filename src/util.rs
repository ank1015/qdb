//! Helpers shared by the commands: option parsing and GNU-style quoting.

/// Why a command line cannot be run as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptError {
    /// A flag or form this version does not support.
    Unsupported(String),
    /// A flag that takes an argument was last on the line.
    MissingArg(u8),
}

#[derive(Debug, Default)]
pub struct Parsed {
    pub flags: Vec<(u8, Option<Vec<u8>>)>,
    pub operands: Vec<Vec<u8>>,
}

impl Parsed {
    pub fn has(&self, f: u8) -> bool {
        self.flags.iter().any(|(c, _)| *c == f)
    }

    /// The argument of the last occurrence of a flag.
    pub fn arg(&self, f: u8) -> Option<&[u8]> {
        self.flags.iter().rev().find(|(c, _)| *c == f).and_then(|(_, a)| a.as_deref())
    }
}

/// Parse options the way GNU getopt does: short flags can be combined
/// (`-rin`), an argument can be attached (`-n5`) or separate (`-n 5`),
/// options and operands can be mixed, and `--` ends the options.
///
/// `bool_flags` and `arg_flags` are the supported flags. Anything else,
/// including every long option, is an `Unsupported` error.
pub fn getopt(cmd: &str, args: &[Vec<u8>], bool_flags: &[u8], arg_flags: &[u8]) -> Result<Parsed, OptError> {
    let mut out = Parsed::default();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == b"--" {
            out.operands.extend(args[i..].iter().cloned());
            break;
        }
        if a.starts_with(b"--") {
            return Err(OptError::Unsupported(format!(
                "flag: {} {} (long options are not supported yet)",
                cmd,
                String::from_utf8_lossy(a)
            )));
        }
        if a.len() < 2 || a[0] != b'-' {
            out.operands.push(a.clone());
            continue;
        }
        let mut j = 1;
        while j < a.len() {
            let c = a[j];
            j += 1;
            if bool_flags.contains(&c) {
                out.flags.push((c, None));
            } else if arg_flags.contains(&c) {
                if j < a.len() {
                    out.flags.push((c, Some(a[j..].to_vec())));
                } else if i < args.len() {
                    out.flags.push((c, Some(args[i].clone())));
                    i += 1;
                } else {
                    return Err(OptError::MissingArg(c));
                }
                break;
            } else {
                return Err(OptError::Unsupported(format!("flag: {} -{}", cmd, char::from(c))));
            }
        }
    }
    Ok(out)
}

/// Quote a name the way GNU coreutils does in error messages in the C
/// locale (the "shell-escape-always" style): `'name'`, with `"it's"` when
/// the only awkward character is a single quote, and `$'\n'` escapes for
/// bytes that are not printable ASCII.
pub fn quote(name: &[u8]) -> String {
    let printable = |c: u8| (0x20..0x7f).contains(&c);
    let has_single = name.contains(&b'\'');
    // With a single quote and nothing else the shell would treat
    // specially, GNU switches to double quotes.
    let compat = |c: u8| c.is_ascii_alphanumeric() || b" '%+,-./:]_{}@".contains(&c);
    if has_single && name.iter().all(|&c| compat(c)) {
        return format!("\"{}\"", String::from_utf8_lossy(name));
    }
    let mut out: Vec<u8> = vec![b'\''];
    let mut in_escape = false;
    for &c in name {
        if printable(c) {
            if in_escape {
                out.extend_from_slice(b"''");
                in_escape = false;
            }
            if c == b'\'' {
                out.extend_from_slice(b"'\\''");
            } else {
                out.push(c);
            }
        } else {
            if !in_escape {
                out.extend_from_slice(b"'$'");
                in_escape = true;
            }
            match c {
                0x07 => out.extend_from_slice(b"\\a"),
                0x08 => out.extend_from_slice(b"\\b"),
                0x0c => out.extend_from_slice(b"\\f"),
                b'\n' => out.extend_from_slice(b"\\n"),
                b'\r' => out.extend_from_slice(b"\\r"),
                b'\t' => out.extend_from_slice(b"\\t"),
                0x0b => out.extend_from_slice(b"\\v"),
                _ => out.extend_from_slice(format!("\\{:03o}", c).as_bytes()),
            }
        }
    }
    out.push(b'\'');
    // Error messages are bytes; every byte produced here is ASCII.
    String::from_utf8(out).unwrap()
}

/// The last component of a path, ignoring trailing slashes.
pub fn basename(path: &[u8]) -> &[u8] {
    let mut end = path.len();
    while end > 1 && path[end - 1] == b'/' {
        end -= 1;
    }
    let p = &path[..end];
    match p.iter().rposition(|c| *c == b'/') {
        Some(i) if p.len() > 1 => &p[i + 1..],
        _ => p,
    }
}

/// Join a directory and a name without doubling the slash.
pub fn join(dir: &[u8], name: &[u8]) -> Vec<u8> {
    let mut out = dir.to_vec();
    if !out.ends_with(b"/") {
        out.push(b'/');
    }
    out.extend_from_slice(name);
    out
}

/// Split content into lines. Each line includes its `\n`, except possibly
/// the last one.
pub fn lines(content: &[u8]) -> impl Iterator<Item = &[u8]> {
    content.split_inclusive(|c| *c == b'\n')
}

pub fn lossy(b: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(quote(b"nope"), "'nope'");
        assert_eq!(quote(b""), "''");
        assert_eq!(quote(b"it's"), "\"it's\"");
        assert_eq!(quote(b"it's $x"), "'it'\\''s $x'");
        assert_eq!(quote(b"q\n"), "'q'$'\\n'");
        assert_eq!(quote("\u{e9}2".as_bytes()), "''$'\\303\\251''2'");
    }

    #[test]
    fn names() {
        assert_eq!(basename(b"a/b/"), b"b");
        assert_eq!(basename(b"a"), b"a");
        assert_eq!(basename(b"/"), b"/");
        assert_eq!(basename(b"d/."), b".");
        assert_eq!(join(b"e/", b"f"), b"e/f");
        assert_eq!(join(b"e", b"f"), b"e/f");
    }

    #[test]
    fn options() {
        let a = |s: &[&str]| s.iter().map(|x| x.as_bytes().to_vec()).collect::<Vec<_>>();
        let p = getopt("grep", &a(&["-rin", "foo", "src", "-c"]), b"rinc", b"").unwrap();
        assert_eq!(p.flags.len(), 4);
        assert_eq!(p.operands.len(), 2);
        let p = getopt("head", &a(&["-n5", "f", "-n", "-3", "--", "-x"]), b"", b"n").unwrap();
        assert_eq!(p.arg(b'n'), Some(&b"-3"[..]));
        assert_eq!(p.operands, a(&["f", "-x"]));
        assert!(matches!(getopt("ls", &a(&["-l"]), b"a", b""), Err(OptError::Unsupported(_))));
        assert!(matches!(getopt("ls", &a(&["--all"]), b"a", b""), Err(OptError::Unsupported(_))));
        assert!(matches!(getopt("head", &a(&["-n"]), b"", b"n"), Err(OptError::MissingArg(b'n'))));
    }
}
