"""Compare dev profiling cache budgets in the same CPU/native binaries."""

import argparse
import hashlib
import json
import math
import os
import runpy
import statistics
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path

helpers = runpy.run_path(str(Path(__file__).with_name("compare-text-interactions.py")))
read_rows, write_rows, run = (helpers[name] for name in ("read_rows", "write_rows", "run"))


def summarize(path, collected):
    summaries = []
    for budget, rows in collected.items():
        groups = defaultdict(list)
        for row in rows:
            groups[row.get("case", "native"), row["phase"]].append(row)
        for (case, phase), group in groups.items():
            summary = dict(budget_mib=budget, case=case, phase=phase, samples=len(group))
            for field in group[0]:
                if field in {"case", "phase", "step", "sample"}:
                    continue
                values = sorted(float(row[field]) for row in group)
                summary[field + "_median"] = statistics.median(values)
                if field.endswith("_ms"):
                    summary[field + "_p95"] = values[math.ceil(0.95 * len(values)) - 1]
                if field == "cache_bytes":
                    summary[field + "_max"] = max(values)
            summaries.append(summary)
    write_rows(path, summaries)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cpu", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--native", type=Path)
    parser.add_argument("--budgets", type=int, nargs="+", default=[32, 48, 64])
    parser.add_argument("--blocks", type=int, default=3)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--native-runs", type=int, default=6)
    args = parser.parse_args()
    if min(args.blocks, args.samples, args.native_runs) < 1 or not all(1 <= n <= 1024 for n in args.budgets):
        parser.error("positive sample counts and budgets between 1 and 1024 MiB required")
    if len(set(args.budgets)) != len(args.budgets):
        parser.error("budgets must be distinct")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    order = []
    for mode, binary, blocks in [("cpu", args.cpu, args.blocks), ("native", args.native, args.native_runs)]:
        if binary is None:
            continue
        directory = output / mode
        directory.mkdir(exist_ok=True)
        collected = {budget: [] for budget in args.budgets}
        for block in range(blocks):
            # Rotate each budget through every position, then reverse for the next cycle.
            budgets = args.budgets[::-1] if (block // len(args.budgets)) % 2 else args.budgets
            offset = block % len(budgets)
            budgets = budgets[offset:] + budgets[:offset]
            reference = None
            for budget in budgets:
                print(f"{mode} {block + 1}/{blocks}: {budget} MiB", flush=True)
                order.append([mode, block, budget])
                path = directory / f"{budget}-{block}.csv"
                env = dict(os.environ, LURQ_TEXT_CACHE_MIB=str(budget))
                env.pop("LURQ_TEXT_METRICS", None)
                env.pop("LURQ_TEXT_INTERACTIONS", None)
                if mode == "cpu":
                    env.update(LURQ_TEXT_INTERACTIONS=str(path), LURQ_TEXT_METRICS_SAMPLES=str(args.samples))
                run(binary.resolve(), [path] if mode == "native" else [], path.with_suffix(".log"), env)
                rows = read_rows(path)
                if mode == "cpu":
                    counts = Counter((r["case"], int(r["sample"]), r["phase"]) for r in rows)
                    expected = {(case, sample, phase): count
                                for case in ("document", "document_dpi150") for sample in range(args.samples)
                                for phase, count in [("cold", 1), ("edit", 8), ("resize", 8), ("scroll", 8), ("warm", 24)]}
                    assert counts == expected, f"incomplete CPU sequence: {path}"
                    signature = [(r["case"], r["sample"], r["phase"], r["glyph_count"]) for r in rows]
                else:
                    assert Counter(r["phase"] for r in rows) == Counter(cold=1, edit=8, resize=8, scroll=8)
                    signature = [(r["phase"], r["step"], r["glyph_count"], r["scale_factor"]) for r in rows]
                if reference is not None:
                    assert signature == reference, f"glyph counts/scale changed: {path}"
                reference = signature
                for row in rows:
                    assert int(row["cache_budget"]) == budget * 1024 * 1024, "cache override was not applied"
                    assert int(row["cache_bytes"]) <= int(row["cache_budget"]), "retained cache exceeded budget"
                    if row["phase"] in {"edit", "resize"}:
                        scaled = row.get("case") == "document_dpi150" if mode == "cpu" else float(row["scale_factor"]) != 1.0
                        expected = (2 if scaled else 1) if row["phase"] == "edit" else 0
                        assert int(row["shaped_paragraphs"]) == expected, f"paragraph reuse regressed: {path}"
                    row["sample"] = str(int(row.get("sample", 0)) + block * (args.samples if mode == "cpu" else 1))
                collected[budget].extend(rows)
        for budget, rows in collected.items():
            write_rows(directory / f"{budget}.csv", rows)
        summarize(directory / "summary.csv", collected)
    metadata = dict(timestamp_utc=datetime.now(timezone.utc).isoformat(), order=order,
                    cpu_samples_per_block=args.samples,
                    binary_sha256={name: hashlib.sha256(path.read_bytes()).hexdigest()
                                   for name, path in [("cpu", args.cpu), ("native", args.native)] if path})
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    print(f"Saved {output}; glyph counts, budgets, and shaping counters verified.")


if __name__ == "__main__":
    main()
