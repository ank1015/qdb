# Benchmark: postgres

- Data: PostgreSQL 16.15 source tree, from the Ubuntu source package postgresql-16_16.15.orig.tar.gz (sha256 4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b). Skipped on import: 0 symbolic links, 63 files that are not valid text, 0 other
- Size: 8419 files in 611 folders, 161.2 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 1), and include starting bash and the tool (7.147 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 109.3 | 109.3 | 163.5 MB | 258.5 MB | yes |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Agree |
|---|---:|---:|---:|---|
| `ls` | 7.910 |  | 0.003 | yes |
| `ls -a` | 7.213 |  | 0.003 | yes |
| `ls -R` | 15.00 |  | 0.914 | yes |
| `ls -aR src/backend` | 8.091 |  | 0.102 | yes |
| `ls -p src/backend` | 6.488 |  | 0.003 | yes |
| `find .` | 14.75 |  | 2.207 | yes |
| `find . -name '*.html'` | 15.27 |  | 1.447 | yes |
| `find . -name '*.c'` | 15.79 |  | 2.411 | yes |
| `find . -type d` | 13.74 |  | 1.917 | yes |
| `find . -type f` | 15.04 |  | 1.601 | yes |
| `find . -maxdepth 2 -type f` | 7.147 |  | 0.013 | yes |
| `find src/backend -name '*.html' -type f` | 7.717 |  | 0.152 | yes |
| `find . -name 'Makefile'` | 12.72 |  | 1.271 | yes |
| `cat doc/src/sgml/html/bug-reporting.html` | 6.776 |  | 0.003 | yes |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 6.232 |  | 0.027 | yes |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 6.141 |  | 0.014 | yes |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 6.937 |  | 0.021 | yes |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 6.684 |  | 0.021 | yes |
| `wc doc/src/sgml/html/bug-reporting.html` | 6.625 |  | 0.035 | yes |
| `wc -l doc/src/sgml/html/*.html` | 18.92 |  | 30.97 | yes |
| `cat doc/src/sgml/html/*.html \| wc -l` | 39.70 |  | 35.75 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 9.150 |  | 0.029 | yes |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 6.563 |  | 0.230 | yes |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 6.809 |  | 0.225 | yes |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 6.826 |  | 0.299 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 6.677 |  | 0.491 | yes |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 7.265 |  | 0.086 | yes |
| `grep -rn qdbzzabsentterm .` | 156.7 | 73.69 | 2179.8 | yes |
| `grep -rl qdbzzabsentterm .` | 106.1 | 69.90 | 1991.3 | yes |
| `grep -rn abcdefghijklabcdefgh .` | 139.0 | 74.15 | 2113.7 | yes |
| `grep -rl abcdefghijklabcdefgh .` | 93.71 | 77.80 | 2229.1 | yes |
| `grep -rn btvarstrequalimage .` | 165.7 | 75.09 | 2248.9 | yes |
| `grep -rl btvarstrequalimage .` | 111.1 | 68.26 | 2222.5 | yes |
| `grep -rn undesirable .` | 185.5 | 73.37 | 2014.0 | yes |
| `grep -rl undesirable .` | 132.2 | 69.00 | 2284.4 | yes |
| `grep -rn passed .` | 175.7 | 79.94 | 2138.0 | yes |
| `grep -rl passed .` | 112.4 | 62.54 | 1557.0 | yes |
| `grep -rn not .` | 320.4 | 142.4 | 2318.7 | yes |
| `grep -rl not .` | 66.67 | 73.34 | 314.7 | yes |
| `grep -rc undesirable .` | 136.8 | 68.39 | 2066.5 | yes |
| `grep -rin BTVARSTREQUALIMAGE .` | 143.7 | 89.65 | 2297.0 | yes |
| `grep -ri undesirable .` | 158.0 | 94.19 | 2281.0 | yes |
| `grep -rn undesirable src/backend` | 72.79 | 28.20 | 824.4 | yes |
| `grep -rl passed src/backend` | 30.34 | 23.02 | 388.8 | yes |
| `grep -rF 'undesirable' .` | 134.4 | 63.98 | 245.0 | yes |
| `grep -rE 'size_[a-z]+\(' .` | 90.65 | 71.30 | 2152.1 | yes |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 207.8 | 81.27 | 3054.1 | yes |
| `grep -rE 'passed\|undesirable' .` | 272.0 | 74.12 | 2612.7 | yes |
| `grep -rE '[0-9]{4,}' .` | 290.6 | 336.1 | 2192.2 | yes |
| `grep -rn '^#include' .` | 193.0 | 89.62 | 1673.7 | yes |
| `grep -rE '^(import\|from) ' .` | 161.7 | 78.12 | 2040.8 | yes |
| `grep -rin 'return [a-z_]*;$' src/backend` | 100.9 | 42.33 | 956.9 | yes |
| `grep -rv e src/include/access` | 12.02 | 14.69 | 7.669 | yes |
| `grep -r '\(undesirable\).*\1' .` | 140.1 |  | 3929.1 | yes |
| `grep -rl passed . \| wc -l` | 111.5 |  | 1450.1 | yes |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 220.2 |  | 2196.3 | yes |
| `find . -name '*.html' \| wc -l` | 18.28 |  | 1.709 | yes |
| `find . -type f \| grep -c html` | 16.33 |  | 4.903 | yes |
| `grep -rn not . \| head -n 5` | 8.938 |  | 2280.0 | order differs |
| `grep -rn passed . \| head -n 1` | 25.35 |  | 2205.4 | order differs |
| `find . -type f \| head -n 3` | 9.534 |  | 2.421 | order differs |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 8.396 |  | 15.07 | yes |
| `mkdir -p bench_out/a/b` | 10.87 |  | 0.025 | yes |
| `touch bench_out/empty.txt` | 13.66 |  | 0.018 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 7.555 |  | 0.018 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 13.24 |  | 0.030 | yes |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 7.511 |  | 0.041 | yes |
| `cp -r src/backend bench_out/dir_copy` | 66.94 |  | 29.83 | yes |
| `cp -r doc/src/sgml/html bench_out/work` | 36.61 |  | 12.97 | yes |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 11.66 |  | 0.758 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 10.99 |  | 0.701 | yes |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 130.5 |  | 320.7 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 37.08 |  | 1111.1 | yes |
| `grep -rl QDBBENCHMANY .` | 91.35 |  | 3145.5 | yes |
| `grep -rl QDBBENCHONE .` | 109.3 |  | 3329.2 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 10.35 |  | 0.035 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 8.519 |  | 0.031 | yes |
| `ls -R bench_out/a` | 8.466 |  | 0.022 | yes |
| `rm bench_out/a/b/hello.txt` | 7.471 |  | 0.023 | yes |
| `rmdir bench_out/a/b` | 7.852 |  | 0.019 | yes |
| `rm -r bench_out/dir_moved` | 31.71 |  | 1.733 | yes |
| `rm -rf bench_out` | 105.7 |  | 2.232 | yes |

## Peak memory per command

| Command | Output | Stage 1 |
|---|---:|---:|
| `ls` | 144 B | 1121 B |
| `ls -a` | 307 B | 1945 B |
| `ls -R` | 151.8 KB | 271.4 KB |
| `ls -aR src/backend` | 23.1 KB | 49.1 KB |
| `ls -p src/backend` | 279 B | 2235 B |
| `find .` | 347.5 KB | 443.4 KB |
| `find . -name '*.html'` | 49.6 KB | 69.1 KB |
| `find . -name '*.c'` | 54.0 KB | 65.0 KB |
| `find . -type d` | 16.8 KB | 34.0 KB |
| `find . -type f` | 330.7 KB | 410.9 KB |
| `find . -maxdepth 2 -type f` | 992 B | 2980 B |
| `find src/backend -name '*.html' -type f` | 0 B | 1876 B |
| `find . -name 'Makefile'` | 9792 B | 11.8 KB |
| `cat doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 20.8 KB |
| `cat -n doc/src/sgml/html/bug-reporting.html` | 21.7 KB | 33.2 KB |
| `head -n 20 doc/src/sgml/html/bug-reporting.html` | 3143 B | 4235 B |
| `tail -n 20 doc/src/sgml/html/bug-reporting.html` | 2038 B | 3130 B |
| `tail -n +100 doc/src/sgml/html/bug-reporting.html` | 11.5 KB | 12.6 KB |
| `wc doc/src/sgml/html/bug-reporting.html` | 55 B | 992 B |
| `wc -l doc/src/sgml/html/*.html` | 57.8 KB | 268.9 KB |
| `cat doc/src/sgml/html/*.html \| wc -l` | 7 B | 22.2 MB |
| `cat doc/src/sgml/html/bug-reporting.html \| head -n 50 \| tail -n 10` | 565 B | 27.1 KB |
| `grep -c not doc/src/sgml/html/bug-reporting.html` | 3 B | 3631 B |
| `grep -n not doc/src/sgml/html/bug-reporting.html` | 3886 B | 9263 B |
| `sed 's/not/XX/g' doc/src/sgml/html/bug-reporting.html` | 19.9 KB | 35.9 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' doc/src/sgml/html/bug-reporting.html` | 20.0 KB | 37.2 KB |
| `cat doc/src/sgml/html/bug-reporting.html \| sed 's/^/> /' \| wc -c` | 6 B | 55.5 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 11.3 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 11.3 KB |
| `grep -rn abcdefghijklabcdefgh .` | 170 B | 15.3 KB |
| `grep -rl abcdefghijklabcdefgh .` | 41 B | 15.1 KB |
| `grep -rn btvarstrequalimage .` | 1660 B | 17.1 KB |
| `grep -rl btvarstrequalimage .` | 316 B | 14.4 KB |
| `grep -rn undesirable .` | 11.2 KB | 22.7 KB |
| `grep -rl undesirable .` | 2982 B | 12.3 KB |
| `grep -rn passed .` | 225.8 KB | 362.1 KB |
| `grep -rl passed .` | 30.4 KB | 42.7 KB |
| `grep -rn not .` | 13.4 MB | 26.2 MB |
| `grep -rl not .` | 189.7 KB | 208.7 KB |
| `grep -rc undesirable .` | 347.5 KB | 418.6 KB |
| `grep -rin BTVARSTREQUALIMAGE .` | 1730 B | 17.1 KB |
| `grep -ri undesirable .` | 10.8 KB | 22.1 KB |
| `grep -rn undesirable src/backend` | 3398 B | 13.7 KB |
| `grep -rl passed src/backend` | 10.7 KB | 21.1 KB |
| `grep -rF 'undesirable' .` | 10.8 KB | 14.5 KB |
| `grep -rE 'size_[a-z]+\(' .` | 20.4 KB | 44.7 KB |
| `grep -rE 'btvarstrequalimage\|abcdefghijklabcdefgh' .` | 1754 B | 30.4 KB |
| `grep -rE 'passed\|undesirable' .` | 228.0 KB | 349.3 KB |
| `grep -rE '[0-9]{4,}' .` | 39.4 MB | 52.4 MB |
| `grep -rn '^#include' .` | 1219.8 KB | 1383.7 KB |
| `grep -rE '^(import\|from) ' .` | 61.8 KB | 83.0 KB |
| `grep -rin 'return [a-z_]*;$' src/backend` | 902.7 KB | 991.2 KB |
| `grep -rv e src/include/access` | 232.5 KB | 428.4 KB |
| `grep -r '\(undesirable\).*\1' .` | 0 B | 5754 B |
| `grep -rl passed . \| wc -l` | 4 B | 43.2 KB |
| `grep -rc not . \| grep -v ':0$' \| wc -l` | 5 B | 634.8 KB |
| `find . -name '*.html' \| wc -l` | 5 B | 69.7 KB |
| `find . -type f \| grep -c html` | 5 B | 414.3 KB |
| `grep -rn not . \| head -n 5` | 386 B | 26.2 MB |
| `grep -rn passed . \| head -n 1` | 88 B | 362.8 KB |
| `find . -type f \| head -n 3` | 61 B | 411.6 KB |
| `cat doc/src/sgml/html/*.html \| head -n 2` | 1555 B | 22.2 MB |
| `mkdir -p bench_out/a/b` | 0 B | 1987 B |
| `touch bench_out/empty.txt` | 0 B | 883 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B |
| `cp doc/src/sgml/html/bug-reporting.html bench_out/copy.txt` | 0 B | 21.1 KB |
| `cp -r src/backend bench_out/dir_copy` | 0 B | 65.1 MB |
| `cp -r doc/src/sgml/html bench_out/work` | 0 B | 16.8 MB |
| `sed -i 's/not/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 36.0 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 35.3 KB |
| `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` | 0 B | 7758.3 KB |
| `grep -rc QDBBENCHMANY bench_out` | 110.1 KB | 156.9 KB |
| `grep -rl QDBBENCHMANY .` | 36.1 KB | 64.9 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 9209 B |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B |
| `ls -R bench_out/a` | 41 B | 1213 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B |
| `rmdir bench_out/a/b` | 0 B | 849 B |
| `rm -r bench_out/dir_moved` | 0 B | 1179 B |
| `rm -rf bench_out` | 0 B | 9514 B |

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
