//! Helpers shared by the test harness and the benchmark: running a command
//! on the real GNU tools and comparing results.

use crate::engine::Outcome;
use crate::shell;
use std::path::Path;
use std::process::{Command, Stdio};

/// Real find and grep -r list a folder in whatever order the filesystem
/// returns, so their output can only be compared as a set of lines.
pub fn order_is_unspecified(command: &str) -> bool {
    let script = match shell::parse(command.as_bytes()) {
        Ok(s) => s,
        _ => return false,
    };
    script.all_commands().iter().any(|c| {
        let words: Vec<Vec<u8>> = c.words.iter().map(|w| w.text()).collect();
        // Every letter given as a short flag.
        let flags: Vec<u8> = words
            .iter()
            .skip(1)
            .take_while(|w| w.as_slice() != b"--")
            .filter(|w| w.starts_with(b"-") && !w.starts_with(b"--"))
            .flat_map(|w| w[1..].to_vec())
            .collect();
        let has = |f: &[u8]| f.iter().any(|c| flags.contains(c));
        match words.first().map(|w| w.as_slice()) {
            Some(b"find") | Some(b"du") => true,
            Some(b"grep") => has(b"rR"),
            // The lines -v prints for a tree come in the order of the walk.
            Some(b"rm") | Some(b"cp") => has(b"v") && has(b"rRa"),
            // rg walks in directory order unless told to sort.
            Some(b"rg") => !words.windows(2).any(|w| w[0] == b"--sort" && w[1] == b"path") && !words.iter().any(|w| w == b"--sort=path"),
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
    let script = match shell::parse(command.as_bytes()) {
        Ok(s) => s,
        _ => return Ok(()),
    };
    let root = root.to_string_lossy().into_owned().into_bytes();
    for c in &script.all_commands() {
        let words = c.words.iter().skip(1);
        let targets = c.redirects.iter().filter_map(|r| r.target());
        // A sed script is not a path, though one can name a file to
        // read or write: that it must not do with a path from the root.
        let is_sed = c.words.first().is_some_and(|w| w.text() == b"sed");
        let explicit = is_sed && c.words.iter().any(|w| w.text().starts_with(b"-e") || (w.text().starts_with(b"-") && !w.text().starts_with(b"--") && w.text().ends_with(b"e")));
        let mut script_next = false;
        let mut seen_script = explicit;
        for w in words.chain(targets).flat_map(shell::brace_expand).map(|w| w.text()) {
            if w == b"/dev/null" {
                continue;
            }
            if is_sed {
                let is_script = if script_next {
                    script_next = false;
                    true
                } else if w.starts_with(b"-") && w.len() > 1 {
                    script_next = !w.starts_with(b"--") && w.ends_with(b"e");
                    w.starts_with(b"-e") && w.len() > 2
                } else if !seen_script {
                    seen_script = true;
                    true
                } else {
                    false
                };
                if is_script {
                    let names_file = (0..w.len()).any(|i| {
                        let rest = &w[i + 1..];
                        let name = rest.trim_ascii_start();
                        b"wWrR".contains(&w[i]) && (i == 0 || w[i - 1] != b'\\') && name.len() < rest.len() && (name.starts_with(b"/") || name.starts_with(b".."))
                    });
                    if names_file {
                        return Err(format!("refusing to run on the real tools (sed script names a file): {}", command));
                    }
                    continue;
                }
            }
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

/// In lines that look like `ls -l` entries, replace the owner and group
/// with `qdb`.
fn mask_owner(out: &[u8]) -> Vec<u8> {
    let mut masked = Vec::with_capacity(out.len());
    for line in out.split_inclusive(|c| *c == b'\n') {
        let is_entry = line.len() > 11 && matches!(line[0], b'-' | b'd') && line[1..10].iter().all(|c| b"rwx-".contains(c)) && line[10] == b' ';
        if !is_entry {
            masked.extend_from_slice(line);
            continue;
        }
        static NAMES: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
        let names = NAMES.get_or_init(|| {
            let id = |flag: &str| {
                let out = Command::new("id").arg(flag).output().expect("cannot run id");
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            };
            format!(" {} {} ", id("-un"), id("-gn")).into_bytes()
        });
        match line.windows(names.len()).position(|w| w == &names[..]) {
            Some(at) => {
                masked.extend_from_slice(&line[..at]);
                masked.extend_from_slice(b" qdb qdb ");
                masked.extend_from_slice(&line[at + names.len()..]);
            }
            None => masked.extend_from_slice(line),
        }
    }
    masked
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
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .output()
        .expect("cannot run bash");
    let root_bytes = root.to_string_lossy().into_owned().into_bytes();
    let stdout = replace(&out.stdout, &root_bytes, b"");
    let stderr = replace(&out.stderr, &root_bytes, b"");
    let stderr = replace(&stderr, b"bash: line 1: ", b"qdb: ");
    // What the data model has no fact for is fixed on the engine's side;
    // the owner and group of ls -l are made to agree here.
    let stdout = if command.contains("ls ") { mask_owner(&stdout) } else { stdout };
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
    timed_real_to(root, command, None)
}

/// As `timed_real`, with standard output sent to a file instead of being
/// handed back, for output too large to hold.
pub fn timed_real_to(root: &Path, command: &str, stdout_to: Option<&Path>) -> (Duration, RealOutput) {
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
    let to_file = stdout_to.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    for part in [root.as_bytes(), command.as_bytes(), to_file.as_bytes()] {
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
        let (root, command, to_file) = match (field(), field(), field()) {
            (Some(r), Some(c), Some(f)) => (String::from_utf8_lossy(&r).into_owned(), String::from_utf8_lossy(&c).into_owned(), String::from_utf8_lossy(&f).into_owned()),
            _ => return,
        };
        let stdout = if to_file.is_empty() { Stdio::piped() } else { Stdio::from(std::fs::File::create(&to_file).expect("creating the file for output")) };
        let t = Instant::now();
        let out = Command::new("bash")
            .args(["--noprofile", "--norc", "-c", &command])
            .current_dir(&root)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|c| c.wait_with_output())
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


/// A file's bytes, read through a mapping: they are the operating
/// system's file cache, not memory of this process.
pub struct Mapped {
    ptr: *mut libc::c_void,
    len: usize,
}

impl Mapped {
    pub fn open(path: &Path) -> std::io::Result<Mapped> {
        use std::os::unix::io::AsRawFd;
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len() as usize;
        if len == 0 {
            return Ok(Mapped { ptr: std::ptr::null_mut(), len: 0 });
        }
        let ptr = unsafe { libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ, libc::MAP_SHARED, file.as_raw_fd(), 0) };
        if ptr == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Mapped { ptr, len })
    }
}

impl std::ops::Deref for Mapped {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len) }
        }
    }
}

impl Drop for Mapped {
    fn drop(&mut self) {
        if self.len > 0 {
            unsafe { libc::munmap(self.ptr, self.len) };
        }
    }
}

/// The lines of an output as a set with counts, in a form that does not
/// depend on their order and takes no memory: how many, and the sum of a
/// hash of each.
pub fn lines_digest(bytes: &[u8]) -> (u64, u128) {
    use std::hash::Hasher;
    let (mut n, mut sum) = (0u64, 0u128);
    for line in bytes.split_inclusive(|c| *c == b'\n') {
        let mut a = std::collections::hash_map::DefaultHasher::new();
        a.write(line);
        let mut b = std::collections::hash_map::DefaultHasher::new();
        b.write(b"qdb");
        b.write(line);
        n += 1;
        sum = sum.wrapping_add((a.finish() as u128) << 64 | b.finish() as u128);
    }
    (n, sum)
}
