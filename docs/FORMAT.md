# The on-disk format

This is the exact layout of a qdb database directory, format version 3.
Version 3 differs from version 2 in three things, all to make a small
database small (stage 6 hosts many of them): a checkpoint of at most
3,072 bytes is kept inside `MANIFEST` instead of in a file of its own; an
index file says how many bucket bits it uses instead of always 16; and
the data file being written gets an index file at close only once it
holds 256 KB. Version 2 differed from version 1 in three things: nodes
have a modification time (in checkpoints, and as change 6 in the log), a
piece may be used by several files, and a write that changes no content
may still log a time.
The code that reads and writes it is `src/disk.rs` (the files) and
`src/tree.rs` (what is put in them, and recovery). Everything goes through
the I/O layer in `src/vfs.rs`.

All numbers are little-endian. A *varint* is an unsigned number in 7-bit
groups, low group first, with the top bit of a byte set when another byte
follows. A *string* is a varint length followed by that many bytes.
A *checksum* is CRC-32 (the IEEE polynomial, as computed by the
`crc32fast` crate) of the bytes it is said to cover.

## The directory

| File | What it is | Written |
|---|---|---|
| `LOCK` | Empty. An exclusive `flock` on it is held while the database is open, so a second process is refused. | never |
| `MANIFEST` | Which checkpoint is current, and the checkpoint itself when it is small. 24 bytes plus the checkpoint kept in it. | replaced whole |
| `tree-NNNNNNNN.ckp` | A checkpoint: the folder tree, and the position in the log from which the log has to be applied to it. Only when it is larger than 3,072 bytes; a smaller one is kept in `MANIFEST`. | written once |
| `NNNNNNNN.seg` | A data file: file content and the log. `NNNNNNNN` is its number in decimal, from 1. | at its end only |
| `NNNNNNNN.idx` | The search index for the pieces in data file `NNNNNNNN`. | written once, replaced whole |
| `*.tmp` | A file being written that has not yet been given its name, or (`spool-N.tmp`) output of a command on its way into a file. Removed at open. | |

Nothing is ever changed in place. A data file only grows at its end (and
after a crash is cut back to its last whole command line). Every other
file is written under a `.tmp` name, synced, renamed to its real name, and
the directory is synced; a reader sees the old file or the new one, never
part of one.

A data file or index file that is no longer needed is not removed in one
go but cut shorter a megabyte at a time and removed when empty (removing a
large file at once stalls the write that does it). Such a file belongs to
nothing; if the process ends first, the next open removes it.

Every file starts with an 8-byte magic and a 4-byte format version.
Any version but the reader's own is refused, a higher one as "written by
a newer version of qdb" and a lower one as "written in format N by an
earlier version": nothing is guessed and nothing is converted.

## Pieces

The content of a file is a row of *pieces*. Each piece is a run of whole
lines: every piece of a file except its last ends with a newline. A piece
holds at most the piece size (65,536 bytes unless `--piece` says
otherwise) unless a single line is longer than that, in which case the
line is a piece by itself. The piece size is a setting of the writer, not
of the format: a database may hold pieces written with different sizes.

A piece is stored in one data file as a run of bytes, and is known by
eight numbers:

| Field | Meaning |
|---|---|
| `seg` | number of the data file |
| `ord` | position of the piece among all pieces ever stored in that data file, from 0. The index knows the piece as document `ord + 1`. |
| `off` | offset of its stored bytes in the data file |
| `slen` | number of stored bytes |
| `len` | number of bytes of text |
| `lines` | number of newline bytes in the text |
| `crc` | checksum of the stored bytes (of the compressed bytes, for a compressed piece) |
| `enc` | encoding marker: how the stored bytes become text. `0`: they are the text (UTF-8 without NUL bytes). `1`: they are one LZ4 block (the block format, no frame) that unpacks to exactly `len` bytes of text; written with `--compress` for a piece that comes out smaller that way. A database may hold pieces of both kinds. A reader that meets a value it does not know reports an error naming the file. |

When a piece is written into a checkpoint or a log record it is eight
varints in this order: `seg ord off slen len lines crc enc`.

**Shared pieces.** The same piece (the same `seg` and `ord`) may appear in
the rows of several files, and more than once in one row. That is how a
copy is stored: `cp` writes no content, only a splice that gives the copy
the pieces of its source. Nothing on disk counts the uses of a piece. A
reader finds them by reading the tree: a piece is in use while any file's
row holds it, and its bytes may be reclaimed only when none does. A
change to one of the files replaces pieces in that file's row and leaves
the other rows as they were, so sharing never shows.

