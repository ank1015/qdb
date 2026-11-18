# Benchmark: bigfile

- Data: 
- Size:  files in  folders, 0.0 MB of text
- Tools: 
- GNU and rg times are from the latest run (Stage 3, in memory), and include starting bash and the tool ( ms for `bash -c 'echo -n'`). Their output is captured through a pipe. rg runs on one thread with `-j 1 --no-ignore --hidden` and is there for timing only.
- Engine times are in process, on one thread, and include parsing the command. Read-only commands: median of up to 5 runs, fewer above 0.3 s; from Part 3 on, 16 runs for a command under 50 ms and 51 for one under 2 ms. Commands that change data: one run.
- Peak memory is the most heap the engine held during the command, above what it held before it.

## Loading and memory

| Run | Index | Load (ms) | Load with index build (ms) | Heap after load | Resident after load | New 64 KB file (ms) | Agrees with GNU |
|---|---|---:|---:|---:|---:|---:|---|
| Stage 3, in memory | true | 0.000 | 0.000 | 0 B | 0 B |  | NO |

## Time per command, in milliseconds

| Command | GNU | rg | Stage 3, in memory | Agree |
|---|---:|---:|---:|---|
| `grep -n qdbrareterm big.txt` | 843.7 | 0.318 | 15208.0 | true |
| `grep -c qdbrareterm big.txt` | 711.2 | 0.310 | 15973.0 | true |
| `wc -l big.txt` | 184.8 | 0.091 | 1060.0 | true |
| `tail -n 5 big.txt` | 3.305 | 0.019 | 1148.0 | true |
| `head -n 3 big.txt` | 2.364 | 0.018 | 994.0 | true |
| `echo 'one more line at the end' >> big.txt` | 1.999 | 0.043 | 1246.0 | true |
| `echo 'and a second one' >> big.txt` | 1.809 | 0.034 | 1214.0 | true |
| `echo 'and a third' >> big.txt` | 2.113 | 0.029 | 533386.0 | true |
| `sed -i 's/QDBUNIQUELINE/QDBCHANGEDLINE/' big.txt` | 3445.1 | 2.281 | 703746.0 | true |
| `grep -n QDBCHANGEDLINE big.txt` | 646.3 | 0.325 | 19132.0 | true |
| `tail -n 4 big.txt` | 2.997 | 0.024 | 1051.0 | true |
| `wc big.txt` | 14826.8 | 1920.5 | 892.0 | true |
| `cp big.txt copy.txt` | 560.4 | 6295.9 | 1201681993.0 | true |
| `cat big.txt > cat.txt` | 2273.9 | 10753.0 | 3355700014.0 | true |
| `grep -c e cat.txt` | 774.0 | 473.2 | 3411.0 | true |
| `tail -n 1 copy.txt` | 2.353 | 0.021 | 1221.0 | true |
| `rm copy.txt cat.txt` | 543.6 | 96.72 | 533071.0 | true |

## Peak memory per command

| Command | Output | Stage 3, in memory |
|---|---:|---:|
| `grep -n qdbrareterm big.txt` | 0 B | 0 B |
| `grep -c qdbrareterm big.txt` | 0 B | 0 B |
| `wc -l big.txt` | 0 B | 0 B |
| `tail -n 5 big.txt` | 0 B | 0 B |
| `head -n 3 big.txt` | 0 B | 0 B |
| `echo 'one more line at the end' >> big.txt` | 65 B | 0 B |
| `echo 'and a second one' >> big.txt` | 82 B | 0 B |
| `echo 'and a third' >> big.txt` | 52 B | 0 B |
| `sed -i 's/QDBUNIQUELINE/QDBCHANGEDLINE/' big.txt` | 65.5 KB | 0 B |
| `grep -n QDBCHANGEDLINE big.txt` | 0 B | 0 B |
| `tail -n 4 big.txt` | 0 B | 0 B |
| `wc big.txt` | 0 B | 0 B |
| `cp big.txt copy.txt` | 1113.7 MB | 40 B |
| `cat big.txt > cat.txt` | 1113.7 MB | 40 B |
| `grep -c e cat.txt` | 0 B | 0 B |
| `tail -n 1 copy.txt` | 0 B | 0 B |
| `rm copy.txt cat.txt` | 0 B | 0 B |
