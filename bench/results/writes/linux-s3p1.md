# 100000 writes on linux, run `s3p1`

Seed 1, search index on. The whole run took 29.8 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.010 | 0.030 | 0.109 | 0.252 | 13.6 |
| **every large write** | 9088 | 0.096 | 2.11 | 14.4 | 41.9 | 173 |
| append | 39801 | 0.010 | 0.037 | 0.134 | 0.277 | 1.24 |
| create | 20249 | 0.008 | 0.023 | 0.076 | 0.203 | 13.6 |
| overwrite | 9827 | 0.008 | 0.028 | 0.124 | 0.260 | 0.485 |
| sed | 9994 | 0.015 | 0.035 | 0.089 | 0.206 | 0.653 |
| rm | 5069 | 0.004 | 0.011 | 0.051 | 0.127 | 0.212 |
| mv | 3940 | 0.007 | 0.012 | 0.035 | 0.050 | 0.059 |
| same | 2032 | 0.020 | 0.038 | 0.069 | 0.093 | 0.106 |
| cp file | 5017 | 0.076 | 0.288 | 0.735 | 2.07 | 13.1 |
| append to a dataset file | 2595 | 0.078 | 0.297 | 0.797 | 3.05 | 31.3 |
| sed over a folder | 479 | 3.14 | 9.80 | 30.8 | 39.0 | 39.0 |
| cp -r | 307 | 4.14 | 17.8 | 70.2 | 173 | 173 |
| cat into one | 211 | 1.81 | 2.98 | 5.52 | 21.4 | 21.4 |
| sed on a large file | 285 | 4.18 | 8.48 | 36.3 | 80.6 | 80.6 |
| rm -r | 194 | 0.048 | 0.972 | 5.51 | 11.4 | 11.4 |

The ten slowest small writes:

| ms | Write number | Command |
|---:|---:|---|
| 13.6 | 87078 | `echo 'fanesetr onrare sevuatetr fale ermu kine neisgu pare semimi mimumifa isis sean tojule dois soo` |
| 1.67 | 35711 | `echo 'fatr entvuso semu lesopami netrloso sobanest entlech oustistto faerleba islolovu mutier paerne` |
| 1.24 | 14041 | `echo 'judo sese titoanki leous miso soate refami soba' >> stress/d7/f3654.txt` |
| 0.833 | 90491 | `echo 'mitrstmu raer vuate guouski mifami anousstst setoguvu vusoingba stto leulan ousraate gustisle'` |
| 0.653 | 63567 | `sed -i 's/faniousre/atemuch/' stress/d6/f16420.txt` |
| 0.521 | 89299 | `echo 'ingju ismidopa ateso entki judoentba isis ingseate ateisdogu ananstso paent jusejuch faer toer` |
| 0.519 | 91312 | `echo 'isanatean ermu anleis doonra nieringis' >> stress/d1/f3418.txt` |
| 0.508 | 94005 | `echo 'chentjupa onate neentvu erul totr tilo' >> stress/d3/f6566.txt` |
| 0.485 | 31897 | `echo 'ersetiso lostnilo erbaraba juouson stousnise' > stress/d5/m1312.txt` |
| 0.472 | 91178 | `echo 'pasoonki antier patrmimi batrdo bakiba nereisre' >> stress/d0/f13243.txt` |

Index work over the run: 281825 documents added, 170842 retired, 336687156 bytes of lists rewritten by sweeps, 0 rebuilds.
