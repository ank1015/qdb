# 100000 writes on postgres, run `s3-disk-always`

Seed 1, search index on. The whole run took 44.8 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.193 | 0.333 | 0.815 | 4.00 | 104 |
| **every large write** | 9088 | 0.472 | 5.22 | 36.0 | 158 | 464 |
| append | 39801 | 0.200 | 0.346 | 0.881 | 4.43 | 104 |
| create | 20249 | 0.204 | 0.352 | 0.893 | 4.28 | 25.1 |
| overwrite | 9827 | 0.204 | 0.353 | 0.880 | 3.97 | 8.14 |
| sed | 9994 | 0.043 | 0.268 | 0.550 | 1.24 | 22.3 |
| rm | 5069 | 0.183 | 0.285 | 0.657 | 3.90 | 10.5 |
| mv | 3940 | 0.187 | 0.295 | 0.573 | 3.99 | 10.2 |
| same | 2032 | 0.061 | 0.100 | 0.153 | 0.366 | 0.493 |
| cp file | 5017 | 0.533 | 1.41 | 6.08 | 20.8 | 132 |
| append to a dataset file | 2595 | 0.210 | 0.541 | 1.97 | 5.93 | 9.55 |
| sed over a folder | 479 | 5.30 | 12.1 | 19.6 | 23.1 | 23.1 |
| cp -r | 307 | 9.29 | 59.7 | 342 | 464 | 464 |
| cat into one | 211 | 6.36 | 22.6 | 72.4 | 93.6 | 93.6 |
| sed on a large file | 285 | 13.2 | 30.8 | 73.2 | 99.4 | 99.4 |
| rm -r | 194 | 0.302 | 1.40 | 3.52 | 5.52 | 5.52 |

The ten slowest small writes:

| ms | Write number | Command |
|---:|---:|---|
| 104 | 34762 | `echo 'toousremi guatele nemi tise trous isis' >> stress/d5/m3295.txt` |
| 100 | 90563 | `echo 'semu done neratovu muju paulate muanreto' >> stress/d0/f6123.txt` |
| 26.4 | 77921 | `echo 'gustgu muleate sere toan' >> stress/d5/f16828.txt` |
| 25.7 | 33849 | `echo 'anmifa juchulfa onate solovu trisni paing trgureso' >> stress/d1/m8275.txt` |
| 25.1 | 22155 | `echo 'faniousre chulra atelopa nion trne oustrdoba sestdogu ermu nist kist soing' > stress/d1/f6504.` |
| 23.4 | 19042 | `echo 'anchti doki ateso guneul ousousne famu ateguentvu fase niraragu rane anous lostse jusejuch guf` |
| 22.3 | 12437 | `sed -i 's/soing/tiul/' stress/d7/f1717.txt` |
| 22.2 | 82504 | `echo 'onmukilo pajulele doto ousmimiate losoingki trnitoso semu jugune isanba' > stress/d2/f24281.tx` |
| 21.9 | 19044 | `echo 'ingdoti muju guba chnimiis paentchpa totran sting ateso lotr ateous ratiatese ateousent muju c` |
| 20.3 | 6073 | `echo 'ulnesoent soso mious toer ousanfa ousneentni tistnich seate entan paateti trnitoso nevu pami b` |

On disk, sync always.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 842424488 |
| log of the command lines | 10879441 |
| content moved to free space | 509727305 |
| index files | 153009954 |
| saves of the tree (11) | 11976314 |
| **all** | 1528017502 |
| **write amplification**: all, over content and log the commands caused | 1.79 |

Syncs: 90237 over 90179 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 9823 | 90146 | 8 | 19 | 4 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 180595416 | 161175531 | 1.120 |
| at its worst (after write 34600; looked at every 200 writes) | 508100600 | 289290744 | 1.756 |
| at the end | 612907040 | 430420450 | 1.424 |

At the end, by what it is: text 430420450, index lists 57893496, index tables 5771112, tree 1648460, log 3654978, record and file headers 3993684, space not yet reclaimed 109524860.

Most heap the engine held during the run: 63.4 MB.

Check of the database after the run: ok: 40054 files and 886 folders, 430420450 bytes of text in 50360 pieces, 5 data files with 122577 records, 640824 posting lists compared with a fresh index

Settled afterwards (`--compact`, 4.3 s): 487300202 bytes, 1.132 of the raw text.

Index work over the run: 324865 documents added, 105365 retired, 183221725 bytes of lists rewritten by sweeps, 0 rebuilds.
