# Benchmark: medium dataset

- Dataset: 2005 files in 1421 folders, 13.7 MB of text
- Engine load time (reading the dataset from disk into memory): 18.22 ms
- Real tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0
- Process start-up cost of the real tools (`bash -c 'echo -n'`): 2.206 ms. Every real time below includes it; engine times are measured in-process and include parsing the command.
- Read-only commands: median of 5 runs. Commands that change data: one run.
- Results agree with the real tools: yes, for every command and for the final tree

| Command | Real (ms) | Engine (ms) | Engine / real | Output (bytes) | Agree |
|---|---:|---:|---:|---:|---|
| `ls` | 2.786 | 0.009 | 0.00x | 44 | yes |
| `ls -a` | 2.808 | 0.013 | 0.00x | 60 | yes |
| `ls -R` | 9.896 | 0.760 | 0.08x | 78519 | yes |
| `ls -aR src` | 5.471 | 0.248 | 0.05x | 32595 | yes |
| `ls -p src` | 2.859 | 0.019 | 0.01x | 2306 | yes |
| `find .` | 11.45 | 0.603 | 0.05x | 108581 | yes |
| `find . -name '*.c'` | 11.38 | 0.597 | 0.05x | 29761 | yes |
| `find . -name '*.md'` | 10.72 | 0.558 | 0.05x | 5028 | yes |
| `find . -type d` | 9.972 | 0.448 | 0.04x | 39965 | yes |
| `find . -type f` | 10.39 | 0.487 | 0.05x | 68616 | yes |
| `find . -maxdepth 2 -type f` | 3.337 | 0.057 | 0.02x | 7140 | yes |
| `find src -name 'mod_*' -type d` | 5.787 | 0.207 | 0.04x | 13733 | yes |
| `find . -name 'README.md'` | 10.76 | 0.616 | 0.06x | 12 | yes |
| `cat README.md` | 2.829 | 0.008 | 0.00x | 2225 | yes |
| `cat src/main.c` | 2.692 | 0.014 | 0.01x | 25592 | yes |
| `cat -n src/main.c` | 2.731 | 0.069 | 0.03x | 29435 | yes |
| `head -n 20 src/main.c` | 2.940 | 0.024 | 0.01x | 673 | yes |
| `head -n 1 README.md` | 2.919 | 0.012 | 0.00x | 12 | yes |
| `tail -n 20 src/main.c` | 2.749 | 0.043 | 0.02x | 971 | yes |
| `tail -n +100 src/main.c` | 2.684 | 0.023 | 0.01x | 21347 | yes |
| `wc src/main.c` | 3.295 | 0.073 | 0.02x | 29 | yes |
| `wc -l src/main.c` | 2.788 | 0.103 | 0.04x | 15 | yes |
| `wc -l src/*.c` | 3.862 | 2.967 | 0.77x | 2981 | yes |
| `cat src/*.c \| wc -l` | 5.954 | 2.392 | 0.40x | 6 | yes |
| `cat src/main.c \| head -n 50 \| tail -n 10` | 3.710 | 0.046 | 0.01x | 351 | yes |
| `grep -c the src/main.c` | 3.008 | 0.412 | 0.14x | 3 | yes |
| `grep -n TODO src/main.c` | 2.871 | 0.371 | 0.13x | 82 | yes |
| `grep -rn TODO .` | 21.03 | 179.4 | 8.53x | 118662 | yes |
| `grep -rl TODO .` | 16.29 | 114.7 | 7.04x | 41463 | yes |
| `grep -rc TODO .` | 17.16 | 181.3 | 10.56x | 72626 | yes |
| `grep -r error .` | 48.62 | 203.3 | 4.18x | 3055734 | yes |
| `grep -ri fixme .` | 33.62 | 211.7 | 6.30x | 15017 | yes |
| `grep -rn handle_request src` | 15.86 | 100.7 | 6.35x | 139771 | yes |
| `grep -rn zzz_not_present .` | 30.40 | 188.8 | 6.21x | 0 | yes |
| `grep -rl zzz_not_present .` | 22.80 | 187.0 | 8.20x | 0 | yes |
| `grep -rF 'error:' .` | 18.52 | 33.28 | 1.80x | 207750 | yes |
| `grep -rE 'TODO\|FIXME' .` | 25.91 | 249.7 | 9.64x | 127436 | yes |
| `grep -rE '[0-9]{4,}' .` | 67.46 | 181.8 | 2.69x | 6286208 | yes |
| `grep -rn '^#include' src` | 16.09 | 83.56 | 5.19x | 118286 | yes |
| `grep -rE '^(import\|from) ' .` | 29.05 | 181.1 | 6.24x | 30137 | yes |
| `grep -rv e docs` | 7.591 | 7.913 | 1.04x | 181320 | yes |
| `grep -rin 'return [a-z_]*;$' src` | 21.92 | 100.5 | 4.59x | 208613 | yes |
| `grep -rE '(alpha\|beta)_[a-z]+\(' .` | 48.02 | 312.0 | 6.50x | 577858 | yes |
| `grep -r '\(TODO\).*\1' .` | 18.40 | 351.9 | 19.12x | 0 | yes |
| `grep -rl error . \| wc -l` | 17.18 | 14.08 | 0.82x | 5 | yes |
| `grep -rc the . \| grep -v ':0$' \| wc -l` | 40.63 | 196.2 | 4.83x | 5 | yes |
| `find . -name '*.md' \| wc -l` | 11.60 | 1.074 | 0.09x | 4 | yes |
| `find . -type f \| grep -c mod_` | 12.15 | 1.550 | 0.13x | 5 | yes |
| `sed 's/the/THE/g' src/main.c` | 3.871 | 0.505 | 0.13x | 25592 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/main.c` | 4.996 | 1.064 | 0.21x | 25592 | yes |
| `cat src/main.c \| sed 's/^/> /' \| wc -c` | 3.881 | 0.271 | 0.07x | 6 | yes |
| `mkdir -p bench_out/a/b` | 2.953 | 0.021 | 0.01x | 0 | yes |
| `touch bench_out/empty.txt` | 3.079 | 0.015 | 0.00x | 0 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 2.425 | 0.021 | 0.01x | 0 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 2.407 | 0.015 | 0.01x | 0 | yes |
| `cp src/main.c bench_out/main_copy.c` | 3.911 | 0.037 | 0.01x | 0 | yes |
| `cp -r src bench_out/src_copy` | 33.52 | 5.626 | 0.17x | 0 | yes |
| `cp -r docs bench_out/docs_copy` | 10.96 | 1.712 | 0.16x | 0 | yes |
| `sed -i 's/TODO/DONE/g' bench_out/main_copy.c` | 4.232 | 0.493 | 0.12x | 0 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/src_copy/main.c` | 3.769 | 0.527 | 0.14x | 0 | yes |
| `sed -i 's/the/THE/g' bench_out/docs_copy/guide.md` | 3.736 | 0.183 | 0.05x | 0 | yes |
| `grep -rc DONE bench_out` | 11.41 | 111.9 | 9.81x | 55957 | yes |
| `mv bench_out/src_copy bench_out/src_moved` | 5.325 | 0.028 | 0.01x | 0 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 4.427 | 0.027 | 0.01x | 0 | yes |
| `ls -R bench_out/a` | 3.209 | 0.018 | 0.01x | 41 | yes |
| `rm bench_out/a/b/hello.txt` | 3.193 | 0.016 | 0.01x | 0 | yes |
| `rmdir bench_out/a/b` | 4.496 | 0.016 | 0.00x | 0 | yes |
| `rm -r bench_out/src_moved` | 44.15 | 1.316 | 0.03x | 0 | yes |
| `rm -rf bench_out` | 17.29 | 0.465 | 0.03x | 0 | yes |

19 of 69 commands were slower on the engine than on the real tools.
