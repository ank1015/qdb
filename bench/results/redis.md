# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 1), and include starting bash and the tool (2.244 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 13.20 | 13.20 | 17.7 MB | 28.2 MB | yes |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Agree |
|---|---:|---:|---:|---|
| `ls` | 3.541 |  | 0.005 | yes |
| `ls -a` | 2.844 |  | 0.003 | yes |
| `ls -R` | 4.112 |  | 0.148 | yes |
| `ls -aR tests` | 3.774 |  | 0.032 | yes |
| `ls -p tests` | 3.326 |  | 0.004 | yes |
| `find .` | 4.582 |  | 0.336 | yes |
| `find . -name '*.json'` | 4.053 |  | 0.323 | yes |
| `find . -name '*.c'` | 4.089 |  | 0.219 | yes |
| `find . -type d` | 4.347 |  | 0.107 | yes |
| `find . -type f` | 4.272 |  | 0.153 | yes |
| `find . -maxdepth 2 -type f` | 3.428 |  | 0.031 | yes |
| `find tests -name '*.json' -type f` | 2.848 |  | 0.038 | yes |
| `find . -name 'Makefile'` | 3.737 |  | 0.146 | yes |
| `cat src/commands/xinfo-stream.json` | 3.198 |  | 0.002 | yes |
| `cat -n src/commands/xinfo-stream.json` | 2.847 |  | 0.037 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 3.926 |  | 0.026 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 3.078 |  | 0.044 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 3.149 |  | 0.014 | yes |
| `wc src/commands/xinfo-stream.json` | 2.894 |  | 0.015 | yes |
| `wc -l src/commands/*.json` | 6.499 |  | 1.258 | yes |
| `cat src/commands/*.json \| wc -l` | 10.76 |  | 1.140 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 3.662 |  | 0.014 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 2.719 |  | 0.260 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.965 |  | 0.260 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 3.334 |  | 0.268 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 3.350 |  | 0.384 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.862 |  | 0.094 | yes |
| `grep -rn qdbzzabsentterm .` | 18.59 | 11.75 | 194.2 | yes |
| `grep -rl qdbzzabsentterm .` | 15.82 | 13.07 | 224.0 | yes |
| `grep -rn nestedcallreplyarray .` | 21.00 | 14.86 | 235.1 | yes |
| `grep -rl nestedcallreplyarray .` | 14.69 | 13.46 | 258.8 | yes |
| `grep -rn xmallctlmibnametomib .` | 20.60 | 14.35 | 241.2 | yes |
| `grep -rl xmallctlmibnametomib .` | 14.30 | 13.35 | 249.2 | yes |
| `grep -rn implementing .` | 23.32 | 14.27 | 254.5 | yes |
| `grep -rl implementing .` | 16.37 | 13.53 | 240.1 | yes |
| `grep -rn between .` | 26.29 | 16.07 | 221.6 | yes |
| `grep -rl between .` | 16.45 | 13.65 | 147.8 | yes |
| `grep -rn for .` | 42.66 | 24.16 | 238.5 | yes |
| `grep -rl for .` | 12.21 | 14.87 | 21.83 | yes |
| `grep -rc implementing .` | 14.66 | 11.99 | 204.6 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 21.63 | 15.58 | 200.7 | yes |
| `grep -ri implementing .` | 18.59 | 15.80 | 272.6 | yes |
| `grep -rn implementing tests` | 7.313 | 6.999 | 42.18 | yes |
| `grep -rl between tests` | 6.974 | 6.611 | 30.16 | yes |
| `grep -rF 'implementing' .` | 16.33 | 13.39 | 32.40 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 16.62 | 12.44 | 252.9 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 29.43 | 13.78 | 353.5 | yes |
| `grep -rE 'between\|implementing' .` | 39.44 | 14.24 | 343.7 | yes |
| `grep -rE '[0-9]{4,}' .` | 29.03 | 26.61 | 213.6 | yes |
| `grep -rn '^#include' .` | 27.61 | 16.98 | 185.2 | yes |
| `grep -rE '^(import\|from) ' .` | 22.67 | 13.69 | 229.1 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 10.70 | 8.421 | 39.76 | yes |
| `grep -rv e .github` | 4.288 | 4.603 | 0.459 | yes |
| `grep -r '\(implementing\).*\1' .` | 14.98 |  | 501.5 | yes |
| `grep -rl between . \| wc -l` | 17.19 |  | 122.5 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 30.18 |  | 241.1 | yes |
| `find . -name '*.json' \| wc -l` | 4.852 |  | 0.223 | yes |
| `find . -type f \| grep -c json` | 4.541 |  | 0.739 | yes |
| `grep -rn for . \| head -n 5` | 3.658 |  | 209.5 | order differs |
| `grep -rn between . \| head -n 1` | 5.502 |  | 205.1 | order differs |
| `find . -type f \| head -n 3` | 4.153 |  | 0.160 | order differs |
| `cat src/commands/*.json \| head -n 2` | 3.872 |  | 0.799 | yes |
| `mkdir -p bench_out/a/b` | 3.355 |  | 0.019 | yes |
| `touch bench_out/empty.txt` | 3.766 |  | 0.012 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 2.493 |  | 0.016 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 2.270 |  | 0.014 | yes |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 3.566 |  | 0.052 | yes |
| `cp -r tests bench_out/dir_copy` | 6.153 |  | 1.627 | yes |
| `cp -r src/commands bench_out/work` | 6.697 |  | 0.968 | yes |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 3.655 |  | 0.305 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 4.219 |  | 0.394 | yes |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 31.72 |  | 9.896 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 8.332 |  | 40.57 | yes |
| `grep -rl QDBBENCHMANY .` | 19.14 |  | 294.6 | yes |
| `grep -rl QDBBENCHONE .` | 16.30 |  | 253.5 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 4.598 |  | 0.023 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 3.461 |  | 0.023 | yes |
| `ls -R bench_out/a` | 3.267 |  | 0.019 | yes |
| `rm bench_out/a/b/hello.txt` | 3.842 |  | 0.019 | yes |
| `rmdir bench_out/a/b` | 4.931 |  | 0.018 | yes |
| `rm -r bench_out/dir_moved` | 8.172 |  | 0.432 | yes |
| `rm -rf bench_out` | 36.09 |  | 0.489 | yes |

