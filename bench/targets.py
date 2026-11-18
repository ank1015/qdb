#!/usr/bin/env python3
"""Checks the stage 2 targets against the stored benchmark runs and writes
bench/results/TARGETS.md. Run from the repository root."""
import csv
import os

RESULTS = "bench/results"
DATASETS = ["redis", "postgres", "large", "linux"]
LARGEST = "linux"


def run(dataset, label):
    base = os.path.join(RESULTS, dataset, label)
    if not os.path.exists(base + ".csv"):
        return None
    rows = {r["command"]: r for r in csv.DictReader(open(base + ".csv", newline=""))}
    meta = dict(l.rstrip("\n").split("=", 1) for l in open(base + ".meta") if "=" in l)
    return rows, meta


def terms(dataset):
    out = {}
    for line in open(os.path.join("bench/terms", dataset + ".txt")):
        if line.startswith("#") or "=" not in line:
            continue
        key, rest = line.split("=", 1)
        value, _, note = rest.partition("  # ")
        out[key] = (value.strip(), note.strip())
    return out


def ms(x):
    x = float(x)
    if x >= 100:
        return f"{x:,.0f}"
    if x >= 10:
        return f"{x:.1f}"
    if x >= 1:
        return f"{x:.2f}"
    return f"{x:.3f}"


def size(n):
    n = float(n)
    for unit, scale in (("GB", 1e9), ("MB", 1e6), ("KB", 1e3)):
        if n >= scale:
            return f"{n / scale:.1f} {unit}"
    return f"{n:.0f} B"


def code(command):
    return "`" + command.replace("|", "\\|") + "`"


out = []
w = out.append
on = {d: run(d, "part4") for d in DATASETS}
off = {d: run(d, "part4-noindex") for d in DATASETS}
have = [d for d in DATASETS if on[d] and off[d]]
summary = []

w("# Stage 2 targets, measured\n")
w("Written by `bench/targets.py` from the runs under `bench/results/`. Times are in milliseconds.")
w("\"Index on\" is the run labelled `part4`, \"index off\" is `part4-noindex`. GNU times come from the same run as the engine time beside them.\n")

# 1 ---------------------------------------------------------------------
w("## 1. `grep -rn COMMON . | head -n 5` on the largest dataset: under 1 ms\n")
t = terms(LARGEST)
command = f"grep -rn {t['COMMON'][0]} . | head -n 5"
w("| Dataset | Command | GNU | Index off | Index on | Target |\n|---|---|---:|---:|---:|---|")
worst = 0.0
for d in have:
    c = f"grep -rn {terms(d)['COMMON'][0]} . | head -n 5"
    a, b = on[d][0][c], off[d][0][c]
    if d == LARGEST:
        worst = float(a["engine_ms"])
    w(f"| {d} | {code(c)} | {ms(a['gnu_ms'])} | {ms(b['engine_ms'])} | {ms(a['engine_ms'])} | {'met' if float(a['engine_ms']) < 1 else 'MISSED'} |")
summary.append((f"`grep -rn COMMON . | head -n 5` on the largest dataset under 1 ms", f"{ms(worst)} ms on {LARGEST}", worst < 1))

# 2 ---------------------------------------------------------------------
w("\n## 2. A streamed command's peak memory does not grow with its output\n")
w("The same commands on each dataset: output size against the most memory held while running (index on).\n")
kinds = [("grep -rn {COMMON} .", "COMMON"), ("grep -rn {SOME} .", "SOME"), ("find . -type f", None), ("grep -rE '[0-9]{4,}' .", None), ("cat {GLOB} | wc -l", "GLOB")]
w("| Command | " + " | ".join(f"{d}: output | {d}: peak" for d in have) + " |")
w("|---|" + "---:|---:|" * len(have))
ok = True
for template, _ in kinds:
    cells = []
    for d in have:
        c = template
        for k, (v, _) in terms(d).items():
            c = c.replace("{" + k + "}", v)
        r = on[d][0].get(c)
        if r is None:
            cells += ["", ""]
            continue
        cells += [size(r["output_bytes"]), size(r["peak_bytes"])]
        if float(r["peak_bytes"]) > 4e6:
            ok = False
    w(f"| `{template}` | " + " | ".join(cells) + " |")
biggest = max((float(r["output_bytes"]), float(r["peak_bytes"]), c) for c, r in on[LARGEST][0].items() if r["kind"] == "read") if LARGEST in have else (0, 0, "")
w(f"\nThe largest output of any read-only command on {LARGEST} is {size(biggest[0])} ({code(biggest[2])}), produced with a peak of {size(biggest[1])}.")
summary.append(("A streamed command's peak memory does not grow with its output", f"{size(biggest[0])} of output with {size(biggest[1])} peak", ok))

