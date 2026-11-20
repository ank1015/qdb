//! The service through HTTP: an in-process server on a local port.

use qdb::net::codec::unbase64;
use qdb::net::http::Client;
use qdb::net::json::Json;
use qdb::service::{serve, Service, ServiceConfig};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const ADMIN: &str = "test-admin-key-0123456789";

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let path = std::env::temp_dir().join(format!("qdb-api-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Dir(path)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Server {
    svc: Arc<Service>,
    addr: String,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn start(dir: &Dir, tweak: impl FnOnce(&mut ServiceConfig)) -> Server {
        let mut cfg = ServiceConfig::new(&dir.0, ADMIN);
        tweak(&mut cfg);
        let svc = Service::open(cfg).unwrap();
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let s2 = svc.clone();
        let thread = std::thread::spawn(move || serve(s2, l));
        Server { svc, addr, thread: Some(thread) }
    }

    fn client(&self) -> Client {
        Client::new(&self.addr)
    }

    fn stop(mut self) {
        self.svc.shutdown();
        self.thread.take().unwrap().join().unwrap();
    }
}

fn call(c: &mut Client, method: &str, path: &str, key: Option<&str>, body: &str) -> (u16, Json) {
    let r = c.request(method, path, key, body.as_bytes()).unwrap();
    let j = Json::parse(&r.body).unwrap_or_else(|e| panic!("{} {}: {} in {:?}", method, path, e, String::from_utf8_lossy(&r.body)));
    (r.status, j)
}

fn account(c: &mut Client, body: &str) -> (String, String) {
    let (s, j) = call(c, "POST", "/v1/accounts", Some(ADMIN), body);
    assert_eq!(s, 201, "{:?}", j);
    (j.get("account").unwrap().as_str().unwrap().to_string(), j.get("key").unwrap().as_str().unwrap().to_string())
}

fn new_fs(c: &mut Client, key: &str) -> String {
    let (s, j) = call(c, "POST", "/v1/filesystems", Some(key), "");
    assert_eq!(s, 201, "{:?}", j);
    j.get("filesystem").unwrap().as_str().unwrap().to_string()
}

fn run(c: &mut Client, key: &str, fs: &str, body: Json) -> Json {
    let (s, j) = call(c, "POST", &format!("/v1/filesystems/{}/run", fs), Some(key), &body.to_string());
    assert_eq!(s, 200, "{:?}", j);
    j
}

fn cmd(c: &mut Client, key: &str, fs: &str, line: &str) -> Json {
    run(c, key, fs, Json::obj().with("command", line))
}

fn out(j: &Json) -> &str {
    j.get("stdout").and_then(Json::as_str).unwrap_or_else(|| panic!("no stdout in {:?}", j))
}

#[test]
fn accounts_filesystems_and_units() {
    let dir = Dir::new("basic");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    assert_eq!(call(&mut c, "GET", "/v1/health", None, ""), (200, Json::obj().with("ok", true).with("stopping", false)));
    let (acc, key) = account(&mut c, "");
    assert!(key.starts_with("qdbk_"));
    let fs = new_fs(&mut c, &key);
    let j = cmd(&mut c, &key, &fs, "mkdir -p src && echo hello > src/a.txt && grep -rn hello .");
    assert_eq!(out(&j), "./src/a.txt:1:hello\n");
    assert_eq!(j.get("exit_status").unwrap().as_i64(), Some(0));
    assert_eq!(j.get("stopped"), Some(&Json::Null));
    assert_eq!(j.get("kind").unwrap().as_str(), Some("writer"));
    assert_eq!(j.get("cost").unwrap().get("syncs").unwrap().as_i64(), Some(1));
    // A unit that fails is still a 200.
    let j = cmd(&mut c, &key, &fs, "cat nope");
    assert_eq!(j.get("exit_status").unwrap().as_i64(), Some(1));
    assert_eq!(j.get("stderr").unwrap().as_str(), Some("cat: nope: No such file or directory\n"));
    assert_eq!(j.get("kind").unwrap().as_str(), Some("reader"));
    // Bytes that are not UTF-8.
    let j = cmd(&mut c, &key, &fs, r"printf '\377\n'");
    assert!(j.get("stdout").is_none());
    assert_eq!(unbase64(j.get("stdout_base64").unwrap().as_str().unwrap()).unwrap(), b"\xff\n");
    // Standard input, as text and as base64.
    let j = run(&mut c, &key, &fs, Json::obj().with("command", "wc -c").with("stdin", "abc"));
    assert_eq!(out(&j), "3\n");
    let j = run(&mut c, &key, &fs, Json::obj().with("command", "cat -v").with("stdin_base64", "/wA="));
    assert_eq!(out(&j), "M-^?^@");
    // Info and list.
    let (s, j) = call(&mut c, "GET", &format!("/v1/filesystems/{}", fs), Some(&key), "");
    assert_eq!(s, 200);
    assert_eq!(j.get("files").unwrap().as_i64(), Some(1));
    assert_eq!(j.get("text_bytes").unwrap().as_i64(), Some(6));
    assert_eq!(j.get("usage").unwrap().get("units").unwrap().as_i64(), Some(5));
    let (_, j) = call(&mut c, "GET", "/v1/filesystems", Some(&key), "");
    assert_eq!(j.get("filesystems"), Some(&Json::Arr(vec![Json::from(fs.as_str())])));
    let (_, j) = call(&mut c, "GET", &format!("/v1/accounts/{}", acc), Some(ADMIN), "");
    assert_eq!(j.get("usage").unwrap().get("units").unwrap().as_i64(), Some(5));
    // Delete.
    let (s, _) = call(&mut c, "DELETE", &format!("/v1/filesystems/{}", fs), Some(&key), "");
    assert_eq!(s, 200);
    let (s, _) = call(&mut c, "POST", &format!("/v1/filesystems/{}/run", fs), Some(&key), r#"{"command":"ls"}"#);
    assert_eq!(s, 404);
    // The usage of a deleted filesystem stays with the account.
    let (_, j) = call(&mut c, "GET", &format!("/v1/accounts/{}", acc), Some(ADMIN), "");
    assert_eq!(j.get("usage").unwrap().get("units").unwrap().as_i64(), Some(5));
    srv.stop();
}

#[test]
fn errors() {
    let dir = Dir::new("errors");
    let srv = Server::start(&dir, |c| c.max_request = 1000);
    let mut c = srv.client();
    let (acc, key) = account(&mut c, r#"{"quotas":{"filesystems":1}}"#);
    let fs = new_fs(&mut c, &key);
    let p = format!("/v1/filesystems/{}/run", fs);
    let cases: Vec<(&str, String, Option<&str>, String, u16)> = vec![
        ("POST", p.clone(), None, r#"{"command":"ls"}"#.into(), 401),
        ("POST", p.clone(), Some("qdbk_nope"), r#"{"command":"ls"}"#.into(), 401),
        ("POST", p.clone(), Some(ADMIN), r#"{"command":"ls"}"#.into(), 401),
        ("POST", "/v1/accounts".into(), Some(&key), "".into(), 401),
        ("POST", p.clone(), Some(&key), "{".into(), 400),
        ("POST", p.clone(), Some(&key), "[]".into(), 400),
        ("POST", p.clone(), Some(&key), r#"{"command":1}"#.into(), 400),
        ("POST", p.clone(), Some(&key), r#"{"command":"ls","what":1}"#.into(), 400),
        ("POST", p.clone(), Some(&key), r#"{"command":"ls","stdin_base64":"!"}"#.into(), 400),
        ("POST", p.clone(), Some(&key), r#"{"command":"ls","limits":{"time_ms":0}}"#.into(), 400),
        ("POST", p.clone(), Some(&key), r#"{"command":"ls","session":"s_nope"}"#.into(), 404),
        ("POST", "/v1/filesystems/0123456789abcdef0123456789abcdef/run".into(), Some(&key), r#"{"command":"ls"}"#.into(), 404),
        ("POST", "/v1/filesystems".into(), Some(&key), "".into(), 507),
        ("PUT", p.clone(), Some(&key), "".into(), 405),
        ("GET", "/v2/health".into(), None, "".into(), 404),
        ("PATCH", format!("/v1/accounts/{}", acc), Some(ADMIN), r#"{"quotas":{"what":1}}"#.into(), 400),
    ];
    for (m, path, k, body, want) in cases {
        let (s, j) = call(&mut c, m, &path, k, &body);
        assert_eq!(s, want, "{} {} {}: {:?}", m, path, body, j);
        assert!(j.get("error").and_then(Json::as_str).is_some(), "{:?}", j);
    }
    // Too large: answered, and the connection closed.
    let big = format!(r#"{{"command":"echo {}"}}"#, "x".repeat(2000));
    let (s, _) = call(&mut c, "POST", &p, Some(&key), &big);
    assert_eq!(s, 413);
    // A quota inside a unit: it ran, so 200, stopped by the quota.
    let (s, _) = call(&mut c, "PATCH", &format!("/v1/accounts/{}", acc), Some(ADMIN), r#"{"quotas":{"text_bytes":10}}"#);
    assert_eq!(s, 200);
    let j = cmd(&mut c, &key, &fs, "echo 12345 > a; echo 1234567890 > b");
    assert_eq!(j.get("stopped").unwrap().as_str(), Some("quota"), "{:?}", j);
    assert_eq!(out(&cmd(&mut c, &key, &fs, "ls")), "");
    srv.stop();
}

#[test]
fn sessions_hold_the_directory_and_variables() {
    let dir = Dir::new("sessions");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    let (_, key) = account(&mut c, "");
    let fs = new_fs(&mut c, &key);
    let other = new_fs(&mut c, &key);
    cmd(&mut c, &key, &fs, "mkdir -p a/b");
    let (s, j) = call(&mut c, "POST", &format!("/v1/filesystems/{}/sessions", fs), Some(&key), "");
    assert_eq!(s, 201);
    let sid = j.get("session").unwrap().as_str().unwrap().to_string();
    run(&mut c, &key, &fs, Json::obj().with("command", "cd a/b; X=5").with("session", sid.as_str()));
    let j = run(&mut c, &key, &fs, Json::obj().with("command", "pwd; echo $X").with("session", sid.as_str()));
    assert_eq!(out(&j), "/a/b\n5\n");
    // Without it, a unit starts fresh at /.
    assert_eq!(out(&cmd(&mut c, &key, &fs, "pwd; echo \"[$X]\"")), "/\n[]\n");
    // A session belongs to its filesystem.
    let (s, _) = call(&mut c, "POST", &format!("/v1/filesystems/{}/run", other), Some(&key), &Json::obj().with("command", "pwd").with("session", sid.as_str()).to_string());
    assert_eq!(s, 404);
    let (s, _) = call(&mut c, "DELETE", &format!("/v1/filesystems/{}/sessions/{}", fs, sid), Some(&key), "");
    assert_eq!(s, 200);
    let (s, _) = call(&mut c, "POST", &format!("/v1/filesystems/{}/run", fs), Some(&key), &Json::obj().with("command", "pwd").with("session", sid.as_str()).to_string());
    assert_eq!(s, 404);
    srv.stop();
}

#[test]
fn output_is_capped_or_streamed() {
    let dir = Dir::new("stream");
    let srv = Server::start(&dir, |c| c.output_cap = 1000);
    let mut c = srv.client();
    let (_, key) = account(&mut c, "");
    let fs = new_fs(&mut c, &key);
    let j = cmd(&mut c, &key, &fs, "seq 100000");
    assert_eq!(j.get("truncated").unwrap().as_bool(), Some(true));
    assert_eq!(out(&j).len(), 1000);
    assert!(out(&j).starts_with("1\n2\n3\n"));
    // Streamed: everything, in lines of JSON.
    let mut lines = Vec::new();
    let mut partial = Vec::new();
    let body = Json::obj().with("command", "seq 100000; echo oops >&2; printf '\\377'").with("stream", true).to_string();
    let r = c
        .request_with(
            "POST",
            &format!("/v1/filesystems/{}/run", fs),
            Some(&key),
            body.as_bytes(),
            Some(&mut |chunk: &[u8]| {
                partial.extend_from_slice(chunk);
                while let Some(i) = partial.iter().position(|b| *b == b'\n') {
                    let line: Vec<u8> = partial.drain(..=i).collect();
                    lines.push(Json::parse(&line).unwrap());
                }
            }),
        )
        .unwrap();
    assert_eq!(r.status, 200);
    let mut stdout = Vec::new();
    let mut stderr = String::new();
    for l in &lines[..lines.len() - 1] {
        if let Some(s) = l.get("stdout").and_then(Json::as_str) {
            stdout.extend_from_slice(s.as_bytes());
        } else if let Some(s) = l.get("stdout_base64").and_then(Json::as_str) {
            stdout.extend_from_slice(&unbase64(s).unwrap());
        } else if let Some(s) = l.get("stderr").and_then(Json::as_str) {
            stderr.push_str(s);
        }
    }
    let mut want: Vec<u8> = (1..=100000).map(|i| format!("{}\n", i)).collect::<String>().into_bytes();
    want.push(0xff);
    assert_eq!(stdout, want);
    assert_eq!(stderr, "oops\n");
    let last = lines.last().unwrap();
    assert_eq!(last.get("done").unwrap().as_bool(), Some(true));
    assert_eq!(last.get("exit_status").unwrap().as_i64(), Some(0));
    // The connection is still good.
    assert_eq!(out(&cmd(&mut c, &key, &fs, "echo again")), "again\n");
    srv.stop();
}

#[test]
fn a_client_that_stops_reading_a_stream_is_gone() {
    let dir = Dir::new("noread");
    let srv = Server::start(&dir, |c| c.write_timeout = Duration::from_millis(300));
    let mut c = srv.client();
    let (_, key) = account(&mut c, "");
    let fs = new_fs(&mut c, &key);
    cmd(&mut c, &key, &fs, "echo keep > k.txt");
    // Endless output, never read; the connection stays open.
    let mut s = std::net::TcpStream::connect(&srv.addr).unwrap();
    let body = r#"{"command":"rm k.txt; while true; do echo xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; done","stream":true}"#;
    use std::io::Write;
    write!(s, "POST /v1/filesystems/{}/run HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {}\r\nContent-Length: {}\r\n\r\n{}", fs, key, body.len(), body).unwrap();
    // It has started once the answer's head comes; then nothing more is
    // read.
    use std::io::Read;
    let mut head = [0u8; 64];
    s.read_exact(&mut head).unwrap();
    assert!(head.starts_with(b"HTTP/1.1 200"));
    let started = Instant::now();
    // The writer is stopped as its client gone, and the filesystem is
    // as it was.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let j = cmd(&mut c, &key, &fs, "ls");
        if out(&j) == "k.txt\n" {
            break;
        }
        assert!(Instant::now() < deadline, "{:?}", j);
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(started.elapsed() < Duration::from_secs(5));
    drop(s);
    srv.stop();
}

#[test]
fn limits_stop_units_and_leave_nothing() {
    let dir = Dir::new("limits");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    let (acc, key) = account(&mut c, "");
    let fs = new_fs(&mut c, &key);
    cmd(&mut c, &key, &fs, "echo keep > k.txt");
    let t = Instant::now();
    let j = run(&mut c, &key, &fs, Json::obj().with("command", "rm k.txt; while true; do echo x >> x.txt; done").with("limits", Json::obj().with("time_ms", 200)));
    let took = t.elapsed();
    assert_eq!(j.get("stopped").unwrap().as_str(), Some("deadline"));
    assert_eq!(j.get("exit_status").unwrap().as_i64(), Some(124));
    assert!(took < Duration::from_millis(250), "{:?}", took);
    assert_eq!(out(&cmd(&mut c, &key, &fs, "ls")), "k.txt\n");
    // A caller may not raise its limits.
    call(&mut c, "PATCH", &format!("/v1/accounts/{}", acc), Some(ADMIN), r#"{"limits":{"commands":10}}"#);
    let j = run(&mut c, &key, &fs, Json::obj().with("command", "echo 1 > a; echo 2 > b; for i in 1 2 3 4 5 6 7 8 9 10 11; do :; done").with("limits", Json::obj().with("commands", 1000)));
    assert_eq!(j.get("stopped").unwrap().as_str(), Some("command_limit"));
    assert_eq!(out(&cmd(&mut c, &key, &fs, "ls")), "k.txt\n");
    srv.stop();
}

#[test]
fn a_caller_going_away_stops_its_unit() {
    let dir = Dir::new("gone");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    let (_, key) = account(&mut c, "");
    let fs = new_fs(&mut c, &key);
    cmd(&mut c, &key, &fs, "echo keep > k.txt");
    let addr = srv.addr.clone();
    let (k2, f2) = (key.clone(), fs.clone());
    let t = std::thread::spawn(move || {
        let mut c = Client::new(&addr);
        let _ = c.stream().unwrap().set_read_timeout(Some(Duration::from_millis(200)));
        let _ = c.request("POST", &format!("/v1/filesystems/{}/run", f2), Some(&k2), br#"{"command":"rm k.txt; while true; do echo x >> x.txt; done"}"#);
        // Timed out: the connection is dropped here.
    });
    t.join().unwrap();
    // The unit is stopped soon after, and left nothing.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let j = cmd(&mut c, &key, &fs, "ls");
        if out(&j) == "k.txt\n" {
            break;
        }
        assert!(Instant::now() < deadline, "{:?}", j);
        std::thread::sleep(Duration::from_millis(20));
    }
    // Its usage is counted just after the filesystem is let go.
    loop {
        let (_, j) = call(&mut c, "GET", &format!("/v1/filesystems/{}", fs), Some(&key), "");
        if j.get("usage").unwrap().get("stopped").unwrap().as_i64() == Some(1) {
            break;
        }
        assert!(Instant::now() < deadline, "{:?}", j);
        std::thread::sleep(Duration::from_millis(20));
    }
    srv.stop();
}

#[test]
fn too_many_units_of_one_account() {
    let dir = Dir::new("many");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    let (_, key) = account(&mut c, r#"{"limits":{"units_at_once":1}}"#);
    let fs = new_fs(&mut c, &key);
    let addr = srv.addr.clone();
    let (k2, f2) = (key.clone(), fs.clone());
    let t = std::thread::spawn(move || {
        let mut c = Client::new(&addr);
        let r = c.request("POST", &format!("/v1/filesystems/{}/run", f2), Some(&k2), br#"{"command":"while true; do :; done","limits":{"time_ms":500}}"#).unwrap();
        assert_eq!(r.status, 200);
    });
    std::thread::sleep(Duration::from_millis(150));
    let (s, j) = call(&mut c, "POST", &format!("/v1/filesystems/{}/run", fs), Some(&key), r#"{"command":"true"}"#);
    assert_eq!(s, 429, "{:?}", j);
    t.join().unwrap();
    assert_eq!(cmd(&mut c, &key, &fs, "true").get("exit_status").unwrap().as_i64(), Some(0));
    srv.stop();
}

#[test]
fn accounts_and_usage_outlive_a_restart() {
    let dir = Dir::new("restart");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    let (acc, key) = account(&mut c, r#"{"limits":{"time_ms":1234}}"#);
    let (gone_acc, gone_key) = account(&mut c, "");
    let fs = new_fs(&mut c, &key);
    let gone_fs = new_fs(&mut c, &gone_key);
    cmd(&mut c, &key, &fs, "echo kept > a.txt");
    cmd(&mut c, &key, &fs, "cat a.txt");
    call(&mut c, "DELETE", &format!("/v1/accounts/{}", gone_acc), Some(ADMIN), "");
    srv.stop();
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    assert_eq!(out(&cmd(&mut c, &key, &fs, "cat a.txt")), "kept\n");
    let (_, j) = call(&mut c, "GET", &format!("/v1/accounts/{}", acc), Some(ADMIN), "");
    assert_eq!(j.get("usage").unwrap().get("units").unwrap().as_i64(), Some(3), "{:?}", j);
    assert_eq!(j.get("settings").unwrap().get("limits").unwrap().get("time_ms").unwrap().as_i64(), Some(1234));
    let (s, _) = call(&mut c, "GET", "/v1/filesystems", Some(&gone_key), "");
    assert_eq!(s, 401);
    assert!(!srv.svc.host().exists(&gone_fs));
    srv.stop();
}

#[test]
fn a_damaged_or_missing_registry_is_refused() {
    let dir = Dir::new("registry");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    let (_, key) = account(&mut c, "");
    let fs = new_fs(&mut c, &key);
    cmd(&mut c, &key, &fs, "echo kept > a.txt");
    srv.stop();
    let reg = dir.0.join("server").join("registry.log");
    let good = std::fs::read(&reg).unwrap();
    // A last line cut short by a crash is dropped.
    let mut cut = good.clone();
    cut.extend_from_slice(b"{\"op\":\"fs\",\"id\":\"0123");
    std::fs::write(&reg, &cut).unwrap();
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    assert_eq!(out(&cmd(&mut c, &key, &fs, "cat a.txt")), "kept\n");
    srv.stop();
    // A bad line before the last: the server will not start, and
    // removes nothing.
    let mut bad = b"{not json\n".to_vec();
    bad.extend_from_slice(&std::fs::read(&reg).unwrap());
    std::fs::write(&reg, &bad).unwrap();
    let e = Service::open(ServiceConfig::new(&dir.0, ADMIN)).err().unwrap();
    assert!(e.contains("line 1 is damaged"), "{}", e);
    // Missing, with filesystems there.
    std::fs::remove_file(&reg).unwrap();
    let e = Service::open(ServiceConfig::new(&dir.0, ADMIN)).err().unwrap();
    assert!(e.contains("is missing or empty"), "{}", e);
    std::fs::write(&reg, &good).unwrap();
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    assert_eq!(out(&cmd(&mut c, &key, &fs, "cat a.txt")), "kept\n");
    srv.stop();
}

#[test]
fn one_account_never_sees_another() {
    let dir = Dir::new("isolation");
    let srv = Server::start(&dir, |_| {});
    let mut c = srv.client();
    let (_, ka) = account(&mut c, "");
    let (_, kb) = account(&mut c, "");
    let fa = new_fs(&mut c, &ka);
    let fb = new_fs(&mut c, &kb);
    cmd(&mut c, &ka, &fa, "echo MARKER-A > m.txt");
    cmd(&mut c, &kb, &fb, "echo MARKER-B > m.txt");
    let paths = |f: &str| {
        vec![
            format!("/v1/filesystems/{}", f),
            format!("/v1/filesystems/{}/run", f),
            format!("/v1/filesystems/{}/sessions", f),
            format!("/v1/filesystems/{}/../{}/run", fa, f),
            format!("/v1/filesystems/{}/run", f.to_uppercase()),
        ]
    };
    for p in paths(&fb) {
        for m in ["GET", "POST", "DELETE"] {
            let r = c.request(m, &p, Some(&ka), br#"{"command":"cat m.txt; ls /; cat ../../m.txt"}"#).unwrap();
            let body = String::from_utf8_lossy(&r.body);
            assert!(!body.contains("MARKER-B") && !body.contains(&fb), "{} {}: {}", m, p, body);
            // The same as for a filesystem that does not exist.
            let none = c.request(m, &p.replace(&fb, "0123456789abcdef0123456789abcdef"), Some(&ka), br#"{"command":"cat m.txt; ls /; cat ../../m.txt"}"#).unwrap();
            assert_eq!(r.status, none.status, "{} {}", m, p);
        }
    }
    for line in ["cat /../m.txt", "cat ../../../fs/*/*/m.txt", "ls -a /..", "find / -name '*'", "grep -r MARKER-B /"] {
        let j = cmd(&mut c, &ka, &fa, line);
        assert!(!j.to_string().contains("MARKER-B"), "{}: {:?}", line, j);
    }
    assert_eq!(out(&cmd(&mut c, &kb, &fb, "cat m.txt")), "MARKER-B\n");
    srv.stop();
}
