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
./target/release/qdb-harness --store disk          # the same cases on a database directory
./target/release/qdb-harness --store reopen        # ... closed and opened again after every step
./target/release/qdb-harness --crash 10000 --seed 1 --sync batch   # the power-cut test
./target/release/qdb-harness --kill 1000 --seed 1                  # the kill test
cargo test --release                               # unit tests, and tests/disk.rs
```

## The three stores

`--store memory` (the default) runs the engine on a tree in memory.
`--store disk` runs it on a database directory, and after each case runs
every check `qdb --check` makes. `--store reopen` also closes the
database and opens it again after every step of every case, and checks it
each time, so everything a case leaves behind has been through the files.
`--sync`, `--piece BYTES`, `--segment BYTES`, `--compress` and
`--index-later` set the switches; a small `--piece` (16) makes the small
test files span many pieces, and a small `--segment` makes data files
finish, indexes get written and space get reclaimed during the cases.
`--diff` takes the same options: the engine under test is then compared,
step for step, with an engine in memory without an index.

## The power-cut test (`--crash N`)

The engine does all its file work through one layer (`src/vfs.rs`). The
test runs it on files in memory that remember, for every file, what has
been synced and what has only been written, and for the directory which
creations, removals and renames have been synced. A generated run of
command lines (the generator of `--diff`) goes on; at N points in it,
chosen at any file operation (a write, a sync, a rename, ...) of a command
line, of an index file being written, of a tree being saved, of a close
or a compaction, the power is cut: a crash image is made that keeps
everything synced, and of what was not synced a random part (whole writes
in order up to some point, then possibly a torn write; an unsynced rename
or creation may or may not have happened).

Every image must open. The opened database must hold exactly the tree
after some whole number of the run's command lines (the test keeps the
tree after every line), no fewer than the sync setting allows (none lost
that had returned under `always`; at most 7 with the test's batch of 8
lines), and it must pass every check. Then, on a share of the images: the
power is cut again at every file operation of the recovery itself, and
each of those images must recover to the same tree; the recovered
database is written to, checked, closed and reopened; and the run goes on
from the image instead of from where it was, sometimes after a simulated
disk failure (every operation fails from some point on, the database
must stop with an error, and the image must recover).

`tests/results/crash/power-cuts.txt` holds the official runs,
`mutations.md` the faults put into the engine on purpose to see that the
test catches them, and every real fault the test found.

## The kill test (`--kill N`)

A real `qdb --db` process on a real directory is fed command lines and
killed with SIGKILL at a random moment, N times; after each kill the
directory is opened and must hold the tree after a whole number of
command lines, with every line whose output had been seen present.
`tests/results/crash/kills.txt` holds the official runs.

## tests/disk.rs

What survives closing; that reads write nothing; the lock; a missing or
damaged index file is built again; damaged content, a damaged tree or
MANIFEST is an error that names the file; a file from a newer format
version is refused; a directory that is not a database is refused;
compaction gives back space.

The real side runs under `bash --noprofile --norc -c` with an empty
environment, `LC_ALL=C` and no standard input. The harness refuses any
command whose paths could leave the scratch directory.

The expected results were produced with bash 5.2.21, grep 3.11, sed 4.9,
coreutils 9.4, findutils 4.9.0, diffutils 3.10 and GNU awk 5.2.1. Ubuntu's own awk is mawk, so `bench/fetch.sh` unpacks the gawk package beside the datasets (`../qdb-data/gawk`, or the folder `QDB_GAWK` names) and the harness puts it first on the real side's PATH; without it the awk cases compare against mawk and many fail. Other versions word some messages
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
| `files_flags.case`, `read_flags.case`, `grep_flags.case`, `find_more.case`, `ls_long.case` | the flags added in stage 4 to the first fifteen commands, with file times set by `touch -d` |
| `sed_script.case` | sed addresses, commands, blocks and `-i` with a suffix |
| `shell_lists.case` | command lists, redirections of all three streams, `/dev/null`, here-documents, command substitution, brace expansion |
| `rg.case` | `rg`: flags, globs, types, and the ignore rules, on the fixture `rg.fx` |
| `small.case` | `printf`, `test` and `[`, `seq`, `basename`, `dirname`, `true`, `false`, `:`, `sleep` |
| `session.case` | variables, the parameter forms, arithmetic, tilde, field splitting, `cd` and `pwd`, `export`, `unset` |
| `control.case` | `for`, `if`, `while`, `until`, `break`, `continue`, `read`, constructs in pipelines and under redirections, the command limit, and what `xargs` and `find -exec` refuse to run |
| `awk.case` | `awk` against GNU awk 5.2.1: program shapes, fields, numbers and strings, the functions, printf, regular expressions, what is refused |
| `diff.case` | `diff`: both formats, folders, the ignore flags, labels, standard input, and every message |
| `p5_text.case`, `p5_sort.case`, `p5_sys.case` | `cut`, `tr`, `uniq`, `nl`, `tee`; `sort`; `du`, `date`, `xargs` |

## Clock and file times

Both sides must show the same times. A case sets the times it prints
with `touch -d`, which runs on both sides. For the generated tests the
harness sets every file's and folder's time afresh before each case, to
the same age on both sides. `ls -l` is compared with the owner and group
of the real side replaced by `qdb`.

## The coverage table (`--coverage`)

`tests/coverage.txt` has one row for every command and every flag,
syntax form or behaviour the engine supports, with a pattern that finds
the cases exercising it:

```
grep<TAB>-o<TAB>-o             a short flag of that command
rg<TAB>--files<TAB>--files     a long flag
shell<TAB>&&<TAB>re:&&         a regular expression over the whole case
```

Every run of the harness prints the table with the number of cases per
row, and fails if a row has none. `--coverage` prints the table alone.

## Generated tests against the real tools (`--gen FAMILY`)

`src/testgen.rs` writes random command lines over a small generated tree
for eleven families: `shell`, `grep`, `sed`, `find`, `rg`, `text` (sort,
uniq, cut, tr, nl, seq, printf), `ls` (ls and du), `xargs`, `scripts`
(variables, arithmetic, `for`, `while`, `until`, `if`, `read` loops,
nested, piped and redirected) and `diff` (versions of one file changed
in a few places, folders with files on one side only, a file against a
folder, every flag) and `awk` (programs built from the whole supported
language over files of numbers written every way awk has a rule for;
programs that walk an array are piped through `sort`). Each line
runs on the real tool and on the engine and everything is compared, as
for a hand-written case.

```
./target/release/qdb-harness --gen all --count 10000 --seed 1
./target/release/qdb-harness --gen sed --count 10000 --seed 2 --store reopen
```

A line of the result reads `N passed, K known differences, F failed`. A
known difference is one where the real tool's output is a defect that
the engine does not copy; there is one kind, in `known_difference` in
`src/bin/harness.rs`: GNU grep 3.11 with `-o`, `-w` and `-x` together
takes the line's newline as part of the match for some patterns and
prints an empty line after it (and prints an empty match for `^$`). The
engine prints the match alone. It showed in 9 of 30,000 generated grep
lines.

The generator stays inside what the engine supports and inside what is
deterministic on the real side: only the last command of a pipeline
writes a file or standard error, and `find -size` is asked only of
files. Where the real tool prints in directory order (`find`, `grep -r`,
`du`, `rg` without `--sort path`) the lines are compared as a sorted set.

A generated loop steps its counter first, so it ends in bash too, and
the real side runs under `timeout`. Loops are not piped into `head`
(bash may or may not have run one more turn when the pipe closes), and
a nested construct never redirects to the file an outer one writes.

