# qdb stage 6 brief: a service that hosts many filesystems

Nov 19, 2026 · @Ananya Khandelwal

This is the brief for stage 6 of qdb. It turns the engine into a server:
one process that holds many filesystems, each its own database, and runs
bash commands on them for many callers over an HTTP API. It is
self-contained. It has 14 numbered sections and ends with the line "End
of brief". If your copy stops earlier, ask for the full text before you
start.

## 1. Where things stand

Stage 5 is accepted. The engine now runs 36 commands and scripts with
variables and loops, and reports exactly what each unit costs.

qdb is a database engine built from scratch for text files in nested
folders, queried with bash commands. The code is at
github.com/ank1015/qdb, branch claude/project-thread-3wnfpo. Read
README.md, docs/FORMAT.md and the five hand-backs under docs/ first.

The review on 19 Nov 2026 re-ran the work on a second machine:

- **Harness.** All 5,706 cases pass in memory, on disk, and reopened
  after every step, with GNU awk 5.2.1 unpacked as you describe.

- **The storage runs you shortened, at full size.** 10,000 power cuts on
  each of two fresh seeds, 1,000 kills under each sync setting, and
  10,000 differential steps on disk: all passed.

- **Cases written from outside.** Of 72 hand-written lines over
  variables, loops, cd, diff and awk, 61 matched the real tools. The
  other 11 were refusals by design or lines the test could not compare.

- **The cost report works** on a database of the review's own, and its
  counters repeat.

The review found three faults, all for part 1:

| Fault                                         | Detail                                                                                                                                                                                                             |
|-----------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| rg --sort path on a missing path              | Real rg prints rg: data: IO error for operation on data: No such file or directory (os error 2). The engine prints the message without IO error for operation on data: . 35 of 2,000 generated cases on seed 90210 |
| xargs exit status                             | find . -name '\*.txt' -type f \| sort \| xargs -L 3 -r -x -P 1 -s 62 du -b exits 123 on GNU and 1 on the engine. Seed 90210, case 735                                                                              |
| One on-disk harness run in 22 failed one case | Not reproduced, and the failing case was not kept. Something on disk depends on timing                                                                                                                             |

A measurement that matters for this stage: a database holding one
six-byte file takes 276 KB on disk, 262 KB of it an index table, and a
process that opens it and reads the file takes about 3 ms beyond process
start.

## 2. Answers to your questions

| Question from the hand-back                                                   | Answer                                                                                                                             |
|-------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| Should awk treat non-ASCII bytes as characters, as gawk does in the C locale? | Yes. Stopping on ordinary files is worse than counting bytes. Document that length, substr, index, toupper and tolower count bytes |
| awk's exit, delete and do ... while?                                          | Add all three                                                                                                                      |
| cd -?                                                                         | Add it                                                                                                                             |
| Should a unit stopped at the command limit keep what it wrote?                | No longer. From this stage, a unit the server stops leaves nothing. Section 7 has the rule                                         |
| apply_patch: skipped or dropped?                                              | Dropped by the owner. Take it out of the plans and the README                                                                      |
| rg on standard input 1.3 times slower than real rg                            | Accepted                                                                                                                           |
| awk not using the index                                                       | Accepted                                                                                                                           |

## 3. What is being built

The owner will run qdb as a hosted service. In the owner's words: users,
through an API, create a filesystem and run these bash commands on it,
and a user can run commands only on their own filesystems.

The design decision behind this brief is that **each filesystem is its
own database**: its own directory, its own log, its own index. One
server process holds many of them.

| Why                       | What it gives                                                                                                                 |
|---------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| Isolation by construction | A unit runs inside one database. There is no path that leads to another user's files, so no permission check can be forgotten |
| Nothing shared to leak    | A shared index would mix every user's text in one set of lists                                                                |
| Faults stay local         | Damage, a full quota or a long upkeep step touches one filesystem                                                             |
| Simple lifecycle          | Deleting a filesystem removes a directory. Moving one to another machine copies a directory                                   |
| Parallelism for free      | Units on different filesystems share nothing and run on different cores                                                       |

What it costs is that a filesystem must be cheap to hold: small on disk
when it is small, quick to open, and gone from memory when idle. Today's
276 KB and 3 ms were sized for one large tree. Part 2 fixes that.

The shape of the server:

1.  A caller sends an HTTP request with a key.

