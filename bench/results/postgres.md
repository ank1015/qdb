# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 4, on disk, after the writes), and include starting bash and the tool (1.697 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 178.8 | 978.7 | 187.2 MB | 192.5 MB | 0.272 | yes |
| Stage 3, on disk, settled | on | 1562.6 | 1562.6 | 2238.5 KB | 136.4 MB | 0.316 | yes |
| Stage 4, in memory | on | 268.8 | 1338.7 | 199.7 MB | 205.8 MB | 0.284 | yes |
| Stage 4, on disk, settled | on | 1600.7 | 1600.7 | 2311.0 KB | 135.7 MB | 0.320 | yes |
| Stage 4, on disk, after the writes | on | 1600.7 | 1600.7 | 2311.0 KB | 135.7 MB | 0.320 | yes |

## The index (Stage 2, in memory)

- Built in 799.9 ms while loading.
- Size: 21.4 MB in memory, 13.3% of the text (4194.3 KB table, 17.0 MB lists, 262.1 KB document table). The posting data itself is 12.7 MB.
- 179569 distinct trigrams, 11900359 postings, 8388 documents.
- Share of posting data in the longest lists: top 100 trigrams 4.7%, top 1,000 29.6%, top 10,000 77.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|
| 1 | 45612 |
| 2 | 23673 |
| 3-4 | 27046 |
| 5-8 | 23649 |
| 9-16 | 17089 |
| 17-32 | 14301 |
| 33-64 | 9985 |
| 65-128 | 5560 |
| 129-256 | 4643 |
| 257-512 | 3165 |
| 513-1024 | 1930 |
| 1025-2048 | 1644 |
| 2049-4096 | 962 |
| 4097-8192 | 310 |

Index work per command that changes data:

| Command | Index work |
|---|---|
| `mkdir -p bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/backend bench_out/dir_copy` | documents +1346 -0; postings +2878944; list bytes rewritten 0; rebuilds 0 |
| `cp -r doc/src/sgml/html bench_out/work` | documents +1173 -0; postings +2134398; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -1; postings +2793; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2724; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | documents +1169 -1169; postings +2139969; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1346; postings +0; list bytes rewritten 3153154; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1174; postings +0; list bytes rewritten 5973942; rebuilds 0 |

## The index (Stage 3, on disk, settled)

- Built in 0.000 ms while loading.
- Size: 18.3 MB in memory, 11.4% of the text (2653.7 KB table, 15.6 MB lists, 85.1 KB document table). The posting data itself is 15.6 MB.
- 152072 distinct trigrams, 0 postings, 9604 documents.
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
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | documents +1 -0; postings +2774; list bytes rewritten 8676; rebuilds 0 |
| `cp -r src/backend bench_out/dir_copy` | documents +2007 -0; postings +4681094; list bytes rewritten 37137; rebuilds 0 |
| `cp -r doc/src/sgml/html bench_out/work` | documents +1212 -0; postings +2215436; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -1; postings +2793; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2724; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | documents +984 -954; postings +1919421; list bytes rewritten 315271; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | documents +1 -1; postings +2482; list bytes rewritten 26379; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +1 -0; postings +2774; list bytes rewritten 21414; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 142; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -2007; postings +0; list bytes rewritten 10088756; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1244; postings +0; list bytes rewritten 4460697; rebuilds 0 |

## The index (Stage 4, in memory)

- Built in 1070.0 ms while loading.
- Size: 23.0 MB in memory, 14.3% of the text (5688.0 KB table, 17.2 MB lists, 114.7 KB document table). The posting data itself is 12.9 MB.
- 152178 distinct trigrams, 0 postings, 9604 documents.
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
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/backend bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r doc/src/sgml/html bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +2793; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2724; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | documents +984 -0; postings +1919421; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | documents +1 -1; postings +2482; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat doc/src/sgml/html/bug-reporting.html \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `sort doc/src/sgml/html/bug-reporting.html > bench_out/sorted.txt` | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `sort -u doc/src/sgml/html/bug-reporting.html >> bench_out/sorted.txt` | documents +1 -1; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find src/include/access -name '*.html' \| sort \| xargs cat > bench_out/joined.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
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
| `rm -rf bench_out` | documents +0 -988; postings +0; list bytes rewritten 2145889; rebuilds 0 |

## The index (Stage 4, on disk, settled)

- Built in 0.000 ms while loading.
- Size: 18.3 MB in memory, 11.4% of the text (2653.8 KB table, 15.6 MB lists, 85.2 KB document table). The posting data itself is 15.6 MB.
- 152016 distinct trigrams, 0 postings, 9604 documents.
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
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/backend bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r doc/src/sgml/html bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +2793; list bytes rewritten 8718; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2724; list bytes rewritten 11330; rebuilds 0 |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | documents +984 -0; postings +1919421; list bytes rewritten 17765; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | documents +1 -1; postings +2482; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat doc/src/sgml/html/bug-reporting.html \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `sort doc/src/sgml/html/bug-reporting.html > bench_out/sorted.txt` | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `sort -u doc/src/sgml/html/bug-reporting.html >> bench_out/sorted.txt` | documents +1 -1; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find src/include/access -name '*.html' \| sort \| xargs cat > bench_out/joined.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
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
| `rm -rf bench_out` | documents +0 -988; postings +0; list bytes rewritten 3053554; rebuilds 0 |

## The index (Stage 4, on disk, after the writes)

