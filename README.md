# qdb

A custom text database that speaks bash. It stores one tree of folders and
text files in memory and runs a subset of GNU command lines against it
(`ls`, `find`, `cat`, `grep`, `sed`, ...), producing the same bytes and the
same exit codes as the real tools.

This is stage 1 from the design document: a simple, correct, naive engine,
a test harness that compares it with the real GNU tools, and a benchmark.
There is no on-disk storage, no index, no concurrency and no optimisation
yet. Nothing is borrowed from an existing database or search library; the
crate has zero dependencies.

## Quick start

```
cargo build --release

# run commands against a fixture
./target/release/qdb --fixture tests/fixtures/basic.fx -c 'grep -rn TODO .' -c 'ls -R src'

# load a real directory into memory, change it, write the result elsewhere
./target/release/qdb --load some/dir --dump out/dir -c "sed -i 's/old/new/g' notes.txt"

# compare the engine with the real tools (1,496 cases)
./target/release/qdb-harness

# time the engine against the real tools
./target/release/qdb-bench --dataset small
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
| `src/tree.rs` | The data model: one strict tree of folders and files, ids, names, parent links. |
| `src/shell.rs` | A recursive-descent parser that follows the structure of the bash grammar (list, pipeline, command, redirect, word), so later syntax has a place to go. |
| `src/glob.rs` | Pathname expansion with bash's rules. |
| `src/regex.rs` | POSIX basic and extended regular expressions with leftmost-longest matching, plus the GNU extensions grep and sed accept. |
| `src/engine.rs` | Runs a parsed line: expands globs, checks every stage is supported, runs the stages, applies redirects, undoes a write that is not valid text. |
| `src/cmd/` | One module per group of commands. |
| `src/store.rs` | Loading a directory or fixture into memory and dumping it back. Only used by the tools, never by the engine. |
| `src/testing.rs` | Running a command line on the real tools in a scratch directory. |
| `src/bin/harness.rs` | The test harness. See `tests/README.md`. |
| `src/bin/bench.rs` | The benchmark. See `bench/README.md`. |

## Where to read next

- `docs/HANDBACK.md`: what was built, the decisions taken where the design
  document was silent, and every known gap.
- `tests/README.md`: how the harness works and how to add a case.
- `bench/README.md`: the datasets, the command list and the results.
