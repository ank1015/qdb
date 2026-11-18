# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 4, on disk, after the writes), and include starting bash and the tool (1.822 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 19.50 | 110.4 | 24.7 MB | 27.5 MB | 0.234 | yes |
| Stage 3, on disk, settled | on | 184.4 | 184.4 | 426.3 KB | 25.0 MB | 0.288 | yes |
| Stage 4, in memory | on | 33.20 | 141.1 | 27.3 MB | 29.6 MB | 0.336 | yes |
| Stage 4, on disk, settled | on | 214.5 | 214.5 | 440.5 KB | 26.6 MB | 0.413 | yes |
| Stage 4, on disk, after the writes | on | 214.5 | 214.5 | 440.5 KB | 26.6 MB | 0.413 | yes |

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

## The index (Stage 3, on disk, settled)

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

## The index (Stage 4, in memory)

- Built in 107.9 ms while loading.
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
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r tests bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/commands bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +886; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +867; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | documents +133 -0; postings +56822; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat src/commands/xinfo-stream.json \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `sort src/commands/xinfo-stream.json > bench_out/sorted.txt` | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `sort -u src/commands/xinfo-stream.json >> bench_out/sorted.txt` | documents +1 -1; postings +878; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find .github -name '*.json' \| sort \| xargs cat > bench_out/joined.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1d' bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat bench_out/both.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -137; postings +0; list bytes rewritten 0; rebuilds 0 |

## The index (Stage 4, on disk, settled)

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
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r tests bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/commands bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +886; list bytes rewritten 1713; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +867; list bytes rewritten 5464; rebuilds 0 |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | documents +133 -0; postings +56822; list bytes rewritten 11979; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat src/commands/xinfo-stream.json \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `sort src/commands/xinfo-stream.json > bench_out/sorted.txt` | documents +1 -0; postings +878; list bytes rewritten 0; rebuilds 0 |
| `sort -u src/commands/xinfo-stream.json >> bench_out/sorted.txt` | documents +1 -1; postings +878; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find .github -name '*.json' \| sort \| xargs cat > bench_out/joined.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1d' bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat bench_out/both.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -137; postings +0; list bytes rewritten 70160; rebuilds 0 |

## The index (Stage 4, on disk, after the writes)

