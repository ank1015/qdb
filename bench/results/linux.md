# Benchmark: linux

- Data: Linux 6.8 source tree, from the Ubuntu source package linux_6.8.0.orig.tar.gz (sha256 26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8). Skipped on import: 64 symbolic links, 6 files that are not valid text, 0 other
- Size: 83405 files in 5451 folders, 1396.9 MB of text
- Tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0; ripgrep 14.1.0
- GNU and rg times are from the latest run (Part 3, no index), and include starting bash and the tool (26.482 ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | Agrees with GNU |
|---|---|---:|---:|---:|---:|---|
| Stage 1 | on | 886.3 | 886.3 | 1416.9 MB | 2279.2 MB | yes |
| Part 2 | on | 887.5 | 887.5 | 1416.9 MB | 2279.3 MB | yes |
| Part 3, no index | off | 1485.8 | 1485.8 | 1416.9 MB | 1425.3 MB | yes |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 1 | Part 2 | Part 3, no index | Agree |
|---|---:|---:|---:|---:|---:|---|
| `ls` | 24.05 |  | 0.003 | 0.003 | 0.002 | yes |
| `ls -a` | 22.69 |  | 0.008 | 0.004 | 0.004 | yes |
| `ls -R` | 85.44 |  | 21.31 | 15.79 | 17.25 | yes |
| `ls -aR Documentation` | 39.35 |  | 1.501 | 1.091 | 2.003 | yes |
| `ls -p Documentation` | 26.41 |  | 0.005 | 0.005 | 0.005 | yes |
| `find .` | 101.4 |  | 20.91 | 21.94 | 26.47 | yes |
| `find . -name '*.h'` | 100.5 |  | 31.50 | 23.56 | 25.93 | yes |
| `find . -name '*.c'` | 100.8 |  | 27.93 | 37.57 | 26.99 | yes |
| `find . -type d` | 86.15 |  | 25.24 | 30.06 | 32.27 | yes |
| `find . -type f` | 97.66 |  | 26.23 | 23.04 | 25.89 | yes |
| `find . -maxdepth 2 -type f` | 27.17 |  | 0.144 | 0.201 | 0.155 | yes |
| `find Documentation -name '*.h' -type f` | 38.84 |  | 1.703 | 1.839 | 2.138 | yes |
| `find . -name 'Makefile'` | 97.17 |  | 24.61 | 21.08 | 22.21 | yes |
| `cat include/linux/kernfs.h` | 25.58 |  | 0.002 | 0.001 | 0.002 | yes |
| `cat -n include/linux/kernfs.h` | 21.04 |  | 0.079 | 0.077 | 0.058 | yes |
| `head -n 20 include/linux/kernfs.h` | 21.43 |  | 0.011 | 0.002 | 0.002 | yes |
| `tail -n 20 include/linux/kernfs.h` | 20.89 |  | 0.030 | 0.002 | 0.003 | yes |
| `tail -n +100 include/linux/kernfs.h` | 21.58 |  | 0.015 | 0.004 | 0.004 | yes |
| `wc include/linux/kernfs.h` | 22.16 |  | 0.037 | 0.032 | 0.048 | yes |
| `wc -l include/linux/*.h` | 33.86 |  | 24.16 | 24.60 | 21.34 | yes |
| `cat include/linux/*.h \| wc -l` | 57.80 |  | 28.57 | 25.07 | 33.47 | yes |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 29.34 |  | 0.017 | 0.017 | 0.008 | yes |
| `grep -c define include/linux/kernfs.h` | 25.68 |  | 0.271 | 0.237 | 0.008 | yes |
| `grep -n define include/linux/kernfs.h` | 24.11 |  | 0.426 | 0.306 | 0.010 | yes |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 23.07 |  | 0.334 | 0.326 | 0.278 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 24.55 |  | 0.740 | 0.664 | 0.636 | yes |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 24.00 |  | 0.168 | 0.145 | 0.139 | yes |
| `grep -rn qdbzzabsentterm .` | 1165.7 | 591.9 | 17359.5 | 17645.1 | 187.6 | yes |
| `grep -rl qdbzzabsentterm .` | 803.3 | 490.6 | 17711.1 | 18597.6 | 179.0 | yes |
| `grep -rn ausgabenspezifisches .` | 1057.0 | 559.3 | 19018.3 | 19501.9 | 179.0 | yes |
| `grep -rl ausgabenspezifisches .` | 785.4 | 531.0 | 18956.5 | 19324.0 | 181.7 | yes |
| `grep -rn mispredicted .` | 1317.7 | 572.6 | 17982.9 | 18955.5 | 196.5 | yes |
| `grep -rl mispredicted .` | 973.7 | 523.2 | 19048.8 | 18957.5 | 184.2 | yes |
| `grep -rn separately .` | 968.0 | 614.5 | 18360.5 | 18397.0 | 182.2 | yes |
| `grep -rl separately .` | 564.6 | 536.6 | 19440.4 | 19427.6 | 189.5 | yes |
| `grep -rn conditions .` | 1288.5 | 648.9 | 18999.5 | 19519.0 | 208.6 | yes |
| `grep -rl conditions .` | 692.3 | 534.2 | 12072.4 | 12435.8 | 153.2 | yes |
| `grep -rn define .` | 3589.6 | 3092.8 | 15389.6 | 15237.7 | 437.2 | yes |
| `grep -rl define .` | 578.1 | 667.8 | 4161.3 | 3944.5 | 77.25 | yes |
| `grep -rc separately .` | 618.4 | 585.9 | 18956.0 | 18220.2 | 195.6 | yes |
| `grep -rin MISPREDICTED .` | 1512.8 | 705.1 | 18850.7 | 18964.1 | 235.2 | yes |
| `grep -ri separately .` | 857.8 | 683.4 | 19767.8 | 18645.3 | 233.4 | yes |
| `grep -rn separately Documentation` | 93.86 | 74.59 | 711.5 | 557.6 | 10.28 | yes |
| `grep -rl conditions Documentation` | 86.09 | 69.71 | 628.8 | 626.2 | 8.525 | yes |
| `grep -rF 'separately' .` | 646.3 | 505.0 | 2095.5 | 2388.0 | 201.7 | yes |
| `grep -rE 'atomic_[a-z]+\(' .` | 1229.1 | 583.0 | 18552.0 | 18591.7 | 208.5 | yes |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 1561.3 | 663.4 | 25700.0 | 27700.5 | 251.5 | yes |
| `grep -rE 'conditions\|separately' .` | 1997.0 | 678.3 | 26745.7 | 26952.0 | 278.9 | yes |
| `grep -rE '[0-9]{4,}' .` | 2701.3 | 3060.5 | 18321.8 | 19699.7 | 1623.0 | yes |
| `grep -rn '^#include' .` | 1705.8 | 932.6 | 17544.7 | 16706.7 | 233.9 | yes |
| `grep -rE '^(import\|from) ' .` | 1119.0 | 683.8 | 17091.8 | 18141.6 | 296.2 | yes |
| `grep -rin 'return [a-z_]*;$' Documentation` | 120.2 | 85.51 | 723.2 | 710.6 | 13.19 | yes |
| `grep -rv e drivers/usb` | 89.21 | 122.7 | 132.2 | 107.4 | 17.98 | yes |
| `grep -r '\(separately\).*\1' .` | 577.0 |  | 34553.4 | 35931.9 | 203.7 | yes |
| `grep -rl conditions . \| wc -l` | 777.0 |  | 12392.2 | 11985.0 | 148.5 | yes |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 1157.8 |  | 14309.4 | 14923.1 | 320.0 | yes |
| `find . -name '*.h' \| wc -l` | 101.9 |  | 28.57 | 24.95 | 29.91 | yes |
| `find . -type f \| grep -c h` | 101.3 |  | 63.40 | 58.08 | 31.15 | yes |
| `grep -rn define . \| head -n 5` | 28.44 |  | 15337.2 | 3.769 | 0.021 | order differs |
| `grep -rn conditions . \| head -n 1` | 34.14 |  | 18581.5 | 6.938 | 0.040 | order differs |
| `find . -type f \| head -n 3` | 29.91 |  | 28.01 | 0.092 | 0.062 | order differs |
| `cat include/linux/*.h \| head -n 2` | 29.12 |  | 9.193 | 0.347 | 0.284 | yes |
| `mkdir -p bench_out/a/b` | 24.78 |  | 0.043 | 0.026 | 0.045 | yes |
| `touch bench_out/empty.txt` | 25.35 |  | 0.028 | 0.019 | 0.061 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 23.11 |  | 0.030 | 0.025 | 0.031 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 23.29 |  | 0.021 | 0.023 | 0.026 | yes |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 23.71 |  | 0.071 | 0.044 | 0.051 | yes |
| `cp -r Documentation bench_out/dir_copy` | 1274.5 |  | 36.54 | 55.14 | 46.77 | yes |
| `cp -r include/linux bench_out/work` | 399.9 |  | 17.02 | 21.90 | 24.02 | yes |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 33.13 |  | 0.369 | 0.371 | 0.289 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 32.54 |  | 0.608 | 0.397 | 0.327 | yes |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 367.3 |  | 154.4 | 187.8 | 142.3 | yes |
| `grep -rc QDBBENCHMANY bench_out` | 79.34 |  | 907.6 | 911.1 | 14.41 | yes |
| `grep -rl QDBBENCHMANY .` | 605.8 |  | 18985.7 | 19134.6 | 211.5 | yes |
| `grep -rl QDBBENCHONE .` | 881.6 |  | 18818.5 | 18986.5 | 212.4 | yes |
| `mv bench_out/dir_copy bench_out/dir_moved` | 30.29 |  | 0.036 | 0.048 | 0.038 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 30.16 |  | 0.046 | 0.046 | 0.050 | yes |
| `ls -R bench_out/a` | 29.33 |  | 0.026 | 0.035 | 0.036 | yes |
| `rm bench_out/a/b/hello.txt` | 24.99 |  | 0.030 | 0.035 | 0.030 | yes |
| `rmdir bench_out/a/b` | 24.87 |  | 0.035 | 0.038 | 0.034 | yes |
| `rm -r bench_out/dir_moved` | 140.0 |  | 10.95 | 9.650 | 8.896 | yes |
| `rm -rf bench_out` | 261.4 |  | 3.379 | 2.941 | 3.203 | yes |

## Peak memory per command

| Command | Output | Stage 1 | Part 2 | Part 3, no index |
|---|---:|---:|---:|---:|
| `ls` | 212 B | 1617 B | 1457 B | 1457 B |
| `ls -a` | 330 B | 2841 B | 2681 B | 2681 B |
| `ls -R` | 1377.7 KB | 2155.2 KB | 74.3 KB | 74.3 KB |
| `ls -aR Documentation` | 216.8 KB | 437.9 KB | 56.6 KB | 56.6 KB |
| `ls -p Documentation` | 796 B | 5053 B | 4893 B | 4893 B |
| `find .` | 3590.9 KB | 4457.9 KB | 19.2 KB | 19.2 KB |
| `find . -name '*.h'` | 969.5 KB | 1312.4 KB | 22.7 KB | 22.7 KB |
| `find . -name '*.c'` | 1233.5 KB | 1247.0 KB | 21.7 KB | 21.7 KB |
| `find . -type d` | 166.7 KB | 280.0 KB | 19.5 KB | 19.5 KB |
| `find . -type f` | 3424.2 KB | 3933.8 KB | 32.8 KB | 32.8 KB |
| `find . -maxdepth 2 -type f` | 23.5 KB | 32.1 KB | 32.0 KB | 32.0 KB |
| `find Documentation -name '*.h' -type f` | 76 B | 2097 B | 1937 B | 1937 B |
| `find . -name 'Makefile'` | 100.1 KB | 198.2 KB | 26.9 KB | 26.9 KB |
| `cat include/linux/kernfs.h` | 19.9 KB | 20.7 KB | 994 B | 994 B |
| `cat -n include/linux/kernfs.h` | 24.4 KB | 27.0 KB | 26.8 KB | 26.8 KB |
| `head -n 20 include/linux/kernfs.h` | 459 B | 1505 B | 1345 B | 1345 B |
| `tail -n 20 include/linux/kernfs.h` | 478 B | 1524 B | 1364 B | 1364 B |
| `tail -n +100 include/linux/kernfs.h` | 16.9 KB | 18.0 KB | 1306 B | 1306 B |
| `wc include/linux/kernfs.h` | 41 B | 946 B | 1002 B | 1002 B |
| `wc -l include/linux/*.h` | 49.6 KB | 237.0 KB | 237.0 KB | 237.0 KB |
| `cat include/linux/*.h \| wc -l` | 7 B | 12.4 MB | 237.2 KB | 237.2 KB |
| `cat include/linux/kernfs.h \| head -n 50 \| tail -n 10` | 447 B | 23.1 KB | 4720 B | 4720 B |
| `grep -c define include/linux/kernfs.h` | 2 B | 5500 B | 5284 B | 9342 B |
| `grep -n define include/linux/kernfs.h` | 374 B | 5916 B | 5700 B | 9342 B |
| `sed 's/define/XX/g' include/linux/kernfs.h` | 19.9 KB | 27.1 KB | 27.0 KB | 27.0 KB |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' include/linux/kernfs.h` | 19.9 KB | 26.6 KB | 26.4 KB | 26.6 KB |
| `cat include/linux/kernfs.h \| sed 's/^/> /' \| wc -c` | 6 B | 45.1 KB | 25.5 KB | 25.8 KB |
| `grep -rn qdbzzabsentterm .` | 0 B | 12.2 KB | 12.0 KB | 19.6 KB |
| `grep -rl qdbzzabsentterm .` | 0 B | 12.2 KB | 12.0 KB | 19.6 KB |
| `grep -rn ausgabenspezifisches .` | 116 B | 16.0 KB | 15.8 KB | 26.1 KB |
| `grep -rl ausgabenspezifisches .` | 32 B | 16.0 KB | 15.8 KB | 26.1 KB |
| `grep -rn mispredicted .` | 111.0 KB | 165.5 KB | 19.4 KB | 16.4 KB |
| `grep -rl mispredicted .` | 4531 B | 14.7 KB | 10.4 KB | 16.4 KB |
| `grep -rn separately .` | 103.9 KB | 176.7 KB | 10.5 KB | 14.2 KB |
| `grep -rl separately .` | 30.7 KB | 50.9 KB | 9318 B | 14.2 KB |
| `grep -rn conditions .` | 1360.5 KB | 2564.6 KB | 13.5 KB | 14.2 KB |
| `grep -rl conditions .` | 389.6 KB | 647.9 KB | 9392 B | 14.2 KB |
| `grep -rn define .` | 821.1 MB | 830.5 MB | 31.8 KB | 26.8 KB |
| `grep -rl define .` | 1867.6 KB | 1972.3 KB | 6624 B | 9290 B |
| `grep -rc separately .` | 3591.0 KB | 3941.1 KB | 40.0 KB | 19.9 KB |
| `grep -rin MISPREDICTED .` | 135.7 KB | 165.5 KB | 19.4 KB | 16.4 KB |
| `grep -ri separately .` | 100.6 KB | 176.7 KB | 10.5 KB | 14.2 KB |
| `grep -rn separately Documentation` | 19.9 KB | 28.7 KB | 9151 B | 14.2 KB |
| `grep -rl conditions Documentation` | 9541 B | 27.3 KB | 8675 B | 14.2 KB |
| `grep -rF 'separately' .` | 99.7 KB | 169.6 KB | 3480 B | 2284 B |
| `grep -rE 'atomic_[a-z]+\(' .` | 1294.9 KB | 2040.2 KB | 16.5 KB | 23.0 KB |
| `grep -rE 'mispredicted\|ausgabenspezifisches' .` | 109.2 KB | 179.1 KB | 33.0 KB | 41.2 KB |
| `grep -rE 'conditions\|separately' .` | 1427.8 KB | 2571.4 KB | 20.3 KB | 26.8 KB |
| `grep -rE '[0-9]{4,}' .` | 467.5 MB | 729.8 MB | 27.1 KB | 28.6 KB |
| `grep -rn '^#include' .` | 24.9 MB | 33.6 MB | 24.5 KB | 29.4 KB |
| `grep -rE '^(import\|from) ' .` | 116.4 KB | 144.7 KB | 11.5 KB | 20.9 KB |
| `grep -rin 'return [a-z_]*;$' Documentation` | 24.1 KB | 44.1 KB | 12.5 KB | 17.5 KB |
| `grep -rv e drivers/usb` | 9638.5 KB | 9964.0 KB | 21.8 KB | 21.9 KB |
| `grep -r '\(separately\).*\1' .` | 188 B | 6770 B | 6376 B | 9274 B |
| `grep -rl conditions . \| wc -l` | 5 B | 648.5 KB | 10.2 KB | 14.9 KB |
| `grep -rc define . \| grep -v ':0$' \| wc -l` | 6 B | 6164.4 KB | 25.4 KB | 25.8 KB |
| `find . -name '*.h' \| wc -l` | 6 B | 1313.0 KB | 23.4 KB | 23.4 KB |
| `find . -type f \| grep -c h` | 6 B | 3935.0 KB | 34.9 KB | 35.8 KB |
| `grep -rn define . \| head -n 5` | 473 B | 830.5 MB | 6667 B | 10.2 KB |
| `grep -rn conditions . \| head -n 1` | 107 B | 2565.4 KB | 9150 B | 15.0 KB |
| `find . -type f \| head -n 3` | 47 B | 3934.6 KB | 32.6 KB | 32.6 KB |
| `cat include/linux/*.h \| head -n 2` | 42 B | 12.4 MB | 237.3 KB | 237.3 KB |
| `mkdir -p bench_out/a/b` | 0 B | 1619 B | 1459 B | 1459 B |
| `touch bench_out/empty.txt` | 0 B | 883 B | 979 B | 979 B |
| `echo 'hello world' > bench_out/hello.txt` | 0 B | 1207 B | 1242 B | 1242 B |
| `echo 'another line' >> bench_out/hello.txt` | 0 B | 1150 B | 1246 B | 1246 B |
| `cp include/linux/kernfs.h bench_out/copy.txt` | 0 B | 21.0 KB | 20.8 KB | 20.8 KB |
| `cp -r Documentation bench_out/dir_copy` | 0 B | 48.1 MB | 48.1 MB | 48.1 MB |
| `cp -r include/linux bench_out/work` | 0 B | 19.1 MB | 19.1 MB | 19.1 MB |
| `sed -i 's/define/QDBBENCHONE/g' bench_out/copy.txt` | 0 B | 27.3 KB | 27.2 KB | 27.2 KB |
| `sed -i -E 's/[0-9]+/N/g' bench_out/copy.txt` | 0 B | 24.9 KB | 24.7 KB | 24.8 KB |
| `sed -i 's/define/QDBBENCHMANY/g' bench_out/work/*.h` | 0 B | 4911.5 KB | 4911.3 KB | 4911.4 KB |
| `grep -rc QDBBENCHMANY bench_out` | 687.0 KB | 1189.3 KB | 28.1 KB | 28.7 KB |
| `grep -rl QDBBENCHMANY .` | 39.7 KB | 65.9 KB | 10.4 KB | 16.4 KB |
| `grep -rl QDBBENCHONE .` | 21 B | 10.0 KB | 9829 B | 15.3 KB |
| `mv bench_out/dir_copy bench_out/dir_moved` | 0 B | 1095 B | 1191 B | 1191 B |
| `mv bench_out/hello.txt bench_out/a/b` | 0 B | 1448 B | 1288 B | 1288 B |
| `ls -R bench_out/a` | 41 B | 1213 B | 1089 B | 1089 B |
| `rm bench_out/a/b/hello.txt` | 0 B | 901 B | 997 B | 997 B |
| `rmdir bench_out/a/b` | 0 B | 849 B | 945 B | 945 B |
| `rm -r bench_out/dir_moved` | 0 B | 1611 B | 1451 B | 1451 B |
| `rm -rf bench_out` | 0 B | 12.5 KB | 12.4 KB | 12.4 KB |

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
