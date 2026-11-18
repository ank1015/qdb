# qdb stage 2 brief: fast search

Nov 18, 2026 · @Ananya Khandelwal

This is the brief for stage 2 of qdb: make reading and searching fast, in memory, and measure what that costs. It is self-contained. It has 11 numbered sections and ends with the line "End of brief". If your copy stops earlier, ask for the full text before you start.

## 1. Where things stand

Stage 1 is accepted: the engine is correct, and its one weakness is recursive search.

qdb is a database engine built from scratch for text files in nested folders, queried with bash commands. Stage 1 delivered a correct in-memory engine, a test harness and a benchmark. The code is at github.com/ank1015/qdb, branch claude/project-thread-3wnfpo. Read README.md and docs/HANDBACK.md there first.

The review on 18 Nov 2026 re-ran everything on a second machine with the same GNU versions:

- Correctness holds. All 1,496 harness cases pass. Of 155 extra hand-written cases, 143 match the real tools. The 12 misses are refusals of out-of-scope flags, shell error wording, and the known cp -r d d/sub difference.
- Everything except recursive search is already fast. The stage 1 report puts listing, finding, reading and editing one file at 10 to 300 times faster than the real tools.
- Recursive search is about 10 times slower than GNU grep, and far worse when the pipeline ends in head.

| Command | Data | GNU | Engine |
|---|---|---|---|
| grep -rn TODO . | Generated large set, 87.4 MB (stage 1 report) | 111 ms | 1,212 ms |
| grep -rl xdl_merge . | git source tree, 50.1 MB, term in 5 files (review) | 51 ms | 605 ms |
| grep -rn TODO . \| head -n 5 | Generated large set (review) | 6 ms | about 1,000 ms |

The cause is visible in the code. src/cmd/grep.rs runs the regex matcher on every line of every file, src/regex.rs allocates its work lists for each line, and nothing skips ahead on a literal. src/engine.rs runs each pipeline stage to completion and holds its whole output, so head cannot stop the search.

The review also found four gaps in the benchmark. Part 1 fixes them:

- The generated data has no selective search term. error is in 12,004 of 12,005 files and TODO in 7,363, so an index cannot be judged on it.
- The largest dataset is 87.4 MB of generated text. The plan was real-world text at about 10 MB, 100 MB and 1 GB.
- --load stops at the first file that is not text, so a real repository cannot be imported. The git source tree has 34 such entries among 4,874.
- No benchmark command ends in | head, and peak memory is not measured.
## 2. Answers to your open questions

| Question from the hand-back | Answer |
|---|---|
| Two sections were missing | Your copy of the stage 1 brief lost its last three sections. What they asked for that is still owed is in part 1 below. Rust stays |
| Locale | The C locale stays for this stage, for everything. Keep the definition of "one character" and of case folding in one place each, because a move to UTF-8 behaviour is still an open decision |
| Command lists (;) | Stay refused, like && |
| Known gaps you listed | Accepted as they are: the cp -r d d/sub partial copy, the back-reference submatch cases, the wording for malformed intervals, and the step limit |
| Decisions you took where the brief was silent | All accepted, including walk order, one stage at a time for pipelines (this changes in part 2), and the qdb: prefix |

## 3. Your task

Make reads fast in memory, in four parts, in this order. Re-run the full benchmark after each part and keep every result, so each gain and each cost can be traced to the part that caused it.

- Benchmark and loader. Real data at three sizes, selective search terms, memory figures. Then re-measure the stage 1 engine as the "before".
- Pipelines that stop early. head ends the work of the stages before it.
- Fast scan. Recursive search with no index at least as fast as GNU grep.
- Trigram index. Search touches only the files that can match, and the index stays current on every write.
Out of scope:

- On-disk storage and crash safety. Stage 3 will design the on-disk format around the index you measure here, which is why the index comes first.
- New commands, flags or shell syntax, and any change of locale.
- Threads. Everything stays on one thread, so the numbers show what the algorithms gain.
- Concurrent users and a network server.
The stage is done when:

- The harness passes in full with the index on and with it off.
- The differential test in part 4 passes.
- Every target in section 8 is met, or the miss is reported with its numbers and cause.
- Everything in section 10 exists.
## 4. Part 1: benchmark and loader

