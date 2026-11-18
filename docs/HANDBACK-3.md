# Hand-back notes for stage 3: storage on disk

Branch `claude/project-thread-3wnfpo`. Everything below was measured on
one machine (4 cores, 15 GB, one thread used, a virtual disk with ext4)
with bash 5.2.21, grep 3.11, sed 4.9, coreutils 9.4, findutils 4.9.0 and
ripgrep 14.1.0. The brief is in `docs/briefs/stage-3-storage-on-disk.md`,
the format in `docs/FORMAT.md`.

## Summary

All six parts are done. Eighteen of the nineteen targets are met; one is
not met reliably.

| Target | Measured | Result |
|---|---|---|
| Harness in memory, on disk, reopened after every step | 1,617 of 1,617 in each | met |
| Open after a clean close under 200 ms | 28 ms (stage 2 took 8.2 s to start) | met |
| Import into `--db` at most 16.4 s | 13.8 s | met |
| Read-only commands, settled, within 1.5x of stage 2 in memory | 62 of 62; the median command takes 0.77 of its stage 2 time | met |
| Disk settled at most 1.2x raw text | 1.109 | met |
| 10,000 power cuts on each of 3 seeds | 110,000 in 11 runs, all recovered | met |
| 1,000 `kill -9` | 2,000, all recovered | met |
| One sync per command line under always | 90,140 of the 90,176 lines that synced did so once | met |
| Small write under always within 1.2x of a bare append and sync | 1.01x (0.153 against 0.152 ms) | met |
| Small write under batch, median under 0.1 ms | 0.012 ms | met |
| Open after a crash under 2 s | 0.13 to 0.30 s | met |
| Own memory after open under 200 MB | 21.7 MB (stage 2 held 1,578 MB) | met |
| Whole benchmark under a 512 MB limit, identical results | completed, identical | met |
| Append a line to a 1 GB file under 1 ms | 0.03 to 0.14 ms, about 120 bytes written | met |
| `sed -i` on one line of a 1 GB file writes a few pieces | one piece, 2.4 ms | met |
| Search for the term in 84 files under 5 ms | 2.5 ms (was 17.5) | met |
| Slowest small write of 100,000, batch, under 50 ms and no larger on Linux than PostgreSQL | Linux 90 ms and 11 ms in two runs of the same writes; PostgreSQL 46 ms | NOT MET reliably |
| Disk at any moment of that run at most 2x live text | worst 1.43x (Linux), 1.76x (PostgreSQL) | met |
| Both comparison tables | delivered | met |

`bench/results/TARGETS-3.md` has the same table with the file each figure
comes from.

The one miss: the engine's own work in a small write is now bounded (the
slowest small write in memory is 15 ms, and that one finishes an index
file), but every slow small write on disk is a single sync of a few
hundred bytes that the disk took 20 to 150 ms over. The same 100,000
writes on the same build gave a slowest write of 11 ms in one run and 90
ms in the next. Section 9 has the evidence and what could be done.

## 1. Code

| Part | Commit | What |
|---|---|---|
| 1 | `81f7cdd`, `f5e6f68` | A write that changes nothing writes nothing; `sed -i` and `grep` over named files ask the index; a prose dataset, a 1 GB file run, a 100,000-write run. |
| 2 | `45edcc4` | Storage on disk: data files, the saved tree, MANIFEST, index files; `--db`, `--check`, `--stats`, `--compact`, `--dump`; harness modes `disk` and `reopen`. |
| 3 | `1393460`, `cbf12f6` | The log of command lines, sync rules, recovery; a simulated disk that loses what was not synced; the power-cut test, the kill test, faults put in on purpose; `docs/FORMAT.md`. |
| 4 to 6 | `5b936d6`, `d40fe1b` | Redirected output past 8 MB waits in a scratch file; large files stored and compared in a stream; upkeep in steps (index files, saves of the tree, moving content, removing files); `--compress`; `--index later`; the comparison, sync-cost and cold runs. |
| docs | `b8dc7bc` and the commit with this file | README files, the fault list, results, this hand-back. |

Source layout: `src/vfs.rs` is the only code that touches files (a real
file system, and an in-memory one that can lose unsynced writes on
demand). `src/disk.rs` holds the file formats. `src/tree.rs` is the
storage layer: the tree, pieces, the log, recovery, upkeep, and the one
question commands ask of the index. Commands still reach data only
through it.

New dependencies: `libc` (mmap, flock, limits), `crc32fast` (checksums),
`lz4_flex` (the one codec, safe code only, used with `--compress`).

`README.md` describes every option. In short: `qdb --db DIR` keeps the
tree on disk; `--sync always|batch`, `--compress`, `--index write|later`,
`--piece`, `--segment`, `--checkpoint` are the switches; `--check`,
`--stats`, `--compact`, `--dump` are the tools.

## 2. Format

`docs/FORMAT.md`, complete enough to write a reader from. The shape:

- File content is cut into **pieces** of at most 64 KB at line ends.
  A file is a list of pieces. Changing a line rewrites one piece;
  appending adds a piece.
