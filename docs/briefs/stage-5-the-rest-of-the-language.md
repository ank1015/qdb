# qdb stage 5 brief: the rest of the language, and what each command costs

Nov 19, 2026 · @Ananya Khandelwal

This is the brief for stage 5 of qdb. It finishes the owner's language
list (variables, control flow, a current directory, diff, apply_patch
and awk) and makes the engine report what every command costs. It is
self-contained. It has 14 numbered sections and ends with the line "End
of brief". If your copy stops earlier, ask for the full text before you
start.

## 1. Where things stand

Stage 4 is accepted: the engine runs 34 commands and a real slice of the
shell, and every check repeated in review held.

qdb is a database engine built from scratch for text files in nested
folders, queried with bash commands. The code is at
github.com/ank1015/qdb, branch claude/project-thread-3wnfpo. Read
README.md, docs/FORMAT.md and the four hand-backs under docs/ first.

The review on 19 Nov 2026 re-ran the work on a second machine and added
tests of its own:

- **Harness.** All 4,689 cases pass in memory, on disk, and reopened
  after every step.

- **Generated tests, a fresh seed.** 3,000 cases for each of the eight
  families on seed 7771: none failed, and 2 hit the known GNU grep
  defect.

- **Cases written from outside.** Of 153 hand-written command lines over
  the new syntax, flags, rg and the new commands, 147 matched the real
  tools. Five of the rest are refusals of things outside stage 4, and
  one is a link that the test itself could not compare.

- **Crash tests, fresh seeds.** 3,000 power cuts on each of two seeds,
  100 real kills, and 5,000 differential steps on disk: all recovered,
  all identical.

- **Shared copies on a real tree.** After cp -r of 992 files and an edit
  to the copy, the source was untouched, searches listed both sides, and
  --check passed.

Two things are still missing:

- **The rest of the owner's language list.** Variables, control flow, a
  current directory, diff, apply_patch and awk.

- **A way to see what work costs.** The owner wants to know how disk,
  memory and CPU are used by each kind of command. Your question 4
  points the same way: this machine cannot repeat a time to 5%, but it
  can repeat a count of bytes and syncs exactly.

## 2. Answers to your questions

| Question from the hand-back                                              | Answer                                                                                                                            |
|--------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------|
| Keep du totals current at every write, or remember until the next write? | Remembering is enough. 6 to 8 ms after a write, against 150 ms for GNU, needs no storage change                                   |
| Is 2.6 ms for the 1 GB copy acceptable?                                  | Yes. The two storage changes wait for the stage that brings several users, which reworks the tree anyway                          |
| May xargs and rg on standard input hold their input in memory?           | Taking the whole input first is fine. Holding it in memory without bound is not: past 8 MB it goes to the scratch file, in part 1 |
| Would a rule on bytes logged and syncs serve in place of the 5% rule?    | Yes. Part 1 builds the counters and section 11 uses them. The stage 4 row counts as met                                           |
| Is awk wanted whole, or the common forms?                                | The owner's list fixes it: section 9. It is more than {print \$N} and less than POSIX                                             |
| The 133 changed cases, the masked columns, sleep, the GNU grep defect    | All accepted as you reported them                                                                                                 |
| grep -o where matches are dense                                          | Fix it in part 1. Reads come first, and sed and awk will lean on the same matcher                                                 |

## 3. Your task

Six parts, in this order. If time runs out, drop from the end and say so
in the hand-back. awk is last because it is the largest and stands
alone.

1.  **Carry-overs and the cost report.** Three fixes from stage 4, then
    counters that say exactly what each command read, wrote and synced.

2.  **Sessions, variables and the current directory.** Parameters,
    assignment, arithmetic, ~, cd and pwd.

3.  **Control flow.** for, if, while, until, break, continue and read.

4.  **diff.**

5.  **apply_patch.** The patch format coding agents use to edit files.

6.  **awk.** The subset on the owner's list.

Sections 5 to 9 list exactly what is in. Anything not listed stays
refused by name.

Out of scope:

- **Not planned at all:** case, select, functions, arrays, \[\[ \]\],
  subshells, (( )), process substitution, background &, positional
  parameters, cd -, and ln.

