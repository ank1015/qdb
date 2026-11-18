# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 3 part 1), and include starting bash and the tool (1.385 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 1 | none | 13.20 | 13.20 | 17.7 MB | 28.2 MB |  | yes |
| Part 2 | none | 15.40 | 15.40 | 17.7 MB | 20.8 MB |  | yes |
| Part 3, no index | none | 21.70 | 21.70 | 17.7 MB | 21.0 MB | 0.001 | yes |
| Part 4, index off | off | 20.80 | 20.80 | 17.7 MB | 21.1 MB | 0.000 | yes |
| Part 4, index on | on | 19.50 | 110.4 | 24.7 MB | 27.5 MB | 0.234 | yes |
| Stage 3 part 1 | on | 482.6 | 627.9 | 24.7 MB | 27.5 MB | 0.258 | yes |

## The index (Part 4, index on)

- Built in 90.90 ms while loading.
- Size: 4862.9 KB in memory, 28.0% of the text (2097.2 KB table, 2733.0 KB lists, 32.8 KB document table). The posting data itself is 1874.2 KB.
- 74777 distinct trigrams, 1750115 postings, 1669 documents.
- Share of posting data in the longest lists: top 100 trigrams 5.6%, top 1,000 32.4%, top 10,000 79.8%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|
| 1 | 27621 |
| 2 | 10099 |
| 3-4 | 9474 |
| 5-8 | 7783 |
| 9-16 | 6538 |
| 17-32 | 4554 |
| 33-64 | 3327 |
| 65-128 | 2231 |
| 129-256 | 1505 |
| 257-512 | 1073 |
| 513-1024 | 532 |
| 1025-2048 | 40 |

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
| `rm -r bench_out/dir_moved` | documents +0 -262; postings +0; list bytes rewritten 34111; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -407; postings +0; list bytes rewritten 396061; rebuilds 0 |

## The index (Stage 3 part 1)

