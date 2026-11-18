# Benchmark: linux

- Data: Linux 6.8 source tree, from the Ubuntu source package linux_6.8.0.orig.tar.gz (sha256 26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8). Skipped on import: 64 symbolic links, 6 files that are not valid text, 0 other
- Size: 83405 files in 5451 folders, 1396.9 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (On disk, 512 MB limit, after the writes), and include starting bash and the tool (1.546 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 1374.6 | 8146.1 | 1568.1 MB | 1578.0 MB | 0.280 | yes |
| Stage 3, in memory | on | 2019.3 | 9426.5 | 1666.7 MB | 1620.8 MB | 0.529 | yes |
| On disk, settled | on | 13807.8 | 13807.8 | 21.7 MB | 967.2 MB | 0.310 | yes |
| On disk, after the writes | on | 13807.8 | 13807.8 | 21.7 MB | 967.2 MB | 0.310 | yes |
| On disk, cold | on | 13400.4 | 13400.4 | 21.7 MB | 965.8 MB | 0.297 | yes |
| On disk, 512 MB limit | on | 13818.0 | 13818.0 | 21.7 MB | 962.9 MB | 0.309 | yes |
| On disk, 512 MB limit, after the writes | on | 13818.0 | 13818.0 | 21.7 MB | 962.9 MB | 0.309 | yes |

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

## The index (Stage 3, in memory)

- Built in 7407.3 ms while loading.
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
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +9762 -0; postings +9359558; list bytes rewritten 0; rebuilds 0 |
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
| `rm -rf bench_out` | documents +0 -1432; postings +0; list bytes rewritten 2503261; rebuilds 0 |

## The index (On disk, settled)

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

## The index (On disk, after the writes)

- Built in 0.000 ms while loading.
- Size: 141.5 MB in memory, 10.1% of the text (10.5 MB table, 129.9 MB lists, 1064.6 KB document table). The posting data itself is 129.9 MB.
- 183791 distinct trigrams, 0 postings, 93472 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

## The index (On disk, cold)

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
| `echo 'hello world' > bench_out/hello.txt` | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp include/linux/kernfs.h bench_out/copy.txt` | documents +1 -0; postings +2610; list bytes rewritten 0; rebuilds 0 |
| `cp -r Documentation bench_out/dir_copy` | documents +9762 -0; postings +9359558; list bytes rewritten 0; rebuilds 0 |
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

## The index (On disk, 512 MB limit)

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

## The index (On disk, 512 MB limit, after the writes)

- Built in 0.000 ms while loading.
- Size: 141.5 MB in memory, 10.1% of the text (10.5 MB table, 129.9 MB lists, 1064.6 KB document table). The posting data itself is 129.9 MB.
- 183791 distinct trigrams, 0 postings, 93472 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, in memory | On disk, settled | On disk, after the writes | On disk, cold | On disk, 512 MB limit | On disk, 512 MB limit, after the writes | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.986 |  | 0.001 | 0.001 | 0.001 | 0.003 | 0.002 | 0.001 | 0.003 | yes |
| `ls -a` | 2.357 |  | 0.004 | 0.003 | 0.004 | 0.004 | 0.003 | 0.002 | 0.004 | yes |
| `ls -R` | 85.50 |  | 8.025 | 6.371 | 3.303 | 4.837 | 4.449 | 3.287 | 4.094 | yes |
| `ls -aR Documentation` | 14.21 |  | 0.747 | 0.646 | 0.861 | 0.624 | 0.597 | 0.596 | 0.556 | yes |
| `ls -p Documentation` | 2.529 |  | 0.004 | 0.002 | 0.004 | 0.005 | 0.003 | 0.003 | 0.005 | yes |
| `find .` | 76.28 |  | 20.82 | 14.62 | 10.16 | 6.155 | 7.557 | 11.35 | 7.765 | yes |
| `find . -name '*.h'` | 70.42 |  | 25.53 | 16.80 | 10.22 | 8.135 | 9.202 | 8.609 | 14.31 | yes |
| `find . -name '*.c'` | 80.44 |  | 28.25 | 17.73 | 10.50 | 10.16 | 23.36 | 9.513 | 13.30 | yes |
| `find . -type d` | 61.45 |  | 25.02 | 14.25 | 6.396 | 6.658 | 6.677 | 7.242 | 6.303 | yes |
| `find . -type f` | 67.30 |  | 23.39 | 12.87 | 11.97 | 8.067 | 10.81 | 9.662 | 10.15 | yes |
| `find . -maxdepth 2 -type f` | 4.552 |  | 0.114 | 0.103 | 0.109 | 0.102 | 0.111 | 0.112 | 0.113 | yes |
| `find Documentation -name '*.h' -type f` | 12.72 |  | 1.770 | 1.538 | 1.071 | 0.979 | 1.062 | 0.877 | 0.964 | yes |
| `find . -name 'Makefile'` | 71.53 |  | 27.58 | 14.11 | 7.526 | 12.33 | 11.50 | 7.116 | 10.82 | yes |
| `cat include/linux/kernfs.h` | 2.304 |  | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.002 | yes |
| `cat -n include/linux/kernfs.h` | 2.320 |  | 0.057 | 0.054 | 0.055 | 0.057 | 0.052 | 0.057 | 0.086 | yes |
| `head -n 20 include/linux/kernfs.h` | 2.418 |  | 0.002 | 0.002 | 0.002 | 0.001 | 0.002 | 0.002 | 0.003 | yes |
| `tail -n 20 include/linux/kernfs.h` | 2.166 |  | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `tail -n +100 include/linux/kernfs.h` | 2.243 |  | 0.003 | 0.003 | 0.003 | 0.003 | 0.004 | 0.003 | 0.003 | yes |
| `wc include/linux/kernfs.h` | 2.736 |  | 0.024 | 0.023 | 0.023 | 0.024 | 0.023 | 0.022 | 0.023 | yes |
| `wc -l include/linux/*.h` | 10.71 |  | 28.62 | 0.900 | 0.885 | 0.934 | 0.872 | 0.885 | 0.971 | yes |
| `cat include/linux/*.h \| wc -l` | 34.16 |  | 26.33 | 24.10 | 24.59 | 25.27 | 24.73 | 23.63 | 26.66 | yes |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 3.251 |  | 0.005 | 0.009 | 0.006 | 0.007 | 0.012 | 0.004 | 0.009 | yes |
| `grep -c define include/linux/kernfs.h` | 2.316 |  | 0.006 | 0.009 | 0.009 | 0.009 | 0.044 | 0.007 | 0.010 | yes |
| `grep -n define include/linux/kernfs.h` | 2.465 |  | 0.006 | 0.009 | 0.009 | 0.010 | 0.046 | 0.007 | 0.013 | yes |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 2.513 |  | 0.249 | 0.254 | 0.226 | 0.244 | 0.243 | 0.188 | 0.274 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 3.499 |  | 0.588 | 0.618 | 0.560 | 0.625 | 0.589 | 0.620 | 0.604 | yes |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 3.127 |  | 0.152 | 0.123 | 0.120 | 0.117 | 0.139 | 0.166 | 0.127 | yes |
| `grep -rn qdbzzabsentterm .` | 1224.9 | 637.4 | 0.012 | 0.019 | 0.020 | 0.020 | 0.100 | 0.021 | 0.022 | yes |
| `grep -rl qdbzzabsentterm .` | 869.0 | 585.9 | 0.011 | 0.021 | 0.021 | 0.021 | 0.109 | 0.026 | 0.032 | yes |
| `grep -rn ausgabenspezifisches .` | 1146.5 | 642.9 | 0.098 | 0.118 | 0.125 | 0.230 | 0.295 | 0.099 | 0.146 | yes |
| `grep -rl ausgabenspezifisches .` | 845.7 | 587.1 | 0.095 | 0.115 | 0.100 | 0.138 | 0.259 | 0.095 | 0.142 | yes |
| `grep -rn mispredicted .` | 1422.3 | 638.5 | 17.54 | 1.661 | 2.475 | 2.011 | 52.53 | 1.789 | 2.269 | yes |
| `grep -rl mispredicted .` | 945.4 | 585.8 | 17.60 | 1.546 | 1.544 | 1.696 | 57.06 | 1.418 | 1.734 | yes |
| `grep -rn separately .` | 1001.1 | 591.5 | 24.09 | 14.62 | 14.82 | 15.08 | 257.6 | 15.70 | 15.12 | yes |
| `grep -rl separately .` | 634.2 | 607.7 | 18.81 | 10.42 | 11.89 | 11.43 | 286.6 | 10.56 | 12.10 | yes |
| `grep -rn conditions .` | 1288.4 | 657.6 | 129.9 | 71.84 | 62.60 | 64.60 | 571.0 | 57.43 | 62.91 | yes |
| `grep -rl conditions .` | 756.3 | 590.3 | 65.31 | 43.63 | 40.80 | 40.43 | 498.5 | 42.13 | 38.71 | yes |
| `grep -rn define .` | 2552.1 | 2388.8 | 460.7 | 689.6 | 518.8 | 596.8 | 1235.2 | 561.3 | 461.1 | yes |
| `grep -rl define .` | 578.9 | 641.3 | 100.9 | 70.12 | 59.67 | 64.45 | 534.2 | 67.66 | 60.12 | yes |
| `grep -rc separately .` | 683.9 | 533.4 | 44.65 | 30.71 | 17.95 | 18.40 | 216.1 | 17.81 | 19.31 | yes |
| `grep -rin MISPREDICTED .` | 1533.4 | 710.9 | 21.12 | 2.259 | 2.271 | 1.749 | 3.533 | 2.135 | 1.826 | yes |
| `grep -ri separately .` | 787.2 | 674.2 | 28.93 | 17.99 | 16.61 | 20.03 | 195.3 | 16.62 | 15.74 | yes |
| `grep -rn separately Documentation` | 74.79 | 57.15 | 2.473 | 1.750 | 1.884 | 2.003 | 3.901 | 2.159 | 1.839 | yes |
| `grep -rl conditions Documentation` | 67.95 | 57.61 | 3.711 | 3.882 | 3.362 | 3.086 | 6.199 | 3.074 | 3.871 | yes |
| `grep -rF 'separately' .` | 622.4 | 585.5 | 24.42 | 13.02 | 14.81 | 17.24 | 218.6 | 13.13 | 14.11 | yes |
| `grep -rE 'atomic_[a-z]+\(' .` | 1324.0 | 640.1 | 74.42 | 55.26 | 51.84 | 54.47 | 587.0 | 56.22 | 56.92 | yes |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 1732.9 | 704.4 | 29.62 | 3.222 | 2.512 | 3.729 | 69.66 | 2.318 | 3.013 | yes |
| `grep -rE 'conditions\|separately' .` | 2326.3 | 757.5 | 180.1 | 83.68 | 75.05 | 74.51 | 544.7 | 78.97 | 81.12 | yes |
| `grep -rE '[0-9]{4,}' .` | 2220.1 | 2878.1 | 2025.8 | 2133.8 | 2153.3 | 1789.9 | 2550.2 | 2002.8 | 2083.6 | yes |
| `grep -rn '^#include' .` | 1585.3 | 906.4 | 244.7 | 239.4 | 217.7 | 273.4 | 692.1 | 246.9 | 225.8 | yes |
| `grep -rE '^(import\|from) ' .` | 1137.9 | 629.4 | 315.7 | 324.2 | 263.9 | 272.7 | 661.0 | 267.3 | 275.4 | yes |
| `grep -rin 'return [a-z_]*;$' Documentation` | 105.8 | 61.93 | 11.32 | 6.830 | 4.304 | 12.64 | 6.407 | 4.412 | 7.670 | yes |
| `grep -rv e drivers/usb` | 49.82 | 52.02 | 17.91 | 17.84 | 18.05 | 22.93 | 19.59 | 19.20 | 18.28 | yes |
| `grep -r '\(separately\).*\1' .` | 656.7 |  | 24.81 | 19.55 | 15.77 | 17.44 | 218.2 | 17.21 | 16.64 | yes |
| `grep -n mispredicted include/linux/*.h` | 17.74 |  | | 1.306 | 0.998 | 1.286 | 2.115 | 1.223 | 1.459 | yes |
| `grep -l separately include/linux/*.h` | 12.14 |  | | 1.568 | 1.514 | 1.550 | 1.792 | 1.522 | 1.544 | yes |
| `grep -c qdbzzabsentterm include/linux/*.h` | 14.29 |  | | 0.577 | 0.595 | 0.620 | 0.674 | 0.556 | 0.649 | yes |
| `grep -rl conditions . \| wc -l` | 820.6 |  | 58.03 | 45.58 | 42.95 | 48.87 | 526.0 | 39.31 | 38.51 | yes |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1345.6 |  | 342.3 | 304.4 | 315.0 | 320.8 | 674.5 | 339.7 | 337.3 | yes |
| `find . -name '*.h' \| wc -l` | 96.11 |  | 23.83 | 22.44 | 11.43 | 11.56 | 11.21 | 10.95 | 16.45 | yes |
| `find . -type f \| grep -c h` | 76.60 |  | 27.69 | 19.45 | 13.79 | 17.37 | 13.40 | 14.54 | 13.93 | yes |
| `grep -rn define . \| head -n 5` | 4.714 |  | 0.019 | 0.023 | 0.024 | 0.026 | 0.067 | 0.025 | 0.023 | order differs |
| `grep -rn conditions . \| head -n 1` | 12.83 |  | 0.027 | 0.042 | 0.040 | 0.046 | 0.151 | 0.043 | 0.056 | order differs |
| `find . -type f \| head -n 3` | 3.271 |  | 0.058 | 0.057 | 0.093 | 0.090 | 0.055 | 0.056 | 0.072 | order differs |
| `cat include/linux/*.h \| head -n 2` | 3.780 |  | 0.346 | 0.306 | 0.335 | 0.315 | 0.326 | 0.339 | 0.333 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, in memory | On disk, settled | On disk, after the writes | On disk, cold | On disk, 512 MB limit | On disk, 512 MB limit, after the writes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 212 B | 1457 B | 1457 B | 1457 B | 1457 B | 1457 B | 1457 B | 1457 B |
| `ls -a` | 330 B | 2681 B | 2681 B | 2681 B | 2681 B | 2681 B | 2681 B | 2681 B |
| `ls -R` | 1377.7 KB | 74.3 KB | 74.3 KB | 74.3 KB | 74.3 KB | 74.3 KB | 74.3 KB | 74.3 KB |
| `ls -aR Documentation` | 216.8 KB | 56.6 KB | 56.6 KB | 56.6 KB | 56.6 KB | 56.6 KB | 56.6 KB | 56.6 KB |
| `ls -p Documentation` | 796 B | 4893 B | 4893 B | 4893 B | 4893 B | 4893 B | 4893 B | 4893 B |
| `find .` | 3590.9 KB | 19.2 KB | 19.2 KB | 19.2 KB | 19.2 KB | 19.2 KB | 19.2 KB | 19.2 KB |
| `find . -name '*.h'` | 969.5 KB | 22.7 KB | 22.7 KB | 22.7 KB | 22.7 KB | 22.7 KB | 22.7 KB | 22.7 KB |
| `find . -name '*.c'` | 1233.5 KB | 21.7 KB | 21.7 KB | 21.7 KB | 21.7 KB | 21.7 KB | 21.7 KB | 21.7 KB |
| `find . -type d` | 166.7 KB | 19.5 KB | 19.5 KB | 19.5 KB | 19.5 KB | 19.5 KB | 19.5 KB | 19.5 KB |
| `find . -type f` | 3424.2 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB |
| `find . -maxdepth 2 -type f` | 23.5 KB | 32.0 KB | 32.0 KB | 32.0 KB | 32.0 KB | 32.0 KB | 32.0 KB | 32.0 KB |
| `find Documentation -name '*.h' -type f` | 76 B | 1937 B | 1937 B | 1937 B | 1937 B | 1937 B | 1937 B | 1937 B |
| `find . -name 'Makefile'` | 100.1 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB |
| `cat include/linux/kernfs.h` | 19.9 KB | 994 B | 994 B | 994 B | 994 B | 994 B | 994 B | 994 B |
| `cat -n include/linux/kernfs.h` | 24.4 KB | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB |
| `head -n 20 include/linux/kernfs.h` | 459 B | 1345 B | 1345 B | 1345 B | 1345 B | 1345 B | 1345 B | 1345 B |
| `tail -n 20 include/linux/kernfs.h` | 478 B | 1364 B | 1364 B | 1364 B | 1364 B | 1364 B | 1364 B | 1364 B |
| `tail -n +100 include/linux/kernfs.h` | 16.9 KB | 1306 B | 1306 B | 1306 B | 1306 B | 1306 B | 1306 B | 1306 B |
| `wc include/linux/kernfs.h` | 41 B | 1002 B | 1002 B | 1002 B | 1002 B | 1002 B | 1002 B | 1002 B |
| `wc -l include/linux/*.h` | 49.6 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB |
| `cat include/linux/*.h \| wc -l` | 7 B | 237.2 KB | 237.2 KB | 237.2 KB | 237.2 KB | 237.2 KB | 237.2 KB | 237.2 KB |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 447 B | 4720 B | 4944 B | 4944 B | 4944 B | 4944 B | 4944 B | 4944 B |
| `grep -c define include/linux/kernfs.h` | 2 B | 9470 B | 9470 B | 9470 B | 9470 B | 9470 B | 9470 B | 9470 B |
| `grep -n define include/linux/kernfs.h` | 374 B | 9470 B | 9470 B | 9470 B | 9470 B | 9470 B | 9470 B | 9470 B |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 19.9 KB | 27.0 KB | 27.0 KB | 27.0 KB | 27.0 KB | 27.0 KB | 27.0 KB | 27.0 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 19.9 KB | 26.6 KB | 26.6 KB | 26.6 KB | 26.6 KB | 26.6 KB | 26.6 KB | 26.6 KB |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 6 B | 25.8 KB | 26.0 KB | 26.0 KB | 26.0 KB | 26.0 KB | 26.0 KB | 26.0 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rn ausgabenspezifisches .` | 116 B | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rl ausgabenspezifisches .` | 32 B | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rn mispredicted .` | 111.0 KB | 90.8 KB | 51.9 KB | 51.9 KB | 51.9 KB | 57.0 KB | 51.9 KB | 51.9 KB |
| `grep -rl mispredicted .` | 4531 B | 90.8 KB | 51.9 KB | 51.9 KB | 51.9 KB | 52.0 KB | 51.9 KB | 51.9 KB |
| `grep -rn separately .` | 103.9 KB | 343.4 KB | 279.6 KB | 279.6 KB | 281.5 KB | 279.7 KB | 279.6 KB | 281.5 KB |
| `grep -rl separately .` | 30.7 KB | 342.9 KB | 279.0 KB | 279.0 KB | 280.9 KB | 279.2 KB | 279.0 KB | 280.9 KB |
| `grep -rn conditions .` | 1360.5 KB | 136.2 KB | 191.1 KB | 191.1 KB | 196.8 KB | 191.4 KB | 191.1 KB | 196.8 KB |
| `grep -rl conditions .` | 389.6 KB | 133.0 KB | 189.4 KB | 189.4 KB | 195.1 KB | 189.7 KB | 189.4 KB | 195.1 KB |
| `grep -rn define .` | 821.1 MB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB | 43.6 KB | 26.9 KB | 26.9 KB |
| `grep -rl define .` | 1867.6 KB | 9418 B | 9418 B | 9418 B | 9418 B | 18.0 KB | 9418 B | 9418 B |
| `grep -rc separately .` | 3591.0 KB | 361.3 KB | 297.5 KB | 297.5 KB | 299.4 KB | 297.6 KB | 297.5 KB | 299.4 KB |
| `grep -rin MISPREDICTED .` | 135.7 KB | 90.8 KB | 51.8 KB | 51.8 KB | 51.9 KB | 51.8 KB | 51.8 KB | 51.9 KB |
| `grep -ri separately .` | 100.6 KB | 343.3 KB | 279.5 KB | 279.5 KB | 281.4 KB | 279.6 KB | 279.5 KB | 281.4 KB |
| `grep -rn separately Documentation` | 19.9 KB | 35.4 KB | 43.8 KB | 43.8 KB | 45.7 KB | 43.8 KB | 43.8 KB | 45.7 KB |
| `grep -rl conditions Documentation` | 9541 B | 133.0 KB | 189.4 KB | 189.4 KB | 195.1 KB | 189.4 KB | 189.4 KB | 195.1 KB |
| `grep -rF 'separately' .` | 99.7 KB | 343.3 KB | 279.5 KB | 279.5 KB | 281.4 KB | 279.6 KB | 279.5 KB | 281.4 KB |
| `grep -rE 'atomic_[a-z]+\(' .` | 1294.9 KB | 945.1 KB | 1037.7 KB | 1037.7 KB | 1039.1 KB | 1037.8 KB | 1037.7 KB | 1039.1 KB |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 109.2 KB | 62.6 KB | 53.6 KB | 53.6 KB | 53.6 KB | 57.7 KB | 53.6 KB | 53.6 KB |
| `grep -rE 'conditions\|separately' .` | 1427.8 KB | 188.8 KB | 195.7 KB | 195.7 KB | 202.1 KB | 196.0 KB | 195.7 KB | 202.1 KB |
| `grep -rE '[0-9]{4,}' .` | 467.5 MB | 28.6 KB | 28.6 KB | 28.6 KB | 28.6 KB | 45.2 KB | 28.6 KB | 28.6 KB |
| `grep -rn '^#include' .` | 24.9 MB | 29.6 KB | 29.6 KB | 29.6 KB | 29.6 KB | 46.2 KB | 29.6 KB | 29.6 KB |
| `grep -rE '^(import\|from) ' .` | 116.4 KB | 21.0 KB | 21.0 KB | 21.0 KB | 21.0 KB | 33.1 KB | 21.0 KB | 21.0 KB |
| `grep -rin 'return [a-z_]*;$' Documentation` | 24.1 KB | 17.5 KB | 714.2 KB | 729.5 KB | 17.5 KB | 714.1 KB | 729.5 KB | 17.5 KB |
| `grep -rv e drivers/usb` | 9638.5 KB | 21.9 KB | 21.9 KB | 21.9 KB | 21.9 KB | 21.9 KB | 21.9 KB | 21.9 KB |
| `grep -r '\(separately\).*\1' .` | 188 B | 346.3 KB | 282.5 KB | 282.5 KB | 284.4 KB | 282.6 KB | 282.5 KB | 284.4 KB |
| `grep -n mispredicted include/linux/*.h` | 0 B | | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB |
| `grep -l separately include/linux/*.h` | 458 B | | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB |
| `grep -c qdbzzabsentterm include/linux/*.h` | 39.5 KB | | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB |
| `grep -rl conditions . \| wc -l` | 5 B | 133.7 KB | 190.3 KB | 190.3 KB | 196.0 KB | 190.5 KB | 190.3 KB | 196.0 KB |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 6 B | 26.0 KB | 26.2 KB | 26.2 KB | 26.2 KB | 42.9 KB | 26.2 KB | 26.2 KB |
| `find . -name '*.h' \| wc -l` | 6 B | 23.4 KB | 23.6 KB | 23.6 KB | 23.6 KB | 23.6 KB | 23.6 KB | 23.6 KB |
| `find . -type f \| grep -c h` | 6 B | 35.8 KB | 35.9 KB | 35.9 KB | 35.9 KB | 35.9 KB | 35.9 KB | 35.9 KB |
| `grep -rn define . \| head -n 5` | 473 B | 10.3 KB | 10.4 KB | 10.4 KB | 10.4 KB | 10.4 KB | 10.4 KB | 10.4 KB |
| `grep -rn conditions . \| head -n 1` | 107 B | 15.3 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB | 15.4 KB |
| `find . -type f \| head -n 3` | 47 B | 32.6 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB |
| `cat include/linux/*.h \| head -n 2` | 42 B | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB |

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
