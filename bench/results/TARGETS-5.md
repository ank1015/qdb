# Stage 5 targets, measured

Unless a row says otherwise: the Linux tree (83,405 files, 1,396.9 MB),
on disk, settled, sync always, one thread, file cache warm. Real tools
were timed on one thread and their time includes starting bash and the
tool. The figures behind each row are in the second half of this file,
and `docs/HANDBACK-5.md` discusses them. Part 5 (apply_patch) was
skipped at the owner's request, so its two rows have no measurement.

| Part | Target | Measured | Result |
|---|---|---|---|
| All | Harness passes in memory, on disk and reopened after every step | 5,706 of 5,706 in each | met |
| All | No item of sections 4 to 9 with zero cases | 509 rows, none at zero (apply_patch has no rows) | met |
| All | Generated tests: scripts, diff, awk; 10,000 on each of 3 seeds | 90,000 cases (seeds 41, 42, 43), 0 failed, 0 listed as known differences | met |
| All | Stage 4 benchmark commands against their stored counters | Of 182 units, none reads, writes or syncs more, except the last `rm -rf bench_out`: 9 more bytes of records, because the list now leaves four more entries in that folder | met, one listed |
| 1 | The same units twice on the same database | 224 units, 0 with different counters (part of every harness run) | met |
| 1 | Cost of counting | No log bytes and no syncs: the counts are kept in memory only. Time: 1.000 of the time without counting for commands of 1 ms or more (median of 92, largest 1.095); 0.66 microseconds a unit for the shortest | met |
| 1 | `xargs` and `rg` fed 1 GB on standard input, inside 512 MB | Complete, same output; most heap 18 MB | met |
| 1 | `grep -o '[a-z]*ing'` over the 1 GB file at least as fast as GNU grep | 8.9 s against 11.2 s | met |
| 2, 3 | `for i in $(seq 100000); do :; done` faster than bash | 38.5 ms against 99.2 ms | met |
| 3 | A loop of 10,000 small writes in one unit, always: one sync, faster than bash on real files | 1 sync; 34.2 ms against 60.1 ms | met |
| 3 | A unit that loops forever | `while :; do :; done` stopped after 1,000,000 commands in 0.23 s with status 3 and a message; the database passes `--check` and takes further units | met |
| 4 | `diff -u` of two versions of an ordinary file at least as fast as GNU diff | 0.093 ms against 3.03 ms (20 KB file, three changes) | met |
| 4 | `diff -rq Documentation COPY` after `cp -r` and one edit, under 10 ms, text read follows the changed piece | 3.44 ms; 115 bytes of text read in 2 pieces for 9,699 pairs of files | met |
| 4 | `diff` of the 1 GB file against a copy with one line changed, inside 512 MB, under 50 ms | 2.21 ms (0.63 ms with `-u`); most heap 0.5 MB | met |
| 5 | `apply_patch` over 100 files, always | not built | skipped at the owner's request |
| 5 | `apply_patch` changing one line of the 1 GB file | not built | skipped at the owner's request |
| 6 | `awk '{print $2}'`, `awk -F: '{s+=$3} END {print s}'`, `awk '/RE/ {n++} END {print n}'` over the 1 GB file at least as fast as gawk, inside 512 MB | 1.13 s against 3.14 s; 0.79 s against 2.41 s; 2.01 s against 4.71 s; most heap 0.2 MB | met |

## Measured, from the stored runs

Written by `bench/targets5.py`. The Linux tree, on disk, settled, sync always, one thread, the file cache warm, run `s5-disk`, unless a table says otherwise. Real tools include starting bash and the tool; the engine is timed in process. The count columns are the counters of part 1 for the first run of the command; they repeat exactly.

### Loops, variables and `if`

