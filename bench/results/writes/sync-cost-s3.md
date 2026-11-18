# What a sync costs, and a small write against it, run `s3`

5000 times each. Times in milliseconds.

| | Median | 90% | 99% | Bytes written per line | Syncs |
|---|---:|---:|---:|---:|---:|
| a plain file: append the line (77 bytes), sync | 0.155 | 0.210 | 0.498 | 77 | 5000 |
| a plain file: append as many bytes as the engine writes for the line, sync | 0.152 | 0.220 | 0.424 | 617 | 5000 |
| the engine, `echo '...' >> f.txt`, sync always | 0.153 | 0.216 | 0.388 | 617 | 5000 |
| the engine, `echo '...' >> f.txt`, sync batch | 0.007 | 0.017 | 0.453 | 617 | 78 |

The engine under sync always against the bare append and sync of the same bytes: 1.01 times (median).
