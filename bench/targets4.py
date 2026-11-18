#!/usr/bin/env python3
"""Writes the measured half of bench/results/TARGETS-4.md from the stored
runs: every figure of the stage 4 targets that comes from a benchmark
file. Run from the repository root; the result goes to standard output."""
import csv
import statistics

R = "bench/results"


def run(dataset, label):
    rows = {r["command"]: r for r in csv.DictReader(open(f"{R}/{dataset}/{label}.csv", newline=""))}
    meta = dict(l.rstrip("\n").split("=", 1) for l in open(f"{R}/{dataset}/{label}.meta") if "=" in l)
    return rows, meta


def ms(x):
    x = float(x)
    if x >= 100:
        return f"{x:,.0f}"
    if x >= 10:
        return f"{x:.1f}"
    if x >= 1:
        return f"{x:.2f}"
    return f"{x:.3f}"


def code(c):
    return "`" + c.replace("|", "\\|") + "`"


def table(title, rows, data, real="GNU"):
    print(f"\n### {title}\n")
    print(f"| Command | {real}, ms | Engine, ms | Times faster | Same output |\n|---|---:|---:|---:|---|")
    worst = None
    for c in rows:
        r = data[c]
        g, e = float(r["gnu_ms"]), float(r["engine_ms"])
        ratio = g / max(e, 1e-6)
        worst = ratio if worst is None else min(worst, ratio)
        print(f"| {code(c)} | {ms(g)} | {ms(e)} | {ratio:,.1f} | {r['agree']} |")
    return worst


disk, meta = run("linux", "s4-disk")
mem, _ = run("linux", "s4-mem")
old, old_meta = run("linux", "s3-disk")
stage2, _ = run("linux", "part4")
small, small_meta = run("linux", "s4-disk-small")
cap, cap_meta = run("linux", "s4-disk-cap512")

print("## Measured, from the stored runs\n")
print("Written by `bench/targets4.py`. The Linux tree, on disk, settled, sync always, one thread, the file cache warm, run `s4-disk`, unless a table says otherwise. Real tools include starting bash and the tool; the engine is timed in process.")

# Storage facts.
print("\n### Storage figures of stage 3, again\n")
print("| Figure | Stage 3 | Stage 4 |\n|---|---:|---:|")
for key, name, f in [
    ("open_ms", "Open after a clean close, ms", ms),
    ("load_ms", "Import with `--load` into `--db`, ms", ms),
    ("heap_after_load", "Heap after open, MB", lambda v: f"{float(v) / 1e6:.1f}"),
]:
    print(f"| {name} | {f(old_meta[key])} | {f(meta[key])} |")
ratio = lambda m: float(m["disk_settled_total"]) / float(m["disk_settled_raw_text"])
print(f"| Disk settled, against the raw text | {ratio(old_meta):.3f} | {ratio(meta):.3f} |")
print(f"| Tree file, bytes (it now holds a time for each file and folder) | {int(old_meta['disk_settled_tree']):,} | {int(meta['disk_settled_tree']):,} |")
few = [c for c in disk if c.startswith("grep -rn ") and "mispredicted" in c and c.endswith(" .") and "-rnw" not in c]
for c in few[:1]:
    print(f"| {code(c)} (the term in 84 files), ms | {ms(old[c]['engine_ms'])} | {ms(disk[c]['engine_ms'])} |")

# Read-only commands against stage 2.
common = [c for c in stage2 if c in disk and disk[c]["kind"] == "read"]
outside = [c for c in common if float(disk[c]["engine_ms"]) > 1.5 * float(stage2[c]["engine_ms"]) and float(disk[c]["engine_ms"]) - float(stage2[c]["engine_ms"]) > 0.05]
med = statistics.median(float(disk[c]["engine_ms"]) / max(float(stage2[c]["engine_ms"]), 1e-6) for c in common)
print(f"\nRead-only commands of stage 2 ({len(common)}), settled, against their stage 2 time in memory: median {med:.2f} of it; outside 1.5 times (and by more than 0.05 ms): {len(outside)}" + ("." if not outside else ": " + "; ".join(f"{code(c)} {ms(stage2[c]['engine_ms'])} to {ms(disk[c]['engine_ms'])} ms" for c in outside) + "."))

# The cost of keeping times.
print("\n### Stage 3 commands, stage 3 run against stage 4 run\n")
both = [c for c in old if c in disk]
r = sorted(((float(disk[c]["engine_ms"]) / max(float(old[c]["engine_ms"]), 1e-6), c) for c in both), reverse=True)
med = statistics.median(x for x, _ in r)
worse = [(x, c) for x, c in r if x > 1.05]
better = [(x, c) for x, c in r if x < 1 / 1.05]
print(f"{len(both)} commands are in both runs. Median of stage 4 time over stage 3 time: {med:.2f}. More than 5% slower: {len(worse)}. More than 5% faster: {len(better)}. The two runs were made on different days on a shared machine.\n")
print("Slower by more than 5% and by more than 0.05 ms:\n")
print("| Command | Kind | Stage 3, ms | Stage 4, ms | Ratio |\n|---|---|---:|---:|---:|")
for x, c in worse:
    a, b = float(old[c]["engine_ms"]), float(disk[c]["engine_ms"])
    if b - a > 0.05:
        print(f"| {code(c)} | {disk[c]['kind']} | {ms(a)} | {ms(b)} | {x:.2f} |")

