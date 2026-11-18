# qdb stage 4 brief: more of the language

Nov 18, 2026 · @Ananya Khandelwal

This is the brief for stage 4 of qdb: widen the bash the engine
understands, from fifteen commands with a few flags each to what people
type in a day's work, including rg. It is self-contained. It has 13
numbered sections and ends with the line "End of brief". If your copy
stops earlier, ask for the full text before you start.

## 1. Where things stand

Stage 3 is accepted: the database lives on disk, survives a crash, and
keeps the read speed of stage 2.

qdb is a database engine built from scratch for text files in nested
folders, queried with bash commands. The code is at
github.com/ank1015/qdb, branch claude/project-thread-3wnfpo. Read
README.md, docs/FORMAT.md and the three hand-backs under docs/ first.

The review on 18 Nov 2026 re-ran the work on a second machine and added
tests of its own:

- **Correctness.** All 1,617 harness cases pass in memory, on disk, and
  reopened after every step under mixed settings. All 37 unit tests
  pass.

- **Your crash tests, new seeds.** 3,000 power cuts on each of two fresh
  seeds, and 150 real kills, all recovered.

- **A crash test written from outside.** A real process appending
  numbered lines was killed 25 times. The file always held lines 1 to N
  with nothing missing. A cp -r of 992 files was killed 20 times and was
  absent 18 times, complete twice, and never partial.

- **Speed on disk.** On the git source tree (50.1 MB), grep -rl
  xdl_merge . takes 0.05 ms on disk, the same as in memory. A whole
  process that opens the database and runs that search takes 12 ms,
  against 55 to 60 ms for GNU grep alone.

The owner has confirmed the defaults your comparison recommended: sync
always, index at the write, text plain.

What the engine lacks now is language. It accepts 15 commands with 28
flags between them, pipes and two redirections. Everything else is
refused, including ;, &&, 2\>/dev/null, ls -l, sort and rg.

The owner has supplied a list of the shell syntax and commands that
covers more than 95% of what users of this database type. The lists in
this brief are taken from it. What this brief leaves out of that list is
named in section 3 and is the stage after this one.

## 2. Answers to your questions

| Question from the hand-back                               | Answer                                                                                                                                                                     |
|-----------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Is the slowest-write target about the engine or the disk? | The engine. Its own work is 15 to 27 ms, so the target counts as met. Neither rule bends now: a thread for syncs and upkeep belongs to the stage that brings several users |
| Should cp share pieces?                                   | Yes, in part 1, before more is built on the format                                                                                                                         |
| Is 64 lines the right batch?                              | Keep 64 lines or 4 MB, and make both settings. The owner will choose                                                                                                       |
| Should always group syncs when several writers arrive?    | Yes, and that is where the later stage will start. Not now                                                                                                                 |
| Recommended defaults                                      | Confirmed by the owner: sync always, index at the write, text plain                                                                                                        |
| Deviations you listed                                     | All accepted, including close and --compact running in one go                                                                                                              |
| Gaps you listed                                           | Part 1 takes two: --compact syncs first, and a database larger than memory gets measured. The rest stand as documented limits                                              |

## 3. Your task

Widen the language in five parts, in this order. If time runs out, drop
from the end and say so in the hand-back.

1.  **Storage carry-overs and modification time.** cp shares content,
    files and folders gain a modification time, and two gaps from stage
    3 are closed. Every format change of the stage is here.

2.  **The shell.** Command lists, more redirections, here-documents,
    command substitution and brace expansion.

3.  **The fifteen commands, completed.** The flags on the owner's list
    that are still refused.

4.  **rg.** With its own defaults, its own regex syntax and its ignore
    files.

5.  **Eighteen new commands.** sort, uniq, cut, tr, xargs, printf, du
    and eleven smaller ones.

Sections 5 to 8 list exactly what is in. Anything not listed stays
refused by name, as today.

On the owner's list, and left for the stage after this one:

- Variables and parameters such as \$NAME, \${NAME:-word} and \$?,
  arithmetic, export, unset and read.

- Control flow: for, if, while, until, break and continue.

- cd, pwd and ~. There is still no current directory.

