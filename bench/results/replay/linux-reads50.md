## Linux tree, 50 in 100 units only read (20,000 units, always)

20001 units in 31.57 s (634 a second), sync always, index on. 0 ended with a status above 1. The most memory any one unit added: 1.8 MB. Times are measured and differ from run to run; every other column is counted and is the same on every run over the same database.

### By what the unit did

| What it did | Units | Share of units | CPU, ms | Share of CPU | Elapsed, ms | Not on the CPU, ms | Text read, MB | Index read, MB | Tree entries | Content written, MB | Records written, MB | Upkeep written, MB | Syncs | Scratch, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| read a little | 5525 | 27.6% | 79.8 | 0.3% | 82.7 | 2.95 | 37.6 | 0 | 80636 | 0 | 0 | 0 | 0 | 0 |
| read a lot | 85 | 0.4% | 1.18 | 0.0% | 1.18 | 0.000 | 34.0 | 0 | 808 | 0 | 0 | 0 | 0 | 0 |
| searched with the index | 1084 | 5.4% | 16983 | 57.4% | 17109 | 126 | 85386 | 252 | 10180508 | 0 | 0 | 0 | 0 | 0 |
| searched without the index | 3404 | 17.0% | 11827 | 40.0% | 11894 | 67.0 | 78751 | 0 | 5117929 | 0 | 0 | 0 | 0 | 0 |
| wrote a little | 9903 | 49.5% | 687 | 2.3% | 2392 | 1705 | 2.1 | 0.000 | 43660 | 1.6 | 0.735 | 0 | 9903 | 0 |
| **all** | 20001 | 100.0% | 29578 | 100.0% | 31479 | 1901 | 164211 | 252 | 15423541 | 1.6 | 0.735 | 0 | 9903 | 0 |

### By first command

| First command | Units | Share of units | CPU, ms | Share of CPU | Elapsed, ms | Not on the CPU, ms | Text read, MB | Index read, MB | Tree entries | Content written, MB | Records written, MB | Upkeep written, MB | Syncs | Scratch, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `grep` | 4488 | 22.4% | 28810 | 97.4% | 29003 | 193 | 164137 | 252 | 15298437 | 0 | 0 | 0 | 0 | 0 |
| `echo` | 7485 | 37.4% | 507 | 1.7% | 1809 | 1302 | 1.9 | 0 | 27611 | 1.5 | 0.575 | 0 | 7485 | 0 |
| `sed` | 1001 | 5.0% | 102 | 0.3% | 279 | 176 | 0.242 | 0.000 | 3003 | 0.075 | 0.061 | 0 | 1001 | 0 |
| `cat` | 2514 | 12.6% | 31.9 | 0.1% | 34.8 | 2.93 | 54.2 | 0 | 23620 | 0 | 0 | 0 | 0 | 0 |
| `cp` | 522 | 2.6% | 30.9 | 0.1% | 120 | 88.7 | 0 | 0 | 6682 | 0 | 0.049 | 0 | 522 | 0 |
| `rm` | 486 | 2.4% | 24.9 | 0.1% | 101 | 75.6 | 0 | 0 | 1458 | 0 | 0.022 | 0 | 486 | 0 |
| `head` | 1615 | 8.1% | 22.1 | 0.1% | 22.1 | 0.000 | 17.4 | 0 | 7624 | 0 | 0 | 0 | 0 | 0 |
| `mv` | 408 | 2.0% | 21.5 | 0.1% | 83.5 | 62.0 | 0 | 0 | 4896 | 0 | 0.027 | 0 | 408 | 0 |
| `find` | 500 | 2.5% | 14.9 | 0.1% | 14.9 | 0.001 | 0 | 0 | 45633 | 0 | 0 | 0 | 0 | 0 |
| `wc` | 981 | 4.9% | 12.0 | 0.0% | 12.1 | 0.034 | 0 | 0 | 4567 | 0 | 0 | 0 | 0 | 0 |
| `mkdir` | 1 | 0.0% | 0.158 | 0.0% | 0.729 | 0.571 | 0 | 0 | 10 | 0 | 0.000 | 0 | 1 | 0 |
| **all** | 20001 | 100.0% | 29578 | 100.0% | 31479 | 1901 | 164211 | 252 | 15423541 | 1.6 | 0.735 | 0 | 9903 | 0 |
