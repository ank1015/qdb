# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 4, on disk, settled), and include starting bash and the tool (1.652 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 19.50 | 110.4 | 24.7 MB | 27.5 MB | 0.234 | yes |
| Stage 3, on disk, settled | on | 184.4 | 184.4 | 426.3 KB | 25.0 MB | 0.288 | yes |
| Stage 4, in memory | on | 26.60 | 122.1 | 27.3 MB | 29.6 MB | 0.324 | yes |
| Stage 4, on disk, settled | on | 178.8 | 178.8 | 440.5 KB | 26.5 MB | 0.450 | yes |
| Stage 4, on disk, after the writes | on | 178.8 | 178.8 | 440.5 KB | 26.5 MB | 0.450 | yes |

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

- Built in 95.50 ms while loading.
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
| `ls` | 2.382 |  | 0.001 | 0.001 | 0.001 | 0.002 | 0.001 | yes |
| `ls -a` | 2.852 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | yes |
| `ls -R` | 3.809 |  | 0.043 | 0.049 | 0.042 | 0.070 | 0.047 | yes |
| `ls -aR tests` | 2.764 |  | 0.013 | 0.016 | 0.015 | 0.020 | 0.015 | yes |
| `ls -p tests` | 3.109 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.002 | yes |
| `find .` | 3.954 |  | 0.143 | 0.183 | 0.122 | 0.124 | 0.139 | yes |
| `find . -name '*.json'` | 4.784 |  | 0.227 | 0.238 | 0.168 | 0.186 | 0.180 | yes |
| `find . -name '*.c'` | 4.305 |  | 0.162 | 0.251 | 0.186 | 0.159 | 0.288 | yes |
| `find . -type d` | 4.267 |  | 0.127 | 0.168 | 0.133 | 0.116 | 0.201 | yes |
| `find . -type f` | 3.365 |  | 0.116 | 0.119 | 0.143 | 0.143 | 0.122 | yes |
| `find . -maxdepth 2 -type f` | 2.587 |  | 0.017 | 0.017 | 0.021 | 0.021 | 0.031 | yes |
| `find tests -name '*.json' -type f` | 2.854 |  | 0.022 | 0.026 | 0.029 | 0.028 | 0.047 | yes |
| `find . -name 'Makefile'` | 3.878 |  | 0.129 | 0.127 | 0.132 | 0.132 | 0.155 | yes |
| `cat src/commands/xinfo-stream.json` | 2.177 |  | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `cat -n src/commands/xinfo-stream.json` | 1.951 |  | 0.027 | 0.031 | 0.034 | 0.031 | 0.030 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 2.103 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 2.646 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.002 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 2.367 |  | 0.004 | 0.004 | 0.004 | 0.004 | 0.005 | yes |
| `wc src/commands/xinfo-stream.json` | 2.668 |  | 0.014 | 0.014 | 0.013 | 0.014 | 0.017 | yes |
| `wc -l src/commands/*.json` | 6.379 |  | 1.148 | 0.225 | 0.257 | 0.381 | 0.242 | yes |
| `cat src/commands/*.json \| wc -l` | 10.61 |  | 1.115 | 1.100 | 0.395 | 0.198 | 0.418 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 3.026 |  | 0.006 | 0.006 | 0.006 | 0.004 | 0.006 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 2.221 |  | 0.004 | 0.004 | 0.008 | 0.005 | 0.005 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.829 |  | 0.004 | 0.004 | 0.008 | 0.005 | 0.006 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 2.439 |  | 0.190 | 0.196 | 0.016 | 0.010 | 0.017 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 2.439 |  | 0.292 | 0.242 | 0.069 | 0.044 | 0.076 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.095 |  | 0.065 | 0.077 | 0.075 | 0.042 | 0.088 | yes |
| `grep -rn qdbzzabsentterm .` | 23.09 | 14.69 | 0.008 | 0.010 | 0.018 | 0.014 | 0.018 | yes |
| `grep -rl qdbzzabsentterm .` | 15.90 | 14.85 | 0.009 | 0.018 | 0.016 | 0.012 | 0.012 | yes |
| `grep -rn nestedcallreplyarray .` | 22.00 | 14.10 | 0.078 | 0.038 | 0.035 | 0.059 | 0.037 | yes |
| `grep -rl nestedcallreplyarray .` | 15.28 | 16.56 | 0.095 | 0.032 | 0.054 | 0.050 | 0.063 | yes |
| `grep -rn xmallctlmibnametomib .` | 21.30 | 14.18 | 0.023 | 0.048 | 0.041 | 0.029 | 0.031 | yes |
| `grep -rl xmallctlmibnametomib .` | 13.65 | 12.76 | 0.033 | 0.031 | 0.039 | 0.030 | 0.042 | yes |
| `grep -rn implementing .` | 22.72 | 15.64 | 0.807 | 0.834 | 0.765 | 0.949 | 0.983 | yes |
| `grep -rl implementing .` | 14.96 | 12.44 | 0.958 | 0.947 | 0.683 | 0.580 | 0.953 | yes |
| `grep -rn between .` | 25.55 | 15.02 | 0.859 | 0.660 | 0.994 | 0.746 | 0.738 | yes |
| `grep -rl between .` | 17.98 | 16.31 | 0.242 | 0.280 | 0.436 | 0.256 | 0.206 | yes |
| `grep -rn for .` | 43.69 | 24.42 | 3.332 | 3.684 | 3.988 | 3.825 | 5.906 | yes |
| `grep -rl for .` | 12.53 | 15.86 | 0.321 | 0.283 | 0.396 | 0.303 | 0.329 | yes |
| `grep -rc implementing .` | 17.15 | 15.54 | 0.910 | 0.841 | 0.832 | 0.800 | 1.192 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 22.08 | 18.16 | 0.026 | 0.032 | 0.048 | 0.030 | 0.053 | yes |
| `grep -ri implementing .` | 18.44 | 16.08 | 0.875 | 0.698 | 0.999 | 1.127 | 0.855 | yes |
| `grep -rn implementing tests` | 5.882 | 5.487 | 0.064 | 0.072 | 0.081 | 0.122 | 0.061 | yes |
| `grep -rl between tests` | 5.097 | 5.013 | 0.025 | 0.030 | 0.035 | 0.047 | 0.035 | yes |
| `grep -rF 'implementing' .` | 16.69 | 14.41 | 0.701 | 0.595 | 0.648 | 0.980 | 0.569 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 23.61 | 18.68 | 0.473 | 0.459 | 0.679 | 0.549 | 0.477 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 32.80 | 18.31 | 0.203 | 0.078 | 0.096 | 0.140 | 0.137 | yes |
| `grep -rE 'between\|implementing' .` | 41.00 | 14.99 | 1.703 | 1.150 | 1.778 | 1.562 | 1.270 | yes |
| `grep -rE '[0-9]{4,}' .` | 39.03 | 30.00 | 22.20 | 20.87 | 18.76 | 20.65 | 20.59 | yes |
| `grep -rn '^#include' .` | 28.60 | 18.39 | 1.348 | 1.167 | 1.268 | 1.186 | 0.953 | yes |
| `grep -rE '^(import\|from) ' .` | 23.55 | 16.23 | 2.596 | 2.421 | 2.201 | 3.467 | 2.904 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 8.367 | 8.750 | 0.723 | 0.598 | 0.703 | 0.989 | 0.790 | yes |
| `grep -rv e .github` | 3.466 | 4.684 | 0.037 | 0.039 | 0.054 | 0.068 | 0.052 | yes |
| `grep -r '\(implementing\).*\1' .` | 17.93 |  | 0.855 | 0.809 | 0.660 | 1.192 | 0.925 | yes |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 4.393 |  | | 0.165 | 0.261 | 0.278 | 0.222 | yes |
| `grep -l implementing src/commands/*.json` | 4.340 |  | | 0.304 | 0.208 | 0.304 | 0.249 | yes |
| `grep -c qdbzzabsentterm src/commands/*.json` | 4.115 |  | | 0.145 | 0.160 | 0.259 | 0.213 | yes |
| `grep -rl between . \| wc -l` | 18.82 |  | 0.244 | 0.174 | 0.240 | 0.178 | 0.236 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 31.37 |  | 1.997 | 2.592 | 2.401 | 2.219 | 2.740 | yes |
| `find . -name '*.json' \| wc -l` | 4.681 |  | 0.172 | 0.172 | 0.193 | 0.190 | 0.255 | yes |
| `find . -type f \| grep -c json` | 4.160 |  | 0.212 | 0.234 | 0.265 | 0.234 | 0.376 | yes |
| `grep -rn for . \| head -n 5` | 3.453 |  | 0.005 | 0.006 | 0.007 | 0.007 | 0.010 | order differs |
| `grep -rn between . \| head -n 1` | 4.673 |  | 0.007 | 0.009 | 0.009 | 0.010 | 0.010 | order differs |
| `find . -type f \| head -n 3` | 3.190 |  | 0.036 | 0.033 | 0.032 | 0.035 | 0.039 | order differs |
| `cat src/commands/*.json \| head -n 2` | 3.190 |  | 0.098 | 0.106 | 0.099 | 0.098 | 0.092 | yes |
| `grep -rno implementing .` | 22.86 | 15.19 | | | 0.653 | 0.808 | 0.795 | yes |
| `grep -rw implementing .` | 17.04 | 14.42 | | | 0.647 | 0.794 | 0.698 | yes |
| `grep -rnw xmallctlmibnametomib .` | 21.00 | 12.76 | | | 0.149 | 0.110 | 0.100 | yes |
| `grep -rn -C 2 xmallctlmibnametomib .` | 20.55 | 30.12 | | | 0.029 | 0.029 | 0.032 | yes |
| `grep -rn -A 1 -B 1 implementing .` | 22.38 | 38.11 | | | 0.716 | 0.690 | 0.924 | yes |
| `grep -rho 'arena_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 22.84 |  | | | 2.150 | 2.146 | 1.989 | yes |
| `grep -rhow '[a-z]*xmallctlmibnametomib[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 17.43 |  | | | 0.119 | 0.125 | 0.123 | yes |
| `rg -j 1 qdbzzabsentterm` | 24.64 |  | | | 0.065 | 0.038 | 0.037 | yes |
| `rg -j 1 nestedcallreplyarray` | 19.26 |  | | | 0.118 | 0.054 | 0.062 | yes |
| `rg -j 1 xmallctlmibnametomib` | 17.78 |  | | | 0.243 | 0.143 | 0.102 | yes |
| `rg -j 1 implementing` | 18.26 |  | | | 1.033 | 1.033 | 1.090 | yes |
| `rg -j 1 between` | 18.96 |  | | | 0.865 | 0.889 | 0.904 | yes |
| `rg -j 1 for` | 36.94 |  | | | 3.797 | 3.096 | 3.776 | yes |
| `rg -j 1 -l implementing` | 23.83 |  | | | 1.220 | 0.963 | 1.343 | yes |
| `rg -j 1 -l for` | 26.10 |  | | | 2.213 | 1.519 | 1.519 | yes |
| `rg -j 1 -c between` | 19.83 |  | | | 0.753 | 0.926 | 0.828 | yes |
| `rg -j 1 -i XMALLCTLMIBNAMETOMIB` | 28.93 |  | | | 0.309 | 0.353 | 0.272 | yes |
| `rg -j 1 -w implementing` | 24.46 |  | | | 1.924 | 2.720 | 1.919 | yes |
| `rg -j 1 -F 'implementing'` | 24.59 |  | | | 1.143 | 0.847 | 1.053 | yes |
| `rg -j 1 -n -C 2 xmallctlmibnametomib` | 19.73 |  | | | 0.142 | 0.137 | 0.190 | yes |
| `rg -j 1 'arena_[a-z]+\('` | 19.93 |  | | | 0.773 | 0.877 | 0.754 | yes |
| `rg -j 1 'xmallctlmibnametomib\|nestedcallreplyarray'` | 21.54 |  | | | 0.226 | 0.181 | 0.340 | yes |
| `rg -j 1 '\bxmallctlmibnametomib\w*'` | 23.42 |  | | | 3.183 | 3.123 | 3.175 | yes |
| `rg -j 1 -t c implementing` | 15.72 |  | | | 1.284 | 0.775 | 0.919 | yes |
| `rg -j 1 -g '*.json' implementing` | 9.991 |  | | | 0.758 | 0.758 | 0.761 | yes |
| `rg -j 1 implementing tests` | 6.609 |  | | | 0.264 | 0.256 | 0.259 | yes |
| `rg -j 1 -uu implementing` | 14.23 |  | | | 0.627 | 0.965 | 0.778 | yes |
| `rg -j 1 --sort path xmallctlmibnametomib` | 20.07 |  | | | 0.139 | 0.123 | 0.209 | yes |
| `rg -j 1 --files \| wc -l` | 10.05 |  | | | 1.228 | 1.113 | 1.878 | yes |
| `rg -j 1 -l between \| wc -l` | 19.18 |  | | | 0.475 | 0.714 | 0.355 | yes |
| `ls -ltR` | 8.099 |  | | | 1.037 | 1.015 | 0.740 | yes |
| `ls -lt src/commands` | 3.901 |  | | | 0.198 | 0.150 | 0.154 | yes |
| `ls -ltr tests \| tail -n 3` | 3.357 |  | | | 0.009 | 0.007 | 0.009 | yes |
| `find . -newer src/commands/xinfo-stream.json` | 5.922 |  | | | 0.223 | 0.231 | 0.139 | yes |
| `find . -newer src/commands/xinfo-stream.json -name '*.json'` | 4.650 |  | | | 0.255 | 0.193 | 0.149 | yes |
| `find . -type f -mtime +30` | 6.508 |  | | | 0.128 | 0.113 | 0.136 | yes |
| `du -s .` | 4.768 |  | | | 0.002 | 0.001 | 0.001 | not compared |
| `du -sh tests` | 2.482 |  | | | 0.002 | 0.001 | 0.001 | not compared |
| `du -sb .` | 4.776 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `du -sb tests` | 2.678 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `du -b -d 1 tests` | 2.796 |  | | | 0.002 | 0.003 | 0.003 | yes |
| `sort src/commands/xinfo-stream.json` | 2.765 |  | | | 0.030 | 0.034 | 0.029 | yes |
| `sort -r src/commands/xinfo-stream.json \| head -n 3` | 3.384 |  | | | 0.036 | 0.036 | 0.053 | yes |
| `sort -u src/commands/xinfo-stream.json \| wc -l` | 3.354 |  | | | 0.033 | 0.029 | 0.033 | yes |
| `sort -k 2 src/commands/xinfo-stream.json \| tail -n 3` | 3.472 |  | | | 0.083 | 0.052 | 0.049 | yes |
| `cat src/commands/*.json \| sort \| uniq -c \| sort -rn \| head -n 5` | 17.74 |  | | | 4.621 | 4.543 | 4.853 | yes |
| `uniq -c src/commands/xinfo-stream.json \| wc -l` | 2.904 |  | | | 0.022 | 0.031 | 0.023 | yes |
| `cut -c 1-20 src/commands/xinfo-stream.json` | 2.916 |  | | | 0.017 | 0.016 | 0.017 | yes |
| `cut -d ' ' -f 1 src/commands/xinfo-stream.json \| sort \| uniq \| wc -l` | 5.874 |  | | | 0.063 | 0.071 | 0.061 | yes |
| `tr a-z A-Z < src/commands/xinfo-stream.json` | 3.428 |  | | | 0.003 | 0.003 | 0.003 | yes |
| `tr -d '\n' < src/commands/xinfo-stream.json \| wc -c` | 4.167 |  | | | 0.008 | 0.009 | 0.008 | yes |
| `tr -s ' ' < src/commands/xinfo-stream.json \| wc -c` | 3.558 |  | | | 0.013 | 0.013 | 0.013 | yes |
| `nl src/commands/xinfo-stream.json` | 3.370 |  | | | 0.033 | 0.034 | 0.030 | yes |
| `nl -ba src/commands/xinfo-stream.json \| tail -n 3` | 4.747 |  | | | 0.035 | 0.035 | 0.036 | yes |
| `seq 1 100000 \| wc -l` | 4.661 |  | | | 2.474 | 2.631 | 2.522 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.941 |  | | | 0.024 | 0.037 | 0.026 | yes |
| `basename src/commands/xinfo-stream.json` | 2.688 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `basename -s .json src/commands/xinfo-stream.json` | 2.967 |  | | | 0.002 | 0.002 | 0.001 | yes |
| `dirname src/commands/xinfo-stream.json` | 3.432 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `printf '%s\n' a b c` | 1.761 |  | | | 0.001 | 0.002 | 0.002 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 1.835 |  | | | 0.002 | 0.003 | 0.002 | yes |
| `test -f src/commands/xinfo-stream.json && echo yes` | 1.619 |  | | | 0.002 | 0.002 | 0.003 | yes |
| `[ -d tests ] && echo folder` | 1.553 |  | | | 0.002 | 0.003 | 0.003 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 2.640 |  | | | 0.003 | 0.003 | 0.004 | yes |
| `echo a b c \| xargs -n 1 echo` | 6.682 |  | | | 0.003 | 0.003 | 0.004 | yes |
| `find .github -name '*.json' \| xargs cat \| wc -l` | 4.459 |  | | | 0.005 | 0.007 | 0.005 | yes |
| `find . -name '*.json' \| xargs grep -l implementing \| wc -l` | 7.594 |  | | | 0.782 | 0.433 | 0.729 | yes |
| `echo {a,b,c}{1..3}` | 1.741 |  | | | 0.004 | 0.006 | 0.005 | yes |
| `echo {1..20..3} {a..e}` | 1.897 |  | | | 0.004 | 0.007 | 0.006 | yes |
| `echo $(wc -l < src/commands/xinfo-stream.json) `basename src/commands/xinfo-stream.json`` | 4.716 |  | | | 0.006 | 0.006 | 0.009 | yes |
| `echo "first line: $(head -n 1 src/commands/xinfo-stream.json)"` | 3.021 |  | | | 0.004 | 0.004 | 0.006 | yes |
| `wc -l < src/commands/xinfo-stream.json` | 2.565 |  | | | 0.002 | 0.002 | 0.002 | yes |
| `cat src/commands/xinfo-stream.json > /dev/null 2>&1` | 2.947 |  | | | 0.002 | 0.002 | 0.002 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 2.827 |  | | | 0.003 | 0.003 | 0.004 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 3.278 |  | | | 0.003 | 0.004 | 0.003 | yes |
| `grep -q for src/commands/xinfo-stream.json && echo found \|\| echo none` | 2.942 |  | | | 0.006 | 0.006 | 0.009 | yes |
| `false \|\| echo fallback` | 1.534 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `true && grep -c for src/commands/xinfo-stream.json` | 2.570 |  | | | 0.005 | 0.006 | 0.009 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 3.310 |  | | | 0.003 | 0.002 | 0.004 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2.804 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `echo one \⏎two` | 1.586 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `grep -c for src/commands/xinfo-stream.json⏎wc -l src/commands/xinfo-stream.json` | 4.221 |  | | | 0.006 | 0.007 | 0.007 | yes |
| `mkdir -p bench_out/a/b` | 3.472 |  | 0.015 | 0.517 | 0.015 | 0.850 | | yes |
| `touch bench_out/empty.txt` | 2.788 |  | 0.015 | 0.502 | 0.016 | 0.734 | | yes |
| `echo 'hello world' > bench_out/hello.txt` | 2.161 |  | 0.024 | 0.433 | 0.081 | 0.764 | | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.764 |  | 0.020 | 0.380 | 0.029 | 0.587 | | yes |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 3.522 |  | 0.121 | 0.663 | 0.023 | 0.487 | | yes |
| `cp -r tests bench_out/dir_copy` | 29.23 |  | 19.65 | 32.53 | 0.694 | 1.467 | | yes |
| `cp -r src/commands bench_out/work` | 51.10 |  | 6.664 | 10.46 | 0.526 | 2.277 | | yes |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 3.796 |  | 0.302 | 1.325 | 1.173 | 1.196 | | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 4.126 |  | 0.364 | 1.483 | 0.476 | 1.012 | | yes |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 64.76 |  | 24.77 | 15.35 | 7.728 | 11.29 | | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | 165.8 |  | | 0.419 | 0.400 | 3.378 | | yes |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | 174.8 |  | | 0.439 | 0.380 | 2.842 | | yes |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 3.634 |  | | 0.023 | 0.019 | 0.557 | | yes |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 2.799 |  | | 0.023 | 0.019 | 0.557 | | yes |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | 3.733 |  | | 0.037 | 0.027 | 0.526 | | yes |
| `grep -rc QDBBENCHMANY bench_out` | 6.411 |  | 0.458 | 0.332 | 0.306 | 0.351 | | yes |
| `grep -rl QDBBENCHMANY .` | 19.15 |  | 0.295 | 0.202 | 0.203 | 0.201 | | yes |
| `grep -rl QDBBENCHONE .` | 18.38 |  | 0.091 | 0.072 | 0.116 | 0.086 | | yes |
| `cat src/commands/xinfo-stream.json \| tee bench_out/tee.txt \| wc -l` | 3.709 |  | | | 0.194 | 0.973 | | yes |
| `sort src/commands/xinfo-stream.json > bench_out/sorted.txt` | 3.771 |  | | | 0.289 | 1.132 | | yes |
| `sort -u src/commands/xinfo-stream.json >> bench_out/sorted.txt` | 3.548 |  | | | 0.287 | 1.075 | | yes |
| `cat qdb_no_such_file &> bench_out/both.txt` | 3.205 |  | | | 0.043 | 0.774 | | yes |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 3.068 |  | | | 0.031 | 0.581 | | yes |
| `find .github -name '*.json' \| sort \| xargs cat > bench_out/joined.txt` | 5.421 |  | | | 0.049 | 0.557 | | yes |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 2.627 |  | | | 0.019 | 0.452 | | yes |
| `sed -i '1d' bench_out/joined2.txt` | 2.486 |  | | | 0.025 | 0.405 | | yes |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 2.011 |  | | | 0.034 | 0.019 | | yes |
| `cat bench_out/both.txt` | 2.340 |  | | | 0.012 | 0.014 | | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.841 |  | 0.024 | 0.529 | 0.021 | 0.582 | | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.567 |  | 0.024 | 0.406 | 0.017 | 0.794 | | yes |
| `ls -R bench_out/a` | 2.715 |  | 0.026 | 0.021 | 0.011 | 0.020 | | yes |
| `rm bench_out/a/b/hello.txt` | 2.728 |  | 0.015 | 0.518 | 0.009 | 0.680 | | yes |
| `rmdir bench_out/a/b` | 3.512 |  | 0.014 | 0.387 | 0.013 | 0.461 | | yes |
| `rm -r bench_out/dir_moved` | 7.266 |  | 0.599 | 3.588 | 0.194 | 0.807 | | yes |
| `rm -rf bench_out` | 37.49 |  | 3.563 | 2.277 | 0.214 | 1.438 | | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes |
|---|---:|---:|---:|---:|---:|---:|
| `ls` | 288 B | 1681 B | 1681 B | 3643 B | 3643 B | 3643 B |
| `ls -a` | 338 B | 1849 B | 1849 B | 3941 B | 3941 B | 3941 B |
| `ls -R` | 25.0 KB | 38.2 KB | 38.2 KB | 46.5 KB | 46.5 KB | 46.5 KB |
| `ls -aR tests` | 4713 B | 12.5 KB | 12.5 KB | 15.6 KB | 15.6 KB | 15.6 KB |
| `ls -p tests` | 118 B | 1309 B | 1309 B | 3406 B | 3406 B | 3406 B |
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
| `ls -ltR` | 94.8 KB | | | 70.9 KB | 70.9 KB | 70.9 KB |
| `ls -lt src/commands` | 22.6 KB | | | 48.6 KB | 48.6 KB | 48.6 KB |
| `ls -ltr tests \| tail -n 3` | 145 B | | | 5454 B | 5454 B | 5454 B |
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
| `mkdir -p bench_out/a/b` | 0 B | 1459 B | 1747 B | 3711 B | 3711 B | |
| `touch bench_out/empty.txt` | 0 B | 979 B | 979 B | 2566 B | 2566 B | |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1242 B | 1292 B | 3665 B | 3661 B | |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1246 B | 1246 B | 3485 B | 3473 B | |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0 B | 19.5 KB | 36.8 KB | 3294 B | 3294 B | |
| `cp -r tests bench_out/dir_copy` | 0 B | 3535.7 KB | 1932.8 KB | 125.9 KB | 177.2 KB | |
| `cp -r src/commands bench_out/work` | 0 B | 993.0 KB | 343.8 KB | 168.0 KB | 222.4 KB | |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 25.4 KB | 42.9 KB | 46.3 KB | 46.3 KB | |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 25.1 KB | 44.8 KB | 48.4 KB | 48.4 KB | |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 0 B | 519.9 KB | 128.2 KB | 129.9 KB | 311.7 KB | |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | 0 B | | 82.9 KB | 85.0 KB | 85.0 KB | |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | 0 B | | 86.5 KB | 88.6 KB | 88.6 KB | |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 B | | 18.7 KB | 2874 B | 2874 B | |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 B | | 18.7 KB | 2874 B | 2874 B | |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | 0 B | | 18.9 KB | 21.3 KB | 21.3 KB | |
| `grep -rc QDBBENCHMANY bench_out` | 25.7 KB | 17.1 KB | 17.1 KB | 26.5 KB | 26.5 KB | |
| `grep -rl QDBBENCHMANY .` | 4346 B | 19.5 KB | 21.1 KB | 26.5 KB | 26.5 KB | |
| `grep -rl QDBBENCHONE .` | 21 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | |
| `cat src/commands/xinfo-stream.json \| tee bench_out/tee.txt \| wc -l` | 4 B | | | 40.3 KB | 40.3 KB | |
| `sort src/commands/xinfo-stream.json > bench_out/sorted.txt` | 0 B | | | 117.9 KB | 117.9 KB | |
| `sort -u src/commands/xinfo-stream.json >> bench_out/sorted.txt` | 0 B | | | 111.2 KB | 111.2 KB | |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 B | | | 4144 B | 3842 B | |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 0 B | | | 3756 B | 3714 B | |
| `find .github -name '*.json' \| sort \| xargs cat > bench_out/joined.txt` | 0 B | | | 70.9 KB | 70.9 KB | |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 B | | | 3790 B | 3790 B | |
| `sed -i '1d' bench_out/joined2.txt` | 0 B | | | 3550 B | 3550 B | |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 118 B | | | 4165 B | 4165 B | |
| `cat bench_out/both.txt` | 98 B | | | 2497 B | 2497 B | |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1191 B | 1191 B | 2900 B | 2900 B | |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1288 B | 1288 B | 3250 B | 3250 B | |
| `ls -R bench_out/a` | 41 B | 1089 B | 1089 B | 3092 B | 3092 B | |
| `rm bench_out/a/b/hello.txt` | 0 B | 997 B | 997 B | 2504 B | 2504 B | |
| `rmdir bench_out/a/b` | 0 B | 945 B | 945 B | 2383 B | 2383 B | |
| `rm -r bench_out/dir_moved` | 0 B | 1129 B | 1129 B | 2707 B | 2707 B | |
| `rm -rf bench_out` | 0 B | 3586 B | 3394 B | 5306 B | 5306 B | |

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
