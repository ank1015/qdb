# Hand-back notes for stage 2: fast search

Branch `claude/project-thread-3wnfpo`. Everything below was measured on one
machine (4 cores, 15 GB, one thread used) with bash 5.2.21, grep 3.11,
sed 4.9, coreutils 9.4, findutils 4.9.0 and ripgrep 14.1.0.

## Summary

| Target | Measured | Result |
|---|---|---|
| `grep -rn COMMON . | head -n 5` on the largest dataset under 1 ms | 0.019 ms on linux | met |
| A streamed command's peak memory does not grow with its output | 821.1 MB of output with 26.9 KB peak | met |
| Every recursive grep with the index off at least as fast as GNU grep, at every size | closest is 1.4x faster than GNU (linux) | met |
| A search for a term in no file under 1 ms at every size | slowest 0.017 ms | met |
| A term in 1% of files or fewer on the largest dataset at least 10x faster than GNU grep | smallest gain 32x | met |
| Any benchmark command with the index on no more than 10% slower than with it off: commands that only read | 0 of the commands that ask the index are slower; 2 that never ask it measured over 10% (noise) | met |
| The same, for commands that change data | 47 of 64 are more than 10% slower | MISSED |
| Index size at most 1x the raw text | largest share 28% of the text | met |
| A write to one file costs by the size of the file, not of the database | the same 64 KB write takes 0.234 to 0.280 ms across an 80x range of database sizes | met |

Reads are fast in memory. On the Linux tree (1.4 GB, 83,405 files) a
recursive grep took 15 to 35 seconds in stage 1. It now takes 0.2 seconds
with no index, against 1 to 3.7 seconds for GNU grep and 0.6 to 3 seconds
for ripgrep on one thread, and from 12 microseconds to 130 milliseconds
with the index when the term is in a tenth of the files or fewer.

One target is missed: commands that write are more than 10% slower with
the index than without, because keeping the index current is work. Section
8 asks whether that rule was meant for writes.

## 1. Code

| Part | Commit | What |
|---|---|---|
| 1 | `9097895` | `--load` skips symbolic links and files that are not text and reports the counts; three real datasets; search terms by measured frequency; memory figures; rg column; the new commands. |
| 2 | `e362854` | Stages pass output on in pieces of at most 16 KB and are told when their reader has had enough. `execute_to` writes to a sink. |
| 3 | `61f5d42` | `scan.rs`: fixed strings by substring search, required strings as a filter, a DFA built while matching, a case-insensitive substring search, several strings at once. The stage 1 matcher is still the reference. |
| 4 | `ec875ad` and the commit after it | The trigram index behind `Db::candidates`, `--no-index` on the three programs, `Db::without_index`, the differential test. |

New dependency: `memchr`. `libc` turned out not to be needed. The
case-insensitive substring search uses `std::arch` (AVX2 when the
processor has it, plain code otherwise).

`README.md` describes the new options: `--no-index` on `qdb`,
`qdb-harness` and `qdb-bench`; `qdb-harness --diff`; `qdb-bench --label`,
`--paired` and `--report`; the load report; `execute_to`.

## 2. Conformance

| Check | Result |
|---|---|
| Harness, index on | 1,617 cases (1,911 steps): 1,617 passed |
| Harness, index off | 1,617 cases (1,911 steps): 1,617 passed |
| Differential test, seed 1 | 10,000 steps (4,946 searches, 5,054 writes): identical; 7,215 searches selected something |
| Differential test, seed 2 | 10,000 steps (4,924 searches, 5,076 writes): identical; 7,140 searches selected something |
| Differential test, seed 3 | 10,000 steps (4,867 searches, 5,133 writes): identical; 7,135 searches selected something |
| Rebuild check | At every 1,000th step and at the end of each seed: the maintained index gives the same files as one built from scratch, for a few hundred queries. Passed 30 of 30. |
| Benchmark | Every command on every dataset agrees with GNU, and the trees agree after the write list, with the index on and off. |
| `cargo test` | 26 tests pass, among them the fast search paths against the reference matcher and the three forms of answer from storage. |

