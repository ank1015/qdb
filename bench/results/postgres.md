# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 3 part 1), and include starting bash and the tool (1.569 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 1 | none | 109.3 | 109.3 | 163.5 MB | 258.5 MB |  | yes |
| Part 2 | none | 131.2 | 131.2 | 163.5 MB | 167.1 MB |  | yes |
| Part 3, no index | none | 183.1 | 183.1 | 163.5 MB | 167.3 MB | 0.001 | yes |
| Part 4, index off | off | 214.4 | 214.4 | 163.7 MB | 167.5 MB | 0.001 | yes |
| Part 4, index on | on | 178.8 | 978.7 | 187.2 MB | 192.5 MB | 0.272 | yes |
| Stage 3 part 1 | on | 2875.6 | 3675.4 | 187.2 MB | 192.5 MB | 0.272 | yes |

## The index (Part 4, index on)

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

## The index (Stage 3 part 1)

- Built in 799.8 ms while loading.
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
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | documents +920 -920; postings +1840631; list bytes rewritten 0; rebuilds 0 |
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
| `rm -r bench_out/dir_moved` | documents +0 -1346; postings +0; list bytes rewritten 2564442; rebuilds 0 |
| `rm -rf bench_out` | documents +0 -1175; postings +0; list bytes rewritten 5912714; rebuilds 0 |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Part 4, index off | Part 4, index on | Stage 3 part 1 | Agree |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `ls` | 2.563 |  | 0.003 | 0.001 | 0.001 | 0.001 | 0.001 | 0.001 | yes |
| `ls -a` | 2.279 |  | 0.003 | 0.004 | 0.002 | 0.002 | 0.002 | 0.002 | yes |
| `ls -R` | 11.29 |  | 0.914 | 0.813 | 1.043 | 0.547 | 0.563 | 0.702 | yes |
| `ls -aR src/backend` | 3.957 |  | 0.102 | 0.093 | 0.097 | 0.062 | 0.059 | 0.062 | yes |
| `ls -p src/backend` | 2.746 |  | 0.003 | 0.003 | 0.002 | 0.002 | 0.002 | 0.003 | yes |
| `find .` | 10.27 |  | 2.207 | 1.367 | 1.657 | 1.010 | 1.298 | 1.354 | yes |
| `find . -name '*.html'` | 10.28 |  | 1.447 | 1.570 | 2.109 | 1.582 | 1.444 | 1.570 | yes |
| `find . -name '*.c'` | 13.40 |  | 2.411 | 1.344 | 2.195 | 1.616 | 1.706 | 2.007 | yes |
| `find . -type d` | 8.479 |  | 1.917 | 1.323 | 1.903 | 1.728 | 1.403 | 1.252 | yes |
| `find . -type f` | 10.25 |  | 1.601 | 1.394 | 1.475 | 1.251 | 1.397 | 1.229 | yes |
| `find . -maxdepth 2 -type f` | 2.748 |  | 0.013 | 0.013 | 0.010 | 0.009 | 0.015 | 0.009 | yes |
| `find src/backend -name '*.html' -type f` | 3.512 |  | 0.152 | 0.147 | 0.129 | 0.129 | 0.177 | 0.262 | yes |
| `find . -name 'Makefile'` | 11.11 |  | 1.271 | 2.822 | 1.136 | 1.289 | 2.052 | 2.292 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 2.559 |  | 0.003 | 0.002 | 0.001 | 0.001 | 0.001 | 0.003 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 2.288 |  | 0.027 | 0.030 | 0.025 | 0.028 | 0.028 | 0.059 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 2.365 |  | 0.014 | 0.004 | 0.003 | 0.003 | 0.003 | 0.003 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2.342 |  | 0.021 | 0.004 | 0.003 | 0.002 | 0.003 | 0.002 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 2.375 |  | 0.021 | 0.006 | 0.009 | 0.006 | 0.006 | 0.006 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 2.879 |  | 0.035 | 0.034 | 0.029 | 0.025 | 0.023 | 0.023 | yes |
| `wc -l doc/src/sgml/html/*.html` | 19.82 |  | 30.97 | 39.35 | 23.41 | 27.80 | 27.94 | 25.82 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 34.33 |  | 35.75 | 25.09 | 29.57 | 31.04 | 30.00 | 27.92 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 3.172 |  | 0.029 | 0.014 | 0.010 | 0.016 | 0.010 | 0.010 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 2.207 |  | 0.230 | 0.193 | 0.005 | 0.007 | 0.004 | 0.005 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 2.222 |  | 0.225 | 0.226 | 0.007 | 0.011 | 0.006 | 0.007 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 2.347 |  | 0.299 | 0.266 | 0.234 | 0.413 | 0.205 | 0.234 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 2.421 |  | 0.491 | 0.656 | 0.485 | 0.929 | 0.445 | 0.422 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 3.187 |  | 0.086 | 0.109 | 0.077 | 0.079 | 0.078 | 0.073 | yes |
| `grep -rn qdbzzabsentterm .` | 159.1 | 79.09 | 2179.8 | 2014.7 | 24.53 | 23.80 | 0.014 | 0.022 | yes |
| `grep -rl qdbzzabsentterm .` | 112.1 | 76.27 | 1991.3 | 2239.2 | 23.84 | 23.17 | 0.015 | 0.020 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 153.3 | 77.27 | 2113.7 | 2376.2 | 23.63 | 21.30 | 0.051 | 0.073 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 94.58 | 69.80 | 2229.1 | 2274.5 | 21.98 | 24.34 | 0.038 | 0.048 | yes |
| `grep -rn btvarstrequalimage .` | 155.3 | 75.71 | 2248.9 | 2281.9 | 23.54 | 22.78 | 1.605 | 1.662 | yes |
| `grep -rl btvarstrequalimage .` | 105.6 | 66.95 | 2222.5 | 2018.0 | 22.82 | 21.89 | 1.672 | 1.133 | yes |
| `grep -rn undesirable .` | 184.2 | 72.28 | 2014.0 | 2167.2 | 24.38 | 22.74 | 2.512 | 1.829 | yes |
| `grep -rl undesirable .` | 128.8 | 84.84 | 2284.4 | 2056.6 | 23.35 | 21.96 | 1.837 | 1.825 | yes |
| `grep -rn passed .` | 195.9 | 89.38 | 2138.0 | 2294.0 | 26.53 | 27.48 | 21.08 | 15.95 | yes |
| `grep -rl passed .` | 120.1 | 72.49 | 1557.0 | 1569.2 | 17.29 | 17.25 | 10.66 | 8.428 | yes |
| `grep -rn not .` | 365.3 | 146.1 | 2318.7 | 2155.5 | 40.43 | 40.89 | 35.22 | 40.68 | yes |
| `grep -rl not .` | 84.77 | 76.89 | 314.7 | 381.4 | 4.460 | 3.100 | 3.506 | 4.766 | yes |
| `grep -rc undesirable .` | 139.9 | 74.27 | 2066.5 | 2304.3 | 24.24 | 22.44 | 3.520 | 3.865 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 160.7 | 80.93 | 2297.0 | 2111.8 | 31.59 | 25.56 | 1.457 | 1.141 | yes |
| `grep -ri undesirable .` | 156.1 | 91.07 | 2281.0 | 2218.5 | 26.83 | 29.04 | 3.253 | 2.891 | yes |
| `grep -rn undesirable src/backend` | 71.43 | 30.60 | 824.4 | 790.0 | 6.626 | 9.305 | 1.529 | 1.537 | yes |
| `grep -rl passed src/backend` | 32.68 | 17.84 | 388.8 | 496.3 | 2.703 | 2.000 | 1.544 | 1.550 | yes |
| `grep -rF 'undesirable' .` | 143.8 | 63.07 | 245.0 | 238.6 | 23.62 | 26.15 | 1.579 | 1.509 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 110.6 | 70.21 | 2152.1 | 2400.4 | 23.40 | 23.56 | 8.872 | 8.740 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 210.9 | 77.23 | 3054.1 | 3024.7 | 32.38 | 31.18 | 1.877 | 2.732 | yes |
| `grep -rE 'passed\|undesirable' .` | 249.7 | 89.31 | 2612.7 | 3216.1 | 32.24 | 31.31 | 19.57 | 19.63 | yes |
| `grep -rE '[0-9]{4,}' .` | 292.4 | 364.4 | 2192.2 | 2054.3 | 151.6 | 186.8 | 176.6 | 202.3 | yes |
| `grep -rn '^#include' .` | 179.4 | 112.8 | 1673.7 | 1990.0 | 25.30 | 27.63 | 11.65 | 11.38 | yes |
| `grep -rE '^(import\|from) ' .` | 163.8 | 90.33 | 2040.8 | 2116.2 | 37.76 | 38.14 | 43.93 | 36.40 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 95.81 | 39.89 | 956.9 | 843.6 | 17.26 | 18.77 | 16.54 | 16.45 | yes |
| `grep -rv e src/include/access` | 4.971 | 6.996 | 7.669 | 3.327 | 0.490 | 0.439 | 0.482 | 0.464 | yes |
| `grep -r '\(undesirable\).*\1' .` | 144.0 |  | 3929.1 | 3988.6 | 23.34 | 26.49 | 2.322 | 2.472 | yes |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 19.11 |  | | | | | | 0.691 | yes |
| `grep -l undesirable doc/src/sgml/html/*.html` | 20.80 |  | | | | | | 1.224 | yes |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 16.77 |  | | | | | | 0.608 | yes |
| `grep -rl passed . \| wc -l` | 113.8 |  | 1450.1 | 1465.5 | 18.28 | 20.56 | 5.880 | 7.251 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 237.2 |  | 2196.3 | 2213.1 | 31.56 | 33.97 | 32.83 | 32.51 | yes |
| `find . -name '*.html' \| wc -l` | 10.40 |  | 1.709 | 1.848 | 1.851 | 1.779 | 1.615 | 1.327 | yes |
| `find . -type f \| grep -c html` | 11.44 |  | 4.903 | 6.074 | 1.905 | 1.615 | 1.328 | 3.127 | yes |
| `grep -rn not . \| head -n 5` | 4.253 |  | 2280.0 | 0.467 | 0.008 | 0.012 | 0.008 | 0.008 | order differs |
| `grep -rn passed . \| head -n 1` | 18.15 |  | 2205.4 | 1.723 | 0.012 | 0.021 | 0.012 | 0.012 | order differs |
| `find . -type f \| head -n 3` | 3.729 |  | 2.421 | 0.044 | 0.050 | 0.045 | 0.046 | 0.034 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 4.758 |  | 15.07 | 0.539 | 0.315 | 0.253 | 0.318 | 0.341 | yes |
| `mkdir -p bench_out/a/b` | 4.361 |  | 0.025 | 0.022 | 0.011 | 0.011 | 0.010 | 0.021 | yes |
| `touch bench_out/empty.txt` | 3.389 |  | 0.018 | 0.019 | 0.007 | 0.013 | 0.012 | 0.013 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 2.219 |  | 0.018 | 0.018 | 0.022 | 0.016 | 0.019 | 0.024 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 3.186 |  | 0.030 | 0.013 | 0.011 | 0.013 | 0.018 | 0.023 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 2.999 |  | 0.041 | 0.034 | 0.026 | 0.026 | 0.316 | 0.252 | yes |
| `cp -r src/backend bench_out/dir_copy` | 86.56 |  | 29.83 | 51.93 | 35.82 | 42.23 | 372.3 | 308.3 | yes |
| `cp -r doc/src/sgml/html bench_out/work` | 33.84 |  | 12.97 | 15.61 | 12.29 | 13.43 | 101.0 | 105.8 | yes |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 3.655 |  | 0.758 | 0.386 | 0.312 | 0.295 | 0.538 | 0.559 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 3.683 |  | 0.701 | 0.343 | 0.284 | 0.253 | 0.492 | 0.509 | yes |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 102.9 |  | 320.7 | 336.1 | 190.7 | 231.0 | 296.5 | 301.9 | yes |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | 197.5 |  | | | | | | 1.845 | yes |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | 242.5 |  | | | | | | 1.940 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 3.335 |  | | | | | | 0.030 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 3.230 |  | | | | | | 0.030 | yes |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | 3.310 |  | | | | | | 0.030 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 26.30 |  | 1111.1 | 1046.2 | 12.00 | 12.17 | 4.294 | 3.427 | yes |
| `grep -rl QDBBENCHMANY .` | 80.85 |  | 3145.5 | 3030.1 | 33.71 | 32.67 | 1.838 | 1.794 | yes |
| `grep -rl QDBBENCHONE .` | 115.5 |  | 3329.2 | 3342.9 | 35.44 | 35.46 | 0.384 | 0.546 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 3.195 |  | 0.035 | 0.041 | 0.031 | 0.021 | 0.024 | 0.024 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 8.089 |  | 0.031 | 0.039 | 0.019 | 0.027 | 0.022 | 0.030 | yes |
| `ls -R bench_out/a` | 2.573 |  | 0.022 | 0.022 | 0.014 | 0.014 | 0.012 | 0.025 | yes |
| `rm bench_out/a/b/hello.txt` | 2.629 |  | 0.023 | 0.027 | 0.012 | 0.013 | 0.008 | 0.015 | yes |
| `rmdir bench_out/a/b` | 3.019 |  | 0.019 | 0.028 | 0.010 | 0.011 | 0.010 | 0.010 | yes |
| `rm -r bench_out/dir_moved` | 21.37 |  | 1.733 | 1.902 | 0.920 | 0.748 | 12.58 | 10.94 | yes |
| `rm -rf bench_out` | 106.5 |  | 2.232 | 1.934 | 0.869 | 0.609 | 19.33 | 23.70 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 | Part 3, no index | Part 4, index off | Part 4, index on | Stage 3 part 1 |
|---|---:|---:|---:|---:|---:|---:|---:|
| `ls` | 144 B | 1121 B | 961 B | 961 B | 961 B | 961 B | 961 B |
| `ls -a` | 307 B | 1945 B | 1785 B | 1785 B | 1785 B | 1785 B | 1785 B |
| `ls -R` | 151.8 KB | 271.4 KB | 73.0 KB | 73.0 KB | 73.0 KB | 73.0 KB | 73.0 KB |
| `ls -aR src/backend` | 23.1 KB | 49.1 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.2 KB | 29.2 KB |
| `ls -p src/backend` | 279 B | 2235 B | 2075 B | 2075 B | 2075 B | 2075 B | 2075 B |
| `find .` | 347.5 KB | 443.4 KB | 28.5 KB | 28.5 KB | 28.5 KB | 28.5 KB | 28.5 KB |
| `find . -name '*.html'` | 49.6 KB | 69.1 KB | 18.3 KB | 18.3 KB | 18.3 KB | 18.3 KB | 18.3 KB |
| `find . -name '*.c'` | 54.0 KB | 65.0 KB | 33.0 KB | 33.0 KB | 33.0 KB | 33.0 KB | 33.0 KB |
| `find . -type d` | 16.8 KB | 34.0 KB | 17.5 KB | 17.5 KB | 17.5 KB | 17.5 KB | 17.5 KB |
| `find . -type f` | 330.7 KB | 410.9 KB | 26.7 KB | 26.7 KB | 26.7 KB | 26.7 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 992 B | 2980 B | 2820 B | 2820 B | 2820 B | 2820 B | 2820 B |
| `find src/backend -name '*.html' -type f` | 0 B | 1876 B | 1952 B | 1952 B | 1952 B | 1952 B | 1952 B |
| `find . -name 'Makefile'` | 9792 B | 11.8 KB | 11.6 KB | 11.6 KB | 11.6 KB | 11.6 KB | 11.6 KB |
| `cat doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 20.8 KB | 1068 B | 1068 B | 1068 B | 1068 B | 1068 B |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 21.7 KB | 33.2 KB | 33.0 KB | 33.0 KB | 33.0 KB | 33.0 KB | 33.0 KB |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 3143 B | 4235 B | 4075 B | 4075 B | 4075 B | 4075 B | 4075 B |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2038 B | 3130 B | 2970 B | 2970 B | 2970 B | 2970 B | 2970 B |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 11.5 KB | 12.6 KB | 12.5 KB | 12.5 KB | 12.5 KB | 12.5 KB | 12.5 KB |
| `wc doc/src/sgml/html/bug-reporting.html` | 55 B | 992 B | 1076 B | 1076 B | 1076 B | 1076 B | 1076 B |
| `wc -l doc/src/sgml/html/*.html` | 57.8 KB | 268.9 KB | 268.9 KB | 268.9 KB | 268.9 KB | 268.9 KB | 268.9 KB |
| `cat doc/src/sgml/html/*.html \| wc -l` | 7 B | 22.2 MB | 269.1 KB | 269.1 KB | 269.1 KB | 269.1 KB | 269.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 565 B | 27.1 KB | 10.3 KB | 10.3 KB | 10.3 KB | 10.3 KB | 10.3 KB |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | 3631 B | 3415 B | 5856 B | 5856 B | 5856 B | 5856 B |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3886 B | 9263 B | 9047 B | 6933 B | 6933 B | 6933 B | 6933 B |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19.9 KB | 35.9 KB | 35.7 KB | 35.9 KB | 35.9 KB | 35.9 KB | 35.9 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 37.2 KB | 37.1 KB | 37.1 KB | 37.1 KB | 37.1 KB | 37.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6 B | 55.5 KB | 36.4 KB | 36.7 KB | 36.7 KB | 36.7 KB | 36.7 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB | 20.3 KB | 20.3 KB | 20.3 KB |
| `grep -rn abcdefghijklabcdefgh .` | 170 B | 15.3 KB | 15.1 KB | 26.1 KB | 26.8 KB | 26.8 KB | 26.8 KB |
| `grep -rl abcdefghijklabcdefgh .` | 41 B | 15.1 KB | 14.9 KB | 26.1 KB | 26.8 KB | 26.8 KB | 26.8 KB |
| `grep -rn btvarstrequalimage .` | 1660 B | 17.1 KB | 14.4 KB | 24.0 KB | 24.6 KB | 24.6 KB | 24.6 KB |
| `grep -rl btvarstrequalimage .` | 316 B | 14.4 KB | 13.8 KB | 24.0 KB | 24.6 KB | 24.6 KB | 24.6 KB |
| `grep -rn undesirable .` | 11.2 KB | 22.7 KB | 9295 B | 15.3 KB | 16.0 KB | 29.2 KB | 29.2 KB |
| `grep -rl undesirable .` | 2982 B | 12.3 KB | 8971 B | 15.3 KB | 16.0 KB | 29.2 KB | 29.2 KB |
| `grep -rn passed .` | 225.8 KB | 362.1 KB | 11.2 KB | 9290 B | 9418 B | 15.3 KB | 15.3 KB |
| `grep -rl passed .` | 30.4 KB | 42.7 KB | 5658 B | 9290 B | 9418 B | 9696 B | 9696 B |
| `grep -rn not .` | 13.4 MB | 26.2 MB | 29.3 KB | 31.1 KB | 31.1 KB | 31.1 KB | 31.1 KB |
| `grep -rl not .` | 189.7 KB | 208.7 KB | 3771 B | 5758 B | 5758 B | 5758 B | 5758 B |
| `grep -rc undesirable .` | 347.5 KB | 418.6 KB | 34.5 KB | 30.1 KB | 30.6 KB | 44.4 KB | 44.4 KB |
| `grep -rin BTVARSTREQUALIMAGE .` | 1730 B | 17.1 KB | 14.4 KB | 24.1 KB | 24.6 KB | 24.6 KB | 24.6 KB |
| `grep -ri undesirable .` | 10.8 KB | 22.1 KB | 9279 B | 15.3 KB | 16.0 KB | 29.2 KB | 29.2 KB |
| `grep -rn undesirable src/backend` | 3398 B | 13.7 KB | 9144 B | 15.3 KB | 16.0 KB | 16.0 KB | 16.0 KB |
| `grep -rl passed src/backend` | 10.7 KB | 21.1 KB | 5631 B | 9315 B | 9443 B | 9717 B | 9717 B |
| `grep -rF 'undesirable' .` | 10.8 KB | 14.5 KB | 1661 B | 1830 B | 2342 B | 29.2 KB | 29.2 KB |
| `grep -rE 'size_[a-z]+\(' .` | 20.4 KB | 44.7 KB | 9118 B | 15.1 KB | 15.1 KB | 117.2 KB | 117.2 KB |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1754 B | 30.4 KB | 27.8 KB | 48.5 KB | 49.6 KB | 49.6 KB | 49.6 KB |
| `grep -rE 'passed\|undesirable' .` | 228.0 KB | 349.3 KB | 18.4 KB | 23.2 KB | 24.0 KB | 24.0 KB | 24.0 KB |
| `grep -rE '[0-9]{4,}' .` | 39.4 MB | 52.4 MB | 29.5 KB | 23.1 KB | 23.1 KB | 23.1 KB | 23.1 KB |
| `grep -rn '^#include' .` | 1219.8 KB | 1383.7 KB | 17.9 KB | 17.3 KB | 17.5 KB | 33.9 KB | 33.9 KB |
| `grep -rE '^(import\|from) ' .` | 61.8 KB | 83.0 KB | 18.3 KB | 23.3 KB | 23.9 KB | 23.9 KB | 23.9 KB |
| `grep -rin 'return [a-z_]*;$' src/backend` | 902.7 KB | 991.2 KB | 38.6 KB | 33.8 KB | 34.0 KB | 50.4 KB | 50.4 KB |
| `grep -rv e src/include/access` | 232.5 KB | 428.4 KB | 28.8 KB | 27.5 KB | 27.5 KB | 27.5 KB | 27.5 KB |
| `grep -r '\(undesirable\).*\1' .` | 0 B | 5754 B | 5538 B | 9822 B | 10.3 KB | 33.0 KB | 33.0 KB |
| `grep -n btvarstrequalimage doc/src/sgml/html/*.html` | 125 B | | | | | | 269.2 KB |
| `grep -l undesirable doc/src/sgml/html/*.html` | 976 B | | | | | | 269.2 KB |
| `grep -c qdbzzabsentterm doc/src/sgml/html/*.html` | 49.6 KB | | | | | | 269.2 KB |
| `grep -rl passed . \| wc -l` | 4 B | 43.2 KB | 6419 B | 10.1 KB | 10.2 KB | 10.5 KB | 10.5 KB |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 5 B | 634.8 KB | 10.3 KB | 10.9 KB | 10.9 KB | 10.9 KB | 10.9 KB |
| `find . -name '*.html' \| wc -l` | 5 B | 69.7 KB | 19.0 KB | 19.0 KB | 19.0 KB | 19.0 KB | 19.0 KB |
| `find . -type f \| grep -c html` | 5 B | 414.3 KB | 30.7 KB | 34.2 KB | 34.3 KB | 34.3 KB | 34.3 KB |
| `grep -rn not . \| head -n 5` | 386 B | 26.2 MB | 5922 B | 6624 B | 6624 B | 6624 B | 6624 B |
| `grep -rn passed . \| head -n 1` | 88 B | 362.8 KB | 6289 B | 10.2 KB | 10.3 KB | 10.3 KB | 10.3 KB |
| `find . -type f \| head -n 3` | 61 B | 411.6 KB | 27.5 KB | 27.5 KB | 27.5 KB | 27.5 KB | 27.5 KB |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 1555 B | 22.2 MB | 269.2 KB | 269.2 KB | 269.2 KB | 269.2 KB | 269.2 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1987 B | 1827 B | 1827 B | 1827 B | 1827 B | 1827 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B | 979 B | 979 B | 979 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B | 1242 B | 1242 B | 1242 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B | 1246 B | 1246 B | 1246 B | 1246 B |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0 B | 21.1 KB | 20.9 KB | 20.9 KB | 20.9 KB | 24.1 KB | 24.1 KB |
| `cp -r src/backend bench_out/dir_copy` | 0 B | 65.1 MB | 65.1 MB | 65.1 MB | 65.1 MB | 68.8 MB | 68.8 MB |
| `cp -r doc/src/sgml/html bench_out/work` | 0 B | 16.8 MB | 16.8 MB | 16.8 MB | 16.8 MB | 19.2 MB | 19.2 MB |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 36.0 KB | 35.9 KB | 36.0 KB | 36.0 KB | 39.7 KB | 40.0 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 35.3 KB | 35.1 KB | 35.2 KB | 35.2 KB | 38.3 KB | 41.0 KB |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 0 B | 7758.3 KB | 7758.2 KB | 7758.3 KB | 7758.3 KB | 10.3 MB | 9358.9 KB |
| `sed -i 's/qdbzzabsentterm/QDBBENCHNONE/g' bench_out/work/*.html` | 0 B | | | | | | 263.0 KB |
| `sed -i 's/btvarstrequalimage/QDBBENCHFEW/g' bench_out/work/*.html` | 0 B | | | | | | 265.5 KB |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 B | | | | | | 20.9 KB |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/same.txt` | 0 B | | | | | | 20.9 KB |
| `cat doc/src/sgml/html/bug-reporting.html > bench_out/same.txt` | 0 B | | | | | | 21.2 KB |
| `grep -rc QDBBENCHMANY bench_out` | 110.1 KB | 156.9 KB | 27.9 KB | 28.7 KB | 29.2 KB | 33.3 KB | 33.3 KB |
| `grep -rl QDBBENCHMANY .` | 36.1 KB | 64.9 KB | 9532 B | 16.4 KB | 17.1 KB | 97.6 KB | 97.6 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 9209 B | 8993 B | 15.3 KB | 16.0 KB | 16.0 KB | 16.0 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B | 1191 B | 1191 B | 1191 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B | 1288 B | 1288 B | 1288 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B | 1089 B | 1089 B | 1089 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B | 997 B | 997 B | 997 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B | 945 B | 945 B | 945 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1179 B | 1129 B | 1129 B | 1129 B | 1129 B | 1129 B |
| `rm -rf bench_out` | 0 B | 9514 B | 9354 B | 9354 B | 9354 B | 9354 B | 9354 B |

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
