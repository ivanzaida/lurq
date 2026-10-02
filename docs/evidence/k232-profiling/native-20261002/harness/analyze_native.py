"""Summarize immutable real captures without reading or waking the app."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "results/native2"


def read(name):
    return json.loads((ROOT / f"{name}.json").read_text(encoding="utf-8"))


def summary(name):
    report = read(name)
    passes = [sample for sample in report["samples"] if sample["data"]["kind"] == "pass"]
    rendered = [sample for sample in passes if sample["data"]["rendered"]]
    canvas = [sample["data"]["frame"]["render"]["canvas"] for sample in rendered
              if sample["data"]["frame"]]
    acquire = [sample["data"]["frame"]["render"]["cpu_timings_ms"]["acquire"] for sample in rendered
               if sample["data"]["frame"]]
    longest = sorted(rendered, key=lambda sample: sample["data"]["cpu_timings_ms"]["total"],
                     reverse=True)[:3]
    return {
        "id": report["id"], "started_ms": report["started_ms"], "ended_ms": report["ended_ms"],
        "status": report["status"], "sample_age_ms": report["sample_age_ms"],
        "completed_samples": report["completed_samples"], "returned_samples": report["returned_samples"],
        "dropped_samples": report["dropped_samples"],
        "boundary_excluded_samples": report["boundary_excluded_samples"],
        "truncated": report["truncated"], "in_flight": report["in_flight"],
        "pass_count": len(passes), "rendered_count": len(rendered),
        "canvas_counts_sums": {key: sum(row["counts"][key] for row in canvas) for key in
                               ("batches", "command_groups", "tiles", "vertices", "uploaded_asset_bytes")},
        "cpu_acquire_ms": {"total": sum(acquire), "max": max(acquire, default=0)},
        "longest_rendered_passes": [{"frame_id": sample["data"]["frame_id"],
            "started_ms": sample["started_ms"], "completed_ms": sample["completed_ms"],
            "pass_cpu_ms": sample["data"]["cpu_timings_ms"],
            "frame_cpu_ms": sample["data"]["frame"]["cpu_timings_ms"] if sample["data"]["frame"] else None,
            "render_cpu_ms": sample["data"]["frame"]["render"]["cpu_timings_ms"] if sample["data"]["frame"] else None,
            "canvas_cpu_ms": sample["data"]["frame"]["render"]["canvas"]["cpu_timings_ms"] if sample["data"]["frame"] else None
        } for sample in longest]}


def main():
    names = ("cold-open-end1", "overlap-end1", "overlap-end2", "overhead-1-capture",
             "overhead-2-capture", "stall-end1", "stall-end2")
    overhead = read("overhead-summary")
    result = {"completed_reports": {name: summary(name) for name in names},
              "overhead_segments": [{key: segment[key] for key in
                    ("segment", "collection_active", "cycles", "wall_ms", "process_cpu_ms")}
                    for segment in overhead["segments"]],
              "overhead_causal_attribution": "not established; variable workload/ongoing admission and external compiler contention",
              "stall_causal_attribution": "first observed layout_update began before session1; not proof of the10s report or zoom causation",
              "cleanup": read("cleanup"), "hash_preservation": read("reversible-preservation")}
    (ROOT / "diagnostic-summary.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps({"reports": {name: {key: row[key] for key in
        ("completed_samples", "returned_samples", "dropped_samples", "boundary_excluded_samples",
         "truncated", "canvas_counts_sums", "cpu_acquire_ms")} for name, row in result["completed_reports"].items()},
        "overhead_segments": result["overhead_segments"]}, indent=2))


if __name__ == "__main__":
    main()
