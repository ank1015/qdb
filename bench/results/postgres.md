# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 4, on disk, settled), and include starting bash and the tool (2.043 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
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

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.421 |  | 0.001 | 0.001 | 0.001 | 0.002 | 0.003 | yes |
| `ls -a` | 2.745 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.004 | yes |
| `ls -R` | 9.726 |  | 0.563 | 0.288 | 0.440 | 0.571 | 0.406 | yes |
| `ls -aR src/backend` | 3.599 |  | 0.059 | 0.055 | 0.058 | 0.069 | 0.071 | yes |
| `ls -p src/backend` | 2.512 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | yes |
| `find .` | 11.41 |  | 1.298 | 0.767 | 0.922 | 0.732 | 0.979 | yes |
| `find . -name '*.html'` | 10.83 |  | 1.444 | 0.953 | 1.051 | 1.539 | 1.869 | yes |
| `find . -name '*.c'` | 10.23 |  | 1.706 | 1.780 | 1.301 | 1.179 | 1.711 | yes |
| `find . -type d` | 11.43 |  | 1.403 | 0.718 | 0.898 | 0.852 | 1.184 | yes |
| `find . -type f` | 11.38 |  | 1.397 | 1.182 | 0.999 | 0.834 | 0.840 | yes |
| `find . -maxdepth 2 -type f` | 3.124 |  | 0.015 | 0.015 | 0.009 | 0.011 | 0.013 | yes |
| `find src/backend -name '*.html' -type f` | 4.623 |  | 0.177 | 0.189 | 0.145 | 0.211 | 0.145 | yes |
| `find . -name 'Makefile'` | 10.34 |  | 2.052 | 1.344 | 0.917 | 1.113 | 1.290 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 2.912 |  | 0.001 | 0.002 | 0.001 | 0.002 | 0.001 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 2.636 |  | 0.028 | 0.048 | 0.040 | 0.027 | 0.025 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 2.697 |  | 0.003 | 0.003 | 0.003 | 0.003 | 0.003 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2.206 |  | 0.003 | 0.002 | 0.004 | 0.003 | 0.004 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 2.325 |  | 0.006 | 0.005 | 0.010 | 0.006 | 0.006 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 3.342 |  | 0.023 | 0.020 | 0.030 | 0.026 | 0.034 | yes |
| `wc -l doc/src/sgml/html/*.html` | 20.90 |  | 27.94 | 0.901 | 0.905 | 0.824 | 1.334 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 37.12 |  | 30.00 | 30.51 | 1.850 | 2.098 | 2.310 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 3.082 |  | 0.010 | 0.010 | 0.005 | 0.005 | 0.006 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 2.727 |  | 0.004 | 0.005 | 0.006 | 0.006 | 0.007 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 2.558 |  | 0.006 | 0.007 | 0.008 | 0.009 | 0.010 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 2.525 |  | 0.205 | 0.233 | 0.082 | 0.048 | 0.048 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 2.522 |  | 0.445 | 0.590 | 0.233 | 0.142 | 0.143 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 3.010 |  | 0.078 | 0.083 | 0.036 | 0.032 | 0.037 | yes |
| `grep -rn qdbzzabsentterm .` | 139.6 | 76.79 | 0.014 | 0.021 | 0.021 | 0.033 | 0.022 | yes |
| `grep -rl qdbzzabsentterm .` | 102.5 | 66.22 | 0.015 | 0.024 | 0.016 | 0.019 | 0.019 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 128.1 | 72.11 | 0.051 | 0.027 | 0.026 | 0.038 | 0.028 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 80.44 | 66.14 | 0.038 | 0.027 | 0.033 | 0.030 | 0.032 | yes |
| `grep -rn btvarstrequalimage .` | 151.1 | 71.79 | 1.605 | 0.183 | 0.085 | 0.109 | 0.111 | yes |
| `grep -rl btvarstrequalimage .` | 90.43 | 64.93 | 1.672 | 0.085 | 0.143 | 0.091 | 0.086 | yes |
| `grep -rn undesirable .` | 164.1 | 76.10 | 2.512 | 1.028 | 0.796 | 0.656 | 0.938 | yes |
| `grep -rl undesirable .` | 132.2 | 72.34 | 1.837 | 0.535 | 0.601 | 0.594 | 0.614 | yes |
| `grep -rn passed .` | 187.8 | 75.95 | 21.08 | 15.87 | 13.12 | 13.13 | 15.90 | yes |
| `grep -rl passed .` | 97.28 | 62.32 | 10.66 | 2.978 | 4.956 | 3.612 | 3.870 | yes |
| `grep -rn not .` | 289.7 | 141.9 | 35.22 | 36.87 | 39.04 | 34.77 | 38.21 | yes |
| `grep -rl not .` | 77.13 | 71.91 | 3.506 | 2.882 | 4.009 | 3.057 | 3.189 | yes |
| `grep -rc undesirable .` | 140.7 | 89.33 | 3.520 | 1.431 | 1.312 | 1.623 | 1.181 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 141.8 | 84.80 | 1.457 | 0.106 | 0.123 | 0.131 | 0.232 | yes |
| `grep -ri undesirable .` | 127.1 | 72.84 | 3.253 | 0.632 | 0.820 | 0.601 | 0.691 | yes |
| `grep -rn undesirable src/backend` | 70.30 | 22.75 | 1.529 | 0.418 | 0.390 | 0.402 | 0.345 | yes |
| `grep -rl passed src/backend` | 30.41 | 18.39 | 1.544 | 1.218 | 1.175 | 1.162 | 1.003 | yes |
| `grep -rF 'undesirable' .` | 119.5 | 68.46 | 1.579 | 0.744 | 0.633 | 0.924 | 0.598 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 98.31 | 68.53 | 8.872 | 6.092 | 3.126 | 3.044 | 3.709 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 226.6 | 70.68 | 1.877 | 0.263 | 0.214 | 0.220 | 0.245 | yes |
| `grep -rE 'passed\|undesirable' .` | 246.6 | 82.08 | 19.57 | 20.71 | 18.64 | 15.57 | 15.67 | yes |
| `grep -rE '[0-9]{4,}' .` | 286.3 | 349.9 | 176.6 | 185.0 | 188.5 | 183.5 | 185.9 | yes |
| `grep -rn '^#include' .` | 195.8 | 89.26 | 11.65 | 9.657 | 9.201 | 5.065 | 9.599 | yes |
| `grep -rE '^(import\|from) ' .` | 159.5 | 76.50 | 43.93 | 36.34 | 36.60 | 33.28 | 33.94 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 89.66 | 41.17 | 16.54 | 16.58 | 15.39 | 14.41 | 16.76 | yes |
| `grep -rv e src/include/access` | 4.933 | 6.670 | 0.482 | 0.757 | 0.511 | 0.602 | 0.423 | yes |
| `grep -r '\(undesirable\).*\1' .` | 133.5 |  | 2.322 | 1.364 | 0.935 | 1.193 | 0.816 | yes |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 18.33 |  | | 0.675 | 0.698 | 0.614 | 0.635 | yes |
| `grep -l undesirable doc/src/sgml/html/*.html` | 18.69 |  | | 1.091 | 0.617 | 0.685 | 1.273 | yes |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 14.74 |  | | 0.607 | 0.483 | 0.594 | 0.589 | yes |
| `grep -rl passed . \| wc -l` | 110.0 |  | 5.880 | 7.873 | 4.437 | 3.142 | 4.107 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 217.0 |  | 32.83 | 30.56 | 38.24 | 26.39 | 28.83 | yes |
| `find . -name '*.html' \| wc -l` | 12.66 |  | 1.615 | 1.048 | 1.528 | 1.142 | 1.753 | yes |
| `find . -type f \| grep -c html` | 13.50 |  | 1.328 | 1.278 | 2.604 | 1.143 | 1.109 | yes |
| `grep -rn not . \| head -n 5` | 4.067 |  | 0.008 | 0.008 | 0.015 | 0.009 | 0.012 | order differs |
| `grep -rn passed . \| head -n 1` | 17.86 |  | 0.012 | 0.022 | 0.018 | 0.015 | 0.015 | order differs |
| `find . -type f \| head -n 3` | 3.720 |  | 0.046 | 0.039 | 0.045 | 0.044 | 0.051 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 4.701 |  | 0.318 | 0.500 | 0.315 | 0.305 | 0.303 | yes |
| `grep -rno undesirable .` | 177.6 | 70.91 | | | 0.749 | 0.758 | 0.916 | yes |
| `grep -rw undesirable .` | 135.5 | 69.62 | | | 0.681 | 0.725 | 0.665 | yes |
| `grep -rnw btvarstrequalimage .` | 142.4 | 70.83 | | | 0.306 | 0.194 | 0.341 | yes |
| `grep -rn -C 2 btvarstrequalimage .` | 153.9 | 294.5 | | | 0.388 | 0.393 | 0.308 | yes |
| `grep -rn -A 1 -B 1 undesirable .` | 164.7 | 318.1 | | | 1.862 | 1.569 | 2.438 | yes |
| `grep -rho 'size_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 108.9 |  | | | 7.819 | 8.254 | 8.689 | yes |
| `grep -rhow '[a-z]*btvarstrequalimage[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 102.4 |  | | | 0.232 | 0.189 | 0.194 | yes |
| `rg -j 1 qdbzzabsentterm` | 77.02 |  | | | 0.037 | 0.032 | 0.052 | yes |
| `rg -j 1 abcdefghijklabcdefgh` | 83.02 |  | | | 0.047 | 0.048 | 0.057 | yes |
| `rg -j 1 btvarstrequalimage` | 83.51 |  | | | 0.226 | 0.157 | 0.168 | yes |
| `rg -j 1 undesirable` | 83.80 |  | | | 0.918 | 1.171 | 1.109 | yes |
| `rg -j 1 passed` | 89.17 |  | | | 19.95 | 19.66 | 17.70 | yes |
| `rg -j 1 not` | 145.0 |  | | | 38.35 | 37.86 | 38.17 | yes |
| `rg -j 1 -l undesirable` | 84.84 |  | | | 0.913 | 1.119 | 0.934 | yes |
| `rg -j 1 -l not` | 86.95 |  | | | 11.81 | 12.55 | 13.02 | yes |
| `rg -j 1 -c passed` | 83.58 |  | | | 18.23 | 19.79 | 18.26 | yes |
| `rg -j 1 -i BTVARSTREQUALIMAGE` | 101.1 |  | | | 1.079 | 0.941 | 1.725 | yes |
| `rg -j 1 -w undesirable` | 88.00 |  | | | 14.62 | 14.84 | 14.09 | yes |
| `rg -j 1 -F 'undesirable'` | 94.73 |  | | | 1.115 | 0.975 | 1.109 | yes |
| `rg -j 1 -n -C 2 btvarstrequalimage` | 96.03 |  | | | 0.365 | 0.545 | 0.419 | yes |
| `rg -j 1 'size_[a-z]+\('` | 91.60 |  | | | 4.521 | 8.717 | 8.127 | yes |
| `rg -j 1 'btvarstrequalimage\|abcdefghijklabcdefgh'` | 123.9 |  | | | 0.332 | 0.231 | 0.207 | yes |
| `rg -j 1 '\bbtvarstrequalimage\w*'` | 98.88 |  | | | 3.338 | 3.475 | 4.571 | yes |
| `rg -j 1 -t c undesirable` | 48.22 |  | | | 1.155 | 0.750 | 0.676 | yes |
| `rg -j 1 -g '*.html' undesirable` | 38.37 |  | | | 0.448 | 0.650 | 0.465 | yes |
| `rg -j 1 undesirable src/backend` | 25.71 |  | | | 0.777 | 0.473 | 0.561 | yes |
| `rg -j 1 -uu undesirable` | 68.85 |  | | | 0.739 | 0.738 | 0.604 | yes |
| `rg -j 1 --sort path btvarstrequalimage` | 84.24 |  | | | 0.240 | 0.299 | 0.148 | yes |
| `rg -j 1 --files \| wc -l` | 25.90 |  | | | 11.03 | 6.631 | 7.619 | yes |
| `rg -j 1 -l passed \| wc -l` | 89.32 |  | | | 12.41 | 15.16 | 13.88 | yes |
| `ls -ltR` | 40.34 |  | | | 4.384 | 3.721 | 4.162 | yes |
| `ls -lt doc/src/sgml/html` | 6.614 |  | | | 0.550 | 0.443 | 0.441 | yes |
| `ls -ltr src/backend \| tail -n 3` | 3.117 |  | | | 0.033 | 0.021 | 0.022 | yes |
| `find . -newer doc/src/sgml/html/bug-reporting.html` | 22.27 |  | | | 1.745 | 0.779 | 0.857 | yes |
| `find . -newer doc/src/sgml/html/bug-reporting.html -name '*.html'` | 16.11 |  | | | 1.884 | 1.039 | 1.044 | yes |
| `find . -type f -mtime +30` | 22.64 |  | | | 1.434 | 0.797 | 0.794 | yes |
| `du -s .` | 16.63 |  | | | 0.001 | 0.001 | 0.001 | not compared |
| `du -sh src/backend` | 4.505 |  | | | 0.002 | 0.001 | 0.001 | not compared |
| `du -sb .` | 15.94 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `du -sb src/backend` | 4.893 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `du -b -d 1 src/backend` | 5.342 |  | | | 0.004 | 0.004 | 0.005 | yes |
| `sort doc/src/sgml/html/bug-reporting.html` | 2.510 |  | | | 0.022 | 0.021 | 0.024 | yes |
| `sort -r doc/src/sgml/html/bug-reporting.html \| head -n 3` | 3.378 |  | | | 0.026 | 0.027 | 0.023 | yes |
| `sort -u doc/src/sgml/html/bug-reporting.html \| wc -l` | 3.811 |  | | | 0.026 | 0.023 | 0.028 | yes |
| `sort -k 2 doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 3.441 |  | | | 0.026 | 0.027 | 0.031 | yes |
| `cat doc/src/sgml/html/*.html \| sort \| uniq -c \| sort -rn \| head -n 5` | 222.3 |  | | | 103.5 | 120.2 | 105.0 | yes |
| `uniq -c doc/src/sgml/html/bug-reporting.html \| wc -l` | 3.108 |  | | | 0.017 | 0.020 | 0.021 | yes |
| `cut -c 1-20 doc/src/sgml/html/bug-reporting.html` | 2.597 |  | | | 0.016 | 0.016 | 0.014 | yes |
| `cut -d ' ' -f 1 doc/src/sgml/html/bug-reporting.html \| sort \| uniq \| wc -l` | 5.282 |  | | | 0.042 | 0.037 | 0.042 | yes |
| `tr a-z A-Z < doc/src/sgml/html/bug-reporting.html` | 2.762 |  | | | 0.004 | 0.003 | 0.003 | yes |
| `tr -d '\n' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 3.092 |  | | | 0.008 | 0.008 | 0.008 | yes |
| `tr -s ' ' < doc/src/sgml/html/bug-reporting.html \| wc -c` | 3.506 |  | | | 0.015 | 0.020 | 0.015 | yes |
| `nl doc/src/sgml/html/bug-reporting.html` | 2.102 |  | | | 0.028 | 0.039 | 0.023 | yes |
| `nl -ba doc/src/sgml/html/bug-reporting.html \| tail -n 3` | 3.303 |  | | | 0.027 | 0.049 | 0.026 | yes |
| `seq 1 100000 \| wc -l` | 4.460 |  | | | 2.278 | 4.282 | 2.486 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.293 |  | | | 0.026 | 0.026 | 0.027 | yes |
| `basename doc/src/sgml/html/bug-reporting.html` | 2.147 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `basename -s .html doc/src/sgml/html/bug-reporting.html` | 2.171 |  | | | 0.001 | 0.001 | 0.002 | yes |
| `dirname doc/src/sgml/html/bug-reporting.html` | 3.274 |  | | | 0.001 | 0.001 | 0.001 | yes |
| `printf '%s\n' a b c` | 1.679 |  | | | 0.002 | 0.002 | 0.003 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 1.750 |  | | | 0.002 | 0.003 | 0.003 | yes |
| `test -f doc/src/sgml/html/bug-reporting.html && echo yes` | 1.698 |  | | | 0.002 | 0.002 | 0.002 | yes |
| `[ -d src/backend ] && echo folder` | 1.373 |  | | | 0.002 | 0.003 | 0.003 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 2.286 |  | | | 0.003 | 0.003 | 0.005 | yes |
| `echo a b c \| xargs -n 1 echo` | 7.933 |  | | | 0.003 | 0.005 | 0.006 | yes |
| `find src/include/access -name '*.html' \| xargs cat \| wc -l` | 5.409 |  | | | 0.011 | 0.017 | 0.014 | yes |
| `find . -name '*.html' \| xargs grep -l undesirable \| wc -l` | 30.80 |  | | | 2.239 | 1.992 | 2.037 | yes |
| `echo {a,b,c}{1..3}` | 1.931 |  | | | 0.004 | 0.004 | 0.004 | yes |
| `echo {1..20..3} {a..e}` | 1.472 |  | | | 0.005 | 0.004 | 0.006 | yes |
| `echo $(wc -l < doc/src/sgml/html/bug-reporting.html) `basename doc/src/sgml/html/bug-reporting.html`` | 5.683 |  | | | 0.005 | 0.008 | 0.006 | yes |
| `echo "first line: $(head -n 1 doc/src/sgml/html/bug-reporting.html)"` | 3.072 |  | | | 0.005 | 0.005 | 0.006 | yes |
| `wc -l < doc/src/sgml/html/bug-reporting.html` | 3.470 |  | | | 0.002 | 0.003 | 0.002 | yes |
| `cat doc/src/sgml/html/bug-reporting.html > /dev/null 2>&1` | 3.031 |  | | | 0.002 | 0.002 | 0.002 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 3.174 |  | | | 0.002 | 0.003 | 0.006 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 3.470 |  | | | 0.002 | 0.003 | 0.004 | yes |
| `grep -q not doc/src/sgml/html/bug-reporting.html && echo found \|\| echo none` | 3.634 |  | | | 0.005 | 0.009 | 0.007 | yes |
| `false \|\| echo fallback` | 2.056 |  | | | 0.001 | 0.002 | 0.002 | yes |
| `true && grep -c not doc/src/sgml/html/bug-reporting.html` | 3.681 |  | | | 0.007 | 0.012 | 0.007 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 4.469 |  | | | 0.003 | 0.003 | 0.006 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 3.510 |  | | | 0.001 | 0.002 | 0.001 | yes |
| `echo one \⏎two` | 2.077 |  | | | 0.001 | 0.002 | 0.001 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html⏎wc -l doc/src/sgml/html/bug-reporting.html` | 4.879 |  | | | 0.007 | 0.008 | 0.009 | yes |
| `mkdir -p bench_out/a/b` | 3.088 |  | 0.010 | 0.676 | 0.015 | 0.695 | | yes |
| `touch bench_out/empty.txt` | 2.803 |  | 0.012 | 0.692 | 0.016 | 0.529 | | yes |
| `echo 'hello world' > bench_out/hello.txt` | 2.032 |  | 0.019 | 0.624 | 0.030 | 0.513 | | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.665 |  | 0.018 | 0.620 | 0.026 | 0.443 | | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 3.216 |  | 0.316 | 1.333 | 0.025 | 0.539 | | yes |
| `cp -r src/backend bench_out/dir_copy` | 329.3 |  | 372.3 | 491.9 | 1.700 | 3.259 | | yes |
| `cp -r doc/src/sgml/html bench_out/work` | 335.7 |  | 101.0 | 169.3 | 1.688 | 4.011 | | yes |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 4.088 |  | 0.538 | 1.175 | 0.585 | 1.791 | | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.454 |  | 0.492 | 1.118 | 0.461 | 1.462 | | yes |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 412.1 |  | 296.5 | 311.9 | 313.4 | 316.5 | | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | 835.0 |  | | 2.342 | 1.067 | 18.80 | | yes |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | 831.1 |  | | 20.30 | 2.182 | 20.43 | | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 3.205 |  | | 0.042 | 0.029 | 0.650 | | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 3.360 |  | | 0.042 | 0.029 | 0.650 | | yes |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | 3.329 |  | | 0.035 | 0.039 | 0.628 | | yes |
| `grep -rc QDBBENCHMANY bench_out` | 30.16 |  | 4.294 | 4.209 | 3.365 | 4.242 | | yes |
| `grep -rl QDBBENCHMANY .` | 108.4 |  | 1.838 | 1.769 | 1.368 | 1.389 | | yes |
| `grep -rl QDBBENCHONE .` | 109.1 |  | 0.384 | 0.368 | 0.368 | 0.329 | | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| tee bench_out/tee.txt \| wc -l` | 4.666 |  | | | 0.343 | 1.450 | | yes |
| `sort doc/src/sgml/html/bug-reporting.html > bench_out/sorted.txt` | 4.210 |  | | | 0.331 | 0.995 | | yes |
| `sort -u doc/src/sgml/html/bug-reporting.html >> bench_out/sorted.txt` | 3.454 |  | | | 0.408 | 1.046 | | yes |
| `cat qdb_no_such_file &> bench_out/both.txt` | 2.665 |  | | | 0.035 | 0.549 | | yes |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 2.691 |  | | | 0.032 | 0.533 | | yes |
| `find src/include/access -name '*.html' \| sort \| xargs cat > bench_out/joined.txt` | 5.008 |  | | | 0.068 | 0.503 | | yes |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 2.363 |  | | | 0.020 | 0.441 | | yes |
| `sed -i '1d' bench_out/joined2.txt` | 2.458 |  | | | 0.021 | 0.462 | | yes |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 2.324 |  | | | 0.067 | 0.026 | | yes |
| `cat bench_out/both.txt` | 2.099 |  | | | 0.009 | 0.018 | | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.597 |  | 0.024 | 0.805 | 0.019 | 0.565 | | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.684 |  | 0.022 | 0.852 | 0.021 | 0.556 | | yes |
| `ls -R bench_out/a` | 2.562 |  | 0.012 | 0.026 | 0.020 | 0.017 | | yes |
| `rm bench_out/a/b/hello.txt` | 2.182 |  | 0.008 | 0.704 | 0.021 | 0.540 | | yes |
| `rmdir bench_out/a/b` | 2.818 |  | 0.010 | 0.327 | 0.017 | 0.355 | | yes |
| `rm -r bench_out/dir_moved` | 26.93 |  | 12.58 | 38.26 | 0.736 | 1.271 | | yes |
| `rm -rf bench_out` | 116.3 |  | 19.33 | 25.07 | 10.61 | 11.43 | | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes |
|---|---:|---:|---:|---:|---:|---:|
| `ls` | 144 B | 961 B | 961 B | 2795 B | 2795 B | 2795 B |
| `ls -a` | 307 B | 1785 B | 1785 B | 3877 B | 3877 B | 3877 B |
| `ls -R` | 151.8 KB | 73.0 KB | 73.0 KB | 93.5 KB | 93.5 KB | 93.5 KB |
| `ls -aR src/backend` | 23.1 KB | 29.2 KB | 29.2 KB | 33.1 KB | 33.1 KB | 33.1 KB |
| `ls -p src/backend` | 279 B | 2075 B | 2075 B | 4306 B | 4306 B | 4306 B |
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
| `ls -ltR` | 513.7 KB | | | 144.6 KB | 144.6 KB | 144.6 KB |
| `ls -lt doc/src/sgml/html` | 74.4 KB | | | 105.9 KB | 105.9 KB | 105.9 KB |
| `ls -ltr src/backend \| tail -n 3` | 149 B | | | 7100 B | 7100 B | 7100 B |
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
| `mkdir -p bench_out/a/b` | 0 B | 1827 B | 2115 B | 4079 B | 4079 B | |
| `touch bench_out/empty.txt` | 0 B | 979 B | 979 B | 2566 B | 2566 B | |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1242 B | 1292 B | 3665 B | 3661 B | |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1246 B | 1246 B | 3485 B | 3473 B | |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0 B | 24.1 KB | 283.4 KB | 3344 B | 3344 B | |
| `cp -r src/backend bench_out/dir_copy` | 0 B | 68.8 MB | 9479.9 KB | 665.9 KB | 1025.5 KB | |
| `cp -r doc/src/sgml/html bench_out/work` | 0 B | 19.2 MB | 5277.4 KB | 456.1 KB | 579.1 KB | |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 39.7 KB | 56.2 KB | 59.5 KB | 301.3 KB | |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 38.3 KB | 57.9 KB | 61.4 KB | 61.4 KB | |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 0 B | 10.3 MB | 2498.0 KB | 19.5 MB | 3604.7 KB | |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | 0 B | | 263.0 KB | 265.0 KB | 265.0 KB | |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | 0 B | | 265.5 KB | 267.6 KB | 267.6 KB | |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 B | | 20.9 KB | 2924 B | 2924 B | |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 B | | 20.9 KB | 2924 B | 2924 B | |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | 0 B | | 21.1 KB | 23.5 KB | 23.5 KB | |
| `grep -rc QDBBENCHMANY bench_out` | 110.1 KB | 33.3 KB | 33.1 KB | 35.4 KB | 35.4 KB | |
| `grep -rl QDBBENCHMANY .` | 36.1 KB | 97.6 KB | 105.6 KB | 107.8 KB | 107.8 KB | |
| `grep -rl QDBBENCHONE .` | 21 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | |
| `cat doc/src/sgml/html/bug-reporting.html \| tee bench_out/tee.txt \| wc -l` | 4 B | | | 44.8 KB | 44.8 KB | |
| `sort doc/src/sgml/html/bug-reporting.html > bench_out/sorted.txt` | 0 B | | | 127.3 KB | 127.3 KB | |
| `sort -u doc/src/sgml/html/bug-reporting.html >> bench_out/sorted.txt` | 0 B | | | 126.7 KB | 126.7 KB | |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 B | | | 3786 B | 3795 B | |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 0 B | | | 3714 B | 3714 B | |
| `find src/include/access -name '*.html' \| sort \| xargs cat > bench_out/joined.txt` | 0 B | | | 70.9 KB | 70.9 KB | |
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
| `rm -rf bench_out` | 0 B | 9354 B | 708.3 KB | 11.1 KB | 11.1 KB | |

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
