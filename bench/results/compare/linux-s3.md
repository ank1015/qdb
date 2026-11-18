# The switches compared on linux, run `s3`

Each row is a fresh import of the dataset with the row's switches (sync in batches during the import), then 20000 command lines, seed 1. Times in milliseconds. Disk is the database directory after the run and a clean close, and against the raw text of all files then. Peak memory is the most heap the engine held from the import on. Import is the time of the import, in seconds.

| Sync | Index built | Text | Workload | Commands/s | Read median | Read 99% | Write median | Write 99% | Disk MB | Disk / raw text | Peak memory MB | Import s |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| always | at write | plain | 95% reads | 297 | 0.027 | 109 | 0.669 | 2.77 | 1550.2 | 1.109 | 67.1 | 14.2 |
| always | at write | compressed | 95% reads | 90 | 0.037 | 353 | 0.710 | 2.04 | 597.1 | 0.427 | 104.0 | 14.3 |
| always | later | plain | 95% reads | 309 | 0.024 | 95.2 | 0.615 | 1.50 | 1549.7 | 1.109 | 59.8 | 11.9 |
| always | later | compressed | 95% reads | 95 | 0.033 | 310 | 0.690 | 2.54 | 597.1 | 0.427 | 93.8 | 13.8 |
| batch | at write | plain | 95% reads | 300 | 0.026 | 108 | 0.051 | 1.34 | 1550.2 | 1.109 | 67.1 | 14.0 |
| batch | at write | compressed | 95% reads | 91 | 0.035 | 342 | 0.057 | 1.01 | 597.1 | 0.427 | 104.0 | 14.9 |
| batch | later | plain | 95% reads | 309 | 0.024 | 96.5 | 0.036 | 1.07 | 1549.7 | 1.109 | 59.8 | 12.0 |
| batch | later | compressed | 95% reads | 98 | 0.032 | 310 | 0.039 | 1.17 | 597.1 | 0.427 | 93.8 | 14.8 |
| always | at write | plain | 50% reads | 533 | 0.024 | 96.1 | 0.219 | 1.22 | 1559.0 | 1.111 | 67.1 | 14.1 |
| always | at write | compressed | 50% reads | 165 | 0.039 | 308 | 0.231 | 1.38 | 602.9 | 0.430 | 104.0 | 15.4 |
| always | later | plain | 50% reads | 510 | 0.024 | 92.4 | 0.212 | 1.22 | 1557.9 | 1.110 | 59.8 | 13.9 |
| always | later | compressed | 50% reads | 176 | 0.034 | 285 | 0.215 | 1.27 | 602.3 | 0.429 | 93.8 | 15.2 |
| batch | at write | plain | 50% reads | 537 | 0.023 | 108 | 0.020 | 1.04 | 1559.0 | 1.111 | 67.1 | 15.1 |
| batch | at write | compressed | 50% reads | 171 | 0.034 | 331 | 0.022 | 1.00 | 602.9 | 0.430 | 104.0 | 14.9 |
| batch | later | plain | 50% reads | 580 | 0.021 | 88.5 | 0.008 | 1.17 | 1557.9 | 1.110 | 59.8 | 13.6 |
| batch | later | compressed | 50% reads | 185 | 0.032 | 271 | 0.010 | 0.953 | 602.3 | 0.429 | 93.8 | 14.5 |
| always | at write | plain | 5% reads | 2761 | 0.033 | 94.2 | 0.162 | 0.855 | 1575.7 | 1.112 | 67.1 | 14.0 |
| always | at write | compressed | 5% reads | 1347 | 0.044 | 296 | 0.167 | 0.976 | 609.7 | 0.430 | 104.0 | 13.9 |
| always | later | plain | 5% reads | 2692 | 0.028 | 81.0 | 0.170 | 0.665 | 1574.3 | 1.111 | 60.0 | 12.6 |
| always | later | compressed | 5% reads | 1440 | 0.039 | 263 | 0.165 | 0.694 | 608.9 | 0.430 | 93.8 | 14.9 |
| batch | at write | plain | 5% reads | 5049 | 0.022 | 85.0 | 0.010 | 0.866 | 1575.7 | 1.112 | 67.1 | 13.7 |
| batch | at write | compressed | 5% reads | 1855 | 0.037 | 250 | 0.012 | 0.591 | 609.7 | 0.430 | 104.0 | 14.5 |
| batch | later | plain | 5% reads | 5652 | 0.018 | 75.7 | 0.005 | 0.529 | 1574.3 | 1.111 | 60.0 | 12.6 |
| batch | later | compressed | 5% reads | 2050 | 0.030 | 211 | 0.006 | 0.539 | 608.9 | 0.430 | 93.8 | 14.0 |
