## Linux tree, 5 in 100 units only read (20,000 units, always)

20001 units in 6.30 s (3172 a second), sync always, index on. 0 ended with a status above 1. The most memory any one unit added: 0.973 MB. Times are measured and differ from run to run; every other column is counted and is the same on every run over the same database.

### By what the unit did

| What it did | Units | Share of units | CPU, ms | Share of CPU | Elapsed, ms | Not on the CPU, ms | Text read, MB | Index read, MB | Tree entries | Content written, MB | Records written, MB | Upkeep written, MB | Syncs | Scratch, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| read a little | 529 | 2.6% | 9.34 | 0.2% | 9.34 | 0.000 | 3.4 | 0 | 7348 | 0 | 0 | 0 | 0 | 0 |
| read a lot | 7 | 0.0% | 0.183 | 0.0% | 0.183 | 0.000 | 0.754 | 0 | 62 | 0 | 0 | 0 | 0 | 0 |
| searched with the index | 113 | 0.6% | 2024 | 52.3% | 2038 | 14.3 | 9917 | 27.1 | 1455930 | 0 | 0 | 0 | 0 | 0 |
| searched without the index | 356 | 1.8% | 678 | 17.5% | 678 | 0.531 | 4385 | 0 | 290536 | 0 | 0 | 0 | 0 | 0 |
| wrote a little | 18996 | 95.0% | 1159 | 30.0% | 3487 | 2328 | 4.2 | 0.000 | 82171 | 3.0 | 1.4 | 0 | 18996 | 0 |
| **all** | 20001 | 100.0% | 3870 | 100.0% | 6212 | 2342 | 14310 | 27.1 | 1836047 | 3.0 | 1.4 | 0 | 18996 | 0 |

### By first command

| First command | Units | Share of units | CPU, ms | Share of CPU | Elapsed, ms | Not on the CPU, ms | Text read, MB | Index read, MB | Tree entries | Content written, MB | Records written, MB | Upkeep written, MB | Syncs | Scratch, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `grep` | 469 | 2.3% | 2701 | 69.8% | 2716 | 14.8 | 14302 | 27.1 | 1746466 | 0 | 0 | 0 | 0 | 0 |
| `echo` | 14442 | 72.2% | 851 | 22.0% | 2630 | 1779 | 3.7 | 0 | 53074 | 2.9 | 1.1 | 0 | 14442 | 0 |
| `sed` | 1969 | 9.8% | 186 | 4.8% | 444 | 258 | 0.495 | 0.000 | 5907 | 0.160 | 0.121 | 0 | 1969 | 0 |
| `cp` | 886 | 4.4% | 44.9 | 1.2% | 145 | 100 | 0 | 0 | 11282 | 0 | 0.086 | 0 | 886 | 0 |
| `rm` | 942 | 4.7% | 42.2 | 1.1% | 148 | 106 | 0 | 0 | 2826 | 0 | 0.042 | 0 | 942 | 0 |
| `mv` | 756 | 3.8% | 35.5 | 0.9% | 119 | 83.6 | 0 | 0 | 9072 | 0 | 0.051 | 0 | 756 | 0 |
| `cat` | 238 | 1.2% | 4.15 | 0.1% | 4.15 | 0.000 | 2.5 | 0 | 2244 | 0 | 0 | 0 | 0 | 0 |
| `head` | 150 | 0.7% | 2.46 | 0.1% | 2.46 | 0.000 | 1.7 | 0 | 680 | 0 | 0 | 0 | 0 | 0 |
| `find` | 42 | 0.2% | 1.49 | 0.0% | 1.49 | 0.000 | 0 | 0 | 3985 | 0 | 0 | 0 | 0 | 0 |
| `wc` | 106 | 0.5% | 1.43 | 0.0% | 1.43 | 0.000 | 0 | 0 | 501 | 0 | 0 | 0 | 0 | 0 |
| `mkdir` | 1 | 0.0% | 0.198 | 0.0% | 0.764 | 0.566 | 0 | 0 | 10 | 0 | 0.000 | 0 | 1 | 0 |
| **all** | 20001 | 100.0% | 3870 | 100.0% | 6212 | 2342 | 14310 | 27.1 | 1836047 | 3.0 | 1.4 | 0 | 18996 | 0 |
