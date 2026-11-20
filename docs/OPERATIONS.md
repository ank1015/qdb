# Running qdb-server

qdb-server is one process that hosts many filesystems under one data
directory and answers the HTTP/JSON API of docs/API.md. This is how to
run it, what it keeps on disk, and what to do when something goes wrong.

## Building

```
cargo build --release
./target/release/qdb-server --help
```

The program is `target/release/qdb-server`. It needs nothing else at
run time: no database, no runtime, no configuration files.

## Starting

```
umask 077
head -c 32 /dev/urandom | base64 > /etc/qdb/admin.key
qdb-server --data /var/lib/qdb --admin-key-file /etc/qdb/admin.key
```

The server makes the data directory if it is not there, opens it, and
prints the address it listens on to standard output (one line, such as
`127.0.0.1:7878`); then it logs to standard error. It listens on
127.0.0.1:7878 unless told otherwise. With `--listen 127.0.0.1:0` the
system picks a free port, which the line on standard output names.

The admin key is read, in this order, from `--admin-key-file FILE` (the
first line of the file), the `QDB_ADMIN_KEY` environment variable, or
`--admin-key KEY`. The last is visible to other users of the machine
in the process list, so avoid it. The key must be at least 16
characters. The server refuses to start without one.

Only one server can use a data directory at a time: a second one stops
at once with `another server is using this data directory`. The lock is
an flock on `DATA/LOCK`, which the system releases when the process
ends however it ends, so there is never a stale lock to remove.

There is no TLS. Put a proxy that terminates TLS in front of the server
(nginx, Caddy, a load balancer), or keep it on a private network.

## Stopping

Send SIGTERM (or SIGINT, Ctrl-C). The server then:

1. Answers new requests with 503 `the server is shutting down`.
2. Stops every running unit. Each is answered with
   `"stopped":"shutdown"` and exit status 143, and nothing it changed is
   kept. Units already answered are on disk.
3. Closes every filesystem and writes the usage totals.
4. Closes the connections, logs `stopped`, and exits with status 0.

This takes as long as the slowest unit takes to notice, which is
milliseconds. There is no need to drain traffic first, but a proxy in
front should stop sending before the server stops, so that clients see
fewer 503 answers.

Killing the server with SIGKILL, or a power cut, loses nothing that was
answered (see After a crash).

## Settings

Every setting is a flag, and also a field of a JSON object in the file
given with `--config FILE` (the flag's name without the dashes, with `_`
for `-`). A flag wins over the file.

```
{"listen":"0.0.0.0:7878","workers":16,"time_ms":10000,"units_at_once":8}
```

| Flag                    | Default              | What it does                                                                                     |
|-------------------------|----------------------|--------------------------------------------------------------------------------------------------|
| `--data DIR`            | (required)           | The data directory                                                                               |
| `--listen ADDR`         | 127.0.0.1:7878       | Address and port to listen on                                                                    |
| `--admin-key-file FILE` |                      | The admin key, from a file                                                                       |
| `--sync always\|batch`  | always               | `always`: a unit's changes are on disk before it is answered. `batch`: synced every 64 units or 4 MB on each filesystem, so a power cut can lose the last few answered units (never an earlier one while keeping a later one) |
| `--group-commit on\|off`| on                   | Under `always`, writers on one filesystem share syncs: a writer lets the filesystem go before its sync, and one sync covers every unit written by then. A unit is still answered only once it is on disk. `off` makes each writer sync alone, holding the filesystem |
| `--workers N`           | 2 per core           | Units running at once over all accounts. Idle workers also help a long `grep -r` or `rg` search |
| `--queue N`             | 256                  | Units that may wait for a worker. More are answered 429                                          |
| `--max-connections N`   | 1024                 | Connections open at once. More are answered 503 and closed                                       |
| `--max-request-bytes N` | 16777216             | The largest request body. Larger is answered 413                                                 |
| `--output-cap-bytes N`  | 1048576              | Standard output, and standard error, kept for one answer that is not streamed                    |
| `--session-idle-secs N` | 900                  | A session not used for this long ends                                                            |
| `--max-open N`          | 1024                 | Filesystems open at once. Past it, the least recently used is closed                             |
| `--max-open-bytes N`    | 2147483648           | Memory open filesystems may hold together, about. Past it, the least recently used is closed     |
| `--idle-close-secs N`   | 60                   | A filesystem no unit has used for this long is closed                                            |

