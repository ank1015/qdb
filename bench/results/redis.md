# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Part 2), and include starting bash and the tool (1.860 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 13.20 | 13.20 | 17.7 MB | 28.2 MB | yes |
| Part 2 | on | 15.40 | 15.40 | 17.7 MB | 20.8 MB | yes |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Agree |
|---|---:|---:|---:|---:|---|
| `ls` | 2.767 |  | 0.005 | 0.003 | yes |
| `ls -a` | 2.733 |  | 0.003 | 0.003 | yes |
| `ls -R` | 3.610 |  | 0.148 | 0.124 | yes |
| `ls -aR tests` | 2.841 |  | 0.032 | 0.018 | yes |
| `ls -p tests` | 2.721 |  | 0.004 | 0.002 | yes |
| `find .` | 4.165 |  | 0.336 | 0.148 | yes |
| `find . -name '*.json'` | 4.045 |  | 0.323 | 0.235 | yes |
| `find . -name '*.c'` | 4.079 |  | 0.219 | 0.164 | yes |
| `find . -type d` | 3.816 |  | 0.107 | 0.141 | yes |
| `find . -type f` | 3.660 |  | 0.153 | 0.188 | yes |
| `find . -maxdepth 2 -type f` | 3.139 |  | 0.031 | 0.030 | yes |
| `find tests -name '*.json' -type f` | 2.707 |  | 0.038 | 0.037 | yes |
| `find . -name 'Makefile'` | 4.101 |  | 0.146 | 0.141 | yes |
| `cat src/commands/xinfo-stream.json` | 2.669 |  | 0.002 | 0.001 | yes |
| `cat -n src/commands/xinfo-stream.json` | 2.684 |  | 0.037 | 0.040 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 2.583 |  | 0.026 | 0.002 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 2.673 |  | 0.044 | 0.002 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 2.610 |  | 0.014 | 0.005 | yes |
| `wc src/commands/xinfo-stream.json` | 2.397 |  | 0.015 | 0.013 | yes |
| `wc -l src/commands/*.json` | 6.285 |  | 1.258 | 1.030 | yes |
| `cat src/commands/*.json \| wc -l` | 8.953 |  | 1.140 | 1.314 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 3.373 |  | 0.014 | 0.010 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 2.868 |  | 0.260 | 0.239 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.925 |  | 0.260 | 0.247 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 2.991 |  | 0.268 | 0.214 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 2.951 |  | 0.384 | 0.391 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.576 |  | 0.094 | 0.124 | yes |
| `grep -rn qdbzzabsentterm .` | 21.29 | 14.11 | 194.2 | 238.6 | yes |
| `grep -rl qdbzzabsentterm .` | 15.84 | 13.02 | 224.0 | 200.8 | yes |
| `grep -rn nestedcallreplyarray .` | 20.75 | 14.24 | 235.1 | 216.5 | yes |
| `grep -rl nestedcallreplyarray .` | 14.45 | 12.21 | 258.8 | 213.1 | yes |
| `grep -rn xmallctlmibnametomib .` | 20.52 | 13.60 | 241.2 | 240.6 | yes |
| `grep -rl xmallctlmibnametomib .` | 15.25 | 13.40 | 249.2 | 213.8 | yes |
| `grep -rn implementing .` | 22.08 | 14.00 | 254.5 | 253.1 | yes |
| `grep -rl implementing .` | 14.23 | 12.04 | 240.1 | 241.1 | yes |
| `grep -rn between .` | 27.59 | 12.56 | 221.6 | 205.9 | yes |
| `grep -rl between .` | 16.62 | 12.63 | 147.8 | 144.8 | yes |
| `grep -rn for .` | 41.74 | 24.33 | 238.5 | 212.3 | yes |
| `grep -rl for .` | 11.27 | 15.44 | 21.83 | 26.56 | yes |
| `grep -rc implementing .` | 16.12 | 13.56 | 204.6 | 254.9 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 22.29 | 17.29 | 200.7 | 233.1 | yes |
| `grep -ri implementing .` | 18.00 | 15.49 | 272.6 | 256.6 | yes |
| `grep -rn implementing tests` | 5.816 | 5.187 | 42.18 | 42.00 | yes |
| `grep -rl between tests` | 5.693 | 5.192 | 30.16 | 33.32 | yes |
| `grep -rF 'implementing' .` | 14.48 | 11.42 | 32.40 | 36.16 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 19.06 | 14.31 | 252.9 | 253.2 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 32.25 | 13.22 | 353.5 | 351.4 | yes |
| `grep -rE 'between\|implementing' .` | 39.78 | 15.75 | 343.7 | 358.8 | yes |
| `grep -rE '[0-9]{4,}' .` | 29.13 | 26.31 | 213.6 | 246.7 | yes |
| `grep -rn '^#include' .` | 28.28 | 17.13 | 185.2 | 222.3 | yes |
| `grep -rE '^(import\|from) ' .` | 22.68 | 15.34 | 229.1 | 225.5 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 7.904 | 6.789 | 39.76 | 44.69 | yes |
| `grep -rv e .github` | 3.173 | 4.412 | 0.459 | 0.471 | yes |
| `grep -r '\(implementing\).*\1' .` | 16.36 |  | 501.5 | 359.8 | yes |
| `grep -rl between . \| wc -l` | 17.19 |  | 122.5 | 146.5 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 30.45 |  | 241.1 | 241.4 | yes |
| `find . -name '*.json' \| wc -l` | 4.905 |  | 0.223 | 0.220 | yes |
| `find . -type f \| grep -c json` | 4.875 |  | 0.739 | 0.818 | yes |
| `grep -rn for . \| head -n 5` | 3.313 |  | 209.5 | 0.031 | order differs |
| `grep -rn between . \| head -n 1` | 4.794 |  | 205.1 | 0.031 | order differs |
| `find . -type f \| head -n 3` | 4.146 |  | 0.160 | 0.049 | order differs |
| `cat src/commands/*.json \| head -n 2` | 3.609 |  | 0.799 | 0.107 | yes |
| `mkdir -p bench_out/a/b` | 2.983 |  | 0.019 | 0.020 | yes |
| `touch bench_out/empty.txt` | 2.777 |  | 0.012 | 0.014 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.765 |  | 0.016 | 0.015 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.661 |  | 0.014 | 0.013 | yes |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 2.986 |  | 0.052 | 0.028 | yes |
| `cp -r tests bench_out/dir_copy` | 7.782 |  | 1.627 | 1.739 | yes |
| `cp -r src/commands bench_out/work` | 8.315 |  | 0.968 | 1.333 | yes |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 3.604 |  | 0.305 | 0.317 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.774 |  | 0.394 | 0.355 | yes |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 26.08 |  | 9.896 | 12.62 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 5.849 |  | 40.57 | 50.70 | yes |
| `grep -rl QDBBENCHMANY .` | 14.72 |  | 294.6 | 285.1 | yes |
| `grep -rl QDBBENCHONE .` | 22.12 |  | 253.5 | 227.2 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.406 |  | 0.023 | 0.025 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.855 |  | 0.023 | 0.020 | yes |
| `ls -R bench_out/a` | 3.773 |  | 0.019 | 0.016 | yes |
| `rm bench_out/a/b/hello.txt` | 2.759 |  | 0.019 | 0.014 | yes |
| `rmdir bench_out/a/b` | 4.298 |  | 0.018 | 0.017 | yes |
| `rm -r bench_out/dir_moved` | 7.157 |  | 0.432 | 0.345 | yes |
| `rm -rf bench_out` | 41.16 |  | 0.489 | 0.491 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 |
|---|---:|---:|---:|
| `ls` | 288 B | 1841 B | 1681 B |
| `ls -a` | 338 B | 2009 B | 1849 B |
| `ls -R` | 25.0 KB | 56.8 KB | 38.2 KB |
| `ls -aR tests` | 4713 B | 12.6 KB | 12.5 KB |
| `ls -p tests` | 118 B | 1469 B | 1309 B |
| `find .` | 57.3 KB | 82.8 KB | 21.4 KB |
| `find . -name '*.json'` | 12.4 KB | 19.3 KB | 19.1 KB |
| `find . -name '*.c'` | 12.3 KB | 15.2 KB | 15.0 KB |
| `find . -type d` | 2349 B | 5217 B | 5057 B |
| `find . -type f` | 54.9 KB | 103.5 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 4257 B | 7803 B | 7643 B |
| `find tests -name '*.json' -type f` | 0 B | 1830 B | 1926 B |
| `find . -name 'Makefile'` | 462 B | 2076 B | 1916 B |
| `cat src/commands/xinfo-stream.json` | 17.8 KB | 18.6 KB | 1018 B |
| `cat -n src/commands/xinfo-stream.json` | 20.3 KB | 33.7 KB | 17.1 KB |
| `head -n 20 src/commands/xinfo-stream.json` | 640 B | 1694 B | 1534 B |
| `tail -n 20 src/commands/xinfo-stream.json` | 568 B | 1622 B | 1462 B |
| `tail -n +100 src/commands/xinfo-stream.json` | 13.9 KB | 15.0 KB | 14.8 KB |
| `wc src/commands/xinfo-stream.json` | 49 B | 954 B | 1026 B |
| `wc -l src/commands/*.json` | 14.4 KB | 71.0 KB | 71.0 KB |
| `cat src/commands/*.json \| wc -l` | 6 B | 1312.9 KB | 71.2 KB |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 261 B | 21.1 KB | 4591 B |
| `grep -c for src/commands/xinfo-stream.json` | 2 B | 3593 B | 3377 B |
| `grep -n for src/commands/xinfo-stream.json` | 330 B | 4065 B | 3849 B |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17.8 KB | 22.2 KB | 22.1 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17.8 KB | 23.8 KB | 23.7 KB |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 6 B | 41.5 KB | 24.1 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB |
| `grep -rn nestedcallreplyarray .` | 165 B | 15.1 KB | 14.9 KB |
| `grep -rl nestedcallreplyarray .` | 25 B | 15.1 KB | 14.9 KB |
| `grep -rn xmallctlmibnametomib .` | 206 B | 15.4 KB | 15.1 KB |
| `grep -rl xmallctlmibnametomib .` | 76 B | 15.2 KB | 15.0 KB |
| `grep -rn implementing .` | 2538 B | 12.8 KB | 10.9 KB |
| `grep -rl implementing .` | 371 B | 9894 B | 9434 B |
| `grep -rn between .` | 45.4 KB | 54.8 KB | 9081 B |
| `grep -rl between .` | 4657 B | 11.3 KB | 6219 B |
| `grep -rn for .` | 1815.1 KB | 3083.9 KB | 27.7 KB |
| `grep -rl for .` | 35.6 KB | 69.2 KB | 3799 B |
| `grep -rc implementing .` | 58.3 KB | 111.9 KB | 35.0 KB |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 206 B | 15.4 KB | 15.1 KB |
| `grep -ri implementing .` | 2754 B | 12.8 KB | 10.9 KB |
| `grep -rn implementing tests` | 200 B | 9669 B | 9453 B |
| `grep -rl between tests` | 722 B | 7058 B | 6026 B |
| `grep -rF 'implementing' .` | 2435 B | 4638 B | 1926 B |
| `grep -rE 'arena_[a-z]+\(' .` | 24.7 KB | 34.1 KB | 13.9 KB |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 355 B | 28.7 KB | 28.4 KB |
| `grep -rE 'between\|implementing' .` | 46.2 KB | 62.2 KB | 17.2 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 888.7 KB | 31.5 KB |
| `grep -rn '^#include' .` | 172.4 KB | 253.0 KB | 14.9 KB |
| `grep -rE '^(import\|from) ' .` | 8992 B | 19.3 KB | 10.2 KB |
| `grep -rin 'return [a-z_]*;$' tests` | 53.9 KB | 106.3 KB | 13.9 KB |
| `grep -rv e .github` | 14.9 KB | 20.8 KB | 11.4 KB |
| `grep -r '\(implementing\).*\1' .` | 0 B | 6012 B | 5796 B |
| `grep -rl between . \| wc -l` | 4 B | 11.9 KB | 6980 B |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 5 B | 180.1 KB | 7969 B |
| `find . -name '*.json' \| wc -l` | 4 B | 19.9 KB | 19.9 KB |
| `find . -type f \| grep -c json` | 4 B | 107.1 KB | 30.7 KB |
| `grep -rn for . \| head -n 5` | 502 B | 3084.6 KB | 4870 B |
| `grep -rn between . \| head -n 1` | 191 B | 55.6 KB | 7155 B |
| `find . -type f \| head -n 3` | 82 B | 104.2 KB | 27.6 KB |
| `cat src/commands/*.json \| head -n 2` | 15 B | 1313.0 KB | 71.3 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1619 B | 1459 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0 B | 18.9 KB | 18.7 KB |
| `cp -r tests bench_out/dir_copy` | 0 B | 3153.2 KB | 3153.0 KB |
| `cp -r src/commands bench_out/work` | 0 B | 787.8 KB | 787.6 KB |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 24.7 KB | 24.5 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 24.1 KB | 24.0 KB |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 0 B | 338.7 KB | 338.6 KB |
| `grep -rc QDBBENCHMANY bench_out` | 25.7 KB | 46.3 KB | 27.7 KB |
| `grep -rl QDBBENCHMANY .` | 4346 B | 18.1 KB | 9490 B |
| `grep -rl QDBBENCHONE .` | 21 B | 9127 B | 8911 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1070 B | 1129 B |
| `rm -rf bench_out` | 0 B | 3746 B | 3586 B |

## Search terms and paths

| Placeholder | Value | Measured |
|---|---|---|
| ABSENT | `qdbzzabsentterm` | 0 of 1672 files, 0.00% |
| COMMON | `for` | 1104 of 1672 files, 66.03% |
| DIR | `tests` | holds 263 of 1672 files, 15.73% |
| EXT | `json` | 406 files in the whole dataset |
| EXT2 | `c` | 435 files in the whole dataset |
| FEW | `implementing` | 17 of 1672 files, 1.02% |
| FILE | `src/commands/xinfo-stream.json` | 17801 bytes |
| GLOB | `src/commands/*.json` | 405 files |
| GLOBDIR | `src/commands` | the folder of GLOB |
| ONE | `nestedcallreplyarray` | 1 of 1672 files, 0.06% |
| PREFIX | `arena` | PREFIX_[a-z]+\( matches in 34 of 1672 files, 2.03% |
| RARE | `xmallctlmibnametomib` | 2 of 1672 files, 0.12% |
| RAREUP | `XMALLCTLMIBNAMETOMIB` | the RARE term in capitals, for -i |
| SMALLDIR | `.github` | holds 15 of 1672 files, 0.90% |
| SOME | `between` | 167 of 1672 files, 9.99% |
