//! Readers and writers, and units that are stopped: a stopped unit leaves
//! nothing, on a database in memory and on disk.

use qdb::engine::{classify, run_unit, Access, Ending, Kind};
use qdb::unit::{self, Reason, Stop};
use qdb::{execute, Config, Db, Session};
use std::path::PathBuf;
use std::time::{Duration, Instant};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let path = std::env::temp_dir().join(format!("qdb-units-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Dir(path)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn readers_and_writers_by_their_text() {
    let readers = [
        "cat a.txt",
        "grep -rn x . | head -n 3",
        "ls > /dev/null 2>&1",
        "find . -name '*.c' -exec grep -l y {} +",
        "sed -n 'p' a.txt",
        "x=1; echo $x",
        "cd src; ls -la; cd -",
        "echo $(cat a.txt | wc -l)",
        "for f in *.txt; do wc -l $f; done",
        "find . -type f | xargs grep -c x",
        "awk '{ n += NF } END { print n }' a.txt",
        "sort -u a.txt | uniq -c",
        "tee < a.txt",
        "while read l; do echo $l; done < a.txt",
        "echo 'never > closed",
    ];
    for r in readers {
        assert_eq!(classify(r.as_bytes()), Kind::Reader, "{}", r);
    }
    let writers = [
        "echo a > f",
        "echo a >> f",
        "cat a 2> err.txt",
        "sed -i s/a/b/ f",
        "$cmd f",
        "rm f",
        "mkdir -p d",
        "find . -exec rm {} \\;",
        "find . -name x -exec rm -f {} +",
        "ls | xargs rm",
        "tee f < a.txt",
        "echo $(rm f)",
        "for f in *; do cat $f; done > out.txt",
        "if true; then echo x; fi > out.txt",
        "sed -n p $f",
        "sed -n p *.txt",
        "cat a > $out",
        "e{cho,} x",
        "nosuchcommand",
    ];
    for w in writers {
        assert_eq!(classify(w.as_bytes()), Kind::Writer, "{}", w);
    }
}

fn snapshot(db: &Db) -> Vec<(Vec<u8>, Option<Vec<u8>>, i64)> {
    db.snapshot().into_iter().map(|(p, c)| {
        let t = db.mtime(db.resolve(&[b"/", &p[..]].concat()).unwrap());
        (p, c, t)
    }).collect()
}

fn setup(db: &mut Db) {
    let script = "mkdir -p src/a src/b; echo one > src/a/x.txt; seq 1 1000 > src/b/n.txt; echo keep > top.txt; cp src/b/n.txt copy.txt";
    let out = execute(db, script.as_bytes());
    assert!(out.stderr.is_empty(), "{}", String::from_utf8_lossy(&out.stderr));
}

/// Units that change everything they can, then run into the command limit.
const BUSY: &[&str] = &[
    "rm -r src; mv top.txt moved.txt; echo new > top.txt; while true; do echo x >> top.txt; done",
    "mkdir -p n/m; mv src n/m/; cp copy.txt n/c.txt; sed -i 's/1/one/' n/c.txt copy.txt; rm -r n/m/src/a; while true; do :; done",
    "echo a > copy.txt; echo b >> copy.txt; cp copy.txt src/a/x.txt; mv src/a src/c; touch src/c/new.txt; rm -r src/c; while :; do mkdir -p q/r; rm -r q; done",
    "mkdir t; mv src t/; mkdir src; mv t/src/b src/; rm -r t; echo z > src/b/n.txt; until false; do touch top.txt; done",
];

fn small_limit(cfg: Config) -> Config {
    Config { command_limit: 2000, ..cfg }
}

#[test]
fn a_unit_at_the_command_limit_leaves_nothing() {
    for busy in BUSY {
        let mut db = Db::in_memory(small_limit(Config::default()));
        db.set_clock(Some(1_700_000_000_000_000_000));
        setup(&mut db);
        let before = snapshot(&db);
        db.set_clock(Some(1_800_000_000_000_000_000));
        let out = execute(&mut db, busy.as_bytes());
        assert_eq!(out.code, 3, "{}: {}", busy, String::from_utf8_lossy(&out.stderr));
        assert_eq!(snapshot(&db), before, "{}", busy);
        db.check().unwrap();
        // It takes the next unit.
        let out = execute(&mut db, b"echo after > after.txt; cat after.txt");
        assert_eq!(out.stdout, b"after\n");
    }
}

#[test]
fn a_stopped_unit_leaves_nothing_on_disk() {
    for (i, busy) in BUSY.iter().enumerate() {
        let dir = Dir::new(&format!("limit{}", i));
        let before;
        {
            let (mut db, _) = Db::open_dir(&dir.0, small_limit(Config::default())).unwrap();
            setup(&mut db);
            before = snapshot(&db);
            let out = execute(&mut db, busy.as_bytes());
            assert_eq!(out.code, 3);
            assert_eq!(snapshot(&db), before, "{}", busy);
            db.check().unwrap();
            let out = execute(&mut db, b"echo after > after.txt");
            assert_eq!(out.code, 0);
            db.close().unwrap();
        }
        let (db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
        db.check().unwrap();
        let mut after = snapshot(&db);
        after.retain(|(p, _, _)| p != b"after.txt" as &[u8]);
        let strip = |v: &[(Vec<u8>, Option<Vec<u8>>, i64)]| v.iter().map(|(p, c, _)| (p.clone(), c.clone())).collect::<Vec<_>>();
        // The root's time moved with after.txt; the rest is as it was.
        assert_eq!(strip(&after), strip(&before), "{}", busy);
        assert_eq!(db.content(db.resolve(b"/after.txt").unwrap()).as_ref(), b"after\n");
    }
}

#[test]
fn a_unit_stopped_from_outside_leaves_nothing_and_stops_soon() {
    unit::quiet_panics();
    for (i, busy) in BUSY.iter().enumerate() {
        let dir = Dir::new(&format!("stop{}", i));
        let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
        setup(&mut db);
        let before = snapshot(&db);
        let stop = Stop::new();
        let s2 = stop.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            s2.request(Reason::Deadline);
            Instant::now()
        });
        let mut sess = Session::new();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let end = {
            let _in = unit::enter(&stop);
            run_unit(Access::Write(&mut db), &mut sess, busy.as_bytes(), &mut out, &mut err)
        };
        let done = Instant::now();
        let asked = t.join().unwrap();
        assert_eq!(end.ending, Ending::Stopped(Reason::Deadline), "{}: {}", busy, String::from_utf8_lossy(&err));
        assert_eq!(end.code, 124);
        assert!(done.duration_since(asked) < Duration::from_millis(20), "{}: stopped {:?} after the request", busy, done.duration_since(asked));
        assert_eq!(snapshot(&db), before, "{}", busy);
        db.check().unwrap();
        assert_eq!(execute(&mut db, b"echo after > after.txt").code, 0);
        db.close().unwrap();
        let (db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
        db.check().unwrap();
    }
}

#[test]
fn a_reader_that_writes_is_stopped_having_changed_nothing() {
    unit::quiet_panics();
    let mut db = Db::in_memory(Config::default());
    setup(&mut db);
    let before = snapshot(&db);
    let mut sess = Session::new();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let end = run_unit(Access::Read(&db), &mut sess, b"cat top.txt; rm top.txt", &mut out, &mut err);
    assert!(matches!(end.ending, Ending::Fault(_)), "{:?}", end);
    assert_eq!(out, b"keep\n");
    assert_eq!(snapshot(&db), before);
}

#[test]
fn readers_run_together() {
    let mut db = Db::in_memory(Config::default());
    setup(&mut db);
    let db = &db;
    let want = execute(&mut db.clone(), b"grep -rc 1 src | sort; wc -l copy.txt").stdout;
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..200 {
                    let mut sess = Session::new();
                    let (mut out, mut err) = (Vec::new(), Vec::new());
                    let end = run_unit(Access::Read(db), &mut sess, b"grep -rc 1 src | sort; wc -l copy.txt", &mut out, &mut err);
                    assert_eq!(end.ending, Ending::Ran);
                    assert_eq!(out, want);
                }
            });
        }
    });
}

