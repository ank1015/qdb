# Benchmark: small dataset

- Dataset: 205 files in 134 folders, 1.1 MB of text
- Engine load time (reading the dataset from disk into memory): 1.479 ms
- Real tools: GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu); grep (GNU grep) 3.11; sed (GNU sed) 4.9; ls (GNU coreutils) 9.4; find (GNU findutils) 4.9.0
- Process start-up cost of the real tools (`bash -c 'echo -n'`): 1.436 ms. Every real time below includes it; engine times are measured in-process and include parsing the command.
- Read-only commands: median of 5 runs. Commands that change data: one run.
- Results agree with the real tools: yes, for every command and for the final tree

| Command | Real (ms) | Engine (ms) | Engine / real | Output (bytes) | Agree |
|---|---:|---:|---:|---:|---|
| `ls` | 2.202 | 0.014 | 0.01x | 44 | yes |
| `ls -a` | 2.438 | 0.013 | 0.01x | 60 | yes |
| `ls -R` | 2.739 | 0.066 | 0.02x | 7132 | yes |
| `ls -aR src` | 2.245 | 0.034 | 0.02x | 2363 | yes |
| `ls -p src` | 2.110 | 0.012 | 0.01x | 228 | yes |
| `find .` | 2.870 | 0.042 | 0.01x | 9916 | yes |
| `find . -name '*.c'` | 2.853 | 0.048 | 0.02x | 2989 | yes |
| `find . -name '*.md'` | 2.936 | 0.060 | 0.02x | 551 | yes |
| `find . -type d` | 2.795 | 0.052 | 0.02x | 3477 | yes |
| `find . -type f` | 2.899 | 0.050 | 0.02x | 6439 | yes |
| `find . -maxdepth 2 -type f` | 2.254 | 0.021 | 0.01x | 913 | yes |
| `find src -name 'mod_*' -type d` | 2.378 | 0.031 | 0.01x | 875 | yes |
| `find . -name 'README.md'` | 2.949 | 0.047 | 0.02x | 12 | yes |
| `cat README.md` | 1.965 | 0.007 | 0.00x | 2898 | yes |
| `cat src/main.c` | 2.145 | 0.010 | 0.00x | 11666 | yes |
| `cat -n src/main.c` | 2.101 | 0.039 | 0.02x | 13409 | yes |
| `head -n 20 src/main.c` | 1.929 | 0.016 | 0.01x | 956 | yes |
| `head -n 1 README.md` | 1.980 | 0.013 | 0.01x | 11 | yes |
| `tail -n 20 src/main.c` | 2.008 | 0.022 | 0.01x | 940 | yes |
| `tail -n +100 src/main.c` | 1.962 | 0.019 | 0.01x | 6930 | yes |
| `wc src/main.c` | 2.167 | 0.044 | 0.02x | 29 | yes |
| `wc -l src/main.c` | 2.074 | 0.042 | 0.02x | 15 | yes |
| `wc -l src/*.c` | 2.211 | 0.252 | 0.11x | 383 | yes |
| `cat src/*.c \| wc -l` | 2.822 | 0.304 | 0.11x | 5 | yes |
| `cat src/main.c \| head -n 50 \| tail -n 10` | 2.780 | 0.033 | 0.01x | 483 | yes |
| `grep -c the src/main.c` | 2.225 | 0.182 | 0.08x | 3 | yes |
| `grep -n TODO src/main.c` | 2.153 | 0.172 | 0.08x | 49 | yes |
| `grep -rn TODO .` | 3.930 | 14.59 | 3.71x | 8770 | yes |
| `grep -rl TODO .` | 3.500 | 10.75 | 3.07x | 3135 | yes |
| `grep -rc TODO .` | 3.246 | 14.51 | 4.47x | 6849 | yes |
| `grep -r error .` | 5.876 | 16.73 | 2.85x | 233507 | yes |
| `grep -ri fixme .` | 4.510 | 16.04 | 3.56x | 864 | yes |
| `grep -rn handle_request src` | 2.872 | 6.675 | 2.32x | 10334 | yes |
| `grep -rn zzz_not_present .` | 3.969 | 15.81 | 3.98x | 0 | yes |
| `grep -rl zzz_not_present .` | 3.972 | 15.03 | 3.78x | 0 | yes |
| `grep -rF 'error:' .` | 3.582 | 2.810 | 0.78x | 16200 | yes |
| `grep -rE 'TODO\|FIXME' .` | 4.473 | 21.53 | 4.81x | 9181 | yes |
| `grep -rE '[0-9]{4,}' .` | 6.918 | 13.84 | 2.00x | 494798 | yes |
| `grep -rn '^#include' src` | 2.994 | 5.716 | 1.91x | 10704 | yes |
| `grep -rE '^(import\|from) ' .` | 4.611 | 14.55 | 3.16x | 3097 | yes |
| `grep -rv e docs` | 2.909 | 0.800 | 0.27x | 18945 | yes |
| `grep -rin 'return [a-z_]*;$' src` | 3.805 | 7.888 | 2.07x | 15090 | yes |
| `grep -rE '(alpha\|beta)_[a-z]+\(' .` | 5.952 | 24.88 | 4.18x | 41428 | yes |
| `grep -r '\(TODO\).*\1' .` | 3.701 | 31.64 | 8.55x | 0 | yes |
| `grep -rl error . \| wc -l` | 3.879 | 1.514 | 0.39x | 4 | yes |
| `grep -rc the . \| grep -v ':0$' \| wc -l` | 5.756 | 16.51 | 2.87x | 4 | yes |
| `find . -name '*.md' \| wc -l` | 3.950 | 0.070 | 0.02x | 3 | yes |
| `find . -type f \| grep -c mod_` | 4.036 | 0.127 | 0.03x | 4 | yes |
| `sed 's/the/THE/g' src/main.c` | 2.544 | 0.212 | 0.08x | 11666 | yes |
| `sed -E 's/([a-z]+)_([a-z]+)/\2_\1/g' src/main.c` | 3.253 | 0.432 | 0.13x | 11666 | yes |
| `cat src/main.c \| sed 's/^/> /' \| wc -c` | 2.927 | 0.131 | 0.04x | 6 | yes |
| `mkdir -p bench_out/a/b` | 2.789 | 0.016 | 0.01x | 0 | yes |
| `touch bench_out/empty.txt` | 2.217 | 0.010 | 0.00x | 0 | yes |
| `echo 'hello world' > bench_out/hello.txt` | 1.323 | 0.012 | 0.01x | 0 | yes |
| `echo 'another line' >> bench_out/hello.txt` | 1.283 | 0.011 | 0.01x | 0 | yes |
| `cp src/main.c bench_out/main_copy.c` | 2.136 | 0.029 | 0.01x | 0 | yes |
| `cp -r src bench_out/src_copy` | 6.550 | 0.409 | 0.06x | 0 | yes |
| `cp -r docs bench_out/docs_copy` | 4.028 | 0.181 | 0.04x | 0 | yes |
| `sed -i 's/TODO/DONE/g' bench_out/main_copy.c` | 2.333 | 0.220 | 0.09x | 0 | yes |
| `sed -i -E 's/[0-9]+/N/g' bench_out/src_copy/main.c` | 2.230 | 0.227 | 0.10x | 0 | yes |
| `sed -i 's/the/THE/g' bench_out/docs_copy/guide.md` | 2.264 | 0.200 | 0.09x | 0 | yes |
| `grep -rc DONE bench_out` | 2.773 | 8.217 | 2.96x | 5176 | yes |
| `mv bench_out/src_copy bench_out/src_moved` | 2.105 | 0.018 | 0.01x | 0 | yes |
| `mv bench_out/hello.txt bench_out/a/b` | 2.186 | 0.018 | 0.01x | 0 | yes |
| `ls -R bench_out/a` | 2.049 | 0.013 | 0.01x | 41 | yes |
| `rm bench_out/a/b/hello.txt` | 1.954 | 0.014 | 0.01x | 0 | yes |
| `rmdir bench_out/a/b` | 2.140 | 0.012 | 0.01x | 0 | yes |
| `rm -r bench_out/src_moved` | 4.729 | 0.121 | 0.03x | 0 | yes |
| `rm -rf bench_out` | 3.413 | 0.072 | 0.02x | 0 | yes |

17 of 69 commands were slower on the engine than on the real tools.
