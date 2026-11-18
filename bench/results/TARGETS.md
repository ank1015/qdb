# Stage 2 targets, measured

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

Written by `bench/targets.py` from the runs under `bench/results/`. Times are in milliseconds.
"Index on" is the run labelled `part4`, "index off" is `part4-noindex`. GNU times come from the same run as the engine time beside them.

## 1. `grep -rn COMMON . | head -n 5` on the largest dataset: under 1 ms

| Dataset | Command | GNU | Index off | Index on | Target |
|---|---|---:|---:|---:|---|
| redis | `grep -rn for . \| head -n 5` | 2.95 | 0.005 | 0.005 | met |
| postgres | `grep -rn not . \| head -n 5` | 4.31 | 0.012 | 0.008 | met |
| large | `grep -rn werefo . \| head -n 5` | 4.69 | 0.008 | 0.013 | met |
| linux | `grep -rn define . \| head -n 5` | 4.66 | 0.020 | 0.019 | met |

## 2. A streamed command's peak memory does not grow with its output

The same commands on each dataset: output size against the most memory held while running (index on).

| Command | redis: output | redis: peak | postgres: output | postgres: peak | large: output | large: peak | linux: output | linux: peak |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `grep -rn {COMMON} .` | 1.8 MB | 25.5 KB | 13.4 MB | 31.1 KB | 1.4 MB | 9.4 KB | 821.1 MB | 26.9 KB |
| `grep -rn {SOME} .` | 45.4 KB | 23.7 KB | 225.8 KB | 15.3 KB | 121.7 KB | 34.7 KB | 1.4 MB | 136.2 KB |
| `find . -type f` | 54.9 KB | 26.7 KB | 330.7 KB | 26.7 KB | 412.7 KB | 25.6 KB | 3.4 MB | 32.8 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 33.9 KB | 39.4 MB | 23.1 KB | 40.3 MB | 16.4 KB | 467.5 MB | 28.6 KB |
| `cat {GLOB} | wc -l` | 6 B | 71.2 KB | 7 B | 269.1 KB | 7 B | 100.6 KB | 7 B | 237.2 KB |

The largest output of any read-only command on linux is 821.1 MB (`grep -rn define .`), produced with a peak of 26.9 KB.

## 3. Every recursive grep, index off: at least as fast as GNU grep, at every size

| Dataset | Recursive greps | Slower than GNU | Closest to GNU | Engine | GNU | GNU / engine |
|---|---:|---:|---|---:|---:|---:|
| redis | 34 | 0 | `grep -rE '[0-9]{4,}' .` | 18.7 | 26.8 | 1.4x |
| postgres | 34 | 0 | `grep -rE '[0-9]{4,}' .` | 187 | 296 | 1.6x |
| large | 29 | 0 | `grep -rE '[0-9]{4,}' .` | 104 | 282 | 2.7x |
| linux | 34 | 0 | `grep -rE '[0-9]{4,}' .` | 2,033 | 2,803 | 1.4x |

## 4. Search for a term in no file: under 1 ms at every size (index on)

| Dataset | Command | GNU | rg | Index off | Index on | Target |
|---|---|---:|---:|---:|---:|---|
| redis | `grep -rn qdbzzabsentterm .` | 21.9 | 13.2 | 1.33 | 0.008 | met |
| redis | `grep -rl qdbzzabsentterm .` | 13.7 | 13.6 | 1.54 | 0.009 | met |
| postgres | `grep -rn qdbzzabsentterm .` | 160 | 71.1 | 23.8 | 0.014 | met |
| postgres | `grep -rl qdbzzabsentterm .` | 112 | 74.1 | 23.2 | 0.015 | met |
| large | `grep -rn qdbzzabsentterm .` | 134 | 92.2 | 11.2 | 0.015 | met |
| large | `grep -rl qdbzzabsentterm .` | 96.9 | 98.4 | 12.2 | 0.017 | met |
| linux | `grep -rn qdbzzabsentterm .` | 1,173 | 550 | 206 | 0.012 | met |
| linux | `grep -rl qdbzzabsentterm .` | 733 | 523 | 214 | 0.011 | met |

