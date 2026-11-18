# 100000 writes on linux, run `s4-disk-batch`

Seed 1, search index on. The whole run took 10.6 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.014 | 0.049 | 0.737 | 1.77 | 16.4 |
| **every large write** | 9088 | 0.017 | 1.50 | 10.8 | 34.4 | 110 |
| append | 39801 | 0.012 | 0.041 | 0.696 | 1.70 | 16.4 |
| create | 20249 | 0.014 | 0.038 | 0.738 | 1.73 | 15.9 |
| overwrite | 9827 | 0.015 | 0.044 | 0.753 | 1.66 | 5.49 |
| sed | 9994 | 0.036 | 0.074 | 0.808 | 1.89 | 5.41 |
| rm | 5069 | 0.008 | 0.020 | 0.851 | 2.30 | 5.26 |
| mv | 3940 | 0.010 | 0.017 | 0.721 | 1.55 | 5.59 |
| same | 2032 | 0.051 | 0.091 | 0.817 | 2.07 | 3.10 |
| cp file | 5017 | 0.013 | 0.022 | 0.844 | 1.82 | 6.60 |
| append to a dataset file | 2595 | 0.023 | 0.053 | 0.662 | 1.61 | 2.14 |
| sed over a folder | 479 | 4.43 | 10.5 | 20.1 | 29.4 | 29.4 |
| cp -r | 307 | 0.120 | 0.400 | 1.46 | 2.82 | 2.82 |
| cat into one | 211 | 3.59 | 6.60 | 16.5 | 65.4 | 65.4 |
| sed on a large file | 285 | 6.28 | 12.1 | 67.2 | 110 | 110 |
| rm -r | 194 | 0.057 | 0.170 | 0.790 | 1.80 | 1.80 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 16.4 | 14296 | `echo 'doonra jusejuch mufa muanba isneki soousermu muso nist` 2 syncs; bytes: 95 content, 0 of it moved, 10200581 index files, 0 tree |
| 15.9 | 48204 | `echo 'pavuto ananpaan fami trnitoso anstfato trmiingne vuate` 0 syncs; bytes: 157 content, 0 of it moved, 0 index files, 0 tree |
| 13.9 | 70577 | `echo 'toching anousstst pavuto onate isdo nech doonra ingbak` 2 syncs; bytes: 114 content, 0 of it moved, 3478142 index files, 0 tree |
| 7.64 | 62361 | `echo 'raerrera nelesoent trpane stonto loent rele trnitoso c` 3 syncs; bytes: 142 content, 0 of it moved, 0 index files, 5515864 tree |
| 6.04 | 70551 | `echo 'ullo doingniki bagutolo semu vugu dostselo baing niate` 1 syncs; bytes: 204 content, 0 of it moved, 0 index files, 0 tree |
| 5.59 | 14120 | `mv stress/d7/f2075.txt stress/d5/m4151.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 5.49 | 14244 | `echo 'ingletoent nirele faguing tilo ergutoous' > stress/d3/` 1 syncs; bytes: 81 content, 0 of it moved, 0 index files, 0 tree |
| 5.41 | 14275 | `sed -i 's/semu/ousst/' stress/d0/f2382.txt` 1 syncs; bytes: 162 content, 0 of it moved, 0 index files, 0 tree |
| 5.35 | 14182 | `echo 'entmini soan repa michtois gunest seso mugune niateer ` 1 syncs; bytes: 104 content, 0 of it moved, 0 index files, 0 tree |
| 5.26 | 14151 | `rm stress/d7/f3474.txt` 1 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |

On disk, sync batch.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 243334784 |
| log of the command lines | 17474804 |
| content moved to free space | 0 |
| index files | 13678723 |
| saves of the tree (1) | 5515864 |
| **all** | 280004175 |
| **write amplification**: all, over content and log the commands caused | 1.07 |

Syncs: 1588 over 100002 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 98419 | 1576 | 3 | 2 | 0 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 1550823893 | 1396914445 | 1.110 |
| at its worst (after write 97000; looked at every 200 writes) | 1810562811 | 1563092992 | 1.158 |
| at the end | 1816890893 | 1587633253 | 1.144 |

At the end, by what it is: text 1402200517, index lists 133661350, index tables 11073816, tree 5515864, log 22852366, record and file headers 10593748, space not yet reclaimed 230993232.

Most heap the engine held during the run: 58.4 MB.

Check of the database after the run: ok: 111007 files and 5693 folders, 1587633253 bytes of text in 116852 pieces, 13 data files with 358827 records, 1006615 posting lists compared with a fresh index

Settled afterwards (`--compact`, 19.3 s): 1731560638 bytes, 1.091 of the raw text.

Index work over the run: 298417 documents added, 137543 retired, 189139530 bytes of lists rewritten by sweeps, 0 rebuilds.
