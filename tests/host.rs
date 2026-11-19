//! Many filesystems in one process: the host layer.

use qdb::engine::Ending;
use qdb::host::{Host, HostConfig, HostError, Limits, Quotas, Request};
use qdb::unit::{self, Reason, Stop};
use qdb::Session;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let path = std::env::temp_dir().join(format!("qdb-host-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Dir(path)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn req(line: &str) -> (Request<'_>, Arc<Stop>) {
    let stop = Stop::new();
    (Request { line: line.as_bytes(), quotas: Quotas::default(), limits: Limits::default(), stop: stop.clone() }, stop)
}

fn run(host: &Host, id: &str, line: &str) -> Result<(String, String, i32, Ending), HostError> {
    let (r, _) = req(line);
    let mut sess = Session::new();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let end = host.run(id, &r, &mut sess, &mut out, &mut err)?;
    Ok((String::from_utf8_lossy(&out).into_owned(), String::from_utf8_lossy(&err).into_owned(), end.code, end.ending))
}

fn ok(host: &Host, id: &str, line: &str) -> String {
    let (out, err, code, _) = run(host, id, line).unwrap();
    assert_eq!(code, 0, "{}: {}", line, err);
    out
}

#[test]
fn filesystems_are_made_used_and_deleted() {
    let dir = Dir::new("basic");
    let host = Host::open(&dir.0, HostConfig::default()).unwrap();
    // One host per data directory.
    assert!(Host::open(&dir.0, HostConfig::default()).is_err());
    let a = host.create().unwrap();
    let b = host.create().unwrap();
    assert_ne!(a, b);
    assert_eq!(a.len(), 32);
    ok(&host, &a, "echo marker-a > a.txt");
    ok(&host, &b, "echo marker-b > b.txt");
    assert_eq!(ok(&host, &a, "cat a.txt; ls"), "marker-a\na.txt\n");
    assert_eq!(ok(&host, &b, "ls"), "b.txt\n");
    let info = host.info(&a).unwrap();
    assert_eq!((info.files, info.folders, info.text_bytes), (1, 1, 9));
    let mut ids = host.list();
    ids.sort();
    let mut want = vec![a.clone(), b.clone()];
    want.sort();
    assert_eq!(ids, want);
    for bad in ["", "..", "../fs", "0123456789abcdef0123456789abcdeg", "0123456789ABCDEF0123456789ABCDEF", "00000000000000000000000000000000"] {
        assert_eq!(run(&host, bad, "ls").unwrap_err(), HostError::NotFound, "{}", bad);
    }
    host.delete(&a).unwrap();
    assert_eq!(run(&host, &a, "ls").unwrap_err(), HostError::NotFound);
    assert_eq!(host.delete(&a).unwrap_err(), HostError::NotFound);
    assert_eq!(host.list(), vec![b.clone()]);
    host.shutdown();
    drop(host);
    // Everything is there after a restart.
    let host = Host::open(&dir.0, HostConfig::default()).unwrap();
    assert_eq!(ok(&host, &b, "cat b.txt"), "marker-b\n");
    assert_eq!(run(&host, &a, "ls").unwrap_err(), HostError::NotFound);
}

#[test]
fn a_small_filesystem_is_small() {
    let dir = Dir::new("small");
    let host = Host::open(&dir.0, HostConfig::default()).unwrap();
    let a = host.create().unwrap();
    ok(&host, &a, "echo hello > one.txt");
    host.shutdown();
    let path = dir.0.join("fs").join(&a[..2]).join(&a);
    let bytes: u64 = std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().metadata().unwrap().len()).sum();
    assert!(bytes < 1024, "{} bytes", bytes);
}

#[test]
fn idle_and_surplus_filesystems_are_closed() {
    let dir = Dir::new("idle");
    let cfg = HostConfig { max_open: 3, idle_close: Duration::from_millis(100), ..HostConfig::default() };
    let host = Host::open(&dir.0, cfg).unwrap();
    let ids: Vec<String> = (0..8).map(|_| host.create().unwrap()).collect();
    for (i, id) in ids.iter().enumerate() {
        ok(&host, id, &format!("echo {} > n.txt", i));
        assert!(host.open_counts().0 <= 3, "{:?}", host.open_counts());
    }
    let t = Instant::now();
    while host.open_counts().0 > 0 {
        assert!(t.elapsed() < Duration::from_secs(5), "still open: {:?}", host.open_counts());
        std::thread::sleep(Duration::from_millis(20));
    }
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(ok(&host, id, "cat n.txt"), format!("{}\n", i));
    }
}