| Command | bash, ms | Engine, ms | Times faster | Text read, bytes | Tree entries | Content written | Records written | Syncs | Same output |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `for i in $(seq 100000); do :; done` | 99.2 | 38.5 | 2.6 | 0 | 0 | 0 | 0 | 0 | yes |
| `i=0; while [ $i -lt 10000 ]; do i=$((i+1)); done; echo $i` | 37.1 | 29.8 | 1.2 | 0 | 390,039 | 0 | 0 | 0 | yes |
| `i=10; until [ $i -eq 0 ]; do i=$((i-1)); done; echo $i` | 2.20 | 0.031 | 70.0 | 0 | 429 | 0 | 0 | 0 | yes |
| `for f in include/linux/*.h; do echo "$f"; done \| wc -l` | 19.6 | 2.09 | 9.4 | 0 | 1,504 | 0 | 0 | 0 | yes |
| `for f in include/linux/*.h; do grep -c define "$f"; done \| sort -n \| tail -n 3` | 1,892 | 17.2 | 110.1 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `n=0; for f in include/linux/*.h; do if grep -q conditions "$f"; then n=$((n+1)); fi; done; echo $n` | 1,883 | 25.1 | 75.1 | 10,736,325 | 10,096 | 0 | 0 | 0 | yes |
| `for f in include/linux/*.h; do if [ -s "$f" ]; then continue; fi; echo "$f"; done` | 9.84 | 2.93 | 3.4 | 0 | 61,648 | 0 | 0 | 0 | yes |
| `H=$PWD; for d in Documentation drivers/usb include/linux; do cd "$d"; ls \| wc -l; cd "$H"; done` | 8.36 | 0.063 | 132.1 | 0 | 1,623 | 0 | 0 | 0 | yes |
| `X=define; N=$(grep -c "$X" include/linux/kernfs.h); echo "$X: $((N * 2)) ${#X} ${N:-none}"` | 3.71 | 0.021 | 174.0 | 19,937 | 6 | 0 | 0 | 0 | yes |
| `find drivers/usb -name '*.h' \| while read -r f; do wc -l < "$f"; done \| sort -n \| tail -n 1` | 293 | 0.735 | 398.4 | 1,878,603 | 1,877 | 0 | 0 | 0 | yes |
| `for f in include/linux/*.h; do head -n 1 "$f"; done > /dev/null` | 1,598 | 2.54 | 628.5 | 10,104,438 | 5,800 | 0 | 0 | 0 | yes |
| `for i in 1 2 3; do for j in a b; do if [ $i -eq 2 ]; then break; fi; echo $i$j; done; done` | 1.80 | 0.018 | 98.3 | 0 | 195 | 0 | 0 | 0 | yes |
| `for i in $(seq 10000); do echo "line $i" >> bench_out/loop.txt; done` | 60.1 | 34.2 | 1.8 | 1,580,743 | 20,001 | 1,239,829 | 254,400 | 1 | yes |
| `for f in bench_out/work/*.h; do echo done >> "$f"; done` | 17.2 | 4.98 | 3.4 | 9,606,666 | 5,800 | 64,440 | 54,444 | 1 | yes |

### awk, against GNU awk 5.2.1

| Command | gawk, ms | Engine, ms | Times faster | Text read, bytes | Tree entries | Content written | Records written | Syncs | Same output |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `awk '{ print $2 }' include/linux/kernfs.h` | 3.17 | 0.041 | 78.3 | 19,937 | 3 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ print $2 }' \| wc -l` | 113 | 30.2 | 3.7 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '/define/ { n++ } END { print n }'` | 76.1 | 21.4 | 3.6 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ s += NF } END { print s, NR }'` | 109 | 41.8 | 2.6 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk -F: '{ s += $3 } END { print s }'` | 69.4 | 23.1 | 3.0 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'FNR == 1 { print FILENAME }' \| wc -l` | 62.9 | 18.7 | 3.4 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'length($0) > 100 { n++ } END { print n + 0 }'` | 63.6 | 19.3 | 3.3 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ c[$1]++ } END { for (k in c) n++; print n }'` | 104 | 50.2 | 2.1 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk '{ for (i = 1; i <= NF; i++) if ($i == "define") n++ } END { print n }'` | 255 | 152 | 1.7 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `awk '{ gsub(/[0-9]+/, "N"); print }' include/linux/kernfs.h \| wc -c` | 4.88 | 0.317 | 15.4 | 19,937 | 3 | 0 | 0 | 0 | yes |
| `awk -v OFS=- '{ $1 = NR; print }' include/linux/kernfs.h \| tail -n 1` | 4.19 | 0.219 | 19.1 | 19,937 | 3 | 0 | 0 | 0 | yes |
| `awk 'BEGIN { for (i = 0; i < 100000; i++) s += i * i; printf "%d\n", s }'` | 9.93 | 5.44 | 1.8 | 0 | 0 | 0 | 0 | 0 | yes |
| `grep -rn separately . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5` | 1,075 | 9.09 | 118.3 | 81,868,155 | 821 | 0 | 0 | 0 | yes |
| `cat include/linux/*.h \| tr -cd '[:print:][:space:]' \| awk 'NF > 3 { print $1, $NF }' \| sort \| uniq -c \| sort -rn \| head -n 5` | 183 | 88.2 | 2.1 | 10,767,883 | 10,096 | 0 | 0 | 0 | yes |
| `awk '{ print NR ": " $0 }' include/linux/kernfs.h > bench_out/numbered.txt` | 5.66 | 1.04 | 5.4 | 19,937 | 6 | 23,034 | 104 | 1 | yes |
| `awk 'END { print NR }' bench_out/numbered.txt` | 4.56 | 0.060 | 75.6 | 22,994 | 2 | 0 | 0 | 0 | yes |

