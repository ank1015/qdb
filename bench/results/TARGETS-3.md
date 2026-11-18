# Stage 3 targets, measured

The Linux tree (83,405 files, 1,396.9 MB of text), one thread, the
system's file cache warm, text stored plain, new content indexed at the
write, unless a row says otherwise. "Settled" is after `--compact`. The
machine has 4 cores and 15 GB; the disk is a virtual disk with ext4.
Where a figure comes from is in the last column; paths are under
`bench/results/` unless they start with `tests/`.

| Part | Target | Measured | Result | From |
|---|---|---|---|---|
| 2 | Harness passes in memory, on disk, and reopened after every step | 1,617 of 1,617 in each of the three, and in six more settings | met | `docs/HANDBACK-3.md` section 3 |
| 2 | Open after a clean close under 200 ms, following files and not bytes | 28 ms on Linux, 3.5 ms on PostgreSQL, 0.7 ms on Redis: 0.3 to 0.45 microseconds per file at every size | met | `*/s3-disk.meta`, `open_ms` |
| 2 | Import with `--load` into `--db` at most 16.4 s | 13.8 s | met | `linux/s3-disk.meta`, `load_ms` |
| 2 | Every read-only command, settled, within 1.5x of stage 2 in memory, or within 0.05 ms under 0.1 ms | 62 of 62 on Linux; the median command takes 0.77 of its stage 2 time | met | `linux/part4.csv` against `linux/s3-disk.csv` |
| 2 | Disk settled at most 1.2x the raw text | 1.109 on Linux, 1.120 on PostgreSQL, 1.185 on Redis | met | `*/s3-disk.meta`, `disk_settled_total` |
| 3 | 10,000 power cuts on each of 3 seeds, all recovering to a prefix, check clean | 110,000 power cuts in 11 runs of 10,000 (8 seeds; both sync settings, index on and off, compressed, index later): all recovered | met | `tests/results/crash/power-cuts.txt` |
| 3 | 1,000 `kill -9`, all recovering | 2,000 (1,000 under each sync setting): all recovered, no answered line lost | met | `tests/results/crash/kills.txt` |
| 3 | Syncs per command line under always: one in the usual case | Of 100,000 command lines: 9,824 none (they changed nothing), 90,140 one, 29 two, 4 three, 3 four or more | met | `writes/linux-s3-disk-always.md` |
| 3 | A small write under always within 1.2x of a bare append and sync of the same bytes | 0.153 ms against 0.152 ms: 1.01x | met | `writes/sync-cost-s3.md` |
| 3 | A small write under batch: median under 0.1 ms | 0.012 ms (0.009 ms in memory) | met | `writes/linux-s3-disk-batch.md` |
| 3 | Open after a crash, default settings, under 2 s | 0.13 to 0.30 s after `kill -9` with 4,400 to 62,000 command lines to apply from the log | met | `docs/HANDBACK-3.md` section 4 |
| 4 | The engine's own memory after open under 200 MB | 21.7 MB of heap | met | `linux/s3-disk.meta`, `heap_after_load` |
| 4 | The whole benchmark under a 512 MB limit completes with identical results | Completed; every command and the final tree agree with GNU; times in `linux.md` | met | `linux/s3-disk-cap512.meta` |
| 4 | Append one line to a 1 GB file, batch: under 1 ms, disk written follows the piece size | 0.03 to 0.14 ms; 105 to 135 bytes written at every piece size (an append adds a small piece, it does not rewrite one) | met | `bigfile/s3-disk-batch-cap512-piece*.csv` |
| 4 | `sed -i` changing one line of a 1 GB file writes a few pieces, under the limit | One piece written: 16,371, 65,535 or 262,229 bytes at piece sizes of 16, 64 and 256 KB; 1.7, 2.4 and 7.6 ms; 3 to 4 MB of heap | met | same |
| 4 | Search for the term in 84 files under 5 ms | 2.5 ms settled, 2.0 ms after the writes (17.5 ms in stage 2) | met | `linux/s3-disk.csv` |
| 5 | Slowest small write in the 100,000-write run, batch: under 50 ms, and no larger on Linux than on PostgreSQL | Linux 90 ms in one run and 11 ms in another of the same writes; PostgreSQL 46 ms. Each of the slow ones is a single sync of under 1 KB that the disk took that long over | NOT MET reliably | `writes/*-s3-disk-batch*.md`, and section 9 of the hand-back |
| 5 | Disk at any moment of that run at most 2x the live raw text | Worst 1.43x on Linux, 1.76x on PostgreSQL (2.63x with upkeep switched off) | met | same |
| 6 | Both comparison tables | Delivered: 24 rows per dataset | met | `compare/postgres-s3.md`, `compare/linux-s3.md` |

## The read-only target on the other datasets

The target names the Linux tree. For completeness, the same rule on the
other two:

| Dataset | Read-only commands | Median of on disk / stage 2 | Outside the rule |
|---|---:|---:|---|
| redis | 62 | 1.01 | 1: `find . -name '*.c'` 0.162 to 0.251 ms |
| postgres | 62 | 0.85 | 2: `cat doc/src/sgml/html/*.html \| head -n 2` 0.318 to 0.500 ms; `grep -rv e src/include/access` 0.482 to 0.757 ms |
| linux | 62 | 0.77 | none |

These are sub-millisecond commands on a machine whose timings move by
this much between runs, and I do not have evidence of a cause in the
engine: the benchmark was run twice on the final storage code, and the
two PostgreSQL commands were inside the rule the first time (the `cat`
took 0.303 ms then), while the first Redis run had four `find` and `grep`
commands between 1.51x and 1.62x. `find` never touches file content, so
its time does not depend on where content is kept. A third run would be
needed to say more.
