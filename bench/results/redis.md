# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (On disk, after the writes), and include starting bash and the tool (1.781 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 19.50 | 110.4 | 24.7 MB | 27.5 MB | 0.234 | yes |
| Stage 3, in memory | on | 145.3 | 255.5 | 27.3 MB | 28.2 MB | 0.310 | yes |
| On disk, settled | on | 184.4 | 184.4 | 426.3 KB | 25.0 MB | 0.288 | yes |
| On disk, after the writes | on | 184.4 | 184.4 | 426.3 KB | 25.0 MB | 0.288 | yes |

## The index (Stage 2, in memory)

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

## The index (Stage 3, in memory)

- Built in 110.2 ms while loading.
- Size: 5136.7 KB in memory, 29.6% of the text (2097.2 KB table, 3015.0 KB lists, 24.6 KB document table). The posting data itself is 0 B.
- 0 distinct trigrams, 0 postings, 1770 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

Index work per command that changes data:

| Command | Index work |
|---|---|
| `mkdir -p bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `cp -r tests bench_out/dir_copy` | documents +269 -0; postings +316501; list bytes rewritten 0; rebuilds 0 |
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
| `rm -r bench_out/dir_moved` | documents +0 -269; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -408; postings +0; list bytes rewritten 119814; rebuilds 0 |

## The index (On disk, settled)

- Built in 0.000 ms while loading.
- Size: 3008.2 KB in memory, 17.3% of the text (863.6 KB table, 2130.4 KB lists, 14.2 KB document table). The posting data itself is 2130.4 KB.
- 74777 distinct trigrams, 0 postings, 1770 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

Index work per command that changes data:

| Command | Index work |
|---|---|
| `mkdir -p bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | documents +1 -0; postings +9; list bytes rewritten 82; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | documents +1 -1; postings +19; list bytes rewritten 195; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | documents +1 -0; postings +878; list bytes rewritten 1713; rebuilds 0 |
| `cp -r tests bench_out/dir_copy` | documents +269 -0; postings +316501; list bytes rewritten 22781; rebuilds 0 |
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
| `rm -r bench_out/dir_moved` | documents +0 -269; postings +0; list bytes rewritten 465956; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -408; postings +0; list bytes rewritten 280084; rebuilds 0 |

## The index (On disk, after the writes)

