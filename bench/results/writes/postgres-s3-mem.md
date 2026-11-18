# 100000 writes on postgres, run `s3-mem`

Seed 1, search index on. The whole run took 18.6 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.010 | 0.037 | 0.112 | 0.527 | 27.3 |
| **every large write** | 9088 | 0.080 | 3.81 | 29.9 | 123 | 375 |
| append | 39801 | 0.009 | 0.030 | 0.113 | 0.537 | 19.7 |
| create | 20249 | 0.010 | 0.030 | 0.112 | 0.475 | 26.5 |
| overwrite | 9827 | 0.011 | 0.035 | 0.116 | 0.530 | 27.3 |
| sed | 9994 | 0.025 | 0.057 | 0.114 | 0.494 | 1.73 |
| rm | 5069 | 0.005 | 0.013 | 0.060 | 0.663 | 11.8 |
| mv | 3940 | 0.008 | 0.015 | 0.070 | 0.269 | 0.458 |
| same | 2032 | 0.039 | 0.075 | 0.132 | 0.718 | 0.867 |
| cp file | 5017 | 0.100 | 0.560 | 4.04 | 22.1 | 50.8 |
| append to a dataset file | 2595 | 0.011 | 0.097 | 0.746 | 2.83 | 10.8 |
| sed over a folder | 479 | 4.00 | 9.85 | 20.4 | 34.1 | 34.1 |
| cp -r | 307 | 6.68 | 46.4 | 259 | 375 | 375 |
| cat into one | 211 | 4.53 | 17.2 | 49.1 | 54.8 | 54.8 |
| sed on a large file | 285 | 10.2 | 28.6 | 68.5 | 98.7 | 98.7 |
| rm -r | 194 | 0.055 | 0.789 | 4.87 | 16.5 | 16.5 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 27.3 | 76568 | `echo 'inger onsoto bane tilo vule kimion ulisan neingantr vu` 1 syncs; bytes: 303 content, 0 of it moved, 17967557 index files, 0 tree |
| 26.5 | 88823 | `echo 'trmiingne juan tiousousst isvunele vusoingba vujuba ju` 1 syncs; bytes: 104 content, 0 of it moved, 15328185 index files, 0 tree |
| 19.7 | 99723 | `echo 'trmiingne nierfa sest onmu trnitoso muraerto sotrse so` 1 syncs; bytes: 228 content, 0 of it moved, 16672629 index files, 0 tree |
| 11.8 | 54562 | `rm stress/d5/f14669.txt` 1 syncs; bytes: 0 content, 0 of it moved, 14984663 index files, 0 tree |
| 10.9 | 13150 | `echo 'sttost mini tistnich baul ulni sole dolomu vuousvu soi` 1 syncs; bytes: 323 content, 0 of it moved, 7549622 index files, 0 tree |
| 7.78 | 15994 | `echo 'jumuerle nist trse vugu kistvu doeran anousstst loento` 0 syncs; bytes: 31064 content, 30129 of it moved, 0 index files, 0 tree |
| 4.92 | 44315 | `echo 'lenimu vujuba sobais palegu dois nist nionseon jusogus` 1 syncs; bytes: 102 content, 0 of it moved, 4078470 index files, 0 tree |
| 4.78 | 23022 | `echo 'ateulerfa toer entvustso faniki kirenemi anatemist tio` 1 syncs; bytes: 181 content, 0 of it moved, 0 index files, 0 tree |
| 3.63 | 99328 | `echo 'palegu nist doonra ismikiso onate setoneti leki negudo` 0 syncs; bytes: 324 content, 0 of it moved, 0 index files, 0 tree |
| 3.57 | 47931 | `echo 'atefach ateingtrni trnitoso isratipa vusoingba paisdor` 0 syncs; bytes: 1012 content, 571 of it moved, 0 index files, 0 tree |

Index work over the run: 283621 documents added, 100560 retired, 160950422 bytes of lists rewritten by sweeps, 0 rebuilds.
