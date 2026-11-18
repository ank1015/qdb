# The switches compared on postgres, run `s3`

Each row is a fresh import of the dataset with the row's switches (sync in batches during the import), then 20000 command lines, seed 1. Times in milliseconds. Disk is the database directory after the run and a clean close, and against the raw text of all files then. Peak memory is the most heap the engine held from the import on. Import is the time of the import, in seconds.

| Sync | Index built | Text | Workload | Commands/s | Read median | Read 99% | Write median | Write 99% | Disk MB | Disk / raw text | Peak memory MB | Import s |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| always | at write | plain | 95% reads | 1313 | 0.017 | 18.8 | 0.592 | 2.44 | 182.3 | 1.122 | 39.3 | 1.8 |
| always | at write | compressed | 95% reads | 438 | 0.023 | 59.8 | 0.684 | 3.76 | 80.0 | 0.493 | 32.4 | 1.7 |
| always | later | plain | 95% reads | 1147 | 0.017 | 23.4 | 0.571 | 1.73 | 182.2 | 1.122 | 32.6 | 1.5 |
| always | later | compressed | 95% reads | 324 | 0.023 | 83.2 | 0.597 | 2.65 | 79.9 | 0.492 | 30.8 | 1.6 |
| batch | at write | plain | 95% reads | 1413 | 0.016 | 18.9 | 0.040 | 1.24 | 182.3 | 1.122 | 39.3 | 1.6 |
| batch | at write | compressed | 95% reads | 463 | 0.021 | 58.2 | 0.048 | 1.12 | 80.0 | 0.493 | 32.4 | 1.6 |
| batch | later | plain | 95% reads | 1226 | 0.016 | 22.7 | 0.026 | 1.07 | 182.2 | 1.122 | 32.6 | 1.4 |
| batch | later | compressed | 95% reads | 327 | 0.022 | 83.2 | 0.037 | 1.02 | 79.9 | 0.492 | 30.8 | 1.6 |
| always | at write | plain | 50% reads | 2126 | 0.017 | 18.4 | 0.191 | 0.971 | 200.0 | 1.135 | 39.3 | 1.5 |
| always | at write | compressed | 50% reads | 799 | 0.025 | 61.4 | 0.207 | 1.77 | 88.8 | 0.504 | 37.3 | 1.6 |
| always | later | plain | 50% reads | 1410 | 0.018 | 29.0 | 0.196 | 0.945 | 199.5 | 1.132 | 32.6 | 1.6 |
| always | later | compressed | 50% reads | 393 | 0.025 | 103 | 0.193 | 1.14 | 88.2 | 0.501 | 36.8 | 1.6 |
| batch | at write | plain | 50% reads | 2653 | 0.016 | 19.1 | 0.015 | 1.04 | 200.0 | 1.135 | 39.3 | 1.6 |
| batch | at write | compressed | 50% reads | 908 | 0.021 | 59.8 | 0.017 | 0.984 | 88.8 | 0.504 | 37.3 | 1.6 |
| batch | later | plain | 50% reads | 1851 | 0.014 | 27.7 | 0.007 | 0.903 | 199.5 | 1.132 | 32.6 | 1.5 |
| batch | later | compressed | 50% reads | 427 | 0.020 | 99.2 | 0.009 | 0.939 | 88.2 | 0.501 | 36.8 | 1.7 |
| always | at write | plain | 5% reads | 4148 | 0.025 | 21.8 | 0.169 | 0.846 | 207.5 | 1.153 | 39.3 | 1.7 |
| always | at write | compressed | 5% reads | 3139 | 0.029 | 69.0 | 0.157 | 0.795 | 94.6 | 0.526 | 38.7 | 1.7 |
| always | later | plain | 5% reads | 3541 | 0.024 | 34.7 | 0.161 | 0.587 | 206.6 | 1.148 | 32.6 | 1.4 |
| always | later | compressed | 5% reads | 1969 | 0.028 | 117 | 0.158 | 0.685 | 93.7 | 0.520 | 37.3 | 1.7 |
| batch | at write | plain | 5% reads | 13632 | 0.018 | 20.4 | 0.009 | 0.652 | 207.5 | 1.153 | 39.3 | 1.6 |
| batch | at write | compressed | 5% reads | 6058 | 0.023 | 64.5 | 0.011 | 0.694 | 94.6 | 0.526 | 38.7 | 1.6 |
| batch | later | plain | 5% reads | 8684 | 0.016 | 29.5 | 0.005 | 0.660 | 206.6 | 1.148 | 32.6 | 1.4 |
| batch | later | compressed | 5% reads | 2923 | 0.022 | 104 | 0.006 | 0.697 | 93.7 | 0.520 | 37.3 | 1.5 |
