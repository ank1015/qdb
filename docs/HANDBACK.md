# Hand-back notes for stage 1

## Status against "done when"

| Requirement | State |
|---|---|
| Every first-version command and flag passes the harness | Yes. 1,496 cases (1,776 steps) pass against the real tools. |
| Every unsupported thing fails clearly | Yes. Unsupported commands, flags and syntax are refused with status 2 and a named reason before any stage of the line runs. The harness has `!` cases for each. |
| Everything under "What to hand back" exists | Cannot be confirmed. See the first open question below. |

What exists: the engine (`src/`), the harness and its cases (`tests/`), the
benchmark with datasets, command list and measured results (`bench/`), and
these notes.

## Open questions for the author of the design document

1. **Two sections are missing.** The document refers to "How to build it"
   and "What to hand back", but the copy received contains neither. The
   deliverables here are inferred from the task description (engine,
   harness, benchmark). The choice of Rust was also made without guidance.
2. **Locale.** The document says sorting is by byte order (`LC_ALL=C`). The
   engine applies the C locale to everything, not only to sorting, so that
   there is one consistent rule. The visible effects: `.` in a regex and
   `?` in a glob match one byte rather than one character, `grep -i`
   folds ASCII letters only, and `wc -w` treats non-ASCII bytes the way
   coreutils does in the C locale. If the intent is UTF-8 behaviour with
   byte-order sorting, that is a contained change in `regex.rs`, `glob.rs`
   and `wc`.
3. **Command lists (`;`).** The document rules out `&&`, variables, loops
   and `$(...)` but does not mention `;`. It is refused for now.

## Decisions taken where the document was silent

- **Reference versions.** bash 5.2.21, grep 3.11, sed 4.9, coreutils 9.4,
  findutils 4.9.0. Error wording is matched to these.
- **Output form.** Always what the tool prints into a pipe: `ls` is one
  name per line with no colour and no quoting of odd names.
- **Walk order.** `find`, `grep -r`, `ls -R`, `cp -r` and `rm -r` visit
  children sorted by name bytes, depth first. Real `find` and `grep -r`
  use directory order, so the harness compares those as sorted lines.
- **Pipelines run one stage at a time**, left to right, each to
  completion. Real bash runs stages at once, which only matters for lines
  such as `mkdir d | ls` where the result depends on a race.
- **No current directory.** Relative and absolute paths both start at the
  root. `..` at the root stays at the root, as it does at `/`.
- **Messages from the shell itself** carry the prefix `qdb:` where bash
  prints `bash: line 1:`.
- **Text-only rule.** A write whose result is not valid UTF-8 or contains
  a NUL byte is undone completely: the file keeps its old content, or is
  not created, and the line fails with a clear message.
- **Standard input** of the first stage is empty unless the caller
  supplies some.

## Known gaps

Refused cleanly (status 2, named reason), though the real tool accepts them:

- Long options (`--all`), by design for now.
- `head -5`, `tail -5`, `tail +5` (old-style counts) and size suffixes
  such as `head -n 1k`.
- `echo -e`, `echo -E`.
- `sed`: any command other than a single `s`, flags on `s` other than
  `g`, `-n`, `-e`, the escapes `\x`, `\d`, `\o`, `\c` and the case
  conversions `\U`, `\L`, `\u`, `\l`.
- `find` tests and actions other than `-name`, `-type f/d`, `-maxdepth`.
- Brace and tilde expansion.

Accepted but different from the real tool:

- `sed` with no script prints a shortened usage text.
- `find -name` with a `/` in the pattern does not print GNU's warning.
- `cp -r d d/sub` copies in sorted order, so the partial copy made before
  the "cannot copy a directory into itself" error can differ from the
  real one on some file systems.
- Regular expressions that use a back-reference together with a repeated
  or alternated group can pick a different submatch from glibc in rare
  cases (about 1 in 1,000 randomly generated patterns). Patterns without
  back-references always agree.
- A badly formed interval such as `a\{1,x\}` is rejected as it should be,
  but the wording of the error can differ.
- Matching with back-references is by backtracking and stops with an
  error after 20 million steps instead of running without bound.

## What the benchmark says about stage 2

Details are in `bench/README.md`. In short: everything that touches the
tree or one file is already far faster than starting a process, and
recursive `grep` is 2 to 25 times slower than GNU grep because the engine
runs its regex matcher over every line of every file. A literal
pre-filter or an index for content search is the obvious next step.
