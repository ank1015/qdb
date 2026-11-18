# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (dev), and include starting bash and the tool (2.237 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 13.20 | 13.20 | 17.7 MB | 28.2 MB | yes |
| Part 2 | on | 15.40 | 15.40 | 17.7 MB | 20.8 MB | yes |
| Part 3, no index | off | 29.00 | 29.00 | 17.7 MB | 20.9 MB | yes |
| dev | on | 108.7 | 196.2 | 22.6 MB | 26.3 MB | yes |

## The index (dev)

- Built in 87.60 ms while loading.
- Size: 5603.6 KB in memory, 32.2% of the text (2097.2 KB table, 3440.9 KB lists, 65.5 KB document table). The posting data itself is 2434.3 KB.
- 74790 distinct trigrams, 2287862 postings, 1669 documents.
- Share of posting data in the longest lists: top 100 trigrams 7.0%, top 1,000 37.2%, top 10,000 82.3%.
- These figures are taken after the command list has run, so they include its writes.

| Files in a trigram's list | Trigrams |
|---:|---:|
| 1 | 24884 |
| 2-2 | 10965 |
| 3-4 | 9637 |
| 5-8 | 8051 |
| 9-16 | 6774 |
| 17-32 | 4822 |
| 33-64 | 3512 |
| 65-128 | 2454 |
| 129-256 | 1589 |
| 257-512 | 1140 |
| 513-1024 | 675 |
| 1025-2048 | 279 |
| 2049-4096 | 8 |

Index work per command that changes data:

