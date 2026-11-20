# Stage 6 hand-back: a service that hosts many filesystems

## Summary

qdb is now a service. `qdb-server` keeps many filesystems under one data
directory, each owned by one account, and runs bash command lines on
them for many callers at once over HTTP and JSON. The engine underneath
is the stages 1 to 5 engine, now shared between threads.

What was built, by part of the brief:

1. **Carry-overs.** rg's single-thread wording for a missing path, the
   xargs exit status after a failed command, awk on any bytes, awk
   `exit`, `delete` and `do ... while`, `cd -`. The intermittent
   on-disk failure was found and fixed (section 4). apply_patch is
   gone from the README and the plans.
2. **A host layer** (`src/host.rs`): filesystems by random 128-bit ID,
   opened when a unit needs one, closed when idle or when too many are
   open, deleted by way of a trash folder. Format 3 makes a small
   filesystem small: 12 KB on disk for one file, where stage 5 took
   276 KB, and it opens in 0.05 ms.
3. **Units at once.** Readers together and a writer alone on each
   filesystem, decided from the line's text before it runs, let in
   fairly in order of arrival. A unit the server stops (time limit,
   command limit, quota, caller gone, shutdown, a fault) leaves nothing:
   a writer's changes are journalled and taken back exactly.
4. **The API** (`qdb-server`, docs/API.md): accounts and keys, filesystems,
   units with standard input and tighter limits, sessions, capped or
   streamed output, usage totals kept across restarts. HTTP, JSON,
   SHA-256 and base64 are written in the crate; it has no new
   dependencies.
5. **One search on several threads.** A recursive `grep` or `rg` that
   has read 4 MB borrows idle workers and writes their results in
   order. `rg -j N` limits it.
6. **Shared syncs.** Writers on one filesystem let it go before their
   sync, and one sync covers every unit logged by then.

