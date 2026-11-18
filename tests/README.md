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
./target/release/qdb-harness --no-index            # the same cases with the search index off
./target/release/qdb-harness --diff 10000 --seed 1 # index on against index off
```

The real side runs under `bash --noprofile --norc -c` with an empty
environment, `LC_ALL=C` and no standard input. The harness refuses any
command whose paths could leave the scratch directory.

The expected results were produced with bash 5.2.21, grep 3.11, sed 4.9,
coreutils 9.4 and findutils 4.9.0. Other versions word some messages
differently, so a handful of error cases can fail on them.

## Index on against index off (`--diff`)

The search index must never change a result. `--diff N` runs N generated
steps on two engines that start empty, one with the index and one without.
About half the steps are searches: `grep -r` with a random pattern (words,
pieces of words, alternations, classes, repeats, anchors, back-references,
fixed strings, terms that are in no file), random flags out of
`-n -l -c -i -v -E -F`, on the whole tree or on a folder, sometimes piped
into `head` or `wc`, sometimes redirected into a file that is itself
searched. The other half are writes: create, overwrite, append, `sed -i`,
`cp -r`, `rm -r`, `mv`, and files copied onto themselves until they are
large. Every write is followed by a search over everything.

After every command the two engines must have produced the same standard
output, standard error and exit status, and the trees are compared every
50 steps. Every 1,000 steps and at the end, the maintained index is
compared with one built from scratch over the same files: both must give
the same files for a few hundred queries. The run prints how many
searches selected something and how much work the index did, so that an
empty test cannot pass for a good one. `--seed` picks the sequence.

`cargo test` also runs `tests/index.rs` (each kind of answer storage can
give, and a fixed list of commands with the index on and off),
`tests/stream.rs` (bounded memory and stopping early) and
`tests/scan_fuzz.rs` (the fast search paths against the reference
matcher).

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
| `pipes.case` | pipelines that stop early: what is printed, what is written, and which errors are reported |
| `text.case` | the text-only rule and awkward content |