- **Data files** (`NNNNNNNN.seg`, 128 MB each) are append-only. They hold
  the pieces and, between them, one record per command line that says
  what the line did to the tree. Those records are the log; there is no
  separate log file, so content and the record that makes it visible
  reach the disk with one sync.
- **The tree** is saved to a checkpoint file now and then; opening reads
  the last one and applies the log written since.
- **Index files** (`NNNNNNNN.idx`), one per finished data file, hold the
  trigram lists for that file's pieces. The data file still being written
  has its index in memory, and it is built again from that file's content
  after a crash.
- Everything on disk has a checksum. Index files and the tree are derived
  or replaceable: a damaged index file is removed and built again; a
  damaged data file stops the open with a message naming the file and
  offset.

## 3. Conformance

All on the final build.

| Check | Result |
|---|---|
| Harness, in memory | 1,617 cases (1,911 steps): 1,617 passed |
| Harness, on disk | 1,617 passed; the database passes `--check` after every case |
| Harness, on disk, closed and reopened after every step | 1,617 passed; checked after every step |
| The same, sync batch, compressed | 1,617 passed |
| On disk, index built later | 1,617 passed |
| On disk, pieces of 16 bytes (every test file spans many pieces) | 1,617 passed |
| Reopened after every step, data files of 64 KB (every case spans several) | 1,617 passed |
| On disk, sync batch, compressed, index later, pieces of 4 KB | 1,617 passed |
| On disk, index off | 1,617 passed |
| Differential test on disk (10,000 generated steps against an engine in memory with no index), seed 1, sync always | identical; the kept index equals a fresh one at every check |
| The same, seed 2 (sync batch), seed 3 (reopened after every step), seed 4 (compressed, index later) | identical, each |
| Benchmark | Every command on every dataset agrees with GNU, and the trees agree after the write list: in memory, on disk, cold, and under the 512 MB limit |
| `cargo test` | 37 tests pass, 10 of them on storage with damaged and cut-off files |

The differential test mixes about 4,900 searches with 5,100 writes per
seed; about 7,100 searches select something. Run any row with
`qdb-harness --store disk|reopen` and the switches in `tests/README.md`.

## 4. Crash tests

**Power cuts** (`qdb-harness --crash N`, results in
`tests/results/crash/power-cuts.txt`). The engine runs a generated
workload on a simulated disk. At a chosen write or sync the power goes:
everything not synced is lost, in whole or in part, including directory
entries. The database is then opened, must equal the tree after a whole
number of command lines (all returned lines under always; all but the
stated bound under batch), and must pass `--check`. One in ten is then
written to, closed and reopened. Some runs go on from the crashed image,
so crashes follow crashes, and some cuts fall inside recovery itself.

| Seed | Settings | Power cuts | Recovered |
|---|---|---:|---|
| 1, 2, 3 | sync always | 10,000 each | all |
| 1, 2, 3 | sync batch | 10,000 each | all |
| 4, 5 | index off, always and batch | 10,000 each | all |
| 6 | batch, compressed | 10,000 | all |
| 7 | batch, compressed, index later | 10,000 | all |
| 8 | always, index later | 10,000 | all |

In these runs batch syncs every 8 command lines, so that the bound is
reached often. Of the 10,000 cuts of a run, about half fall while a command line runs, a
quarter while an index file is written, a sixth while the tree is saved,
and a fifth during recovery (the kinds overlap).

**Real kills** (`qdb-harness --kill N`, `tests/results/crash/kills.txt`).
A `qdb` process is fed command lines and killed with SIGKILL, 1,000 times
under each sync setting, two thirds of them without waiting for the last
answer. Every one recovered with every answered line present.

**Open after a crash** on the Linux tree, default settings, measured as
the whole `qdb --db D -c 'ls crash'` process after `kill -9`:

| Before the kill | Applied from the log | Index built again from | Open and answer |
|---|---:|---:|---:|
| 3,000 appends, sync always | 4,399 lines | 0.3 MB | 125 ms |
| 20,000 appends, sync always | 21,446 lines | 1.7 MB | 156 ms |
| 20,000 appends, sync batch | 22,000 lines | 2.3 MB | 180 ms |
| 60,000 appends, sync batch | 62,000 lines | 7.0 MB | 300 ms |

The same process after a clean close takes 93 ms, nearly all of it
process start and mapping files; the open itself is 28 ms. Recovery time
is bounded by two things: the log since the last save of the tree (a
save starts when that log passes 1 MB or twice the size of the saved
tree, whichever is more), and one data file's worth of index (at most 128
MB of text, about 1 s; in these runs 7 MB).

**Does the test catch faults?** `tests/results/crash/mutations.md` lists
13 faults put into the engine by hand (a missing sync here, a wrong order
there). Twelve are caught; the thirteenth turned out not to be a fault.
Two of the first attempts passed, which exposed three gaps in the test
itself, since closed.

**Faults the tests found in the engine**, each fixed (the same file has
the detail):

1. After a crash the count of log bytes since the last save of the tree
   started from zero, so a database that kept crashing never saved the
   tree and recovery grew longer each time.
