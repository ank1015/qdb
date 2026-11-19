# Benchmark: redis

- Data: Redis 8.0.6 source tree, from the Ubuntu source package redis_8.0.6.orig.tar.gz (sha256 763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e). Skipped on import: 0 symbolic links, 13 files that are not valid text, 0 other
- Size: 1672 files in 98 folders, 17.4 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 5, on disk, settled), and include starting bash and the tool (1.697 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
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
| Stage 5, in memory | on | 841.4 | 964.8 | 27.3 MB | 29.9 MB | 0.366 | yes |
| Stage 5, on disk, settled | on | 188.0 | 188.0 | 440.6 KB | 27.0 MB | 0.338 | yes |

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

## The index (Stage 5, in memory)

- Built in 123.4 ms while loading.
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
| `cp src/commands/xinfo-stream.json bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | documents +1 -0; postings +867; list bytes rewritten 0; rebuilds 0 |
| `diff -u --label old --label new src/commands/xinfo-stream.json bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff src/commands/xinfo-stream.json bench_out/v2.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -q src/commands/xinfo-stream.json bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -u src/commands/xinfo-stream.json bench_out/copy.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -rq tests bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | documents +1 -0; postings +897; list bytes rewritten 0; rebuilds 0 |
| `diff -rq tests bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -r src/commands bench_out/work \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -ru src/commands bench_out/work \| grep -c '^@@'` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | documents +10000 -9994; postings +138134; list bytes rewritten 0; rebuilds 0 |
| `wc -l bench_out/loop.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for f in bench_out/work/*.json; do echo done >> "$f"; done` | documents +405 -3; postings +2553; list bytes rewritten 0; rebuilds 0 |
| `awk '{ print NR ": " $0 }' src/commands/xinfo-stream.json > bench_out/numbered.txt` | documents +1 -0; postings +1253; list bytes rewritten 0; rebuilds 0 |
| `awk 'END { print NR }' bench_out/numbered.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -547; postings +0; list bytes rewritten 0; rebuilds 0 |

## The index (Stage 5, on disk, settled)

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
| `cp src/commands/xinfo-stream.json bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | documents +1 -0; postings +867; list bytes rewritten 0; rebuilds 0 |
| `diff -u --label old --label new src/commands/xinfo-stream.json bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff src/commands/xinfo-stream.json bench_out/v2.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -q src/commands/xinfo-stream.json bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -u src/commands/xinfo-stream.json bench_out/copy.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -rq tests bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | documents +1 -0; postings +897; list bytes rewritten 0; rebuilds 0 |
| `diff -rq tests bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -r src/commands bench_out/work \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -ru src/commands bench_out/work \| grep -c '^@@'` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | documents +10000 -9994; postings +138134; list bytes rewritten 638191; rebuilds 0 |
| `wc -l bench_out/loop.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for f in bench_out/work/*.json; do echo done >> "$f"; done` | documents +405 -3; postings +2553; list bytes rewritten 1338; rebuilds 0 |
| `awk '{ print NR ": " $0 }' src/commands/xinfo-stream.json > bench_out/numbered.txt` | documents +1 -0; postings +1253; list bytes rewritten 0; rebuilds 0 |
| `awk 'END { print NR }' bench_out/numbered.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1; postings +0; list bytes rewritten 1665; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -547; postings +0; list bytes rewritten 124020; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 5, in memory | Stage 5, on disk, settled | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.488 |  | 0.001 | 0.001 | 0.001 | 0.002 | 0.001 | 0.003 | 0.003 | yes |
| `ls -a` | 2.475 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.004 | 0.004 | yes |
| `ls -R` | 4.020 |  | 0.043 | 0.049 | 0.042 | 0.070 | 0.047 | 0.059 | 0.061 | yes |
| `ls -aR tests` | 2.671 |  | 0.013 | 0.016 | 0.015 | 0.020 | 0.015 | 0.019 | 0.019 | yes |
| `ls -p tests` | 2.459 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.002 | 0.004 | 0.004 | yes |
| `find .` | 3.940 |  | 0.143 | 0.183 | 0.122 | 0.124 | 0.139 | 0.172 | 0.172 | yes |
| `find . -name '*.json'` | 4.124 |  | 0.227 | 0.238 | 0.168 | 0.186 | 0.180 | 0.216 | 0.227 | yes |
| `find . -name '*.c'` | 4.123 |  | 0.162 | 0.251 | 0.186 | 0.159 | 0.288 | 0.220 | 0.223 | yes |
| `find . -type d` | 3.802 |  | 0.127 | 0.168 | 0.133 | 0.116 | 0.201 | 0.162 | 0.162 | yes |
| `find . -type f` | 3.931 |  | 0.116 | 0.119 | 0.143 | 0.143 | 0.122 | 0.176 | 0.179 | yes |
| `find . -maxdepth 2 -type f` | 2.844 |  | 0.017 | 0.017 | 0.021 | 0.021 | 0.031 | 0.030 | 0.029 | yes |
| `find tests -name '*.json' -type f` | 3.934 |  | 0.022 | 0.026 | 0.029 | 0.028 | 0.047 | 0.036 | 0.037 | yes |
| `find . -name 'Makefile'` | 3.874 |  | 0.129 | 0.127 | 0.132 | 0.132 | 0.155 | 0.166 | 0.165 | yes |
| `cat src/commands/xinfo-stream.json` | 2.473 |  | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.004 | 0.003 | yes |
| `cat -n src/commands/xinfo-stream.json` | 2.583 |  | 0.027 | 0.031 | 0.034 | 0.031 | 0.030 | 0.043 | 0.042 | yes |
| `head -n 20 src/commands/xinfo-stream.json` | 2.305 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.004 | yes |
| `tail -n 20 src/commands/xinfo-stream.json` | 2.312 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.002 | 0.004 | 0.004 | yes |
| `tail -n +100 src/commands/xinfo-stream.json` | 2.969 |  | 0.004 | 0.004 | 0.004 | 0.004 | 0.005 | 0.009 | 0.009 | yes |
| `wc src/commands/xinfo-stream.json` | 3.314 |  | 0.014 | 0.014 | 0.013 | 0.014 | 0.017 | 0.016 | 0.016 | yes |
| `wc -l src/commands/*.json` | 7.692 |  | 1.148 | 0.225 | 0.257 | 0.381 | 0.242 | 0.308 | 0.316 | yes |
| `cat src/commands/*.json \| wc -l` | 10.11 |  | 1.115 | 1.100 | 0.395 | 0.198 | 0.418 | 0.297 | 0.303 | yes |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 2.933 |  | 0.006 | 0.006 | 0.006 | 0.004 | 0.006 | 0.008 | 0.008 | yes |
| `grep -c for src/commands/xinfo-stream.json` | 2.685 |  | 0.004 | 0.004 | 0.008 | 0.005 | 0.005 | 0.008 | 0.008 | yes |
| `grep -n for src/commands/xinfo-stream.json` | 2.530 |  | 0.004 | 0.004 | 0.008 | 0.005 | 0.006 | 0.009 | 0.011 | yes |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 2.732 |  | 0.190 | 0.196 | 0.016 | 0.010 | 0.017 | 0.034 | 0.036 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 2.740 |  | 0.292 | 0.242 | 0.069 | 0.044 | 0.076 | 0.069 | 0.071 | yes |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 3.152 |  | 0.065 | 0.077 | 0.075 | 0.042 | 0.088 | 0.072 | 0.112 | yes |
| `grep -rn qdbzzabsentterm .` | 23.32 | 14.07 | 0.008 | 0.010 | 0.018 | 0.014 | 0.018 | 0.026 | 0.019 | yes |
| `grep -rl qdbzzabsentterm .` | 15.19 | 13.64 | 0.009 | 0.018 | 0.016 | 0.012 | 0.012 | 0.017 | 0.019 | yes |
| `grep -rn nestedcallreplyarray .` | 22.85 | 13.53 | 0.078 | 0.038 | 0.035 | 0.059 | 0.037 | 0.049 | 0.054 | yes |
| `grep -rl nestedcallreplyarray .` | 14.30 | 12.43 | 0.095 | 0.032 | 0.054 | 0.050 | 0.063 | 0.047 | 0.053 | yes |
| `grep -rn xmallctlmibnametomib .` | 20.66 | 13.23 | 0.023 | 0.048 | 0.041 | 0.029 | 0.031 | 0.037 | 0.040 | yes |
| `grep -rl xmallctlmibnametomib .` | 13.98 | 12.33 | 0.033 | 0.031 | 0.039 | 0.030 | 0.042 | 0.048 | 0.036 | yes |
| `grep -rn implementing .` | 22.61 | 13.25 | 0.807 | 0.834 | 0.765 | 0.949 | 0.983 | 0.935 | 0.616 | yes |
| `grep -rl implementing .` | 16.21 | 13.15 | 0.958 | 0.947 | 0.683 | 0.580 | 0.953 | 0.564 | 0.552 | yes |
| `grep -rn between .` | 30.77 | 15.01 | 0.859 | 0.660 | 0.994 | 0.746 | 0.738 | 0.636 | 0.695 | yes |
| `grep -rl between .` | 16.58 | 12.12 | 0.242 | 0.280 | 0.436 | 0.256 | 0.206 | 0.149 | 0.154 | yes |
| `grep -rn for .` | 45.50 | 25.72 | 3.332 | 3.684 | 3.988 | 3.825 | 5.906 | 4.438 | 4.154 | yes |
| `grep -rl for .` | 11.87 | 15.15 | 0.321 | 0.283 | 0.396 | 0.303 | 0.329 | 0.310 | 0.310 | yes |
| `grep -rc implementing .` | 15.75 | 12.24 | 0.910 | 0.841 | 0.832 | 0.800 | 1.192 | 0.675 | 0.937 | yes |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 21.35 | 14.88 | 0.026 | 0.032 | 0.048 | 0.030 | 0.053 | 0.038 | 0.057 | yes |
| `grep -ri implementing .` | 17.47 | 14.59 | 0.875 | 0.698 | 0.999 | 1.127 | 0.855 | 0.726 | 0.890 | yes |
| `grep -rn implementing tests` | 5.785 | 5.030 | 0.064 | 0.072 | 0.081 | 0.122 | 0.061 | 0.082 | 0.113 | yes |
| `grep -rl between tests` | 5.141 | 4.764 | 0.025 | 0.030 | 0.035 | 0.047 | 0.035 | 0.042 | 0.059 | yes |
| `grep -rF 'implementing' .` | 16.23 | 15.41 | 0.701 | 0.595 | 0.648 | 0.980 | 0.569 | 0.604 | 0.579 | yes |
| `grep -rE 'arena_[a-z]+\(' .` | 21.44 | 15.65 | 0.473 | 0.459 | 0.679 | 0.549 | 0.477 | 0.546 | 0.534 | yes |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 31.86 | 16.20 | 0.203 | 0.078 | 0.096 | 0.140 | 0.137 | 0.103 | 0.111 | yes |
| `grep -rE 'between\|implementing' .` | 43.52 | 16.65 | 1.703 | 1.150 | 1.778 | 1.562 | 1.270 | 1.883 | 1.291 | yes |
| `grep -rE '[0-9]{4,}' .` | 31.56 | 29.19 | 22.20 | 20.87 | 18.76 | 20.65 | 20.59 | 23.97 | 23.42 | yes |
| `grep -rn '^#include' .` | 31.26 | 17.65 | 1.348 | 1.167 | 1.268 | 1.186 | 0.953 | 1.120 | 1.102 | yes |
| `grep -rE '^(import\|from) ' .` | 25.27 | 15.24 | 2.596 | 2.421 | 2.201 | 3.467 | 2.904 | 3.565 | 2.457 | yes |
| `grep -rin 'return [a-z_]*;$' tests` | 8.906 | 6.770 | 0.723 | 0.598 | 0.703 | 0.989 | 0.790 | 0.728 | 0.744 | yes |
| `grep -rv e .github` | 2.887 | 4.095 | 0.037 | 0.039 | 0.054 | 0.068 | 0.052 | 0.051 | 0.080 | yes |
| `grep -r '\(implementing\).*\1' .` | 16.90 |  | 0.855 | 0.809 | 0.660 | 1.192 | 0.925 | 0.975 | 0.718 | yes |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 4.425 |  | | 0.165 | 0.261 | 0.278 | 0.222 | 0.258 | 0.223 | yes |
| `grep -l implementing src/commands/*.json` | 5.072 |  | | 0.304 | 0.208 | 0.304 | 0.249 | 0.257 | 0.246 | yes |
| `grep -c qdbzzabsentterm src/commands/*.json` | 4.431 |  | | 0.145 | 0.160 | 0.259 | 0.213 | 0.217 | 0.216 | yes |
| `grep -rl between . \| wc -l` | 18.38 |  | 0.244 | 0.174 | 0.240 | 0.178 | 0.236 | 0.355 | 0.294 | yes |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 34.80 |  | 1.997 | 2.592 | 2.401 | 2.219 | 2.740 | 2.700 | 2.486 | yes |
| `find . -name '*.json' \| wc -l` | 5.487 |  | 0.172 | 0.172 | 0.193 | 0.190 | 0.255 | 0.242 | 0.221 | yes |
| `find . -type f \| grep -c json` | 4.937 |  | 0.212 | 0.234 | 0.265 | 0.234 | 0.376 | 0.300 | 0.271 | yes |
| `grep -rn for . \| head -n 5` | 3.453 |  | 0.005 | 0.006 | 0.007 | 0.007 | 0.010 | 0.011 | 0.016 | order differs |
| `grep -rn between . \| head -n 1` | 5.508 |  | 0.007 | 0.009 | 0.009 | 0.010 | 0.010 | 0.014 | 0.024 | order differs |
| `find . -type f \| head -n 3` | 3.750 |  | 0.036 | 0.033 | 0.032 | 0.035 | 0.039 | 0.051 | 0.078 | order differs |
| `cat src/commands/*.json \| head -n 2` | 3.531 |  | 0.098 | 0.106 | 0.099 | 0.098 | 0.092 | 0.126 | 0.161 | yes |
| `grep -rno implementing .` | 23.49 | 16.30 | | | 0.653 | 0.808 | 0.795 | 0.673 | 0.663 | yes |
| `grep -rw implementing .` | 17.03 | 13.11 | | | 0.647 | 0.794 | 0.698 | 0.693 | 0.682 | yes |
| `grep -rnw xmallctlmibnametomib .` | 22.06 | 14.18 | | | 0.149 | 0.110 | 0.100 | 0.141 | 0.126 | yes |
| `grep -rn -C 2 xmallctlmibnametomib .` | 22.26 | 29.73 | | | 0.029 | 0.029 | 0.032 | 0.037 | 0.042 | yes |
| `grep -rn -A 1 -B 1 implementing .` | 24.76 | 39.12 | | | 0.716 | 0.690 | 0.924 | 1.058 | 1.041 | yes |
| `grep -rho 'arena_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 25.37 |  | | | 2.150 | 2.146 | 1.989 | 1.285 | 1.329 | yes |
| `grep -rhow '[a-z]*xmallctlmibnametomib[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 17.06 |  | | | 0.119 | 0.125 | 0.123 | 0.255 | 0.394 | yes |
| `rg -j 1 qdbzzabsentterm` | 20.64 |  | | | 0.065 | 0.038 | 0.037 | 0.086 | 0.050 | yes |
| `rg -j 1 nestedcallreplyarray` | 20.10 |  | | | 0.118 | 0.054 | 0.062 | 0.083 | 0.085 | yes |
| `rg -j 1 xmallctlmibnametomib` | 19.83 |  | | | 0.243 | 0.143 | 0.102 | 0.296 | 0.266 | yes |
| `rg -j 1 implementing` | 23.77 |  | | | 1.033 | 1.033 | 1.090 | 0.966 | 1.488 | yes |
| `rg -j 1 between` | 19.69 |  | | | 0.865 | 0.889 | 0.904 | 0.848 | 1.299 | yes |
| `rg -j 1 for` | 28.80 |  | | | 3.797 | 3.096 | 3.776 | 4.522 | 4.172 | yes |
| `rg -j 1 -l implementing` | 20.75 |  | | | 1.220 | 0.963 | 1.343 | 0.899 | 0.886 | yes |
| `rg -j 1 -l for` | 20.79 |  | | | 2.213 | 1.519 | 1.519 | 1.712 | 1.688 | yes |
| `rg -j 1 -c between` | 18.76 |  | | | 0.753 | 0.926 | 0.828 | 0.822 | 0.827 | yes |
| `rg -j 1 -i XMALLCTLMIBNAMETOMIB` | 21.19 |  | | | 0.309 | 0.353 | 0.272 | 0.422 | 0.631 | yes |
| `rg -j 1 -w implementing` | 20.42 |  | | | 1.924 | 2.720 | 1.919 | 2.039 | 2.008 | yes |
| `rg -j 1 -F 'implementing'` | 19.24 |  | | | 1.143 | 0.847 | 1.053 | 0.996 | 0.985 | yes |
| `rg -j 1 -n -C 2 xmallctlmibnametomib` | 19.84 |  | | | 0.142 | 0.137 | 0.190 | 0.176 | 0.253 | yes |
| `rg -j 1 'arena_[a-z]+\('` | 19.26 |  | | | 0.773 | 0.877 | 0.754 | 0.957 | 0.951 | yes |
| `rg -j 1 'xmallctlmibnametomib\|nestedcallreplyarray'` | 27.88 |  | | | 0.226 | 0.181 | 0.340 | 0.280 | 0.426 | yes |
| `rg -j 1 '\bxmallctlmibnametomib\w*'` | 27.52 |  | | | 3.183 | 3.123 | 3.175 | 3.420 | 5.851 | yes |
| `rg -j 1 -t c implementing` | 19.80 |  | | | 1.284 | 0.775 | 0.919 | 0.939 | 1.288 | yes |
| `rg -j 1 -g '*.json' implementing` | 11.28 |  | | | 0.758 | 0.758 | 0.761 | 0.937 | 1.650 | yes |
| `rg -j 1 implementing tests` | 7.120 |  | | | 0.264 | 0.256 | 0.259 | 0.343 | 0.580 | yes |
| `rg -j 1 -uu implementing` | 13.94 |  | | | 0.627 | 0.965 | 0.778 | 0.647 | 0.786 | yes |
| `rg -j 1 --sort path xmallctlmibnametomib` | 19.81 |  | | | 0.139 | 0.123 | 0.209 | 0.169 | 0.165 | yes |
| `rg -j 1 --files \| wc -l` | 10.30 |  | | | 1.228 | 1.113 | 1.878 | 1.385 | 1.325 | yes |
| `rg -j 1 -l between \| wc -l` | 19.64 |  | | | 0.475 | 0.714 | 0.355 | 0.478 | 0.559 | yes |
| `ls -ltR` | 9.621 |  | | | 1.037 | 1.015 | 0.740 | 0.917 | 0.887 | yes |
| `ls -lt src/commands` | 5.507 |  | | | 0.198 | 0.150 | 0.154 | 0.189 | 0.191 | yes |
| `ls -ltr tests \| tail -n 3` | 3.728 |  | | | 0.009 | 0.007 | 0.009 | 0.013 | 0.022 | yes |
| `find . -newer src/commands/xinfo-stream.json` | 5.259 |  | | | 0.223 | 0.231 | 0.139 | 0.182 | 0.170 | yes |
| `find . -newer src/commands/xinfo-stream.json -name '*.json'` | 4.366 |  | | | 0.255 | 0.193 | 0.149 | 0.194 | 0.190 | yes |
| `find . -type f -mtime +30` | 5.431 |  | | | 0.128 | 0.113 | 0.136 | 0.170 | 0.179 | yes |
| `du -s .` | 4.528 |  | | | 0.002 | 0.001 | 0.001 | 0.003 | 0.005 | not compared |
| `du -sh tests` | 2.866 |  | | | 0.002 | 0.001 | 0.001 | 0.003 | 0.005 | not compared |
| `du -sb .` | 4.457 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.005 | yes |
| `du -sb tests` | 2.758 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | yes |
| `du -b -d 1 tests` | 2.891 |  | | | 0.002 | 0.003 | 0.003 | 0.005 | 0.005 | yes |
| `sort src/commands/xinfo-stream.json` | 3.717 |  | | | 0.030 | 0.034 | 0.029 | 0.037 | 0.037 | yes |
| `sort -r src/commands/xinfo-stream.json \| head -n 3` | 3.441 |  | | | 0.036 | 0.036 | 0.053 | 0.041 | 0.041 | yes |
| `sort -u src/commands/xinfo-stream.json \| wc -l` | 3.410 |  | | | 0.033 | 0.029 | 0.033 | 0.041 | 0.041 | yes |
| `sort -k 2 src/commands/xinfo-stream.json \| tail -n 3` | 3.402 |  | | | 0.083 | 0.052 | 0.049 | 0.059 | 0.059 | yes |
| `cat src/commands/*.json \| sort \| uniq -c \| sort -rn \| head -n 5` | 20.09 |  | | | 4.621 | 4.543 | 4.853 | 4.702 | 4.821 | yes |
| `uniq -c src/commands/xinfo-stream.json \| wc -l` | 3.414 |  | | | 0.022 | 0.031 | 0.023 | 0.024 | 0.024 | yes |
| `cut -c 1-20 src/commands/xinfo-stream.json` | 3.020 |  | | | 0.017 | 0.016 | 0.017 | 0.018 | 0.018 | yes |
| `cut -d ' ' -f 1 src/commands/xinfo-stream.json \| sort \| uniq \| wc -l` | 4.115 |  | | | 0.063 | 0.071 | 0.061 | 0.071 | 0.071 | yes |
| `tr a-z A-Z < src/commands/xinfo-stream.json` | 3.231 |  | | | 0.003 | 0.003 | 0.003 | 0.005 | 0.006 | yes |
| `tr -d '\n' < src/commands/xinfo-stream.json \| wc -c` | 3.242 |  | | | 0.008 | 0.009 | 0.008 | 0.013 | 0.013 | yes |
| `tr -s ' ' < src/commands/xinfo-stream.json \| wc -c` | 3.356 |  | | | 0.013 | 0.013 | 0.013 | 0.019 | 0.018 | yes |
| `nl src/commands/xinfo-stream.json` | 3.279 |  | | | 0.033 | 0.034 | 0.030 | 0.042 | 0.043 | yes |
| `nl -ba src/commands/xinfo-stream.json \| tail -n 3` | 3.475 |  | | | 0.035 | 0.035 | 0.036 | 0.046 | 0.046 | yes |
| `seq 1 100000 \| wc -l` | 4.983 |  | | | 2.474 | 2.631 | 2.522 | 3.065 | 3.070 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.898 |  | | | 0.024 | 0.037 | 0.026 | 0.037 | 0.038 | yes |
| `basename src/commands/xinfo-stream.json` | 2.634 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | yes |
| `basename -s .json src/commands/xinfo-stream.json` | 3.100 |  | | | 0.002 | 0.002 | 0.001 | 0.003 | 0.004 | yes |
| `dirname src/commands/xinfo-stream.json` | 2.664 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | yes |
| `printf '%s\n' a b c` | 1.714 |  | | | 0.001 | 0.002 | 0.002 | 0.004 | 0.004 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 2.091 |  | | | 0.002 | 0.003 | 0.002 | 0.004 | 0.006 | yes |
| `test -f src/commands/xinfo-stream.json && echo yes` | 1.872 |  | | | 0.002 | 0.002 | 0.003 | 0.004 | 0.007 | yes |
| `[ -d tests ] && echo folder` | 2.058 |  | | | 0.002 | 0.003 | 0.003 | 0.005 | 0.008 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 2.647 |  | | | 0.003 | 0.003 | 0.004 | 0.005 | 0.005 | yes |
| `echo a b c \| xargs -n 1 echo` | 6.944 |  | | | 0.003 | 0.003 | 0.004 | 0.007 | 0.007 | yes |
| `find .github -name '*.json' \| xargs cat \| wc -l` | 4.936 |  | | | 0.005 | 0.007 | 0.005 | 0.010 | 0.010 | yes |
| `find . -name '*.json' \| xargs grep -l implementing \| wc -l` | 7.741 |  | | | 0.782 | 0.433 | 0.729 | 1.042 | 0.622 | yes |
| `echo {a,b,c}{1..3}` | 1.593 |  | | | 0.004 | 0.006 | 0.005 | 0.010 | 0.006 | yes |
| `echo {1..20..3} {a..e}` | 1.582 |  | | | 0.004 | 0.007 | 0.006 | 0.007 | 0.007 | yes |
| `echo $(wc -l < src/commands/xinfo-stream.json) `basename src/commands/xinfo-stream.json`` | 4.860 |  | | | 0.006 | 0.006 | 0.009 | 0.015 | 0.011 | yes |
| `echo "first line: $(head -n 1 src/commands/xinfo-stream.json)"` | 3.330 |  | | | 0.004 | 0.004 | 0.006 | 0.007 | 0.008 | yes |
| `wc -l < src/commands/xinfo-stream.json` | 3.335 |  | | | 0.002 | 0.002 | 0.002 | 0.004 | 0.004 | yes |
| `cat src/commands/xinfo-stream.json > /dev/null 2>&1` | 3.217 |  | | | 0.002 | 0.002 | 0.002 | 0.003 | 0.003 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 2.873 |  | | | 0.003 | 0.003 | 0.004 | 0.005 | 0.005 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2.925 |  | | | 0.003 | 0.004 | 0.003 | 0.005 | 0.005 | yes |
| `grep -q for src/commands/xinfo-stream.json && echo found \|\| echo none` | 2.841 |  | | | 0.006 | 0.006 | 0.009 | 0.009 | 0.009 | yes |
| `false \|\| echo fallback` | 1.576 |  | | | 0.001 | 0.001 | 0.001 | 0.004 | 0.004 | yes |
| `true && grep -c for src/commands/xinfo-stream.json` | 2.582 |  | | | 0.005 | 0.006 | 0.009 | 0.009 | 0.009 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 3.205 |  | | | 0.003 | 0.002 | 0.004 | 0.005 | 0.006 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2.889 |  | | | 0.001 | 0.001 | 0.002 | 0.003 | 0.003 | yes |
| `echo one \⏎two` | 1.609 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | yes |
| `grep -c for src/commands/xinfo-stream.json⏎wc -l src/commands/xinfo-stream.json` | 3.850 |  | | | 0.006 | 0.007 | 0.007 | 0.011 | 0.011 | yes |
| `for i in $(seq 100000); do :; done` | 106.8 |  | | | | | | 37.88 | 40.91 | yes |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 30.97 |  | | | | | | 24.90 | 25.37 | yes |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 1.715 |  | | | | | | 0.032 | 0.032 | yes |
| `for f in src/commands/*.json; do echo "$f"; done \| wc -l` | 3.318 |  | | | | | | 0.411 | 0.403 | yes |
| `for f in src/commands/*.json; do grep -c for "$f"; done \| sort -n \| tail -n 3` | 590.8 |  | | | | | | 2.327 | 2.235 | yes |
| `n=0; for f in src/commands/*.json; do if grep -q between "$f"; then n=$((n+1)); fi; done; echo $n` | 572.0 |  | | | | | | 3.477 | 3.592 | yes |
| `for f in src/commands/*.json; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 3.518 |  | | | | | | 0.837 | 0.847 | yes |
| `H=$PWD; for d in tests .github src/commands; do cd "$d"; ls \| wc -l; cd "$H"; done` | 7.595 |  | | | | | | 0.033 | 0.033 | yes |
| `X=for; N=$(grep -c "$X" src/commands/xinfo-stream.json); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 3.620 |  | | | | | | 0.016 | 0.015 | yes |
| `find .github -name '*.json' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 3.664 |  | | | | | | 0.015 | 0.021 | yes |
| `for f in src/commands/*.json; do head -n 1 "$f"; done > /dev/null` | 491.6 |  | | | | | | 0.635 | 0.651 | yes |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 1.849 |  | | | | | | 0.019 | 0.033 | yes |
| `awk '{ print $2 }' src/commands/xinfo-stream.json` | 3.236 |  | | | | | | 0.032 | 0.032 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 10.65 |  | | | | | | 2.305 | 3.492 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '/for/ { n++ } END { print n }'` | 9.177 |  | | | | | | 1.643 | 1.677 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 12.82 |  | | | | | | 3.823 | 2.413 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 9.672 |  | | | | | | 3.170 | 2.134 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 8.934 |  | | | | | | 2.419 | 2.530 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 9.533 |  | | | | | | 1.558 | 1.600 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 11.63 |  | | | | | | 3.857 | 3.975 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "for") n++ } END { print n }'` | 19.33 |  | | | | | | 6.781 | 7.036 | yes |
| `awk '{ gsub(/[0-9]+/, "N"); print }' src/commands/xinfo-stream.json \| wc -c` | 3.868 |  | | | | | | 0.229 | 0.232 | yes |
| `awk -v OFS=- '{ $1 = NR; print }' src/commands/xinfo-stream.json \| tail -n 1` | 3.759 |  | | | | | | 0.111 | 0.202 | yes |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 10.06 |  | | | | | | 5.377 | 7.670 | yes |
| `grep -rn implementing . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 25.88 |  | | | | | | 0.688 | 0.814 | yes |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 11.65 |  | | | | | | 2.979 | 3.006 | yes |
| `mkdir -p bench_out/a/b` | 2.883 |  | 0.015 | 0.517 | 0.015 | 0.850 | | 0.021 | 6.152 | yes |
| `touch bench_out/empty.txt` | 2.388 |  | 0.015 | 0.502 | 0.016 | 0.734 | | 0.018 | 0.413 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.761 |  | 0.024 | 0.433 | 0.081 | 0.764 | | 0.126 | 0.326 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.730 |  | 0.020 | 0.380 | 0.029 | 0.587 | | 0.098 | 0.332 | yes |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 2.746 |  | 0.121 | 0.663 | 0.023 | 0.487 | | 0.019 | 0.321 | yes |
| `cp -r tests bench_out/dir_copy` | 33.18 |  | 19.65 | 32.53 | 0.694 | 1.467 | | 0.547 | 1.372 | yes |
| `cp -r src/commands bench_out/work` | 60.99 |  | 6.664 | 10.46 | 0.526 | 2.277 | | 0.628 | 1.613 | yes |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 3.041 |  | 0.302 | 1.325 | 1.173 | 1.196 | | 0.747 | 1.201 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.205 |  | 0.364 | 1.483 | 0.476 | 1.012 | | 0.360 | 0.980 | yes |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 77.56 |  | 24.77 | 15.35 | 7.728 | 11.29 | | 5.813 | 14.04 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | 140.7 |  | | 0.419 | 0.400 | 3.378 | | 0.446 | 3.119 | yes |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | 154.9 |  | | 0.439 | 0.380 | 2.842 | | 0.442 | 2.941 | yes |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 3.412 |  | | 0.023 | 0.019 | 0.557 | | 0.024 | 0.433 | yes |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 2.832 |  | | 0.023 | 0.019 | 0.557 | | 0.024 | 0.433 | yes |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | 3.772 |  | | 0.037 | 0.027 | 0.526 | | 0.033 | 0.481 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 5.850 |  | 0.458 | 0.332 | 0.306 | 0.351 | | 0.415 | 0.473 | yes |
| `grep -rl QDBBENCHMANY .` | 15.86 |  | 0.295 | 0.202 | 0.203 | 0.201 | | 0.176 | 0.187 | yes |
| `grep -rl QDBBENCHONE .` | 17.52 |  | 0.091 | 0.072 | 0.116 | 0.086 | | 0.080 | 0.094 | yes |
| `cat src/commands/xinfo-stream.json \| tee bench_out/tee.txt \| wc -l` | 3.677 |  | | | 0.194 | 0.973 | | 0.223 | 1.020 | yes |
| `sort src/commands/xinfo-stream.json > bench_out/sorted.txt` | 3.999 |  | | | 0.289 | 1.132 | | 0.279 | 1.046 | yes |
| `sort -u src/commands/xinfo-stream.json >> bench_out/sorted.txt` | 3.909 |  | | | 0.287 | 1.075 | | 0.243 | 1.150 | yes |
| `cat qdb_no_such_file &> bench_out/both.txt` | 3.590 |  | | | 0.043 | 0.774 | | 0.041 | 0.635 | yes |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 3.211 |  | | | 0.031 | 0.581 | | 0.036 | 0.617 | yes |
| `find .github -name '*.json' \| sort \| xargs cat > bench_out/joined.txt` | 5.149 |  | | | 0.049 | 0.557 | | 0.061 | 0.580 | yes |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 2.704 |  | | | 0.019 | 0.452 | | 0.023 | 0.477 | yes |
| `sed -i '1d' bench_out/joined2.txt` | 2.722 |  | | | 0.025 | 0.405 | | 0.029 | 0.565 | yes |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 3.431 |  | | | 0.034 | 0.019 | | 0.031 | 0.026 | yes |
| `cat bench_out/both.txt` | 2.385 |  | | | 0.012 | 0.014 | | 0.018 | 0.017 | yes |
| `cp src/commands/xinfo-stream.json bench_out/v2.txt` | 2.961 |  | | | | | | 0.024 | 0.627 | yes |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 3.830 |  | | | | | | 0.329 | 1.228 | yes |
| `diff -u --label old --label new src/commands/xinfo-stream.json bench_out/v2.txt` | 3.390 |  | | | | | | 0.110 | 0.092 | yes |
| `diff src/commands/xinfo-stream.json bench_out/v2.txt \| wc -l` | 3.460 |  | | | | | | 0.106 | 0.085 | yes |
| `diff -q src/commands/xinfo-stream.json bench_out/same.txt` | 2.758 |  | | | | | | 0.024 | 0.024 | yes |
| `diff -u src/commands/xinfo-stream.json bench_out/copy.txt \| wc -l` | 3.349 |  | | | | | | 0.202 | 0.155 | yes |
| `diff -rq tests bench_out/dir_copy` | 5.571 |  | | | | | | 0.275 | 0.162 | yes |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 5.449 |  | | | | | | 0.418 | 1.254 | yes |
| `diff -rq tests bench_out/dir_copy` | 7.227 |  | | | | | | 0.275 | 0.162 | yes |
| `diff -r src/commands bench_out/work \| wc -l` | 8.453 |  | | | | | | 2.302 | 1.727 | yes |
| `diff -ru src/commands bench_out/work \| grep -c '^@@'` | 9.181 |  | | | | | | 3.321 | 2.099 | yes |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 57.94 |  | | | | | | 43.99 | 43.89 | yes |
| `wc -l bench_out/loop.txt` | 3.346 |  | | | | | | 0.023 | 0.024 | yes |
| `for f in bench_out/work/*.json; do echo done >> "$f"; done` | 6.807 |  | | | | | | 1.091 | 2.720 | yes |
| `awk '{ print NR ": " $0 }' src/commands/xinfo-stream.json > bench_out/numbered.txt` | 5.180 |  | | | | | | 0.254 | 1.285 | yes |
| `awk 'END { print NR }' bench_out/numbered.txt` | 3.432 |  | | | | | | 0.048 | 0.054 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.959 |  | 0.024 | 0.529 | 0.021 | 0.582 | | 0.027 | 0.521 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 3.606 |  | 0.024 | 0.406 | 0.017 | 0.794 | | 0.025 | 0.435 | yes |
| `ls -R bench_out/a` | 2.859 |  | 0.026 | 0.021 | 0.011 | 0.020 | | 0.016 | 0.017 | yes |
| `rm bench_out/a/b/hello.txt` | 3.248 |  | 0.015 | 0.518 | 0.009 | 0.680 | | 0.022 | 0.421 | yes |
| `rmdir bench_out/a/b` | 3.556 |  | 0.014 | 0.387 | 0.013 | 0.461 | | 0.018 | 0.344 | yes |
| `rm -r bench_out/dir_moved` | 8.472 |  | 0.599 | 3.588 | 0.194 | 0.807 | | 0.163 | 0.450 | yes |
| `rm -rf bench_out` | 26.89 |  | 3.563 | 2.277 | 0.214 | 1.438 | | 0.235 | 1.798 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 5, in memory | Stage 5, on disk, settled |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 288 B | 1681 B | 1681 B | 3643 B | 3643 B | 3643 B | 3456 B | 3456 B |
| `ls -a` | 338 B | 1849 B | 1849 B | 3941 B | 3941 B | 3941 B | 3850 B | 3850 B |
| `ls -R` | 25.0 KB | 38.2 KB | 38.2 KB | 46.5 KB | 46.5 KB | 46.5 KB | 46.4 KB | 46.4 KB |
| `ls -aR tests` | 4713 B | 12.5 KB | 12.5 KB | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB |
| `ls -p tests` | 118 B | 1309 B | 1309 B | 3406 B | 3406 B | 3406 B | 3411 B | 3411 B |
| `find .` | 57.3 KB | 21.4 KB | 21.4 KB | 23.2 KB | 23.2 KB | 23.2 KB | 23.1 KB | 23.1 KB |
| `find . -name '*.json'` | 12.4 KB | 19.1 KB | 19.1 KB | 21.2 KB | 21.2 KB | 21.2 KB | 21.3 KB | 21.3 KB |
| `find . -name '*.c'` | 12.3 KB | 15.0 KB | 15.0 KB | 17.1 KB | 17.1 KB | 17.1 KB | 17.2 KB | 17.2 KB |
| `find . -type d` | 2349 B | 5057 B | 5057 B | 7196 B | 7196 B | 7196 B | 7297 B | 7297 B |
| `find . -type f` | 54.9 KB | 26.7 KB | 26.7 KB | 28.9 KB | 28.9 KB | 28.9 KB | 29.0 KB | 29.0 KB |
| `find . -maxdepth 2 -type f` | 4257 B | 7643 B | 7643 B | 10.2 KB | 10.2 KB | 10.2 KB | 10.5 KB | 10.5 KB |
| `find tests -name '*.json' -type f` | 0 B | 1926 B | 1926 B | 4042 B | 4042 B | 4042 B | 4335 B | 4335 B |
| `find . -name 'Makefile'` | 462 B | 1916 B | 1916 B | 3976 B | 3976 B | 3976 B | 4077 B | 4077 B |
| `cat src/commands/xinfo-stream.json` | 17.8 KB | 1018 B | 1018 B | 2423 B | 2423 B | 2423 B | 2340 B | 2340 B |
| `cat -n src/commands/xinfo-stream.json` | 20.3 KB | 17.1 KB | 17.1 KB | 35.6 KB | 35.6 KB | 35.6 KB | 35.6 KB | 35.6 KB |
| `head -n 20 src/commands/xinfo-stream.json` | 640 B | 1534 B | 1534 B | 3620 B | 3620 B | 3620 B | 3721 B | 3721 B |
| `tail -n 20 src/commands/xinfo-stream.json` | 568 B | 1462 B | 1462 B | 3548 B | 3548 B | 3548 B | 3649 B | 3649 B |
| `tail -n +100 src/commands/xinfo-stream.json` | 13.9 KB | 14.8 KB | 14.8 KB | 16.9 KB | 16.9 KB | 16.9 KB | 17.0 KB | 17.0 KB |
| `wc src/commands/xinfo-stream.json` | 49 B | 1026 B | 1026 B | 2618 B | 2618 B | 2618 B | 2527 B | 2527 B |
| `wc -l src/commands/*.json` | 14.4 KB | 71.0 KB | 71.0 KB | 71.7 KB | 71.7 KB | 71.7 KB | 72.3 KB | 72.3 KB |
| `cat src/commands/*.json \| wc -l` | 6 B | 71.2 KB | 71.2 KB | 72.1 KB | 72.1 KB | 72.1 KB | 73.0 KB | 73.0 KB |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 261 B | 4591 B | 4815 B | 7495 B | 7495 B | 7495 B | 10.0 KB | 10.0 KB |
| `grep -c for src/commands/xinfo-stream.json` | 2 B | 5818 B | 5818 B | 10.1 KB | 10.1 KB | 10.1 KB | 10.3 KB | 10.3 KB |
| `grep -n for src/commands/xinfo-stream.json` | 330 B | 5818 B | 5818 B | 10.1 KB | 10.1 KB | 10.1 KB | 10.3 KB | 10.3 KB |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17.8 KB | 22.2 KB | 22.2 KB | 29.0 KB | 29.0 KB | 29.0 KB | 32.0 KB | 32.0 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17.8 KB | 23.8 KB | 23.8 KB | 45.4 KB | 45.4 KB | 45.4 KB | 49.0 KB | 49.0 KB |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 6 B | 24.4 KB | 24.6 KB | 28.0 KB | 28.0 KB | 28.0 KB | 31.9 KB | 31.9 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.7 KB | 31.7 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.7 KB | 31.7 KB |
| `grep -rn nestedcallreplyarray .` | 165 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 42.1 KB | 42.1 KB |
| `grep -rl nestedcallreplyarray .` | 25 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 42.1 KB | 42.1 KB |
| `grep -rn xmallctlmibnametomib .` | 206 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 42.1 KB | 42.1 KB |
| `grep -rl xmallctlmibnametomib .` | 76 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 42.1 KB | 42.1 KB |
| `grep -rn implementing .` | 2538 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.3 KB | 29.3 KB |
| `grep -rl implementing .` | 371 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.3 KB | 29.3 KB |
| `grep -rn between .` | 45.4 KB | 23.7 KB | 25.7 KB | 27.9 KB | 27.9 KB | 27.9 KB | 28.0 KB | 28.0 KB |
| `grep -rl between .` | 4657 B | 23.4 KB | 25.3 KB | 27.5 KB | 27.5 KB | 27.5 KB | 27.6 KB | 27.6 KB |
| `grep -rn for .` | 1815.1 KB | 25.5 KB | 25.5 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.9 KB | 27.9 KB |
| `grep -rl for .` | 35.6 KB | 5758 B | 5758 B | 9977 B | 9977 B | 9977 B | 10.2 KB | 10.2 KB |
| `grep -rc implementing .` | 58.3 KB | 45.1 KB | 46.2 KB | 48.4 KB | 48.4 KB | 48.4 KB | 48.5 KB | 48.5 KB |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 206 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 42.1 KB | 42.1 KB |
| `grep -ri implementing .` | 2754 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.3 KB | 29.3 KB |
| `grep -rn implementing tests` | 200 B | 17.1 KB | 17.1 KB | 26.5 KB | 26.5 KB | 26.5 KB | 26.7 KB | 26.7 KB |
| `grep -rl between tests` | 722 B | 10.6 KB | 10.6 KB | 17.2 KB | 17.2 KB | 17.2 KB | 17.4 KB | 17.4 KB |
| `grep -rF 'implementing' .` | 2435 B | 25.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.3 KB | 29.3 KB |
| `grep -rE 'arena_[a-z]+\(' .` | 24.7 KB | 27.5 KB | 28.0 KB | 30.2 KB | 30.2 KB | 30.2 KB | 30.3 KB | 30.3 KB |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 355 B | 53.1 KB | 53.1 KB | 78.8 KB | 78.8 KB | 78.8 KB | 79.0 KB | 79.0 KB |
| `grep -rE 'between\|implementing' .` | 46.2 KB | 26.3 KB | 26.3 KB | 39.9 KB | 39.9 KB | 39.9 KB | 40.1 KB | 40.1 KB |
| `grep -rE '[0-9]{4,}' .` | 798.7 KB | 33.9 KB | 33.9 KB | 36.1 KB | 36.1 KB | 36.1 KB | 36.3 KB | 36.3 KB |
| `grep -rn '^#include' .` | 172.4 KB | 19.6 KB | 23.2 KB | 23.0 KB | 25.4 KB | 23.0 KB | 23.2 KB | 25.5 KB |
| `grep -rE '^(import\|from) ' .` | 8992 B | 21.0 KB | 21.0 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.3 KB | 30.3 KB |
| `grep -rin 'return [a-z_]*;$' tests` | 53.9 KB | 25.0 KB | 27.6 KB | 26.7 KB | 29.9 KB | 26.7 KB | 26.8 KB | 30.0 KB |
| `grep -rv e .github` | 14.9 KB | 10.8 KB | 10.8 KB | 13.0 KB | 13.0 KB | 13.0 KB | 13.1 KB | 13.1 KB |
| `grep -r '\(implementing\).*\1' .` | 0 B | 30.1 KB | 31.1 KB | 33.3 KB | 33.3 KB | 33.3 KB | 33.4 KB | 33.4 KB |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 0 B | | 71.3 KB | 72.3 KB | 72.3 KB | 72.3 KB | 73.1 KB | 73.1 KB |
| `grep -l implementing src/commands/*.json` | 0 B | | 71.3 KB | 72.2 KB | 72.2 KB | 72.2 KB | 73.0 KB | 73.0 KB |
| `grep -c qdbzzabsentterm src/commands/*.json` | 12.4 KB | | 71.3 KB | 72.3 KB | 72.3 KB | 72.3 KB | 73.0 KB | 73.0 KB |
| `grep -rl between . \| wc -l` | 4 B | 24.1 KB | 26.2 KB | 28.5 KB | 28.5 KB | 28.5 KB | 30.1 KB | 30.1 KB |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 5 B | 8971 B | 9195 B | 13.5 KB | 13.5 KB | 13.5 KB | 16.4 KB | 16.4 KB |
| `find . -name '*.json' \| wc -l` | 4 B | 19.9 KB | 20.0 KB | 22.2 KB | 22.2 KB | 22.2 KB | 23.7 KB | 23.7 KB |
| `find . -type f \| grep -c json` | 4 B | 34.5 KB | 34.6 KB | 39.8 KB | 39.8 KB | 39.8 KB | 41.7 KB | 41.7 KB |
| `grep -rn for . \| head -n 5` | 502 B | 6624 B | 6736 B | 11.2 KB | 11.2 KB | 11.2 KB | 13.0 KB | 13.0 KB |
| `grep -rn between . \| head -n 1` | 191 B | 11.5 KB | 11.6 KB | 18.4 KB | 18.4 KB | 18.4 KB | 20.1 KB | 20.1 KB |
| `find . -type f \| head -n 3` | 82 B | 27.6 KB | 27.7 KB | 30.1 KB | 30.1 KB | 30.1 KB | 31.8 KB | 31.8 KB |
| `cat src/commands/*.json \| head -n 2` | 15 B | 71.3 KB | 71.3 KB | 72.3 KB | 72.3 KB | 72.3 KB | 73.3 KB | 73.3 KB |
| `grep -rno implementing .` | 961 B | | | 37.3 KB | 37.3 KB | 37.3 KB | 37.5 KB | 37.5 KB |
| `grep -rw implementing .` | 2435 B | | | 41.0 KB | 41.0 KB | 41.0 KB | 41.1 KB | 41.1 KB |
| `grep -rnw xmallctlmibnametomib .` | 206 B | | | 49.2 KB | 49.2 KB | 49.2 KB | 49.4 KB | 49.4 KB |
| `grep -rn -C 2 xmallctlmibnametomib .` | 768 B | | | 42.6 KB | 42.6 KB | 42.6 KB | 43.0 KB | 43.0 KB |
| `grep -rn -A 1 -B 1 implementing .` | 6398 B | | | 30.4 KB | 30.4 KB | 30.4 KB | 30.9 KB | 30.9 KB |
| `grep -rho 'arena_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 210 B | | | 211.6 KB | 211.6 KB | 211.6 KB | 214.9 KB | 214.9 KB |
| `grep -rhow '[a-z]*xmallctlmibnametomib[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 29 B | | | 74.1 KB | 74.1 KB | 74.1 KB | 77.6 KB | 77.6 KB |
| `rg -j 1 qdbzzabsentterm` | 0 B | | | 30.8 KB | 30.8 KB | 30.8 KB | 30.9 KB | 30.9 KB |
| `rg -j 1 nestedcallreplyarray` | 152 B | | | 40.8 KB | 40.8 KB | 40.8 KB | 41.0 KB | 41.0 KB |
| `rg -j 1 xmallctlmibnametomib` | 195 B | | | 101.7 KB | 101.7 KB | 101.7 KB | 101.8 KB | 101.8 KB |
| `rg -j 1 implementing` | 2383 B | | | 117.6 KB | 117.6 KB | 117.6 KB | 117.8 KB | 117.8 KB |
| `rg -j 1 between` | 42.7 KB | | | 139.3 KB | 139.3 KB | 139.3 KB | 139.4 KB | 139.4 KB |
| `rg -j 1 for` | 1694.2 KB | | | 145.0 KB | 145.0 KB | 145.0 KB | 145.1 KB | 145.1 KB |
| `rg -j 1 -l implementing` | 337 B | | | 115.3 KB | 115.3 KB | 115.3 KB | 115.5 KB | 115.5 KB |
| `rg -j 1 -l for` | 32.9 KB | | | 144.7 KB | 144.7 KB | 144.7 KB | 144.9 KB | 144.9 KB |
| `rg -j 1 -c between` | 4622 B | | | 119.0 KB | 119.0 KB | 119.0 KB | 119.2 KB | 119.2 KB |
| `rg -j 1 -i XMALLCTLMIBNAMETOMIB` | 195 B | | | 124.7 KB | 124.7 KB | 124.7 KB | 125.0 KB | 125.0 KB |
| `rg -j 1 -w implementing` | 2383 B | | | 141.4 KB | 141.4 KB | 141.4 KB | 141.7 KB | 141.7 KB |
| `rg -j 1 -F 'implementing'` | 2383 B | | | 118.1 KB | 118.1 KB | 118.1 KB | 118.3 KB | 118.3 KB |
| `rg -j 1 -n -C 2 xmallctlmibnametomib` | 748 B | | | 103.7 KB | 103.7 KB | 103.7 KB | 104.2 KB | 104.2 KB |
| `rg -j 1 'arena_[a-z]+\('` | 24.2 KB | | | 137.5 KB | 137.5 KB | 137.5 KB | 137.7 KB | 137.7 KB |
| `rg -j 1 'xmallctlmibnametomib\|nestedcallreplyarray'` | 347 B | | | 116.0 KB | 116.0 KB | 116.0 KB | 116.2 KB | 116.2 KB |
| `rg -j 1 '\bxmallctlmibnametomib\w*'` | 195 B | | | 2622.7 KB | 2622.7 KB | 2622.7 KB | 2622.9 KB | 2622.9 KB |
| `rg -j 1 -t c implementing` | 2288 B | | | 119.6 KB | 119.6 KB | 119.6 KB | 120.0 KB | 120.0 KB |
| `rg -j 1 -g '*.json' implementing` | 0 B | | | 115.3 KB | 115.3 KB | 115.3 KB | 115.7 KB | 115.7 KB |
| `rg -j 1 implementing tests` | 195 B | | | 32.4 KB | 32.4 KB | 32.4 KB | 32.6 KB | 32.6 KB |
| `rg -j 1 -uu implementing` | 2383 B | | | 36.9 KB | 36.9 KB | 36.9 KB | 37.2 KB | 37.2 KB |
| `rg -j 1 --sort path xmallctlmibnametomib` | 195 B | | | 102.4 KB | 102.4 KB | 102.4 KB | 102.7 KB | 102.7 KB |
| `rg -j 1 --files \| wc -l` | 5 B | | | 143.0 KB | 143.0 KB | 143.0 KB | 144.6 KB | 144.6 KB |
| `rg -j 1 -l between \| wc -l` | 4 B | | | 120.0 KB | 120.0 KB | 120.0 KB | 121.7 KB | 121.7 KB |
| `ls -ltR` | 94.8 KB | | | 70.9 KB | 70.9 KB | 70.9 KB | 70.8 KB | 70.8 KB |
| `ls -lt src/commands` | 22.6 KB | | | 48.6 KB | 48.6 KB | 48.6 KB | 48.6 KB | 48.6 KB |
| `ls -ltr tests \| tail -n 3` | 145 B | | | 5454 B | 5454 B | 5454 B | 7003 B | 7003 B |
| `find . -newer src/commands/xinfo-stream.json` | 13.7 KB | | | 27.9 KB | 27.9 KB | 27.9 KB | 28.0 KB | 28.0 KB |
| `find . -newer src/commands/xinfo-stream.json -name '*.json'` | 0 B | | | 4460 B | 4460 B | 4460 B | 4753 B | 4753 B |
| `find . -type f -mtime +30` | 0 B | | | 4081 B | 4081 B | 4081 B | 4374 B | 4374 B |
| `du -s .` | 8 B | | | 2695 B | 2695 B | 2695 B | 2700 B | 2700 B |
| `du -sh tests` | 11 B | | | 2724 B | 2724 B | 2724 B | 2729 B | 2729 B |
| `du -sb .` | 11 B | | | 2707 B | 2707 B | 2707 B | 2712 B | 2712 B |
| `du -sb tests` | 14 B | | | 2725 B | 2725 B | 2725 B | 2730 B | 2730 B |
| `du -b -d 1 tests` | 191 B | | | 3873 B | 3873 B | 3873 B | 4070 B | 4070 B |
| `sort src/commands/xinfo-stream.json` | 17.8 KB | | | 99.3 KB | 99.3 KB | 99.3 KB | 99.3 KB | 99.3 KB |
| `sort -r src/commands/xinfo-stream.json \| head -n 3` | 10 B | | | 100.8 KB | 100.8 KB | 100.8 KB | 102.4 KB | 102.4 KB |
| `sort -u src/commands/xinfo-stream.json \| wc -l` | 4 B | | | 111.3 KB | 111.3 KB | 111.3 KB | 112.8 KB | 112.8 KB |
| `sort -k 2 src/commands/xinfo-stream.json \| tail -n 3` | 73 B | | | 118.7 KB | 118.7 KB | 118.7 KB | 120.4 KB | 120.4 KB |
| `cat src/commands/*.json \| sort \| uniq -c \| sort -rn \| head -n 5` | 116 B | | | 2316.4 KB | 2316.4 KB | 2316.4 KB | 2319.7 KB | 2319.7 KB |
| `uniq -c src/commands/xinfo-stream.json \| wc -l` | 4 B | | | 36.9 KB | 36.9 KB | 36.9 KB | 38.3 KB | 38.3 KB |
| `cut -c 1-20 src/commands/xinfo-stream.json` | 7374 B | | | 19.3 KB | 19.3 KB | 19.3 KB | 19.4 KB | 19.4 KB |
| `cut -d ' ' -f 1 src/commands/xinfo-stream.json \| sort \| uniq \| wc -l` | 2 B | | | 84.4 KB | 84.4 KB | 84.4 KB | 87.9 KB | 87.9 KB |
| `tr a-z A-Z < src/commands/xinfo-stream.json` | 17.8 KB | | | 22.8 KB | 22.8 KB | 22.8 KB | 22.9 KB | 22.9 KB |
| `tr -d '\n' < src/commands/xinfo-stream.json \| wc -c` | 6 B | | | 23.8 KB | 23.8 KB | 23.8 KB | 25.3 KB | 25.3 KB |
| `tr -s ' ' < src/commands/xinfo-stream.json \| wc -c` | 5 B | | | 30.3 KB | 30.3 KB | 30.3 KB | 31.9 KB | 31.9 KB |
| `nl src/commands/xinfo-stream.json` | 20.3 KB | | | 35.4 KB | 35.4 KB | 35.4 KB | 35.3 KB | 35.3 KB |
| `nl -ba src/commands/xinfo-stream.json \| tail -n 3` | 39 B | | | 57.2 KB | 57.2 KB | 57.2 KB | 58.8 KB | 58.8 KB |
| `seq 1 100000 \| wc -l` | 7 B | | | 135.0 KB | 135.0 KB | 135.0 KB | 136.4 KB | 136.4 KB |
| `seq -w 1 1000 \| tail -n 2` | 10 B | | | 17.6 KB | 17.6 KB | 17.6 KB | 19.2 KB | 19.2 KB |
| `basename src/commands/xinfo-stream.json` | 18 B | | | 2478 B | 2478 B | 2478 B | 2387 B | 2387 B |
| `basename -s .json src/commands/xinfo-stream.json` | 13 B | | | 3136 B | 3136 B | 3136 B | 3237 B | 3237 B |
| `dirname src/commands/xinfo-stream.json` | 13 B | | | 2467 B | 2467 B | 2467 B | 2376 B | 2376 B |
| `printf '%s\n' a b c` | 6 B | | | 3325 B | 3325 B | 3325 B | 3522 B | 3522 B |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 18 B | | | 3422 B | 3422 B | 3422 B | 3619 B | 3619 B |
| `test -f src/commands/xinfo-stream.json && echo yes` | 4 B | | | 3563 B | 3563 B | 3563 B | 3848 B | 3848 B |
| `[ -d tests ] && echo folder` | 7 B | | | 3764 B | 3764 B | 3764 B | 4145 B | 4145 B |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 35 B | | | 3492 B | 3492 B | 3492 B | 3709 B | 3709 B |
| `echo a b c \| xargs -n 1 echo` | 6 B | | | 4674 B | 4674 B | 4674 B | 6367 B | 6367 B |
| `find .github -name '*.json' \| xargs cat \| wc -l` | 2 B | | | 5069 B | 5069 B | 5069 B | 7582 B | 7582 B |
| `find . -name '*.json' \| xargs grep -l implementing \| wc -l` | 2 B | | | 181.6 KB | 181.6 KB | 181.6 KB | 116.8 KB | 116.8 KB |
| `echo {a,b,c}{1..3}` | 27 B | | | 4485 B | 4485 B | 4485 B | 5917 B | 5917 B |
| `echo {1..20..3} {a..e}` | 28 B | | | 3967 B | 3967 B | 3967 B | 5303 B | 5303 B |
| `echo $(wc -l < src/commands/xinfo-stream.json) `basename src/commands/xinfo-stream.json`` | 22 B | | | 6425 B | 6425 B | 6425 B | 8011 B | 8011 B |
| `echo "first line: $(head -n 1 src/commands/xinfo-stream.json)"` | 14 B | | | 4996 B | 4996 B | 4996 B | 6677 B | 6677 B |
| `wc -l < src/commands/xinfo-stream.json` | 4 B | | | 3348 B | 3348 B | 3348 B | 3369 B | 3369 B |
| `cat src/commands/xinfo-stream.json > /dev/null 2>&1` | 0 B | | | 3449 B | 3449 B | 3449 B | 3486 B | 3486 B |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 6 B | | | 4158 B | 4158 B | 4158 B | 4419 B | 4419 B |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2 B | | | 4072 B | 4072 B | 4072 B | 5445 B | 5445 B |
| `grep -q for src/commands/xinfo-stream.json && echo found \|\| echo none` | 6 B | | | 11.7 KB | 11.7 KB | 11.7 KB | 12.5 KB | 12.5 KB |
| `false \|\| echo fallback` | 9 B | | | 2861 B | 2861 B | 2861 B | 2970 B | 2970 B |
| `true && grep -c for src/commands/xinfo-stream.json` | 2 B | | | 10.5 KB | 10.5 KB | 10.5 KB | 10.9 KB | 10.9 KB |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 14 B | | | 4178 B | 4178 B | 4178 B | 5376 B | 5376 B |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2 B | | | 3130 B | 3130 B | 3130 B | 3079 B | 3079 B |
| `echo one \⏎two` | 8 B | | | 2591 B | 2591 B | 2591 B | 2596 B | 2596 B |
| `grep -c for src/commands/xinfo-stream.json⏎wc -l src/commands/xinfo-stream.json` | 37 B | | | 11.2 KB | 11.2 KB | 11.2 KB | 11.7 KB | 11.7 KB |
| `for i in $(seq 100000); do :; done` | 0 B | | | | | | 7815.6 KB | 7815.6 KB |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 6 B | | | | | | 7252 B | 7252 B |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 2 B | | | | | | 7249 B | 7249 B |
| `for f in src/commands/*.json; do echo "$f"; done \| wc -l` | 4 B | | | | | | 48.0 KB | 48.0 KB |
| `for f in src/commands/*.json; do grep -c for "$f"; done \| sort -n \| tail -n 3` | 6 B | | | | | | 97.1 KB | 97.1 KB |
| `n=0; for f in src/commands/*.json; do if grep -q between "$f"; then n=$((n+1)); fi; done; echo $n` | 2 B | | | | | | 49.3 KB | 49.3 KB |
| `for f in src/commands/*.json; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 0 B | | | | | | 48.3 KB | 48.3 KB |
| `H=$PWD; for d in tests .github src/commands; do cd "$d"; ls \| wc -l; cd "$H"; done` | 9 B | | | | | | 35.2 KB | 35.2 KB |
| `X=for; N=$(grep -c "$X" src/commands/xinfo-stream.json); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 11 B | | | | | | 13.8 KB | 13.8 KB |
| `find .github -name '*.json' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 0 B | | | | | | 77.9 KB | 77.9 KB |
| `for f in src/commands/*.json; do head -n 1 "$f"; done > /dev/null` | 0 B | | | | | | 47.3 KB | 47.3 KB |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 12 B | | | | | | 9721 B | 9721 B |
| `awk '{ print $2 }' src/commands/xinfo-stream.json` | 1587 B | | | | | | 9957 B | 9957 B |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 6 B | | | | | | 128.5 KB | 128.5 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '/for/ { n++ } END { print n }'` | 4 B | | | | | | 122.7 KB | 122.7 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 12 B | | | | | | 121.5 KB | 121.5 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 2 B | | | | | | 119.9 KB | 119.9 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 2 B | | | | | | 120.4 KB | 120.4 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 4 B | | | | | | 119.7 KB | 119.7 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 4 B | | | | | | 207.7 KB | 207.7 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "for") n++ } END { print n }'` | 4 B | | | | | | 122.3 KB | 122.3 KB |
| `awk '{ gsub(/[0-9]+/, "N"); print }' src/commands/xinfo-stream.json \| wc -c` | 6 B | | | | | | 36.8 KB | 36.8 KB |
| `awk -v OFS=- '{ $1 = NR; print }' src/commands/xinfo-stream.json \| tail -n 1` | 4 B | | | | | | 20.2 KB | 20.2 KB |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 16 B | | | | | | 5894 B | 5894 B |
| `grep -rn implementing . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 156 B | | | | | | 81.3 KB | 81.3 KB |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 144 B | | | | | | 308.9 KB | 308.9 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1459 B | 1747 B | 3711 B | 3711 B | | 3716 B | 3716 B |
| `touch bench_out/empty.txt` | 0 B | 979 B | 979 B | 2566 B | 2566 B | | 2475 B | 2475 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1242 B | 1292 B | 3665 B | 3661 B | | 3710 B | 3706 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1246 B | 1246 B | 3485 B | 3473 B | | 3530 B | 3518 B |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0 B | 19.5 KB | 36.8 KB | 3294 B | 3294 B | | 3299 B | 3299 B |
| `cp -r tests bench_out/dir_copy` | 0 B | 3535.7 KB | 1932.8 KB | 125.9 KB | 177.2 KB | | 126.0 KB | 177.4 KB |
| `cp -r src/commands bench_out/work` | 0 B | 993.0 KB | 343.8 KB | 168.0 KB | 222.4 KB | | 168.1 KB | 222.7 KB |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 25.4 KB | 42.9 KB | 46.3 KB | 46.3 KB | | 49.4 KB | 49.4 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 25.1 KB | 44.8 KB | 48.4 KB | 48.4 KB | | 50.8 KB | 50.8 KB |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 0 B | 519.9 KB | 128.2 KB | 129.9 KB | 311.7 KB | | 133.0 KB | 314.8 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | 0 B | | 82.9 KB | 85.0 KB | 85.0 KB | | 85.8 KB | 85.8 KB |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | 0 B | | 86.5 KB | 88.6 KB | 88.6 KB | | 89.5 KB | 89.5 KB |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 B | | 18.7 KB | 2874 B | 2874 B | | 2879 B | 2879 B |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 B | | 18.7 KB | 2874 B | 2874 B | | 2879 B | 2879 B |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | 0 B | | 18.9 KB | 21.3 KB | 21.3 KB | | 21.3 KB | 21.3 KB |
| `grep -rc QDBBENCHMANY bench_out` | 25.7 KB | 17.1 KB | 17.1 KB | 26.5 KB | 26.5 KB | | 26.7 KB | 26.7 KB |
| `grep -rl QDBBENCHMANY .` | 4346 B | 19.5 KB | 21.1 KB | 26.5 KB | 26.5 KB | | 26.7 KB | 26.7 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | | 25.0 KB | 25.0 KB |
| `cat src/commands/xinfo-stream.json \| tee bench_out/tee.txt \| wc -l` | 4 B | | | 40.3 KB | 40.3 KB | | 42.7 KB | 42.7 KB |
| `sort src/commands/xinfo-stream.json > bench_out/sorted.txt` | 0 B | | | 117.9 KB | 117.9 KB | | 117.9 KB | 117.9 KB |
| `sort -u src/commands/xinfo-stream.json >> bench_out/sorted.txt` | 0 B | | | 111.2 KB | 111.2 KB | | 111.3 KB | 111.3 KB |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 B | | | 4144 B | 3842 B | | 4189 B | 3887 B |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 0 B | | | 3756 B | 3714 B | | 3801 B | 3759 B |
| `find .github -name '*.json' \| sort \| xargs cat > bench_out/joined.txt` | 0 B | | | 70.9 KB | 70.9 KB | | 73.4 KB | 73.4 KB |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 B | | | 3790 B | 3790 B | | 3795 B | 3795 B |
| `sed -i '1d' bench_out/joined2.txt` | 0 B | | | 3550 B | 3550 B | | 3651 B | 3651 B |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 118 B | | | 4165 B | 4165 B | | 4458 B | 4458 B |
| `cat bench_out/both.txt` | 98 B | | | 2497 B | 2497 B | | 2406 B | 2406 B |
| `cp src/commands/xinfo-stream.json bench_out/v2.txt` | 0 B | | | | | | 3123 B | 3123 B |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 0 B | | | | | | 46.1 KB | 46.1 KB |
| `diff -u --label old --label new src/commands/xinfo-stream.json bench_out/v2.txt` | 903 B | | | | | | 20.8 KB | 20.8 KB |
| `diff src/commands/xinfo-stream.json bench_out/v2.txt \| wc -l` | 3 B | | | | | | 20.6 KB | 20.6 KB |
| `diff -q src/commands/xinfo-stream.json bench_out/same.txt` | 0 B | | | | | | 3331 B | 3331 B |
| `diff -u src/commands/xinfo-stream.json bench_out/copy.txt \| wc -l` | 4 B | | | | | | 65.9 KB | 65.9 KB |
| `diff -rq tests bench_out/dir_copy` | 0 B | | | | | | 10.5 KB | 10.5 KB |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 0 B | | | | | | 132.8 KB | 132.8 KB |
| `diff -rq tests bench_out/dir_copy` | 62 B | | | | | | 10.5 KB | 10.5 KB |
| `diff -r src/commands bench_out/work \| wc -l` | 4 B | | | | | | 86.6 KB | 86.6 KB |
| `diff -ru src/commands bench_out/work \| grep -c '^@@'` | 4 B | | | | | | 88.7 KB | 88.7 KB |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 0 B | | | | | | 3697.0 KB | 1755.2 KB |
| `wc -l bench_out/loop.txt` | 25 B | | | | | | 2829 B | 2829 B |
| `for f in bench_out/work/*.json; do echo done >> "$f"; done` | 0 B | | | | | | 46.7 KB | 46.7 KB |
| `awk '{ print NR ": " $0 }' src/commands/xinfo-stream.json > bench_out/numbered.txt` | 0 B | | | | | | 50.8 KB | 50.8 KB |
| `awk 'END { print NR }' bench_out/numbered.txt` | 4 B | | | | | | 5754 B | 5754 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1191 B | 1191 B | 2900 B | 2900 B | | 2905 B | 2905 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1288 B | 1288 B | 3250 B | 3250 B | | 3255 B | 3255 B |
| `ls -R bench_out/a` | 41 B | 1089 B | 1089 B | 3092 B | 3092 B | | 3097 B | 3097 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 997 B | 997 B | 2504 B | 2504 B | | 2413 B | 2413 B |
| `rmdir bench_out/a/b` | 0 B | 945 B | 945 B | 2383 B | 2383 B | | 2292 B | 2292 B |
| `rm -r bench_out/dir_moved` | 0 B | 1129 B | 1129 B | 2707 B | 2707 B | | 2712 B | 2712 B |
| `rm -rf bench_out` | 0 B | 3586 B | 3394 B | 5306 B | 5306 B | | 4477 B | 4477 B |

## What each command used (Stage 5, on disk, settled)

Counted by the engine for the first run of the command; the two times are that run's, measured. Bytes unless said otherwise.

| Command | Text read | Pieces | Index read | Index lookups | Tree entries | Content written | Records written | Upkeep written | Syncs | Scratch | Memory | CPU, ms | Elapsed, ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 0 | 0 | 0 | 0 | 29 | 0 | 421 | 0 | 1 | 0 | 3196 | 0.458 | 1.953 |
| `ls -a` | 0 | 0 | 0 | 0 | 29 | 0 | 0 | 0 | 0 | 0 | 3590 | 0.018 | 0.017 |
| `ls -R` | 0 | 0 | 0 | 0 | 1745 | 0 | 0 | 0 | 0 | 0 | 46114 | 0.180 | 0.179 |
| `ls -aR tests` | 0 | 0 | 0 | 0 | 284 | 0 | 0 | 0 | 0 | 0 | 15326 | 0.091 | 0.091 |
| `ls -p tests` | 0 | 0 | 0 | 0 | 13 | 0 | 0 | 0 | 0 | 0 | 3151 | 0.016 | 0.016 |
| `find .` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 22826 | 0.331 | 0.331 |
| `find . -name '*.json'` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 21033 | 0.337 | 0.337 |
| `find . -name '*.c'` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 16927 | 0.333 | 0.333 |
| `find . -type d` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 7037 | 0.257 | 0.257 |
| `find . -type f` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 28729 | 0.312 | 0.312 |
| `find . -maxdepth 2 -type f` | 0 | 0 | 0 | 0 | 279 | 0 | 0 | 0 | 0 | 0 | 10273 | 0.063 | 0.063 |
| `find tests -name '*.json' -type f` | 0 | 0 | 0 | 0 | 284 | 0 | 0 | 0 | 0 | 0 | 4075 | 0.072 | 0.070 |
| `find . -name 'Makefile'` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 3817 | 0.258 | 0.258 |
| `cat src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 2080 | 0.037 | 0.037 |
| `cat -n src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 35308 | 0.092 | 0.091 |
| `head -n 20 src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3461 | 0.019 | 0.019 |
| `tail -n 20 src/commands/xinfo-stream.json` | 53403 | 3 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3389 | 0.017 | 0.016 |
| `tail -n +100 src/commands/xinfo-stream.json` | 35602 | 2 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 16744 | 0.045 | 0.045 |
| `wc src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 2267 | 0.046 | 0.046 |
| `wc -l src/commands/*.json` | 0 | 0 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 72076 | 0.420 | 0.419 |
| `cat src/commands/*.json \| wc -l` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 72725 | 0.465 | 0.465 |
| `cat src/commands/xinfo-stream.json \| head -n 50 \| tail -n 10` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 9748 | 0.031 | 0.031 |
| `grep -c for src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 10026 | 0.055 | 0.055 |
| `grep -n for src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 10026 | 0.045 | 0.045 |
| `sed 's/for/XX/g' src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 31785 | 0.125 | 0.124 |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 48705 | 0.188 | 0.187 |
| `cat src/commands/xinfo-stream.json \| sed 's/^/> /' \| wc -c` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 31612 | 0.185 | 0.185 |
| `grep -rn qdbzzabsentterm .` | 0 | 0 | 0 | 26 | 32 | 0 | 0 | 0 | 0 | 0 | 31432 | 0.102 | 0.102 |
| `grep -rl qdbzzabsentterm .` | 0 | 0 | 0 | 26 | 32 | 0 | 0 | 0 | 0 | 0 | 31432 | 0.094 | 0.093 |
| `grep -rn nestedcallreplyarray .` | 279203 | 28 | 10351 | 36 | 49 | 0 | 0 | 0 | 0 | 0 | 41831 | 0.217 | 0.217 |
| `grep -rl nestedcallreplyarray .` | 279203 | 28 | 10351 | 36 | 49 | 0 | 0 | 0 | 0 | 0 | 41831 | 0.222 | 0.222 |
| `grep -rn xmallctlmibnametomib .` | 184256 | 23 | 6147 | 34 | 49 | 0 | 0 | 0 | 0 | 0 | 41831 | 0.192 | 0.191 |
| `grep -rl xmallctlmibnametomib .` | 184256 | 23 | 6147 | 34 | 49 | 0 | 0 | 0 | 0 | 0 | 41831 | 0.163 | 0.163 |
| `grep -rn implementing .` | 9187946 | 278 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 29019 | 1.682 | 1.682 |
| `grep -rl implementing .` | 8604984 | 268 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 29019 | 1.470 | 1.470 |
| `grep -rn between .` | 7349125 | 227 | 1343 | 10 | 49 | 0 | 0 | 0 | 0 | 0 | 27755 | 1.559 | 1.560 |
| `grep -rl between .` | 4833164 | 180 | 1343 | 10 | 49 | 0 | 0 | 0 | 0 | 0 | 27375 | 0.537 | 0.537 |
| `grep -rn for .` | 17378691 | 1770 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 27591 | 6.582 | 6.583 |
| `grep -rl for .` | 12877317 | 1672 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 9938 | 0.841 | 0.941 |
| `grep -rc implementing .` | 9187946 | 278 | 9477 | 20 | 1770 | 0 | 0 | 0 | 0 | 0 | 48245 | 2.078 | 2.079 |
| `grep -rin XMALLCTLMIBNAMETOMIB .` | 184256 | 23 | 6147 | 34 | 49 | 0 | 0 | 0 | 0 | 0 | 41832 | 0.270 | 0.269 |
| `grep -ri implementing .` | 9187946 | 278 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 29007 | 2.077 | 2.078 |
| `grep -rn implementing tests` | 788264 | 68 | 9477 | 20 | 285 | 0 | 0 | 0 | 0 | 0 | 26446 | 0.241 | 0.241 |
| `grep -rl between tests` | 815756 | 36 | 1343 | 10 | 285 | 0 | 0 | 0 | 0 | 0 | 17141 | 0.222 | 0.222 |
| `grep -rF 'implementing' .` | 9187946 | 278 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 29017 | 1.986 | 1.986 |
| `grep -rE 'arena_[a-z]+\(' .` | 1358939 | 111 | 2191 | 8 | 49 | 0 | 0 | 0 | 0 | 0 | 30072 | 0.841 | 0.841 |
| `grep -rE 'xmallctlmibnametomib\|nestedcallreplyarray' .` | 520332 | 44 | 16498 | 70 | 63 | 0 | 0 | 0 | 0 | 0 | 78712 | 0.341 | 0.341 |
| `grep -rE 'between\|implementing' .` | 10676583 | 366 | 10820 | 30 | 1770 | 0 | 0 | 0 | 0 | 0 | 39850 | 2.570 | 2.570 |
| `grep -rE '[0-9]{4,}' .` | 17378691 | 1770 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 36012 | 25.87 | 25.89 |
| `grep -rn '^#include' .` | 9026487 | 674 | 5680 | 12 | 1770 | 0 | 0 | 0 | 0 | 0 | 25264 | 2.149 | 2.149 |
| `grep -rE '^(import\|from) ' .` | 17378691 | 1770 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 30062 | 4.075 | 4.075 |
| `grep -rin 'return [a-z_]*;$' tests` | 2160546 | 157 | 5109 | 10 | 285 | 0 | 0 | 0 | 0 | 0 | 29744 | 1.233 | 1.233 |
| `grep -rv e .github` | 71159 | 15 | 0 | 0 | 19 | 0 | 0 | 0 | 0 | 0 | 12875 | 0.144 | 0.144 |
| `grep -r '\(implementing\).*\1' .` | 9187946 | 278 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 33168 | 1.633 | 1.633 |
| `grep -n xmallctlmibnametomib src/commands/*.json` | 82829 | 47 | 6147 | 34 | 1623 | 0 | 0 | 0 | 0 | 0 | 72818 | 0.310 | 0.310 |
| `grep -l implementing src/commands/*.json` | 130001 | 83 | 9477 | 20 | 1623 | 0 | 0 | 0 | 0 | 0 | 72762 | 0.399 | 0.399 |
| `grep -c qdbzzabsentterm src/commands/*.json` | 0 | 0 | 0 | 26 | 1623 | 0 | 0 | 0 | 0 | 0 | 72777 | 0.362 | 0.362 |
| `grep -rl between . \| wc -l` | 4833164 | 180 | 1343 | 10 | 49 | 0 | 0 | 0 | 0 | 0 | 29804 | 0.633 | 0.633 |
| `grep -rc for . \| grep -v ':0$' \| wc -l` | 17378691 | 1770 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 16138 | 3.732 | 3.733 |
| `find . -name '*.json' \| wc -l` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 23462 | 0.320 | 0.320 |
| `find . -type f \| grep -c json` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 41429 | 0.400 | 0.400 |
| `grep -rn for . \| head -n 5` | 1661 | 6 | 0 | 0 | 40 | 0 | 0 | 0 | 0 | 0 | 12699 | 0.077 | 0.077 |
| `grep -rn between . \| head -n 1` | 1661 | 6 | 0 | 0 | 40 | 0 | 0 | 0 | 0 | 0 | 19887 | 0.084 | 0.083 |
| `find . -type f \| head -n 3` | 0 | 0 | 0 | 0 | 481 | 0 | 0 | 0 | 0 | 0 | 31490 | 0.171 | 0.171 |
| `cat src/commands/*.json \| head -n 2` | 17126 | 15 | 0 | 0 | 498 | 0 | 0 | 0 | 0 | 0 | 73081 | 0.271 | 0.271 |
| `grep -rno implementing .` | 9187946 | 278 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 37212 | 1.645 | 1.645 |
| `grep -rw implementing .` | 9187946 | 278 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 40815 | 1.794 | 1.795 |
| `grep -rnw xmallctlmibnametomib .` | 184256 | 23 | 6147 | 34 | 49 | 0 | 0 | 0 | 0 | 0 | 49144 | 0.323 | 0.323 |
| `grep -rn -C 2 xmallctlmibnametomib .` | 184812 | 24 | 6147 | 34 | 49 | 0 | 0 | 0 | 0 | 0 | 42739 | 0.197 | 0.197 |
| `grep -rn -A 1 -B 1 implementing .` | 10345539 | 307 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 30643 | 2.349 | 2.350 |
| `grep -rho 'arena_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 1358939 | 111 | 2191 | 8 | 49 | 0 | 0 | 0 | 0 | 0 | 214644 | 1.637 | 1.638 |
| `grep -rhow '[a-z]*xmallctlmibnametomib[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 184256 | 23 | 6147 | 34 | 49 | 0 | 0 | 0 | 0 | 0 | 77358 | 0.617 | 0.617 |
| `rg -j 1 qdbzzabsentterm` | 591 | 1 | 0 | 26 | 33 | 0 | 0 | 0 | 0 | 0 | 30677 | 0.175 | 0.175 |
| `rg -j 1 nestedcallreplyarray` | 172857 | 5 | 10351 | 36 | 16 | 0 | 0 | 0 | 0 | 0 | 40719 | 0.243 | 0.243 |
| `rg -j 1 xmallctlmibnametomib` | 114452 | 5 | 6147 | 34 | 36 | 0 | 0 | 0 | 0 | 0 | 101550 | 0.506 | 0.505 |
| `rg -j 1 implementing` | 9106087 | 261 | 9477 | 20 | 140 | 0 | 0 | 0 | 0 | 0 | 117514 | 2.703 | 2.704 |
| `rg -j 1 between` | 7342112 | 219 | 1343 | 10 | 140 | 0 | 0 | 0 | 0 | 0 | 139161 | 2.339 | 2.339 |
| `rg -j 1 for` | 17266677 | 1740 | 0 | 0 | 2117 | 0 | 0 | 0 | 0 | 0 | 144871 | 6.324 | 6.325 |
| `rg -j 1 -l implementing` | 8523125 | 251 | 9477 | 20 | 140 | 0 | 0 | 0 | 0 | 0 | 115268 | 1.892 | 1.891 |
| `rg -j 1 -l for` | 12765303 | 1642 | 0 | 0 | 2117 | 0 | 0 | 0 | 0 | 0 | 144657 | 2.243 | 2.244 |
| `rg -j 1 -c between` | 7342112 | 219 | 1343 | 10 | 140 | 0 | 0 | 0 | 0 | 0 | 118979 | 1.614 | 1.614 |
| `rg -j 1 -i XMALLCTLMIBNAMETOMIB` | 114452 | 5 | 6147 | 34 | 36 | 0 | 0 | 0 | 0 | 0 | 124756 | 0.853 | 0.853 |
| `rg -j 1 -w implementing` | 9106087 | 261 | 9477 | 20 | 140 | 0 | 0 | 0 | 0 | 0 | 141444 | 3.693 | 3.693 |
| `rg -j 1 -F 'implementing'` | 9106087 | 261 | 9477 | 20 | 140 | 0 | 0 | 0 | 0 | 0 | 118066 | 2.002 | 2.003 |
| `rg -j 1 -n -C 2 xmallctlmibnametomib` | 115008 | 6 | 6147 | 34 | 36 | 0 | 0 | 0 | 0 | 0 | 103900 | 0.445 | 0.445 |
| `rg -j 1 'arena_[a-z]+\('` | 1351715 | 99 | 2191 | 8 | 60 | 0 | 0 | 0 | 0 | 0 | 137461 | 1.375 | 1.414 |
| `rg -j 1 'xmallctlmibnametomib\|nestedcallreplyarray'` | 286718 | 9 | 16498 | 70 | 48 | 0 | 0 | 0 | 0 | 0 | 115924 | 0.664 | 0.664 |
| `rg -j 1 '\bxmallctlmibnametomib\w*'` | 114452 | 5 | 6147 | 34 | 36 | 0 | 0 | 0 | 0 | 0 | 2622607 | 6.472 | 6.473 |
| `rg -j 1 -t c implementing` | 6912746 | 201 | 9477 | 20 | 140 | 0 | 0 | 0 | 0 | 0 | 119710 | 2.255 | 2.256 |
| `rg -j 1 -g '*.json' implementing` | 2866 | 7 | 9477 | 20 | 1038 | 0 | 0 | 0 | 0 | 0 | 115402 | 1.786 | 1.786 |
| `rg -j 1 implementing tests` | 669476 | 27 | 9477 | 20 | 373 | 0 | 0 | 0 | 0 | 0 | 32343 | 0.811 | 0.810 |
| `rg -j 1 -uu implementing` | 9103221 | 254 | 9477 | 20 | 0 | 0 | 0 | 0 | 0 | 0 | 36918 | 1.669 | 1.669 |
| `rg -j 1 --sort path xmallctlmibnametomib` | 114452 | 5 | 6147 | 34 | 36 | 0 | 0 | 0 | 0 | 0 | 102472 | 0.378 | 0.378 |
| `rg -j 1 --files \| wc -l` | 3002 | 14 | 0 | 0 | 2117 | 0 | 0 | 0 | 0 | 0 | 144338 | 1.571 | 1.571 |
| `rg -j 1 -l between \| wc -l` | 4826151 | 172 | 1343 | 10 | 140 | 0 | 0 | 0 | 0 | 0 | 121398 | 0.984 | 1.098 |
| `ls -ltR` | 0 | 0 | 0 | 0 | 5206 | 0 | 0 | 0 | 0 | 0 | 70540 | 1.113 | 1.113 |
| `ls -lt src/commands` | 0 | 0 | 0 | 0 | 814 | 0 | 0 | 0 | 0 | 0 | 48323 | 0.278 | 0.277 |
| `ls -ltr tests \| tail -n 3` | 0 | 0 | 0 | 0 | 317 | 0 | 0 | 0 | 0 | 0 | 6743 | 0.070 | 0.070 |
| `find . -newer src/commands/xinfo-stream.json` | 0 | 0 | 0 | 0 | 1773 | 0 | 0 | 0 | 0 | 0 | 27766 | 0.373 | 0.373 |
| `find . -newer src/commands/xinfo-stream.json -name '*.json'` | 0 | 0 | 0 | 0 | 1773 | 0 | 0 | 0 | 0 | 0 | 4493 | 0.276 | 0.276 |
| `find . -type f -mtime +30` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 4114 | 0.241 | 0.241 |
| `du -s .` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 7257 | 0.091 | 0.091 |
| `du -sh tests` | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 2469 | 0.019 | 0.019 |
| `du -sb .` | 0 | 0 | 0 | 0 | 1770 | 0 | 0 | 0 | 0 | 0 | 8842 | 0.108 | 0.108 |
| `du -sb tests` | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 2470 | 0.014 | 0.013 |
| `du -b -d 1 tests` | 0 | 0 | 0 | 0 | 13 | 0 | 0 | 0 | 0 | 0 | 3810 | 0.020 | 0.020 |
| `sort src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 98993 | 0.119 | 0.119 |
| `sort -r src/commands/xinfo-stream.json \| head -n 3` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 102116 | 0.112 | 0.112 |
| `sort -u src/commands/xinfo-stream.json \| wc -l` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 112496 | 0.115 | 0.115 |
| `sort -k 2 src/commands/xinfo-stream.json \| tail -n 3` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 120111 | 0.134 | 0.134 |
| `cat src/commands/*.json \| sort \| uniq -c \| sort -rn \| head -n 5` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 2319489 | 5.412 | 5.632 |
| `uniq -c src/commands/xinfo-stream.json \| wc -l` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 38064 | 0.065 | 0.065 |
| `cut -c 1-20 src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 19132 | 0.073 | 0.073 |
| `cut -d ' ' -f 1 src/commands/xinfo-stream.json \| sort \| uniq \| wc -l` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 87617 | 0.120 | 0.120 |
| `tr a-z A-Z < src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 22648 | 0.046 | 0.046 |
| `tr -d '\n' < src/commands/xinfo-stream.json \| wc -c` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 25071 | 0.040 | 0.039 |
| `tr -s ' ' < src/commands/xinfo-stream.json \| wc -c` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 31610 | 0.043 | 0.043 |
| `nl src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 35041 | 0.096 | 0.096 |
| `nl -ba src/commands/xinfo-stream.json \| tail -n 3` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 58524 | 0.088 | 0.088 |
| `seq 1 100000 \| wc -l` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 136176 | 3.171 | 3.171 |
| `seq -w 1 1000 \| tail -n 2` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 18979 | 0.064 | 0.064 |
| `basename src/commands/xinfo-stream.json` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2127 | 0.013 | 0.012 |
| `basename -s .json src/commands/xinfo-stream.json` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2977 | 0.014 | 0.013 |
| `dirname src/commands/xinfo-stream.json` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2116 | 0.011 | 0.011 |
| `printf '%s\n' a b c` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3262 | 0.018 | 0.017 |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3359 | 0.021 | 0.021 |
| `test -f src/commands/xinfo-stream.json && echo yes` | 0 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3588 | 0.028 | 0.028 |
| `[ -d tests ] && echo folder` | 0 | 0 | 0 | 0 | 30 | 0 | 0 | 0 | 0 | 0 | 3885 | 0.028 | 0.028 |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3449 | 0.022 | 0.022 |
| `echo a b c \| xargs -n 1 echo` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 6107 | 0.027 | 0.026 |
| `find .github -name '*.json' \| xargs cat \| wc -l` | 0 | 0 | 0 | 0 | 18 | 0 | 0 | 0 | 0 | 0 | 7322 | 0.057 | 0.057 |
| `find . -name '*.json' \| xargs grep -l implementing \| wc -l` | 127887 | 82 | 9477 | 20 | 2988 | 0 | 0 | 0 | 0 | 0 | 116535 | 0.906 | 0.906 |
| `echo {a,b,c}{1..3}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5657 | 0.025 | 0.025 |
| `echo {1..20..3} {a..e}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5043 | 0.023 | 0.023 |
| `echo $(wc -l < src/commands/xinfo-stream.json) `basename src/commands/xinfo-stream.json`` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 7751 | 0.048 | 0.047 |
| `echo "first line: $(head -n 1 src/commands/xinfo-stream.json)"` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 6417 | 0.033 | 0.033 |
| `wc -l < src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3109 | 0.025 | 0.025 |
| `cat src/commands/xinfo-stream.json > /dev/null 2>&1` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 3226 | 0.020 | 0.019 |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 4159 | 0.025 | 0.025 |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 5185 | 0.025 | 0.025 |
| `grep -q for src/commands/xinfo-stream.json && echo found \|\| echo none` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 12219 | 0.056 | 0.056 |
| `false \|\| echo fallback` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2710 | 0.013 | 0.013 |
| `true && grep -c for src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 10678 | 0.049 | 0.049 |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5116 | 0.023 | 0.022 |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2819 | 0.014 | 0.014 |
| `echo one \⏎two` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2336 | 0.009 | 0.009 |
| `grep -c for src/commands/xinfo-stream.json⏎wc -l src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 9 | 0 | 0 | 0 | 0 | 0 | 11452 | 0.059 | 0.059 |
| `for i in $(seq 100000); do :; done` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 7815304 | 41.48 | 42.70 |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 0 | 0 | 0 | 0 | 290029 | 0 | 0 | 0 | 0 | 0 | 6992 | 25.37 | 25.37 |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 0 | 0 | 0 | 0 | 319 | 0 | 0 | 0 | 0 | 0 | 6989 | 0.060 | 0.059 |
| `for f in src/commands/*.json; do echo "$f"; done \| wc -l` | 0 | 0 | 0 | 0 | 408 | 0 | 0 | 0 | 0 | 0 | 47786 | 0.508 | 0.507 |
| `for f in src/commands/*.json; do grep -c for "$f"; done \| sort -n \| tail -n 3` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 96875 | 2.698 | 2.719 |
| `n=0; for f in src/commands/*.json; do if grep -q between "$f"; then n=$((n+1)); fi; done; echo $n` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 49080 | 3.927 | 4.130 |
| `for f in src/commands/*.json; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 0 | 0 | 0 | 0 | 13368 | 0 | 0 | 0 | 0 | 0 | 48063 | 0.953 | 0.953 |
| `H=$PWD; for d in tests .github src/commands; do cd "$d"; ls \| wc -l; cd "$H"; done` | 0 | 0 | 0 | 0 | 425 | 0 | 0 | 0 | 0 | 0 | 35023 | 0.104 | 0.104 |
| `X=for; N=$(grep -c "$X" src/commands/xinfo-stream.json); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 17801 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 13507 | 0.082 | 0.082 |
| `find .github -name '*.json' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 0 | 0 | 0 | 0 | 18 | 0 | 0 | 0 | 0 | 0 | 77612 | 0.058 | 0.057 |
| `for f in src/commands/*.json; do head -n 1 "$f"; done > /dev/null` | 738271 | 405 | 0 | 0 | 1623 | 0 | 0 | 0 | 0 | 0 | 47035 | 0.745 | 0.744 |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 0 | 0 | 0 | 0 | 145 | 0 | 0 | 0 | 0 | 0 | 9461 | 0.051 | 0.050 |
| `awk '{ print $2 }' src/commands/xinfo-stream.json` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 9697 | 0.081 | 0.081 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 128262 | 3.961 | 3.962 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '/for/ { n++ } END { print n }'` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 122407 | 1.864 | 1.865 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 121234 | 2.624 | 2.624 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 119606 | 3.112 | 3.113 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 120122 | 2.360 | 2.361 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 119436 | 1.732 | 1.733 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 207440 | 4.242 | 4.257 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "for") n++ } END { print n }'` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 122073 | 7.036 | 7.315 |
| `awk '{ gsub(/[0-9]+/, "N"); print }' src/commands/xinfo-stream.json \| wc -c` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 36526 | 0.339 | 0.339 |
| `awk -v OFS=- '{ $1 = NR; print }' src/commands/xinfo-stream.json \| tail -n 1` | 17801 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 19986 | 0.269 | 0.269 |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5634 | 5.719 | 5.719 |
| `grep -rn implementing . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 9187946 | 278 | 9477 | 20 | 49 | 0 | 0 | 0 | 0 | 0 | 80996 | 2.198 | 2.199 |
| `cat src/commands/*.json \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 738271 | 405 | 0 | 0 | 2838 | 0 | 0 | 0 | 0 | 0 | 308672 | 3.287 | 3.406 |
| `mkdir -p bench_out/a/b` | 0 | 0 | 0 | 0 | 3 | 0 | 103 | 0 | 1 | 0 | 3456 | 0.134 | 6.147 |
| `touch bench_out/empty.txt` | 0 | 0 | 0 | 0 | 3 | 0 | 67 | 0 | 1 | 0 | 2215 | 0.091 | 0.409 |
| `echo 'hello world' > bench_out/hello.txt` | 0 | 0 | 0 | 0 | 3 | 52 | 90 | 0 | 1 | 0 | 3446 | 0.092 | 0.323 |
| `echo 'another line' >> bench_out/hello.txt` | 24 | 2 | 0 | 0 | 2 | 65 | 63 | 0 | 1 | 0 | 3258 | 0.098 | 0.327 |
| `cp src/commands/xinfo-stream.json bench_out/copy.txt` | 0 | 0 | 0 | 0 | 8 | 0 | 94 | 0 | 1 | 0 | 3039 | 0.089 | 0.317 |
| `cp -r tests bench_out/dir_copy` | 0 | 0 | 0 | 0 | 3482 | 0 | 15975 | 0 | 1 | 0 | 177176 | 0.719 | 1.364 |
| `cp -r src/commands bench_out/work` | 0 | 0 | 0 | 0 | 3661 | 0 | 23444 | 0 | 1 | 0 | 222420 | 0.905 | 1.606 |
| `sed -i 's/for/QDBBENCHONE/g' bench_out/copy.txt` | 17801 | 1 | 0 | 0 | 2 | 17873 | 80 | 0 | 1 | 0 | 49114 | 0.531 | 1.197 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 17833 | 1 | 0 | 0 | 2 | 17873 | 80 | 0 | 1 | 0 | 50572 | 0.469 | 0.977 |
| `sed -i 's/for/QDBBENCHMANY/g' bench_out/work/*.json` | 738271 | 405 | 0 | 0 | 1623 | 330623 | 8239 | 0 | 1 | 0 | 314560 | 10.95 | 14.03 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.json` | 0 | 0 | 0 | 26 | 1623 | 0 | 4900 | 0 | 1 | 0 | 85586 | 0.829 | 3.105 |
| `sed -i 's/xmallctlmibnametomib/QDBBENCHFEW/g' bench_out/work/*.json` | 83027 | 47 | 6147 | 34 | 1623 | 0 | 4900 | 0 | 1 | 0 | 89194 | 0.655 | 2.927 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 | 0 | 0 | 0 | 8 | 0 | 94 | 0 | 1 | 0 | 2891 | 0.112 | 0.522 |
| `cp src/commands/xinfo-stream.json bench_out/same.txt` | 0 | 0 | 0 | 0 | 7 | 0 | 40 | 0 | 1 | 0 | 2619 | 0.076 | 0.427 |
| `cat src/commands/xinfo-stream.json > bench_out/same.txt` | 35602 | 2 | 0 | 0 | 8 | 0 | 40 | 0 | 1 | 0 | 21015 | 0.102 | 0.475 |
| `grep -rc QDBBENCHMANY bench_out` | 346060 | 135 | 1347 | 20 | 699 | 0 | 0 | 0 | 0 | 0 | 26462 | 0.466 | 0.466 |
| `grep -rl QDBBENCHMANY .` | 335145 | 147 | 1347 | 20 | 50 | 0 | 0 | 0 | 0 | 0 | 26431 | 0.182 | 0.182 |
| `grep -rl QDBBENCHONE .` | 27675 | 15 | 872 | 18 | 50 | 0 | 0 | 0 | 0 | 0 | 24764 | 0.088 | 0.087 |
| `cat src/commands/xinfo-stream.json \| tee bench_out/tee.txt \| wc -l` | 17801 | 1 | 0 | 0 | 9 | 17841 | 93 | 0 | 1 | 0 | 42414 | 0.329 | 1.013 |
| `sort src/commands/xinfo-stream.json > bench_out/sorted.txt` | 17801 | 1 | 0 | 0 | 9 | 17841 | 96 | 0 | 1 | 0 | 117650 | 0.349 | 1.038 |
| `sort -u src/commands/xinfo-stream.json >> bench_out/sorted.txt` | 53403 | 3 | 0 | 0 | 8 | 28937 | 68 | 0 | 1 | 0 | 111087 | 0.433 | 1.145 |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 | 0 | 0 | 0 | 5 | 89 | 89 | 0 | 1 | 0 | 3627 | 0.105 | 0.630 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 98 | 2 | 0 | 0 | 4 | 138 | 63 | 0 | 1 | 0 | 3499 | 0.102 | 0.612 |
| `find .github -name '*.json' \| sort \| xargs cat > bench_out/joined.txt` | 0 | 0 | 0 | 0 | 21 | 0 | 68 | 0 | 1 | 0 | 73170 | 0.131 | 0.574 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 | 0 | 0 | 0 | 7 | 0 | 69 | 0 | 1 | 0 | 3535 | 0.082 | 0.473 |
| `sed -i '1d' bench_out/joined2.txt` | 0 | 0 | 0 | 0 | 2 | 0 | 52 | 0 | 1 | 0 | 3391 | 0.094 | 0.559 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 0 | 0 | 0 | 0 | 16 | 0 | 0 | 0 | 0 | 0 | 4198 | 0.022 | 0.022 |
| `cat bench_out/both.txt` | 98 | 1 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 2146 | 0.013 | 0.013 |
| `cp src/commands/xinfo-stream.json bench_out/v2.txt` | 0 | 0 | 0 | 0 | 8 | 0 | 92 | 0 | 1 | 0 | 2863 | 0.103 | 0.621 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 17801 | 1 | 0 | 0 | 2 | 17674 | 80 | 0 | 1 | 0 | 45837 | 0.318 | 1.223 |
| `diff -u --label old --label new src/commands/xinfo-stream.json bench_out/v2.txt` | 212944 | 12 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 20540 | 0.089 | 0.089 |
| `diff src/commands/xinfo-stream.json bench_out/v2.txt \| wc -l` | 159708 | 9 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 20331 | 0.080 | 0.080 |
| `diff -q src/commands/xinfo-stream.json bench_out/same.txt` | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 3071 | 0.020 | 0.020 |
| `diff -u src/commands/xinfo-stream.json bench_out/copy.txt \| wc -l` | 213740 | 12 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 65612 | 0.151 | 0.151 |
| `diff -rq tests bench_out/dir_copy` | 0 | 0 | 0 | 0 | 569 | 0 | 0 | 0 | 0 | 0 | 10143 | 0.171 | 0.171 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 2924 | 1 | 0 | 0 | 288 | 2973 | 77 | 0 | 1 | 0 | 132527 | 0.396 | 1.249 |
| `diff -rq tests bench_out/dir_copy` | 5857 | 2 | 0 | 0 | 569 | 0 | 0 | 0 | 0 | 0 | 10205 | 0.158 | 0.158 |
| `diff -r src/commands bench_out/work \| wc -l` | 2916279 | 1197 | 0 | 0 | 816 | 0 | 0 | 0 | 0 | 0 | 86366 | 1.722 | 1.723 |
| `diff -ru src/commands bench_out/work \| grep -c '^@@'` | 3888372 | 1596 | 0 | 0 | 816 | 0 | 0 | 0 | 0 | 0 | 88434 | 2.094 | 2.094 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 1580743 | 19993 | 0 | 0 | 20001 | 1239829 | 241096 | 0 | 1 | 0 | 1754910 | 40.16 | 43.89 |
| `wc -l bench_out/loop.txt` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 2569 | 0.018 | 0.018 |
| `for f in bench_out/work/*.json; do echo done >> "$f"; done` | 750837 | 408 | 0 | 0 | 1623 | 28883 | 14209 | 0 | 1 | 0 | 46399 | 1.639 | 2.713 |
| `awk '{ print NR ": " $0 }' src/commands/xinfo-stream.json > bench_out/numbered.txt` | 17801 | 1 | 0 | 0 | 6 | 19543 | 98 | 0 | 1 | 0 | 50567 | 0.433 | 1.278 |
| `awk 'END { print NR }' bench_out/numbered.txt` | 19503 | 1 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 5494 | 0.049 | 0.049 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 | 0 | 0 | 0 | 8 | 0 | 55 | 0 | 1 | 0 | 2645 | 0.102 | 0.515 |
| `mv bench_out/hello.txt bench_out/a/b` | 0 | 0 | 0 | 0 | 13 | 0 | 67 | 0 | 1 | 0 | 2995 | 0.089 | 0.431 |
| `ls -R bench_out/a` | 0 | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 2837 | 0.014 | 0.014 |
| `rm bench_out/a/b/hello.txt` | 0 | 0 | 0 | 0 | 4 | 0 | 43 | 0 | 1 | 0 | 2153 | 0.070 | 0.418 |
| `rmdir bench_out/a/b` | 0 | 0 | 0 | 0 | 3 | 0 | 43 | 0 | 1 | 0 | 2032 | 0.087 | 0.339 |
| `rm -r bench_out/dir_moved` | 0 | 0 | 0 | 0 | 2 | 0 | 892 | 0 | 1 | 0 | 2452 | 0.244 | 0.444 |
| `rm -rf bench_out` | 0 | 0 | 0 | 0 | 1 | 0 | 1299 | 0 | 1 | 0 | 4217 | 1.432 | 1.789 |

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