#[test]
fn a_unit_that_would_pass_a_quota_leaves_nothing() {
    unit::quiet_panics();
    let cases: [(&str, u64, u64, u64); 4] = [
        ("for i in 1 2 3 4 5 6 7 8 9 10 11 12; do mkdir d$i; done", u64::MAX, 12, u64::MAX),
        ("seq 1 100000 > big.txt", 20_000, u64::MAX, u64::MAX),
        ("rm copy.txt; seq 1 3000 > a.txt; seq 1 3000 >> a.txt", 20_000, u64::MAX, u64::MAX),
        ("seq 1 200000 > big.txt", u64::MAX, u64::MAX, 400_000),
    ];
    for (i, (line, text, nodes, disk)) in cases.iter().enumerate() {
        let dir = Dir::new(&format!("quota{}", i));
        let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
        setup(&mut db);
        db.set_quotas(*text, *nodes, *disk);
        let before = snapshot(&db);
        let out = execute(&mut db, line.as_bytes());
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.code, 1, "{}: {}", line, err);
        assert!(err.contains("quota") && err.contains("nothing the unit changed was kept"), "{}", err);
        assert_eq!(snapshot(&db), before, "{}", line);
        db.check().unwrap();
        // Below the quota it goes on.
        assert_eq!(execute(&mut db, b"echo small > s.txt").code, 0);
        db.check().unwrap();
    }
}