2. At close, an index file could be saved while content it covers was not
   yet synced (batch).
3. A data file reopened after a crash at a size past its reserved space
   refused further writes.
4. An index file written in steps left out pieces that were freed while
   it was being written; if the power went before the log of those lines
   was synced, the pieces were in use again and a search would have
   missed lines. Found by the first crash run after upkeep in steps was
   written.
5. If the power went between finishing one data file and making the next,
   recovery did not check the end of the finished one the way it checks a
   last file.
6. Recovery applied log that had never been synced and then wrote an
   index file built from it.
7. A damaged index list found by a search stopped the database but left
   the file, so every later open met it again.

## 5. Benchmark

Full tables per dataset, one column per run, are in
`bench/results/redis.md`, `postgres.md` and `linux.md`: GNU, ripgrep,
stage 2 in memory, stage 3 in memory, on disk settled, on disk straight
after the write list, on disk cold, and under the 512 MB limit. A
selection from the Linux tree, in milliseconds:

| Command | GNU | Stage 2, in memory | On disk, settled | On disk, after the writes | On disk, cold |
|---|---:|---:|---:|---:|---:|
| `cat include/linux/kernfs.h` | 2.30 | 0.001 | 0.001 | 0.001 | 0.001 |
| `wc -l include/linux/*.h` | 10.7 | 28.6 | 0.885 | 0.934 | 0.872 |
| `ls -R` | 85.5 | 8.03 | 3.30 | 4.84 | 4.45 |
| `find . -name '*.h'` | 70.4 | 25.5 | 10.2 | 8.14 | 9.20 |
| `grep -rn qdbzzabsentterm .` (no file) | 1,225 | 0.012 | 0.020 | 0.020 | 0.100 |
| `grep -rn ausgabenspezifisches .` (1 file) | 1,147 | 0.098 | 0.125 | 0.230 | 0.295 |
| `grep -rn mispredicted .` (84 files) | 1,422 | 17.5 | 2.48 | 2.01 | 52.5 |
| `grep -rn separately .` (0.9% of files) | 1,001 | 24.1 | 14.8 | 15.1 | 258 |
| `grep -rn conditions .` (9%) | 1,288 | 130 | 62.6 | 64.6 | 571 |
| `grep -rn define .` (over half) | 2,552 | 461 | 519 | 597 | 1,235 |
| `grep -rE '[0-9]{4,}' .` (no index help) | 2,220 | 2,026 | 2,153 | 1,790 | 2,550 |
| `grep -rn define . \| head -n 5` | 4.71 | 0.019 | 0.024 | 0.026 | 0.067 |

Reading the table:

- **On disk is not slower than stage 2 in memory.** Content is read
  through a mapping of the data files, so with a warm cache a read is the
  same memory access as before. Most commands are faster than in stage 2
  because of work done on the way: line counts are kept per piece (`wc
  -l`), the tree's storage was reworked (`find` and `ls -R` are faster; I did
  not take that gain apart), and the index now
  has one entry per 64 KB piece and not per file.
- **The term in 84 files** dropped from 17.5 to 2.5 ms for that last
  reason: in stage 2 the candidates included whole 16 MB headers; now a
  candidate is one piece of one.
- **A term in no file** went from 0.012 to 0.020 ms: there is one index
  per data file (11 here) and each is asked. This was 0.06 ms until the
  final build, in which the lookup inside an index file was changed from a walk
  to a halving search.
- **Straight after the write list** nothing is noticeably slower: the
  write list is small against the dataset. The long write run (section 9)
  is the better test of an unsettled database; there the disk held 1.36x
  the text at the end and `--compact` brought it to 1.11x.
- **Cold** means the database closed, the system's file cache dropped,
  and the database opened again before each command, one run each. GNU's
  column there is cold too (5.5 s for a recursive grep against 1.2 s
  warm). A cold search with the index reads the index lists and then only
  the candidate pieces: 52 ms for the term in 84 files. A cold search
  that must read everything is bounded by the disk.

In-memory mode (no `--db`) still exists and holds everything in heap as
before (1,667 MB for Linux). It is 16% slower to load than stage 2 (9.4
against 8.1 s) because content is now cut into pieces with checksums in
both modes.

## 6. Targets

See the summary above and `bench/results/TARGETS-3.md`.

## 7. Disk, by what it is

Settled (after `--compact`), bytes:

| | redis | postgres | linux |
|---|---:|---:|---:|
| Raw text of all files | 17,378,691 | 161,175,531 | 1,396,914,445 |
| Text as stored | 17,378,691 | 161,175,531 | 1,396,914,445 |
| Index lists | 2,130,417 | 15,606,316 | 129,938,167 |
| Index tables | 863,384 | 2,653,396 | 10,518,316 |
| Saved tree | 67,810 | 378,763 | 3,738,248 |
| Log records | 76,087 | 421,450 | 4,230,272 |
| Record and file headers | 68,852 | 359,960 | 3,538,172 |
| Not yet reclaimed | 0 | 0 | 0 |
| **Directory** | **20,585,241** | **180,595,416** | **1,548,877,620** |
| Against raw text | 1.185 | 1.120 | 1.109 |

