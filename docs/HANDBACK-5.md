# Hand-back notes for stage 5: the rest of the language, and what each command costs

Branch `claude/project-thread-3wnfpo`. Everything below was measured on
one machine (4 cores, 15 GB, one thread used, a virtual disk with ext4)
with bash 5.2.21, grep 3.11, sed 4.9, coreutils 9.4, findutils 4.9.0,
diffutils 3.10, ripgrep 14.1.0 and GNU awk 5.2.1. The brief is in
`docs/briefs/stage-5-the-rest-of-the-language.md`.

## Summary

Parts 1, 2, 3, 4 and 6 are done. **Part 5, `apply_patch`, was not
built: the owner asked for it to be skipped.** Its two target rows, its
tests and item 6 of this hand-back are therefore empty.

- Every unit is counted as it runs, `qdb --cost` prints the counts, and
  `qdb-replay` turns a file of units into the cost tables of section 5.
- The shell has variables, arithmetic, `cd`, `for`, `while`, `until`,
  `if`, `break` and `continue`, sessions, and a limit on how many
  commands one unit may run.
- `diff` and `awk` are the thirty-fifth and thirty-sixth commands.
- 5,706 hand cases pass in memory, on disk and reopened after every
  step. 90,000 generated cases (scripts, diff, awk) agree with the real
  tools with no exception.
- Of the seventeen target rows, fifteen are met and two are the skipped
  apply_patch rows.

Three things you should know that the table does not say:

- **awk refuses input that is not ASCII, as the brief asks, and real
  trees have such files.** On the postgres tree the benchmark's chosen
  file has a non-ASCII byte, so five awk commands there end with status
  2 where gawk prints. On the Linux tree some headers do the same; the
  benchmark's awk commands over many files therefore strip those bytes
  with `tr` first. See question 1 in section 10.
- **Feeding 1 GB to `rg` on standard input completes inside the limit
  but is slower than real rg** (1.17 s against 0.90 s). The target was
  to complete. `xargs` takes the same time as GNU xargs (39 s both).
- **The machine does not repeat a time to better than about a third on
  the longest searches.** `grep -rn define .` took 454 ms in the stage 4
  run, 657 ms in this stage's settled run and 485 ms in this stage's run
  under the memory limit, on the same build as the 657. GNU grep moved
  the same way in the same runs (3.4 s, 4.0 s, 2.6 s). The counters do
  not move at all, which is why the rule about cost is on them.

## 1. Code

On the branch. `README.md` is up to date: the command table (36
commands), the shell table, the sections on sessions, the command
limit, diff, awk, and a new section on what a command costs.

| New or changed | What |
|---|---|
| `src/tree.rs` | The counters (`Cost`), `begin_unit` and `end_unit` |
| `src/bin/replay.rs` | `qdb-replay` |
| `src/shell.rs`, `src/engine.rs` | Variables, arithmetic, constructs, sessions, the command limit |
| `src/cmd/diff.rs` | `diff` |
| `src/cmd/awk/` | `awk`: `parse.rs` (the program), `run.rs` (values, fields, the interpreter, printf), `mod.rs` (options, input) |
| `src/regex.rs` | A third flavour of expression for awk (`.` and negated sets match a newline) |
| `src/testgen.rs` | Three new generated families: scripts, diff, awk |
| `bench/fetch.sh` | Fetches the gawk package (and libsigsegv) from the Ubuntu archive, checks the sums and unpacks it beside the datasets; nothing is installed on the machine |

No crate was added. diff, awk and the awk side of the regular
expressions are written from scratch.

## 2. Conformance

**Harness.** 5,706 cases (6,039 steps), all passing in each of:

| Mode | Result |
|---|---|
| In memory | 5,706 passed |
| On disk, sync always | 5,706 passed |
| On disk, closed and reopened after every step | 5,706 passed |
| In memory, pieces of 512 bytes | 5,706 passed |
| In memory, index off | 5,706 passed |

Stage 4 ended with 4,689. `tests/cases/awk.case` holds 254 of the new
ones and `diff.case` the diff ones.

**Coverage.** `tests/coverage.txt` has 509 rows after this stage; the
harness prints the count of cases for each and none is zero. Sections 4
to 9 of the brief are covered except section 8 (apply_patch).

**Generated tests,** each case run on the real tool and on the engine:

