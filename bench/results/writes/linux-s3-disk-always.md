# 100000 writes on linux, run `s3-disk-always`

Seed 1, search index on. The whole run took 32.9 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.163 | 0.273 | 0.576 | 2.35 | 147 |
| **every large write** | 9088 | 0.363 | 3.85 | 19.9 | 68.0 | 232 |
| append | 39801 | 0.167 | 0.281 | 0.617 | 2.56 | 147 |
| create | 20249 | 0.170 | 0.284 | 0.595 | 2.27 | 23.9 |
| overwrite | 9827 | 0.173 | 0.291 | 0.628 | 2.15 | 7.22 |
| sed | 9994 | 0.058 | 0.250 | 0.483 | 1.29 | 22.2 |
| rm | 5069 | 0.156 | 0.234 | 0.481 | 1.45 | 5.28 |
| mv | 3940 | 0.158 | 0.240 | 0.473 | 1.85 | 60.2 |
| same | 2032 | 0.089 | 0.140 | 0.192 | 0.541 | 1.64 |
| cp file | 5017 | 0.432 | 0.935 | 2.45 | 10.1 | 51.7 |
| append to a dataset file | 2595 | 0.175 | 0.276 | 0.523 | 2.34 | 3.53 |
| sed over a folder | 479 | 4.37 | 9.24 | 22.4 | 200 | 200 |
| cp -r | 307 | 6.50 | 28.8 | 152 | 232 | 232 |
| cat into one | 211 | 3.91 | 6.75 | 14.9 | 33.6 | 33.6 |
| sed on a large file | 285 | 7.27 | 14.9 | 54.7 | 96.6 | 96.6 |
| rm -r | 194 | 0.283 | 2.07 | 11.8 | 30.4 | 30.4 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 147 | 66401 | `echo 'nitr faseanju vule raous entate reous ingdoti mious do` 1 syncs; bytes: 391 content, 0 of it moved, 0 index files, 0 tree |
| 89.4 | 66426 | `echo 'ststra seso banimuul leki ondoti tistnich judoremi mul` 1 syncs; bytes: 461 content, 0 of it moved, 0 index files, 0 tree |
| 60.2 | 66432 | `mv stress/d4/f8076.txt stress/d6/m19506.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 33.1 | 66350 | `echo 'anousstst semu vulo muju sotrch sech raatesest soate v` 1 syncs; bytes: 166 content, 0 of it moved, 0 index files, 0 tree |
| 24.0 | 79827 | `echo 'gufanetr toenton chateon anstfato ulmire vujuba onate ` 1 syncs; bytes: 830 content, 0 of it moved, 0 index files, 0 tree |
| 23.9 | 63151 | `echo 'atemivulo ulvuul vumu onate onate tofatrous trmuonra' ` 1 syncs; bytes: 93 content, 0 of it moved, 0 index files, 0 tree |
| 22.9 | 89230 | `echo 'ermu ousbare mutier chjuba niate istrsoch kivu lestst ` 1 syncs; bytes: 282 content, 0 of it moved, 0 index files, 0 tree |
| 22.3 | 28564 | `echo 'lorean faba paentrest faniousre toenton chbasoto rabaj` 1 syncs; bytes: 245 content, 0 of it moved, 0 index files, 0 tree |
| 22.2 | 22822 | `sed -i 's/sese/loso/' stress/d5/f2503.txt` 1 syncs; bytes: 405 content, 0 of it moved, 0 index files, 0 tree |
| 21.9 | 79891 | `echo 'lotrfa paso oustr loentous anmutr nimi ateingtrni sese` 1 syncs; bytes: 129 content, 0 of it moved, 0 index files, 0 tree |

On disk, sync always.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 694016759 |
| log of the command lines | 9141898 |
| content moved to free space | 11194726 |
| index files | 50056365 |
| saves of the tree (2) | 9619463 |
| **all** | 774029211 |
| **write amplification**: all, over content and log the commands caused | 1.10 |

Syncs: 90245 over 90178 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 9824 | 90140 | 29 | 4 | 3 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 1548877620 | 1396914445 | 1.109 |
| at its worst (after write 87800; looked at every 200 writes) | 2238683237 | 1560871743 | 1.434 |
| at the end | 2166233913 | 1587633253 | 1.364 |

At the end, by what it is: text 1587633253, index lists 162791146, index tables 13788260, tree 4832721, log 12176245, record and file headers 10973404, space not yet reclaimed 374038884.

Most heap the engine held during the run: 74.8 MB.

Check of the database after the run: ok: 111007 files and 5693 folders, 1587633253 bytes of text in 130353 pieces, 15 data files with 344070 records, 1352855 posting lists compared with a fresh index

Settled afterwards (`--compact`, 19.9 s): 1765758600 bytes, 1.112 of the raw text.

Index work over the run: 333429 documents added, 129894 retired, 239837025 bytes of lists rewritten by sweeps, 0 rebuilds.