The index is nine tenths of the overhead. On the small dataset its tables
weigh more, which is why Redis is closest to the 1.2 limit.

During the 100,000-write run (sampled every 200 writes; the run more than
doubles PostgreSQL's text and adds 14% to Linux's):

| | postgres | linux |
|---|---:|---:|
| Worst moment: directory / live text | 508.1 MB / 289.3 MB = 1.756 | 2,238.7 MB / 1,560.9 MB = 1.434 |
| At the end: directory / live text | 612.9 MB / 430.4 MB = 1.424 | 2,166.2 MB / 1,587.6 MB = 1.364 |
| of that: text | 430,420,450 | 1,587,633,253 |
| index lists | 57,893,496 | 162,791,146 |
| index tables | 5,771,112 | 13,788,260 |
| saved tree | 1,648,460 | 4,832,721 |
| log records | 3,654,978 | 12,176,245 |
| record and file headers | 3,993,684 | 10,973,404 |
| not yet reclaimed | 109,524,860 | 374,038,884 |
| Settled afterwards | 487.3 MB = 1.132 | 1,765.8 MB = 1.112 |
| The same run with upkeep off, worst moment | 2.625 | not run |

The breakdown by component is taken at the end of the run, not at the
worst moment; at the worst moment only the two totals were sampled. The
difference between the two moments is space not yet reclaimed.

## 8. Memory

The engine's own memory is its heap and stacks. Mapped data and index
files are the system's file cache: the system can take those pages back
at any time, and they are not counted against the limit. The limit in the
tests is set with `RLIMIT_DATA`, which counts heap and stacks and not
file mappings; a run that goes over it dies.

| | Heap |
|---|---:|
| After opening the Linux tree | 21.7 MB (the tree: 83,405 files, 93,472 pieces) |
| After opening PostgreSQL, Redis | 2.2 MB, 0.4 MB |
| Most during any benchmark command on Linux | 21.7 MB above that (`cp -r Documentation`) |
| Most during the 100,000 writes on Linux | 74.8 MB |
| Most during the 1 GB file run (store it, search it, change it, copy it twice) | 47.4 MB; 99 MB reserved from the system |
| Most in the comparison runs on Linux | 60 to 67 MB plain, 94 to 104 MB compressed |

Under a 512 MB limit: the whole benchmark on Linux, the 100,000 writes on
Linux and the 1 GB file run all complete with the same results as
without it.

What grows with the data: the tree (about 120 bytes per file, folder and piece)
and, while a data file is being written, the index of that one file (at
most 128 MB of text, and an index about a tenth of that). What does not: file content,
the index of finished data files, the output of a command, redirected
output past 8 MB (it waits in a scratch file in the database directory).

The 1 GB file, pieces of 64 KB, sync batch, 512 MB limit
(`bench/results/bigfile/`):

| Command | GNU ms | Engine ms | Peak heap | Disk written |
|---|---:|---:|---:|---:|
| `grep -n qdbrareterm big.txt` | 833 | 0.33 | 15 KB | 0 |
| `wc -l big.txt` | 156 | 0.08 | 1 KB | 0 |
| `echo '...' >> big.txt` (three times) | 1.5 | 0.14, 0.035, 0.028 | 0.5 MB, 2 KB, 1 KB | 118, 135, 105 bytes |
| `sed -i 's/QDBUNIQUELINE/.../' big.txt` | 2,475 | 2.38 | 3.1 MB | 65,535 bytes |
| `wc big.txt` | 15,129 | 1,977 | 3 KB | 0 |
| `cp big.txt copy.txt` | 681 | 7,600 | 25 MB | 1,114 MB |
| `cat big.txt > cat.txt` | 1,539 | 10,739 | 40 MB | 1,114 MB |

With pieces of 16 KB the `sed -i` writes 16,371 bytes in 1.7 ms; with 256
KB, 262,229 bytes in 7.6 ms. Under sync always the appends take 0.25 to
0.6 ms and the `sed -i` 2.7 ms, each with one sync.

`cp` and `cat >` of 1 GB are the slow rows: 7 to 11 s against 0.7 to 1.5
s for GNU. Every byte is checksummed, indexed and written once (64 syncs), and `cp` does not share pieces with its source. See
the questions.

## 9. Write costs

### What a sync costs here

5,000 times each, on the same disk (`bench/results/writes/sync-cost-s3.md`):

| | Median ms | 90% | 99% | Bytes written per line | Syncs |
|---|---:|---:|---:|---:|---:|
| A plain file: append a 77-byte line, sync | 0.155 | 0.210 | 0.498 | 77 | 5,000 |
| A plain file: append as many bytes as the engine writes for that line, sync | 0.152 | 0.220 | 0.424 | 617 | 5,000 |
| The engine, `echo '...' >> f.txt`, sync always | 0.153 | 0.216 | 0.388 | 617 | 5,000 |
| The engine, the same, sync batch | 0.007 | 0.017 | 0.453 | 617 | 78 |