Do this first, then measure the unchanged stage 1 engine on the new data. Those numbers are the "before" for everything else.

- Loader. --load skips symlinks and files that are not valid text, and reports how many it skipped.
- Real datasets. Three real-world source trees of about 10 MB, 100 MB and 1 GB of text after skipping. The Linux kernel source is a candidate for the largest; check its size after import. Record each repository and commit. Keep the generated datasets for quick runs.
- If the machine cannot download. Say so in the hand-back, and change the generator so that term frequencies cover the range in the next point.
- Search terms chosen by measured frequency. For each dataset, pick literal terms found in 0 files, 1 file, about 0.1%, 1%, 10% and over 50% of files. Record the file counts in the report.
- Memory. Record resident memory after load, peak memory per command, and the size of the index once it exists.
- References. GNU tools stay the reference for correctness. Keep capturing their output through a pipe: GNU grep stops at the first match when its output is /dev/null. If rg is installed, add its time as an extra column for recursive searches, on one thread (rg -j 1 --no-ignore --hidden). It is timing only.
Add these commands to bench/commands.txt:

| Kind | Example |
|---|---|
| Each frequency class, lines and file names | grep -rn TERM . and grep -rl TERM . |
| Case-insensitive, selective term | grep -rin TERM . |
| Search inside one subfolder | grep -rn TERM some/dir |
| Regex with a required literal | grep -rE 'TERM_[a-z]+\(' . |
| Regex with no literal | grep -rE '[0-9]{4,}' . |
| Pipelines ending in head | grep -rn COMMON . \| head -n 5, find . -type f \| head -n 3, cat dir/*.c \| head -n 2 |
| Many files written in one command | sed -i 's/TERM/OTHER/g' dir/*.c |

## 5. Part 2: pipelines that stop early

A pipeline must cost what its output needs, not what its first stage could produce. Today grep -rn TODO . | head -n 5 searches every file and then throws away all but five lines.

- Stop early. Stages pass output downstream in pieces. When a stage needs no more input, as head does after its lines, the stages before it stop working.
- Same results. Standard output and exit status still match real bash exactly.
- Errors from a stage that was cut off. In real bash these depend on the size of the pipe buffer, so they cannot be matched byte for byte. The rule here: a stage reports errors only for work it did. Use $~ cases, which skip standard error, for these lines.
- Writes always finish. A stage that changes data runs to completion whatever follows it.
- Bounded memory. Add a library entry point that writes output to a sink as it is produced. For commands that can stream, peak memory must not grow with the size of the output. Keep execute returning an Outcome for callers that want it.
- Unchanged. A line that uses anything unsupported is still refused before any stage runs.
## 6. Part 3: fast scan

With no index, recursive search must be at least as fast as GNU grep on one thread. This comes before the index for two reasons: the index only narrows the list of files and every candidate is still scanned, and an index judged against a slow scan would look better than it is.

Choose the techniques your measurements justify. These are the expected ones:

- Required literals. Work out from the pattern the strings every match must contain. Search the whole file content for them with a fast substring routine. Only on a hit, find the line and confirm it with the full matcher.
- No work per line. Do not split content into lines or allocate before there is a candidate.
- A DFA built as it runs, for patterns with no back-reference and no useful literal, such as [0-9]{4,}.
- Case and alternation. A case-insensitive literal search for -i, and a multi-literal search for patterns such as TODO|FIXME.
Rules for this part:

- Your regex engine stays, and stays the reference. Every fast path must give the same result as the current matcher. Prove it with the regex fuzzer already in the harness.
- Dependencies. The libc and memchr crates are allowed, and so is std::arch. No regex, search, index or database crate.
## 7. Part 4: trigram index

The index records, for every 3-byte sequence, the set of files whose content contains it. A search then scans only the files that can match. Build it over ASCII-lowercased content, so one index serves both exact and -i searches.

A search uses it in three steps:

- Derive from the pattern a query over trigrams, built from AND and OR, that every matching file must satisfy. The method is in Russ Cox, Regular Expression Matching with a Trigram Index (2012).
- Evaluate the query to get candidate files, limited to the folder being searched.
- Scan only the candidates with the part 3 scanner, and print in the same sorted order as a full scan.
Requirements:

- Never a missed match. Results are identical to a search with the index off, for every pattern and flag. A pattern that yields no usable trigram falls back to a full scan.
- Cost follows the candidates. A search with few candidates must not walk the whole folder tree.
- Current after every write. All content changes already pass through create_file, set_content and remove in src/tree.rs, so update the index there. mv costs the index nothing, because files are identified by ID.
- Stale entries are allowed, missing ones never. Extra candidates are acceptable while results stay identical and the size budget holds.
- Behind the storage layer. Commands ask storage one question: which files under this folder can match this query? They never touch index structures. Stage 3 will move the index to disk behind that same question.
- Switchable. Add --no-index to qdb, qdb-harness and qdb-bench, and an equivalent option in the library.
- Built during --load. Report the build time.
Tests:

- Both modes. The full harness passes with the index on and with it off.
- Differential test. Random patterns and flags, interleaved with random writes: create, overwrite, append, sed -i, cp -r, rm -r and mv. After every step, the index-on result must equal the index-off result. Run at least 10,000 steps on each of 3 seeds.
- Rebuild check. After a long write sequence, searches on the maintained index equal searches on an index built fresh from the same data.
## 8. Targets

All times are for the engine in process, on one thread, measured on the real datasets from part 1. "GNU grep" means the same command on the same machine.

| Part | What is measured | Target |
|---|---|---|
| 2 | grep -rn COMMON . \| head -n 5, largest dataset | Under 1 ms |
| 2 | Peak memory of a streamed command | Does not grow with output size |
| 3 | Every recursive grep in the benchmark, index off | At least as fast as GNU grep at every dataset size |
| 4 | Search for a term in 0 files | Under 1 ms at every dataset size |
| 4 | Search for a term in 1% of files or fewer, largest dataset | At least 10 times faster than GNU grep |
| 4 | Any benchmark command, index on against index off | No more than 10% slower |
| 4 | Index size in memory | No more than 1x the raw text. Report the actual ratio |
| 4 | A write to one file, index on | Time depends on that file's size, not on the size of the database |
| 4 | Every write command, and --load | No target. Report the time with the index on and off |

The 1x index budget is a ceiling, not an expectation. Cox's article reports an on-disk trigram index of 77 MB for 420 MB of Linux source, about 20%.

## 9. Rules that stay in force

Nothing in this stage changes what the engine returns, only how fast it returns it.

- Same output as GNU, byte for byte. All 1,496 existing harness cases keep passing. The one new exception is standard error from a cut-off pipeline stage, described in part 2.
- Fail loudly. Anything unsupported is refused with a clear error before any stage runs.
- From scratch. No database, index or search library.
- Data model unchanged. The index is derived data. It is not part of the model, and losing it must lose nothing a rebuild cannot restore.
- Supported commands and flags unchanged. The table in README.md is the list.
- Keep the record in the repository. Commit this brief, and the stage 1 brief if you have it, under docs/briefs/.
## 10. What to hand back

Write docs/HANDBACK-2.md and commit the result files beside the stage 1 results. Stage 3 designs the on-disk format from these eight things, so the index facts in item 7 matter as much as the speed figures.

- Code on a branch, with README.md updated for the new options.
- Conformance. Harness totals with the index on and off, and the differential test's totals and seeds.
- Benchmark tables, one per dataset. For each command: GNU time, rg time where available, and the engine before any change, after part 2, after part 3 with the index off, and after part 4 with the index on.
- Targets. The table from section 8 with a measured value and met or missed for each row.
- Memory. Resident memory after load with and without the index, the index size as a share of the raw text, and peak memory per command.
- Write costs. Each write command with the index on and off at each dataset size, and --load time with and without the index build.
- Index facts. The number of distinct trigrams, how file-list lengths are distributed, the share of the index taken by the most common trigrams, and how much index work each kind of write causes.
- Deviations, gaps and questions. Every difference from GNU, every place this brief was silent and what you chose, and your questions.
## 11. Assumptions to confirm

The owner has not confirmed these four. Follow them unless told otherwise.

- Storage waits for stage 3. The on-disk format will be designed around the measured index, so persistence is deferred once more.
- The C locale stays for now. Whether ., ? and -i should follow UTF-8 characters is still the owner's decision.
- Index budget of 1x the raw text. It comes from the example disk budget of 2x in the stage 1 brief, which was never fixed.
- One thread. Scanning files in parallel is a later option.
End of brief.
