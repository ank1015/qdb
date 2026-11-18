# Benchmark: linux

- Data: Linux 6.8 source tree, from the Ubuntu source package linux_6.8.0.orig.tar.gz (sha256 26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8). Skipped on import: 64 symbolic links, 6 files that are not valid text, 0 other
- Size: 83405 files in 5451 folders, 1396.9 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (s3p4-cap), and include starting bash and the tool (1.650 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 1 | none | 886.3 | 886.3 | 1416.9 MB | 2279.2 MB |  | yes |
| Part 2 | none | 887.5 | 887.5 | 1416.9 MB | 2279.3 MB |  | yes |
| Part 3, no index | none | 1385.5 | 1385.5 | 1416.9 MB | 1425.4 MB | 0.001 | yes |
| Part 4, index off | off | 1461.7 | 1461.7 | 1418.0 MB | 1426.5 MB | 0.001 | yes |
| Part 4, index on | on | 1374.6 | 8146.1 | 1568.1 MB | 1578.0 MB | 0.280 | yes |
| Stage 3 part 1 | on | 24063.9 | 30804.3 | 1568.1 MB | 1578.1 MB | 0.280 | yes |
| s3p4-cap | on | 20764.6 | 20764.6 | 21.7 MB | 967.2 MB | 0.335 | yes |

## The index (Part 4, index on)

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

## The index (Stage 3 part 1)

- Built in 6740.3 ms while loading.
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
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | documents +1390 -1390; postings +1558561; list bytes rewritten 0; rebuilds 0 |
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
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -9698; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -2633; postings +0; list bytes rewritten 0; rebuilds 0 |

## The index (s3p4-cap)

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

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Part 4, index off | Part 4, index on | Stage 3 part 1 | s3p4-cap | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.785 |  | 0.003 | 0.003 | 0.001 | 0.002 | 0.001 | 0.001 | 0.001 | yes |
| `ls -a` | 2.440 |  | 0.008 | 0.004 | 0.002 | 0.003 | 0.004 | 0.002 | 0.002 | yes |
| `ls -R` | 88.41 |  | 21.31 | 15.79 | 17.85 | 5.651 | 8.025 | 9.345 | 5.154 | yes |
| `ls -aR Documentation` | 11.98 |  | 1.501 | 1.091 | 1.367 | 0.939 | 0.747 | 0.772 | 0.575 | yes |
| `ls -p Documentation` | 2.973 |  | 0.005 | 0.005 | 0.003 | 0.003 | 0.004 | 0.002 | 0.002 | yes |
| `find .` | 66.80 |  | 20.91 | 21.94 | 20.50 | 17.36 | 20.82 | 17.28 | 10.70 | yes |
| `find . -name '*.h'` | 65.57 |  | 31.50 | 23.56 | 26.63 | 27.15 | 25.53 | 30.04 | 10.55 | yes |
| `find . -name '*.c'` | 96.27 |  | 27.93 | 37.57 | 25.61 | 30.65 | 28.25 | 24.21 | 11.61 | yes |
| `find . -type d` | 65.71 |  | 25.24 | 30.06 | 19.60 | 22.18 | 25.02 | 26.30 | 12.00 | yes |
| `find . -type f` | 68.80 |  | 26.23 | 23.04 | 26.47 | 25.24 | 23.39 | 20.13 | 13.84 | yes |
| `find . -maxdepth 2 -type f` | 4.861 |  | 0.144 | 0.201 | 0.139 | 0.120 | 0.114 | 0.108 | 0.086 | yes |
| `find Documentation -name '*.h' -type f` | 12.66 |  | 1.703 | 1.839 | 2.193 | 1.667 | 1.770 | 2.824 | 1.735 | yes |
| `find . -name 'Makefile'` | 72.52 |  | 24.61 | 21.08 | 20.22 | 32.01 | 27.58 | 16.92 | 11.33 | yes |
| `cat include/linux/kernfs.h` | 2.536 |  | 0.002 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `cat -n include/linux/kernfs.h` | 2.345 |  | 0.079 | 0.077 | 0.053 | 0.049 | 0.057 | 0.078 | 0.053 | yes |
| `head -n 20 include/linux/kernfs.h` | 2.504 |  | 0.011 | 0.002 | 0.002 | 0.002 | 0.002 | 0.002 | 0.001 | yes |
| `tail -n 20 include/linux/kernfs.h` | 2.256 |  | 0.030 | 0.002 | 0.002 | 0.002 | 0.002 | 0.003 | 0.002 | yes |
| `tail -n +100 include/linux/kernfs.h` | 2.397 |  | 0.015 | 0.004 | 0.003 | 0.003 | 0.003 | 0.005 | 0.006 | yes |
| `wc include/linux/kernfs.h` | 3.026 |  | 0.037 | 0.032 | 0.019 | 0.020 | 0.024 | 0.037 | 0.027 | yes |
| `wc -l include/linux/*.h` | 11.55 |  | 24.16 | 24.60 | 20.12 | 24.55 | 28.62 | 24.18 | 0.722 | yes |
| `cat include/linux/*.h \| wc -l` | 39.88 |  | 28.57 | 25.07 | 24.20 | 24.73 | 26.33 | 22.00 | 25.12 | yes |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 3.501 |  | 0.017 | 0.017 | 0.006 | 0.006 | 0.005 | 0.006 | 0.006 | yes |
| `grep -c define include/linux/kernfs.h` | 2.086 |  | 0.271 | 0.237 | 0.006 | 0.005 | 0.006 | 0.005 | 0.013 | yes |
| `grep -n define include/linux/kernfs.h` | 2.627 |  | 0.426 | 0.306 | 0.005 | 0.007 | 0.006 | 0.006 | 0.014 | yes |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 3.425 |  | 0.334 | 0.326 | 0.279 | 0.247 | 0.249 | 0.217 | 0.251 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 3.431 |  | 0.740 | 0.664 | 0.637 | 0.492 | 0.588 | 0.537 | 0.470 | yes |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 3.658 |  | 0.168 | 0.145 | 0.124 | 0.106 | 0.152 | 0.104 | 0.120 | yes |
| `grep -rn qdbzzabsentterm .` | 1268.1 | 631.1 | 17359.5 | 17645.1 | 199.4 | 205.6 | 0.012 | 0.010 | 0.064 | yes |
| `grep -rl qdbzzabsentterm .` | 821.2 | 592.4 | 17711.1 | 18597.6 | 192.1 | 214.3 | 0.011 | 0.012 | 0.063 | yes |
| `grep -rn ausgabenspezifisches .` | 1227.0 | 679.9 | 19018.3 | 19501.9 | 210.3 | 208.7 | 0.098 | 0.082 | 0.225 | yes |
| `grep -rl ausgabenspezifisches .` | 822.6 | 662.3 | 18956.5 | 19324.0 | 195.5 | 204.9 | 0.095 | 0.077 | 0.187 | yes |
| `grep -rn mispredicted .` | 1451.6 | 668.9 | 17982.9 | 18955.5 | 205.0 | 209.3 | 17.54 | 15.67 | 1.682 | yes |
| `grep -rl mispredicted .` | 998.8 | 590.5 | 19048.8 | 18957.5 | 195.4 | 208.1 | 17.60 | 13.57 | 1.996 | yes |
| `grep -rn separately .` | 1065.1 | 650.7 | 18360.5 | 18397.0 | 212.4 | 211.4 | 24.09 | 24.22 | 15.17 | yes |
| `grep -rl separately .` | 668.9 | 633.6 | 19440.4 | 19427.6 | 213.6 | 210.7 | 18.81 | 19.01 | 14.64 | yes |
| `grep -rn conditions .` | 1388.3 | 736.5 | 18999.5 | 19519.0 | 229.0 | 227.3 | 129.9 | 141.3 | 62.45 | yes |
| `grep -rl conditions .` | 832.8 | 584.1 | 12072.4 | 12435.8 | 158.7 | 151.5 | 65.31 | 58.36 | 39.35 | yes |
| `grep -rn define .` | 2702.1 | 2232.0 | 15389.6 | 15237.7 | 431.2 | 519.4 | 460.7 | 549.6 | 520.1 | yes |
| `grep -rl define .` | 735.6 | 717.3 | 4161.3 | 3944.5 | 78.19 | 79.67 | 100.9 | 80.38 | 56.65 | yes |
| `grep -rc separately .` | 710.1 | 630.2 | 18956.0 | 18220.2 | 199.0 | 220.2 | 44.65 | 40.37 | 21.20 | yes |
| `grep -rin MISPREDICTED .` | 1759.8 | 775.0 | 18850.7 | 18964.1 | 249.3 | 267.2 | 21.12 | 19.79 | 1.993 | yes |
| `grep -ri separately .` | 939.5 | 749.9 | 19767.8 | 18645.3 | 240.0 | 248.3 | 28.93 | 26.42 | 20.27 | yes |
| `grep -rn separately Documentation` | 84.75 | 68.40 | 711.5 | 557.6 | 5.069 | 6.690 | 2.473 | 2.482 | 1.765 | yes |
| `grep -rl conditions Documentation` | 71.33 | 56.20 | 628.8 | 626.2 | 6.650 | 4.002 | 3.711 | 3.973 | 4.457 | yes |
| `grep -rF 'separately' .` | 690.3 | 665.9 | 2095.5 | 2388.0 | 214.0 | 200.6 | 24.42 | 20.66 | 15.01 | yes |
| `grep -rE 'atomic_[a-z]+\(' .` | 1352.6 | 654.0 | 18552.0 | 18591.7 | 227.6 | 213.8 | 74.42 | 88.18 | 54.31 | yes |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 1863.7 | 733.3 | 25700.0 | 27700.5 | 296.1 | 299.4 | 29.62 | 27.21 | 2.946 | yes |
| `grep -rE 'conditions\|separately' .` | 2294.7 | 803.0 | 26745.7 | 26952.0 | 308.1 | 327.2 | 180.1 | 179.8 | 118.0 | yes |
| `grep -rE '[0-9]{4,}' .` | 2229.0 | 3072.0 | 18321.8 | 19699.7 | 2009.7 | 2033.3 | 2025.8 | 1676.8 | 2045.8 | yes |
| `grep -rn '^#include' .` | 1809.1 | 942.9 | 17544.7 | 16706.7 | 236.6 | 263.2 | 244.7 | 237.6 | 218.3 | yes |
| `grep -rE '^(import\|from) ' .` | 1219.0 | 861.7 | 17091.8 | 18141.6 | 328.9 | 319.1 | 315.7 | 342.1 | 273.9 | yes |
| `grep -rin 'return [a-z_]*;$' Documentation` | 100.8 | 66.82 | 723.2 | 710.6 | 7.054 | 14.82 | 11.32 | 12.78 | 5.660 | yes |
| `grep -rv e drivers/usb` | 41.03 | 54.19 | 132.2 | 107.4 | 14.25 | 18.24 | 17.91 | 18.49 | 21.17 | yes |
| `grep -r '\(separately\).*\1' .` | 693.7 |  | 34553.4 | 35931.9 | 216.6 | 216.4 | 24.81 | 24.74 | 17.88 | yes |
| `grep -n mispredicted include/linux/*.h` | 17.93 |  | | | | | | 1.407 | 0.994 | yes |
| `grep -l separately include/linux/*.h` | 11.72 |  | | | | | | 2.152 | 1.924 | yes |
| `grep -c qdbzzabsentterm include/linux/*.h` | 13.54 |  | | | | | | 0.674 | 0.614 | yes |
| `grep -rl conditions . \| wc -l` | 846.0 |  | 12392.2 | 11985.0 | 156.1 | 156.0 | 58.03 | 57.75 | 38.80 | yes |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1559.5 |  | 14309.4 | 14923.1 | 342.4 | 364.1 | 342.3 | 352.0 | 371.8 | yes |
| `find . -name '*.h' \| wc -l` | 91.84 |  | 28.57 | 24.95 | 27.61 | 27.26 | 23.83 | 28.22 | 13.47 | yes |
| `find . -type f \| grep -c h` | 108.5 |  | 63.40 | 58.08 | 27.30 | 27.73 | 27.69 | 37.65 | 19.69 | yes |
| `grep -rn define . \| head -n 5` | 5.840 |  | 15337.2 | 3.769 | 0.020 | 0.020 | 0.019 | 0.020 | 0.049 | order differs |
| `grep -rn conditions . \| head -n 1` | 14.66 |  | 18581.5 | 6.938 | 0.054 | 0.031 | 0.027 | 0.031 | 0.093 | order differs |
| `find . -type f \| head -n 3` | 4.070 |  | 28.01 | 0.092 | 0.096 | 0.061 | 0.058 | 0.056 | 0.074 | order differs |
| `cat include/linux/*.h \| head -n 2` | 4.585 |  | 9.193 | 0.347 | 0.338 | 0.267 | 0.346 | 0.342 | 0.464 | yes |
| `mkdir -p bench_out/a/b` | 3.057 |  | 0.043 | 0.026 | 0.012 | 0.016 | 0.015 | 0.017 | 2.112 | yes |
| `touch bench_out/empty.txt` | 3.041 |  | 0.028 | 0.019 | 0.012 | 0.013 | 0.009 | 0.013 | 0.368 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.747 |  | 0.030 | 0.025 | 0.016 | 0.017 | 0.017 | 0.018 | 0.294 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.482 |  | 0.021 | 0.023 | 0.012 | 0.008 | 0.018 | 0.062 | 0.376 | yes |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 3.082 |  | 0.071 | 0.044 | 0.018 | 0.022 | 0.510 | 0.730 | 1.284 | yes |
| `cp -r Documentation bench_out/dir_copy` | 280.7 |  | 36.54 | 55.14 | 34.99 | 38.25 | 476.3 | 413.4 | 800.1 | yes |
| `cp -r include/linux bench_out/work` | 48.37 |  | 17.02 | 21.90 | 17.83 | 19.81 | 172.5 | 157.4 | 353.8 | yes |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 3.355 |  | 0.369 | 0.371 | 0.257 | 0.387 | 0.545 | 0.602 | 1.369 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.093 |  | 0.608 | 0.397 | 0.267 | 0.328 | 0.478 | 0.500 | 1.074 | yes |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 103.9 |  | 154.4 | 187.8 | 133.3 | 141.4 | 212.9 | 241.7 | 402.8 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 455.8 |  | | | | | | 2.724 | 5.558 | yes |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 405.2 |  | | | | | | 2.975 | 5.212 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 3.780 |  | | | | | | 0.027 | 0.048 | yes |
| `cp include/linux/kernfs.h bench_out/same.txt` | 3.729 |  | | | | | | 0.027 | 0.048 | yes |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 3.186 |  | | | | | | 0.033 | 0.034 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 81.05 |  | 907.6 | 911.1 | 14.07 | 17.24 | 7.128 | 11.97 | 10.22 | yes |
| `grep -rl QDBBENCHMANY .` | 649.4 |  | 18985.7 | 19134.6 | 224.7 | 241.6 | 3.630 | 2.154 | 6.574 | yes |
| `grep -rl QDBBENCHONE .` | 973.2 |  | 18818.5 | 18986.5 | 231.2 | 239.5 | 0.603 | 0.783 | 0.425 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 2.591 |  | 0.036 | 0.048 | 0.023 | 0.026 | 0.035 | 0.028 | 0.331 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.671 |  | 0.046 | 0.046 | 0.020 | 0.020 | 0.015 | 0.018 | 0.296 | yes |
| `ls -R bench_out/a` | 2.399 |  | 0.026 | 0.035 | 0.014 | 0.016 | 0.019 | 0.021 | 0.016 | yes |
| `rm bench_out/a/b/hello.txt` | 2.136 |  | 0.030 | 0.035 | 0.008 | 0.013 | 0.011 | 0.012 | 0.309 | yes |
| `rmdir bench_out/a/b` | 2.563 |  | 0.035 | 0.038 | 0.016 | 0.009 | 0.015 | 0.010 | 0.285 | yes |
| `rm -r bench_out/dir_moved` | 192.3 |  | 10.95 | 9.650 | 5.616 | 6.801 | 8.592 | 6.623 | 13.21 | yes |
| `rm -rf bench_out` | 239.0 |  | 3.379 | 2.941 | 1.655 | 2.071 | 3.696 | 4.601 | 31.98 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 | Part 3, no index | Part 4, index off | Part 4, index on | Stage 3 part 1 | s3p4-cap |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 212 B | 1617 B | 1457 B | 1457 B | 1457 B | 1457 B | 1457 B | 1457 B |
| `ls -a` | 330 B | 2841 B | 2681 B | 2681 B | 2681 B | 2681 B | 2681 B | 2681 B |
| `ls -R` | 1377.7 KB | 2155.2 KB | 74.3 KB | 74.3 KB | 74.3 KB | 74.3 KB | 74.3 KB | 74.3 KB |
| `ls -aR Documentation` | 216.8 KB | 437.9 KB | 56.6 KB | 56.6 KB | 56.6 KB | 56.6 KB | 56.6 KB | 56.6 KB |
| `ls -p Documentation` | 796 B | 5053 B | 4893 B | 4893 B | 4893 B | 4893 B | 4893 B | 4893 B |
| `find .` | 3590.9 KB | 4457.9 KB | 19.2 KB | 19.2 KB | 19.2 KB | 19.2 KB | 19.2 KB | 19.2 KB |
| `find . -name '*.h'` | 969.5 KB | 1312.4 KB | 22.7 KB | 22.7 KB | 22.7 KB | 22.7 KB | 22.7 KB | 22.7 KB |
| `find . -name '*.c'` | 1233.5 KB | 1247.0 KB | 21.7 KB | 21.7 KB | 21.7 KB | 21.7 KB | 21.7 KB | 21.7 KB |
| `find . -type d` | 166.7 KB | 280.0 KB | 19.5 KB | 19.5 KB | 19.5 KB | 19.5 KB | 19.5 KB | 19.5 KB |
| `find . -type f` | 3424.2 KB | 3933.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB | 32.8 KB |
| `find . -maxdepth 2 -type f` | 23.5 KB | 32.1 KB | 32.0 KB | 32.0 KB | 32.0 KB | 32.0 KB | 32.0 KB | 32.0 KB |
| `find Documentation -name '*.h' -type f` | 76 B | 2097 B | 1937 B | 1937 B | 1937 B | 1937 B | 1937 B | 1937 B |
| `find . -name 'Makefile'` | 100.1 KB | 198.2 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB |
| `cat include/linux/kernfs.h` | 19.9 KB | 20.7 KB | 994 B | 994 B | 994 B | 994 B | 994 B | 994 B |
| `cat -n include/linux/kernfs.h` | 24.4 KB | 27.0 KB | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB |
| `head -n 20 include/linux/kernfs.h` | 459 B | 1505 B | 1345 B | 1345 B | 1345 B | 1345 B | 1345 B | 1345 B |
| `tail -n 20 include/linux/kernfs.h` | 478 B | 1524 B | 1364 B | 1364 B | 1364 B | 1364 B | 1364 B | 1364 B |
| `tail -n +100 include/linux/kernfs.h` | 16.9 KB | 18.0 KB | 1306 B | 1306 B | 1306 B | 1306 B | 1306 B | 1306 B |
| `wc include/linux/kernfs.h` | 41 B | 946 B | 1002 B | 1002 B | 1002 B | 1002 B | 1002 B | 1002 B |
| `wc -l include/linux/*.h` | 49.6 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB | 237.0 KB |
| `cat include/linux/*.h \| wc -l` | 7 B | 12.4 MB | 237.2 KB | 237.2 KB | 237.2 KB | 237.2 KB | 237.2 KB | 237.2 KB |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 447 B | 23.1 KB | 4720 B | 4720 B | 4720 B | 4720 B | 4720 B | 4944 B |
| `grep -c define include/linux/kernfs.h` | 2 B | 5500 B | 5284 B | 9342 B | 9470 B | 9470 B | 9470 B | 9470 B |
| `grep -n define include/linux/kernfs.h` | 374 B | 5916 B | 5700 B | 9342 B | 9470 B | 9470 B | 9470 B | 9470 B |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 19.9 KB | 27.1 KB | 27.0 KB | 27.0 KB | 27.0 KB | 27.0 KB | 27.0 KB | 27.0 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 19.9 KB | 26.6 KB | 26.4 KB | 26.6 KB | 26.6 KB | 26.6 KB | 26.6 KB | 26.6 KB |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 6 B | 45.1 KB | 25.5 KB | 25.8 KB | 25.8 KB | 25.8 KB | 25.8 KB | 26.0 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 12.2 KB | 12.0 KB | 19.6 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 12.2 KB | 12.0 KB | 19.6 KB | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rn ausgabenspezifisches .` | 116 B | 16.0 KB | 15.8 KB | 26.1 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rl ausgabenspezifisches .` | 32 B | 16.0 KB | 15.8 KB | 26.1 KB | 27.8 KB | 27.8 KB | 27.8 KB | 27.8 KB |
| `grep -rn mispredicted .` | 111.0 KB | 165.5 KB | 19.4 KB | 16.4 KB | 17.1 KB | 90.8 KB | 90.8 KB | 51.9 KB |
| `grep -rl mispredicted .` | 4531 B | 14.7 KB | 10.4 KB | 16.4 KB | 17.1 KB | 90.8 KB | 90.8 KB | 51.9 KB |
| `grep -rn separately .` | 103.9 KB | 176.7 KB | 10.5 KB | 14.2 KB | 14.4 KB | 343.4 KB | 343.4 KB | 279.6 KB |
| `grep -rl separately .` | 30.7 KB | 50.9 KB | 9318 B | 14.2 KB | 14.4 KB | 342.9 KB | 342.9 KB | 279.0 KB |
| `grep -rn conditions .` | 1360.5 KB | 2564.6 KB | 13.5 KB | 14.2 KB | 14.4 KB | 136.2 KB | 136.2 KB | 191.1 KB |
| `grep -rl conditions .` | 389.6 KB | 647.9 KB | 9392 B | 14.2 KB | 14.4 KB | 133.0 KB | 133.0 KB | 189.4 KB |
| `grep -rn define .` | 821.1 MB | 830.5 MB | 31.8 KB | 26.8 KB | 26.9 KB | 26.9 KB | 26.9 KB | 26.9 KB |
| `grep -rl define .` | 1867.6 KB | 1972.3 KB | 6624 B | 9290 B | 9418 B | 9418 B | 9418 B | 9418 B |
| `grep -rc separately .` | 3591.0 KB | 3941.1 KB | 40.0 KB | 19.9 KB | 20.2 KB | 361.3 KB | 361.3 KB | 297.5 KB |
| `grep -rin MISPREDICTED .` | 135.7 KB | 165.5 KB | 19.4 KB | 16.4 KB | 17.1 KB | 90.8 KB | 90.8 KB | 51.8 KB |
| `grep -ri separately .` | 100.6 KB | 176.7 KB | 10.5 KB | 14.2 KB | 14.5 KB | 343.3 KB | 343.3 KB | 279.5 KB |
| `grep -rn separately Documentation` | 19.9 KB | 28.7 KB | 9151 B | 14.2 KB | 14.5 KB | 35.4 KB | 35.4 KB | 43.8 KB |
| `grep -rl conditions Documentation` | 9541 B | 27.3 KB | 8675 B | 14.2 KB | 14.5 KB | 133.0 KB | 133.0 KB | 189.4 KB |
| `grep -rF 'separately' .` | 99.7 KB | 169.6 KB | 3480 B | 2284 B | 2540 B | 343.3 KB | 343.3 KB | 279.5 KB |
| `grep -rE 'atomic_[a-z]+\(' .` | 1294.9 KB | 2040.2 KB | 16.5 KB | 23.0 KB | 23.2 KB | 945.1 KB | 945.1 KB | 1037.7 KB |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 109.2 KB | 179.1 KB | 33.0 KB | 41.2 KB | 43.1 KB | 62.6 KB | 62.6 KB | 53.6 KB |
| `grep -rE 'conditions\|separately' .` | 1427.8 KB | 2571.4 KB | 20.3 KB | 26.8 KB | 27.4 KB | 188.8 KB | 188.8 KB | 195.7 KB |
| `grep -rE '[0-9]{4,}' .` | 467.5 MB | 729.8 MB | 27.1 KB | 28.6 KB | 28.6 KB | 28.6 KB | 28.6 KB | 28.6 KB |
| `grep -rn '^#include' .` | 24.9 MB | 33.6 MB | 24.5 KB | 29.4 KB | 29.6 KB | 29.6 KB | 29.6 KB | 29.6 KB |
| `grep -rE '^(import\|from) ' .` | 116.4 KB | 144.7 KB | 11.5 KB | 20.9 KB | 21.0 KB | 21.0 KB | 21.0 KB | 21.0 KB |
| `grep -rin 'return [a-z_]*;$' Documentation` | 24.1 KB | 44.1 KB | 12.5 KB | 17.5 KB | 17.5 KB | 17.5 KB | 17.5 KB | 729.5 KB |
| `grep -rv e drivers/usb` | 9638.5 KB | 9964.0 KB | 21.8 KB | 21.9 KB | 21.9 KB | 21.9 KB | 21.9 KB | 21.9 KB |
| `grep -r '\(separately\).*\1' .` | 188 B | 6770 B | 6376 B | 9274 B | 9530 B | 346.3 KB | 346.3 KB | 282.5 KB |
| `grep -n mispredicted include/linux/*.h` | 0 B | | | | | | 237.3 KB | 237.3 KB |
| `grep -l separately include/linux/*.h` | 458 B | | | | | | 237.3 KB | 237.3 KB |
| `grep -c qdbzzabsentterm include/linux/*.h` | 39.5 KB | | | | | | 237.3 KB | 237.3 KB |
| `grep -rl conditions . \| wc -l` | 5 B | 648.5 KB | 10.2 KB | 14.9 KB | 15.2 KB | 133.7 KB | 133.7 KB | 190.3 KB |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 6 B | 6164.4 KB | 25.4 KB | 25.8 KB | 26.0 KB | 26.0 KB | 26.0 KB | 26.2 KB |
| `find . -name '*.h' \| wc -l` | 6 B | 1313.0 KB | 23.4 KB | 23.4 KB | 23.4 KB | 23.4 KB | 23.4 KB | 23.6 KB |
| `find . -type f \| grep -c h` | 6 B | 3935.0 KB | 34.9 KB | 35.8 KB | 35.8 KB | 35.8 KB | 35.8 KB | 35.9 KB |
| `grep -rn define . \| head -n 5` | 473 B | 830.5 MB | 6667 B | 10.2 KB | 10.3 KB | 10.3 KB | 10.3 KB | 10.4 KB |
| `grep -rn conditions . \| head -n 1` | 107 B | 2565.4 KB | 9150 B | 15.0 KB | 15.3 KB | 15.3 KB | 15.3 KB | 15.4 KB |
| `find . -type f \| head -n 3` | 47 B | 3934.6 KB | 32.6 KB | 32.6 KB | 32.6 KB | 32.6 KB | 32.6 KB | 32.8 KB |
| `cat include/linux/*.h \| head -n 2` | 42 B | 12.4 MB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB | 237.3 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1619 B | 1459 B | 1459 B | 1459 B | 1459 B | 1459 B | 1747 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B | 979 B | 979 B | 979 B | 979 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B | 1242 B | 1242 B | 1242 B | 1242 B | 1268 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B | 1246 B | 1246 B | 1246 B | 1246 B | 1246 B |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 0 B | 21.0 KB | 20.8 KB | 20.8 KB | 20.8 KB | 26.0 KB | 26.0 KB | 283.2 KB |
| `cp -r Documentation bench_out/dir_copy` | 0 B | 48.1 MB | 48.1 MB | 48.1 MB | 48.1 MB | 59.4 MB | 59.4 MB | 21.7 MB |
| `cp -r include/linux bench_out/work` | 0 B | 19.1 MB | 19.1 MB | 19.1 MB | 19.1 MB | 22.2 MB | 22.2 MB | 13.6 MB |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 27.3 KB | 27.2 KB | 27.2 KB | 27.2 KB | 27.6 KB | 28.0 KB | 47.8 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 24.9 KB | 24.7 KB | 24.8 KB | 24.8 KB | 25.8 KB | 28.5 KB | 47.6 KB |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 0 B | 4911.5 KB | 4911.3 KB | 4911.4 KB | 4911.4 KB | 6764.6 KB | 6737.5 KB | 6254.3 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.h` | 0 B | | | | | | 250.1 KB | 250.1 KB |
| `sed -i 's/mispredicted/QDBBENCHFEW/g' bench_out/work/*.h` | 0 B | | | | | | 248.6 KB | 248.6 KB |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 B | | | | | | 20.8 KB | 20.8 KB |
| `cp include/linux/kernfs.h bench_out/same.txt` | 0 B | | | | | | 20.8 KB | 20.8 KB |
| `cat include/linux/kernfs.h > bench_out/same.txt` | 0 B | | | | | | 21.1 KB | 21.0 KB |
| `grep -rc QDBBENCHMANY bench_out` | 687.0 KB | 1189.3 KB | 28.1 KB | 28.7 KB | 29.2 KB | 191.9 KB | 191.9 KB | 206.0 KB |
| `grep -rl QDBBENCHMANY .` | 39.7 KB | 65.9 KB | 10.4 KB | 16.4 KB | 17.1 KB | 164.6 KB | 164.6 KB | 178.7 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 10.0 KB | 9829 B | 15.3 KB | 16.0 KB | 16.0 KB | 16.0 KB | 16.0 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B | 1191 B | 1191 B | 1191 B | 1191 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B | 1288 B | 1288 B | 1288 B | 1288 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B | 1089 B | 1089 B | 1089 B | 1089 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B | 997 B | 997 B | 997 B | 997 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B | 945 B | 945 B | 945 B | 945 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1611 B | 1451 B | 1451 B | 1451 B | 1451 B | 1451 B | 1355 B |
| `rm -rf bench_out` | 0 B | 12.5 KB | 12.4 KB | 12.4 KB | 12.4 KB | 12.4 KB | 12.4 KB | 12.2 KB |

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