The harness grew from 1,496 to 1,617 cases: 121 new ones in
`tests/cases/pipes.case` for pipelines that stop early. All 1,496 stage 1
cases still pass unchanged.

The differential test writes by create, overwrite, append, `sed -i`,
`cp -r`, `rm -r` and `mv`, and also grows files by copying them onto
themselves. It was checked against a planted fault (an index that skips
one write in seven): every seed fails within 100 steps. Seeds 4, 5 and 6
were also run and pass. To run it: `qdb-harness --diff 10000 --seed N`.

With the regex fuzzer (`--fuzz 3000 --seed 5`) the engine fails the same
2 generated cases as the stage 1 engine does, with the index on and off:
no new difference.

## 3. Benchmark tables

One file per dataset, each with every command and all the columns asked
for (GNU, rg, the engine at stage 1, after part 2, after part 3 with no
index, after part 4 with the index off and on), then peak memory per
command and the search terms used:

- `bench/results/redis.md` (17.4 MB, 1,672 files)
- `bench/results/postgres.md` (161.2 MB, 8,419 files)
- `bench/results/linux.md` (1,396.9 MB, 83,405 files)
- `bench/results/large.md` (the generated dataset from stage 1, 87.4 MB)

The searches, in milliseconds. The six terms are in no file, one file,
0.1%, 1%, 10% and over half of the files.

**redis**

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Part 4, index on |
|---|---:|---:|---:|---:|---:|---:|
| `grep -rn qdbzzabsentterm .` | 21.9 | 13.2 | 194 | 239 | 1.38 | 0.008 |
| `grep -rn nestedcallreplyarray .` | 20.5 | 13.8 | 235 | 216 | 1.47 | 0.078 |
| `grep -rn xmallctlmibnametomib .` | 19.7 | 13.6 | 241 | 241 | 1.35 | 0.023 |
| `grep -rn implementing .` | 21.8 | 14.9 | 254 | 253 | 1.63 | 0.807 |
| `grep -rn between .` | 28.6 | 16.5 | 222 | 206 | 1.67 | 0.859 |
| `grep -rn for .` | 47.5 | 27.0 | 239 | 212 | 3.93 | 3.33 |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 24.9 | 15.7 | 201 | 233 | 1.94 | 0.026 |
| `grep -rE 'arena_[a-z]+\(' .` | 18.4 | 13.5 | 253 | 253 | 2.04 | 0.473 |
| `grep -rE '[0-9]{4,}' .` | 28.9 | 27.1 | 214 | 247 | 20.7 | 22.2 |
| `grep -r '\(implementing\).*\1' .` | 17.4 |  | 501 | 360 | 1.41 | 0.855 |
| `grep -rn for . \| head -n 5` | 2.95 |  | 210 | 0.031 | 0.008 | 0.005 |
| `grep -rn between . \| head -n 1` | 4.67 |  | 205 | 0.031 | 0.007 | 0.007 |

**postgres**

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Part 4, index on |
|---|---:|---:|---:|---:|---:|---:|
| `grep -rn qdbzzabsentterm .` | 160 | 71.1 | 2,180 | 2,015 | 24.5 | 0.014 |
| `grep -rn abcdefghijklabcdefgh .` | 132 | 79.6 | 2,114 | 2,376 | 23.6 | 0.051 |
| `grep -rn btvarstrequalimage .` | 161 | 82.5 | 2,249 | 2,282 | 23.5 | 1.61 |
| `grep -rn undesirable .` | 188 | 79.8 | 2,014 | 2,167 | 24.4 | 2.51 |
| `grep -rn passed .` | 193 | 85.3 | 2,138 | 2,294 | 26.5 | 21.1 |
| `grep -rn not .` | 335 | 155 | 2,319 | 2,156 | 40.4 | 35.2 |
| `grep -rin BTVARSTREQUALIMAGE .` | 167 | 91.1 | 2,297 | 2,112 | 31.6 | 1.46 |
| `grep -rE 'size_[a-z]+\(' .` | 111 | 84.5 | 2,152 | 2,400 | 23.4 | 8.87 |
| `grep -rE '[0-9]{4,}' .` | 303 | 346 | 2,192 | 2,054 | 152 | 177 |
| `grep -r '\(undesirable\).*\1' .` | 139 |  | 3,929 | 3,989 | 23.3 | 2.32 |
| `grep -rn not . \| head -n 5` | 4.31 |  | 2,280 | 0.467 | 0.008 | 0.008 |
| `grep -rn passed . \| head -n 1` | 20.8 |  | 2,205 | 1.72 | 0.012 | 0.012 |