- awk, diff and apply_patch.

Also out of scope:

- **Not planned at all:** case, select, functions, arrays, \[\[ \]\],
  subshells, (( )), process substitution, background &, positional
  parameters, and ln.

- **A later stage of their own:** several users and threads.

- **Still the owner's decision:** any change to the locale of the GNU
  commands.

The stage is done when:

- The harness passes in memory, on disk, and on disk reopened after
  every step, with cases for everything in sections 4 to 8.

- The generated tests in section 9 show no difference from the real
  tools.

- Every target in section 10 is met, or the miss is reported with its
  numbers and cause.

- Everything in section 12 exists.

## 4. Part 1: storage carry-overs and modification time

Do these first. They hold every format change of the stage, so the
format version goes up once.

Shared content:

- **cp shares pieces.** A copy points at the source's pieces and writes
  only a record. cp and cp -r then cost by the number of files, not
  their bytes. Today a 1 GB copy takes 7.6 s and writes 1,114 MB.

- **Sharing must be invisible.** Changing one of the two files never
  changes the other. A search lists every file that holds a matching
  piece, and its cost still follows the candidates. A piece is reclaimed
  only when its last user is gone. The differential test and the crash
  test both gain cp followed by edits to either side.

Modification time, which the data model has left out until now and the
owner's list needs:

- **Every file and folder has one,** in UTC with nanosecond resolution,
  kept in the saved tree and in the log.

- **It changes as on a real filesystem.** A file's time changes when its
  content is written or it is touched. A folder's time changes when an
  entry in it is added, removed or renamed. mv keeps the moved node's
  time. cp gives the copy the current time unless -p or -a is given.

- **A write that changes nothing still writes no content.** It updates
  the time wherever the real command would, which is a small record and
  not a copy.

- **One clock.** The engine reads the time from one place that a test
  can set. Outside tests it is the system clock.

- **Nothing else is added.** There are still no permissions, owners or
  links.

Housekeeping:

- **The format version goes up.** A database in the stage 3 format is
  refused with a clear message. No migration is needed yet, because no
  database outside the tests exists. Bring docs/FORMAT.md up to date.

- **--compact syncs first,** so a power cut during it loses nothing that
  had been written before it.

- **batch gets its two settings:** the number of lines and the number of
  bytes between syncs.

- **A database larger than memory.** Build one bigger than the machine's
  RAM, for example by importing the Linux tree twelve times under
  different folder names, and run the read benchmark on it. If the disk
  cannot hold it, cap the process with a limit that covers the file
  cache, or say that neither was possible. This is the first real test
  of the memory priority.

## 5. Part 2: the shell

The reference is bash 5.2.21, as before. These join pipes, \>, \>\>,
globs and the three kinds of quoting:

| Feature              | In this stage                                                                                                                                       |
|----------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| Command lists        | ;, newline, && and \|\|                                                                                                                             |
| Redirections         | \<, 2\>, 2\>\>, 2\>&1, \>&2, &\> and &\>\>, applied left to right                                                                                   |
| The null device      | /dev/null as a redirection target and as an input file. It is the only path that is not in the database                                             |
| Here-documents       | \<\<EOF, \<\<'EOF', \<\<"EOF" and \<\<-EOF. The quoted forms do not expand, and \<\<- strips leading tabs. Several in one command are read in order |
| Command substitution | \$(...) and backticks, with bash's field splitting on the default separators and removal of trailing newlines                                       |
| Brace expansion      | {a,b}, {a,{b,c}}, {1..3..2}, {a..z} and {01..10}                                                                                                    |
| Line continuation    | A backslash at the end of a line                                                                                                                    |

Everything section 3 lists as left for later or not planned is still
refused by name.

Four rules to hold to:

- **One call is one unit.** Everything given to one execute call, or one
  -c, is all or nothing after a crash, however many commands it holds.
  From standard input, a unit is one complete command as bash would read
  it, so a here-document belongs to its command.

- **No rollback without a crash.** In a && b, if a writes and b fails,
  the write stays, as in bash. All-or-nothing is about crashes only.

- **Refusal still comes first.** If any part of a unit is unsupported,
  nothing in it runs.

