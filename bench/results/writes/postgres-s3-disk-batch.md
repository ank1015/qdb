# 100000 writes on postgres, run `s3-disk-batch`

Seed 1, search index on. The whole run took 22.5 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.012 | 0.044 | 1.16 | 4.44 | 21.0 |
| **every large write** | 9088 | 0.090 | 3.74 | 29.5 | 131 | 416 |
| append | 39801 | 0.011 | 0.036 | 1.19 | 4.27 | 19.3 |
| create | 20249 | 0.012 | 0.036 | 1.26 | 5.37 | 15.8 |
| overwrite | 9827 | 0.013 | 0.038 | 1.15 | 4.72 | 21.0 |
| sed | 9994 | 0.027 | 0.058 | 0.175 | 1.90 | 16.2 |
| rm | 5069 | 0.006 | 0.015 | 1.12 | 5.76 | 8.18 |
| mv | 3940 | 0.009 | 0.018 | 1.34 | 4.65 | 6.21 |
| same | 2032 | 0.041 | 0.074 | 0.121 | 0.368 | 0.593 |
| cp file | 5017 | 0.109 | 0.786 | 4.02 | 16.3 | 47.5 |
| append to a dataset file | 2595 | 0.013 | 0.133 | 1.41 | 5.69 | 8.72 |
| sed over a folder | 479 | 4.29 | 8.84 | 17.7 | 23.2 | 23.2 |
| cp -r | 307 | 6.54 | 54.0 | 287 | 416 | 416 |
| cat into one | 211 | 4.26 | 17.4 | 59.0 | 94.3 | 94.3 |
| sed on a large file | 285 | 9.70 | 24.9 | 74.3 | 110 | 110 |
| rm -r | 194 | 0.053 | 1.06 | 2.50 | 6.00 | 6.00 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 21.0 | 78383 | `echo 'toer guselole entsolose pavuto vumidodo leki semu tist` 1 syncs; bytes: 260 content, 0 of it moved, 0 index files, 0 tree |
| 19.3 | 33849 | `echo 'anmifa juchulfa onate solovu trisni paing trgureso' >>` 2 syncs; bytes: 91 content, 0 of it moved, 13441647 index files, 0 tree |
| 18.3 | 77921 | `echo 'gustgu muleate sere toan' >> stress/d5/f16828.txt` 2 syncs; bytes: 3882 content, 3777 of it moved, 10593945 index files, 0 tree |
| 16.2 | 12437 | `sed -i 's/soing/tiul/' stress/d7/f1717.txt` 2 syncs; bytes: 347 content, 0 of it moved, 7491918 index files, 0 tree |
| 15.8 | 22155 | `echo 'faniousre chulra atelopa nion trne oustrdoba sestdogu ` 2 syncs; bytes: 115 content, 0 of it moved, 11739970 index files, 0 tree |
| 10.5 | 44791 | `echo 'nevuate fanesetr lodopast toenton netrki kiulti sofain` 1 syncs; bytes: 294 content, 0 of it moved, 0 index files, 0 tree |
| 9.15 | 85093 | `echo 'doonra sese sotring muneba kieris tilo soing neiseran ` 1 syncs; bytes: 213 content, 0 of it moved, 0 index files, 0 tree |
| 8.72 | 48381 | `echo 'sotring anousstst pamuch pavumu anstra tikiousdo' >> s` 1 syncs; bytes: 89 content, 0 of it moved, 0 index files, 0 tree |
| 8.18 | 44581 | `rm stress/d0/f11010.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 7.94 | 50493 | `echo 'ingpamitr kitoni ermu isis pavuto toan ateso soki lone` 1 syncs; bytes: 204 content, 0 of it moved, 0 index files, 0 tree |

On disk, sync batch.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 842424488 |
| log of the command lines | 10879441 |
| content moved to free space | 509727305 |
| index files | 153009954 |
| saves of the tree (11) | 11976314 |
| **all** | 1528017502 |
| **write amplification**: all, over content and log the commands caused | 1.79 |

Syncs: 1621 over 90179 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 98492 | 1473 | 13 | 14 | 8 |

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