## 5. A term in 1% of files or fewer, on the largest dataset: at least 10x faster than GNU grep (index on)

| Term | In files | Command | GNU | rg | Index off | Index on | GNU / index on | Target |
|---|---|---|---:|---:|---:|---:|---:|---|
| ONE | 1 of 83405 files, 0.00% | `grep -rn ausgabenspezifisches .` | 1,154 | 587 | 209 | 0.098 | 11,834x | met |
| ONE | 1 of 83405 files, 0.00% | `grep -rl ausgabenspezifisches .` | 789 | 542 | 205 | 0.095 | 8,281x | met |
| RARE | 84 of 83405 files, 0.10% | `grep -rn mispredicted .` | 1,313 | 588 | 209 | 17.5 | 75x | met |
| RARE | 84 of 83405 files, 0.10% | `grep -rl mispredicted .` | 929 | 543 | 208 | 17.6 | 53x | met |
| FEW | 834 of 83405 files, 1.00% | `grep -rn separately .` | 991 | 583 | 211 | 24.1 | 41x | met |
| FEW | 834 of 83405 files, 1.00% | `grep -rl separately .` | 594 | 550 | 211 | 18.8 | 32x | met |

## 6. Any benchmark command, index on against index off: no more than 10% slower

**Commands that only read** are compared in one process, with searches using the index and ignoring it by turns (the run labelled `paired`, `qdb-bench --paired`). Two separate runs of the benchmark cannot show a difference of a tenth: the same command in two runs of the same configuration differs by up to 30% (where memory happens to land), index or no index.

| Dataset | Read commands | Over 10% slower with the index | Slowest relative to no index | Fastest relative to no index |
|---|---:|---:|---|---|
| redis | 62 | 0 | `grep -rv e .github` 1.02x | `grep -rn qdbzzabsentterm .` 0.0183x |
| postgres | 62 | 0 | `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` 1.05x | `grep -rl qdbzzabsentterm .` 0.0014x |
| large | 59 | 0 | `ls -aR tests` 1.05x | `grep -rn qdbzzabsentterm .` 0.0016x |
| linux | 62 | 2 | `find . -name '*.c'` 1.14x | `grep -rn qdbzzabsentterm .` 0.0003x |

The commands over 10%:

| Dataset | Command | Index ignored | Index used | Ratio | Does it ask the index? |
|---|---|---:|---:|---:|---|
| linux | `find . -name '*.c'` | 23.6 | 26.9 | 1.14x | no: the difference is not the index's doing |
| linux | `find . -name 'Makefile'` | 18.3 | 20.4 | 1.11x | no: the difference is not the index's doing |

**Commands that change data** are compared between the two runs, since the index cannot be ignored while it is being kept current. A write has to keep the index current, and that work is the cost of the index: these miss the target.

