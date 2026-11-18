# Benchmark: linux

- Data: Linux 6.8 source tree, from the Ubuntu source package linux_6.8.0.orig.tar.gz (sha256 26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8). Skipped on import: 64 symbolic links, 6 files that are not valid text, 0 other
- Size: 83405 files in 5451 folders, 1396.9 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 4, on disk, settled), and include starting bash and the tool (1.627 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 1374.6 | 8146.1 | 1568.1 MB | 1578.0 MB | 0.280 | yes |
| Stage 3, on disk, settled | on | 13807.8 | 13807.8 | 21.7 MB | 967.2 MB | 0.310 | yes |
| Stage 4, in memory | on | 3327.4 | 10917.9 | 1667.4 MB | 1602.1 MB | 0.287 | yes |
| Stage 4, on disk, settled | on | 14147.9 | 14147.9 | 22.4 MB | 964.5 MB | 0.316 | yes |
| Stage 4, on disk, after the writes | on | 14147.9 | 14147.9 | 22.4 MB | 964.5 MB | 0.316 | yes |
| Stage 4, on disk, 512 MB limit | on | 13482.9 | 13482.9 | 22.4 MB | 966.4 MB | 0.276 | yes |
| Stage 4, on disk, 1 GB for everything, file cache included | on | 13254.4 | 13254.4 | 22.4 MB | 916.5 MB | 0.287 | yes |

## The index (Stage 2, in memory)

- Built in 6771.5 ms while loading.
- Size: 148.0 MB in memory, 10.6% of the text (8388.6 KB table, 137.5 MB lists, 2097.2 KB document table). The posting data itself is 117.4 MB.
- 236444 distinct trigrams, 111408369 postings, 83382 documents.
- Share of posting data in the longest lists: top 100 trigrams 5.4%, top 1,000 30.2%, top 10,000 79.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|
| 1 | 46218 |
| 2 | 24419 |
| 3-4 | 25488 |
| 5-8 | 23498 |
| 9-16 | 21475 |
| 17-32 | 19750 |
| 33-64 | 17347 |
| 65-128 | 14640 |
| 129-256 | 12623 |
| 257-512 | 9771 |
| 513-1024 | 7266 |
| 1025-2048 | 4979 |
| 2049-4096 | 3386 |
| 4097-8192 | 2227 |
| 8193-16384 | 1755 |
| 16385-32768 | 1139 |
| 32769-65536 | 431 |
| 65537-131072 | 32 |

Index work per command that changes data:

| Command | Index work |
|---|---|
| `mkdir -p bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +9698 -0; postings +9217002; list bytes rewritten 0; rebuilds 0 |
| `cp -r include/linux bench_out/work` | documents +2631 -0; postings +2591832; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -1; postings +2616; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2504; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | documents +1432 -1432; postings +1564925; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -9698; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -2632; postings +0; list bytes rewritten 0; rebuilds 0 |

## The index (Stage 3, on disk, settled)

- Built in 0.000 ms while loading.
- Size: 141.5 MB in memory, 10.1% of the text (10.5 MB table, 129.9 MB lists, 1064.6 KB document table). The posting data itself is 129.9 MB.
- 183791 distinct trigrams, 0 postings, 93472 documents.
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
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +1 -0; postings +2610; list bytes rewritten 8253; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +9762 -0; postings +9359558; list bytes rewritten 27696; rebuilds 0 |
| `cp -r include/linux bench_out/work` | documents +2677 -0; postings +2662814; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -1; postings +2616; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2504; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | documents +1428 -1408; postings +1603987; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1645; postings +0; list bytes rewritten 3352793; rebuilds 0 |

## The index (Stage 4, in memory)

- Built in 7590.4 ms while loading.
- Size: 147.0 MB in memory, 10.5% of the text (13.6 MB table, 132.3 MB lists, 1081.3 KB document table). The posting data itself is 122.0 MB.
- 184112 distinct trigrams, 0 postings, 93472 documents.
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
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r include/linux bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +2616; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2504; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | documents +1428 -0; postings +1603987; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | documents +1 -1; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | documents +29 -0; postings +168701; list bytes rewritten 0; rebuilds 0 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1d' bench_out/joined2.txt` | documents +1 -0; postings +5455; list bytes rewritten 0; rebuilds 0 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat bench_out/both.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1462; postings +0; list bytes rewritten 0; rebuilds 0 |

## The index (Stage 4, on disk, settled)

- Built in 0.000 ms while loading.
- Size: 141.5 MB in memory, 10.1% of the text (10.5 MB table, 129.9 MB lists, 1065.1 KB document table). The posting data itself is 129.9 MB.
- 183764 distinct trigrams, 0 postings, 93472 documents.
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
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r include/linux bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +2616; list bytes rewritten 8268; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2504; list bytes rewritten 10435; rebuilds 0 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | documents +1428 -0; postings +1603987; list bytes rewritten 17932; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | documents +1 -1; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | documents +29 -0; postings +168701; list bytes rewritten 0; rebuilds 0 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1d' bench_out/joined2.txt` | documents +1 -0; postings +5455; list bytes rewritten 0; rebuilds 0 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat bench_out/both.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1462; postings +0; list bytes rewritten 2669308; rebuilds 0 |

## The index (Stage 4, on disk, after the writes)

- Built in 0.000 ms while loading.
- Size: 141.5 MB in memory, 10.1% of the text (10.5 MB table, 129.9 MB lists, 1065.1 KB document table). The posting data itself is 129.9 MB.
- 183764 distinct trigrams, 0 postings, 93472 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

## The index (Stage 4, on disk, 512 MB limit)

- Built in 0.000 ms while loading.
- Size: 141.5 MB in memory, 10.1% of the text (10.5 MB table, 129.9 MB lists, 1065.1 KB document table). The posting data itself is 129.9 MB.
- 183764 distinct trigrams, 0 postings, 93472 documents.
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
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r include/linux bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +2616; list bytes rewritten 8268; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2504; list bytes rewritten 10435; rebuilds 0 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | documents +1428 -0; postings +1603987; list bytes rewritten 17932; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | documents +1 -1; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | documents +29 -0; postings +168701; list bytes rewritten 0; rebuilds 0 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1d' bench_out/joined2.txt` | documents +1 -0; postings +5455; list bytes rewritten 0; rebuilds 0 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat bench_out/both.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1462; postings +0; list bytes rewritten 2669308; rebuilds 0 |

## The index (Stage 4, on disk, 1 GB for everything, file cache included)

- Built in 0.000 ms while loading.
- Size: 141.5 MB in memory, 10.1% of the text (10.5 MB table, 129.9 MB lists, 1065.1 KB document table). The posting data itself is 129.9 MB.
- 183764 distinct trigrams, 0 postings, 93472 documents.
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
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp -r include/linux bench_out/work` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -0; postings +2616; list bytes rewritten 8268; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2504; list bytes rewritten 10435; rebuilds 0 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | documents +1428 -0; postings +1603987; list bytes rewritten 17932; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | documents +1 -1; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file &> bench_out/both.txt` | documents +1 -0; postings +42; list bytes rewritten 0; rebuilds 0 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | documents +1 -1; postings +42; list bytes rewritten 0; rebuilds 0 |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | documents +29 -0; postings +168701; list bytes rewritten 0; rebuilds 0 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1d' bench_out/joined2.txt` | documents +1 -0; postings +5455; list bytes rewritten 0; rebuilds 0 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat bench_out/both.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1462; postings +0; list bytes rewritten 2669308; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 4, on disk, 512 MB limit | Stage 4, on disk, 1 GB for everything, file cache included | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.401 |  | 0.001 | 0.001 | 0.002 | 0.001 | 0.004 | 0.002 | 0.030 | yes |
| `ls -a` | 2.227 |  | 0.004 | 0.004 | 0.002 | 0.003 | 0.005 | 0.002 | 0.004 | yes |
| `ls -R` | 78.15 |  | 8.025 | 3.303 | 11.81 | 4.700 | 3.506 | 3.783 | 11.94 | yes |
| `ls -aR Documentation` | 12.56 |  | 0.747 | 0.861 | 0.838 | 0.950 | 0.692 | 0.659 | 1.219 | yes |
| `ls -p Documentation` | 2.610 |  | 0.004 | 0.004 | 0.004 | 0.005 | 0.005 | 0.004 | 0.011 | yes |
| `find .` | 79.80 |  | 20.82 | 10.16 | 13.43 | 11.32 | 11.69 | 7.845 | 8.976 | yes |
| `find . -name '*.h'` | 69.80 |  | 25.53 | 10.22 | 16.53 | 10.78 | 10.78 | 10.76 | 13.05 | yes |
| `find . -name '*.c'` | 73.98 |  | 28.25 | 10.50 | 15.00 | 10.79 | 11.50 | 9.946 | 11.39 | yes |
| `find . -type d` | 62.17 |  | 25.02 | 6.396 | 14.78 | 6.464 | 6.794 | 7.453 | 7.901 | yes |
| `find . -type f` | 88.77 |  | 23.39 | 11.97 | 15.19 | 7.163 | 6.812 | 8.360 | 9.119 | yes |
| `find . -maxdepth 2 -type f` | 5.036 |  | 0.114 | 0.109 | 0.131 | 0.201 | 0.129 | 0.132 | 0.129 | yes |
| `find Documentation -name '*.h' -type f` | 11.02 |  | 1.770 | 1.071 | 1.110 | 1.629 | 1.302 | 1.167 | 1.088 | yes |
| `find . -name 'Makefile'` | 69.81 |  | 27.58 | 7.526 | 12.93 | 7.135 | 7.509 | 10.78 | 10.33 | yes |
| `cat include/linux/kernfs.h` | 2.478 |  | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `cat -n include/linux/kernfs.h` | 2.153 |  | 0.057 | 0.055 | 0.055 | 0.071 | 0.065 | 0.054 | 0.055 | yes |
| `head -n 20 include/linux/kernfs.h` | 2.191 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.002 | 0.002 | yes |
| `tail -n 20 include/linux/kernfs.h` | 2.188 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.004 | 0.002 | 0.002 | yes |
| `tail -n +100 include/linux/kernfs.h` | 3.031 |  | 0.003 | 0.003 | 0.004 | 0.004 | 0.006 | 0.004 | 0.007 | yes |
| `wc include/linux/kernfs.h` | 3.108 |  | 0.024 | 0.023 | 0.026 | 0.037 | 0.026 | 0.022 | 0.035 | yes |
| `wc -l include/linux/*.h` | 14.86 |  | 28.62 | 0.885 | 0.845 | 1.369 | 1.013 | 0.807 | 0.896 | yes |
| `cat include/linux/*.h \| wc -l` | 35.91 |  | 26.33 | 24.59 | 1.649 | 1.655 | 1.508 | 1.789 | 2.143 | yes |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 3.113 |  | 0.005 | 0.006 | 0.004 | 0.004 | 0.007 | 0.004 | 0.004 | yes |
| `grep -c define include/linux/kernfs.h` | 2.691 |  | 0.006 | 0.009 | 0.010 | 0.010 | 0.011 | 0.010 | 0.043 | yes |
| `grep -n define include/linux/kernfs.h` | 2.848 |  | 0.006 | 0.009 | 0.010 | 0.012 | 0.014 | 0.010 | 0.010 | yes |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 3.102 |  | 0.249 | 0.226 | 0.011 | 0.013 | 0.014 | 0.012 | 0.011 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 4.101 |  | 0.588 | 0.560 | 0.484 | 0.755 | 0.425 | 0.410 | 0.492 | yes |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 3.692 |  | 0.152 | 0.120 | 0.071 | 0.117 | 0.074 | 0.055 | 0.062 | yes |
| `grep -rn qdbzzabsentterm .` | 1164.5 | 635.3 | 0.012 | 0.020 | 0.031 | 0.018 | 0.022 | 0.024 | 0.102 | yes |
| `grep -rl qdbzzabsentterm .` | 782.2 | 574.0 | 0.011 | 0.021 | 0.021 | 0.024 | 0.025 | 0.022 | 0.037 | yes |
| `grep -rn ausgabenspezifisches .` | 1187.1 | 611.2 | 0.098 | 0.125 | 0.255 | 0.175 | 0.145 | 0.208 | 0.284 | yes |
| `grep -rl ausgabenspezifisches .` | 850.4 | 555.8 | 0.095 | 0.100 | 0.237 | 0.123 | 0.113 | 0.216 | 0.120 | yes |
| `grep -rn mispredicted .` | 1377.0 | 669.5 | 17.54 | 2.475 | 2.013 | 1.846 | 2.426 | 1.484 | 3.261 | yes |
| `grep -rl mispredicted .` | 854.5 | 555.0 | 17.60 | 1.544 | 1.323 | 1.388 | 1.433 | 1.228 | 2.483 | yes |
| `grep -rn separately .` | 999.6 | 667.7 | 24.09 | 14.82 | 14.06 | 15.89 | 15.28 | 14.82 | 50.00 | yes |
| `grep -rl separately .` | 641.5 | 544.5 | 18.81 | 11.89 | 9.678 | 7.971 | 11.99 | 11.84 | 11.74 | yes |
| `grep -rn conditions .` | 1306.3 | 647.4 | 129.9 | 62.60 | 64.28 | 60.69 | 68.25 | 61.98 | 321.4 | yes |
| `grep -rl conditions .` | 728.5 | 503.4 | 65.31 | 40.80 | 42.35 | 41.35 | 50.50 | 37.82 | 37.92 | yes |
| `grep -rn define .` | 3397.1 | 3112.8 | 460.7 | 518.8 | 721.8 | 453.8 | 552.3 | 542.8 | 2157.3 | yes |
| `grep -rl define .` | 609.4 | 612.6 | 100.9 | 59.67 | 66.79 | 59.40 | 73.52 | 60.42 | 588.4 | yes |
| `grep -rc separately .` | 645.6 | 596.8 | 44.65 | 17.95 | 22.06 | 17.37 | 19.14 | 19.89 | 18.51 | yes |
| `grep -rin MISPREDICTED .` | 1584.0 | 785.2 | 21.12 | 2.271 | 2.122 | 1.781 | 1.913 | 2.990 | 3.418 | yes |
| `grep -ri separately .` | 809.0 | 726.7 | 28.93 | 16.61 | 15.76 | 16.46 | 17.11 | 21.64 | 19.32 | yes |
| `grep -rn separately Documentation` | 70.90 | 60.14 | 2.473 | 1.884 | 2.131 | 1.981 | 2.047 | 1.869 | 4.498 | yes |
| `grep -rl conditions Documentation` | 66.62 | 61.34 | 3.711 | 3.362 | 3.715 | 4.539 | 3.145 | 4.839 | 7.190 | yes |
| `grep -rF 'separately' .` | 649.0 | 650.1 | 24.42 | 14.81 | 13.94 | 15.19 | 15.36 | 14.80 | 15.08 | yes |
| `grep -rE 'atomic_[a-z]+\(' .` | 1243.4 | 671.0 | 74.42 | 51.84 | 55.70 | 68.58 | 70.09 | 49.99 | 284.3 | yes |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 1721.5 | 676.4 | 29.62 | 2.512 | 2.288 | 3.797 | 3.471 | 3.226 | 6.965 | yes |
| `grep -rE 'conditions\|separately' .` | 2188.5 | 729.3 | 180.1 | 75.05 | 77.90 | 85.92 | 77.01 | 89.75 | 238.5 | yes |
| `grep -rE '[0-9]{4,}' .` | 2784.1 | 3517.9 | 2025.8 | 2153.3 | 2242.9 | 2177.9 | 2079.3 | 2008.0 | 2817.3 | yes |
| `grep -rn '^#include' .` | 1621.3 | 932.4 | 244.7 | 217.7 | 216.7 | 237.1 | 240.7 | 230.0 | 983.1 | yes |
| `grep -rE '^(import\|from) ' .` | 1162.5 | 722.9 | 315.7 | 263.9 | 269.0 | 268.6 | 285.2 | 286.1 | 1002.8 | yes |
| `grep -rin 'return [a-z_]*;$' Documentation` | 103.0 | 55.92 | 11.32 | 4.304 | 5.141 | 4.570 | 5.156 | 3.799 | 57.36 | yes |
| `grep -rv e drivers/usb` | 62.52 | 66.99 | 17.91 | 18.05 | 18.38 | 19.80 | 19.55 | 18.13 | 19.97 | yes |
| `grep -r '\(separately\).*\1' .` | 660.0 |  | 24.81 | 15.77 | 16.42 | 16.14 | 15.66 | 16.99 | 73.82 | yes |
| `grep -n mispredicted include/linux/*.h` | 20.04 |  | | 0.998 | 1.135 | 1.198 | 1.160 | 0.990 | 1.414 | yes |
| `grep -l separately include/linux/*.h` | 11.48 |  | | 1.514 | 1.509 | 1.571 | 1.375 | 1.417 | 1.270 | yes |
| `grep -c qdbzzabsentterm include/linux/*.h` | 12.50 |  | | 0.595 | 0.542 | 0.592 | 0.651 | 0.680 | 0.895 | yes |
| `grep -rl conditions . \| wc -l` | 769.7 |  | 58.03 | 42.95 | 45.55 | 39.57 | 50.85 | 40.97 | 193.2 | yes |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1309.5 |  | 342.3 | 315.0 | 321.6 | 332.4 | 321.1 | 328.5 | 890.4 | yes |
| `find . -name '*.h' \| wc -l` | 78.17 |  | 23.83 | 11.43 | 12.39 | 9.695 | 8.064 | 10.28 | 16.56 | yes |
| `find . -type f \| grep -c h` | 71.33 |  | 27.69 | 13.79 | 20.38 | 12.69 | 14.03 | 13.52 | 17.81 | yes |
| `grep -rn define . \| head -n 5` | 4.478 |  | 0.019 | 0.024 | 0.025 | 0.027 | 0.028 | 0.036 | 0.070 | order differs |
| `grep -rn conditions . \| head -n 1` | 11.31 |  | 0.027 | 0.040 | 0.044 | 0.046 | 0.050 | 0.041 | 0.133 | order differs |
| `find . -type f \| head -n 3` | 3.272 |  | 0.058 | 0.093 | 0.052 | 0.059 | 0.066 | 0.053 | 0.054 | order differs |
| `cat include/linux/*.h \| head -n 2` | 4.462 |  | 0.346 | 0.335 | 0.324 | 0.475 | 0.324 | 0.479 | 0.289 | yes |
| `grep -rno separately .` | 1030.5 | 643.8 | | | 17.59 | 14.42 | 15.68 | 15.83 | 99.75 | yes |
| `grep -rw separately .` | 623.4 | 605.0 | | | 14.01 | 13.63 | 13.77 | 15.16 | 16.39 | yes |
| `grep -rnw mispredicted .` | 1173.8 | 611.2 | | | 1.934 | 1.927 | 1.633 | 1.959 | 4.685 | yes |
| `grep -rn -C 2 mispredicted .` | 1367.0 | 3181.8 | | | 2.482 | 2.294 | 2.781 | 2.381 | 4.147 | yes |
| `grep -rn -A 1 -B 1 separately .` | 1030.6 | 3406.3 | | | 22.36 | 23.01 | 23.65 | 25.13 | 225.7 | yes |
| `grep -rho 'atomic_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 1378.9 |  | | | 85.16 | 85.92 | 98.70 | 108.6 | 431.1 | yes |
| `grep -rhow '[a-z]*mispredicted[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 954.0 |  | | | 4.568 | 4.082 | 4.905 | 4.590 | 5.669 | yes |
| `rg -j 1 qdbzzabsentterm` | 657.3 |  | | | 0.088 | 0.068 | 0.093 | 0.106 | 0.075 | yes |
| `rg -j 1 ausgabenspezifisches` | 621.3 |  | | | 0.112 | 0.101 | 0.154 | 0.169 | 0.244 | yes |
| `rg -j 1 mispredicted` | 660.6 |  | | | 2.173 | 2.226 | 2.261 | 1.902 | 3.054 | yes |
| `rg -j 1 separately` | 672.2 |  | | | 21.23 | 16.57 | 19.18 | 17.65 | 18.47 | yes |
| `rg -j 1 conditions` | 732.7 |  | | | 126.9 | 113.5 | 142.3 | 120.3 | 293.0 | yes |
| `rg -j 1 define` | 2298.7 |  | | | 536.9 | 509.8 | 537.3 | 468.6 | 2370.2 | yes |
| `rg -j 1 -l separately` | 714.9 |  | | | 16.63 | 13.89 | 16.26 | 13.62 | 124.2 | yes |
| `rg -j 1 -l define` | 737.2 |  | | | 117.8 | 113.0 | 116.9 | 125.3 | 720.8 | yes |
| `rg -j 1 -c conditions` | 757.2 |  | | | 120.4 | 127.7 | 133.5 | 110.3 | 341.8 | yes |
| `rg -j 1 -i MISPREDICTED` | 786.3 |  | | | 20.46 | 20.15 | 21.00 | 20.40 | 27.02 | yes |
| `rg -j 1 -w separately` | 715.3 |  | | | 27.44 | 25.90 | 25.67 | 27.16 | 121.4 | yes |
| `rg -j 1 -F 'separately'` | 716.7 |  | | | 17.83 | 18.03 | 18.68 | 17.76 | 18.92 | yes |
| `rg -j 1 -n -C 2 mispredicted` | 737.9 |  | | | 2.546 | 2.716 | 3.209 | 2.969 | 4.415 | yes |
| `rg -j 1 'atomic_[a-z]+\('` | 743.8 |  | | | 64.24 | 64.29 | 69.70 | 60.18 | 243.8 | yes |
| `rg -j 1 'mispredicted\|ausgabenspezifisches'` | 738.0 |  | | | 2.994 | 2.382 | 2.463 | 2.444 | 4.016 | yes |
| `rg -j 1 '\bmispredicted\w*'` | 723.8 |  | | | 6.912 | 5.998 | 6.434 | 5.838 | 7.881 | yes |
| `rg -j 1 -t c separately` | 640.1 |  | | | 15.06 | 14.48 | 16.50 | 14.89 | 16.91 | yes |
| `rg -j 1 -g '*.h' separately` | 328.0 |  | | | 5.155 | 5.530 | 5.312 | 4.912 | 6.451 | yes |
| `rg -j 1 separately Documentation` | 58.77 |  | | | 11.06 | 9.652 | 8.437 | 10.61 | 10.84 | yes |
| `rg -j 1 -uu separately` | 629.4 |  | | | 13.35 | 13.22 | 14.89 | 13.02 | 14.53 | yes |
| `rg -j 1 --sort path mispredicted` | 665.1 |  | | | 2.314 | 1.925 | 3.109 | 2.136 | 3.575 | yes |
| `rg -j 1 --files \| wc -l` | 162.8 |  | | | 72.11 | 88.76 | 61.96 | 61.92 | 92.41 | yes |
| `rg -j 1 -l conditions \| wc -l` | 661.0 |  | | | 100.1 | 96.09 | 110.0 | 102.1 | 113.9 | yes |
| `ls -ltR` | 332.0 |  | | | 45.91 | 45.92 | 43.44 | 44.09 | 55.93 | yes |
| `ls -lt include/linux` | 7.886 |  | | | 0.634 | 0.885 | 0.639 | 0.589 | 0.868 | yes |
| `ls -ltr Documentation \| tail -n 3` | 5.067 |  | | | 0.052 | 0.052 | 0.057 | 0.051 | 0.065 | yes |
| `find . -newer include/linux/kernfs.h` | 158.6 |  | | | 12.62 | 8.639 | 6.778 | 7.574 | 8.893 | yes |
| `find . -newer include/linux/kernfs.h -name '*.h'` | 106.7 |  | | | 14.76 | 8.308 | 8.254 | 8.318 | 9.991 | yes |
| `find . -type f -mtime +30` | 147.6 |  | | | 13.32 | 11.68 | 7.837 | 9.768 | 13.65 | yes |
| `du -s .` | 145.4 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.016 | not compared |
| `du -sh Documentation` | 20.41 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.002 | not compared |
| `du -sb .` | 144.7 |  | | | 0.001 | 0.002 | 0.002 | 0.001 | 0.013 | yes |
| `du -sb Documentation` | 19.98 |  | | | 0.001 | 0.002 | 0.002 | 0.002 | 0.001 | yes |
| `du -b -d 1 Documentation` | 21.69 |  | | | 0.007 | 0.007 | 0.017 | 0.006 | 0.009 | yes |
| `sort include/linux/kernfs.h` | 3.275 |  | | | 0.043 | 0.040 | 0.070 | 0.042 | 0.046 | yes |
| `sort -r include/linux/kernfs.h \| head -n 3` | 4.038 |  | | | 0.045 | 0.072 | 0.071 | 0.045 | 0.074 | yes |
| `sort -u include/linux/kernfs.h \| wc -l` | 3.880 |  | | | 0.040 | 0.047 | 0.053 | 0.037 | 0.075 | yes |
| `sort -k 2 include/linux/kernfs.h \| tail -n 3` | 3.833 |  | | | 0.054 | 0.061 | 0.063 | 0.060 | 0.099 | yes |
| `cat include/linux/*.h \| sort \| uniq -c \| sort -rn \| head -n 5` | 242.9 |  | | | 184.2 | 141.1 | 168.6 | 161.7 | 177.0 | yes |
| `uniq -c include/linux/kernfs.h \| wc -l` | 3.474 |  | | | 0.036 | 0.028 | 0.030 | 0.028 | 0.027 | yes |
| `cut -c 1-20 include/linux/kernfs.h` | 2.450 |  | | | 0.020 | 0.018 | 0.023 | 0.017 | 0.019 | yes |
| `cut -d ' ' -f 1 include/linux/kernfs.h \| sort \| uniq \| wc -l` | 3.966 |  | | | 0.076 | 0.066 | 0.076 | 0.075 | 0.077 | yes |
| `tr a-z A-Z < include/linux/kernfs.h` | 3.143 |  | | | 0.005 | 0.003 | 0.004 | 0.005 | 0.003 | yes |
| `tr -d '\n' < include/linux/kernfs.h \| wc -c` | 3.395 |  | | | 0.015 | 0.008 | 0.010 | 0.009 | 0.011 | yes |
| `tr -s ' ' < include/linux/kernfs.h \| wc -c` | 3.358 |  | | | 0.023 | 0.013 | 0.015 | 0.014 | 0.013 | yes |
| `nl include/linux/kernfs.h` | 2.561 |  | | | 0.087 | 0.043 | 0.050 | 0.047 | 0.046 | yes |
| `nl -ba include/linux/kernfs.h \| tail -n 3` | 3.697 |  | | | 0.099 | 0.046 | 0.080 | 0.045 | 0.051 | yes |
| `seq 1 100000 \| wc -l` | 4.650 |  | | | 4.246 | 2.469 | 2.528 | 2.466 | 3.481 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.460 |  | | | 0.024 | 0.024 | 0.027 | 0.026 | 0.034 | yes |
| `basename include/linux/kernfs.h` | 2.851 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `basename -s .h include/linux/kernfs.h` | 2.549 |  | | | 0.002 | 0.001 | 0.001 | 0.002 | 0.001 | yes |
| `dirname include/linux/kernfs.h` | 2.785 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `printf '%s\n' a b c` | 1.876 |  | | | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 1.939 |  | | | 0.002 | 0.002 | 0.004 | 0.003 | 0.002 | yes |
| `test -f include/linux/kernfs.h && echo yes` | 1.938 |  | | | 0.003 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `[ -d Documentation ] && echo folder` | 1.804 |  | | | 0.002 | 0.004 | 0.003 | 0.003 | 0.002 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 2.949 |  | | | 0.003 | 0.004 | 0.006 | 0.003 | 0.004 | yes |
| `echo a b c \| xargs -n 1 echo` | 6.837 |  | | | 0.003 | 0.004 | 0.006 | 0.003 | 0.003 | yes |
| `find drivers/usb -name '*.h' \| xargs cat \| wc -l` | 10.12 |  | | | 0.403 | 0.558 | 0.444 | 0.429 | 0.657 | yes |
| `find . -name '*.h' \| xargs grep -l separately \| wc -l` | 231.3 |  | | | 54.13 | 52.35 | 39.39 | 35.91 | 74.50 | yes |
| `echo {a,b,c}{1..3}` | 1.988 |  | | | 0.005 | 0.006 | 0.005 | 0.004 | 0.004 | yes |
| `echo {1..20..3} {a..e}` | 1.841 |  | | | 0.005 | 0.007 | 0.006 | 0.005 | 0.005 | yes |
| `echo $(wc -l < include/linux/kernfs.h) `basename include/linux/kernfs.h`` | 5.256 |  | | | 0.005 | 0.008 | 0.007 | 0.006 | 0.006 | yes |
| `echo "first line: $(head -n 1 include/linux/kernfs.h)"` | 3.637 |  | | | 0.005 | 0.004 | 0.007 | 0.004 | 0.006 | yes |
| `wc -l < include/linux/kernfs.h` | 3.445 |  | | | 0.003 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `cat include/linux/kernfs.h > /dev/null 2>&1` | 3.367 |  | | | 0.002 | 0.001 | 0.002 | 0.002 | 0.002 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 3.561 |  | | | 0.002 | 0.003 | 0.004 | 0.003 | 0.003 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 3.752 |  | | | 0.003 | 0.002 | 0.004 | 0.002 | 0.003 | yes |
| `grep -q define include/linux/kernfs.h && echo found \|\| echo none` | 3.653 |  | | | 0.014 | 0.017 | 0.014 | 0.009 | 0.011 | yes |
| `false \|\| echo fallback` | 2.336 |  | | | 0.002 | 0.002 | 0.001 | 0.001 | 0.002 | yes |
| `true && grep -c define include/linux/kernfs.h` | 2.800 |  | | | 0.017 | 0.018 | 0.013 | 0.009 | 0.013 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 4.081 |  | | | 0.004 | 0.004 | 0.003 | 0.002 | 0.003 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 3.739 |  | | | 0.002 | 0.002 | 0.001 | 0.001 | 0.001 | yes |
| `echo one \⏎two` | 2.249 |  | | | 0.002 | 0.002 | 0.001 | 0.001 | 0.001 | yes |
| `grep -c define include/linux/kernfs.h⏎wc -l include/linux/kernfs.h` | 4.893 |  | | | 0.011 | 0.020 | 0.012 | 0.010 | 0.013 | yes |
| `mkdir -p bench_out/a/b` | 3.736 |  | 0.015 | 0.910 | 0.017 | 1.311 | | 0.863 | 0.980 | yes |
| `touch bench_out/empty.txt` | 2.778 |  | 0.009 | 0.578 | 0.021 | 0.527 | | 0.679 | 0.439 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 2.372 |  | 0.017 | 0.702 | 0.172 | 0.461 | | 0.685 | 0.450 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 2.397 |  | 0.018 | 0.723 | 0.174 | 0.513 | | 1.197 | 0.637 | yes |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 3.797 |  | 0.510 | 8.203 | 0.026 | 0.416 | | 0.483 | 0.593 | yes |
| `cp -r Documentation bench_out/dir_copy` | 187.6 |  | 476.3 | 489.7 | 12.68 | 31.02 | | 18.12 | 19.43 | yes |
| `cp -r include/linux bench_out/work` | 54.01 |  | 172.5 | 239.7 | 3.597 | 8.828 | | 5.284 | 7.780 | yes |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 4.449 |  | 0.545 | 1.468 | 2.069 | 2.098 | | 1.553 | 2.311 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 4.247 |  | 0.478 | 1.352 | 0.516 | 1.623 | | 1.413 | 1.982 | yes |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 128.8 |  | 212.9 | 287.3 | 258.2 | 268.9 | | 248.3 | 342.2 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 390.3 |  | | 3.380 | 1.478 | 17.06 | | 14.39 | 17.75 | yes |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 354.2 |  | | 4.160 | 2.061 | 19.27 | | 16.89 | 17.44 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 3.638 |  | | 0.048 | 0.018 | 0.693 | | 0.579 | 0.570 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 3.538 |  | | 0.048 | 0.018 | 0.693 | | 0.579 | 0.570 | yes |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 4.436 |  | | 0.037 | 0.034 | 0.424 | | 0.409 | 0.549 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 63.38 |  | 7.128 | 7.322 | 4.916 | 5.693 | | 5.117 | 23.62 | yes |
| `grep -rl QDBBENCHMANY .` | 645.5 |  | 3.630 | 2.149 | 1.619 | 1.227 | | 1.100 | 1.785 | yes |
| `grep -rl QDBBENCHONE .` | 925.6 |  | 0.603 | 0.501 | 0.427 | 0.502 | | 0.457 | 2.742 | yes |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | 5.022 |  | | | 0.373 | 1.367 | | 1.368 | 2.805 | yes |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | 4.618 |  | | | 0.507 | 1.309 | | 1.300 | 1.442 | yes |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | 3.894 |  | | | 0.520 | 1.264 | | 1.486 | 1.285 | yes |
| `cat qdb_no_such_file &> bench_out/both.txt` | 3.201 |  | | | 0.041 | 0.448 | | 0.694 | 0.677 | yes |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 2.958 |  | | | 0.037 | 0.426 | | 0.634 | 0.804 | yes |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | 8.069 |  | | | 19.22 | 23.91 | | 22.39 | 20.64 | yes |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 4.112 |  | | | 0.042 | 0.612 | | 0.617 | 0.546 | yes |
| `sed -i '1d' bench_out/joined2.txt` | 7.717 |  | | | 2.956 | 4.735 | | 6.197 | 5.755 | yes |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 3.591 |  | | | 0.031 | 0.024 | | 0.029 | 0.025 | yes |
| `cat bench_out/both.txt` | 2.150 |  | | | 0.016 | 0.011 | | 0.013 | 0.014 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.826 |  | 0.035 | 0.891 | 0.034 | 0.586 | | 0.640 | 0.473 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 3.575 |  | 0.015 | 0.687 | 0.021 | 0.769 | | 0.544 | 0.657 | yes |
| `ls -R bench_out/a` | 2.461 |  | 0.019 | 0.019 | 0.019 | 0.017 | | 0.017 | 0.022 | yes |
| `rm bench_out/a/b/hello.txt` | 2.323 |  | 0.011 | 0.583 | 0.019 | 0.497 | | 0.745 | 0.654 | yes |
| `rmdir bench_out/a/b` | 3.474 |  | 0.015 | 0.543 | 0.017 | 0.439 | | 0.539 | 0.437 | yes |
| `rm -r bench_out/dir_moved` | 124.8 |  | 8.592 | 9.350 | 7.356 | 5.433 | | 8.634 | 6.530 | yes |
| `rm -rf bench_out` | 160.4 |  | 3.696 | 24.41 | 0.812 | 16.54 | | 17.33 | 16.90 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 4, on disk, 512 MB limit | Stage 4, on disk, 1 GB for everything, file cache included |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 212 B | 1457 B | 1457 B | 3419 B | 3419 B | 3419 B | 3419 B | 3849 B |
| `ls -a` | 330 B | 2681 B | 2681 B | 5029 B | 5029 B | 5029 B | 5029 B | 5647 B |
| `ls -R` | 1377.7 KB | 74.3 KB | 74.3 KB | 95.2 KB | 95.2 KB | 95.2 KB | 95.2 KB | 150.9 KB |
| `ls -aR Documentation` | 216.8 KB | 56.6 KB | 56.6 KB | 68.2 KB | 68.2 KB | 68.2 KB | 68.2 KB | 84.0 KB |
| `ls -p Documentation` | 796 B | 4893 B | 4893 B | 7894 B | 7894 B | 7894 B | 7894 B | 9231 B |
| `find .` | 3590.9 KB | 19.2 KB | 19.2 KB | 21.0 KB | 21.0 KB | 21.0 KB | 21.0 KB | 21.0 KB |
| `find . -name '*.h'` | 969.5 KB | 22.7 KB | 22.7 KB | 24.7 KB | 24.7 KB | 24.7 KB | 24.7 KB | 24.7 KB |
| `find . -name '*.c'` | 1233.5 KB | 21.7 KB | 21.7 KB | 23.7 KB | 23.7 KB | 23.7 KB | 23.7 KB | 23.7 KB |
| `find . -type d` | 166.7 KB | 19.5 KB | 19.5 KB | 21.6 KB | 21.6 KB | 21.6 KB | 21.6 KB | 21.6 KB |
| `find . -type f` | 3424.2 KB | 32.8 KB | 32.8 KB | 34.9 KB | 34.9 KB | 34.9 KB | 34.9 KB | 34.9 KB |
| `find . -maxdepth 2 -type f` | 23.5 KB | 32.0 KB | 32.0 KB | 34.6 KB | 34.6 KB | 34.6 KB | 34.6 KB | 34.6 KB |
| `find Documentation -name '*.h' -type f` | 76 B | 1937 B | 1937 B | 4418 B | 4418 B | 4418 B | 4418 B | 4418 B |
| `find . -name 'Makefile'` | 100.1 KB | 26.9 KB | 26.9 KB | 28.9 KB | 28.9 KB | 28.9 KB | 28.9 KB | 28.9 KB |
| `cat include/linux/kernfs.h` | 19.9 KB | 994 B | 994 B | 2407 B | 2407 B | 2407 B | 2407 B | 2407 B |
| `cat -n include/linux/kernfs.h` | 24.4 KB | 26.8 KB | 26.8 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB |
| `head -n 20 include/linux/kernfs.h` | 459 B | 1345 B | 1345 B | 3423 B | 3423 B | 3423 B | 3423 B | 3423 B |
| `tail -n 20 include/linux/kernfs.h` | 478 B | 1364 B | 1364 B | 3442 B | 3442 B | 3442 B | 3442 B | 3442 B |
| `tail -n +100 include/linux/kernfs.h` | 16.9 KB | 1306 B | 1306 B | 2966 B | 2966 B | 2966 B | 2966 B | 2966 B |
| `wc include/linux/kernfs.h` | 41 B | 1002 B | 1002 B | 2602 B | 2602 B | 2602 B | 2602 B | 2602 B |
| `wc -l include/linux/*.h` | 49.6 KB | 237.0 KB | 237.0 KB | 237.7 KB | 237.7 KB | 237.7 KB | 237.7 KB | 237.7 KB |
| `cat include/linux/*.h \| wc -l` | 7 B | 237.2 KB | 237.2 KB | 238.1 KB | 238.1 KB | 238.1 KB | 238.1 KB | 238.1 KB |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 447 B | 4720 B | 4944 B | 7543 B | 7543 B | 7543 B | 7543 B | 7543 B |
| `grep -c define include/linux/kernfs.h` | 2 B | 9470 B | 9470 B | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB |
| `grep -n define include/linux/kernfs.h` | 374 B | 9470 B | 9470 B | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 19.9 KB | 27.0 KB | 27.0 KB | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 19.9 KB | 26.6 KB | 26.6 KB | 30.7 KB | 30.7 KB | 30.7 KB | 30.7 KB | 30.7 KB |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 6 B | 25.8 KB | 26.0 KB | 29.5 KB | 29.5 KB | 29.5 KB | 29.5 KB | 29.5 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB |
| `grep -rn ausgabenspezifisches .` | 116 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB |
| `grep -rl ausgabenspezifisches .` | 32 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB |
| `grep -rn mispredicted .` | 111.0 KB | 90.8 KB | 51.9 KB | 54.1 KB | 54.1 KB | 54.7 KB | 54.1 KB | 54.1 KB |
| `grep -rl mispredicted .` | 4531 B | 90.8 KB | 51.9 KB | 54.1 KB | 54.1 KB | 54.7 KB | 54.1 KB | 54.1 KB |
| `grep -rn separately .` | 103.9 KB | 343.4 KB | 279.6 KB | 281.8 KB | 281.8 KB | 282.2 KB | 281.8 KB | 281.8 KB |
| `grep -rl separately .` | 30.7 KB | 342.9 KB | 279.0 KB | 281.3 KB | 281.3 KB | 281.7 KB | 281.3 KB | 281.3 KB |
| `grep -rn conditions .` | 1360.5 KB | 136.2 KB | 191.1 KB | 193.4 KB | 193.4 KB | 193.4 KB | 193.4 KB | 193.4 KB |
| `grep -rl conditions .` | 389.6 KB | 133.0 KB | 189.4 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB |
| `grep -rn define .` | 821.1 MB | 26.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.2 KB | 30.8 KB |
| `grep -rl define .` | 1867.6 KB | 9418 B | 9418 B | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB |
| `grep -rc separately .` | 3591.0 KB | 361.3 KB | 297.5 KB | 299.8 KB | 299.8 KB | 300.1 KB | 299.8 KB | 299.8 KB |
| `grep -rin MISPREDICTED .` | 135.7 KB | 90.8 KB | 51.8 KB | 54.1 KB | 54.1 KB | 54.7 KB | 54.1 KB | 54.1 KB |
| `grep -ri separately .` | 100.6 KB | 343.3 KB | 279.5 KB | 281.7 KB | 281.7 KB | 282.1 KB | 281.7 KB | 281.7 KB |
| `grep -rn separately Documentation` | 19.9 KB | 35.4 KB | 43.8 KB | 46.1 KB | 46.1 KB | 46.5 KB | 46.1 KB | 46.1 KB |
| `grep -rl conditions Documentation` | 9541 B | 133.0 KB | 189.4 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB |
| `grep -rF 'separately' .` | 99.7 KB | 343.3 KB | 279.5 KB | 281.7 KB | 281.7 KB | 282.1 KB | 281.7 KB | 281.7 KB |
| `grep -rE 'atomic_[a-z]+\(' .` | 1294.9 KB | 945.1 KB | 1037.7 KB | 1040.0 KB | 1040.0 KB | 1040.9 KB | 1040.0 KB | 1040.0 KB |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 109.2 KB | 62.6 KB | 53.6 KB | 64.1 KB | 64.1 KB | 64.1 KB | 64.1 KB | 64.1 KB |
| `grep -rE 'conditions\|separately' .` | 1427.8 KB | 188.8 KB | 195.7 KB | 198.0 KB | 198.0 KB | 200.1 KB | 198.0 KB | 198.0 KB |
| `grep -rE '[0-9]{4,}' .` | 467.5 MB | 28.6 KB | 28.6 KB | 30.8 KB | 30.8 KB | 30.8 KB | 30.8 KB | 30.8 KB |
| `grep -rn '^#include' .` | 24.9 MB | 29.6 KB | 29.6 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB |
| `grep -rE '^(import\|from) ' .` | 116.4 KB | 21.0 KB | 21.0 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB |
| `grep -rin 'return [a-z_]*;$' Documentation` | 24.1 KB | 17.5 KB | 729.5 KB | 716.4 KB | 732.0 KB | 732.0 KB | 732.0 KB | 732.0 KB |
| `grep -rv e drivers/usb` | 9638.5 KB | 21.9 KB | 21.9 KB | 24.1 KB | 24.1 KB | 24.1 KB | 24.1 KB | 24.1 KB |
| `grep -r '\(separately\).*\1' .` | 188 B | 346.3 KB | 282.5 KB | 284.8 KB | 284.8 KB | 285.1 KB | 284.8 KB | 284.8 KB |
| `grep -n mispredicted include/linux/*.h` | 0 B | | 237.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB |
| `grep -l separately include/linux/*.h` | 458 B | | 237.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB |
| `grep -c qdbzzabsentterm include/linux/*.h` | 39.5 KB | | 237.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB |
| `grep -rl conditions . \| wc -l` | 5 B | 133.7 KB | 190.3 KB | 192.6 KB | 192.6 KB | 192.6 KB | 192.6 KB | 192.6 KB |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 6 B | 26.0 KB | 26.2 KB | 29.1 KB | 29.1 KB | 29.1 KB | 29.1 KB | 29.1 KB |
| `find . -name '*.h' \| wc -l` | 6 B | 23.4 KB | 23.6 KB | 25.7 KB | 25.7 KB | 25.7 KB | 25.7 KB | 25.7 KB |
| `find . -type f \| grep -c h` | 6 B | 35.8 KB | 35.9 KB | 39.4 KB | 39.4 KB | 39.4 KB | 39.4 KB | 39.4 KB |
| `grep -rn define . \| head -n 5` | 473 B | 10.3 KB | 10.4 KB | 16.6 KB | 16.6 KB | 16.6 KB | 16.6 KB | 16.6 KB |
| `grep -rn conditions . \| head -n 1` | 107 B | 15.3 KB | 15.4 KB | 23.9 KB | 23.9 KB | 23.9 KB | 23.9 KB | 23.9 KB |
| `find . -type f \| head -n 3` | 47 B | 32.6 KB | 32.8 KB | 35.1 KB | 35.1 KB | 35.1 KB | 35.1 KB | 35.1 KB |
| `cat include/linux/*.h \| head -n 2` | 42 B | 237.3 KB | 237.3 KB | 238.4 KB | 238.4 KB | 238.4 KB | 238.4 KB | 238.4 KB |
| `grep -rno separately .` | 53.2 KB | | | 288.9 KB | 288.9 KB | 289.2 KB | 288.9 KB | 288.9 KB |
| `grep -rw separately .` | 99.7 KB | | | 295.1 KB | 295.1 KB | 295.4 KB | 295.1 KB | 295.1 KB |
| `grep -rnw mispredicted .` | 111.0 KB | | | 65.9 KB | 65.9 KB | 66.5 KB | 65.9 KB | 65.9 KB |
| `grep -rn -C 2 mispredicted .` | 269.5 KB | | | 56.2 KB | 56.2 KB | 56.8 KB | 56.2 KB | 56.2 KB |
| `grep -rn -A 1 -B 1 separately .` | 278.0 KB | | | 16.6 MB | 16.6 MB | 16.6 MB | 16.6 MB | 16.6 MB |
| `grep -rho 'atomic_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 207 B | | | 3082.3 KB | 3082.3 KB | 3083.2 KB | 3082.3 KB | 3082.3 KB |
| `grep -rhow '[a-z]*mispredicted[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 21 B | | | 118.3 KB | 118.3 KB | 118.3 KB | 118.3 KB | 118.3 KB |
| `rg -j 1 qdbzzabsentterm` | 0 B | | | 36.7 KB | 36.7 KB | 36.7 KB | 36.7 KB | 36.7 KB |
| `rg -j 1 ausgabenspezifisches` | 110 B | | | 56.5 KB | 56.5 KB | 56.5 KB | 56.5 KB | 56.5 KB |
| `rg -j 1 mispredicted` | 108.1 KB | | | 166.2 KB | 166.2 KB | 166.7 KB | 166.2 KB | 166.2 KB |
| `rg -j 1 separately` | 97.7 KB | | | 462.5 KB | 462.5 KB | 462.9 KB | 462.5 KB | 462.5 KB |
| `rg -j 1 conditions` | 1304.4 KB | | | 1475.9 KB | 1475.9 KB | 1475.9 KB | 1475.9 KB | 1475.9 KB |
| `rg -j 1 define` | 782.9 MB | | | 1414.4 KB | 1414.4 KB | 1414.4 KB | 1414.4 KB | 1414.4 KB |
| `rg -j 1 -l separately` | 29.1 KB | | | 463.0 KB | 463.0 KB | 463.4 KB | 463.0 KB | 463.0 KB |
| `rg -j 1 -l define` | 1770.4 KB | | | 1414.6 KB | 1414.6 KB | 1414.6 KB | 1414.6 KB | 1414.6 KB |
| `rg -j 1 -c conditions` | 389.6 KB | | | 1476.3 KB | 1476.3 KB | 1476.3 KB | 1476.3 KB | 1476.3 KB |
| `rg -j 1 -i MISPREDICTED` | 131.7 KB | | | 186.3 KB | 186.3 KB | 186.9 KB | 186.3 KB | 186.3 KB |
| `rg -j 1 -w separately` | 97.7 KB | | | 482.8 KB | 482.8 KB | 483.2 KB | 482.8 KB | 482.8 KB |
| `rg -j 1 -F 'separately'` | 97.7 KB | | | 463.0 KB | 463.0 KB | 463.4 KB | 463.0 KB | 463.0 KB |
| `rg -j 1 -n -C 2 mispredicted` | 264.9 KB | | | 167.1 KB | 167.1 KB | 167.7 KB | 167.1 KB | 167.1 KB |
| `rg -j 1 'atomic_[a-z]+\('` | 1261.8 KB | | | 1044.0 KB | 1044.0 KB | 1044.9 KB | 1044.0 KB | 1044.0 KB |
| `rg -j 1 'mispredicted\|ausgabenspezifisches'` | 108.2 KB | | | 193.7 KB | 193.7 KB | 194.2 KB | 193.7 KB | 193.7 KB |
| `rg -j 1 '\bmispredicted\w*'` | 108.1 KB | | | 2592.9 KB | 2592.9 KB | 2592.9 KB | 2592.9 KB | 2592.9 KB |
| `rg -j 1 -t c separately` | 72.4 KB | | | 461.0 KB | 461.0 KB | 461.3 KB | 461.0 KB | 461.0 KB |
| `rg -j 1 -g '*.h' separately` | 15.8 KB | | | 460.9 KB | 460.9 KB | 461.2 KB | 460.9 KB | 460.9 KB |
| `rg -j 1 separately Documentation` | 19.3 KB | | | 202.5 KB | 202.5 KB | 202.8 KB | 202.5 KB | 202.5 KB |
| `rg -j 1 -uu separately` | 97.7 KB | | | 287.8 KB | 287.8 KB | 288.2 KB | 287.8 KB | 287.8 KB |
| `rg -j 1 --sort path mispredicted` | 108.1 KB | | | 166.9 KB | 166.9 KB | 167.5 KB | 166.9 KB | 166.9 KB |
| `rg -j 1 --files \| wc -l` | 6 B | | | 1415.1 KB | 1415.1 KB | 1415.1 KB | 1415.1 KB | 1415.1 KB |
| `rg -j 1 -l conditions \| wc -l` | 5 B | | | 1477.3 KB | 1477.3 KB | 1477.3 KB | 1477.3 KB | 1477.3 KB |
| `ls -ltR` | 4983.7 KB | | | 147.8 KB | 147.8 KB | 147.8 KB | 147.8 KB | 163.7 KB |
| `ls -lt include/linux` | 80.2 KB | | | 116.4 KB | 116.4 KB | 116.4 KB | 116.4 KB | 132.0 KB |
| `ls -ltr Documentation \| tail -n 3` | 138 B | | | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB | 16.2 KB |
| `find . -newer include/linux/kernfs.h` | 705.7 KB | | | 20.1 KB | 20.1 KB | 20.1 KB | 20.1 KB | 20.1 KB |
| `find . -newer include/linux/kernfs.h -name '*.h'` | 200.2 KB | | | 34.3 KB | 34.3 KB | 34.3 KB | 34.3 KB | 34.3 KB |
| `find . -type f -mtime +30` | 0 B | | | 4987 B | 4987 B | 4987 B | 4987 B | 4987 B |
| `du -s .` | 10 B | | | 2705 B | 2705 B | 2705 B | 2705 B | 2705 B |
| `du -sh Documentation` | 18 B | | | 2774 B | 2774 B | 2774 B | 2774 B | 2774 B |
| `du -sb .` | 13 B | | | 2713 B | 2713 B | 2713 B | 2713 B | 2713 B |
| `du -sb Documentation` | 23 B | | | 2774 B | 2774 B | 2774 B | 2774 B | 2774 B |
| `du -b -d 1 Documentation` | 2106 B | | | 8740 B | 8740 B | 8740 B | 8740 B | 8740 B |
| `sort include/linux/kernfs.h` | 19.9 KB | | | 114.8 KB | 114.8 KB | 114.8 KB | 114.8 KB | 114.8 KB |
| `sort -r include/linux/kernfs.h \| head -n 3` | 9 B | | | 116.2 KB | 116.2 KB | 116.2 KB | 116.2 KB | 116.2 KB |
| `sort -u include/linux/kernfs.h \| wc -l` | 4 B | | | 116.0 KB | 116.0 KB | 116.0 KB | 116.0 KB | 116.0 KB |
| `sort -k 2 include/linux/kernfs.h \| tail -n 3` | 97 B | | | 136.5 KB | 136.5 KB | 136.5 KB | 136.5 KB | 136.5 KB |
| `cat include/linux/*.h \| sort \| uniq -c \| sort -rn \| head -n 5` | 52 B | | | 56.9 MB | 56.9 MB | 56.9 MB | 56.9 MB | 56.9 MB |
| `uniq -c include/linux/kernfs.h \| wc -l` | 4 B | | | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB |
| `cut -c 1-20 include/linux/kernfs.h` | 9431 B | | | 22.9 KB | 22.9 KB | 22.9 KB | 22.9 KB | 22.9 KB |
| `cut -d ' ' -f 1 include/linux/kernfs.h \| sort \| uniq \| wc -l` | 3 B | | | 106.7 KB | 106.7 KB | 106.7 KB | 106.7 KB | 106.7 KB |
| `tr a-z A-Z < include/linux/kernfs.h` | 19.9 KB | | | 24.9 KB | 24.9 KB | 24.9 KB | 24.9 KB | 24.9 KB |
| `tr -d '\n' < include/linux/kernfs.h \| wc -c` | 6 B | | | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB |
| `tr -s ' ' < include/linux/kernfs.h \| wc -c` | 6 B | | | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB |
| `nl include/linux/kernfs.h` | 24.4 KB | | | 28.2 KB | 28.2 KB | 28.2 KB | 28.2 KB | 28.2 KB |
| `nl -ba include/linux/kernfs.h \| tail -n 3` | 54 B | | | 54.1 KB | 54.1 KB | 54.1 KB | 54.1 KB | 54.1 KB |
| `seq 1 100000 \| wc -l` | 7 B | | | 135.0 KB | 135.0 KB | 135.0 KB | 135.0 KB | 135.0 KB |
| `seq -w 1 1000 \| tail -n 2` | 10 B | | | 17.6 KB | 17.6 KB | 17.6 KB | 17.6 KB | 17.6 KB |
| `basename include/linux/kernfs.h` | 9 B | | | 2444 B | 2444 B | 2444 B | 2444 B | 2444 B |
| `basename -s .h include/linux/kernfs.h` | 7 B | | | 3098 B | 3098 B | 3098 B | 3098 B | 3098 B |
| `dirname include/linux/kernfs.h` | 14 B | | | 2453 B | 2453 B | 2453 B | 2453 B | 2453 B |
| `printf '%s\n' a b c` | 6 B | | | 3325 B | 3325 B | 3325 B | 3325 B | 3325 B |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 18 B | | | 3422 B | 3422 B | 3422 B | 3422 B | 3422 B |
| `test -f include/linux/kernfs.h && echo yes` | 4 B | | | 3547 B | 3547 B | 3547 B | 3547 B | 3547 B |
| `[ -d Documentation ] && echo folder` | 7 B | | | 3788 B | 3788 B | 3788 B | 3788 B | 3788 B |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 35 B | | | 3492 B | 3492 B | 3492 B | 3492 B | 3492 B |
| `echo a b c \| xargs -n 1 echo` | 6 B | | | 4674 B | 4674 B | 4674 B | 4674 B | 4674 B |
| `find drivers/usb -name '*.h' \| xargs cat \| wc -l` | 6 B | | | 109.4 KB | 109.4 KB | 109.4 KB | 109.4 KB | 109.4 KB |
| `find . -name '*.h' \| xargs grep -l separately \| wc -l` | 4 B | | | 6391.8 KB | 6391.8 KB | 6391.8 KB | 6391.8 KB | 6391.8 KB |
| `echo {a,b,c}{1..3}` | 27 B | | | 4485 B | 4485 B | 4485 B | 4485 B | 4485 B |
| `echo {1..20..3} {a..e}` | 28 B | | | 3967 B | 3967 B | 3967 B | 3967 B | 3967 B |
| `echo $(wc -l < include/linux/kernfs.h) `basename include/linux/kernfs.h`` | 13 B | | | 6361 B | 6361 B | 6361 B | 6361 B | 6361 B |
| `echo "first line: $(head -n 1 include/linux/kernfs.h)"` | 56 B | | | 4984 B | 4984 B | 4984 B | 4984 B | 4984 B |
| `wc -l < include/linux/kernfs.h` | 4 B | | | 3332 B | 3332 B | 3332 B | 3332 B | 3332 B |
| `cat include/linux/kernfs.h > /dev/null 2>&1` | 0 B | | | 3433 B | 3433 B | 3433 B | 3433 B | 3433 B |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 6 B | | | 4158 B | 4158 B | 4158 B | 4158 B | 4158 B |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2 B | | | 4072 B | 4072 B | 4072 B | 4072 B | 4072 B |
| `grep -q define include/linux/kernfs.h && echo found \|\| echo none` | 6 B | | | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `false \|\| echo fallback` | 9 B | | | 2861 B | 2861 B | 2861 B | 2861 B | 2861 B |
| `true && grep -c define include/linux/kernfs.h` | 2 B | | | 15.9 KB | 15.9 KB | 15.9 KB | 15.9 KB | 15.9 KB |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 14 B | | | 4178 B | 4178 B | 4178 B | 4178 B | 4178 B |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2 B | | | 3130 B | 3130 B | 3130 B | 3130 B | 3130 B |
| `echo one \⏎two` | 8 B | | | 2591 B | 2591 B | 2591 B | 2591 B | 2591 B |
| `grep -c define include/linux/kernfs.h⏎wc -l include/linux/kernfs.h` | 29 B | | | 16.5 KB | 16.5 KB | 16.5 KB | 16.5 KB | 16.5 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1459 B | 1747 B | 3711 B | 3711 B | | 3711 B | 3711 B |
| `touch bench_out/empty.txt` | 0 B | 979 B | 979 B | 2566 B | 2566 B | | 2566 B | 2566 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1242 B | 1292 B | 3665 B | 3661 B | | 3661 B | 3661 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1246 B | 1246 B | 3485 B | 3473 B | | 3473 B | 3473 B |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 0 B | 26.0 KB | 283.2 KB | 3270 B | 3270 B | | 3270 B | 3270 B |
| `cp -r Documentation bench_out/dir_copy` | 0 B | 59.4 MB | 21.7 MB | 4208.5 KB | 6328.2 KB | | 6328.2 KB | 6328.2 KB |
| `cp -r include/linux bench_out/work` | 0 B | 22.2 MB | 13.6 MB | 1155.8 KB | 1403.4 KB | | 1403.4 KB | 1403.4 KB |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 27.6 KB | 47.8 KB | 51.1 KB | 293.3 KB | | 293.3 KB | 293.3 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 25.8 KB | 47.6 KB | 51.2 KB | 51.2 KB | | 51.2 KB | 51.2 KB |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 0 B | 6764.6 KB | 6254.3 KB | 11.6 MB | 5427.3 KB | | 5427.3 KB | 5427.3 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 0 B | | 250.1 KB | 252.1 KB | 252.1 KB | | 252.1 KB | 252.1 KB |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 0 B | | 248.6 KB | 250.6 KB | 250.6 KB | | 250.6 KB | 250.6 KB |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 B | | 20.8 KB | 2850 B | 2850 B | | 2850 B | 2850 B |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 B | | 20.8 KB | 2850 B | 2850 B | | 2850 B | 2850 B |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 0 B | | 21.0 KB | 23.4 KB | 23.4 KB | | 23.4 KB | 23.4 KB |
| `grep -rc QDBBENCHMANY bench_out` | 687.0 KB | 191.9 KB | 206.0 KB | 208.2 KB | 208.2 KB | | 208.2 KB | 208.2 KB |
| `grep -rl QDBBENCHMANY .` | 39.7 KB | 164.6 KB | 178.7 KB | 180.9 KB | 180.9 KB | | 180.9 KB | 180.9 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | | 24.8 KB | 24.8 KB |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | 4 B | | | 44.6 KB | 44.6 KB | | 44.6 KB | 44.6 KB |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | 0 B | | | 135.6 KB | 135.6 KB | | 135.6 KB | 135.6 KB |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | 0 B | | | 134.4 KB | 134.4 KB | | 134.4 KB | 134.4 KB |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 B | | | 3783 B | 3783 B | | 3783 B | 3783 B |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 0 B | | | 3781 B | 3714 B | | 3714 B | 3714 B |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | 0 B | | | 2433.6 KB | 2429.2 KB | | 2429.2 KB | 2429.2 KB |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 B | | | 8382 B | 8382 B | | 8382 B | 8382 B |
| `sed -i '1d' bench_out/joined2.txt` | 0 B | | | 151.1 KB | 151.1 KB | | 151.1 KB | 151.1 KB |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 128 B | | | 4183 B | 4183 B | | 4183 B | 4183 B |
| `cat bench_out/both.txt` | 98 B | | | 2497 B | 2497 B | | 2497 B | 2497 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1191 B | 1191 B | 2900 B | 2900 B | | 2900 B | 2900 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1288 B | 1288 B | 3250 B | 3250 B | | 3250 B | 3250 B |
| `ls -R bench_out/a` | 41 B | 1089 B | 1089 B | 3092 B | 3092 B | | 3092 B | 3129 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 997 B | 997 B | 2504 B | 2504 B | | 2504 B | 2504 B |
| `rmdir bench_out/a/b` | 0 B | 945 B | 945 B | 2383 B | 2383 B | | 2383 B | 2383 B |
| `rm -r bench_out/dir_moved` | 0 B | 1451 B | 1355 B | 2707 B | 2707 B | | 2707 B | 2707 B |
| `rm -rf bench_out` | 0 B | 12.4 KB | 12.2 KB | 14.1 KB | 14.1 KB | | 14.1 KB | 14.1 KB |

## Search terms and paths

| Placeholder | Value | Measured |
|---|---|---|
| ABSENT | `qdbzzabsentterm` | 0 of 83405 files, 0.00% |
| COMMON | `define` | 48558 of 83405 files, 58.22% |
| DIR | `Documentation` | holds 9699 of 83405 files, 11.63% |
| EXT | `h` | 24213 files in the whole dataset |
| EXT2 | `c` | 33556 files in the whole dataset |
| FEW | `separately` | 834 of 83405 files, 1.00% |
| FILE | `include/linux/kernfs.h` | 19937 bytes |
| GLOB | `include/linux/*.h` | 1432 files |
| GLOBDIR | `include/linux` | the folder of GLOB |
| ONE | `ausgabenspezifisches` | 1 of 83405 files, 0.00% |
| PREFIX | `atomic` | PREFIX_[a-z]+\( matches in 3036 of 83405 files, 3.64% |
| RARE | `mispredicted` | 84 of 83405 files, 0.10% |
| RAREUP | `MISPREDICTED` | the RARE term in capitals, for -i |
| SMALLDIR | `drivers/usb` | holds 851 of 83405 files, 1.02% |
| SOME | `conditions` | 8460 of 83405 files, 10.14% |
