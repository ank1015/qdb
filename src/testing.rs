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

// ------------------------------------------------- real tools, for timing

use std::time::{Duration, Instant};

/// What a real command produced.
pub struct RealOutput {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Run a command on the real tools: bash, in the dataset's folder, with
/// its output captured through pipes. The time is from starting bash to
/// its exit with all output read.
///
/// The command is started by a small helper process, not by this one:
/// the program runs itself again with `--helper` as its only argument,
/// and its `main` must then call `helper`.
/// This process holds the whole dataset in memory, and starting a child
/// from a process that large costs time that would be charged to the
/// real tools; it also leaves this process's memory marked to be copied
/// on the next write, which would be charged to the engine.
pub fn timed_real(root: &Path, command: &str) -> (Duration, RealOutput) {
    use std::io::{Read, Write};
    static HELPER: std::sync::Mutex<Option<(std::process::ChildStdin, std::process::ChildStdout)>> = std::sync::Mutex::new(None);
    let mut helper = HELPER.lock().unwrap();
    if helper.is_none() {
        let exe = std::env::current_exe().expect("own path");
        let mut child = Command::new(exe).arg("--helper").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("starting the helper");
        *helper = Some((child.stdin.take().unwrap(), child.stdout.take().unwrap()));
    }
    let (to, from) = helper.as_mut().unwrap();
    let root = root.to_string_lossy();
    let mut request = Vec::new();
    for part in [root.as_bytes(), command.as_bytes()] {
        request.extend_from_slice(&(part.len() as u64).to_le_bytes());
        request.extend_from_slice(part);
    }
    to.write_all(&request).and_then(|_| to.flush()).expect("writing to the helper");
    let number = |from: &mut std::process::ChildStdout| {
        let mut b = [0u8; 8];
        from.read_exact(&mut b).expect("reading from the helper");
        u64::from_le_bytes(b)
    };
    let nanos = number(from);
    let code = number(from) as i64 as i32;
    let read = |from: &mut std::process::ChildStdout| {
        let mut data = vec![0u8; number(from) as usize];
        from.read_exact(&mut data).expect("reading from the helper");
        data
    };
    let stdout = read(from);
    let stderr = read(from);
    (Duration::from_nanos(nanos), RealOutput { code, stdout, stderr })
}

/// The helper process: runs each command it is sent and sends back the
/// time, exit status and output. Ends when its input does.
pub fn helper() {
    use std::io::{Read, Write};
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut field = || -> Option<Vec<u8>> {
            let mut b = [0u8; 8];
            input.read_exact(&mut b).ok()?;
            let mut data = vec![0u8; u64::from_le_bytes(b) as usize];
            input.read_exact(&mut data).ok()?;
            Some(data)
        };
        let (root, command) = match (field(), field()) {
            (Some(r), Some(c)) => (String::from_utf8_lossy(&r).into_owned(), String::from_utf8_lossy(&c).into_owned()),
            _ => return,
        };
        let t = Instant::now();
        let out = Command::new("bash")
            .args(["--noprofile", "--norc", "-c", &command])
            .current_dir(&root)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .output()
            .expect("cannot run bash");
        let nanos = t.elapsed().as_nanos() as u64;
        let mut reply = Vec::new();
        reply.extend_from_slice(&nanos.to_le_bytes());
        reply.extend_from_slice(&(out.status.code().unwrap_or(-1) as i64 as u64).to_le_bytes());
        for part in [&out.stdout, &out.stderr] {
            reply.extend_from_slice(&(part.len() as u64).to_le_bytes());
            reply.extend_from_slice(part);
        }
        if output.write_all(&reply).and_then(|_| output.flush()).is_err() {
            return;
        }
    }
}

