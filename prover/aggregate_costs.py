#!/usr/bin/env python3
"""Aggregate prover/log/*.jsonl into costs.csv (stdout).

One output row per target, grouped under its phase. Failed-attempt costs are
included by design: the published ledger reflects what verification actually
cost, not what the success path cost.

Usage: python3 prover/aggregate_costs.py > costs.csv
"""

import csv
import glob
import json
import os
import sys
from collections import defaultdict

LOG_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "log")

PHASE_ORDER = ["infra", "S1", "S2", "T4e", "T4d", "T3", "T1", "T2"]


def main() -> None:
    rows = []
    for path in sorted(glob.glob(os.path.join(LOG_DIR, "*.jsonl"))):
        with open(path, encoding="utf-8") as fh:
            for lineno, line in enumerate(fh, 1):
                line = line.strip()
                if not line:
                    continue
                try:
                    rows.append(json.loads(line))
                except json.JSONDecodeError as exc:
                    print(f"{path}:{lineno}: bad JSON: {exc}", file=sys.stderr)
                    sys.exit(1)

    by_target = defaultdict(list)
    for row in rows:
        by_target[(row.get("phase", "?"), row["target"])].append(row)

    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(
        [
            "phase",
            "target",
            "models",
            "attempts",
            "green",
            "tokens_in",
            "tokens_out",
            "usd",
            "wallclock_h",
        ]
    )

    def phase_key(item):
        phase = item[0][0]
        return (
            PHASE_ORDER.index(phase) if phase in PHASE_ORDER else len(PHASE_ORDER),
            item[0][1],
        )

    totals = {"attempts": 0, "tokens_in": 0, "tokens_out": 0, "usd": 0.0, "wall": 0.0}
    for (phase, target), attempts in sorted(by_target.items(), key=phase_key):
        models = sorted({a.get("model", "?") for a in attempts})
        n = len(attempts)
        green = sum(1 for a in attempts if a.get("outcome") == "green")
        tin = sum(int(a.get("tokens_in", 0)) for a in attempts)
        tout = sum(int(a.get("tokens_out", 0)) for a in attempts)
        usd = sum(float(a.get("usd", 0.0)) for a in attempts)
        wall_h = sum(float(a.get("wall_s", 0.0)) for a in attempts) / 3600.0
        writer.writerow(
            [phase, target, "+".join(models), n, green, tin, tout, f"{usd:.2f}", f"{wall_h:.2f}"]
        )
        totals["attempts"] += n
        totals["tokens_in"] += tin
        totals["tokens_out"] += tout
        totals["usd"] += usd
        totals["wall"] += wall_h

    writer.writerow(
        [
            "TOTAL",
            "",
            "",
            totals["attempts"],
            "",
            totals["tokens_in"],
            totals["tokens_out"],
            f"{totals['usd']:.2f}",
            f"{totals['wall']:.2f}",
        ]
    )


if __name__ == "__main__":
    main()
