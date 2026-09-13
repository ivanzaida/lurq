"""Alternate saved text benchmark binaries and optionally their native DX12 probes."""

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


def read_rows(path):
    with path.open(newline="", encoding="utf-8-sig") as source:
        return list(csv.DictReader(source))


def write_rows(path, rows):
    with path.open("w", newline="", encoding="utf-8") as dest:
        writer = csv.DictWriter(dest, fieldnames=list(dict.fromkeys(key for row in rows for key in row)))
        writer.writeheader()
        writer.writerows(rows)


def run(binary, arguments, log, env=None, quiet=False):
    if quiet:
        from text_metrics_quiet import run_quiet
        return run_quiet(binary, arguments, log, env)
    with log.open("w", encoding="utf-8") as output:
        subprocess.run([str(binary), *map(str, arguments)], env=env, stdout=output, stderr=output, check=True, timeout=180)


def verify_incremental(rows, native=False):
    for row in rows:
        if row["phase"] not in {"edit", "resize"}:
            continue
        scaled = float(row["scale_factor"]) != 1.0 if native else row["case"].endswith("_dpi150")
        expected = (2 if scaled else 1) if row["phase"] == "edit" else 0
        assert int(row["shaped_paragraphs"]) == expected, "paragraph shaping reuse regressed"
        # Newer probes also report lookup construction and shape-allocation scans.
        for field in ("reindexed_paragraphs", "accounted_paragraphs"):
            if field in row:
                assert int(row[field]) == expected, f"unexpected {field}: {row}"


def verify_incremental_carets(rows, stable_reflow=False):
    paragraphs = {}
    for row in rows:
        key = row.get("case", "native"), row.get("sample", "0")
        built = int(row["caret_built_paragraphs"])
        reused = int(row["caret_reused_paragraphs"])
        if row["phase"] == "cold":
            assert built > 1 and reused == 0
            paragraphs[key] = built
        elif row["phase"] == "edit":
            assert built == 1 and reused == paragraphs[key] - 1, "caret paragraph reuse regressed"
            assert int(row["caret_built_positions"]) < int(row["caret_positions"]), "entire caret vector was extracted"
        elif row["phase"] == "resize":
            assert built + reused == paragraphs[key], "resize lost paragraph caret geometry"
            if stable_reflow:
                assert reused > 0, "stable wrapped paragraphs were not reused"
                scaled = float(row["scale_factor"]) != 1.0 if "scale_factor" in row else row["case"].endswith("_dpi150")
                layouts = int(row["laid_out_paragraphs"])
                reuses = int(row["reflow_reuses"])
                assert layouts + reuses == paragraphs[key] * (2 if scaled else 1), "resize skipped required layout work"
            else:
                assert reused == 0, "width change must invalidate paragraph caret geometry"
        else:
            assert built == reused == 0


def verify_selectable(rows, native=False):
    for row in rows:
        if native:
            assert row["selectable"] == "1", "native selectable mode was not enabled"
        if row["phase"] in {"cold", "edit", "resize"}:
            assert int(row["caret_requests"]) > 0 and int(row["caret_positions"]) > 0, "caret work was not exercised"
        assert int(row["caret_requests"]) == int(row["caret_hits"]) + int(row["caret_misses"])


def interaction_signature(rows, native=False, carets=False):
    fields = ["phase", "step", "glyph_count", "scale_factor"] if native else ["case", "sample", "phase", "glyph_count"]
    if carets:
        fields.append("caret_positions")
    return [tuple(row[field] for field in fields) for row in rows]