Under always a small write costs one sync and nothing measurable on top
(1.01x). The engine writes 617 bytes for a 77-byte line: the line, the
rewritten end of the file's last piece, and the record of the command
line. A sync on this virtual disk is cheap in the median (0.15 ms); a
real disk that honours a flush would take longer, and the engine's share
would be smaller still.

One more thing the comparison runs show: after a pause in writing a sync
costs more. When only 5% of the commands write, the median write under
always is 0.6 to 0.7 ms, against 0.16 ms when 95% write.

### 100,000 writes

`qdb-stress writes`: 90,912 small writes (append, create, overwrite,
`sed -i`, `rm`, `mv` on one small file) and 9,088 large ones (`cp -r`,
`sed -i` over a folder, `cat` of many files into one, `sed -i` on a large
file, `rm -r`), seed 1, on top of the loaded dataset. Times in
milliseconds (`bench/results/writes/`).

Small writes:

| Dataset | Where | Median | 90% | 99% | 99.9% | Slowest |
|---|---|---:|---:|---:|---:|---:|
| postgres | in memory | 0.010 | 0.037 | 0.112 | 0.527 | 27.3 |
| postgres | disk, batch | 0.013 | 0.046 | 1.14 | 4.58 | 45.7 |
| postgres | disk, always | 0.170 | 0.305 | 1.19 | 2.45 | 21.9 |
| linux | in memory | 0.009 | 0.037 | 0.090 | 0.183 | 14.8 |
| linux | disk, batch | 0.012 | 0.051 | 0.971 | 3.04 | 90.0 |
| linux | disk, batch, 512 MB limit | 0.011 | 0.047 | 0.879 | 2.72 | 11.3 |
| linux | disk, always | 0.163 | 0.273 | 0.576 | 2.35 | 147 |

Large writes:

| Dataset | Where | Median | 90% | 99% | 99.9% | Slowest |
|---|---|---:|---:|---:|---:|---:|
| postgres | in memory | 0.080 | 3.81 | 29.9 | 123 | 375 |
| postgres | disk, batch | 0.097 | 3.95 | 31.0 | 141 | 427 |
| postgres | disk, always | 0.409 | 4.70 | 31.8 | 115 | 422 |
| linux | in memory | 0.058 | 2.47 | 13.7 | 64.3 | 237 |
| linux | disk, batch | 0.080 | 3.25 | 16.1 | 76.8 | 313 |
| linux | disk, always | 0.363 | 3.85 | 19.9 | 68.0 | 232 |

Syncs per command line:

| Dataset | Sync | 0 | 1 | 2 | 3 | 4 or more | Syncs in all |
|---|---|---:|---:|---:|---:|---:|---:|
| postgres | always | 9,823 | 90,085 | 63 | 18 | 11 | 90,361 |
| linux | always | 9,824 | 90,140 | 29 | 4 | 3 | 90,245 |
| postgres | batch | 98,492 | 1,473 | 13 | 14 | 8 | 1,621 |
| linux | batch | 98,549 | 1,440 | 6 | 2 | 3 | 1,490 |

Lines with no sync under always are the ones that changed nothing. Lines
with more than one finish an index file, save the tree or roll to a new
data file in the same line.

Bytes written to disk over the run, and write amplification (everything
written, over the content and log the commands themselves caused):

| | postgres | postgres, upkeep off | linux |
|---|---:|---:|---:|
| Content the commands wrote | 842,424,488 | 839,680,688 | 694,016,759 |
| Log of the command lines | 10,879,441 | 9,134,535 | 9,141,898 |
| Content moved to free space | 509,727,305 | 0 | 11,194,726 |
| Index files | 153,009,954 | 72,550,242 | 50,056,365 |
| Saves of the tree | 11,976,314 (11) | 5,664,594 (5) | 9,619,463 (2) |
| All | 1,528,017,502 | 927,030,059 | 774,029,211 |
| **Write amplification** | **1.79** | **1.09** | **1.10** |
| Worst disk / live text | 1.756 | 2.625 | 1.434 |

The PostgreSQL run rewrites and deletes far more than the dataset holds,
so space has to be reclaimed during it: 0.6 bytes moved per byte written
keeps the disk under 1.76x. With upkeep off nothing is moved and the disk
goes to 2.6x. The Linux run mostly adds, and there is little to reclaim.

### The slowest small writes, and the target that is not met

Each write run lists its ten slowest small writes with what the engine
wrote to disk during each. On the final build, batch:

- Linux, first run: 90.0, 24.4, 18.8, 16.4, 16.0 ms. The 90 ms write is
  an `echo >>` during which the engine wrote 235 bytes and made one sync.
- Linux, second run (the same writes, under the 512 MB limit): 11.3,
  11.3, 10.3, 8.1, 7.4 ms.
- PostgreSQL: 45.7, 40.1, 24.5, 21.7, 20.6 ms. The first two wrote about
  110 bytes and made one sync each.

The writes in which the engine does its heaviest work are in the list
too, and they are not the slowest: finishing an index file (13 MB written
in steps before, one last sync in this line) takes 12 to 25 ms. So the
engine's own pauses are under the 50 ms target, and the miss is a sync
that the disk takes 40 to 150 ms over, a few times in 1,500 syncs. In
memory, where a sync costs nothing, the slowest small write is 14.8 ms on
Linux and 27.3 ms on PostgreSQL (both finish an index).