### diff, against GNU diff 3.10

| Command | GNU, ms | Engine, ms | Times faster | Text read, bytes | Tree entries | Content written | Records written | Syncs | Same output |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `diff -u --label old --label new include/linux/kernfs.h bench_out/v2.txt` | 3.03 | 0.093 | 32.6 | 257,926 | 5 | 0 | 0 | 0 | yes |
| `diff include/linux/kernfs.h bench_out/v2.txt \| wc -l` | 2.86 | 0.078 | 36.9 | 198,366 | 5 | 0 | 0 | 0 | yes |
| `diff -q include/linux/kernfs.h bench_out/same.txt` | 3.14 | 0.016 | 197.4 | 0 | 5 | 0 | 0 | 0 | yes |
| `diff -u include/linux/kernfs.h bench_out/copy.txt \| wc -l` | 3.07 | 0.280 | 10.9 | 239,036 | 5 | 0 | 0 | 0 | yes |
| `diff -rq Documentation bench_out/dir_copy` | 80.8 | 3.44 | 23.5 | 115 | 20,751 | 0 | 0 | 0 | yes |
| `sed -i '1s/^/one edit /' $(find bench_out/dir_copy -type f \| sort \| head -n 1)` | 16.4 | 4.65 | 3.5 | 53 | 10,379 | 102 | 78 | 1 | yes |
| `diff -r include/linux bench_out/work \| wc -l` | 82.8 | 73.2 | 1.1 | 90,403,998 | 5,544 | 0 | 0 | 0 | yes |
| `diff -ru include/linux bench_out/work \| grep -c '^@@'` | 110 | 83.2 | 1.3 | 119,026,792 | 5,544 | 0 | 0 | 0 | yes |

Distinct commands of the whole list (222) slower than the real tool: 1: `find drivers/usb -name '*.h' \| sort \| xargs cat > bench_out/joined.txt` 17.5 ms against 7.01 ms.

Commands whose output differs from the real tool's: 0. The final trees agree: true. In memory (`s5-mem`): all agree true. Under the 512 MB limit (`s5-disk-cap512`): all agree true, final trees agree true.

### Stage 4 commands, stage 4 run against stage 5 run

181 commands are in both runs. Median of stage 5 time over stage 4 time: 1.02. The runs were made on different days on a shared machine. Slower by more than a quarter and by more than 0.05 ms:

| Command | Stage 4, ms | Stage 5, ms | Ratio |
|---|---:|---:|---:|
| `nl -ba include/linux/kernfs.h \| tail -n 3` | 0.046 | 0.123 | 2.68 |
| `nl include/linux/kernfs.h` | 0.043 | 0.101 | 2.36 |
| `cut -d ' ' -f 1 include/linux/kernfs.h \| sort \| uniq \| wc -l` | 0.066 | 0.123 | 1.85 |
| `rg -j 1 qdbzzabsentterm` | 0.068 | 0.122 | 1.80 |
| `grep -rl define .` | 59.4 | 89.8 | 1.51 |
| `grep -rn define .` | 454 | 657 | 1.45 |
| `rg -j 1 conditions` | 113 | 149 | 1.31 |
| `rg -j 1 define` | 510 | 661 | 1.30 |

### The cost of counting

`qdb-bench --paired-counting`: each of the 173 read-only commands timed on the engine alone with the counting on and off by turns in one process (the paired method of stage 2), on the database on disk. Median of time with counting over time without: 1.006. All the commands together: 6,201 ms with, 6,256 ms without (0.991).

- The 92 commands that take 1 ms or more: median 1.000, the largest 1.095.
- The 34 commands under 0.01 ms: median 1.20; the difference is 0.66 microseconds a unit, which is two readings of each clock and a mark of the memory count.

### One 1 GB file, inside 512 MB

Sync always, run `s5-disk-always-cap512`. The commands new in stage 5, and the three of part 1:

