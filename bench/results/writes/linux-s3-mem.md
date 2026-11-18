# 100000 writes on linux, run `s3-mem`

Seed 1, search index on. The whole run took 10.4 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.009 | 0.037 | 0.090 | 0.183 | 14.8 |
| **every large write** | 9088 | 0.058 | 2.47 | 13.7 | 64.3 | 237 |
| append | 39801 | 0.008 | 0.022 | 0.058 | 0.147 | 10.6 |
| create | 20249 | 0.009 | 0.021 | 0.055 | 0.160 | 7.37 |
| overwrite | 9827 | 0.009 | 0.026 | 0.062 | 0.145 | 6.57 |
| sed | 9994 | 0.035 | 0.068 | 0.115 | 0.264 | 10.9 |
| rm | 5069 | 0.005 | 0.011 | 0.039 | 0.090 | 0.267 |
| mv | 3940 | 0.007 | 0.010 | 0.041 | 0.125 | 14.8 |
| same | 2032 | 0.060 | 0.102 | 0.152 | 0.526 | 0.758 |
| cp file | 5017 | 0.081 | 0.307 | 1.05 | 3.28 | 24.9 |
| append to a dataset file | 2595 | 0.011 | 0.018 | 0.051 | 0.132 | 0.409 |
| sed over a folder | 479 | 2.98 | 7.01 | 14.5 | 28.1 | 28.1 |
| cp -r | 307 | 4.54 | 23.6 | 128 | 237 | 237 |
| cat into one | 211 | 2.53 | 4.67 | 8.34 | 22.2 | 22.2 |
| sed on a large file | 285 | 5.27 | 10.2 | 64.3 | 101 | 101 |
| rm -r | 194 | 0.052 | 1.74 | 7.93 | 21.5 | 21.5 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 14.8 | 82641 | `mv stress/d1/f4419.txt stress/d7/m24322.txt` 1 syncs; bytes: 0 content, 0 of it moved, 11421672 index files, 0 tree |
| 10.9 | 61955 | `sed -i 's/fanesetr/tipais/' stress/d0/f13682.txt` 1 syncs; bytes: 402 content, 0 of it moved, 14251835 index files, 0 tree |
| 10.6 | 37777 | `echo 'anrepado erontier loent batris loousingon kipaent loen` 1 syncs; bytes: 119 content, 0 of it moved, 9223956 index files, 0 tree |
| 7.37 | 61841 | `echo 'rele neanse loanso raous vusoingba trniate leseatefa o` 1 syncs; bytes: 205 content, 0 of it moved, 0 index files, 0 tree |
| 6.57 | 62538 | `echo 'mueran loentous vugu toenton baju vujuba chnimiis seso` 0 syncs; bytes: 291 content, 0 of it moved, 0 index files, 0 tree |
| 5.60 | 21304 | `sed -i 's/fanesetr/ulan/' stress/d7/f631.txt` 0 syncs; bytes: 269 content, 0 of it moved, 0 index files, 0 tree |
| 2.87 | 2475 | `echo 'milo ousfapase muju muon ateingon isis mious mutier ti` 0 syncs; bytes: 513 content, 0 of it moved, 0 index files, 0 tree |
| 2.43 | 61713 | `echo 'raranilo faju fanesetr chateon ateso semiulmi sese jue` 1 syncs; bytes: 252 content, 0 of it moved, 0 index files, 0 tree |
| 2.19 | 25124 | `sed -i 's/fase/muingmier/' stress/d1/m4913.txt` 0 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 1.82 | 21337 | `sed -i 's/tistnich/gust/' stress/d0/f4573.txt` 0 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |

Index work over the run: 304757 documents added, 129992 retired, 248938494 bytes of lists rewritten by sweeps, 0 rebuilds.
