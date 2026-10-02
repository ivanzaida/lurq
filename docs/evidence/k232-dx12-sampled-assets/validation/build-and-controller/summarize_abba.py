"""Reduce four closed raw profiles; gauges and nested timings stay separate."""
import json
from pathlib import Path
from statistics import mean

HERE = Path(__file__).resolve().parent
rows = json.loads((HERE / "abba-01/run-receipts.json").read_text(encoding="utf-8-sig"))
assert [row["name"] for row in rows] == ["A1", "B1", "B2", "A2"]
assert all(row["exit_code"] == 0 for row in rows)
timers = ("texture_creation", "pixel_packing", "upload_staging_commands", "descriptor_writes", "cache_eviction")
events = ("cache_hits", "cache_misses", "texture_creations", "descriptor_pairs", "padded_upload_bytes",
          "arena_uploads", "dedicated_uploads", "cache_evictions")
view_keys = ("page", "zoom", "pan_x", "pan_y", "visible_items", "render_instances", "renderer",
             "selection", "text_faces", "text_refusal", "canvas_error")
results = []
for row in rows:
    capture = Path(row["capture_root"]) / "capture"
    read = lambda name: json.loads((capture / f"{name}.json").read_text(encoding="utf-8-sig"))
    report, summary, cleanup = read("pair-profile"), read("summary"), read("cleanup")
    assert cleanup["exited"] and cleanup["port_closed"]
    assert report["finalized"] and not report["in_flight"]
    assert report["dropped_samples"] == report["boundary_excluded_samples"] == 0
    passes = [sample["data"] for sample in report["samples"] if sample["data"]["kind"] == "pass"]
    canvases = [data["frame"]["render"]["canvas"] for data in passes if data["frame"] is not None]
    details = [canvas["asset_upload_details"] for canvas in canvases if canvas["asset_upload_details"] is not None]
    cpu = {key: sum(detail["cpu_timings_ms"][key] for detail in details) for key in timers}
    counts = {key: sum(detail["counts"][key] for detail in details) for key in events}
    upload = sum(canvas["cpu_timings_ms"]["asset_upload"] for canvas in canvases)
    inventories = [read("runtime-inventory-home"), read("runtime-inventory-before-canvas")]
    compiler_ids = sorted({process["ProcessId"] for inventory in inventories
        for process in inventory["processes"] if process["Name"] in ("cargo.exe", "rustc.exe", "link.exe")})
    result = {"name": row["name"], "variant": row["variant"], "fixture": "Genuine18 / 01 Editor",
        "source_head": row["source_head"], "binary_sha256": summary["build_identity"]["binary_sha256"],
        "workload_ms": summary["workload_ms"], "max_pass_ms": max(data["cpu_timings_ms"]["total"] for data in passes),
        "pass_records": len(passes), "completed_records": report["completed_samples"],
        "asset_upload_ms": upload, "upload_cpu_children_ms": cpu, "upload_event_counts": counts,
        "uploaded_asset_bytes": sum(canvas["counts"]["uploaded_asset_bytes"] for canvas in canvases),
        "texture_creation_fraction_of_inclusive_upload": cpu["texture_creation"] / upload,
        "external_compiler_process_ids_observed": compiler_ids, "window": summary["window"],
        "canvas_bounds": summary["canvas_bounds"],
        "viewport_before": {key: summary["viewport_before"][key] for key in view_keys},
        "viewport_after": {key: summary["viewport_after"][key] for key in view_keys},
        "historical_pending_after": summary["viewport_after"]["pending_bytes"], "cleanup": cleanup}
    assert result["viewport_before"] == result["viewport_after"]
    results.append(result)
for result in results:
    for key in ("window", "canvas_bounds", "viewport_before"):
        assert result[key] == results[0][key], (result["name"], key)
groups = {variant: [row for row in results if row["variant"] == variant] for variant in ("baseline", "candidate")}
means = {variant: {key: mean(row[key] for row in group)
    for key in ("workload_ms", "max_pass_ms", "asset_upload_ms")} for variant, group in groups.items()}
for variant, group in groups.items():
    means[variant]["texture_creation_ms"] = mean(row["upload_cpu_children_ms"]["texture_creation"] for row in group)
comparison = {key: {"candidate_div_baseline_mean": means["candidate"][key] / means["baseline"][key],
    "descriptive_reduction_percent": 100 * (1 - means["candidate"][key] / means["baseline"][key])}
    for key in means["baseline"]}
output = HERE / "abba-01/comparison-summary.json"
assert not output.exists()
packet = {"runs": results, "two_observation_means_ms": means, "descriptive_comparison": comparison,
    "upload_bytes_equal": len({row["uploaded_asset_bytes"] for row in results}) == 1,
    "upload_event_counts_equal": all(row["upload_event_counts"] == results[0]["upload_event_counts"] for row in results),
    "scope": "Four finite source/binary/fixture/window/sequence matched runs; means are descriptive, not statistical evidence, GPU time, FPS or instrumentation overhead. Nested upload fractions differ from whole-workload ratios; cache gauges remain in raw per-pass reports."}
output.write_text(json.dumps(packet, indent=2), encoding="utf-8")
print(json.dumps(packet, indent=2))