| Family | Seeds | Cases | Failed | Known differences |
|---|---|---:|---:|---:|
| scripts (variables, loops, `if`, arithmetic) | 41, 42, 43 | 30,000 | 0 | 0 |
| diff | 41, 42, 43 | 30,000 | 0 | 0 |
| awk | 41, 42, 43 | 30,000 | 0 | 0 |

The awk family found seven real differences on earlier seeds (1, 2, 3)
before it came out clean; each was fixed in the engine and has a hand
case: line numbers in run-time errors; `printf` of infinity and
not-a-number (and in capitals for `%G`, `%E`, `%F`, `%X`); `sub` on a
field past the last one; `$0` and fields after an assignment (a record
rebuilt from its fields is a string, a field given a value never set
stays "never set"); strict `else`. The stage 4 families were not run
again at 10,000 cases in this stage; their hand cases are in the 5,706.

**Storage tests on the final build:** 300 power cuts (seed 5), 30 kills
of a real process (seed 5), 3,000 steps of index on against index off
(seed 5, loops included in the steps): all pass. `cargo test`: all pass.

**Counters.** Every in-memory harness run also runs the 224 units of
the benchmark list twice on one database and compares: 0 units with
different counts, 0 counts above the stored ones.

## 3. Targets

The table, with every figure's source, is
`bench/results/TARGETS-5.md`. In short:

| Part | Target | Measured | Result |
|---|---|---|---|
| All | Harness in three modes | 5,706 of 5,706 in each | met |
| All | Coverage with no zero | 509 rows, none at zero | met |
| All | Generated tests, 10,000 × 3 seeds × 3 families | 90,000 cases, 0 differences | met |
| All | Stage 4 commands against stored counters | One of 182 changed, with a reason (below) | met |
| 1 | Identical counters on a second run | 224 units, 0 different | met |
| 1 | Cost of counting | Writes nothing, syncs nothing; time 1.000 of uncounted for commands of 1 ms and more, 0.66 µs a unit for the shortest | met |
| 1 | `xargs` and `rg` fed 1 GB on standard input inside 512 MB | Complete, 18 MB of heap | met |
| 1 | `grep -o '[a-z]*ing'` over 1 GB | 8.9 s against GNU's 11.2 s | met |
| 2, 3 | `for i in $(seq 100000); do :; done` | 38.5 ms against bash's 99.2 ms | met |
| 3 | 10,000 small writes in one unit, always | 1 sync; 34.2 ms against bash's 60.1 ms | met |
| 3 | A unit that loops forever | Stopped after 1,000,000 commands in 0.23 s, status 3; database checks clean and goes on | met |
| 4 | `diff -u` of two versions of a file | 0.093 ms against 3.03 ms | met |
| 4 | `diff -rq Documentation COPY` after one edit | 3.44 ms; 115 bytes of text read | met |
| 4 | `diff` of the 1 GB file and its copy with one line changed | 2.21 ms, 0.5 MB of heap | met |
| 5 | apply_patch, two rows | not built | skipped at the owner's request |
| 6 | Three awk programs over 1 GB, inside 512 MB | 1.13 s, 0.79 s and 2.01 s against gawk's 3.14 s, 2.41 s and 4.71 s; 0.2 MB of heap | met |

The one counter that rose: the benchmark list ends with `rm -rf
bench_out`, and that unit now writes 438 bytes of records where it wrote
429, because this stage's new commands leave four more entries in the
folder it removes. The baseline was written anew three times in this
stage, each time because units were added, and each commit says so; the
units that were already there kept their counts.

## 4. Benchmark tables

`bench/results/linux.md`, `redis.md` and `postgres.md` hold every
command with every stored run. The runs of this stage are `s5-mem`,
`s5-disk`, `s5-disk-after` and `s5-disk-cap512`; their `.csv` files
carry the counters of each command beside its times (text bytes and
pieces, index bytes and lookups, tree entries, content, records and
upkeep written, syncs, scratch bytes, memory). `TARGETS-5.md` shows the
new commands with their counters in three tables.

The list has 224 commands, 42 of them new: 14 for loops, variables and
`if`, 16 for awk, 8 for diff, and 4 writes that set those up.

| Dataset | Commands | Output differs | Slower than the real tool |
|---|---:|---|---|
| Linux (83,405 files) | 224 | none | `find … \| sort \| xargs cat > file`, 17.5 ms against 7.0 ms in this run; it is a write of 2 MB with one sync, and was 6 ms in other runs |
| redis | 224 | none | `mkdir -p`, 6.2 ms against 2.9 ms: one sync, on a slow moment of the disk |
| postgres | 224 | 5 awk commands that meet a non-ASCII byte (refused by design), and the one that reads the file the refused command did not write | none |

