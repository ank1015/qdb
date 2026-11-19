#!/usr/bin/env python3
"""Writes the measured half of bench/results/TARGETS-5.md from the stored
runs. Run from the repository root; the result goes to standard output."""
import csv
import statistics

R = "bench/results"


def run(dataset, label):
    # A command that is in the list twice keeps its last run.
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


COUNTS = [("text_bytes", "Text read, bytes"), ("tree_nodes", "Tree entries"), ("content_written", "Content written"), ("records_written", "Records written"), ("syncs", "Syncs")]


def table(title, rows, data, real="Real"):
    print(f"\n### {title}\n")
    print(f"| Command | {real}, ms | Engine, ms | Times faster | " + " | ".join(n for _, n in COUNTS) + " | Same output |")
    print("|---|---:|---:|---:|" + "---:|" * len(COUNTS) + "---|")
    for c in rows:
        r = data[c]
        g, e = float(r["gnu_ms"]), float(r["engine_ms"])
        print(f"| {code(c)} | {ms(g)} | {ms(e)} | {g / max(e, 1e-6):,.1f} | " + " | ".join(f"{int(r[k]):,}" for k, _ in COUNTS) + f" | {r['agree']} |")


disk, meta = run("linux", "s5-disk")
mem, mem_meta = run("linux", "s5-mem")
cap, cap_meta = run("linux", "s5-disk-cap512")
old, _ = run("linux", "s4-disk")

print("## Measured, from the stored runs\n")
print("Written by `bench/targets5.py`. The Linux tree, on disk, settled, sync always, one thread, the file cache warm, run `s5-disk`, unless a table says otherwise. Real tools include starting bash and the tool; the engine is timed in process. The count columns are the counters of part 1 for the first run of the command; they repeat exactly.")

new = [c for c in disk if c not in old]
loops = [c for c in new if c.split()[0] in ("for", "while", "until") or c[:2] in ("i=", "n=", "H=", "X=") or c.startswith("find ") and "while read" in c]
awks = [c for c in new if "awk" in c and c not in loops]
diffs = [c for c in new if c.startswith("diff") or "one edit" in c]
table("Loops, variables and `if`", loops, disk, real="bash")
table("awk, against GNU awk 5.2.1", awks, disk, real="gawk")
table("diff, against GNU diff 3.10", diffs, disk, real="GNU")

slow = [c for c in disk if disk[c]["gnu_ms"] and float(disk[c]["engine_ms"]) > float(disk[c]["gnu_ms"])]
print(f"\nDistinct commands of the whole list ({len(disk)}) slower than the real tool: {len(slow)}" + ("." if not slow else ": " + "; ".join(f"{code(c)} {ms(disk[c]['engine_ms'])} ms against {ms(disk[c]['gnu_ms'])} ms" for c in slow) + "."))
bad = [c for c in disk if disk[c]["agree"] not in ("yes", "order differs", "not compared")]
print(f"\nCommands whose output differs from the real tool's: {len(bad)}. The final trees agree: {meta['trees_agree']}. In memory (`s5-mem`): all agree {mem_meta['all_agree']}. Under the 512 MB limit (`s5-disk-cap512`): all agree {cap_meta['all_agree']}, final trees agree {cap_meta['trees_agree']}.")

# Stage 4 commands, stage 4 run against this one.
both = [c for c in old if c in disk]
r = sorted(((float(disk[c]["engine_ms"]) / max(float(old[c]["engine_ms"]), 1e-6), c) for c in both), reverse=True)
print(f"\n### Stage 4 commands, stage 4 run against stage 5 run\n")
print(f"{len(both)} commands are in both runs. Median of stage 5 time over stage 4 time: {statistics.median(x for x, _ in r):.2f}. The runs were made on different days on a shared machine. Slower by more than a quarter and by more than 0.05 ms:\n")
print("| Command | Stage 4, ms | Stage 5, ms | Ratio |\n|---|---:|---:|---:|")
for x, c in r:
    a, b = float(old[c]["engine_ms"]), float(disk[c]["engine_ms"])
    if x > 1.25 and b - a > 0.05:
        print(f"| {code(c)} | {ms(a)} | {ms(b)} | {x:.2f} |")

