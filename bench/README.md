# Benchmark

`qdb-bench` times a fixed list of commands on the real GNU tools and on the
engine, checks that both give the same result, and keeps every run so that
runs can be laid side by side.

```
cargo build --release
bench/fetch.sh                                         # download the real datasets
./target/release/qdb-bench --dataset redis --label mine
./target/release/qdb-bench --dataset redis --label mine-noindex --no-index
./target/release/qdb-bench --dataset all               # the generated datasets
./target/release/qdb-bench --dir path/to/tree          # your own data
python3 bench/targets.py                               # the stage 2 targets, from the stored runs
```

## Datasets

Three real source trees, listed in `datasets.txt` and fetched by
`fetch.sh`. They are upstream release tarballs as the Ubuntu archive
carries them, with checksums, because this machine could not reach GitHub
or kernel.org. Sizes are what the loader keeps: symbolic links and files
that are not valid text are skipped and counted.

| Name | What | Files | Text |
|---|---|---:|---:|
| redis | Redis 8.0.6 | 1,672 | 17.4 MB |
| postgres | PostgreSQL 16.15 | 8,419 | 161.0 MB |
| linux | Linux 6.8 | 83,405 | 1,396.9 MB |

The generated datasets from stage 1 (`small`, `medium`, `large`) are still
there: seeded, so every run sees the same bytes. `large` (12,005 files,
87.4 MB) is measured alongside the real ones. `--export DIR` writes a
generated dataset to disk without timing anything.

## Command list and search terms

`commands.txt` holds the commands. Read-only commands are timed as the
median of five runs (three above 0.3 s, one above 2 s); the engine, whose runs cost little, gets 16 runs for a command under 50 ms and 51 for one under 2 ms. Lines starting with
`mut:` change data; they run once, in order, and the two trees are
compared at the end.

Commands contain placeholders such as `{RARE}` and `{DIR}`. Their values
differ per dataset and are kept in `terms/DATASET.txt`. Search terms are
chosen by measured file frequency: a term in no file, in exactly one file,
in about 0.1%, 1% and 10% of the files, and in more than half. Delete the
file to have them chosen again.

## What is measured

- **GNU**: `bash -c COMMAND` in a real copy of the dataset on disk, with
  the file cache warm, output captured through a pipe. The time includes
  starting bash and the tool, which is reported per run (about 2 ms).
- **rg**: for recursive searches, `rg -j 1 --no-ignore --hidden` with the
  equivalent options. A timing reference only; its output is not compared.
- **Engine**: in process, one thread, data already in memory, including
  parsing the command. Output goes to a sink that counts and discards it.
- **Peak memory per command**: the most heap the engine held during the
  command, above what it held before it, from a counting allocator.
- **After load**: heap in use and resident memory of the process, taken in
  a fresh process that has done nothing but load.
- **New 64 KB file**: the time to add one file with the same generated
  64 KB of text, on every dataset. It shows what a write costs as the data
  grows.
- **The index**: build time, size, number of trigrams and postings, the
  spread of list lengths, and what the index did for each command that
  changes data.

The real tools are started by a small helper process, not by the
benchmark itself. A process holding a gigabyte of data is slow to start a
child from (about 50 ms here on the Linux tree), and that time would be
charged to the real tools; it also leaves the big process's memory marked
to be copied on the next write, which would be charged to the engine's
writes. The runs labelled Stage 1 and Part 2 were taken before this was
found. Their engine times for read-only commands are not affected. The GNU
and rg columns in the tables always come from the latest run.

## Results

`results/DATASET.md` is the table for one dataset, built from every stored
run of it. `results/DATASET/LABEL.csv` and `.meta` are the runs
themselves:

| Label | Engine |
|---|---|
| `before` | Stage 1 |
| `part2` | After part 2: pipelines that stop early |
| `part3` | After part 3: fast scan, no index |
| `part4-noindex` | After part 4, run with `--no-index` |
| `part4` | After part 4, index on |
| `paired` | After part 4, `--paired`: each read-only command on the engine alone, with searches using the index and ignoring it by turns in one process |