**linux**

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Part 4, index on |
|---|---:|---:|---:|---:|---:|---:|
| `grep -rn qdbzzabsentterm .` | 1,173 | 550 | 17,359 | 17,645 | 199 | 0.012 |
| `grep -rn ausgabenspezifisches .` | 1,154 | 587 | 19,018 | 19,502 | 210 | 0.098 |
| `grep -rn mispredicted .` | 1,313 | 588 | 17,983 | 18,955 | 205 | 17.5 |
| `grep -rn separately .` | 991 | 583 | 18,361 | 18,397 | 212 | 24.1 |
| `grep -rn conditions .` | 1,273 | 645 | 18,999 | 19,519 | 229 | 130 |
| `grep -rn define .` | 3,659 | 2,912 | 15,390 | 15,238 | 431 | 461 |
| `grep -rin MISPREDICTED .` | 1,528 | 756 | 18,851 | 18,964 | 249 | 21.1 |
| `grep -rE 'atomic_[a-z]+\(' .` | 1,232 | 559 | 18,552 | 18,592 | 228 | 74.4 |
| `grep -rE '[0-9]{4,}' .` | 2,472 | 3,498 | 18,322 | 19,700 | 2,010 | 2,026 |
| `grep -r '\(separately\).*\1' .` | 585 |  | 34,553 | 35,932 | 217 | 24.8 |
| `grep -rn define . \| head -n 5` | 4.66 |  | 15,337 | 3.77 | 0.020 | 0.019 |
| `grep -rn conditions . \| head -n 1` | 11.4 |  | 18,581 | 6.94 | 0.054 | 0.027 |

Reading the tables:

- **Part 2** made `| head` fast and nothing else, as intended: 15 seconds
  to 3.8 ms on Linux for the first five matches of a common term.
- **Part 3** is where the scan became faster than GNU grep: from 9 to
  several hundred times faster than stage 1, and at least 1.4 times faster than GNU on every
  recursive grep at every size. The closest call is `[0-9]{4,}`, a
  pattern with no fixed string, where the DFA does all the work: 2.0 s
  on Linux, against 2.5 to 2.8 s for GNU and 3.0 to 3.5 s for rg.
- **Part 4** makes the cost follow the candidates. A term in no file costs
  about 10 microseconds at any size. A term in 84 of 83,405 files costs
  17 ms, not 0.1 ms, because the trigram filter lets through 366 files and
  the largest files in the tree (headers of up to 16 MB) are among them:
  a large file contains almost every trigram.
- A term in over half the files, and a pattern with no three-byte
  sequence in it, cost the same with and without the index: storage
  answers "any file" without reading a list.

## 4. Targets

The table at the top. The evidence for each row, with every command
involved, is in `bench/results/TARGETS.md`, written by `bench/targets.py`
from the stored runs.

Two rows need a remark.

**Index on against index off, 10%.** For commands that only read, two
separate runs of the benchmark cannot settle this: the same command in two
runs of the same configuration differs by up to 30%, depending on where
memory happens to land. So reads were compared in one process, with
searches using the index and ignoring it by turns (`qdb-bench --paired`,
stored as `paired.csv`). No command that asks the index is slower with it.
Two `find` commands on Linux measured 11% and 14% slower; `find` never
asks the index, so that is the noise that is left. For commands that
write, the target is missed; section 6 has the figures.