## Modification times

Every node has a modification time: a signed 64-bit count of nanoseconds
since 1970-01-01 00:00:00 UTC. It is written as a varint of the same 64
bits read as unsigned. The engine sets it as a filesystem would: a file's
when its content is written (also when the content written is what it
held, and when it is opened to be written anew) or it is touched; a
folder's when an entry in it is made, removed or renamed. A moved node
keeps its time. An import keeps the times the files had.

## Data files (`NNNNNNNN.seg`)

```
offset 0   8 bytes   magic "QDBSEG1\n"
       8   u32       format version (3)
      12   u32       this file's number
      16   u32       folding rule of the index built over this file (1)
      20   8 bytes   zero
      28   u32       checksum of bytes 0..28
      32   records, one after another, to the end of the file
```

The folding rule says how letters were folded when trigrams were taken
for the index. Rule 1: the ASCII letters `A`-`Z` are folded to `a`-`z`;
every other byte is itself. An index built with one rule cannot answer
for another, so the rule is recorded here and in every index file.

A record:

```
u32   length of the payload
u8    kind: 1 content, 2 command line, 3 next file
3     zero bytes
u32   checksum of the 8 bytes above and the covered part of the payload
...   payload
```

**Content record (kind 1).** Pieces of one file's content.

```
u32          ord of the first piece in this record
u32          n, the number of pieces
n times:     u32 enc (low byte; the rest zero), u32 slen, u32 len, u32 lines, u32 crc
then         the stored bytes of the n pieces, end to end
```

The record's checksum covers the header and the table of pieces, not the
bytes of the pieces: each piece has its own checksum in the table. So a
record can be found and stepped over without reading content, and a piece
can be checked by itself. One record holds at most 64 MB of content; a
file up to that size written in one go is one record, its pieces lying
end to end. The `ord` of the first piece of each content record is one
more than the `ord` of the last piece of the content record before it,
starting from 0 in each data file.

**Command line record (kind 2).** What one command line changed in the
tree. Written once, when the command line ends, after the content records
it refers to. This record is the commit: a command line whose record is
not whole in the file did not happen.

```
u64    number of this command line; each is one more than the one before
u64    how many bytes of this data file were known to be on disk (synced)
       when this record was written
then   changes, to the end of the record, each starting with a varint:
```

| Change | Fields | Meaning |
|---|---|---|
| 1 make folder | varint id, varint parent id, string name | |
| 2 make file | varint id, varint parent id, string name | the file is empty |
| 3 splice | varint id, varint version, varint at, varint del, varint n, then n pieces | in file `id`, replace `del` pieces starting at position `at` with the `n` given; the file is then at `version` |
| 4 remove | varint id | one node; a folder removed with its contents is one of these per node |
| 5 rename | varint id, varint parent id, string name | the node keeps its id |
| 6 time | varint id, varint time | the node's modification time; nothing if there is no such node |

Changes 1 and 2 carry no time. A record holds one change 6 for every node
whose time the command line set and that still exists at its end, with
the last time set; they follow the other changes of the command line,
though moves made by upkeep may come after them. A node made by change 1
or 2 has time 0 until a change 6 names it, which the same record always
does.

Only command lines that change the tree are numbered and logged. A read
writes nothing. A write that leaves all content as it was writes no
content, and a record only if it sets a time (as `touch` does, or `>`
with the same output as before). A record
may also hold splices that move a file's content from one data file to
another without changing it (upkeep: see below); they ride in the record
of the command line during which they were done. `--compact` and
`--load` write records of their own, numbered like command lines.

Every file has a version, 0 when it is made, one higher with every
splice. Applying a splice whose version is not above the file's version
does nothing, and making a node that exists does nothing, so applying a
stretch of the log to a tree that already has part of it comes out right.

**Next file record (kind 3).** Payload: `u32`, the number of the data file
the log goes on in. It is the last record of a data file that is
finished. A finished data file is synced before the next one is used.

A data file is at most 128 MB unless a single record needs more.

## The checkpoint (`tree-NNNNNNNN.ckp`)