- **Merged streams.** With 2\>&1, the order of output and errors in the
  real tools depends on how each one buffers. Match it when everything
  fits in one buffer, and say what you do beyond that.

## 6. Part 3: the fifteen commands, completed

Add the flags below to what README.md already lists. The references are
unchanged: grep 3.11, sed 4.9, coreutils 9.4 and findutils 4.9.0, in the
C locale, with times in UTC.

| Command | Add                                                                                                                                                                                               |
|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| ls      | -A, -d, -r, -S, -t, -l, -h                                                                                                                                                                        |
| find    | -iname, -path, -ipath, -mindepth, -empty, -size, -newer, -mmin, -mtime, -print, -print0, -prune, -exec ... ; and -exec ... +, and the operators !, -not, -a, -and, -o, -or and quoted parentheses |
| cat     | -A, -b, -e, -E, -s, -t, -T, -u, -v                                                                                                                                                                |
| head    | -c N, -q, -v, the form -NUM, -n -N to drop the last N, and size suffixes such as K and MiB                                                                                                        |
| tail    | -c N and -c +N, -q, -v, the form -NUM, and size suffixes                                                                                                                                          |
| wc      | -m                                                                                                                                                                                                |
| grep    | -G, -R, -o, -L, -q, -s, -w, -x, -h, -H, -I, -a, -e (repeatable), -A, -B, -C, the form -NUM, -m, --include, --exclude, --exclude-dir                                                               |
| sed     | See the next table                                                                                                                                                                                |
| touch   | -c, -d DATE, -t STAMP                                                                                                                                                                             |
| cp      | -R, -a, -p, -f, -n, -T, -v                                                                                                                                                                        |
| mv      | -f, -n, -v                                                                                                                                                                                        |
| rm      | -R, -d, -v                                                                                                                                                                                        |
| rmdir   | -p                                                                                                                                                                                                |

sed grows from one command to a small language:

| Part of sed   | In this stage                                                                    |
|---------------|----------------------------------------------------------------------------------|
| Options       | -n, -r beside -E, -e (repeatable), -s, -i with a suffix. No long options         |
| Addresses     | N, \$, /re/, \\re%, N,M, N,+M, first~step, 0,/re/, and !                         |
| Commands      | p, d, s, q, Q, =, a, i, c, y, and blocks in { }                                  |
| Flags of s    | g, p, a number, i and I. Replacements can use & and \0 to \9                     |
| Still refused | The hold space (h, H, g, G, x), N, D, P, labels and branches, r, w, e, -f and -z |

Where the data model has no fact, use these fixed values, and mask on
the real side what cannot agree:

| What                                       | Value                                                                                |
|--------------------------------------------|--------------------------------------------------------------------------------------|
| Mode in ls -l                              | -rw-r--r-- for a file, drwxr-xr-x for a folder                                       |
| Link count                                 | 1 for a file, 2 plus the number of subfolders for a folder                           |
| Owner and group                            | qdb                                                                                  |
| Size of a folder                           | 4,096 bytes, as on the reference filesystem. This also decides its place under ls -S |
| Disk blocks, for the total line and for du | Content rounded up to 4 KB blocks, as on the reference filesystem                    |
| Time zone                                  | UTC                                                                                  |

Notes:

- **Dates for touch.** -d accepts @SECONDS, YYYY-MM-DD and YYYY-MM-DD
  HH:MM\[:SS\[.frac\]\] with a space or a T, in UTC. -t accepts
  \[\[CC\]YY\]MMDDhhmm\[.ss\].

- **Accepted with no effect:** grep -I and -a, and cat -u. The database
  holds only text.

- **Still refused:** echo -e and -E (use printf), grep -f, tail -f,
  --help, --version, and long options other than the ones listed in this
  brief.

- **find -size applies to files only.**

- **find -exec runs commands inside the engine,** and only supported
  ones. So does xargs in part 5.

- **The index still does its work.** grep -o, -w and the context flags
  use it like any other search, and sed -n '/re/p' asks it too.

## 7. Part 4: rg

rg is a new command, with ripgrep 14.1.0 as its reference. It is not
grep with other defaults: it has its own regex syntax, it is
Unicode-aware, and it decides for itself which files to search.