2.  The server checks that the key's account owns the filesystem named.

3.  It opens that filesystem's database if it is not open, and runs the
    unit there on a worker thread.

4.  It answers with the output, the exit status and the unit's cost.

## 4. Your task

Six parts, in this order. Parts 1 to 4 are the service. Parts 5 and 6
make it faster and are the first to drop if time runs out; say so in the
hand-back if they are.

1.  **Carry-overs.** The three faults, the answers of section 2, and the
    full generated-test runs.

2.  **Many filesystems in one process.** Small ones are cheap, idle ones
    leave memory, and each has quotas.

3.  **Units from several callers at once.** Readers of one filesystem
    run together, writers take turns, and the server can stop a unit
    cleanly.

4.  **The API.** HTTP and JSON: accounts and keys, filesystems, running
    a unit, sessions, limits, and usage.

5.  **One search on several threads.**

6.  **Shared syncs** for writers that queue on one filesystem.

Out of scope:

- **More than one machine.** No replication, no moving filesystems
  between servers, no backups beyond "a closed filesystem is a directory
  you can copy".

- **TLS, billing, sign-up, a web console.** A proxy in front ends TLS.
  The owner's own product handles people and money, and drives this
  server through the admin API.

- **Uploading and downloading files outside commands.** Content goes in
  through a unit's standard input and comes out as its output.

- **New commands, flags or shell syntax,** beyond the small additions of
  part 1.

- **Sharing a filesystem between accounts,** and permissions inside a
  filesystem.

The stage is done when:

- Every harness case passes when run through the API, as well as in the
  three modes before.

- The tests of section 10 pass.

- Every target in section 11 is met, or the miss is reported with its
  numbers and cause.

- Everything in section 13 exists.

## 5. Part 1: carry-overs

- **The rg message.** Match real rg's wording for a missing path under
  --sort path, and check the other errors that mode can print.

- **The xargs exit status.** Make the case in section 1 exit as GNU
  does, and add the rules behind it to the generated family.

- **The intermittent on-disk failure.** Make the harness write every
  failing case with its output to a file. Run the on-disk mode 200
  times, find the case, and fix the cause. If it does not fail in 200
  runs, say so and leave the logging in.

- **Full generated runs on the final build:** 10,000 cases on each of 3
  new seeds for all eleven families. New seeds matter: seed 90210 found
  what seeds 1 to 3 had not.

- **awk reads any bytes,** as answered in section 2.

- **awk gains exit, delete and do ... while.** The shell gains cd -.

- **apply_patch is dropped.** Remove it from the README and from the
  lists of what is planned.

## 6. Part 2: many filesystems in one process

Build a host layer in the library, under the API and usable without it:
it creates, opens, closes and deletes filesystems by ID under one data
directory, and hands out access to them.

A filesystem must be cheap:

- **Small when it is small.** Nothing on disk is sized for a large tree
  in advance. A filesystem with a few small files takes a few kilobytes,
  and grows as its content does.

- **Quick to create and to open.** Opening reads what the filesystem
  holds, not a fixed table.

- **Out of memory when idle.** The host keeps a bounded number of
  filesystems open and a bounded total of their memory. It closes the
  least recently used when it needs room, and after a set idle time. A
  closed filesystem costs nothing but its disk.

- **No large tree made worse.** The Linux tree's figures from stage 5
  still hold.

Identity and safety:

- **IDs are made by the server,** random and long enough not to be
  guessed. A caller never supplies a name that becomes part of a path on
  the server's disk.

- **Delete is final and complete.** It waits for running units or stops
  them, then removes the directory.

- **The lock file stays.** Two server processes must not open the same
  data directory.

Quotas, checked before a write is accepted and reported in --stats:

| Quota                                        | Default              |
|----------------------------------------------|----------------------|
| Text in one filesystem                       | 10 GB                |
| Files and folders in one filesystem          | 1,000,000            |
| Disk used by one filesystem, upkeep included | Twice its text quota |
| Filesystems per account                      | 1,000                |

The defaults are settings, and each can be set per account. A unit that
would pass a quota fails with a clear error and leaves nothing.

## 7. Part 3: units from several callers at once

The one-thread rule ends here. Units run on a pool of worker threads.

Between filesystems there is nothing to coordinate: they share no data.
Inside one filesystem the rule is the simplest that is correct:

