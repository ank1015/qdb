# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Part 2), and include starting bash and the tool (7.543 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 109.3 | 109.3 | 163.5 MB | 258.5 MB | yes |
| Part 2 | on | 131.2 | 131.2 | 163.5 MB | 167.1 MB | yes |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Agree |
|---|---:|---:|---:|---:|---|
| `ls` | 6.763 |  | 0.003 | 0.001 | yes |
| `ls -a` | 5.295 |  | 0.003 | 0.004 | yes |
| `ls -R` | 13.95 |  | 0.914 | 0.813 | yes |
| `ls -aR src/backend` | 6.535 |  | 0.102 | 0.093 | yes |
| `ls -p src/backend` | 5.461 |  | 0.003 | 0.003 | yes |
| `find .` | 12.01 |  | 2.207 | 1.367 | yes |
| `find . -name '*.html'` | 13.41 |  | 1.447 | 1.570 | yes |
| `find . -name '*.c'` | 13.72 |  | 2.411 | 1.344 | yes |
| `find . -type d` | 12.23 |  | 1.917 | 1.323 | yes |
| `find . -type f` | 12.25 |  | 1.601 | 1.394 | yes |
| `find . -maxdepth 2 -type f` | 6.456 |  | 0.013 | 0.013 | yes |
| `find src/backend -name '*.html' -type f` | 7.136 |  | 0.152 | 0.147 | yes |
| `find . -name 'Makefile'` | 12.92 |  | 1.271 | 2.822 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 6.489 |  | 0.003 | 0.002 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 5.233 |  | 0.027 | 0.030 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 5.124 |  | 0.014 | 0.004 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 5.688 |  | 0.021 | 0.004 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 4.925 |  | 0.021 | 0.006 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 5.945 |  | 0.035 | 0.034 | yes |
| `wc -l doc/src/sgml/html/*.html` | 20.03 |  | 30.97 | 39.35 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 38.16 |  | 35.75 | 25.09 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 7.979 |  | 0.029 | 0.014 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 5.916 |  | 0.230 | 0.193 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 5.595 |  | 0.225 | 0.226 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 6.009 |  | 0.299 | 0.266 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 5.852 |  | 0.491 | 0.656 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 7.397 |  | 0.086 | 0.109 | yes |
| `grep -rn qdbzzabsentterm .` | 154.7 | 77.38 | 2179.8 | 2014.7 | yes |
| `grep -rl qdbzzabsentterm .` | 110.7 | 70.95 | 1991.3 | 2239.2 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 142.5 | 74.00 | 2113.7 | 2376.2 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 94.93 | 71.18 | 2229.1 | 2274.5 | yes |
| `grep -rn btvarstrequalimage .` | 156.1 | 78.01 | 2248.9 | 2281.9 | yes |
| `grep -rl btvarstrequalimage .` | 116.6 | 72.86 | 2222.5 | 2018.0 | yes |
| `grep -rn undesirable .` | 162.7 | 78.24 | 2014.0 | 2167.2 | yes |
| `grep -rl undesirable .` | 116.1 | 65.29 | 2284.4 | 2056.6 | yes |
| `grep -rn passed .` | 185.9 | 86.04 | 2138.0 | 2294.0 | yes |
| `grep -rl passed .` | 112.6 | 71.92 | 1557.0 | 1569.2 | yes |
| `grep -rn not .` | 297.3 | 150.5 | 2318.7 | 2155.5 | yes |
| `grep -rl not .` | 69.92 | 69.86 | 314.7 | 381.4 | yes |
| `grep -rc undesirable .` | 146.4 | 69.49 | 2066.5 | 2304.3 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 166.4 | 97.70 | 2297.0 | 2111.8 | yes |
| `grep -ri undesirable .` | 141.6 | 86.96 | 2281.0 | 2218.5 | yes |
| `grep -rn undesirable src/backend` | 64.38 | 28.54 | 824.4 | 790.0 | yes |
| `grep -rl passed src/backend` | 34.71 | 24.09 | 388.8 | 496.3 | yes |
| `grep -rF 'undesirable' .` | 138.9 | 74.00 | 245.0 | 238.6 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 111.7 | 72.94 | 2152.1 | 2400.4 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 208.9 | 85.74 | 3054.1 | 3024.7 | yes |
| `grep -rE 'passed\|undesirable' .` | 277.5 | 89.34 | 2612.7 | 3216.1 | yes |
| `grep -rE '[0-9]{4,}' .` | 296.4 | 329.2 | 2192.2 | 2054.3 | yes |
| `grep -rn '^#include' .` | 185.1 | 91.72 | 1673.7 | 1990.0 | yes |
| `grep -rE '^(import\|from) ' .` | 173.7 | 89.24 | 2040.8 | 2116.2 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 104.0 | 51.67 | 956.9 | 843.6 | yes |
| `grep -rv e src/include/access` | 10.56 | 8.688 | 7.669 | 3.327 | yes |
| `grep -r '\(undesirable\).*\1' .` | 125.6 |  | 3929.1 | 3988.6 | yes |
| `grep -rl passed . \| wc -l` | 120.2 |  | 1450.1 | 1465.5 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 230.5 |  | 2196.3 | 2213.1 | yes |
| `find . -name '*.html' \| wc -l` | 15.41 |  | 1.709 | 1.848 | yes |
| `find . -type f \| grep -c html` | 16.42 |  | 4.903 | 6.074 | yes |
| `grep -rn not . \| head -n 5` | 7.370 |  | 2280.0 | 0.467 | order differs |
| `grep -rn passed . \| head -n 1` | 25.56 |  | 2205.4 | 1.723 | order differs |
| `find . -type f \| head -n 3` | 7.177 |  | 2.421 | 0.044 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 7.621 |  | 15.07 | 0.539 | yes |
| `mkdir -p bench_out/a/b` | 19.45 |  | 0.025 | 0.022 | yes |
| `touch bench_out/empty.txt` | 6.440 |  | 0.018 | 0.019 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 10.02 |  | 0.018 | 0.018 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 4.863 |  | 0.030 | 0.013 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 5.285 |  | 0.041 | 0.034 | yes |
| `cp -r src/backend bench_out/dir_copy` | 46.89 |  | 29.83 | 51.93 | yes |
| `cp -r doc/src/sgml/html bench_out/work` | 31.38 |  | 12.97 | 15.61 | yes |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 10.26 |  | 0.758 | 0.386 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 9.246 |  | 0.701 | 0.343 | yes |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 113.5 |  | 320.7 | 336.1 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 39.34 |  | 1111.1 | 1046.2 | yes |
| `grep -rl QDBBENCHMANY .` | 92.87 |  | 3145.5 | 3030.1 | yes |
| `grep -rl QDBBENCHONE .` | 111.1 |  | 3329.2 | 3342.9 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 11.76 |  | 0.035 | 0.041 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 10.94 |  | 0.031 | 0.039 | yes |
| `ls -R bench_out/a` | 9.181 |  | 0.022 | 0.022 | yes |
| `rm bench_out/a/b/hello.txt` | 7.512 |  | 0.023 | 0.027 | yes |
| `rmdir bench_out/a/b` | 9.109 |  | 0.019 | 0.028 | yes |
| `rm -r bench_out/dir_moved` | 37.70 |  | 1.733 | 1.902 | yes |
| `rm -rf bench_out` | 109.2 |  | 2.232 | 1.934 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 |
|---|---:|---:|---:|
| `ls` | 144 B | 1121 B | 961 B |
| `ls -a` | 307 B | 1945 B | 1785 B |
| `ls -R` | 151.8 KB | 271.4 KB | 73.0 KB |
| `ls -aR src/backend` | 23.1 KB | 49.1 KB | 29.2 KB |
| `ls -p src/backend` | 279 B | 2235 B | 2075 B |
| `find .` | 347.5 KB | 443.4 KB | 28.5 KB |
| `find . -name '*.html'` | 49.6 KB | 69.1 KB | 18.3 KB |
| `find . -name '*.c'` | 54.0 KB | 65.0 KB | 33.0 KB |
| `find . -type d` | 16.8 KB | 34.0 KB | 17.5 KB |
| `find . -type f` | 330.7 KB | 410.9 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 992 B | 2980 B | 2820 B |
| `find src/backend -name '*.html' -type f` | 0 B | 1876 B | 1952 B |
| `find . -name 'Makefile'` | 9792 B | 11.8 KB | 11.6 KB |
| `cat doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 20.8 KB | 1068 B |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 21.7 KB | 33.2 KB | 33.0 KB |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 3143 B | 4235 B | 4075 B |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2038 B | 3130 B | 2970 B |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 11.5 KB | 12.6 KB | 12.5 KB |
| `wc doc/src/sgml/html/bug-reporting.html` | 55 B | 992 B | 1076 B |
| `wc -l doc/src/sgml/html/*.html` | 57.8 KB | 268.9 KB | 268.9 KB |
| `cat doc/src/sgml/html/*.html \| wc -l` | 7 B | 22.2 MB | 269.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 565 B | 27.1 KB | 10.3 KB |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | 3631 B | 3415 B |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3886 B | 9263 B | 9047 B |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19.9 KB | 35.9 KB | 35.7 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 37.2 KB | 37.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6 B | 55.5 KB | 36.4 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB |
| `grep -rn abcdefghijklabcdefgh .` | 170 B | 15.3 KB | 15.1 KB |
| `grep -rl abcdefghijklabcdefgh .` | 41 B | 15.1 KB | 14.9 KB |
| `grep -rn btvarstrequalimage .` | 1660 B | 17.1 KB | 14.4 KB |
| `grep -rl btvarstrequalimage .` | 316 B | 14.4 KB | 13.8 KB |
| `grep -rn undesirable .` | 11.2 KB | 22.7 KB | 9295 B |
| `grep -rl undesirable .` | 2982 B | 12.3 KB | 8971 B |
| `grep -rn passed .` | 225.8 KB | 362.1 KB | 11.2 KB |
| `grep -rl passed .` | 30.4 KB | 42.7 KB | 5658 B |
| `grep -rn not .` | 13.4 MB | 26.2 MB | 29.3 KB |
| `grep -rl not .` | 189.7 KB | 208.7 KB | 3771 B |
| `grep -rc undesirable .` | 347.5 KB | 418.6 KB | 34.5 KB |
| `grep -rin BTVARSTREQUALIMAGE .` | 1730 B | 17.1 KB | 14.4 KB |
| `grep -ri undesirable .` | 10.8 KB | 22.1 KB | 9279 B |
| `grep -rn undesirable src/backend` | 3398 B | 13.7 KB | 9144 B |
| `grep -rl passed src/backend` | 10.7 KB | 21.1 KB | 5631 B |
| `grep -rF 'undesirable' .` | 10.8 KB | 14.5 KB | 1661 B |
| `grep -rE 'size_[a-z]+\(' .` | 20.4 KB | 44.7 KB | 9118 B |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1754 B | 30.4 KB | 27.8 KB |
| `grep -rE 'passed\|undesirable' .` | 228.0 KB | 349.3 KB | 18.4 KB |
| `grep -rE '[0-9]{4,}' .` | 39.4 MB | 52.4 MB | 29.5 KB |
| `grep -rn '^#include' .` | 1219.8 KB | 1383.7 KB | 17.9 KB |
| `grep -rE '^(import\|from) ' .` | 61.8 KB | 83.0 KB | 18.3 KB |
| `grep -rin 'return [a-z_]*;$' src/backend` | 902.7 KB | 991.2 KB | 38.6 KB |
| `grep -rv e src/include/access` | 232.5 KB | 428.4 KB | 28.8 KB |
| `grep -r '\(undesirable\).*\1' .` | 0 B | 5754 B | 5538 B |
| `grep -rl passed . \| wc -l` | 4 B | 43.2 KB | 6419 B |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 5 B | 634.8 KB | 10.3 KB |
| `find . -name '*.html' \| wc -l` | 5 B | 69.7 KB | 19.0 KB |
| `find . -type f \| grep -c html` | 5 B | 414.3 KB | 30.7 KB |
| `grep -rn not . \| head -n 5` | 386 B | 26.2 MB | 5922 B |
| `grep -rn passed . \| head -n 1` | 88 B | 362.8 KB | 6289 B |
| `find . -type f \| head -n 3` | 61 B | 411.6 KB | 27.5 KB |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 1555 B | 22.2 MB | 269.2 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1987 B | 1827 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0 B | 21.1 KB | 20.9 KB |
| `cp -r src/backend bench_out/dir_copy` | 0 B | 65.1 MB | 65.1 MB |
| `cp -r doc/src/sgml/html bench_out/work` | 0 B | 16.8 MB | 16.8 MB |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 36.0 KB | 35.9 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 35.3 KB | 35.1 KB |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 0 B | 7758.3 KB | 7758.2 KB |
| `grep -rc QDBBENCHMANY bench_out` | 110.1 KB | 156.9 KB | 27.9 KB |
| `grep -rl QDBBENCHMANY .` | 36.1 KB | 64.9 KB | 9532 B |
| `grep -rl QDBBENCHONE .` | 21 B | 9209 B | 8993 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1179 B | 1129 B |
| `rm -rf bench_out` | 0 B | 9514 B | 9354 B |

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
