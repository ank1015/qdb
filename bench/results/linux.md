# Benchmark: linux

- Data: Linux 6.8 source tree, from the Ubuntu source package linux_6.8.0.orig.tar.gz (sha256 26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8). Skipped on import: 64 symbolic links, 6 files that are not valid text, 0 other
- Size: 83405 files in 5451 folders, 1396.9 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 5, on disk, settled), and include starting bash and the tool (1.487 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
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
| Stage 5, in memory | on | 2379.1 | 9679.0 | 1667.4 MB | 1622.7 MB | 0.324 | yes |
| Stage 5, on disk, settled | on | 13156.8 | 13156.8 | 22.4 MB | 966.6 MB | 0.327 | yes |
| Stage 5, on disk, 512 MB limit | on | 13281.4 | 13281.4 | 22.4 MB | 965.2 MB | 0.355 | yes |

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

## The index (Stage 5, in memory)

- Built in 7299.9 ms while loading.
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
| `cp include/linux/kernfs.h bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | documents +1 -0; postings +2585; list bytes rewritten 0; rebuilds 0 |
| `diff -u --label old --label new include/linux/kernfs.h bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff include/linux/kernfs.h bench_out/v2.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -q include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -u include/linux/kernfs.h bench_out/copy.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -rq Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | documents +1 -0; postings +53; list bytes rewritten 0; rebuilds 0 |
| `diff -rq Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -r include/linux bench_out/work \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -ru include/linux bench_out/work \| grep -c '^@@'` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | documents +10000 -9994; postings +138134; list bytes rewritten 0; rebuilds 0 |
| `wc -l bench_out/loop.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for f in bench_out/work/*.h; do echo done >> "$f"; done` | documents +1432 -0; postings +2864; list bytes rewritten 0; rebuilds 0 |
| `awk '{ print NR ": " $0 }' include/linux/kernfs.h > bench_out/numbered.txt` | documents +1 -0; postings +3282; list bytes rewritten 0; rebuilds 0 |
| `awk 'END { print NR }' bench_out/numbered.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -2902; postings +0; list bytes rewritten 192488; rebuilds 0 |

## The index (Stage 5, on disk, settled)

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
| `cp include/linux/kernfs.h bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | documents +1 -0; postings +2585; list bytes rewritten 0; rebuilds 0 |
| `diff -u --label old --label new include/linux/kernfs.h bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff include/linux/kernfs.h bench_out/v2.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -q include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -u include/linux/kernfs.h bench_out/copy.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -rq Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | documents +1 -0; postings +53; list bytes rewritten 0; rebuilds 0 |
| `diff -rq Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -r include/linux bench_out/work \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -ru include/linux bench_out/work \| grep -c '^@@'` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | documents +10000 -9994; postings +138134; list bytes rewritten 0; rebuilds 0 |
| `wc -l bench_out/loop.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for f in bench_out/work/*.h; do echo done >> "$f"; done` | documents +1432 -0; postings +2864; list bytes rewritten 0; rebuilds 0 |
| `awk '{ print NR ": " $0 }' include/linux/kernfs.h > bench_out/numbered.txt` | documents +1 -0; postings +3282; list bytes rewritten 0; rebuilds 0 |
| `awk 'END { print NR }' bench_out/numbered.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -2902; postings +0; list bytes rewritten 3109599; rebuilds 0 |

## The index (Stage 5, on disk, 512 MB limit)

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
| `cp include/linux/kernfs.h bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | documents +1 -0; postings +2585; list bytes rewritten 0; rebuilds 0 |
| `diff -u --label old --label new include/linux/kernfs.h bench_out/v2.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff include/linux/kernfs.h bench_out/v2.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -q include/linux/kernfs.h bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -u include/linux/kernfs.h bench_out/copy.txt \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -rq Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | documents +1 -0; postings +53; list bytes rewritten 0; rebuilds 0 |
| `diff -rq Documentation bench_out/dir_copy` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -r include/linux bench_out/work \| wc -l` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `diff -ru include/linux bench_out/work \| grep -c '^@@'` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | documents +10000 -9994; postings +138134; list bytes rewritten 0; rebuilds 0 |
| `wc -l bench_out/loop.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `for f in bench_out/work/*.h; do echo done >> "$f"; done` | documents +1432 -0; postings +2864; list bytes rewritten 0; rebuilds 0 |
| `awk '{ print NR ": " $0 }' include/linux/kernfs.h > bench_out/numbered.txt` | documents +1 -0; postings +3282; list bytes rewritten 0; rebuilds 0 |
| `awk 'END { print NR }' bench_out/numbered.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -2902; postings +0; list bytes rewritten 3109599; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 4, on disk, 512 MB limit | Stage 4, on disk, 1 GB for everything, file cache included | Stage 5, in memory | Stage 5, on disk, settled | Stage 5, on disk, 512 MB limit | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.609 |  | 0.001 | 0.001 | 0.002 | 0.001 | 0.004 | 0.002 | 0.030 | 0.003 | 0.003 | 0.004 | yes |
| `ls -a` | 2.376 |  | 0.004 | 0.004 | 0.002 | 0.003 | 0.005 | 0.002 | 0.004 | 0.005 | 0.005 | 0.004 | yes |
| `ls -R` | 80.52 |  | 8.025 | 3.303 | 11.81 | 4.700 | 3.506 | 3.783 | 11.94 | 4.186 | 4.149 | 3.843 | yes |
| `ls -aR Documentation` | 13.05 |  | 0.747 | 0.861 | 0.838 | 0.950 | 0.692 | 0.659 | 1.219 | 1.095 | 0.682 | 0.616 | yes |
| `ls -p Documentation` | 2.610 |  | 0.004 | 0.004 | 0.004 | 0.005 | 0.005 | 0.004 | 0.011 | 0.009 | 0.006 | 0.006 | yes |
| `find .` | 83.91 |  | 20.82 | 10.16 | 13.43 | 11.32 | 11.69 | 7.845 | 8.976 | 8.895 | 8.479 | 8.405 | yes |
| `find . -name '*.h'` | 81.04 |  | 25.53 | 10.22 | 16.53 | 10.78 | 10.78 | 10.76 | 13.05 | 11.35 | 10.73 | 11.16 | yes |
| `find . -name '*.c'` | 82.05 |  | 28.25 | 10.50 | 15.00 | 10.79 | 11.50 | 9.946 | 11.39 | 11.91 | 11.03 | 10.58 | yes |
| `find . -type d` | 61.99 |  | 25.02 | 6.396 | 14.78 | 6.464 | 6.794 | 7.453 | 7.901 | 8.565 | 8.007 | 7.830 | yes |
| `find . -type f` | 77.27 |  | 23.39 | 11.97 | 15.19 | 7.163 | 6.812 | 8.360 | 9.119 | 9.074 | 8.326 | 8.612 | yes |
| `find . -maxdepth 2 -type f` | 4.407 |  | 0.114 | 0.109 | 0.131 | 0.201 | 0.129 | 0.132 | 0.129 | 0.160 | 0.173 | 0.151 | yes |
| `find Documentation -name '*.h' -type f` | 10.97 |  | 1.770 | 1.071 | 1.110 | 1.629 | 1.302 | 1.167 | 1.088 | 1.333 | 1.260 | 1.179 | yes |
| `find . -name 'Makefile'` | 70.10 |  | 27.58 | 7.526 | 12.93 | 7.135 | 7.509 | 10.78 | 10.33 | 12.36 | 8.345 | 14.37 | yes |
| `cat include/linux/kernfs.h` | 2.343 |  | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | 0.003 | yes |
| `cat -n include/linux/kernfs.h` | 2.392 |  | 0.057 | 0.055 | 0.055 | 0.071 | 0.065 | 0.054 | 0.055 | 0.065 | 0.065 | 0.060 | yes |
| `head -n 20 include/linux/kernfs.h` | 2.264 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.002 | 0.002 | 0.004 | 0.004 | 0.004 | yes |
| `tail -n 20 include/linux/kernfs.h` | 2.255 |  | 0.002 | 0.002 | 0.002 | 0.003 | 0.004 | 0.002 | 0.002 | 0.004 | 0.006 | 0.004 | yes |
| `tail -n +100 include/linux/kernfs.h` | 2.405 |  | 0.003 | 0.003 | 0.004 | 0.004 | 0.006 | 0.004 | 0.007 | 0.006 | 0.008 | 0.006 | yes |
| `wc include/linux/kernfs.h` | 2.713 |  | 0.024 | 0.023 | 0.026 | 0.037 | 0.026 | 0.022 | 0.035 | 0.024 | 0.026 | 0.025 | yes |
| `wc -l include/linux/*.h` | 15.20 |  | 28.62 | 0.885 | 0.845 | 1.369 | 1.013 | 0.807 | 0.896 | 1.000 | 1.070 | 0.992 | yes |
| `cat include/linux/*.h \| wc -l` | 33.17 |  | 26.33 | 24.59 | 1.649 | 1.655 | 1.508 | 1.789 | 2.143 | 1.511 | 1.529 | 1.457 | yes |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 2.715 |  | 0.005 | 0.006 | 0.004 | 0.004 | 0.007 | 0.004 | 0.004 | 0.008 | 0.008 | 0.008 | yes |
| `grep -c define include/linux/kernfs.h` | 2.386 |  | 0.006 | 0.009 | 0.010 | 0.010 | 0.011 | 0.010 | 0.043 | 0.014 | 0.014 | 0.013 | yes |
| `grep -n define include/linux/kernfs.h` | 2.288 |  | 0.006 | 0.009 | 0.010 | 0.012 | 0.014 | 0.010 | 0.010 | 0.014 | 0.013 | 0.014 | yes |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 2.468 |  | 0.249 | 0.226 | 0.011 | 0.013 | 0.014 | 0.012 | 0.011 | 0.058 | 0.032 | 0.033 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 3.337 |  | 0.588 | 0.560 | 0.484 | 0.755 | 0.425 | 0.410 | 0.492 | 0.253 | 0.256 | 0.242 | yes |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 3.143 |  | 0.152 | 0.120 | 0.071 | 0.117 | 0.074 | 0.055 | 0.062 | 0.100 | 0.098 | 0.094 | yes |
| `grep -rn qdbzzabsentterm .` | 1244.9 | 591.2 | 0.012 | 0.020 | 0.031 | 0.018 | 0.022 | 0.024 | 0.102 | 0.035 | 0.035 | 0.033 | yes |
| `grep -rl qdbzzabsentterm .` | 794.2 | 537.0 | 0.011 | 0.021 | 0.021 | 0.024 | 0.025 | 0.022 | 0.037 | 0.034 | 0.050 | 0.035 | yes |
| `grep -rn ausgabenspezifisches .` | 1281.6 | 586.2 | 0.098 | 0.125 | 0.255 | 0.175 | 0.145 | 0.208 | 0.284 | 0.145 | 0.144 | 0.125 | yes |
| `grep -rl ausgabenspezifisches .` | 795.4 | 546.3 | 0.095 | 0.100 | 0.237 | 0.123 | 0.113 | 0.216 | 0.120 | 0.136 | 0.139 | 0.123 | yes |
| `grep -rn mispredicted .` | 1417.5 | 579.0 | 17.54 | 2.475 | 2.013 | 1.846 | 2.426 | 1.484 | 3.261 | 1.629 | 1.419 | 1.440 | yes |
| `grep -rl mispredicted .` | 961.9 | 566.8 | 17.60 | 1.544 | 1.323 | 1.388 | 1.433 | 1.228 | 2.483 | 2.082 | 1.419 | 1.793 | yes |
| `grep -rn separately .` | 1060.7 | 586.5 | 24.09 | 14.82 | 14.06 | 15.89 | 15.28 | 14.82 | 50.00 | 9.537 | 6.279 | 6.541 | yes |
| `grep -rl separately .` | 634.7 | 541.0 | 18.81 | 11.89 | 9.678 | 7.971 | 11.99 | 11.84 | 11.74 | 5.049 | 5.150 | 4.775 | yes |
| `grep -rn conditions .` | 1375.7 | 659.3 | 129.9 | 62.60 | 64.28 | 60.69 | 68.25 | 61.98 | 321.4 | 68.87 | 58.59 | 57.54 | yes |
| `grep -rl conditions .` | 787.2 | 513.7 | 65.31 | 40.80 | 42.35 | 41.35 | 50.50 | 37.82 | 37.92 | 42.23 | 36.85 | 37.02 | yes |
| `grep -rn define .` | 3979.2 | 3256.5 | 460.7 | 518.8 | 721.8 | 453.8 | 552.3 | 542.8 | 2157.3 | 718.6 | 657.4 | 484.8 | yes |
| `grep -rl define .` | 601.4 | 621.7 | 100.9 | 59.67 | 66.79 | 59.40 | 73.52 | 60.42 | 588.4 | 62.08 | 89.79 | 55.17 | yes |
| `grep -rc separately .` | 748.1 | 627.1 | 44.65 | 17.95 | 22.06 | 17.37 | 19.14 | 19.89 | 18.51 | 22.63 | 20.99 | 20.18 | yes |
| `grep -rin MISPREDICTED .` | 1623.6 | 746.7 | 21.12 | 2.271 | 2.122 | 1.781 | 1.913 | 2.990 | 3.418 | 1.931 | 1.780 | 1.902 | yes |
| `grep -ri separately .` | 897.4 | 651.4 | 28.93 | 16.61 | 15.76 | 16.46 | 17.11 | 21.64 | 19.32 | 17.89 | 6.554 | 6.268 | yes |
| `grep -rn separately Documentation` | 63.99 | 46.77 | 2.473 | 1.884 | 2.131 | 1.981 | 2.047 | 1.869 | 4.498 | 1.893 | 1.734 | 1.816 | yes |
| `grep -rl conditions Documentation` | 60.41 | 46.69 | 3.711 | 3.362 | 3.715 | 4.539 | 3.145 | 4.839 | 7.190 | 3.376 | 3.249 | 3.332 | yes |
| `grep -rF 'separately' .` | 631.2 | 556.5 | 24.42 | 14.81 | 13.94 | 15.19 | 15.36 | 14.80 | 15.08 | 8.940 | 5.825 | 7.483 | yes |
| `grep -rE 'atomic_[a-z]+\(' .` | 1373.0 | 590.4 | 74.42 | 51.84 | 55.70 | 68.58 | 70.09 | 49.99 | 284.3 | 57.47 | 51.46 | 56.00 | yes |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 1887.2 | 664.0 | 29.62 | 2.512 | 2.288 | 3.797 | 3.471 | 3.226 | 6.965 | 2.429 | 2.448 | 2.401 | yes |
| `grep -rE 'conditions\|separately' .` | 2432.5 | 712.3 | 180.1 | 75.05 | 77.90 | 85.92 | 77.01 | 89.75 | 238.5 | 82.79 | 76.32 | 76.92 | yes |
| `grep -rE '[0-9]{4,}' .` | 2937.7 | 3751.1 | 2025.8 | 2153.3 | 2242.9 | 2177.9 | 2079.3 | 2008.0 | 2817.3 | 2421.1 | 2461.6 | 2200.5 | yes |
| `grep -rn '^#include' .` | 1976.4 | 994.5 | 244.7 | 217.7 | 216.7 | 237.1 | 240.7 | 230.0 | 983.1 | 259.7 | 261.6 | 227.6 | yes |
| `grep -rE '^(import\|from) ' .` | 1178.0 | 677.5 | 315.7 | 263.9 | 269.0 | 268.6 | 285.2 | 286.1 | 1002.8 | 274.3 | 262.5 | 267.7 | yes |
| `grep -rin 'return [a-z_]*;$' Documentation` | 96.22 | 53.29 | 11.32 | 4.304 | 5.141 | 4.570 | 5.156 | 3.799 | 57.36 | 4.487 | 4.137 | 4.337 | yes |
| `grep -rv e drivers/usb` | 60.16 | 68.73 | 17.91 | 18.05 | 18.38 | 19.80 | 19.55 | 18.13 | 19.97 | 21.36 | 19.55 | 19.99 | yes |
| `grep -r '\(separately\).*\1' .` | 664.6 |  | 24.81 | 15.77 | 16.42 | 16.14 | 15.66 | 16.99 | 73.82 | 14.76 | 13.61 | 9.138 | yes |
| `grep -n mispredicted include/linux/*.h` | 18.27 |  | | 0.998 | 1.135 | 1.198 | 1.160 | 0.990 | 1.414 | 1.312 | 1.256 | 1.318 | yes |
| `grep -l separately include/linux/*.h` | 9.746 |  | | 1.514 | 1.509 | 1.571 | 1.375 | 1.417 | 1.270 | 1.509 | 1.454 | 1.550 | yes |
| `grep -c qdbzzabsentterm include/linux/*.h` | 14.25 |  | | 0.595 | 0.542 | 0.592 | 0.651 | 0.680 | 0.895 | 0.671 | 0.700 | 0.686 | yes |
| `grep -rl conditions . \| wc -l` | 768.7 |  | 58.03 | 42.95 | 45.55 | 39.57 | 50.85 | 40.97 | 193.2 | 42.98 | 36.19 | 37.08 | yes |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1362.0 |  | 342.3 | 315.0 | 321.6 | 332.4 | 321.1 | 328.5 | 890.4 | 384.1 | 323.0 | 323.2 | yes |
| `find . -name '*.h' \| wc -l` | 82.95 |  | 23.83 | 11.43 | 12.39 | 9.695 | 8.064 | 10.28 | 16.56 | 11.40 | 10.15 | 10.32 | yes |
| `find . -type f \| grep -c h` | 89.90 |  | 27.69 | 13.79 | 20.38 | 12.69 | 14.03 | 13.52 | 17.81 | 14.93 | 14.02 | 12.97 | yes |
| `grep -rn define . \| head -n 5` | 4.787 |  | 0.019 | 0.024 | 0.025 | 0.027 | 0.028 | 0.036 | 0.070 | 0.031 | 0.031 | 0.027 | order differs |
| `grep -rn conditions . \| head -n 1` | 12.01 |  | 0.027 | 0.040 | 0.044 | 0.046 | 0.050 | 0.041 | 0.133 | 0.054 | 0.051 | 0.045 | order differs |
| `find . -type f \| head -n 3` | 3.552 |  | 0.058 | 0.093 | 0.052 | 0.059 | 0.066 | 0.053 | 0.054 | 0.070 | 0.073 | 0.070 | order differs |
| `cat include/linux/*.h \| head -n 2` | 4.754 |  | 0.346 | 0.335 | 0.324 | 0.475 | 0.324 | 0.479 | 0.289 | 0.398 | 0.364 | 0.343 | yes |
| `grep -rno separately .` | 1095.2 | 617.7 | | | 17.59 | 14.42 | 15.68 | 15.83 | 99.75 | 7.652 | 7.952 | 6.494 | yes |
| `grep -rw separately .` | 618.2 | 543.8 | | | 14.01 | 13.63 | 13.77 | 15.16 | 16.39 | 7.653 | 6.396 | 5.817 | yes |
| `grep -rnw mispredicted .` | 1407.2 | 623.9 | | | 1.934 | 1.927 | 1.633 | 1.959 | 4.685 | 1.797 | 1.790 | 1.778 | yes |
| `grep -rn -C 2 mispredicted .` | 1434.9 | 3275.6 | | | 2.482 | 2.294 | 2.781 | 2.381 | 4.147 | 2.205 | 2.145 | 2.044 | yes |
| `grep -rn -A 1 -B 1 separately .` | 1091.3 | 3744.1 | | | 22.36 | 23.01 | 23.65 | 25.13 | 225.7 | 23.27 | 21.46 | 18.32 | yes |
| `grep -rho 'atomic_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 1466.7 |  | | | 85.16 | 85.92 | 98.70 | 108.6 | 431.1 | 67.94 | 62.10 | 69.46 | yes |
| `grep -rhow '[a-z]*mispredicted[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 1050.9 |  | | | 4.568 | 4.082 | 4.905 | 4.590 | 5.669 | 2.456 | 3.496 | 2.420 | yes |
| `rg -j 1 qdbzzabsentterm` | 659.3 |  | | | 0.088 | 0.068 | 0.093 | 0.106 | 0.075 | 0.086 | 0.122 | 0.080 | yes |
| `rg -j 1 ausgabenspezifisches` | 645.5 |  | | | 0.112 | 0.101 | 0.154 | 0.169 | 0.244 | 0.147 | 0.145 | 0.140 | yes |
| `rg -j 1 mispredicted` | 672.1 |  | | | 2.173 | 2.226 | 2.261 | 1.902 | 3.054 | 2.205 | 2.129 | 2.110 | yes |
| `rg -j 1 separately` | 666.9 |  | | | 21.23 | 16.57 | 19.18 | 17.65 | 18.47 | 13.33 | 10.46 | 9.067 | yes |
| `rg -j 1 conditions` | 734.7 |  | | | 126.9 | 113.5 | 142.3 | 120.3 | 293.0 | 130.1 | 148.5 | 131.0 | yes |
| `rg -j 1 define` | 2527.5 |  | | | 536.9 | 509.8 | 537.3 | 468.6 | 2370.2 | 670.0 | 661.5 | 484.0 | yes |
| `rg -j 1 -l separately` | 721.8 |  | | | 16.63 | 13.89 | 16.26 | 13.62 | 124.2 | 8.040 | 7.954 | 8.174 | yes |
| `rg -j 1 -l define` | 802.8 |  | | | 117.8 | 113.0 | 116.9 | 125.3 | 720.8 | 134.6 | 128.4 | 142.9 | yes |
| `rg -j 1 -c conditions` | 803.7 |  | | | 120.4 | 127.7 | 133.5 | 110.3 | 341.8 | 129.4 | 135.1 | 119.4 | yes |
| `rg -j 1 -i MISPREDICTED` | 788.3 |  | | | 20.46 | 20.15 | 21.00 | 20.40 | 27.02 | 22.17 | 21.37 | 22.11 | yes |
| `rg -j 1 -w separately` | 651.1 |  | | | 27.44 | 25.90 | 25.67 | 27.16 | 121.4 | 24.79 | 21.92 | 26.02 | yes |
| `rg -j 1 -F 'separately'` | 642.3 |  | | | 17.83 | 18.03 | 18.68 | 17.76 | 18.92 | 13.05 | 14.75 | 13.18 | yes |
| `rg -j 1 -n -C 2 mispredicted` | 739.7 |  | | | 2.546 | 2.716 | 3.209 | 2.969 | 4.415 | 4.150 | 3.013 | 2.981 | yes |
| `rg -j 1 'atomic_[a-z]+\('` | 702.9 |  | | | 64.24 | 64.29 | 69.70 | 60.18 | 243.8 | 66.81 | 60.35 | 71.27 | yes |
| `rg -j 1 'mispredicted\|ausgabenspezifisches'` | 754.3 |  | | | 2.994 | 2.382 | 2.463 | 2.444 | 4.016 | 2.748 | 2.800 | 2.679 | yes |
| `rg -j 1 '\bmispredicted\w*'` | 663.4 |  | | | 6.912 | 5.998 | 6.434 | 5.838 | 7.881 | 6.880 | 6.729 | 6.556 | yes |
| `rg -j 1 -t c separately` | 561.3 |  | | | 15.06 | 14.48 | 16.50 | 14.89 | 16.91 | 8.908 | 9.370 | 8.423 | yes |
| `rg -j 1 -g '*.h' separately` | 329.8 |  | | | 5.155 | 5.530 | 5.312 | 4.912 | 6.451 | 4.881 | 4.928 | 5.043 | yes |
| `rg -j 1 separately Documentation` | 57.21 |  | | | 11.06 | 9.652 | 8.437 | 10.61 | 10.84 | 11.38 | 11.23 | 18.52 | yes |
| `rg -j 1 -uu separately` | 569.2 |  | | | 13.35 | 13.22 | 14.89 | 13.02 | 14.53 | 6.028 | 6.170 | 8.473 | yes |
| `rg -j 1 --sort path mispredicted` | 684.6 |  | | | 2.314 | 1.925 | 3.109 | 2.136 | 3.575 | 2.461 | 2.156 | 2.687 | yes |
| `rg -j 1 --files \| wc -l` | 161.7 |  | | | 72.11 | 88.76 | 61.96 | 61.92 | 92.41 | 78.91 | 73.78 | 69.52 | yes |
| `rg -j 1 -l conditions \| wc -l` | 599.2 |  | | | 100.1 | 96.09 | 110.0 | 102.1 | 113.9 | 112.5 | 106.1 | 103.8 | yes |
| `ls -ltR` | 331.1 |  | | | 45.91 | 45.92 | 43.44 | 44.09 | 55.93 | 76.10 | 51.19 | 46.14 | yes |
| `ls -lt include/linux` | 7.545 |  | | | 0.634 | 0.885 | 0.639 | 0.589 | 0.868 | 0.870 | 0.839 | 0.789 | yes |
| `ls -ltr Documentation \| tail -n 3` | 3.725 |  | | | 0.052 | 0.052 | 0.057 | 0.051 | 0.065 | 0.067 | 0.066 | 0.064 | yes |
| `find . -newer include/linux/kernfs.h` | 122.8 |  | | | 12.62 | 8.639 | 6.778 | 7.574 | 8.893 | 8.498 | 8.174 | 7.934 | yes |
| `find . -newer include/linux/kernfs.h -name '*.h'` | 96.41 |  | | | 14.76 | 8.308 | 8.254 | 8.318 | 9.991 | 9.214 | 9.435 | 8.678 | yes |
| `find . -type f -mtime +30` | 130.7 |  | | | 13.32 | 11.68 | 7.837 | 9.768 | 13.65 | 8.515 | 8.577 | 7.860 | yes |
| `du -s .` | 123.8 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.016 | 0.004 | 0.003 | 0.003 | not compared |
| `du -sh Documentation` | 16.96 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.002 | 0.003 | 0.003 | 0.003 | not compared |
| `du -sb .` | 121.0 |  | | | 0.001 | 0.002 | 0.002 | 0.001 | 0.013 | 0.003 | 0.003 | 0.003 | yes |
| `du -sb Documentation` | 18.62 |  | | | 0.001 | 0.002 | 0.002 | 0.002 | 0.001 | 0.003 | 0.007 | 0.003 | yes |
| `du -b -d 1 Documentation` | 16.85 |  | | | 0.007 | 0.007 | 0.017 | 0.006 | 0.009 | 0.009 | 0.011 | 0.011 | yes |
| `sort include/linux/kernfs.h` | 4.095 |  | | | 0.043 | 0.040 | 0.070 | 0.042 | 0.046 | 0.036 | 0.042 | 0.036 | yes |
| `sort -r include/linux/kernfs.h \| head -n 3` | 4.034 |  | | | 0.045 | 0.072 | 0.071 | 0.045 | 0.074 | 0.043 | 0.045 | 0.044 | yes |
| `sort -u include/linux/kernfs.h \| wc -l` | 4.245 |  | | | 0.040 | 0.047 | 0.053 | 0.037 | 0.075 | 0.045 | 0.049 | 0.046 | yes |
| `sort -k 2 include/linux/kernfs.h \| tail -n 3` | 4.060 |  | | | 0.054 | 0.061 | 0.063 | 0.060 | 0.099 | 0.060 | 0.064 | 0.058 | yes |
| `cat include/linux/*.h \| sort \| uniq -c \| sort -rn \| head -n 5` | 258.4 |  | | | 184.2 | 141.1 | 168.6 | 161.7 | 177.0 | 142.0 | 144.7 | 157.1 | yes |
| `uniq -c include/linux/kernfs.h \| wc -l` | 3.381 |  | | | 0.036 | 0.028 | 0.030 | 0.028 | 0.027 | 0.030 | 0.043 | 0.029 | yes |
| `cut -c 1-20 include/linux/kernfs.h` | 2.434 |  | | | 0.020 | 0.018 | 0.023 | 0.017 | 0.019 | 0.020 | 0.035 | 0.020 | yes |
| `cut -d ' ' -f 1 include/linux/kernfs.h \| sort \| uniq \| wc -l` | 4.260 |  | | | 0.076 | 0.066 | 0.076 | 0.075 | 0.077 | 0.080 | 0.123 | 0.079 | yes |
| `tr a-z A-Z < include/linux/kernfs.h` | 2.914 |  | | | 0.005 | 0.003 | 0.004 | 0.005 | 0.003 | 0.008 | 0.009 | 0.005 | yes |
| `tr -d '\n' < include/linux/kernfs.h \| wc -c` | 3.470 |  | | | 0.015 | 0.008 | 0.010 | 0.009 | 0.011 | 0.018 | 0.024 | 0.013 | yes |
| `tr -s ' ' < include/linux/kernfs.h \| wc -c` | 3.595 |  | | | 0.023 | 0.013 | 0.015 | 0.014 | 0.013 | 0.018 | 0.032 | 0.018 | yes |
| `nl include/linux/kernfs.h` | 2.621 |  | | | 0.087 | 0.043 | 0.050 | 0.047 | 0.046 | 0.057 | 0.101 | 0.057 | yes |
| `nl -ba include/linux/kernfs.h \| tail -n 3` | 3.461 |  | | | 0.099 | 0.046 | 0.080 | 0.045 | 0.051 | 0.066 | 0.123 | 0.066 | yes |
| `seq 1 100000 \| wc -l` | 4.553 |  | | | 4.246 | 2.469 | 2.528 | 2.466 | 3.481 | 2.904 | 2.937 | 2.950 | yes |
| `seq -w 1 1000 \| tail -n 2` | 4.082 |  | | | 0.024 | 0.024 | 0.027 | 0.026 | 0.034 | 0.038 | 0.036 | 0.036 | yes |
| `basename include/linux/kernfs.h` | 3.211 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | 0.003 | yes |
| `basename -s .h include/linux/kernfs.h` | 3.090 |  | | | 0.002 | 0.001 | 0.001 | 0.002 | 0.001 | 0.004 | 0.003 | 0.003 | yes |
| `dirname include/linux/kernfs.h` | 3.136 |  | | | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | 0.003 | yes |
| `printf '%s\n' a b c` | 2.165 |  | | | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | 0.003 | 0.003 | 0.003 | yes |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 1.766 |  | | | 0.002 | 0.002 | 0.004 | 0.003 | 0.002 | 0.004 | 0.004 | 0.004 | yes |
| `test -f include/linux/kernfs.h && echo yes` | 1.946 |  | | | 0.003 | 0.002 | 0.002 | 0.002 | 0.002 | 0.005 | 0.005 | 0.004 | yes |
| `[ -d Documentation ] && echo folder` | 1.764 |  | | | 0.002 | 0.004 | 0.003 | 0.003 | 0.002 | 0.004 | 0.005 | 0.005 | yes |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 3.064 |  | | | 0.003 | 0.004 | 0.006 | 0.003 | 0.004 | 0.005 | 0.006 | 0.005 | yes |
| `echo a b c \| xargs -n 1 echo` | 6.412 |  | | | 0.003 | 0.004 | 0.006 | 0.003 | 0.003 | 0.006 | 0.006 | 0.006 | yes |
| `find drivers/usb -name '*.h' \| xargs cat \| wc -l` | 9.939 |  | | | 0.403 | 0.558 | 0.444 | 0.429 | 0.657 | 0.405 | 0.422 | 0.432 | yes |
| `find . -name '*.h' \| xargs grep -l separately \| wc -l` | 214.9 |  | | | 54.13 | 52.35 | 39.39 | 35.91 | 74.50 | 47.62 | 42.44 | 40.89 | yes |
| `echo {a,b,c}{1..3}` | 1.770 |  | | | 0.005 | 0.006 | 0.005 | 0.004 | 0.004 | 0.005 | 0.006 | 0.006 | yes |
| `echo {1..20..3} {a..e}` | 1.832 |  | | | 0.005 | 0.007 | 0.006 | 0.005 | 0.005 | 0.006 | 0.007 | 0.006 | yes |
| `echo $(wc -l < include/linux/kernfs.h) `basename include/linux/kernfs.h`` | 4.710 |  | | | 0.005 | 0.008 | 0.007 | 0.006 | 0.006 | 0.009 | 0.010 | 0.010 | yes |
| `echo "first line: $(head -n 1 include/linux/kernfs.h)"` | 2.944 |  | | | 0.005 | 0.004 | 0.007 | 0.004 | 0.006 | 0.007 | 0.007 | 0.007 | yes |
| `wc -l < include/linux/kernfs.h` | 3.008 |  | | | 0.003 | 0.002 | 0.002 | 0.002 | 0.002 | 0.004 | 0.004 | 0.004 | yes |
| `cat include/linux/kernfs.h > /dev/null 2>&1` | 2.814 |  | | | 0.002 | 0.001 | 0.002 | 0.002 | 0.002 | 0.003 | 0.003 | 0.003 | yes |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 2.826 |  | | | 0.002 | 0.003 | 0.004 | 0.003 | 0.003 | 0.005 | 0.005 | 0.005 | yes |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 3.154 |  | | | 0.003 | 0.002 | 0.004 | 0.002 | 0.003 | 0.005 | 0.005 | 0.005 | yes |
| `grep -q define include/linux/kernfs.h && echo found \|\| echo none` | 2.970 |  | | | 0.014 | 0.017 | 0.014 | 0.009 | 0.011 | 0.015 | 0.015 | 0.015 | yes |
| `false \|\| echo fallback` | 1.762 |  | | | 0.002 | 0.002 | 0.001 | 0.001 | 0.002 | 0.005 | 0.003 | 0.004 | yes |
| `true && grep -c define include/linux/kernfs.h` | 2.658 |  | | | 0.017 | 0.018 | 0.013 | 0.009 | 0.013 | 0.015 | 0.013 | 0.013 | yes |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 3.273 |  | | | 0.004 | 0.004 | 0.003 | 0.002 | 0.003 | 0.005 | 0.005 | 0.005 | yes |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2.931 |  | | | 0.002 | 0.002 | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | 0.003 | yes |
| `echo one \⏎two` | 1.815 |  | | | 0.002 | 0.002 | 0.001 | 0.001 | 0.001 | 0.003 | 0.003 | 0.003 | yes |
| `grep -c define include/linux/kernfs.h⏎wc -l include/linux/kernfs.h` | 4.423 |  | | | 0.011 | 0.020 | 0.012 | 0.010 | 0.013 | 0.016 | 0.016 | 0.016 | yes |
| `for i in $(seq 100000); do :; done` | 99.16 |  | | | | | | | | 33.80 | 38.52 | 36.47 | yes |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 37.06 |  | | | | | | | | 23.16 | 29.80 | 24.23 | yes |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 2.200 |  | | | | | | | | 0.028 | 0.031 | 0.030 | yes |
| `for f in include/linux/*.h; do echo "$f"; done \| wc -l` | 19.63 |  | | | | | | | | 1.384 | 2.095 | 1.425 | yes |
| `for f in include/linux/*.h; do grep -c define "$f"; done \| sort -n \| tail -n 3` | 1892.0 |  | | | | | | | | 17.70 | 17.19 | 17.44 | yes |
| `n=0; for f in include/linux/*.h; do if grep -q conditions "$f"; then n=$((n+1)); fi; done; echo $n` | 1883.4 |  | | | | | | | | 24.02 | 25.06 | 25.67 | yes |
| `for f in include/linux/*.h; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 9.844 |  | | | | | | | | 2.703 | 2.929 | 3.127 | yes |
| `H=$PWD; for d in Documentation drivers/usb include/linux; do cd "$d"; ls \| wc -l; cd "$H"; done` | 8.362 |  | | | | | | | | 0.052 | 0.063 | 0.053 | yes |
| `X=define; N=$(grep -c "$X" include/linux/kernfs.h); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 3.707 |  | | | | | | | | 0.020 | 0.021 | 0.020 | yes |
| `find drivers/usb -name '*.h' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 292.8 |  | | | | | | | | 0.743 | 0.735 | 0.726 | yes |
| `for f in include/linux/*.h; do head -n 1 "$f"; done > /dev/null` | 1597.7 |  | | | | | | | | 2.189 | 2.542 | 2.177 | yes |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 1.799 |  | | | | | | | | 0.017 | 0.018 | 0.026 | yes |
| `awk '{ print $2 }' include/linux/kernfs.h` | 3.171 |  | | | | | | | | 0.037 | 0.041 | 0.036 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 113.1 |  | | | | | | | | 29.47 | 30.21 | 30.33 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '/define/ { n++ } END { print n }'` | 76.13 |  | | | | | | | | 21.09 | 21.37 | 21.33 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 108.9 |  | | | | | | | | 41.99 | 41.85 | 43.64 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 69.45 |  | | | | | | | | 23.14 | 23.10 | 22.42 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 62.94 |  | | | | | | | | 21.89 | 18.67 | 19.34 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 63.59 |  | | | | | | | | 18.92 | 19.30 | 19.02 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 103.6 |  | | | | | | | | 50.19 | 50.20 | 47.29 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "define") n++ } END { print n }'` | 255.3 |  | | | | | | | | 143.3 | 152.1 | 132.7 | yes |
| `awk '{ gsub(/[0-9]+/, "N"); print }' include/linux/kernfs.h \| wc -c` | 4.884 |  | | | | | | | | 0.314 | 0.317 | 0.298 | yes |
| `awk -v OFS=- '{ $1 = NR; print }' include/linux/kernfs.h \| tail -n 1` | 4.189 |  | | | | | | | | 0.231 | 0.219 | 0.236 | yes |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 9.932 |  | | | | | | | | 5.533 | 5.435 | 4.971 | yes |
| `grep -rn separately . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 1075.0 |  | | | | | | | | 6.534 | 9.086 | 10.39 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 182.5 |  | | | | | | | | 99.90 | 88.18 | 86.16 | yes |
| `mkdir -p bench_out/a/b` | 3.468 |  | 0.015 | 0.910 | 0.017 | 1.311 | | 0.863 | 0.980 | 0.030 | 0.607 | 0.565 | yes |
| `touch bench_out/empty.txt` | 2.757 |  | 0.009 | 0.578 | 0.021 | 0.527 | | 0.679 | 0.439 | 0.021 | 0.348 | 0.265 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.890 |  | 0.017 | 0.702 | 0.172 | 0.461 | | 0.685 | 0.450 | 0.148 | 0.379 | 0.280 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.748 |  | 0.018 | 0.723 | 0.174 | 0.513 | | 1.197 | 0.637 | 0.168 | 0.455 | 0.554 | yes |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 2.785 |  | 0.510 | 8.203 | 0.026 | 0.416 | | 0.483 | 0.593 | 0.027 | 0.257 | 0.370 | yes |
| `cp -r Documentation bench_out/dir_copy` | 2069.1 |  | 476.3 | 489.7 | 12.68 | 31.02 | | 18.12 | 19.43 | 25.67 | 17.57 | 31.65 | yes |
| `cp -r include/linux bench_out/work` | 450.8 |  | 172.5 | 239.7 | 3.597 | 8.828 | | 5.284 | 7.780 | 3.646 | 4.775 | 8.715 | yes |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 3.919 |  | 0.545 | 1.468 | 2.069 | 2.098 | | 1.553 | 2.311 | 1.754 | 1.158 | 1.680 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.505 |  | 0.478 | 1.352 | 0.516 | 1.623 | | 1.413 | 1.982 | 0.433 | 0.864 | 1.471 | yes |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 416.9 |  | 212.9 | 287.3 | 258.2 | 268.9 | | 248.3 | 342.2 | 235.3 | 201.6 | 191.9 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 526.0 |  | | 3.380 | 1.478 | 17.06 | | 14.39 | 17.75 | 1.114 | 13.70 | 14.52 | yes |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 236.1 |  | | 4.160 | 2.061 | 19.27 | | 16.89 | 17.44 | 2.468 | 14.05 | 17.02 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 3.067 |  | | 0.048 | 0.018 | 0.693 | | 0.579 | 0.570 | 0.015 | 0.307 | 0.466 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 2.947 |  | | 0.048 | 0.018 | 0.693 | | 0.579 | 0.570 | 0.015 | 0.307 | 0.466 | yes |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 2.983 |  | | 0.037 | 0.034 | 0.424 | | 0.409 | 0.549 | 0.038 | 0.225 | 0.378 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 54.66 |  | 7.128 | 7.322 | 4.916 | 5.693 | | 5.117 | 23.62 | 4.449 | 5.209 | 5.044 | yes |
| `grep -rl QDBBENCHMANY .` | 621.8 |  | 3.630 | 2.149 | 1.619 | 1.227 | | 1.100 | 1.785 | 1.313 | 1.255 | 1.192 | yes |
| `grep -rl QDBBENCHONE .` | 941.4 |  | 0.603 | 0.501 | 0.427 | 0.502 | | 0.457 | 2.742 | 0.375 | 0.484 | 0.402 | yes |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | 4.006 |  | | | 0.373 | 1.367 | | 1.368 | 2.805 | 0.321 | 1.027 | 1.082 | yes |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | 3.727 |  | | | 0.507 | 1.309 | | 1.300 | 1.442 | 0.358 | 0.928 | 1.843 | yes |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | 3.164 |  | | | 0.520 | 1.264 | | 1.486 | 1.285 | 0.332 | 0.973 | 1.851 | yes |
| `cat qdb_no_such_file &> bench_out/both.txt` | 2.695 |  | | | 0.041 | 0.448 | | 0.694 | 0.677 | 0.037 | 0.292 | 0.429 | yes |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 3.245 |  | | | 0.037 | 0.426 | | 0.634 | 0.804 | 0.023 | 0.341 | 0.300 | yes |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | 7.014 |  | | | 19.22 | 23.91 | | 22.39 | 20.64 | 13.90 | 17.53 | 20.15 | yes |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 3.537 |  | | | 0.042 | 0.612 | | 0.617 | 0.546 | 0.035 | 0.308 | 0.544 | yes |
| `sed -i '1d' bench_out/joined2.txt` | 8.121 |  | | | 2.956 | 4.735 | | 6.197 | 5.755 | 2.485 | 4.987 | 4.867 | yes |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 2.469 |  | | | 0.031 | 0.024 | | 0.029 | 0.025 | 0.027 | 0.030 | 0.024 | yes |
| `cat bench_out/both.txt` | 2.357 |  | | | 0.016 | 0.011 | | 0.013 | 0.014 | 0.028 | 0.015 | 0.013 | yes |
| `cp include/linux/kernfs.h bench_out/v2.txt` | 2.726 |  | | | | | | | | 0.024 | 0.351 | 0.372 | yes |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 2.777 |  | | | | | | | | 0.248 | 0.735 | 0.722 | yes |
| `diff -u --label old --label new include/linux/kernfs.h bench_out/v2.txt` | 3.028 |  | | | | | | | | 0.083 | 0.093 | 0.108 | yes |
| `diff include/linux/kernfs.h bench_out/v2.txt \| wc -l` | 2.863 |  | | | | | | | | 0.082 | 0.078 | 0.085 | yes |
| `diff -q include/linux/kernfs.h bench_out/same.txt` | 3.139 |  | | | | | | | | 0.021 | 0.016 | 0.019 | yes |
| `diff -u include/linux/kernfs.h bench_out/copy.txt \| wc -l` | 3.067 |  | | | | | | | | 0.239 | 0.280 | 0.218 | yes |
| `diff -rq Documentation bench_out/dir_copy` | 80.47 |  | | | | | | | | 3.735 | 3.443 | 3.477 | yes |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 16.41 |  | | | | | | | | 2.738 | 4.651 | 3.435 | yes |
| `diff -rq Documentation bench_out/dir_copy` | 80.81 |  | | | | | | | | 3.735 | 3.443 | 3.477 | yes |
| `diff -r include/linux bench_out/work \| wc -l` | 82.82 |  | | | | | | | | 63.53 | 73.17 | 70.77 | yes |
| `diff -ru include/linux bench_out/work \| grep -c '^@@'` | 109.6 |  | | | | | | | | 72.07 | 83.21 | 85.85 | yes |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 60.08 |  | | | | | | | | 24.30 | 34.22 | 37.62 | yes |
| `wc -l bench_out/loop.txt` | 3.593 |  | | | | | | | | 0.019 | 0.026 | 0.050 | yes |
| `for f in bench_out/work/*.h; do echo done >> "$f"; done` | 17.16 |  | | | | | | | | 3.166 | 4.982 | 4.773 | yes |
| `awk '{ print NR ": " $0 }' include/linux/kernfs.h > bench_out/numbered.txt` | 5.657 |  | | | | | | | | 0.433 | 1.045 | 1.069 | yes |
| `awk 'END { print NR }' bench_out/numbered.txt` | 4.560 |  | | | | | | | | 0.051 | 0.060 | 0.058 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.259 |  | 0.035 | 0.891 | 0.034 | 0.586 | | 0.640 | 0.473 | 0.026 | 0.360 | 0.397 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 3.912 |  | 0.015 | 0.687 | 0.021 | 0.769 | | 0.544 | 0.657 | 0.025 | 0.321 | 0.282 | yes |
| `ls -R bench_out/a` | 2.749 |  | 0.019 | 0.019 | 0.019 | 0.017 | | 0.017 | 0.022 | 0.017 | 0.022 | 0.018 | yes |
| `rm bench_out/a/b/hello.txt` | 2.906 |  | 0.011 | 0.583 | 0.019 | 0.497 | | 0.745 | 0.654 | 0.019 | 0.256 | 0.304 | yes |
| `rmdir bench_out/a/b` | 3.473 |  | 0.015 | 0.543 | 0.017 | 0.439 | | 0.539 | 0.437 | 0.019 | 0.192 | 0.462 | yes |
| `rm -r bench_out/dir_moved` | 131.5 |  | 8.592 | 9.350 | 7.356 | 5.433 | | 8.634 | 6.530 | 4.554 | 5.282 | 4.973 | yes |
| `rm -rf bench_out` | 133.5 |  | 3.696 | 24.41 | 0.812 | 16.54 | | 17.33 | 16.90 | 1.673 | 17.63 | 25.99 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, on disk, settled | Stage 4, in memory | Stage 4, on disk, settled | Stage 4, on disk, after the writes | Stage 4, on disk, 512 MB limit | Stage 4, on disk, 1 GB for everything, file cache included | Stage 5, in memory | Stage 5, on disk, settled | Stage 5, on disk, 512 MB limit |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 212 B | 1457 B | 1457 B | 3419 B | 3419 B | 3419 B | 3419 B | 3849 B | 3232 B | 3232 B | 3232 B |
| `ls -a` | 330 B | 2681 B | 2681 B | 5029 B | 5029 B | 5029 B | 5029 B | 5647 B | 4938 B | 4938 B | 4938 B |
| `ls -R` | 1377.7 KB | 74.3 KB | 74.3 KB | 95.2 KB | 95.2 KB | 95.2 KB | 95.2 KB | 150.9 KB | 95.1 KB | 95.1 KB | 95.1 KB |
| `ls -aR Documentation` | 216.8 KB | 56.6 KB | 56.6 KB | 68.2 KB | 68.2 KB | 68.2 KB | 68.2 KB | 84.0 KB | 68.2 KB | 68.2 KB | 68.2 KB |
| `ls -p Documentation` | 796 B | 4893 B | 4893 B | 7894 B | 7894 B | 7894 B | 7894 B | 9231 B | 7899 B | 7899 B | 7899 B |
| `find .` | 3590.9 KB | 19.2 KB | 19.2 KB | 21.0 KB | 21.0 KB | 21.0 KB | 21.0 KB | 21.0 KB | 20.9 KB | 20.9 KB | 20.9 KB |
| `find . -name '*.h'` | 969.5 KB | 22.7 KB | 22.7 KB | 24.7 KB | 24.7 KB | 24.7 KB | 24.7 KB | 24.7 KB | 24.8 KB | 24.8 KB | 24.8 KB |
| `find . -name '*.c'` | 1233.5 KB | 21.7 KB | 21.7 KB | 23.7 KB | 23.7 KB | 23.7 KB | 23.7 KB | 23.7 KB | 23.8 KB | 23.8 KB | 23.8 KB |
| `find . -type d` | 166.7 KB | 19.5 KB | 19.5 KB | 21.6 KB | 21.6 KB | 21.6 KB | 21.6 KB | 21.6 KB | 21.7 KB | 21.7 KB | 21.7 KB |
| `find . -type f` | 3424.2 KB | 32.8 KB | 32.8 KB | 34.9 KB | 34.9 KB | 34.9 KB | 34.9 KB | 34.9 KB | 35.0 KB | 35.0 KB | 35.0 KB |
| `find . -maxdepth 2 -type f` | 23.5 KB | 32.0 KB | 32.0 KB | 34.6 KB | 34.6 KB | 34.6 KB | 34.6 KB | 34.6 KB | 34.9 KB | 34.9 KB | 34.9 KB |
| `find Documentation -name '*.h' -type f` | 76 B | 1937 B | 1937 B | 4418 B | 4418 B | 4418 B | 4418 B | 4418 B | 4711 B | 4711 B | 4711 B |
| `find . -name 'Makefile'` | 100.1 KB | 26.9 KB | 26.9 KB | 28.9 KB | 28.9 KB | 28.9 KB | 28.9 KB | 28.9 KB | 29.0 KB | 29.0 KB | 29.0 KB |
| `cat include/linux/kernfs.h` | 19.9 KB | 994 B | 994 B | 2407 B | 2407 B | 2407 B | 2407 B | 2407 B | 2324 B | 2324 B | 2324 B |
| `cat -n include/linux/kernfs.h` | 24.4 KB | 26.8 KB | 26.8 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB |
| `head -n 20 include/linux/kernfs.h` | 459 B | 1345 B | 1345 B | 3423 B | 3423 B | 3423 B | 3423 B | 3423 B | 3524 B | 3524 B | 3524 B |
| `tail -n 20 include/linux/kernfs.h` | 478 B | 1364 B | 1364 B | 3442 B | 3442 B | 3442 B | 3442 B | 3442 B | 3543 B | 3543 B | 3543 B |
| `tail -n +100 include/linux/kernfs.h` | 16.9 KB | 1306 B | 1306 B | 2966 B | 2966 B | 2966 B | 2966 B | 2966 B | 3075 B | 3075 B | 3075 B |
| `wc include/linux/kernfs.h` | 41 B | 1002 B | 1002 B | 2602 B | 2602 B | 2602 B | 2602 B | 2602 B | 2511 B | 2511 B | 2511 B |
| `wc -l include/linux/*.h` | 49.6 KB | 237.0 KB | 237.0 KB | 237.7 KB | 237.7 KB | 237.7 KB | 237.7 KB | 237.7 KB | 238.4 KB | 238.4 KB | 238.4 KB |
| `cat include/linux/*.h \| wc -l` | 7 B | 237.2 KB | 237.2 KB | 238.1 KB | 238.1 KB | 238.1 KB | 238.1 KB | 238.1 KB | 239.0 KB | 239.0 KB | 239.0 KB |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 447 B | 4720 B | 4944 B | 7543 B | 7543 B | 7543 B | 7543 B | 7543 B | 10.1 KB | 10.1 KB | 10.1 KB |
| `grep -c define include/linux/kernfs.h` | 2 B | 9470 B | 9470 B | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.7 KB | 15.7 KB | 15.7 KB |
| `grep -n define include/linux/kernfs.h` | 374 B | 9470 B | 9470 B | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.7 KB | 15.7 KB | 15.7 KB |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 19.9 KB | 27.0 KB | 27.0 KB | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB | 17.3 KB | 17.3 KB | 17.3 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 19.9 KB | 26.6 KB | 26.6 KB | 30.7 KB | 30.7 KB | 30.7 KB | 30.7 KB | 30.7 KB | 34.3 KB | 34.3 KB | 34.3 KB |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 6 B | 25.8 KB | 26.0 KB | 29.5 KB | 29.5 KB | 29.5 KB | 29.5 KB | 29.5 KB | 33.3 KB | 33.3 KB | 33.3 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.7 KB | 31.7 KB | 31.7 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.5 KB | 31.7 KB | 31.7 KB | 31.7 KB |
| `grep -rn ausgabenspezifisches .` | 116 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB | 42.1 KB | 42.1 KB | 42.1 KB |
| `grep -rl ausgabenspezifisches .` | 32 B | 27.8 KB | 27.8 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB | 41.9 KB | 42.1 KB | 42.1 KB | 42.1 KB |
| `grep -rn mispredicted .` | 111.0 KB | 90.8 KB | 51.9 KB | 54.1 KB | 54.1 KB | 54.7 KB | 54.1 KB | 54.1 KB | 54.2 KB | 54.2 KB | 54.2 KB |
| `grep -rl mispredicted .` | 4531 B | 90.8 KB | 51.9 KB | 54.1 KB | 54.1 KB | 54.7 KB | 54.1 KB | 54.1 KB | 54.2 KB | 54.2 KB | 54.2 KB |
| `grep -rn separately .` | 103.9 KB | 343.4 KB | 279.6 KB | 281.8 KB | 281.8 KB | 282.2 KB | 281.8 KB | 281.8 KB | 281.9 KB | 281.9 KB | 281.9 KB |
| `grep -rl separately .` | 30.7 KB | 342.9 KB | 279.0 KB | 281.3 KB | 281.3 KB | 281.7 KB | 281.3 KB | 281.3 KB | 281.4 KB | 281.4 KB | 281.4 KB |
| `grep -rn conditions .` | 1360.5 KB | 136.2 KB | 191.1 KB | 193.4 KB | 193.4 KB | 193.4 KB | 193.4 KB | 193.4 KB | 193.5 KB | 193.5 KB | 193.5 KB |
| `grep -rl conditions .` | 389.6 KB | 133.0 KB | 189.4 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.8 KB | 191.8 KB | 191.8 KB |
| `grep -rn define .` | 821.1 MB | 26.9 KB | 26.9 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.2 KB | 30.8 KB | 29.3 KB | 29.3 KB | 29.3 KB |
| `grep -rl define .` | 1867.6 KB | 9418 B | 9418 B | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.6 KB | 15.6 KB | 15.6 KB |
| `grep -rc separately .` | 3591.0 KB | 361.3 KB | 297.5 KB | 299.8 KB | 299.8 KB | 300.1 KB | 299.8 KB | 299.8 KB | 299.9 KB | 299.9 KB | 299.9 KB |
| `grep -rin MISPREDICTED .` | 135.7 KB | 90.8 KB | 51.8 KB | 54.1 KB | 54.1 KB | 54.7 KB | 54.1 KB | 54.1 KB | 54.2 KB | 54.2 KB | 54.2 KB |
| `grep -ri separately .` | 100.6 KB | 343.3 KB | 279.5 KB | 281.7 KB | 281.7 KB | 282.1 KB | 281.7 KB | 281.7 KB | 281.8 KB | 281.8 KB | 281.8 KB |
| `grep -rn separately Documentation` | 19.9 KB | 35.4 KB | 43.8 KB | 46.1 KB | 46.1 KB | 46.5 KB | 46.1 KB | 46.1 KB | 46.2 KB | 46.2 KB | 46.2 KB |
| `grep -rl conditions Documentation` | 9541 B | 133.0 KB | 189.4 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.7 KB | 191.8 KB | 191.8 KB | 191.8 KB |
| `grep -rF 'separately' .` | 99.7 KB | 343.3 KB | 279.5 KB | 281.7 KB | 281.7 KB | 282.1 KB | 281.7 KB | 281.7 KB | 281.8 KB | 281.8 KB | 281.8 KB |
| `grep -rE 'atomic_[a-z]+\(' .` | 1294.9 KB | 945.1 KB | 1037.7 KB | 1040.0 KB | 1040.0 KB | 1040.9 KB | 1040.0 KB | 1040.0 KB | 1040.1 KB | 1040.1 KB | 1040.1 KB |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 109.2 KB | 62.6 KB | 53.6 KB | 64.1 KB | 64.1 KB | 64.1 KB | 64.1 KB | 64.1 KB | 64.3 KB | 64.3 KB | 64.3 KB |
| `grep -rE 'conditions\|separately' .` | 1427.8 KB | 188.8 KB | 195.7 KB | 198.0 KB | 198.0 KB | 200.1 KB | 198.0 KB | 198.0 KB | 198.1 KB | 198.1 KB | 198.1 KB |
| `grep -rE '[0-9]{4,}' .` | 467.5 MB | 28.6 KB | 28.6 KB | 30.8 KB | 30.8 KB | 30.8 KB | 30.8 KB | 30.8 KB | 30.9 KB | 30.9 KB | 30.9 KB |
| `grep -rn '^#include' .` | 24.9 MB | 29.6 KB | 29.6 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 31.9 KB | 32.0 KB | 32.0 KB | 32.0 KB |
| `grep -rE '^(import\|from) ' .` | 116.4 KB | 21.0 KB | 21.0 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.3 KB | 30.3 KB | 30.3 KB |
| `grep -rin 'return [a-z_]*;$' Documentation` | 24.1 KB | 17.5 KB | 729.5 KB | 716.4 KB | 732.0 KB | 732.0 KB | 732.0 KB | 732.0 KB | 716.6 KB | 732.1 KB | 732.1 KB |
| `grep -rv e drivers/usb` | 9638.5 KB | 21.9 KB | 21.9 KB | 24.1 KB | 24.1 KB | 24.1 KB | 24.1 KB | 24.1 KB | 24.2 KB | 24.2 KB | 24.2 KB |
| `grep -r '\(separately\).*\1' .` | 188 B | 346.3 KB | 282.5 KB | 284.8 KB | 284.8 KB | 285.1 KB | 284.8 KB | 284.8 KB | 284.9 KB | 284.9 KB | 284.9 KB |
| `grep -n mispredicted include/linux/*.h` | 0 B | | 237.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 239.1 KB | 239.1 KB | 239.1 KB |
| `grep -l separately include/linux/*.h` | 458 B | | 237.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 239.0 KB | 239.0 KB | 239.0 KB |
| `grep -c qdbzzabsentterm include/linux/*.h` | 39.5 KB | | 237.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 238.3 KB | 239.1 KB | 239.1 KB | 239.1 KB |
| `grep -rl conditions . \| wc -l` | 5 B | 133.7 KB | 190.3 KB | 192.6 KB | 192.6 KB | 192.6 KB | 192.6 KB | 192.6 KB | 194.2 KB | 194.2 KB | 194.2 KB |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 6 B | 26.0 KB | 26.2 KB | 29.1 KB | 29.1 KB | 29.1 KB | 29.1 KB | 29.1 KB | 31.8 KB | 31.8 KB | 31.8 KB |
| `find . -name '*.h' \| wc -l` | 6 B | 23.4 KB | 23.6 KB | 25.7 KB | 25.7 KB | 25.7 KB | 25.7 KB | 25.7 KB | 27.3 KB | 27.3 KB | 27.3 KB |
| `find . -type f \| grep -c h` | 6 B | 35.8 KB | 35.9 KB | 39.4 KB | 39.4 KB | 39.4 KB | 39.4 KB | 39.4 KB | 41.3 KB | 41.3 KB | 41.3 KB |
| `grep -rn define . \| head -n 5` | 473 B | 10.3 KB | 10.4 KB | 16.6 KB | 16.6 KB | 16.6 KB | 16.6 KB | 16.6 KB | 18.4 KB | 18.4 KB | 18.4 KB |
| `grep -rn conditions . \| head -n 1` | 107 B | 15.3 KB | 15.4 KB | 23.9 KB | 23.9 KB | 23.9 KB | 23.9 KB | 23.9 KB | 25.7 KB | 25.7 KB | 25.7 KB |
| `find . -type f \| head -n 3` | 47 B | 32.6 KB | 32.8 KB | 35.1 KB | 35.1 KB | 35.1 KB | 35.1 KB | 35.1 KB | 36.8 KB | 36.8 KB | 36.8 KB |
| `cat include/linux/*.h \| head -n 2` | 42 B | 237.3 KB | 237.3 KB | 238.4 KB | 238.4 KB | 238.4 KB | 238.4 KB | 238.4 KB | 239.4 KB | 239.4 KB | 239.4 KB |
| `grep -rno separately .` | 53.2 KB | | | 288.9 KB | 288.9 KB | 289.2 KB | 288.9 KB | 288.9 KB | 295.4 KB | 295.4 KB | 295.4 KB |
| `grep -rw separately .` | 99.7 KB | | | 295.1 KB | 295.1 KB | 295.4 KB | 295.1 KB | 295.1 KB | 295.2 KB | 295.2 KB | 295.2 KB |
| `grep -rnw mispredicted .` | 111.0 KB | | | 65.9 KB | 65.9 KB | 66.5 KB | 65.9 KB | 65.9 KB | 66.0 KB | 66.0 KB | 66.0 KB |
| `grep -rn -C 2 mispredicted .` | 269.5 KB | | | 56.2 KB | 56.2 KB | 56.8 KB | 56.2 KB | 56.2 KB | 56.5 KB | 56.5 KB | 56.5 KB |
| `grep -rn -A 1 -B 1 separately .` | 278.0 KB | | | 16.6 MB | 16.6 MB | 16.6 MB | 16.6 MB | 16.6 MB | 16.6 MB | 16.6 MB | 16.6 MB |
| `grep -rho 'atomic_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 207 B | | | 3082.3 KB | 3082.3 KB | 3083.2 KB | 3082.3 KB | 3082.3 KB | 3091.5 KB | 3091.5 KB | 3091.5 KB |
| `grep -rhow '[a-z]*mispredicted[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 21 B | | | 118.3 KB | 118.3 KB | 118.3 KB | 118.3 KB | 118.3 KB | 121.8 KB | 121.8 KB | 121.8 KB |
| `rg -j 1 qdbzzabsentterm` | 0 B | | | 36.7 KB | 36.7 KB | 36.7 KB | 36.7 KB | 36.7 KB | 36.8 KB | 36.8 KB | 36.8 KB |
| `rg -j 1 ausgabenspezifisches` | 110 B | | | 56.5 KB | 56.5 KB | 56.5 KB | 56.5 KB | 56.5 KB | 56.6 KB | 56.6 KB | 56.6 KB |
| `rg -j 1 mispredicted` | 108.1 KB | | | 166.2 KB | 166.2 KB | 166.7 KB | 166.2 KB | 166.2 KB | 166.3 KB | 166.3 KB | 166.3 KB |
| `rg -j 1 separately` | 97.7 KB | | | 462.5 KB | 462.5 KB | 462.9 KB | 462.5 KB | 462.5 KB | 462.7 KB | 462.7 KB | 462.7 KB |
| `rg -j 1 conditions` | 1304.4 KB | | | 1475.9 KB | 1475.9 KB | 1475.9 KB | 1475.9 KB | 1475.9 KB | 1476.0 KB | 1476.0 KB | 1476.0 KB |
| `rg -j 1 define` | 782.9 MB | | | 1414.4 KB | 1414.4 KB | 1414.4 KB | 1414.4 KB | 1414.4 KB | 1414.6 KB | 1414.6 KB | 1414.6 KB |
| `rg -j 1 -l separately` | 29.1 KB | | | 463.0 KB | 463.0 KB | 463.4 KB | 463.0 KB | 463.0 KB | 463.2 KB | 463.2 KB | 463.2 KB |
| `rg -j 1 -l define` | 1770.4 KB | | | 1414.6 KB | 1414.6 KB | 1414.6 KB | 1414.6 KB | 1414.6 KB | 1414.9 KB | 1414.9 KB | 1414.9 KB |
| `rg -j 1 -c conditions` | 389.6 KB | | | 1476.3 KB | 1476.3 KB | 1476.3 KB | 1476.3 KB | 1476.3 KB | 1476.6 KB | 1476.6 KB | 1476.6 KB |
| `rg -j 1 -i MISPREDICTED` | 131.7 KB | | | 186.3 KB | 186.3 KB | 186.9 KB | 186.3 KB | 186.3 KB | 186.6 KB | 186.6 KB | 186.6 KB |
| `rg -j 1 -w separately` | 97.7 KB | | | 482.8 KB | 482.8 KB | 483.2 KB | 482.8 KB | 482.8 KB | 483.1 KB | 483.1 KB | 483.1 KB |
| `rg -j 1 -F 'separately'` | 97.7 KB | | | 463.0 KB | 463.0 KB | 463.4 KB | 463.0 KB | 463.0 KB | 463.2 KB | 463.2 KB | 463.2 KB |
| `rg -j 1 -n -C 2 mispredicted` | 264.9 KB | | | 167.1 KB | 167.1 KB | 167.7 KB | 167.1 KB | 167.1 KB | 167.6 KB | 167.6 KB | 167.6 KB |
| `rg -j 1 'atomic_[a-z]+\('` | 1261.8 KB | | | 1044.0 KB | 1044.0 KB | 1044.9 KB | 1044.0 KB | 1044.0 KB | 1044.2 KB | 1044.2 KB | 1044.2 KB |
| `rg -j 1 'mispredicted\|ausgabenspezifisches'` | 108.2 KB | | | 193.7 KB | 193.7 KB | 194.2 KB | 193.7 KB | 193.7 KB | 193.8 KB | 193.8 KB | 193.8 KB |
| `rg -j 1 '\bmispredicted\w*'` | 108.1 KB | | | 2592.9 KB | 2592.9 KB | 2592.9 KB | 2592.9 KB | 2592.9 KB | 2593.1 KB | 2593.1 KB | 2593.1 KB |
| `rg -j 1 -t c separately` | 72.4 KB | | | 461.0 KB | 461.0 KB | 461.3 KB | 461.0 KB | 461.0 KB | 461.3 KB | 461.3 KB | 461.3 KB |
| `rg -j 1 -g '*.h' separately` | 15.8 KB | | | 460.9 KB | 460.9 KB | 461.2 KB | 460.9 KB | 460.9 KB | 461.2 KB | 461.2 KB | 461.2 KB |
| `rg -j 1 separately Documentation` | 19.3 KB | | | 202.5 KB | 202.5 KB | 202.8 KB | 202.5 KB | 202.5 KB | 202.7 KB | 202.7 KB | 202.7 KB |
| `rg -j 1 -uu separately` | 97.7 KB | | | 287.8 KB | 287.8 KB | 288.2 KB | 287.8 KB | 287.8 KB | 288.1 KB | 288.1 KB | 288.1 KB |
| `rg -j 1 --sort path mispredicted` | 108.1 KB | | | 166.9 KB | 166.9 KB | 167.5 KB | 166.9 KB | 166.9 KB | 167.2 KB | 167.2 KB | 167.2 KB |
| `rg -j 1 --files \| wc -l` | 6 B | | | 1415.1 KB | 1415.1 KB | 1415.1 KB | 1415.1 KB | 1415.1 KB | 1416.7 KB | 1416.7 KB | 1416.7 KB |
| `rg -j 1 -l conditions \| wc -l` | 5 B | | | 1477.3 KB | 1477.3 KB | 1477.3 KB | 1477.3 KB | 1477.3 KB | 1479.0 KB | 1479.0 KB | 1479.0 KB |
| `ls -ltR` | 4983.7 KB | | | 147.8 KB | 147.8 KB | 147.8 KB | 147.8 KB | 163.7 KB | 147.7 KB | 147.7 KB | 147.7 KB |
| `ls -lt include/linux` | 80.2 KB | | | 116.4 KB | 116.4 KB | 116.4 KB | 116.4 KB | 132.0 KB | 116.4 KB | 116.4 KB | 116.4 KB |
| `ls -ltr Documentation \| tail -n 3` | 138 B | | | 15.6 KB | 15.6 KB | 15.6 KB | 15.6 KB | 16.2 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `find . -newer include/linux/kernfs.h` | 705.7 KB | | | 20.1 KB | 20.1 KB | 20.1 KB | 20.1 KB | 20.1 KB | 20.2 KB | 20.2 KB | 20.2 KB |
| `find . -newer include/linux/kernfs.h -name '*.h'` | 200.2 KB | | | 34.3 KB | 34.3 KB | 34.3 KB | 34.3 KB | 34.3 KB | 34.6 KB | 34.6 KB | 34.6 KB |
| `find . -type f -mtime +30` | 0 B | | | 4987 B | 4987 B | 4987 B | 4987 B | 4987 B | 5280 B | 5280 B | 5280 B |
| `du -s .` | 10 B | | | 2705 B | 2705 B | 2705 B | 2705 B | 2705 B | 2710 B | 2710 B | 2710 B |
| `du -sh Documentation` | 18 B | | | 2774 B | 2774 B | 2774 B | 2774 B | 2774 B | 2779 B | 2779 B | 2779 B |
| `du -sb .` | 13 B | | | 2713 B | 2713 B | 2713 B | 2713 B | 2713 B | 2718 B | 2718 B | 2718 B |
| `du -sb Documentation` | 23 B | | | 2774 B | 2774 B | 2774 B | 2774 B | 2774 B | 2779 B | 2779 B | 2779 B |
| `du -b -d 1 Documentation` | 2106 B | | | 8740 B | 8740 B | 8740 B | 8740 B | 8740 B | 8937 B | 8937 B | 8937 B |
| `sort include/linux/kernfs.h` | 19.9 KB | | | 114.8 KB | 114.8 KB | 114.8 KB | 114.8 KB | 114.8 KB | 114.7 KB | 114.7 KB | 114.7 KB |
| `sort -r include/linux/kernfs.h \| head -n 3` | 9 B | | | 116.2 KB | 116.2 KB | 116.2 KB | 116.2 KB | 116.2 KB | 117.8 KB | 117.8 KB | 117.8 KB |
| `sort -u include/linux/kernfs.h \| wc -l` | 4 B | | | 116.0 KB | 116.0 KB | 116.0 KB | 116.0 KB | 116.0 KB | 117.5 KB | 117.5 KB | 117.5 KB |
| `sort -k 2 include/linux/kernfs.h \| tail -n 3` | 97 B | | | 136.5 KB | 136.5 KB | 136.5 KB | 136.5 KB | 136.5 KB | 138.1 KB | 138.1 KB | 138.1 KB |
| `cat include/linux/*.h \| sort \| uniq -c \| sort -rn \| head -n 5` | 52 B | | | 56.9 MB | 56.9 MB | 56.9 MB | 56.9 MB | 56.9 MB | 56.9 MB | 56.9 MB | 56.9 MB |
| `uniq -c include/linux/kernfs.h \| wc -l` | 4 B | | | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB | 30.1 KB | 31.5 KB | 31.5 KB | 31.5 KB |
| `cut -c 1-20 include/linux/kernfs.h` | 9431 B | | | 22.9 KB | 22.9 KB | 22.9 KB | 22.9 KB | 22.9 KB | 23.0 KB | 23.0 KB | 23.0 KB |
| `cut -d ' ' -f 1 include/linux/kernfs.h \| sort \| uniq \| wc -l` | 3 B | | | 106.7 KB | 106.7 KB | 106.7 KB | 106.7 KB | 106.7 KB | 110.2 KB | 110.2 KB | 110.2 KB |
| `tr a-z A-Z < include/linux/kernfs.h` | 19.9 KB | | | 24.9 KB | 24.9 KB | 24.9 KB | 24.9 KB | 24.9 KB | 25.0 KB | 25.0 KB | 25.0 KB |
| `tr -d '\n' < include/linux/kernfs.h \| wc -c` | 6 B | | | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB | 27.5 KB | 27.5 KB | 27.5 KB |
| `tr -s ' ' < include/linux/kernfs.h \| wc -c` | 6 B | | | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB | 25.9 KB | 27.4 KB | 27.4 KB | 27.4 KB |
| `nl include/linux/kernfs.h` | 24.4 KB | | | 28.2 KB | 28.2 KB | 28.2 KB | 28.2 KB | 28.2 KB | 28.1 KB | 28.1 KB | 28.1 KB |
| `nl -ba include/linux/kernfs.h \| tail -n 3` | 54 B | | | 54.1 KB | 54.1 KB | 54.1 KB | 54.1 KB | 54.1 KB | 55.6 KB | 55.6 KB | 55.6 KB |
| `seq 1 100000 \| wc -l` | 7 B | | | 135.0 KB | 135.0 KB | 135.0 KB | 135.0 KB | 135.0 KB | 136.4 KB | 136.4 KB | 136.4 KB |
| `seq -w 1 1000 \| tail -n 2` | 10 B | | | 17.6 KB | 17.6 KB | 17.6 KB | 17.6 KB | 17.6 KB | 19.2 KB | 19.2 KB | 19.2 KB |
| `basename include/linux/kernfs.h` | 9 B | | | 2444 B | 2444 B | 2444 B | 2444 B | 2444 B | 2353 B | 2353 B | 2353 B |
| `basename -s .h include/linux/kernfs.h` | 7 B | | | 3098 B | 3098 B | 3098 B | 3098 B | 3098 B | 3199 B | 3199 B | 3199 B |
| `dirname include/linux/kernfs.h` | 14 B | | | 2453 B | 2453 B | 2453 B | 2453 B | 2453 B | 2362 B | 2362 B | 2362 B |
| `printf '%s\n' a b c` | 6 B | | | 3325 B | 3325 B | 3325 B | 3325 B | 3325 B | 3522 B | 3522 B | 3522 B |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 18 B | | | 3422 B | 3422 B | 3422 B | 3422 B | 3422 B | 3619 B | 3619 B | 3619 B |
| `test -f include/linux/kernfs.h && echo yes` | 4 B | | | 3547 B | 3547 B | 3547 B | 3547 B | 3547 B | 3832 B | 3832 B | 3832 B |
| `[ -d Documentation ] && echo folder` | 7 B | | | 3788 B | 3788 B | 3788 B | 3788 B | 3788 B | 4169 B | 4169 B | 4169 B |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 35 B | | | 3492 B | 3492 B | 3492 B | 3492 B | 3492 B | 3709 B | 3709 B | 3709 B |
| `echo a b c \| xargs -n 1 echo` | 6 B | | | 4674 B | 4674 B | 4674 B | 4674 B | 4674 B | 6367 B | 6367 B | 6367 B |
| `find drivers/usb -name '*.h' \| xargs cat \| wc -l` | 6 B | | | 109.4 KB | 109.4 KB | 109.4 KB | 109.4 KB | 109.4 KB | 73.7 KB | 73.7 KB | 73.7 KB |
| `find . -name '*.h' \| xargs grep -l separately \| wc -l` | 4 B | | | 6391.8 KB | 6391.8 KB | 6391.8 KB | 6391.8 KB | 6391.8 KB | 1903.5 KB | 1903.5 KB | 1903.5 KB |
| `echo {a,b,c}{1..3}` | 27 B | | | 4485 B | 4485 B | 4485 B | 4485 B | 4485 B | 5917 B | 5917 B | 5917 B |
| `echo {1..20..3} {a..e}` | 28 B | | | 3967 B | 3967 B | 3967 B | 3967 B | 3967 B | 5303 B | 5303 B | 5303 B |
| `echo $(wc -l < include/linux/kernfs.h) `basename include/linux/kernfs.h`` | 13 B | | | 6361 B | 6361 B | 6361 B | 6361 B | 6361 B | 7947 B | 7947 B | 7947 B |
| `echo "first line: $(head -n 1 include/linux/kernfs.h)"` | 56 B | | | 4984 B | 4984 B | 4984 B | 4984 B | 4984 B | 6613 B | 6613 B | 6613 B |
| `wc -l < include/linux/kernfs.h` | 4 B | | | 3332 B | 3332 B | 3332 B | 3332 B | 3332 B | 3353 B | 3353 B | 3353 B |
| `cat include/linux/kernfs.h > /dev/null 2>&1` | 0 B | | | 3433 B | 3433 B | 3433 B | 3433 B | 3433 B | 3470 B | 3470 B | 3470 B |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 6 B | | | 4158 B | 4158 B | 4158 B | 4158 B | 4158 B | 4419 B | 4419 B | 4419 B |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 2 B | | | 4072 B | 4072 B | 4072 B | 4072 B | 4072 B | 5445 B | 5445 B | 5445 B |
| `grep -q define include/linux/kernfs.h && echo found \|\| echo none` | 6 B | | | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB | 17.1 KB | 17.9 KB | 17.9 KB | 17.9 KB |
| `false \|\| echo fallback` | 9 B | | | 2861 B | 2861 B | 2861 B | 2861 B | 2861 B | 2970 B | 2970 B | 2970 B |
| `true && grep -c define include/linux/kernfs.h` | 2 B | | | 15.9 KB | 15.9 KB | 15.9 KB | 15.9 KB | 15.9 KB | 16.3 KB | 16.3 KB | 16.3 KB |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 14 B | | | 4178 B | 4178 B | 4178 B | 4178 B | 4178 B | 5376 B | 5376 B | 5376 B |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 2 B | | | 3130 B | 3130 B | 3130 B | 3130 B | 3130 B | 3079 B | 3079 B | 3079 B |
| `echo one \⏎two` | 8 B | | | 2591 B | 2591 B | 2591 B | 2591 B | 2591 B | 2596 B | 2596 B | 2596 B |
| `grep -c define include/linux/kernfs.h⏎wc -l include/linux/kernfs.h` | 29 B | | | 16.5 KB | 16.5 KB | 16.5 KB | 16.5 KB | 16.5 KB | 17.1 KB | 17.1 KB | 17.1 KB |
| `for i in $(seq 100000); do :; done` | 0 B | | | | | | | | 7815.6 KB | 7815.6 KB | 7815.6 KB |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 6 B | | | | | | | | 7252 B | 7252 B | 7252 B |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 2 B | | | | | | | | 7249 B | 7249 B | 7249 B |
| `for f in include/linux/*.h; do echo "$f"; done \| wc -l` | 5 B | | | | | | | | 153.6 KB | 153.6 KB | 153.6 KB |
| `for f in include/linux/*.h; do grep -c define "$f"; done \| sort -n \| tail -n 3` | 15 B | | | | | | | | 160.2 KB | 160.2 KB | 160.2 KB |
| `n=0; for f in include/linux/*.h; do if grep -q conditions "$f"; then n=$((n+1)); fi; done; echo $n` | 3 B | | | | | | | | 154.9 KB | 154.9 KB | 154.9 KB |
| `for f in include/linux/*.h; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 0 B | | | | | | | | 153.9 KB | 153.9 KB | 153.9 KB |
| `H=$PWD; for d in Documentation drivers/usb include/linux; do cd "$d"; ls \| wc -l; cd "$H"; done` | 11 B | | | | | | | | 95.6 KB | 95.6 KB | 95.6 KB |
| `X=define; N=$(grep -c "$X" include/linux/kernfs.h); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 15 B | | | | | | | | 19.1 KB | 19.1 KB | 19.1 KB |
| `find drivers/usb -name '*.h' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 5 B | | | | | | | | 102.8 KB | 102.8 KB | 102.8 KB |
| `for f in include/linux/*.h; do head -n 1 "$f"; done > /dev/null` | 0 B | | | | | | | | 152.9 KB | 152.9 KB | 152.9 KB |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 12 B | | | | | | | | 9721 B | 9721 B | 9721 B |
| `awk '{ print $2 }' include/linux/kernfs.h` | 4720 B | | | | | | | | 17.0 KB | 17.0 KB | 17.0 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 7 B | | | | | | | | 241.3 KB | 241.3 KB | 241.3 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '/define/ { n++ } END { print n }'` | 6 B | | | | | | | | 240.4 KB | 240.4 KB | 240.4 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 15 B | | | | | | | | 240.4 KB | 240.4 KB | 240.4 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 5 B | | | | | | | | 240.7 KB | 240.7 KB | 240.7 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 2 B | | | | | | | | 241.3 KB | 241.3 KB | 241.3 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 4 B | | | | | | | | 240.4 KB | 240.4 KB | 240.4 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 6 B | | | | | | | | 3420.7 KB | 3420.7 KB | 3420.7 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "define") n++ } END { print n }'` | 4 B | | | | | | | | 240.5 KB | 240.5 KB | 240.5 KB |
| `awk '{ gsub(/[0-9]+/, "N"); print }' include/linux/kernfs.h \| wc -c` | 6 B | | | | | | | | 39.3 KB | 39.3 KB | 39.3 KB |
| `awk -v OFS=- '{ $1 = NR; print }' include/linux/kernfs.h \| tail -n 1` | 27 B | | | | | | | | 49.6 KB | 49.6 KB | 49.6 KB |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 16 B | | | | | | | | 5894 B | 5894 B | 5894 B |
| `grep -rn separately . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 215 B | | | | | | | | 296.6 KB | 296.6 KB | 296.6 KB |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 83 B | | | | | | | | 12.2 MB | 12.2 MB | 12.2 MB |
| `mkdir -p bench_out/a/b` | 0 B | 1459 B | 1747 B | 3711 B | 3711 B | | 3711 B | 3711 B | 3716 B | 3716 B | 3716 B |
| `touch bench_out/empty.txt` | 0 B | 979 B | 979 B | 2566 B | 2566 B | | 2566 B | 2566 B | 2475 B | 2475 B | 2475 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1242 B | 1292 B | 3665 B | 3661 B | | 3661 B | 3661 B | 3710 B | 3706 B | 3706 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1246 B | 1246 B | 3485 B | 3473 B | | 3473 B | 3473 B | 3530 B | 3518 B | 3518 B |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 0 B | 26.0 KB | 283.2 KB | 3270 B | 3270 B | | 3270 B | 3270 B | 3275 B | 3275 B | 3275 B |
| `cp -r Documentation bench_out/dir_copy` | 0 B | 59.4 MB | 21.7 MB | 4208.5 KB | 6328.2 KB | | 6328.2 KB | 6328.2 KB | 4208.6 KB | 6328.4 KB | 6328.4 KB |
| `cp -r include/linux bench_out/work` | 0 B | 22.2 MB | 13.6 MB | 1155.8 KB | 1403.4 KB | | 1403.4 KB | 1403.4 KB | 1155.9 KB | 1403.7 KB | 1403.7 KB |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 27.6 KB | 47.8 KB | 51.1 KB | 293.3 KB | | 293.3 KB | 293.3 KB | 56.1 KB | 298.2 KB | 298.2 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 25.8 KB | 47.6 KB | 51.2 KB | 51.2 KB | | 51.2 KB | 51.2 KB | 53.7 KB | 53.7 KB | 53.7 KB |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 0 B | 6764.6 KB | 6254.3 KB | 11.6 MB | 5427.3 KB | | 5427.3 KB | 5427.3 KB | 11.6 MB | 5432.3 KB | 5432.3 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 0 B | | 250.1 KB | 252.1 KB | 252.1 KB | | 252.1 KB | 252.1 KB | 253.0 KB | 253.0 KB | 253.0 KB |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 0 B | | 248.6 KB | 250.6 KB | 250.6 KB | | 250.6 KB | 250.6 KB | 251.5 KB | 251.5 KB | 251.5 KB |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 B | | 20.8 KB | 2850 B | 2850 B | | 2850 B | 2850 B | 2855 B | 2855 B | 2855 B |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 B | | 20.8 KB | 2850 B | 2850 B | | 2850 B | 2850 B | 2855 B | 2855 B | 2855 B |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 0 B | | 21.0 KB | 23.4 KB | 23.4 KB | | 23.4 KB | 23.4 KB | 23.4 KB | 23.4 KB | 23.4 KB |
| `grep -rc QDBBENCHMANY bench_out` | 687.0 KB | 191.9 KB | 206.0 KB | 208.2 KB | 208.2 KB | | 208.2 KB | 208.2 KB | 208.3 KB | 208.3 KB | 208.3 KB |
| `grep -rl QDBBENCHMANY .` | 39.7 KB | 164.6 KB | 178.7 KB | 180.9 KB | 180.9 KB | | 180.9 KB | 180.9 KB | 181.0 KB | 181.0 KB | 181.0 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 16.0 KB | 16.0 KB | 24.8 KB | 24.8 KB | | 24.8 KB | 24.8 KB | 25.0 KB | 25.0 KB | 25.0 KB |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | 4 B | | | 44.6 KB | 44.6 KB | | 44.6 KB | 44.6 KB | 46.9 KB | 46.9 KB | 46.9 KB |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | 0 B | | | 135.6 KB | 135.6 KB | | 135.6 KB | 135.6 KB | 135.6 KB | 135.6 KB | 135.6 KB |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | 0 B | | | 134.4 KB | 134.4 KB | | 134.4 KB | 134.4 KB | 134.5 KB | 134.5 KB | 134.5 KB |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 B | | | 3783 B | 3783 B | | 3783 B | 3783 B | 3828 B | 3828 B | 3828 B |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 0 B | | | 3781 B | 3714 B | | 3714 B | 3714 B | 3826 B | 3759 B | 3759 B |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | 0 B | | | 2433.6 KB | 2429.2 KB | | 2429.2 KB | 2429.2 KB | 2436.2 KB | 2431.7 KB | 2431.7 KB |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 B | | | 8382 B | 8382 B | | 8382 B | 8382 B | 8387 B | 8387 B | 8387 B |
| `sed -i '1d' bench_out/joined2.txt` | 0 B | | | 151.1 KB | 151.1 KB | | 151.1 KB | 151.1 KB | 151.2 KB | 151.2 KB | 151.2 KB |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 128 B | | | 4183 B | 4183 B | | 4183 B | 4183 B | 4476 B | 4476 B | 4476 B |
| `cat bench_out/both.txt` | 98 B | | | 2497 B | 2497 B | | 2497 B | 2497 B | 2406 B | 2406 B | 2406 B |
| `cp include/linux/kernfs.h bench_out/v2.txt` | 0 B | | | | | | | | 3099 B | 3099 B | 3099 B |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 0 B | | | | | | | | 51.0 KB | 51.0 KB | 51.0 KB |
| `diff -u --label old --label new include/linux/kernfs.h bench_out/v2.txt` | 866 B | | | | | | | | 19.0 KB | 19.0 KB | 19.0 KB |
| `diff include/linux/kernfs.h bench_out/v2.txt \| wc -l` | 3 B | | | | | | | | 18.9 KB | 18.9 KB | 18.9 KB |
| `diff -q include/linux/kernfs.h bench_out/same.txt` | 0 B | | | | | | | | 3307 B | 3307 B | 3307 B |
| `diff -u include/linux/kernfs.h bench_out/copy.txt \| wc -l` | 4 B | | | | | | | | 107.3 KB | 107.3 KB | 107.3 KB |
| `diff -rq Documentation bench_out/dir_copy` | 0 B | | | | | | | | 59.4 KB | 59.4 KB | 59.4 KB |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 0 B | | | | | | | | 1516.2 KB | 1516.2 KB | 1516.2 KB |
| `diff -rq Documentation bench_out/dir_copy` | 72 B | | | | | | | | 59.4 KB | 59.4 KB | 59.4 KB |
| `diff -r include/linux bench_out/work \| wc -l` | 6 B | | | | | | | | 979.3 KB | 979.3 KB | 979.3 KB |
| `diff -ru include/linux bench_out/work \| grep -c '^@@'` | 5 B | | | | | | | | 985.4 KB | 985.4 KB | 985.4 KB |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 0 B | | | | | | | | 11.6 MB | 1541.0 KB | 1541.0 KB |
| `wc -l bench_out/loop.txt` | 25 B | | | | | | | | 2829 B | 2829 B | 2829 B |
| `for f in bench_out/work/*.h; do echo done >> "$f"; done` | 0 B | | | | | | | | 152.1 KB | 152.1 KB | 152.1 KB |
| `awk '{ print NR ": " $0 }' include/linux/kernfs.h > bench_out/numbered.txt` | 0 B | | | | | | | | 54.1 KB | 54.1 KB | 54.1 KB |
| `awk 'END { print NR }' bench_out/numbered.txt` | 4 B | | | | | | | | 5584 B | 5584 B | 5584 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1191 B | 1191 B | 2900 B | 2900 B | | 2900 B | 2900 B | 2905 B | 2905 B | 2905 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1288 B | 1288 B | 3250 B | 3250 B | | 3250 B | 3250 B | 3255 B | 3255 B | 3255 B |
| `ls -R bench_out/a` | 41 B | 1089 B | 1089 B | 3092 B | 3092 B | | 3092 B | 3129 B | 3097 B | 3097 B | 3097 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 997 B | 997 B | 2504 B | 2504 B | | 2504 B | 2504 B | 2413 B | 2413 B | 2413 B |
| `rmdir bench_out/a/b` | 0 B | 945 B | 945 B | 2383 B | 2383 B | | 2383 B | 2383 B | 2292 B | 2292 B | 2292 B |
| `rm -r bench_out/dir_moved` | 0 B | 1451 B | 1355 B | 2707 B | 2707 B | | 2707 B | 2707 B | 3295 B | 3295 B | 3295 B |
| `rm -rf bench_out` | 0 B | 12.4 KB | 12.2 KB | 14.1 KB | 14.1 KB | | 14.1 KB | 14.1 KB | 13.2 KB | 13.2 KB | 13.2 KB |

## What each command used (Stage 5, on disk, settled)

Counted by the engine for the first run of the command; the two times are that run's, measured. Bytes unless said otherwise.

| Command | Text read | Pieces | Index read | Index lookups | Tree entries | Content written | Records written | Upkeep written | Syncs | Scratch | Memory | CPU, ms | Elapsed, ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 0 | 0 | 0 | 0 | 39 | 0 | 459 | 0 | 1 | 0 | 2972 | 0.513 | 1.759 |
| `ls -a` | 0 | 0 | 0 | 0 | 39 | 0 | 0 | 0 | 0 | 0 | 4678 | 0.018 | 0.018 |
| `ls -R` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 94848 | 6.668 | 6.668 |
| `ls -aR Documentation` | 0 | 0 | 0 | 0 | 10375 | 0 | 0 | 0 | 0 | 0 | 67940 | 1.226 | 1.226 |
| `ls -p Documentation` | 0 | 0 | 0 | 0 | 88 | 0 | 0 | 0 | 0 | 0 | 7639 | 0.028 | 0.028 |
| `find .` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 20660 | 12.35 | 12.36 |
| `find . -name '*.h'` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 24585 | 19.13 | 19.13 |
| `find . -name '*.c'` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 23561 | 12.99 | 13.01 |
| `find . -type d` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 21443 | 9.542 | 9.563 |
| `find . -type f` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 34755 | 11.78 | 11.83 |
| `find . -maxdepth 2 -type f` | 0 | 0 | 0 | 0 | 1786 | 0 | 0 | 0 | 0 | 0 | 34600 | 0.457 | 0.457 |
| `find Documentation -name '*.h' -type f` | 0 | 0 | 0 | 0 | 10375 | 0 | 0 | 0 | 0 | 0 | 4451 | 1.546 | 1.546 |
| `find . -name 'Makefile'` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 28755 | 9.928 | 9.929 |
| `cat include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 2144 | 0.041 | 0.041 |
| `cat -n include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 31648 | 0.115 | 0.115 |
| `head -n 20 include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3264 | 0.021 | 0.021 |
| `tail -n 20 include/linux/kernfs.h` | 59811 | 3 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3283 | 0.022 | 0.022 |
| `tail -n +100 include/linux/kernfs.h` | 39874 | 2 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 2815 | 0.047 | 0.047 |
| `wc include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 2251 | 0.072 | 0.072 |
| `wc -l include/linux/*.h` | 0 | 0 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 238111 | 1.132 | 1.131 |
| `cat include/linux/*.h \| wc -l` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 238760 | 2.656 | 2.656 |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 9796 | 0.031 | 0.030 |
| `grep -c define include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 15401 | 0.292 | 0.292 |
| `grep -n define include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 15401 | 0.062 | 0.062 |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 17021 | 0.098 | 0.097 |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 34003 | 0.369 | 0.368 |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 33077 | 0.145 | 0.145 |
| `grep -rn qdbzzabsentterm .` | 0 | 0 | 0 | 156 | 39 | 0 | 0 | 0 | 0 | 0 | 31432 | 0.469 | 0.469 |
| `grep -rl qdbzzabsentterm .` | 0 | 0 | 0 | 156 | 39 | 0 | 0 | 0 | 0 | 0 | 31432 | 0.217 | 0.217 |
| `grep -rn ausgabenspezifisches .` | 1211009 | 331 | 79142 | 216 | 732 | 0 | 0 | 0 | 0 | 0 | 41831 | 0.812 | 0.812 |
| `grep -rl ausgabenspezifisches .` | 1211009 | 331 | 79142 | 216 | 732 | 0 | 0 | 0 | 0 | 0 | 41831 | 0.497 | 0.497 |
| `grep -rn mispredicted .` | 15506564 | 992 | 245433 | 120 | 926 | 0 | 0 | 0 | 0 | 0 | 55278 | 3.905 | 3.906 |
| `grep -rl mispredicted .` | 15333647 | 989 | 245433 | 120 | 926 | 0 | 0 | 0 | 0 | 0 | 53939 | 3.020 | 3.020 |
| `grep -rn separately .` | 81868155 | 2828 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 281655 | 16.13 | 16.13 |
| `grep -rl separately .` | 79005707 | 2774 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 281129 | 11.64 | 11.65 |
| `grep -rn conditions .` | 329723559 | 15008 | 434369 | 96 | 88856 | 0 | 0 | 0 | 0 | 0 | 193199 | 67.25 | 67.25 |
| `grep -rl conditions .` | 309280273 | 14636 | 434369 | 96 | 88856 | 0 | 0 | 0 | 0 | 0 | 191503 | 38.29 | 38.97 |
| `grep -rn define .` | 1396914445 | 93472 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 30667 | 1031.2 | 1031.9 |
| `grep -rl define .` | 874806358 | 84061 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 15329 | 133.1 | 133.1 |
| `grep -rc separately .` | 81868155 | 2828 | 241390 | 96 | 88856 | 0 | 0 | 0 | 0 | 0 | 299599 | 52.44 | 52.44 |
| `grep -rin MISPREDICTED .` | 15506564 | 992 | 245433 | 120 | 926 | 0 | 0 | 0 | 0 | 0 | 53928 | 6.631 | 6.756 |
| `grep -ri separately .` | 81868155 | 2828 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 281547 | 24.84 | 25.12 |
| `grep -rn separately Documentation` | 12358847 | 1125 | 241390 | 96 | 10376 | 0 | 0 | 0 | 0 | 0 | 45942 | 3.419 | 3.419 |
| `grep -rl conditions Documentation` | 20455323 | 2095 | 434369 | 96 | 10376 | 0 | 0 | 0 | 0 | 0 | 191510 | 6.142 | 6.142 |
| `grep -rF 'separately' .` | 81868155 | 2828 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 281553 | 16.13 | 16.13 |
| `grep -rE 'atomic_[a-z]+\(' .` | 291364084 | 8724 | 107796 | 60 | 732 | 0 | 0 | 0 | 0 | 0 | 1039852 | 84.82 | 84.85 |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 16729439 | 1142 | 324575 | 336 | 1035 | 0 | 0 | 0 | 0 | 0 | 64088 | 7.104 | 7.105 |
| `grep -rE 'conditions\|separately' .` | 359274398 | 16259 | 675759 | 192 | 88856 | 0 | 0 | 0 | 0 | 0 | 197812 | 109.6 | 109.7 |
| `grep -rE '[0-9]{4,}' .` | 1396914445 | 93472 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 30668 | 2444.8 | 2461.6 |
| `grep -rn '^#include' .` | 1396914445 | 93472 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 31728 | 355.2 | 356.0 |
| `grep -rE '^(import\|from) ' .` | 1396914445 | 93472 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 30062 | 262.5 | 264.2 |
| `grep -rin 'return [a-z_]*;$' Documentation` | 17727828 | 1700 | 249328 | 60 | 10376 | 0 | 0 | 0 | 0 | 0 | 731830 | 6.224 | 6.223 |
| `grep -rv e drivers/usb` | 15871818 | 913 | 0 | 0 | 894 | 0 | 0 | 0 | 0 | 0 | 23935 | 26.43 | 26.46 |
| `grep -r '\(separately\).*\1' .` | 81868155 | 2828 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 284618 | 17.34 | 17.61 |
| `grep -n mispredicted include/linux/*.h` | 3902957 | 550 | 245433 | 120 | 5800 | 0 | 0 | 0 | 0 | 0 | 238797 | 1.995 | 1.993 |
| `grep -l separately include/linux/*.h` | 4752387 | 566 | 241390 | 96 | 5800 | 0 | 0 | 0 | 0 | 0 | 238787 | 1.905 | 1.905 |
| `grep -c qdbzzabsentterm include/linux/*.h` | 0 | 0 | 0 | 156 | 5800 | 0 | 0 | 0 | 0 | 0 | 238812 | 0.922 | 0.921 |
| `grep -rl conditions . \| wc -l` | 309280273 | 14636 | 434369 | 96 | 88856 | 0 | 0 | 0 | 0 | 0 | 193932 | 37.54 | 37.61 |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1396914445 | 93472 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 31572 | 322.8 | 323.0 |
| `find . -name '*.h' \| wc -l` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 27014 | 11.51 | 11.51 |
| `find . -type f \| grep -c h` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 41030 | 15.88 | 16.09 |
| `grep -rn define . \| head -n 5` | 255439 | 49 | 0 | 0 | 213 | 0 | 0 | 0 | 0 | 0 | 18090 | 0.135 | 0.135 |
| `grep -rn conditions . \| head -n 1` | 473575 | 91 | 0 | 0 | 213 | 0 | 0 | 0 | 0 | 0 | 25414 | 0.207 | 0.207 |
| `find . -type f \| head -n 3` | 0 | 0 | 0 | 0 | 732 | 0 | 0 | 0 | 0 | 0 | 36519 | 0.152 | 0.152 |
| `cat include/linux/*.h \| head -n 2` | 48286 | 3 | 0 | 0 | 1522 | 0 | 0 | 0 | 0 | 0 | 239116 | 0.451 | 0.450 |
| `grep -rno separately .` | 81868155 | 2828 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 295104 | 15.16 | 15.25 |
| `grep -rw separately .` | 81868155 | 2828 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 294917 | 14.90 | 14.91 |
| `grep -rnw mispredicted .` | 15506564 | 992 | 245433 | 120 | 926 | 0 | 0 | 0 | 0 | 0 | 65768 | 4.037 | 4.071 |
| `grep -rn -C 2 mispredicted .` | 22904783 | 1163 | 245433 | 120 | 926 | 0 | 0 | 0 | 0 | 0 | 56282 | 4.640 | 4.774 |
| `grep -rn -A 1 -B 1 separately .` | 128715097 | 3805 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 16563813 | 44.84 | 45.03 |
| `grep -rho 'atomic_[a-z_]*' . \| sort \| uniq -c \| sort -rn \| head` | 291364084 | 8724 | 107796 | 60 | 732 | 0 | 0 | 0 | 0 | 0 | 3091210 | 107.4 | 107.4 |
| `grep -rhow '[a-z]*mispredicted[a-z]*' . \| sort \| uniq -c \| sort -rn \| head -n 5` | 15506564 | 992 | 245433 | 120 | 926 | 0 | 0 | 0 | 0 | 0 | 121581 | 10.75 | 10.75 |
| `rg -j 1 qdbzzabsentterm` | 2089 | 1 | 0 | 156 | 43 | 0 | 0 | 0 | 0 | 0 | 36565 | 0.391 | 0.391 |
| `rg -j 1 ausgabenspezifisches` | 31571 | 3 | 79142 | 216 | 20 | 0 | 0 | 0 | 0 | 0 | 56343 | 0.493 | 0.493 |
| `rg -j 1 mispredicted` | 11917571 | 302 | 245433 | 120 | 996 | 0 | 0 | 0 | 0 | 0 | 166050 | 6.323 | 17.40 |
| `rg -j 1 separately` | 78795217 | 2219 | 241390 | 96 | 4188 | 0 | 0 | 0 | 0 | 0 | 462429 | 33.43 | 34.35 |
| `rg -j 1 conditions` | 326059201 | 14408 | 434369 | 96 | 110656 | 0 | 0 | 0 | 0 | 0 | 1475772 | 189.8 | 189.8 |
| `rg -j 1 define` | 1396848040 | 93447 | 0 | 0 | 110656 | 0 | 0 | 0 | 0 | 0 | 1414304 | 1028.7 | 1036.2 |
| `rg -j 1 -l separately` | 75932769 | 2165 | 241390 | 96 | 4188 | 0 | 0 | 0 | 0 | 0 | 462983 | 34.89 | 34.92 |
| `rg -j 1 -l define` | 874739953 | 84036 | 0 | 0 | 110656 | 0 | 0 | 0 | 0 | 0 | 1414602 | 200.9 | 202.8 |
| `rg -j 1 -c conditions` | 326059201 | 14408 | 434369 | 96 | 110656 | 0 | 0 | 0 | 0 | 0 | 1476326 | 178.6 | 179.2 |
| `rg -j 1 -i MISPREDICTED` | 12794975 | 316 | 245433 | 168 | 1020 | 0 | 0 | 0 | 0 | 0 | 186316 | 21.96 | 21.97 |
| `rg -j 1 -w separately` | 78795217 | 2219 | 241390 | 96 | 4188 | 0 | 0 | 0 | 0 | 0 | 482815 | 30.84 | 30.87 |
| `rg -j 1 -F 'separately'` | 78795217 | 2219 | 241390 | 96 | 4188 | 0 | 0 | 0 | 0 | 0 | 462979 | 17.35 | 17.35 |
| `rg -j 1 -n -C 2 mispredicted` | 19375004 | 475 | 245433 | 120 | 996 | 0 | 0 | 0 | 0 | 0 | 167320 | 5.955 | 5.976 |
| `rg -j 1 'atomic_[a-z]+\('` | 289774105 | 8358 | 107796 | 60 | 6628 | 0 | 0 | 0 | 0 | 0 | 1043941 | 103.7 | 103.7 |
| `rg -j 1 'mispredicted\|ausgabenspezifisches'` | 11947053 | 304 | 324575 | 336 | 1004 | 0 | 0 | 0 | 0 | 0 | 193540 | 6.119 | 6.119 |
| `rg -j 1 '\bmispredicted\w*'` | 11917571 | 302 | 245433 | 120 | 996 | 0 | 0 | 0 | 0 | 0 | 2592839 | 8.677 | 8.677 |
| `rg -j 1 -t c separately` | 64466419 | 1649 | 241390 | 96 | 4188 | 0 | 0 | 0 | 0 | 0 | 461041 | 23.89 | 23.91 |
| `rg -j 1 -g '*.h' separately` | 9441203 | 354 | 241390 | 96 | 4188 | 0 | 0 | 0 | 0 | 0 | 460951 | 7.283 | 7.282 |
| `rg -j 1 separately Documentation` | 9347015 | 422 | 241390 | 96 | 13084 | 0 | 0 | 0 | 0 | 0 | 202454 | 13.37 | 13.38 |
| `rg -j 1 -uu separately` | 78783087 | 2145 | 241390 | 96 | 0 | 0 | 0 | 0 | 0 | 0 | 287796 | 15.62 | 15.92 |
| `rg -j 1 --sort path mispredicted` | 11917571 | 302 | 245433 | 120 | 996 | 0 | 0 | 0 | 0 | 0 | 166972 | 3.916 | 3.944 |
| `rg -j 1 --files \| wc -l` | 32122 | 334 | 0 | 0 | 110656 | 0 | 0 | 0 | 0 | 0 | 1416482 | 73.71 | 73.77 |
| `rg -j 1 -l conditions \| wc -l` | 305627055 | 14038 | 434369 | 96 | 110656 | 0 | 0 | 0 | 0 | 0 | 1478755 | 118.3 | 118.4 |
| `ls -ltR` | 0 | 0 | 0 | 0 | 266529 | 0 | 0 | 0 | 0 | 0 | 147402 | 50.86 | 50.93 |
| `ls -lt include/linux` | 0 | 0 | 0 | 0 | 5098 | 0 | 0 | 0 | 0 | 0 | 116120 | 1.146 | 1.146 |
| `ls -ltr Documentation \| tail -n 3` | 0 | 0 | 0 | 0 | 3243 | 0 | 0 | 0 | 0 | 0 | 16854 | 0.169 | 0.169 |
| `find . -newer include/linux/kernfs.h` | 0 | 0 | 0 | 0 | 88859 | 0 | 0 | 0 | 0 | 0 | 19948 | 10.73 | 10.84 |
| `find . -newer include/linux/kernfs.h -name '*.h'` | 0 | 0 | 0 | 0 | 88859 | 0 | 0 | 0 | 0 | 0 | 34305 | 12.38 | 12.45 |
| `find . -type f -mtime +30` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 5020 | 9.753 | 9.754 |
| `du -s .` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 309657 | 5.078 | 5.087 |
| `du -sh Documentation` | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 2519 | 0.022 | 0.022 |
| `du -sb .` | 0 | 0 | 0 | 0 | 88856 | 0 | 0 | 0 | 0 | 0 | 412042 | 3.653 | 3.652 |
| `du -sb Documentation` | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 2519 | 0.029 | 0.029 |
| `du -b -d 1 Documentation` | 0 | 0 | 0 | 0 | 88 | 0 | 0 | 0 | 0 | 0 | 8677 | 0.071 | 0.070 |
| `sort include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 114401 | 0.212 | 0.211 |
| `sort -r include/linux/kernfs.h \| head -n 3` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 117524 | 0.140 | 0.140 |
| `sort -u include/linux/kernfs.h \| wc -l` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 117192 | 0.143 | 0.143 |
| `sort -k 2 include/linux/kernfs.h \| tail -n 3` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 137847 | 0.173 | 0.173 |
| `cat include/linux/*.h \| sort \| uniq -c \| sort -rn \| head -n 5` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 56896333 | 156.1 | 156.5 |
| `uniq -c include/linux/kernfs.h \| wc -l` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 31252 | 0.106 | 0.106 |
| `cut -c 1-20 include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 22709 | 0.099 | 0.099 |
| `cut -d ' ' -f 1 include/linux/kernfs.h \| sort \| uniq \| wc -l` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 109911 | 0.245 | 0.246 |
| `tr a-z A-Z < include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 24768 | 0.059 | 0.060 |
| `tr -d '\n' < include/linux/kernfs.h \| wc -c` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 27191 | 0.050 | 0.049 |
| `tr -s ' ' < include/linux/kernfs.h \| wc -c` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 27189 | 0.058 | 0.057 |
| `nl include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 27857 | 0.203 | 0.203 |
| `nl -ba include/linux/kernfs.h \| tail -n 3` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 55373 | 0.158 | 0.158 |
| `seq 1 100000 \| wc -l` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 136176 | 2.943 | 2.943 |
| `seq -w 1 1000 \| tail -n 2` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 18979 | 0.057 | 0.057 |
| `basename include/linux/kernfs.h` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2093 | 0.010 | 0.010 |
| `basename -s .h include/linux/kernfs.h` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2939 | 0.011 | 0.011 |
| `dirname include/linux/kernfs.h` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2102 | 0.010 | 0.010 |
| `printf '%s\n' a b c` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3262 | 0.015 | 0.014 |
| `printf '%05d %-8s\|%x\n' 42 name 255` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3359 | 0.016 | 0.016 |
| `test -f include/linux/kernfs.h && echo yes` | 0 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3572 | 0.023 | 0.023 |
| `[ -d Documentation ] && echo folder` | 0 | 0 | 0 | 0 | 40 | 0 | 0 | 0 | 0 | 0 | 3909 | 0.021 | 0.020 |
| `date -u -d '2024-02-29 12:00:00' '+%Y-%m-%d %H:%M:%S %s %a'` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3449 | 0.024 | 0.023 |
| `echo a b c \| xargs -n 1 echo` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 6107 | 0.026 | 0.026 |
| `find drivers/usb -name '*.h' \| xargs cat \| wc -l` | 1878603 | 229 | 0 | 0 | 2862 | 0 | 0 | 0 | 0 | 0 | 73452 | 1.209 | 1.209 |
| `find . -name '*.h' \| xargs grep -l separately \| wc -l` | 36980222 | 4595 | 1931120 | 768 | 209146 | 0 | 0 | 0 | 0 | 0 | 1903217 | 50.49 | 51.30 |
| `echo {a,b,c}{1..3}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5657 | 0.022 | 0.022 |
| `echo {1..20..3} {a..e}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5043 | 0.021 | 0.020 |
| `echo $(wc -l < include/linux/kernfs.h) `basename include/linux/kernfs.h`` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 7687 | 0.041 | 0.041 |
| `echo "first line: $(head -n 1 include/linux/kernfs.h)"` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 6353 | 0.029 | 0.029 |
| `wc -l < include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3093 | 0.020 | 0.019 |
| `cat include/linux/kernfs.h > /dev/null 2>&1` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 3210 | 0.016 | 0.016 |
| `cat qdb_no_such_file 2> /dev/null ; echo after` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 4159 | 0.020 | 0.020 |
| `cat qdb_no_such_file 2>&1 \| wc -l` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 5185 | 0.023 | 0.023 |
| `grep -q define include/linux/kernfs.h && echo found \|\| echo none` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 17594 | 0.121 | 0.121 |
| `false \|\| echo fallback` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2710 | 0.012 | 0.012 |
| `true && grep -c define include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 16053 | 0.065 | 0.065 |
| `cat <<EOF⏎one $(echo two)⏎three⏎EOF` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5116 | 0.022 | 0.022 |
| `wc -l <<'EOF'⏎a⏎b⏎EOF` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2819 | 0.013 | 0.013 |
| `echo one \⏎two` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2336 | 0.010 | 0.009 |
| `grep -c define include/linux/kernfs.h⏎wc -l include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 9 | 0 | 0 | 0 | 0 | 0 | 16819 | 0.076 | 0.076 |
| `for i in $(seq 100000); do :; done` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 7815304 | 39.30 | 40.40 |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 0 | 0 | 0 | 0 | 390039 | 0 | 0 | 0 | 0 | 0 | 6992 | 40.66 | 40.72 |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 0 | 0 | 0 | 0 | 429 | 0 | 0 | 0 | 0 | 0 | 6989 | 0.068 | 0.068 |
| `for f in include/linux/*.h; do echo "$f"; done \| wc -l` | 0 | 0 | 0 | 0 | 1504 | 0 | 0 | 0 | 0 | 0 | 153357 | 1.533 | 1.532 |
| `for f in include/linux/*.h; do grep -c define "$f"; done \| sort -n \| tail -n 3` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 159902 | 18.99 | 19.02 |
| `n=0; for f in include/linux/*.h; do if grep -q conditions "$f"; then n=$((n+1)); fi; done; echo $n` | 10736325 | 1449 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 154662 | 26.69 | 26.74 |
| `for f in include/linux/*.h; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 0 | 0 | 0 | 0 | 61648 | 0 | 0 | 0 | 0 | 0 | 153634 | 2.983 | 2.983 |
| `H=$PWD; for d in Documentation drivers/usb include/linux; do cd "$d"; ls \| wc -l; cd "$H"; done` | 0 | 0 | 0 | 0 | 1623 | 0 | 0 | 0 | 0 | 0 | 95478 | 0.195 | 0.195 |
| `X=define; N=$(grep -c "$X" include/linux/kernfs.h); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 19937 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 18869 | 0.133 | 0.133 |
| `find drivers/usb -name '*.h' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 1878603 | 229 | 0 | 0 | 1877 | 0 | 0 | 0 | 0 | 0 | 102506 | 1.397 | 1.398 |
| `for f in include/linux/*.h; do head -n 1 "$f"; done > /dev/null` | 10104438 | 1432 | 0 | 0 | 5800 | 0 | 0 | 0 | 0 | 0 | 152606 | 2.892 | 2.893 |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 0 | 0 | 0 | 0 | 195 | 0 | 0 | 0 | 0 | 0 | 9461 | 0.044 | 0.044 |
| `awk '{ print $2 }' include/linux/kernfs.h` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 16734 | 0.107 | 0.105 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 241000 | 30.18 | 30.18 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '/define/ { n++ } END { print n }'` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 240124 | 21.61 | 21.61 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 240122 | 41.84 | 41.84 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 240477 | 23.55 | 24.22 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 241030 | 19.62 | 19.62 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 240148 | 19.71 | 19.71 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 3420460 | 82.94 | 82.95 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "define") n++ } END { print n }'` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 240198 | 141.6 | 141.8 |
| `awk '{ gsub(/[0-9]+/, "N"); print }' include/linux/kernfs.h \| wc -c` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 39012 | 0.456 | 0.456 |
| `awk -v OFS=- '{ $1 = NR; print }' include/linux/kernfs.h \| tail -n 1` | 19937 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 49374 | 0.333 | 0.333 |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5634 | 5.322 | 5.322 |
| `grep -rn separately . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 81868155 | 2828 | 241390 | 96 | 821 | 0 | 0 | 0 | 0 | 0 | 296374 | 24.25 | 24.25 |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 10767883 | 1451 | 0 | 0 | 10096 | 0 | 0 | 0 | 0 | 0 | 12204886 | 88.16 | 88.17 |
| `mkdir -p bench_out/a/b` | 0 | 0 | 0 | 0 | 3 | 0 | 111 | 0 | 1 | 0 | 3456 | 0.115 | 0.602 |
| `touch bench_out/empty.txt` | 0 | 0 | 0 | 0 | 3 | 0 | 71 | 0 | 1 | 0 | 2215 | 0.060 | 0.344 |
| `echo 'hello world' > bench_out/hello.txt` | 0 | 0 | 0 | 0 | 3 | 52 | 95 | 0 | 1 | 0 | 3446 | 0.110 | 0.374 |
| `echo 'another line' >> bench_out/hello.txt` | 24 | 2 | 0 | 0 | 2 | 65 | 65 | 0 | 1 | 0 | 3258 | 0.173 | 0.452 |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 0 | 0 | 0 | 0 | 8 | 0 | 99 | 0 | 1 | 0 | 3015 | 0.068 | 0.253 |
| `cp -r Documentation bench_out/dir_copy` | 0 | 0 | 0 | 0 | 164925 | 0 | 649761 | 0 | 1 | 0 | 6328188 | 16.42 | 17.56 |
| `cp -r include/linux bench_out/work` | 0 | 0 | 0 | 0 | 29419 | 0 | 157241 | 0 | 1 | 0 | 1403412 | 4.172 | 4.766 |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 19937 | 1 | 0 | 0 | 2 | 20017 | 83 | 0 | 1 | 0 | 297979 | 0.723 | 1.154 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 19977 | 1 | 0 | 0 | 2 | 19925 | 83 | 0 | 1 | 0 | 53392 | 0.571 | 0.861 |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 10767883 | 1451 | 0 | 0 | 5800 | 10971528 | 56519 | 0 | 1 | 0 | 5432029 | 173.1 | 201.5 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 0 | 0 | 0 | 156 | 5800 | 0 | 18657 | 0 | 1 | 0 | 252751 | 1.383 | 13.67 |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 3946647 | 550 | 248866 | 120 | 5800 | 0 | 18657 | 0 | 1 | 0 | 251191 | 2.867 | 14.04 |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 | 0 | 0 | 0 | 8 | 0 | 99 | 0 | 1 | 0 | 2867 | 0.084 | 0.453 |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 | 0 | 0 | 0 | 7 | 0 | 41 | 0 | 1 | 0 | 2595 | 0.057 | 0.304 |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 39874 | 2 | 0 | 0 | 8 | 0 | 41 | 0 | 1 | 0 | 23135 | 0.085 | 0.221 |
| `grep -rc QDBBENCHMANY bench_out` | 11764871 | 1689 | 58007 | 120 | 13154 | 0 | 0 | 0 | 0 | 0 | 208030 | 5.203 | 5.204 |
| `grep -rl QDBBENCHMANY .` | 11046107 | 1608 | 58007 | 120 | 733 | 0 | 0 | 0 | 0 | 0 | 180775 | 1.247 | 1.247 |
| `grep -rl QDBBENCHONE .` | 945403 | 189 | 44938 | 108 | 733 | 0 | 0 | 0 | 0 | 0 | 24764 | 0.475 | 0.475 |
| `cat include/linux/kernfs.h \| tee bench_out/tee.txt \| wc -l` | 19937 | 1 | 0 | 0 | 9 | 19977 | 98 | 0 | 1 | 0 | 46678 | 0.515 | 1.020 |
| `sort include/linux/kernfs.h > bench_out/sorted.txt` | 19937 | 1 | 0 | 0 | 9 | 19977 | 101 | 0 | 1 | 0 | 135386 | 0.545 | 0.922 |
| `sort -u include/linux/kernfs.h >> bench_out/sorted.txt` | 59811 | 3 | 0 | 0 | 8 | 38599 | 70 | 0 | 1 | 0 | 134213 | 0.587 | 0.968 |
| `cat qdb_no_such_file &> bench_out/both.txt` | 0 | 0 | 0 | 0 | 5 | 89 | 94 | 0 | 1 | 0 | 3568 | 0.092 | 0.287 |
| `cat qdb_no_such_file 2>> bench_out/both.txt` | 98 | 2 | 0 | 0 | 4 | 138 | 65 | 0 | 1 | 0 | 3499 | 0.110 | 0.336 |
| `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` | 1878603 | 229 | 0 | 0 | 2865 | 1879203 | 687 | 0 | 1 | 0 | 2431449 | 14.98 | 17.52 |
| `cp bench_out/joined.txt bench_out/joined2.txt` | 0 | 0 | 0 | 0 | 7 | 0 | 688 | 0 | 1 | 0 | 8127 | 0.096 | 0.304 |
| `sed -i '1d' bench_out/joined2.txt` | 1878603 | 29 | 0 | 0 | 2 | 65524 | 83 | 0 | 1 | 0 | 150919 | 3.231 | 4.981 |
| `wc -c bench_out/joined.txt bench_out/joined2.txt bench_out/tee.txt bench_out/sorted.txt` | 0 | 0 | 0 | 0 | 16 | 0 | 0 | 0 | 0 | 0 | 4216 | 0.026 | 0.026 |
| `cat bench_out/both.txt` | 98 | 1 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 2146 | 0.011 | 0.011 |
| `cp include/linux/kernfs.h bench_out/v2.txt` | 0 | 0 | 0 | 0 | 8 | 0 | 97 | 0 | 1 | 0 | 2839 | 0.087 | 0.345 |
| `sed -i -e '10s/$/ changed/' -e '40,45d' -e '100i an added line' bench_out/v2.txt` | 19937 | 1 | 0 | 0 | 2 | 19726 | 83 | 0 | 1 | 0 | 50705 | 0.320 | 0.725 |
| `diff -u --label old --label new include/linux/kernfs.h bench_out/v2.txt` | 257926 | 13 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 18708 | 0.089 | 0.089 |
| `diff include/linux/kernfs.h bench_out/v2.txt \| wc -l` | 198366 | 10 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 18617 | 0.074 | 0.074 |
| `diff -q include/linux/kernfs.h bench_out/same.txt` | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 3047 | 0.013 | 0.013 |
| `diff -u include/linux/kernfs.h bench_out/copy.txt \| wc -l` | 239036 | 12 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 107042 | 0.276 | 0.276 |
| `diff -rq Documentation bench_out/dir_copy` | 0 | 0 | 0 | 0 | 20751 | 0 | 0 | 0 | 0 | 0 | 59045 | 5.020 | 5.021 |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 53 | 1 | 0 | 0 | 10379 | 102 | 78 | 0 | 1 | 0 | 1515904 | 4.286 | 4.643 |
| `diff -rq Documentation bench_out/dir_copy` | 115 | 2 | 0 | 0 | 20751 | 0 | 0 | 0 | 0 | 0 | 59117 | 3.435 | 3.436 |
| `diff -r include/linux bench_out/work \| wc -l` | 90403998 | 12566 | 0 | 0 | 5544 | 0 | 0 | 0 | 0 | 0 | 979021 | 72.88 | 73.17 |
| `diff -ru include/linux bench_out/work \| grep -c '^@@'` | 119026792 | 16730 | 0 | 0 | 5544 | 0 | 0 | 0 | 0 | 0 | 985147 | 82.72 | 83.20 |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 1580743 | 19993 | 0 | 0 | 20001 | 1239829 | 254400 | 0 | 1 | 0 | 1540770 | 31.41 | 34.21 |
| `wc -l bench_out/loop.txt` | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 2569 | 0.020 | 0.019 |
| `for f in bench_out/work/*.h; do echo done >> "$f"; done` | 9606666 | 1432 | 0 | 0 | 5800 | 64440 | 54444 | 0 | 1 | 0 | 151852 | 4.279 | 4.973 |
| `awk '{ print NR ": " $0 }' include/linux/kernfs.h > bench_out/numbered.txt` | 19937 | 1 | 0 | 0 | 6 | 23034 | 104 | 0 | 1 | 0 | 53866 | 0.534 | 1.038 |
| `awk 'END { print NR }' bench_out/numbered.txt` | 22994 | 1 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 5324 | 0.056 | 0.056 |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 | 0 | 0 | 0 | 8 | 0 | 58 | 0 | 1 | 0 | 2645 | 0.094 | 0.355 |
| `mv bench_out/hello.txt bench_out/a/b` | 0 | 0 | 0 | 0 | 13 | 0 | 71 | 0 | 1 | 0 | 2995 | 0.090 | 0.316 |
| `ls -R bench_out/a` | 0 | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 2837 | 0.017 | 0.017 |
| `rm bench_out/a/b/hello.txt` | 0 | 0 | 0 | 0 | 4 | 0 | 45 | 0 | 1 | 0 | 2153 | 0.068 | 0.252 |
| `rmdir bench_out/a/b` | 0 | 0 | 0 | 0 | 3 | 0 | 45 | 0 | 1 | 0 | 2032 | 0.048 | 0.189 |
| `rm -r bench_out/dir_moved` | 0 | 0 | 0 | 0 | 2 | 0 | 41541 | 0 | 1 | 0 | 3035 | 4.910 | 5.273 |
| `rm -rf bench_out` | 0 | 0 | 0 | 0 | 1 | 0 | 11175 | 0 | 1 | 0 | 12985 | 17.34 | 17.62 |

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
