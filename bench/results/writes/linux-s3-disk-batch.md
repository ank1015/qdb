# 100000 writes on linux, run `s3-disk-batch`

Seed 1, search index on. The whole run took 16.7 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.013 | 0.055 | 0.921 | 3.08 | 153 |
| **every large write** | 9088 | 0.082 | 3.26 | 17.2 | 76.2 | 281 |
| append | 39801 | 0.012 | 0.036 | 0.995 | 3.68 | 153 |
| create | 20249 | 0.013 | 0.034 | 0.964 | 3.37 | 146 |
| overwrite | 9827 | 0.014 | 0.040 | 1.11 | 2.94 | 6.90 |
| sed | 9994 | 0.048 | 0.091 | 0.173 | 2.10 | 7.13 |
| rm | 5069 | 0.007 | 0.017 | 0.891 | 2.05 | 6.24 |
| mv | 3940 | 0.010 | 0.017 | 1.02 | 2.80 | 6.43 |
| same | 2032 | 0.084 | 0.134 | 0.196 | 0.526 | 2.55 |
| cp file | 5017 | 0.110 | 0.410 | 1.73 | 5.90 | 21.2 |
| append to a dataset file | 2595 | 0.018 | 0.034 | 1.06 | 2.80 | 6.02 |
| sed over a folder | 479 | 3.99 | 8.37 | 18.0 | 25.9 | 25.9 |
| cp -r | 307 | 5.57 | 27.7 | 157 | 281 | 281 |
| cat into one | 211 | 2.89 | 6.28 | 10.4 | 35.3 | 35.3 |
| sed on a large file | 285 | 6.45 | 14.2 | 57.3 | 231 | 231 |
| rm -r | 194 | 0.076 | 1.75 | 12.3 | 32.4 | 32.4 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 153 | 1501 | `echo 'isulmumi soing ulnikiis titoouson lodoju tientjuvu tov` 1 syncs; bytes: 145 content, 0 of it moved, 0 index files, 0 tree |
| 146 | 13107 | `echo 'anan sost erul ateulerfa toousremi rele paing atemisop` 1 syncs; bytes: 298 content, 0 of it moved, 0 index files, 0 tree |
| 40.9 | 6697 | `echo 'jupa ousulstate toenton tistnich paju ousdostan dooust` 1 syncs; bytes: 524 content, 0 of it moved, 0 index files, 0 tree |
| 37.7 | 17534 | `echo 'eringanne baing leki entis kisora soing ulingerra sotr` 1 syncs; bytes: 370 content, 0 of it moved, 0 index files, 0 tree |
| 34.3 | 16451 | `echo 'sefa chent nevuate kidovutr toer' >> stress/d4/f4467.t` 1 syncs; bytes: 73 content, 0 of it moved, 0 index files, 0 tree |
| 24.9 | 88091 | `echo 'paentchpa sobajuon entjurado atenimu toer anate doonra` 0 syncs; bytes: 104 content, 0 of it moved, 0 index files, 0 tree |
| 22.0 | 18730 | `echo 'nevuate isulchni toer faan juous ramu tistnich vujuba ` 1 syncs; bytes: 189 content, 0 of it moved, 0 index files, 0 tree |
| 15.9 | 6628 | `echo 'kine neate onate lorekito raous tofatrous soing trmiin` 1 syncs; bytes: 218 content, 0 of it moved, 0 index files, 0 tree |
| 12.4 | 60418 | `echo 'mious tofatrous pavuto vugu soousousous stondo chateon` 2 syncs; bytes: 135 content, 0 of it moved, 13417845 index files, 0 tree |
| 10.9 | 37042 | `echo 'anerentfa trle pavuto ingtiulfa ulguso' > stress/d0/f1` 1 syncs; bytes: 79 content, 0 of it moved, 0 index files, 0 tree |

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

Most heap the engine held during the run: 74.8 MB.

Check of the database after the run: ok: 111007 files and 5693 folders, 1587633253 bytes of text in 130353 pieces, 15 data files with 344070 records, 1352855 posting lists compared with a fresh index

Settled afterwards (`--compact`, 15.4 s): 1765758600 bytes, 1.112 of the raw text.

Index work over the run: 333429 documents added, 129894 retired, 239837025 bytes of lists rewritten by sweeps, 0 rebuilds.