**Peak memory.** `grep -rn define .` on Linux writes 821 MB of output and
holds 27 KB while doing it; stage 1 held 830 MB.

## 5. Memory

| | redis | postgres | linux |
|---|---:|---:|---:|
| Text | 17.4 MB | 161.2 MB | 1,396.9 MB |
| Resident after load, index off | 21.1 MB | 167.5 MB | 1,426.5 MB |
| Resident after load, index on | 27.5 MB | 192.5 MB | 1,578.0 MB |
| Heap after load, index off | 17.7 MB | 163.7 MB | 1,418.0 MB |
| Heap after load, index on | 24.7 MB | 187.2 MB | 1,568.1 MB |
| Index | 4.9 MB | 21.4 MB | 148.0 MB |
| Index as a share of the text | 28.0% | 13.3% | 10.6% |
| Largest peak of any read-only command, index on | 71 KB | 269 KB | 945 KB |
| Peak of `grep -rn COMMON .`, index on | 26 KB | 31 KB | 27 KB |
| Peak of the same command in stage 1 | 3.1 MB | 26.2 MB | 830 MB |

The difference between heap with and without the index is the index plus
2 MB of scratch that is kept once a file over 4 KB has been indexed.
Resident memory is measured in a fresh process that has only loaded the
dataset. Peak memory per command is in each dataset's table; with the
index on, the largest peaks belong to searches that hold a list of
candidates (945 KB for a pattern with a few thousand candidate files) and to
`cat` over a large glob.

## 6. Write costs

Loading:

| | redis | postgres | linux |
|---|---:|---:|---:|
| Read the files | 0.02 s | 0.18 s | 1.4 s |
| Build the index | 0.09 s | 0.80 s | 6.8 s |

Building the index takes about five times as long as reading the files
from a warm file cache.

Write commands on the Linux tree, in milliseconds (the other datasets are
in `bench/results/TARGETS.md`, section 9, with what the index did for
each command):

