# Benchmark: rfc

- Data: The RFC series as text, to January 2017 (prose, not source code), from the Ubuntu source package doc-rfc_20170121.orig.tar.gz (sha256 7d0d6d76ade0a75cd87635b4794a407e1f845ad9fe0c849b660fcce9196855b2). Skipped on import: 0 symbolic links, 181 files that are not valid text, 0 other
- Size: 7946 files in 4 folders, 443.7 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 3 part 1), and include starting bash and the tool (1.698 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 3 part 1 | on | 464.3 | 2105.7 | 484.6 MB | 490.2 MB | 0.344 | yes |

## The index (Stage 3 part 1)

- Built in 1641.4 ms while loading.
- Size: 36.5 MB in memory, 8.2% of the text (4194.3 KB table, 32.2 MB lists, 131.1 KB document table). The posting data itself is 26.2 MB.
- 179043 distinct trigrams, 25202658 postings, 7946 documents.
- Share of posting data in the longest lists: top 100 trigrams 3.0%, top 1,000 27.9%, top 10,000 84.7%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|
| 1 | 45390 |
| 2 | 18402 |
| 3-4 | 25151 |
| 5-8 | 26301 |
| 9-16 | 19397 |
| 17-32 | 12309 |
| 33-64 | 10220 |
| 65-128 | 6489 |
| 129-256 | 4339 |
| 257-512 | 3463 |
| 513-1024 | 2459 |
| 1025-2048 | 1717 |
| 2049-4096 | 1295 |
| 4097-8192 | 2111 |

Index work per command that changes data:

| Command | Index work |
|---|---|
| `mkdir -p bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `touch bench_out/empty.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `echo 'hello world' > bench_out/hello.txt` | documents +1 -0; postings +9; list bytes rewritten 0; rebuilds 0 |
| `echo 'another line' >> bench_out/hello.txt` | documents +1 -1; postings +19; list bytes rewritten 0; rebuilds 0 |
| `cp rfc1641.ps bench_out/copy.txt` | documents +1 -0; postings +4365; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/including/QDBBENCHONE/g' bench_out/copy.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +3374; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/including/QDBBENCHMANY/g' bench_out/work/*.ps` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.ps` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/currentcolortransfer/QDBBENCHFEW/g' bench_out/work/*.ps` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cp rfc1641.ps bench_out/same.txt` | documents +1 -0; postings +4365; list bytes rewritten 0; rebuilds 0 |
| `cp rfc1641.ps bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat rfc1641.ps > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -2; postings +0; list bytes rewritten 0; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 3 part 1 | Agree |
|---|---:|---:|---:|---|
| `ls` | 7.603 |  | 0.418 | yes |
| `ls -a` | 7.479 |  | 0.695 | yes |
| `ls -R` | 9.220 |  | 0.487 | yes |
| `find .` | 7.422 |  | 1.296 | yes |
| `find . -name '*.ps'` | 6.995 |  | 1.050 | yes |
| `find . -name '*.txt'` | 6.426 |  | 1.601 | yes |
| `find . -type d` | 5.153 |  | 0.868 | yes |
| `find . -type f` | 6.103 |  | 0.849 | yes |
| `find . -maxdepth 2 -type f` | 5.261 |  | 1.403 | yes |
| `find . -name 'Makefile'` | 5.543 |  | 0.837 | yes |
| `cat rfc1641.ps` | 2.320 |  | 0.001 | yes |
| `cat -n rfc1641.ps` | 2.211 |  | 0.007 | yes |
| `head -n 20 rfc1641.ps` | 2.213 |  | 0.009 | yes |
| `tail -n 20 rfc1641.ps` | 2.174 |  | 0.010 | yes |
| `tail -n +100 rfc1641.ps` | 2.118 |  | 0.010 | yes |
| `wc rfc1641.ps` | 2.529 |  | 0.023 | yes |
| `wc -l *.ps` | 8.217 |  | 40.59 | yes |
| `cat *.ps \| wc -l` | 28.35 |  | 38.61 | yes |
| `cat rfc1641.ps \| head -n 50 \| tail -n 10` | 2.908 |  | 0.017 | yes |
| `grep -c including rfc1641.ps` | 2.427 |  | 0.007 | yes |
| `grep -n including rfc1641.ps` | 2.354 |  | 0.007 | yes |
| `sed 's/including/XX/g' rfc1641.ps` | 2.549 |  | 0.206 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' rfc1641.ps` | 2.993 |  | 0.484 | yes |
| `cat rfc1641.ps \| sed 's/^/> /' \| wc -c` | 3.931 |  | 0.061 | yes |
| `grep -rn qdbzzabsentterm .` | 347.1 | 120.4 | 0.049 | yes |
| `grep -rl qdbzzabsentterm .` | 239.0 | 119.7 | 0.071 | yes |
| `grep -rn adjacencyisdestroyed .` | 324.9 | 142.8 | 0.262 | yes |
| `grep -rl adjacencyisdestroyed .` | 222.3 | 110.1 | 0.116 | yes |
| `grep -rn currentcolortransfer .` | 355.0 | 136.1 | 1.082 | yes |
| `grep -rl currentcolortransfer .` | 238.9 | 115.8 | 0.397 | yes |
| `grep -rn characterizes .` | 402.9 | 124.2 | 26.95 | yes |
| `grep -rl characterizes .` | 288.4 | 131.2 | 24.25 | yes |
| `grep -rn statistics .` | 404.7 | 152.4 | 61.10 | yes |
| `grep -rl statistics .` | 288.1 | 107.1 | 51.45 | yes |
| `grep -rn including .` | 359.3 | 156.5 | 72.71 | yes |
| `grep -rl including .` | 160.2 | 115.6 | 30.05 | yes |
| `grep -rc characterizes .` | 284.2 | 131.3 | 24.20 | yes |
| `grep -rin CURRENTCOLORTRANSFER .` | 395.0 | 163.8 | 1.281 | yes |
| `grep -ri characterizes .` | 333.4 | 160.5 | 30.60 | yes |
| `grep -rF 'characterizes' .` | 268.9 | 111.5 | 22.95 | yes |
| `grep -rE 'client_[a-z]+\(' .` | 105.6 | 111.7 | 2.727 | yes |
| `grep -rE 'currentcolortransfer\|adjacencyisdestroyed' .` | 897.7 | 162.5 | 3.156 | yes |
| `grep -rE 'statistics\|characterizes' .` | 880.7 | 149.3 | 86.57 | yes |
| `grep -rE '[0-9]{4,}' .` | 719.0 | 621.9 | 414.6 | yes |
| `grep -rn '^#include' .` | 416.6 | 127.0 | 0.909 | yes |
| `grep -rE '^(import\|from) ' .` | 382.2 | 157.0 | 89.07 | yes |
| `grep -r '\(characterizes\).*\1' .` | 278.8 |  | 27.40 | yes |
| `grep -n currentcolortransfer *.ps` | 22.28 |  | 1.519 | yes |
| `grep -l characterizes *.ps` | 16.71 |  | 1.538 | yes |
| `grep -c qdbzzabsentterm *.ps` | 15.30 |  | 0.556 | yes |
| `grep -rl statistics . \| wc -l` | 304.9 |  | 53.90 | yes |
| `grep -rc including . \| grep -v ':0$' \| wc -l` | 259.6 |  | 64.12 | yes |
| `find . -name '*.ps' \| wc -l` | 8.889 |  | 1.249 | yes |
| `find . -type f \| grep -c ps` | 9.118 |  | 1.769 | yes |
| `grep -rn including . \| head -n 5` | 8.151 |  | 0.141 | order differs |
| `grep -rn statistics . \| head -n 1` | 18.28 |  | 0.072 | order differs |
| `find . -type f \| head -n 3` | 6.180 |  | 0.686 | order differs |
| `cat *.ps \| head -n 2` | 5.106 |  | 0.355 | yes |
| `mkdir -p bench_out/a/b` | 3.523 |  | 0.011 | yes |
| `touch bench_out/empty.txt` | 3.097 |  | 0.008 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.663 |  | 0.041 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.596 |  | 0.011 | yes |
| `cp rfc1641.ps bench_out/copy.txt` | 3.384 |  | 0.516 | yes |
| `sed -i 's/including/QDBBENCHONE/g' bench_out/copy.txt` | 2.371 |  | 0.052 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 4.164 |  | 0.686 | yes |
| `sed -i 's/including/QDBBENCHMANY/g' bench_out/work/*.ps` | 3.383 |  | 0.038 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.ps` | 3.061 |  | 0.047 | yes |
| `sed -i 's/currentcolortransfer/QDBBENCHFEW/g' bench_out/work/*.ps` | 3.113 |  | 0.059 | yes |
| `cp rfc1641.ps bench_out/same.txt` | 3.106 |  | 0.017 | yes |
| `cp rfc1641.ps bench_out/same.txt` | 3.547 |  | 0.017 | yes |
| `cat rfc1641.ps > bench_out/same.txt` | 4.120 |  | 0.019 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 3.281 |  | 0.045 | yes |
| `grep -rl QDBBENCHMANY .` | 109.9 |  | 0.173 | yes |
| `grep -rl QDBBENCHONE .` | 180.1 |  | 0.261 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 4.112 |  | 0.022 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 3.299 |  | 0.015 | yes |
| `ls -R bench_out/a` | 3.905 |  | 0.024 | yes |
| `rm bench_out/a/b/hello.txt` | 2.838 |  | 0.016 | yes |
| `rmdir bench_out/a/b` | 3.958 |  | 0.012 | yes |
| `rm -r bench_out/dir_moved` | 3.092 |  | 0.017 | yes |
| `rm -rf bench_out` | 4.480 |  | 0.021 | yes |

## Peak memory per command

| Command | Output | Stage 3 part 1 |
|---|---:|---:|
| `ls` | 94.2 KB | 213.4 KB |
| `ls -a` | 94.2 KB | 387.9 KB |
| `ls -R` | 94.3 KB | 216.1 KB |
| `find .` | 110.2 KB | 16.9 KB |
| `find . -name '*.ps'` | 657 B | 1758 B |
| `find . -name '*.txt'` | 109.5 KB | 32.8 KB |
| `find . -type d` | 52 B | 1205 B |
| `find . -type f` | 110.2 KB | 32.6 KB |
| `find . -maxdepth 2 -type f` | 110.1 KB | 31.9 KB |
| `find . -name 'Makefile'` | 0 B | 1436 B |
| `cat rfc1641.ps` | 20.5 KB | 942 B |
| `cat -n rfc1641.ps` | 20.5 KB | 1086 B |
| `head -n 20 rfc1641.ps` | 20.5 KB | 1254 B |
| `tail -n 20 rfc1641.ps` | 20.5 KB | 1254 B |
| `tail -n +100 rfc1641.ps` | 0 B | 1254 B |
| `wc rfc1641.ps` | 29 B | 950 B |
| `wc -l *.ps` | 1022 B | 6267 B |
| `cat *.ps \| wc -l` | 8 B | 6449 B |
| `cat rfc1641.ps \| head -n 50 \| tail -n 10` | 20.5 KB | 22.7 KB |
| `grep -c including rfc1641.ps` | 2 B | 13.4 KB |
| `grep -n including rfc1641.ps` | 0 B | 13.4 KB |
| `sed 's/including/XX/g' rfc1641.ps` | 20.5 KB | 27.4 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' rfc1641.ps` | 20.5 KB | 25.0 KB |
| `cat rfc1641.ps \| sed 's/^/> /' \| wc -c` | 6 B | 63.7 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB |
| `grep -rn adjacencyisdestroyed .` | 49.3 KB | 27.8 KB |
| `grep -rl adjacencyisdestroyed .` | 13 B | 27.8 KB |
| `grep -rn currentcolortransfer .` | 49.9 KB | 27.8 KB |
| `grep -rl currentcolortransfer .` | 103 B | 27.8 KB |
| `grep -rn characterizes .` | 8005 B | 18.5 KB |
| `grep -rl characterizes .` | 1113 B | 18.4 KB |
| `grep -rn statistics .` | 368.4 KB | 14.4 KB |
| `grep -rl statistics .` | 11.1 KB | 14.4 KB |
| `grep -rn including .` | 1749.5 KB | 33.2 KB |
| `grep -rl including .` | 66.2 KB | 13.3 KB |
| `grep -rc characterizes .` | 126.1 KB | 26.9 KB |
| `grep -rin CURRENTCOLORTRANSFER .` | 49.9 KB | 27.8 KB |
| `grep -ri characterizes .` | 8198 B | 18.5 KB |
| `grep -rF 'characterizes' .` | 7616 B | 18.5 KB |
| `grep -rE 'client_[a-z]+\(' .` | 1183 B | 46.7 KB |
| `grep -rE 'currentcolortransfer\|adjacencyisdestroyed' .` | 99.2 KB | 53.1 KB |
| `grep -rE 'statistics\|characterizes' .` | 360.8 KB | 31.0 KB |
| `grep -rE '[0-9]{4,}' .` | 73.0 MB | 22.5 KB |
| `grep -rn '^#include' .` | 5500 B | 19.6 KB |
| `grep -rE '^(import\|from) ' .` | 12.1 KB | 21.0 KB |
| `grep -r '\(characterizes\).*\1' .` | 0 B | 22.8 KB |
| `grep -n currentcolortransfer *.ps` | 49.9 KB | 29.8 KB |
| `grep -l characterizes *.ps` | 0 B | 20.3 KB |
| `grep -c qdbzzabsentterm *.ps` | 657 B | 22.3 KB |
| `grep -rl statistics . \| wc -l` | 4 B | 15.2 KB |
| `grep -rc including . \| grep -v ':0$' \| wc -l` | 5 B | 15.6 KB |
| `find . -name '*.ps' \| wc -l` | 3 B | 2519 B |
| `find . -type f \| grep -c ps` | 3 B | 38.0 KB |
| `grep -rn including . \| head -n 5` | 409 B | 14.2 KB |
| `grep -rn statistics . \| head -n 1` | 95 B | 15.3 KB |
| `find . -type f \| head -n 3` | 59 B | 33.5 KB |
| `cat *.ps \| head -n 2` | 39 B | 6578 B |
| `mkdir -p bench_out/a/b` | 0 B | 1459 B |
| `touch bench_out/empty.txt` | 0 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1246 B |
| `cp rfc1641.ps bench_out/copy.txt` | 0 B | 24.7 KB |
| `sed -i 's/including/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 18.2 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 44.8 KB |
| `sed -i 's/including/QDBBENCHMANY/g' bench_out/work/*.ps` | 0 B | 18.2 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.ps` | 0 B | 28.6 KB |
| `sed -i 's/currentcolortransfer/QDBBENCHFEW/g' bench_out/work/*.ps` | 0 B | 39.0 KB |
| `cp rfc1641.ps bench_out/same.txt` | 0 B | 21.3 KB |
| `cp rfc1641.ps bench_out/same.txt` | 0 B | 21.3 KB |
| `cat rfc1641.ps > bench_out/same.txt` | 0 B | 21.6 KB |
| `grep -rc QDBBENCHMANY bench_out` | 86 B | 17.1 KB |
| `grep -rl QDBBENCHMANY .` | 0 B | 17.1 KB |
| `grep -rl QDBBENCHONE .` | 0 B | 16.0 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1129 B |
| `rm -rf bench_out` | 0 B | 1083 B |

## Search terms and paths

| Placeholder | Value | Measured |
|---|---|---|
| ABSENT | `qdbzzabsentterm` | 0 of 7946 files, 0.00% |
| COMMON | `including` | 4751 of 7946 files, 59.79% |
| EXT | `ps` | 50 files in the whole dataset |
| EXT2 | `txt` | 7896 files in the whole dataset |
| FEW | `characterizes` | 80 of 7946 files, 1.01% |
| FILE | `rfc1641.ps` | 20451 bytes |
| GLOB | `*.ps` | 50 files |
| ONE | `adjacencyisdestroyed` | 1 of 7946 files, 0.01% |
| PREFIX | `client` | PREFIX_[a-z]+\( matches in 11 of 7946 files, 0.14% |
| RARE | `currentcolortransfer` | 8 of 7946 files, 0.10% |
| RAREUP | `CURRENTCOLORTRANSFER` | the RARE term in capitals, for -i |
| SOME | `statistics` | 796 of 7946 files, 10.02% |
