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

{{SUMMARY_TARGETS}}

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

{{CONFORMANCE}}

## 3. Targets

{{TARGETS}}

## 4. The intermittent on-disk failure

The harness now writes every failing case in full, with its output and
the command line of the run, to `target/harness-failures.txt`
(`--failures FILE`). The failure that stage 5 saw now and then on disk
was an rg case: `rg -0` output piped through `cat -A`. Real rg's order
of output for a recursive search depends on its threads, so the
reference output itself changed from run to run, and the comparison,
which sorts lines when the order is not defined, could not sort output
whose "lines" are NUL-separated. Those cases now sort by path before
comparing. {{DISK200}}

## 5. A capacity sheet

{{CAPACITY}}

## 6. The concurrency tests

Every run below is on the final build unless it says otherwise; the
output of each is in `bench/results/service/`.

{{CONCURRENCY}}

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
  `--threads` the harness's counter check leaves out the 6 units of the
  benchmark list whose search a reader stops early, and says so.
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

