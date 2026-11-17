# Benchmark

`qdb-bench` times a fixed list of commands on the real GNU tools and on the
engine, checks that both give the same result, and writes a table.

```
cargo build --release
./target/release/qdb-bench --dataset all      # small, medium and large
./target/release/qdb-bench --dir path/to/tree # your own data
```

## Datasets

The datasets are generated from a fixed seed, so every run sees the same
bytes. They look like a source repository: nested folders of code, docs,
config and logs, with a few fixed paths the command list relies on.

| Name | Files | Folders | Text |
|---|---:|---:|---:|
| small | 205 | 134 | 1.1 MB |
| medium | 2,005 | 1,421 | 13.7 MB |
| large | 12,005 | 6,925 | 87.4 MB |

`--export DIR` writes a dataset to disk without timing anything.

## Command list

`commands.txt` holds 69 command lines. Read-only commands are timed as the
median of five runs. Lines starting with `mut:` change data; they run once,
in order, and the two trees are compared at the end.

## Results

`results/<dataset>.md` and `.csv` hold the full tables from one machine.
All three datasets agree with the real tools on every command and on the
final tree.

How to read the numbers: the real side pays for starting bash and the tool
on every command (1.4 to 5 ms here), and reads from a file system whose
cache is warm. The engine is timed in-process with the data already in
memory. So the comparison is not like for like, and it is meant as a
baseline for later stages rather than a contest.

What the large dataset shows:

- Listing, finding, reading one file and editing one file are 10 to 300
  times faster in the engine, mostly because there is no process to
  start. Copying a large folder is about 3 times faster.
- Recursive `grep` is the weak spot (20 of the 69 commands are slower on
  the engine, all of them recursive greps): 2 to 14 times slower than GNU grep,
  and 25 times slower for a pattern with a back-reference. GNU grep skips
  through text looking for a literal before it runs any regex; the naive
  engine runs its matcher on every line. This is the first thing an index
  or a literal pre-filter would fix.
