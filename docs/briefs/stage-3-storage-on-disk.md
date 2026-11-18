# qdb stage 3 brief: storage on disk

Nov 18, 2026

This is the brief for stage 3 of qdb: keep the data on disk, survive a
crash, and work within a memory limit, without giving back the read
speed of stage 2. It is self-contained. It has 14 numbered sections and
ends with the line "End of brief". If your copy stops earlier, ask for
the full text before you start.

## 1. Where things stand

Stage 2 is accepted: reads are fast in memory, and every claim checked
in review held.

qdb is a database engine built from scratch for text files in nested
folders, queried with bash commands. The code is at
github.com/ank1015/qdb, branch claude/project-thread-3wnfpo. Read
README.md, docs/HANDBACK.md and docs/HANDBACK-2.md there first.

The review on 18 Nov 2026 re-ran the work on a second machine:

- **Correctness.** All 1,617 harness cases pass with the index on and
  off. The differential test is identical over 10,000 steps on seed 1
  and on a fresh seed, 77. All 26 unit tests pass.

- **Speed.** On the git source tree (50.1 MB), grep -rl xdl_merge . took
  605 ms in stage 1. It now takes 5.6 ms with no index and 0.05 ms with
  it, against 60 ms for GNU grep and 29 ms for rg.

What the engine still lacks is the subject of this stage. The figures
are from your hand-back, on the Linux tree (1,397 MB, 83,405 files):

| Gap                           | Today                                                                                                       |
|-------------------------------|-------------------------------------------------------------------------------------------------------------|
| Nothing is saved              | The data lives in memory and is gone when the process exits                                                 |
| Starting is slow              | 8.2 s every time: 1.4 s to read the files and 6.8 s to build the index                                      |
| Memory is the whole dataset   | 1,578 MB resident for 1,397 MB of text                                                                      |
| One write can stall           | A renumbering rebuilds the whole index, 6 to 8 s                                                            |
| A write stores the whole file | Appending one line re-reads and re-indexes the file, and sed -i writes a new copy even when nothing changed |
| Large files weaken the index  | A term in 84 files yields 366 candidates, among them 16 MB headers, and costs 17.5 ms                       |

## 2. The priorities this stage serves

Read speed comes first, then write speed, then disk, with memory as a
fixed limit. This order was in a part of the stage 1 brief that never
reached you: the copy committed under docs/briefs/ has no section on
priorities and stops soon after "Behaviour details".

| Priority | Goal                  | What it means                                                                                             |
|----------|-----------------------|-----------------------------------------------------------------------------------------------------------|
| 1        | Read speed            | Searching and listing are the most common operations. Stage 2 won this, and stage 3 must not give it back |
| 2        | Write speed           | Writes stay fast, but each one may do extra work to keep reads fast and data safe                         |
| 3        | Disk, within a budget | Disk is the cheapest resource. It may be spent, up to a ceiling                                           |
| Limit    | Memory                | The engine must work when the data is larger than RAM. Memory is a cache, not a requirement               |

Stage 2 only paid for the first row. This stage is where the other three
are first paid for. When two goals conflict and this brief does not
settle it, this order does.

## 3. Answers to your questions

| Question from the hand-back                                    | Answer                                                                                                                                                                                                      |
|----------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Is a write that sometimes waits for a full rebuild acceptable? | It was until now. Part 5 removes it                                                                                                                                                                         |
| Should the 10% rule apply to writes?                           | No. It was meant for reads, and the stage 2 brief contradicted itself by also saying write costs had no target. Stage 2 counts as fully met. Your idea of indexing a write later becomes a switch in part 6 |
| Should other commands ask the index?                           | Yes, in part 1                                                                                                                                                                                              |
| Is ASCII-only folding still right if the locale changes?       | The C locale stays. Record in the index files which folding rule built them, so a later change means a rebuild and not a broken format                                                                      |
| Deviations you listed                                          | All accepted: release tarballs with checksums, the dataset sizes, no index for -v or for patterns with no trigram, and the 2 MB of scratch                                                                  |
| Gaps you listed                                                | Part 1 closes them: data that is not source code, large files, and a write run long enough to trigger every kind of upkeep                                                                                  |

