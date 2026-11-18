# 100000 writes on redis, run `s3p1`

Seed 1, search index on. The whole run took 13.6 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.010 | 0.031 | 0.074 | 0.150 | 1.11 |
| **every large write** | 9088 | 0.063 | 2.05 | 12.6 | 47.7 | 113 |
| append | 39801 | 0.011 | 0.036 | 0.084 | 0.170 | 0.591 |
| create | 20249 | 0.009 | 0.022 | 0.055 | 0.117 | 0.578 |
| overwrite | 9827 | 0.009 | 0.028 | 0.067 | 0.145 | 1.11 |
| sed | 9994 | 0.014 | 0.034 | 0.078 | 0.127 | 0.403 |
| rm | 5069 | 0.005 | 0.011 | 0.038 | 0.100 | 0.188 |
| mv | 3940 | 0.007 | 0.011 | 0.034 | 0.053 | 0.131 |
| same | 2032 | 0.019 | 0.036 | 0.068 | 0.126 | 0.139 |
| cp file | 5017 | 0.049 | 0.214 | 0.909 | 2.59 | 3.63 |
| append to a dataset file | 2595 | 0.052 | 0.213 | 0.979 | 1.58 | 3.63 |
| sed over a folder | 479 | 3.25 | 9.17 | 20.6 | 27.6 | 27.6 |
| cp -r | 307 | 4.69 | 27.4 | 62.6 | 113 | 113 |
| cat into one | 211 | 1.46 | 2.63 | 3.84 | 8.03 | 8.03 |
| sed on a large file | 285 | 3.88 | 7.25 | 15.2 | 23.2 | 23.2 |
| rm -r | 194 | 0.066 | 1.85 | 9.85 | 12.1 | 12.1 |

The ten slowest small writes:

| ms | Write number | Command |
|---:|---:|---|
| 1.11 | 76469 | `echo 'lefaso vujuba paatean vusoingba muerra faju tistnich semu' > stress/d7/f5782.txt` |
| 0.591 | 83067 | `echo 'isanis nitoraous palegu juneer niate' >> stress/d1/f11468.txt` |
| 0.578 | 40164 | `echo 'tofarera paisdore vuate faniousre mule ulst ateulerfa rele tidoso ingchanpa vuiskire senelelo ` |
| 0.461 | 13547 | `echo 'anstfato fanesetr pare loni somutora fase isis ateantifa ousentle faniousre setr vufast trentd` |
| 0.448 | 75863 | `echo 'ateulerfa trsene leki patotr erpakier sovu paan vugu reguneto vujuba mutier fanesetr seso' >> ` |
| 0.403 | 13469 | `sed -i 's/ateso/gurean/' stress/d3/f1972.txt` |
| 0.398 | 76532 | `echo 'baentousju sese bami loistran anch leki stulentre ateseniso raate sese ateanle gure leju ateen` |
| 0.396 | 81163 | `echo 'nese toreguan mious ulpaent doonra anousstst isra tobach guateismi isis tistnich' >> stress/d5` |
| 0.312 | 81174 | `echo 'lore ateulerfa tofatrous sotring gudostni mistra seti vusoingba dogumu tistnich ondoju' >> str` |
| 0.303 | 81331 | `echo 'ulfa trti mifasoul vusoingba chent kivu tinetise vusoingba pajulele jubara trisni tilo atelefa` |

Index work over the run: 213978 documents added, 177632 retired, 356221757 bytes of lists rewritten by sweeps, 0 rebuilds.