Before upkeep was done in steps the slowest small write was 150 to 240
ms from the engine's own work: an index file written in one go, a 128 MB
data file removed in one call (60 to 240 ms on ext4), cleaning debt
carried into a small write. Those are gone: index files and saves of the
tree are written 32 KB and 8 KB per command line plus a share of what
the line wrote and synced every 1 MB; files are removed 1 MB per line;
moving content is paid for by the lines that write.

What would bring the tail down further, none of it done:

1. Under batch, ask the system to start writing out a batch without
   waiting for it, and wait only at the next batch. The bound on what a
   power cut can lose would double (two batches).
2. Sync on a second thread. The brief keeps this stage to one thread.
3. Nothing, on a disk whose sync time is steadier than this one's.

"No larger on Linux than on PostgreSQL" holds for the engine's own work
(the write run does the same per-line upkeep on both, and the in-memory
figures are 14.8 against 27.3 ms) and cannot be shown either way for the
sync stalls, which vary more between two runs than between the datasets.

## 10. The comparison

Eight settings (sync always or batch; index at the write or later in
bulk; text plain or compressed), three workloads, two datasets. Each row
is a fresh import with the row's switches, then 20,000 command lines
with seed 1. `bench/results/compare/postgres-s3.md` and `linux-s3.md`
hold all 24 rows per dataset with every column asked for.

**The workloads.** A command line is a read with probability 95%, 50% or
5%. Reads: `cat FILE` 25%, `head -n 20 FILE` 15%, `wc -l FILE` 10%, `grep
-n WORD FILE` 15%, `grep -rl WORD DIR` 20%, `grep -rn WORD .` 10%, `find
DIR -name PATTERN` 5%. Writes: append a line 40%, create a small file
25%, overwrite 10%, `sed -i` 10%, `rm` 5%, `cp` 5%, `mv` 4%, append to a
file of the dataset 1%. Files and folders are picked from the dataset
and from what the run has made so far; search words are picked from the
dataset's text, so most are common words and a recursive search over
everything is the expensive read.

Commands per second:

| Sync | Index | Text | postgres 95% reads | 50% | 5% | linux 95% reads | 50% | 5% |
|---|---|---|---:|---:|---:|---:|---:|---:|
| always | at write | plain | 1,313 | 2,126 | 4,148 | 297 | 533 | 2,761 |
| always | at write | compressed | 438 | 799 | 3,139 | 90 | 165 | 1,347 |
| always | later | plain | 1,147 | 1,410 | 3,541 | 309 | 510 | 2,692 |
| always | later | compressed | 324 | 393 | 1,969 | 95 | 176 | 1,440 |
| batch | at write | plain | 1,413 | 2,653 | 13,632 | 300 | 537 | 5,049 |
| batch | at write | compressed | 463 | 908 | 6,058 | 91 | 171 | 1,855 |
| batch | later | plain | 1,226 | 1,851 | 8,684 | 309 | 580 | 5,652 |
| batch | later | compressed | 327 | 427 | 2,923 | 98 | 185 | 2,050 |

Medians and 99% for reads and writes, plain text only (the compressed
rows are in the files), in milliseconds:

| Dataset | Workload | Sync | Index | Read median | Read 99% | Write median | Write 99% | Disk / raw text | Peak heap MB |
|---|---|---|---|---:|---:|---:|---:|---:|---:|
| postgres | 95% reads | always | at write | 0.017 | 18.8 | 0.592 | 2.44 | 1.122 | 39.3 |
| postgres | 95% reads | batch | at write | 0.016 | 18.9 | 0.040 | 1.24 | 1.122 | 39.3 |
| postgres | 95% reads | batch | later | 0.016 | 22.7 | 0.026 | 1.07 | 1.122 | 32.6 |
| postgres | 50% | always | at write | 0.017 | 18.4 | 0.191 | 0.971 | 1.135 | 39.3 |
| postgres | 50% | batch | at write | 0.016 | 19.1 | 0.015 | 1.04 | 1.135 | 39.3 |
| postgres | 50% | batch | later | 0.014 | 27.7 | 0.007 | 0.903 | 1.132 | 32.6 |
| postgres | 5% reads | always | at write | 0.025 | 21.8 | 0.169 | 0.846 | 1.153 | 39.3 |
| postgres | 5% reads | batch | at write | 0.018 | 20.4 | 0.009 | 0.652 | 1.153 | 39.3 |
| postgres | 5% reads | batch | later | 0.016 | 29.5 | 0.005 | 0.660 | 1.148 | 32.6 |
| linux | 95% reads | always | at write | 0.027 | 109 | 0.669 | 2.77 | 1.109 | 67.1 |
| linux | 95% reads | batch | at write | 0.026 | 108 | 0.051 | 1.34 | 1.109 | 67.1 |
| linux | 95% reads | batch | later | 0.024 | 96.5 | 0.036 | 1.07 | 1.109 | 59.8 |
| linux | 50% | always | at write | 0.024 | 96.1 | 0.219 | 1.22 | 1.111 | 67.1 |
| linux | 50% | batch | at write | 0.023 | 108 | 0.020 | 1.04 | 1.111 | 67.1 |
| linux | 50% | batch | later | 0.021 | 88.5 | 0.008 | 1.17 | 1.110 | 59.8 |
| linux | 5% reads | always | at write | 0.033 | 94.2 | 0.162 | 0.855 | 1.112 | 67.1 |
| linux | 5% reads | batch | at write | 0.022 | 85.0 | 0.010 | 0.866 | 1.112 | 67.1 |
| linux | 5% reads | batch | later | 0.018 | 75.7 | 0.005 | 0.529 | 1.111 | 60.0 |