| Command | Index work |
|---|---|
| `mkdir -p bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `cp -r tests bench_out/dir_copy` | documents +262 -0; postings +308931; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/commands bench_out/work` | documents +406 -0; postings +152926; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -1; postings +886; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +867; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | documents +405 -405; postings +153510; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -262; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -407; postings +0; list bytes rewritten 375623; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | dev | Agree |
|---|---:|---:|---:|---:|---:|---:|---|
| `ls` | 3.051 |  | 0.005 | 0.003 | 0.003 | 0.003 | yes |
| `ls -a` | 3.098 |  | 0.003 | 0.003 | 0.004 | 0.003 | yes |
| `ls -R` | 9.297 |  | 0.148 | 0.124 | 0.172 | 0.075 | yes |
| `ls -aR tests` | 4.452 |  | 0.032 | 0.018 | 0.045 | 0.017 | yes |
| `ls -p tests` | 3.128 |  | 0.004 | 0.002 | 0.003 | 0.002 | yes |
| `find .` | 4.472 |  | 0.336 | 0.148 | 0.253 | 0.144 | yes |
| `find . -name '*.json'` | 4.188 |  | 0.323 | 0.235 | 0.221 | 0.218 | yes |
| `find . -name '*.c'` | 4.290 |  | 0.219 | 0.164 | 0.187 | 0.200 | yes |
| `find . -type d` | 3.836 |  | 0.107 | 0.141 | 0.159 | 0.146 | yes |
| `find . -type f` | 4.176 |  | 0.153 | 0.188 | 0.178 | 0.152 | yes |
| `find . -maxdepth 2 -type f` | 3.315 |  | 0.031 | 0.030 | 0.021 | 0.033 | yes |
| `find tests -name '*.json' -type f` | 3.623 |  | 0.038 | 0.037 | 0.036 | 0.057 | yes |
| `find . -name 'Makefile'` | 4.509 |  | 0.146 | 0.141 | 0.208 | 0.141 | yes |
| `cat src/commands/xinfo-stream.json` | 3.137 |  | 0.002 | 0.001 | 0.002 | 0.001 | yes |
| `cat -n src/commands/xinfo-stream.json` | 3.138 |  | 0.037 | 0.040 | 0.035 | 0.037 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 2.987 |  | 0.026 | 0.002 | 0.002 | 0.002 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 2.599 |  | 0.044 | 0.002 | 0.003 | 0.004 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 2.702 |  | 0.014 | 0.005 | 0.005 | 0.005 | yes |
| `wc src/commands/xinfo-stream.json` | 2.808 |  | 0.015 | 0.013 | 0.013 | 0.017 | yes |
| `wc -l src/commands/*.json` | 5.960 |  | 1.258 | 1.030 | 1.201 | 1.119 | yes |
| `cat src/commands/*.json \| wc -l` | 10.12 |  | 1.140 | 1.314 | 0.986 | 1.109 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 3.910 |  | 0.014 | 0.010 | 0.009 | 0.011 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 3.053 |  | 0.260 | 0.239 | 0.006 | 0.007 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.819 |  | 0.260 | 0.247 | 0.006 | 0.006 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 2.960 |  | 0.268 | 0.214 | 0.235 | 0.188 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 3.008 |  | 0.384 | 0.391 | 0.304 | 0.260 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.819 |  | 0.094 | 0.124 | 0.082 | 0.101 | yes |
| `grep -rn qdbzzabsentterm .` | 23.30 | 15.67 | 194.2 | 238.6 | 1.909 | 0.012 | yes |
| `grep -rl qdbzzabsentterm .` | 15.44 | 13.51 | 224.0 | 200.8 | 2.003 | 0.015 | yes |
| `grep -rn nestedcallreplyarray .` | 21.17 | 15.34 | 235.1 | 216.5 | 1.437 | 0.107 | yes |
| `grep -rl nestedcallreplyarray .` | 16.55 | 18.52 | 258.8 | 213.1 | 3.039 | 0.150 | yes |
| `grep -rn xmallctlmibnametomib .` | 20.24 | 19.18 | 241.2 | 240.6 | 1.406 | 0.025 | yes |
| `grep -rl xmallctlmibnametomib .` | 16.15 | 14.78 | 249.2 | 213.8 | 1.558 | 0.043 | yes |
| `grep -rn implementing .` | 24.17 | 15.37 | 254.5 | 253.1 | 1.593 | 0.934 | yes |
| `grep -rl implementing .` | 16.59 | 16.02 | 240.1 | 241.1 | 1.473 | 0.759 | yes |
| `grep -rn between .` | 28.28 | 15.54 | 221.6 | 205.9 | 1.872 | 1.282 | yes |
| `grep -rl between .` | 18.17 | 13.96 | 147.8 | 144.8 | 0.759 | 0.179 | yes |
| `grep -rn for .` | 45.82 | 26.75 | 238.5 | 212.3 | 3.336 | 4.292 | yes |
| `grep -rl for .` | 13.24 | 16.82 | 21.83 | 26.56 | 0.457 | 0.632 | yes |
| `grep -rc implementing .` | 17.97 | 13.91 | 204.6 | 254.9 | 1.703 | 0.947 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 21.38 | 17.24 | 200.7 | 233.1 | 1.958 | 0.024 | yes |
| `grep -ri implementing .` | 19.52 | 16.69 | 272.6 | 256.6 | 1.671 | 0.994 | yes |
| `grep -rn implementing tests` | 6.401 | 5.684 | 42.18 | 42.00 | 0.288 | 0.084 | yes |
| `grep -rl between tests` | 5.580 | 5.061 | 30.16 | 33.32 | 0.293 | 0.041 | yes |
| `grep -rF 'implementing' .` | 15.74 | 14.55 | 32.40 | 36.16 | 1.270 | 0.840 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 19.96 | 15.17 | 252.9 | 253.2 | 2.028 | 0.506 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 30.18 | 15.12 | 353.5 | 351.4 | 5.286 | 0.243 | yes |
| `grep -rE 'between\|implementing' .` | 41.10 | 18.89 | 343.7 | 358.8 | 2.563 | 2.042 | yes |
| `grep -rE '[0-9]{4,}' .` | 30.52 | 26.47 | 213.6 | 246.7 | 19.74 | 18.89 | yes |
| `grep -rn '^#include' .` | 29.16 | 20.93 | 185.2 | 222.3 | 2.394 | 1.810 | yes |
| `grep -rE '^(import\|from) ' .` | 23.74 | 18.10 | 229.1 | 225.5 | 2.708 | 2.518 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 9.109 | 8.499 | 39.76 | 44.69 | 0.700 | 0.573 | yes |
| `grep -rv e .github` | 3.626 | 5.812 | 0.459 | 0.471 | 0.041 | 0.044 | yes |
| `grep -r '\(implementing\).*\1' .` | 17.59 |  | 501.5 | 359.8 | 1.631 | 0.838 | yes |
| `grep -rl between . \| wc -l` | 18.11 |  | 122.5 | 146.5 | 0.896 | 0.227 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 31.17 |  | 241.1 | 241.4 | 2.700 | 2.739 | yes |
| `find . -name '*.json' \| wc -l` | 5.388 |  | 0.223 | 0.220 | 0.240 | 0.370 | yes |
| `find . -type f \| grep -c json` | 5.621 |  | 0.739 | 0.818 | 0.295 | 0.221 | yes |
| `grep -rn for . \| head -n 5` | 3.784 |  | 209.5 | 0.031 | 0.007 | 0.130 | order differs |
| `grep -rn between . \| head -n 1` | 5.575 |  | 205.1 | 0.031 | 0.013 | 0.049 | order differs |
| `find . -type f \| head -n 3` | 3.948 |  | 0.160 | 0.049 | 0.046 | 0.053 | order differs |
| `cat src/commands/*.json \| head -n 2` | 5.213 |  | 0.799 | 0.107 | 0.097 | 0.119 | yes |
| `mkdir -p bench_out/a/b` | 4.613 |  | 0.019 | 0.020 | 0.018 | 0.020 | yes |
| `touch bench_out/empty.txt` | 2.907 |  | 0.012 | 0.014 | 0.014 | 0.014 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 2.807 |  | 0.016 | 0.015 | 0.020 | 0.040 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 2.220 |  | 0.014 | 0.013 | 0.015 | 0.061 | yes |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 6.307 |  | 0.052 | 0.028 | 0.031 | 1.006 | yes |
| `cp -r tests bench_out/dir_copy` | 10.33 |  | 1.627 | 1.739 | 2.065 | 32.78 | yes |
| `cp -r src/commands bench_out/work` | 17.05 |  | 0.968 | 1.333 | 1.301 | 6.967 | yes |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 4.031 |  | 0.305 | 0.317 | 0.232 | 1.013 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 5.347 |  | 0.394 | 0.355 | 0.307 | 1.205 | yes |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 27.23 |  | 9.896 | 12.62 | 10.24 | 15.99 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 9.282 |  | 40.57 | 50.70 | 0.803 | 0.445 | yes |
| `grep -rl QDBBENCHMANY .` | 17.52 |  | 294.6 | 285.1 | 3.467 | 0.477 | yes |
| `grep -rl QDBBENCHONE .` | 23.71 |  | 253.5 | 227.2 | 4.375 | 0.174 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 5.157 |  | 0.023 | 0.025 | 0.024 | 0.035 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 6.358 |  | 0.023 | 0.020 | 0.019 | 0.044 | yes |
| `ls -R bench_out/a` | 3.086 |  | 0.019 | 0.016 | 0.021 | 0.029 | yes |
| `rm bench_out/a/b/hello.txt` | 3.691 |  | 0.019 | 0.014 | 0.018 | 0.026 | yes |
| `rmdir bench_out/a/b` | 4.295 |  | 0.018 | 0.017 | 0.012 | 0.024 | yes |
| `rm -r bench_out/dir_moved` | 8.981 |  | 0.432 | 0.345 | 0.343 | 0.698 | yes |
| `rm -rf bench_out` | 42.52 |  | 0.489 | 0.491 | 0.527 | 14.85 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 | Part 3, no index | dev |
|---|---:|---:|---:|---:|---:|
| `ls` | 288 B | 1841 B | 1681 B | 1681 B | 1681 B |
| `ls -a` | 338 B | 2009 B | 1849 B | 1849 B | 1849 B |
| `ls -R` | 25.0 KB | 56.8 KB | 38.2 KB | 38.2 KB | 38.2 KB |
| `ls -aR tests` | 4713 B | 12.6 KB | 12.5 KB | 12.5 KB | 12.5 KB |
| `ls -p tests` | 118 B | 1469 B | 1309 B | 1309 B | 1309 B |
| `find .` | 57.3 KB | 82.8 KB | 21.4 KB | 21.4 KB | 21.4 KB |
| `find . -name '*.json'` | 12.4 KB | 19.3 KB | 19.1 KB | 19.1 KB | 19.1 KB |
| `find . -name '*.c'` | 12.3 KB | 15.2 KB | 15.0 KB | 15.0 KB | 15.0 KB |
| `find . -type d` | 2349 B | 5217 B | 5057 B | 5057 B | 5057 B |
| `find . -type f` | 54.9 KB | 103.5 KB | 26.7 KB | 26.7 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 4257 B | 7803 B | 7643 B | 7643 B | 7643 B |
| `find tests -name '*.json' -type f` | 0 B | 1830 B | 1926 B | 1926 B | 1926 B |
| `find . -name 'Makefile'` | 462 B | 2076 B | 1916 B | 1916 B | 1916 B |
| `cat src/commands/xinfo-stream.json` | 17.8 KB | 18.6 KB | 1018 B | 1018 B | 1018 B |
| `cat -n src/commands/xinfo-stream.json` | 20.3 KB | 33.7 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `head -n 20 src/commands/xinfo-stream.json` | 640 B | 1694 B | 1534 B | 1534 B | 1534 B |
| `tail -n 20 src/commands/xinfo-stream.json` | 568 B | 1622 B | 1462 B | 1462 B | 1462 B |
| `tail -n +100 src/commands/xinfo-stream.json` | 13.9 KB | 15.0 KB | 14.8 KB | 14.8 KB | 14.8 KB |
| `wc src/commands/xinfo-stream.json` | 49 B | 954 B | 1026 B | 1026 B | 1026 B |
| `wc -l src/commands/*.json` | 14.4 KB | 71.0 KB | 71.0 KB | 71.0 KB | 71.0 KB |
| `cat src/commands/*.json \| wc -l` | 6 B | 1312.9 KB | 71.2 KB | 71.2 KB | 71.2 KB |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 261 B | 21.1 KB | 4591 B | 4591 B | 4591 B |
| `grep -c for src/commands/xinfo-stream.json` | 2 B | 3593 B | 3377 B | 5818 B | 5818 B |
| `grep -n for src/commands/xinfo-stream.json` | 330 B | 4065 B | 3849 B | 5818 B | 5818 B |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17.8 KB | 22.2 KB | 22.1 KB | 22.2 KB | 22.2 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17.8 KB | 23.8 KB | 23.7 KB | 23.8 KB | 23.8 KB |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 6 B | 41.5 KB | 24.1 KB | 24.4 KB | 24.4 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB | 20.3 KB |
| `grep -rn nestedcallreplyarray .` | 165 B | 15.1 KB | 14.9 KB | 26.1 KB | 27.8 KB |
| `grep -rl nestedcallreplyarray .` | 25 B | 15.1 KB | 14.9 KB | 26.1 KB | 27.8 KB |
| `grep -rn xmallctlmibnametomib .` | 206 B | 15.4 KB | 15.1 KB | 26.1 KB | 27.8 KB |
| `grep -rl xmallctlmibnametomib .` | 76 B | 15.2 KB | 15.0 KB | 26.1 KB | 27.8 KB |
| `grep -rn implementing .` | 2538 B | 12.8 KB | 10.9 KB | 16.4 KB | 25.9 KB |
| `grep -rl implementing .` | 371 B | 9894 B | 9434 B | 16.4 KB | 25.9 KB |
| `grep -rn between .` | 45.4 KB | 54.8 KB | 9081 B | 10.4 KB | 24.3 KB |
| `grep -rl between .` | 4657 B | 11.3 KB | 6219 B | 10.4 KB | 24.3 KB |
| `grep -rn for .` | 1815.1 KB | 3083.9 KB | 27.7 KB | 25.5 KB | 123.8 KB |
| `grep -rl for .` | 35.6 KB | 69.2 KB | 3799 B | 5758 B | 116.0 KB |
| `grep -rc implementing .` | 58.3 KB | 111.9 KB | 35.0 KB | 32.2 KB | 45.1 KB |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 206 B | 15.4 KB | 15.1 KB | 26.3 KB | 27.8 KB |
| `grep -ri implementing .` | 2754 B | 12.8 KB | 10.9 KB | 16.4 KB | 25.9 KB |
| `grep -rn implementing tests` | 200 B | 9669 B | 9453 B | 16.4 KB | 17.1 KB |
| `grep -rl between tests` | 722 B | 7058 B | 6026 B | 10.4 KB | 10.6 KB |
| `grep -rF 'implementing' .` | 2435 B | 4638 B | 1926 B | 2514 B | 25.9 KB |
| `grep -rE 'arena_[a-z]+\(' .` | 24.7 KB | 34.1 KB | 13.9 KB | 19.5 KB | 27.5 KB |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 355 B | 28.7 KB | 28.4 KB | 50.7 KB | 53.1 KB |
| `grep -rE 'between\|implementing' .` | 46.2 KB | 62.2 KB | 17.2 KB | 25.4 KB | 29.7 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 888.7 KB | 31.5 KB | 33.9 KB | 33.9 KB |
| `grep -rn '^#include' .` | 172.4 KB | 253.0 KB | 14.9 KB | 15.9 KB | 68.4 KB |
| `grep -rE '^(import\|from) ' .` | 8992 B | 19.3 KB | 10.2 KB | 20.9 KB | 72.2 KB |
| `grep -rin 'return [a-z_]*;$' tests` | 53.9 KB | 106.3 KB | 13.9 KB | 20.6 KB | 32.2 KB |
| `grep -rv e .github` | 14.9 KB | 20.8 KB | 11.4 KB | 10.8 KB | 10.8 KB |
| `grep -r '\(implementing\).*\1' .` | 0 B | 6012 B | 5796 B | 10.4 KB | 30.0 KB |
| `grep -rl between . \| wc -l` | 4 B | 11.9 KB | 6980 B | 11.1 KB | 25.0 KB |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 5 B | 180.1 KB | 7969 B | 9019 B | 118.2 KB |
| `find . -name '*.json' \| wc -l` | 4 B | 19.9 KB | 19.9 KB | 19.9 KB | 19.9 KB |
| `find . -type f \| grep -c json` | 4 B | 107.1 KB | 30.7 KB | 34.3 KB | 34.5 KB |
| `grep -rn for . \| head -n 5` | 502 B | 3084.6 KB | 4870 B | 6624 B | 116.9 KB |
| `grep -rn between . \| head -n 1` | 191 B | 55.6 KB | 7155 B | 11.2 KB | 25.1 KB |
| `find . -type f \| head -n 3` | 82 B | 104.2 KB | 27.6 KB | 27.6 KB | 27.6 KB |
| `cat src/commands/*.json \| head -n 2` | 15 B | 1313.0 KB | 71.3 KB | 71.3 KB | 71.3 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1619 B | 1459 B | 1459 B | 1459 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B | 979 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B | 1242 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B | 1246 B | 1246 B |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0 B | 18.9 KB | 18.7 KB | 18.7 KB | 285.3 KB |
| `cp -r tests bench_out/dir_copy` | 0 B | 3153.2 KB | 3153.0 KB | 3153.0 KB | 5633.9 KB |
| `cp -r src/commands bench_out/work` | 0 B | 787.8 KB | 787.6 KB | 787.6 KB | 993.7 KB |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 24.7 KB | 24.5 KB | 24.6 KB | 24.8 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 24.1 KB | 24.0 KB | 24.1 KB | 24.7 KB |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 0 B | 338.7 KB | 338.6 KB | 338.7 KB | 520.8 KB |
| `grep -rc QDBBENCHMANY bench_out` | 25.7 KB | 46.3 KB | 27.7 KB | 16.4 KB | 26.0 KB |
| `grep -rl QDBBENCHMANY .` | 4346 B | 18.1 KB | 9490 B | 16.4 KB | 22.3 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 9127 B | 8911 B | 15.3 KB | 16.0 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B | 1191 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B | 1288 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B | 1089 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B | 997 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B | 945 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1070 B | 1129 B | 1129 B | 1129 B |
| `rm -rf bench_out` | 0 B | 3746 B | 3586 B | 3586 B | 3586 B |

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
