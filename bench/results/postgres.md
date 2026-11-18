# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (On disk, after the writes), and include starting bash and the tool (1.874 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 2, in memory | on | 178.8 | 978.7 | 187.2 MB | 192.5 MB | 0.272 | yes |
| Stage 3, in memory | on | 771.7 | 1719.3 | 199.6 MB | 204.4 MB | 0.284 | yes |
| On disk, settled | on | 1562.6 | 1562.6 | 2238.5 KB | 136.4 MB | 0.316 | yes |
| On disk, after the writes | on | 1562.6 | 1562.6 | 2238.5 KB | 136.4 MB | 0.316 | yes |

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

## The index (Stage 3, in memory)

- Built in 947.7 ms while loading.
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
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `cp -r src/backend bench_out/dir_copy` | documents +2007 -0; postings +4681094; list bytes rewritten 0; rebuilds 0 |
| `cp -r doc/src/sgml/html bench_out/work` | documents +1212 -0; postings +2215436; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | documents +1 -1; postings +2793; list bytes rewritten 0; rebuilds 0 |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | documents +1 -1; postings +2724; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | documents +984 -954; postings +1919421; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | documents +1 -1; postings +2482; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +1 -0; postings +2774; list bytes rewritten 0; rebuilds 0 |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rc QDBBENCHMANY bench_out` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHMANY .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `grep -rl QDBBENCHONE .` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/dir_copy bench_out/dir_moved` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `mv bench_out/hello.txt bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `ls -R bench_out/a` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm bench_out/a/b/hello.txt` | documents +0 -1; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rmdir bench_out/a/b` | documents +0 -0; postings +0; list bytes rewritten 0; rebuilds 0 |
| `rm -r bench_out/dir_moved` | documents +0 -2007; postings +0; list bytes rewritten 9828306; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1244; postings +0; list bytes rewritten 5063410; rebuilds 0 |

## The index (On disk, settled)

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

## The index (On disk, after the writes)

- Built in 0.000 ms while loading.
- Size: 18.3 MB in memory, 11.4% of the text (2653.7 KB table, 15.6 MB lists, 85.1 KB document table). The posting data itself is 15.6 MB.
- 152072 distinct trigrams, 0 postings, 9604 documents.
- Share of posting data in the longest lists: top 100 trigrams 0.0%, top 1,000 0.0%, top 10,000 0.0%.
- These figures are taken straight after loading.

| Files in a trigram's list | Trigrams |
|---:|---:|

## Time per command, in milliseconds

| Command | GNU | rg | Stage 2, in memory | Stage 3, in memory | On disk, settled | On disk, after the writes | Agree |
|---|---:|---:|---:|---:|---:|---:|---|
| `ls` | 3.451 |  | 0.001 | 0.001 | 0.001 | 0.002 | yes |
| `ls -a` | 3.458 |  | 0.002 | 0.002 | 0.002 | 0.003 | yes |
| `ls -R` | 10.99 |  | 0.563 | 0.408 | 0.288 | 0.391 | yes |
| `ls -aR src/backend` | 4.545 |  | 0.059 | 0.056 | 0.055 | 0.058 | yes |
| `ls -p src/backend` | 2.845 |  | 0.002 | 0.002 | 0.002 | 0.003 | yes |
| `find .` | 14.46 |  | 1.298 | 1.121 | 0.767 | 0.757 | yes |
| `find . -name '*.html'` | 10.91 |  | 1.444 | 1.093 | 0.953 | 0.959 | yes |
| `find . -name '*.c'` | 13.22 |  | 1.706 | 1.131 | 1.780 | 1.029 | yes |
| `find . -type d` | 8.916 |  | 1.403 | 0.891 | 0.718 | 0.775 | yes |
| `find . -type f` | 10.67 |  | 1.397 | 1.540 | 1.182 | 0.802 | yes |
| `find . -maxdepth 2 -type f` | 3.310 |  | 0.015 | 0.014 | 0.015 | 0.011 | yes |
| `find src/backend -name '*.html' -type f` | 4.182 |  | 0.177 | 0.114 | 0.189 | 0.119 | yes |
| `find . -name 'Makefile'` | 9.555 |  | 2.052 | 0.853 | 1.344 | 0.804 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 2.653 |  | 0.001 | 0.001 | 0.002 | 0.001 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 2.618 |  | 0.028 | 0.027 | 0.048 | 0.041 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 2.931 |  | 0.003 | 0.004 | 0.003 | 0.004 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2.646 |  | 0.003 | 0.003 | 0.002 | 0.004 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 2.807 |  | 0.006 | 0.006 | 0.005 | 0.009 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 3.621 |  | 0.023 | 0.021 | 0.020 | 0.021 | yes |
| `wc -l doc/src/sgml/html/*.html` | 17.86 |  | 27.94 | 0.822 | 0.901 | 0.956 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 39.82 |  | 30.00 | 28.92 | 30.51 | 31.12 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 4.476 |  | 0.010 | 0.010 | 0.010 | 0.015 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3.720 |  | 0.004 | 0.009 | 0.005 | 0.007 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3.011 |  | 0.006 | 0.007 | 0.007 | 0.009 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 3.803 |  | 0.205 | 0.183 | 0.233 | 0.221 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 3.682 |  | 0.445 | 0.515 | 0.590 | 0.564 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 4.112 |  | 0.078 | 0.079 | 0.083 | 0.092 | yes |
| `grep -rn qdbzzabsentterm .` | 169.9 | 84.14 | 0.014 | 0.013 | 0.021 | 0.018 | yes |
| `grep -rl qdbzzabsentterm .` | 108.9 | 72.13 | 0.015 | 0.021 | 0.024 | 0.019 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 137.6 | 74.61 | 0.051 | 0.026 | 0.027 | 0.047 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 96.30 | 74.37 | 0.038 | 0.020 | 0.027 | 0.037 | yes |
| `grep -rn btvarstrequalimage .` | 158.9 | 78.61 | 1.605 | 0.104 | 0.183 | 0.114 | yes |
| `grep -rl btvarstrequalimage .` | 102.5 | 73.97 | 1.672 | 0.089 | 0.085 | 0.093 | yes |
| `grep -rn undesirable .` | 187.2 | 90.06 | 2.512 | 0.797 | 1.028 | 0.900 | yes |
| `grep -rl undesirable .` | 137.5 | 79.81 | 1.837 | 0.503 | 0.535 | 0.763 | yes |
| `grep -rn passed .` | 200.0 | 85.23 | 21.08 | 13.36 | 15.87 | 12.91 | yes |
| `grep -rl passed .` | 125.0 | 64.92 | 10.66 | 6.376 | 2.978 | 2.805 | yes |
| `grep -rn not .` | 311.0 | 175.9 | 35.22 | 38.07 | 36.87 | 41.52 | yes |
| `grep -rl not .` | 78.51 | 66.81 | 3.506 | 4.208 | 2.882 | 2.658 | yes |
| `grep -rc undesirable .` | 143.6 | 69.06 | 3.520 | 1.188 | 1.431 | 0.881 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 174.2 | 96.27 | 1.457 | 0.123 | 0.106 | 0.132 | yes |
| `grep -ri undesirable .` | 146.0 | 98.47 | 3.253 | 0.677 | 0.632 | 0.875 | yes |
| `grep -rn undesirable src/backend` | 71.65 | 30.82 | 1.529 | 0.388 | 0.418 | 0.565 | yes |
| `grep -rl passed src/backend` | 34.81 | 19.14 | 1.544 | 1.404 | 1.218 | 1.192 | yes |
| `grep -rF 'undesirable' .` | 145.0 | 71.90 | 1.579 | 0.586 | 0.744 | 0.706 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 110.8 | 82.19 | 8.872 | 3.157 | 6.092 | 3.183 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 216.5 | 84.71 | 1.877 | 0.231 | 0.263 | 0.254 | yes |
| `grep -rE 'passed\|undesirable' .` | 250.4 | 95.39 | 19.57 | 19.68 | 20.71 | 14.96 | yes |
| `grep -rE '[0-9]{4,}' .` | 322.6 | 416.8 | 176.6 | 174.3 | 185.0 | 198.5 | yes |
| `grep -rn '^#include' .` | 195.6 | 86.56 | 11.65 | 9.960 | 9.657 | 8.264 | yes |
| `grep -rE '^(import\|from) ' .` | 170.9 | 83.11 | 43.93 | 33.92 | 36.34 | 37.31 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 103.7 | 47.10 | 16.54 | 15.11 | 16.58 | 15.36 | yes |
| `grep -rv e src/include/access` | 5.201 | 6.393 | 0.482 | 0.584 | 0.757 | 0.476 | yes |
| `grep -r '\(undesirable\).*\1' .` | 148.2 |  | 2.322 | 1.288 | 1.364 | 0.858 | yes |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 19.08 |  | | 0.630 | 0.675 | 0.679 | yes |
| `grep -l undesirable doc/src/sgml/html/*.html` | 20.21 |  | | 0.694 | 1.091 | 1.236 | yes |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 17.60 |  | | 0.454 | 0.607 | 0.549 | yes |
| `grep -rl passed . \| wc -l` | 113.0 |  | 5.880 | 8.441 | 7.873 | 4.563 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 239.9 |  | 32.83 | 27.42 | 30.56 | 29.11 | yes |
| `find . -name '*.html' \| wc -l` | 10.97 |  | 1.615 | 1.200 | 1.048 | 1.142 | yes |
| `find . -type f \| grep -c html` | 11.71 |  | 1.328 | 1.334 | 1.278 | 0.993 | yes |
| `grep -rn not . \| head -n 5` | 4.354 |  | 0.008 | 0.010 | 0.008 | 0.010 | order differs |
| `grep -rn passed . \| head -n 1` | 21.75 |  | 0.012 | 0.012 | 0.022 | 0.022 | order differs |
| `find . -type f \| head -n 3` | 4.482 |  | 0.046 | 0.038 | 0.039 | 0.055 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 5.694 |  | 0.318 | 0.282 | 0.500 | 0.573 | yes |

## Peak memory per command

| Command | Output | Stage 2, in memory | Stage 3, in memory | On disk, settled | On disk, after the writes |
|---|---:|---:|---:|---:|---:|
| `ls` | 144 B | 961 B | 961 B | 961 B | 961 B |
| `ls -a` | 307 B | 1785 B | 1785 B | 1785 B | 1785 B |
| `ls -R` | 151.8 KB | 73.0 KB | 73.0 KB | 73.0 KB | 73.0 KB |
| `ls -aR src/backend` | 23.1 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.2 KB |
| `ls -p src/backend` | 279 B | 2075 B | 2075 B | 2075 B | 2075 B |
| `find .` | 347.5 KB | 28.5 KB | 28.5 KB | 28.5 KB | 28.5 KB |
| `find . -name '*.html'` | 49.6 KB | 18.3 KB | 18.3 KB | 18.3 KB | 18.3 KB |
| `find . -name '*.c'` | 54.0 KB | 33.0 KB | 33.0 KB | 33.0 KB | 33.0 KB |
| `find . -type d` | 16.8 KB | 17.5 KB | 17.5 KB | 17.5 KB | 17.5 KB |
| `find . -type f` | 330.7 KB | 26.7 KB | 26.7 KB | 26.7 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 992 B | 2820 B | 2820 B | 2820 B | 2820 B |
| `find src/backend -name '*.html' -type f` | 0 B | 1952 B | 1952 B | 1952 B | 1952 B |
| `find . -name 'Makefile'` | 9792 B | 11.6 KB | 11.6 KB | 11.6 KB | 11.6 KB |
| `cat doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 1068 B | 1068 B | 1068 B | 1068 B |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 21.7 KB | 33.0 KB | 33.0 KB | 33.0 KB | 33.0 KB |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 3143 B | 4075 B | 4075 B | 4075 B | 4075 B |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2038 B | 2970 B | 2970 B | 2970 B | 2970 B |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 11.5 KB | 12.5 KB | 12.5 KB | 12.5 KB | 12.5 KB |
| `wc doc/src/sgml/html/bug-reporting.html` | 55 B | 1076 B | 1076 B | 1076 B | 1076 B |
| `wc -l doc/src/sgml/html/*.html` | 57.8 KB | 268.9 KB | 268.9 KB | 268.9 KB | 268.9 KB |
| `cat doc/src/sgml/html/*.html \| wc -l` | 7 B | 269.1 KB | 269.1 KB | 269.1 KB | 269.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 565 B | 10.3 KB | 10.5 KB | 10.5 KB | 10.5 KB |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | 5856 B | 5856 B | 5856 B | 5856 B |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3886 B | 6933 B | 6933 B | 6933 B | 6933 B |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19.9 KB | 35.9 KB | 35.9 KB | 35.9 KB | 35.9 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 37.1 KB | 37.1 KB | 37.1 KB | 37.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6 B | 36.7 KB | 36.9 KB | 36.9 KB | 36.9 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 20.3 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rn abcdefghijklabcdefgh .` | 170 B | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB |
| `grep -rl abcdefghijklabcdefgh .` | 41 B | 26.8 KB | 26.8 KB | 26.8 KB | 26.8 KB |
| `grep -rn btvarstrequalimage .` | 1660 B | 24.6 KB | 24.6 KB | 24.6 KB | 24.6 KB |
| `grep -rl btvarstrequalimage .` | 316 B | 24.6 KB | 24.6 KB | 24.6 KB | 24.6 KB |
| `grep -rn undesirable .` | 11.2 KB | 29.2 KB | 29.1 KB | 29.1 KB | 29.1 KB |
| `grep -rl undesirable .` | 2982 B | 29.2 KB | 29.1 KB | 29.1 KB | 29.1 KB |
| `grep -rn passed .` | 225.8 KB | 15.3 KB | 25.9 KB | 25.9 KB | 26.9 KB |
| `grep -rl passed .` | 30.4 KB | 9696 B | 25.6 KB | 25.6 KB | 26.5 KB |
| `grep -rn not .` | 13.4 MB | 31.1 KB | 31.1 KB | 31.1 KB | 31.1 KB |
| `grep -rl not .` | 189.7 KB | 5758 B | 5758 B | 5758 B | 5758 B |
| `grep -rc undesirable .` | 347.5 KB | 44.4 KB | 44.9 KB | 44.9 KB | 44.9 KB |
| `grep -rin BTVARSTREQUALIMAGE .` | 1730 B | 24.6 KB | 24.6 KB | 24.6 KB | 24.6 KB |
| `grep -ri undesirable .` | 10.8 KB | 29.2 KB | 29.0 KB | 29.0 KB | 29.1 KB |
| `grep -rn undesirable src/backend` | 3398 B | 16.0 KB | 16.0 KB | 16.0 KB | 16.0 KB |
| `grep -rl passed src/backend` | 10.7 KB | 9717 B | 25.7 KB | 25.7 KB | 26.6 KB |
| `grep -rF 'undesirable' .` | 10.8 KB | 29.2 KB | 29.1 KB | 29.1 KB | 29.1 KB |
| `grep -rE 'size_[a-z]+\(' .` | 20.4 KB | 117.2 KB | 124.6 KB | 124.5 KB | 125.1 KB |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1754 B | 49.6 KB | 49.6 KB | 49.6 KB | 49.6 KB |
| `grep -rE 'passed\|undesirable' .` | 228.0 KB | 24.0 KB | 27.2 KB | 27.2 KB | 28.1 KB |
| `grep -rE '[0-9]{4,}' .` | 39.4 MB | 23.1 KB | 23.1 KB | 23.1 KB | 23.1 KB |
| `grep -rn '^#include' .` | 1219.8 KB | 33.9 KB | 54.0 KB | 55.2 KB | 55.2 KB |
| `grep -rE '^(import\|from) ' .` | 61.8 KB | 23.9 KB | 23.9 KB | 23.9 KB | 23.9 KB |
| `grep -rin 'return [a-z_]*;$' src/backend` | 902.7 KB | 50.4 KB | 63.7 KB | 66.9 KB | 66.9 KB |
| `grep -rv e src/include/access` | 232.5 KB | 27.5 KB | 27.5 KB | 27.5 KB | 27.5 KB |
| `grep -r '\(undesirable\).*\1' .` | 0 B | 33.0 KB | 32.9 KB | 32.9 KB | 32.9 KB |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 125 B | | 269.2 KB | 269.2 KB | 269.2 KB |
| `grep -l undesirable doc/src/sgml/html/*.html` | 976 B | | 269.2 KB | 269.2 KB | 269.2 KB |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 49.6 KB | | 269.2 KB | 269.2 KB | 269.2 KB |
| `grep -rl passed . \| wc -l` | 4 B | 10.5 KB | 26.5 KB | 26.5 KB | 27.4 KB |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 5 B | 10.9 KB | 11.1 KB | 11.1 KB | 11.1 KB |
| `find . -name '*.html' \| wc -l` | 5 B | 19.0 KB | 19.1 KB | 19.1 KB | 19.1 KB |
| `find . -type f \| grep -c html` | 5 B | 34.3 KB | 34.5 KB | 34.5 KB | 34.5 KB |
| `grep -rn not . \| head -n 5` | 386 B | 6624 B | 6736 B | 6736 B | 6736 B |
| `grep -rn passed . \| head -n 1` | 88 B | 10.3 KB | 10.4 KB | 10.4 KB | 10.4 KB |
| `find . -type f \| head -n 3` | 61 B | 27.5 KB | 27.6 KB | 27.6 KB | 27.6 KB |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 1555 B | 269.2 KB | 269.2 KB | 269.2 KB | 269.2 KB |

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
