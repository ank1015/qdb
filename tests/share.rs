//! Files that share their content after `cp`, and modification times.

use qdb::{execute, Config, Db};
use std::path::PathBuf;

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let path = std::env::temp_dir().join(format!("qdb-share-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Dir(path)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(db: &mut Db, command: &str) -> String {
    let out = execute(db, command.as_bytes());
    assert!(out.stderr.is_empty(), "{}: {}", command, String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

const T: i64 = 1_700_000_000_000_000_000;

fn mtime(db: &Db, path: &str) -> i64 {
    db.mtime(db.resolve(path.as_bytes()).unwrap()) - T
}

fn body(lines: usize, tag: &str) -> String {
    (0..lines).map(|i| format!("{} line {} of a file that is long enough to span pieces\n", tag, i)).collect()
}

/// Storage settings under which a file of a few kilobytes has many pieces.
fn small_pieces() -> Config {
    Config { piece: 256, ..Config::default() }
}

fn fill(db: &mut Db, path: &str, text: &str) {
    let (parent, name) = db.resolve_parent(path.as_bytes()).unwrap();
    db.create_file(parent, &name, text.as_bytes());
    db.commit();
}

#[test]
fn a_copy_writes_a_record_and_no_content() {
    let dir = Dir::new("record");
    let (mut db, _) = Db::open_dir(&dir.0, small_pieces()).unwrap();
    let text = body(400, "a");
    fill(&mut db, "a.txt", &text);
    let before = db.io_stats();
    run(&mut db, "cp a.txt b.txt");
    let after = db.io_stats();
    assert_eq!(after.content_bytes, before.content_bytes);
    assert_eq!(after.syncs, before.syncs + 1);
    assert_eq!(run(&mut db, "cat b.txt"), text);
    db.check().unwrap();
    // A second copy onto the same file writes only the new time.
    let before = db.io_stats();
    run(&mut db, "cp a.txt b.txt");
    assert!(db.io_stats().log_bytes - before.log_bytes < 64);
    db.check().unwrap();
}

#[test]
fn changing_one_side_leaves_the_other() {
    for reopen in [false, true] {
        let dir = Dir::new(if reopen { "sides-reopen" } else { "sides" });
        let (mut db, _) = Db::open_dir(&dir.0, small_pieces()).unwrap();
        let text = body(300, "x");
        fill(&mut db, "a.txt", &text);
        run(&mut db, "mkdir d");
        run(&mut db, "cp a.txt d/b.txt");
        run(&mut db, "cp d/b.txt c.txt");
        let again = |db: Db| -> Db {
            if !reopen {
                return db;
            }
            db.close().unwrap();
            let (db, report) = Db::open_dir(&dir.0, small_pieces()).unwrap();
            assert!(report.messages().is_empty(), "{:?}", report.messages());
            db
        };
        db = again(db);
        // Append to the copy, change a line in the middle of the source,
        // empty the third.
        run(&mut db, "echo 'one more' >> d/b.txt");
        db = again(db);
        assert_eq!(run(&mut db, "cat a.txt"), text);
        run(&mut db, "sed -i 's/x line 150 /CHANGED /' a.txt");
        db = again(db);
        assert_eq!(run(&mut db, "cat d/b.txt"), format!("{}one more\n", text));
        assert_eq!(run(&mut db, "cat c.txt"), text);
        assert_eq!(run(&mut db, "grep -c CHANGED a.txt"), "1\n");
        // A search lists every file that holds a matching piece.
        assert_eq!(run(&mut db, "grep -rl 'x line 7 ' ."), "./a.txt\n./c.txt\n./d/b.txt\n");
        assert_eq!(run(&mut db, "grep -rl 'x line 150 ' ."), "./c.txt\n./d/b.txt\n");
        assert_eq!(run(&mut db, "grep -rl 'one more' ."), "./d/b.txt\n");
        db.check().unwrap();
        // The content goes only when its last user does.
        run(&mut db, "rm a.txt");
        db = again(db);
        assert_eq!(run(&mut db, "cat c.txt"), text);
        run(&mut db, "rm c.txt");
        assert_eq!(run(&mut db, "cat d/b.txt"), format!("{}one more\n", text));
        db.check().unwrap();
        db.compact().unwrap();
        db.check().unwrap();
        assert_eq!(run(&mut db, "cat d/b.txt"), format!("{}one more\n", text));
        db.close().unwrap();
    }
}

#[test]
fn sharing_survives_compaction_and_cleaning() {
    let dir = Dir::new("compact");
    let cfg = Config { piece: 256, segment: 64 * 1024, ..Config::default() };
    let (mut db, _) = Db::open_dir(&dir.0, cfg.clone()).unwrap();
    let text = body(200, "s");
    fill(&mut db, "a.txt", &text);
    run(&mut db, "mkdir copies");
    for i in 0..20 {
        run(&mut db, &format!("cp a.txt copies/c{}.txt", i));
    }
    // Churn, so that content is moved out of data files as they empty.
    for i in 0..300 {
        run(&mut db, &format!("echo '{}' > churn{}.txt", body(20, "churn").trim_end(), i % 5));
    }
    db.check().unwrap();
    let stats = db.disk_stats();
    assert!(stats.text < 2 * text.len() as u64 + 5 * 2000, "copies are not stored again: {} bytes of text for a file of {}", stats.text, text.len());
    db.compact().unwrap();
    db.check().unwrap();
    let stats = db.disk_stats();
    assert!(stats.text < 2 * text.len() as u64 + 5 * 2000, "compaction keeps copies shared: {}", stats.text);
    for i in 0..20 {
        assert_eq!(run(&mut db, &format!("cat copies/c{}.txt", i)), text);
    }
    db.close().unwrap();
    let (db, _) = Db::open_dir(&dir.0, cfg).unwrap();
    db.check().unwrap();
}

#[test]
fn times_change_as_on_a_filesystem_and_survive_reopening() {
    let dir = Dir::new("times");
    let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    let at = |db: &mut Db, n: i64| db.set_clock(Some(T + n));
    at(&mut db, 1);
    run(&mut db, "mkdir d");
    run(&mut db, "echo one > d/a.txt");
    at(&mut db, 2);
    run(&mut db, "echo two > top.txt");
    assert_eq!((mtime(&db, "d"), mtime(&db, "d/a.txt"), mtime(&db, "top.txt"), mtime(&db, "/")), (1, 1, 2, 2));
    // Appending changes the file, not its folder.
    at(&mut db, 3);
    run(&mut db, "echo more >> d/a.txt");
    assert_eq!((mtime(&db, "d"), mtime(&db, "d/a.txt")), (1, 3));
    // Writing the same content again writes no content and sets the time.
    at(&mut db, 4);
    let before = db.io_stats();
    run(&mut db, "echo two > top.txt");
    assert_eq!(db.io_stats().content_bytes, before.content_bytes);
    assert_eq!(mtime(&db, "top.txt"), 4);
    // A copy is new; a move keeps the node's time and changes both folders.
    at(&mut db, 5);
    run(&mut db, "cp top.txt d/copy.txt");
    assert_eq!((mtime(&db, "d/copy.txt"), mtime(&db, "top.txt"), mtime(&db, "d")), (5, 4, 5));
    at(&mut db, 6);
    run(&mut db, "mv d/a.txt moved.txt");
    assert_eq!((mtime(&db, "moved.txt"), mtime(&db, "d"), mtime(&db, "/")), (3, 6, 6));
    at(&mut db, 7);
    run(&mut db, "rm d/copy.txt");
    assert_eq!((mtime(&db, "d"), mtime(&db, "/")), (7, 6));
    at(&mut db, 8);
    run(&mut db, "touch moved.txt");
    assert_eq!(mtime(&db, "moved.txt"), 8);
    // An empty file emptied again still gets the time.
    run(&mut db, "touch empty.txt");
    at(&mut db, 9);
    run(&mut db, "grep nothing moved.txt > empty.txt");
    assert_eq!(mtime(&db, "empty.txt"), 9);
    let want: Vec<i64> = ["/", "d", "moved.txt", "top.txt", "empty.txt"].iter().map(|p| mtime(&db, p)).collect();
    db.close().unwrap();
    // After a clean close, and after a crash (the log applied again).
    let (db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    assert_eq!(want, ["/", "d", "moved.txt", "top.txt", "empty.txt"].iter().map(|p| mtime(&db, p)).collect::<Vec<i64>>());
    drop(db);
    let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    db.set_clock(Some(T + 20));
    run(&mut db, "echo late >> top.txt");
    run(&mut db, "mkdir e");
    // The files as they are now, with the database not closed.
    let image = Dir::new("times-image");
    std::fs::create_dir_all(&image.0).unwrap();
    for entry in std::fs::read_dir(&dir.0).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), image.0.join(entry.file_name())).unwrap();
    }
    drop(db);
    let (db, report) = Db::open_dir(&image.0, Config::default()).unwrap();
    assert!(report.replayed > 0);
    assert_eq!((mtime(&db, "top.txt"), mtime(&db, "e"), mtime(&db, "/"), mtime(&db, "moved.txt")), (20, 20, 20, 8));
    db.check().unwrap();
}