## 4. Your task

Put the database on disk in six parts, in this order. Keep a benchmark
run after each part, as in stage 2. If time runs out, drop from the end:
part 6 goes first, and say so in the hand-back.

1.  **Carry-overs.** Small fixes and benchmark additions that stage 2
    pointed to.

2.  **The on-disk format.** A database is a directory that can be closed
    and opened again.

3.  **Crash safety.** A crash loses at most what the chosen setting
    allows, and never leaves a broken database.

4.  **Memory limit and large files.** Content is read from disk as
    needed, and no command needs a whole large file in memory.

5.  **Upkeep without pauses.** No write waits for work the size of the
    database.

6.  **Switches and the comparison.** Three design choices become
    settings, and one table shows what each combination costs.

Out of scope:

- Several users, several processes or a server. One process opens a
  database at a time, and a lock enforces it.

- Threads. Upkeep runs in bounded steps on the one thread.

- New commands, flags or shell syntax, and any change of locale.

- Moving the folder tree to disk. The tree and the table of trigrams may
  stay in memory. Content and file lists may not.

The stage is done when:

- The harness passes in memory, on disk, and on disk with the database
  closed and reopened after every step.

- The crash tests in part 3 pass.

- Every target in section 11 is met, or the miss is reported with its
  numbers and cause.

- Everything in section 13 exists.

## 5. Part 1: carry-overs

Do these in memory, before any storage work, so their effect is measured
on its own.

- **A write that changes nothing writes nothing.** Today src/cmd/sed.rs
  calls set_content for every file sed -i is given, changed or not. With
  no modification time in the data model, skipping an unchanged file is
  invisible to the user, and on disk it saves a whole copy. Apply the
  same rule to \>, \>\> and cp when the result equals what is there.

- **More commands ask the index.** sed -i over many files skips the
  files that cannot contain a match, since they cannot change. grep over
  a list of files named on the command line does the same. Both use the
  question storage already answers.

- **Data that is not source code.** Add one dataset of at least 100 MB
  of prose or logs from whatever your machine can reach. The RFC text
  packages in the Ubuntu archive are one candidate, if they are there.

- **Large files.** Add benchmark cases on a single generated file of 1
  GB: append a line, search for a rare term, wc -l, tail -n 5, and sed
  -i that changes one line.

- **A long write run.** Add a run of 100,000 small writes mixed with
  occasional large ones, long enough to trigger every kind of upkeep.
  Record the time of every write, not only the median.

## 6. Part 2: the on-disk format

A database is a directory. qdb --db PATH opens it, or creates it if it
is not there. --load DIR together with --db imports a folder into an
empty database. Running without --db stays in memory, as today, and
remains the reference for speed.

The format must have these properties:

- **Content is never changed in place.** New content is added and old
  content is retired, as documents already are in the stage 2 index.

- **Opening reads no content.** Open time follows the number of files
  and the amount of log to replay, not the bytes of text.

- **The index is stored packed.** Lists sit end to end, numbered so that
  neighbours in the tree stay neighbours in a list. Stage 2 measured
  1.05 bytes per posting that way, 8.4% of the text.

- **The index stays derived data.** If an index file is missing or
  damaged, rebuild it from content at open and say so. Damaged content
  is reported as an error that names the file. Neither is ever silently
  wrong.

- **Every file is self-describing.** A magic number and a format version
  at the start, a checksum on every record or block, and a fixed byte
  order. A database written by a newer version is refused with a clear
  error.

- **Node IDs persist.** A file keeps its ID across close and open.

- **One process at a time.** A lock in the directory makes a second
  opener fail with a clear error.

- **Room for part 6.** Each stored piece of content carries a marker for
  how it is encoded, so compression can be added without a new format.

This is the shape I expect. You may choose another if it gives the same
properties, and say why:

1.  A log that records each change as it happens.

