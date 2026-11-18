//! What a unit used: the counts returned with every result.

use qdb::{execute, Db};

fn tree() -> Db {
    let mut db = Db::new();
    let root = db.root();
    let dir = db.create_folder(root, b"src");
    for n in 0..50 {
        let mut text = format!("file number {}\nsecond line\n", n);
        if n == 7 {
            text.push_str("the needleword is here\n");
        }
        db.create_file(dir, format!("f{:02}.txt", n).as_bytes(), text.as_bytes());
    }
    db
}

#[test]
fn a_read_counts_the_bytes_it_looked_at() {
    let mut db = tree();
    let out = execute(&mut db, b"cat src/f07.txt");
    assert_eq!(out.cost.text_bytes, out.stdout.len() as u64);
    assert_eq!(out.cost.text_pieces, 1);
    // src, then f07.txt, each time the path is looked up.
    assert!(out.cost.tree_nodes >= 2 && out.cost.tree_nodes % 2 == 0);
    assert_eq!((out.cost.content_written, out.cost.records_written, out.cost.syncs), (0, 0, 0));
    assert_eq!(out.cost.index_lookups, 0);
}

#[test]
fn a_search_with_the_index_reads_only_what_can_match() {
    let mut db = tree();
    let with = execute(&mut db, b"grep -rn needleword .");
    assert_eq!(with.stdout, b"./src/f07.txt:3:the needleword is here\n");
    assert!(with.cost.index_lookups > 0 && with.cost.index_bytes > 0);
    assert_eq!(with.cost.text_pieces, 1);
    let mut plain = Db::without_index();
    let root = plain.root();
    let dir = plain.create_folder(root, b"src");
    for n in 0..50 {
        plain.create_file(dir, format!("f{:02}.txt", n).as_bytes(), b"nothing\n");
    }
    let without = execute(&mut plain, b"grep -rn needleword .");
    assert_eq!(without.cost.index_lookups, 0);
    assert_eq!(without.cost.text_pieces, 50);
}

#[test]
fn a_write_counts_content_and_records_apart() {
    let mut db = tree();
    let out = execute(&mut db, b"echo twelve bytes > new.txt");
    assert_eq!(out.code, 0);
    // In memory nothing reaches a data file.
    assert_eq!(out.cost.syncs, 0);
    let again = execute(&mut db, b"cat new.txt");
    assert_eq!(again.cost.text_bytes, 13);
}

#[test]
fn the_same_units_count_the_same() {
    let units: [&[u8]; 6] = [
        b"grep -rn needleword .",
        b"ls -R",
        b"echo more >> src/f01.txt",
        b"sed -i s/second/2nd/ src/f02.txt",
        b"cat src/*.txt | sort | uniq -c | head -n 3",
        b"find . -name 'f1*' | xargs grep -c line",
    ];
    let run = || {
        let mut db = tree();
        units.iter().map(|u| execute(&mut db, u).cost.counts()).collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn the_line_printed_has_a_fixed_shape() {
    let mut db = tree();
    let line = execute(&mut db, b"cat src/f07.txt").cost.to_string();
    let names: Vec<&str> = line.split(' ').map(|f| f.split('=').next().unwrap()).collect();
    assert_eq!(
        names,
        ["text_bytes", "text_pieces", "index_bytes", "index_lookups", "tree_nodes", "content_written", "records_written", "upkeep_written", "syncs", "scratch_bytes", "memory", "cpu_us", "elapsed_us"]
    );
    assert!(line.split(' ').all(|f| f.split('=').nth(1).map_or(false, |v| v.parse::<u64>().is_ok())));
}
