# What a sync costs, and a small write against it, run `s4`

5000 times each. Times in milliseconds.

| | Median | 90% | 99% | Bytes written per line | Syncs |
|---|---:|---:|---:|---:|---:|
| a plain file: append the line (77 bytes), sync | 0.149 | 0.208 | 0.439 | 77 | 5000 |
| a plain file: append as many bytes as the engine writes for the line, sync | 0.154 | 0.295 | 0.510 | 628 | 5000 |
| the engine, `echo '...' >> f.txt`, sync always | 0.174 | 0.280 | 0.554 | 628 | 5000 |
| the engine, `echo '...' >> f.txt`, sync batch | 0.006 | 0.013 | 0.649 | 628 | 78 |

The engine under sync always against the bare append and sync of the same bytes: 1.13 times (median).