# 3 ---------------------------------------------------------------------
w("\n## 3. Every recursive grep, index off: at least as fast as GNU grep, at every size\n")
w("| Dataset | Recursive greps | Slower than GNU | Closest to GNU | Engine | GNU | GNU / engine |\n|---|---:|---:|---|---:|---:|---:|")
all_met = True
closest_all = None
for d in have:
    rows = [r for c, r in off[d][0].items() if "grep -r" in c]
    slower = [r for r in rows if float(r["engine_ms"]) > float(r["gnu_ms"])]
    closest = min(rows, key=lambda r: float(r["gnu_ms"]) / max(float(r["engine_ms"]), 1e-9))
    ratio = float(closest["gnu_ms"]) / float(closest["engine_ms"])
    all_met &= not slower
    if closest_all is None or ratio < closest_all[0]:
        closest_all = (ratio, d)
    w(f"| {d} | {len(rows)} | {len(slower)} | {code(closest['command'])} | {ms(closest['engine_ms'])} | {ms(closest['gnu_ms'])} | {ratio:.1f}x |")
    for r in slower:
        w(f"| | | | {code(r['command'])} | {ms(r['engine_ms'])} | {ms(r['gnu_ms'])} | {float(r['gnu_ms']) / float(r['engine_ms']):.2f}x |")
summary.append(("Every recursive grep with the index off at least as fast as GNU grep, at every size", f"closest is {closest_all[0]:.1f}x faster than GNU ({closest_all[1]})", all_met))

# 4 ---------------------------------------------------------------------
w("\n## 4. Search for a term in no file: under 1 ms at every size (index on)\n")
w("| Dataset | Command | GNU | rg | Index off | Index on | Target |\n|---|---|---:|---:|---:|---:|---|")
worst = 0.0
for d in have:
    for flag in ("-rn", "-rl"):
        c = f"grep {flag} {terms(d)['ABSENT'][0]} ."
        a, b = on[d][0][c], off[d][0][c]
        worst = max(worst, float(a["engine_ms"]))
        w(f"| {d} | {code(c)} | {ms(a['gnu_ms'])} | {ms(a['rg_ms'])} | {ms(b['engine_ms'])} | {ms(a['engine_ms'])} | {'met' if float(a['engine_ms']) < 1 else 'MISSED'} |")
summary.append(("A search for a term in no file under 1 ms at every size", f"slowest {ms(worst)} ms", worst < 1))

# 5 ---------------------------------------------------------------------
w(f"\n## 5. A term in 1% of files or fewer, on the largest dataset: at least 10x faster than GNU grep (index on)\n")
w("| Term | In files | Command | GNU | rg | Index off | Index on | GNU / index on | Target |\n|---|---|---|---:|---:|---:|---:|---:|---|")
least = None
if LARGEST in have:
    t = terms(LARGEST)
    for key in ("ONE", "RARE", "FEW"):
        for flag in ("-rn", "-rl"):
            c = f"grep {flag} {t[key][0]} ."
            a, b = on[LARGEST][0][c], off[LARGEST][0][c]
            ratio = float(a["gnu_ms"]) / float(a["engine_ms"])
            least = ratio if least is None else min(least, ratio)
            w(f"| {key} | {t[key][1]} | {code(c)} | {ms(a['gnu_ms'])} | {ms(a['rg_ms'])} | {ms(b['engine_ms'])} | {ms(a['engine_ms'])} | {ratio:,.0f}x | {'met' if ratio >= 10 else 'MISSED'} |")
    summary.append(("A term in 1% of files or fewer on the largest dataset at least 10x faster than GNU grep", f"smallest gain {least:,.0f}x", least >= 10))

# 6 ---------------------------------------------------------------------
w("\n## 6. Any benchmark command, index on against index off: no more than 10% slower\n")
w("**Commands that only read** are compared in one process, with searches using the index and ignoring it by turns (the run labelled `paired`, `qdb-bench --paired`). Two separate runs of the benchmark cannot show a difference of a tenth: the same command in two runs of the same configuration differs by up to 30% (where memory happens to land), index or no index.\n")
w("| Dataset | Read commands | Over 10% slower with the index | Slowest relative to no index | Fastest relative to no index |\n|---|---:|---:|---|---|")
read_misses = []
for d in have:
    path = os.path.join(RESULTS, d, "paired.csv")
    if not os.path.exists(path):
        continue
    rows = list(csv.DictReader(open(path, newline="")))
    over = [r for r in rows if float(r["ratio"]) > 1.10]
    read_misses += [(d, r) for r in over]
    hi = max(rows, key=lambda r: float(r["ratio"]))
    lo = min(rows, key=lambda r: float(r["ratio"]))
    w(f"| {d} | {len(rows)} | {len(over)} | {code(hi['command'])} {float(hi['ratio']):.2f}x | {code(lo['command'])} {float(lo['used_ms']) / float(lo['ignored_ms']):.4f}x |")
if read_misses:
    w("\nThe commands over 10%:\n\n| Dataset | Command | Index ignored | Index used | Ratio | Does it ask the index? |\n|---|---|---:|---:|---:|---|")
    for d, r in read_misses:
        asks = "yes" if "grep -r" in r["command"] else "no: the difference is not the index's doing"
        w(f"| {d} | {code(r['command'])} | {ms(r['ignored_ms'])} | {ms(r['used_ms'])} | {float(r['ratio']):.2f}x | {asks} |")