2.  Segment files that are never modified once written, each holding
    content and the part of the index that covers it.

3.  A small file that names the live segments and is replaced in one
    atomic step.

4.  A saved copy of the folder tree, so opening does not replay the
    whole history.

Merging segments is then the place where retired documents are dropped
and documents renumbered, which is the work stage 2's sweeps and
rebuilds did.

Also deliver:

- **docs/FORMAT.md.** The exact layout of every file, precise enough
  that another program could read a database.

- **--check.** Verifies every checksum and every invariant, including
  that the stored index equals a fresh rebuild.

- **--stats.** Disk bytes by component: text, lists, tables, tree, log,
  and space not yet reclaimed.

- **--compact.** Finishes all pending upkeep and leaves the database at
  its smallest.

- **Three harness modes.** In memory, on disk, and on disk with the
  database closed and reopened after every step. The differential test
  also runs on disk.

## 7. Part 3: crash safety

After any crash, the database opens and holds exactly the result of some
number of whole command lines, in the order they ran.

The guarantees:

- **All or nothing for each command line.** A crash never leaves half of
  one, whether the line wrote one file or the 9,698 files of a cp -r.

- **Two settings for when a write is safe.** With --sync always, a
  command line's changes are on disk when it returns. With --sync batch,
  changes are synced in groups. A crash may then lose the most recent
  lines, but never an earlier line while keeping a later one. State the
  exact bound on what batch can lose.

- **Recovery is automatic.** Opening after a crash repairs the database
  and reports what it did.

- **Never silently wrong.** An unfinished tail of the log is discarded.
  Damage anywhere else is an error that names the file.

- **Reads write nothing.** A command that only reads causes no disk
  write and no sync.

How to prove it:

- **One I/O layer.** Every file operation goes through one small layer,
  so a test can replace the disk.

- **Simulated power loss.** The test layer tracks what has been written
  and what has been synced. At a crash point it drops what was not
  synced, and may keep any part of it, in any order. Then the database
  is reopened.

- **What must hold after each crash.** --check passes, and the content
  equals the state after some prefix of the command lines. Under always,
  the prefix includes every line that had returned.

- **Coverage.** Crash at I/O calls throughout random write sequences
  from the differential test's generator: at least 10,000 crash points
  on each of 3 seeds. Include crashes during merges, during a save of
  the tree, and during recovery itself.

- **Real kills.** Kill the real process with kill -9 at random moments
  at least 1,000 times, reopening and checking each time.

- **Known traps.** Sync the directory after creating or renaming a file.
  Replace a file by writing a new one and renaming it. Treat a failed
  sync as fatal, never as something to retry.

## 8. Part 4: memory limit and large files

The engine's own memory must not grow with the amount of text, or with
the size of any one file, for reads and for writes.

- **Content and file lists come from disk as needed.** Mapping the files
  or reading them through a cache are both acceptable. Say which you
  chose and why.

- **What may stay in memory.** The folder tree and the table of
  trigrams. On the Linux tree those are about 30 MB and 8 MB.

- **What counts as the engine's own memory.** Memory the system cannot
  take back: on Linux, the anonymous part of resident memory. File pages
  the system is caching do not count.

Large files need three things the current design cannot give:

- **Appending costs what is added.** Adding a line to a 1 GB file must
  not rewrite or re-index 1 GB.

- **Editing costs what changes.** A sed -i that changes one line of a 1
  GB file writes about that much, and never holds the file in memory.

- **Searching reads only the parts that can match.** This also fixes the
  weak filter that large files gave the index in stage 2.

The way I expect this to be done:

1.  Store a file as a sequence of pieces, each ending on a line
    boundary. Small files are one piece.

2.  Make each piece its own document in the index.

3.  Keep a line count with each piece, so grep -n, wc -l, head and tail
    can skip pieces they do not need.

4.  Let a single line longer than the piece limit be one oversized
    piece.

The piece size is a setting. Measure two or three sizes and choose one.

How to prove it:

