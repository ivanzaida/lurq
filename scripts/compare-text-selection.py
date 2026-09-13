"""Alternate saved dev benchmarks for caret navigation and selection geometry."""

import argparse
import csv
import hashlib
import json
import math
import os
import statistics
import subprocess
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path


def write_rows(path, rows):
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--quiet-compilers", action="store_true", help="retry samples that overlap Windows compiler/linker activity")
    parser.add_argument("--blocks", type=int, default=3)
    parser.add_argument("--samples", type=int, default=5)
    args = parser.parse_args()
    if min(args.blocks, args.samples) < 1:
        parser.error("sample counts must be positive")
    output = args.output.resolve()
    blocks = output / "blocks"
    blocks.mkdir(parents=True, exist_ok=True)
    binaries = {name: getattr(args, name).resolve() for name in ("before", "after")}
    collected = {name: [] for name in binaries}
    order = []
    for block in range(args.blocks):
        for name in (["before", "after"] if block % 2 == 0 else ["after", "before"]):
            print(f"Selection block {block + 1}/{args.blocks}: {name}", flush=True)
            order.append([block, name])
            path = blocks / f"{name}-{block}.csv"
            env = dict(os.environ)
            for key in ("LURQ_TEXT_INTERACTIONS", "LURQ_TEXT_METRICS", "LURQ_TEXT_SELECTABLE"):
                env.pop(key, None)
            env.update(LURQ_TEXT_SELECTION=str(path), LURQ_TEXT_METRICS_SAMPLES=str(args.samples))
            if args.quiet_compilers:
                from text_metrics_quiet import run_quiet
                run_quiet(binaries[name], [], path.with_suffix(".log"), env)
            else:
                with path.with_suffix(".log").open("w", encoding="utf-8") as log:
                    subprocess.run([str(binaries[name])], env=env, stdout=log, stderr=log, check=True, timeout=180)
            with path.open(newline="", encoding="utf-8") as stream:
                rows = list(csv.DictReader(stream))
            counts = Counter((r["case"], r["scale"], r["location"], r["sample"], r["phase"]) for r in rows)
            expected = Counter()
            for case in ("selectable", "input"):
                phases = {"press": 1, "drag": 24, "release": 1}
                if case == "input":
                    phases.update(arrow=24, shift_arrow=24)
                for scale in ("1", "1.5"):
                    for location in ("top", "middle", "bottom"):
                        for sample in range(args.samples):
                            for phase, count in phases.items():
                                expected[case, scale, location, str(sample), phase] = count
            assert counts == expected, f"incomplete sequence: {path}"
            assert all(int(r["glyphs"]) > 0 for r in rows)
            assert all(int(r["selection_rects"]) > 0 for r in rows if r["phase"] == "drag")
            assert any(int(r["selection_rects"]) > 0 for r in rows if r["phase"] == "shift_arrow")
            for row in rows:
                row["sample"] = str(int(row["sample"]) + block * args.samples)
            collected[name].extend(rows)
    signature_fields = ("case", "scale", "location", "sample", "phase", "step", "selection_rects", "signature", "glyphs", "caret_positions")
    signatures = {name: [tuple(row[field] for field in signature_fields) for row in rows] for name, rows in collected.items()}
    assert signatures["before"] == signatures["after"], "selection/caret geometry or glyph counts changed"
    summaries = []
    for name, rows in collected.items():
        write_rows(output / f"{name}.csv", rows)
        groups = defaultdict(list)
        for row in rows:
            groups[row["case"], row["scale"], row["location"], row["phase"]].append(row)
        for (case, scale, location, phase), group in groups.items():
            summary = dict(variant=name, case=case, scale=scale, location=location, phase=phase, samples=len(group))
            for field in ("event_ms", "pass_ms", "total_ms", "caret_ms", "caret_extract_ms"):
                values = sorted(float(row[field]) for row in group)
                summary[field + "_median"] = statistics.median(values)
                summary[field + "_p95"] = values[math.ceil(0.95 * len(values)) - 1]
            summaries.append(summary)
    write_rows(output / "summary.csv", summaries)
    metadata = dict(timestamp_utc=datetime.now(timezone.utc).isoformat(), blocks=args.blocks,
                    samples_per_block=args.samples, order=order, geometry_matches=True, quiet_compilers=args.quiet_compilers,
                    binary_sha256={name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in binaries.items()})
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    print(f"Saved {output}; all ordered geometry signatures and glyph/caret counts agree.")


if __name__ == "__main__":
    main()