# New commands.
rec = [c for c in disk if c.startswith("grep -r") and c not in old and "|" not in c]
w1 = table("Recursive grep with the new flags", rec, disk)
w2 = table("`ls -lt` and `find -newer`", [c for c in disk if c.startswith("ls -lt") or "-newer" in c or "-mtime" in c or "-mmin" in c], disk)
rgs = [c for c in disk if c.startswith("rg ")]
w3 = table("Every `rg` command, against real `rg -j 1`", rgs, disk, real="rg")
print(f"\nThe slowest of the {len(rgs)} against real rg: {w3:.1f} times faster.")
table("`du`", [c for c in disk if c.startswith("du ")], disk)
table("Pipelines that rank", [c for c in disk if "uniq -c" in c], disk)
table("Copies", [c for c in disk if c.startswith("cp ")], disk)
slow = [c for c in disk if disk[c]["gnu_ms"] and float(disk[c]["engine_ms"]) > float(disk[c]["gnu_ms"])]
print(f"\nCommands of the whole list ({len(disk)}) slower than the real tool: {len(slow)}" + ("." if not slow else ": " + "; ".join(f"{code(c)} {ms(disk[c]['engine_ms'])} ms against {ms(disk[c]['gnu_ms'])} ms" for c in slow) + "."))
bad = [c for c in disk if disk[c]["agree"] not in ("yes", "order differs", "not compared")]
print(f"\nCommands whose output differs from the real tool's: {len(bad)}. Not compared (timed only, see `bench/commands.txt`): {sum(1 for c in disk if disk[c]['agree'] == 'not compared')}. The final trees agree: {meta['trees_agree']}.")

# The big file.
print("\n### Text commands over a 1 GB file, inside 512 MB\n")
big = {r["command"]: r for r in csv.DictReader(open(f"{R}/bigfile/s4-disk-batch-cap512.csv", newline=""))}
print("| Command | GNU, ms | Engine, ms | Times faster | Engine's most heap, MB | Bytes written | Same output |\n|---|---:|---:|---:|---:|---:|---|")
for c, r in big.items():
    g, e = float(r["gnu_ms"]), float(r["engine_ms"])
    print(f"| {code(c)} | {ms(g)} | {ms(e)} | {g / max(e, 1e-6):,.1f} | {float(r['engine_peak_bytes']) / 1e6:.1f} | {int(r['disk_bytes_written']):,} | {'yes' if r['agree'] == 'true' else 'NO'} |")

# Less memory than the database.
print("\n### A database larger than memory\n")
reads = [c for c in disk if c in small and disk[c]["kind"] == "read"]
rr = sorted(((float(small[c]["engine_ms"]) / max(float(disk[c]["engine_ms"]), 1e-6), c) for c in reads), reverse=True)
print(f"Run `s4-disk-small`: the whole benchmark in a control group limited to 1 GB for everything, file cache included, against a database of {int(meta['disk_settled_total']) / 1e9:.2f} GB (the real tools ran in the same group, on their copy of the tree). One run of each command. All results agree with the real tools: {small_meta['all_agree']}; final trees agree: {small_meta['trees_agree']}.\n")
print(f"Of {len(reads)} read-only commands, the median took {statistics.median(x for x, _ in rr):.2f} of its warm time; {sum(1 for x, _ in rr if x > 2)} took more than twice and {sum(1 for x, _ in rr if x > 5)} more than five times. Every time is in `linux.md` beside its warm figure. The twelve that lost most, among those over 1 ms:\n")
print("| Command | Warm, ms | 1 GB of memory, ms | Ratio | Real tool in the same group, ms |\n|---|---:|---:|---:|---:|")
for x, c in [(x, c) for x, c in rr if float(small[c]["engine_ms"]) > 1][:12]:
    print(f"| {code(c)} | {ms(disk[c]['engine_ms'])} | {ms(small[c]['engine_ms'])} | {x:.1f} | {ms(small[c]['gnu_ms'])} |")
slow = [c for c in small if small[c]["gnu_ms"] and float(small[c]["engine_ms"]) > float(small[c]["gnu_ms"])]
print(f"\nSlower than the real tool in that run: {len(slow)}" + ("." if not slow else ": " + "; ".join(f"{code(c)} {ms(small[c]['engine_ms'])} ms against {ms(small[c]['gnu_ms'])} ms" for c in slow) + "."))
print(f"\nUnder the 512 MB limit on the engine's own memory (run `s4-disk-cap512`): completed, all results agree: {cap_meta['all_agree']}, final trees agree: {cap_meta['trees_agree']}.")

# The other datasets.
print("\n### The other datasets\n")
print("| Dataset | Commands | Output differs | Slower than the real tool |\n|---|---:|---:|---|")
for d in ["redis", "postgres"]:
    rows, _ = run(d, "s4-disk")
    bad = [c for c in rows if rows[c]["agree"] not in ("yes", "order differs", "not compared")]
    slow = [c for c in rows if rows[c]["gnu_ms"] and float(rows[c]["engine_ms"]) > float(rows[c]["gnu_ms"])]
    print(f"| {d} | {len(rows)} | {len(bad)} | {'; '.join(code(c) + ' ' + ms(rows[c]['engine_ms']) + ' against ' + ms(rows[c]['gnu_ms']) for c in slow) or 'none'} |")
