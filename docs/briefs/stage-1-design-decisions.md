# Custom text database: design decisions

Nov 17, 2026 · @Ananya Khandelwal

This is a self-contained brief for a coding agent. It gives the background, the specification agreed so far, the task to complete now, and what to hand back. No other context is needed.

## Context

We are building a database engine from scratch for text data. The data is files in nested folders, like a source repository. The query language is bash commands: grep, sed, mv and the rest are how a user adds, edits, finds and deletes data.

- Fully custom. The owner wants an engine designed for this one workload, fast and efficient. Do not build it on an existing database or search library (SQLite, RocksDB, Lucene and similar).
- Not a filesystem. The real tools are never run over real files. The engine parses each command line and executes it itself, which is what will later let it answer grep from an index instead of scanning every file.
- An engine for an existing language. The data model (files in folders) and the language (bash commands, defined by POSIX and the GNU tools) already exist. Nothing new is designed there. This is the position SQLite and Postgres were in with SQL: a new engine for a language defined elsewhere, starting with a subset and growing.
The overall plan has three steps:

- Write down the data model exactly. Done, see "Data model".
- Pick the subset of commands and flags to support first. Done, see "Command subset" and "Flags".
- Build the engine in stages. Your task is the first stage.
## Your task

Build the first stage of the engine: a simple, correct implementation of the specification below, plus a test harness and a benchmark that compare it against the real GNU tools. Speed is not a goal at this stage. Correctness and measurements are.

In scope, with details under "How to build it":

- A benchmark set: datasets and a list of commands.
- A test harness that runs each command on the real tools and on the engine and compares the results.
- A naive engine that implements the data model, the 15 commands and their first-version flags.
Out of scope:

- On-disk storage, crash safety, indexes and any performance optimisation. These come next and will be designed from your measurements.
- Anything marked "Later" or "Out for now" in the tables below.
- Concurrent users and a network server.
The stage is done when:

- Every command and flag in the first version passes the harness.
- Every unsupported command, flag or piece of shell syntax fails with a clear error.
- Everything under "What to hand back" exists.
## Data model

The database is one tree of folders and files, where a file is a name plus UTF-8 text.

Database = one tree with a root folderNode     = Folder | FileFolder   = { id, name, parent, children: name -> Node }File     = { id, name, parent, content: bytes of UTF-8 text }

| Decision | What we chose | Why |
|---|---|---|
| Folders | Real nodes, not shared path prefixes | mkdir, empty folders and moving a folder all assume a folder exists in its own right |
| Identity | An internal ID per node; name and parent are properties | mv changes where a file is, and it stays the same file |
| Shape | Strict tree: one parent per node, no symlinks or hard links | No cycles and no file in two places, in every later layer |
| Content | Bytes, with a line defined as the bytes between \n characters | cat and > must return exactly what was written, including a missing final newline |
| Text only | Content must be valid UTF-8 with no NUL bytes; binary is rejected | The engine can rely on text everywhere |
| Metadata | None stored. Size and line count are derived from content. No permissions, owners or modification time | Each stored field is one more thing every write must update |

Rules:

- Names follow POSIX: any characters except / and NUL, case-sensitive, unique within a folder.
- A folder cannot be moved into its own descendant.
## Command subset, first version

The first version supports 15 commands plus pipes, redirection and globs, with no current directory. The subset is meant to grow later.

| Operation | Commands |
|---|---|
| List | ls, find (by name only) |
| Read | cat, head, tail, wc |
| Find | grep |
| Add | mkdir, touch, cp, echo text > file |
| Edit | sed -i 's/old/new/' (substitution only), mv |
| Delete | rm, rmdir |

| Part of the shell | First version |
|---|---|
| Pipes (\|) | In |
| Redirection (>, >>) | In |
| Globs (*.txt) and quoting | In |
| Variables, loops, $(...), && | Out for now |
| cd and a current directory | Out. Every path is written from the root and each command stands alone |

Rules:

- Exact behaviour: whatever we support produces the same output as the real GNU tool, byte for byte.
- Fail loudly: an unsupported command or flag returns a clear error, never a wrong answer.
- Each command gets a handful of flags, not all of them. grep comes before rg.
## Flags, first version

A flag is in the first version if people use it constantly or if it forces the engine to work differently. Flags that only change how output looks wait.

| Command | First version | Later |
|---|---|---|
| ls | -a, -R, -1, -p | -S, -d |
| find | -name, -type f/d, -maxdepth | -iname, -path, -size, -exec |
| cat | -n | -A, -s |
| head | -n N | -c |
| tail | -n N, -n +N | -c |
| wc | -l, -w, -c | -m |
| grep | -r -i -n -l -v -c -E -F | -w, -o, -A/-B/-C, --include |
| mkdir | -p |  |
| touch | none |  |
| cp | -r | -n |
| echo | -n | -e |
| sed | -i, -E, s/old/new/ with g | line ranges, -n '5,10p', delete (d) |
| mv | none | -n |
| rm | -r, -f |  |
| rmdir | none |  |

Rules:

- Combined short flags such as grep -rin work from the start. Long forms such as --recursive come later.
- No ls -l: it prints permissions, owner and modification time, which we don't store. ls -p marks folders with a trailing /.
- touch on an existing file does nothing. Its only job is creating an empty file.
- Output order is sorted by name. Real find and grep -r use whatever order the filesystem returns, so there is no single order to match.
- Output is always the piped form: one name per line, no columns, no colours.
- Multi-line content is written with a quoted newline inside echo "...". Heredocs (cat > file <<EOF) are the first shell feature to add after this version.
## Behaviour details

These fill gaps in the tables above. Where something is still unspecified, match the GNU tool and report the gap.

| Area | Rule |
|---|---|
| Paths | There is no current directory. Every path resolves from the root, so src/a.txt, ./src/a.txt and /src/a.txt name the same file |
| Pipes | cat, head, tail, wc, grep and sed read from the pipe when given no file |
| Redirection | > and >> apply to any command's output and write to a file in the database. < and 2> are out for now |
| Globs | *, ? and [...] with bash defaults: * skips names starting with ., and a pattern that matches nothing is passed through unchanged. No ** |
| Quoting | Single quotes, double quotes and backslash. $name, $(...) and backticks are errors, including inside double quotes |
| Regex | grep and sed use POSIX basic regex by default and extended with -E. grep -F is a fixed string |
| Exit codes | Standard output and exit code match GNU exactly, for example grep exits 1 when nothing matches |
| Errors | Error text goes to standard error and follows GNU wording where practical |
| Text only | A write that would store invalid UTF-8 or a NUL byte is rejected with an error and changes nothing |
| Sort order | "Sorted by name" means byte order, as with LC_ALL=C. Recursive commands go depth first and visit a folder's children in that order |

Two requirements on the implementation:

- Parser. Write it against the structure of the real bash grammar, so unsupported syntax is recognised and rejected by name. A parser that only splits on spaces and | is not acceptable, because variables, loops and heredocs will be added later.
- Regex matching. A regex library is allowed. POSIX matching is leftmost-longest and many libraries are leftmost-first, which changes what sed replaces: echo ab | sed -E 's/a|ab/X/' prints X with GNU sed 4.9, and a leftmost-first library gives Xb.
## Open questions

None right now.

Modification time was decided on 17 Nov 2026: left out for now, so there is no ls -lt or find -newer.