- **A hard cap.** Run the whole benchmark on the Linux tree with the
  process capped at 512 MB. If your machine cannot set such a cap, say
  so and report the peak of the engine's own memory.

- **The 1 GB file cases** from part 1, under the same cap.

- **Cold reads.** If you can evict the database's files from the system
  cache, report first-read times too.

## 9. Part 5: upkeep without pauses

No single write may do work in proportion to the size of the database.
Today a renumbering rebuilds the whole index while one write waits.

- **Bounded steps.** Merging segments, dropping retired content,
  renumbering and saving the tree all run in small steps on the one
  thread, each step charged to a write. --compact runs them all to the
  end.

- **Disk stays within its ceiling while this happens.** Retired content
  takes space until a merge removes it, so the pace of merging decides
  the peak. The targets are in section 11.

- **Reads stay correct and fast between steps.** A search gives the same
  answer whatever state the merging is in, and the read targets hold
  with several segments live.

- **Write amplification.** Over the long write run, report the bytes
  written to disk for each byte of content written.

## 10. Part 6: switches and the comparison

Three design choices become settings, so the owner can see what each
costs before choosing. The purpose of this part is the comparison table,
not a winner picked by you.

| Switch                      | Settings                                                                               | What it trades                            |
|-----------------------------|----------------------------------------------------------------------------------------|-------------------------------------------|
| When a write is synced      | always, or batch                                                                       | Write speed against what a crash can lose |
| When new content is indexed | At the write, as in stage 2. Or later, in bulk, when recent writes move into a segment | Write speed against the slowest search    |
| How text is stored          | Plain, or compressed                                                                   | Disk against read speed                   |

Notes on each:

- **Sync.** Both settings exist from part 3.

- **Indexing later.** Until recent content is indexed, a search must
  scan it. It is correct either way, and the amount that can be waiting
  is bounded by a setting. Stage 2's figures show no saving per posting
  from building in bulk: about 30 ns at a write, and about 60 ns during
  load (111.4 million postings in 6.8 s). So any gain comes from content
  that is replaced or deleted before it is ever indexed, and from
  keeping index work out of the write's own time. Measure whether that
  is worth a slower worst-case search.

- **Compression.** One codec, applied to each piece on its own. A
  compression crate is allowed. Report its ratio and its speed in both
  directions on each dataset.

The comparison:

- **Eight combinations** of the three switches.

- **Three workloads:** 95% reads, an even mix, and 95% writes. Build
  them from the benchmark's commands with a fixed seed, and describe
  them exactly.

- **Two datasets:** PostgreSQL (161 MB) and Linux (1,397 MB).

- **For each run:** commands per second, median and 99th-percentile time
  for reads and for writes separately, disk size at the end, and the
  peak of the engine's own memory.

Deliver one table per dataset, and one sentence per workload saying
which combination did best and what it gave up. Recommend defaults if
you see them, and mark them as recommendations.

## 11. Targets

Unless a row says otherwise: the Linux tree, one thread, the system's
file cache warm, text stored plain, and new content indexed at the
write. "Settled" means after --compact. Stage 2 figures are from
docs/HANDBACK-2.md.