| Dataset | Command | Index off | Index on | Ratio |
|---|---|---:|---:|---:|
| redis | `mkdir -p bench_out/a/b` | 0.011 | 0.015 | 1.4x |
| redis | `touch bench_out/empty.txt` | 0.009 | 0.015 | 1.6x |
| redis | `echo 'hello world' > bench_out/hello.txt` | 0.012 | 0.024 | 2.1x |
| redis | `echo 'another line' >> bench_out/hello.txt` | 0.011 | 0.020 | 1.9x |
| redis | `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0.022 | 0.121 | 5.6x |
| redis | `cp -r tests bench_out/dir_copy` | 1.35 | 19.7 | 14.5x |
| redis | `cp -r src/commands bench_out/work` | 1.30 | 6.66 | 5.1x |
| redis | `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0.271 | 0.364 | 1.3x |
| redis | `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 14.8 | 24.8 | 1.7x |
| redis | `mv bench_out/dir_copy bench_out/dir_moved` | 0.017 | 0.024 | 1.4x |
| redis | `mv bench_out/hello.txt bench_out/a/b` | 0.013 | 0.024 | 1.9x |
| redis | `rm bench_out/a/b/hello.txt` | 0.013 | 0.015 | 1.2x |
| redis | `rm -r bench_out/dir_moved` | 0.203 | 0.599 | 3.0x |
| redis | `rm -rf bench_out` | 0.209 | 3.56 | 17.0x |
| postgres | `echo 'hello world' > bench_out/hello.txt` | 0.016 | 0.019 | 1.2x |
| postgres | `echo 'another line' >> bench_out/hello.txt` | 0.013 | 0.018 | 1.4x |
| postgres | `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0.026 | 0.316 | 12.1x |
| postgres | `cp -r src/backend bench_out/dir_copy` | 42.2 | 372 | 8.8x |
| postgres | `cp -r doc/src/sgml/html bench_out/work` | 13.4 | 101 | 7.5x |
| postgres | `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 0.295 | 0.538 | 1.8x |
| postgres | `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0.253 | 0.492 | 1.9x |
| postgres | `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 231 | 297 | 1.3x |
| postgres | `mv bench_out/dir_copy bench_out/dir_moved` | 0.021 | 0.024 | 1.1x |
| postgres | `rm -r bench_out/dir_moved` | 0.748 | 12.6 | 16.8x |
| postgres | `rm -rf bench_out` | 0.609 | 19.3 | 31.8x |
| large | `touch bench_out/empty.txt` | 0.006 | 0.013 | 2.1x |
| large | `echo 'hello world' > bench_out/hello.txt` | 0.009 | 0.018 | 2.1x |
| large | `echo 'another line' >> bench_out/hello.txt` | 0.009 | 0.016 | 1.7x |
| large | `cp src/main.c bench_out/copy.txt` | 0.018 | 0.261 | 14.7x |
| large | `cp -r tests bench_out/dir_copy` | 9.65 | 115 | 11.9x |
| large | `cp -r src bench_out/work` | 42.8 | 356 | 8.3x |
| large | `sed -i 's/werefo/QDBBENCHONE/g' bench_out/copy.txt` | 0.311 | 0.530 | 1.7x |
| large | `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0.340 | 0.476 | 1.4x |
| large | `sed -i 's/werefo/QDBBENCHMANY/g' bench_out/work/*.c` | 79.7 | 112 | 1.4x |
| large | `rmdir bench_out/a/b` | 0.014 | 0.016 | 1.2x |
| large | `rm -rf bench_out` | 4.30 | 63.3 | 14.7x |
| linux | `echo 'another line' >> bench_out/hello.txt` | 0.008 | 0.018 | 2.3x |
| linux | `cp include/linux/kernfs.h bench_out/copy.txt` | 0.022 | 0.510 | 23.4x |
| linux | `cp -r Documentation bench_out/dir_copy` | 38.2 | 476 | 12.5x |
| linux | `cp -r include/linux bench_out/work` | 19.8 | 173 | 8.7x |
| linux | `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 0.387 | 0.545 | 1.4x |
| linux | `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0.328 | 0.478 | 1.5x |
| linux | `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 141 | 213 | 1.5x |
| linux | `mv bench_out/dir_copy bench_out/dir_moved` | 0.026 | 0.035 | 1.3x |
| linux | `rmdir bench_out/a/b` | 0.009 | 0.015 | 1.7x |
| linux | `rm -r bench_out/dir_moved` | 6.80 | 8.59 | 1.3x |
| linux | `rm -rf bench_out` | 2.07 | 3.70 | 1.8x |

## 7. Index size: at most 1x the raw text

| Dataset | Text | Index in memory | Share of text | Posting data alone | Target |
|---|---:|---:|---:|---:|---|
| redis | 17.4 MB | 4.9 MB | 28.0% | 1.9 MB (10.8%) | met |
| postgres | 161.2 MB | 21.4 MB | 13.3% | 12.7 MB (7.9%) | met |
| large | 87.4 MB | 21.6 MB | 24.7% | 18.9 MB (21.6%) | met |
| linux | 1.4 GB | 148.0 MB | 10.6% | 117.4 MB (8.4%) | met |

## 8. A write to one file: time depends on the size of the file, not of the database

The same 64 KB of text added as a new file to each dataset (median of nine files), and two commands from the list that write the same small content everywhere.

| Dataset | Text in the database | New 64 KB file, index off | New 64 KB file, index on | `echo 'hello world' > file`, index on | `echo 'another line' >> file`, index on |
|---|---:|---:|---:|---:|---:|
| redis | 17.4 MB | 0.000 | 0.234 | 0.024 | 0.020 |
| postgres | 161.2 MB | 0.001 | 0.272 | 0.019 | 0.018 |
| large | 87.4 MB | 0.001 | 0.250 | 0.018 | 0.016 |
| linux | 1.4 GB | 0.001 | 0.280 | 0.017 | 0.018 |

## 9. Write commands and loading, index on and off (reported, no target)

| Dataset | Read the files | Build the index | Heap after load, index off | Heap after load, index on | Resident, index off | Resident, index on |
|---|---:|---:|---:|---:|---:|---:|
| redis | 19.5 | 90.9 | 17.7 MB | 24.7 MB | 21.1 MB | 27.5 MB |
| postgres | 179 | 800 | 163.7 MB | 187.2 MB | 167.5 MB | 192.5 MB |
| large | 146 | 754 | 94.0 MB | 117.7 MB | 98.5 MB | 123.1 MB |
| linux | 1,375 | 6,772 | 1.4 GB | 1.6 GB | 1.4 GB | 1.6 GB |

### Commands that change data: redis

| Command | GNU | Index off | Index on | Index work |
|---|---:|---:|---:|---|
| `mkdir -p bench_out/a/b` | 3.98 | 0.011 | 0.015 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | 5.31 | 0.009 | 0.015 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | 1.93 | 0.012 | 0.024 | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | 1.55 | 0.011 | 0.020 | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 2.83 | 0.022 | 0.121 | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `cp -r tests bench_out/dir_copy` | 133 | 1.35 | 19.7 | documents +262 -0; postings +308931; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/commands bench_out/work` | 158 | 1.30 | 6.66 | documents +406 -0; postings +152926; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 4.69 | 0.313 | 0.302 | documents +1 -1; postings +886; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.53 | 0.271 | 0.364 | documents +1 -1; postings +867; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 173 | 14.8 | 24.8 | documents +405 -405; postings +153510; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | 6.16 | 0.790 | 0.458 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | 18.0 | 4.07 | 0.295 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | 15.2 | 3.34 | 0.091 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.62 | 0.017 | 0.024 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | 2.57 | 0.013 | 0.024 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | 2.96 | 0.016 | 0.026 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | 2.07 | 0.013 | 0.015 | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | 3.05 | 0.013 | 0.014 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | 6.83 | 0.203 | 0.599 | documents +0 -262; postings +0; list bytes rewritten 34111; rebuilds 0 |
| `rm -rf bench_out` | 35.8 | 0.209 | 3.56 | documents +0 -407; postings +0; list bytes rewritten 396061; rebuilds 0 |

