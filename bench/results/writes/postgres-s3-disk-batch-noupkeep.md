# 100000 writes on postgres, run `s3-disk-batch-noupkeep`

Seed 1, search index on. The whole run took 16.4 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.012 | 0.041 | 1.00 | 3.65 | 22.1 |
| **every large write** | 9088 | 0.081 | 3.06 | 18.7 | 61.9 | 396 |
| append | 39801 | 0.011 | 0.036 | 1.09 | 3.83 | 22.1 |
| create | 20249 | 0.012 | 0.033 | 1.09 | 4.10 | 20.5 |
| overwrite | 9827 | 0.013 | 0.037 | 1.02 | 3.68 | 15.3 |
| sed | 9994 | 0.028 | 0.059 | 0.134 | 1.48 | 4.93 |
| rm | 5069 | 0.006 | 0.015 | 1.14 | 3.50 | 6.30 |
| mv | 3940 | 0.008 | 0.015 | 1.23 | 2.96 | 4.81 |
| same | 2032 | 0.042 | 0.078 | 0.119 | 0.743 | 1.05 |
| cp file | 5017 | 0.100 | 0.417 | 2.21 | 12.9 | 22.7 |
| append to a dataset file | 2595 | 0.013 | 0.115 | 0.989 | 2.50 | 4.04 |
| sed over a folder | 479 | 3.65 | 8.28 | 16.6 | 20.3 | 20.3 |
| cp -r | 307 | 6.30 | 30.5 | 323 | 396 | 396 |
| cat into one | 211 | 3.54 | 7.43 | 17.5 | 22.1 | 22.1 |
| sed on a large file | 285 | 8.68 | 18.9 | 32.0 | 59.5 | 59.5 |
| rm -r | 194 | 0.048 | 1.01 | 2.67 | 3.56 | 3.56 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 22.1 | 4131 | `echo 'mious istimi toer dore baonmufa onbapa ratiatese tosob` 1 syncs; bytes: 96 content, 0 of it moved, 0 index files, 0 tree |
| 20.5 | 23514 | `echo 'semu senene stmi chst chent neki faatetini sese tinitr` 1 syncs; bytes: 145 content, 0 of it moved, 0 index files, 0 tree |
| 15.3 | 68992 | `echo 'vugu chbamian guni ermu ateous semu tofatrous tomutr m` 2 syncs; bytes: 162 content, 0 of it moved, 13146449 index files, 0 tree |
| 15.2 | 90832 | `echo 'anousstst fanesetr niousba lebaanvu fase tofatrous tis` 2 syncs; bytes: 100 content, 0 of it moved, 12436521 index files, 0 tree |
| 13.7 | 21557 | `echo 'soatech pavuto jumient raous nechstki sere vugu vujuba` 1 syncs; bytes: 302 content, 0 of it moved, 0 index files, 0 tree |
| 13.4 | 40534 | `echo 'raous jufale tilo pavuto raous vujuba tionis jugu anvu` 2 syncs; bytes: 302 content, 0 of it moved, 14036514 index files, 0 tree |
| 12.6 | 34939 | `echo 'loanchse stison ousantiing nevuate miguanous iserul is` 1 syncs; bytes: 245 content, 0 of it moved, 0 index files, 0 tree |
| 8.15 | 85867 | `echo 'sese anousmupa raous fadoon muju pastlo leto' > stress` 1 syncs; bytes: 85 content, 0 of it moved, 0 index files, 0 tree |
| 8.01 | 36623 | `echo 'loloonso pavuto lole pavuto lodofavu kiatemu kipaleis ` 1 syncs; bytes: 295 content, 0 of it moved, 0 index files, 0 tree |
| 7.91 | 64135 | `echo 'leto balo midore letoju anstgu chateon ermu batris loi` 1 syncs; bytes: 247 content, 0 of it moved, 0 index files, 0 tree |

On disk, sync batch, upkeep off.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 839680688 |
| log of the command lines | 9134535 |
| content moved to free space | 0 |
| index files | 72550242 |
| saves of the tree (5) | 5664594 |
| **all** | 927030059 |
| **write amplification**: all, over content and log the commands caused | 1.09 |

Syncs: 1515 over 90179 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 98530 | 1453 | 7 | 6 | 4 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 180595416 | 161175531 | 1.120 |
| at its worst (after write 91200; looked at every 200 writes) | 1030415642 | 392576057 | 2.625 |
| at the end | 1099411581 | 430420450 | 2.554 |

At the end, by what it is: text 430420450, index lists 77445929, index tables 9452432, tree 1740768, log 9555985, record and file headers 8763068, space not yet reclaimed 562032949.

Most heap the engine held during the run: 49.3 MB.

Check of the database after the run: ok: 40054 files and 886 folders, 430420450 bytes of text in 50360 pieces, 8 data files with 305284 records, 1068989 posting lists compared with a fresh index

Settled afterwards (`--compact`, 4.9 s): 487300202 bytes, 1.132 of the raw text.

Index work over the run: 256270 documents added, 114087 retired, 220970142 bytes of lists rewritten by sweeps, 0 rebuilds.