## Peak memory per command

| Command | Output | Stage 1 |
|---|---:|---:|
| `ls` | 288 B | 1841 B |
| `ls -a` | 338 B | 2009 B |
| `ls -R` | 25.0 KB | 56.8 KB |
| `ls -aR tests` | 4713 B | 12.6 KB |
| `ls -p tests` | 118 B | 1469 B |
| `find .` | 57.3 KB | 82.8 KB |
| `find . -name '*.json'` | 12.4 KB | 19.3 KB |
| `find . -name '*.c'` | 12.3 KB | 15.2 KB |
| `find . -type d` | 2349 B | 5217 B |
| `find . -type f` | 54.9 KB | 103.5 KB |
| `find . -maxdepth 2 -type f` | 4257 B | 7803 B |
| `find tests -name '*.json' -type f` | 0 B | 1830 B |
| `find . -name 'Makefile'` | 462 B | 2076 B |
| `cat src/commands/xinfo-stream.json` | 17.8 KB | 18.6 KB |
| `cat -n src/commands/xinfo-stream.json` | 20.3 KB | 33.7 KB |
| `head -n 20 src/commands/xinfo-stream.json` | 640 B | 1694 B |
| `tail -n 20 src/commands/xinfo-stream.json` | 568 B | 1622 B |
| `tail -n +100 src/commands/xinfo-stream.json` | 13.9 KB | 15.0 KB |
| `wc src/commands/xinfo-stream.json` | 49 B | 954 B |
| `wc -l src/commands/*.json` | 14.4 KB | 71.0 KB |
| `cat src/commands/*.json \| wc -l` | 6 B | 1312.9 KB |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 261 B | 21.1 KB |
| `grep -c for src/commands/xinfo-stream.json` | 2 B | 3593 B |
| `grep -n for src/commands/xinfo-stream.json` | 330 B | 4065 B |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17.8 KB | 22.2 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17.8 KB | 23.8 KB |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 6 B | 41.5 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB |
| `grep -rn nestedcallreplyarray .` | 165 B | 15.1 KB |
| `grep -rl nestedcallreplyarray .` | 25 B | 15.1 KB |
| `grep -rn xmallctlmibnametomib .` | 206 B | 15.4 KB |
| `grep -rl xmallctlmibnametomib .` | 76 B | 15.2 KB |
| `grep -rn implementing .` | 2538 B | 12.8 KB |
| `grep -rl implementing .` | 371 B | 9894 B |
| `grep -rn between .` | 45.4 KB | 54.8 KB |
| `grep -rl between .` | 4657 B | 11.3 KB |
| `grep -rn for .` | 1815.1 KB | 3083.9 KB |
| `grep -rl for .` | 35.6 KB | 69.2 KB |
| `grep -rc implementing .` | 58.3 KB | 111.9 KB |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 206 B | 15.4 KB |
| `grep -ri implementing .` | 2754 B | 12.8 KB |
| `grep -rn implementing tests` | 200 B | 9669 B |
| `grep -rl between tests` | 722 B | 7058 B |
| `grep -rF 'implementing' .` | 2435 B | 4638 B |
| `grep -rE 'arena_[a-z]+\(' .` | 24.7 KB | 34.1 KB |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 355 B | 28.7 KB |
| `grep -rE 'between\|implementing' .` | 46.2 KB | 62.2 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 888.7 KB |
| `grep -rn '^#include' .` | 172.4 KB | 253.0 KB |
| `grep -rE '^(import\|from) ' .` | 8992 B | 19.3 KB |
| `grep -rin 'return [a-z_]*;$' tests` | 53.9 KB | 106.3 KB |
| `grep -rv e .github` | 14.9 KB | 20.8 KB |
| `grep -r '\(implementing\).*\1' .` | 0 B | 6012 B |
| `grep -rl between . \| wc -l` | 4 B | 11.9 KB |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 5 B | 180.1 KB |
| `find . -name '*.json' \| wc -l` | 4 B | 19.9 KB |
| `find . -type f \| grep -c json` | 4 B | 107.1 KB |
| `grep -rn for . \| head -n 5` | 502 B | 3084.6 KB |
| `grep -rn between . \| head -n 1` | 191 B | 55.6 KB |
| `find . -type f \| head -n 3` | 82 B | 104.2 KB |
| `cat src/commands/*.json \| head -n 2` | 15 B | 1313.0 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1619 B |
| `touch bench_out/empty.txt` | 0 B | 883 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0 B | 18.9 KB |
| `cp -r tests bench_out/dir_copy` | 0 B | 3153.2 KB |
| `cp -r src/commands bench_out/work` | 0 B | 787.8 KB |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 24.7 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 24.1 KB |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 0 B | 338.7 KB |
| `grep -rc QDBBENCHMANY bench_out` | 25.7 KB | 46.3 KB |
| `grep -rl QDBBENCHMANY .` | 4346 B | 18.1 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 9127 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B |
| `ls -R bench_out/a` | 41 B | 1213 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B |
| `rmdir bench_out/a/b` | 0 B | 849 B |
| `rm -r bench_out/dir_moved` | 0 B | 1070 B |
| `rm -rf bench_out` | 0 B | 3746 B |

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
