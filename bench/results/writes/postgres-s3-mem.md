# 100000 writes on postgres, run `s3-mem`

Seed 1, search index on. The whole run took 15.5 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.009 | 0.028 | 0.088 | 0.411 | 22.1 |
| **every large write** | 9088 | 0.067 | 3.24 | 25.5 | 119 | 255 |
| append | 39801 | 0.008 | 0.021 | 0.091 | 0.387 | 22.1 |
| create | 20249 | 0.009 | 0.021 | 0.091 | 0.426 | 10.5 |
| overwrite | 9827 | 0.009 | 0.026 | 0.093 | 0.565 | 10.7 |
| sed | 9994 | 0.020 | 0.043 | 0.087 | 0.410 | 2.01 |
| rm | 5069 | 0.005 | 0.010 | 0.043 | 0.335 | 18.2 |
| mv | 3940 | 0.007 | 0.011 | 0.054 | 0.191 | 0.444 |
| same | 2032 | 0.031 | 0.057 | 0.101 | 0.674 | 0.853 |
| cp file | 5017 | 0.084 | 0.472 | 3.47 | 18.6 | 36.1 |
| append to a dataset file | 2595 | 0.009 | 0.080 | 0.636 | 2.80 | 8.62 |
| sed over a folder | 479 | 3.43 | 7.66 | 16.2 | 25.9 | 25.9 |
| cp -r | 307 | 5.56 | 41.0 | 216 | 255 | 255 |
| cat into one | 211 | 3.75 | 14.1 | 46.2 | 57.7 | 57.7 |
| sed on a large file | 285 | 8.92 | 26.0 | 60.2 | 77.8 | 77.8 |
| rm -r | 194 | 0.041 | 0.682 | 3.65 | 11.6 | 11.6 |

The ten slowest small writes:

| ms | Write number | Command |
|---:|---:|---|
| 22.1 | 39757 | `echo 'famipa nevu anstfato trnitoso atelopa mion anvuermi raonvuate tilo neguis' >> stress/d5/f4615.` |
| 18.2 | 54562 | `rm stress/d5/f14669.txt` |
| 11.6 | 99723 | `echo 'trmiingne nierfa sest onmu trnitoso muraerto sotrse soju oustibani ateulerfa chateon pavuto on` |
| 10.7 | 76568 | `echo 'inger onsoto bane tilo vule kimion ulisan neingantr vule mutier ulnikiis paanba ingtrpa mupalo` |
| 10.5 | 88823 | `echo 'trmiingne juan tiousousst isvunele vusoingba vujuba juiser soch' > stress/d4/f26115.txt` |
| 7.85 | 13150 | `echo 'sttost mini tistnich baul ulni sole dolomu vuousvu soing soing antr ateousmuis' >> stress/d0/f` |
| 5.44 | 44315 | `echo 'lenimu vujuba sobais palegu dois nist nionseon jusogust aning' >> stress/d5/f1536.txt` |
| 4.06 | 15994 | `echo 'jumuerle nist trse vugu kistvu doeran anousstst loentous ateso fanesetr vusoingba nitivupa soi` |
| 3.99 | 23022 | `echo 'ateulerfa toer entvustso faniki kirenemi anatemist tion leki toer raravu tobach faniousre fane` |
| 3.52 | 99328 | `echo 'palegu nist doonra ismikiso onate setoneti leki negudoso tovu muju fanesetr mutier rekiingfa g` |

Index work over the run: 283621 documents added, 100560 retired, 160950422 bytes of lists rewritten by sweeps, 0 rebuilds.
