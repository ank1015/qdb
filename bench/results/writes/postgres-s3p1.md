# 100000 writes on postgres, run `s3p1`

Seed 1, search index on. The whole run took 18.7 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.010 | 0.032 | 0.079 | 0.154 | 3.13 |
| **every large write** | 9088 | 0.090 | 2.36 | 15.1 | 41.6 | 274 |
| append | 39801 | 0.011 | 0.040 | 0.092 | 0.175 | 0.356 |
| create | 20249 | 0.008 | 0.023 | 0.059 | 0.131 | 3.13 |
| overwrite | 9827 | 0.008 | 0.030 | 0.076 | 0.136 | 0.246 |
| sed | 9994 | 0.014 | 0.035 | 0.080 | 0.149 | 0.658 |
| rm | 5069 | 0.004 | 0.012 | 0.038 | 0.085 | 0.206 |
| mv | 3940 | 0.007 | 0.011 | 0.032 | 0.047 | 0.057 |
| same | 2032 | 0.020 | 0.036 | 0.071 | 0.101 | 0.149 |
| cp file | 5017 | 0.073 | 0.292 | 0.946 | 3.41 | 10.0 |
| append to a dataset file | 2595 | 0.074 | 0.313 | 1.11 | 2.88 | 5.85 |
| sed over a folder | 479 | 3.29 | 9.61 | 23.1 | 28.1 | 28.1 |
| cp -r | 307 | 4.21 | 22.3 | 172 | 274 | 274 |
| cat into one | 211 | 2.17 | 4.23 | 10.7 | 14.7 | 14.7 |
| sed on a large file | 285 | 6.63 | 14.5 | 24.7 | 48.1 | 48.1 |
| rm -r | 194 | 0.041 | 1.56 | 5.17 | 31.5 | 31.5 |

The ten slowest small writes:

| ms | Write number | Command |
|---:|---:|---|
| 3.13 | 55185 | `echo 'vusoingba toisbavu faniousre fanesetr vustmira faon' > stress/d6/f16181.txt` |
| 0.658 | 97142 | `sed -i 's/pavuto/nejusest/' stress/d0/f6717.txt` |
| 0.356 | 76509 | `echo 'niate neseis soing seloti lesokivu ulingmitr seso togu vujuba' >> stress/d2/f9885.txt` |
| 0.345 | 91016 | `echo 'mitoju chtrra anousstst nelo sestlest vuleous semu guer vusoingba juvumu reoussose mious ervus` |
| 0.286 | 79735 | `sed -i 's/doonra/tipato/' stress/d3/m7872.txt` |
| 0.277 | 93399 | `echo 'jupaguju seso lochulvu ousse faju ousguon chto ermu ingtoneso juingentso' >> stress/d0/f21300.` |
| 0.270 | 92996 | `echo 'mutier atetoentso loonlo leneate' >> stress/d4/f3560.txt` |
| 0.261 | 44001 | `echo 'trpane ulki ousateentgu erfa gumuatech ingulstate loseentmi anousstst leis doneni muervuan anm` |
| 0.248 | 80752 | `echo 'soing vugu migule trmiingne resostni loingnido toer sese ingchpa stent tise reon stingtr semu ` |
| 0.246 | 92252 | `echo 'vugu faan guousing onulpaing anstfato fanesetr ingpagu retonigu chju nist fanesetr pane ingmi ` |

Index work over the run: 207705 documents added, 167766 retired, 338259830 bytes of lists rewritten by sweeps, 0 rebuilds.