Compressed text leaves the disk at 0.49 to 0.53 of the raw text on
PostgreSQL and 0.43 on Linux (index included), against 1.11 to 1.15
plain.

**Which did best, per workload:**

- **PostgreSQL, 95% reads:** batch, index at write, plain (1,413/s). It
  gives up the last 64 command lines in a power cut and 2.3 times the
  disk of the compressed rows. Sync always costs only 7% here, because
  few commands write.
- **PostgreSQL, even:** batch, index at write, plain (2,653/s). Against
  always it is 25% faster and gives up the same 64 lines.
- **PostgreSQL, 95% writes:** batch, index at write, plain (13,632/s),
  3.3 times always. It gives up the 64 lines, nothing else.
- **Linux, 95% reads:** all four plain rows are within 4% (297 to 309/s);
  the workload is bound by recursive searches. Always with the index
  later is nominally first and gives up nothing that shows here.
- **Linux, even:** batch, index later, plain (580/s), 8% ahead of batch
  with the index at write. It gives up the 64 lines, and searches read up
  to 16 MB of text that is not indexed yet.
- **Linux, 95% writes:** batch, index later, plain (5,652/s), 12% ahead
  of index at write and twice always, giving up the same two things.

**What each switch does, across the tables:**

- **Sync.** Batch matters in proportion to the share of writes: nothing
  at 95% reads, two to three times at 95% writes. The write median goes
  from about 0.17 ms (one sync) to about 0.01 ms.
- **Index later.** It halves the median small write under batch (0.010
  to 0.005 ms) and lowers peak memory a little. On Linux it comes out
  between 4% behind and 12% ahead overall. On PostgreSQL it comes out
  13 to 36% behind with plain text (more with compressed),
  because the read 99% rises (19 to 28 ms): my reading is that with a
  smaller dataset a recursive search is cheap, and reading up to 16 MB
  of waiting text unindexed is a large part of it, while on Linux it is
  lost in a search that takes 100 ms anyway. I did not measure that
  directly.
- **Compression.** It cuts the disk by 2.3x (PostgreSQL) to 2.6x (Linux)
  and costs a factor of three in commands per second wherever searches
  dominate: the read 99% goes from 19 to 60 ms on PostgreSQL and from
  108 to 340 ms on Linux, because a search has to unpack every piece it
  reads. Small reads and small writes are barely touched (read median
  0.017 to 0.023 ms).

The codec by itself, a piece at a time, one thread
(`qdb-stress codec`):

| Dataset | Pieces | Text MB | Stored MB | Stored / text | Pieces kept plain | Compress MB/s | Unpack MB/s |
|---|---:|---:|---:|---:|---:|---:|---:|
| postgres | 9,604 | 161.2 | 60.6 | 0.376 | 304 | 419 | 1,965 |
| linux | 93,472 | 1,396.9 | 449.1 | 0.321 | 1,418 | 518 | 2,020 |

A piece is kept plain when packing would not make it smaller. Unpacking
at 2 GB/s is slower than the search itself, which is why searches pay.

**Recommended defaults** (recommendations, not decisions; all three
switches can be changed per open and mixed in one database, except that
the sync setting is per process):

1. **`--sync always`.** A returned command line is on disk. It costs one
   sync per line that changes something, and nothing beyond it. Use
   batch for bulk work and for callers who can replay their last 64
   lines; that is worth 2 to 3 times on write-heavy work.
2. **Index at the write.** The gain from indexing later is at most 12%
   and only on the larger dataset; the loss on the smaller one is
   larger, and "every search is fully indexed" is the simpler promise.
3. **Plain text.** Compression is the right choice only where disk is
   the constraint and searches over much of the tree are rare; then it
   saves 55 to 60% of the disk.

## 11. What would change for several readers and writers

Today one process holds a lock on the database directory and one thread
does everything. What the format already gives, and what would have to
be built:

- **Readers alongside one writer.** Data files and index files are never
  changed in place once written: content is only appended, and files are
  replaced by rename. A reader that holds a version of the tree can keep
  reading its pieces while the writer goes on. What is missing: the tree
  is one mutable structure in memory. It would need versions (copy the
  path from the changed node to the root, which the piece lists already
  do for file content), a way to publish a new root at the end of each
  command line, and a count of readers per version so that upkeep does
  not truncate or remove a data file an old reader still needs. Today
  "dying" files are cut down 1 MB per command line as soon as nothing in
  the current tree points into them; with readers they would wait for
  the oldest reader.