| Command | Real, ms | Engine, ms | Times faster | Engine's most heap, MB | Bytes written | Syncs | Same output |
|---|---:|---:|---:|---:|---:|---:|---|
| `grep -o '[a-z]*ing' big.txt \| sort --parallel=1 \| uniq -c \| sort -rn \| head` | 13,713 | 10,468 | 1.3 | 136.4 | 0 | 0 | yes |
| `grep -o '[a-z]*ing' big.txt \| wc -l` | 11,155 | 8,900 | 1.3 | 0.0 | 0 | 0 | yes |
| `cat big.txt \| xargs echo \| wc -c` | 39,116 | 38,946 | 1.0 | 18.3 | 0 | 0 | yes |
| `xargs -s 100000 echo < big.txt \| tail -c 40` | 38,391 | 39,379 | 1.0 | 18.0 | 0 | 0 | yes |
| `rg -j 1 -c ing < big.txt` | 896 | 1,173 | 0.8 | 17.3 | 0 | 0 | yes |
| `cat big.txt \| rg -j 1 -n qdbrareterm` | 825 | 958 | 0.9 | 16.8 | 0 | 0 | yes |
| `grep -c ing - < big.txt` | 1,324 | 905 | 1.5 | 17.3 | 0 | 0 | yes |
| `cat big.txt \| sort --parallel=1 - \| tail -n 1` | 6,532 | 2,875 | 2.3 | 130.1 | 0 | 0 | yes |
| `awk '{print $2}' big.txt \| tail -n 2` | 3,144 | 1,131 | 2.8 | 0.2 | 0 | 0 | yes |
| `awk -F: '{s+=$3} END {print s}' big.txt` | 2,409 | 789 | 3.1 | 0.0 | 0 | 0 | yes |
| `awk '{s+=$1} END {print s}' big.txt` | 2,687 | 854 | 3.1 | 0.0 | 0 | 0 | yes |
| `awk '/[a-z]+ing / {n++} END {print n}' big.txt` | 4,715 | 2,007 | 2.3 | 0.0 | 0 | 0 | yes |
| `awk '{n[length($2)]++} END {for (k in n) print k, n[k]}' big.txt \| sort -n \| tail -n 3` | 2,783 | 2,256 | 1.2 | 0.1 | 0 | 0 | yes |
| `diff big.txt copy.txt` | 2,402 | 2.21 | 1,088.2 | 0.5 | 0 | 0 | yes |
| `sed -i '100000s/$/ and one changed line/' copy.txt` | 2,832 | 758 | 3.7 | 0.7 | 65,648 | 1 | yes |
| `diff -u big.txt copy.txt \| wc -l` | 1,922 | 0.634 | 3,032.0 | 0.5 | 0 | 0 | yes |
| `diff -q big.txt copy.txt` | 2.58 | 0.185 | 13.9 | 0.5 | 0 | 0 | yes |

Of all 46 commands of that run, slower than the real tool: `xargs -s 100000 echo < big.txt \| tail -c 40` 39,379 against 38,391; `rg -j 1 -c ing < big.txt` 1,173 against 896; `cat big.txt \| rg -j 1 -n qdbrareterm` 958 against 825; `cat big.txt > cat.txt` 8,974 against 989.

Under batch (`s5-disk-batch-cap512`) every result agrees: True; slower than the real tool there: `rg -j 1 -c ing < big.txt` 964 against 795; `cat big.txt > cat.txt` 8,541 against 936.

### The other datasets

| Dataset | Distinct commands | Output differs | Slower than the real tool |
|---|---:|---|---|
| redis | 222 | none | `mkdir -p bench_out/a/b` 6.15 against 2.88 |
| postgres | 222 | `awk '{ print $2 }' doc/src/sgml/html/bug-reporting.html`; `awk '{ gsub(/[0-9]+/, "N"); print }' doc/src/sgml/html/bug-reporting.html \| wc -c`; `awk -v OFS=- '{ $1 = NR; print }' doc/src/sgml/html/bug-reporting.html \| tail -n 1`; `grep -rn undesirable . \| awk -F: '{ print $1 }' \| uniq -c \| sort -rn \| head -n 5`; `awk '{ print NR ": " $0 }' doc/src/sgml/html/bug-reporting.html > bench_out/numbered.txt`; `awk 'END { print NR }' bench_out/numbered.txt` | `sed -i 's/not/QDBBENCHMANY/g' bench_out/work/*.html` 247 against 193; `for f in bench_out/work/*.html; do echo done >> "$f"; done` 155 against 10.8 |
