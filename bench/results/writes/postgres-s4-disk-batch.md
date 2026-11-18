# 100000 writes on postgres, run `s4-disk-batch`

Seed 1, search index on. The whole run took 13.1 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.014 | 0.046 | 0.990 | 2.84 | 13.9 |
| **every large write** | 9088 | 0.014 | 1.88 | 16.5 | 46.8 | 140 |
| append | 39801 | 0.013 | 0.044 | 0.984 | 3.00 | 8.89 |
| create | 20249 | 0.015 | 0.042 | 0.979 | 3.36 | 13.9 |
| overwrite | 9827 | 0.016 | 0.048 | 0.963 | 2.34 | 5.01 |
| sed | 9994 | 0.025 | 0.063 | 1.02 | 2.50 | 7.24 |
| rm | 5069 | 0.008 | 0.020 | 0.910 | 2.75 | 12.5 |
| mv | 3940 | 0.010 | 0.018 | 0.979 | 2.86 | 5.07 |
| same | 2032 | 0.033 | 0.065 | 1.09 | 2.40 | 4.08 |
| cp file | 5017 | 0.011 | 0.020 | 0.983 | 2.97 | 4.93 |
| append to a dataset file | 2595 | 0.017 | 0.136 | 1.25 | 3.76 | 6.29 |
| sed over a folder | 479 | 4.46 | 11.7 | 19.6 | 26.0 | 26.0 |
| cp -r | 307 | 0.120 | 0.476 | 4.92 | 9.26 | 9.26 |
| cat into one | 211 | 4.06 | 10.5 | 91.7 | 140 | 140 |
| sed on a large file | 285 | 9.71 | 22.9 | 78.9 | 106 | 106 |
| rm -r | 194 | 0.052 | 0.162 | 1.17 | 2.30 | 2.30 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 13.9 | 50853 | `echo 'kiseleer tilo neulent pasevuis muju vusoingba tilo isu` 2 syncs; bytes: 7213 content, 6847 of it moved, 7149471 index files, 0 tree |
| 12.5 | 83918 | `rm stress/d3/f16664.txt` 2 syncs; bytes: 0 content, 0 of it moved, 8756418 index files, 0 tree |
| 8.89 | 67110 | `echo 'toenton anstfato soinggu milo trmiingne faniousre tilo` 3 syncs; bytes: 1438 content, 1287 of it moved, 0 index files, 1540593 tree |
| 8.37 | 66995 | `echo 'kipafa raous palegu mulosoer museate' >> stress/d5/f33` 1 syncs; bytes: 560 content, 0 of it moved, 0 index files, 0 tree |
| 7.64 | 50695 | `echo 'stju ateretrous ateso oningre onate mimi isis toan ses` 2 syncs; bytes: 916 content, 563 of it moved, 0 index files, 0 tree |
| 7.63 | 11198 | `rm stress/d2/f2655.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 7.24 | 36621 | `sed -i 's/mious/rachateul/' stress/d5/f4797.txt` 3 syncs; bytes: 0 content, 0 of it moved, 0 index files, 1130156 tree |
| 6.96 | 67942 | `echo 'miso revuch nemukise dotiguse baous anvuisba chmuso fa` 1 syncs; bytes: 168 content, 0 of it moved, 0 index files, 0 tree |
| 6.76 | 36313 | `echo 'muanti ingti mura trpaate' >> stress/d5/m7067.txt` 1 syncs; bytes: 66 content, 0 of it moved, 0 index files, 0 tree |
| 6.75 | 66683 | `rm stress/d2/f14103.txt` 2 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |

On disk, sync batch.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 299889837 |
| log of the command lines | 19223859 |
| content moved to free space | 156647908 |
| index files | 35020271 |
| saves of the tree (10) | 13411848 |
| **all** | 524193723 |
| **write amplification**: all, over content and log the commands caused | 1.64 |

Syncs: 1634 over 100002 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 98400 | 1580 | 8 | 10 | 2 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 180785915 | 161175531 | 1.122 |
| at its worst (after write 19600; looked at every 200 writes) | 261435250 | 202192750 | 1.293 |
| at the end | 224720318 | 430420450 | 0.522 |

At the end, by what it is: text 168375764, index lists 12846132, index tables 1492364, tree 1969075, log 4553934, record and file headers 2321580, space not yet reclaimed 33161469.

Most heap the engine held during the run: 41.2 MB.

Check of the database after the run: ok: 40054 files and 886 folders, 430420450 bytes of text in 32648 pieces, 2 data files with 73639 records, 273818 posting lists compared with a fresh index

Settled afterwards (`--compact`, 2.2 s): 246026449 bytes, 0.572 of the raw text.

Index work over the run: 263897 documents added, 141778 retired, 200669498 bytes of lists rewritten by sweeps, 0 rebuilds.