- Built in 0.000 ms while loading.
- Size: 18.3 MB in memory, 11.4% of the text (2653.8 KB table, 15.6 MB lists, 85.2 KB document table). The posting data itself is 15.6 MB.
- 152016 distinct trigrams, 0 postings, 9604 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.607 |  | 0.001 | 0.001 | 0.002 | 0.002 | 0.003 | yes |
| `ls -a` | 2.575 |  | 0.002 | 0.002 | 0.003 | 0.003 | 0.006 | yes |
| `ls -R` | 10.87 |  | 0.563 | 0.288 | 1.185 | 1.214 | 0.816 | yes |
| `ls -aR src/backend` | 3.827 |  | 0.059 | 0.055 | 0.192 | 0.197 | 0.124 | yes |
| `ls -p src/backend` | 2.370 |  | 0.002 | 0.002 | 0.005 | 0.005 | 0.005 | yes |
| `find .` | 9.053 |  | 1.298 | 0.767 | 1.649 | 1.206 | 0.812 | yes |
| `find . -name '*.html'` | 9.113 |  | 1.444 | 0.953 | 1.159 | 1.653 | 1.118 | yes |
| `find . -name '*.c'` | 10.97 |  | 1.706 | 1.780 | 1.164 | 0.988 | 0.973 | yes |
| `find . -type d` | 11.48 |  | 1.403 | 0.718 | 0.960 | 0.842 | 0.697 | yes |
| `find . -type f` | 9.530 |  | 1.397 | 1.182 | 1.026 | 0.910 | 0.702 | yes |
| `find . -maxdepth 2 -type f` | 3.067 |  | 0.015 | 0.015 | 0.012 | 0.011 | 0.012 | yes |
| `find src/backend -name '*.html' -type f` | 3.845 |  | 0.177 | 0.189 | 0.134 | 0.218 | 0.134 | yes |
| `find . -name 'Makefile'` | 14.20 |  | 2.052 | 1.344 | 0.866 | 1.187 | 0.751 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 2.715 |  | 0.001 | 0.002 | 0.001 | 0.002 | 0.002 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 2.707 |  | 0.028 | 0.048 | 0.027 | 0.041 | 0.049 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 2.668 |  | 0.003 | 0.003 | 0.003 | 0.005 | 0.006 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2.171 |  | 0.003 | 0.002 | 0.003 | 0.004 | 0.005 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 2.207 |  | 0.006 | 0.005 | 0.013 | 0.013 | 0.017 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 2.678 |  | 0.023 | 0.020 | 0.036 | 0.029 | 0.044 | yes |
| `wc -l doc/src/sgml/html/*.html` | 17.30 |  | 27.94 | 0.901 | 0.949 | 1.246 | 0.870 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 32.82 |  | 30.00 | 30.51 | 2.793 | 2.640 | 2.088 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 3.521 |  | 0.010 | 0.010 | 0.009 | 0.008 | 0.006 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 2.514 |  | 0.004 | 0.005 | 0.010 | 0.006 | 0.006 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 2.605 |  | 0.006 | 0.007 | 0.013 | 0.008 | 0.009 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 2.717 |  | 0.205 | 0.233 | 0.081 | 0.049 | 0.052 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 2.914 |  | 0.445 | 0.590 | 0.255 | 0.148 | 0.209 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 3.535 |  | 0.078 | 0.083 | 0.037 | 0.034 | 0.053 | yes |
| `grep -rn qdbzzabsentterm .` | 151.9 | 72.27 | 0.014 | 0.021 | 0.025 | 0.026 | 0.025 | yes |
| `grep -rl qdbzzabsentterm .` | 107.1 | 65.27 | 0.015 | 0.024 | 0.027 | 0.028 | 0.022 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 140.1 | 72.24 | 0.051 | 0.027 | 0.045 | 0.045 | 0.054 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 96.68 | 65.96 | 0.038 | 0.027 | 0.029 | 0.031 | 0.045 | yes |
| `grep -rn btvarstrequalimage .` | 152.9 | 74.92 | 1.605 | 0.183 | 0.220 | 0.184 | 0.131 | yes |
| `grep -rl btvarstrequalimage .` | 97.77 | 69.27 | 1.672 | 0.085 | 0.115 | 0.104 | 0.152 | yes |
| `grep -rn undesirable .` | 169.2 | 70.69 | 2.512 | 1.028 | 1.129 | 1.088 | 1.067 | yes |
| `grep -rl undesirable .` | 120.4 | 63.39 | 1.837 | 0.535 | 0.776 | 0.611 | 0.690 | yes |
| `grep -rn passed .` | 188.3 | 67.37 | 21.08 | 15.87 | 14.94 | 12.45 | 12.78 | yes |
| `grep -rl passed .` | 112.0 | 77.40 | 10.66 | 2.978 | 6.995 | 2.955 | 4.050 | yes |
| `grep -rn not .` | 323.3 | 129.5 | 35.22 | 36.87 | 50.66 | 50.27 | 39.23 | yes |
| `grep -rl not .` | 69.18 | 71.02 | 3.506 | 2.882 | 6.565 | 2.848 | 2.737 | yes |
| `grep -rc undesirable .` | 143.9 | 71.67 | 3.520 | 1.431 | 1.532 | 1.538 | 1.008 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 166.8 | 88.22 | 1.457 | 0.106 | 0.126 | 0.148 | 0.138 | yes |
| `grep -ri undesirable .` | 157.2 | 81.65 | 3.253 | 0.632 | 0.880 | 0.699 | 0.700 | yes |
| `grep -rn undesirable src/backend` | 69.54 | 21.28 | 1.529 | 0.418 | 0.347 | 0.508 | 0.440 | yes |
| `grep -rl passed src/backend` | 31.87 | 18.40 | 1.544 | 1.218 | 1.210 | 1.482 | 1.010 | yes |
| `grep -rF 'undesirable' .` | 127.4 | 69.13 | 1.579 | 0.744 | 0.727 | 0.729 | 0.788 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 117.0 | 68.43 | 8.872 | 6.092 | 3.095 | 3.356 | 4.141 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 209.8 | 75.52 | 1.877 | 0.263 | 0.258 | 0.401 | 0.318 | yes |
| `grep -rE 'passed\|undesirable' .` | 244.6 | 75.58 | 19.57 | 20.71 | 16.38 | 16.09 | 15.13 | yes |
| `grep -rE '[0-9]{4,}' .` | 284.5 | 358.1 | 176.6 | 185.0 | 188.7 | 186.0 | 186.8 | yes |
| `grep -rn '^#include' .` | 194.5 | 88.52 | 11.65 | 9.657 | 10.83 | 10.85 | 7.255 | yes |
| `grep -rE '^(import\|from) ' .` | 160.5 | 105.8 | 43.93 | 36.34 | 43.11 | 38.59 | 34.08 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 90.81 | 51.61 | 16.54 | 16.58 | 17.19 | 15.54 | 15.88 | yes |
| `grep -rv e src/include/access` | 5.743 | 7.619 | 0.482 | 0.757 | 0.525 | 0.535 | 0.549 | yes |
| `grep -r '\(undesirable\).*\1' .` | 136.9 |  | 2.322 | 1.364 | 0.959 | 0.979 | 0.982 | yes |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 18.89 |  | | 0.675 | 0.547 | 0.702 | 0.791 | yes |
| `grep -l undesirable doc/src/sgml/html/*.html` | 20.30 |  | | 1.091 | 0.773 | 0.676 | 0.710 | yes |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 15.41 |  | | 0.607 | 0.576 | 0.836 | 0.634 | yes |
| `grep -rl passed . \| wc -l` | 109.0 |  | 5.880 | 7.873 | 2.989 | 2.812 | 6.909 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 224.7 |  | 32.83 | 30.56 | 31.08 | 29.90 | 27.88 | yes |
| `find . -name '*.html' \| wc -l` | 14.65 |  | 1.615 | 1.048 | 1.919 | 1.079 | 1.496 | yes |
| `find . -type f \| grep -c html` | 12.90 |  | 1.328 | 1.278 | 2.354 | 1.297 | 2.054 | yes |
| `grep -rn not . \| head -n 5` | 4.496 |  | 0.008 | 0.008 | 0.014 | 0.007 | 0.013 | order differs |
| `grep -rn passed . \| head -n 1` | 19.95 |  | 0.012 | 0.022 | 0.015 | 0.012 | 0.018 | order differs |
| `find . -type f \| head -n 3` | 4.642 |  | 0.046 | 0.039 | 0.050 | 0.049 | 0.092 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 5.547 |  | 0.318 | 0.500 | 0.316 | 0.304 | 0.500 | yes |
| `grep -rno undesirable .` | 165.7 | 74.77 | | | 0.947 | 0.752 | 0.985 | yes |
| `grep -rw undesirable .` | 135.1 | 76.64 | | | 1.017 | 0.680 | 0.734 | yes |
| `grep -rnw btvarstrequalimage .` | 156.8 | 79.34 | | | 0.188 | 0.204 | 0.344 | yes |
| `grep -rn -C 2 btvarstrequalimage .` | 161.0 | 328.2 | | | 0.237 | 0.312 | 0.453 | yes |
| `grep -rn -A 1 -B 1 undesirable .` | 182.2 | 347.7 | | | 1.684 | 2.129 | 2.648 | yes |
| `grep -rho 'size_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 118.3 |  | | | 11.58 | 9.023 | 9.130 | yes |
| `grep -rhow '[a-z]*btvarstrequalimage[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 112.8 |  | | | 0.228 | 0.222 | 0.246 | yes |
| `rg -j 1 qdbzzabsentterm` | 98.36 |  | | | 0.035 | 0.036 | 0.049 | yes |
| `rg -j 1 abcdefghijklabcdefgh` | 97.66 |  | | | 0.054 | 0.052 | 0.047 | yes |
| `rg -j 1 btvarstrequalimage` | 110.1 |  | | | 0.169 | 0.157 | 0.225 | yes |
| `rg -j 1 undesirable` | 110.8 |  | | | 0.945 | 1.702 | 1.351 | yes |
| `rg -j 1 passed` | 98.70 |  | | | 18.80 | 17.87 | 19.76 | yes |
| `rg -j 1 not` | 184.2 |  | | | 39.14 | 38.59 | 45.88 | yes |
| `rg -j 1 -l undesirable` | 86.06 |  | | | 0.934 | 0.983 | 0.989 | yes |
| `rg -j 1 -l not` | 91.24 |  | | | 12.28 | 9.828 | 13.42 | yes |
| `rg -j 1 -c passed` | 105.2 |  | | | 19.14 | 19.28 | 21.06 | yes |
| `rg -j 1 -i BTVARSTREQUALIMAGE` | 112.0 |  | | | 1.009 | 0.959 | 1.456 | yes |
| `rg -j 1 -w undesirable` | 108.8 |  | | | 14.47 | 11.94 | 19.75 | yes |
| `rg -j 1 -F 'undesirable'` | 109.2 |  | | | 1.329 | 1.351 | 1.652 | yes |
| `rg -j 1 -n -C 2 btvarstrequalimage` | 94.51 |  | | | 0.582 | 0.524 | 0.399 | yes |
| `rg -j 1 'size_[a-z]+\('` | 104.7 |  | | | 6.340 | 6.527 | 5.716 | yes |
| `rg -j 1 'btvarstrequalimage\|abcdefghijklabcdefgh'` | 98.94 |  | | | 0.200 | 0.212 | 0.189 | yes |
| `rg -j 1 '\bbtvarstrequalimage\w*'` | 86.26 |  | | | 3.093 | 3.360 | 3.932 | yes |
| `rg -j 1 -t c undesirable` | 49.40 |  | | | 0.680 | 0.693 | 0.722 | yes |
| `rg -j 1 -g '*.html' undesirable` | 42.28 |  | | | 0.452 | 0.604 | 0.771 | yes |
| `rg -j 1 undesirable src/backend` | 28.70 |  | | | 0.557 | 0.654 | 0.514 | yes |
| `rg -j 1 -uu undesirable` | 88.55 |  | | | 1.091 | 0.746 | 0.794 | yes |
| `rg -j 1 --sort path btvarstrequalimage` | 86.40 |  | | | 0.186 | 0.174 | 0.135 | yes |
| `rg -j 1 --files \| wc -l` | 27.37 |  | | | 6.596 | 7.054 | 11.10 | yes |
| `rg -j 1 -l passed \| wc -l` | 89.13 |  | | | 14.02 | 12.22 | 14.74 | yes |
| `ls -ltR` | 37.31 |  | | | 8.570 | 4.957 | 4.853 | yes |
| `ls -lt doc/src/sgml/html` | 7.311 |  | | | 0.574 | 0.560 | 0.928 | yes |
| `ls -ltr src/backend \| tail -n 3` | 4.011 |  | | | 0.021 | 0.037 | 0.025 | yes |
| `find . -newer doc/src/sgml/html/bug-reporting.html` | 18.50 |  | | | 0.800 | 0.909 | 0.885 | yes |
| `find . -newer doc/src/sgml/html/bug-reporting.html -name '*.html'` | 13.00 |  | | | 1.139 | 1.189 | 1.120 | yes |
| `find . -type f -mtime +30` | 20.08 |  | | | 0.968 | 1.085 | 0.830 | yes |
| `du -s .` | 15.32 |  | | | 0.001 | 0.001 | 0.001 | not compared |
| `du -sh src/backend` | 4.559 |  | | | 0.001 | 0.001 | 0.001 | not compared |
| `du -sb .` | 21.00 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `du -sb src/backend` | 5.216 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `du -b -d 1 src/backend` | 5.847 |  | | | 0.004 | 0.004 | 0.007 | yes |
| `sort doc/src/sgml/html/bug-reporting.html` | 3.525 |  | | | 0.022 | 0.024 | 0.021 | yes |
| `sort -r doc/src/sgml/html/bug-reporting.html \| head -n 3` | 3.813 |  | | | 0.025 | 0.026 | 0.045 | yes |
| `sort -u doc/src/sgml/html/bug-reporting.html \| wc -l` | 3.562 |  | | | 0.026 | 0.023 | 0.043 | yes |
| `sort -k 2 doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 3.607 |  | | | 0.027 | 0.027 | 0.047 | yes |
| `cat doc/src/sgml/html/*.html \| sort \| uniq -c \| sort -rn \| head -n 5` | 252.9 |  | | | 156.7 | 105.3 | 158.5 | yes |
| `uniq -c doc/src/sgml/html/bug-reporting.html \| wc -l` | 3.011 |  | | | 0.018 | 0.019 | 0.029 | yes |
| `cut -c 1-20 doc/src/sgml/html/bug-reporting.html` | 2.517 |  | | | 0.015 | 0.014 | 0.024 | yes |
| `cut -d ' ' -f 1 doc/src/sgml/html/bug-reporting.html \| sort \| uniq \| wc -l` | 4.161 |  | | | 0.047 | 0.043 | 0.053 | yes |
| `tr a-z A-Z < doc/src/sgml/html/bug-reporting.html` | 3.455 |  | | | 0.004 | 0.003 | 0.003 | yes |
| `tr -d '\n' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 3.567 |  | | | 0.009 | 0.010 | 0.016 | yes |
| `tr -s ' ' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 3.399 |  | | | 0.013 | 0.013 | 0.015 | yes |
| `nl doc/src/sgml/html/bug-reporting.html` | 3.117 |  | | | 0.028 | 0.026 | 0.053 | yes |
| `nl -ba doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 3.645 |  | | | 0.046 | 0.028 | 0.049 | yes |
| `seq 1 100000 \| wc -l` | 5.199 |  | | | 3.622 | 2.389 | 3.923 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.557 |  | | | 0.038 | 0.026 | 0.041 | yes |
| `basename doc/src/sgml/html/bug-reporting.html` | 2.408 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `basename -s .html doc/src/sgml/html/bug-reporting.html` | 2.446 |  | | | 0.001 | 0.002 | 0.003 | yes |
| `dirname doc/src/sgml/html/bug-reporting.html` | 2.791 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `printf '%s\n' a b c` | 1.606 |  | | | 0.002 | 0.002 | 0.004 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 2.000 |  | | | 0.002 | 0.002 | 0.004 | yes |
| `test -f doc/src/sgml/html/bug-reporting.html && echo yes` | 1.966 |  | | | 0.002 | 0.002 | 0.004 | yes |
| `[ -d src/backend ] && echo folder` | 1.548 |  | | | 0.003 | 0.002 | 0.006 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 2.788 |  | | | 0.003 | 0.003 | 0.007 | yes |
| `echo a b c \| xargs -n 1 echo` | 7.719 |  | | | 0.003 | 0.003 | 0.008 | yes |
| `find src/include/access -name '*.html' \| xargs cat \| wc -l` | 5.239 |  | | | 0.010 | 0.017 | 0.018 | yes |
| `find . -name '*.html' \| xargs grep -l undesirable \| wc -l` | 29.81 |  | | | 2.311 | 2.115 | 2.303 | yes |
| `echo {a,b,c}{1..3}` | 1.776 |  | | | 0.004 | 0.004 | 0.005 | yes |
| `echo {1..20..3} {a..e}` | 1.601 |  | | | 0.005 | 0.005 | 0.006 | yes |
| `echo $(wc -l < doc/src/sgml/html/bug-reporting.html) `basename doc/src/sgml/html/bug-reporting.html`` | 5.271 |  | | | 0.006 | 0.006 | 0.012 | yes |
| `echo "first line: $(head -n 1 doc/src/sgml/html/bug-reporting.html)"` | 2.972 |  | | | 0.004 | 0.004 | 0.008 | yes |
| `wc -l < doc/src/sgml/html/bug-reporting.html` | 3.007 |  | | | 0.002 | 0.002 | 0.004 | yes |
| `cat doc/src/sgml/html/bug-reporting.html > /dev/null 2>&1` | 3.025 |  | | | 0.002 | 0.002 | 0.003 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 2.926 |  | | | 0.002 | 0.002 | 0.005 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 3.370 |  | | | 0.003 | 0.002 | 0.006 | yes |
| `grep -q not doc/src/sgml/html/bug-reporting.html && echo found \|\| echo none` | 3.377 |  | | | 0.006 | 0.006 | 0.010 | yes |
| `false \|\| echo fallback` | 1.812 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `true && grep -c not doc/src/sgml/html/bug-reporting.html` | 3.039 |  | | | 0.007 | 0.007 | 0.012 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 3.695 |  | | | 0.002 | 0.003 | 0.006 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 3.332 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `echo one \⏎two` | 1.858 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html⏎wc -l doc/src/sgml/html/bug-reporting.html` | 4.704 |  | | | 0.008 | 0.008 | 0.014 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes |
|---|---:|---:|---:|---:|---:|---:|
| `ls` | 144 B | 961 B | 961 B | 3045 B | 3045 B | 3045 B |
| `ls -a` | 307 B | 1785 B | 1785 B | 4374 B | 4374 B | 4374 B |
| `ls -R` | 151.8 KB | 73.0 KB | 73.0 KB | 160.9 KB | 160.9 KB | 160.9 KB |
| `ls -aR src/backend` | 23.1 KB | 29.2 KB | 29.2 KB | 36.9 KB | 36.9 KB | 36.9 KB |
| `ls -p src/backend` | 279 B | 2075 B | 2075 B | 4792 B | 4792 B | 4792 B |
| `find .` | 347.5 KB | 28.5 KB | 28.5 KB | 30.3 KB | 30.3 KB | 30.3 KB |
| `find . -name '*.html'` | 49.6 KB | 18.3 KB | 18.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `find . -name '*.c'` | 54.0 KB | 33.0 KB | 33.0 KB | 35.1 KB | 35.1 KB | 35.1 KB |
| `find . -type d` | 16.8 KB | 17.5 KB | 17.5 KB | 19.6 KB | 19.6 KB | 19.6 KB |
| `find . -type f` | 330.7 KB | 26.7 KB | 26.7 KB | 28.9 KB | 28.9 KB | 28.9 KB |
| `find . -maxdepth 2 -type f` | 992 B | 2820 B | 2820 B | 5417 B | 5417 B | 5417 B |
| `find src/backend -name '*.html' -type f` | 0 B | 1952 B | 1952 B | 4198 B | 4198 B | 4198 B |
| `find . -name 'Makefile'` | 9792 B | 11.6 KB | 11.6 KB | 13.7 KB | 13.7 KB | 13.7 KB |
| `cat doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 1068 B | 1068 B | 2467 B | 2467 B | 2467 B |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 21.7 KB | 33.0 KB | 33.0 KB | 35.2 KB | 35.2 KB | 35.2 KB |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 3143 B | 4075 B | 4075 B | 6167 B | 6167 B | 6167 B |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2038 B | 2970 B | 2970 B | 5062 B | 5062 B | 5062 B |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 11.5 KB | 12.5 KB | 12.5 KB | 14.6 KB | 14.6 KB | 14.6 KB |
| `wc doc/src/sgml/html/bug-reporting.html` | 55 B | 1076 B | 1076 B | 2662 B | 2662 B | 2662 B |
| `wc -l doc/src/sgml/html/*.html` | 57.8 KB | 268.9 KB | 268.9 KB | 269.6 KB | 269.6 KB | 269.6 KB |
| `cat doc/src/sgml/html/*.html \| wc -l` | 7 B | 269.1 KB | 269.1 KB | 270.0 KB | 270.0 KB | 270.0 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 565 B | 10.3 KB | 10.5 KB | 15.4 KB | 15.4 KB | 15.4 KB |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | 5856 B | 5856 B | 10.1 KB | 10.1 KB | 10.1 KB |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3886 B | 6933 B | 6933 B | 10.1 KB | 10.1 KB | 10.1 KB |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19.9 KB | 35.9 KB | 35.9 KB | 24.7 KB | 24.7 KB | 24.7 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 37.1 KB | 37.1 KB | 30.0 KB | 30.0 KB | 30.0 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6 B | 36.7 KB | 36.9 KB | 39.8 KB | 39.8 KB | 39.8 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB |
| `grep -rn abcdefghijklabcdefgh .` | 170 B | 26.8 KB | 26.8 KB | 40.8 KB | 40.8 KB | 40.8 KB |
| `grep -rl abcdefghijklabcdefgh .` | 41 B | 26.8 KB | 26.8 KB | 40.8 KB | 40.8 KB | 40.8 KB |
| `grep -rn btvarstrequalimage .` | 1660 B | 24.6 KB | 24.6 KB | 37.5 KB | 37.5 KB | 37.5 KB |
| `grep -rl btvarstrequalimage .` | 316 B | 24.6 KB | 24.6 KB | 37.5 KB | 37.5 KB | 37.5 KB |
| `grep -rn undesirable .` | 11.2 KB | 29.2 KB | 29.1 KB | 31.3 KB | 31.3 KB | 31.3 KB |
| `grep -rl undesirable .` | 2982 B | 29.2 KB | 29.1 KB | 31.3 KB | 31.3 KB | 31.3 KB |
| `grep -rn passed .` | 225.8 KB | 15.3 KB | 25.9 KB | 28.2 KB | 28.2 KB | 29.1 KB |
| `grep -rl passed .` | 30.4 KB | 9696 B | 25.6 KB | 27.8 KB | 27.8 KB | 28.7 KB |
| `grep -rn not .` | 13.4 MB | 31.1 KB | 31.1 KB | 33.3 KB | 33.3 KB | 33.3 KB |
| `grep -rl not .` | 189.7 KB | 5758 B | 5758 B | 9977 B | 9977 B | 9977 B |
| `grep -rc undesirable .` | 347.5 KB | 44.4 KB | 44.9 KB | 47.1 KB | 47.1 KB | 47.1 KB |
| `grep -rin BTVARSTREQUALIMAGE .` | 1730 B | 24.6 KB | 24.6 KB | 37.5 KB | 37.5 KB | 37.5 KB |
| `grep -ri undesirable .` | 10.8 KB | 29.2 KB | 29.0 KB | 31.3 KB | 31.3 KB | 31.3 KB |
| `grep -rn undesirable src/backend` | 3398 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | 24.8 KB |
| `grep -rl passed src/backend` | 10.7 KB | 9717 B | 25.7 KB | 27.9 KB | 27.9 KB | 28.8 KB |
| `grep -rF 'undesirable' .` | 10.8 KB | 29.2 KB | 29.1 KB | 31.3 KB | 31.3 KB | 31.3 KB |
| `grep -rE 'size_[a-z]+\(' .` | 20.4 KB | 117.2 KB | 124.5 KB | 126.8 KB | 126.8 KB | 126.8 KB |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1754 B | 49.6 KB | 49.6 KB | 74.2 KB | 74.2 KB | 74.2 KB |
| `grep -rE 'passed\|undesirable' .` | 228.0 KB | 24.0 KB | 27.2 KB | 36.4 KB | 36.4 KB | 36.4 KB |
| `grep -rE '[0-9]{4,}' .` | 39.4 MB | 23.1 KB | 23.1 KB | 25.4 KB | 25.4 KB | 25.4 KB |
| `grep -rn '^#include' .` | 1219.8 KB | 33.9 KB | 55.2 KB | 56.2 KB | 57.5 KB | 57.5 KB |
| `grep -rE '^(import\|from) ' .` | 61.8 KB | 23.9 KB | 23.9 KB | 30.1 KB | 30.1 KB | 30.1 KB |
| `grep -rin 'return [a-z_]*;$' src/backend` | 902.7 KB | 50.4 KB | 66.9 KB | 66.0 KB | 69.2 KB | 66.0 KB |
| `grep -rv e src/include/access` | 232.5 KB | 27.5 KB | 27.5 KB | 29.7 KB | 29.7 KB | 29.7 KB |
| `grep -r '\(undesirable\).*\1' .` | 0 B | 33.0 KB | 32.9 KB | 35.1 KB | 35.1 KB | 35.1 KB |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 125 B | | 269.2 KB | 270.2 KB | 270.2 KB | 270.2 KB |
| `grep -l undesirable doc/src/sgml/html/*.html` | 976 B | | 269.2 KB | 270.2 KB | 270.2 KB | 270.2 KB |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 49.6 KB | | 269.2 KB | 270.2 KB | 270.2 KB | 270.2 KB |
| `grep -rl passed . \| wc -l` | 4 B | 10.5 KB | 26.5 KB | 28.8 KB | 28.8 KB | 29.7 KB |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 5 B | 10.9 KB | 11.1 KB | 14.0 KB | 14.0 KB | 14.0 KB |
| `find . -name '*.html' \| wc -l` | 5 B | 19.0 KB | 19.1 KB | 21.3 KB | 21.3 KB | 21.3 KB |
| `find . -type f \| grep -c html` | 5 B | 34.3 KB | 34.5 KB | 39.7 KB | 39.7 KB | 39.7 KB |
| `grep -rn not . \| head -n 5` | 386 B | 6624 B | 6736 B | 11.2 KB | 11.2 KB | 11.2 KB |
| `grep -rn passed . \| head -n 1` | 88 B | 10.3 KB | 10.4 KB | 16.6 KB | 16.6 KB | 16.6 KB |
| `find . -type f \| head -n 3` | 61 B | 27.5 KB | 27.6 KB | 30.0 KB | 30.0 KB | 30.0 KB |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 1555 B | 269.2 KB | 269.2 KB | 270.3 KB | 270.3 KB | 270.3 KB |
| `grep -rno undesirable .` | 5210 B | | | 38.9 KB | 38.9 KB | 38.9 KB |
| `grep -rw undesirable .` | 10.8 KB | | | 42.3 KB | 42.3 KB | 42.3 KB |
| `grep -rnw btvarstrequalimage .` | 1660 B | | | 44.4 KB | 44.4 KB | 44.4 KB |
| `grep -rn -C 2 btvarstrequalimage .` | 6371 B | | | 38.2 KB | 38.2 KB | 38.2 KB |
| `grep -rn -A 1 -B 1 undesirable .` | 31.9 KB | | | 32.5 KB | 32.5 KB | 32.5 KB |
| `grep -rho 'size_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 188 B | | | 227.5 KB | 227.4 KB | 227.4 KB |
| `grep -rhow '[a-z]*btvarstrequalimage[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 27 B | | | 75.7 KB | 75.7 KB | 75.7 KB |
| `rg -j 1 qdbzzabsentterm` | 0 B | | | 30.8 KB | 30.8 KB | 30.8 KB |
| `rg -j 1 abcdefghijklabcdefgh` | 158 B | | | 39.8 KB | 39.8 KB | 39.8 KB |
| `rg -j 1 btvarstrequalimage` | 1562 B | | | 58.2 KB | 58.2 KB | 58.2 KB |
| `rg -j 1 undesirable` | 10.6 KB | | | 109.8 KB | 109.8 KB | 109.8 KB |
| `rg -j 1 passed` | 213.1 KB | | | 436.7 KB | 436.7 KB | 437.6 KB |
| `rg -j 1 not` | 12.6 MB | | | 426.5 KB | 426.5 KB | 426.5 KB |
| `rg -j 1 -l undesirable` | 2812 B | | | 100.4 KB | 100.4 KB | 100.4 KB |
| `rg -j 1 -l not` | 179.6 KB | | | 425.4 KB | 425.4 KB | 425.4 KB |
| `rg -j 1 -c passed` | 30.4 KB | | | 433.1 KB | 433.1 KB | 434.0 KB |
| `rg -j 1 -i BTVARSTREQUALIMAGE` | 1625 B | | | 90.0 KB | 89.9 KB | 89.9 KB |
| `rg -j 1 -w undesirable` | 10.6 KB | | | 131.8 KB | 131.8 KB | 131.8 KB |
| `rg -j 1 -F 'undesirable'` | 10.6 KB | | | 110.3 KB | 110.3 KB | 110.3 KB |
| `rg -j 1 -n -C 2 btvarstrequalimage` | 6253 B | | | 68.1 KB | 68.1 KB | 68.1 KB |
| `rg -j 1 'size_[a-z]+\('` | 19.9 KB | | | 262.2 KB | 262.1 KB | 262.1 KB |
| `rg -j 1 'btvarstrequalimage\|abcdefghijklabcdefgh'` | 1720 B | | | 72.6 KB | 72.6 KB | 72.6 KB |
| `rg -j 1 '\bbtvarstrequalimage\w*'` | 1562 B | | | 2620.7 KB | 2620.7 KB | 2620.7 KB |
| `rg -j 1 -t c undesirable` | 3729 B | | | 105.4 KB | 105.4 KB | 105.4 KB |
| `rg -j 1 -g '*.html' undesirable` | 3303 B | | | 102.1 KB | 102.1 KB | 102.1 KB |
| `rg -j 1 undesirable src/backend` | 3255 B | | | 54.9 KB | 54.9 KB | 54.9 KB |
| `rg -j 1 -uu undesirable` | 10.6 KB | | | 38.5 KB | 38.5 KB | 38.5 KB |
| `rg -j 1 --sort path btvarstrequalimage` | 1562 B | | | 58.9 KB | 58.9 KB | 58.9 KB |
| `rg -j 1 --files \| wc -l` | 5 B | | | 423.7 KB | 423.7 KB | 423.7 KB |
| `rg -j 1 -l passed \| wc -l` | 4 B | | | 446.3 KB | 446.3 KB | 447.2 KB |
| `ls -ltR` | 513.7 KB | | | 172.7 KB | 172.7 KB | 172.7 KB |
| `ls -lt doc/src/sgml/html` | 74.4 KB | | | 131.0 KB | 131.0 KB | 131.0 KB |
| `ls -ltr src/backend \| tail -n 3` | 149 B | | | 7263 B | 7263 B | 7263 B |
| `find . -newer doc/src/sgml/html/bug-reporting.html` | 288.1 KB | | | 20.0 KB | 20.0 KB | 20.0 KB |
| `find . -newer doc/src/sgml/html/bug-reporting.html -name '*.html'` | 38.8 KB | | | 24.0 KB | 24.0 KB | 24.0 KB |
| `find . -type f -mtime +30` | 0 B | | | 4081 B | 4081 B | 4081 B |
| `du -s .` | 9 B | | | 2704 B | 2704 B | 2704 B |
| `du -sh src/backend` | 16 B | | | 2750 B | 2750 B | 2750 B |
| `du -sb .` | 12 B | | | 2710 B | 2710 B | 2710 B |
| `du -sb src/backend` | 21 B | | | 2768 B | 2768 B | 2768 B |
| `du -b -d 1 src/backend` | 776 B | | | 5038 B | 5038 B | 5038 B |
| `sort doc/src/sgml/html/bug-reporting.html` | 20.0 KB | | | 106.3 KB | 106.3 KB | 106.3 KB |
| `sort -r doc/src/sgml/html/bug-reporting.html \| head -n 3` | 2009 B | | | 109.6 KB | 109.6 KB | 109.6 KB |
| `sort -u doc/src/sgml/html/bug-reporting.html \| wc -l` | 4 B | | | 107.6 KB | 107.6 KB | 107.6 KB |
| `sort -k 2 doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 209 B | | | 128.1 KB | 128.1 KB | 128.1 KB |
| `cat doc/src/sgml/html/*.html \| sort \| uniq -c \| sort -rn \| head -n 5` | 106 B | | | 57.5 MB | 57.5 MB | 57.5 MB |
| `uniq -c doc/src/sgml/html/bug-reporting.html \| wc -l` | 4 B | | | 39.5 KB | 39.5 KB | 39.5 KB |
| `cut -c 1-20 doc/src/sgml/html/bug-reporting.html` | 4915 B | | | 18.2 KB | 18.2 KB | 18.2 KB |
| `cut -d ' ' -f 1 doc/src/sgml/html/bug-reporting.html \| sort \| uniq \| wc -l` | 2 B | | | 78.4 KB | 78.4 KB | 78.4 KB |
| `tr a-z A-Z < doc/src/sgml/html/bug-reporting.html` | 20.0 KB | | | 25.0 KB | 25.0 KB | 25.0 KB |
| `tr -d '\n' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 6 B | | | 26.0 KB | 26.0 KB | 26.0 KB |
| `tr -s ' ' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 6 B | | | 26.0 KB | 26.0 KB | 26.0 KB |
| `nl doc/src/sgml/html/bug-reporting.html` | 21.7 KB | | | 36.3 KB | 36.3 KB | 36.3 KB |
| `nl -ba doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 903 B | | | 58.8 KB | 58.8 KB | 58.8 KB |
| `seq 1 100000 \| wc -l` | 7 B | | | 135.0 KB | 135.0 KB | 135.0 KB |
| `seq -w 1 1000 \| tail -n 2` | 10 B | | | 17.6 KB | 17.6 KB | 17.6 KB |
| `basename doc/src/sgml/html/bug-reporting.html` | 19 B | | | 2524 B | 2524 B | 2524 B |
| `basename -s .html doc/src/sgml/html/bug-reporting.html` | 14 B | | | 3182 B | 3182 B | 3182 B |
| `dirname doc/src/sgml/html/bug-reporting.html` | 18 B | | | 2521 B | 2521 B | 2521 B |
| `printf '%s\n' a b c` | 6 B | | | 3325 B | 3325 B | 3325 B |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 18 B | | | 3422 B | 3422 B | 3422 B |
| `test -f doc/src/sgml/html/bug-reporting.html && echo yes` | 4 B | | | 3607 B | 3607 B | 3607 B |
| `[ -d src/backend ] && echo folder` | 7 B | | | 3784 B | 3784 B | 3784 B |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 35 B | | | 3492 B | 3492 B | 3492 B |
| `echo a b c \| xargs -n 1 echo` | 6 B | | | 4674 B | 4674 B | 4674 B |
| `find src/include/access -name '*.html' \| xargs cat \| wc -l` | 2 B | | | 5091 B | 5091 B | 5091 B |
| `find . -name '*.html' \| xargs grep -l undesirable \| wc -l` | 3 B | | | 581.5 KB | 581.5 KB | 581.5 KB |
| `echo {a,b,c}{1..3}` | 27 B | | | 4485 B | 4485 B | 4485 B |
| `echo {1..20..3} {a..e}` | 28 B | | | 3967 B | 3967 B | 3967 B |
| `echo $(wc -l < doc/src/sgml/html/bug-reporting.html) `basename doc/src/sgml/html/bug-reporting.html`` | 23 B | | | 6537 B | 6537 B | 6537 B |
| `echo "first line: $(head -n 1 doc/src/sgml/html/bug-reporting.html)"` | 67 B | | | 5111 B | 5111 B | 5111 B |
| `wc -l < doc/src/sgml/html/bug-reporting.html` | 4 B | | | 3392 B | 3392 B | 3392 B |
| `cat doc/src/sgml/html/bug-reporting.html > /dev/null 2>&1` | 0 B | | | 3493 B | 3493 B | 3493 B |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 6 B | | | 4158 B | 4158 B | 4158 B |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2 B | | | 4072 B | 4072 B | 4072 B |
| `grep -q not doc/src/sgml/html/bug-reporting.html && echo found \|\| echo none` | 6 B | | | 11.8 KB | 11.8 KB | 11.8 KB |
| `false \|\| echo fallback` | 9 B | | | 2861 B | 2861 B | 2861 B |
| `true && grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | | | 10.6 KB | 10.6 KB | 10.6 KB |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 14 B | | | 4178 B | 4178 B | 4178 B |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2 B | | | 3130 B | 3130 B | 3130 B |
| `echo one \⏎two` | 8 B | | | 2591 B | 2591 B | 2591 B |
| `grep -c not doc/src/sgml/html/bug-reporting.html⏎wc -l doc/src/sgml/html/bug-reporting.html` | 44 B | | | 11.2 KB | 11.2 KB | 11.2 KB |

## Search terms and paths

| Placeholder | Value | Measured |
|---|---|---|
| ABSENT | `qdbzzabsentterm` | 0 of 8419 files, 0.00% |
| COMMON | `not` | 5013 of 8419 files, 59.54% |
| DIR | `src/backend` | holds 1350 of 8419 files, 16.04% |
| EXT | `html` | 1169 files in the whole dataset |
| EXT2 | `c` | 1437 files in the whole dataset |
| FEW | `undesirable` | 85 of 8419 files, 1.01% |
| FILE | `doc/src/sgml/html/bug-reporting.html` | 19990 bytes |
| GLOB | `doc/src/sgml/html/*.html` | 1169 files |
| GLOBDIR | `doc/src/sgml/html` | the folder of GLOB |
| ONE | `abcdefghijklabcdefgh` | 1 of 8419 files, 0.01% |
| PREFIX | `size` | PREFIX_[a-z]+\( matches in 45 of 8419 files, 0.53% |
| RARE | `btvarstrequalimage` | 9 of 8419 files, 0.11% |
| RAREUP | `BTVARSTREQUALIMAGE` | the RARE term in capitals, for -i |
| SMALLDIR | `src/include/access` | holds 88 of 8419 files, 1.05% |
| SOME | `passed` | 843 of 8419 files, 10.01% |