Defaults for new accounts (each can be set per account through the API;
see docs/API.md, Settings):

| Flag                    | Default       | Per                               |
|-------------------------|---------------|-----------------------------------|
| `--text-bytes N`        | 10737418240   | filesystem: text it may hold      |
| `--nodes N`             | 1000000       | filesystem: files and folders     |
| `--disk-bytes N`        | 21474836480   | filesystem: disk it may use       |
| `--filesystems N`       | 1000          | account: filesystems it may have  |
| `--time-ms N`           | 30000         | unit: time                        |
| `--commands N`          | 1000000       | unit: commands run                |
| `--scratch-bytes N`     | 1073741824    | unit: scratch space               |
| `--memory-bytes N`      | 1073741824    | unit: memory, all its threads     |
| `--units-at-once N`     | 16            | account: units running or waiting |

Changing a default changes it for accounts made afterwards, not for
accounts that exist: their settings were stored when they were made.

Two settings matter most for capacity. `--workers` bounds the CPU the
server uses: at 2 per core, a machine full of long searches still
answers small units, slower. `--max-open` and `--max-open-bytes` bound
the memory of open filesystems: an idle filesystem costs no memory and
no open files once closed, and opening a small one again takes well
under a millisecond.

## What is on disk

```
DATA/LOCK                       held while a server runs
DATA/server/registry.log        accounts, key hashes, settings, and which
                                account owns which filesystem
DATA/server/usage.log           usage totals by filesystem and account
DATA/fs/ab/ab12...ef/           one filesystem: a qdb database directory
DATA/fs/ab/ab12...ef/LOCK       held while the filesystem is open
DATA/fs/ab/ab12...ef/MANIFEST   which data files make the database
DATA/fs/ab/ab12...ef/*.seg      data files; tree-*.ckp checkpoints
DATA/trash/                     filesystems being deleted
```

A filesystem's directory is named by its ID, under a folder named by
the ID's first two characters, so that no folder holds more than a few
thousand entries. Its format is in docs/FORMAT.md. Each filesystem is a
database qdb itself can open: `qdb --db DATA/fs/ab/ab12...ef --check`
checks one, and `qdb --db ... -c 'ls -R'` reads one, while the server is
stopped.

`registry.log` has one JSON line per change, and each line is synced
before the change is answered. `usage.log` has a line for each
filesystem whose totals changed, appended every second and at shutdown.
At start each file is read, the last line for each account or
filesystem winning, and written again whole; `usage.log` is also
written whole again whenever what was appended to it reaches several
times its size. Keys are never on disk, only their SHA-256 hashes.

A deleted filesystem is renamed into `DATA/trash/` and then removed. A
crash between the two leaves it there, and the next start empties the
folder. A filesystem in `DATA/fs/` that no account owns (one made just
before a crash, before its owner was recorded) is removed at start, and
the log says so.

## Copying and backing up

**Stop the server, then copy the whole data directory.** Copy
`DATA/server/` and `DATA/fs/` together: the registry says who owns each
filesystem, and a filesystem without its registry line is removed at
the next start, as one no account owns.

```
systemctl stop qdb            # or kill -TERM
rsync -a /var/lib/qdb/ /backup/qdb/
systemctl start qdb
```

A copy taken while the server runs is not safe: a filesystem's files
can change between the reads of the copy. A snapshot of the whole
volume taken at one instant (LVM, ZFS, a cloud disk snapshot) is safe,
because qdb recovers from it as from a power cut at that instant.

