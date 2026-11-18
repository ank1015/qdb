# 100000 writes on postgres, run `s3-disk-always`

Seed 1, search index on. The whole run took 40.5 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.170 | 0.305 | 1.19 | 2.45 | 21.9 |
| **every large write** | 9088 | 0.409 | 4.70 | 31.8 | 115 | 422 |
| append | 39801 | 0.176 | 0.319 | 1.25 | 2.55 | 21.9 |
| create | 20249 | 0.181 | 0.325 | 1.25 | 2.37 | 14.3 |
| overwrite | 9827 | 0.182 | 0.327 | 1.21 | 2.54 | 5.40 |
| sed | 9994 | 0.037 | 0.246 | 0.563 | 1.72 | 14.5 |
| rm | 5069 | 0.162 | 0.255 | 1.04 | 2.55 | 6.05 |
| mv | 3940 | 0.164 | 0.259 | 1.25 | 1.68 | 4.41 |
| same | 2032 | 0.052 | 0.083 | 0.130 | 0.537 | 0.669 |
| cp file | 5017 | 0.462 | 1.35 | 4.72 | 21.6 | 42.1 |
| append to a dataset file | 2595 | 0.182 | 0.510 | 1.69 | 3.18 | 5.45 |
| sed over a folder | 479 | 4.83 | 10.2 | 18.9 | 28.6 | 28.6 |
| cp -r | 307 | 8.74 | 51.5 | 288 | 422 | 422 |
| cat into one | 211 | 6.03 | 21.1 | 56.6 | 98.7 | 98.7 |
| sed on a large file | 285 | 11.4 | 28.7 | 69.9 | 126 | 126 |
| rm -r | 194 | 0.252 | 1.48 | 3.86 | 5.98 | 5.98 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 21.9 | 55262 | `echo 'seulnelo loto ateulerfa vustpaso minira tolois ouson n` 1 syncs; bytes: 99 content, 0 of it moved, 0 index files, 0 tree |
| 16.3 | 67176 | `echo 'toenton domuso rele paso sese sese muon' >> stress/d5/` 1 syncs; bytes: 80 content, 0 of it moved, 0 index files, 0 tree |
| 14.5 | 12437 | `sed -i 's/soing/tiul/' stress/d7/f1717.txt` 2 syncs; bytes: 347 content, 0 of it moved, 7491918 index files, 0 tree |
| 14.3 | 22155 | `echo 'faniousre chulra atelopa nion trne oustrdoba sestdogu ` 2 syncs; bytes: 115 content, 0 of it moved, 11739970 index files, 0 tree |
| 13.8 | 33849 | `echo 'anmifa juchulfa onate solovu trisni paing trgureso' >>` 2 syncs; bytes: 91 content, 0 of it moved, 13441647 index files, 0 tree |
| 12.9 | 77921 | `echo 'gustgu muleate sere toan' >> stress/d5/f16828.txt` 2 syncs; bytes: 3882 content, 3777 of it moved, 10593945 index files, 0 tree |
| 8.42 | 72317 | `echo 'vukineis muanju dofapaan reateto somi chistine ullo at` 1 syncs; bytes: 130 content, 0 of it moved, 0 index files, 0 tree |
| 6.05 | 22939 | `rm stress/d6/f5595.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 6.05 | 90563 | `echo 'semu done neratovu muju paulate muanreto' >> stress/d0` 3 syncs; bytes: 81 content, 0 of it moved, 0 index files, 1648460 tree |
| 5.40 | 44197 | `echo 'fanesetr trfaneki tivuentso antron tofatrous' > stress` 1 syncs; bytes: 35034 content, 34909 of it moved, 0 index files, 0 tree |

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

Syncs: 90361 over 90179 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 9823 | 90085 | 63 | 18 | 11 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 180595416 | 161175531 | 1.120 |
| at its worst (after write 34600; looked at every 200 writes) | 508100600 | 289290744 | 1.756 |
| at the end | 612907040 | 430420450 | 1.424 |

At the end, by what it is: text 430420450, index lists 57893496, index tables 5771112, tree 1648460, log 3654978, record and file headers 3993684, space not yet reclaimed 109524860.

Most heap the engine held during the run: 63.4 MB.

Check of the database after the run: ok: 40054 files and 886 folders, 430420450 bytes of text in 50360 pieces, 5 data files with 122577 records, 640824 posting lists compared with a fresh index

Settled afterwards (`--compact`, 4.5 s): 487300202 bytes, 1.132 of the raw text.

Index work over the run: 324865 documents added, 105365 retired, 183221725 bytes of lists rewritten by sweeps, 0 rebuilds.