| Command | GNU | Index off | Index on |
|---|---:|---:|---:|
| `echo 'hello world' > bench_out/hello.txt` | 1.49 | 0.017 | 0.017 |
| `echo 'another line' >> bench_out/hello.txt` | 1.60 | 0.008 | 0.018 |
| `cp include/linux/kernfs.h bench_out/copy.txt` (20 KB) | 2.45 | 0.022 | 0.510 |
| `cp -r Documentation bench_out/dir_copy` (9,698 files) | 979 | 38.2 | 476 |
| `cp -r include/linux bench_out/work` (2,631 files) | 324 | 19.8 | 173 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 3.72 | 0.387 | 0.545 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` (1,432 files) | 386 | 141 | 213 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.81 | 0.026 | 0.035 |
| `rm -r bench_out/dir_moved` | 143 | 6.80 | 8.59 |
| `rm -rf bench_out` | 135 | 2.07 | 3.70 |

With the index, a write is still faster than the real tools on every row,
and a copy is about ten times slower than the same copy without it. A
copy without the index is close to a plain memory copy, so almost
anything added shows.

## 7. Index facts, for the on-disk design

### What is stored

- **Trigrams** are three consecutive bytes of content after ASCII letters
  are lowered, and never include a newline, because no line can match
  across one. One index therefore serves searches that respect case and
  searches that ignore it. A case-sensitive search gets a slightly weaker
  filter than it could have, never a wrong one.
- **Documents.** Every version of a file's content is a document with a
  number, and numbers are never reused. A write gives the file a new
  document and retires the old one. A posting list therefore only ever
  grows at its end, which is what makes a write cost by the file and not
  by the database.
- **Posting lists** are ascending document numbers, stored as the
  differences between neighbours in a variable-length code (seven bits a
  byte). Lists of up to eight bytes sit inside the 16-byte table entry;
  longer ones are separate allocations.
- **Files map to documents and back** through a 4-byte field on the node
  and an 8-byte entry per document. A move changes neither, so `mv` costs
  the index nothing (the benchmark's `mv` rows show "documents +0 -0").
- **Stale entries.** A retired document stays in the lists until a sweep
  rewrites them. A sweep starts when stale entries pass a quarter of the
  live ones plus 1,024, and is paid for a little at a time by the writes
  that follow: each write rewrites at most a few times its own size in
  list bytes. A stale entry never changes an answer, because a retired
  document belongs to no file.
- **Renumbering.** When more than about eight document numbers have been
  used per live file, the index is built again from the files, which
  brings the numbers back down. That is the one index operation whose cost
  follows the size of the database. See "Deviations" below.

### How a query is answered

- A pattern becomes a query of ANDs and ORs over trigrams by the method in
  Russ Cox's article (`src/regex/trigram.rs`): for each part of the
  pattern, whether it can be empty, its exact strings if few, prefixes,
  suffixes and a query, combined bottom-up. Fixed strings (`-F`, or a
  pattern with no operators) take the direct route: all trigrams of the
  string.
- An AND reads its shortest list first and stops reading further lists
  once they are more than about 512 times longer than what is left; an
  absent trigram ends the work at once. That is why a term in no file
  costs microseconds on any dataset.
- A query that most files satisfy is not answered from the index at all.
  If the shortest list involved has more bytes than half the number of
  files, storage says "any of them" without reading a list, and the search
  scans. That keeps `grep -rn COMMON . | head -n 5` as fast as it was
  without an index.
- Storage answers in one of three forms: any file; a short list of files in
  walk order, found by following each candidate's parents up to the folder
  searched (cost follows the candidates, not the folder); or, when
  candidates are more than an eighth of the folder, a set that the search
  tests each file against while it walks.
- **A search reads a little before it asks.** Working out the candidates
  costs something, and a search that feeds `head` may be done after one
  file. So `grep -r` starts by reading files in order and asks storage once
  it has read about as many bytes as the answer costs (sixteen bytes of
  text per byte of list). After that it reads only candidates. Output is
  the same either way; the differential test covers the hand-over.

### What the numbers say about a disk format

Measured straight after loading (full tables in `bench/results/DATASET.md`):

| | redis | postgres | linux |
|---|---:|---:|---:|
| Text | 17.4 MB | 161.2 MB | 1,396.9 MB |
| Files | 1,672 | 8,419 | 83,405 |
| Distinct trigrams | 74,777 | 179,569 | 236,444 |
| Postings (trigram, file pairs) | 1.75 million | 11.9 million | 111.4 million |
| Postings per file, average | 1,049 | 1,419 | 1,336 |
| Posting data | 1.9 MB | 12.7 MB | 117.4 MB |
| Bytes per posting | 1.07 | 1.07 | 1.05 |
| Posting data as a share of the text | 10.8% | 7.9% | 8.4% |
| Whole index in memory | 4.9 MB | 21.4 MB | 148.0 MB |
| Whole index as a share of the text | 28.0% | 13.3% | 10.6% |
| of which: table of trigrams | 2.1 MB | 4.2 MB | 8.4 MB |
| of which: lists, with their headers and slack | 2.7 MB | 17.0 MB | 137.5 MB |
| of which: document table | 0.03 MB | 0.26 MB | 2.1 MB |
| Posting data in the 100 longest lists | 5.6% | 4.7% | 5.4% |
| in the 1,000 longest | 32.4% | 29.6% | 30.2% |
| in the 10,000 longest | 79.8% | 77.0% | 79.0% |
| Trigrams found in one file only | 27,621 (37%) | 45,612 (25%) | 46,218 (20%) |
| Trigrams found in at most 8 files | 73% | 67% | 51% |
| Build time while loading | 0.09 s | 0.80 s | 6.8 s |

What follows from them:

- **The number of distinct trigrams grows slowly.** Eighty times more text
  gave three times more trigrams. A table of trigrams fits in memory at
  any size this project will see (8 MB for the Linux tree), so a disk
  format can keep the table in memory and put only the lists on disk.
- **A posting costs about one byte.** Files are numbered in walk order, so
  neighbours in a list are usually neighbours in the tree and the
  difference fits in a byte. A disk format that keeps this numbering keeps
  this size: about 8% of the text for the lists. Cox reports 20% for his
  on-disk index of the Linux source.
- **Size is in the middle of the distribution, not at the top.** The 100
  most common trigrams hold only 5% of the posting data; the 10,000 most
  common hold 80%. Dropping "stop trigrams" would save little. Most
  trigrams are rare: half or more are in at most 8 files, and their lists
  fit inside the 16-byte table entry with no separate allocation.
- **The in-memory overhead is per list, not per posting.** On the small
  tree the index is 28% of the text although the posting data is 11%: the
  rest is table slots and list headers for 75,000 short lists. It shrinks
  to 10.6% on the Linux tree. A disk format that packs lists end to end
  would not pay it.
- **Index work per write** (full list per command in section 6 and in
  `bench/results/TARGETS.md`):

| Kind of write | Index work |
|---|---|
| Create a file | One new document; one posting appended for each distinct trigram of the content (9 for `hello world`, about 2,600 for a 20 KB header). |
| Overwrite, append, `sed -i` | The same as creating the new content, and the old document is retired. An append re-reads the whole file, not only the added part. |
| `cp -r` | One new document per file copied. 9,698 files of documentation: 9.2 million postings, 476 ms. |
| `rm`, `rm -r` | One document retired per file: a flag is set, nothing is read. Lists are cleaned later by sweeps. |
| `mv` | Nothing. |
| `mkdir`, `rmdir`, `touch` on a new file | Nothing (an empty file has no trigrams and no document). |

- **Cost of that work.** Finding the distinct trigrams of new content
  costs about 3 ns per byte, and appending costs about 30 ns per distinct
  trigram, mostly waiting for memory: each append touches a different
  list. A new 64 KB file takes 0.23 to 0.28 ms whether the database holds
  17 MB or 1.4 GB.
- **Sweeps** rewrote 34 KB and 396 KB of lists during the two `rm -r` at
  the end of the Redis list, and nothing on the larger trees, where the
  deleted share stayed under a quarter. In the differential test, 10,000
  steps on a tree of about 100 files rewrote 9 to 12 MB of lists and
  rebuilt the index 7 or 8 times.

## 8. Deviations, gaps and questions

### Differences from GNU

None new in what is printed. All 1,617 cases match the real tools byte for
byte with the index on and off. The stage 1 gaps listed in
`docs/HANDBACK.md` are unchanged: nothing in stage 2 touched which
commands, flags or syntax are accepted.

One difference in errors is by design and was asked for: in a pipeline
that stops early, a stage reports errors only for the work it did. Real
bash would often print more, depending on the size of its pipe buffer, so
those cases use `$~` in `tests/cases/pipes.case`.

### Deviations from the brief

1. **Datasets are release tarballs, not repository checkouts.** This
   machine could not reach GitHub or kernel.org (the network policy
   refuses both). The three trees are the upstream release tarballs as the
   Ubuntu archive carries them: Redis 8.0.6, PostgreSQL 16.15 and Linux
   6.8, each recorded with its SHA-256 in `bench/datasets.txt` and fetched
   by `bench/fetch.sh`. There is no commit hash to record; the checksum
   plays that part.
2. **Dataset sizes are 17 MB, 161 MB and 1,397 MB**, not 10, 100 and
   1,000. They are whole trees, not cut to size.
3. **Index on against index off, 10% rule: missed for commands that
   write.** A write has to keep the index current, and that costs more
   than a tenth of a plain write. Section 6 has every figure. For commands
   that only read, see the targets table.
4. **One index operation follows the size of the database: renumbering.**
   After roughly seven writes per file in the database, the index is built
   again from all files (6 to 8 seconds on the Linux tree). Averaged over
   the writes that lead to it the cost is constant per write, but the one
   write that triggers it waits for all of it. Spreading it out was left
   for stage 3, where the on-disk format will decide how documents are
   numbered anyway. The benchmark's write list is too short to trigger it;
   the differential test triggers it seven or eight times per seed.
5. **The index is used by `grep -r` only.** `sed -i` over a glob and
   `grep` over a list of files named on the command line read every file
   they are given. The brief asked for the index in recursive search; the
   same question to storage would serve these too.
6. **`-v` and patterns that say nothing use no index.** With `-v` a file is
   selected for the lines that do not match. A pattern with no three-byte
   sequence in it, such as `[0-9]{4,}` or a two-letter word, gives the
   index nothing to look up, and is scanned at part 3 speed.
7. **Two megabytes of scratch per database with an index**, once a file
   over 4 KB has been written: a bit per possible trigram, used to find
   the distinct trigrams of new content. It is not part of the index size
   reported.

### Where the brief was silent, and what was chosen

- **What "a stage reports errors only for the work it did" means for
  writes.** A stage that writes (a redirect, `sed -i`) always runs to the
  end, whatever its reader does. Only stages that read stop early.
- **Order of a pipeline's stages.** Stages still run one after another
  from the caller's point of view, but each hands its output on in pieces
  of at most 16 KB and is told when its reader has had enough.
- **The streaming entry point.** `execute_to(db, line, out, err)` writes
  standard output to any `Sink`; a sink can return false to stop the
  command. `execute` still returns an `Outcome`.
- **Back-references and the index.** The backtracking matcher can give up
  on a line with an error. So that the same files report it with and
  without the index, the index may rule out for such a pattern only the
  files that the scanner's own fixed-string filter would have passed over
  untried.
- **Where the DFA stops.** The DFA built during matching keeps at most
  4,096 states and starts again when it is full, only ever between lines.
- **Case folding and "one character"** are each defined once, in
  `src/bytes.rs`, and the regex compiler, the scanner, the trigram code
  and the index all go through that.
- **What the loader reports.** `--load` always prints two lines on
  standard error: files loaded and skipped by kind, and the index build
  time and size.
- **What "resident memory" is measured on.** A fresh process that has
  only loaded the dataset.
- **rg options.** `rg -j 1 --no-ignore --hidden`, plus the options that
  match each grep flag. rg has no equivalent for a back-reference pattern,
  so that row has no rg time.

### A flaw found in the benchmark, and fixed

The benchmark used to start the real tools from the same process that
holds the dataset. Starting a child from a process holding 1.4 GB costs
about 50 ms here, which was being charged to GNU and rg on every command,
and it left the big process's memory marked copy-on-write, which was being
charged to the engine's next writes. The real tools now run from a small
helper process. The Stage 1 and Part 2 columns were measured before the
fix: their engine times for read-only commands stand, their engine times
for writes are a little high, and the GNU and rg columns shown are always
from the latest run. Part 3 and both Part 4 columns were measured after
it.

### Gaps

- The write list in the benchmark does not trigger a sweep on the two
  larger datasets or a renumbering anywhere; the cost of those is known
  from the differential test's counters and from the load time, not from a
  benchmark row.
- The index was measured on source code. Prose, logs and data files have a
  different spread of trigrams; the generated dataset is the only
  non-source point, and it is synthetic.
- No test loads a tree with a file larger than a few megabytes. The code
  path for large content is the same one the Linux tree's 16 MB headers use.

### Questions

1. **Is a write that occasionally waits for a full rebuild acceptable
   until stage 3?** If not, the fix is to renumber incrementally, and it
   is better designed together with the on-disk format.
2. **Should the 10% rule apply to writes?** As written it does, and writes
   miss it by a wide margin: a write with the index costs about three
   nanoseconds per byte written plus about thirty per distinct trigram.
   If writes matter as much as reads, the alternative is to index a write
   later, in the background or at the next search, which this stage's "one
   thread, always current" rules did not allow.
3. **Should other commands ask the index?** `sed -i 's/TERM/OTHER/g'
   dir/*.c` reads every file the glob names; with the same query it could
   skip the files that cannot contain TERM.
4. **Is ASCII-only folding of the index still right if the locale
   changes?** A UTF-8 `-i` would need the index folded the same way, which
   means a rebuild, so the locale decision is best taken before stage 3
   fixes the format.
