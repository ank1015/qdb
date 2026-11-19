# qdb

A custom text database that speaks bash. It stores one tree of folders and
text files, in a database directory on disk or in memory, and runs a
subset of GNU command lines against it (`ls`, `find`, `cat`, `grep`,
`sed`, ...), producing the same bytes and the same exit codes as the real
tools.

Stage 1 built a simple, correct engine, a test harness that compares it
with the real GNU tools, and a benchmark. Stage 2 made reads fast in
memory: pipelines that stop as soon as their reader has enough, a search
that is faster than GNU grep on one thread with no index, and a trigram
index that lets a search read only the files that can match. Stage 3 put
the data on disk: a format of its own, recovery after a crash to the
result of a whole number of command lines, the engine's own memory
independent of how much text there is, and upkeep done in small steps.
Stage 4 widened the language: command lists, all three streams,
here-documents, command substitution and brace expansion; most of the
flags people use on the first fifteen commands; `rg` with its ignore
rules; and eighteen more commands, from `sort` and `xargs` to `date`.
Files and folders now carry a modification time, and a copy shares its
content with the original until one of them changes.
There is no concurrency yet: one process, one thread.

Nothing is borrowed from an existing database, storage engine, search
library or regex library. The crate depends on `memchr` (finding a byte
or a substring), `libc` (mmap, flock, limits), `crc32fast` (checksums),
`lz4_flex` (the one compression codec, used with `--compress`) and
`regex-syntax` (parsing `rg` patterns only; the matching is the
engine's own).

## Quick start

```
cargo build --release

# run commands against a fixture
./target/release/qdb --fixture tests/fixtures/basic.fx -c 'grep -rn TODO .' -c 'ls -R src'

# load a real directory into memory, change it, write the result elsewhere
./target/release/qdb --load some/dir --dump out/dir -c "sed -i 's/old/new/g' notes.txt"

# the same search with and without the search index: same output, different speed
./target/release/qdb --load some/dir -c 'grep -rn needle .'
./target/release/qdb --load some/dir --no-index -c 'grep -rn needle .'

# keep the tree in a database directory: made if it is not there,
# recovered if the last process using it was killed
./target/release/qdb --db my.qdb --load some/dir -c 'ls'
./target/release/qdb --db my.qdb -c "echo 'a new line' >> notes.txt" -c 'grep -rn needle .'
./target/release/qdb --db my.qdb --check --stats
./target/release/qdb --db my.qdb --compact

# compare the engine with the real tools (5,452 cases): in memory, on
# disk, and on disk closed and opened again after every step
./target/release/qdb-harness
./target/release/qdb-harness --no-index
./target/release/qdb-harness --store disk
./target/release/qdb-harness --store reopen

# cut the power at 10,000 points of a generated run; kill a real process 1,000 times
./target/release/qdb-harness --crash 10000 --seed 1 --sync always
./target/release/qdb-harness --kill 1000 --seed 1

# index on against index off over 10,000 random searches and writes
./target/release/qdb-harness --diff 10000 --seed 1

# 10,000 generated command lines per family against the real tools
./target/release/qdb-harness --gen all --count 10000 --seed 1

# time the engine against the real tools
./target/release/qdb-bench --dataset small
```

`--load` skips symbolic links and files that are not valid text, and
reports on standard error how many files it loaded and skipped, how long
that took, and how long the index took to build:

```
qdb: some/dir: loaded 1672 files (17.4 MB) in 0.02 s; skipped 13 (0 symbolic links, 13 not valid text, 0 other)
qdb: some/dir: search index built in 0.08 s (4.9 MB, 74777 trigrams)
```

As a library:

```rust
let mut db = qdb::Db::new();
qdb::execute(&mut db, b"mkdir -p docs");
qdb::execute(&mut db, b"echo hello > docs/a.txt");
let out = qdb::execute(&mut db, b"cat docs/a.txt | wc -c");
assert_eq!(out.stdout, b"6\n");
assert_eq!(out.code, 0);
```

`execute` gathers the output. `execute_to` hands it to a `Sink` in pieces
as it is produced, so memory does not grow with the output, and the sink
can say it has had enough, which stops the command:

```rust
struct FirstKilobyte(Vec<u8>);
impl qdb::Sink for FirstKilobyte {
    fn write(&mut self, data: &[u8]) -> bool {
        self.0.extend_from_slice(data);
        self.0.len() < 1024 // false: stop
    }
}
let mut out = FirstKilobyte(Vec::new());
let mut err = Vec::new();
let code = qdb::execute_to(&mut db, b"grep -rn hello .", &mut out, &mut err);
```

`Db::new()` keeps the search index; `Db::without_index()` and
`db.set_index(false)` do without it. Results never depend on the index.
`Db::open_dir(path, Config::default())` opens a database directory and
returns the database with a report of what opening found and did.

## A database on disk

`qdb --db PATH` keeps everything in the directory `PATH`. The layout is
documented byte for byte in `docs/FORMAT.md`. In short: data files that
only grow, holding file content in pieces of whole lines (64 KB unless
`--piece` says otherwise) and, between them, a log record per command
line; an index file per data file; a checkpoint of the tree.

- **A command line happens or does not.** Its record is written when it
  ends; after a crash the database is the result of a whole number of
  command lines, in order. Opening after a crash recovers by itself and
  says on standard error what it did.
- **`--sync always`** (the default): the changes are on disk before the
  line returns, one sync per line. **`--sync batch`**: one sync per 64
  command lines or 4 MB written, whichever comes first; a power cut can
  lose up to the last 63 lines that returned (and nothing earlier). A
  process that is killed while the machine stays up loses nothing that
  returned, under either setting.
- **Reads write nothing.** No read and no write that changes nothing
  causes a disk write or a sync.
- **Memory.** Content is read through file mappings, so it is the
  operating system's cache and not memory of the engine. What the
  engine holds grows with the number of files and pieces (the tree) and
  with the index of the data file being written, not with the text. A
  file is never held whole: a 1 GB file is appended to, edited, copied
  and searched a piece at a time.
- **Damage is an error.** Every record, piece, checkpoint block and index
  block has a checksum. Damage is reported with the file's name and
  nothing more is written; the one exception is an unfinished end of the
  log after a crash, which is cut off. A damaged index file is built
  again.
- **Upkeep** (reclaiming space, saving the tree, writing index files) is
  done in small steps at the end of command lines that write.
  `--compact` does all of it at once and leaves the database settled.
- **Switches.** `--compress` stores pieces LZ4-compressed; `--index
  later` indexes in bulk instead of at each write. `qdb --help` lists
  everything.

## What is supported

Thirty-five commands. Short flags combine (`ls -ltR`, `grep -rin`), and
the long forms listed are accepted too.

| Command | Flags |
|---|---|
| `ls` | `-a -A -R -1 -p -d -r -S -t -l -h` |
| `find` | `-name -iname -path -ipath -type f/d -maxdepth -mindepth -empty -size -newer -mmin -mtime -print -print0 -prune`, `-exec ... ;` and `-exec ... +`, `! -not -a -and -o -or ( )` |
| `cat` | `-n -b -s -A -e -E -t -T -v -u` |
| `head` | `-n N`, `-n -N`, `-c N`, `-NUM`, `-q -v`, size suffixes |
| `tail` | `-n N`, `-n +N`, `-c N`, `-c +N`, `-NUM`, `-q -v`, size suffixes |
| `wc` | `-l -w -c -m` |
| `grep` | `-r -R -i -n -l -L -v -c -o -q -s -w -x -h -H -I -a`, `-E -F -G`, `-e PATTERN` (several), `-A -B -C N`, `-NUM`, `-m N`, `--include --exclude --exclude-dir` |
| `rg` | `-i -S -s -F -w -x -v -c -l -n -N -0`, `-A -B -C N`, `-m N`, `-g GLOB`, `-t TYPE`, `-T TYPE`, `-u -uu -uuu`, `--hidden --no-ignore --files --no-heading --sort path`, `-j N` (accepted; there is one thread) |
| `sed` | `-n -E -r -i -i.SUFFIX -s -e`; addresses `N`, `$`, `/re/`, `\%re%`, `N,M`, `N,+M`, `first~step`, `0,/re/`, `!`; commands `s p d q Q = a i c y` and `{ }` blocks; `s` flags `g p N i I`; `&` and `\1`..`\9` |
| `diff` | `-u`, `-U N`, `--unified[=N]`, `-r`, `-N`, `-x PAT`, `-q`, `-s`, `-i -w -b -B`, `-a`, `--label NAME`; the default and the unified format; `-` for standard input |
| `sort` | `-u -n -r -h -f -b -s -c -C`, `-t SEP`, `-k` with character positions and modifiers, `--parallel=N` (accepted) |
| `uniq` | `-c -d -u -i`, `-f N`, `-s N`, `-w N` |
| `cut` | `-b -c -f`, `-d`, `-s`, `--complement`, `--output-delimiter` |
| `tr` | `-d -s -c -C -t`, ranges, classes, `[c*n]` repeats, escapes |
| `nl` | `-b -h -f` with `a t n`, `-v -i -w -s`, `-n ln/rn/rz` |
| `tee` | `-a`, several files |
| `du` | `-s -h -a -c -d N -b -k -m -x`, `--apparent-size`, `--exclude=PAT` |
| `date` | `-u`, `-d WHEN`, `-I`, `-R`, `+FORMAT` |
| `xargs` | `-n -I -i -L -l -0 -d -r -t -s -x`, `-P 1` |
| `printf` | `%s %b %c %d %i %u %x %X %o %f %F %e %E %g %G %%`, flags, width, precision, escapes |
| `test`, `[` | `-f -d -e -s -z -n`, `= == !=`, `-eq -ne -lt -le -gt -ge`, `! -a -o`, quoted parentheses |
| `seq` | `-s -w -f` |
| `basename` | `-a -s SUFFIX -z` |
| `dirname` | several operands, `-z` |
| `echo` | `-n` |
| `true`, `false`, `:`, `sleep` | |
| `mkdir` | `-p` |
| `touch` | `-c`, `-d WHEN`, `-t STAMP` |
| `cp` | `-r -R -a -p -f -n -T -v` |
| `mv` | `-f -n -v` |
| `rm` | `-r -R -f -d -v` |
| `rmdir` | `-p` |

Shell syntax, as bash 5.2 reads it:

| Kind | What |
|---|---|
| Lists | `;`, newline, `&&`, `||` |
| Pipes | `a \| b \| c`, with the exit status of the last stage; `! pipeline` turns the status round |
| Redirections | `>`, `>>`, `<`, `2>`, `2>>`, `2>&1`, `>&2`, `&>`, `&>>`, and `/dev/null` as a source or a target |
| Here-documents | `<<EOF`, `<<'EOF'`, `<<"EOF"`, `<<-EOF`, several on one line |
| Substitution | `$(...)` and backticks, nested, in or out of double quotes |
| Braces | `{a,b}`, nested, `{1..9}`, `{1..9..2}`, `{a..z}`, `{01..10}` |
| Globs | `*`, `?`, `[...]` with bash's default rules |
| Quoting | single quotes, double quotes, backslash, line continuation |
| Variables | `NAME=value`, `NAME=value command`, `$NAME`, `${NAME}`, `${#NAME}`, `${NAME:-word}`, `${NAME:=word}`, `$?`, `export`, `unset`, `read` with `-r` and several names, `IFS` |
| Arithmetic | `$((...))` with bash's operators on 64-bit integers, assignments and `++`/`--` included, and constants in base 8, 16 and `base#digits` |
| Tilde | `~` and `~/path`, where a word or an assigned value begins |
| Field splitting | unquoted `$NAME`, `$(...)` and `$((...))` are split at `IFS` and then matched as globs, as bash does |
| Current directory | `cd`, `cd -`, `pwd`, `PWD` and `OLDPWD`; relative paths start there |
| Loops and tests | `for NAME in WORDS; do ...; done`, `while`, `until`, `if`/`elif`/`else`/`fi`, `break [N]`, `continue [N]`, nested, in a pipeline, and with redirections on the whole construct |

What the data model has no fact for prints a fixed value: `ls -l` shows
mode `-rw-r--r--` or `drwxr-xr-x`, owner and group `qdb`, and a folder's
size as 4096; `du` and the `total` line count content in 4 KB blocks.
The one fact kept per file and folder besides name and content is the
modification time. Dates print in UTC.

Anything else is refused before any part of the unit runs, with exit
status 2 and a message that names what was refused:

```
$ qdb -c 'echo ${X%.c}'   qdb: unsupported syntax: parameter expansion ${X%.c}
$ qdb -c 'case $X in a) ls;; esac'
                          qdb: unsupported syntax: case statement
$ qdb -c 'f() { ls; }'    qdb: unsupported syntax: function definition
$ qdb -c 'ls --color'     qdb: unsupported flag: ls --color (long options other than the ones listed in the README are not supported)
$ qdb -c 'sort -g a'      qdb: unsupported flag: sort -g
$ qdb -c 'ln -s a b'      qdb: unsupported command: ln (the database has no links, hard or symbolic)
```

The check before running covers what the text of the unit shows, the
bodies of loops and the insides of `$(...)` included. A command name or
a flag that only an expansion reveals (`X=awk; $X 1`) is refused when
the unit reaches it, with the same message; what ran before it stays
done, as it does when a command fails.

A unit is one line, or several when a quote, a here-document or a
construct is still open at the end of the line. After a crash everything a unit
wrote is there or none of it is, and a unit under `--sync always`
ends with one sync, however many commands its loops ran.

**Sessions.** Variables, the current directory and `$?` belong to a
session, not to the database. `qdb` reading lines from standard input or
a file is one session from the first line to the last; `qdb -c` is a
session for its one argument. A session starts with `HOME=/`, `PWD=/`
and the usual `IFS`, in the root folder. In the library a caller makes a
`Session` and passes it to `execute_in`; `execute` runs a unit in a
fresh one. Each stage of a pipeline and each `$(...)` works on a copy,
as bash's subshells do, so `echo a | read X` leaves `X` unset.

**The command limit.** A unit may run 1,000,000 commands (`--command-limit N`;
loop turns count too). The unit that reaches the limit is stopped with
status 3 and the message `qdb: the unit was stopped after N commands,
the most one unit may run; what it changed before that stays`. As the
message says, its writes up to there are kept, as they are when a
command fails half way through a unit, and the database goes on as
before.

`find -exec` and `xargs` run the engine's commands, but not the ones
that only make sense inside a session: `cd`, `export`, `unset`, `read`,
`break`, `continue` and `:` are refused there.

**diff.** `diff` gives the lines GNU diff 3.10 gives, in the default and
the unified format, with the same exit status (0 the same, 1 different,
2 trouble). It uses what the storage knows: two files that share their
pieces are the same without a byte being read, the equal start and end
of two files are skipped piece by piece, and only the part between is
compared line by line. That part is held in memory; if it is larger than
256 MB (two unrelated huge files), `diff` refuses with exit status 2 and
says so, instead of using memory without limit. The times in a unified
header are the files' modification times in UTC. `-c`, `-y`, `-e`, `-I`
and the other output formats are refused by name.

**One difference from bash in constructs.** When the output of a whole
loop or `if` is redirected to a file, the file receives it when the
construct ends. A command inside the construct that reads that same
file, or appends to it under its own redirection, sees the file without
the construct's output so far.

## How it is put together

| File | Job |
|---|---|
| `src/tree.rs` | The storage layer. The data model (one strict tree of folders and files, ids, names, parent links), file content as pieces in data files, the log, checkpoints, recovery, upkeep, `--check`. Also the storage layer's one question for searches, `Db::candidates`: which files under this folder can match this query? The same code serves a database directory and a tree in memory. |
| `src/disk.rs` | The files of the format: data files and their records, index files, checkpoints, MANIFEST. |
| `src/vfs.rs` | The one place the engine touches files: a real directory, or files in memory that can also show what a power cut would leave (the crash test). |
| `src/query.rs` | The query a search hands to storage: AND and OR over trigrams. |
| `src/index.rs` | The trigram index in memory, for what is not yet in an index file, and the evaluation of a query over lists from either. Derived data: it can be thrown away and built again. |
| `src/shell.rs` | A recursive-descent parser that follows the structure of the bash grammar (list, pipeline, command, redirect, word), so later syntax has a place to go. |
| `src/glob.rs` | Pathname expansion with bash's rules. |
| `src/regex.rs` | POSIX basic and extended regular expressions with leftmost-longest matching, plus the GNU extensions grep and sed accept. This matcher is the reference for what matches. |
| `src/regex/dfa.rs` | A DFA built as it runs, for patterns without back-references. |
| `src/regex/trigram.rs` | The trigram query for a regular expression. |
| `src/scan.rs` | Finding the matching lines of a buffer fast: fixed strings, required strings, the DFA, and the reference matcher as the last resort. |
| `src/bytes.rs` | Substring search, including one that ignores case, and the engine's single definitions of "one character" and of case folding. |
| `src/engine.rs` | Runs a parsed line: expands globs, checks every stage is supported, runs the stages as a stream, applies redirects, undoes a write that is not valid text. |
| `src/cmd/` | One module per group of commands. |
| `src/store.rs` | Loading a directory or fixture into a database and dumping it back. Only used by the tools, never by the engine. |
| `src/testing.rs` | Running a command line on the real tools in a scratch directory. |
| `src/bin/harness.rs` | The test harness. See `tests/README.md`. |
| `src/bin/bench.rs` | The benchmark. See `bench/README.md`. |
| `src/bin/stress.rs` | One very large file, a long run of writes, the cost of a sync, and the comparison of the switches. See `bench/README.md`. |

## Where to read next

- `docs/briefs/`: the three briefs this was built from.
- `docs/HANDBACK-3.md`: stage 3. What was built, every target with its
  measured value, the crash tests and every fault they found, disk and
  memory in numbers, write costs, the comparison of the switches,
  deviations, gaps and questions.
- `docs/FORMAT.md`: the on-disk format, byte for byte.
- `docs/HANDBACK-2.md`: stage 2. What was built, every target with its
  measured value, the index in numbers, deviations, gaps and questions.
- `docs/HANDBACK.md`: the same for stage 1.
- `tests/README.md`: how the harness works and how to add a case.
- `bench/README.md`: the datasets, the command list and the results.
