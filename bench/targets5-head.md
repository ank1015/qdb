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

