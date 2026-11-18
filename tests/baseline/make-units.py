#!/usr/bin/env python3
"""Writes tests/baseline/units.txt: the benchmark command list with the
terms of the generated dataset `small` filled in. Run from the repository
root, then `qdb-harness --update-baseline` and say why in the commit."""
import sys

# With arguments DATASET FILE: the same list for another dataset, written
# to FILE (for qdb-replay).
dataset = sys.argv[1] if len(sys.argv) > 2 else "small"
target = sys.argv[2] if len(sys.argv) > 2 else "tests/baseline/units.txt"
terms = {}
for line in open(f"bench/terms/{dataset}.txt"):
    if line.startswith("#") or "=" not in line:
        continue
    key, value = line.split("=", 1)
    terms[key] = value.split("  # ")[0].strip()
out = [] if dataset != "small" else [
    "# The benchmark command list (bench/commands.txt) with the search terms of",
    "# the generated dataset `small` (bench/terms/small.txt) filled in. Made by",
    "# tests/baseline/make-units.py; make it again when the command list changes.",
    "# The harness runs these in order and compares what each one reads, writes",
    "# and syncs with counters.tsv.",
]
for line in open("bench/commands.txt"):
    line = line.rstrip("\n")
    if not line.strip() or line.startswith("#"):
        continue
    for prefix in ("mut: ", "nocmp: "):
        if line.startswith(prefix):
            line = line[len(prefix):]
    for key, value in terms.items():
        line = line.replace("{" + key + "}", value)
    out.append(line)
open(target, "w").write("\n".join(out) + "\n")