# The cost of counting.
rows = list(csv.DictReader(open(f"{R}/linux/s5-disk-counting.csv", newline="")))
ratios = [float(r["ratio"]) for r in rows]
long = [r for r in rows if float(r["off_ms"]) >= 1]
short = [r for r in rows if float(r["off_ms"]) < 0.01]
on, off = sum(float(r["on_ms"]) for r in rows), sum(float(r["off_ms"]) for r in rows)
print("\n### The cost of counting\n")
print(f"`qdb-bench --paired-counting`: each of the {len(rows)} read-only commands timed on the engine alone with the counting on and off by turns in one process (the paired method of stage 2), on the database on disk. Median of time with counting over time without: {statistics.median(ratios):.3f}. All the commands together: {on:,.0f} ms with, {off:,.0f} ms without ({on / off:.3f}).\n")
print(f"- The {len(long)} commands that take 1 ms or more: median {statistics.median(float(r['ratio']) for r in long):.3f}, the largest {max(float(r['ratio']) for r in long):.3f}.")
print(f"- The {len(short)} commands under 0.01 ms: median {statistics.median(float(r['ratio']) for r in short):.2f}; the difference is {statistics.median((float(r['on_ms']) - float(r['off_ms'])) * 1000 for r in short):.2f} microseconds a unit, which is two readings of each clock and a mark of the memory count.")

# The big file.
print("\n### One 1 GB file, inside 512 MB\n")
for sync in ("always", "batch"):
    big = {r["command"]: r for r in csv.DictReader(open(f"{R}/bigfile/s5-disk-{sync}-cap512.csv", newline=""))}
    if sync == "batch":
        slow = [c for c in big if float(big[c]["engine_ms"]) > float(big[c]["gnu_ms"])]
        print(f"\nUnder batch (`s5-disk-batch-cap512`) every result agrees: {all(r['agree'] == 'true' for r in big.values())}; slower than the real tool there: " + ("; ".join(f"{code(c)} {ms(big[c]['engine_ms'])} against {ms(big[c]['gnu_ms'])}" for c in slow) or "none") + ".")
        continue
    print("Sync always, run `s5-disk-always-cap512`. The commands new in stage 5, and the three of part 1:\n")
    print("| Command | Real, ms | Engine, ms | Times faster | Engine's most heap, MB | Bytes written | Syncs | Same output |\n|---|---:|---:|---:|---:|---:|---:|---|")
    for c, r in big.items():
        if not ("awk" in c or "diff" in c or "xargs" in c or "rg " in c or "grep -o" in c or "100000s" in c or " - " in c):
            continue
        g, e = float(r["gnu_ms"]), float(r["engine_ms"])
        print(f"| {code(c)} | {ms(g)} | {ms(e)} | {g / max(e, 1e-6):,.1f} | {float(r['engine_peak_bytes']) / 1e6:.1f} | {int(r['disk_bytes_written']):,} | {r['syncs']} | {'yes' if r['agree'] == 'true' else 'NO'} |")
    slow = [c for c in big if float(big[c]["engine_ms"]) > float(big[c]["gnu_ms"])]
    print(f"\nOf all {len(big)} commands of that run, slower than the real tool: " + ("; ".join(f"{code(c)} {ms(big[c]['engine_ms'])} against {ms(big[c]['gnu_ms'])}" for c in slow) or "none") + ".")

# The other datasets.
print("\n### The other datasets\n")
print("| Dataset | Distinct commands | Output differs | Slower than the real tool |\n|---|---:|---|---|")
for d in ["redis", "postgres"]:
    rows, _ = run(d, "s5-disk")
    bad = [c for c in rows if rows[c]["agree"] not in ("yes", "order differs", "not compared")]
    slow = [c for c in rows if rows[c]["gnu_ms"] and float(rows[c]["engine_ms"]) > float(rows[c]["gnu_ms"])]
    print(f"| {d} | {len(rows)} | {'; '.join(code(c) for c in bad) or 'none'} | {'; '.join(code(c) + ' ' + ms(rows[c]['engine_ms']) + ' against ' + ms(rows[c]['gnu_ms']) for c in slow) or 'none'} |")