- Built in 0.000 ms while loading.
- Size: 3008.2 KB in memory, 17.3% of the text (863.6 KB table, 2130.4 KB lists, 14.2 KB document table). The posting data itself is 2130.4 KB.
- 74777 distinct trigrams, 0 postings, 1770 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, in memory | On disk, settled | On disk, after the writes | Agree |
|---|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.665 |  | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `ls -a` | 2.543 |  | 0.002 | 0.002 | 0.002 | 0.003 | yes |
| `ls -R` | 3.847 |  | 0.043 | 0.035 | 0.049 | 0.033 | yes |
| `ls -aR tests` | 2.599 |  | 0.013 | 0.011 | 0.016 | 0.013 | yes |
| `ls -p tests` | 2.664 |  | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `find .` | 4.402 |  | 0.143 | 0.126 | 0.183 | 0.121 | yes |
| `find . -name '*.json'` | 4.620 |  | 0.227 | 0.154 | 0.238 | 0.170 | yes |
| `find . -name '*.c'` | 4.640 |  | 0.162 | 0.149 | 0.251 | 0.171 | yes |
| `find . -type d` | 4.270 |  | 0.127 | 0.126 | 0.168 | 0.192 | yes |
| `find . -type f` | 4.601 |  | 0.116 | 0.127 | 0.119 | 0.123 | yes |
| `find . -maxdepth 2 -type f` | 3.282 |  | 0.017 | 0.016 | 0.017 | 0.018 | yes |
| `find tests -name '*.json' -type f` | 2.790 |  | 0.022 | 0.023 | 0.026 | 0.026 | yes |
| `find . -name 'Makefile'` | 4.660 |  | 0.129 | 0.115 | 0.127 | 0.121 | yes |
| `cat src/commands/xinfo-stream.json` | 2.517 |  | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `cat -n src/commands/xinfo-stream.json` | 2.459 |  | 0.027 | 0.032 | 0.031 | 0.034 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 2.877 |  | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 2.853 |  | 0.002 | 0.002 | 0.002 | 0.003 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 2.827 |  | 0.004 | 0.004 | 0.004 | 0.006 | yes |
| `wc src/commands/xinfo-stream.json` | 3.010 |  | 0.014 | 0.014 | 0.014 | 0.013 | yes |
| `wc -l src/commands/*.json` | 6.000 |  | 1.148 | 0.267 | 0.225 | 0.265 | yes |
| `cat src/commands/*.json \| wc -l` | 12.07 |  | 1.115 | 1.046 | 1.100 | 1.221 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 3.787 |  | 0.006 | 0.006 | 0.006 | 0.007 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 2.949 |  | 0.004 | 0.004 | 0.004 | 0.004 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.983 |  | 0.004 | 0.007 | 0.004 | 0.004 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 3.115 |  | 0.190 | 0.279 | 0.196 | 0.190 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 2.942 |  | 0.292 | 0.341 | 0.242 | 0.274 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.837 |  | 0.065 | 0.134 | 0.077 | 0.077 | yes |
| `grep -rn qdbzzabsentterm .` | 24.26 | 15.18 | 0.008 | 0.010 | 0.010 | 0.012 | yes |
| `grep -rl qdbzzabsentterm .` | 15.67 | 12.57 | 0.009 | 0.011 | 0.018 | 0.012 | yes |
| `grep -rn nestedcallreplyarray .` | 20.20 | 13.11 | 0.078 | 0.035 | 0.038 | 0.040 | yes |
| `grep -rl nestedcallreplyarray .` | 14.48 | 13.93 | 0.095 | 0.033 | 0.032 | 0.037 | yes |
| `grep -rn xmallctlmibnametomib .` | 19.67 | 14.26 | 0.023 | 0.025 | 0.048 | 0.028 | yes |
| `grep -rl xmallctlmibnametomib .` | 14.57 | 13.88 | 0.033 | 0.022 | 0.031 | 0.026 | yes |
| `grep -rn implementing .` | 21.17 | 17.44 | 0.807 | 0.605 | 0.834 | 0.777 | yes |
| `grep -rl implementing .` | 18.56 | 13.73 | 0.958 | 0.721 | 0.947 | 0.660 | yes |
| `grep -rn between .` | 28.32 | 15.30 | 0.859 | 0.721 | 0.660 | 0.618 | yes |
| `grep -rl between .` | 16.37 | 12.60 | 0.242 | 0.180 | 0.280 | 0.155 | yes |
| `grep -rn for .` | 40.11 | 25.78 | 3.332 | 3.756 | 3.684 | 4.123 | yes |
| `grep -rl for .` | 14.98 | 14.91 | 0.321 | 0.321 | 0.283 | 0.339 | yes |
| `grep -rc implementing .` | 17.38 | 14.03 | 0.910 | 0.701 | 0.841 | 0.659 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 21.16 | 15.76 | 0.026 | 0.033 | 0.032 | 0.028 | yes |
| `grep -ri implementing .` | 17.31 | 14.06 | 0.875 | 0.943 | 0.698 | 1.122 | yes |
| `grep -rn implementing tests` | 5.296 | 4.814 | 0.064 | 0.060 | 0.072 | 0.120 | yes |
| `grep -rl between tests` | 4.485 | 4.797 | 0.025 | 0.038 | 0.030 | 0.056 | yes |
| `grep -rF 'implementing' .` | 14.29 | 13.59 | 0.701 | 0.749 | 0.595 | 0.960 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 18.71 | 12.70 | 0.473 | 0.590 | 0.459 | 0.572 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 30.89 | 17.57 | 0.203 | 0.121 | 0.078 | 0.085 | yes |
| `grep -rE 'between\|implementing' .` | 39.20 | 16.54 | 1.703 | 1.161 | 1.150 | 1.897 | yes |
| `grep -rE '[0-9]{4,}' .` | 25.51 | 30.01 | 22.20 | 20.99 | 20.87 | 18.37 | yes |
| `grep -rn '^#include' .` | 27.68 | 16.90 | 1.348 | 1.129 | 1.167 | 1.169 | yes |
| `grep -rE '^(import\|from) ' .` | 22.85 | 15.45 | 2.596 | 2.217 | 2.421 | 2.477 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 9.239 | 6.821 | 0.723 | 0.669 | 0.598 | 0.747 | yes |
| `grep -rv e .github` | 2.676 | 4.633 | 0.037 | 0.036 | 0.039 | 0.043 | yes |
| `grep -r '\(implementing\).*\1' .` | 16.83 |  | 0.855 | 0.655 | 0.809 | 1.155 | yes |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 4.528 |  | | 0.161 | 0.165 | 0.177 | yes |
| `grep -l implementing src/commands/*.json` | 5.184 |  | | 0.216 | 0.304 | 0.203 | yes |
| `grep -c qdbzzabsentterm src/commands/*.json` | 5.136 |  | | 0.160 | 0.145 | 0.171 | yes |
| `grep -rl between . \| wc -l` | 17.84 |  | 0.244 | 0.176 | 0.174 | 0.155 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 28.64 |  | 1.997 | 2.141 | 2.592 | 1.971 | yes |
| `find . -name '*.json' \| wc -l` | 4.774 |  | 0.172 | 0.230 | 0.172 | 0.185 | yes |
| `find . -type f \| grep -c json` | 4.419 |  | 0.212 | 0.213 | 0.234 | 0.212 | yes |
| `grep -rn for . \| head -n 5` | 3.262 |  | 0.005 | 0.005 | 0.006 | 0.005 | order differs |
| `grep -rn between . \| head -n 1` | 4.808 |  | 0.007 | 0.011 | 0.009 | 0.007 | order differs |
| `find . -type f \| head -n 3` | 3.051 |  | 0.036 | 0.057 | 0.033 | 0.036 | order differs |
| `cat src/commands/*.json \| head -n 2` | 3.454 |  | 0.098 | 0.081 | 0.106 | 0.096 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, in memory | On disk, settled | On disk, after the writes |
|---|---:|---:|---:|---:|---:|
| `ls` | 288 B | 1681 B | 1681 B | 1681 B | 1681 B |
| `ls -a` | 338 B | 1849 B | 1849 B | 1849 B | 1849 B |
| `ls -R` | 25.0 KB | 38.2 KB | 38.2 KB | 38.2 KB | 38.2 KB |
| `ls -aR tests` | 4713 B | 12.5 KB | 12.5 KB | 12.5 KB | 12.5 KB |
| `ls -p tests` | 118 B | 1309 B | 1309 B | 1309 B | 1309 B |
| `find .` | 57.3 KB | 21.4 KB | 21.4 KB | 21.4 KB | 21.4 KB |
| `find . -name '*.json'` | 12.4 KB | 19.1 KB | 19.1 KB | 19.1 KB | 19.1 KB |
| `find . -name '*.c'` | 12.3 KB | 15.0 KB | 15.0 KB | 15.0 KB | 15.0 KB |
| `find . -type d` | 2349 B | 5057 B | 5057 B | 5057 B | 5057 B |
| `find . -type f` | 54.9 KB | 26.7 KB | 26.7 KB | 26.7 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 4257 B | 7643 B | 7643 B | 7643 B | 7643 B |
| `find tests -name '*.json' -type f` | 0 B | 1926 B | 1926 B | 1926 B | 1926 B |
| `find . -name 'Makefile'` | 462 B | 1916 B | 1916 B | 1916 B | 1916 B |
| `cat src/commands/xinfo-stream.json` | 17.8 KB | 1018 B | 1018 B | 1018 B | 1018 B |
| `cat -n src/commands/xinfo-stream.json` | 20.3 KB | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `head -n 20 src/commands/xinfo-stream.json` | 640 B | 1534 B | 1534 B | 1534 B | 1534 B |
| `tail -n 20 src/commands/xinfo-stream.json` | 568 B | 1462 B | 1462 B | 1462 B | 1462 B |
| `tail -n +100 src/commands/xinfo-stream.json` | 13.9 KB | 14.8 KB | 14.8 KB | 14.8 KB | 14.8 KB |
| `wc src/commands/xinfo-stream.json` | 49 B | 1026 B | 1026 B | 1026 B | 1026 B |
| `wc -l src/commands/*.json` | 14.4 KB | 71.0 KB | 71.0 KB | 71.0 KB | 71.0 KB |
| `cat src/commands/*.json \| wc -l` | 6 B | 71.2 KB | 71.2 KB | 71.2 KB | 71.2 KB |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 261 B | 4591 B | 4815 B | 4815 B | 4815 B |
| `grep -c for src/commands/xinfo-stream.json` | 2 B | 5818 B | 5818 B | 5818 B | 5818 B |
| `grep -n for src/commands/xinfo-stream.json` | 330 B | 5818 B | 5818 B | 5818 B | 5818 B |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17.8 KB | 22.2 KB | 22.2 KB | 22.2 KB | 22.2 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17.8 KB | 23.8 KB | 23.8 KB | 23.8 KB | 23.8 KB |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 6 B | 24.4 KB | 24.6 KB | 24.6 KB | 24.6 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rn nestedcallreplyarray .` | 165 B | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rl nestedcallreplyarray .` | 25 B | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rn xmallctlmibnametomib .` | 206 B | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rl xmallctlmibnametomib .` | 76 B | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rn implementing .` | 2538 B | 25.9 KB | 26.9 KB | 26.9 KB | 26.9 KB |
| `grep -rl implementing .` | 371 B | 25.9 KB | 26.9 KB | 26.9 KB | 26.9 KB |
| `grep -rn between .` | 45.4 KB | 23.7 KB | 25.7 KB | 25.7 KB | 26.5 KB |
| `grep -rl between .` | 4657 B | 23.4 KB | 25.3 KB | 25.3 KB | 26.2 KB |
| `grep -rn for .` | 1815.1 KB | 25.5 KB | 25.5 KB | 25.5 KB | 25.5 KB |
| `grep -rl for .` | 35.6 KB | 5758 B | 5758 B | 5758 B | 5758 B |
| `grep -rc implementing .` | 58.3 KB | 45.1 KB | 46.2 KB | 46.2 KB | 46.2 KB |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 206 B | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -ri implementing .` | 2754 B | 25.9 KB | 26.9 KB | 26.9 KB | 26.9 KB |
| `grep -rn implementing tests` | 200 B | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `grep -rl between tests` | 722 B | 10.6 KB | 10.6 KB | 10.6 KB | 10.6 KB |
| `grep -rF 'implementing' .` | 2435 B | 25.9 KB | 26.9 KB | 26.9 KB | 26.9 KB |
| `grep -rE 'arena_[a-z]+\(' .` | 24.7 KB | 27.5 KB | 28.0 KB | 28.0 KB | 28.0 KB |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 355 B | 53.1 KB | 53.1 KB | 53.1 KB | 53.1 KB |
| `grep -rE 'between\|implementing' .` | 46.2 KB | 26.3 KB | 26.3 KB | 26.3 KB | 26.3 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 33.9 KB | 33.9 KB | 33.9 KB | 33.9 KB |
| `grep -rn '^#include' .` | 172.4 KB | 19.6 KB | 20.6 KB | 23.2 KB | 20.5 KB |
| `grep -rE '^(import\|from) ' .` | 8992 B | 21.0 KB | 21.0 KB | 21.0 KB | 21.0 KB |
| `grep -rin 'return [a-z_]*;$' tests` | 53.9 KB | 25.0 KB | 24.5 KB | 27.6 KB | 27.6 KB |
| `grep -rv e .github` | 14.9 KB | 10.8 KB | 10.8 KB | 10.8 KB | 10.8 KB |
| `grep -r '\(implementing\).*\1' .` | 0 B | 30.1 KB | 31.1 KB | 31.1 KB | 31.1 KB |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 0 B | | 71.3 KB | 71.3 KB | 71.3 KB |
| `grep -l implementing src/commands/*.json` | 0 B | | 71.3 KB | 71.3 KB | 71.3 KB |
| `grep -c qdbzzabsentterm src/commands/*.json` | 12.4 KB | | 71.3 KB | 71.3 KB | 71.3 KB |
| `grep -rl between . \| wc -l` | 4 B | 24.1 KB | 26.2 KB | 26.2 KB | 27.0 KB |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 5 B | 8971 B | 9195 B | 9195 B | 9195 B |
| `find . -name '*.json' \| wc -l` | 4 B | 19.9 KB | 20.0 KB | 20.0 KB | 20.0 KB |
| `find . -type f \| grep -c json` | 4 B | 34.5 KB | 34.6 KB | 34.6 KB | 34.6 KB |
| `grep -rn for . \| head -n 5` | 502 B | 6624 B | 6736 B | 6736 B | 6736 B |
| `grep -rn between . \| head -n 1` | 191 B | 11.5 KB | 11.6 KB | 11.6 KB | 11.6 KB |
| `find . -type f \| head -n 3` | 82 B | 27.6 KB | 27.7 KB | 27.7 KB | 27.7 KB |
| `cat src/commands/*.json \| head -n 2` | 15 B | 71.3 KB | 71.3 KB | 71.3 KB | 71.3 KB |

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