- **Many readers or one writer.** Units that only read run together. A
  unit that may write runs alone on that filesystem.

- **Which is which is decided from the unit's text, before it runs.** A
  unit is a reader if nothing in it can write: no writing command, no
  redirection to a file, and no command name that comes from an
  expansion. Anything else is a writer.

- **Waiting is fair.** A writer that is waiting is not overtaken by
  readers that arrive after it.

- **The result is as if units ran one at a time.** Every unit sees the
  filesystem as some whole number of earlier units left it, in an order
  that agrees with the order in which callers were answered.

- **Durability is unchanged.** Under always, no caller is answered with
  anything a power cut could still take back.

A server must be able to stop a unit. From this stage:

- **A unit the server stops leaves nothing.** That covers the command
  limit, a deadline, a quota, a caller who went away, shutdown, and a
  fault inside the engine. The filesystem is as it was before the unit
  began. Stage 5 kept such a unit's writes; that rule is replaced.

- **A unit that ends by itself keeps what it wrote,** whatever its exit
  status, as in bash. In a && b, a failing b does not undo a.

- **Stopping is prompt.** Loops, walks and scans check for a stop
  request often enough to end within a few milliseconds.

- **A fault in one unit stays in one unit.** A panic is caught, that
  unit is stopped, its filesystem is closed and recovered from disk, and
  the server carries on. Other filesystems notice nothing.

Keep the total bounded: a cap on units running at once, a queue with a
limit behind it, and the per-unit memory limit that already exists.

## 8. Part 4: the API

A program qdb-server listens for HTTP/1.1 and speaks JSON. It serves
plain HTTP; a proxy in front provides TLS.

| Request                                        | What it does                                                   | Who may call it |
|------------------------------------------------|----------------------------------------------------------------|-----------------|
| POST /v1/accounts                              | Create an account. Returns its ID and its key, shown this once | Admin           |
| DELETE /v1/accounts/{account}                  | Delete an account and every filesystem it owns                 | Admin           |
| PATCH /v1/accounts/{account}                   | Set the account's quotas and limits                            | Admin           |
| POST /v1/filesystems                           | Create an empty filesystem. Returns its ID                     | Account         |
| GET /v1/filesystems                            | List the account's filesystems                                 | Account         |
| GET /v1/filesystems/{fs}                       | Size, counts, disk used, and usage totals                      | Account         |
| DELETE /v1/filesystems/{fs}                    | Delete it                                                      | Account         |
| POST /v1/filesystems/{fs}/run                  | Run one unit. Returns output, exit status and cost             | Account         |
| POST /v1/filesystems/{fs}/sessions             | Start a session. Returns its ID                                | Account         |
| DELETE /v1/filesystems/{fs}/sessions/{session} | End a session                                                  | Account         |
| GET /v1/health                                 | Whether the server is up                                       | Anyone          |

Keys and ownership:

- **Every request carries a key** as a bearer token. The admin key comes
  from the server's configuration. Account keys are made by the server
  and stored only as hashes.

- **An account sees only its own filesystems.** A request for another
  account's filesystem gets the same answer as for one that does not
  exist.

Running a unit:

- **The request** holds the command text, and may hold standard input, a
  session ID, and tighter limits than the account's.

- **The answer** holds standard output, standard error, the exit status,
  the counters of stage 5, and whether anything was cut short.

- **Sessions** hold variables and the current directory between
  requests, as in stage 5. They live in memory, end after a set idle
  time, and are gone after a restart. Without a session ID, a unit
  starts fresh at /. Units of one session run one at a time.

- **Output is capped,** at 1 MB a stream by default. Past the cap the
  unit is ended early, as head ends a pipeline, and the answer says so.
  A streamed form of the same request delivers output of any size as it
  is produced.

- **Bytes that are not valid UTF-8** can appear in output. Say how the
  JSON carries them.

Limits on one unit, each a setting with a default and settable per
account:

| Limit                                 | Default   |
|---------------------------------------|-----------|
| Time                                  | 30 s      |
| Commands                              | 1,000,000 |
| Scratch space                         | 1 GB      |
| Units running at once for one account | 16        |

Errors:

- **A unit that ran** is always answered with HTTP 200, whatever its
  exit status. A unit the server stopped is also 200, with a field
  naming the reason.

- **A request that could not run** gets an HTTP error with a JSON body:
  400 malformed, 401 no valid key, 404 no such filesystem, 413 too
  large, 429 too many at once, 507 quota reached.