```
offset 0   8 bytes   magic "QDBTREE\n"
       8   u32       format version (3)
      12   u32       this checkpoint's number
      16   u32       checksum of bytes 0..16
      20   blocks, one after another
then       end mark:  u32 zero, 8 bytes "QDBTREND", u64 number of blocks,
                      u32 checksum of those 20 bytes
```

A block is `u32 length`, `u32 checksum of the entries`, then entries; a
length of zero is the end mark. Blocks hold about 64 KB. An entry starts
with a varint tag:

| Tag | Fields |
|---|---|
| 0 position | varint: number of the last command line the tree includes; varint: data file the log goes on in; varint: offset in it; varint: `ord` the next piece stored in it will get; varint: the next node id to hand out |
| 1 folder | varint id, varint parent id plus 1 (0 for the root), string name, varint time |
| 2 file | varint id, varint parent id plus 1, string name, varint time, varint version, varint n, then n pieces |

The position entry comes first, in a block of its own. The root folder has
id 0. Entries are in ascending order of id; a folder's children are found
by their parent ids.

A checkpoint written while the database is in use is written a few
kilobytes at a time, as command lines end. The position entry is the place
the log had reached when the writing began; each node is written as it is
when its turn comes, so a node may already include changes logged after
that position, and a node made after it may be missing or present. That
is sound because applying the log is idempotent (versions, above): the log
from the position on, applied to such a tree, gives the tree as of the end
of the log. Before the checkpoint is made current, the log is synced up
to where it stood when the last node was written, so the checkpoint never
holds a change that the log on disk does not.

Node ids are never reused: a node keeps its id through moves and
restarts, and the id of a removed node is not given out again.

## MANIFEST

```
offset 0   8 bytes   magic "QDBMANI\n"
       8   u32       format version (3)
      12   u32       number of the current checkpoint
      16   u32       n: length of the checkpoint kept here, 0 if it is in
                     its own file
      20   n bytes   the checkpoint, byte for byte as it would be in
                     tree-NNNNNNNN.ckp
  20 + n   u32       checksum of bytes 0 .. 20 + n
```

The version is checked before the checksum, so that a MANIFEST of another
version is refused as that, not as damage. A checkpoint is written to
memory first; only when it grows past 3,072 bytes is its file made and
what was written so far put in it. A database of a few files is
therefore three small files: `LOCK`, `MANIFEST` and one data file.

## Index files (`NNNNNNNN.idx`)

The index is derived from content and can always be built again. An index
file holds, for every trigram that occurs in the pieces of one data file,
the list of pieces it occurs in.

```
header   offset 0   8 bytes  magic "QDBIDX1\n"
                8   u32      format version (3)
               12   u32      number of the data file
lists    offset 16: every posting list, end to end, in ascending order of
         trigram
table    one u64 per trigram, ascending: trigram << 40 | offset of its
         list, counted from the start of `lists`. Then one more entry,
         0xFFFFFF << 40 | length of `lists`, so that every list's end is
         the next entry's offset.
buckets  2^b + 1 u32, where b is the bucket bits in the footer: for each
         value of a trigram's top b bits, the position in `table` of the
         first trigram at or above it. b is the least number, at most 16,
         that leaves no more than about 8 trigrams to a bucket, so that a
         file of few trigrams has few buckets.
sums     one u32 per 4,096 bytes of everything above: its checksum.
footer   72 bytes:
               0   8 bytes  "QDBIDXE\n"
               8   u32      ndocs: the lists cover the pieces with ord < ndocs
              12   u32      number of trigrams
              16   u64      offset of lists (16)
              24   u64      length of lists
              32   u64      offset of table
              40   u64      offset of buckets
              48   u64      offset of sums
              56   u32      folding rule (1)
              60   u32      checksum of sums
              64   u32      bucket bits b (0 to 16)
              68   u32      checksum of footer bytes 0..68
```

A trigram is three bytes of folded text read as a 24-bit number, first
byte highest. A posting list is the numbers of the documents (pieces: `ord
+ 1`) the trigram occurs in, ascending, each written as a varint of the
difference from the one before (the first as itself). A difference of 0
never occurs between two entries, and is used as a mark: after a 0 byte
the next entry is a document number in full, not a difference. (An index
file made by joining an older index file with newer entries has such
marks where the parts meet.)

