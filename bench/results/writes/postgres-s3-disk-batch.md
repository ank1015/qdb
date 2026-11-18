# 100000 writes on postgres, run `s3-disk-batch`

Seed 1, search index on. The whole run took 23.7 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.013 | 0.046 | 1.14 | 4.58 | 45.7 |
| **every large write** | 9088 | 0.097 | 3.95 | 31.0 | 141 | 427 |
| append | 39801 | 0.012 | 0.040 | 1.19 | 4.56 | 45.7 |
| create | 20249 | 0.013 | 0.039 | 1.21 | 5.72 | 24.5 |
| overwrite | 9827 | 0.014 | 0.042 | 1.18 | 4.58 | 7.22 |
| sed | 9994 | 0.029 | 0.062 | 0.175 | 1.99 | 14.9 |
| rm | 5069 | 0.007 | 0.016 | 1.06 | 5.50 | 18.6 |
| mv | 3940 | 0.010 | 0.020 | 1.25 | 4.43 | 20.6 |
| same | 2032 | 0.042 | 0.076 | 0.121 | 0.483 | 0.617 |
| cp file | 5017 | 0.119 | 0.814 | 4.10 | 18.1 | 48.7 |
| append to a dataset file | 2595 | 0.015 | 0.135 | 1.61 | 5.55 | 7.73 |
| sed over a folder | 479 | 4.20 | 9.17 | 15.8 | 25.0 | 25.0 |
| cp -r | 307 | 7.78 | 54.2 | 339 | 427 | 427 |
| cat into one | 211 | 4.41 | 16.5 | 55.1 | 91.1 | 91.1 |
| sed on a large file | 285 | 11.2 | 29.6 | 65.1 | 115 | 115 |
| rm -r | 194 | 0.053 | 1.06 | 3.07 | 6.10 | 6.10 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 45.7 | 13194 | `echo 'fanesetr juvugu atelopa lereent tilo enttido ononateen` 1 syncs; bytes: 106 content, 0 of it moved, 0 index files, 0 tree |
| 40.1 | 78868 | `echo 'doonra tofatrous paent nient ousmu fase tilo sotring f` 1 syncs; bytes: 113 content, 0 of it moved, 0 index files, 0 tree |
| 24.5 | 22155 | `echo 'faniousre chulra atelopa nion trne oustrdoba sestdogu ` 2 syncs; bytes: 115 content, 0 of it moved, 11739970 index files, 0 tree |
| 21.7 | 33849 | `echo 'anmifa juchulfa onate solovu trisni paing trgureso' >>` 2 syncs; bytes: 91 content, 0 of it moved, 13441647 index files, 0 tree |
| 20.6 | 13122 | `mv stress/d6/f3754.txt stress/d5/m3879.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 19.4 | 77921 | `echo 'gustgu muleate sere toan' >> stress/d5/f16828.txt` 2 syncs; bytes: 3882 content, 3777 of it moved, 10593945 index files, 0 tree |
| 18.6 | 22022 | `rm stress/d6/m4598.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 14.9 | 12437 | `sed -i 's/soing/tiul/' stress/d7/f1717.txt` 2 syncs; bytes: 347 content, 0 of it moved, 7491918 index files, 0 tree |
| 9.10 | 89229 | `echo 'fase neing batrmi gulo pavuto trmiingne neentvu raous ` 1 syncs; bytes: 1201 content, 1008 of it moved, 0 index files, 0 tree |
| 9.01 | 85867 | `echo 'sese anousmupa raous fadoon muju pastlo leto' > stress` 1 syncs; bytes: 85 content, 0 of it moved, 0 index files, 0 tree |

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

Settled afterwards (`--compact`, 4.6 s): 487300202 bytes, 1.132 of the raw text.

Index work over the run: 324865 documents added, 105365 retired, 183221725 bytes of lists rewritten by sweeps, 0 rebuilds.