Usage: the server adds up the counters of every unit by filesystem and
by account, and keeps the totals across restarts. This is what the owner
will bill from.

## 9. Parts 5 and 6: one search on several threads, and shared syncs

Your cost report named the two limits: one core for searches, and one
sync for each write. Part 3 lets different units use different cores.
These two parts go further, for one caller's own work.

**Part 5: one search on several threads.** A recursive grep or rg that
must read much of a large tree splits its work by file across idle
workers.

- **Output is unchanged:** the same bytes in the same order as on one
  thread.

- **Counters are unchanged,** apart from CPU time and elapsed time.

- **It is polite.** A search takes extra threads only when workers are
  idle, and gives them back when other units arrive. The memory limit
  covers all of a unit's threads together.

- **Small searches stay on one thread.** An index answer of a few files
  gains nothing from help.

- **rg -j N** now sets the most threads that unit may use. -j 1 keeps it
  on one.

**Part 6: shared syncs.** When several writers queue on one filesystem
under always, they should not each wait for a sync of their own.

- **One sync can cover several units** that were applied one after
  another while the previous sync was running.

- **No one is answered early.** A writer is answered after the sync that
  covers its unit. A reader that saw a unit's changes is answered after
  that sync too.

- **A power cut still loses only whole units, in order,** and none that
  was answered. The crash tests must show it with several writers.

## 10. How to test a service

Everything from the earlier stages still runs. A server needs six more
kinds of test.

- **The harness through the API.** A fourth harness mode sends every
  case to a running server and compares with the real tools as before.

- **Units at once on one filesystem.** Many clients send generated reads
  and writes to the same filesystem. A checker then looks for an order
  of the units, one at a time and consistent with when each was
  answered, that explains every output. If none exists, the test fails
  and prints the units involved.

- **Killing the server.** Under load from many clients on many
  filesystems, kill the server with kill -9, start it again, and check
  every filesystem: --check passes, and every unit that was answered is
  there. Repeat as the kill test does.

- **Stopping units.** Generated units are stopped at random moments by
  deadline, by a caller going away, and by a planted panic. Each time
  the filesystem equals its state before the unit, passes --check, and
  takes the next unit.

- **Isolation.** Two accounts, each with filesystems holding known
  marker text. A generator sends requests with each key against every
  ID, guessed IDs, and paths with .., absolute paths and odd characters.
  No answer to one account may ever contain the other's markers or
  reveal that its filesystems exist.

- **A long run.** At least 30 minutes of mixed load over many
  filesystems being created, used, left idle and deleted. Memory and
  open files must be level after the first minutes, and no filesystem
  may be left locked.

Run the race-prone tests many times, and say how many. If a thread
sanitizer is available on your toolchain, run the concurrency tests
under it and report what it found.

## 11. Targets

Measured on your 4-core machine with client and server on the same host,
default settings, sync always. "The Linux filesystem" is the Linux tree
loaded as one filesystem. Stage 5 figures are from docs/HANDBACK-5.md.

| Part | What is measured                                                       | Target                                                                        |
|------|------------------------------------------------------------------------|-------------------------------------------------------------------------------|
| 1    | Generated tests, all eleven families, 3 new seeds                      | 10,000 each with no unexplained difference                                    |
| 1    | On-disk harness, 200 runs                                              | No failing run, or the cause found and fixed                                  |
| 2    | Disk used by a filesystem holding one small file                       | Under 16 KB. Today 276 KB                                                     |
| 2    | Opening a small filesystem that is not in memory                       | Under 0.2 ms. Today about 3 ms                                                |
| 2    | Creating a filesystem                                                  | Under 1 ms plus its syncs                                                     |
| 2    | 100,000 small filesystems on one server, or as many as the disk allows | All created. With all idle, the server's own memory under 200 MB              |
| 2    | The Linux filesystem: open time, own memory, read figures              | No worse than stage 5 by the counters                                         |
| 3    | The 95%-read workload on the Linux filesystem from 4 clients           | At least 3 times the units a second of 1 client. Stage 5: 339 a second        |
| 3    | 64 filesystems, one client each, mixed workload                        | At least 3 times the total of one filesystem alone                            |
| 3    | Small reads on one filesystem while another runs a long search         | 99th percentile under twice what it is alone                                  |
| 3    | A unit stopped by its deadline                                         | Answered within 50 ms of the deadline, nothing left behind, filesystem usable |
| 3    | The units-at-once checker                                              | 1,000,000 units over the runs with every output explained                     |
| 4    | Every harness case through the API                                     | All pass                                                                      |
| 4    | true through the API on one connection                                 | Median under 0.2 ms                                                           |
| 4    | true through the API from 8 connections                                | At least 10,000 a second                                                      |
| 4    | Server killed under load                                               | 200 kills, every filesystem recovered with every answered unit                |
| 4    | Isolation test                                                         | 1,000,000 requests, no leak                                                   |
| 4    | The long run                                                           | 30 minutes, memory and open files level                                       |
| 5    | grep -rn define . on the Linux filesystem, idle server                 | Under 0.4 of its one-thread time, same output                                 |
| 6    | 8 writers on one filesystem                                            | At least 2.5 times the units a second of 1 writer                             |