### Commands that change data: postgres

| Command | GNU | Index off | Index on | Index work |
|---|---:|---:|---:|---|
| `mkdir -p bench_out/a/b` | 3.09 | 0.011 | 0.010 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | 2.68 | 0.013 | 0.012 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | 1.88 | 0.016 | 0.019 | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | 2.00 | 0.013 | 0.018 | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 3.02 | 0.026 | 0.316 | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/backend bench_out/dir_copy` | 495 | 42.2 | 372 | documents +1346 -0; postings +2878944; list bytes rewritten 0; rebuilds 0 |
| `cp -r doc/src/sgml/html bench_out/work` | 355 | 13.4 | 101 | documents +1173 -0; postings +2134398; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 4.81 | 0.295 | 0.538 | documents +1 -1; postings +2793; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.78 | 0.253 | 0.492 | documents +1 -1; postings +2724; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 427 | 231 | 297 | documents +1169 -1169; postings +2139969; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | 27.9 | 12.2 | 4.29 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | 81.8 | 32.7 | 1.84 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | 115 | 35.5 | 0.384 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.60 | 0.021 | 0.024 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | 3.57 | 0.027 | 0.022 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | 3.08 | 0.014 | 0.012 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | 2.79 | 0.013 | 0.008 | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | 3.29 | 0.011 | 0.010 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | 27.1 | 0.748 | 12.6 | documents +0 -1346; postings +0; list bytes rewritten 3153154; rebuilds 0 |
| `rm -rf bench_out` | 96.5 | 0.609 | 19.3 | documents +0 -1174; postings +0; list bytes rewritten 5973942; rebuilds 0 |

### Commands that change data: large

| Command | GNU | Index off | Index on | Index work |
|---|---:|---:|---:|---|
| `mkdir -p bench_out/a/b` | 2.65 | 0.011 | 0.010 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | 2.42 | 0.006 | 0.013 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | 1.59 | 0.009 | 0.018 | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | 1.51 | 0.009 | 0.016 | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp src/main.c bench_out/copy.txt` | 2.57 | 0.018 | 0.261 | documents +1 -0; postings +2642; list bytes rewritten 0; rebuilds 0 |
| `cp -r tests bench_out/dir_copy` | 234 | 9.65 | 115 | documents +1787 -0; postings +2730624; list bytes rewritten 0; rebuilds 0 |
| `cp -r src bench_out/work` | 800 | 42.8 | 356 | documents +5425 -0; postings +8936125; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/werefo/QDBBENCHONE/g' bench_out/copy.txt` | 4.07 | 0.311 | 0.530 | documents +1 -1; postings +2649; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 4.01 | 0.340 | 0.476 | documents +1 -1; postings +1979; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/werefo/QDBBENCHMANY/g' bench_out/work/*.c` | 96.5 | 79.7 | 112 | documents +773 -773; postings +1280848; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | 68.3 | 14.8 | 3.64 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | 153 | 32.0 | 1.15 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | 139 | 32.2 | 0.085 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.71 | 0.028 | 0.023 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | 2.31 | 0.022 | 0.020 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | 2.29 | 0.018 | 0.013 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | 2.77 | 0.015 | 0.014 | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | 2.78 | 0.014 | 0.016 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | 106 | 1.67 | 1.80 | documents +0 -1787; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | 271 | 4.30 | 63.3 | documents +0 -5426; postings +0; list bytes rewritten 40260515; rebuilds 0 |

### Commands that change data: linux

| Command | GNU | Index off | Index on | Index work |
|---|---:|---:|---:|---|
| `mkdir -p bench_out/a/b` | 2.61 | 0.016 | 0.015 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | 2.32 | 0.013 | 0.009 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | 1.49 | 0.017 | 0.017 | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | 1.60 | 0.008 | 0.018 | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 2.45 | 0.022 | 0.510 | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | 979 | 38.2 | 476 | documents +9698 -0; postings +9217002; list bytes rewritten 0; rebuilds 0 |
| `cp -r include/linux bench_out/work` | 324 | 19.8 | 173 | documents +2631 -0; postings +2591832; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 3.72 | 0.387 | 0.545 | documents +1 -1; postings +2616; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.09 | 0.328 | 0.478 | documents +1 -1; postings +2504; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 386 | 141 | 213 | documents +1432 -1432; postings +1564925; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | 57.9 | 17.2 | 7.13 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | 604 | 242 | 3.63 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | 843 | 240 | 0.603 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.81 | 0.026 | 0.035 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | 2.55 | 0.020 | 0.015 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | 2.33 | 0.016 | 0.019 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | 2.30 | 0.013 | 0.011 | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | 3.39 | 0.009 | 0.015 | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | 143 | 6.80 | 8.59 | documents +0 -9698; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | 135 | 2.07 | 3.70 | documents +0 -2632; postings +0; list bytes rewritten 0; rebuilds 0 |
