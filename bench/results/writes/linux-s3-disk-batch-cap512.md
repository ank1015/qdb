# 100000 writes on linux, run `s3-disk-batch-cap512`

Seed 1, search index on. The whole run took 14.2 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.011 | 0.047 | 0.879 | 2.72 | 11.3 |
| **every large write** | 9088 | 0.072 | 2.73 | 13.6 | 62.1 | 270 |
| append | 39801 | 0.010 | 0.031 | 0.926 | 2.94 | 11.3 |
| create | 20249 | 0.011 | 0.028 | 0.930 | 3.00 | 10.3 |
| overwrite | 9827 | 0.012 | 0.034 | 1.04 | 2.34 | 7.23 |
| sed | 9994 | 0.040 | 0.076 | 0.153 | 1.48 | 7.12 |
| rm | 5069 | 0.006 | 0.015 | 0.885 | 2.00 | 6.60 |
| mv | 3940 | 0.008 | 0.014 | 0.965 | 2.67 | 7.28 |
| same | 2032 | 0.068 | 0.107 | 0.163 | 0.428 | 0.970 |
| cp file | 5017 | 0.098 | 0.356 | 1.43 | 4.55 | 15.4 |
| append to a dataset file | 2595 | 0.017 | 0.033 | 0.984 | 2.63 | 4.68 |
| sed over a folder | 479 | 3.41 | 7.20 | 13.7 | 20.1 | 20.1 |
| cp -r | 307 | 4.94 | 24.9 | 164 | 270 | 270 |
| cat into one | 211 | 2.54 | 5.23 | 9.60 | 45.6 | 45.6 |
| sed on a large file | 285 | 5.38 | 11.8 | 58.6 | 121 | 121 |
| rm -r | 194 | 0.058 | 1.76 | 12.2 | 22.1 | 22.1 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 11.3 | 68688 | `echo 'juch sese ousloondo stniramu isent' >> stress/d0/f1491` 1 syncs; bytes: 75 content, 0 of it moved, 0 index files, 0 tree |
| 11.3 | 60418 | `echo 'mious tofatrous pavuto vugu soousousous stondo chateon` 2 syncs; bytes: 135 content, 0 of it moved, 13417845 index files, 0 tree |
| 10.3 | 88902 | `echo 'toenton judo fanesetr lostanvu loentous ousjust leki g` 1 syncs; bytes: 241 content, 0 of it moved, 0 index files, 0 tree |
| 8.13 | 97978 | `echo 'isul chmuki muse munise rach ison tistnich remini trra` 1 syncs; bytes: 293 content, 0 of it moved, 0 index files, 0 tree |
| 7.42 | 52117 | `echo 'ateulerfa chateon chateon bafafa pavuto tistnich pavut` 1 syncs; bytes: 265 content, 0 of it moved, 0 index files, 0 tree |
| 7.28 | 39733 | `mv stress/d2/f8794.txt stress/d6/m11655.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 7.23 | 87980 | `echo 'ismu doonra nine ergu entledo doch trnitoso vuch ontrl` 3 syncs; bytes: 304 content, 0 of it moved, 0 index files, 4832721 tree |
| 7.12 | 6558 | `sed -i 's/sotring/gutofaki/' stress/d1/f1556.txt` 1 syncs; bytes: 190 content, 0 of it moved, 0 index files, 0 tree |
| 6.95 | 3571 | `sed -i 's/loentous/nevu/' stress/d1/f235.txt` 1 syncs; bytes: 249 content, 0 of it moved, 0 index files, 0 tree |
| 6.92 | 82567 | `echo 'bast leki ousdo dobarani nipaateti loent guvuerra toer` 1 syncs; bytes: 196 content, 0 of it moved, 0 index files, 0 tree |

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

Most heap the engine held during the run: 74.8 MB (limit 512 MB).

Check of the database after the run: ok: 111007 files and 5693 folders, 1587633253 bytes of text in 130353 pieces, 15 data files with 344070 records, 1352855 posting lists compared with a fresh index

Settled afterwards (`--compact`, 19.3 s): 1765758600 bytes, 1.112 of the raw text.

Index work over the run: 333429 documents added, 129894 retired, 239837025 bytes of lists rewritten by sweeps, 0 rebuilds.
