//! Helpers shared by the test harness and the benchmark: running a command
//! on the real GNU tools and comparing results.

use crate::engine::Outcome;
use crate::shell;
use std::path::Path;
use std::process::{Command, Stdio};

/// Real find and grep -r list a folder in whatever order the filesystem
/// returns, so their output can only be compared as a set of lines.
pub fn order_is_unspecified(command: &str) -> bool {
    let pipeline = match shell::parse(command.as_bytes()) {
        Ok(Some(p)) => p,
        _ => return false,
    };
    pipeline.commands.iter().any(|c| {
        let words: Vec<Vec<u8>> = c.words.iter().map(|w| w.text()).collect();
        match words.first().map(|w| w.as_slice()) {
            Some(b"find") => true,
            Some(b"grep") => words[1..]
                .iter()
                .take_while(|w| w.as_slice() != b"--")
                .any(|w| w.starts_with(b"-") && !w.starts_with(b"--") && w.contains(&b'r')),
            _ => false,
        }
    })
}

pub fn sorted_lines(bytes: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<&[u8]> = bytes.split_inclusive(|c| *c == b'\n').collect();
    lines.sort();
    lines
}

/// Compare two outputs, as sets of lines when `unordered`.
pub fn same_output(a: &[u8], b: &[u8], unordered: bool) -> bool {
    if unordered {
        sorted_lines(a) == sorted_lines(b)
    } else {
        a == b
    }
}

pub fn replace(hay: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return hay.to_vec();
    }
    let mut out = Vec::with_capacity(hay.len());
    let mut i = 0;
    while i < hay.len() {
        if hay[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(hay[i]);
            i += 1;
        }
    }
    out
}

/// Refuse a command that could touch anything outside `root` if it were
/// handed to the real tools.
pub fn check_safe(command: &str, root: &Path) -> Result<(), String> {
    let pipeline = match shell::parse(command.as_bytes()) {
        Ok(Some(p)) => p,
        _ => return Ok(()),
    };
    let root = root.to_string_lossy().into_owned().into_bytes();
    for c in &pipeline.commands {
        let words = c.words.iter().skip(1).map(|w| w.text());
        let targets = c.redirects.iter().map(|r| r.target.text());
        for w in words.chain(targets) {
            // Follow the path: it must never climb above where it starts,
            // and an absolute path must start inside the root.
            let rel: &[u8] = if w.starts_with(b"/") {
                match w.strip_prefix(&root[..]) {
                    Some(rest) => rest,
                    None => return Err(format!("refusing to run on the real tools (absolute path): {}", command)),
                }
            } else {
                &w
            };
            let mut depth = 0i64;
            for comp in rel.split(|c| *c == b'/') {
                match comp {
                    b"" | b"." => {}
                    b".." => depth -= 1,
                    _ => depth += 1,
                }
                if depth < 0 {
                    return Err(format!("refusing to run on the real tools (path leaves the scratch directory): {}", command));
                }
            }
        }
    }
    Ok(())
}

/// Run a command line with the real tools: bash, in the C locale, with
/// `root` as the working directory and no standard input. The root's path
/// is removed from the output, and bash's own error prefix is replaced by
/// the engine's, so the result can be compared with the engine's.
pub fn run_real(root: &Path, command: &str) -> Outcome {
    let out = Command::new("bash")
        .args(["--noprofile", "--norc", "-c", command])
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .expect("cannot run bash");
    let root_bytes = root.to_string_lossy().into_owned().into_bytes();
    let stdout = replace(&out.stdout, &root_bytes, b"");
    let stderr = replace(&out.stderr, &root_bytes, b"");
    let stderr = replace(&stderr, b"bash: line 1: ", b"qdb: ");
    Outcome { stdout, stderr, code: out.status.code().unwrap_or(-1) }
}
