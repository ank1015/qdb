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

`results/TARGETS-3.md` checks each stage 3 target against these runs.

`results/TARGETS.md` checks each stage 2 target against these runs, and
`docs/HANDBACK-2.md` discusses them. `results/stage1/` holds the stage 1
results in their original form.

`--report` rebuilds the tables from the stored runs without measuring.
