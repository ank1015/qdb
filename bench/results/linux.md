# Benchmark: linux

- Data: Linux 6.8 source tree, from the Ubuntu source package linux_6.8.0.orig.tar.gz (sha256 26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8). Skipped on import: 64 symbolic links, 6 files that are not valid text, 0 other
- Size: 83405 files in 5451 folders, 1396.9 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 4, on disk, 1 GB for everything, file cache included), and include starting bash and the tool (1.405 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 1374.6 | 8146.1 | 1568.1 MB | 1578.0 MB | 0.280 | yes |
| Stage 3, on disk, settled | on | 13807.8 | 13807.8 | 21.7 MB | 967.2 MB | 0.310 | yes |
| Stage 4, in memory | on | 2456.6 | 9899.6 | 1667.4 MB | 1602.2 MB | 0.558 | yes |
| Stage 4, on disk, settled | on | 13848.4 | 13848.4 | 22.4 MB | 964.5 MB | 0.540 | yes |
| Stage 4, on disk, after the writes | on | 13848.4 | 13848.4 | 22.4 MB | 964.5 MB | 0.540 | yes |
| Stage 4, on disk, 512 MB limit | on | 14739.0 | 14739.0 | 22.4 MB | 966.5 MB | 0.289 | yes |
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

- Built in 7443.0 ms while loading.
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
| `ls` | 2.866 |  | 0.001 | 0.001 | 0.004 | 0.004 | 0.005 | 0.003 | 0.030 | yes |
| `ls -a` | 2.949 |  | 0.004 | 0.004 | 0.004 | 0.006 | 0.006 | 0.004 | 0.004 | yes |
| `ls -R` | 78.71 |  | 8.025 | 3.303 | 16.60 | 9.463 | 9.746 | 14.14 | 11.94 | yes |
| `ls -aR Documentation` | 14.03 |  | 0.747 | 0.861 | 1.336 | 1.700 | 1.184 | 1.139 | 1.219 | yes |
| `ls -p Documentation` | 2.869 |  | 0.004 | 0.004 | 0.007 | 0.010 | 0.011 | 0.005 | 0.011 | yes |
| `find .` | 52.86 |  | 20.82 | 10.16 | 17.62 | 7.720 | 9.901 | 11.01 | 8.976 | yes |
| `find . -name '*.h'` | 58.92 |  | 25.53 | 10.22 | 16.60 | 10.03 | 10.28 | 13.23 | 13.05 | yes |
| `find . -name '*.c'` | 62.78 |  | 28.25 | 10.50 | 19.16 | 10.22 | 13.70 | 11.64 | 11.39 | yes |
| `find . -type d` | 62.65 |  | 25.02 | 6.396 | 14.61 | 6.624 | 7.292 | 10.70 | 7.901 | yes |
| `find . -type f` | 70.50 |  | 23.39 | 11.97 | 15.95 | 9.729 | 13.99 | 9.918 | 9.119 | yes |
| `find . -maxdepth 2 -type f` | 6.367 |  | 0.114 | 0.109 | 0.155 | 0.137 | 0.126 | 0.135 | 0.129 | yes |
| `find Documentation -name '*.h' -type f` | 10.35 |  | 1.770 | 1.071 | 1.844 | 1.228 | 1.071 | 1.704 | 1.088 | yes |
| `find . -name 'Makefile'` | 70.45 |  | 27.58 | 7.526 | 18.50 | 10.28 | 8.004 | 7.454 | 10.33 | yes |
| `cat include/linux/kernfs.h` | 2.486 |  | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `cat -n include/linux/kernfs.h` | 2.037 |  | 0.057 | 0.055 | 0.070 | 0.048 | 0.055 | 0.047 | 0.055 | yes |
| `head -n 20 include/linux/kernfs.h` | 2.359 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.002 | 0.002 | yes |
| `tail -n 20 include/linux/kernfs.h` | 2.344 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.002 | 0.002 | yes |
| `tail -n +100 include/linux/kernfs.h` | 2.017 |  | 0.003 | 0.003 | 0.005 | 0.004 | 0.009 | 0.006 | 0.007 | yes |
| `wc include/linux/kernfs.h` | 2.647 |  | 0.024 | 0.023 | 0.025 | 0.024 | 0.037 | 0.025 | 0.035 | yes |
| `wc -l include/linux/*.h` | 9.783 |  | 28.62 | 0.885 | 0.796 | 0.917 | 1.576 | 0.857 | 0.896 | yes |
| `cat include/linux/*.h \| wc -l` | 35.55 |  | 26.33 | 24.59 | 1.471 | 1.663 | 1.809 | 2.091 | 2.143 | yes |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 3.299 |  | 0.005 | 0.006 | 0.004 | 0.005 | 0.005 | 0.006 | 0.004 | yes |
| `grep -c define include/linux/kernfs.h` | 2.422 |  | 0.006 | 0.009 | 0.013 | 0.015 | 0.010 | 0.015 | 0.043 | yes |
| `grep -n define include/linux/kernfs.h` | 2.394 |  | 0.006 | 0.009 | 0.011 | 0.015 | 0.013 | 0.016 | 0.010 | yes |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 2.327 |  | 0.249 | 0.226 | 0.013 | 0.012 | 0.014 | 0.019 | 0.011 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 3.370 |  | 0.588 | 0.560 | 0.432 | 0.510 | 0.452 | 0.480 | 0.492 | yes |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 2.765 |  | 0.152 | 0.120 | 0.064 | 0.069 | 0.075 | 0.076 | 0.062 | yes |
| `grep -rn qdbzzabsentterm .` | 1217.8 | 696.6 | 0.012 | 0.020 | 0.022 | 0.027 | 0.022 | 0.023 | 0.102 | yes |
| `grep -rl qdbzzabsentterm .` | 822.4 | 566.7 | 0.011 | 0.021 | 0.021 | 0.021 | 0.040 | 0.023 | 0.037 | yes |
| `grep -rn ausgabenspezifisches .` | 1216.0 | 650.7 | 0.098 | 0.125 | 0.138 | 0.138 | 0.201 | 0.110 | 0.284 | yes |
| `grep -rl ausgabenspezifisches .` | 761.3 | 577.0 | 0.095 | 0.100 | 0.109 | 0.218 | 0.128 | 0.183 | 0.120 | yes |
| `grep -rn mispredicted .` | 1284.7 | 596.2 | 17.54 | 2.475 | 1.775 | 1.727 | 1.857 | 1.566 | 3.261 | yes |
| `grep -rl mispredicted .` | 978.3 | 604.1 | 17.60 | 1.544 | 1.667 | 1.688 | 2.407 | 1.225 | 2.483 | yes |
| `grep -rn separately .` | 1032.5 | 544.7 | 24.09 | 14.82 | 18.55 | 14.49 | 15.80 | 15.80 | 50.00 | yes |
| `grep -rl separately .` | 552.4 | 498.4 | 18.81 | 11.89 | 11.42 | 11.77 | 11.21 | 11.62 | 11.74 | yes |
| `grep -rn conditions .` | 1268.9 | 616.4 | 129.9 | 62.60 | 65.58 | 60.83 | 66.01 | 62.86 | 321.4 | yes |
| `grep -rl conditions .` | 790.9 | 538.5 | 65.31 | 40.80 | 53.57 | 42.95 | 39.12 | 40.45 | 37.92 | yes |
| `grep -rn define .` | 2423.1 | 2694.2 | 460.7 | 518.8 | 614.7 | 513.6 | 613.5 | 476.5 | 2157.3 | yes |
| `grep -rl define .` | 623.4 | 553.3 | 100.9 | 59.67 | 63.16 | 54.55 | 57.43 | 63.76 | 588.4 | yes |
| `grep -rc separately .` | 640.0 | 515.2 | 44.65 | 17.95 | 21.69 | 19.86 | 20.69 | 21.49 | 18.51 | yes |
| `grep -rin MISPREDICTED .` | 1495.6 | 760.4 | 21.12 | 2.271 | 2.735 | 2.262 | 2.353 | 1.785 | 3.418 | yes |
| `grep -ri separately .` | 862.1 | 719.9 | 28.93 | 16.61 | 20.22 | 15.56 | 17.54 | 15.90 | 19.32 | yes |
| `grep -rn separately Documentation` | 72.24 | 53.74 | 2.473 | 1.884 | 2.481 | 1.651 | 2.061 | 1.697 | 4.498 | yes |
| `grep -rl conditions Documentation` | 58.70 | 54.15 | 3.711 | 3.362 | 4.642 | 3.534 | 3.146 | 3.614 | 7.190 | yes |
| `grep -rF 'separately' .` | 590.0 | 622.1 | 24.42 | 14.81 | 14.06 | 20.48 | 14.67 | 16.88 | 15.08 | yes |
| `grep -rE 'atomic_[a-z]+\(' .` | 1308.0 | 622.4 | 74.42 | 51.84 | 65.38 | 51.41 | 54.79 | 55.23 | 284.3 | yes |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 1716.4 | 669.0 | 29.62 | 2.512 | 3.436 | 3.009 | 2.284 | 2.545 | 6.965 | yes |
| `grep -rE 'conditions\|separately' .` | 2021.5 | 735.8 | 180.1 | 75.05 | 80.28 | 85.46 | 85.30 | 80.99 | 238.5 | yes |
| `grep -rE '[0-9]{4,}' .` | 2135.1 | 2958.9 | 2025.8 | 2153.3 | 2564.9 | 2089.3 | 2073.9 | 2159.6 | 2817.3 | yes |
| `grep -rn '^#include' .` | 1411.3 | 862.8 | 244.7 | 217.7 | 229.1 | 232.2 | 227.9 | 220.7 | 983.1 | yes |
| `grep -rE '^(import\|from) ' .` | 1094.8 | 641.4 | 315.7 | 263.9 | 312.9 | 281.3 | 279.4 | 300.8 | 1002.8 | yes |
| `grep -rin 'return [a-z_]*;$' Documentation` | 94.93 | 60.85 | 11.32 | 4.304 | 6.282 | 7.140 | 4.736 | 3.663 | 57.36 | yes |
| `grep -rv e drivers/usb` | 40.44 | 48.32 | 17.91 | 18.05 | 24.20 | 25.02 | 18.14 | 22.29 | 19.97 | yes |
| `grep -r '\(separately\).*\1' .` | 655.8 |  | 24.81 | 15.77 | 20.76 | 22.13 | 17.22 | 18.27 | 73.82 | yes |
| `grep -n mispredicted include/linux/*.h` | 21.68 |  | | 0.998 | 1.318 | 0.988 | 1.841 | 1.902 | 1.414 | yes |
| `grep -l separately include/linux/*.h` | 13.77 |  | | 1.514 | 1.659 | 1.446 | 1.345 | 1.357 | 1.270 | yes |
| `grep -c qdbzzabsentterm include/linux/*.h` | 11.66 |  | | 0.595 | 0.554 | 0.527 | 1.361 | 0.480 | 0.895 | yes |
| `grep -rl conditions . \| wc -l` | 693.6 |  | 58.03 | 42.95 | 53.92 | 40.79 | 42.73 | 40.75 | 193.2 | yes |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1273.8 |  | 342.3 | 315.0 | 342.6 | 348.9 | 331.1 | 336.0 | 890.4 | yes |
| `find . -name '*.h' \| wc -l` | 90.77 |  | 23.83 | 11.43 | 17.15 | 10.72 | 10.12 | 11.26 | 16.56 | yes |
| `find . -type f \| grep -c h` | 79.19 |  | 27.69 | 13.79 | 28.20 | 11.07 | 14.14 | 14.71 | 17.81 | yes |
| `grep -rn define . \| head -n 5` | 6.430 |  | 0.019 | 0.024 | 0.025 | 0.026 | 0.028 | 0.046 | 0.070 | order differs |
| `grep -rn conditions . \| head -n 1` | 11.18 |  | 0.027 | 0.040 | 0.043 | 0.041 | 0.079 | 0.073 | 0.133 | order differs |
| `find . -type f \| head -n 3` | 4.754 |  | 0.058 | 0.093 | 0.055 | 0.060 | 0.065 | 0.082 | 0.054 | order differs |
| `cat include/linux/*.h \| head -n 2` | 4.070 |  | 0.346 | 0.335 | 0.286 | 0.344 | 0.507 | 0.295 | 0.289 | yes |
| `grep -rno separately .` | 1036.9 | 648.7 | | | 19.67 | 16.57 | 16.38 | 16.47 | 99.75 | yes |
| `grep -rw separately .` | 651.7 | 590.4 | | | 14.94 | 14.52 | 19.47 | 15.09 | 16.39 | yes |
| `grep -rnw mispredicted .` | 1386.9 | 626.9 | | | 2.691 | 2.066 | 2.706 | 1.970 | 4.685 | yes |
| `grep -rn -C 2 mispredicted .` | 1359.6 | 2467.5 | | | 2.284 | 2.362 | 2.847 | 3.288 | 4.147 | yes |
| `grep -rn -A 1 -B 1 separately .` | 1042.7 | 2579.1 | | | 26.68 | 26.40 | 30.78 | 23.53 | 225.7 | yes |
| `grep -rho 'atomic_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 1242.0 |  | | | 80.25 | 88.05 | 119.9 | 88.80 | 431.1 | yes |
| `grep -rhow '[a-z]*mispredicted[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 892.7 |  | | | 3.865 | 7.437 | 3.523 | 5.022 | 5.669 | yes |
| `rg -j 1 qdbzzabsentterm` | 619.4 |  | | | 0.065 | 0.058 | 0.064 | 0.058 | 0.075 | yes |
| `rg -j 1 ausgabenspezifisches` | 672.8 |  | | | 0.101 | 0.165 | 0.100 | 0.110 | 0.244 | yes |
| `rg -j 1 mispredicted` | 606.9 |  | | | 2.326 | 2.184 | 2.831 | 1.974 | 3.054 | yes |
| `rg -j 1 separately` | 678.2 |  | | | 19.45 | 20.61 | 18.62 | 18.58 | 18.47 | yes |
| `rg -j 1 conditions` | 657.3 |  | | | 144.1 | 129.2 | 132.9 | 130.8 | 293.0 | yes |
| `rg -j 1 define` | 1871.5 |  | | | 646.9 | 530.2 | 489.0 | 427.1 | 2370.2 | yes |
| `rg -j 1 -l separately` | 632.0 |  | | | 16.93 | 16.05 | 14.85 | 21.22 | 124.2 | yes |
| `rg -j 1 -l define` | 756.8 |  | | | 143.3 | 146.7 | 142.0 | 120.0 | 720.8 | yes |
| `rg -j 1 -c conditions` | 702.8 |  | | | 127.5 | 128.8 | 144.0 | 139.0 | 341.8 | yes |
| `rg -j 1 -i MISPREDICTED` | 766.6 |  | | | 20.67 | 26.21 | 18.73 | 20.46 | 27.02 | yes |
| `rg -j 1 -w separately` | 659.8 |  | | | 27.52 | 35.36 | 27.29 | 25.54 | 121.4 | yes |
| `rg -j 1 -F 'separately'` | 719.9 |  | | | 17.28 | 18.06 | 18.55 | 18.01 | 18.92 | yes |
| `rg -j 1 -n -C 2 mispredicted` | 668.7 |  | | | 2.982 | 2.673 | 3.172 | 3.083 | 4.415 | yes |
| `rg -j 1 'atomic_[a-z]+\('` | 837.9 |  | | | 74.42 | 68.85 | 70.89 | 64.45 | 243.8 | yes |
| `rg -j 1 'mispredicted\|ausgabenspezifisches'` | 837.7 |  | | | 2.603 | 2.781 | 2.482 | 3.300 | 4.016 | yes |
| `rg -j 1 '\bmispredicted\w*'` | 669.0 |  | | | 7.375 | 7.790 | 7.135 | 8.173 | 7.881 | yes |
| `rg -j 1 -t c separately` | 649.0 |  | | | 19.54 | 15.74 | 21.06 | 17.86 | 16.91 | yes |
| `rg -j 1 -g '*.h' separately` | 337.3 |  | | | 5.215 | 5.405 | 6.848 | 5.093 | 6.451 | yes |
| `rg -j 1 separately Documentation` | 63.24 |  | | | 9.728 | 11.46 | 10.97 | 9.928 | 10.84 | yes |
| `rg -j 1 -uu separately` | 679.8 |  | | | 14.69 | 14.28 | 13.43 | 12.23 | 14.53 | yes |
| `rg -j 1 --sort path mispredicted` | 720.7 |  | | | 1.983 | 1.935 | 3.246 | 2.193 | 3.575 | yes |
| `rg -j 1 --files \| wc -l` | 192.8 |  | | | 62.73 | 62.56 | 63.24 | 57.24 | 92.41 | yes |
| `rg -j 1 -l conditions \| wc -l` | 632.1 |  | | | 140.9 | 104.4 | 122.3 | 90.07 | 113.9 | yes |
| `ls -ltR` | 310.9 |  | | | 49.93 | 56.55 | 48.94 | 50.58 | 55.93 | yes |
| `ls -lt include/linux` | 9.586 |  | | | 0.814 | 1.187 | 0.631 | 0.819 | 0.868 | yes |
| `ls -ltr Documentation \| tail -n 3` | 4.016 |  | | | 0.053 | 0.127 | 0.060 | 0.050 | 0.065 | yes |
| `find . -newer include/linux/kernfs.h` | 156.8 |  | | | 17.66 | 13.27 | 9.673 | 8.694 | 8.893 | yes |
| `find . -newer include/linux/kernfs.h -name '*.h'` | 102.7 |  | | | 20.08 | 8.477 | 14.79 | 8.027 | 9.991 | yes |
| `find . -type f -mtime +30` | 171.3 |  | | | 12.34 | 13.54 | 13.05 | 8.090 | 13.65 | yes |
| `du -s .` | 188.8 |  | | | 0.002 | 0.001 | 0.001 | 0.001 | 0.016 | not compared |
| `du -sh Documentation` | 20.91 |  | | | 0.001 | 0.001 | 0.002 | 0.001 | 0.002 | not compared |
| `du -sb .` | 146.6 |  | | | 0.002 | 0.002 | 0.001 | 0.001 | 0.013 | yes |
| `du -sb Documentation` | 18.72 |  | | | 0.002 | 0.002 | 0.002 | 0.001 | 0.001 | yes |
| `du -b -d 1 Documentation` | 19.63 |  | | | 0.007 | 0.007 | 0.016 | 0.013 | 0.009 | yes |
| `sort include/linux/kernfs.h` | 3.122 |  | | | 0.041 | 0.041 | 0.068 | 0.069 | 0.046 | yes |
| `sort -r include/linux/kernfs.h \| head -n 3` | 3.630 |  | | | 0.044 | 0.062 | 0.047 | 0.075 | 0.074 | yes |
| `sort -u include/linux/kernfs.h \| wc -l` | 4.206 |  | | | 0.057 | 0.069 | 0.045 | 0.076 | 0.075 | yes |
| `sort -k 2 include/linux/kernfs.h \| tail -n 3` | 4.298 |  | | | 0.061 | 0.062 | 0.058 | 0.104 | 0.099 | yes |
| `cat include/linux/*.h \| sort \| uniq -c \| sort -rn \| head -n 5` | 270.1 |  | | | 135.9 | 173.4 | 170.2 | 157.0 | 177.0 | yes |
| `uniq -c include/linux/kernfs.h \| wc -l` | 3.708 |  | | | 0.025 | 0.026 | 0.028 | 0.041 | 0.027 | yes |
| `cut -c 1-20 include/linux/kernfs.h` | 3.044 |  | | | 0.030 | 0.030 | 0.021 | 0.036 | 0.019 | yes |
| `cut -d ' ' -f 1 include/linux/kernfs.h \| sort \| uniq \| wc -l` | 4.735 |  | | | 0.111 | 0.084 | 0.078 | 0.132 | 0.077 | yes |
| `tr a-z A-Z < include/linux/kernfs.h` | 3.561 |  | | | 0.004 | 0.003 | 0.004 | 0.006 | 0.003 | yes |
| `tr -d '\n' < include/linux/kernfs.h \| wc -c` | 3.340 |  | | | 0.009 | 0.010 | 0.014 | 0.015 | 0.011 | yes |
| `tr -s ' ' < include/linux/kernfs.h \| wc -c` | 3.354 |  | | | 0.021 | 0.014 | 0.025 | 0.026 | 0.013 | yes |
| `nl include/linux/kernfs.h` | 2.264 |  | | | 0.048 | 0.075 | 0.082 | 0.080 | 0.046 | yes |
| `nl -ba include/linux/kernfs.h \| tail -n 3` | 3.390 |  | | | 0.054 | 0.083 | 0.096 | 0.090 | 0.051 | yes |
| `seq 1 100000 \| wc -l` | 4.455 |  | | | 2.531 | 3.703 | 3.224 | 3.754 | 3.481 | yes |
| `seq -w 1 1000 \| tail -n 2` | 3.544 |  | | | 0.023 | 0.047 | 0.047 | 0.024 | 0.034 | yes |
| `basename include/linux/kernfs.h` | 2.764 |  | | | 0.001 | 0.002 | 0.001 | 0.001 | 0.001 | yes |
| `basename -s .h include/linux/kernfs.h` | 2.219 |  | | | 0.002 | 0.002 | 0.002 | 0.001 | 0.001 | yes |
| `dirname include/linux/kernfs.h` | 1.972 |  | | | 0.001 | 0.001 | 0.002 | 0.001 | 0.001 | yes |
| `printf '%s\n' a b c` | 1.291 |  | | | 0.002 | 0.003 | 0.004 | 0.002 | 0.002 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 1.516 |  | | | 0.002 | 0.004 | 0.004 | 0.003 | 0.002 | yes |
| `test -f include/linux/kernfs.h && echo yes` | 1.889 |  | | | 0.004 | 0.002 | 0.003 | 0.002 | 0.002 | yes |
| `[ -d Documentation ] && echo folder` | 1.756 |  | | | 0.004 | 0.005 | 0.004 | 0.002 | 0.002 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 1.960 |  | | | 0.005 | 0.003 | 0.008 | 0.003 | 0.004 | yes |
| `echo a b c \| xargs -n 1 echo` | 6.305 |  | | | 0.003 | 0.004 | 0.010 | 0.004 | 0.003 | yes |
| `find drivers/usb -name '*.h' \| xargs cat \| wc -l` | 9.505 |  | | | 0.436 | 0.437 | 0.563 | 0.374 | 0.657 | yes |
| `find . -name '*.h' \| xargs grep -l separately \| wc -l` | 231.7 |  | | | 48.33 | 40.12 | 45.10 | 37.16 | 74.50 | yes |
| `echo {a,b,c}{1..3}` | 2.087 |  | | | 0.003 | 0.003 | 0.007 | 0.004 | 0.004 | yes |
| `echo {1..20..3} {a..e}` | 1.972 |  | | | 0.005 | 0.005 | 0.008 | 0.005 | 0.005 | yes |
| `echo $(wc -l < include/linux/kernfs.h) `basename include/linux/kernfs.h`` | 5.184 |  | | | 0.006 | 0.008 | 0.007 | 0.006 | 0.006 | yes |
| `echo "first line: $(head -n 1 include/linux/kernfs.h)"` | 2.743 |  | | | 0.004 | 0.006 | 0.005 | 0.004 | 0.006 | yes |
| `wc -l < include/linux/kernfs.h` | 3.047 |  | | | 0.002 | 0.003 | 0.002 | 0.002 | 0.002 | yes |
| `cat include/linux/kernfs.h > /dev/null 2>&1` | 2.805 |  | | | 0.001 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 2.911 |  | | | 0.003 | 0.006 | 0.004 | 0.003 | 0.003 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2.976 |  | | | 0.003 | 0.007 | 0.004 | 0.002 | 0.003 | yes |
| `grep -q define include/linux/kernfs.h && echo found \|\| echo none` | 3.076 |  | | | 0.011 | 0.021 | 0.018 | 0.014 | 0.011 | yes |
| `false \|\| echo fallback` | 1.329 |  | | | 0.001 | 0.002 | 0.002 | 0.001 | 0.002 | yes |
| `true && grep -c define include/linux/kernfs.h` | 2.166 |  | | | 0.011 | 0.015 | 0.019 | 0.012 | 0.013 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 2.916 |  | | | 0.003 | 0.004 | 0.006 | 0.003 | 0.003 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2.521 |  | | | 0.001 | 0.002 | 0.002 | 0.001 | 0.001 | yes |
| `echo one \⏎two` | 1.286 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `grep -c define include/linux/kernfs.h⏎wc -l include/linux/kernfs.h` | 3.634 |  | | | 0.012 | 0.021 | 0.020 | 0.014 | 0.013 | yes |
| `mkdir -p bench_out/a/b` | 2.203 |  | 0.015 | 0.910 | 0.043 | 1.098 | | 0.838 | 0.980 | yes |
| `touch bench_out/empty.txt` | 1.949 |  | 0.009 | 0.578 | 0.018 | 0.685 | | 0.612 | 0.439 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.222 |  | 0.017 | 0.702 | 0.143 | 0.693 | | 0.698 | 0.450 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.410 |  | 0.018 | 0.723 | 0.143 | 0.974 | | 0.624 | 0.637 | yes |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 2.413 |  | 0.510 | 8.203 | 0.025 | 0.672 | | 0.483 | 0.593 | yes |
| `cp -r Documentation bench_out/dir_copy` | 182.3 |  | 476.3 | 489.7 | 13.72 | 18.85 | | 19.01 | 19.43 | yes |
| `cp -r include/linux bench_out/work` | 55.29 |  | 172.5 | 239.7 | 3.420 | 5.274 | | 8.332 | 7.780 | yes |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 3.551 |  | 0.545 | 1.468 | 1.541 | 1.493 | | 1.823 | 2.311 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.680 |  | 0.478 | 1.352 | 0.654 | 1.442 | | 1.685 | 1.982 | yes |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 107.9 |  | 212.9 | 287.3 | 240.6 | 340.2 | | 408.9 | 342.2 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 306.8 |  | | 3.380 | 1.942 | 16.94 | | 17.66 | 17.75 | yes |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 323.5 |  | | 4.160 | 2.700 | 17.04 | | 17.08 | 17.44 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 3.201 |  | | 0.048 | 0.022 | 0.599 | | 0.617 | 0.570 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 5.143 |  | | 0.048 | 0.022 | 0.599 | | 0.617 | 0.570 | yes |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 3.896 |  | | 0.037 | 0.034 | 0.831 | | 0.450 | 0.549 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 81.84 |  | 7.128 | 7.322 | 4.903 | 7.400 | | 6.197 | 23.62 | yes |
| `grep -rl QDBBENCHMANY .` | 651.5 |  | 3.630 | 2.149 | 2.067 | 1.401 | | 1.298 | 1.785 | yes |
| `grep -rl QDBBENCHONE .` | 914.7 |  | 0.603 | 0.501 | 0.421 | 0.450 | | 0.454 | 2.742 | yes |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | 3.651 |  | | | 0.290 | 1.871 | | 1.689 | 2.805 | yes |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | 3.807 |  | | | 0.392 | 1.024 | | 1.327 | 1.442 | yes |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | 3.544 |  | | | 0.589 | 1.177 | | 1.507 | 1.285 | yes |
| `cat qdb_no_such_file &> bench_out/both.txt` | 2.705 |  | | | 0.049 | 0.521 | | 0.521 | 0.677 | yes |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 2.965 |  | | | 0.044 | 0.539 | | 0.481 | 0.804 | yes |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | 7.258 |  | | | 17.83 | 24.09 | | 17.10 | 20.64 | yes |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 3.484 |  | | | 0.037 | 0.489 | | 0.627 | 0.546 | yes |
| `sed -i '1d' bench_out/joined2.txt` | 7.922 |  | | | 2.425 | 6.316 | | 5.129 | 5.755 | yes |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 2.527 |  | | | 0.036 | 0.033 | | 0.031 | 0.025 | yes |
| `cat bench_out/both.txt` | 2.401 |  | | | 0.018 | 0.019 | | 0.018 | 0.014 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.717 |  | 0.035 | 0.891 | 0.021 | 0.563 | | 0.686 | 0.473 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.282 |  | 0.015 | 0.687 | 0.022 | 0.504 | | 0.567 | 0.657 | yes |
| `ls -R bench_out/a` | 3.263 |  | 0.019 | 0.019 | 0.025 | 0.023 | | 0.027 | 0.022 | yes |
| `rm bench_out/a/b/hello.txt` | 2.298 |  | 0.011 | 0.583 | 0.022 | 0.706 | | 0.615 | 0.654 | yes |
| `rmdir bench_out/a/b` | 3.333 |  | 0.015 | 0.543 | 0.021 | 0.532 | | 0.638 | 0.437 | yes |
| `rm -r bench_out/dir_moved` | 140.5 |  | 8.592 | 9.350 | 5.596 | 6.600 | | 8.485 | 6.530 | yes |
| `rm -rf bench_out` | 170.6 |  | 3.696 | 24.41 | 1.470 | 17.93 | | 17.28 | 16.90 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 4, on disk, 512 MB limit | Stage 4, on disk, 1 GB for everything, file cache included |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 212 B | 1457 B | 1457 B | 3849 B | 3849 B | 3849 B | 3849 B | 3849 B |
| `ls -a` | 330 B | 2681 B | 2681 B | 5647 B | 5647 B | 5647 B | 5647 B | 5647 B |
| `ls -R` | 1377.7 KB | 74.3 KB | 74.3 KB | 150.9 KB | 150.9 KB | 150.9 KB | 150.9 KB | 150.9 KB |
| `ls -aR Documentation` | 216.8 KB | 56.6 KB | 56.6 KB | 84.0 KB | 84.0 KB | 84.0 KB | 84.0 KB | 84.0 KB |
| `ls -p Documentation` | 796 B | 4893 B | 4893 B | 9231 B | 9231 B | 9231 B | 9231 B | 9231 B |
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
| `ls -ltR` | 4983.7 KB | | | 163.7 KB | 163.7 KB | 163.7 KB | 163.7 KB | 163.7 KB |
| `ls -lt include/linux` | 80.2 KB | | | 132.0 KB | 132.0 KB | 132.0 KB | 132.0 KB | 132.0 KB |
| `ls -ltr Documentation \| tail -n 3` | 138 B | | | 16.2 KB | 16.2 KB | 16.2 KB | 16.2 KB | 16.2 KB |
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
| `ls -R bench_out/a` | 41 B | 1089 B | 1089 B | 3129 B | 3129 B | | 3129 B | 3129 B |
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