#[test]
fn a_unit_past_its_deadline_is_stopped_and_leaves_nothing() {
    unit::quiet_panics();
    let dir = Dir::new("deadline");
    let host = Host::open(&dir.0, HostConfig::default()).unwrap();
    let a = host.create().unwrap();
    ok(&host, &a, "echo keep > k.txt");
    let stop = Stop::new();
    let r = Request {
        line: b"rm k.txt; while true; do echo x >> x.txt; done",
        quotas: Quotas::default(),
        limits: Limits { time: Duration::from_millis(100), commands: u64::MAX, ..Limits::default() },
        stop,
    };
    let mut sess = Session::new();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let t = Instant::now();
    let end = host.run(&a, &r, &mut sess, &mut out, &mut err).unwrap();
    let took = t.elapsed();
    assert_eq!(end.ending, Ending::Stopped(Reason::Deadline), "{}", String::from_utf8_lossy(&err));
    assert!(took < Duration::from_millis(150), "{:?}", took);
    assert_eq!(ok(&host, &a, "ls; cat k.txt"), "k.txt\nkeep\n");
}

#[test]
fn a_fault_in_one_unit_stays_in_it() {
    unit::quiet_panics();
    let dir = Dir::new("fault");
    let host = Host::open(&dir.0, HostConfig::default()).unwrap();
    let a = host.create().unwrap();
    let b = host.create().unwrap();
    ok(&host, &a, "echo keep > k.txt");
    ok(&host, &b, "echo other > o.txt");
    for line in ["rm k.txt; echo y > y.txt; cat y.txt", "cat k.txt"] {
        let (r, stop) = req(line);
        stop.request(Reason::Fault);
        let mut sess = Session::new();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let end = host.run(&a, &r, &mut sess, &mut out, &mut err).unwrap();
        assert!(matches!(end.ending, Ending::Fault(_)), "{:?}", end);
        assert!(String::from_utf8_lossy(&err).contains("internal error"));
        assert_eq!(ok(&host, &a, "ls; cat k.txt"), "k.txt\nkeep\n");
        assert_eq!(ok(&host, &b, "cat o.txt"), "other\n");
    }
}

#[test]
fn delete_stops_what_runs_there() {
    unit::quiet_panics();
    let dir = Dir::new("delete");
    let host = Host::open(&dir.0, HostConfig::default()).unwrap();
    let a = host.create().unwrap();
    let h = host.clone();
    let a2 = a.clone();
    let t = std::thread::spawn(move || {
        let stop = Stop::new();
        let r = Request { line: b"while true; do echo x >> x.txt; done", quotas: Quotas::default(), limits: Limits { commands: u64::MAX, ..Limits::default() }, stop };
        let mut sess = Session::new();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        h.run(&a2, &r, &mut sess, &mut out, &mut err)
    });
    std::thread::sleep(Duration::from_millis(50));
    host.delete(&a).unwrap();
    let end = t.join().unwrap().unwrap();
    assert_eq!(end.ending, Ending::Stopped(Reason::Deleted));
    assert!(!dir.0.join("fs").join(&a[..2]).join(&a).exists());
    assert!(host.list().is_empty());
}

#[test]
fn units_from_many_threads_are_as_if_one_at_a_time() {
    let dir = Dir::new("many");
    let host = Host::open(&dir.0, HostConfig::default()).unwrap();
    let a = host.create().unwrap();
    ok(&host, &a, "echo 0 > n.txt");
    std::thread::scope(|s| {
        for t in 0..8 {
            let host = &host;
            let a = &a;
            s.spawn(move || {
                for i in 0..50 {
                    if (i + t) % 3 == 0 {
                        ok(host, a, &format!("echo t{}i{} >> log.txt", t, i));
                    } else {
                        let out = ok(host, a, "cat n.txt; wc -l < log.txt 2>/dev/null || true");
                        assert!(out.starts_with("0\n"));
                    }
                }
            });
        }
    });
    let n: usize = ok(&host, &a, "wc -l < log.txt").trim().parse().unwrap();
    let want = (0..8).map(|t| (0..50).filter(|i| (i + t) % 3 == 0).count()).sum::<usize>();
    assert_eq!(n, want);
}
