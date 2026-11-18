# 100000 writes on linux, run `s3-disk-batch`

Seed 1, search index on. The whole run took 16.3 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.012 | 0.051 | 0.971 | 3.04 | 90.0 |
| **every large write** | 9088 | 0.080 | 3.25 | 16.1 | 76.8 | 313 |
| append | 39801 | 0.011 | 0.036 | 1.05 | 4.01 | 90.0 |
| create | 20249 | 0.012 | 0.033 | 1.01 | 2.87 | 9.58 |
| overwrite | 9827 | 0.013 | 0.040 | 1.08 | 2.92 | 14.3 |
| sed | 9994 | 0.043 | 0.080 | 0.150 | 1.99 | 8.08 |
| rm | 5069 | 0.006 | 0.016 | 0.898 | 2.28 | 6.70 |
| mv | 3940 | 0.009 | 0.015 | 1.05 | 2.92 | 6.35 |
| same | 2032 | 0.071 | 0.112 | 0.163 | 0.450 | 2.68 |
| cp file | 5017 | 0.109 | 0.438 | 1.82 | 7.28 | 18.3 |
| append to a dataset file | 2595 | 0.017 | 0.033 | 1.23 | 3.16 | 5.73 |
| sed over a folder | 479 | 3.78 | 8.59 | 16.5 | 24.1 | 24.1 |
| cp -r | 307 | 5.93 | 28.1 | 182 | 313 | 313 |
| cat into one | 211 | 3.11 | 7.05 | 11.1 | 42.8 | 42.8 |
| sed on a large file | 285 | 6.11 | 13.0 | 69.9 | 114 | 114 |
| rm -r | 194 | 0.070 | 1.86 | 12.5 | 21.6 | 21.6 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 90.0 | 90471 | `echo 'nevuate fainger sogu soing totran toer tilo ingtrous v` 1 syncs; bytes: 235 content, 0 of it moved, 0 index files, 0 tree |
| 24.4 | 41903 | `echo 'leki anousstst anlechst eranleous vufa atejudore repav` 1 syncs; bytes: 338 content, 0 of it moved, 0 index files, 0 tree |
| 18.8 | 60418 | `echo 'mious tofatrous pavuto vugu soousousous stondo chateon` 2 syncs; bytes: 135 content, 0 of it moved, 13417845 index files, 0 tree |
| 16.4 | 75311 | `echo 'anousstst isdo vujuba stonsttr tiba tomu tofatrous ras` 1 syncs; bytes: 100 content, 0 of it moved, 0 index files, 0 tree |
| 16.0 | 75240 | `echo 'chateon vune vugu ousjuvu' >> stress/d3/f12919.txt` 1 syncs; bytes: 66 content, 0 of it moved, 0 index files, 0 tree |
| 14.3 | 66990 | `echo 'ateulerfa mious juismuul ateulerfa chpachne dois muju ` 1 syncs; bytes: 112 content, 0 of it moved, 0 index files, 0 tree |
| 12.7 | 47575 | `echo 'baloch eratedo atelopa sotring paso kitierse doch reju` 1 syncs; bytes: 341 content, 0 of it moved, 0 index files, 0 tree |
| 12.3 | 47652 | `echo 'kipatrti erni paateanti ermu anousstst oner tistnich t` 1 syncs; bytes: 507 content, 0 of it moved, 0 index files, 0 tree |
| 9.58 | 89253 | `echo 'entre seermi seate atekito bati nese ulontoni soing vu` 1 syncs; bytes: 101 content, 0 of it moved, 0 index files, 0 tree |
| 8.08 | 20693 | `sed -i 's/raous/bavuul/' stress/d0/f4002.txt` 1 syncs; bytes: 234 content, 0 of it moved, 0 index files, 0 tree |

On disk, sync batch.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 694016759 |
| log of the command lines | 9141898 |
| content moved to free space | 11194726 |
| index files | 50056365 |
| saves of the tree (2) | 9619463 |
| **all** | 774029211 |
| **write amplification**: all, over content and log the commands caused | 1.10 |

Syncs: 1490 over 90178 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 98549 | 1440 | 6 | 2 | 3 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 1548877620 | 1396914445 | 1.109 |
| at its worst (after write 87800; looked at every 200 writes) | 2238683237 | 1560871743 | 1.434 |
| at the end | 2166233913 | 1587633253 | 1.364 |

At the end, by what it is: text 1587633253, index lists 162791146, index tables 13788260, tree 4832721, log 12176245, record and file headers 10973404, space not yet reclaimed 374038884.

Most heap the engine held during the run: 74.8 MB.

Check of the database after the run: ok: 111007 files and 5693 folders, 1587633253 bytes of text in 130353 pieces, 15 data files with 344070 records, 1352855 posting lists compared with a fresh index

Settled afterwards (`--compact`, 18.7 s): 1765758600 bytes, 1.112 of the raw text.

Index work over the run: 333429 documents added, 129894 retired, 239837025 bytes of lists rewritten by sweeps, 0 rebuilds.