- **A later stage of their own:** several users, threads, and the two
  storage changes for the 1 GB copy.

- **Not in this stage:** every other item in section 6 of your stage 4
  hand-back, such as sed's hold space, rg -o, and the small text tools.

The stage is done when:

- The harness passes in memory, on disk, and on disk reopened after
  every step, with cases for everything in sections 4 to 9.

- The generated tests in section 10 show no unexplained difference from
  the real tools.

- Every target in section 11 is met, or the miss is reported with its
  numbers and cause.

- Everything in section 13 exists.

## 4. Part 1: carry-overs and the cost report

Do the cost report before the language work, so every later part is
measured with it.

Carry-overs:

- **Held input goes to scratch.** xargs, rg and grep reading standard
  input, and any command given -, may take their whole input first. Past
  8 MB that input waits in the scratch file, as redirected output does.

- **grep -o where matches are dense.** Teach the fast matcher to report
  where a match ends, so grep -o and sed 's/.../.../g' stop falling back
  to the slow one. Today grep -o '\[a-z\]\*ing' over the 1 GB file takes
  twice GNU's time.

The cost report makes the engine say what each unit of work used. It has
three pieces.

**Counters, kept for every unit and exact:**

| Counter      | What it counts                                                     |
|--------------|--------------------------------------------------------------------|
| Text read    | Bytes of file content the unit looked at, and the number of pieces |
| Index read   | Bytes of index lists read, and the number of lookups               |
| Tree visited | Files and folders the unit walked                                  |
| Written      | Bytes added to the data files, split into content and records      |
| Syncs        | Calls that waited for the disk                                     |
| Scratch      | Bytes that went to the scratch file                                |
| Memory       | The most the engine's own memory rose during the unit              |

Two more are measured, and so vary from run to run: CPU time and elapsed
time.

**How a caller sees them:**

- The library returns the counters with each result.

- qdb --cost prints one line per unit to standard error, in a fixed
  format that a script can read.

- The counters cost nothing on disk: no log bytes and no syncs.

**A replay tool for the owner's question.** qdb-replay runs a file of
units against a database and prints, for each kind of work, its share of
the units, of CPU time, of text read, of bytes written and of syncs.
Group the units two ways: by first command, and by what they did (read a
little, read a lot, searched with the index, searched without it, wrote
a little, wrote a lot). The owner will run it on real logs. For the
hand-back, run it on the three workloads of the stage 3 comparison and
on the benchmark's command list.

**The rule that replaces 5%.** Store the counters of every benchmark
command as a baseline in the repository. A harness run fails when a
command reads, writes or syncs more than its baseline, unless the
baseline is updated in the same commit with a reason.

## 5. Part 2: sessions, variables and the current directory

Variables and a current directory need somewhere to live between
commands. That place is a session. The reference for behaviour is bash
5.2.21.

A session:

- **Holds the variables and the current directory,** and nothing else.

- **Lives in memory and belongs to the caller.** It is never stored in
  the database and does not survive a restart. A caller that wants state
  to last keeps the session and passes it to each execute call.

- **Starts the same way every time:** current directory /, HOME=/,
  PWD=/, and OLDPWD unset.

- **In the command-line program,** all -c commands of one run share a
  session, and so do all units read from standard input.

In this stage:

| Feature           | Forms                                                                                                                                 |
|-------------------|---------------------------------------------------------------------------------------------------------------------------------------|
| Parameters        | \$NAME, \${NAME}, \$?, \${NAME:-word}, \${NAME:=word}, \${#NAME}. An unset variable is empty                                          |
| Assignment        | NAME=value, and NAME=value cmd, which applies to that command only                                                                    |
| Built-ins         | export NAME and export NAME=value, unset NAME and unset -v NAME                                                                       |
| Tilde             | ~ and ~/path, from HOME                                                                                                               |
| Arithmetic        | \$((expr)) on 64-bit integers: + - \* / % \*\*, comparisons, && and \|\|, bitwise operators, ?:, assignment, ++ and --, and the comma |
| Field splitting   | On the default separators, or on an IFS of ASCII characters                                                                           |
| Current directory | cd with no argument, cd DIR, pwd, pwd -L and pwd -P. PWD and OLDPWD are kept up to date                                               |

Rules:

- **Relative paths resolve from the current directory, in every
  command.** Output shows paths as the user wrote them, as the real
  tools do. A path that starts with / is from the root of the database.

- **Each stage of a pipeline gets its own copy of the variables and the
  current directory.** What a stage changes there is gone when it ends.
  Changes to files are shared.

- **Expansions happen in bash's order:** braces, tilde, parameters with
  arithmetic and command substitution, field splitting, globs, then
  quote removal.

- **Still refused:** every other \${...} form, \$@, \$#, \$\$, \$!, \$1
  and the like, and cd -.

- **An earlier decision is reversed on purpose.** The stage 1 brief said
  there is no current directory. There is one now, and it starts at the
  root, so every existing test keeps its meaning.

## 6. Part 3: control flow

| Construct                   | Forms                                                                       |
|-----------------------------|-----------------------------------------------------------------------------|
| Loop over words             | for NAME in WORDS; do ...; done. A for with no in is refused                |
| Condition                   | if ...; then ...; elif ...; else ...; fi                                    |
| Loop on a test              | while ...; do ...; done and until ...; do ...; done                         |
| Leaving a loop              | break, break N, continue, continue N                                        |
| Reading a line              | read, read -r, with zero or more names. With no name the line goes to REPLY |
| Redirections on a construct | Allowed on for, if, while and until, applied left to right                  |

Rules:

- **One call is still one unit.** A loop that writes 10,000 files is all
  or nothing after a crash, and costs one sync under always.

- **A unit cannot run forever.** With one thread, an endless loop would
  stop the database. A unit that runs more than a set number of commands
  is stopped with a clear error. The limit is a setting, 1,000,000 by
  default. What the unit wrote before the stop stays, as after any
  failed command.

- **A loop that reads input while it writes** takes its whole input
  first, as xargs does: find ... \| while read f; do sed -i ... "\$f";
  done. Past 8 MB the input waits in the scratch file.

- **find -exec and xargs can run any command except** cd, export, unset,
  read, break, continue and :.

- **Checking before running.** Until now a unit with anything
  unsupported ran nothing. That still holds for whatever the text of the
  unit shows: syntax, and command names and flags written out. A name or
  flag that only appears after expansion is refused when it is reached,
  and what ran before it stays.

## 7. Part 4: diff

The reference is the GNU diff on your machine, in the C locale with
times in UTC. Record its version; the review machine has diffutils 3.10.

| Kind                 | In this stage                                                     |
|----------------------|-------------------------------------------------------------------|
| Output               | The default format, and unified with -u, -U N and --unified\[=N\] |
| Whole trees          | -r, -N and --new-file, -x PAT and --exclude PAT                   |
| Less output          | -q and --brief, -s and --report-identical-files                   |
| What counts as equal | -i, -w, -b, -B                                                    |
| Other                | -a (accepted, no effect), --label                                 |
| Exit status          | 0 for no differences, 1 for some, 2 for trouble                   |

Requirements:

- **The same edit script as GNU.** Two files can differ in many equally
  short ways, and GNU makes particular choices. The generated tests in
  section 10 decide whether you match them.

- **Headers carry file times,** in UTC with nanoseconds, from the
  modification times added in stage 4. --label replaces them.

- **Identical without reading.** Two files that share every piece are
  equal, and two that share most pieces differ only where the pieces
  differ. diff -r of a folder against its copy must cost by what
  changed, not by the size of the tree.

- **Line numbers come from the piece counts,** so a change deep in a
  large file does not read the pieces before it.

- **Memory.** Two large files that share most of their pieces must be
  compared inside the memory limit. For two unrelated files larger than
  the limit, either spill to scratch or refuse with a clear message, and
  say which you chose.

- **-r walks in sorted order,** which is what GNU does too, so its
  output compares exactly.

## 8. Part 5: apply_patch

apply_patch edits files from a patch in the format used by OpenAI's
Codex. There is no reference program on your machine, so this section is
the specification. It was written from the Codex source, read through a
mirror: the
[parser](https://forge.lthn.ai/core/core-agent-ide/src/commit/6c254ca3e7bb7e30cbcc51dfc26c9591de09460f/codex-rs/apply-patch/src/parser.rs)
and the [code that applies a
patch](https://forge.lthn.ai/core/core-agent-ide/src/commit/e290d482647f01bf05b8a6dc01715ee15fd1c1e0/codex-rs/apply-patch/src/lib.rs).

The patch is one argument, or standard input when there is no argument.

| Line                                 | Meaning                                                                                                                                                   |
|--------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
| \*\*\* Begin Patch                   | First line of every patch                                                                                                                                 |
| \*\*\* Add File: PATH                | Start a new file. Each following line starts with + and is a line of its content                                                                          |
| \*\*\* Delete File: PATH             | Remove a file                                                                                                                                             |
| \*\*\* Update File: PATH             | Change a file. One or more chunks follow                                                                                                                  |
| \*\*\* Move to: NEWPATH              | Optional, straight after an update line: the changed file is written to the new path and the old one removed                                              |
| @@ or @@ TEXT                        | Start of a chunk. With text, that line is found in the file first and the chunk is looked for after it. The first chunk of a file may leave this line out |
| A line starting with a space, - or + | Inside a chunk: a line that stays, a line removed, a line added                                                                                           |
| \*\*\* End of File                   | Optional end of a chunk: its lines are at the end of the file                                                                                             |
| \*\*\* End Patch                     | Last line of every patch                                                                                                                                  |

Reading the patch:

- **Tolerances.** White space around the whole patch and around each
  header line is ignored. Blank lines between chunks are skipped. A
  patch wrapped in a here-document's own lines (\<\<EOF, \<\<'EOF' or
  \<\<"EOF" first, EOF last) is unwrapped.

- **Errors in the patch itself** go to standard error as Invalid patch:
  MESSAGE or Invalid patch hunk on line N: MESSAGE. The messages cover a
  missing first or last line, an unknown header, an update with no
  chunks, a later chunk with no @@, and a line with no valid prefix.

Applying it:

- **Add** creates missing parent folders and replaces a file that is
  already there.

- **Delete** fails if the file is missing.

- **Update** finds each chunk's old lines (those that stay and those
  removed) at or after the place the previous chunk ended, and puts the
  staying and added lines in their place. A chunk with no old lines adds
  its lines at the end of the file.

- **Finding lines is forgiving, in passes.** Try an exact match first.
  Then ignore white space at line ends. Then ignore it at both ends.
  Then treat typographic dashes, quotes and spaces as their plain ASCII
  forms. This order is my recollection of upstream, because the file
  that holds it could not be read. List the characters you map in the
  last pass.

- **One retry.** If the old lines end with an empty line and are not
  found, try again without it.

- **When lines are not found,** the error is Failed to find context
  'TEXT' in PATH or Failed to find expected lines in PATH: followed by
  the lines.

- **A changed file always ends with a newline.**

- **On success** standard output gets Success. Updated the following
  files: and then one line per file: A PATH for added, M PATH for
  changed, D PATH for deleted, in that order. A moved file is listed at
  its new path. A patch that changes nothing fails with No files were
  modified.

Where the engine differs from upstream, on purpose:

- **All or nothing.** Upstream applies files in order and stops at the
  first failure, leaving earlier files changed. Here every chunk is
  located first, and if any fails, nothing is changed.

- **Exit status** is 0 on success and 1 on any failure. I could not read
  upstream's.

- **The text-only rule applies,** as to every other write.

- **Paths** resolve from the current directory of the session.

## 9. Part 6: awk

The reference is GNU awk (gawk) in its default mode and the C locale.
Ubuntu's own awk is mawk, so install gawk from the archive you used for
the datasets. If you cannot, use mawk, say so, and record the version.

| Part of awk          | In this stage                                                                                                      |
|----------------------|--------------------------------------------------------------------------------------------------------------------|
| Options              | -F SEP and -v VAR=VAL                                                                                              |
| Program shape        | BEGIN and END, a pattern with an action, either alone, and ranges pattern1, pattern2                               |
| Patterns             | /regex/, and any expression                                                                                        |
| Fields and variables | \$N and \$0, NF, NR, FNR, FILENAME, FS, OFS, ORS, and RSTART and RLENGTH from match                                |
| Statements           | print, printf, if with else, while, for (;;), for (k in a), next, break, continue, and assignment in all its forms |
| Arrays               | One-dimensional, indexed by any string, with in                                                                    |
| Functions            | length, substr, index, split, sub, gsub, match, tolower, toupper, sprintf                                          |
| Refused by name      | User-defined functions, getline, system, output redirection (\>, \>\>, \|), and everything else not in this table  |
| Refused at run time  | Input that is not ASCII, with a clear message                                                                      |

Requirements:

- **Numbers and strings behave as in awk:** double-precision numbers,
  output through %.6g unless the value is a whole number, and awk's
  rules for when a field compares as a number.

- **Regular expressions use the engine's own matcher** in extended
  syntax, with the extensions gawk accepts.

- **The order of for (k in a)** is unspecified in awk and is an accident
  of each implementation. Ours is the order in which keys were first
  set. Tests of such loops sort their output on both sides.

- **It streams and stays inside the memory limit,** apart from what the
  program itself stores in arrays.

- **It can ask the index.** A program whose only pattern is a regex with
  no END block that depends on line counts may skip pieces that cannot
  match. This is allowed, not required; if you do it, the differential
  test must cover it.

## 10. How to test

The method of stage 4 carries on: hand cases for every item, a coverage
row for each that may not be zero, and generated tests against the real
tool. Three things are new.

Generated families to add:

| Family  | What is generated                                                                                                                                                  | Compared with     |
|---------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------|
| Scripts | Small programs built from assignments, the parameter forms, arithmetic, cd, for, if, while, until, break, continue and read, around the commands already supported | bash              |
| diff    | Pairs of files made by random edits to one text, and pairs of trees, with random flags                                                                             | GNU diff          |
| awk     | Programs built from the grammar of section 9 over generated text                                                                                                   | The reference awk |

apply_patch has no real tool to compare with, so it is tested by what
must be true:

- **Round trip.** Take a tree, edit it at random, write a patch that
  describes the edit, apply the patch to the first tree, and require the
  second.

- **Forgiveness.** Damage the patch's context in each of the ways the
  matching passes allow, and require the same result. Damage it further,
  and require a failure.

- **All or nothing.** A patch whose last file cannot be matched leaves
  every file as it was.

- **Hand cases** for each line of the format, each error message and
  each tolerance.

Storage tests to extend:

- **The crash tests** gain units that hold loops and units that are one
  apply_patch over many files. After a power cut, such a unit is
  entirely there or entirely absent.

- **The differential test** gains apply_patch, and awk if it asks the
  index.

- **The three old suites run when a part lands, not at the end:** the
  kill test, the differential test and --check. Stage 4 found three
  stale tests on its last day.

The counters of part 1 need one test of their own: the same units on the
same database give the same counts, run after run.

## 11. Targets

Unless a row says otherwise: the Linux tree on disk, settled, default
settings, one thread, file cache warm, and real tools timed on one
thread. Rules about cost are on the counters of part 1, because those
repeat exactly.

| Part | What is measured                                                                                          | Target                                                                                                         |
|------|-----------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| All  | Harness in memory, on disk, and reopened after every step                                                 | All cases pass in all three                                                                                    |
| All  | Coverage table                                                                                            | No item from sections 4 to 9 with zero cases                                                                   |
| All  | Generated tests: scripts, diff, awk                                                                       | 10,000 cases on each of 3 seeds with no difference from the real tool, or each difference listed and explained |
| All  | Stage 4 benchmark commands against their stored counters                                                  | None reads, writes or syncs more than it did when part 1 was finished, or it is listed with a reason           |
| 1    | The same units run twice on the same database                                                             | Identical counters                                                                                             |
| 1    | Cost of counting                                                                                          | No log bytes, no syncs. Report the time overhead by the paired method of stage 2                               |
| 1    | xargs and rg fed 1 GB on standard input, inside 512 MB                                                    | Complete inside the limit                                                                                      |
| 1    | grep -o '\[a-z\]\*ing' over the 1 GB file                                                                 | At least as fast as GNU grep. Stage 4 took twice GNU's time                                                    |
| 2, 3 | for i in \$(seq 100000); do :; done                                                                       | Faster than bash                                                                                               |
| 3    | One unit whose loop makes 10,000 small writes, always                                                     | One sync for the unit, and faster than bash doing the same on real files                                       |
| 3    | A unit that loops forever                                                                                 | Stopped at the limit with a clear error, and the database still usable                                         |
| 4    | diff -u of two versions of an ordinary file                                                               | At least as fast as GNU diff                                                                                   |
| 4    | diff -rq Documentation COPY after cp -r and one edit (9,699 files)                                        | Under 10 ms, and text read follows the one changed piece                                                       |
| 4    | diff of the 1 GB file against a copy with one line changed, inside 512 MB                                 | Under 50 ms                                                                                                    |
| 5    | apply_patch over 100 files, always                                                                        | One sync, and all or nothing in the crash test                                                                 |
| 5    | apply_patch changing one line of the 1 GB file                                                            | Writes a few pieces, under 10 ms                                                                               |
| 6    | awk '{print \$2}', awk -F: '{s+=\$3} END {print s}' and awk '/RE/ {n++} END {print n}' over the 1 GB file | At least as fast as the reference awk, inside 512 MB                                                           |

Add benchmark commands for every row, and at least one for each new
construct and command.

## 12. Rules that stay in force, and the two that change

Two rules change in this stage:

- **There is a current directory.** It belongs to the session and starts
  at the root.

- **Checking before running covers what the text shows.** What only
  expansion reveals is refused when it is reached. Section 6 has the
  detail.

The rest stand:

- **Same output as the real tool, byte for byte.** Output is the piped
  form, unordered walks are sorted by name, and the fixed values of
  stage 4 fill in what the model does not hold.

- **Fail loudly.** Anything not listed in a brief or in README.md is
  refused by name.

- **From scratch.** No database, storage, index, search or
  regex-matching crate, and no crate for diff, awk or patching. The
  crates already in use stay.

- **Data model unchanged.** Sessions and variables are not data and are
  never stored.

- **Storage guarantees unchanged.** Every unit is all or nothing after a
  crash, costs one sync under always, and stays inside the memory limit.

- **Commands reach data only through the storage layer.**

- **Streaming and early exit** for everything that can.

- **One thread.** The C locale for the GNU commands and for awk, and UTC
  for every time.

- **Keep the record in the repository.** Commit this brief under
  docs/briefs/, and update the tables in README.md.

## 13. What to hand back

Write docs/HANDBACK-5.md and commit the result files beside the earlier
ones. Item 5 answers a question the owner has asked directly, so write
it for a reader who has not seen the code.

1.  **Code** on the branch, with README.md brought up to date.

2.  **Conformance.** Harness totals in the three modes, the coverage
    table, and the generated-test totals and seeds for each family.

3.  **Targets.** The table from section 11 with a measured value and met
    or missed for each row.

4.  **Benchmark tables** per dataset, with the counters beside the
    times.

5.  **The cost report.** The replay tables for the three workloads and
    for the benchmark list, and one page on what they show: which kinds
    of work use the CPU, which read the disk, which wait on syncs, and
    what would be the first thing to run short as load grows.

6.  **apply_patch.** The characters mapped in the last matching pass,
    and every place where this brief's description was not enough and
    what you chose.

7.  **What is still refused,** as in stage 4, updated.

8.  **Differences from the real tools,** updated.

9.  **For the stage after this.** With the cost report in hand: which
    work would gain most from more threads, and what several users would
    need from sessions.

10. **Deviations, gaps and questions.**

## 14. Assumptions to confirm

The owner has not confirmed these six. Follow them unless told
otherwise.

- **Session state belongs to the caller.** It is never stored, and a
  fresh session starts at / with HOME=/.

- **A unit is stopped after 1,000,000 commands.** The figure is a
  setting.

- **apply_patch is all or nothing,** which differs from the upstream
  tool, and exits with 0 or 1.

- **awk is measured against gawk,** and is the subset on the owner's
  list.

- **The cost report comes before the stage with several users,** so that
  stage can be designed from measurements.

- **The 1 GB copy stays at 2.6 ms for now.**

End of brief.