- **Readers in other processes.** They would need the tree from disk,
  not from the writer's memory: either a saved tree per version (too
  slow as it is: 3.7 MB for Linux) or a tree file that is itself
  append-only and mapped, with the root's position published in a small
  shared file. The checkpoint format would change; data and index files
  would not.
- **Several writers.** Command lines are already the unit of atomicity
  and the log is a sequence of them, so the simple step is one writer at
  a time with a queue, and one sync for all lines that queued up while
  the last sync ran (group commit). That would make always nearly as
  fast as batch under load without losing its promise, and it is where I
  would start. Writers running truly in parallel would need a rule for
  two lines that touch the same file or folder, which the data model
  does not have.
- **Upkeep on its own thread.** Index files, saves of the tree and
  moving content are already cut into steps that leave the database
  valid after each; they run inside command lines only because there is
  one thread. Moving them to a thread removes them from write latency,
  and is also the fix for the sync stalls of section 9.
- **The in-memory index of the data file being written** is the one
  index structure that changes in place; readers would need it guarded
  or versioned like the tree.

## 12. Deviations, gaps and questions

### Differences from GNU

None new. All 1,617 cases match byte for byte in every mode and switch
setting listed in section 3.

### Deviations from the brief

1. **The slowest-small-write target is not met reliably** (section 9).
2. **Close and `--compact` are done in one go, not in steps.** Close
   saves the tree and the index of the data file being written: 91 ms on
   the settled Linux tree. `--compact` after the 100,000 writes takes 4.5
   s on PostgreSQL and 15 to 20 s on Linux. Neither runs inside a command
   line.
3. **Under batch the bound is 64 command lines or 4 MB written**,
   whichever comes first.
4. **Disk by component at the peak of the write run** is given as totals
   at the peak and components at the end (section 7).
5. **Power-cut seeds.** The brief asks for 3 seeds; the 3 are there
   under both sync settings, plus 5 more runs with other switches. The
   kill test has one seed per sync setting.

### Where the brief was silent, and what was chosen

- **What is one sync.** One `fdatasync` (Rust's `sync_data`) of the data
  file being written.
  A command line that also finishes an index file, saves the tree or
  starts a new data file makes more, and those are the lines with 2 or
  more in the tables.
- **What recovery says.** A database that was not closed is recovered on
  open without being asked, and says on standard error what it did
  (lines applied, a tail cut off, an index built again). Damage to
  content, to the tree with no way to rebuild it, or to the MANIFEST
  stops the open with exit status 2.
- **A second process.** Refused with a clear error while another holds
  the directory.
- **Switches and existing data.** `--compress`, `--piece` and `--index`
  apply to what is written from then on. A database can hold plain and
  compressed pieces and pieces of different sizes together.
- **Index later.** Content waits unindexed until 16 MB wait, a data file
  is finished, or the database is closed or compacted; searches read
  what waits, so results never differ.
- **Space.** Cleaning starts when dead bytes pass 30% of live bytes plus
  4 MB, and works faster past 60%.

### Gaps

- **A single line longer than memory** is held whole. Pieces are cut at
  line ends; a line over the piece size is one large piece.
- **Pipelines**: a stage that needs all of its input before it can
  answer holds that input in memory, as before. Only
  output redirected to a file goes to the scratch file.
- **In-memory mode** gathers redirected output in memory.
- **`cp` copies bytes.** It could share pieces with its source and write
  only a record; it does not.
- **Under batch, a power cut during `--compact`** can lose the unsynced
  lines before it, like any other power cut; `--compact` does not sync
  first.
- **The disk.** Everything was measured on one virtual disk whose sync
  takes 0.15 ms in the median and sometimes 100 ms. I have no figures
  from a disk with a real flush.
- **The cold run was run once**, and the read-only comparison with stage 2 moves by tens of percent on
  sub-millisecond commands between runs (`bench/results/TARGETS-3.md`).
- **Not measured:** a database much larger than memory. The 1 GB file
  and the Linux tree both fit in this machine's file cache.

### Questions

1. **Is the slowest-write target about the engine or about the disk?**
   If it is about what the caller sees on this kind of disk, the next
   step is one of the two changes in section 9 (start a batch's
   write-out early, or sync on a second thread), and I would like to
   know which rule may bend: the batch bound or the one thread.
2. **Should `cp` share pieces?** It would make `cp` and `cp -r` cost by
   the number of files and not their bytes (the 1 GB copy would go from
   7.6 s to well under a millisecond) and halve the disk written in the
   large-write rows. It needs a count of users per piece, which changes
   the saved tree's format, so it is cheaper to decide before more is
   built on the format.
3. **Is 64 lines the right batch?** Smaller loses less and costs more;
   the 100,000 writes on Linux take 10 s in memory, 16 s under batch at
   64 and 33 s under always.
4. **Should always group syncs when several writers arrive (section
   11)?** If several writers are next, that design decides more than
   anything else here.
