## Linux tree, 95 in 100 units only read (20,000 units, always)

20001 units in 58.95 s (339 a second), sync always, index on. 0 ended with a status above 1. The most memory any one unit added: 1.8 MB. Times are measured and differ from run to run; every other column is counted and is the same on every run over the same database.

### By what the unit did

| What it did | Units | Share of units | CPU, ms | Share of CPU | Elapsed, ms | Not on the CPU, ms | Text read, MB | Index read, MB | Tree entries | Content written, MB | Records written, MB | Upkeep written, MB | Syncs | Scratch, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| read a little | 10292 | 51.5% | 138 | 0.2% | 139 | 0.805 | 71.0 | 0 | 171316 | 0 | 0 | 0 | 0 | 0 |
| read a lot | 160 | 0.8% | 1.70 | 0.0% | 1.70 | 0.000 | 37.1 | 0 | 1584 | 0 | 0 | 0 | 0 | 0 |
| searched with the index | 2145 | 10.7% | 35392 | 61.9% | 35613 | 222 | 181554 | 504 | 20958767 | 0 | 0 | 0 | 0 | 0 |
| searched without the index | 6400 | 32.0% | 21558 | 37.7% | 21705 | 147 | 143630 | 0 | 9259060 | 0 | 0 | 0 | 0 | 0 |
| wrote a little | 1004 | 5.0% | 123 | 0.2% | 1411 | 1288 | 0.137 | 0 | 4407 | 0.161 | 0.074 | 0 | 1004 | 0 |
| **all** | 20001 | 100.0% | 57212 | 100.0% | 58869 | 1657 | 325292 | 504 | 30395134 | 0.161 | 0.074 | 0 | 1004 | 0 |

### By first command

| First command | Units | Share of units | CPU, ms | Share of CPU | Elapsed, ms | Not on the CPU, ms | Text read, MB | Index read, MB | Tree entries | Content written, MB | Records written, MB | Upkeep written, MB | Syncs | Scratch, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `grep` | 8545 | 42.7% | 56949 | 99.5% | 57318 | 368 | 325184 | 504 | 30217827 | 0 | 0 | 0 | 0 | 0 |
| `echo` | 752 | 3.8% | 92.4 | 0.2% | 494 | 401 | 0.111 | 0 | 2759 | 0.155 | 0.058 | 0 | 752 | 0 |
| `cat` | 4808 | 24.0% | 53.5 | 0.1% | 53.6 | 0.097 | 78.5 | 0 | 45362 | 0 | 0 | 0 | 0 | 0 |
| `head` | 2904 | 14.5% | 35.6 | 0.1% | 35.8 | 0.159 | 29.7 | 0 | 13694 | 0 | 0 | 0 | 0 | 0 |
| `find` | 932 | 4.7% | 30.4 | 0.1% | 30.6 | 0.182 | 0 | 0 | 105217 | 0 | 0 | 0 | 0 | 0 |
| `wc` | 1808 | 9.0% | 20.4 | 0.0% | 20.7 | 0.366 | 0 | 0 | 8627 | 0 | 0 | 0 | 0 | 0 |
| `sed` | 105 | 0.5% | 14.9 | 0.0% | 47.6 | 32.8 | 0.026 | 0 | 315 | 0.006 | 0.006 | 0 | 105 | 0 |
| `cp` | 61 | 0.3% | 6.53 | 0.0% | 25.8 | 19.3 | 0 | 0 | 780 | 0 | 0.006 | 0 | 61 | 0 |
| `rm` | 53 | 0.3% | 5.51 | 0.0% | 808 | 803 | 0 | 0 | 159 | 0 | 0.002 | 0 | 53 | 0 |
| `mv` | 32 | 0.2% | 3.30 | 0.0% | 34.3 | 31.0 | 0 | 0 | 384 | 0 | 0.002 | 0 | 32 | 0 |
| `mkdir` | 1 | 0.0% | 0.171 | 0.0% | 0.606 | 0.435 | 0 | 0 | 10 | 0 | 0.000 | 0 | 1 | 0 |
| **all** | 20001 | 100.0% | 57212 | 100.0% | 58869 | 1657 | 325292 | 504 | 30395134 | 0.161 | 0.074 | 0 | 1004 | 0 |
