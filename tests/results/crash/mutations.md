# Does the crash test catch a fault? Faults put in on purpose

The crash test (`qdb-harness --crash N`) is only worth something if it
fails when the engine is wrong. Each row is one fault put into the engine
by hand, the crash test run on it (seeds 11 and 12, both sync settings,
3,000 to 6,000 power cuts each), and the result. Every fault was taken out
again afterwards.

| | Fault put in | Caught | How it showed |
|---|---|---|---|
| M1 | no sync when a command line ends, under `--sync always` | yes (always) | a line that had returned was gone after the power cut |
| M2 | recovery trusts content records it did not check | yes (both) | recovered database fails its check: content does not match its checksum |
| M3 | the tree is saved without syncing the data file first | yes (batch) | recovery fails: the checkpoint names a position the data file does not have |
| M4 | checkpoint file not synced before its rename | yes (both) | recovery fails: checkpoint damaged |
| M5 | a finished data file not synced before the next is used | yes (both) | returned lines lost beyond the stated bound |
| M6 | MANIFEST not synced before its rename | yes (both) | recovery fails: MANIFEST damaged |
| M7 | index file saved before the data file is synced, and the check at open for an index that covers more than the data file removed | yes (batch) | the index lists pieces that are not there |
| M8 | index file not synced before its rename | yes (both) | recovery has to build an index after a clean close, or the stored index is wrong |
| M9 | batch syncs half as often as stated | yes (batch) | more returned lines lost than the stated bound |
| M11 | directory not synced after an index file is removed | not a fault | a removed index file that comes back is checked at open like any other |
| M12 | directory not synced after a new data file is made | yes (index off) | recovery fails: the data file the log goes on in is not there |
| M13 | a tree saved in steps is made current without first syncing the log it may hold changes from | yes (batch) | recovery fails: the saved tree has content that the data file does not |
| M14 | an index file written in steps is given its name without first syncing the log | yes (both; found as a real fault, below) | after recovery the index lacks pieces that are in use |

"always" or "batch" says under which sync setting the fault can show at
all: M3, M7 and M9 need content that has returned but is not yet synced,
which only batch has.

## What the mutations found wrong with the test itself

The first runs of M7 and M12 passed, which showed three gaps in the test,
since closed:

1. No power cut ever fell while the database was being closed or
   compacted. The run now closes and reopens, and compacts, at random
   points, with power cuts during those too.
2. With the index on, a missing data file was hidden: building the index
   again made the file's directory entry durable as a side effect. The
   official runs include two with the index off (seeds 4 and 5).
3. Compaction writes log records of its own, so the number of the last
   record is not the number of the user's command lines. The test now
   counts them apart.

## Faults the crash test found in the engine

| Fault | Found by | Fix |
|---|---|---|
| After a crash the count of log bytes since the last save of the tree started again from zero, so a database that kept crashing never saved the tree and recovery grew longer each time | the test's own statistics (no tree saves in went-on runs) | the bytes replayed at open are counted |
| At close and compact the index file of the data file still being written could be saved while content it covers was not yet synced (batch); after a power cut the index would name pieces that are not there | reasoning, then M7 | the data file is synced before its index is saved; and at open an index that covers more pieces than the data file holds is removed and built again |
| A data file reopened after a crash at a size past its reserved space refused further writes | crash runs that went on from a crash image | the reserve is taken from the file's real length |
| An index file written in steps over many command lines left out pieces that files had stopped using in the meantime. If the power went before the log of those command lines was synced, the pieces were in use again after recovery and the index did not have them: a search would have missed lines. Two ways in: the index file was given its name without a sync of the log, and (inside one command line) pieces were forgotten before the command line's record was written, while writing that record could finish an index file | the crash test, on the first run after upkeep in steps was written (seeds 11, 12 and 21) | the log is synced before an index file is given its name; a command line's record is written before its pieces are forgotten; and an index built in bulk in the middle of a command line includes the pieces that command line has taken out of files so far |
| A data file is synced and then the next one is made. If the power went in between, the first could name a next file that was not there while its own end was not all on disk; recovery made the next file and went on, without looking at that end the way it looks at the end of the last file. Content that a command line's record pointed to could be garbage | the crash test (seed 32), once upkeep made data files finish more often | a data file that names a next one that is missing or has no whole header is checked like a last file, and the log ends in it if its end is not whole |
| Recovery applied log that it could read but that had never been synced, and then wrote an index file built from it. A second power cut could take that log away again and leave the index file | reasoning, while looking for the first of these three | recovery syncs the data file the log ends in before it writes anything made from it |
| Damage in an index list found by a search stopped the database but left the damaged index file, so every later open met it again | tests/disk.rs | the damaged index file is removed; the next open builds it again and says so |
