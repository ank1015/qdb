# Test harness

`qdb-harness` runs each command twice: once on the real GNU tools in a
scratch directory, once on the engine with the same starting tree. A case
passes when standard output, standard error, the exit status and the tree
left behind are all identical.

```
cargo build --release
./target/release/qdb-harness                       # every case
./target/release/qdb-harness tests/cases/grep.case # one file
./target/release/qdb-harness --filter 'sed -E'     # cases containing text
./target/release/qdb-harness --fuzz 5000 --seed 7  # plus generated regex cases
```

The real side runs under `bash --noprofile --norc -c` with an empty
environment, `LC_ALL=C` and no standard input. The harness refuses any
command whose paths could leave the scratch directory.

The expected results were produced with bash 5.2.21, grep 3.11, sed 4.9,
coreutils 9.4 and findutils 4.9.0. Other versions word some messages
differently, so a handful of error cases can fail on them.

## Case files (`tests/cases/*.case`)

One case per line. Blank lines and lines starting with `#` are ignored.

| Line | Meaning |
|---|---|
| `@fixture name` | Use `tests/fixtures/name.fx` for the cases that follow. |
| `$ command` | Run on both sides and compare everything. |
| `$~ command` | The same, but do not compare standard error. |
| `+ command` | A further step of the previous case, run on the same tree. |
| `! command ## text` | The engine must refuse this with status 2, a message containing `text`, and an unchanged tree. Not run on the real tools. |
| `= command` | Engine only, for the few places where there is nothing real to compare with. Followed by `> line` (expected output line), `>: line` (last line, no newline), `2> line` (expected error line) and `? code` (expected status). |

Inside a command, `⏎` stands for a newline and `{ROOT}` for the root of the
tree (the scratch directory on the real side, nothing on the engine side).

`find` and `grep -r` walk folders in whatever order the file system hands
out, which the design document replaces with sorted order. For those
commands the harness compares the lines as a sorted set.

## Fixtures (`tests/fixtures/*.fx`)

```
D path/to/folder        an empty folder
F path/to/file          a file; its lines follow
|a line of text
:a last line with no newline at the end
```

## What the cases cover

| File | Cases about |
|---|---|
| `ls.case` | `ls` |
| `find.case` | `find` |
| `read.case` | `cat`, `head`, `tail`, `wc` |
| `grep.case` | `grep` and the regex engine |
| `sed.case` | `sed` |
| `write.case` | `echo`, `touch`, `mkdir`, redirects |
| `copy_move.case` | `cp`, `mv` |
| `remove.case` | `rm`, `rmdir` |
| `shell.case` | quoting, globs, pipes, and everything that must be refused |
| `text.case` | the text-only rule and awkward content |