Against stage 4, the 181 commands in both runs have a median of 1.02 of
their stage 4 time. The eight that differ by more than a quarter are in
`TARGETS-5.md`; they are the long searches discussed in the summary and
a few commands of a twentieth of a millisecond.

The 1 GB file (`bench/results/bigfile/s5-disk-always-cap512.csv` and
the batch one): 46 commands, all with the real tools' output, the most
heap 137 MB (sort). Slower than the real tool: the two `rg` on standard
input, and `cat big.txt > cat.txt` (9.0 s against 1.0 s), which is the
known cost from stage 4 of writing 1 GB as new content with its index.

## 5. The cost report

This section is written for a reader who has not seen the code.

### What was run

The database is the Linux source tree: 83,405 files, 1.4 GB of text,
on disk, with every write made safe before it is answered ("sync
always"). Four lists of commands were run against it, one command line
(a "unit") at a time, on one thread, with the files already in the
machine's file cache:

- three made-up workloads of 20,000 units each, in which 95, 50 and 5
  of every hundred units only read (searches, `cat`, `head`, `wc`,
  `find`) and the rest write (a new small file, an append, a `sed -i`,
  `cp`, `mv`, `rm`);
- the benchmark list of 224 units, which has every command and
  construct at least once.

The full tables are in `bench/results/replay/`. For each unit the
engine counts what it did. Those counts are exact and repeat; only the
two times are measured.

### What the tables show

| | 95 in 100 read | 50 in 100 read | 5 in 100 read |
|---|---:|---:|---:|
| Units a second | 339 | 634 | 3,172 |
| Total time for 20,000 units | 59.0 s | 31.6 s | 6.3 s |
| CPU time | 57.2 s | 29.6 s | 3.9 s |
| of which searches | 99.5% | 97.4% | 69.8% |
| of which writes | 0.2% | 2.3% | 30.0% |
| Waiting, not on the CPU | 1.7 s | 1.9 s | 2.3 s |
| of which in writes (waiting for the disk) | 1.3 s | 1.7 s | 2.3 s |
| Syncs | 1,004 | 9,903 | 18,996 |
| Text looked at | 325 GB | 164 GB | 14 GB |
| New content written | 0.16 MB | 1.6 MB | 3.0 MB |
| Most memory one unit added | 1.8 MB | 1.8 MB | 1.0 MB |

**Which work uses the CPU.** Searching, and almost nothing else. A
search over the whole tree for a common word has to look at most of the
1.4 GB, at 5 to 6 GB a second, and costs 3 to 17 ms of CPU; in the
workload that is 95% reads the searches are 43% of the units and 99.5%
of the CPU. Reading a file, listing, counting lines and finding by name
together are half the units and 0.2% of the CPU: a `cat` or `head`
costs about 0.01 ms. A small write costs 0.06 to 0.12 ms of CPU. The
search index is what keeps searches for rare words cheap (they read a
few files, not the tree); the ones that stay expensive are searches for
words that are in most files, where there is nothing to skip.

**Which work reads the disk.** With the file cache warm, none: "not on
the CPU" is under 1% for every kind of read. What a read would need
from a cold disk is the "text read" column, and it is all in searches:
a search for a common word touches hundreds of megabytes, any other
read a few kilobytes. The index adds little (252 MB read against
85,000 MB of text in the middle workload). Stage 4 measured the cold
case: with 1 GB of memory for a 1.55 GB database the median read took
1.2 times its warm time.

**Which work waits on syncs.** Every unit that writes, and only those:
one sync a unit, however many commands or loop turns it has. A write
spends two thirds to nine tenths of its time waiting for it. On this
disk a sync costs 0.12 ms when writes follow each other closely and
1.3 ms when they are spread out among reads (the disk batches what
arrives together). What is written is small: about 150 bytes of content
and 75 bytes of records a write in these workloads, and no upkeep fell
due in any of them.

**What runs short first as load grows.**

1. *One thread's CPU, on searches.* A workload that is mostly reads
   gets 339 units a second because 43 in 100 are searches at 6.7 ms
   each. Nothing else is close. More threads help exactly here, because
   reads change nothing (section 9).
2. *The sync, on writes.* A workload that is mostly writes is bounded
   by one sync a unit: about 5,000 a second on this disk if the units
   did nothing else, 3,172 measured. `--sync batch` removes the wait
   for callers who can lose the last moments on a power cut; with
   several users, letting units that arrive together share one sync
   would do the same without that loss.
3. *Not memory, and not disk space.* No unit added more than 1.8 MB
   (56.9 MB in the benchmark list, for a sort), and 20,000 units of the
   write-heavy workload added 4.4 MB to a 1.5 GB database.

### The benchmark list

224 units, 7.2 s, of which 7.0 s on the CPU. Searches without the
index (22 units: patterns the index cannot narrow, and files named on
the command line) take 64% of the CPU and searches with it 17%. The new
constructs are cheap: the ten `for` units together take 126 ms, and
most of that is the `grep` and `echo` they run. The eight `diff` units
take 163 ms, nearly all in the two that compare 1,432 changed files
line by line. The six `awk` units that start a line take 7 ms.

## 6. apply_patch

Not built, at the owner's request ("Skip the apply_patch thing"). There
is no command, no test, no benchmark row and no character table. The
name is refused like any unknown command.

## 7. What is still refused

Refused by name before anything runs, with status 2:

- **Shell:** `case`, functions, `select`, `[[ ]]`, `(( ))` as a command,
  C-style `for ((;;))`, subshells `( )` and groups `{ }`, background
  `&`, process substitution, arrays, `${NAME%pattern}` and every
  expansion form but `${NAME}`, `${#NAME}`, `${NAME:-word}`,
  `${NAME:=word}`; `$1`, `$@`, `$$`, `$!`; `local`, `eval`, `source`,
  `trap`, `set`, `shift`, `exit`, `return`; `cd -` (use
  `cd "$OLDPWD"`) and `cd -L/-P`; aliases; here-strings.
- **diff:** `-c`, `-y`, `-e`, `-n`, `-I`, `--from-file`, and the other
  output formats; two inputs whose differing middle is over 256 MB.
- **awk:** user-defined functions, `getline`, `system`, output to files
  and pipes, `delete`, `exit`, `do … while`, `nextfile`, `switch`,
  arrays of more than one subscript, the built-in functions outside the
  brief's list (`int`, `sqrt`, `log`, `gensub`, …), special variables
  outside the list (`RS`, `OFMT`, `ENVIRON`, …), `-f` and every option
  but `-F` and `-v`, `NAME=value` among the file names. At run time:
  input that is not ASCII.
- **Inside `find -exec` and `xargs`:** `cd`, `export`, `unset`, `read`,
  `break`, `continue`, `:`.
- Everything stage 4 refused and this stage did not add: links,
  permissions, owners, `sed` scripts beyond the listed commands, the
  flags not in the README tables.

A name or flag that only an expansion reveals (`X=perl; $X`) is refused
when the unit reaches it; what ran before stays done.

## 8. Differences from the real tools

Kept from stage 4: `ls -l` prints fixed modes and owners, folders are
4096 bytes, dates are UTC, `du` counts content in 4 KB blocks.

New in this stage:

- **A whole construct redirected to a file.** `for …; done > f` gives
  `f` its content when the construct ends. A command inside the loop
  that reads `f`, or appends to it itself, sees it without the loop's
  output so far. bash writes as it goes.
- **A session starts with** `HOME=/`, `PWD=/`, the usual `IFS` and
  nothing else: no `PATH`, `USER` or environment of the caller.
- **The command limit.** A unit is stopped after 1,000,000 commands
  (status 3); bash would run on. What it wrote before stays.
- **diff headers** show the files' stored modification times in UTC;
  GNU diff shows local time with nanoseconds. `--label` gives the same
  text on both. Two unrelated inputs with a differing middle over
  256 MB are refused where GNU diff would try.
- **awk `for (k in a)`** visits keys in the order they were first set.
  gawk's order is its own; the language leaves it open. Sort the output.
- **awk syntax errors.** The first complaint and the status are gawk's.
  Where gawk prints several complaints for one mistake, or puts its
  mark one column off (as for `--1`), the text differs.
- **awk `\S` and `\W`** do not match a newline inside a string; gawk's
  do. Records never hold one, so this shows only in strings a program
  builds.
- **awk's own two limits:** 1,000,000,000 loop turns and strings of
  256 MB; past either it stops with status 2.
- **awk reading non-ASCII input** stops with status 2 at that line,
  having processed the lines before it; gawk in the C locale treats the
  bytes as characters and goes on.

## 9. For the stage after this

**Which work would gain most from more threads.** Searches, by a wide
margin: they are 97% or more of the CPU in any workload that is not
nearly all writes, they change nothing, and the content they read is in
pieces that are never changed in place. Two ways, in order of value:

1. *Several units at once, when they only read.* The simplest gain and
   the one several users need anyway: N threads give close to N times
   the units a second on the read-heavy workload. It needs the tree and
   the index to be readable while a write is being prepared, which the
   storage design allows (a write adds pieces and then switches the
   tree), and it needs the counters to become per unit instead of per
   database.
2. *One search over several threads.* A recursive search splits by
   file. This is what makes one user's `grep -rn common-word .` go from
   0.5 s toward 0.15 s. Output order must stay the tree's order.

Writes gain nothing from threads; they gain from sharing a sync. With
several users, units that arrive within a fraction of a millisecond of
each other can be logged together and answered after one sync. On this
disk that is the difference between about 3,000 and well over 10,000
small writes a second.

`sort` over a very large input and `diff -r` over many changed files
are the other two places where one command uses real CPU; both split
well, and neither showed up in the workloads.

**What several users would need from sessions.** A session already
holds everything that belongs to one user (variables, current
directory, `$?`) apart from the database, and the library takes a
session as an argument. What is missing:

- *A process that holds one database and many sessions,* with a way to
  name a session and to end it. Today each `qdb` process locks the
  database for itself.
- *A rule for units that overlap.* The storage already makes a unit all
  or nothing after a crash. With several users it must also say what a
  unit sees: the simplest rule that fits the design is one writing unit
  at a time, and every reading unit sees the tree as it stood when the
  unit began.
- *Limits per session,* so one user cannot starve the rest: the command
  limit exists; a limit on CPU time or text read per unit (the counters
  make this easy), on memory, and on scratch space would be new.
- *Fairness.* A 600 ms search must not hold up a 0.1 ms write. With
  readers on their own threads this mostly follows; the writer queue
  needs an order.
- *Costs per session.* The counters are per unit now; adding them up by
  session gives accounting for nothing extra.
- *Who may do what.* The data model has no owners or permissions. If
  users must not see or change each other's folders, that is a new fact
  per folder and a check in the one place where paths are resolved.

## 10. Deviations, gaps and questions

**Deviations**

- apply_patch was skipped, as said.
- The reference awk is gawk 5.2.1 from the Ubuntu archive, unpacked
  into the data folder and not installed (the machine's own awk is
  mawk). The harness finds it there or through `QDB_GAWK`; without it,
  awk cases run against whatever `awk` is on the machine and many fail.
- The benchmark's awk commands over many files pipe through
  `tr -cd '[:print:][:space:]'` first, because of the ASCII rule. Both
  sides run the same pipeline, so the comparison is fair, but part of
  each time is `cat` and `tr`.
- "Cost of counting" was measured by adding a switch that turns the
  counting off (`qdb::tree::set_counting`), used only by
  `qdb-bench --paired-counting`. With counting off one test of a flag
  remains at every count, so the true cost is that much higher than
  measured; it is below what the method can see.

**Gaps**

- The stage 4 generated families were not rerun at 10,000 cases on the
  final build, and the crash test ran 300 cuts and 30 kills on it, not
  the 10,000 and 1,000 of stage 4's final run.
- awk does not use the search index: `awk '/word/'` over a tree reads
  every file. The brief allowed that; `grep -l word | xargs awk …` is
  the fast form.
- `rg` on 1 GB of standard input is 1.3 times slower than real rg.
- The longest-search times vary by a third between runs on this
  machine, so "no slower than stage 4" can be shown on the counters
  only.

**Questions**

1. *awk and non-ASCII input.* The brief says to refuse it. On real
   trees that makes awk stop on ordinary files (a name with an accent
   in a comment). gawk in the C locale simply treats every byte as a
   character, and doing the same would be less code than the check.
   Should awk do that, with only `length`, `substr`, `index`,
   `toupper` and `tolower` documented as counting bytes?
2. *awk `exit`, `delete` and `do … while`.* They are not in the brief's
   table, so they are refused, but `exit` in particular is common
   (`awk '/x/ { print; exit }'`). Wanted?
3. *`cd -`.* Refused because it prints the directory as a side effect
   and the brief did not list it. Wanted?
4. *The command limit* is 1,000,000 commands a unit, and the unit's
   writes up to the limit are kept. Is keeping them right, or should a
   stopped unit leave nothing?
5. *apply_patch.* Skipped for now or dropped? The storage side it
   needs (many files changed in one unit with one sync, all or nothing)
   is already what a loop of writes gets.
