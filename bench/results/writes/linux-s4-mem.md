# 100000 writes on linux, run `s4-mem`

Seed 1, search index on. The whole run took 6.7 s.

Times in milliseconds.

| Kind of write | Writes | Median | 90% | 99% | 99.9% | Slowest |
|---|---:|---:|---:|---:|---:|---:|
| **every small write** | 90912 | 0.010 | 0.033 | 0.073 | 0.143 | 40.7 |
| **every large write** | 9088 | 0.012 | 1.12 | 9.30 | 23.7 | 76.5 |
| append | 39801 | 0.009 | 0.027 | 0.063 | 0.121 | 40.7 |
| create | 20249 | 0.010 | 0.024 | 0.056 | 0.147 | 8.15 |
| overwrite | 9827 | 0.010 | 0.031 | 0.067 | 0.194 | 6.33 |
| sed | 9994 | 0.027 | 0.053 | 0.098 | 0.169 | 2.99 |
| rm | 5069 | 0.005 | 0.013 | 0.039 | 0.091 | 0.285 |
| mv | 3940 | 0.007 | 0.011 | 0.036 | 0.104 | 0.413 |
| same | 2032 | 0.041 | 0.070 | 0.116 | 0.221 | 0.370 |
| cp file | 5017 | 0.010 | 0.016 | 0.040 | 0.077 | 0.149 |
| append to a dataset file | 2595 | 0.012 | 0.023 | 0.052 | 0.101 | 0.299 |
| sed over a folder | 479 | 3.47 | 9.13 | 18.7 | 29.4 | 29.4 |
| cp -r | 307 | 0.094 | 0.318 | 1.11 | 2.14 | 2.14 |
| cat into one | 211 | 2.91 | 5.05 | 8.57 | 62.3 | 62.3 |
| sed on a large file | 285 | 5.65 | 10.9 | 34.6 | 76.5 | 76.5 |
| rm -r | 194 | 0.051 | 0.136 | 0.337 | 0.438 | 0.438 |

The ten slowest small writes:

| ms | Write number | Command, and what was written to disk during it |
|---:|---:|---|
| 40.7 | 16788 | `echo 'vujuba faneti fati faneto' >> stress/d3/m3597.txt` 1 syncs; bytes: 66 content, 0 of it moved, 11065220 index files, 0 tree |
| 8.15 | 74912 | `echo 'sotring oner stousvu ululju enttiateing vugu pavuto ti` 0 syncs; bytes: 247 content, 0 of it moved, 0 index files, 0 tree |
| 6.33 | 76772 | `echo 'niulon leki tiulmi isan falele lotr isrevuan lojura tr` 1 syncs; bytes: 172 content, 0 of it moved, 3551894 index files, 0 tree |
| 3.20 | 76746 | `echo 'onbaentju tringate seso vujuba kimu palegu dodo doanlo` 1 syncs; bytes: 254 content, 0 of it moved, 0 index files, 0 tree |
| 3.02 | 16773 | `echo 'fase tistnich kibalo muju leki trulmi bare dore' >> st` 1 syncs; bytes: 88 content, 0 of it moved, 0 index files, 0 tree |
| 2.99 | 20975 | `sed -i 's/fase/oustr/' stress/d4/f3922.txt` 0 syncs; bytes: 0 content, 0 of it moved, 0 index files, 0 tree |
| 2.02 | 74864 | `echo 'leki anvuisba levu nitoto ateateba niate uldoering nev` 0 syncs; bytes: 142 content, 0 of it moved, 0 index files, 0 tree |
| 1.64 | 30592 | `echo 'ingguis toererch soing juju somune leki kipa anchne fa` 0 syncs; bytes: 292 content, 0 of it moved, 0 index files, 0 tree |
| 1.55 | 16742 | `echo 'dostselo vusoingba ousanse vuous soing nebapa istrsoch` 1 syncs; bytes: 146 content, 0 of it moved, 0 index files, 0 tree |
| 1.30 | 20987 | `echo 'justse stlokiis faniousre toer sotring ermu mumuer sts` 0 syncs; bytes: 441 content, 0 of it moved, 0 index files, 0 tree |

Index work over the run: 271128 documents added, 136951 retired, 187188114 bytes of lists rewritten by sweeps, 0 rebuilds.
