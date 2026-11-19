# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 5, on disk, settled), and include starting bash and the tool (1.865 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 178.8 | 978.7 | 187.2 MB | 192.5 MB | 0.272 | yes |
| Stage 3, on disk, settled | on | 1562.6 | 1562.6 | 2238.5 KB | 136.4 MB | 0.316 | yes |
| Stage 4, in memory | on | 293.8 | 1358.5 | 199.7 MB | 205.8 MB | 0.297 | yes |
| Stage 4, on disk, settled | on | 1563.3 | 1563.3 | 2311.0 KB | 135.6 MB | 0.392 | yes |
| Stage 4, on disk, after the writes | on | 1563.3 | 1563.3 | 2311.0 KB | 135.6 MB | 0.392 | yes |
| Stage 5, in memory | on | 2864.9 | 3849.9 | 199.7 MB | 203.2 MB | 0.339 | NO |
| Stage 5, on disk, settled | on | 1722.9 | 1722.9 | 2311.0 KB | 136.0 MB | 0.343 | NO |

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

- Built in 1064.7 ms while loading.
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

## The index (Stage 5, in memory)

- Built in 984.9 ms while loading.
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
| `cp doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | documents +1 -0; postings +2752; list bytes rewritten 0; rebuilds 0 |
| `diff -u --label old --label new doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff doc/src/sgml/html/bug-reporting.html bench_out/v2.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -q doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -u doc/src/sgml/html/bug-reporting.html bench_out/copy.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -rq src/backend bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | documents +1 -0; postings +23; list bytes rewritten 0; rebuilds 0 |
| `diff -rq src/backend bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -r doc/src/sgml/html bench_out/work \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -ru doc/src/sgml/html bench_out/work \| grep -c '^@@'` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | documents +10000 -9994; postings +138134; list bytes rewritten 0; rebuilds 0 |
| `wc -l bench_out/loop.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for f in bench_out/work/*.html; do echo done >> "$f"; done` | documents +1169 -942; postings +2086058; list bytes rewritten 1388804; rebuilds 0 |
| `awk '{ print NR ": " $0 }' doc/src/sgml/html/bug-reporting.html > bench_out/numbered.txt` | documents +1 -0; postings +55; list bytes rewritten 513; rebuilds 0 |
| `awk 'END { print NR }' bench_out/numbered.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 97; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1; postings +0; list bytes rewritten 90; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1223; postings +0; list bytes rewritten 4937842; rebuilds 0 |

## The index (Stage 5, on disk, settled)

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
| `cp doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | documents +1 -0; postings +2752; list bytes rewritten 0; rebuilds 0 |
| `diff -u --label old --label new doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff doc/src/sgml/html/bug-reporting.html bench_out/v2.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -q doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -u doc/src/sgml/html/bug-reporting.html bench_out/copy.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -rq src/backend bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | documents +1 -0; postings +23; list bytes rewritten 0; rebuilds 0 |
| `diff -rq src/backend bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -r doc/src/sgml/html bench_out/work \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -ru doc/src/sgml/html bench_out/work \| grep -c '^@@'` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | documents +10000 -9994; postings +138134; list bytes rewritten 0; rebuilds 0 |
| `wc -l bench_out/loop.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for f in bench_out/work/*.html; do echo done >> "$f"; done` | documents +1169 -942; postings +2086058; list bytes rewritten 2488086; rebuilds 0 |
| `awk '{ print NR ": " $0 }' doc/src/sgml/html/bug-reporting.html > bench_out/numbered.txt` | documents +1 -0; postings +55; list bytes rewritten 2113; rebuilds 0 |
| `awk 'END { print NR }' bench_out/numbered.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 1389; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1; postings +0; list bytes rewritten 475; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1223; postings +0; list bytes rewritten 4807299; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 5, in memory | Stage 5, on disk, settled | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.637 |  | 0.001 | 0.001 | 0.001 | 0.002 | 0.003 | 0.003 | 0.003 | yes |
| `ls -a` | 2.590 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.004 | 0.004 | 0.004 | yes |
| `ls -R` | 11.17 |  | 0.563 | 0.288 | 0.440 | 0.571 | 0.406 | 0.425 | 0.412 | yes |
| `ls -aR src/backend` | 3.859 |  | 0.059 | 0.055 | 0.058 | 0.069 | 0.071 | 0.083 | 0.085 | yes |
| `ls -p src/backend` | 2.490 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.005 | 0.005 | yes |
| `find .` | 10.71 |  | 1.298 | 0.767 | 0.922 | 0.732 | 0.979 | 0.972 | 0.936 | yes |
| `find . -name '*.html'` | 10.64 |  | 1.444 | 0.953 | 1.051 | 1.539 | 1.869 | 1.257 | 1.186 | yes |
| `find . -name '*.c'` | 11.22 |  | 1.706 | 1.780 | 1.301 | 1.179 | 1.711 | 1.260 | 1.201 | yes |
| `find . -type d` | 9.566 |  | 1.403 | 0.718 | 0.898 | 0.852 | 1.184 | 0.942 | 0.885 | yes |
| `find . -type f` | 10.87 |  | 1.397 | 1.182 | 0.999 | 0.834 | 0.840 | 1.112 | 0.951 | yes |
| `find . -maxdepth 2 -type f` | 2.753 |  | 0.015 | 0.015 | 0.009 | 0.011 | 0.013 | 0.024 | 0.016 | yes |
| `find src/backend -name '*.html' -type f` | 5.281 |  | 0.177 | 0.189 | 0.145 | 0.211 | 0.145 | 0.181 | 0.171 | yes |
| `find . -name 'Makefile'` | 10.54 |  | 2.052 | 1.344 | 0.917 | 1.113 | 1.290 | 1.003 | 0.914 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 3.163 |  | 0.001 | 0.002 | 0.001 | 0.002 | 0.001 | 0.003 | 0.003 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 2.676 |  | 0.028 | 0.048 | 0.040 | 0.027 | 0.025 | 0.035 | 0.034 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 2.276 |  | 0.003 | 0.003 | 0.003 | 0.003 | 0.003 | 0.005 | 0.005 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2.329 |  | 0.003 | 0.002 | 0.004 | 0.003 | 0.004 | 0.005 | 0.005 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 2.312 |  | 0.006 | 0.005 | 0.010 | 0.006 | 0.006 | 0.012 | 0.013 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 2.733 |  | 0.023 | 0.020 | 0.030 | 0.026 | 0.034 | 0.025 | 0.025 | yes |
| `wc -l doc/src/sgml/html/*.html` | 17.42 |  | 27.94 | 0.901 | 0.905 | 0.824 | 1.334 | 1.072 | 1.038 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 34.52 |  | 30.00 | 30.51 | 1.850 | 2.098 | 2.310 | 1.913 | 1.959 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 3.132 |  | 0.010 | 0.010 | 0.005 | 0.005 | 0.006 | 0.017 | 0.011 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 2.504 |  | 0.004 | 0.005 | 0.006 | 0.006 | 0.007 | 0.015 | 0.009 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 2.575 |  | 0.006 | 0.007 | 0.008 | 0.009 | 0.010 | 0.021 | 0.012 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 2.647 |  | 0.205 | 0.233 | 0.082 | 0.048 | 0.048 | 0.102 | 0.055 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 2.765 |  | 0.445 | 0.590 | 0.233 | 0.142 | 0.143 | 0.236 | 0.157 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 3.386 |  | 0.078 | 0.083 | 0.036 | 0.032 | 0.037 | 0.062 | 0.061 | yes |
| `grep -rn qdbzzabsentterm .` | 166.1 | 79.95 | 0.014 | 0.021 | 0.021 | 0.033 | 0.022 | 0.023 | 0.026 | yes |
| `grep -rl qdbzzabsentterm .` | 111.7 | 67.36 | 0.015 | 0.024 | 0.016 | 0.019 | 0.019 | 0.022 | 0.025 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 149.2 | 73.91 | 0.051 | 0.027 | 0.026 | 0.038 | 0.028 | 0.039 | 0.041 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 96.59 | 73.79 | 0.038 | 0.027 | 0.033 | 0.030 | 0.032 | 0.060 | 0.039 | yes |
| `grep -rn btvarstrequalimage .` | 178.7 | 86.52 | 1.605 | 0.183 | 0.085 | 0.109 | 0.111 | 0.139 | 0.151 | yes |
| `grep -rl btvarstrequalimage .` | 116.9 | 69.38 | 1.672 | 0.085 | 0.143 | 0.091 | 0.086 | 0.117 | 0.124 | yes |
| `grep -rn undesirable .` | 201.1 | 76.93 | 2.512 | 1.028 | 0.796 | 0.656 | 0.938 | 0.721 | 0.719 | yes |
| `grep -rl undesirable .` | 146.3 | 68.34 | 1.837 | 0.535 | 0.601 | 0.594 | 0.614 | 0.489 | 0.487 | yes |
| `grep -rn passed .` | 208.2 | 79.97 | 21.08 | 15.87 | 13.12 | 13.13 | 15.90 | 6.684 | 11.73 | yes |
| `grep -rl passed .` | 119.7 | 66.14 | 10.66 | 2.978 | 4.956 | 3.612 | 3.870 | 3.018 | 2.940 | yes |
| `grep -rn not .` | 362.3 | 165.2 | 35.22 | 36.87 | 39.04 | 34.77 | 38.21 | 45.43 | 42.42 | yes |
| `grep -rl not .` | 76.87 | 76.38 | 3.506 | 2.882 | 4.009 | 3.057 | 3.189 | 2.777 | 2.814 | yes |
| `grep -rc undesirable .` | 150.5 | 70.33 | 3.520 | 1.431 | 1.312 | 1.623 | 1.181 | 0.987 | 1.047 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 176.4 | 95.34 | 1.457 | 0.106 | 0.123 | 0.131 | 0.232 | 0.152 | 0.153 | yes |
| `grep -ri undesirable .` | 179.1 | 98.93 | 3.253 | 0.632 | 0.820 | 0.601 | 0.691 | 0.679 | 1.034 | yes |
| `grep -rn undesirable src/backend` | 78.34 | 25.35 | 1.529 | 0.418 | 0.390 | 0.402 | 0.345 | 0.526 | 0.342 | yes |
| `grep -rl passed src/backend` | 34.13 | 18.46 | 1.544 | 1.218 | 1.175 | 1.162 | 1.003 | 1.034 | 1.035 | yes |
| `grep -rF 'undesirable' .` | 161.7 | 69.76 | 1.579 | 0.744 | 0.633 | 0.924 | 0.598 | 0.596 | 0.568 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 116.2 | 69.80 | 8.872 | 6.092 | 3.126 | 3.044 | 3.709 | 2.736 | 4.089 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 235.8 | 85.53 | 1.877 | 0.263 | 0.214 | 0.220 | 0.245 | 0.272 | 0.276 | yes |
| `grep -rE 'passed\|undesirable' .` | 307.9 | 89.38 | 19.57 | 20.71 | 18.64 | 15.57 | 15.67 | 16.27 | 15.72 | yes |
| `grep -rE '[0-9]{4,}' .` | 346.5 | 363.0 | 176.6 | 185.0 | 188.5 | 183.5 | 185.9 | 209.6 | 214.2 | yes |
| `grep -rn '^#include' .` | 215.5 | 91.14 | 11.65 | 9.657 | 9.201 | 5.065 | 9.599 | 5.316 | 4.903 | yes |
| `grep -rE '^(import\|from) ' .` | 188.6 | 87.73 | 43.93 | 36.34 | 36.60 | 33.28 | 33.94 | 37.80 | 36.54 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 107.9 | 43.98 | 16.54 | 16.58 | 15.39 | 14.41 | 16.76 | 13.78 | 11.45 | yes |
| `grep -rv e src/include/access` | 4.987 | 6.474 | 0.482 | 0.757 | 0.511 | 0.602 | 0.423 | 0.613 | 0.610 | yes |
| `grep -r '\(undesirable\).*\1' .` | 146.6 |  | 2.322 | 1.364 | 0.935 | 1.193 | 0.816 | 0.889 | 0.900 | yes |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 19.79 |  | | 0.675 | 0.698 | 0.614 | 0.635 | 0.776 | 0.800 | yes |
| `grep -l undesirable doc/src/sgml/html/*.html` | 21.35 |  | | 1.091 | 0.617 | 0.685 | 1.273 | 0.822 | 0.830 | yes |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 16.60 |  | | 0.607 | 0.483 | 0.594 | 0.589 | 0.676 | 0.700 | yes |
| `grep -rl passed . \| wc -l` | 118.6 |  | 5.880 | 7.873 | 4.437 | 3.142 | 4.107 | 2.944 | 2.954 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 264.0 |  | 32.83 | 30.56 | 38.24 | 26.39 | 28.83 | 29.36 | 28.74 | yes |
| `find . -name '*.html' \| wc -l` | 11.88 |  | 1.615 | 1.048 | 1.528 | 1.142 | 1.753 | 1.291 | 1.183 | yes |
| `find . -type f \| grep -c html` | 11.72 |  | 1.328 | 1.278 | 2.604 | 1.143 | 1.109 | 1.503 | 1.507 | yes |
| `grep -rn not . \| head -n 5` | 3.728 |  | 0.008 | 0.008 | 0.015 | 0.009 | 0.012 | 0.014 | 0.014 | order differs |
| `grep -rn passed . \| head -n 1` | 19.96 |  | 0.012 | 0.022 | 0.018 | 0.015 | 0.015 | 0.020 | 0.020 | order differs |
| `find . -type f \| head -n 3` | 3.816 |  | 0.046 | 0.039 | 0.045 | 0.044 | 0.051 | 0.062 | 0.061 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 4.424 |  | 0.318 | 0.500 | 0.315 | 0.305 | 0.303 | 0.368 | 0.367 | yes |
| `grep -rno undesirable .` | 210.5 | 78.36 | | | 0.749 | 0.758 | 0.916 | 0.755 | 0.790 | yes |
| `grep -rw undesirable .` | 148.0 | 73.93 | | | 0.681 | 0.725 | 0.665 | 0.682 | 0.786 | yes |
| `grep -rnw btvarstrequalimage .` | 176.0 | 73.96 | | | 0.306 | 0.194 | 0.341 | 0.233 | 0.430 | yes |
| `grep -rn -C 2 btvarstrequalimage .` | 169.4 | 332.0 | | | 0.388 | 0.393 | 0.308 | 0.431 | 0.278 | yes |
| `grep -rn -A 1 -B 1 undesirable .` | 214.9 | 373.0 | | | 1.862 | 1.569 | 2.438 | 1.299 | 1.307 | yes |
| `grep -rho 'size_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 118.7 |  | | | 7.819 | 8.254 | 8.689 | 4.510 | 4.624 | yes |
| `grep -rhow '[a-z]*btvarstrequalimage[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 119.8 |  | | | 0.232 | 0.189 | 0.194 | 0.328 | 0.356 | yes |
| `rg -j 1 qdbzzabsentterm` | 89.19 |  | | | 0.037 | 0.032 | 0.052 | 0.044 | 0.075 | yes |
| `rg -j 1 abcdefghijklabcdefgh` | 87.20 |  | | | 0.047 | 0.048 | 0.057 | 0.065 | 0.064 | yes |
| `rg -j 1 btvarstrequalimage` | 87.96 |  | | | 0.226 | 0.157 | 0.168 | 0.225 | 0.213 | yes |
| `rg -j 1 undesirable` | 89.70 |  | | | 0.918 | 1.171 | 1.109 | 1.038 | 1.703 | yes |
| `rg -j 1 passed` | 101.7 |  | | | 19.95 | 19.66 | 17.70 | 19.89 | 20.01 | yes |
| `rg -j 1 not` | 172.1 |  | | | 38.35 | 37.86 | 38.17 | 40.24 | 39.70 | yes |
| `rg -j 1 -l undesirable` | 95.41 |  | | | 0.913 | 1.119 | 0.934 | 0.896 | 1.468 | yes |
| `rg -j 1 -l not` | 108.9 |  | | | 11.81 | 12.55 | 13.02 | 11.96 | 12.17 | yes |
| `rg -j 1 -c passed` | 91.67 |  | | | 18.23 | 19.79 | 18.26 | 19.20 | 20.02 | yes |
| `rg -j 1 -i BTVARSTREQUALIMAGE` | 103.9 |  | | | 1.079 | 0.941 | 1.725 | 1.272 | 1.309 | yes |
| `rg -j 1 -w undesirable` | 88.97 |  | | | 14.62 | 14.84 | 14.09 | 16.79 | 16.97 | yes |
| `rg -j 1 -F 'undesirable'` | 98.65 |  | | | 1.115 | 0.975 | 1.109 | 1.022 | 0.985 | yes |
| `rg -j 1 -n -C 2 btvarstrequalimage` | 97.32 |  | | | 0.365 | 0.545 | 0.419 | 0.369 | 0.389 | yes |
| `rg -j 1 'size_[a-z]+\('` | 89.33 |  | | | 4.521 | 8.717 | 8.127 | 4.143 | 4.198 | yes |
| `rg -j 1 'btvarstrequalimage\|abcdefghijklabcdefgh'` | 100.2 |  | | | 0.332 | 0.231 | 0.207 | 0.287 | 0.296 | yes |
| `rg -j 1 '\bbtvarstrequalimage\w*'` | 91.53 |  | | | 3.338 | 3.475 | 4.571 | 3.658 | 3.676 | yes |
| `rg -j 1 -t c undesirable` | 49.82 |  | | | 1.155 | 0.750 | 0.676 | 0.776 | 0.783 | yes |
| `rg -j 1 -g '*.html' undesirable` | 40.38 |  | | | 0.448 | 0.650 | 0.465 | 0.593 | 0.591 | yes |
| `rg -j 1 undesirable src/backend` | 28.74 |  | | | 0.777 | 0.473 | 0.561 | 0.795 | 0.503 | yes |
| `rg -j 1 -uu undesirable` | 73.18 |  | | | 0.739 | 0.738 | 0.604 | 0.645 | 0.631 | yes |
| `rg -j 1 --sort path btvarstrequalimage` | 91.54 |  | | | 0.240 | 0.299 | 0.148 | 0.221 | 0.221 | yes |
| `rg -j 1 --files \| wc -l` | 29.10 |  | | | 11.03 | 6.631 | 7.619 | 8.269 | 8.418 | yes |
| `rg -j 1 -l passed \| wc -l` | 89.35 |  | | | 12.41 | 15.16 | 13.88 | 14.81 | 12.44 | yes |
| `ls -ltR` | 38.77 |  | | | 4.384 | 3.721 | 4.162 | 5.188 | 4.993 | yes |
| `ls -lt doc/src/sgml/html` | 7.668 |  | | | 0.550 | 0.443 | 0.441 | 0.624 | 0.610 | yes |
| `ls -ltr src/backend \| tail -n 3` | 3.329 |  | | | 0.033 | 0.021 | 0.022 | 0.054 | 0.029 | yes |
| `find . -newer doc/src/sgml/html/bug-reporting.html` | 18.05 |  | | | 1.745 | 0.779 | 0.857 | 1.776 | 1.075 | yes |
| `find . -newer doc/src/sgml/html/bug-reporting.html -name '*.html'` | 11.87 |  | | | 1.884 | 1.039 | 1.044 | 2.218 | 2.221 | yes |
| `find . -type f -mtime +30` | 16.68 |  | | | 1.434 | 0.797 | 0.794 | 0.972 | 1.696 | yes |
| `du -s .` | 15.31 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | not compared |
| `du -sh src/backend` | 4.478 |  | | | 0.002 | 0.001 | 0.001 | 0.003 | 0.003 | not compared |
| `du -sb .` | 14.93 |  | | | 0.001 | 0.001 | 0.001 | 0.004 | 0.003 | yes |
| `du -sb src/backend` | 4.226 |  | | | 0.001 | 0.001 | 0.002 | 0.005 | 0.003 | yes |
| `du -b -d 1 src/backend` | 4.346 |  | | | 0.004 | 0.004 | 0.005 | 0.007 | 0.011 | yes |
| `sort doc/src/sgml/html/bug-reporting.html` | 2.949 |  | | | 0.022 | 0.021 | 0.024 | 0.027 | 0.026 | yes |
| `sort -r doc/src/sgml/html/bug-reporting.html \| head -n 3` | 3.693 |  | | | 0.026 | 0.027 | 0.023 | 0.030 | 0.031 | yes |
| `sort -u doc/src/sgml/html/bug-reporting.html \| wc -l` | 3.437 |  | | | 0.026 | 0.023 | 0.028 | 0.049 | 0.031 | yes |
| `sort -k 2 doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 3.757 |  | | | 0.026 | 0.027 | 0.031 | 0.060 | 0.033 | yes |
| `cat doc/src/sgml/html/*.html \| sort \| uniq -c \| sort -rn \| head -n 5` | 235.9 |  | | | 103.5 | 120.2 | 105.0 | 121.7 | 124.6 | yes |
| `uniq -c doc/src/sgml/html/bug-reporting.html \| wc -l` | 3.649 |  | | | 0.017 | 0.020 | 0.021 | 0.023 | 0.023 | yes |
| `cut -c 1-20 doc/src/sgml/html/bug-reporting.html` | 2.504 |  | | | 0.016 | 0.016 | 0.014 | 0.019 | 0.020 | yes |
| `cut -d ' ' -f 1 doc/src/sgml/html/bug-reporting.html \| sort \| uniq \| wc -l` | 3.668 |  | | | 0.042 | 0.037 | 0.042 | 0.058 | 0.057 | yes |
| `tr a-z A-Z < doc/src/sgml/html/bug-reporting.html` | 2.871 |  | | | 0.004 | 0.003 | 0.003 | 0.006 | 0.006 | yes |
| `tr -d '\n' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 3.106 |  | | | 0.008 | 0.008 | 0.008 | 0.014 | 0.014 | yes |
| `tr -s ' ' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 3.089 |  | | | 0.015 | 0.020 | 0.015 | 0.019 | 0.019 | yes |
| `nl doc/src/sgml/html/bug-reporting.html` | 2.407 |  | | | 0.028 | 0.039 | 0.023 | 0.037 | 0.038 | yes |
| `nl -ba doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 2.989 |  | | | 0.027 | 0.049 | 0.026 | 0.042 | 0.042 | yes |
| `seq 1 100000 \| wc -l` | 4.299 |  | | | 2.278 | 4.282 | 2.486 | 3.080 | 3.167 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.087 |  | | | 0.026 | 0.026 | 0.027 | 0.063 | 0.038 | yes |
| `basename doc/src/sgml/html/bug-reporting.html` | 2.395 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | yes |
| `basename -s .html doc/src/sgml/html/bug-reporting.html` | 2.369 |  | | | 0.001 | 0.001 | 0.002 | 0.004 | 0.004 | yes |
| `dirname doc/src/sgml/html/bug-reporting.html` | 2.342 |  | | | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | yes |
| `printf '%s\n' a b c` | 1.540 |  | | | 0.002 | 0.002 | 0.003 | 0.004 | 0.004 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 1.570 |  | | | 0.002 | 0.003 | 0.003 | 0.004 | 0.004 | yes |
| `test -f doc/src/sgml/html/bug-reporting.html && echo yes` | 1.586 |  | | | 0.002 | 0.002 | 0.002 | 0.005 | 0.005 | yes |
| `[ -d src/backend ] && echo folder` | 1.587 |  | | | 0.002 | 0.003 | 0.003 | 0.005 | 0.005 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 2.350 |  | | | 0.003 | 0.003 | 0.005 | 0.005 | 0.005 | yes |
| `echo a b c \| xargs -n 1 echo` | 6.360 |  | | | 0.003 | 0.005 | 0.006 | 0.007 | 0.007 | yes |
| `find src/include/access -name '*.html' \| xargs cat \| wc -l` | 4.273 |  | | | 0.011 | 0.017 | 0.014 | 0.018 | 0.017 | yes |
| `find . -name '*.html' \| xargs grep -l undesirable \| wc -l` | 29.71 |  | | | 2.239 | 1.992 | 2.037 | 2.605 | 2.568 | yes |
| `echo {a,b,c}{1..3}` | 1.570 |  | | | 0.004 | 0.004 | 0.004 | 0.006 | 0.006 | yes |
| `echo {1..20..3} {a..e}` | 1.633 |  | | | 0.005 | 0.004 | 0.006 | 0.007 | 0.007 | yes |
| `echo $(wc -l < doc/src/sgml/html/bug-reporting.html) `basename doc/src/sgml/html/bug-reporting.html`` | 4.756 |  | | | 0.005 | 0.008 | 0.006 | 0.011 | 0.011 | yes |
| `echo "first line: $(head -n 1 doc/src/sgml/html/bug-reporting.html)"` | 2.804 |  | | | 0.005 | 0.005 | 0.006 | 0.011 | 0.007 | yes |
| `wc -l < doc/src/sgml/html/bug-reporting.html` | 3.028 |  | | | 0.002 | 0.003 | 0.002 | 0.006 | 0.007 | yes |
| `cat doc/src/sgml/html/bug-reporting.html > /dev/null 2>&1` | 3.062 |  | | | 0.002 | 0.002 | 0.002 | 0.005 | 0.004 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 2.683 |  | | | 0.002 | 0.003 | 0.006 | 0.007 | 0.005 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 3.360 |  | | | 0.002 | 0.003 | 0.004 | 0.008 | 0.005 | yes |
| `grep -q not doc/src/sgml/html/bug-reporting.html && echo found \|\| echo none` | 2.853 |  | | | 0.005 | 0.009 | 0.007 | 0.015 | 0.010 | yes |
| `false \|\| echo fallback` | 1.492 |  | | | 0.001 | 0.002 | 0.002 | 0.005 | 0.003 | yes |
| `true && grep -c not doc/src/sgml/html/bug-reporting.html` | 2.518 |  | | | 0.007 | 0.012 | 0.007 | 0.010 | 0.011 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 3.076 |  | | | 0.003 | 0.003 | 0.006 | 0.005 | 0.006 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2.863 |  | | | 0.001 | 0.002 | 0.001 | 0.003 | 0.003 | yes |
| `echo one \⏎two` | 1.578 |  | | | 0.001 | 0.002 | 0.001 | 0.003 | 0.003 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html⏎wc -l doc/src/sgml/html/bug-reporting.html` | 3.834 |  | | | 0.007 | 0.008 | 0.009 | 0.012 | 0.012 | yes |
| `for i in $(seq 100000); do :; done` | 104.4 |  | | | | | | 38.46 | 38.82 | yes |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 31.16 |  | | | | | | 25.00 | 24.43 | yes |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 1.607 |  | | | | | | 0.032 | 0.032 | yes |
| `for f in doc/src/sgml/html/*.html; do echo "$f"; done \| wc -l` | 16.78 |  | | | | | | 2.300 | 1.228 | yes |
| `for f in doc/src/sgml/html/*.html; do grep -c not "$f"; done \| sort -n \| tail -n 3` | 1657.5 |  | | | | | | 7.874 | 8.304 | yes |
| `n=0; for f in doc/src/sgml/html/*.html; do if grep -q passed "$f"; then n=$((n+1)); fi; done; echo $n` | 1650.3 |  | | | | | | 17.88 | 12.49 | yes |
| `for f in doc/src/sgml/html/*.html; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 7.546 |  | | | | | | 2.425 | 4.545 | yes |
| `H=$PWD; for d in src/backend src/include/access doc/src/sgml/html; do cd "$d"; ls \| wc -l; cd "$H"; done` | 8.085 |  | | | | | | 0.055 | 0.054 | yes |
| `X=not; N=$(grep -c "$X" doc/src/sgml/html/bug-reporting.html); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 3.463 |  | | | | | | 0.016 | 0.016 | yes |
| `find src/include/access -name '*.html' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 3.642 |  | | | | | | 0.035 | 0.022 | yes |
| `for f in doc/src/sgml/html/*.html; do head -n 1 "$f"; done > /dev/null` | 1421.2 |  | | | | | | 2.087 | 1.989 | yes |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 1.870 |  | | | | | | 0.019 | 0.019 | yes |
| `awk '{ print $2 }' doc/src/sgml/html/bug-reporting.html` | 3.171 |  | | | | | | 0.006 | 0.006 | NO: exit status, real 0, engine 2 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 59.55 |  | | | | | | 25.55 | 26.44 | yes |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '/not/ { n++ } END { print n }'` | 50.93 |  | | | | | | 19.47 | 19.13 | yes |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 86.74 |  | | | | | | 48.13 | 47.43 | yes |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 43.60 |  | | | | | | 21.38 | 19.37 | yes |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 34.33 |  | | | | | | 17.57 | 16.91 | yes |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 35.94 |  | | | | | | 17.85 | 17.20 | yes |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 61.23 |  | | | | | | 41.26 | 39.74 | yes |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "not") n++ } END { print n }'` | 222.7 |  | | | | | | 163.3 | 148.9 | yes |
| `awk '{ gsub(/[0-9]+/, "N"); print }' doc/src/sgml/html/bug-reporting.html \| wc -c` | 3.827 |  | | | | | | 0.059 | 0.056 | NO: standard output differs |
| `awk -v OFS=- '{ $1 = NR; print }' doc/src/sgml/html/bug-reporting.html \| tail -n 1` | 4.102 |  | | | | | | 0.011 | 0.011 | NO: standard output differs |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 15.06 |  | | | | | | 6.178 | 5.380 | yes |
| `grep -rn undesirable . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 201.5 |  | | | | | | 0.447 | 0.255 | NO: standard output differs |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 175.4 |  | | | | | | 121.1 | 110.7 | yes |
| `mkdir -p bench_out/a/b` | 3.610 |  | 0.010 | 0.676 | 0.015 | 0.695 | | 0.025 | 0.783 | yes |
| `touch bench_out/empty.txt` | 3.125 |  | 0.012 | 0.692 | 0.016 | 0.529 | | 0.026 | 0.478 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.920 |  | 0.019 | 0.624 | 0.030 | 0.513 | | 0.060 | 0.465 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.603 |  | 0.018 | 0.620 | 0.026 | 0.443 | | 0.091 | 0.649 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 2.868 |  | 0.316 | 1.333 | 0.025 | 0.539 | | 0.031 | 0.443 | yes |
| `cp -r src/backend bench_out/dir_copy` | 66.60 |  | 372.3 | 491.9 | 1.700 | 3.259 | | 2.301 | 3.649 | yes |
| `cp -r doc/src/sgml/html bench_out/work` | 110.5 |  | 101.0 | 169.3 | 1.688 | 4.011 | | 3.370 | 3.226 | yes |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 3.610 |  | 0.538 | 1.175 | 0.585 | 1.791 | | 1.097 | 1.310 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.406 |  | 0.492 | 1.118 | 0.461 | 1.462 | | 0.669 | 1.145 | yes |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 192.7 |  | 296.5 | 311.9 | 313.4 | 316.5 | | 201.0 | 247.1 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | 313.6 |  | | 2.342 | 1.067 | 18.80 | | 1.106 | 16.59 | yes |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | 377.9 |  | | 20.30 | 2.182 | 20.43 | | 1.796 | 19.63 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 3.373 |  | | 0.042 | 0.029 | 0.650 | | 0.016 | 0.502 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 4.015 |  | | 0.042 | 0.029 | 0.650 | | 0.016 | 0.502 | yes |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | 3.146 |  | | 0.035 | 0.039 | 0.628 | | 0.043 | 0.536 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 28.98 |  | 4.294 | 4.209 | 3.365 | 4.242 | | 4.132 | 5.267 | yes |
| `grep -rl QDBBENCHMANY .` | 87.85 |  | 1.838 | 1.769 | 1.368 | 1.389 | | 2.194 | 1.428 | yes |
| `grep -rl QDBBENCHONE .` | 114.6 |  | 0.384 | 0.368 | 0.368 | 0.329 | | 0.432 | 0.349 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| tee bench_out/tee.txt \| wc -l` | 4.100 |  | | | 0.343 | 1.450 | | 0.441 | 0.989 | yes |
| `sort doc/src/sgml/html/bug-reporting.html > bench_out/sorted.txt` | 3.849 |  | | | 0.331 | 0.995 | | 0.444 | 0.927 | yes |
| `sort -u doc/src/sgml/html/bug-reporting.html >> bench_out/sorted.txt` | 3.319 |  | | | 0.408 | 1.046 | | 0.409 | 1.109 | yes |
| `cat qdb_no_such_file &> bench_out/both.txt` | 3.115 |  | | | 0.035 | 0.549 | | 0.039 | 0.538 | yes |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 2.769 |  | | | 0.032 | 0.533 | | 0.039 | 0.386 | yes |
| `find src/include/access -name '*.html' \| sort \| xargs cat > bench_out/joined.txt` | 5.462 |  | | | 0.068 | 0.503 | | 0.072 | 0.501 | yes |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 2.918 |  | | | 0.020 | 0.441 | | 0.025 | 0.418 | yes |
| `sed -i '1d' bench_out/joined2.txt` | 2.831 |  | | | 0.021 | 0.462 | | 0.026 | 0.372 | yes |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 2.655 |  | | | 0.067 | 0.026 | | 0.025 | 0.029 | yes |
| `cat bench_out/both.txt` | 2.301 |  | | | 0.009 | 0.018 | | 0.018 | 0.019 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | 5.061 |  | | | | | | 0.028 | 0.506 | yes |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 2.920 |  | | | | | | 0.369 | 0.940 | yes |
| `diff -u --label old --label new doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | 2.456 |  | | | | | | 0.109 | 0.107 | yes |
| `diff doc/src/sgml/html/bug-reporting.html bench_out/v2.txt \| wc -l` | 3.115 |  | | | | | | 0.101 | 0.091 | yes |
| `diff -q doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 2.534 |  | | | | | | 0.024 | 0.025 | yes |
| `diff -u doc/src/sgml/html/bug-reporting.html bench_out/copy.txt \| wc -l` | 3.077 |  | | | | | | 0.184 | 0.178 | yes |
| `diff -rq src/backend bench_out/dir_copy` | 36.07 |  | | | | | | 0.737 | 0.625 | yes |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 6.980 |  | | | | | | 0.702 | 1.016 | yes |
| `diff -rq src/backend bench_out/dir_copy` | 35.33 |  | | | | | | 0.737 | 0.625 | yes |
| `diff -r doc/src/sgml/html bench_out/work \| wc -l` | 61.42 |  | | | | | | 78.77 | 50.82 | yes |
| `diff -ru doc/src/sgml/html bench_out/work \| grep -c '^@@'` | 71.95 |  | | | | | | 71.02 | 63.98 | yes |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 53.34 |  | | | | | | 27.57 | 42.63 | yes |
| `wc -l bench_out/loop.txt` | 3.055 |  | | | | | | 0.026 | 0.023 | yes |
| `for f in bench_out/work/*.html; do echo done >> "$f"; done` | 10.85 |  | | | | | | 128.9 | 155.1 | yes |
| `awk '{ print NR ": " $0 }' doc/src/sgml/html/bug-reporting.html > bench_out/numbered.txt` | 5.020 |  | | | | | | 0.092 | 0.718 | NO: exit status, real 0, engine 2 |
| `awk 'END { print NR }' bench_out/numbered.txt` | 3.220 |  | | | | | | 0.034 | 0.042 | NO: standard output differs |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.767 |  | 0.024 | 0.805 | 0.019 | 0.565 | | 0.026 | 0.608 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.639 |  | 0.022 | 0.852 | 0.021 | 0.556 | | 0.024 | 0.363 | yes |
| `ls -R bench_out/a` | 2.605 |  | 0.012 | 0.026 | 0.020 | 0.017 | | 0.021 | 0.021 | yes |
| `rm bench_out/a/b/hello.txt` | 2.432 |  | 0.008 | 0.704 | 0.021 | 0.540 | | 0.021 | 0.633 | yes |
| `rmdir bench_out/a/b` | 2.628 |  | 0.010 | 0.327 | 0.017 | 0.355 | | 0.019 | 0.387 | yes |
| `rm -r bench_out/dir_moved` | 22.29 |  | 12.58 | 38.26 | 0.736 | 1.271 | | 0.719 | 1.270 | yes |
| `rm -rf bench_out` | 115.1 |  | 19.33 | 25.07 | 10.61 | 11.43 | | 22.55 | 19.63 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 5, in memory | Stage 5, on disk, settled |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 144 B | 961 B | 961 B | 2795 B | 2795 B | 2795 B | 2608 B | 2608 B |
| `ls -a` | 307 B | 1785 B | 1785 B | 3877 B | 3877 B | 3877 B | 3786 B | 3786 B |
| `ls -R` | 151.8 KB | 73.0 KB | 73.0 KB | 93.5 KB | 93.5 KB | 93.5 KB | 93.4 KB | 93.4 KB |
| `ls -aR src/backend` | 23.1 KB | 29.2 KB | 29.2 KB | 33.1 KB | 33.1 KB | 33.1 KB | 33.1 KB | 33.1 KB |
| `ls -p src/backend` | 279 B | 2075 B | 2075 B | 4306 B | 4306 B | 4306 B | 4311 B | 4311 B |
| `find .` | 347.5 KB | 28.5 KB | 28.5 KB | 30.3 KB | 30.3 KB | 30.3 KB | 30.3 KB | 30.3 KB |
| `find . -name '*.html'` | 49.6 KB | 18.3 KB | 18.3 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.4 KB | 20.4 KB |
| `find . -name '*.c'` | 54.0 KB | 33.0 KB | 33.0 KB | 35.1 KB | 35.1 KB | 35.1 KB | 35.2 KB | 35.2 KB |
| `find . -type d` | 16.8 KB | 17.5 KB | 17.5 KB | 19.6 KB | 19.6 KB | 19.6 KB | 19.7 KB | 19.7 KB |
| `find . -type f` | 330.7 KB | 26.7 KB | 26.7 KB | 28.9 KB | 28.9 KB | 28.9 KB | 29.0 KB | 29.0 KB |
| `find . -maxdepth 2 -type f` | 992 B | 2820 B | 2820 B | 5417 B | 5417 B | 5417 B | 5710 B | 5710 B |
| `find src/backend -name '*.html' -type f` | 0 B | 1952 B | 1952 B | 4198 B | 4198 B | 4198 B | 4491 B | 4491 B |
| `find . -name 'Makefile'` | 9792 B | 11.6 KB | 11.6 KB | 13.7 KB | 13.7 KB | 13.7 KB | 13.8 KB | 13.8 KB |
| `cat doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 1068 B | 1068 B | 2467 B | 2467 B | 2467 B | 2384 B | 2384 B |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 21.7 KB | 33.0 KB | 33.0 KB | 35.2 KB | 35.2 KB | 35.2 KB | 35.2 KB | 35.2 KB |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 3143 B | 4075 B | 4075 B | 6167 B | 6167 B | 6167 B | 6268 B | 6268 B |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2038 B | 2970 B | 2970 B | 5062 B | 5062 B | 5062 B | 5163 B | 5163 B |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 11.5 KB | 12.5 KB | 12.5 KB | 14.6 KB | 14.6 KB | 14.6 KB | 14.7 KB | 14.7 KB |
| `wc doc/src/sgml/html/bug-reporting.html` | 55 B | 1076 B | 1076 B | 2662 B | 2662 B | 2662 B | 2571 B | 2571 B |
| `wc -l doc/src/sgml/html/*.html` | 57.8 KB | 268.9 KB | 268.9 KB | 269.6 KB | 269.6 KB | 269.6 KB | 270.3 KB | 270.3 KB |
| `cat doc/src/sgml/html/*.html \| wc -l` | 7 B | 269.1 KB | 269.1 KB | 270.0 KB | 270.0 KB | 270.0 KB | 270.9 KB | 270.9 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 565 B | 10.3 KB | 10.5 KB | 15.4 KB | 15.4 KB | 15.4 KB | 17.9 KB | 17.9 KB |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | 5856 B | 5856 B | 10.1 KB | 10.1 KB | 10.1 KB | 10.3 KB | 10.3 KB |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3886 B | 6933 B | 6933 B | 10.1 KB | 10.1 KB | 10.1 KB | 10.3 KB | 10.3 KB |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19.9 KB | 35.9 KB | 35.9 KB | 24.7 KB | 24.7 KB | 24.7 KB | 27.7 KB | 27.7 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 37.1 KB | 37.1 KB | 30.0 KB | 30.0 KB | 30.0 KB | 33.6 KB | 33.6 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6 B | 36.7 KB | 36.9 KB | 39.8 KB | 39.8 KB | 39.8 KB | 43.6 KB | 43.6 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.7 KB | 31.7 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.7 KB | 31.7 KB |
| `grep -rn abcdefghijklabcdefgh .` | 170 B | 26.8 KB | 26.8 KB | 40.8 KB | 40.8 KB | 40.8 KB | 41.1 KB | 41.1 KB |
| `grep -rl abcdefghijklabcdefgh .` | 41 B | 26.8 KB | 26.8 KB | 40.8 KB | 40.8 KB | 40.8 KB | 41.1 KB | 41.1 KB |
| `grep -rn btvarstrequalimage .` | 1660 B | 24.6 KB | 24.6 KB | 37.5 KB | 37.5 KB | 37.5 KB | 37.7 KB | 37.7 KB |
| `grep -rl btvarstrequalimage .` | 316 B | 24.6 KB | 24.6 KB | 37.5 KB | 37.5 KB | 37.5 KB | 37.7 KB | 37.7 KB |
| `grep -rn undesirable .` | 11.2 KB | 29.2 KB | 29.1 KB | 31.3 KB | 31.3 KB | 31.3 KB | 31.4 KB | 31.4 KB |
| `grep -rl undesirable .` | 2982 B | 29.2 KB | 29.1 KB | 31.3 KB | 31.3 KB | 31.3 KB | 31.4 KB | 31.4 KB |
| `grep -rn passed .` | 225.8 KB | 15.3 KB | 25.9 KB | 28.2 KB | 28.2 KB | 29.1 KB | 28.3 KB | 28.3 KB |
| `grep -rl passed .` | 30.4 KB | 9696 B | 25.6 KB | 27.8 KB | 27.8 KB | 28.7 KB | 27.9 KB | 27.9 KB |
| `grep -rn not .` | 13.4 MB | 31.1 KB | 31.1 KB | 33.3 KB | 33.3 KB | 33.3 KB | 33.4 KB | 33.4 KB |
| `grep -rl not .` | 189.7 KB | 5758 B | 5758 B | 9977 B | 9977 B | 9977 B | 10.2 KB | 10.2 KB |
| `grep -rc undesirable .` | 347.5 KB | 44.4 KB | 44.9 KB | 47.1 KB | 47.1 KB | 47.1 KB | 47.2 KB | 47.2 KB |
| `grep -rin BTVARSTREQUALIMAGE .` | 1730 B | 24.6 KB | 24.6 KB | 37.5 KB | 37.5 KB | 37.5 KB | 37.7 KB | 37.7 KB |
| `grep -ri undesirable .` | 10.8 KB | 29.2 KB | 29.0 KB | 31.3 KB | 31.3 KB | 31.3 KB | 31.4 KB | 31.4 KB |
| `grep -rn undesirable src/backend` | 3398 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | 24.8 KB | 25.1 KB | 25.1 KB |
| `grep -rl passed src/backend` | 10.7 KB | 9717 B | 25.7 KB | 27.9 KB | 27.9 KB | 28.8 KB | 28.0 KB | 28.0 KB |
| `grep -rF 'undesirable' .` | 10.8 KB | 29.2 KB | 29.1 KB | 31.3 KB | 31.3 KB | 31.3 KB | 31.4 KB | 31.4 KB |
| `grep -rE 'size_[a-z]+\(' .` | 20.4 KB | 117.2 KB | 124.5 KB | 126.8 KB | 126.8 KB | 126.8 KB | 126.9 KB | 126.9 KB |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1754 B | 49.6 KB | 49.6 KB | 74.2 KB | 74.2 KB | 74.2 KB | 74.4 KB | 74.4 KB |
| `grep -rE 'passed\|undesirable' .` | 228.0 KB | 24.0 KB | 27.2 KB | 36.4 KB | 36.4 KB | 36.4 KB | 36.6 KB | 36.6 KB |
| `grep -rE '[0-9]{4,}' .` | 39.4 MB | 23.1 KB | 23.1 KB | 25.4 KB | 25.4 KB | 25.4 KB | 25.5 KB | 25.5 KB |
| `grep -rn '^#include' .` | 1219.8 KB | 33.9 KB | 55.2 KB | 56.2 KB | 57.5 KB | 57.5 KB | 56.4 KB | 57.6 KB |
| `grep -rE '^(import\|from) ' .` | 61.8 KB | 23.9 KB | 23.9 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.3 KB | 30.3 KB |
| `grep -rin 'return [a-z_]*;$' src/backend` | 902.7 KB | 50.4 KB | 66.9 KB | 66.0 KB | 69.2 KB | 66.0 KB | 66.1 KB | 69.3 KB |
| `grep -rv e src/include/access` | 232.5 KB | 27.5 KB | 27.5 KB | 29.7 KB | 29.7 KB | 29.7 KB | 29.8 KB | 29.8 KB |
| `grep -r '\(undesirable\).*\1' .` | 0 B | 33.0 KB | 32.9 KB | 35.1 KB | 35.1 KB | 35.1 KB | 35.3 KB | 35.3 KB |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 125 B | | 269.2 KB | 270.2 KB | 270.2 KB | 270.2 KB | 271.0 KB | 271.0 KB |
| `grep -l undesirable doc/src/sgml/html/*.html` | 976 B | | 269.2 KB | 270.2 KB | 270.2 KB | 270.2 KB | 271.0 KB | 271.0 KB |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 49.6 KB | | 269.2 KB | 270.2 KB | 270.2 KB | 270.2 KB | 271.0 KB | 271.0 KB |
| `grep -rl passed . \| wc -l` | 4 B | 10.5 KB | 26.5 KB | 28.8 KB | 28.8 KB | 29.7 KB | 30.4 KB | 30.4 KB |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 5 B | 10.9 KB | 11.1 KB | 14.0 KB | 14.0 KB | 14.0 KB | 16.8 KB | 16.8 KB |
| `find . -name '*.html' \| wc -l` | 5 B | 19.0 KB | 19.1 KB | 21.3 KB | 21.3 KB | 21.3 KB | 22.9 KB | 22.9 KB |
| `find . -type f \| grep -c html` | 5 B | 34.3 KB | 34.5 KB | 39.7 KB | 39.7 KB | 39.7 KB | 41.6 KB | 41.6 KB |
| `grep -rn not . \| head -n 5` | 386 B | 6624 B | 6736 B | 11.2 KB | 11.2 KB | 11.2 KB | 13.0 KB | 13.0 KB |
| `grep -rn passed . \| head -n 1` | 88 B | 10.3 KB | 10.4 KB | 16.6 KB | 16.6 KB | 16.6 KB | 18.4 KB | 18.4 KB |
| `find . -type f \| head -n 3` | 61 B | 27.5 KB | 27.6 KB | 30.0 KB | 30.0 KB | 30.0 KB | 31.6 KB | 31.6 KB |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 1555 B | 269.2 KB | 269.2 KB | 270.3 KB | 270.3 KB | 270.3 KB | 271.3 KB | 271.3 KB |
| `grep -rno undesirable .` | 5210 B | | | 38.9 KB | 38.9 KB | 38.9 KB | 39.1 KB | 39.1 KB |
| `grep -rw undesirable .` | 10.8 KB | | | 42.3 KB | 42.3 KB | 42.3 KB | 42.4 KB | 42.4 KB |
| `grep -rnw btvarstrequalimage .` | 1660 B | | | 44.4 KB | 44.4 KB | 44.4 KB | 44.6 KB | 44.6 KB |
| `grep -rn -C 2 btvarstrequalimage .` | 6371 B | | | 38.2 KB | 38.2 KB | 38.2 KB | 38.6 KB | 38.6 KB |
| `grep -rn -A 1 -B 1 undesirable .` | 31.9 KB | | | 32.5 KB | 32.5 KB | 32.5 KB | 33.0 KB | 33.0 KB |
| `grep -rho 'size_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 188 B | | | 227.5 KB | 227.4 KB | 227.4 KB | 234.9 KB | 234.9 KB |
| `grep -rhow '[a-z]*btvarstrequalimage[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 27 B | | | 75.7 KB | 75.7 KB | 75.7 KB | 79.2 KB | 79.2 KB |
| `rg -j 1 qdbzzabsentterm` | 0 B | | | 30.8 KB | 30.8 KB | 30.8 KB | 30.9 KB | 30.9 KB |
| `rg -j 1 abcdefghijklabcdefgh` | 158 B | | | 39.8 KB | 39.8 KB | 39.8 KB | 40.0 KB | 40.0 KB |
| `rg -j 1 btvarstrequalimage` | 1562 B | | | 58.2 KB | 58.2 KB | 58.2 KB | 58.3 KB | 58.3 KB |
| `rg -j 1 undesirable` | 10.6 KB | | | 109.8 KB | 109.8 KB | 109.8 KB | 109.9 KB | 109.9 KB |
| `rg -j 1 passed` | 213.1 KB | | | 436.7 KB | 436.7 KB | 437.6 KB | 436.8 KB | 436.8 KB |
| `rg -j 1 not` | 12.6 MB | | | 426.5 KB | 426.5 KB | 426.5 KB | 426.6 KB | 426.6 KB |
| `rg -j 1 -l undesirable` | 2812 B | | | 100.4 KB | 100.4 KB | 100.4 KB | 100.6 KB | 100.6 KB |
| `rg -j 1 -l not` | 179.6 KB | | | 425.4 KB | 425.4 KB | 425.4 KB | 425.6 KB | 425.6 KB |
| `rg -j 1 -c passed` | 30.4 KB | | | 433.1 KB | 433.1 KB | 434.0 KB | 433.3 KB | 433.3 KB |
| `rg -j 1 -i BTVARSTREQUALIMAGE` | 1625 B | | | 90.0 KB | 89.9 KB | 89.9 KB | 90.3 KB | 90.2 KB |
| `rg -j 1 -w undesirable` | 10.6 KB | | | 131.8 KB | 131.8 KB | 131.8 KB | 132.0 KB | 132.0 KB |
| `rg -j 1 -F 'undesirable'` | 10.6 KB | | | 110.3 KB | 110.3 KB | 110.3 KB | 110.5 KB | 110.5 KB |
| `rg -j 1 -n -C 2 btvarstrequalimage` | 6253 B | | | 68.1 KB | 68.1 KB | 68.1 KB | 68.6 KB | 68.6 KB |
| `rg -j 1 'size_[a-z]+\('` | 19.9 KB | | | 262.2 KB | 262.1 KB | 262.1 KB | 262.3 KB | 262.3 KB |
| `rg -j 1 'btvarstrequalimage\|abcdefghijklabcdefgh'` | 1720 B | | | 72.6 KB | 72.6 KB | 72.6 KB | 72.8 KB | 72.8 KB |
| `rg -j 1 '\bbtvarstrequalimage\w*'` | 1562 B | | | 2620.7 KB | 2620.7 KB | 2620.7 KB | 2620.8 KB | 2620.8 KB |
| `rg -j 1 -t c undesirable` | 3729 B | | | 105.4 KB | 105.4 KB | 105.4 KB | 105.7 KB | 105.7 KB |
| `rg -j 1 -g '*.html' undesirable` | 3303 B | | | 102.1 KB | 102.1 KB | 102.1 KB | 102.5 KB | 102.5 KB |
| `rg -j 1 undesirable src/backend` | 3255 B | | | 54.9 KB | 54.9 KB | 54.9 KB | 55.1 KB | 55.1 KB |
| `rg -j 1 -uu undesirable` | 10.6 KB | | | 38.5 KB | 38.5 KB | 38.5 KB | 38.8 KB | 38.8 KB |
| `rg -j 1 --sort path btvarstrequalimage` | 1562 B | | | 58.9 KB | 58.9 KB | 58.9 KB | 59.2 KB | 59.2 KB |
| `rg -j 1 --files \| wc -l` | 5 B | | | 423.7 KB | 423.7 KB | 423.7 KB | 425.3 KB | 425.3 KB |
| `rg -j 1 -l passed \| wc -l` | 4 B | | | 446.3 KB | 446.3 KB | 447.2 KB | 448.0 KB | 448.0 KB |
| `ls -ltR` | 513.7 KB | | | 144.6 KB | 144.6 KB | 144.6 KB | 144.5 KB | 144.5 KB |
| `ls -lt doc/src/sgml/html` | 74.4 KB | | | 105.9 KB | 105.9 KB | 105.9 KB | 105.9 KB | 105.9 KB |
| `ls -ltr src/backend \| tail -n 3` | 149 B | | | 7100 B | 7100 B | 7100 B | 8649 B | 8649 B |
| `find . -newer doc/src/sgml/html/bug-reporting.html` | 288.1 KB | | | 20.0 KB | 20.0 KB | 20.0 KB | 20.1 KB | 20.1 KB |
| `find . -newer doc/src/sgml/html/bug-reporting.html -name '*.html'` | 38.8 KB | | | 24.0 KB | 24.0 KB | 24.0 KB | 24.3 KB | 24.3 KB |
| `find . -type f -mtime +30` | 0 B | | | 4081 B | 4081 B | 4081 B | 4374 B | 4374 B |
| `du -s .` | 9 B | | | 2704 B | 2704 B | 2704 B | 2709 B | 2709 B |
| `du -sh src/backend` | 16 B | | | 2750 B | 2750 B | 2750 B | 2755 B | 2755 B |
| `du -sb .` | 12 B | | | 2710 B | 2710 B | 2710 B | 2715 B | 2715 B |
| `du -sb src/backend` | 21 B | | | 2768 B | 2768 B | 2768 B | 2773 B | 2773 B |
| `du -b -d 1 src/backend` | 776 B | | | 5038 B | 5038 B | 5038 B | 5235 B | 5235 B |
| `sort doc/src/sgml/html/bug-reporting.html` | 20.0 KB | | | 106.3 KB | 106.3 KB | 106.3 KB | 106.2 KB | 106.2 KB |
| `sort -r doc/src/sgml/html/bug-reporting.html \| head -n 3` | 2009 B | | | 109.6 KB | 109.6 KB | 109.6 KB | 111.2 KB | 111.2 KB |
| `sort -u doc/src/sgml/html/bug-reporting.html \| wc -l` | 4 B | | | 107.6 KB | 107.6 KB | 107.6 KB | 109.0 KB | 109.0 KB |
| `sort -k 2 doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 209 B | | | 128.1 KB | 128.1 KB | 128.1 KB | 129.7 KB | 129.7 KB |
| `cat doc/src/sgml/html/*.html \| sort \| uniq -c \| sort -rn \| head -n 5` | 106 B | | | 57.5 MB | 57.5 MB | 57.5 MB | 57.5 MB | 57.5 MB |
| `uniq -c doc/src/sgml/html/bug-reporting.html \| wc -l` | 4 B | | | 39.5 KB | 39.5 KB | 39.5 KB | 41.0 KB | 41.0 KB |
| `cut -c 1-20 doc/src/sgml/html/bug-reporting.html` | 4915 B | | | 18.2 KB | 18.2 KB | 18.2 KB | 18.3 KB | 18.3 KB |
| `cut -d ' ' -f 1 doc/src/sgml/html/bug-reporting.html \| sort \| uniq \| wc -l` | 2 B | | | 78.4 KB | 78.4 KB | 78.4 KB | 81.8 KB | 81.8 KB |
| `tr a-z A-Z < doc/src/sgml/html/bug-reporting.html` | 20.0 KB | | | 25.0 KB | 25.0 KB | 25.0 KB | 25.1 KB | 25.1 KB |
| `tr -d '\n' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 6 B | | | 26.0 KB | 26.0 KB | 26.0 KB | 27.6 KB | 27.6 KB |
| `tr -s ' ' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 6 B | | | 26.0 KB | 26.0 KB | 26.0 KB | 27.6 KB | 27.6 KB |
| `nl doc/src/sgml/html/bug-reporting.html` | 21.7 KB | | | 36.3 KB | 36.3 KB | 36.3 KB | 36.2 KB | 36.2 KB |
| `nl -ba doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 903 B | | | 58.8 KB | 58.8 KB | 58.8 KB | 60.4 KB | 60.4 KB |
| `seq 1 100000 \| wc -l` | 7 B | | | 135.0 KB | 135.0 KB | 135.0 KB | 136.4 KB | 136.4 KB |
| `seq -w 1 1000 \| tail -n 2` | 10 B | | | 17.6 KB | 17.6 KB | 17.6 KB | 19.2 KB | 19.2 KB |
| `basename doc/src/sgml/html/bug-reporting.html` | 19 B | | | 2524 B | 2524 B | 2524 B | 2433 B | 2433 B |
| `basename -s .html doc/src/sgml/html/bug-reporting.html` | 14 B | | | 3182 B | 3182 B | 3182 B | 3283 B | 3283 B |
| `dirname doc/src/sgml/html/bug-reporting.html` | 18 B | | | 2521 B | 2521 B | 2521 B | 2430 B | 2430 B |
| `printf '%s\n' a b c` | 6 B | | | 3325 B | 3325 B | 3325 B | 3522 B | 3522 B |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 18 B | | | 3422 B | 3422 B | 3422 B | 3619 B | 3619 B |
| `test -f doc/src/sgml/html/bug-reporting.html && echo yes` | 4 B | | | 3607 B | 3607 B | 3607 B | 3892 B | 3892 B |
| `[ -d src/backend ] && echo folder` | 7 B | | | 3784 B | 3784 B | 3784 B | 4165 B | 4165 B |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 35 B | | | 3492 B | 3492 B | 3492 B | 3709 B | 3709 B |
| `echo a b c \| xargs -n 1 echo` | 6 B | | | 4674 B | 4674 B | 4674 B | 6367 B | 6367 B |
| `find src/include/access -name '*.html' \| xargs cat \| wc -l` | 2 B | | | 5091 B | 5091 B | 5091 B | 7604 B | 7604 B |
| `find . -name '*.html' \| xargs grep -l undesirable \| wc -l` | 3 B | | | 581.5 KB | 581.5 KB | 581.5 KB | 350.2 KB | 350.2 KB |
| `echo {a,b,c}{1..3}` | 27 B | | | 4485 B | 4485 B | 4485 B | 5917 B | 5917 B |
| `echo {1..20..3} {a..e}` | 28 B | | | 3967 B | 3967 B | 3967 B | 5303 B | 5303 B |
| `echo $(wc -l < doc/src/sgml/html/bug-reporting.html) `basename doc/src/sgml/html/bug-reporting.html`` | 23 B | | | 6537 B | 6537 B | 6537 B | 8123 B | 8123 B |
| `echo "first line: $(head -n 1 doc/src/sgml/html/bug-reporting.html)"` | 67 B | | | 5111 B | 5111 B | 5111 B | 6757 B | 6757 B |
| `wc -l < doc/src/sgml/html/bug-reporting.html` | 4 B | | | 3392 B | 3392 B | 3392 B | 3413 B | 3413 B |
| `cat doc/src/sgml/html/bug-reporting.html > /dev/null 2>&1` | 0 B | | | 3493 B | 3493 B | 3493 B | 3530 B | 3530 B |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 6 B | | | 4158 B | 4158 B | 4158 B | 4419 B | 4419 B |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2 B | | | 4072 B | 4072 B | 4072 B | 5445 B | 5445 B |
| `grep -q not doc/src/sgml/html/bug-reporting.html && echo found \|\| echo none` | 6 B | | | 11.8 KB | 11.8 KB | 11.8 KB | 12.5 KB | 12.5 KB |
| `false \|\| echo fallback` | 9 B | | | 2861 B | 2861 B | 2861 B | 2970 B | 2970 B |
| `true && grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | | | 10.6 KB | 10.6 KB | 10.6 KB | 11.0 KB | 11.0 KB |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 14 B | | | 4178 B | 4178 B | 4178 B | 5376 B | 5376 B |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2 B | | | 3130 B | 3130 B | 3130 B | 3079 B | 3079 B |
| `echo one \⏎two` | 8 B | | | 2591 B | 2591 B | 2591 B | 2596 B | 2596 B |
| `grep -c not doc/src/sgml/html/bug-reporting.html⏎wc -l doc/src/sgml/html/bug-reporting.html` | 44 B | | | 11.2 KB | 11.2 KB | 11.2 KB | 11.8 KB | 11.8 KB |
| `for i in $(seq 100000); do :; done` | 0 B | | | | | | 7815.6 KB | 7815.6 KB |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 6 B | | | | | | 7252 B | 7252 B |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 2 B | | | | | | 7249 B | 7249 B |
| `for f in doc/src/sgml/html/*.html; do echo "$f"; done \| wc -l` | 5 B | | | | | | 170.1 KB | 170.1 KB |
| `for f in doc/src/sgml/html/*.html; do grep -c not "$f"; done \| sort -n \| tail -n 3` | 12 B | | | | | | 173.4 KB | 173.4 KB |
| `n=0; for f in doc/src/sgml/html/*.html; do if grep -q passed "$f"; then n=$((n+1)); fi; done; echo $n` | 4 B | | | | | | 171.4 KB | 171.4 KB |
| `for f in doc/src/sgml/html/*.html; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 0 B | | | | | | 170.4 KB | 170.4 KB |
| `H=$PWD; for d in src/backend src/include/access doc/src/sgml/html; do cd "$d"; ls \| wc -l; cd "$H"; done` | 11 B | | | | | | 101.9 KB | 101.9 KB |
| `X=not; N=$(grep -c "$X" doc/src/sgml/html/bug-reporting.html); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 13 B | | | | | | 13.8 KB | 13.8 KB |
| `find src/include/access -name '*.html' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 0 B | | | | | | 77.9 KB | 77.9 KB |
| `for f in doc/src/sgml/html/*.html; do head -n 1 "$f"; done > /dev/null` | 0 B | | | | | | 169.4 KB | 169.4 KB |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 12 B | | | | | | 9721 B | 9721 B |
| `awk '{ print $2 }' doc/src/sgml/html/bug-reporting.html` | 14 B | | | | | | 6532 B | 6532 B |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 7 B | | | | | | 506.5 KB | 506.5 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '/not/ { n++ } END { print n }'` | 5 B | | | | | | 481.9 KB | 481.9 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 15 B | | | | | | 609.8 KB | 609.8 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 12 B | | | | | | 479.1 KB | 479.1 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 2 B | | | | | | 479.7 KB | 479.7 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 6 B | | | | | | 479.0 KB | 479.0 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 6 B | | | | | | 2791.3 KB | 2791.3 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "not") n++ } END { print n }'` | 5 B | | | | | | 610.6 KB | 610.6 KB |
| `awk '{ gsub(/[0-9]+/, "N"); print }' doc/src/sgml/html/bug-reporting.html \| wc -c` | 3 B | | | | | | 17.4 KB | 17.4 KB |
| `awk -v OFS=- '{ $1 = NR; print }' doc/src/sgml/html/bug-reporting.html \| tail -n 1` | 51 B | | | | | | 10.5 KB | 10.5 KB |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 16 B | | | | | | 5894 B | 5894 B |
| `grep -rn undesirable . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 231 B | | | | | | 81.4 KB | 81.4 KB |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 160 B | | | | | | 11.7 MB | 11.7 MB |
| `mkdir -p bench_out/a/b` | 0 B | 1827 B | 2115 B | 4079 B | 4079 B | | 4084 B | 4084 B |
| `touch bench_out/empty.txt` | 0 B | 979 B | 979 B | 2566 B | 2566 B | | 2475 B | 2475 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1242 B | 1292 B | 3665 B | 3661 B | | 3710 B | 3706 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1246 B | 1246 B | 3485 B | 3473 B | | 3530 B | 3518 B |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0 B | 24.1 KB | 283.4 KB | 3344 B | 3344 B | | 3349 B | 3349 B |
| `cp -r src/backend bench_out/dir_copy` | 0 B | 68.8 MB | 9479.9 KB | 665.9 KB | 1025.5 KB | | 666.0 KB | 1025.7 KB |
| `cp -r doc/src/sgml/html bench_out/work` | 0 B | 19.2 MB | 5277.4 KB | 456.1 KB | 579.1 KB | | 456.2 KB | 579.4 KB |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 39.7 KB | 56.2 KB | 59.5 KB | 301.3 KB | | 62.6 KB | 304.4 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 38.3 KB | 57.9 KB | 61.4 KB | 61.4 KB | | 63.9 KB | 63.9 KB |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 0 B | 10.3 MB | 2498.0 KB | 19.5 MB | 3604.7 KB | | 19.5 MB | 3607.8 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | 0 B | | 263.0 KB | 265.0 KB | 265.0 KB | | 265.9 KB | 265.9 KB |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | 0 B | | 265.5 KB | 267.6 KB | 267.6 KB | | 268.5 KB | 268.5 KB |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 B | | 20.9 KB | 2924 B | 2924 B | | 2929 B | 2929 B |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 B | | 20.9 KB | 2924 B | 2924 B | | 2929 B | 2929 B |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | 0 B | | 21.1 KB | 23.5 KB | 23.5 KB | | 23.5 KB | 23.5 KB |
| `grep -rc QDBBENCHMANY bench_out` | 110.1 KB | 33.3 KB | 33.1 KB | 35.4 KB | 35.4 KB | | 35.5 KB | 35.5 KB |
| `grep -rl QDBBENCHMANY .` | 36.1 KB | 97.6 KB | 105.6 KB | 107.8 KB | 107.8 KB | | 107.9 KB | 107.9 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | | 25.0 KB | 25.0 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| tee bench_out/tee.txt \| wc -l` | 4 B | | | 44.8 KB | 44.8 KB | | 47.1 KB | 47.1 KB |
| `sort doc/src/sgml/html/bug-reporting.html > bench_out/sorted.txt` | 0 B | | | 127.3 KB | 127.3 KB | | 127.3 KB | 127.3 KB |
| `sort -u doc/src/sgml/html/bug-reporting.html >> bench_out/sorted.txt` | 0 B | | | 126.7 KB | 126.7 KB | | 126.8 KB | 126.8 KB |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 B | | | 3786 B | 3795 B | | 3831 B | 3840 B |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 0 B | | | 3714 B | 3714 B | | 3759 B | 3759 B |
| `find src/include/access -name '*.html' \| sort \| xargs cat > bench_out/joined.txt` | 0 B | | | 70.9 KB | 70.9 KB | | 73.5 KB | 73.5 KB |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 B | | | 3790 B | 3790 B | | 3795 B | 3795 B |
| `sed -i '1d' bench_out/joined2.txt` | 0 B | | | 3550 B | 3550 B | | 3651 B | 3651 B |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 118 B | | | 4165 B | 4165 B | | 4458 B | 4458 B |
| `cat bench_out/both.txt` | 98 B | | | 2497 B | 2497 B | | 2406 B | 2406 B |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | 0 B | | | | | | 3173 B | 3173 B |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 0 B | | | | | | 60.9 KB | 60.9 KB |
| `diff -u --label old --label new doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | 1807 B | | | | | | 25.9 KB | 25.9 KB |
| `diff doc/src/sgml/html/bug-reporting.html bench_out/v2.txt \| wc -l` | 3 B | | | | | | 25.3 KB | 25.3 KB |
| `diff -q doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 B | | | | | | 3381 B | 3381 B |
| `diff -u doc/src/sgml/html/bug-reporting.html bench_out/copy.txt \| wc -l` | 4 B | | | | | | 78.0 KB | 78.0 KB |
| `diff -rq src/backend bench_out/dir_copy` | 0 B | | | | | | 15.6 KB | 15.6 KB |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 0 B | | | | | | 275.4 KB | 275.4 KB |
| `diff -rq src/backend bench_out/dir_copy` | 70 B | | | | | | 15.6 KB | 15.6 KB |
| `diff -r doc/src/sgml/html bench_out/work \| wc -l` | 6 B | | | | | | 1418.3 KB | 1418.3 KB |
| `diff -ru doc/src/sgml/html bench_out/work \| grep -c '^@@'` | 5 B | | | | | | 1429.2 KB | 1429.2 KB |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 0 B | | | | | | 1294.3 KB | 1666.6 KB |
| `wc -l bench_out/loop.txt` | 25 B | | | | | | 2829 B | 2829 B |
| `for f in bench_out/work/*.html; do echo done >> "$f"; done` | 0 B | | | | | | 21.7 MB | 2864.0 KB |
| `awk '{ print NR ": " $0 }' doc/src/sgml/html/bug-reporting.html > bench_out/numbered.txt` | 0 B | | | | | | 8114 B | 8114 B |
| `awk 'END { print NR }' bench_out/numbered.txt` | 2 B | | | | | | 5549 B | 5549 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1191 B | 1191 B | 2900 B | 2900 B | | 2905 B | 2905 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1288 B | 1288 B | 3250 B | 3250 B | | 3255 B | 3255 B |
| `ls -R bench_out/a` | 41 B | 1089 B | 1089 B | 3092 B | 3092 B | | 3097 B | 3097 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 997 B | 997 B | 2504 B | 2504 B | | 2413 B | 2413 B |
| `rmdir bench_out/a/b` | 0 B | 945 B | 945 B | 2383 B | 2383 B | | 2292 B | 2292 B |
| `rm -r bench_out/dir_moved` | 0 B | 1129 B | 1129 B | 2707 B | 2707 B | | 2863 B | 2863 B |
| `rm -rf bench_out` | 0 B | 9354 B | 708.3 KB | 11.1 KB | 11.1 KB | | 10.2 KB | 10.2 KB |

## What each command used (Stage 5, on disk, settled)

Counted by the engine for the first run of the command; the two times are that run's, measured. Bytes unless said otherwise.

| Command | Text read | Pieces | Index read | Index lookups | Tree entries | Content written | Records written | Upkeep written | Syncs | Scratch | Memory | CPU, ms | Elapsed, ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 0 | 0 | 0 | 0 | 25 | 0 | 421 | 0 | 1 | 0 | 2348 | 0.515 | 1.891 |
| `ls -a` | 0 | 0 | 0 | 0 | 25 | 0 | 0 | 0 | 0 | 0 | 3526 | 0.020 | 0.020 |
| `ls -R` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 93126 | 0.834 | 0.835 |
| `ls -aR src/backend` | 0 | 0 | 0 | 0 | 1456 | 0 | 0 | 0 | 0 | 0 | 32820 | 0.214 | 0.243 |
| `ls -p src/backend` | 0 | 0 | 0 | 0 | 35 | 0 | 0 | 0 | 0 | 0 | 4051 | 0.020 | 0.019 |
| `find .` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 29994 | 1.403 | 1.403 |
| `find . -name '*.html'` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 20165 | 1.451 | 1.451 |
| `find . -name '*.c'` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 34943 | 1.465 | 1.465 |
| `find . -type d` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 19477 | 1.312 | 1.312 |
| `find . -type f` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 28729 | 1.320 | 1.320 |
| `find . -maxdepth 2 -type f` | 0 | 0 | 0 | 0 | 133 | 0 | 0 | 0 | 0 | 0 | 5450 | 0.046 | 0.046 |
| `find src/backend -name '*.html' -type f` | 0 | 0 | 0 | 0 | 1456 | 0 | 0 | 0 | 0 | 0 | 4231 | 0.226 | 0.225 |
| `find . -name 'Makefile'` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 13513 | 1.340 | 1.339 |
| `cat doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 2292 | 0.058 | 0.058 |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 34978 | 0.083 | 0.083 |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 6008 | 0.029 | 0.029 |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 59970 | 3 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 4903 | 0.021 | 0.021 |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 39980 | 2 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 14398 | 0.052 | 0.051 |
| `wc doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 2311 | 0.075 | 0.075 |
| `wc -l doc/src/sgml/html/*.html` | 0 | 0 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 270024 | 1.249 | 1.248 |
| `cat doc/src/sgml/html/*.html \| wc -l` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 270673 | 4.765 | 4.766 |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 17654 | 0.037 | 0.037 |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 10070 | 0.075 | 0.074 |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 10070 | 0.057 | 0.057 |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 27483 | 0.132 | 0.132 |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 33313 | 0.327 | 0.327 |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 43353 | 0.115 | 0.115 |
| `grep -rn qdbzzabsentterm .` | 58579 | 13 | 5224 | 39 | 25 | 0 | 0 | 0 | 0 | 0 | 31432 | 0.193 | 0.193 |
| `grep -rl qdbzzabsentterm .` | 58579 | 13 | 5224 | 39 | 25 | 0 | 0 | 0 | 0 | 0 | 31432 | 0.124 | 0.124 |
| `grep -rn abcdefghijklabcdefgh .` | 185750 | 20 | 11112 | 36 | 44 | 0 | 0 | 0 | 0 | 0 | 40807 | 0.240 | 0.240 |
| `grep -rl abcdefghijklabcdefgh .` | 185750 | 20 | 11112 | 36 | 44 | 0 | 0 | 0 | 0 | 0 | 40807 | 0.205 | 0.205 |
| `grep -rn btvarstrequalimage .` | 980580 | 49 | 53543 | 48 | 44 | 0 | 0 | 0 | 0 | 0 | 37473 | 0.507 | 0.507 |
| `grep -rl btvarstrequalimage .` | 980580 | 49 | 53543 | 48 | 44 | 0 | 0 | 0 | 0 | 0 | 37473 | 0.444 | 0.444 |
| `grep -rn undesirable .` | 7928648 | 216 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 31140 | 2.618 | 5.231 |
| `grep -rl undesirable .` | 7437550 | 208 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 31140 | 1.255 | 1.254 |
| `grep -rn passed .` | 74419127 | 1956 | 15683 | 12 | 9030 | 0 | 0 | 0 | 0 | 0 | 28016 | 16.43 | 16.43 |
| `grep -rl passed .` | 54587193 | 1598 | 15683 | 12 | 9030 | 0 | 0 | 0 | 0 | 0 | 27668 | 7.111 | 7.111 |
| `grep -rn not .` | 161175531 | 9604 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 33167 | 55.28 | 59.04 |
| `grep -rl not .` | 102937209 | 8473 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 9938 | 5.974 | 5.974 |
| `grep -rc undesirable .` | 7928648 | 216 | 33100 | 27 | 9030 | 0 | 0 | 0 | 0 | 0 | 46953 | 2.620 | 2.620 |
| `grep -rin BTVARSTREQUALIMAGE .` | 980580 | 49 | 53543 | 48 | 44 | 0 | 0 | 0 | 0 | 0 | 37474 | 0.411 | 0.411 |
| `grep -ri undesirable .` | 7928648 | 216 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 31129 | 3.063 | 3.063 |
| `grep -rn undesirable src/backend` | 3814015 | 101 | 33100 | 27 | 83 | 0 | 0 | 0 | 0 | 0 | 24799 | 0.931 | 0.931 |
| `grep -rl passed src/backend` | 21522827 | 535 | 15683 | 12 | 1458 | 0 | 0 | 0 | 0 | 0 | 27746 | 2.399 | 2.400 |
| `grep -rF 'undesirable' .` | 7928648 | 216 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 31137 | 1.548 | 1.549 |
| `grep -rE 'size_[a-z]+\(' .` | 31331368 | 959 | 8413 | 9 | 44 | 0 | 0 | 0 | 0 | 0 | 126627 | 7.954 | 8.023 |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1704860 | 70 | 64655 | 84 | 118 | 0 | 0 | 0 | 0 | 0 | 74140 | 0.629 | 0.629 |
| `grep -rE 'passed\|undesirable' .` | 76056758 | 2024 | 48783 | 39 | 9030 | 0 | 0 | 0 | 0 | 0 | 36382 | 16.39 | 16.39 |
| `grep -rE '[0-9]{4,}' .` | 161175531 | 9604 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 25224 | 226.1 | 226.1 |
| `grep -rn '^#include' .` | 40208201 | 2367 | 26906 | 18 | 9030 | 0 | 0 | 0 | 0 | 0 | 57355 | 9.426 | 9.426 |
| `grep -rE '^(import\|from) ' .` | 161175531 | 9604 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 30062 | 46.85 | 46.85 |
| `grep -rin 'return [a-z_]*;$' src/backend` | 44882188 | 1229 | 23527 | 15 | 1458 | 0 | 0 | 0 | 0 | 0 | 69016 | 15.11 | 15.12 |
| `grep -rv e src/include/access` | 598315 | 89 | 0 | 0 | 94 | 0 | 0 | 0 | 0 | 0 | 29557 | 0.865 | 0.865 |
| `grep -r '\(undesirable\).*\1' .` | 7928648 | 216 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 35000 | 2.077 | 2.078 |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 869297 | 35 | 53543 | 48 | 7022 | 0 | 0 | 0 | 0 | 0 | 270756 | 1.104 | 1.104 |
| `grep -l undesirable doc/src/sgml/html/*.html` | 1703472 | 65 | 33100 | 27 | 7022 | 0 | 0 | 0 | 0 | 0 | 270705 | 1.124 | 1.124 |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 76545 | 4 | 5224 | 39 | 7022 | 0 | 0 | 0 | 0 | 0 | 270725 | 0.874 | 0.874 |
| `grep -rl passed . \| wc -l` | 54587193 | 1598 | 15683 | 12 | 9030 | 0 | 0 | 0 | 0 | 0 | 30097 | 9.928 | 9.929 |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 161175531 | 9604 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 16508 | 33.29 | 33.30 |
| `find . -name '*.html' \| wc -l` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 22594 | 1.460 | 1.460 |
| `find . -type f \| grep -c html` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 41307 | 2.866 | 2.866 |
| `grep -rn not . \| head -n 5` | 36716 | 3 | 0 | 0 | 25 | 0 | 0 | 0 | 0 | 0 | 12699 | 0.079 | 0.079 |
| `grep -rn passed . \| head -n 1` | 123119 | 14 | 0 | 0 | 25 | 0 | 0 | 0 | 0 | 0 | 18090 | 0.113 | 0.112 |
| `find . -type f \| head -n 3` | 0 | 0 | 0 | 0 | 582 | 0 | 0 | 0 | 0 | 0 | 31373 | 0.183 | 0.183 |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 21557 | 1 | 0 | 0 | 1187 | 0 | 0 | 0 | 0 | 0 | 271029 | 0.498 | 0.498 |
| `grep -rno undesirable .` | 7928648 | 216 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 38821 | 2.304 | 2.305 |
| `grep -rw undesirable .` | 7928648 | 216 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 42132 | 1.739 | 1.864 |
| `grep -rnw btvarstrequalimage .` | 980580 | 49 | 53543 | 48 | 44 | 0 | 0 | 0 | 0 | 0 | 44374 | 0.744 | 0.743 |
| `grep -rn -C 2 btvarstrequalimage .` | 3081158 | 84 | 53543 | 48 | 44 | 0 | 0 | 0 | 0 | 0 | 38381 | 1.079 | 1.079 |
| `grep -rn -A 1 -B 1 undesirable .` | 17727354 | 395 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 32764 | 4.600 | 4.600 |
| `grep -rho 'size_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 31331368 | 959 | 8413 | 9 | 44 | 0 | 0 | 0 | 0 | 0 | 234638 | 10.06 | 10.06 |
| `grep -rhow '[a-z]*btvarstrequalimage[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 980580 | 49 | 53543 | 48 | 44 | 0 | 0 | 0 | 0 | 0 | 78947 | 0.873 | 0.873 |
| `rg -j 1 qdbzzabsentterm` | 674 | 1 | 5224 | 39 | 29 | 0 | 0 | 0 | 0 | 0 | 30677 | 0.230 | 0.229 |
| `rg -j 1 abcdefghijklabcdefgh` | 38851 | 4 | 11112 | 36 | 20 | 0 | 0 | 0 | 0 | 0 | 39695 | 0.313 | 0.313 |
| `rg -j 1 btvarstrequalimage` | 650274 | 23 | 53543 | 48 | 80 | 0 | 0 | 0 | 0 | 0 | 58065 | 0.531 | 0.531 |
| `rg -j 1 undesirable` | 7610004 | 209 | 33100 | 27 | 296 | 0 | 0 | 0 | 0 | 0 | 109687 | 3.398 | 3.472 |
| `rg -j 1 passed` | 74308154 | 2102 | 15683 | 12 | 11462 | 0 | 0 | 0 | 0 | 0 | 436577 | 24.41 | 24.41 |
| `rg -j 1 not` | 161123389 | 9595 | 0 | 0 | 11462 | 0 | 0 | 0 | 0 | 0 | 426356 | 53.74 | 53.74 |
| `rg -j 1 -l undesirable` | 7118906 | 201 | 33100 | 27 | 296 | 0 | 0 | 0 | 0 | 0 | 100385 | 2.443 | 2.444 |
| `rg -j 1 -l not` | 102885067 | 8464 | 0 | 0 | 11462 | 0 | 0 | 0 | 0 | 0 | 425374 | 14.33 | 14.40 |
| `rg -j 1 -c passed` | 74308154 | 2102 | 15683 | 12 | 11462 | 0 | 0 | 0 | 0 | 0 | 433035 | 20.28 | 20.29 |
| `rg -j 1 -i BTVARSTREQUALIMAGE` | 650274 | 23 | 53543 | 60 | 80 | 0 | 0 | 0 | 0 | 0 | 89951 | 1.576 | 1.902 |
| `rg -j 1 -w undesirable` | 7610004 | 209 | 33100 | 27 | 296 | 0 | 0 | 0 | 0 | 0 | 131773 | 26.88 | 26.88 |
| `rg -j 1 -F 'undesirable'` | 7610004 | 209 | 33100 | 27 | 296 | 0 | 0 | 0 | 0 | 0 | 110238 | 2.791 | 2.792 |
| `rg -j 1 -n -C 2 btvarstrequalimage` | 2750852 | 58 | 53543 | 48 | 80 | 0 | 0 | 0 | 0 | 0 | 68295 | 1.142 | 1.143 |
| `rg -j 1 'size_[a-z]+\('` | 31275587 | 1028 | 8413 | 9 | 704 | 0 | 0 | 0 | 0 | 0 | 262037 | 9.351 | 9.361 |
| `rg -j 1 'btvarstrequalimage\|abcdefghijklabcdefgh'` | 688193 | 24 | 64655 | 84 | 80 | 0 | 0 | 0 | 0 | 0 | 72561 | 0.583 | 0.582 |
| `rg -j 1 '\bbtvarstrequalimage\w*'` | 650274 | 23 | 53543 | 48 | 80 | 0 | 0 | 0 | 0 | 0 | 2620565 | 4.609 | 4.608 |
| `rg -j 1 -t c undesirable` | 3354838 | 102 | 33100 | 27 | 296 | 0 | 0 | 0 | 0 | 0 | 105483 | 1.374 | 1.374 |
| `rg -j 1 -g '*.html' undesirable` | 1286004 | 69 | 33100 | 27 | 296 | 0 | 0 | 0 | 0 | 0 | 102199 | 1.065 | 1.065 |
| `rg -j 1 undesirable src/backend` | 3338041 | 80 | 33100 | 27 | 160 | 0 | 0 | 0 | 0 | 0 | 54850 | 1.085 | 1.084 |
| `rg -j 1 -uu undesirable` | 7607461 | 181 | 33100 | 27 | 0 | 0 | 0 | 0 | 0 | 0 | 38535 | 1.693 | 1.693 |
| `rg -j 1 --sort path btvarstrequalimage` | 650274 | 23 | 53543 | 48 | 80 | 0 | 0 | 0 | 0 | 0 | 58987 | 0.483 | 0.483 |
| `rg -j 1 --files \| wc -l` | 11749 | 167 | 0 | 0 | 11462 | 0 | 0 | 0 | 0 | 0 | 425055 | 8.723 | 8.723 |
| `rg -j 1 -l passed \| wc -l` | 54476220 | 1744 | 15683 | 12 | 11462 | 0 | 0 | 0 | 0 | 0 | 447752 | 15.81 | 15.81 |
| `ls -ltR` | 0 | 0 | 0 | 0 | 27065 | 0 | 0 | 0 | 0 | 0 | 144208 | 5.722 | 5.722 |
| `ls -lt doc/src/sgml/html` | 0 | 0 | 0 | 0 | 2350 | 0 | 0 | 0 | 0 | 0 | 105616 | 0.735 | 0.734 |
| `ls -ltr src/backend \| tail -n 3` | 0 | 0 | 0 | 0 | 1214 | 0 | 0 | 0 | 0 | 0 | 8389 | 0.094 | 0.094 |
| `find . -newer doc/src/sgml/html/bug-reporting.html` | 0 | 0 | 0 | 0 | 9035 | 0 | 0 | 0 | 0 | 0 | 19812 | 1.535 | 1.587 |
| `find . -newer doc/src/sgml/html/bug-reporting.html -name '*.html'` | 0 | 0 | 0 | 0 | 9035 | 0 | 0 | 0 | 0 | 0 | 23999 | 2.427 | 2.428 |
| `find . -type f -mtime +30` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 4114 | 2.319 | 2.319 |
| `du -s .` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 40857 | 0.468 | 0.468 |
| `du -sh src/backend` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 2495 | 0.021 | 0.021 |
| `du -sb .` | 0 | 0 | 0 | 0 | 9030 | 0 | 0 | 0 | 0 | 0 | 53642 | 0.469 | 0.469 |
| `du -sb src/backend` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 2513 | 0.018 | 0.018 |
| `du -b -d 1 src/backend` | 0 | 0 | 0 | 0 | 35 | 0 | 0 | 0 | 0 | 0 | 4975 | 0.162 | 0.182 |
| `sort doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 105981 | 0.108 | 0.108 |
| `sort -r doc/src/sgml/html/bug-reporting.html \| head -n 3` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 110921 | 0.095 | 0.095 |
| `sort -u doc/src/sgml/html/bug-reporting.html \| wc -l` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 108772 | 0.090 | 0.090 |
| `sort -k 2 doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 129481 | 0.104 | 0.104 |
| `cat doc/src/sgml/html/*.html \| sort \| uniq -c \| sort -rn \| head -n 5` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 57454614 | 135.0 | 135.2 |
| `uniq -c doc/src/sgml/html/bug-reporting.html \| wc -l` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 40712 | 0.073 | 0.072 |
| `cut -c 1-20 doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 18006 | 0.081 | 0.081 |
| `cut -d ' ' -f 1 doc/src/sgml/html/bug-reporting.html \| sort \| uniq \| wc -l` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 81575 | 0.109 | 0.108 |
| `tr a-z A-Z < doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 24881 | 0.055 | 0.055 |
| `tr -d '\n' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 27304 | 0.050 | 0.050 |
| `tr -s ' ' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 27302 | 0.050 | 0.050 |
| `nl doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 35977 | 0.095 | 0.095 |
| `nl -ba doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 60126 | 0.086 | 0.086 |
| `seq 1 100000 \| wc -l` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 136176 | 3.272 | 3.296 |
| `seq -w 1 1000 \| tail -n 2` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 18979 | 0.064 | 0.064 |
| `basename doc/src/sgml/html/bug-reporting.html` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2173 | 0.012 | 0.012 |
| `basename -s .html doc/src/sgml/html/bug-reporting.html` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3023 | 0.014 | 0.014 |
| `dirname doc/src/sgml/html/bug-reporting.html` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2170 | 0.013 | 0.012 |
| `printf '%s\n' a b c` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3262 | 0.016 | 0.015 |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3359 | 0.017 | 0.017 |
| `test -f doc/src/sgml/html/bug-reporting.html && echo yes` | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 3632 | 0.025 | 0.024 |
| `[ -d src/backend ] && echo folder` | 0 | 0 | 0 | 0 | 27 | 0 | 0 | 0 | 0 | 0 | 3905 | 0.023 | 0.023 |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3449 | 0.029 | 0.029 |
| `echo a b c \| xargs -n 1 echo` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 6107 | 0.030 | 0.030 |
| `find src/include/access -name '*.html' \| xargs cat \| wc -l` | 0 | 0 | 0 | 0 | 91 | 0 | 0 | 0 | 0 | 0 | 7344 | 0.069 | 0.068 |
| `find . -name '*.html' \| xargs grep -l undesirable \| wc -l` | 1703472 | 65 | 33100 | 27 | 14875 | 0 | 0 | 0 | 0 | 0 | 349916 | 3.081 | 3.084 |
| `echo {a,b,c}{1..3}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5657 | 0.021 | 0.021 |
| `echo {1..20..3} {a..e}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5043 | 0.021 | 0.021 |
| `echo $(wc -l < doc/src/sgml/html/bug-reporting.html) `basename doc/src/sgml/html/bug-reporting.html`` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 7863 | 0.047 | 0.047 |
| `echo "first line: $(head -n 1 doc/src/sgml/html/bug-reporting.html)"` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 6497 | 0.035 | 0.035 |
| `wc -l < doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 3153 | 0.031 | 0.031 |
| `cat doc/src/sgml/html/bug-reporting.html > /dev/null 2>&1` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 3270 | 0.021 | 0.020 |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 4159 | 0.023 | 0.022 |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 5185 | 0.025 | 0.025 |
| `grep -q not doc/src/sgml/html/bug-reporting.html && echo found \|\| echo none` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 12263 | 0.060 | 0.060 |
| `false \|\| echo fallback` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2710 | 0.013 | 0.013 |
| `true && grep -c not doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 10722 | 0.058 | 0.057 |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5116 | 0.024 | 0.024 |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2819 | 0.016 | 0.016 |
| `echo one \⏎two` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2336 | 0.009 | 0.009 |
| `grep -c not doc/src/sgml/html/bug-reporting.html⏎wc -l doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 15 | 0 | 0 | 0 | 0 | 0 | 11534 | 0.075 | 0.075 |
| `for i in $(seq 100000); do :; done` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 7815304 | 40.21 | 40.21 |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 0 | 0 | 0 | 0 | 250025 | 0 | 0 | 0 | 0 | 0 | 6992 | 24.44 | 24.44 |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 0 | 0 | 0 | 0 | 275 | 0 | 0 | 0 | 0 | 0 | 6989 | 0.047 | 0.047 |
| `for f in doc/src/sgml/html/*.html; do echo "$f"; done \| wc -l` | 0 | 0 | 0 | 0 | 1177 | 0 | 0 | 0 | 0 | 0 | 169888 | 1.333 | 1.333 |
| `for f in doc/src/sgml/html/*.html; do grep -c not "$f"; done \| sort -n \| tail -n 3` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 173187 | 9.672 | 9.700 |
| `n=0; for f in doc/src/sgml/html/*.html; do if grep -q passed "$f"; then n=$((n+1)); fi; done; echo $n` | 15829851 | 1187 | 15683 | 12 | 12867 | 0 | 0 | 0 | 0 | 0 | 171181 | 12.36 | 12.48 |
| `for f in doc/src/sgml/html/*.html; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 0 | 0 | 0 | 0 | 36247 | 0 | 0 | 0 | 0 | 0 | 170165 | 4.619 | 4.620 |
| `H=$PWD; for d in src/backend src/include/access doc/src/sgml/html; do cd "$d"; ls \| wc -l; cd "$H"; done` | 0 | 0 | 0 | 0 | 1303 | 0 | 0 | 0 | 0 | 0 | 101717 | 0.148 | 0.147 |
| `X=not; N=$(grep -c "$X" doc/src/sgml/html/bug-reporting.html); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 19990 | 1 | 0 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 13531 | 0.095 | 0.095 |
| `find src/include/access -name '*.html' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 0 | 0 | 0 | 0 | 91 | 0 | 0 | 0 | 0 | 0 | 77647 | 0.105 | 0.105 |
| `for f in doc/src/sgml/html/*.html; do head -n 1 "$f"; done > /dev/null` | 15036191 | 1169 | 0 | 0 | 7022 | 0 | 0 | 0 | 0 | 0 | 169137 | 2.323 | 2.323 |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 0 | 0 | 0 | 0 | 125 | 0 | 0 | 0 | 0 | 0 | 9461 | 0.049 | 0.049 |
| `awk '{ print $2 }' doc/src/sgml/html/bug-reporting.html` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 6272 | 0.068 | 0.067 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 506262 | 38.68 | 38.69 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '/not/ { n++ } END { print n }'` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 481685 | 19.10 | 19.10 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 609536 | 46.78 | 46.81 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 478884 | 19.46 | 19.46 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 479400 | 17.65 | 17.68 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 478714 | 17.57 | 17.57 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 2791022 | 41.01 | 41.03 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "not") n++ } END { print n }'` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 610375 | 159.6 | 159.9 |
| `awk '{ gsub(/[0-9]+/, "N"); print }' doc/src/sgml/html/bug-reporting.html \| wc -c` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 17144 | 0.143 | 0.143 |
| `awk -v OFS=- '{ $1 = NR; print }' doc/src/sgml/html/bug-reporting.html \| tail -n 1` | 19990 | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 10215 | 0.056 | 0.056 |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5634 | 5.836 | 5.891 |
| `grep -rn undesirable . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 1596256 | 69 | 33100 | 27 | 44 | 0 | 0 | 0 | 0 | 0 | 81177 | 0.666 | 0.666 |
| `cat doc/src/sgml/html/*.html \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 16611227 | 1208 | 0 | 0 | 12867 | 0 | 0 | 0 | 0 | 0 | 11709830 | 110.2 | 110.7 |
| `mkdir -p bench_out/a/b` | 0 | 0 | 0 | 0 | 3 | 0 | 103 | 0 | 1 | 0 | 3824 | 0.128 | 0.774 |
| `touch bench_out/empty.txt` | 0 | 0 | 0 | 0 | 3 | 0 | 67 | 0 | 1 | 0 | 2215 | 0.100 | 0.471 |
| `echo 'hello world' > bench_out/hello.txt` | 0 | 0 | 0 | 0 | 3 | 52 | 90 | 0 | 1 | 0 | 3446 | 0.113 | 0.460 |
| `echo 'another line' >> bench_out/hello.txt` | 24 | 2 | 0 | 0 | 2 | 65 | 63 | 0 | 1 | 0 | 3258 | 0.147 | 0.645 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0 | 0 | 0 | 0 | 10 | 0 | 94 | 0 | 1 | 0 | 3089 | 0.084 | 0.439 |
| `cp -r src/backend bench_out/dir_copy` | 0 | 0 | 0 | 0 | 20560 | 0 | 93164 | 0 | 1 | 0 | 1025482 | 2.983 | 3.639 |
| `cp -r doc/src/sgml/html bench_out/work` | 0 | 0 | 0 | 0 | 12912 | 0 | 77205 | 0 | 1 | 0 | 579116 | 2.362 | 3.215 |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 19990 | 1 | 0 | 0 | 2 | 20406 | 80 | 0 | 1 | 0 | 304111 | 0.850 | 1.304 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 20366 | 1 | 0 | 0 | 2 | 20379 | 80 | 0 | 1 | 0 | 63606 | 0.661 | 1.141 |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 16611227 | 1208 | 0 | 0 | 4682 | 15278348 | 39316 | 0 | 1 | 0 | 3607582 | 203.8 | 247.1 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | 76770 | 4 | 5224 | 39 | 4682 | 0 | 14068 | 0 | 1 | 0 | 265619 | 1.178 | 16.58 |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | 997004 | 49 | 61699 | 48 | 4682 | 22112 | 14096 | 0 | 1 | 0 | 268191 | 3.057 | 19.61 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 | 0 | 0 | 0 | 10 | 0 | 94 | 0 | 1 | 0 | 2941 | 0.113 | 0.448 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 | 0 | 0 | 0 | 9 | 0 | 40 | 0 | 1 | 0 | 2669 | 0.138 | 0.495 |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | 39980 | 2 | 0 | 0 | 12 | 0 | 40 | 0 | 1 | 0 | 23248 | 0.143 | 0.528 |
| `grep -rc QDBBENCHMANY bench_out` | 15630654 | 983 | 26296 | 30 | 2637 | 0 | 0 | 0 | 0 | 0 | 35198 | 5.260 | 5.260 |
| `grep -rl QDBBENCHMANY .` | 14229152 | 950 | 26296 | 30 | 95 | 0 | 0 | 0 | 0 | 0 | 107683 | 1.420 | 1.420 |
| `grep -rl QDBBENCHONE .` | 1019195 | 48 | 22116 | 27 | 95 | 0 | 0 | 0 | 0 | 0 | 24764 | 0.341 | 0.340 |
| `cat doc/src/sgml/html/bug-reporting.html \| tee bench_out/tee.txt \| wc -l` | 19990 | 1 | 0 | 0 | 13 | 20030 | 93 | 0 | 1 | 0 | 46830 | 0.484 | 0.983 |
| `sort doc/src/sgml/html/bug-reporting.html > bench_out/sorted.txt` | 19990 | 1 | 0 | 0 | 13 | 20031 | 96 | 0 | 1 | 0 | 127020 | 0.460 | 0.920 |
| `sort -u doc/src/sgml/html/bug-reporting.html >> bench_out/sorted.txt` | 59972 | 3 | 0 | 0 | 12 | 39415 | 68 | 0 | 1 | 0 | 126555 | 0.587 | 1.101 |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 | 0 | 0 | 0 | 5 | 89 | 89 | 0 | 1 | 0 | 3580 | 0.110 | 0.526 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 98 | 2 | 0 | 0 | 4 | 138 | 63 | 0 | 1 | 0 | 3499 | 0.097 | 0.382 |
| `find src/include/access -name '*.html' \| sort \| xargs cat > bench_out/joined.txt` | 0 | 0 | 0 | 0 | 94 | 0 | 68 | 0 | 1 | 0 | 73205 | 0.135 | 0.495 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 | 0 | 0 | 0 | 7 | 0 | 69 | 0 | 1 | 0 | 3535 | 0.089 | 0.413 |
| `sed -i '1d' bench_out/joined2.txt` | 0 | 0 | 0 | 0 | 2 | 0 | 52 | 0 | 1 | 0 | 3391 | 0.083 | 0.367 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 0 | 0 | 0 | 0 | 16 | 0 | 0 | 0 | 0 | 0 | 4198 | 0.025 | 0.024 |
| `cat bench_out/both.txt` | 98 | 1 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 2146 | 0.014 | 0.014 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | 0 | 0 | 0 | 0 | 10 | 0 | 92 | 0 | 1 | 0 | 2913 | 0.096 | 0.500 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 19990 | 1 | 0 | 0 | 2 | 19655 | 80 | 0 | 1 | 0 | 60602 | 0.447 | 0.933 |
| `diff -u --label old --label new doc/src/sgml/html/bug-reporting.html bench_out/v2.txt` | 238380 | 12 | 0 | 0 | 7 | 0 | 0 | 0 | 0 | 0 | 25600 | 0.103 | 0.102 |
| `diff doc/src/sgml/html/bug-reporting.html bench_out/v2.txt \| wc -l` | 178785 | 9 | 0 | 0 | 7 | 0 | 0 | 0 | 0 | 0 | 25023 | 0.086 | 0.086 |
| `diff -q doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 | 0 | 0 | 0 | 7 | 0 | 0 | 0 | 0 | 0 | 3121 | 0.020 | 0.020 |
| `diff -u doc/src/sgml/html/bug-reporting.html bench_out/copy.txt \| wc -l` | 221286 | 11 | 0 | 0 | 7 | 0 | 0 | 0 | 0 | 0 | 77699 | 0.173 | 0.173 |
| `diff -rq src/backend bench_out/dir_copy` | 0 | 0 | 0 | 0 | 2912 | 0 | 0 | 0 | 0 | 0 | 15263 | 0.607 | 0.609 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 38 | 1 | 0 | 0 | 1459 | 87 | 75 | 0 | 1 | 0 | 275140 | 0.661 | 1.011 |
| `diff -rq src/backend bench_out/dir_copy` | 85 | 2 | 0 | 0 | 2912 | 0 | 0 | 0 | 0 | 0 | 15333 | 0.593 | 0.593 |
| `diff -r doc/src/sgml/html bench_out/work \| wc -l` | 123895635 | 8380 | 0 | 0 | 2352 | 0 | 0 | 0 | 0 | 0 | 1418075 | 50.76 | 50.81 |
| `diff -ru doc/src/sgml/html bench_out/work \| grep -c '^@@'` | 162232585 | 11124 | 0 | 0 | 2352 | 0 | 0 | 0 | 0 | 0 | 1428949 | 63.66 | 63.97 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 1580743 | 19993 | 0 | 0 | 20001 | 1239829 | 241096 | 0 | 1 | 0 | 1666342 | 38.79 | 42.62 |
| `wc -l bench_out/loop.txt` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 2569 | 0.018 | 0.018 |
| `for f in bench_out/work/*.html; do echo done >> "$f"; done` | 28583773 | 2363 | 0 | 0 | 4682 | 14350030 | 44043 | 0 | 1 | 0 | 2863697 | 128.7 | 155.1 |
| `awk '{ print NR ": " $0 }' doc/src/sgml/html/bug-reporting.html > bench_out/numbered.txt` | 19990 | 1 | 0 | 0 | 8 | 98 | 93 | 0 | 1 | 0 | 7854 | 0.163 | 0.710 |
| `awk 'END { print NR }' bench_out/numbered.txt` | 58 | 1 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 5289 | 0.038 | 0.037 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 | 0 | 0 | 0 | 8 | 0 | 55 | 0 | 1 | 0 | 2645 | 0.125 | 0.601 |
| `mv bench_out/hello.txt bench_out/a/b` | 0 | 0 | 0 | 0 | 13 | 0 | 67 | 0 | 1 | 0 | 2995 | 0.083 | 0.359 |
| `ls -R bench_out/a` | 0 | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 2837 | 0.017 | 0.017 |
| `rm bench_out/a/b/hello.txt` | 0 | 0 | 0 | 0 | 4 | 0 | 43 | 0 | 1 | 0 | 2153 | 0.096 | 0.626 |
| `rmdir bench_out/a/b` | 0 | 0 | 0 | 0 | 3 | 0 | 43 | 0 | 1 | 0 | 2032 | 0.069 | 0.382 |
| `rm -r bench_out/dir_moved` | 0 | 0 | 0 | 0 | 2 | 0 | 4405 | 0 | 1 | 0 | 2603 | 0.854 | 1.263 |
| `rm -rf bench_out` | 0 | 0 | 0 | 0 | 1 | 0 | 3600 | 0 | 1 | 0 | 9985 | 19.23 | 19.62 |

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