| Part | What is measured                                                  | Target                                                                                              |
|------|-------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------|
| 2    | Harness in memory, on disk, and on disk reopened after every step | All cases pass in all three                                                                         |
| 2    | Open after a clean close                                          | Under 200 ms, and following the number of files, not the bytes of text. Stage 2 took 8.2 s to start |
| 2    | Import with --load into --db                                      | At most twice stage 2's 8.2 s                                                                       |
| 2    | Every read-only benchmark command, settled                        | Within 1.5x of its stage 2 in-memory time, or within 0.05 ms for commands under 0.1 ms              |
| 2    | Disk size, settled                                                | At most 1.2x the raw text                                                                           |
| 3    | Simulated power loss                                              | 10,000 crash points on each of 3 seeds, every one recovering to a prefix with --check clean         |
| 3    | Real kill -9                                                      | 1,000 kills, every one recovering                                                                   |
| 3    | Syncs per command line under always                               | One in the usual case. Report the distribution                                                      |
| 3    | A small write under always                                        | Within 1.2x of one bare append-and-sync of the same bytes on the same disk. Report both             |
| 3    | A small write under batch                                         | Median under 0.1 ms. Stage 2 took 0.017 ms in memory                                                |
| 3    | Open after a crash, default settings                              | Under 2 s                                                                                           |
| 4    | The engine's own memory after open                                | Under 200 MB. Stage 2 held 1,578 MB                                                                 |
| 4    | The whole benchmark under a 512 MB cap                            | Completes with identical results. Report the times                                                  |
| 4    | Append one line to a 1 GB file, batch                             | Under 1 ms, and disk written follows the piece size                                                 |
| 4    | sed -i changing one line of a 1 GB file                           | Writes a few pieces, and stays under the cap                                                        |
| 4    | Search for the term in 84 files                                   | Under 5 ms. Stage 2 took 17.5 ms                                                                    |
| 5    | Slowest single small write in the 100,000-write run, batch        | Under 50 ms, and no larger on the Linux tree than on PostgreSQL                                     |
| 5    | Disk size at any moment of that run                               | At most 2x the raw text then live                                                                   |
| 6    | The comparison                                                    | Both tables delivered                                                                               |

## 12. Rules that stay in force

Nothing in this stage changes what a command returns.

- **Same output as GNU, byte for byte.** All 1,617 harness cases keep
  passing, in every mode and with every switch setting.

- **Fail loudly.** Anything unsupported is refused with a clear error
  before any stage runs. The same goes for a database that cannot be
  opened safely.

- **From scratch.** No database, key-value store, storage engine, index
  or search crate. Small low-level crates are allowed: system calls and
  file mapping, checksums, memchr, and one compression codec.

- **Data model unchanged.** Pieces, documents and segments are storage
  detail. The index is derived data.

- **Commands reach data only through the storage layer.** Change that
  interface as the work needs, for example to read content piece by
  piece.

- **One thread, and the C locale.**

- **Keep the record in the repository.** Commit this brief under
  docs/briefs/.

## 13. What to hand back

Write docs/HANDBACK-3.md and docs/FORMAT.md, and commit the result files
beside the earlier ones. The owner will choose default settings from
items 9 and 10, so those need the most care.

1.  **Code** on the branch, with README.md updated for the new options.

2.  **docs/FORMAT.md,** as described in part 2.

3.  **Conformance.** Harness totals in all three modes, and the
    differential test's totals on disk.

4.  **Crash tests.** Crash points, seeds and kills run, and every fault
    they found along the way with its fix.

5.  **Benchmark tables,** one per dataset. For each command: GNU, stage
    2 in memory, on disk settled, on disk straight after the write list,
    and cold if you could measure it.

6.  **Targets.** The table from section 11 with a measured value and met
    or missed for each row.

7.  **Disk.** Bytes by component, settled and at the peak of the long
    write run.

8.  **Memory.** The engine's own memory after open and at its peak, and
    the result of the capped run.

9.  **Write costs.** Every write command under each sync setting, the
    bare cost of a sync on your disk, the spread of times in the long
    write run, and write amplification.

10. **The comparison.** The two tables and three sentences from part 6.

11. **For the stage after this.** What in the storage design would have
    to change for several readers and writers at once.

12. **Deviations, gaps and questions.**

## 14. Assumptions to confirm

The owner has not confirmed these six. Follow them unless told
otherwise.

- **The folder tree stays in memory.** At about 30 MB for the Linux
  tree's 83,405 files, that sets a practical ceiling of a few million
  files. Report the bytes per file so the ceiling is known.

- **Every command line is all or nothing after a crash.** A plain
  filesystem does not promise this.

- **always is the default sync setting** until the owner chooses from
  the part 6 comparison.

- **The C locale stays.**

- **One process opens a database at a time.**

- **Disk ceilings of 1.2x settled and 2x at peak.** The 2x figure began
  as an example in the stage 1 brief and has not been fixed.

End of brief.
