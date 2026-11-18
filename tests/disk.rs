//! A database on disk: what survives closing, and what happens when its
//! files are missing, damaged or from a newer version.

use qdb::{execute, Config, Db};
use std::path::{Path, PathBuf};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let path = std::env::temp_dir().join(format!("qdb-test-{}-{}", name, std::process::id()));
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

/// A small database with a few files, closed.
fn make(dir: &Path) {
    let (mut db, report) = Db::open_dir(dir, Config::default()).unwrap();
    assert!(report.created);
    run(&mut db, "mkdir -p src/deep docs");
    for i in 0..40 {
        run(&mut db, &format!("echo 'line one of file {0} holds needle{0}' > src/f{0}.txt", i));
        run(&mut db, &format!("echo 'a second line, the same in every file' >> src/f{}.txt", i));
    }
    run(&mut db, "echo 'deep down' > src/deep/x.txt");
    run(&mut db, "cp src/f7.txt docs/copy.txt");
    run(&mut db, "rm src/f3.txt");
    run(&mut db, "mv src/f4.txt docs/moved.txt");
    db.close().unwrap();
}

fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.to_string_lossy().ends_with(ext)).collect();
    v.sort();
    v
}

/// Change one byte of a file.
fn flip(path: &Path, at: u64) {
    flip_range(path, at, at + 1);
}