These defaults were checked on ripgrep 14.1.0 with its output piped:

- **Recursive from the root** when no path is given, printing paths with
  no leading ./.

- **No line numbers** unless -n is given. Lines come out as path:text,
  and as bare text when one file is named.

- **Hidden files and folders are skipped,** meaning names that start
  with a dot.

- **.ignore is always obeyed. .gitignore is obeyed only when the tree
  holds a .git folder.**

- **Exit status** is 0 for a match, 1 for none and 2 for an error.

In this stage:

| Kind                        | Flags                                                                                                               |
|-----------------------------|---------------------------------------------------------------------------------------------------------------------|
| Matching                    | -i, -S, -s, -F, -w, -x, -v                                                                                          |
| Output                      | -c, -l, -n, -N, -0 and --null, -A, -B, -C, -m, --no-heading                                                         |
| Which files                 | -g and --glob, -t and --type, -T and --type-not, -u up to -uuu, --files, --hidden, --no-ignore                      |
| File types                  | c, cpp, css, go, html, js, json, md, py, rust, sh, toml, ts, txt, yaml, each with the globs rg --type-list gives it |
| For tests and compatibility | --sort path, and -j N with no effect                                                                                |
| Refused                     | Multiline search (-U), PCRE2 (-P), other file types, and every flag not in this table                               |

Requirements:

- **The regex syntax is rg's default,** with Unicode on. The
  regex-syntax crate is allowed for parsing patterns and for its Unicode
  tables. Matching, scanning and the index stay yours.

- **Ignore files follow rg exactly:** .rgignore, .ignore, the
  repository's .gitignore and .git/info/exclude, under the same
  conditions and with the same precedence. Assume no home directory and
  no global git settings, as in the harness.

- **The trigram index serves rg as it serves grep.** A case-insensitive
  search for non-ASCII letters may have to drop some trigrams. It must
  never miss a match.

- **Order.** Real rg prints files in an order that depends on the
  filesystem. Ours is sorted by path, as for find and grep -r. The
  harness compares unordered output as sets, and compares exactly when
  --sort path is given.

- **This does not change grep or sed.** They stay in the C locale, and
  the owner has accepted that the two differ on non-ASCII text. Each
  command behaves as the real one does.

## 8. Part 5: eighteen new commands

These are what turns a search into an answer: count, rank, pick a
column, act on a list. The references are coreutils 9.4 and findutils
4.9.0 in the C locale, and bash 5.2.21 for the commands that are bash
builtins.

