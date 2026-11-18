# qdb

A custom text database that speaks bash. It stores one tree of folders and
text files in memory and runs a subset of GNU command lines against it
(`ls`, `find`, `cat`, `grep`, `sed`, ...), producing the same bytes and the
same exit codes as the real tools.

Stage 1 built a simple, correct engine, a test harness that compares it
with the real GNU tools, and a benchmark. Stage 2 made reads fast in
memory: pipelines that stop as soon as their reader has enough, a search
that is faster than GNU grep on one thread with no index, and a trigram
index that lets a search read only the files that can match. There is no
on-disk storage and no concurrency yet.

Nothing is borrowed from an existing database, search library or regex
library. The crate depends on `memchr` for finding a byte or a substring,
and on nothing else.

## Quick start

```
cargo build --release

# run commands against a fixture
./target/release/qdb --fixture tests/fixtures/basic.fx -c 'grep -rn TODO .' -c 'ls -R src'

# load a real directory into memory, change it, write the result elsewhere
./target/release/qdb --load some/dir --dump out/dir -c "sed -i 's/old/new/g' notes.txt"

# the same search with and without the search index: same output, different speed
./target/release/qdb --load some/dir -c 'grep -rn needle .'
./target/release/qdb --load some/dir --no-index -c 'grep -rn needle .'

# compare the engine with the real tools (1,617 cases), index on and off
./target/release/qdb-harness
./target/release/qdb-harness --no-index

# index on against index off over 10,000 random searches and writes
./target/release/qdb-harness --diff 10000 --seed 1

# time the engine against the real tools
./target/release/qdb-bench --dataset small
```

`--load` skips symbolic links and files that are not valid text, and
reports on standard error how many files it loaded and skipped, how long
that took, and how long the index took to build:

```
qdb: some/dir: loaded 1672 files (17.4 MB) in 0.02 s; skipped 13 (0 symbolic links, 13 not valid text, 0 other)
qdb: some/dir: search index built in 0.08 s (4.9 MB, 74777 trigrams)
```

As a library:

```rust
let mut db = qdb::Db::new();
qdb::execute(&mut db, b"mkdir -p docs");
qdb::execute(&mut db, b"echo hello > docs/a.txt");
let out = qdb::execute(&mut db, b"cat docs/a.txt | wc -c");
assert_eq!(out.stdout, b"6\n");
assert_eq!(out.code, 0);
```

`execute` gathers the output. `execute_to` hands it to a `Sink` in pieces
as it is produced, so memory does not grow with the output, and the sink
can say it has had enough, which stops the command:

```rust
struct FirstKilobyte(Vec<u8>);
impl qdb::Sink for FirstKilobyte {
    fn write(&mut self, data: &[u8]) -> bool {
        self.0.extend_from_slice(data);
        self.0.len() < 1024 // false: stop
    }
}
let mut out = FirstKilobyte(Vec::new());
let mut err = Vec::new();
let code = qdb::execute_to(&mut db, b"grep -rn hello .", &mut out, &mut err);
```

`Db::new()` keeps the search index; `Db::without_index()` and
`db.set_index(false)` do without it. Results never depend on the index.

## What is supported

| Command | Flags |
|---|---|
| `ls` | `-a -R -1 -p` |
| `find` | `-name -type f/d -maxdepth` |
| `cat` | `-n` |
| `head` | `-n N` |
| `tail` | `-n N`, `-n +N` |
| `wc` | `-l -w -c` |
| `grep` | `-r -i -n -l -v -c -E -F` |
| `mkdir` | `-p` |
| `touch` | |
| `cp` | `-r` |
| `echo` | `-n` |
| `sed` | `-i -E`, `s/old/new/` with `g` |
| `mv` | |
| `rm` | `-r -f` |
| `rmdir` | |

Shell syntax: pipes, `>` and `>>`, globs (`*`, `?`, `[...]` with bash's
default rules), single quotes, double quotes and backslash escapes.
Combined short flags (`ls -aR`, `grep -rin`) work.

Anything else is refused before any part of the line runs, with exit
status 2 and a message that names what was refused:

```
$ qdb -c 'ls ; ls'        qdb: unsupported syntax: command lists (;)
$ qdb -c 'cat $HOME'      qdb: unsupported syntax: variable expansion ($HOME)
$ qdb -c 'ls --all'       qdb: unsupported flag: ls --all (long options are not supported yet)
$ qdb -c 'awk 1'          qdb: unsupported command: awk
```

There is no current directory. Every path is resolved from the root, so
`a`, `./a` and `/a` name the same thing.

## How it is put together

| File | Job |
|---|---|
| `src/tree.rs` | The data model: one strict tree of folders and files, ids, names, parent links. Also the storage layer's one question for searches, `Db::candidates`: which files under this folder can match this query? |
| `src/query.rs` | The query a search hands to storage: AND and OR over trigrams. |
| `src/index.rs` | The in-memory trigram index that answers it, kept current by every change to file content. Derived data: it can be thrown away and built again. |
| `src/shell.rs` | A recursive-descent parser that follows the structure of the bash grammar (list, pipeline, command, redirect, word), so later syntax has a place to go. |
| `src/glob.rs` | Pathname expansion with bash's rules. |
| `src/regex.rs` | POSIX basic and extended regular expressions with leftmost-longest matching, plus the GNU extensions grep and sed accept. This matcher is the reference for what matches. |
| `src/regex/dfa.rs` | A DFA built as it runs, for patterns without back-references. |
| `src/regex/trigram.rs` | The trigram query for a regular expression. |
| `src/scan.rs` | Finding the matching lines of a buffer fast: fixed strings, required strings, the DFA, and the reference matcher as the last resort. |
| `src/bytes.rs` | Substring search, including one that ignores case, and the engine's single definitions of "one character" and of case folding. |
| `src/engine.rs` | Runs a parsed line: expands globs, checks every stage is supported, runs the stages as a stream, applies redirects, undoes a write that is not valid text. |
| `src/cmd/` | One module per group of commands. |
| `src/store.rs` | Loading a directory or fixture into memory and dumping it back. Only used by the tools, never by the engine. |
| `src/testing.rs` | Running a command line on the real tools in a scratch directory. |
| `src/bin/harness.rs` | The test harness. See `tests/README.md`. |
| `src/bin/bench.rs` | The benchmark. See `bench/README.md`. |

## Where to read next

- `docs/briefs/`: the two briefs this was built from.
- `docs/HANDBACK-2.md`: stage 2. What was built, every target with its
  measured value, the index in numbers, deviations, gaps and questions.
- `docs/HANDBACK.md`: the same for stage 1.
- `tests/README.md`: how the harness works and how to add a case.
- `bench/README.md`: the datasets, the command list and the results.
