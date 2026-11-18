# Benchmark: linux

- Data: Linux 6.8 source tree, from the Ubuntu source package linux_6.8.0.orig.tar.gz (sha256 26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8). Skipped on import: 64 symbolic links, 6 files that are not valid text, 0 other
- Size: 83405 files in 5451 folders, 1396.9 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Stage 1), and include starting bash and the tool (52.944 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 886.3 | 886.3 | 1416.9 MB | 2279.2 MB | yes |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Agree |
|---|---:|---:|---:|---|
| `ls` | 48.58 |  | 0.003 | yes |
| `ls -a` | 48.67 |  | 0.008 | yes |
| `ls -R` | 116.1 |  | 21.31 | yes |
| `ls -aR Documentation` | 63.87 |  | 1.501 | yes |
| `ls -p Documentation` | 57.96 |  | 0.005 | yes |
| `find .` | 116.1 |  | 20.91 | yes |
| `find . -name '*.h'` | 114.7 |  | 31.50 | yes |
| `find . -name '*.c'` | 132.6 |  | 27.93 | yes |
| `find . -type d` | 110.0 |  | 25.24 | yes |
| `find . -type f` | 128.2 |  | 26.23 | yes |
| `find . -maxdepth 2 -type f` | 56.12 |  | 0.144 | yes |
| `find Documentation -name '*.h' -type f` | 63.51 |  | 1.703 | yes |
| `find . -name 'Makefile'` | 119.5 |  | 24.61 | yes |
| `cat include/linux/kernfs.h` | 52.90 |  | 0.002 | yes |
| `cat -n include/linux/kernfs.h` | 52.86 |  | 0.079 | yes |
| `head -n 20 include/linux/kernfs.h` | 50.64 |  | 0.011 | yes |
| `tail -n 20 include/linux/kernfs.h` | 49.87 |  | 0.030 | yes |
| `tail -n +100 include/linux/kernfs.h` | 48.94 |  | 0.015 | yes |
| `wc include/linux/kernfs.h` | 51.64 |  | 0.037 | yes |
| `wc -l include/linux/*.h` | 66.06 |  | 24.16 | yes |
| `cat include/linux/*.h \| wc -l` | 81.01 |  | 28.57 | yes |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 54.86 |  | 0.017 | yes |
| `grep -c define include/linux/kernfs.h` | 52.72 |  | 0.271 | yes |
| `grep -n define include/linux/kernfs.h` | 53.08 |  | 0.426 | yes |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 55.99 |  | 0.334 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 57.16 |  | 0.740 | yes |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 56.35 |  | 0.168 | yes |
| `grep -rn qdbzzabsentterm .` | 1083.1 | 624.9 | 17359.5 | yes |
| `grep -rl qdbzzabsentterm .` | 755.6 | 568.2 | 17711.1 | yes |
| `grep -rn ausgabenspezifisches .` | 1093.3 | 628.1 | 19018.3 | yes |
| `grep -rl ausgabenspezifisches .` | 812.5 | 601.3 | 18956.5 | yes |
| `grep -rn mispredicted .` | 1341.9 | 613.0 | 17982.9 | yes |
| `grep -rl mispredicted .` | 910.3 | 600.0 | 19048.8 | yes |
| `grep -rn separately .` | 1012.6 | 628.5 | 18360.5 | yes |
| `grep -rl separately .` | 608.7 | 566.1 | 19440.4 | yes |
| `grep -rn conditions .` | 1351.0 | 658.7 | 18999.5 | yes |
| `grep -rl conditions .` | 810.5 | 556.2 | 12072.4 | yes |
| `grep -rn define .` | 3753.1 | 3173.8 | 15389.6 | yes |
| `grep -rl define .` | 637.5 | 637.1 | 4161.3 | yes |
| `grep -rc separately .` | 701.2 | 540.1 | 18956.0 | yes |
| `grep -rin MISPREDICTED .` | 1556.1 | 671.4 | 18850.7 | yes |
| `grep -ri separately .` | 878.8 | 673.7 | 19767.8 | yes |
| `grep -rn separately Documentation` | 118.6 | 99.74 | 711.5 | yes |
| `grep -rl conditions Documentation` | 98.80 | 100.3 | 628.8 | yes |
| `grep -rF 'separately' .` | 623.1 | 583.6 | 2095.5 | yes |
| `grep -rE 'atomic_[a-z]+\(' .` | 1302.8 | 621.3 | 18552.0 | yes |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 1557.2 | 665.7 | 25700.0 | yes |
| `grep -rE 'conditions\|separately' .` | 2245.3 | 734.0 | 26745.7 | yes |
| `grep -rE '[0-9]{4,}' .` | 2734.7 | 3338.7 | 18321.8 | yes |
| `grep -rn '^#include' .` | 1703.9 | 901.9 | 17544.7 | yes |
| `grep -rE '^(import\|from) ' .` | 1122.4 | 680.1 | 17091.8 | yes |
| `grep -rin 'return [a-z_]*;$' Documentation` | 153.6 | 115.5 | 723.2 | yes |
| `grep -rv e drivers/usb` | 113.9 | 123.3 | 132.2 | yes |
| `grep -r '\(separately\).*\1' .` | 668.8 |  | 34553.4 | yes |
| `grep -rl conditions . \| wc -l` | 770.5 |  | 12392.2 | yes |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1331.7 |  | 14309.4 | yes |
| `find . -name '*.h' \| wc -l` | 125.6 |  | 28.57 | yes |
| `find . -type f \| grep -c h` | 128.1 |  | 63.40 | yes |
| `grep -rn define . \| head -n 5` | 57.27 |  | 15337.2 | order differs |
| `grep -rn conditions . \| head -n 1` | 73.87 |  | 18581.5 | order differs |
| `find . -type f \| head -n 3` | 55.07 |  | 28.01 | order differs |
| `cat include/linux/*.h \| head -n 2` | 57.89 |  | 9.193 | yes |
| `mkdir -p bench_out/a/b` | 58.44 |  | 0.043 | yes |
| `touch bench_out/empty.txt` | 57.02 |  | 0.028 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 52.70 |  | 0.030 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 56.41 |  | 0.021 | yes |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 55.79 |  | 0.071 | yes |
| `cp -r Documentation bench_out/dir_copy` | 224.6 |  | 36.54 | yes |
| `cp -r include/linux bench_out/work` | 87.08 |  | 17.02 | yes |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 59.27 |  | 0.369 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 57.99 |  | 0.608 | yes |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 152.4 |  | 154.4 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 104.8 |  | 907.6 | yes |
| `grep -rl QDBBENCHMANY .` | 662.5 |  | 18985.7 | yes |
| `grep -rl QDBBENCHONE .` | 888.4 |  | 18818.5 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 56.04 |  | 0.036 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 52.19 |  | 0.046 | yes |
| `ls -R bench_out/a` | 53.96 |  | 0.026 | yes |
| `rm bench_out/a/b/hello.txt` | 52.54 |  | 0.030 | yes |
| `rmdir bench_out/a/b` | 55.58 |  | 0.035 | yes |
| `rm -r bench_out/dir_moved` | 868.9 |  | 10.95 | yes |
| `rm -rf bench_out` | 302.4 |  | 3.379 | yes |

## Peak memory per command

| Command | Output | Stage 1 |
|---|---:|---:|
| `ls` | 212 B | 1617 B |
| `ls -a` | 330 B | 2841 B |
| `ls -R` | 1377.7 KB | 2155.2 KB |
| `ls -aR Documentation` | 216.8 KB | 437.9 KB |
| `ls -p Documentation` | 796 B | 5053 B |
| `find .` | 3590.9 KB | 4457.9 KB |
| `find . -name '*.h'` | 969.5 KB | 1312.4 KB |
| `find . -name '*.c'` | 1233.5 KB | 1247.0 KB |
| `find . -type d` | 166.7 KB | 280.0 KB |
| `find . -type f` | 3424.2 KB | 3933.8 KB |
| `find . -maxdepth 2 -type f` | 23.5 KB | 32.1 KB |
| `find Documentation -name '*.h' -type f` | 76 B | 2097 B |
| `find . -name 'Makefile'` | 100.1 KB | 198.2 KB |
| `cat include/linux/kernfs.h` | 19.9 KB | 20.7 KB |
| `cat -n include/linux/kernfs.h` | 24.4 KB | 27.0 KB |
| `head -n 20 include/linux/kernfs.h` | 459 B | 1505 B |
| `tail -n 20 include/linux/kernfs.h` | 478 B | 1524 B |
| `tail -n +100 include/linux/kernfs.h` | 16.9 KB | 18.0 KB |
| `wc include/linux/kernfs.h` | 41 B | 946 B |
| `wc -l include/linux/*.h` | 49.6 KB | 237.0 KB |
| `cat include/linux/*.h \| wc -l` | 7 B | 12.4 MB |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 447 B | 23.1 KB |
| `grep -c define include/linux/kernfs.h` | 2 B | 5500 B |
| `grep -n define include/linux/kernfs.h` | 374 B | 5916 B |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 19.9 KB | 27.1 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 19.9 KB | 26.6 KB |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 6 B | 45.1 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 12.2 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 12.2 KB |
| `grep -rn ausgabenspezifisches .` | 116 B | 16.0 KB |
| `grep -rl ausgabenspezifisches .` | 32 B | 16.0 KB |
| `grep -rn mispredicted .` | 111.0 KB | 165.5 KB |
| `grep -rl mispredicted .` | 4531 B | 14.7 KB |
| `grep -rn separately .` | 103.9 KB | 176.7 KB |
| `grep -rl separately .` | 30.7 KB | 50.9 KB |
| `grep -rn conditions .` | 1360.5 KB | 2564.6 KB |
| `grep -rl conditions .` | 389.6 KB | 647.9 KB |
| `grep -rn define .` | 821.1 MB | 830.5 MB |
| `grep -rl define .` | 1867.6 KB | 1972.3 KB |
| `grep -rc separately .` | 3591.0 KB | 3941.1 KB |
| `grep -rin MISPREDICTED .` | 135.7 KB | 165.5 KB |
| `grep -ri separately .` | 100.6 KB | 176.7 KB |
| `grep -rn separately Documentation` | 19.9 KB | 28.7 KB |
| `grep -rl conditions Documentation` | 9541 B | 27.3 KB |
| `grep -rF 'separately' .` | 99.7 KB | 169.6 KB |
| `grep -rE 'atomic_[a-z]+\(' .` | 1294.9 KB | 2040.2 KB |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 109.2 KB | 179.1 KB |
| `grep -rE 'conditions\|separately' .` | 1427.8 KB | 2571.4 KB |
| `grep -rE '[0-9]{4,}' .` | 467.5 MB | 729.8 MB |
| `grep -rn '^#include' .` | 24.9 MB | 33.6 MB |
| `grep -rE '^(import\|from) ' .` | 116.4 KB | 144.7 KB |
| `grep -rin 'return [a-z_]*;$' Documentation` | 24.1 KB | 44.1 KB |
| `grep -rv e drivers/usb` | 9638.5 KB | 9964.0 KB |
| `grep -r '\(separately\).*\1' .` | 188 B | 6770 B |
| `grep -rl conditions . \| wc -l` | 5 B | 648.5 KB |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 6 B | 6164.4 KB |
| `find . -name '*.h' \| wc -l` | 6 B | 1313.0 KB |
| `find . -type f \| grep -c h` | 6 B | 3935.0 KB |
| `grep -rn define . \| head -n 5` | 473 B | 830.5 MB |
| `grep -rn conditions . \| head -n 1` | 107 B | 2565.4 KB |
| `find . -type f \| head -n 3` | 47 B | 3934.6 KB |
| `cat include/linux/*.h \| head -n 2` | 42 B | 12.4 MB |
| `mkdir -p bench_out/a/b` | 0 B | 1619 B |
| `touch bench_out/empty.txt` | 0 B | 883 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 0 B | 21.0 KB |
| `cp -r Documentation bench_out/dir_copy` | 0 B | 48.1 MB |
| `cp -r include/linux bench_out/work` | 0 B | 19.1 MB |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 27.3 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 24.9 KB |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 0 B | 4911.5 KB |
| `grep -rc QDBBENCHMANY bench_out` | 687.0 KB | 1189.3 KB |
| `grep -rl QDBBENCHMANY .` | 39.7 KB | 65.9 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 10.0 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B |
| `ls -R bench_out/a` | 41 B | 1213 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B |
| `rmdir bench_out/a/b` | 0 B | 849 B |
| `rm -r bench_out/dir_moved` | 0 B | 1611 B |
| `rm -rf bench_out` | 0 B | 12.5 KB |

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
