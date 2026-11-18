# 100000 writes on postgres, run `s3-disk-always`

Seed 1, search index on. The whole run took 44.9 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.186 | 0.334 | 1.35 | 4.00 | 117 |
| **every large write** | 9088 | 0.439 | 5.18 | 33.6 | 123 | 406 |
| append | 39801 | 0.194 | 0.346 | 1.40 | 4.01 | 117 |
| create | 20249 | 0.197 | 0.354 | 1.46 | 4.01 | 69.0 |
| overwrite | 9827 | 0.197 | 0.356 | 1.34 | 3.98 | 6.86 |
| sed | 9994 | 0.044 | 0.263 | 0.669 | 2.47 | 13.1 |
| rm | 5069 | 0.177 | 0.288 | 1.19 | 3.98 | 6.59 |
| mv | 3940 | 0.182 | 0.302 | 1.30 | 4.00 | 4.35 |
| same | 2032 | 0.061 | 0.104 | 0.145 | 0.382 | 0.755 |
| cp file | 5017 | 0.493 | 1.50 | 5.46 | 24.1 | 43.3 |
| append to a dataset file | 2595 | 0.202 | 0.540 | 1.99 | 5.34 | 7.95 |
| sed over a folder | 479 | 5.37 | 12.0 | 19.2 | 31.8 | 31.8 |
| cp -r | 307 | 10.0 | 55.5 | 282 | 406 | 406 |
| cat into one | 211 | 5.99 | 22.8 | 64.4 | 103 | 103 |
| sed on a large file | 285 | 12.4 | 32.0 | 93.6 | 145 | 145 |
| rm -r | 194 | 0.273 | 1.45 | 4.41 | 5.93 | 5.93 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 117 | 92044 | `echo 'sech vusoingba erbaraba faneti mutier' >> stress/d2/f2` 1 syncs; bytes: 78 content, 0 of it moved, 0 index files, 0 tree |
| 69.0 | 92047 | `echo 'lora tilo fanesetr anstfato ullotira anousstst muju fa` 1 syncs; bytes: 291 content, 0 of it moved, 0 index files, 0 tree |
| 65.4 | 92053 | `echo 'andomu ispa tistnich isis chateon ravuousous' > stress` 1 syncs; bytes: 85 content, 0 of it moved, 0 index files, 0 tree |
| 23.8 | 33849 | `echo 'anmifa juchulfa onate solovu trisni paing trgureso' >>` 2 syncs; bytes: 91 content, 0 of it moved, 13441647 index files, 0 tree |
| 23.6 | 91975 | `echo 'entchjuer anstfato anstfato ulmu oustr sese trmiingne'` 1 syncs; bytes: 169 content, 0 of it moved, 0 index files, 0 tree |
| 23.6 | 91717 | `echo 'onpa somune ontira fase semu ingsere' >> stress/d5/f63` 1 syncs; bytes: 77 content, 0 of it moved, 0 index files, 0 tree |
| 22.1 | 69623 | `echo 'guanmu tier tient isis ateseis paatean toer vugu chate` 1 syncs; bytes: 241 content, 0 of it moved, 0 index files, 0 tree |
| 21.0 | 30845 | `echo 'soing ismich tistnich pakiso netrloso chneing tomu ise` 1 syncs; bytes: 173 content, 0 of it moved, 0 index files, 0 tree |
| 19.6 | 91974 | `echo 'mutrmian ulmi guju ousto inglest ismupa rekibati' >> s` 1 syncs; bytes: 116 content, 0 of it moved, 0 index files, 0 tree |
| 15.6 | 77921 | `echo 'gustgu muleate sere toan' >> stress/d5/f16828.txt` 2 syncs; bytes: 3882 content, 3777 of it moved, 10593945 index files, 0 tree |

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

Settled afterwards (`--compact`, 4.4 s): 487300202 bytes, 1.132 of the raw text.

Index work over the run: 324865 documents added, 105365 retired, 183221725 bytes of lists rewritten by sweeps, 0 rebuilds.