fn flip_range(path: &Path, from: u64, to: u64) {
    let mut bytes = std::fs::read(path).unwrap();
    for b in &mut bytes[from as usize..to as usize] {
        *b ^= 0x55;
    }
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn closing_and_opening_keeps_everything() {
    let dir = Dir::new("keep");
    make(&dir.0);
    let (mut db, report) = Db::open_dir(&dir.0, Config::default()).unwrap();
    assert!(!report.created && report.replayed == 0 && report.messages().is_empty(), "{:?}", report);
    assert_eq!(run(&mut db, "grep -rl needle7 ."), "./docs/copy.txt\n./src/f7.txt\n");
    assert_eq!(run(&mut db, "cat docs/moved.txt"), "line one of file 4 holds needle4\na second line, the same in every file\n");
    assert_eq!(run(&mut db, "find . -type f | wc -l"), "41\n");
    db.check().unwrap();
    // Node ids persist: the file made first still has the id it was given.
    let id = db.resolve(b"src/f0.txt").unwrap();
    db.close().unwrap();
    let (db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    assert_eq!(db.resolve(b"src/f0.txt").unwrap(), id);
}

#[test]
fn a_read_writes_nothing() {
    let dir = Dir::new("reads");
    make(&dir.0);
    let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    let before = db.io_stats();
    run(&mut db, "grep -rn needle .");
    run(&mut db, "cat src/f1.txt | wc -l");
    run(&mut db, "find . -name '*.txt' | head -n 3");
    assert_eq!(db.io_stats(), before);
    // Writes that change nothing write no content: only the record that
    // carries the new modification time.
    run(&mut db, "cp src/f7.txt docs/copy.txt");
    run(&mut db, "sed -i 's/absent/x/' src/f1.txt");
    let after = db.io_stats();
    assert_eq!(after.content_bytes, before.content_bytes);
    assert_eq!(after.commits, before.commits + 2);
    assert!(after.log_bytes - before.log_bytes < 200);
}

#[test]
fn only_one_process_at_a_time() {
    let dir = Dir::new("lock");
    make(&dir.0);
    let (db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    let second = Db::open_dir(&dir.0, Config::default());
    assert!(second.err().is_some_and(|e| e.contains("in use")));
    drop(db);
    Db::open_dir(&dir.0, Config::default()).unwrap();
}

#[test]
fn a_missing_index_is_built_again() {
    let dir = Dir::new("noindex");
    make(&dir.0);
    for f in files(&dir.0, ".idx") {
        std::fs::remove_file(f).unwrap();
    }
    let (mut db, report) = Db::open_dir(&dir.0, Config::default()).unwrap();
    assert_eq!(report.rebuilt.len(), 1, "{:?}", report);
    assert!(report.messages().iter().any(|m| m.contains("built the search index again")));
    assert_eq!(run(&mut db, "grep -rl needle7 ."), "./docs/copy.txt\n./src/f7.txt\n");
    db.check().unwrap();
    db.close().unwrap();
    // And it was saved: the next open has nothing to do.
    let (_, report) = Db::open_dir(&dir.0, Config::default()).unwrap();
    assert!(report.messages().is_empty(), "{:?}", report);
}

#[test]
fn a_damaged_index_is_built_again() {
    // In its footer and in its table: found when it is opened.
    for from_end in [10u64, 120] {
        let dir = Dir::new(&format!("badindex{}", from_end));
        make(&dir.0);
        let idx = files(&dir.0, ".idx").remove(0);
        flip(&idx, std::fs::metadata(&idx).unwrap().len() - from_end);
        let (mut db, report) = Db::open_dir(&dir.0, Config::default()).unwrap();
        assert_eq!(report.rebuilt.len(), 1, "{:?}", report);
        assert_eq!(run(&mut db, "grep -rl needle7 ."), "./docs/copy.txt\n./src/f7.txt\n");
        db.check().unwrap();
    }
    // In a list: found when the list is read. The command fails with the
    // file's name, and the next open builds the index again.
    let dir = Dir::new("badlist");
    {
        // Enough files that a search asks the index instead of reading.
        let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
        let root = db.root();
        for i in 0..4000 {
            db.create_file(root, format!("f{}.txt", i).as_bytes(), format!("line one of file {0} holds needle{0}\n{1}", i, "and many more lines that are the same in every file\n".repeat(60)).as_bytes());
        }
        db.close().unwrap();
    }
    let idx = files(&dir.0, ".idx").remove(0);
    // Past the first block, which is checked when the file is opened.
    flip_range(&idx, 5000, 60000);
    let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    let mut failed = None;
    for i in 0..40 {
        let out = execute(&mut db, format!("grep -rl 'file {} holds' .", i * 97).as_bytes());
        if out.code == 2 {
            failed = Some(String::from_utf8(out.stderr).unwrap());
            break;
        }
    }
    let failed = failed.expect("a search read the damaged lists");
    assert!(failed.contains(".idx") && failed.contains("damaged") && failed.contains("built again"), "{}", failed);
    // The database is stopped.
    assert_eq!(execute(&mut db, b"ls").code, 2);
    drop(db);
    let (mut db, report) = Db::open_dir(&dir.0, Config::default()).unwrap();
    assert_eq!(report.rebuilt.len(), 1, "{:?}", report);
    assert_eq!(run(&mut db, "grep -rl needle777 ."), "./f777.txt\n");
    db.check().unwrap();
}

#[test]
fn damaged_content_is_an_error_that_names_the_file() {
    let dir = Dir::new("badcontent");
    make(&dir.0);
    let seg = files(&dir.0, ".seg").remove(0);
    // Find the text of one file in the data file and change a byte of it.
    let bytes = std::fs::read(&seg).unwrap();
    let needle = b"needle9\n";
    // The last copy: earlier ones are content the file no longer uses.
    let at = bytes.windows(needle.len()).rposition(|w| w == needle).unwrap();
    flip(&seg, at as u64 + 3);
    let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    // Other files read as before.
    assert_eq!(run(&mut db, "cat src/f8.txt | head -n 1"), "line one of file 8 holds needle8\n");
    let out = execute(&mut db, b"cat src/f9.txt");
    let err = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.code, 2);
    assert!(out.stdout.is_empty());
    assert!(err.contains("00000001.seg") && err.contains("damaged"), "{}", err);
    drop(db);
    let (db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    let found = db.check().unwrap_err();
    assert!(found.contains("00000001.seg") && found.contains("damaged"), "{}", found);
}

#[test]
fn a_damaged_tree_or_manifest_is_an_error() {
    let dir = Dir::new("badtree");
    make(&dir.0);
    let ckp = files(&dir.0, ".ckp").remove(0);
    flip(&ckp, 40);
    let err = Db::open_dir(&dir.0, Config::default()).err().unwrap();
    assert!(err.contains("tree-") && err.contains("damaged"), "{}", err);
    flip(&ckp, 40);
    Db::open_dir(&dir.0, Config::default()).unwrap();
    flip(&dir.0.join("MANIFEST"), 13);
    let err = Db::open_dir(&dir.0, Config::default()).err().unwrap();
    assert!(err.contains("MANIFEST"), "{}", err);
}

#[test]
fn a_newer_format_is_refused() {
    for (version, said) in [(3u8, "newer version"), (1u8, "earlier version")] {
    for ext in [".seg", ".ckp", "MANIFEST"] {
        let dir = Dir::new(&format!("version{}{}", version, ext.trim_start_matches('.')));
        make(&dir.0);
        let file = files(&dir.0, ext).remove(0);
        // The format version is the four bytes after the magic. Give it
        // a version from the future, with a checksum that is right, by
        // asking the engine's own checksum.
        let mut bytes = std::fs::read(&file).unwrap();
        bytes[8] = version;
        let covered = if ext == ".seg" { 28 } else { 16 };
        let sum = qdb::disk::crc(&bytes[..covered]).to_le_bytes();
        bytes[covered..covered + 4].copy_from_slice(&sum);
        std::fs::write(&file, bytes).unwrap();
        let err = Db::open_dir(&dir.0, Config::default()).err().unwrap();
        assert!(err.contains(said), "{}: {}", ext, err);
    }
    }
}

#[test]
fn a_directory_that_is_not_a_database_is_refused() {
    let dir = Dir::new("notdb");
    std::fs::create_dir_all(&dir.0).unwrap();
    std::fs::write(dir.0.join("notes.txt"), "hello").unwrap();
    let err = Db::open_dir(&dir.0, Config::default()).err().unwrap();
    assert!(err.contains("not a qdb database"), "{}", err);
    assert!(dir.0.join("notes.txt").exists());
}

#[test]
fn compacting_gives_back_space() {
    let dir = Dir::new("compact");
    let (mut db, _) = Db::open_dir(&dir.0, Config::default()).unwrap();
    for round in 0..30 {
        for f in 0..10 {
            run(&mut db, &format!("echo 'round {} of file {} with some words to fill the line' > f{}.txt", round, f, f));
        }
    }
    let before = db.disk_stats();
    assert!(before.unreclaimed > before.text);
    let tree = db.snapshot();
    db.compact().unwrap();
    let after = db.disk_stats();
    assert_eq!(after.unreclaimed, 0, "{:?}", after);
    assert_eq!(after.text, after.raw_text);
    assert_eq!(db.snapshot(), tree);
    db.check().unwrap();
    db.close().unwrap();
    let (db, report) = Db::open_dir(&dir.0, Config::default()).unwrap();
    assert!(report.messages().is_empty());
    assert_eq!(db.snapshot(), tree);
    assert_eq!(db.disk_stats().data_files, 1);
}