Of the 20 targets in the brief, 18 are met and 2 are missed (section 3):
a recursive `grep` on the Linux filesystem with idle workers helping
takes 0.53 of its one-thread time against a target of 0.4, and the
95%-read workload with 4 clients gives 2.7 times one client on one
thread against a target of 3 (1.5 times one client that has the idle
workers' help). The search is held back by its one thread writing 821 MB
of output in order, and the read workload by searches and writers
waiting for each other at the filesystem's gate; section 3 says more.

## 1. Code

The branch is `claude/project-thread-s39doz`, on top of stages 1 to 5.

| File | What it is |
|---|---|
| `src/host.rs` | Many filesystems in one process: create, open on demand, close when idle or over the limits, delete; run a unit with its deadline watched; a fault closes the filesystem and opens it again from disk |
| `src/gate.rs` | The gate of one filesystem: readers together, a writer alone, first come first served, and a unit stopped while it waits leaves the queue |
| `src/unit.rs` | Stopping a unit: the stop request, the points where the engine looks at it, the memory and scratch limits over all of a unit's threads, and the planted panic of the stop test |
| `src/group.rs` | Group commit |
| `src/par.rs` | One search on several threads |
| `src/service.rs` | Accounts, keys, quotas, sessions, usage, the requests |
| `src/net/` | HTTP/1.1 (`http.rs`), JSON (`json.rs`), SHA-256, base64, hex and constant-time comparison (`codec.rs`) |
| `src/bin/server.rs` | `qdb-server` |
| `src/bin/load.rs` | `qdb-load`: load, and the service's tests (bench/README.md) |
| `tests/api.rs`, `tests/host.rs`, `tests/units.rs` | The service, the host and stopped units, in `cargo test` |

How the parts fit, for a reader who has not seen the code: a request
arrives on a connection thread, which checks the key and the
filesystem's owner, then waits for one of the server's workers (a
counted slot; there are 2 per core). The unit then asks the
filesystem's gate to let it in, as a reader or a writer. Inside, the
engine runs the line as in stage 5. A writer's changes go to the
filesystem's log as they are made, with an undo journal beside them; if
the unit is stopped, the journal puts everything back and the log
records nothing. A writer that finishes logs its unit, lets the gate
go, and waits for a sync that covers it before the answer is sent. A
watchdog thread stops units at their deadlines, and another watches
each connection so that a caller who hangs up stops its unit.

## 2. Conformance

**Harness.** 5,760 cases (6,094 steps; stage 5 had 5,706), all passing
in each mode on the final build:

| Mode | Result | Run |
|---|---|---|
| In memory | 5,760 passed; counters of the 224 benchmark units identical on a second run and none above the baseline | `harness-modes-final.txt` |
| On disk, sync always | 5,755 passed in each of the 98 runs of the 200 made after the change to the xargs case (section 4); the 5 xargs cases added since pass in the other modes | `disk200.txt` |
| On disk, closed and reopened after every step | 5,760 passed | `harness-modes-final.txt` |
| Through the API (`--store api`), the fourth mode | 5,760 passed | `harness-modes-final.txt` |
| In memory, every recursive search on 4 threads from its first file (`--threads 4`) | 5,760 passed; counters as above, less the read-ahead units (section 9) | `harness-modes-final.txt` |
| In memory, index off | 5,760 passed; counters as above | `harness-modes-final.txt` |

New cases this stage cover the carry-overs (rg's wording, xargs's
status, awk on any bytes, `exit`, `delete`, `do ... while`, `cd -`).
The service's own behaviour is tested in `tests/api.rs`, `tests/host.rs`
and `tests/units.rs` and by the tests of section 6, not by harness
cases, since the harness compares with the real tools.

**Generated tests,** 10,000 cases on each of 3 new seeds for every
family, on the final build:

| Family | Seed 601 | Seed 602 | Seed 603 |
|---|---|---|---|
| awk | 10,000 passed | 10,000 passed | 10,000 passed |
| diff | 10,000 passed | 10,000 passed | 10,000 passed |
| find | 10,000 passed | 10,000 passed | 10,000 passed |
| grep | 9,997 passed, 3 known | 9,997 passed, 3 known | 9,995 passed, 5 known |
| ls | 10,000 passed | 10,000 passed | 10,000 passed |
| rg | 10,000 passed | 10,000 passed | 10,000 passed |
| scripts | 10,000 passed | 10,000 passed | 10,000 passed |
| sed | 10,000 passed | 10,000 passed | 10,000 passed |
| shell | 10,000 passed | 10,000 passed | 10,000 passed |
| text | 10,000 passed | 10,000 passed | 10,000 passed |
| xargs | 10,000 passed | 10,000 passed | 10,000 passed |

330,000 cases, none failing. The first pass over these seeds found two
things, both fixed or explained before the final run (the first pass is
in `generated.txt`, the final grep and xargs runs in
`generated-after-fixes.txt`):

- **xargs `-L` and `-l` imply `-x`,** as GNU documents: when a batch
  of so many lines would pass the `-s` size, GNU stops with `argument
  list too long` and status 1 instead of running a shorter batch. A
  later `-n` takes the implied `-x` back. The engine ran the shorter
  batch. 9 failures over the three seeds; fixed, with new cases.
- **GNU grep with `-o`, `-v`, `-x` and `-m`** forgets `-x` in the
  context after the last selected line, and prints parts of lines the
  pattern does not match whole (`printf 'a\nz1 b\n' | grep -v -o -x
  -m1 -A1 z1` prints `z1`). This is a defect of the real tool; the
  engine keeps `-x`. It is now the second kind of known difference
  (tests/README.md), 1 case. The other 10 known differences are the
  `-o -w -x` defect listed in stage 5.


## 3. Targets

Measured on this 4-core machine (15 GB of memory, one virtual disk),
client and server on the same host, default settings, sync always, on
the final build. The output of every run is in `bench/results/service/`
under the name in the last column.

| Part | What is measured | Target | Measured | Result | Run |
|---|---|---|---|---|---|
| 1 | Generated tests, all eleven families, 3 new seeds | 10,000 each, no unexplained difference | 330,000 cases, none failing. 11 known differences, all GNU grep defects listed in tests/README.md; the first pass found an xargs difference (`-L` implies `-x`), now fixed (section 2) | met | `generated.txt`, `generated-after-fixes.txt` |
| 1 | On-disk harness, 200 runs | No failing run, or the cause found and fixed | 197 of 200 passed every case; the 3 failures were all one xargs case whose real output order varies, now split into cases that do not depend on it (section 4) | met: cause found and fixed | `disk200.txt` |
| 2 | Disk used by a filesystem holding one small file | Under 16 KB (stage 5: 276 KB) | 12.0 KB in disk blocks, directory included; 298 bytes of files | met | `small-100k.txt` |
| 2 | Opening a small filesystem not in memory | Under 0.2 ms (stage 5: about 3 ms) | 0.035 ms median in process; through the server, a unit on a closed filesystem takes 0.135 ms longer than on an open one | met | `open-small.txt`, `small-100k.txt` |
| 2 | Creating a filesystem | Under 1 ms plus its syncs | 0.137 ms median where syncs cost nothing (tmpfs); 3.9 ms on this disk with its 5 syncs | met | `create.txt` |
| 2 | 100,000 small filesystems, all idle | All created; server's own memory under 200 MB | 100,000 created in 78 s from 8 connections; 98.3 MB resident with all closed, 10 open files | met | `small-100k.txt` |
| 2 | The Linux filesystem: open time, own memory, read figures | No worse than stage 5 by the counters | Open 30.4 ms median (stage 4's figures, which stage 5 kept: 28.2 and 32.1 ms in two runs); heap once open 21.3 MB (21.7 MB); the counter baseline of 224 units, 0 above it | met | `open-linux.txt`, `harness-modes-final.txt` |
| 3 | The 95%-read workload on the Linux filesystem, 4 clients against 1 | At least 3 times; stage 5: 339 a second | 1 client on one thread: 572 a second. 1 client with idle workers helping its searches: 1,032. 4 clients: 1,525, which is 2.7 times one thread and 1.5 times one client with help, and 4.5 times stage 5 | missed | `reads95-*.txt` |
| 3 | 64 filesystems, one client each, mixed | At least 3 times one filesystem alone | 43,530 units a second against 9,787: 4.4 times | met | `mixed-*.txt` |
| 3 | Small reads while another filesystem runs a long search | 99th percentile under twice alone | 0.125 ms against 0.131 ms alone, while `grep -rn define . \| wc -l` ran 48 times | met | `interfere-long.txt` |
| 3 | A unit stopped by its deadline | Within 50 ms, nothing left, usable | 440 units of 11 kinds given 100 ms: median 0.9 ms after the deadline, 99th percentile 14.6 ms, most 39.7 ms; nothing left after any, and a write after each | met | `deadline.txt` |
| 3 | The units-at-once checker | 1,000,000 units, every output explained | 100 runs × 10,000 units from 16 connections, 1,000,000 units (20,038 stopped at a deadline), every output explained | met | `order.txt` |
| 4 | Every harness case through the API | All pass | 5,760 of 5,760 | met | `harness-modes-final.txt` |
| 4 | `true` through the API on one connection | Median under 0.2 ms | 0.054 ms median, 0.109 ms 99th percentile; 15,289 a second | met | `true-1.txt` |
| 4 | `true` through the API from 8 connections | At least 10,000 a second | 93,543 a second | met | `true-8.txt` |
| 4 | Server killed under load | 200 kills, every filesystem recovered with every answered unit | 200 kills with SIGKILL under 16 connections on 8 filesystems; 753,563 units, 566,219 answered writes, every one found after every kill | met | `kill.txt` |
| 4 | Isolation test | 1,000,000 requests, no leak | 1,000,000 requests from 8 connections, no leak | met | `isolation.txt` |
| 4 | The long run | 30 minutes, memory and open files level | 2,108 s from 8 connections: 9.26 million units, 57,411 filesystems made and 54,413 deleted. Resident memory 160 to 186 MB after the first minutes (most 184 MB in the first 7 minutes, 186 MB after); open files 2,185 to 2,235; every one of the 2,998 filesystems left took a write at once at the end | met | `long.txt`, `long-samples.tsv` |
| 5 | `grep -rn define .` on the Linux filesystem, idle server | Under 0.4 of its one-thread time, same output | 324 ms median against 614 ms with one worker: 0.53. Same output (5,372,557 lines) | missed | `all.txt` |
| 6 | 8 writers on one filesystem | At least 2.5 times 1 writer | 12,187 units a second against 3,659: 3.3 times, with 53,692 syncs for 182,834 units. Without group commit, 8 writers make 3,092 a second | met | `writers-*.txt` |

**The two missed rows.**

*The 95%-read workload.* The target was set against one thread per
client, as in stage 5. Part 5 changed that: one client's large searches
now borrow the idle workers, so one client already uses most of the
machine for 43 units in 100. Against one thread (`--workers 1`), 4
clients give 2.7 times; against one client as it runs today, 1.5 times.
What stops 4 clients going further is the filesystem's gate: 5 units in
100 write, and each write waits for every search running on the
filesystem to finish (up to 40 ms), while every search that arrives
after the writer waits for it. With 8 clients this queueing costs more
than the extra clients bring (942 a second). Readers that do not wait
for a writer (section 8) are what would lift it.

*grep -rn define . on several threads.* 0.53 of one thread, short of
0.4. The search finds 5.4 million lines, 821 MB of output, and that
output has to be written in order by the unit's own thread, through the
pipe to `wc`; the helpers search ahead, but the unit's thread spends
most of the time copying their results out, so 4 threads give 1.9
times. Memory bandwidth was ruled out (4 threads reach 21 GB/s here);
samples show the helper threads mostly busy with less work each as
more run.


## 4. The intermittent on-disk failure

The harness now writes every failing case in full, with its output and
the command line of the run, to `target/harness-failures.txt`
(`--failures FILE`). The failure that stage 5 saw now and then on disk
was an rg case: `rg -0` output piped through `cat -A`. Real rg's order
of output for a recursive search depends on its threads, so the
reference output itself changed from run to run, and the comparison,
which sorts lines when the order is not defined, could not sort output
whose "lines" are NUL-separated. Those cases now sort by path before
comparing.

The 200 on-disk runs on the final build found one more case of the
same kind. 197 runs passed every case; 3 failed one case each, all
`printf '%s\n' a 'b"' | xargs cat; echo $?`. Real xargs stops at the
unmatched quote after starting `cat a`, and the two write their errors
to the same standard error at once: here 62 runs in 300 put cat's
message first, and one run put xargs's message inside cat's line. The
engine's order is fixed. The case is now two: the status with standard
error dropped, and xargs's own message picked out with `grep -o`. The
case was changed while the runs went on, so the first two failures are
on its first form and the third on its second (sorted lines); the
the 98 runs that started after the last change all passed. The
harness's other cases did not fail once in 200 runs.

## 5. A capacity sheet

For one machine like this one: 4 cores, 15 GB of memory, one virtual
disk on which a sync costs 0.1 to 1.3 ms. Default settings, sync always.

**What a filesystem costs.**

| | Small (one file of a few bytes) | Large (the Linux tree: 83,405 files, 1.4 GB of text) |
|---|---|---|
| On disk | 12 KB in blocks: a directory, MANIFEST, one data file and LOCK (298 bytes of files) | 1.55 GB, 1.11 times its text |
| In memory when closed | Nothing in the host; about 1 KB in the server for its owner, usage totals and registry entry (100,000 idle filesystems: 98 MB resident) | The same 1 KB |
| Opening | 0.035 ms; 0.135 ms more for the first unit through the server | 30 ms |
| In memory when open | 14 KB of heap, 2 to 3 open files | 21 MB of heap, plus its index files mapped (the server went from 5 MB to 57 MB resident on opening it) |
| Making one | 0.14 ms of work and 5 syncs: 3.9 ms on this disk | `qdb-load import` loads the tree in 26 s |

The server closes a filesystem 60 s after its last unit, and the least
recently used when more than 1,024 are open or open ones hold more than
2 GB. So with the defaults memory for open filesystems is bounded by
about 2 GB, which is about 90 filesystems the size of the Linux tree,
or every one of 1,024 small ones (14 MB).

**Units a second, by workload and clients.**

| Workload | 1 client | More clients |
|---|---:|---:|
| `true` (the cost of a request) | 15,289 (0.054 ms median) | 93,543 from 8 |
| Small writes on one filesystem | 3,659 | 12,187 from 8 (3,092 without group commit) |
| 95 reads to 5 writes on small filesystems, one each | 9,787 on 1 | 43,530 on 64 |
| 95 reads to 5 writes on the Linux filesystem | 572 on one thread; 1,032 with idle workers helping | 1,525 from 4; 942 from 8 |
| `grep -rn define .` on the Linux filesystem (5.4 million lines out) | 1.6 a second on one thread, 3.1 with helpers | |
| Small reads beside a long search on another filesystem | 13,000 alone, 99th percentile 0.13 ms | unchanged with the search running |

**Which limit is reached first.**

- *As requests grow:* the CPU. Small units cost 0.05 to 0.2 ms each of
  server time, so the machine answers 40,000 to 90,000 small units a
  second before anything else matters. Searches over large trees use
  the CPU fastest: a search for a common word over the Linux tree takes
  0.3 to 0.6 s of all four cores.
- *On one filesystem:* the gate, then the sync. Writers on one
  filesystem run one at a time; group commit lets them share syncs, so
  8 writers reach 12,000 a second where one reaches 3,700. Readers are
  held up by a writer and the writer by long readers; in a workload
  with large searches this is what stops more clients helping (942 a
  second from 8 clients on the Linux filesystem, below 1,525 from 4).
- *As filesystems grow:* nothing in memory until millions. Idle
  filesystems cost about 1 KB each in the server, so 1,000,000 would
  take about 1 GB; small ones take 12 KB each on disk (12 GB for a
  million) and 5 syncs each to make (about 250 a second from one
  connection, 1,300 a second from 8, on this disk). The registry file
  grows by about 100 bytes for each one made, and is rewritten at start.
- *As accounts grow:* the same per-filesystem costs; an account itself
  costs a few hundred bytes. Workers are shared by all accounts with no
  fairness between them (section 7), so the first limit two busy
  accounts reach is each other.
- *Open files:* each open filesystem holds 2 to 3 files and each
  connection one, so the defaults may need about 4,000; most systems
  allow 1,024 to a process unless raised (docs/OPERATIONS.md).


## 6. The concurrency tests

Every run below is on the final build unless it says otherwise; the
output of each is in `bench/results/service/`.

| Test | Runs and seeds | What was checked | Result | Run |
|---|---|---|---|---|
| Units at once on one filesystem (`qdb-load order`) | 100 runs of 10,000 units from 16 connections, seeds 1000 to 1099 | 1,000,000 units, 20,038 of them stopped at a deadline: an order of the units, one at a time and consistent with when each was answered, explains every output; stopped ones left nothing | passed | `order.txt` |
| The order checker's self-test | `QDB_ORDER_MUTATE=1`, seed 9 | One answer changed on purpose | caught, with the units on the cycle printed | `order-mutate.txt` |
| Killing the server (`qdb-load kill`) | 200 kills with SIGKILL, seed 7, 16 connections, 8 filesystems | 753,563 units, 566,219 answered writes; after each kill every filesystem opened, passed `--check`, and held every answered unit whole and in order | passed | `kill.txt` |
| Stopping units (`qdb-harness --stops`) | 3 × 2,000 units, seeds 1, 2, 3 | Units stopped at random moments: 324 at a deadline, 248 by the caller going away, 253 by a panic planted in the engine; after each the filesystem equalled a reference that ran only the units that finished, and passed its check | passed | `stops.txt` |
| Power cuts with several writers (`qdb-harness --crash-writers`) | 1,000 cuts with 8 writers (seed 2), 1,000 with 2 (seed 3) | Every cut recovered to the units in the order they were applied up to some point, with every answered unit; 3.4 and 1.4 units a sync | passed | `crash-writers.txt` |
| Isolation (`qdb-load isolation`) | 1,000,000 requests, seed 1, 8 connections | Section 7 | no leak | `isolation.txt` |
| The isolation test's self-test | `QDB_ISOLATION_MUTATE=1`, 5,000 requests, seed 2 | One request in a hundred with the other account's key | caught (10 leaks found) | `isolation-mutate.txt` |
| The long run (`qdb-load long`) | 35 minutes, 8 connections, up to 3,000 filesystems, seed 5, on the final build | Memory and open files sampled every 10 s, level after the first minutes; at the end every filesystem left must take a write at once | passed: memory 184 MB at most in the first 7 minutes and 186 MB after, open files 2,235 at most after the first minutes, all 2,998 filesystems left took a write. Three earlier runs: the first found the usage log's growth and was stopped for the fix; the second stopped at 23 minutes on a fault of the test (a unit stopped by a delete taken for a missed deadline); the third (seed 4) was level for its 35 minutes, 160 to 181 MB and 2,185 to 2,230 open files, but its last check failed on another fault of the test: its first connection had been idle longer than the server keeps one | `long.txt` |
| Every harness case on 4 threads (`qdb-harness --threads 4`) | All cases, seed 77 for the generated grep and rg families | Output unchanged with every recursive search on 4 threads from its first file | 5,760 passed; 3,000 + 3,000 generated passed | `harness-modes-final.txt`, `threads-generated.txt` |
| `cargo test` | Run 8 times in this stage's last day | Includes the service tests: a caller going away, a client that stops reading, limits, sessions, restarts, a damaged registry | passed | |
| Thread sanitizer | Once: nightly Rust with `-Zsanitizer=thread` and the standard library rebuilt with it | `cargo test` (lib, api, host, units, stream, disk, share); `qdb-harness --threads 4`, `--stops 300`, `--crash-writers 200`; against a sanitized server: the order checker (3 × 2,000 units), isolation (20,000), mixed and writers (20 s each), and a 2-minute long run | no report; one test (`a_caller_going_away_stops_its_unit`) failed under it, by timing: it read the usage before the stopped unit had been counted, which happens just after its filesystem is let go. The test now waits for the count | `tsan.txt` |

**Faults the tests found, and their fixes.**

- *The usage log grew without end* (the long run): every second it
  appended each changed total, and was rewritten only at start, 84 MB
  in four minutes under load. It is now written whole again once what
  was appended is several times its size.
- *A unit with a deadline was answered 250 ms late* (`qdb-load
  deadline`): sort's three-way split of a few million lines did not look
  for a stop request. It now does before each large group.
- *A streamed unit whose client stopped reading held its worker, and a
  writer its filesystem, past its deadline* (found while writing the
  security notes, then shown by a test): the output went out with a
  send that waited for the client. Sends no longer wait; between them
  the unit's stop request and a 10-second limit on no progress are
  checked. `tests/api.rs` has the case.
- *A damaged or missing registry would have removed every filesystem*
  (found while writing docs/OPERATIONS.md): unreadable lines were
  skipped, and filesystems no account owned were removed at start. Now
  only a last line cut short by a crash is dropped, and anything else
  stops the server before it changes anything. `tests/api.rs` has the
  case.
- *The intermittent on-disk failure* (section 4), and a second one the
  200 runs found: real xargs and the cat it starts write their errors
  in either order.
- *Sessions had no cap* (the security review): now 1,000 an account.
- The tests' own faults: the long run took a unit stopped because
  another connection deleted its filesystem for a missed deadline, and
  stopped after 23 minutes; it then made its last check on a connection
  idle for 35 minutes, which the server had closed; the order checker failed on a read of a file
  not yet written (`cat` exits 1). Both checkers were fixed and run
  again.


## 7. Security notes

**What the isolation test covers.** Two accounts, each with three
filesystems holding a marker string in a plain file, a deep one, a
hidden one and one with an odd name. One million requests, from 8
connections, each with one of the two keys: reads, deletes, sessions,
units and account reads, against the caller's own filesystems, the other
account's, random IDs, the other's ID in capitals, near misses (one
character changed), and paths with `..` that climb from one ID to
another. The units include lines that reach for what lies outside a
filesystem: `/..`, `../../..`, `find /`, `grep -r MARKER /`, `rg
MARKER /`, `$HOME`, `/proc/self/environ`, `/etc/passwd`, and the
server's own files by relative path. Every answer is checked for the
other account's marker, any of its filesystem IDs, its account ID, its
key and its session ID; and every request that names one of the other
account's filesystems must get, byte for byte, the answer a request for
a random ID that does not exist gets. A self-test
(`QDB_ISOLATION_MUTATE=1`) sends one request in a hundred with the
wrong key and is caught at once.

Why it holds: a filesystem's path on disk is made from its ID only after
the ID has been checked to be 32 lowercase hexadecimal characters and
owned by the caller; a caller never supplies any part of a path on the
server's disk. Inside a unit, every path is resolved within the
filesystem's own tree, whose root is its own `/`; there is no route from
a command to the real filesystem, the network or other processes. The
check of ownership comes before anything that could take a different
time for a filesystem that exists.

**What it does not cover.**

- Timing. The answer for another account's filesystem is the same as
  for a missing one, and both come from the same map lookup, but the
  test does not measure times.
- The keys themselves. Account keys are 256 random bits from
  `getrandom` and kept as SHA-256 hashes; the admin key is compared in
  constant time. Nothing limits how fast a caller may guess, so a proxy
  should limit failed requests.
- The network. There is no TLS; keys cross the wire in the clear unless
  a proxy in front adds it.
- Bugs in the engine that a unit could use to read outside its tree.
  There is no such route that we know of, and the harness and the
  isolation units try the obvious ones; the engine is not a sandbox
  that has been reviewed as one.

**What a caller can do to slow or harm other accounts.**

- **Take every worker.** An account may run 16 units at once by
  default and the server has 8 workers on this machine, so one account
  can keep every worker busy for up to its time limit (30 s), and other
  accounts' units wait in the queue. Set `--units-at-once` well below
  `--workers` when accounts do not trust each other; fair sharing of
  workers between accounts is not built.
- **Use memory.** Each unit may hold 1 GB (all its threads together),
  so 8 workers may hold 8 GB at once. Lower `--memory-bytes` on a
  smaller machine.
- **Use disk.** Quotas are per filesystem (20 GB of disk by default) and
  an account may have 1,000 filesystems, so one account could fill the
  disk long before any quota stops it; then every account's writes fail.
  An account-wide disk quota is not built. A unit's scratch (sort's
  runs) is limited to 1 GB.
- **Hold connections.** The server takes 1,024 connections in all and
  closes one only after 5 minutes of silence; one caller can hold them
  all. A request body is read whole (up to 16 MB) before it is checked,
  so 1,024 large requests could ask for 16 GB of memory. Put a proxy in
  front that limits connections and body size per client, and lower
  `--max-request-bytes` if units never need large standard input.
- **Slow one filesystem.** A long writer holds its own filesystem; that
  harms only the account that owns it.

Fixed in this stage after the tests found them: a streamed unit whose
client stopped reading held its worker (and a writer its filesystem)
past its deadline, since the output went out with a send that waited for
the client; now it is stopped once its deadline passes or the client
has taken nothing for 10 seconds. Sessions had no cap; an account may
now hold 1,000.

## 8. For the stage after this

**More than one machine.** Each filesystem is already a directory that
nothing else refers to, so filesystems can be spread over machines by
ID with little change to the engine. What would have to change:

- *Who owns what* lives in one registry file on one machine. It would
  move to a small replicated store (or a database the owner's product
  already runs) that maps each filesystem to the machine that holds it,
  with a router in front that sends each request there.
- *Moving a filesystem* between machines needs a way to copy one while
  it is in use: stop new writers, copy the directory, switch the map,
  let them in on the new machine. A copy of a closed filesystem is
  already consistent; an open one needs its gate held for the copy.
- *Usage totals* are kept per server; they would be added up across
  machines, which the per-filesystem lines already allow.
- *Durability against the loss of a machine* would need each filesystem's
  log sent to a second machine before a unit is answered. The log is
  already a stream of whole units in order, which is what replication
  needs, and group commit already gathers units into batches.

**Readers that do not wait for a writer.** Today a reader waits for a
writer running on its filesystem, and a writer for the readers. To let
readers go on while a writer runs, each reader would read the
filesystem as of the last committed unit while the writer changes a
newer version. The storage layer is half way there: file content is
never changed in place (a write adds pieces and a new piece list), so
old content stays readable until upkeep reclaims it. What it would take:

- The tree (names, folders, piece lists) in a form that a reader can
  hold a snapshot of: either copy-on-write nodes or a version number on
  each change, so that a reader sees the tree as of its start.
- Upkeep that does not reclaim a piece while any reader that started
  before its removal is still running.
- The in-memory index treated the same way, or readers that use only
  the index files and scan what is newer.
- Counters and costs per reader, which are already per thread.

The gain would be largest for the mixed workloads, where a write now
waits for searches of several milliseconds to finish and searches wait
for it.

## 9. Deviations, gaps and questions

**Deviations from the brief.**

- **Counters with several threads.** Output is always the same bytes.
  The counts are the same, but for two cases. A search whose reader stops
  it early (`grep -rn x . | head -n 1`, and `-q`, `-m`, `-l`) may have
  read a few files ahead on other threads, so `text_bytes`,
  `text_pieces` and `tree_nodes` can be higher than on one thread. And
  `memory` is what the unit's own thread held: buffers on helper threads
  count toward the unit's memory limit but not toward that figure. Under
  `--threads` the harness's counter check leaves out the memory column
  and the 29 units of the benchmark list whose line has a reader that
  can stop a search early, and says so; the other 195 units count the
  same on several threads as on one.
- **`syncs` depends on timing.** With group commit, a unit whose
  changes were made durable by another unit's sync reports 0 syncs, and
  the unit that made the sync reports 1. Summed over units, `syncs` is
  the number of syncs made. Stage 5's rule that every count repeats
  exactly holds for every other count.
- **A unit's time limit starts when it gets a worker,** not when the
  request arrives; time waiting at its filesystem's gate counts, time
  in the server's queue does not. A caller who goes away while its unit
  is queued is dropped from the queue.
- **A key of the wrong kind gets 401,** with a message naming the kind
  needed, where 403 would also fit. The brief lists 401 for "no valid
  key".
- **Answers carry one field the brief did not ask for:** `kind`, whether
  the unit ran as a reader or a writer.

**Gaps.**

- No fair sharing of workers between accounts (section 7).
- No account-wide disk quota (section 7).
- The registry is written whole only at start; between starts it grows
  by one line per account or filesystem made or deleted (about 100
  bytes each).
- No way to move one filesystem to another server (docs/OPERATIONS.md).
- The thread sanitizer was run on the tests listed in section 6, not on
  the 30-minute run.

**Questions.**

1. Should a unit's time limit include its time in the server's queue?
   Counting it would make an overloaded server stop units it never ran;
   not counting it means a caller can wait longer than its limit.
2. Should `units_at_once` default below `workers`, so that one account
   cannot take every worker? A default of 4 on an 8-worker server would
   leave room for others at the cost of one account's peak.
3. Is an account-wide disk quota wanted, and should it replace the
   per-filesystem one or sit beside it?
4. For billing, `memory_max` in the usage is the most any one unit held.
   Would a sum of unit-seconds of memory serve better?