| Command        | Flags in this stage                                                                                                 | Reference                    |
|----------------|---------------------------------------------------------------------------------------------------------------------|------------------------------|
| sort           | -u, -n, -r, -k KEY, -t SEP, -h, -f, -b, -s, -c                                                                      | coreutils                    |
| uniq           | -c, -d, -u, -i, -f N, -s N                                                                                          | coreutils                    |
| cut            | -b LIST, -c LIST, -f LIST, -d DELIM, -s, --complement                                                               | coreutils                    |
| tr             | -d, -s, -c, -C, -t                                                                                                  | coreutils                    |
| tee            | -a, and several files                                                                                               | coreutils                    |
| nl             | -b a and -b t, -v N, -w N, -s SEP, -n ln, -n rn, -n rz                                                              | coreutils                    |
| seq            | -s SEP, -w, -f FMT                                                                                                  | coreutils                    |
| basename       | -a, -s SUFFIX                                                                                                       | coreutils                    |
| dirname        | Several operands                                                                                                    | coreutils                    |
| du             | -s, -h, -a, -c, -d N, -b, -k, -m, -x, with their long forms, --apparent-size and --exclude=PAT                      | coreutils                    |
| date           | -u, -d DATE, -I, -R, +FORMAT                                                                                        | coreutils                    |
| xargs          | -n N, -I STR and -i, -L N and -l, -0, -d DELIM, -r, -t, -s N, -x, -P1, with their long forms                        | findutils                    |
| printf         | %s %d %i %u %f %F %e %E %g %G %x %X %o %c %% %b, with flags, width, precision, escapes, reuse of the format, and -- | bash builtin                 |
| test and \[    | -f, -d, -e, -s, -z, -n, =, ==, !=, -eq, -ne, -lt, -le, -gt, -ge, !, -a, -o, and quoted parentheses                  | bash builtin                 |
| true, false, : | None                                                                                                                | bash builtin                 |
| sleep          | Checks its argument and returns at once                                                                             | None: the engine never waits |

ln is refused with a message saying the database has no links.

Requirements:

- **They stream** where the real command can. cut, tr, tee, nl, uniq and
  xargs hold no more than a line or a batch.

- **sort stays under the memory limit.** Input larger than memory is
  sorted through the scratch file that redirected output already uses.

- **xargs and find -exec can run any supported command.** A stage that
  writes through them runs to completion, as other writing stages do.

- **du and date follow the conventions of section 6:** 4 KB blocks,
  4,096 bytes for a folder, UTC, and the engine's one clock.

- **sort -k** takes field and character positions and the b, f, n and r
  modifiers.

## 9. How to test this much surface

This stage adds far more behaviour than any before it, and each flag
multiplies with the others. Hand-written cases alone will not cover it.

- **Cases for every item.** Each flag, command and piece of syntax in
  sections 5 to 8 gets hand-written cases, including its error paths and
  its use with the flags it is commonly combined with.

- **A coverage table.** The harness prints one row per command and flag
  with the number of cases that use it. A row with zero cases fails the
  run.

- **Generated tests against the real tools,** one generator per family,
  each producing random inputs and random combinations of supported
  flags:

| Family                               | What is generated                                                                                 |
|--------------------------------------|---------------------------------------------------------------------------------------------------|
| Shell                                | Lists, redirections, here-documents, substitutions and brace expansions around simple commands    |
| grep                                 | Flag combinations, including the context flags with -o, -c, -v, -m and several -e patterns        |
| sed                                  | Scripts built from the supported addresses and commands                                           |
| find                                 | Expression trees built from the supported tests, actions and operators, over files with set times |
| rg                                   | Flags, patterns in rg's syntax with Unicode text, and trees with ignore files at several levels   |
| sort, uniq, cut, tr, nl, seq, printf | Flag combinations over generated text and arguments                                               |
| ls, du                               | Flag combinations over generated trees with set times and sizes                                   |
| xargs                                | Flag combinations around supported commands                                                       |

- **All three store modes.** The cases and the generated tests run in
  memory, on disk, and on disk reopened after every step.

- **Time.** Output that depends on the clock or on file times is
  compared only after the engine's clock and the real files' times are
  set to the same values. Two real files written in quick succession can
  share a timestamp, so never rely on write order for ls -t.

- **Masked columns.** Owner, group and anything else from the fixed
  values in section 6 are masked on the real side, and checked on the
  engine side by cases of their own.

- **Refusals.** For each command, cases for flags that are not
  supported, checking the refusal names the flag and changes nothing.

- **Old tests keep their meaning.** The 1,617 existing cases pass
  unchanged, except those that checked a refusal of something now
  supported. List the ones you changed.

## 10. Targets

Unless a row says otherwise: the Linux tree on disk, settled, default
settings, one thread, file cache warm. Real tools are timed on one
thread too: rg -j 1 and sort --parallel=1.

| Part | What is measured                                                               | Target                                                                                                         |
|------|--------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| All  | Harness in memory, on disk, and reopened after every step                      | All cases pass in all three                                                                                    |
| All  | Coverage table                                                                 | No command or flag from sections 5 to 8 with zero cases                                                        |
| All  | Generated tests, each family                                                   | 10,000 cases on each of 3 seeds with no difference from the real tool, or each difference listed and explained |
| All  | Stage 3 targets, run again                                                     | All still met, counting the slowest-write row on the engine's own work                                         |
| 1    | cp of the 1 GB file                                                            | Under 1 ms, with disk written following the record, not the content. Stage 3 took 7.6 s and wrote 1,114 MB     |
| 1    | cp -r Documentation (9,698 files)                                              | Under 50 ms. Stage 2 took 476 ms in memory                                                                     |
| 1    | Crash and differential tests with shared pieces and times                      | Pass, with cp followed by edits to either copy in the generated steps, and times surviving every crash         |
| 1    | Cost of keeping times                                                          | No stage 3 read or write figure more than 5% worse                                                             |
| 1    | A database larger than memory                                                  | The read benchmark completes with correct results. Report every time beside its warm figure                    |
| 3    | Recursive grep with -o, -w and the context flags                               | At least as fast as GNU grep, and the index used                                                               |
| 3    | ls -ltR and find . -newer FILE over the whole tree                             | At least as fast as the GNU command                                                                            |
| 4    | rg for a term in 1% of files or fewer                                          | At least 10 times faster than real rg -j 1                                                                     |
| 4    | Every rg command in the benchmark                                              | At least as fast as real rg -j 1                                                                               |
| 5    | sort, uniq, cut and tr over the 1 GB file                                      | At least as fast as the GNU command, and inside the 512 MB limit                                               |
| 5    | du -s on the root                                                              | Under 1 ms                                                                                                     |
| 5    | A ranking pipeline: grep -rho PATTERN . \| sort \| uniq -c \| sort -rn \| head | Faster than the same pipeline on the real tools                                                                |

Add benchmark commands for everything in this table, and at least one
for each new command and each kind of shell syntax.

## 11. Rules that stay in force

Every rule from the earlier stages still applies to everything added
here.

- **Same output as the real tool, byte for byte.** Output is always the
  piped form, and unordered walks are sorted by name.

- **Fail loudly.** Anything not listed in this brief or in README.md is
  refused by name before any part of the unit runs.

- **From scratch.** No database, storage, index, search or
  regex-matching crate. regex-syntax is the one addition, for rg
  patterns only. The crates already in use stay.

- **One change to the data model.** Modification time is added in
  part 1. Nothing else is added to serve a flag: where a flag needs a
  fact the model lacks, it prints the fixed values of section 6 or is
  refused.

- **Storage guarantees unchanged.** Every new command that writes is all
  or nothing after a crash, costs one sync under always, and stays
  within the memory limit.

- **Commands reach data only through the storage layer.**

- **Streaming and early exit.** New commands pass output on in pieces
  and stop when their reader has had enough.

- **One thread.** The C locale for the GNU commands, and UTC for every
  time.

- **Keep the record in the repository.** Commit this brief under
  docs/briefs/, and update the tables in README.md.

## 12. What to hand back

Write docs/HANDBACK-4.md and commit the result files beside the earlier
ones. Item 6 will decide the next language stage, so make it complete.

1.  **Code** on the branch, with the supported tables in README.md
    brought up to date.

2.  **docs/FORMAT.md** updated for shared pieces, modification times and
    the new format version.

3.  **Conformance.** Harness totals in the three modes, the coverage
    table, and the generated-test totals and seeds for each family.

4.  **Targets.** The table from section 10 with a measured value and met
    or missed for each row.

5.  **Benchmark tables** per dataset with the new commands, and the
    larger-than-memory run.

6.  **What is still refused.** Everything in bash and in the supported
    commands that a user might try and the engine refuses, grouped by
    kind, with your view of how much work each group is.

7.  **Differences from the real tools.** Every place output differs or
    cannot be matched, and why.

8.  **Storage after sharing.** Disk and write figures for the copy rows
    before and after, and anything sharing made harder.

9.  **Deviations, gaps and questions.**

## 13. Assumptions to confirm

The owner has confirmed the default settings, the split between this
stage and the next, and rg being Unicode-aware while grep stays in the C
locale. These five are still assumptions. Follow them unless told
otherwise.

- **Modification time joins the data model.** The owner's list needs it
  for ls -l, ls -t, find -newer and touch -d, but an earlier decision
  had left it out "for now".

- **The fixed values of section 6** for mode, owner, link count, folder
  size and blocks.

- **cd, awk, diff and apply_patch wait for the next stage** along with
  variables and loops. The owner asked only for variables and loops to
  wait.

- **One unit is one call,** so several commands joined by ; succeed or
  vanish together after a crash.

- **The stage 3 format can be dropped without migration.**

End of brief.