| `s5-mem`, `s5-disk`, `s5-disk-after`, `s5-disk-cap512` | Stage 5: as the stage 4 runs of the same names, with the commands for loops, awk and diff, and the counters of each command beside its time |
| `s5-disk-counting` | Stage 5, `--paired-counting`: each read-only command on the engine alone, with the counting of costs on and off by turns in one process |
| `s3-mem` | Stage 3, in memory |
| `s3-disk` | Stage 3, `--db`: imported, compacted, closed and opened again (settled); warm file cache |
| `s3-disk-after` | The same database straight after the command list's writes: the read-only commands again |
| `s3-disk-batch` | As `s3-disk` with `--sync batch` |
| `s3-disk-cold` | `--cold --reps 1`: before every read-only command the database is closed, the operating system is told to forget its file cache, and the database is opened again; the real tools start from an empty cache too |
| `s3-disk-cap512` | `--cap 512`: the whole run with the process's own memory limited to 512 MB (`RLIMIT_DATA`); outputs are kept in files and compared through mappings so that the benchmark's own bookkeeping is not what hits the limit |

With `--db DIR` the dataset is imported into a database (sync in batches
during the import), compacted, closed and opened again before the
commands run; `.meta` then also holds the times of those steps and what
is on disk by component, settled (`disk_settled_*`) and after the writes
(`disk_after_*`).

## qdb-stress

Measurements the command list does not make. Results are under
`results/bigfile/`, `results/writes/` and `results/compare/`.

- `qdb-stress bigfile --size 1024 --db DIR --cap 512 [--piece N] [--sync MODE]`:
  one generated 1 GB file, built without ever being held whole; commands
  on it on the real tools and on the engine, with time, peak heap, bytes
  written to disk and syncs for each.
- `qdb-stress writes --dataset NAME --count 100000 --db DIR --sync MODE`:
  a long run of small writes mixed with occasional large ones. The time
  of every write, the ten slowest small writes with what was written to
  disk during each, bytes written by kind and the write amplification,
  syncs per command line, and the size of the directory against the raw
  text every 200 writes.
- `qdb-stress sync-cost --db DIR`: what a sync costs here, and a small
  write under each sync setting against it.
- `qdb-stress compare --dataset NAME --count 20000 --db DIR`: the eight
  settings of the three switches on three mixes of reads and writes.
- `qdb-stress codec --dataset NAME`: what the compression codec does to
  the dataset a piece at a time: stored size, and speed in both directions.

`results/TARGETS-4.md` checks each stage 4 target. Its first half is
`targets4-head.md`, written by hand; the second is written by
`python3 bench/targets4.py` from the runs labelled `s4-*`.

Stage 4 added to the command list: a prefix `nocmp:` for a command that
is timed on both sides but whose output is known to differ (`du -s`,
whose answer on the real side includes the size of real folders); the
mark `⏎`, which stands for a newline in a command, for here-documents
and lists over several lines; and the rule that the output of `ls -l`
is compared after the owner, the group, a folder's size and the `total`
lines are masked on both sides. The run labelled `s4-disk-small` was
made in a control group limited to 1 GB of memory, file cache included.

`results/TARGETS-3.md` checks each stage 3 target against these runs.

`results/TARGETS.md` checks each stage 2 target against these runs, and
`docs/HANDBACK-2.md` discusses them. `results/stage1/` holds the stage 1
results in their original form.

`--report` rebuilds the tables from the stored runs without measuring.

## What units cost: `results/replay/`

`qdb-replay --db DIR UNITS_FILE` runs a file of units against a database
and prints two tables of what they cost: by first command and by what
the unit did. `results/replay/` holds four of them for the Linux tree
under sync always: three workloads of 20,000 units made by
`qdb-stress units --reads N` (95, 50 and 5 in a hundred only read) and
the benchmark command list. Every column but the times is counted and
repeats exactly. `docs/HANDBACK-5.md` reads them.

