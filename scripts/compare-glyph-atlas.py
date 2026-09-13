"""Alternate saved native document probes and verify glyph-atlas upload reuse."""

import argparse
import hashlib
import importlib.util
import json
import os
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--runs", type=int, default=6)
    parser.add_argument("--quiet-compilers", action="store_true")
    parser.add_argument("--quiet-timeout", type=float, default=60, help="maximum seconds waiting/retrying for compiler quiet per probe")
    args = parser.parse_args()
    if args.runs < 1 or not 0 < args.quiet_timeout < float("inf"):
        parser.error("--runs and --quiet-timeout must be positive and finite")
    spec = importlib.util.spec_from_file_location("text_interactions", Path(__file__).with_name("compare-text-interactions.py"))
    helpers = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helpers)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    binaries = {name: getattr(args, name).resolve() for name in ("before", "after")}
    collected = {name: [] for name in binaries}
    env = dict(os.environ, LURQ_TEXT_SELECTABLE="1")
    for key in ("LURQ_TEXT_SELECTION", "LURQ_TEXT_INTERACTIONS", "LURQ_TEXT_METRICS", "LURQ_TEXT_CACHE_MIB"):
        env.pop(key, None)
    order = []
    for sample in range(args.runs):
        pair = {}
        for name in (["before", "after"] if sample % 2 == 0 else ["after", "before"]):
            print(f"Native window {sample + 1}/{args.runs}: {name}", flush=True)
            path = output / f"{name}-{sample}.csv"
            order.append([sample, name])
            if args.quiet_compilers:
                from text_metrics_quiet import run_quiet
                run_quiet(binaries[name], [path], path.with_suffix(".log"), env, settle_timeout=args.quiet_timeout)
            else:
                helpers.run(binaries[name], [path], path.with_suffix(".log"), env)
            rows = helpers.read_rows(path)
            assert Counter(r["phase"] for r in rows) == Counter(cold=1, edit=8, resize=8, scroll=8)
            helpers.verify_selectable(rows, native=True)
            helpers.verify_incremental(rows, native=True)
            helpers.verify_incremental_carets(rows, stable_reflow=True)
            for row in rows:
                uploads = int(int(row["atlas_bytes"]) > 0)
                assert int(row["atlas_arena_uploads"]) == (uploads if name == "after" else 0)
                assert int(row["atlas_dedicated_uploads"]) == (uploads if name == "before" else 0)
                row["sample"] = str(sample)
            pair[name] = rows
            collected[name].extend(rows)
        assert helpers.interaction_signature(pair["before"], native=True, carets=True) == helpers.interaction_signature(pair["after"], native=True, carets=True)
        assert [r["atlas_bytes"] for r in pair["before"]] == [r["atlas_bytes"] for r in pair["after"]], "atlas transfer size changed"
    for name, rows in collected.items():
        helpers.write_rows(output / f"{name}.csv", rows)
    helpers.summarize_native(output)
    metadata = dict(timestamp_utc=datetime.now(timezone.utc).isoformat(), runs=args.runs,
                    quiet_compilers=args.quiet_compilers, quiet_timeout_seconds=args.quiet_timeout, order=order,
                    binary_sha256={name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in binaries.items()},
                    geometry_counts_match=True, atlas_bytes_match=True, atlas_resource_reuse_verified=True)
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    print(f"Saved {output}; geometry, transfer sizes, and arena reuse verified.")


if __name__ == "__main__":
    main()
