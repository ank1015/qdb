# 100000 writes on linux, run `s4-disk-always`

Seed 1, search index on. The whole run took 29.3 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.179 | 0.311 | 0.690 | 2.32 | 23.8 |
| **every large write** | 9088 | 0.184 | 2.34 | 11.8 | 31.7 | 95.1 |
| append | 39801 | 0.173 | 0.305 | 0.701 | 2.35 | 23.8 |
| create | 20249 | 0.178 | 0.321 | 0.719 | 2.33 | 12.3 |
| overwrite | 9827 | 0.180 | 0.314 | 0.667 | 2.55 | 22.8 |
| sed | 9994 | 0.214 | 0.336 | 0.686 | 1.86 | 3.44 |
| rm | 5069 | 0.159 | 0.243 | 0.547 | 1.74 | 3.32 |
| mv | 3940 | 0.162 | 0.242 | 0.493 | 1.61 | 3.16 |
| same | 2032 | 0.243 | 0.365 | 0.760 | 2.49 | 2.97 |
| cp file | 5017 | 0.167 | 0.261 | 0.631 | 2.39 | 12.8 |
| append to a dataset file | 2595 | 0.190 | 0.299 | 0.722 | 2.36 | 2.48 |
| sed over a folder | 479 | 5.64 | 11.3 | 22.9 | 32.3 | 32.3 |
| cp -r | 307 | 0.533 | 1.14 | 2.14 | 9.58 | 9.58 |
| cat into one | 211 | 4.61 | 7.94 | 12.8 | 37.8 | 37.8 |
| sed on a large file | 285 | 7.62 | 14.2 | 54.7 | 95.1 | 95.1 |
| rm -r | 194 | 0.249 | 0.490 | 0.904 | 1.29 | 1.29 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 23.8 | 2412 | `echo 'entreous anstfato anstfato stseate erontier guni ulraa` 1 syncs; bytes: 518 content, 0 of it moved, 0 index files, 0 tree |
| 22.8 | 81672 | `echo 'nist vujuba chreso semu ulfakient seso ousulse trmiing` 1 syncs; bytes: 276 content, 0 of it moved, 0 index files, 0 tree |
| 21.6 | 48328 | `echo 'lereent gurane raous ateso toreto doonra mimi tistnich` 1 syncs; bytes: 468 content, 0 of it moved, 0 index files, 0 tree |
| 20.8 | 12720 | `echo 'nilesoul tilo sese fatr seso raous onlolo soent toniis` 1 syncs; bytes: 95 content, 0 of it moved, 0 index files, 0 tree |
| 13.1 | 44575 | `echo 'patoki kikisone faniousre faniousre tianvu torerara re` 1 syncs; bytes: 390 content, 0 of it moved, 0 index files, 0 tree |
| 12.3 | 13943 | `echo 'paent trto juisonre reantotr anneti anousstst chmi mie` 1 syncs; bytes: 297 content, 0 of it moved, 0 index files, 0 tree |
| 12.1 | 14296 | `echo 'doonra jusejuch mufa muanba isneki soousermu muso nist` 2 syncs; bytes: 95 content, 0 of it moved, 10200581 index files, 0 tree |
| 5.09 | 70577 | `echo 'toching anousstst pavuto onate isdo nech doonra ingbak` 2 syncs; bytes: 114 content, 0 of it moved, 3478142 index files, 0 tree |
| 5.02 | 90124 | `echo 'faul ernibaba oustiju vujuba nist stulmiki atelodomi n` 1 syncs; bytes: 394 content, 0 of it moved, 0 index files, 0 tree |
| 4.76 | 62488 | `echo 'atevu ousdo faseanju mutier ankichso ison entnimi erch` 1 syncs; bytes: 515 content, 0 of it moved, 0 index files, 0 tree |

On disk, sync always.

| Bytes written to disk over the run | |
|---|---:|
| content the commands wrote | 243334784 |
| log of the command lines | 17474804 |
| content moved to free space | 0 |
| index files | 13678723 |
| saves of the tree (1) | 5515864 |
| **all** | 280004175 |
| **write amplification**: all, over content and log the commands caused | 1.07 |

Syncs: 100025 over 100002 command lines that changed something.

| Syncs in one command line | 0 | 1 | 2 | 3 | 4 or more |
|---|---:|---:|---:|---:|---:|
| command lines | 0 | 99981 | 16 | 2 | 1 |

| Disk against the text held | Directory, bytes | Raw text of all files, bytes | Ratio |
|---|---:|---:|---:|
| at the start (settled) | 1550823893 | 1396914445 | 1.110 |
| at its worst (after write 97000; looked at every 200 writes) | 1810562811 | 1563092992 | 1.158 |
| at the end | 1816890893 | 1587633253 | 1.144 |

At the end, by what it is: text 1402200517, index lists 133661350, index tables 11073816, tree 5515864, log 22852366, record and file headers 10593748, space not yet reclaimed 230993232.

Most heap the engine held during the run: 58.4 MB.

Check of the database after the run: ok: 111007 files and 5693 folders, 1587633253 bytes of text in 116852 pieces, 13 data files with 358827 records, 1006615 posting lists compared with a fresh index

Settled afterwards (`--compact`, 18.4 s): 1731560638 bytes, 1.091 of the raw text.

Index work over the run: 298417 documents added, 137543 retired, 189139530 bytes of lists rewritten by sweeps, 0 rebuilds.