- Built in 145.3 ms while loading.
- Size: 4862.9 KB in memory, 28.0% of the text (2097.2 KB table, 2733.0 KB lists, 32.8 KB document table). The posting data itself is 1874.2 KB.
- 74777 distinct trigrams, 1750115 postings, 1669 documents.
- Share of posting data in the longest lists: top 100 trigrams 5.6%, top 1,000 32.4%, top 10,000 79.8%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|
| 1 | 27621 |
| 2 | 10099 |
| 3-4 | 9474 |
| 5-8 | 7783 |
| 9-16 | 6538 |
| 17-32 | 4554 |
| 33-64 | 3327 |
| 65-128 | 2231 |
| 129-256 | 1505 |
| 257-512 | 1073 |
| 513-1024 | 532 |
| 1025-2048 | 40 |

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
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | documents +133 -133; postings +56822; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -262; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -408; postings +0; list bytes rewritten 236016; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Part 4, index off | Part 4, index on | Stage 3 part 1 | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.128 |  | 0.005 | 0.003 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `ls -a` | 2.484 |  | 0.003 | 0.003 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `ls -R` | 4.307 |  | 0.148 | 0.124 | 0.084 | 0.042 | 0.043 | 0.037 | yes |
| `ls -aR tests` | 2.623 |  | 0.032 | 0.018 | 0.019 | 0.013 | 0.013 | 0.013 | yes |
| `ls -p tests` | 2.387 |  | 0.004 | 0.002 | 0.001 | 0.001 | 0.002 | 0.001 | yes |
| `find .` | 3.487 |  | 0.336 | 0.148 | 0.147 | 0.235 | 0.143 | 0.119 | yes |
| `find . -name '*.json'` | 4.664 |  | 0.323 | 0.235 | 0.197 | 0.308 | 0.227 | 0.160 | yes |
| `find . -name '*.c'` | 3.619 |  | 0.219 | 0.164 | 0.196 | 0.296 | 0.162 | 0.162 | yes |
| `find . -type d` | 4.472 |  | 0.107 | 0.141 | 0.107 | 0.221 | 0.127 | 0.122 | yes |
| `find . -type f` | 4.710 |  | 0.153 | 0.188 | 0.145 | 0.218 | 0.116 | 0.123 | yes |
| `find . -maxdepth 2 -type f` | 2.939 |  | 0.031 | 0.030 | 0.020 | 0.033 | 0.017 | 0.016 | yes |
| `find tests -name '*.json' -type f` | 3.454 |  | 0.038 | 0.037 | 0.030 | 0.043 | 0.022 | 0.025 | yes |
| `find . -name 'Makefile'` | 4.846 |  | 0.146 | 0.141 | 0.151 | 0.230 | 0.129 | 0.125 | yes |
| `cat src/commands/xinfo-stream.json` | 2.382 |  | 0.002 | 0.001 | 0.001 | 0.002 | 0.001 | 0.001 | yes |
| `cat -n src/commands/xinfo-stream.json` | 2.807 |  | 0.037 | 0.040 | 0.033 | 0.061 | 0.027 | 0.032 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 2.259 |  | 0.026 | 0.002 | 0.002 | 0.003 | 0.002 | 0.002 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 2.188 |  | 0.044 | 0.002 | 0.002 | 0.003 | 0.002 | 0.002 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 2.272 |  | 0.014 | 0.005 | 0.004 | 0.008 | 0.004 | 0.004 | yes |
| `wc src/commands/xinfo-stream.json` | 2.353 |  | 0.015 | 0.013 | 0.010 | 0.021 | 0.014 | 0.012 | yes |
| `wc -l src/commands/*.json` | 5.447 |  | 1.258 | 1.030 | 0.967 | 1.178 | 1.148 | 1.076 | yes |
| `cat src/commands/*.json \| wc -l` | 10.46 |  | 1.140 | 1.314 | 1.148 | 1.122 | 1.115 | 1.274 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 3.283 |  | 0.014 | 0.010 | 0.005 | 0.006 | 0.006 | 0.011 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 3.061 |  | 0.260 | 0.239 | 0.004 | 0.004 | 0.004 | 0.007 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.992 |  | 0.260 | 0.247 | 0.004 | 0.005 | 0.004 | 0.008 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 3.115 |  | 0.268 | 0.214 | 0.150 | 0.191 | 0.190 | 0.362 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 3.239 |  | 0.384 | 0.391 | 0.290 | 0.288 | 0.292 | 0.515 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.861 |  | 0.094 | 0.124 | 0.074 | 0.071 | 0.065 | 0.141 | yes |
| `grep -rn qdbzzabsentterm .` | 22.99 | 14.69 | 194.2 | 238.6 | 1.382 | 1.327 | 0.008 | 0.010 | yes |
| `grep -rl qdbzzabsentterm .` | 16.90 | 13.92 | 224.0 | 200.8 | 1.252 | 1.543 | 0.009 | 0.015 | yes |
| `grep -rn nestedcallreplyarray .` | 19.47 | 14.71 | 235.1 | 216.5 | 1.471 | 1.115 | 0.078 | 0.153 | yes |
| `grep -rl nestedcallreplyarray .` | 14.57 | 14.06 | 258.8 | 213.1 | 1.443 | 1.407 | 0.095 | 0.146 | yes |
| `grep -rn xmallctlmibnametomib .` | 19.72 | 15.42 | 241.2 | 240.6 | 1.348 | 0.996 | 0.023 | 0.031 | yes |
| `grep -rl xmallctlmibnametomib .` | 15.27 | 14.16 | 249.2 | 213.8 | 1.354 | 1.317 | 0.033 | 0.030 | yes |
| `grep -rn implementing .` | 21.88 | 19.48 | 254.5 | 253.1 | 1.632 | 1.089 | 0.807 | 0.853 | yes |
| `grep -rl implementing .` | 17.20 | 17.87 | 240.1 | 241.1 | 1.633 | 1.168 | 0.958 | 0.840 | yes |
| `grep -rn between .` | 34.08 | 19.02 | 221.6 | 205.9 | 1.666 | 1.480 | 0.859 | 0.907 | yes |
| `grep -rl between .` | 18.35 | 13.47 | 147.8 | 144.8 | 0.826 | 0.631 | 0.242 | 0.172 | yes |
| `grep -rn for .` | 43.36 | 31.65 | 238.5 | 212.3 | 3.926 | 3.676 | 3.332 | 3.429 | yes |
| `grep -rl for .` | 12.17 | 17.91 | 21.83 | 26.56 | 0.444 | 0.332 | 0.321 | 0.297 | yes |
| `grep -rc implementing .` | 16.62 | 14.58 | 204.6 | 254.9 | 1.399 | 1.307 | 0.910 | 1.257 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 21.62 | 15.90 | 200.7 | 233.1 | 1.941 | 1.593 | 0.026 | 0.025 | yes |
| `grep -ri implementing .` | 18.95 | 19.87 | 272.6 | 256.6 | 1.655 | 1.743 | 0.875 | 1.050 | yes |
| `grep -rn implementing tests` | 5.752 | 5.102 | 42.18 | 42.00 | 0.180 | 0.214 | 0.064 | 0.072 | yes |
| `grep -rl between tests` | 4.242 | 4.785 | 30.16 | 33.32 | 0.170 | 0.126 | 0.025 | 0.022 | yes |
| `grep -rF 'implementing' .` | 16.04 | 14.47 | 32.40 | 36.16 | 1.432 | 1.429 | 0.701 | 0.780 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 19.39 | 13.75 | 252.9 | 253.2 | 2.044 | 1.376 | 0.473 | 0.435 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 29.43 | 15.64 | 353.5 | 351.4 | 1.822 | 2.087 | 0.203 | 0.184 | yes |
| `grep -rE 'between\|implementing' .` | 37.22 | 15.85 | 343.7 | 358.8 | 2.588 | 2.010 | 1.703 | 1.202 | yes |
| `grep -rE '[0-9]{4,}' .` | 28.29 | 25.41 | 213.6 | 246.7 | 20.74 | 18.67 | 22.20 | 20.78 | yes |
| `grep -rn '^#include' .` | 26.65 | 16.13 | 185.2 | 222.3 | 1.637 | 1.655 | 1.348 | 1.329 | yes |
| `grep -rE '^(import\|from) ' .` | 23.08 | 15.86 | 229.1 | 225.5 | 2.632 | 2.560 | 2.596 | 3.722 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 8.133 | 6.678 | 39.76 | 44.69 | 0.640 | 0.719 | 0.723 | 0.845 | yes |
| `grep -rv e .github` | 2.657 | 3.832 | 0.459 | 0.471 | 0.039 | 0.046 | 0.037 | 0.032 | yes |
| `grep -r '\(implementing\).*\1' .` | 15.68 |  | 501.5 | 359.8 | 1.406 | 1.356 | 0.855 | 1.103 | yes |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 5.010 |  | | | | | | 0.140 | yes |
| `grep -l implementing src/commands/*.json` | 4.629 |  | | | | | | 0.201 | yes |
| `grep -c qdbzzabsentterm src/commands/*.json` | 4.123 |  | | | | | | 0.142 | yes |
| `grep -rl between . \| wc -l` | 16.69 |  | 122.5 | 146.5 | 0.995 | 0.838 | 0.244 | 0.142 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 31.26 |  | 241.1 | 241.4 | 2.445 | 2.293 | 1.997 | 2.226 | yes |
| `find . -name '*.json' \| wc -l` | 4.452 |  | 0.223 | 0.220 | 0.209 | 0.186 | 0.172 | 0.142 | yes |
| `find . -type f \| grep -c json` | 5.300 |  | 0.739 | 0.818 | 0.319 | 0.215 | 0.212 | 0.215 | yes |
| `grep -rn for . \| head -n 5` | 3.417 |  | 209.5 | 0.031 | 0.008 | 0.005 | 0.005 | 0.009 | order differs |
| `grep -rn between . \| head -n 1` | 5.509 |  | 205.1 | 0.031 | 0.007 | 0.007 | 0.007 | 0.007 | order differs |
| `find . -type f \| head -n 3` | 3.748 |  | 0.160 | 0.049 | 0.041 | 0.036 | 0.036 | 0.035 | order differs |
| `cat src/commands/*.json \| head -n 2` | 3.647 |  | 0.799 | 0.107 | 0.101 | 0.089 | 0.098 | 0.100 | yes |
| `mkdir -p bench_out/a/b` | 2.417 |  | 0.019 | 0.020 | 0.007 | 0.011 | 0.015 | 0.011 | yes |
| `touch bench_out/empty.txt` | 2.303 |  | 0.012 | 0.014 | 0.011 | 0.009 | 0.015 | 0.009 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.991 |  | 0.016 | 0.015 | 0.009 | 0.012 | 0.024 | 0.026 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.650 |  | 0.014 | 0.013 | 0.046 | 0.011 | 0.020 | 0.025 | yes |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 2.585 |  | 0.052 | 0.028 | 0.019 | 0.022 | 0.121 | 0.213 | yes |
| `cp -r tests bench_out/dir_copy` | 9.790 |  | 1.627 | 1.739 | 1.149 | 1.355 | 19.65 | 29.52 | yes |
| `cp -r src/commands bench_out/work` | 7.126 |  | 0.968 | 1.333 | 1.043 | 1.299 | 6.664 | 6.582 | yes |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 2.450 |  | 0.305 | 0.317 | 0.251 | 0.313 | 0.302 | 0.343 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.219 |  | 0.394 | 0.355 | 0.263 | 0.271 | 0.364 | 0.409 | yes |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 28.50 |  | 9.896 | 12.62 | 15.81 | 14.77 | 24.77 | 7.262 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | 122.7 |  | | | | | | 0.549 | yes |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | 58.07 |  | | | | | | 0.398 | yes |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 3.411 |  | | | | | | 0.026 | yes |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 2.514 |  | | | | | | 0.026 | yes |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | 3.351 |  | | | | | | 0.020 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 5.261 |  | 40.57 | 50.70 | 1.075 | 0.790 | 0.458 | 0.341 | yes |
| `grep -rl QDBBENCHMANY .` | 13.93 |  | 294.6 | 285.1 | 4.625 | 4.073 | 0.295 | 0.186 | yes |
| `grep -rl QDBBENCHONE .` | 16.26 |  | 253.5 | 227.2 | 4.362 | 3.341 | 0.091 | 0.067 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.671 |  | 0.023 | 0.025 | 0.021 | 0.017 | 0.024 | 0.019 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.605 |  | 0.023 | 0.020 | 0.038 | 0.013 | 0.024 | 0.020 | yes |
| `ls -R bench_out/a` | 2.551 |  | 0.019 | 0.016 | 0.015 | 0.016 | 0.026 | 0.014 | yes |
| `rm bench_out/a/b/hello.txt` | 2.319 |  | 0.019 | 0.014 | 0.013 | 0.013 | 0.015 | 0.013 | yes |
| `rmdir bench_out/a/b` | 2.608 |  | 0.018 | 0.017 | 0.010 | 0.013 | 0.014 | 0.011 | yes |
| `rm -r bench_out/dir_moved` | 6.023 |  | 0.432 | 0.345 | 0.169 | 0.203 | 0.599 | 0.174 | yes |
| `rm -rf bench_out` | 40.24 |  | 0.489 | 0.491 | 0.225 | 0.209 | 3.563 | 2.346 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 | Part 3, no index | Part 4, index off | Part 4, index on | Stage 3 part 1 |
|---|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 288 B | 1841 B | 1681 B | 1681 B | 1681 B | 1681 B | 1681 B |
| `ls -a` | 338 B | 2009 B | 1849 B | 1849 B | 1849 B | 1849 B | 1849 B |
| `ls -R` | 25.0 KB | 56.8 KB | 38.2 KB | 38.2 KB | 38.2 KB | 38.2 KB | 38.2 KB |
| `ls -aR tests` | 4713 B | 12.6 KB | 12.5 KB | 12.5 KB | 12.5 KB | 12.5 KB | 12.5 KB |
| `ls -p tests` | 118 B | 1469 B | 1309 B | 1309 B | 1309 B | 1309 B | 1309 B |
| `find .` | 57.3 KB | 82.8 KB | 21.4 KB | 21.4 KB | 21.4 KB | 21.4 KB | 21.4 KB |
| `find . -name '*.json'` | 12.4 KB | 19.3 KB | 19.1 KB | 19.1 KB | 19.1 KB | 19.1 KB | 19.1 KB |
| `find . -name '*.c'` | 12.3 KB | 15.2 KB | 15.0 KB | 15.0 KB | 15.0 KB | 15.0 KB | 15.0 KB |
| `find . -type d` | 2349 B | 5217 B | 5057 B | 5057 B | 5057 B | 5057 B | 5057 B |
| `find . -type f` | 54.9 KB | 103.5 KB | 26.7 KB | 26.7 KB | 26.7 KB | 26.7 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 4257 B | 7803 B | 7643 B | 7643 B | 7643 B | 7643 B | 7643 B |
| `find tests -name '*.json' -type f` | 0 B | 1830 B | 1926 B | 1926 B | 1926 B | 1926 B | 1926 B |
| `find . -name 'Makefile'` | 462 B | 2076 B | 1916 B | 1916 B | 1916 B | 1916 B | 1916 B |
| `cat src/commands/xinfo-stream.json` | 17.8 KB | 18.6 KB | 1018 B | 1018 B | 1018 B | 1018 B | 1018 B |
| `cat -n src/commands/xinfo-stream.json` | 20.3 KB | 33.7 KB | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `head -n 20 src/commands/xinfo-stream.json` | 640 B | 1694 B | 1534 B | 1534 B | 1534 B | 1534 B | 1534 B |
| `tail -n 20 src/commands/xinfo-stream.json` | 568 B | 1622 B | 1462 B | 1462 B | 1462 B | 1462 B | 1462 B |
| `tail -n +100 src/commands/xinfo-stream.json` | 13.9 KB | 15.0 KB | 14.8 KB | 14.8 KB | 14.8 KB | 14.8 KB | 14.8 KB |
| `wc src/commands/xinfo-stream.json` | 49 B | 954 B | 1026 B | 1026 B | 1026 B | 1026 B | 1026 B |
| `wc -l src/commands/*.json` | 14.4 KB | 71.0 KB | 71.0 KB | 71.0 KB | 71.0 KB | 71.0 KB | 71.0 KB |
| `cat src/commands/*.json \| wc -l` | 6 B | 1312.9 KB | 71.2 KB | 71.2 KB | 71.2 KB | 71.2 KB | 71.2 KB |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 261 B | 21.1 KB | 4591 B | 4591 B | 4591 B | 4591 B | 4591 B |
| `grep -c for src/commands/xinfo-stream.json` | 2 B | 3593 B | 3377 B | 5818 B | 5818 B | 5818 B | 5818 B |
| `grep -n for src/commands/xinfo-stream.json` | 330 B | 4065 B | 3849 B | 5818 B | 5818 B | 5818 B | 5818 B |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17.8 KB | 22.2 KB | 22.1 KB | 22.2 KB | 22.2 KB | 22.2 KB | 22.2 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17.8 KB | 23.8 KB | 23.7 KB | 23.8 KB | 23.8 KB | 23.8 KB | 23.8 KB |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 6 B | 41.5 KB | 24.1 KB | 24.4 KB | 24.4 KB | 24.4 KB | 24.4 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rn nestedcallreplyarray .` | 165 B | 15.1 KB | 14.9 KB | 26.1 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rl nestedcallreplyarray .` | 25 B | 15.1 KB | 14.9 KB | 26.1 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rn xmallctlmibnametomib .` | 206 B | 15.4 KB | 15.1 KB | 26.1 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rl xmallctlmibnametomib .` | 76 B | 15.2 KB | 15.0 KB | 26.1 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rn implementing .` | 2538 B | 12.8 KB | 10.9 KB | 16.4 KB | 17.1 KB | 25.9 KB | 25.9 KB |
| `grep -rl implementing .` | 371 B | 9894 B | 9434 B | 16.4 KB | 17.1 KB | 25.9 KB | 25.9 KB |
| `grep -rn between .` | 45.4 KB | 54.8 KB | 9081 B | 10.4 KB | 10.6 KB | 23.7 KB | 23.7 KB |
| `grep -rl between .` | 4657 B | 11.3 KB | 6219 B | 10.4 KB | 10.6 KB | 23.4 KB | 23.4 KB |
| `grep -rn for .` | 1815.1 KB | 3083.9 KB | 27.7 KB | 25.5 KB | 25.5 KB | 25.5 KB | 25.5 KB |
| `grep -rl for .` | 35.6 KB | 69.2 KB | 3799 B | 5758 B | 5758 B | 5758 B | 5758 B |
| `grep -rc implementing .` | 58.3 KB | 111.9 KB | 35.0 KB | 32.2 KB | 32.7 KB | 45.1 KB | 45.1 KB |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 206 B | 15.4 KB | 15.1 KB | 26.3 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -ri implementing .` | 2754 B | 12.8 KB | 10.9 KB | 16.4 KB | 17.1 KB | 25.9 KB | 25.9 KB |
| `grep -rn implementing tests` | 200 B | 9669 B | 9453 B | 16.4 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `grep -rl between tests` | 722 B | 7058 B | 6026 B | 10.4 KB | 10.6 KB | 10.6 KB | 10.6 KB |
| `grep -rF 'implementing' .` | 2435 B | 4638 B | 1926 B | 2514 B | 3026 B | 25.9 KB | 25.9 KB |
| `grep -rE 'arena_[a-z]+\(' .` | 24.7 KB | 34.1 KB | 13.9 KB | 19.5 KB | 19.6 KB | 27.5 KB | 27.5 KB |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 355 B | 28.7 KB | 28.4 KB | 50.7 KB | 53.1 KB | 53.1 KB | 53.1 KB |
| `grep -rE 'between\|implementing' .` | 46.2 KB | 62.2 KB | 17.2 KB | 25.4 KB | 26.3 KB | 26.3 KB | 26.3 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 888.7 KB | 31.5 KB | 33.9 KB | 33.9 KB | 33.9 KB | 33.9 KB |
| `grep -rn '^#include' .` | 172.4 KB | 253.0 KB | 14.9 KB | 15.9 KB | 15.8 KB | 19.6 KB | 19.6 KB |
| `grep -rE '^(import\|from) ' .` | 8992 B | 19.3 KB | 10.2 KB | 20.9 KB | 21.0 KB | 21.0 KB | 21.0 KB |
| `grep -rin 'return [a-z_]*;$' tests` | 53.9 KB | 106.3 KB | 13.9 KB | 20.6 KB | 20.9 KB | 25.0 KB | 25.0 KB |
| `grep -rv e .github` | 14.9 KB | 20.8 KB | 11.4 KB | 10.8 KB | 10.8 KB | 10.8 KB | 10.8 KB |
| `grep -r '\(implementing\).*\1' .` | 0 B | 6012 B | 5796 B | 10.4 KB | 10.9 KB | 30.1 KB | 30.1 KB |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 0 B | | | | | | 71.3 KB |
| `grep -l implementing src/commands/*.json` | 0 B | | | | | | 71.3 KB |
| `grep -c qdbzzabsentterm src/commands/*.json` | 12.4 KB | | | | | | 71.3 KB |
| `grep -rl between . \| wc -l` | 4 B | 11.9 KB | 6980 B | 11.1 KB | 11.4 KB | 24.1 KB | 24.1 KB |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 5 B | 180.1 KB | 7969 B | 9019 B | 8971 B | 8971 B | 8971 B |
| `find . -name '*.json' \| wc -l` | 4 B | 19.9 KB | 19.9 KB | 19.9 KB | 19.9 KB | 19.9 KB | 19.9 KB |
| `find . -type f \| grep -c json` | 4 B | 107.1 KB | 30.7 KB | 34.3 KB | 34.5 KB | 34.5 KB | 34.5 KB |
| `grep -rn for . \| head -n 5` | 502 B | 3084.6 KB | 4870 B | 6624 B | 6624 B | 6624 B | 6624 B |
| `grep -rn between . \| head -n 1` | 191 B | 55.6 KB | 7155 B | 11.2 KB | 11.5 KB | 11.5 KB | 11.5 KB |
| `find . -type f \| head -n 3` | 82 B | 104.2 KB | 27.6 KB | 27.6 KB | 27.6 KB | 27.6 KB | 27.6 KB |
| `cat src/commands/*.json \| head -n 2` | 15 B | 1313.0 KB | 71.3 KB | 71.3 KB | 71.3 KB | 71.3 KB | 71.3 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1619 B | 1459 B | 1459 B | 1459 B | 1459 B | 1459 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B | 979 B | 979 B | 979 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B | 1242 B | 1242 B | 1242 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B | 1246 B | 1246 B | 1246 B | 1246 B |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0 B | 18.9 KB | 18.7 KB | 18.7 KB | 18.7 KB | 19.5 KB | 19.5 KB |
| `cp -r tests bench_out/dir_copy` | 0 B | 3153.2 KB | 3153.0 KB | 3153.0 KB | 3169.4 KB | 3535.7 KB | 3535.7 KB |
| `cp -r src/commands bench_out/work` | 0 B | 787.8 KB | 787.6 KB | 787.6 KB | 787.6 KB | 993.0 KB | 993.0 KB |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 24.7 KB | 24.5 KB | 24.6 KB | 24.6 KB | 25.4 KB | 25.7 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 24.1 KB | 24.0 KB | 24.1 KB | 24.1 KB | 25.1 KB | 27.9 KB |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 0 B | 338.7 KB | 338.6 KB | 338.7 KB | 338.7 KB | 519.9 KB | 218.3 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | 0 B | | | | | | 82.9 KB |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | 0 B | | | | | | 86.5 KB |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 B | | | | | | 18.7 KB |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 B | | | | | | 18.7 KB |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | 0 B | | | | | | 19.0 KB |
| `grep -rc QDBBENCHMANY bench_out` | 25.7 KB | 46.3 KB | 27.7 KB | 16.4 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `grep -rl QDBBENCHMANY .` | 4346 B | 18.1 KB | 9490 B | 16.4 KB | 17.1 KB | 19.5 KB | 19.5 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 9127 B | 8911 B | 15.3 KB | 16.0 KB | 16.0 KB | 16.0 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B | 1191 B | 1191 B | 1191 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B | 1288 B | 1288 B | 1288 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B | 1089 B | 1089 B | 1089 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B | 997 B | 997 B | 997 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B | 945 B | 945 B | 945 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1070 B | 1129 B | 1129 B | 1129 B | 1129 B | 1129 B |
| `rm -rf bench_out` | 0 B | 3746 B | 3586 B | 3586 B | 3586 B | 3586 B | 3586 B |

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