A list may name pieces that no file uses any more; a reader drops them by
looking the piece up. A list never lacks a piece that is in use and has
the trigram. "In use" is by the log on disk: the log is synced before an
index file is given its name, so that an index file never reflects a
change that a power cut could still undo.

The index file of a finished data file covers all its pieces, once it is
written: that happens a step at a time over the command lines after the
data file is finished, and until then (and after a crash in between) the
file is missing or covers only the first `ndocs` pieces. The data file
still being written may likewise have an index file that covers only its
first `ndocs` pieces; it is given one at close only once it holds 256 KB
of records, and below that its content is indexed again at open, which
is quicker than reading a file. The rest are indexed in memory as they are written
(or later in bulk, with `--index later`), and after a crash from their
content.

## Opening, and recovery

1. Take the lock. Read `MANIFEST`; if there is none and the directory
   holds nothing else, make a new database. If there is none and there are
   other files, refuse.
2. Read the checkpoint it names (kept in `MANIFEST`, or in its own
   file), checking every block.
3. Read the log from the position the checkpoint gives: record after
   record, following next-file records from one data file to the next.
   Only record headers and command line records are read, not content.
   A data file is synced before the next one is made; if a data file
   names a next one that is missing or has no whole header, the power went
   in between, the next one is made now, and this file is treated as the
   last one in step 4.
4. In the last data file the records after the last sync that is known
   (the largest synced offset carried by a command line record) may be
   there only in part. The content records among them are checked against
   their piece checksums. The log ends at the last command line record
   before the first record that is not whole, is out of sequence, or has
   content that does not match; the data file is cut back to there and
   synced. This is the only place where damage is treated as "did not
   happen" rather than as an error. If nothing had to be cut but command
   lines were applied, the file is synced all the same: what was read may
   never have been, and what recovery writes next is made from it.
5. Apply the command line records up to there to the tree, in order.
6. Remove files that belong to nothing: `*.tmp`, data and index files
   that no piece or log position refers to, checkpoints other than the
   current one.
7. Open the index file of each data file, checking its header, footer,
   table, buckets and sums. One that is missing, damaged, built with
   another folding rule, or that does not cover a finished data file is
   built again from the content, and the open says so (except, quietly,
   for a small data file being written, which has none by design).

Opening reads no file content unless an index has to be built or step 4
has records to check. Its cost follows the number of nodes and pieces (the
checkpoint) and the length of the log since the checkpoint.

A checkpoint is written when the database is closed and when `--compact`
runs (in one go), and in steps, as described above, when the log since
the last one is longer than 1 MB and longer than twice the last
checkpoint, or when a data file has become empty and could be removed.

## Upkeep

Nothing in the format needs upkeep to stay correct; upkeep keeps space
and recovery time bounded. All of it is done in bounded steps at the end
of command lines that write, each step sized by what that command line
wrote, and `--compact` does all of it at once.

- **Space.** When the bytes of data files that no file uses pass 30% of
  the bytes in use (plus 4 MB), content is moved out of the data file
  with the largest unused share into the one being written: a content
  record for the piece, and a splice in the command line's record (one
  splice for every file that uses the piece, all naming the same new
  piece, so that the files still share it). A
  command line moves at most 4 times the bytes it wrote (16 times once
  unused space passes 60%). A data file that holds nothing in use, and
  that the log from the current checkpoint does not run through, is
  removed after the next checkpoint.
- **Index files** and **checkpoints** are written in steps as said above.
- **Lists in memory** drop pieces no longer in use a little at a time.

## What is checked, and when

| What | Checked |
|---|---|
| data file header | when the file is opened |
| record headers and command line records | when read at open; all of them by `--check` |
| the bytes of a piece | the first time the piece is read after the database is opened; all of them by `--check` |
| checkpoint | every block, at open |
| MANIFEST | at open |
| index file header, footer, table, buckets, sums | at open |
| index lists | each 4,096-byte block the first time a list in it is read; all of them by `--check` |

A piece or record that does not match is an error that names the file; the
command stops, nothing more is written, and the next open recovers every
whole command line. An index block that does not match is the same, except
that the index file is also removed, so that the next open builds it again.

`qdb --db PATH --check` additionally checks that the tree is sound (every
node in its folder under its name, file counts, piece lengths and line
counts, every piece but a file's last ending with a newline, valid text),
that the tree and the data files agree about every piece, and that for
every trigram the stored lists name exactly the pieces in use that a fresh
index of the same pieces names.