- Built in 0.000 ms while loading.
- Size: 3008.2 KB in memory, 17.3% of the text (863.6 KB table, 2130.4 KB lists, 14.2 KB document table). The posting data itself is 2130.4 KB.
- 74777 distinct trigrams, 0 postings, 1770 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.955 |  | 0.001 | 0.001 | 0.002 | 0.004 | 0.002 | yes |
| `ls -a` | 2.702 |  | 0.002 | 0.002 | 0.003 | 0.005 | 0.005 | yes |
| `ls -R` | 4.085 |  | 0.043 | 0.049 | 0.124 | 0.181 | 0.197 | yes |
| `ls -aR tests` | 3.287 |  | 0.013 | 0.016 | 0.025 | 0.036 | 0.038 | yes |
| `ls -p tests` | 2.986 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.003 | yes |
| `find .` | 4.166 |  | 0.143 | 0.183 | 0.175 | 0.199 | 0.205 | yes |
| `find . -name '*.json'` | 3.902 |  | 0.227 | 0.238 | 0.197 | 0.175 | 0.187 | yes |
| `find . -name '*.c'` | 4.659 |  | 0.162 | 0.251 | 0.177 | 0.274 | 0.269 | yes |
| `find . -type d` | 3.499 |  | 0.127 | 0.168 | 0.128 | 0.200 | 0.190 | yes |
| `find . -type f` | 3.848 |  | 0.116 | 0.119 | 0.149 | 0.227 | 0.155 | yes |
| `find . -maxdepth 2 -type f` | 2.856 |  | 0.017 | 0.017 | 0.032 | 0.021 | 0.018 | yes |
| `find tests -name '*.json' -type f` | 2.863 |  | 0.022 | 0.026 | 0.030 | 0.045 | 0.029 | yes |
| `find . -name 'Makefile'` | 3.864 |  | 0.129 | 0.127 | 0.157 | 0.172 | 0.121 | yes |
| `cat src/commands/xinfo-stream.json` | 2.226 |  | 0.001 | 0.001 | 0.001 | 0.001 | 0.002 | yes |
| `cat -n src/commands/xinfo-stream.json` | 2.457 |  | 0.027 | 0.031 | 0.038 | 0.033 | 0.048 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 2.299 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.003 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 2.548 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.003 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 3.001 |  | 0.004 | 0.004 | 0.007 | 0.007 | 0.008 | yes |
| `wc src/commands/xinfo-stream.json` | 3.256 |  | 0.014 | 0.014 | 0.013 | 0.018 | 0.021 | yes |
| `wc -l src/commands/*.json` | 7.443 |  | 1.148 | 0.225 | 0.344 | 0.392 | 0.361 | yes |
| `cat src/commands/*.json \| wc -l` | 10.07 |  | 1.115 | 1.100 | 0.253 | 0.202 | 0.234 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 3.826 |  | 0.006 | 0.006 | 0.004 | 0.004 | 0.004 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 3.070 |  | 0.004 | 0.004 | 0.007 | 0.005 | 0.005 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.579 |  | 0.004 | 0.004 | 0.008 | 0.006 | 0.006 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 3.157 |  | 0.190 | 0.196 | 0.011 | 0.008 | 0.010 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 3.247 |  | 0.292 | 0.242 | 0.063 | 0.035 | 0.070 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.556 |  | 0.065 | 0.077 | 0.047 | 0.047 | 0.049 | yes |
| `grep -rn qdbzzabsentterm .` | 26.81 | 17.43 | 0.008 | 0.010 | 0.013 | 0.014 | 0.021 | yes |
| `grep -rl qdbzzabsentterm .` | 18.31 | 18.54 | 0.009 | 0.018 | 0.012 | 0.018 | 0.018 | yes |
| `grep -rn nestedcallreplyarray .` | 25.48 | 20.07 | 0.078 | 0.038 | 0.034 | 0.041 | 0.038 | yes |
| `grep -rl nestedcallreplyarray .` | 18.83 | 17.87 | 0.095 | 0.032 | 0.038 | 0.035 | 0.039 | yes |
| `grep -rn xmallctlmibnametomib .` | 23.60 | 19.32 | 0.023 | 0.048 | 0.040 | 0.027 | 0.036 | yes |
| `grep -rl xmallctlmibnametomib .` | 17.99 | 19.87 | 0.033 | 0.031 | 0.038 | 0.040 | 0.042 | yes |
| `grep -rn implementing .` | 27.99 | 15.88 | 0.807 | 0.834 | 0.676 | 0.756 | 1.028 | yes |
| `grep -rl implementing .` | 18.72 | 16.90 | 0.958 | 0.947 | 0.879 | 0.877 | 0.943 | yes |
| `grep -rn between .` | 30.57 | 19.28 | 0.859 | 0.660 | 1.006 | 0.791 | 0.846 | yes |
| `grep -rl between .` | 20.53 | 17.52 | 0.242 | 0.280 | 0.270 | 0.205 | 0.233 | yes |
| `grep -rn for .` | 45.90 | 26.99 | 3.332 | 3.684 | 4.830 | 4.468 | 5.305 | yes |
| `grep -rl for .` | 13.79 | 16.98 | 0.321 | 0.283 | 0.378 | 0.343 | 0.472 | yes |
| `grep -rc implementing .` | 18.02 | 14.94 | 0.910 | 0.841 | 0.861 | 1.112 | 0.840 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 27.66 | 25.12 | 0.026 | 0.032 | 0.048 | 0.041 | 0.034 | yes |
| `grep -ri implementing .` | 21.69 | 19.30 | 0.875 | 0.698 | 0.723 | 0.734 | 0.966 | yes |
| `grep -rn implementing tests` | 6.616 | 5.925 | 0.064 | 0.072 | 0.077 | 0.077 | 0.091 | yes |
| `grep -rl between tests` | 5.317 | 6.150 | 0.025 | 0.030 | 0.032 | 0.042 | 0.042 | yes |
| `grep -rF 'implementing' .` | 18.68 | 15.29 | 0.701 | 0.595 | 0.683 | 0.947 | 0.777 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 20.51 | 19.09 | 0.473 | 0.459 | 0.533 | 0.448 | 0.395 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 28.63 | 18.23 | 0.203 | 0.078 | 0.084 | 0.160 | 0.153 | yes |
| `grep -rE 'between\|implementing' .` | 40.21 | 20.28 | 1.703 | 1.150 | 1.205 | 1.283 | 2.037 | yes |
| `grep -rE '[0-9]{4,}' .` | 36.49 | 34.46 | 22.20 | 20.87 | 18.47 | 21.23 | 21.60 | yes |
| `grep -rn '^#include' .` | 28.06 | 18.91 | 1.348 | 1.167 | 1.321 | 1.110 | 1.617 | yes |
| `grep -rE '^(import\|from) ' .` | 30.01 | 19.54 | 2.596 | 2.421 | 2.241 | 3.165 | 2.659 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 8.660 | 6.968 | 0.723 | 0.598 | 0.716 | 0.680 | 0.720 | yes |
| `grep -rv e .github` | 2.817 | 3.781 | 0.037 | 0.039 | 0.043 | 0.052 | 0.070 | yes |
| `grep -r '\(implementing\).*\1' .` | 17.55 |  | 0.855 | 0.809 | 1.215 | 0.774 | 1.070 | yes |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 5.308 |  | | 0.165 | 0.278 | 0.257 | 0.182 | yes |
| `grep -l implementing src/commands/*.json` | 4.714 |  | | 0.304 | 0.273 | 0.282 | 0.218 | yes |
| `grep -c qdbzzabsentterm src/commands/*.json` | 5.262 |  | | 0.145 | 0.180 | 0.284 | 0.222 | yes |
| `grep -rl between . \| wc -l` | 19.63 |  | 0.244 | 0.174 | 0.186 | 0.353 | 0.266 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 32.79 |  | 1.997 | 2.592 | 2.626 | 2.096 | 2.885 | yes |
| `find . -name '*.json' \| wc -l` | 6.259 |  | 0.172 | 0.172 | 0.191 | 0.300 | 0.178 | yes |
| `find . -type f \| grep -c json` | 5.394 |  | 0.212 | 0.234 | 0.211 | 0.391 | 0.356 | yes |
| `grep -rn for . \| head -n 5` | 3.648 |  | 0.005 | 0.006 | 0.007 | 0.010 | 0.007 | order differs |
| `grep -rn between . \| head -n 1` | 5.724 |  | 0.007 | 0.009 | 0.009 | 0.014 | 0.015 | order differs |
| `find . -type f \| head -n 3` | 4.069 |  | 0.036 | 0.033 | 0.041 | 0.037 | 0.059 | order differs |
| `cat src/commands/*.json \| head -n 2` | 3.913 |  | 0.098 | 0.106 | 0.097 | 0.088 | 0.096 | yes |
| `grep -rno implementing .` | 28.01 | 20.06 | | | 0.798 | 0.836 | 0.831 | yes |
| `grep -rw implementing .` | 18.15 | 19.38 | | | 1.197 | 0.944 | 0.722 | yes |
| `grep -rnw xmallctlmibnametomib .` | 26.56 | 15.55 | | | 0.076 | 0.104 | 0.110 | yes |
| `grep -rn -C 2 xmallctlmibnametomib .` | 25.11 | 33.45 | | | 0.043 | 0.031 | 0.044 | yes |
| `grep -rn -A 1 -B 1 implementing .` | 28.20 | 47.71 | | | 1.122 | 0.799 | 1.034 | yes |
| `grep -rho 'arena_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 25.23 |  | | | 2.842 | 2.913 | 3.013 | yes |
| `grep -rhow '[a-z]*xmallctlmibnametomib[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 17.75 |  | | | 0.122 | 0.201 | 0.192 | yes |
| `rg -j 1 qdbzzabsentterm` | 20.67 |  | | | 0.038 | 0.059 | 0.044 | yes |
| `rg -j 1 nestedcallreplyarray` | 23.25 |  | | | 0.102 | 0.103 | 0.067 | yes |
| `rg -j 1 xmallctlmibnametomib` | 20.13 |  | | | 0.210 | 0.213 | 0.120 | yes |
| `rg -j 1 implementing` | 24.34 |  | | | 1.458 | 1.478 | 0.941 | yes |
| `rg -j 1 between` | 24.40 |  | | | 0.936 | 1.269 | 1.137 | yes |
| `rg -j 1 for` | 35.13 |  | | | 3.968 | 5.572 | 4.267 | yes |
| `rg -j 1 -l implementing` | 24.54 |  | | | 0.844 | 1.319 | 0.861 | yes |
| `rg -j 1 -l for` | 25.09 |  | | | 1.776 | 2.390 | 1.688 | yes |
| `rg -j 1 -c between` | 25.89 |  | | | 0.955 | 0.818 | 1.008 | yes |
| `rg -j 1 -i XMALLCTLMIBNAMETOMIB` | 28.85 |  | | | 0.351 | 0.349 | 0.515 | yes |
| `rg -j 1 -w implementing` | 25.54 |  | | | 1.784 | 1.746 | 3.288 | yes |
| `rg -j 1 -F 'implementing'` | 26.11 |  | | | 0.949 | 0.940 | 1.371 | yes |
| `rg -j 1 -n -C 2 xmallctlmibnametomib` | 20.16 |  | | | 0.142 | 0.158 | 0.184 | yes |
| `rg -j 1 'arena_[a-z]+\('` | 20.26 |  | | | 0.749 | 0.706 | 1.036 | yes |
| `rg -j 1 'xmallctlmibnametomib\|nestedcallreplyarray'` | 23.08 |  | | | 0.204 | 0.212 | 0.278 | yes |
| `rg -j 1 '\bxmallctlmibnametomib\w*'` | 26.41 |  | | | 2.819 | 2.464 | 4.025 | yes |
| `rg -j 1 -t c implementing` | 20.51 |  | | | 0.907 | 0.844 | 1.088 | yes |
| `rg -j 1 -g '*.json' implementing` | 14.83 |  | | | 0.786 | 0.739 | 1.307 | yes |
| `rg -j 1 implementing tests` | 9.857 |  | | | 0.272 | 0.285 | 0.418 | yes |
| `rg -j 1 -uu implementing` | 19.88 |  | | | 0.940 | 0.747 | 1.106 | yes |
| `rg -j 1 --sort path xmallctlmibnametomib` | 26.24 |  | | | 0.135 | 0.125 | 0.165 | yes |
| `rg -j 1 --files \| wc -l` | 13.22 |  | | | 0.859 | 1.107 | 1.547 | yes |
| `rg -j 1 -l between \| wc -l` | 24.59 |  | | | 0.471 | 0.384 | 0.775 | yes |
| `ls -ltR` | 11.86 |  | | | 0.931 | 1.334 | 1.418 | yes |
| `ls -lt src/commands` | 4.841 |  | | | 0.175 | 0.263 | 0.278 | yes |
| `ls -ltr tests \| tail -n 3` | 3.530 |  | | | 0.009 | 0.012 | 0.011 | yes |
| `find . -newer src/commands/xinfo-stream.json` | 7.025 |  | | | 0.210 | 0.210 | 0.133 | yes |
| `find . -newer src/commands/xinfo-stream.json -name '*.json'` | 5.953 |  | | | 0.152 | 0.234 | 0.152 | yes |
| `find . -type f -mtime +30` | 7.375 |  | | | 0.233 | 0.136 | 0.229 | yes |
| `du -s .` | 6.784 |  | | | 0.001 | 0.001 | 0.002 | not compared |
| `du -sh tests` | 3.890 |  | | | 0.002 | 0.002 | 0.001 | not compared |
| `du -sb .` | 6.862 |  | | | 0.001 | 0.002 | 0.002 | yes |
| `du -sb tests` | 3.752 |  | | | 0.002 | 0.002 | 0.002 | yes |
| `du -b -d 1 tests` | 3.843 |  | | | 0.002 | 0.004 | 0.003 | yes |
| `sort src/commands/xinfo-stream.json` | 3.924 |  | | | 0.029 | 0.044 | 0.032 | yes |
| `sort -r src/commands/xinfo-stream.json \| head -n 3` | 4.304 |  | | | 0.031 | 0.055 | 0.034 | yes |
| `sort -u src/commands/xinfo-stream.json \| wc -l` | 4.088 |  | | | 0.034 | 0.048 | 0.052 | yes |
| `sort -k 2 src/commands/xinfo-stream.json \| tail -n 3` | 3.580 |  | | | 0.049 | 0.068 | 0.049 | yes |
| `cat src/commands/*.json \| sort \| uniq -c \| sort -rn \| head -n 5` | 18.82 |  | | | 4.757 | 6.187 | 4.626 | yes |
| `uniq -c src/commands/xinfo-stream.json \| wc -l` | 3.974 |  | | | 0.028 | 0.031 | 0.019 | yes |
| `cut -c 1-20 src/commands/xinfo-stream.json` | 3.592 |  | | | 0.021 | 0.022 | 0.022 | yes |
| `cut -d ' ' -f 1 src/commands/xinfo-stream.json \| sort \| uniq \| wc -l` | 4.679 |  | | | 0.074 | 0.076 | 0.060 | yes |
| `tr a-z A-Z < src/commands/xinfo-stream.json` | 3.383 |  | | | 0.004 | 0.004 | 0.004 | yes |
| `tr -d '\n' < src/commands/xinfo-stream.json \| wc -c` | 3.942 |  | | | 0.009 | 0.014 | 0.017 | yes |
| `tr -s ' ' < src/commands/xinfo-stream.json \| wc -c` | 3.968 |  | | | 0.015 | 0.013 | 0.023 | yes |
| `nl src/commands/xinfo-stream.json` | 2.922 |  | | | 0.029 | 0.033 | 0.057 | yes |
| `nl -ba src/commands/xinfo-stream.json \| tail -n 3` | 3.440 |  | | | 0.046 | 0.058 | 0.061 | yes |
| `seq 1 100000 \| wc -l` | 5.093 |  | | | 2.104 | 3.980 | 4.115 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.830 |  | | | 0.025 | 0.039 | 0.048 | yes |
| `basename src/commands/xinfo-stream.json` | 2.470 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `basename -s .json src/commands/xinfo-stream.json` | 2.413 |  | | | 0.002 | 0.002 | 0.003 | yes |
| `dirname src/commands/xinfo-stream.json` | 2.376 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `printf '%s\n' a b c` | 1.531 |  | | | 0.002 | 0.002 | 0.004 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 2.202 |  | | | 0.002 | 0.002 | 0.004 | yes |
| `test -f src/commands/xinfo-stream.json && echo yes` | 1.681 |  | | | 0.002 | 0.003 | 0.004 | yes |
| `[ -d tests ] && echo folder` | 1.623 |  | | | 0.003 | 0.002 | 0.003 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 3.355 |  | | | 0.004 | 0.002 | 0.003 | yes |
| `echo a b c \| xargs -n 1 echo` | 7.315 |  | | | 0.003 | 0.002 | 0.004 | yes |
| `find .github -name '*.json' \| xargs cat \| wc -l` | 4.965 |  | | | 0.005 | 0.008 | 0.004 | yes |
| `find . -name '*.json' \| xargs grep -l implementing \| wc -l` | 8.977 |  | | | 0.394 | 0.495 | 0.829 | yes |
| `echo {a,b,c}{1..3}` | 1.855 |  | | | 0.004 | 0.004 | 0.004 | yes |
| `echo {1..20..3} {a..e}` | 1.793 |  | | | 0.006 | 0.005 | 0.005 | yes |
| `echo $(wc -l < src/commands/xinfo-stream.json) `basename src/commands/xinfo-stream.json`` | 5.255 |  | | | 0.005 | 0.005 | 0.006 | yes |
| `echo "first line: $(head -n 1 src/commands/xinfo-stream.json)"` | 3.271 |  | | | 0.006 | 0.005 | 0.004 | yes |
| `wc -l < src/commands/xinfo-stream.json` | 3.096 |  | | | 0.003 | 0.002 | 0.002 | yes |
| `cat src/commands/xinfo-stream.json > /dev/null 2>&1` | 3.024 |  | | | 0.001 | 0.002 | 0.002 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 3.210 |  | | | 0.002 | 0.003 | 0.002 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 3.733 |  | | | 0.002 | 0.003 | 0.003 | yes |
| `grep -q for src/commands/xinfo-stream.json && echo found \|\| echo none` | 3.003 |  | | | 0.006 | 0.008 | 0.005 | yes |
| `false \|\| echo fallback` | 1.668 |  | | | 0.001 | 0.002 | 0.002 | yes |
| `true && grep -c for src/commands/xinfo-stream.json` | 3.460 |  | | | 0.005 | 0.005 | 0.006 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 3.809 |  | | | 0.003 | 0.003 | 0.003 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 3.057 |  | | | 0.002 | 0.001 | 0.001 | yes |
| `echo one \⏎two` | 1.563 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `grep -c for src/commands/xinfo-stream.json⏎wc -l src/commands/xinfo-stream.json` | 4.294 |  | | | 0.010 | 0.010 | 0.006 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes |
|---|---:|---:|---:|---:|---:|---:|
| `ls` | 288 B | 1681 B | 1681 B | 4107 B | 4107 B | 4107 B |
| `ls -a` | 338 B | 1849 B | 1849 B | 4497 B | 4497 B | 4497 B |
| `ls -R` | 25.0 KB | 38.2 KB | 38.2 KB | 69.1 KB | 69.1 KB | 69.1 KB |
| `ls -aR tests` | 4713 B | 12.5 KB | 12.5 KB | 17.7 KB | 17.7 KB | 17.7 KB |
| `ls -p tests` | 118 B | 1309 B | 1309 B | 3604 B | 3604 B | 3604 B |
| `find .` | 57.3 KB | 21.4 KB | 21.4 KB | 23.2 KB | 23.2 KB | 23.2 KB |
| `find . -name '*.json'` | 12.4 KB | 19.1 KB | 19.1 KB | 21.2 KB | 21.2 KB | 21.2 KB |
| `find . -name '*.c'` | 12.3 KB | 15.0 KB | 15.0 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `find . -type d` | 2349 B | 5057 B | 5057 B | 7196 B | 7196 B | 7196 B |
| `find . -type f` | 54.9 KB | 26.7 KB | 26.7 KB | 28.9 KB | 28.9 KB | 28.9 KB |
| `find . -maxdepth 2 -type f` | 4257 B | 7643 B | 7643 B | 10.2 KB | 10.2 KB | 10.2 KB |
| `find tests -name '*.json' -type f` | 0 B | 1926 B | 1926 B | 4042 B | 4042 B | 4042 B |
| `find . -name 'Makefile'` | 462 B | 1916 B | 1916 B | 3976 B | 3976 B | 3976 B |
| `cat src/commands/xinfo-stream.json` | 17.8 KB | 1018 B | 1018 B | 2423 B | 2423 B | 2423 B |
| `cat -n src/commands/xinfo-stream.json` | 20.3 KB | 17.1 KB | 17.1 KB | 35.6 KB | 35.6 KB | 35.6 KB |
| `head -n 20 src/commands/xinfo-stream.json` | 640 B | 1534 B | 1534 B | 3620 B | 3620 B | 3620 B |
| `tail -n 20 src/commands/xinfo-stream.json` | 568 B | 1462 B | 1462 B | 3548 B | 3548 B | 3548 B |
| `tail -n +100 src/commands/xinfo-stream.json` | 13.9 KB | 14.8 KB | 14.8 KB | 16.9 KB | 16.9 KB | 16.9 KB |
| `wc src/commands/xinfo-stream.json` | 49 B | 1026 B | 1026 B | 2618 B | 2618 B | 2618 B |
| `wc -l src/commands/*.json` | 14.4 KB | 71.0 KB | 71.0 KB | 71.7 KB | 71.7 KB | 71.7 KB |
| `cat src/commands/*.json \| wc -l` | 6 B | 71.2 KB | 71.2 KB | 72.1 KB | 72.1 KB | 72.1 KB |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 261 B | 4591 B | 4815 B | 7495 B | 7495 B | 7495 B |
| `grep -c for src/commands/xinfo-stream.json` | 2 B | 5818 B | 5818 B | 10.1 KB | 10.1 KB | 10.1 KB |
| `grep -n for src/commands/xinfo-stream.json` | 330 B | 5818 B | 5818 B | 10.1 KB | 10.1 KB | 10.1 KB |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17.8 KB | 22.2 KB | 22.2 KB | 29.0 KB | 29.0 KB | 29.0 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17.8 KB | 23.8 KB | 23.8 KB | 45.4 KB | 45.4 KB | 45.4 KB |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 6 B | 24.4 KB | 24.6 KB | 28.0 KB | 28.0 KB | 28.0 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB |
| `grep -rn nestedcallreplyarray .` | 165 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB |
| `grep -rl nestedcallreplyarray .` | 25 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB |
| `grep -rn xmallctlmibnametomib .` | 206 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB |
| `grep -rl xmallctlmibnametomib .` | 76 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB |
| `grep -rn implementing .` | 2538 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB |
| `grep -rl implementing .` | 371 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB |
| `grep -rn between .` | 45.4 KB | 23.7 KB | 25.7 KB | 27.9 KB | 27.9 KB | 27.9 KB |
| `grep -rl between .` | 4657 B | 23.4 KB | 25.3 KB | 27.5 KB | 27.5 KB | 27.5 KB |
| `grep -rn for .` | 1815.1 KB | 25.5 KB | 25.5 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rl for .` | 35.6 KB | 5758 B | 5758 B | 9977 B | 9977 B | 9977 B |
| `grep -rc implementing .` | 58.3 KB | 45.1 KB | 46.2 KB | 48.4 KB | 48.4 KB | 48.4 KB |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 206 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB |
| `grep -ri implementing .` | 2754 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB |
| `grep -rn implementing tests` | 200 B | 17.1 KB | 17.1 KB | 26.5 KB | 26.5 KB | 26.5 KB |
| `grep -rl between tests` | 722 B | 10.6 KB | 10.6 KB | 17.2 KB | 17.2 KB | 17.2 KB |
| `grep -rF 'implementing' .` | 2435 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB |
| `grep -rE 'arena_[a-z]+\(' .` | 24.7 KB | 27.5 KB | 28.0 KB | 30.2 KB | 30.2 KB | 30.2 KB |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 355 B | 53.1 KB | 53.1 KB | 78.8 KB | 78.8 KB | 78.8 KB |
| `grep -rE 'between\|implementing' .` | 46.2 KB | 26.3 KB | 26.3 KB | 39.9 KB | 39.9 KB | 39.9 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 33.9 KB | 33.9 KB | 36.1 KB | 36.1 KB | 36.1 KB |
| `grep -rn '^#include' .` | 172.4 KB | 19.6 KB | 23.2 KB | 23.0 KB | 25.4 KB | 23.0 KB |
| `grep -rE '^(import\|from) ' .` | 8992 B | 21.0 KB | 21.0 KB | 30.1 KB | 30.1 KB | 30.1 KB |
| `grep -rin 'return [a-z_]*;$' tests` | 53.9 KB | 25.0 KB | 27.6 KB | 26.7 KB | 29.9 KB | 26.7 KB |
| `grep -rv e .github` | 14.9 KB | 10.8 KB | 10.8 KB | 13.0 KB | 13.0 KB | 13.0 KB |
| `grep -r '\(implementing\).*\1' .` | 0 B | 30.1 KB | 31.1 KB | 33.3 KB | 33.3 KB | 33.3 KB |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 0 B | | 71.3 KB | 72.3 KB | 72.3 KB | 72.3 KB |
| `grep -l implementing src/commands/*.json` | 0 B | | 71.3 KB | 72.2 KB | 72.2 KB | 72.2 KB |
| `grep -c qdbzzabsentterm src/commands/*.json` | 12.4 KB | | 71.3 KB | 72.3 KB | 72.3 KB | 72.3 KB |
| `grep -rl between . \| wc -l` | 4 B | 24.1 KB | 26.2 KB | 28.5 KB | 28.5 KB | 28.5 KB |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 5 B | 8971 B | 9195 B | 13.5 KB | 13.5 KB | 13.5 KB |
| `find . -name '*.json' \| wc -l` | 4 B | 19.9 KB | 20.0 KB | 22.2 KB | 22.2 KB | 22.2 KB |
| `find . -type f \| grep -c json` | 4 B | 34.5 KB | 34.6 KB | 39.8 KB | 39.8 KB | 39.8 KB |
| `grep -rn for . \| head -n 5` | 502 B | 6624 B | 6736 B | 11.2 KB | 11.2 KB | 11.2 KB |
| `grep -rn between . \| head -n 1` | 191 B | 11.5 KB | 11.6 KB | 18.4 KB | 18.4 KB | 18.4 KB |
| `find . -type f \| head -n 3` | 82 B | 27.6 KB | 27.7 KB | 30.1 KB | 30.1 KB | 30.1 KB |
| `cat src/commands/*.json \| head -n 2` | 15 B | 71.3 KB | 71.3 KB | 72.3 KB | 72.3 KB | 72.3 KB |
| `grep -rno implementing .` | 961 B | | | 37.3 KB | 37.3 KB | 37.3 KB |
| `grep -rw implementing .` | 2435 B | | | 41.0 KB | 41.0 KB | 41.0 KB |
| `grep -rnw xmallctlmibnametomib .` | 206 B | | | 49.2 KB | 49.2 KB | 49.2 KB |
| `grep -rn -C 2 xmallctlmibnametomib .` | 768 B | | | 42.6 KB | 42.6 KB | 42.6 KB |
| `grep -rn -A 1 -B 1 implementing .` | 6398 B | | | 30.4 KB | 30.4 KB | 30.4 KB |
| `grep -rho 'arena_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 210 B | | | 211.6 KB | 211.6 KB | 211.6 KB |
| `grep -rhow '[a-z]*xmallctlmibnametomib[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 29 B | | | 74.1 KB | 74.1 KB | 74.1 KB |
| `rg -j 1 qdbzzabsentterm` | 0 B | | | 30.8 KB | 30.8 KB | 30.8 KB |
| `rg -j 1 nestedcallreplyarray` | 152 B | | | 40.8 KB | 40.8 KB | 40.8 KB |
| `rg -j 1 xmallctlmibnametomib` | 195 B | | | 101.7 KB | 101.7 KB | 101.7 KB |
| `rg -j 1 implementing` | 2383 B | | | 117.6 KB | 117.6 KB | 117.6 KB |
| `rg -j 1 between` | 42.7 KB | | | 139.3 KB | 139.3 KB | 139.3 KB |
| `rg -j 1 for` | 1694.2 KB | | | 145.0 KB | 145.0 KB | 145.0 KB |
| `rg -j 1 -l implementing` | 337 B | | | 115.3 KB | 115.3 KB | 115.3 KB |
| `rg -j 1 -l for` | 32.9 KB | | | 144.7 KB | 144.7 KB | 144.7 KB |
| `rg -j 1 -c between` | 4622 B | | | 119.0 KB | 119.0 KB | 119.0 KB |
| `rg -j 1 -i XMALLCTLMIBNAMETOMIB` | 195 B | | | 124.7 KB | 124.7 KB | 124.7 KB |
| `rg -j 1 -w implementing` | 2383 B | | | 141.4 KB | 141.4 KB | 141.4 KB |
| `rg -j 1 -F 'implementing'` | 2383 B | | | 118.1 KB | 118.1 KB | 118.1 KB |
| `rg -j 1 -n -C 2 xmallctlmibnametomib` | 748 B | | | 103.7 KB | 103.7 KB | 103.7 KB |
| `rg -j 1 'arena_[a-z]+\('` | 24.2 KB | | | 137.5 KB | 137.5 KB | 137.5 KB |
| `rg -j 1 'xmallctlmibnametomib\|nestedcallreplyarray'` | 347 B | | | 116.0 KB | 116.0 KB | 116.0 KB |
| `rg -j 1 '\bxmallctlmibnametomib\w*'` | 195 B | | | 2622.7 KB | 2622.7 KB | 2622.7 KB |
| `rg -j 1 -t c implementing` | 2288 B | | | 119.6 KB | 119.6 KB | 119.6 KB |
| `rg -j 1 -g '*.json' implementing` | 0 B | | | 115.3 KB | 115.3 KB | 115.3 KB |
| `rg -j 1 implementing tests` | 195 B | | | 32.4 KB | 32.4 KB | 32.4 KB |
| `rg -j 1 -uu implementing` | 2383 B | | | 36.9 KB | 36.9 KB | 36.9 KB |
| `rg -j 1 --sort path xmallctlmibnametomib` | 195 B | | | 102.4 KB | 102.4 KB | 102.4 KB |
| `rg -j 1 --files \| wc -l` | 5 B | | | 143.0 KB | 143.0 KB | 143.0 KB |
| `rg -j 1 -l between \| wc -l` | 4 B | | | 120.0 KB | 120.0 KB | 120.0 KB |
| `ls -ltR` | 94.8 KB | | | 78.9 KB | 78.9 KB | 78.9 KB |
| `ls -lt src/commands` | 22.6 KB | | | 54.5 KB | 54.5 KB | 54.5 KB |
| `ls -ltr tests \| tail -n 3` | 145 B | | | 5556 B | 5556 B | 5556 B |
| `find . -newer src/commands/xinfo-stream.json` | 13.7 KB | | | 27.9 KB | 27.9 KB | 27.9 KB |
| `find . -newer src/commands/xinfo-stream.json -name '*.json'` | 0 B | | | 4460 B | 4460 B | 4460 B |
| `find . -type f -mtime +30` | 0 B | | | 4081 B | 4081 B | 4081 B |
| `du -s .` | 8 B | | | 2695 B | 2695 B | 2695 B |
| `du -sh tests` | 11 B | | | 2724 B | 2724 B | 2724 B |
| `du -sb .` | 11 B | | | 2707 B | 2707 B | 2707 B |
| `du -sb tests` | 14 B | | | 2725 B | 2725 B | 2725 B |
| `du -b -d 1 tests` | 191 B | | | 3873 B | 3873 B | 3873 B |
| `sort src/commands/xinfo-stream.json` | 17.8 KB | | | 99.3 KB | 99.3 KB | 99.3 KB |
| `sort -r src/commands/xinfo-stream.json \| head -n 3` | 10 B | | | 100.8 KB | 100.8 KB | 100.8 KB |
| `sort -u src/commands/xinfo-stream.json \| wc -l` | 4 B | | | 111.3 KB | 111.3 KB | 111.3 KB |
| `sort -k 2 src/commands/xinfo-stream.json \| tail -n 3` | 73 B | | | 118.7 KB | 118.7 KB | 118.7 KB |
| `cat src/commands/*.json \| sort \| uniq -c \| sort -rn \| head -n 5` | 116 B | | | 2316.4 KB | 2316.4 KB | 2316.4 KB |
| `uniq -c src/commands/xinfo-stream.json \| wc -l` | 4 B | | | 36.9 KB | 36.9 KB | 36.9 KB |
| `cut -c 1-20 src/commands/xinfo-stream.json` | 7374 B | | | 19.3 KB | 19.3 KB | 19.3 KB |
| `cut -d ' ' -f 1 src/commands/xinfo-stream.json \| sort \| uniq \| wc -l` | 2 B | | | 84.4 KB | 84.4 KB | 84.4 KB |
| `tr a-z A-Z < src/commands/xinfo-stream.json` | 17.8 KB | | | 22.8 KB | 22.8 KB | 22.8 KB |
| `tr -d '\n' < src/commands/xinfo-stream.json \| wc -c` | 6 B | | | 23.8 KB | 23.8 KB | 23.8 KB |
| `tr -s ' ' < src/commands/xinfo-stream.json \| wc -c` | 5 B | | | 30.3 KB | 30.3 KB | 30.3 KB |
| `nl src/commands/xinfo-stream.json` | 20.3 KB | | | 35.4 KB | 35.4 KB | 35.4 KB |
| `nl -ba src/commands/xinfo-stream.json \| tail -n 3` | 39 B | | | 57.2 KB | 57.2 KB | 57.2 KB |
| `seq 1 100000 \| wc -l` | 7 B | | | 135.0 KB | 135.0 KB | 135.0 KB |
| `seq -w 1 1000 \| tail -n 2` | 10 B | | | 17.6 KB | 17.6 KB | 17.6 KB |
| `basename src/commands/xinfo-stream.json` | 18 B | | | 2478 B | 2478 B | 2478 B |
| `basename -s .json src/commands/xinfo-stream.json` | 13 B | | | 3136 B | 3136 B | 3136 B |
| `dirname src/commands/xinfo-stream.json` | 13 B | | | 2467 B | 2467 B | 2467 B |
| `printf '%s\n' a b c` | 6 B | | | 3325 B | 3325 B | 3325 B |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 18 B | | | 3422 B | 3422 B | 3422 B |
| `test -f src/commands/xinfo-stream.json && echo yes` | 4 B | | | 3563 B | 3563 B | 3563 B |
| `[ -d tests ] && echo folder` | 7 B | | | 3764 B | 3764 B | 3764 B |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 35 B | | | 3492 B | 3492 B | 3492 B |
| `echo a b c \| xargs -n 1 echo` | 6 B | | | 4674 B | 4674 B | 4674 B |
| `find .github -name '*.json' \| xargs cat \| wc -l` | 2 B | | | 5069 B | 5069 B | 5069 B |
| `find . -name '*.json' \| xargs grep -l implementing \| wc -l` | 2 B | | | 181.6 KB | 181.6 KB | 181.6 KB |
| `echo {a,b,c}{1..3}` | 27 B | | | 4485 B | 4485 B | 4485 B |
| `echo {1..20..3} {a..e}` | 28 B | | | 3967 B | 3967 B | 3967 B |
| `echo $(wc -l < src/commands/xinfo-stream.json) `basename src/commands/xinfo-stream.json`` | 22 B | | | 6425 B | 6425 B | 6425 B |
| `echo "first line: $(head -n 1 src/commands/xinfo-stream.json)"` | 14 B | | | 4996 B | 4996 B | 4996 B |
| `wc -l < src/commands/xinfo-stream.json` | 4 B | | | 3348 B | 3348 B | 3348 B |
| `cat src/commands/xinfo-stream.json > /dev/null 2>&1` | 0 B | | | 3449 B | 3449 B | 3449 B |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 6 B | | | 4158 B | 4158 B | 4158 B |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2 B | | | 4072 B | 4072 B | 4072 B |
| `grep -q for src/commands/xinfo-stream.json && echo found \|\| echo none` | 6 B | | | 11.7 KB | 11.7 KB | 11.7 KB |
| `false \|\| echo fallback` | 9 B | | | 2861 B | 2861 B | 2861 B |
| `true && grep -c for src/commands/xinfo-stream.json` | 2 B | | | 10.5 KB | 10.5 KB | 10.5 KB |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 14 B | | | 4178 B | 4178 B | 4178 B |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2 B | | | 3130 B | 3130 B | 3130 B |
| `echo one \⏎two` | 8 B | | | 2591 B | 2591 B | 2591 B |
| `grep -c for src/commands/xinfo-stream.json⏎wc -l src/commands/xinfo-stream.json` | 37 B | | | 11.2 KB | 11.2 KB | 11.2 KB |

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