## 12. Rules that stay in force, and the ones that change

Three rules change:

- **Threads are allowed.** The engine's results must not depend on how
  many there are.

- **A unit the server stops leaves nothing.** Section 7.

- **Crates for the network edge are allowed:** an HTTP library, a JSON
  library, and what they need. Prefer small and plain ones, and keep the
  engine itself free of them. An async runtime is not needed; worker
  threads are enough.

The rest stand:

- **Same output as the real tool, byte for byte.** A unit gives the same
  output through the API as in process.

- **Fail loudly.** Unsupported commands, flags and syntax are refused by
  name.

- **The engine is from scratch.** No database, storage, index, search or
  regex-matching crate.

- **Data model unchanged.** Accounts, keys, quotas and usage belong to
  the server, not to any filesystem's data.

- **Storage guarantees unchanged.** A unit is all or nothing after a
  crash, an answered unit is on disk under always, and memory per unit
  is bounded.

- **The format may change** to make small filesystems cheap. Raise the
  version, refuse older databases with a clear message, and update
  docs/FORMAT.md.

- **The C locale for the GNU commands and awk,** and UTC for every time.

- **Keep the record in the repository.** Commit this brief under
  docs/briefs/.

## 13. What to hand back

Write docs/HANDBACK-6.md, and the two documents the owner will run the
service from. Write items 2, 3 and 6 for a reader who has not seen the
code.

1.  **Code** on the branch, with README.md brought up to date.

2.  **docs/API.md.** Every request and answer with an example, every
    error, every limit and its default, and how to use sessions and
    streamed output.

3.  **docs/OPERATIONS.md.** How to start and stop the server, every
    setting, what is on disk and how to copy it safely, what the logs
    say, and what to do after a crash.

4.  **Conformance.** Harness totals in the four modes, the
    generated-test totals and seeds, and what the 200 on-disk runs
    found.

5.  **Targets.** The table from section 11 with a measured value and met
    or missed for each row.

6.  **A capacity sheet.** For one machine like yours: what a filesystem
    costs on disk and in memory when small, when large, when idle and
    when open; units a second by workload and by number of clients; and
    which limit is reached first as accounts, filesystems and requests
    grow.

7.  **The concurrency tests.** Runs, seeds, units checked, kills, and
    every fault they found with its fix.

8.  **Security notes.** What the isolation test covers, what it does
    not, and anything a caller could do to slow or harm other accounts.

9.  **For the stage after this.** What would have to change to run on
    more than one machine, and what readers that do not wait for a
    writer would take.

10. **Deviations, gaps and questions.**

## 14. Assumptions to confirm

The owner has confirmed the hosted service, that a user may only touch
their own filesystems, and that apply_patch is dropped. These eight are
still assumptions. Follow them unless told otherwise.

- **Each filesystem is its own database.** The owner was unsure whether
  it should be one shared database; this is the designer's choice, for
  the reasons in section 3.

- **The API is HTTP and JSON,** with TLS left to a proxy.

- **Accounts and keys are built into the server** in the simple form of
  section 8. The owner's own product sits in front and uses the admin
  key.

- **Files go in and out through commands only.** There is no upload or
  download request.

- **A unit the server stops leaves nothing,** which replaces the stage 5
  rule.

- **Inside one filesystem, a long search can make a write wait.**
  Readers that never block a writer are left for later.

- **The defaults of sections 6 and 8** for quotas and limits.

- **One machine.**

End of brief.
