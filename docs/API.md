# The qdb API

qdb-server hosts many filesystems, each a tree of folders and text files
owned by one account, and runs bash command lines against them. This is
every request it takes and every answer it gives. How to run the server
is in docs/OPERATIONS.md.

The API is HTTP/1.1 with JSON bodies. There is no TLS: put the server
behind a proxy that terminates it, or keep it on a private network.
Connections are kept open between requests unless the client asks
otherwise (`Connection: close`).

## Keys

Every request but `GET /v1/health` carries a key:

```
Authorization: Bearer KEY
```

There are two kinds of key.

- **The admin key** is set when the server is started. It makes, changes
  and deletes accounts, and can do nothing else. It is meant for the
  product that sits in front of qdb, not for end users.
- **An account key** is made with the account and returned once, when
  the account is made. The server keeps only its SHA-256 hash, so a lost
  key cannot be read back; delete the account and make another. An
  account key reaches that account's filesystems and nothing else.

A request with no key, or a key the server does not know, is answered
`401 {"error":"no valid key: send Authorization: Bearer KEY"}`. A key of
the wrong kind for the request is also answered 401, saying which kind
the request needs.

## IDs

Accounts and filesystems have IDs of 32 lowercase hexadecimal
characters, made from the system's random source: for example
`a2593e17917a4e37af60f44c3f2e8def`. Sessions have IDs of the form
`s_` and 32 hexadecimal characters. IDs cannot be guessed, but an ID is
not a secret: what protects a filesystem is the key. A filesystem of
another account is answered exactly as one that does not exist.

## Answers and errors

Every answer that is not streamed has `Content-Type: application/json`.
An error is an object with one field:

```
{"error":"no such filesystem"}
```

