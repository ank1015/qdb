//! The search index: each way storage can answer "which files can match",
//! and the same output with the index on and off.

use qdb::query::Query;
use qdb::tree::Candidates;
use qdb::{execute, Db};

/// 400 files in 20 folders: `rareword` in two of them, `midword` in a
/// third of them, `commonword` in nine of ten.
fn tree(index: bool) -> Db {
    let mut db = if index { Db::new() } else { Db::without_index() };
    let root = db.root();
    for d in 0..20 {
        let dir = db.create_folder(root, format!("dir{:02}", d).as_bytes());
        for f in 0..20 {
            let n = d * 20 + f;
            let mut text = format!("file {} line one\nsecond Line of {}\n", n, n);
            if n == 77 || n == 301 {
                text.push_str("here is a RareWord indeed\n");
            }
            if n % 3 == 0 {
                text.push_str("a midword appears\nand midword again\n");
            }
            if n % 10 != 0 {
                text.push_str("the commonword\n");
            }
            db.create_file(dir, format!("f{:02}.txt", f).as_bytes(), text.as_bytes());
        }
    }
    db
}

#[test]
fn three_kinds_of_answer() {
    let db = tree(true);
    let root = db.root();
    // Few files: a list, in the order a walk visits them.
    match db.candidates(root, &Query::string(b"rareword")) {
        Candidates::Files(files, _) => {
            let paths: Vec<&[u8]> = files.iter().map(|f| f.path.as_slice()).collect();
            assert_eq!(paths, [b"dir03/f17.txt".as_slice(), b"dir15/f01.txt".as_slice()]);
        }
        other => panic!("expected a list, got {:?}", other),
    }
    // Limited to the folder asked about.
    let dir = db.resolve(b"dir15").unwrap();
    match db.candidates(dir, &Query::string(b"rareword")) {
        Candidates::Files(files, _) => assert_eq!(files.len(), 1),
        other => panic!("expected a list, got {:?}", other),
    }
    // A good share of the files: a set to test each file against.
    match db.candidates(root, &Query::string(b"midword")) {
        Candidates::Among(set) => {
            let passing = (0..400).filter(|n| {
                let id = db.resolve(format!("dir{:02}/f{:02}.txt", n / 20, n % 20).as_bytes()).unwrap();
                db.passes(&set, id)
            });
            assert_eq!(passing.collect::<Vec<_>>(), (0..400).filter(|n| n % 3 == 0).collect::<Vec<_>>());
        }
        other => panic!("expected a set, got {:?}", other),
    }
    // Most files, or nothing to go on: read them all.
    assert_eq!(db.candidates(root, &Query::string(b"commonword")), Candidates::All);
    assert_eq!(db.candidates(root, &Query::string(b"ab")), Candidates::All);
    // No file at all.
    assert!(matches!(db.candidates(root, &Query::string(b"zzzqqq")), Candidates::Files(files, _) if files.is_empty()));
    assert_eq!(Db::without_index().candidates(0, &Query::string(b"rareword")), Candidates::All);
}

#[test]
fn same_output_with_and_without() {
    let mut on = tree(true);
    let mut off = tree(false);
    let commands = [
        "grep -rn rareword .",
        "grep -rin RAREWORD . | head -n 1",
        "grep -rc rareword dir03 dir15",
        "grep -rl midword .",
        "grep -rc midword . | grep -v ':0$' | wc -l",
        "grep -rn commonword dir00",
        "grep -rnv rareword dir03",
        "grep -rnE 'rare(word|bird)|second L' dir15",
        // The file written to is among the files searched.
        "grep -rn rareword . > dir03/out.txt",
        "grep -rn midword . > dir00/f00.txt",
        "grep -rn rareword . >> dir15/f01.txt",
        "sed -i 's/RareWord/OtherWord/' dir03/f17.txt",
        "grep -rn rareword .",
        "grep -rli otherword .",
        "mv dir15 dir03/moved",
        "grep -rn rareword dir03",
        "cp -r dir03 copy",
        "grep -rc RareWord copy dir03 | grep -v ':0$'",
        "rm -r dir03",
        "grep -rn -i rareword .",
        "echo 'a rareword at last' >> dir07/f07.txt",
        "grep -rn rareword .",
    ];
    for c in commands {
        let a = execute(&mut on, c.as_bytes());
        let b = execute(&mut off, c.as_bytes());
        assert_eq!(a, b, "{}", c);
        assert_eq!(on.snapshot(), off.snapshot(), "{}", c);
    }
}

#[test]
fn a_move_is_free_and_writes_are_counted() {
    let mut db = tree(true);
    let before = db.index_work().unwrap();
    execute(&mut db, b"mv dir05 dir06/inside");
    execute(&mut db, b"mv dir06/inside/f01.txt renamed.txt");
    assert_eq!(db.index_work().unwrap(), before);
    execute(&mut db, b"echo 'three new words' >> renamed.txt");
    let after = db.index_work().unwrap();
    // What is added is indexed, as a piece of its own or joined with a
    // small piece at the file's end.
    assert_eq!(after.documents_added - before.documents_added, 1);
    assert!(after.documents_retired - before.documents_retired <= 1);
    // The maintained index is, list by list, what one built now would be.
    db.check().unwrap();
}