To move one filesystem to another server there is no command yet. A
filesystem's directory can be copied while the server is stopped, but
the target server must also be told its owner, which today means an
account line and a filesystem line in its registry.

## The log

The server logs to standard error, one line per event, each starting
with the time in UTC:

```
2026-11-19T20:34:26Z opened /var/lib/qdb: 12 accounts, 3405 filesystems
2026-11-19T20:34:26Z listening on 127.0.0.1:7878
2026-11-19T20:41:03Z removing filesystem 5c0e..., which no account owns
2026-11-19T20:52:17Z filesystem 9a1f...: damage found: ...; opened again from disk
2026-11-19T20:52:17Z filesystem 9a1f...: a sync failed: ...; opened again from disk on its next unit
2026-11-19T20:52:18Z filesystem 9a1f... could not be opened: ...
2026-11-19T21:00:00Z usage could not be written: No space left on device
2026-11-19T21:10:42Z stopping
2026-11-19T21:10:42Z stopped
```

Requests are not logged. Units are counted in the usage totals (docs/API.md,
Usage), which a monitor can read through the API.

What each line means, and what to do:

- **opened, listening, stopping, stopped:** normal.
- **removing filesystem ..., which no account owns:** a filesystem was
  being made when the server last stopped without a clean shutdown. Its
  creation was never answered, so no one had its ID. Nothing to do.
- **damage found ... opened again from disk:** a unit read something on
  disk that failed its checksum. The unit was answered with exit status
  2 and `"stopped":"damage"`, and the filesystem was closed and opened
  again, which recovers what can be recovered. If it happens again on
  the same filesystem, stop the server and run
  `qdb --db DATA/fs/xx/ID --check` on it, and check the disk.
- **a sync failed ... opened again from disk on its next unit:** the
  disk refused a sync, so units waiting for it were answered with
  `"stopped":"fault"` and the filesystem is opened again from what is on
  disk. Check the disk (full, failing, or gone read-only).
- **could not be opened:** units on that filesystem are answered with
  `"stopped":"fault"` until it can be. Check the disk, then
  `qdb --db ... --check`.
- **usage could not be written:** the totals are kept in memory and
  written at the next success. Usually a full disk.

## After a crash

Start the server again; there is nothing else to do. Each filesystem
recovers when it is next opened: the part of its log after its last
checkpoint is applied again, and a unit that was cut off part way is
dropped whole. Every unit that was answered under `sync always` is
there, and none that was not answered is there in part. The registry
and the usage log are read up to their last whole line.

The kill test in bench/README.md does exactly this 200 times under load
and checks every filesystem after each kill.

If the server will not start:

- `another server is using this data directory`: one is still running.
  Find it with `fuser DATA/LOCK`.
- An error naming a file in `DATA/server/`: that file could not be read
  or written; check permissions and free space.
- `registry.log: line N is damaged`, or `registry.log is missing or
  empty, but the data directory holds N filesystems`: the server will
  not start without knowing who owns each filesystem, since it would
  otherwise remove them as owned by no one. Restore `DATA/server/` from
  the last copy. Accounts and filesystems made since that copy are not
  in it, and their filesystems will be removed at the next start; move
  any you want to keep out of `DATA/fs/` first. (Only a damaged last
  line, which a crash leaves, is dropped quietly.)

## Running under systemd

```
[Unit]
Description=qdb
After=network.target

[Service]
ExecStart=/usr/local/bin/qdb-server --data /var/lib/qdb --admin-key-file /etc/qdb/admin.key --config /etc/qdb/config.json
KillSignal=SIGTERM
TimeoutStopSec=30
LimitNOFILE=65536
User=qdb

[Install]
WantedBy=multi-user.target
```

Each open filesystem holds about three open files, and each connection
one, so with the defaults the server may hold about 4,200: raise
`LimitNOFILE` above the default 1,024.

## Sizing

docs/HANDBACK-6.md has a capacity sheet for a 4-core machine: what a
filesystem costs on disk and in memory, units a second by workload and
clients, and which limit is reached first.
