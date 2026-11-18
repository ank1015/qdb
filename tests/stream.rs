//! The streaming entry point: output is handed on as it is produced, a
//! reader can stop it, and memory does not grow with the output.

use qdb::{execute, execute_to, Db, Sink};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(layout.size(), Relaxed) + layout.size();
        PEAK.fetch_max(live, Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// Takes output until it has seen `limit` bytes, then refuses more.
struct Limited {
    limit: usize,
    seen: usize,
    writes: usize,
}

impl Sink for Limited {
    fn write(&mut self, data: &[u8]) -> bool {
        self.seen += data.len();
        self.writes += 1;
        self.seen < self.limit
    }
}

fn big_db(files: usize) -> Db {
    let mut db = Db::new();
    execute(&mut db, b"mkdir -p d");
    let line = "the quick brown fox jumps over the lazy dog\n".repeat(2000);
    let root = db.resolve(b"d").unwrap();
    for i in 0..files {
        db.create_file(root, format!("f{:04}.txt", i).as_bytes(), line.clone().into_bytes());
    }
    db
}

/// The most heap a command holds above what is held before it starts.
fn peak_of(db: &mut Db, command: &str) -> (usize, usize) {
    let mut out = Limited { limit: usize::MAX, seen: 0, writes: 0 };
    let mut err = Vec::new();
    let base = LIVE.load(Relaxed);
    PEAK.store(base, Relaxed);
    execute_to(db, command.as_bytes(), &mut out, &mut err);
    (PEAK.load(Relaxed) - base, out.seen)
}

// One test, so that nothing else allocates while memory is measured.
#[test]
fn streaming() {
    // Memory does not grow with the output: forty times the data, the
    // same peak.
    let mut small = big_db(5);
    let mut large = big_db(200);
    for command in [
        "cat d/*.txt",
        "grep -rn fox .",
        "grep -rc fox .",
        "grep -rv zzz d",
        "cat d/*.txt | grep -n lazy",
        "cat d/*.txt | sed 's/fox/cat/'",
        "cat d/*.txt | cat -n | tail -n +5",
        "sed 's/dog/cat/g' d/f0001.txt d/f0002.txt d/f0003.txt d/f0004.txt",
        "find . -type f | cat",
        "cat d/*.txt | wc",
        "cat d/*.txt | tail -n 3",
    ] {
        let (peak_small, out_small) = peak_of(&mut small, command);
        let (peak_large, out_large) = peak_of(&mut large, command);
        if !command.starts_with("sed") && !command.ends_with("wc") && !command.ends_with("tail -n 3") {
            assert!(out_large > 10 * out_small, "{}: the larger database should print far more", command);
        }
        assert!(peak_large < peak_small + 256 * 1024, "{}: peak memory grew from {} to {} bytes", command, peak_small, peak_large);
        assert!(peak_large < 1024 * 1024, "{}: peak memory is {} bytes", command, peak_large);
    }

    // A reader that wants no more stops a command that only reads.
    let mut out = Limited { limit: 1, seen: 0, writes: 0 };
    let mut err = Vec::new();
    execute_to(&mut large, b"grep -rn fox .", &mut out, &mut err);
    assert_eq!(out.writes, 1);
    assert!(out.seen < 200_000, "grep went on after its output was refused: {} bytes", out.seen);

    // A command that changes data finishes whatever its reader does.
    let mut out = Limited { limit: 1, seen: 0, writes: 0 };
    execute_to(&mut large, b"cat d/f0000.txt | sed 's/fox/cat/' > d/new.txt | head -n 0", &mut out, &mut err);
    assert_eq!(execute(&mut large, b"grep -c cat d/new.txt").stdout, b"2000\n");
    execute_to(&mut large, b"cp -r d e | head -n 0", &mut out, &mut err);
    assert_eq!(execute(&mut large, b"ls e | wc -l").stdout, b"201\n");

    // head stops the stages before it: the same answer, and no more work
    // than the answer needs.
    let got = execute(&mut large, b"grep -rn fox . | head -n 2");
    assert_eq!(got.stdout, b"./d/f0000.txt:1:the quick brown fox jumps over the lazy dog\n./d/f0000.txt:2:the quick brown fox jumps over the lazy dog\n");
    assert_eq!(got.code, 0);
}
