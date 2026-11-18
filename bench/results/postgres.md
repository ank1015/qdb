# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Part 3, no index), and include starting bash and the tool (5.505 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 109.3 | 109.3 | 163.5 MB | 258.5 MB | yes |
| Part 2 | on | 131.2 | 131.2 | 163.5 MB | 167.1 MB | yes |
| Part 3, no index | off | 163.0 | 163.0 | 163.5 MB | 167.1 MB | yes |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Agree |
|---|---:|---:|---:|---:|---:|---|
| `ls` | 5.953 |  | 0.003 | 0.001 | 0.002 | yes |
| `ls -a` | 5.580 |  | 0.003 | 0.004 | 0.004 | yes |
| `ls -R` | 16.65 |  | 0.914 | 0.813 | 1.334 | yes |
| `ls -aR src/backend` | 7.848 |  | 0.102 | 0.093 | 0.183 | yes |
| `ls -p src/backend` | 5.601 |  | 0.003 | 0.003 | 0.004 | yes |
| `find .` | 16.06 |  | 2.207 | 1.367 | 1.686 | yes |
| `find . -name '*.html'` | 16.15 |  | 1.447 | 1.570 | 1.908 | yes |
| `find . -name '*.c'` | 15.83 |  | 2.411 | 1.344 | 1.737 | yes |
| `find . -type d` | 13.84 |  | 1.917 | 1.323 | 1.312 | yes |
| `find . -type f` | 14.09 |  | 1.601 | 1.394 | 1.556 | yes |
| `find . -maxdepth 2 -type f` | 6.506 |  | 0.013 | 0.013 | 0.011 | yes |
| `find src/backend -name '*.html' -type f` | 6.734 |  | 0.152 | 0.147 | 0.122 | yes |
| `find . -name 'Makefile'` | 13.81 |  | 1.271 | 2.822 | 1.560 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 6.343 |  | 0.003 | 0.002 | 0.002 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 5.374 |  | 0.027 | 0.030 | 0.027 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 5.471 |  | 0.014 | 0.004 | 0.006 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 5.609 |  | 0.021 | 0.004 | 0.004 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 5.353 |  | 0.021 | 0.006 | 0.007 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 5.545 |  | 0.035 | 0.034 | 0.039 | yes |
| `wc -l doc/src/sgml/html/*.html` | 19.63 |  | 30.97 | 39.35 | 33.82 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 40.84 |  | 35.75 | 25.09 | 32.67 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 7.292 |  | 0.029 | 0.014 | 0.015 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 5.929 |  | 0.230 | 0.193 | 0.006 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 6.164 |  | 0.225 | 0.226 | 0.016 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 6.888 |  | 0.299 | 0.266 | 0.245 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 6.085 |  | 0.491 | 0.656 | 0.598 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6.354 |  | 0.086 | 0.109 | 0.128 | yes |
| `grep -rn qdbzzabsentterm .` | 156.7 | 77.52 | 2179.8 | 2014.7 | 24.90 | yes |
| `grep -rl qdbzzabsentterm .` | 107.2 | 82.60 | 1991.3 | 2239.2 | 21.63 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 143.7 | 78.59 | 2113.7 | 2376.2 | 21.59 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 98.58 | 67.24 | 2229.1 | 2274.5 | 23.68 | yes |
| `grep -rn btvarstrequalimage .` | 143.8 | 78.73 | 2248.9 | 2281.9 | 25.56 | yes |
| `grep -rl btvarstrequalimage .` | 100.1 | 71.01 | 2222.5 | 2018.0 | 22.52 | yes |
| `grep -rn undesirable .` | 185.2 | 77.51 | 2014.0 | 2167.2 | 23.70 | yes |
| `grep -rl undesirable .` | 126.2 | 73.62 | 2284.4 | 2056.6 | 22.20 | yes |
| `grep -rn passed .` | 199.8 | 83.85 | 2138.0 | 2294.0 | 25.60 | yes |
| `grep -rl passed .` | 115.0 | 75.09 | 1557.0 | 1569.2 | 23.71 | yes |
| `grep -rn not .` | 331.4 | 151.7 | 2318.7 | 2155.5 | 35.41 | yes |
| `grep -rl not .` | 80.96 | 79.12 | 314.7 | 381.4 | 4.416 | yes |
| `grep -rc undesirable .` | 143.0 | 68.85 | 2066.5 | 2304.3 | 26.96 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 167.1 | 92.09 | 2297.0 | 2111.8 | 29.30 | yes |
| `grep -ri undesirable .` | 155.4 | 80.52 | 2281.0 | 2218.5 | 25.83 | yes |
| `grep -rn undesirable src/backend` | 77.96 | 29.34 | 824.4 | 790.0 | 8.451 | yes |
| `grep -rl passed src/backend` | 37.09 | 23.68 | 388.8 | 496.3 | 3.655 | yes |
| `grep -rF 'undesirable' .` | 138.6 | 69.38 | 245.0 | 238.6 | 31.88 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 112.7 | 73.49 | 2152.1 | 2400.4 | 24.35 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 213.0 | 85.29 | 3054.1 | 3024.7 | 32.25 | yes |
| `grep -rE 'passed\|undesirable' .` | 275.4 | 91.01 | 2612.7 | 3216.1 | 34.63 | yes |
| `grep -rE '[0-9]{4,}' .` | 278.6 | 325.7 | 2192.2 | 2054.3 | 170.5 | yes |
| `grep -rn '^#include' .` | 180.8 | 86.23 | 1673.7 | 1990.0 | 23.03 | yes |
| `grep -rE '^(import\|from) ' .` | 159.2 | 91.37 | 2040.8 | 2116.2 | 37.43 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 99.60 | 47.75 | 956.9 | 843.6 | 16.87 | yes |
| `grep -rv e src/include/access` | 9.129 | 10.53 | 7.669 | 3.327 | 0.475 | yes |
| `grep -r '\(undesirable\).*\1' .` | 139.0 |  | 3929.1 | 3988.6 | 24.74 | yes |
| `grep -rl passed . \| wc -l` | 112.9 |  | 1450.1 | 1465.5 | 18.33 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 231.3 |  | 2196.3 | 2213.1 | 30.46 | yes |
| `find . -name '*.html' \| wc -l` | 21.75 |  | 1.709 | 1.848 | 1.547 | yes |
| `find . -type f \| grep -c html` | 15.17 |  | 4.903 | 6.074 | 1.915 | yes |
| `grep -rn not . \| head -n 5` | 8.228 |  | 2280.0 | 0.467 | 0.011 | order differs |
| `grep -rn passed . \| head -n 1` | 23.20 |  | 2205.4 | 1.723 | 0.018 | order differs |
| `find . -type f \| head -n 3` | 8.637 |  | 2.421 | 0.044 | 0.097 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 9.865 |  | 15.07 | 0.539 | 0.597 | yes |
| `mkdir -p bench_out/a/b` | 8.068 |  | 0.025 | 0.022 | 0.021 | yes |
| `touch bench_out/empty.txt` | 9.177 |  | 0.018 | 0.019 | 0.015 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 7.468 |  | 0.018 | 0.018 | 0.018 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 8.079 |  | 0.030 | 0.013 | 0.020 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 9.987 |  | 0.041 | 0.034 | 0.042 | yes |
| `cp -r src/backend bench_out/dir_copy` | 150.6 |  | 29.83 | 51.93 | 41.81 | yes |
| `cp -r doc/src/sgml/html bench_out/work` | 35.98 |  | 12.97 | 15.61 | 12.32 | yes |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 13.41 |  | 0.758 | 0.386 | 0.345 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 12.65 |  | 0.701 | 0.343 | 0.349 | yes |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 136.8 |  | 320.7 | 336.1 | 207.3 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 38.24 |  | 1111.1 | 1046.2 | 11.39 | yes |
| `grep -rl QDBBENCHMANY .` | 88.13 |  | 3145.5 | 3030.1 | 33.30 | yes |
| `grep -rl QDBBENCHONE .` | 122.1 |  | 3329.2 | 3342.9 | 38.72 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 13.33 |  | 0.035 | 0.041 | 0.039 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 12.45 |  | 0.031 | 0.039 | 0.045 | yes |
| `ls -R bench_out/a` | 10.85 |  | 0.022 | 0.022 | 0.062 | yes |
| `rm bench_out/a/b/hello.txt` | 12.12 |  | 0.023 | 0.027 | 0.029 | yes |
| `rmdir bench_out/a/b` | 11.20 |  | 0.019 | 0.028 | 0.030 | yes |
| `rm -r bench_out/dir_moved` | 36.92 |  | 1.733 | 1.902 | 1.696 | yes |
| `rm -rf bench_out` | 110.0 |  | 2.232 | 1.934 | 1.958 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 | Part 3, no index |
|---|---:|---:|---:|---:|
| `ls` | 144 B | 1121 B | 961 B | 961 B |
| `ls -a` | 307 B | 1945 B | 1785 B | 1785 B |
| `ls -R` | 151.8 KB | 271.4 KB | 73.0 KB | 73.0 KB |
| `ls -aR src/backend` | 23.1 KB | 49.1 KB | 29.2 KB | 29.2 KB |
| `ls -p src/backend` | 279 B | 2235 B | 2075 B | 2075 B |
| `find .` | 347.5 KB | 443.4 KB | 28.5 KB | 28.5 KB |
| `find . -name '*.html'` | 49.6 KB | 69.1 KB | 18.3 KB | 18.3 KB |
| `find . -name '*.c'` | 54.0 KB | 65.0 KB | 33.0 KB | 33.0 KB |
| `find . -type d` | 16.8 KB | 34.0 KB | 17.5 KB | 17.5 KB |
| `find . -type f` | 330.7 KB | 410.9 KB | 26.7 KB | 26.7 KB |
| `find . -maxdepth 2 -type f` | 992 B | 2980 B | 2820 B | 2820 B |
| `find src/backend -name '*.html' -type f` | 0 B | 1876 B | 1952 B | 1952 B |
| `find . -name 'Makefile'` | 9792 B | 11.8 KB | 11.6 KB | 11.6 KB |
| `cat doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 20.8 KB | 1068 B | 1068 B |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 21.7 KB | 33.2 KB | 33.0 KB | 33.0 KB |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 3143 B | 4235 B | 4075 B | 4075 B |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2038 B | 3130 B | 2970 B | 2970 B |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 11.5 KB | 12.6 KB | 12.5 KB | 12.5 KB |
| `wc doc/src/sgml/html/bug-reporting.html` | 55 B | 992 B | 1076 B | 1076 B |
| `wc -l doc/src/sgml/html/*.html` | 57.8 KB | 268.9 KB | 268.9 KB | 268.9 KB |
| `cat doc/src/sgml/html/*.html \| wc -l` | 7 B | 22.2 MB | 269.1 KB | 269.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 565 B | 27.1 KB | 10.3 KB | 10.3 KB |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | 3631 B | 3415 B | 5856 B |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3886 B | 9263 B | 9047 B | 6933 B |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19.9 KB | 35.9 KB | 35.7 KB | 35.9 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 37.2 KB | 37.1 KB | 37.1 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6 B | 55.5 KB | 36.4 KB | 36.7 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB | 11.1 KB | 19.6 KB |
| `grep -rn abcdefghijklabcdefgh .` | 170 B | 15.3 KB | 15.1 KB | 26.1 KB |
| `grep -rl abcdefghijklabcdefgh .` | 41 B | 15.1 KB | 14.9 KB | 26.1 KB |
| `grep -rn btvarstrequalimage .` | 1660 B | 17.1 KB | 14.4 KB | 24.0 KB |
| `grep -rl btvarstrequalimage .` | 316 B | 14.4 KB | 13.8 KB | 24.0 KB |
| `grep -rn undesirable .` | 11.2 KB | 22.7 KB | 9295 B | 15.3 KB |
| `grep -rl undesirable .` | 2982 B | 12.3 KB | 8971 B | 15.3 KB |
| `grep -rn passed .` | 225.8 KB | 362.1 KB | 11.2 KB | 9290 B |
| `grep -rl passed .` | 30.4 KB | 42.7 KB | 5658 B | 9290 B |
| `grep -rn not .` | 13.4 MB | 26.2 MB | 29.3 KB | 31.1 KB |
| `grep -rl not .` | 189.7 KB | 208.7 KB | 3771 B | 5758 B |
| `grep -rc undesirable .` | 347.5 KB | 418.6 KB | 34.5 KB | 30.1 KB |
| `grep -rin BTVARSTREQUALIMAGE .` | 1730 B | 17.1 KB | 14.4 KB | 24.1 KB |
| `grep -ri undesirable .` | 10.8 KB | 22.1 KB | 9279 B | 15.3 KB |
| `grep -rn undesirable src/backend` | 3398 B | 13.7 KB | 9144 B | 15.3 KB |
| `grep -rl passed src/backend` | 10.7 KB | 21.1 KB | 5631 B | 9315 B |
| `grep -rF 'undesirable' .` | 10.8 KB | 14.5 KB | 1661 B | 1830 B |
| `grep -rE 'size_[a-z]+\(' .` | 20.4 KB | 44.7 KB | 9118 B | 15.1 KB |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1754 B | 30.4 KB | 27.8 KB | 48.5 KB |
| `grep -rE 'passed\|undesirable' .` | 228.0 KB | 349.3 KB | 18.4 KB | 23.2 KB |
| `grep -rE '[0-9]{4,}' .` | 39.4 MB | 52.4 MB | 29.5 KB | 23.1 KB |
| `grep -rn '^#include' .` | 1219.8 KB | 1383.7 KB | 17.9 KB | 17.3 KB |
| `grep -rE '^(import\|from) ' .` | 61.8 KB | 83.0 KB | 18.3 KB | 23.3 KB |
| `grep -rin 'return [a-z_]*;$' src/backend` | 902.7 KB | 991.2 KB | 38.6 KB | 33.8 KB |
| `grep -rv e src/include/access` | 232.5 KB | 428.4 KB | 28.8 KB | 27.5 KB |
| `grep -r '\(undesirable\).*\1' .` | 0 B | 5754 B | 5538 B | 9822 B |
| `grep -rl passed . \| wc -l` | 4 B | 43.2 KB | 6419 B | 10.1 KB |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 5 B | 634.8 KB | 10.3 KB | 10.9 KB |
| `find . -name '*.html' \| wc -l` | 5 B | 69.7 KB | 19.0 KB | 19.0 KB |
| `find . -type f \| grep -c html` | 5 B | 414.3 KB | 30.7 KB | 34.2 KB |
| `grep -rn not . \| head -n 5` | 386 B | 26.2 MB | 5922 B | 6624 B |
| `grep -rn passed . \| head -n 1` | 88 B | 362.8 KB | 6289 B | 10.2 KB |
| `find . -type f \| head -n 3` | 61 B | 411.6 KB | 27.5 KB | 27.5 KB |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 1555 B | 22.2 MB | 269.2 KB | 269.2 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1987 B | 1827 B | 1827 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B | 1246 B |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0 B | 21.1 KB | 20.9 KB | 20.9 KB |
| `cp -r src/backend bench_out/dir_copy` | 0 B | 65.1 MB | 65.1 MB | 65.1 MB |
| `cp -r doc/src/sgml/html bench_out/work` | 0 B | 16.8 MB | 16.8 MB | 16.8 MB |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 36.0 KB | 35.9 KB | 36.0 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 35.3 KB | 35.1 KB | 35.2 KB |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 0 B | 7758.3 KB | 7758.2 KB | 7758.3 KB |
| `grep -rc QDBBENCHMANY bench_out` | 110.1 KB | 156.9 KB | 27.9 KB | 28.7 KB |
| `grep -rl QDBBENCHMANY .` | 36.1 KB | 64.9 KB | 9532 B | 16.4 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 9209 B | 8993 B | 15.3 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1179 B | 1129 B | 1129 B |
| `rm -rf bench_out` | 0 B | 9514 B | 9354 B | 9354 B |

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