| Status | When                                                                                       |
|--------|--------------------------------------------------------------------------------------------|
| 200    | Done                                                                                       |
| 201    | Made: an account, a filesystem, a session                                                  |
| 400    | The body is not a JSON object, has a field the request does not take, or a bad value      |
| 401    | No key, an unknown key, or a key of the wrong kind                                        |
| 404    | No such request, account, filesystem or session (or another account's)                     |
| 405    | A request the path does not take, such as `PUT /v1/filesystems`                           |
| 413    | The request body is larger than the server's limit (16 MB by default); the connection closes |
| 429    | The account has as many units running as it may, or the server's queue is full: try again |
| 500    | The server could not store a change (a disk error); nothing was changed                   |
| 503    | The server is shutting down, or has as many connections as it takes                       |
| 507    | The account has as many filesystems as its quota                                          |

A command line that fails is not an HTTP error. It is answered 200 with
its exit status, as a shell would give it (see Running a command line).

## Health

```
GET /v1/health
```

No key needed.

```
200 {"ok":true,"stopping":false}
```

`stopping` is true once the server has been told to stop.

## Accounts (admin key)

### Make an account

```
POST /v1/accounts
{"limits":{"units_at_once":4}}
```

The body is optional. It may set any of the account's settings; the rest
take the server's defaults.

```
201 {"account":"3e7f0d66f826adc6cd78f4de9ee283d8",
     "key":"qdbk_43efdd9ac9f5994509da701e828a1a18e848b9a29c0901a2a794b5d4480039e5",
     "settings":{"limits":{"commands":1000000,"memory_bytes":1073741824,"scratch_bytes":1073741824,"time_ms":30000,"units_at_once":4},
                 "quotas":{"disk_bytes":21474836480,"filesystems":1000,"nodes":1000000,"text_bytes":10737418240}}}
```

The key is in this answer only. Keep it.

### Settings

| Setting                 | Default (server flag)            | What it limits                                              |
|-------------------------|----------------------------------|-------------------------------------------------------------|
| `quotas.text_bytes`     | 10 GiB (`--text-bytes`)          | Text in one filesystem                                      |
| `quotas.nodes`          | 1,000,000 (`--nodes`)            | Files and folders in one filesystem                         |
| `quotas.disk_bytes`     | 20 GiB (`--disk-bytes`)          | Disk used by one filesystem                                 |
| `quotas.filesystems`    | 1,000 (`--filesystems`)          | Filesystems the account may have                            |
| `limits.time_ms`        | 30,000 (`--time-ms`)             | Time one unit may take, waiting included                    |
| `limits.commands`       | 1,000,000 (`--commands`)         | Commands one unit may run (loops count each pass)           |
| `limits.scratch_bytes`  | 1 GiB (`--scratch-bytes`)        | Scratch space one unit may write (sort's runs, for example) |
| `limits.memory_bytes`   | 1 GiB (`--memory-bytes`)         | Memory one unit may hold at once, all its threads together  |
| `limits.units_at_once`  | 16 (`--units-at-once`)           | Units of the account running or waiting at once             |

Each value is a positive whole number. An unknown setting, or a value
that is not a positive whole number, is refused with 400 and nothing is
changed.

### Change an account's settings

```
PATCH /v1/accounts/ACCOUNT
{"quotas":{"filesystems":5}}
```

Only the settings named change. Quotas are checked when a unit makes a
filesystem larger, so a filesystem above a lowered quota keeps what it
has, and a unit that would make it larger is stopped (`"stopped":"quota"`).

```
200 {"account":"3e7f...","settings":{...}}
```

### Read an account

```
GET /v1/accounts/ACCOUNT
```

```
200 {"account":"3e7f...","filesystems":1,"settings":{...},
     "usage":{"units":8,"stopped":1,"counters":{"content_written":46,"cpu_us":95521,...}}}
```

`usage` is described under Usage.

### Delete an account

```
DELETE /v1/accounts/ACCOUNT
```

Deletes the account, its key, its sessions and every filesystem it has.
Units running on them are stopped. This cannot be undone.

```
200 {"account":"3e7f...","deleted_filesystems":3}
```

## Filesystems (account key)

### Make a filesystem

```
POST /v1/filesystems
```

No body. A new filesystem is empty: one folder, `/`.

```
201 {"filesystem":"a2593e17917a4e37af60f44c3f2e8def"}
```

`507 {"error":"the account has 1000 filesystems, its quota"}` when the
account has its quota of filesystems.

### List the account's filesystems

```
GET /v1/filesystems
```

```
200 {"filesystems":["a2593e17917a4e37af60f44c3f2e8def"],"settings":{...},"usage":{...}}
```

### Read a filesystem

```
GET /v1/filesystems/FS
```

```
200 {"filesystem":"a2593e...","files":1,"folders":2,"text_bytes":6,"disk_bytes":270,
     "usage":{"units":8,"stopped":1,"counters":{...}}}
```

`folders` counts `/`. `disk_bytes` is what the filesystem's directory
takes on disk.

### Delete a filesystem

```
DELETE /v1/filesystems/FS
```

Units running on it are stopped (their answer says `"stopped":"deleted"`),
its sessions end, and its files are removed. Its usage moves to the
account's total. This cannot be undone.

```
200 {"filesystem":"a2593e...","deleted":true}
```

## Running a command line

```
POST /v1/filesystems/FS/run
{"command":"mkdir -p notes && echo hello > notes/a.txt && cat notes/a.txt"}
```

One request runs one command line: a **unit**. A unit is all or
nothing. If it finishes, everything it changed is kept, and under the
default `sync always` it is on disk before the answer is sent. If it is
stopped (by its time limit, a quota, its caller going away, the server
shutting down, or a fault), nothing it changed is kept.

Units on one filesystem behave as if they ran one at a time, in some
order consistent with when each was answered: a unit sees everything
that a unit answered before it was sent did. Units that only read run
together; a unit that may write runs alone on its filesystem. Units on
different filesystems never wait for one another, beyond sharing the
machine.

Body fields:

| Field          | Type    | Meaning                                                                                 |
|----------------|---------|-----------------------------------------------------------------------------------------|
| `command`      | string  | The command line. Required                                                              |
| `stdin`        | string  | Standard input of the line, as text                                                     |
| `stdin_base64` | string  | Standard input as base64, for bytes that are not UTF-8. Not with `stdin`                |
| `session`      | string  | A session's ID: the line starts in that session's directory with its variables         |
| `limits`       | object  | Tighter limits for this unit: any of `time_ms`, `commands`, `scratch_bytes`, `memory_bytes`. A value above the account's is ignored |
| `stream`       | boolean | Send the output as it is made (see Streamed output). Also `?stream=1` on the path       |

Any other field is refused with 400.

The answer:

```
200 {"exit_status":0,"stdout":"hello\n","stderr":"","stopped":null,"truncated":false,"kind":"writer",
     "cost":{"text_bytes":6,"text_pieces":1,"index_bytes":0,"index_lookups":0,"tree_nodes":8,
             "content_written":46,"records_written":97,"upkeep_written":0,"syncs":1,
             "scratch_bytes":0,"memory":11565,"cpu_us":67,"elapsed_us":67}}
```

| Field          | Meaning                                                                                                    |
|----------------|------------------------------------------------------------------------------------------------------------|
| `exit_status`  | The line's exit status, as bash gives it. 124 when stopped by its time limit, 137 by its memory or scratch limit, 143 when stopped otherwise, 1 when a quota is reached, 2 for a refused command or a fault |
| `stdout`, `stderr` | The output, as a string. Output that is not UTF-8 is sent as `stdout_base64` or `stderr_base64` instead |
| `truncated`    | True if either output was longer than the server's cap (1 MiB each by default) and was cut there. Standard output past the cap stops being produced, as with `| head`: the line is told its reader went away |
| `stopped`      | null if the unit ran to its end. Otherwise why it was stopped, and nothing it changed was kept: `deadline`, `caller_gone`, `shutdown`, `memory`, `scratch`, `deleted`, `quota`, `command_limit`, `fault`, `damage` |
| `kind`         | `reader` if the line could not change the filesystem and ran beside other readers; `writer` if it may have |
| `cost`         | What the unit read and wrote, as in qdb's `--stats`: see the README's "What a command costs". `syncs` is the syncs this unit made; a unit whose changes were made durable by another unit's sync shows 0. `memory` is the most the unit held at once |

Examples:

```
{"command":"wc -l","stdin":"a\nb\n"}
→ {"exit_status":0,"stdout":"2\n",...}

{"command":"while true; do :; done","limits":{"time_ms":100}}
→ {"exit_status":124,"stdout":"","stopped":"deadline",
   "stderr":"qdb: the unit was stopped at its time limit; nothing it changed was kept\n",...}

{"command":"cat nope.txt; vim x"}
→ {"exit_status":2,"stdout":"","stderr":"qdb: unsupported command: vim\n",...}

{"command":"printf \"\\xff\\xfe\""}
→ {"exit_status":0,"stdout_base64":"//4=","stderr":"",...}
```

A line that uses a command, flag or syntax qdb does not support is
refused whole, by name, before anything runs. The README lists what is
supported.

A unit's time limit counts from when it gets a worker, so time spent
waiting for a writer on the same filesystem counts; time in the server's
queue before that does not, but a unit whose caller goes away while it
waits there is dropped. A unit stopped by its time limit is answered
within a few milliseconds of it.

If the client closes the connection while its unit runs, the unit is
stopped (`caller_gone`) and nothing it changed is kept. A unit that has
already been answered cannot be taken back.

`429` answers, which mean try again later:

- `{"error":"the account has 16 units running, the most it may"}`
- `{"error":"too many units are waiting; try again"}`: every worker is
  busy and the server's queue of waiting units is full.

## Sessions

A unit starts in `/` with no variables of its own. A session keeps the
current directory and the shell variables from one unit to the next, as
one shell would. It belongs to one filesystem and one account and lives
in the server's memory only: it is not stored, and a restart ends it.

```
POST /v1/filesystems/FS/sessions
201 {"session":"s_23188069744f69474402fb931af8268a","idle_timeout_ms":900000}

POST /v1/filesystems/FS/run
{"command":"cd notes; X=5","session":"s_2318..."}

POST /v1/filesystems/FS/run
{"command":"pwd; echo $X","session":"s_2318..."}
→ {"stdout":"/notes\n5\n",...}

DELETE /v1/filesystems/FS/sessions/s_2318...
200 {"session":"s_2318...","ended":true}
```

A session not used for `idle_timeout_ms` (15 minutes by default) ends.
A unit stopped part way leaves its session as it was before the unit.
Units in one session run one at a time. A
session ID used on another filesystem, by another account, or after it
ended is answered `404 {"error":"no such session"}`.

## Streamed output

With `"stream":true` the answer is sent as it is made, which suits long
output and long-running lines:

```
POST /v1/filesystems/FS/run
{"command":"seq 3","stream":true}

200
Content-Type: application/x-ndjson
Transfer-Encoding: chunked

{"stdout":"1\n2\n3\n"}
{"done":true,"exit_status":0,"stopped":null,"truncated":false,"kind":"reader","cost":{...}}
```

The body is lines of JSON. Each line but the last is
`{"stdout":"..."}` (or `stdout_base64`), sent each time 64 KiB has
gathered and at the end; a line never splits a UTF-8 character.
Standard error is gathered (up to the output cap) and sent as one
`{"stderr":"..."}` line after the last standard output. The last line
has `"done":true` and the fields of an answer that is not streamed,
without the output.

Streamed standard output has no cap. A client that stops reading holds
the unit up; one that closes the connection stops it (`caller_gone`).

Because the status is sent before the line runs, a unit that cannot run
at all (its filesystem deleted meanwhile, the server shutting down) is
answered 200 with a last line saying why:
`{"done":true,"exit_status":2,"stopped":"shutdown",...}`.

## Usage

Accounts and filesystems report what their units used:

```
"usage":{"units":8,"stopped":1,
         "counters":{"text_bytes":12,"text_pieces":2,"index_bytes":0,"index_lookups":0,"tree_nodes":11,
                     "content_written":46,"records_written":97,"upkeep_written":0,"syncs":1,
                     "scratch_bytes":0,"memory_max":13936,"cpu_us":95521,"elapsed_us":100370}}
```

`units` is the units run, `stopped` how many were stopped, and the
counters are the sums of the units' costs, but for `memory_max`, the
most any one unit held. An account's usage includes its deleted
filesystems. Usage is written to disk every second, so a crash can lose
the last second of it; the units themselves are not lost.

## Limits of the server

| What                                   | Default    | Server flag             |
|----------------------------------------|------------|-------------------------|
| Request body                           | 16 MiB     | `--max-request-bytes`   |
| Output kept for an answer, each stream | 1 MiB      | `--output-cap-bytes`    |
| Connections open at once               | 1,024      | `--max-connections`     |
| Units running at once                  | 2 per core | `--workers`             |
| Units waiting for a worker             | 256        | `--queue`               |
| Session idle time                      | 15 minutes | `--session-idle-secs`   |
| A connection idle between requests     | 5 minutes  | (fixed)                 |

## An example with curl

```
export A=the-admin-key
K=$(curl -s -X POST -H "Authorization: Bearer $A" localhost:7878/v1/accounts | jq -r .key)
F=$(curl -s -X POST -H "Authorization: Bearer $K" localhost:7878/v1/filesystems | jq -r .filesystem)
curl -s -X POST -H "Authorization: Bearer $K" localhost:7878/v1/filesystems/$F/run \
     -d '{"command":"seq 1 5 > n.txt; grep -c . n.txt"}' | jq -r .stdout
```

Files go in and out through command lines: `cat > f` with `stdin` to
write one, `cat f` to read one. There is no upload or download request.
