# Benchmark: large dataset

- Dataset: 12005 files in 6925 folders, 87.4 MB of text
- Engine load time (reading the dataset from disk into memory): 105.8 ms
- Real tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0
- Process start-up cost of the real tools (`bash -c 'echo -n'`): 5.211 ms. Every real time below includes it; engine times are measured in-process and include parsing the command.
- Read-only commands: median of 5 runs. Commands that change data: one run.
- Results agree with the real tools: yes, for every command and for the final tree

| Command | Real (ms) | Engine (ms) | Engine / real | Output (bytes) | Agree |
|---|---:|---:|---:|---:|---|
| `ls` | 5.920 | 0.010 | 0.00x | 44 | yes |
| `ls -a` | 6.134 | 0.013 | 0.00x | 60 | yes |
| `ls -R` | 48.64 | 4.896 | 0.10x | 426265 | yes |
| `ls -aR src` | 21.68 | 2.016 | 0.09x | 175990 | yes |
| `ls -p src` | 7.052 | 0.117 | 0.02x | 13736 | yes |
| `find .` | 52.76 | 6.425 | 0.12x | 615364 | yes |
| `find . -name '*.c'` | 54.89 | 6.721 | 0.12x | 180817 | yes |
| `find . -name '*.md'` | 48.10 | 6.188 | 0.13x | 33403 | yes |
| `find . -type d` | 49.08 | 5.352 | 0.11x | 202645 | yes |
| `find . -type f` | 51.67 | 5.767 | 0.11x | 412719 | yes |
| `find . -maxdepth 2 -type f` | 8.901 | 0.339 | 0.04x | 47225 | yes |
| `find src -name 'mod_*' -type d` | 22.02 | 1.995 | 0.09x | 69115 | yes |
| `find . -name 'README.md'` | 46.90 | 5.483 | 0.12x | 12 | yes |
| `cat README.md` | 7.274 | 0.008 | 0.00x | 2610 | yes |
| `cat src/main.c` | 6.310 | 0.018 | 0.00x | 25437 | yes |
| `cat -n src/main.c` | 5.868 | 0.078 | 0.01x | 29210 | yes |
| `head -n 20 src/main.c` | 5.653 | 0.024 | 0.00x | 873 | yes |
| `head -n 1 README.md` | 5.562 | 0.013 | 0.00x | 12 | yes |
| `tail -n 20 src/main.c` | 6.002 | 0.041 | 0.01x | 959 | yes |
| `tail -n +100 src/main.c` | 5.982 | 0.030 | 0.00x | 21039 | yes |
| `wc src/main.c` | 6.338 | 0.077 | 0.01x | 29 | yes |
| `wc -l src/main.c` | 6.324 | 0.079 | 0.01x | 15 | yes |
| `wc -l src/*.c` | 13.93 | 13.85 | 0.99x | 19447 | yes |
| `cat src/*.c \| wc -l` | 23.37 | 16.81 | 0.72x | 7 | yes |
| `cat src/main.c \| head -n 50 \| tail -n 10` | 7.254 | 0.061 | 0.01x | 387 | yes |
| `grep -c the src/main.c` | 6.286 | 0.369 | 0.06x | 3 | yes |
| `grep -n TODO src/main.c` | 5.820 | 0.354 | 0.06x | 134 | yes |
| `grep -rn TODO .` | 111.4 | 1212.1 | 10.88x | 752066 | yes |
| `grep -rl TODO .` | 78.15 | 783.7 | 10.03x | 253153 | yes |
| `grep -rc TODO .` | 87.01 | 1223.0 | 14.06x | 436729 | yes |
| `grep -r error .` | 322.3 | 1372.0 | 4.26x | 19771535 | yes |
| `grep -ri fixme .` | 190.4 | 1324.7 | 6.96x | 103681 | yes |
| `grep -rn handle_request src` | 75.76 | 621.3 | 8.20x | 893510 | yes |
| `grep -rn zzz_not_present .` | 146.5 | 1226.5 | 8.37x | 0 | yes |
| `grep -rl zzz_not_present .` | 120.2 | 1211.5 | 10.08x | 0 | yes |
| `grep -rF 'error:' .` | 95.64 | 223.0 | 2.33x | 1387815 | yes |
| `grep -rE 'TODO\|FIXME' .` | 148.8 | 1616.0 | 10.86x | 816124 | yes |
| `grep -rE '[0-9]{4,}' .` | 434.6 | 1192.5 | 2.74x | 40299254 | yes |
| `grep -rn '^#include' src` | 85.44 | 534.4 | 6.25x | 694663 | yes |
| `grep -rE '^(import\|from) ' .` | 146.0 | 1169.9 | 8.01x | 210570 | yes |
| `grep -rv e docs` | 35.11 | 45.94 | 1.31x | 1213635 | yes |
| `grep -rin 'return [a-z_]*;$' src` | 122.7 | 650.0 | 5.30x | 1352470 | yes |
| `grep -rE '(alpha\|beta)_[a-z]+\(' .` | 277.0 | 1977.1 | 7.14x | 3817115 | yes |
| `grep -r '\(TODO\).*\1' .` | 93.92 | 2338.0 | 24.89x | 0 | yes |
| `grep -rl error . \| wc -l` | 82.64 | 84.27 | 1.02x | 6 | yes |
| `grep -rc the . \| grep -v ':0$' \| wc -l` | 218.1 | 1255.9 | 5.76x | 6 | yes |
| `find . -name '*.md' \| wc -l` | 51.54 | 5.898 | 0.11x | 4 | yes |
| `find . -type f \| grep -c mod_` | 49.30 | 9.481 | 0.19x | 5 | yes |
| `sed 's/the/THE/g' src/main.c` | 7.234 | 0.432 | 0.06x | 25437 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/main.c` | 7.615 | 0.873 | 0.11x | 25437 | yes |
| `cat src/main.c \| sed 's/^/> /' \| wc -c` | 7.582 | 0.269 | 0.04x | 6 | yes |
| `mkdir -p bench_out/a/b` | 6.817 | 0.022 | 0.00x | 0 | yes |
| `touch bench_out/empty.txt` | 6.940 | 0.012 | 0.00x | 0 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 5.943 | 0.016 | 0.00x | 0 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 7.223 | 0.015 | 0.00x | 0 | yes |
| `cp src/main.c bench_out/main_copy.c` | 6.883 | 0.039 | 0.01x | 0 | yes |
| `cp -r src bench_out/src_copy` | 127.3 | 37.74 | 0.30x | 0 | yes |
| `cp -r docs bench_out/docs_copy` | 45.07 | 13.67 | 0.30x | 0 | yes |
| `sed -i 's/TODO/DONE/g' bench_out/main_copy.c` | 9.579 | 0.421 | 0.04x | 0 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/src_copy/main.c` | 9.463 | 0.453 | 0.05x | 0 | yes |
| `sed -i 's/the/THE/g' bench_out/docs_copy/guide.md` | 7.918 | 0.356 | 0.05x | 0 | yes |
| `grep -rc DONE bench_out` | 52.55 | 722.8 | 13.75x | 331552 | yes |
| `mv bench_out/src_copy bench_out/src_moved` | 9.072 | 0.032 | 0.00x | 0 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 8.195 | 0.025 | 0.00x | 0 | yes |
| `ls -R bench_out/a` | 7.098 | 0.017 | 0.00x | 41 | yes |
| `rm bench_out/a/b/hello.txt` | 6.990 | 0.017 | 0.00x | 0 | yes |
| `rmdir bench_out/a/b` | 9.092 | 0.016 | 0.00x | 0 | yes |
| `rm -r bench_out/src_moved` | 191.5 | 7.053 | 0.04x | 0 | yes |
| `rm -rf bench_out` | 79.86 | 1.998 | 0.03x | 0 | yes |

20 of 69 commands were slower on the engine than on the real tools.