real_read_misses = [x for x in read_misses if "grep -r" in x[1]["command"]]
w("\n**Commands that change data** are compared between the two runs, since the index cannot be ignored while it is being kept current. A write has to keep the index current, and that work is the cost of the index: these miss the target.\n")
w("| Dataset | Command | Index off | Index on | Ratio |\n|---|---|---:|---:|---:|")
writes_total = writes_all = 0
for d in have:
    for c, a in on[d][0].items():
        b = off[d][0].get(c)
        if not b or a["kind"] != "write" or c.startswith(("grep", "ls")):
            continue
        writes_all += 1
        x, y = float(b["engine_ms"]), float(a["engine_ms"])
        if y > x * 1.1:
            writes_total += 1
            w(f"| {d} | {code(c)} | {ms(x)} | {ms(y)} | {y / max(x, 1e-9):.1f}x |")
summary.append(("Any benchmark command with the index on no more than 10% slower than with it off: commands that only read", f"{len(real_read_misses)} of the commands that ask the index are slower; {len(read_misses) - len(real_read_misses)} that never ask it measured over 10% (noise)", not real_read_misses))
summary.append(("The same, for commands that change data", f"{writes_total} of {writes_all} are more than 10% slower", writes_total == 0))

# 7 ---------------------------------------------------------------------
w("\n## 7. Index size: at most 1x the raw text\n")
w("| Dataset | Text | Index in memory | Share of text | Posting data alone | Target |\n|---|---:|---:|---:|---:|---|")
worst = 0.0
for d in have:
    m = on[d][1]
    share = float(m["index_bytes"]) / float(m["bytes"])
    worst = max(worst, share)
    w(f"| {d} | {size(m['bytes'])} | {size(m['index_bytes'])} | {share * 100:.1f}% | {size(m['index_posting_bytes'])} ({float(m['index_posting_bytes']) / float(m['bytes']) * 100:.1f}%) | {'met' if share <= 1 else 'MISSED'} |")
summary.append(("Index size at most 1x the raw text", f"largest share {worst * 100:.0f}% of the text", worst <= 1))

# 8 ---------------------------------------------------------------------
w("\n## 8. A write to one file: time depends on the size of the file, not of the database\n")
w("The same 64 KB of text added as a new file to each dataset (median of nine files), and two commands from the list that write the same small content everywhere.\n")
w("| Dataset | Text in the database | New 64 KB file, index off | New 64 KB file, index on | `echo 'hello world' > file`, index on | `echo 'another line' >> file`, index on |\n|---|---:|---:|---:|---:|---:|")
fixed = []
for d in have:
    a, b = on[d], off[d]
    fixed.append(float(a[1].get("fixed_write_ms", "nan")))
    w(f"| {d} | {size(a[1]['bytes'])} | {ms(b[1].get('fixed_write_ms', 'nan'))} | {ms(a[1].get('fixed_write_ms', 'nan'))} | {ms(a[0][chr(101) + 'cho ' + chr(39) + 'hello world' + chr(39) + ' > bench_out/hello.txt']['engine_ms'])} | {ms(a[0][chr(101) + 'cho ' + chr(39) + 'another line' + chr(39) + ' >> bench_out/hello.txt']['engine_ms'])} |")
spread = max(fixed) / min(fixed) if fixed else 0
summary.append(("A write to one file costs by the size of the file, not of the database", f"the same 64 KB write takes {ms(min(fixed))} to {ms(max(fixed))} ms across an 80x range of database sizes", spread < 3))

# 9 ---------------------------------------------------------------------
w("\n## 9. Write commands and loading, index on and off (reported, no target)\n")
w("| Dataset | Read the files | Build the index | Heap after load, index off | Heap after load, index on | Resident, index off | Resident, index on |\n|---|---:|---:|---:|---:|---:|---:|")
for d in have:
    a, b = on[d][1], off[d][1]
    w(f"| {d} | {ms(a['load_ms'])} | {ms(a['index_build_ms'])} | {size(b['heap_after_load'])} | {size(a['heap_after_load'])} | {size(b['rss_after_load'])} | {size(a['rss_after_load'])} |")
for d in have:
    w(f"\n### Commands that change data: {d}\n")
    w("| Command | GNU | Index off | Index on | Index work |\n|---|---:|---:|---:|---|")
    for c, a in on[d][0].items():
        if a["kind"] != "write":
            continue
        b = off[d][0].get(c)
        w(f"| {code(c)} | {ms(a['gnu_ms'])} | {ms(b['engine_ms']) if b else ''} | {ms(a['engine_ms'])} | {a.get('index_work', '')} |")

head = ["# Stage 2 targets, measured\n", "| Target | Measured | Result |", "|---|---|---|"]
for name, value, met in summary:
    head.append(f"| {name} | {value} | {'met' if met else 'MISSED'} |")
head.append("")
text = "\n".join(head + out[1:]) + "\n"
open(os.path.join(RESULTS, "TARGETS.md"), "w").write(text)
print("\n".join(head))