## The service: `qdb-load` and `results/service/`

`qdb-load` puts load on a `qdb-server` and runs the service's tests.
It either talks to a running server (`--server ADDR`) or starts the
`qdb-server` beside it on a data directory (`--spawn DATA`), passing it
any flags in `QDB_SERVER_ARGS`, and stops it at the end. The admin key
comes from `--admin-key` or `QDB_ADMIN_KEY`. Every workload prints one
line of results, or a line starting `FAIL`. `qdb-load --help` has every
option.

Load, for the targets and the capacity sheet:

- `true --clients N --seconds S`: the shortest unit, for the cost of a
  request.
- `writers --clients N --seconds S`: N writers on one filesystem, each
  appending to a file of its own; group commit shows in the syncs.
- `units FILE --key K --fs ID --clients N --count C`: the lines of a
  file of units (as `qdb-stress units` writes them) on one filesystem.
- `mixed --filesystems N --seconds S`: N small filesystems with one
  client each, 95 reads to 5 writes.
- `interfere --key K --fs ID --seconds S`: small reads on a filesystem of
  their own, alone and then while another connection runs `--search`
  (default `grep -rn define .`) over and over on `ID`.
- `small --count N --clients C`: N small filesystems made, the time to
  make one, what they take on disk, the server's memory once all are
  closed (start it with `--idle-close-secs 1`), and the cost of opening
  one, as a unit on a closed filesystem against one on an open one.
- `deadline --count N`: units that cannot finish, each given 100 ms: how
  long after the deadline each is answered, and that nothing is left.
- `open DIR --count N`: open a database directory N times in process:
  the median time and the heap once open.
- `import --data DATA --tree DIR`: load a directory into a new
  filesystem of a new account in DATA, with the server stopped. This is
  how the Linux tree was made a filesystem.

The service's tests:

- `order --clients 16 --count 10000 --runs 100 --seed 1000`: the
  units-at-once checker. Each run sends generated reads and appends from
  many connections to one new filesystem; then it looks for an order of
  the units, one at a time and consistent with when each was answered,
  that explains every output, and fails naming the units involved when
  there is none. Some units are given a deadline they cannot meet, and
  must leave nothing. `QDB_ORDER_MUTATE=1` breaks one answer on purpose,
  to show that the checker catches it.
- `kill --spawn DATA --count 200 --clients 16 --filesystems 8`: kills
  the server with SIGKILL at random moments under load, starts it
  again, and checks that every filesystem opens, passes `--check` and
  holds every unit that was answered, whole and in order.
- `isolation --count 1000000 --clients 8`: two accounts with marker text;
  requests with each key against its own and the other's IDs, guessed
  IDs, near misses, `..` and odd paths, and command lines that reach for
  what lies outside a filesystem. No answer may hold the other account's
  markers or IDs, and a request for the other's filesystem must be
  answered exactly as one for a filesystem that does not exist.
  `QDB_ISOLATION_MUTATE=1` sends one request in a hundred with the other
  key, to show that the test catches a leak.
- `long --spawn DATA --seconds 2100 --clients 8 --filesystems 3000`: the
  long run. Filesystems are made, used (most units on a few, so that the
  rest go idle and are closed), and deleted; some units are stopped at a
  deadline, some streamed, some in sessions. The server's memory and
  open files are sampled every 10 s into `DATA/long-samples.tsv`; at the
  end every filesystem left must take a write at once.

The harness also runs against a server (`qdb-harness --store api`), and
has `--stops N` (units stopped at random moments by a deadline, a caller
going away or a planted panic, each leaving the filesystem as it was),
`--crash-writers N` (power cuts with several writers sharing syncs) and
`--threads N` (every recursive search on N threads, output unchanged).

`results/service/` holds the output of each run reported in
docs/HANDBACK-6.md.