def summarize_native(native):
    summaries = []
    for name in ("before", "after"):
        groups = defaultdict(list)
        for row in read_rows(native / f"{name}.csv"):
            groups[row["phase"]].append(row)
        for phase, rows in groups.items():
            summary = dict(variant=name, phase=phase, samples=len(rows))
            for field in rows[0]:
                if field in {"phase", "step", "sample"}:
                    continue
                values = sorted(float(row[field]) for row in rows)
                summary[field + "_median"] = statistics.median(values)
                if field in {"action_to_paint_ms", "pass_ms"}:
                    summary[field + "_p95"] = values[math.ceil(0.95 * len(values)) - 1]
            summaries.append(summary)
    write_rows(native / "summary.csv", summaries)
    return summaries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--quiet-compilers", action="store_true", help="retry samples that overlap Windows compiler/linker activity")
    parser.add_argument("--blocks", type=int, default=3)
    parser.add_argument("--samples", type=int, default=5, help="fresh sequences per CPU block and display scale")
    parser.add_argument("--native-before", type=Path)
    parser.add_argument("--native-after", type=Path)
    parser.add_argument("--native-runs", type=int, default=5)
    parser.add_argument("--expect-incremental", action="store_true", help="verify after-build edit/resize shaping counters")
    parser.add_argument("--expect-incremental-carets", action="store_true", help="verify after-build paragraph caret extraction/reuse counts")
    parser.add_argument("--expect-stable-reflow", action="store_true", help="also verify layout/caret reuse across width changes")
    parser.add_argument("--selectable", action="store_true", help="measure selectable document edits/reflow; not keyboard typing")
    args = parser.parse_args()
    if min(args.blocks, args.samples, args.native_runs) < 1:
        parser.error("sample counts must be positive")
    if bool(args.native_before) != bool(args.native_after):
        parser.error("provide both native binaries")
    if args.expect_stable_reflow and not (args.selectable and args.expect_incremental_carets):
        parser.error("--expect-stable-reflow requires --selectable --expect-incremental-carets")
    output = args.output.resolve()
    blocks = output / "blocks"
    blocks.mkdir(parents=True, exist_ok=True)
    binaries = {"before": args.before.resolve(), "after": args.after.resolve()}
    collected = {name: [] for name in binaries}
    order = []
    run_env = dict(os.environ)
    run_env.pop("LURQ_TEXT_SELECTION", None)
    run_env.pop("LURQ_TEXT_SELECTABLE", None)
    if args.selectable:
        run_env["LURQ_TEXT_SELECTABLE"] = "1"
    for block in range(args.blocks):
        for name in (["before", "after"] if block % 2 == 0 else ["after", "before"]):
            print(f"CPU block {block + 1}/{args.blocks}: {name}", flush=True)
            order.append(["cpu", block, name])
            path = blocks / f"{name}-{block}.csv"
            env = dict(run_env, LURQ_TEXT_INTERACTIONS=str(path), LURQ_TEXT_METRICS_SAMPLES=str(args.samples))
            env.pop("LURQ_TEXT_METRICS", None)
            run(binaries[name], [], path.with_suffix(".log"), env, quiet=args.quiet_compilers)
            rows = read_rows(path)
            counts = Counter((row["case"], row["sample"], row["phase"]) for row in rows)
            base_case = "document_selectable" if args.selectable else "document"
            for case in (base_case, base_case + "_dpi150"):
                for sample in range(args.samples):
                    for phase, count in [("cold", 1), ("edit", 8), ("resize", 8), ("scroll", 8), ("warm", 24)]:
                        assert counts[case, str(sample), phase] == count, f"incomplete sequence: {path}/{case}/{phase}"
            if name == "after" and args.expect_incremental:
                verify_incremental(rows)
            if args.selectable:
                verify_selectable(rows)
            if name == "after" and args.expect_incremental_carets:
                verify_incremental_carets(rows, args.expect_stable_reflow)
            for row in rows:
                row["sample"] = str(int(row["sample"]) + block * args.samples)
            collected[name].extend(rows)
    for name, rows in collected.items():
        write_rows(output / f"{name}.csv", rows)
    # Older saved ordinary-text probes have no caret columns. Compare them when
    # both versions expose the metric; selectable mode requires the columns above.
    carets = all("caret_positions" in rows[0] for rows in collected.values())
    signatures = {name: interaction_signature(rows, carets=carets) for name, rows in collected.items()}
    assert signatures["before"] == signatures["after"], "CPU glyph/caret counts changed"
    native_binaries = {}
    if args.native_before:
        native = output / "native"
        native.mkdir(exist_ok=True)
        native_binaries = {"before": args.native_before.resolve(), "after": args.native_after.resolve()}
        native_rows = {name: [] for name in native_binaries}
        for sample in range(args.native_runs):
            pair = {}
            for name in (["before", "after"] if sample % 2 == 0 else ["after", "before"]):
                print(f"Native window {sample + 1}/{args.native_runs}: {name}", flush=True)
                order.append(["native", sample, name])
                path = native / f"{name}-{sample}.csv"
                run(native_binaries[name], [path], path.with_suffix(".log"), run_env, quiet=args.quiet_compilers)
                rows = read_rows(path)
                assert Counter(row["phase"] for row in rows) == Counter(cold=1, edit=8, resize=8, scroll=8), "incomplete native sequence"
                if name == "after" and args.expect_incremental:
                    verify_incremental(rows, native=True)
                if args.selectable:
                    verify_selectable(rows, native=True)
                if name == "after" and args.expect_incremental_carets:
                    verify_incremental_carets(rows, args.expect_stable_reflow)
                pair[name] = rows
                for row in rows:
                    row["sample"] = str(sample)
                native_rows[name].extend(rows)
            carets = all("caret_positions" in rows[0] for rows in pair.values())
            assert interaction_signature(pair["before"], native=True, carets=carets) == interaction_signature(pair["after"], native=True, carets=carets), "native glyph/caret counts or display scale changed"
        for name, rows in native_rows.items():
            write_rows(native / f"{name}.csv", rows)
            groups = defaultdict(list)
            for row in rows:
                groups[row["phase"]].append(float(row["action_to_paint_ms"]))
            print(name, "native action-to-paint medians:", {phase: round(statistics.median(values), 3) for phase, values in groups.items()})
        summarize_native(native)
    metadata = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "cpu_blocks": args.blocks, "cpu_samples_per_block": args.samples,
        "selectable": args.selectable,
        "quiet_compilers": args.quiet_compilers,
        "incremental_carets_validated": args.expect_incremental_carets,
        "stable_reflow_validated": args.expect_stable_reflow,
        "native_runs": args.native_runs if native_binaries else 0,
        "order": order,
        "cpu_binary_sha256": {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in binaries.items()},
        "native_binary_sha256": {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in native_binaries.items()},
    }
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    print(f"Saved {output}; ordered glyph counts agree.")


if __name__ == "__main__":
    main()
