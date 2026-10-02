"""One owned 0.33.1 integration smoke; prepare only until root grants execution."""
import argparse
import asyncio
import base64
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
HELPERS = HERE.parent
WARM = HELPERS.parents[1]
UPSTREAM = WARM.parent / "lurq-canvas-text-performance"
LURQ_HEAD = "cf0c5d81ddab2c960f1aff34a4b5117a73ade9e5"
LURQ_TREE = "43dda4b9073c327091dc9fc7d167a41b04e75f98"
KONTUR_HEAD = "61d73d08b7c047d6c62ad55d6ed127a495881772"
HELPER_SHA = "9e2426516256fb5aebd4a21cd9939034d5cab8a20c5bfdb7a02fecb67b12c7cb"
SIZES = ((2160, 1440), (1800, 1200), (2160, 1440))
SCALE = 1.5


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_object(expression, checkout=UPSTREAM):
    value = subprocess.run(["git", "rev-parse", expression], cwd=checkout,
                           capture_output=True, text=True, check=True, timeout=10)
    return value.stdout.strip()


def runtime_inventory(pid):
    # Retain only safe inventory fields, never raw command lines or environment.
    command = ("$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process | "
               f"Where-Object {{$_.ProcessId -eq {int(pid)} -or $_.Name -in "
               "@('cargo.exe','rustc.exe','link.exe','kontur-desktop.exe')} | "
               "Select-Object ProcessId,ParentProcessId,Name,ExecutablePath,"
               "@{Name='Crate';Expression={if($_.Name -eq 'rustc.exe' -and "
               "$_.CommandLine -match '--crate-name\\s+([a-zA-Z0-9_]{1,128})'){ $Matches[1] }}} | "
               "ConvertTo-Json -Compress; exit 0")
    result = subprocess.run(["powershell", "-NoProfile", "-NonInteractive", "-Command", command],
        capture_output=True, text=True, check=True, timeout=15,
        creationflags=subprocess.CREATE_NO_WINDOW)
    return {"observed_utc": datetime.now(timezone.utc).isoformat(), "owned_pid": pid,
            "processes": json.loads(result.stdout) if result.stdout.strip() else [],
            "scope": "fresh safe process inventory; preserve external processes; uncontrolled load, no utilization or performance comparison claim"}


def walk(node):
    yield node
    for child in node.get("children", []):
        yield from walk(child)


async def capture_home(p, stage, window):
    response = await p.call("lurq_inspect", {"window": "main", "max_depth": 64, "max_nodes": 2000})
    p.save(f"{stage}-inspect-response", response.model_dump(mode="json", exclude_none=True))
    inspection = response.structuredContent
    if inspection is None:
        inspection = json.loads("\n".join(block.text for block in response.content if block.type == "text"))
    p.save(f"{stage}-inspect-raw", inspection)
    assert not inspection["truncated"] and inspection["tree"] is not None
    controls = [node for node in walk(inspection["tree"]) if node.get("id") == "new-document"]
    assert len(controls) == 1
    control = controls[0]
    # inspection_attrs is Vec<(String,String)>; serde exports an array of pairs.
    attrs = dict(control["attrs"])
    assert attrs["available"] == "true" and attrs["pending"] == "false"
    labels = [node for node in walk(control) if node["role"] == "text" and node.get("name")]
    assert labels, "actual new-document text child unavailable"
    p.save(f"{stage}-semantic", {"window": window, "control": control, "labels": labels})
    for name, node in [("control", control), *[(f"label-{i}", node) for i, node in enumerate(labels)]]:
        assert node["ref"] and node["bounds"] and all(value > 0 for value in node["bounds"][2:])
        image = await p.call("lurq_screenshot", {"ref": node["ref"]})
        pictures = [block for block in image.content if block.type == "image"]
        assert len(pictures) == 1 and pictures[0].mimeType == "image/png"
        path = owned.OUT / f"{stage}-{name}.png"
        path.write_bytes(base64.b64decode(pictures[0].data, validate=True))
        p.save(f"{stage}-{name}-image", {"png_sha256": digest(path), "bounds": node["bounds"],
            "name": node.get("name"), "scope": "authenticated bounded PNG readback; no click/create"})
    return {"window": window, "control_bounds": control["bounds"],
            "labels": [{"name": node["name"], "bounds": node["bounds"]} for node in labels]}


async def integration_windows(p, viewport):
    p.save("runtime-inventory-home", runtime_inventory(PID))
    availability = await p.profile("lurq_profile_read")
    p.save("integration-runtime-availability", availability)
    assert availability["build"]["lurq_version"] == "0.33.1"
    await p.steps([{"tool": "lurq_navigate", "args": {"path": "/home", "replace": True}}])
    observations = []
    for index, (width, height) in enumerate(SIZES):
        window = await original_windows(p, {"width": width, "height": height, "scale_factor": SCALE})
        observations.append(await capture_home(p, f"home-{index}-{width}x{height}", window))
    assert observations[0] == observations[2], "restored actual Home label/bounds differ"
    p.save("home-resize-summary", {"stages": observations, "scope": "one resize sequence, no document creation"})
    p.save("runtime-inventory-before-canvas", runtime_inventory(PID))
    return await original_windows(p, viewport)


async def journey(app):
    global PID
    PID = app.pid
    # Reuse its exact authenticated SDK session, Canvas sequence and assertions.
    await matched.journey(app)


def upload_details(capture):
    report = json.loads((capture / "pair-profile.json").read_text(encoding="utf-8-sig"))
    assert report["finalized"] and not report["in_flight"]
    assert report["dropped_samples"] == report["boundary_excluded_samples"] == 0
    rows = []
    children = ("texture_creation", "pixel_packing", "upload_staging_commands", "descriptor_writes")
    event_counts = ("cache_hits", "cache_misses", "texture_creations", "descriptor_pairs",
                    "padded_upload_bytes", "arena_uploads", "dedicated_uploads", "cache_evictions")
    for sample in report["samples"]:
        data = sample["data"]
        if data["kind"] != "pass" or data["frame"] is None:
            continue
        render = data["frame"]["render"]
        canvas = render["canvas"]
        detail = canvas["asset_upload_details"]
        if detail is None:
            assert canvas["counts"]["uploaded_asset_bytes"] == 0
            continue
        assert render["coverage"]["canvas_asset_upload_details"]
        timing, count = detail["cpu_timings_ms"], detail["counts"]
        assert count["cache_misses"] == count["texture_creations"]
        assert count["arena_uploads"] + count["dedicated_uploads"] == count["texture_creations"]
        assert count["padded_upload_bytes"] >= canvas["counts"]["uploaded_asset_bytes"]
        assert count["descriptor_pairs"] <= count["cache_hits"] + count["cache_misses"]
        assert count["cache_entries_after"] == count["cache_entries_before"] + count["texture_creations"] - count["cache_evictions"]
        assert count["cache_charged_bytes_peak"] >= max(count["cache_charged_bytes_before"], count["cache_charged_bytes_after"])
        assert sum(timing[key] for key in children) <= canvas["cpu_timings_ms"]["asset_upload"] + 0.000001
        rows.append({"window": sample["window"], "frame_id": data["frame_id"],
            "pass_total_ms": data["cpu_timings_ms"]["total"], "canvas_cpu_timings_ms": canvas["cpu_timings_ms"],
            "canvas_counts": canvas["counts"], "asset_upload_details": detail})
    assert rows and sum(row["canvas_counts"]["uploaded_asset_bytes"] for row in rows) > 0
    result = {"fixture": "Genuine18 / 01 Editor", "detail_pass_records": len(rows), "passes": rows,
        "cpu_totals_ms": {key: sum(row["asset_upload_details"]["cpu_timings_ms"][key] for row in rows)
                          for key in (*children, "cache_eviction")},
        "event_counts": {key: sum(row["asset_upload_details"]["counts"][key] for row in rows) for key in event_counts},
        "uploaded_asset_bytes": sum(row["canvas_counts"]["uploaded_asset_bytes"] for row in rows),
        "coarse_asset_upload_ms": sum(row["canvas_cpu_timings_ms"]["asset_upload"] for row in rows),
        "cache_gauge_scope": "before/peak/after bytes and entry counts retained per pass; not summed as events",
        "scope": "four CPU child scopes nest in asset_upload; cache_eviction is outside it inside Canvas total; no GPU timestamps or speedup/overhead claim"}
    (capture / "upload-details.json").write_text(json.dumps(result, indent=2), encoding="utf-8", newline="\n")


def main():
    global owned, matched, original_windows
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--build-identity", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary, identity_path, output = args.binary.resolve(), args.build_identity.resolve(), args.output.resolve()
    assert binary.is_relative_to(HELPERS) and binary.is_file()
    assert identity_path.is_relative_to(HELPERS) and identity_path.is_file()
    assert output.is_relative_to(HERE) and not output.exists()
    build = json.loads(identity_path.read_text(encoding="utf-8-sig"))
    assert build["exit_code"] == 0 and binary == identity_path.parent / "kontur-desktop.exe"
    assert digest(binary) == build["binary_sha256"]
    assert build["lurq_head"] == LURQ_HEAD and build["kontur_head"] == KONTUR_HEAD
    assert git_object(f"{LURQ_HEAD}^{{tree}}") == LURQ_TREE
    assert git_object("HEAD") == LURQ_HEAD and git_object("HEAD", WARM) == KONTUR_HEAD
    assert digest(WARM / "Cargo.lock") == build["registry_lock_sha256"]
    assert digest(identity_path.parent / "Cargo.lock.local") == build["local_lock_sha256"]
    assert "--release" in build["args"]
    assert digest(HELPERS / "matched_compare.py") == HELPER_SHA
    output.mkdir()
    temp = output / "temp"
    temp.mkdir()
    os.environ.update(TEMP=str(temp), TMP=str(temp))
    sys.path.insert(0, str(HELPERS))
    import native as owned
    import matched_compare as matched
    matched.owned, matched.httpx = owned, owned.httpx
    matched.ClientSession, matched.streamable_http_client = owned.ClientSession, owned.streamable_http_client
    matched.CONFIG = {"viewport": {"width": 2160, "height": 1440, "scale_factor": SCALE}, "screenshots": True}
    matched.CASE, matched.BUILD, matched.EXPECTED_WINDOW = {"name": "dx12-sampled-assets"}, build, None
    original_windows, matched.windows = matched.windows, integration_windows
    seed = WARM / ".tmp/k230-independent-native2/profile"
    preference_hash = matched.seed_profile(seed, output / "profile")
    owned.PROFILE, owned.OUT, owned.BINARY = output / "profile", output / "capture", binary
    owned.FILE = owned.PROFILE / "Kontur/local-documents" / f"{owned.DOC}.kontur"
    owned.LEDGER, owned.PORT, owned.journey = owned.FILE.parent / "ledger.json", 4903, journey
    matched.write(output, "frozen-inputs", {"build": build, "build_identity_sha256": digest(identity_path),
        "derived_source_trees": {"lurq": LURQ_TREE, "lurq_crate": git_object(f"{LURQ_HEAD}:crates/lurq"),
            "kontur": git_object(f"{KONTUR_HEAD}^{{tree}}", WARM)}, "seed_preference_sha256": preference_hash,
        "document_sha256": matched.DOCUMENT_SHA, "ledger_sha256": matched.LEDGER_SHA,
        "sources_sha256": {str(path): digest(path) for path in
            (Path(__file__), HELPERS / "native.py", HELPERS / "matched_compare.py", owned.PREP / "probe.py")},
        "historical_launch_inventory": "native.py launch field is historical; use fresh runtime-inventory receipts",
        "claim_scope": "integration smoke only; no comparative speedup claim on 0.33.1"})
    position = os.environ.pop("KONTUR_DESKTOP_WINDOW_POSITION", None)
    try:
        owned.main()
        upload_details(output / "capture")
        expected = {"document": matched.DOCUMENT_SHA, "ledger": matched.LEDGER_SHA}
        for name in ("before-launch-hashes", "after-run-hashes"):
            assert json.loads((owned.OUT / f"{name}.json").read_text(encoding="utf-8-sig")) == expected
        cleanup = json.loads((owned.OUT / "cleanup.json").read_text(encoding="utf-8-sig"))
        assert cleanup["exited"] and cleanup["port_closed"]
        matched.write(output, "smoke-result", {"status": "completed", "cleanup": cleanup,
            "scope": "Genuine18 / 01 Editor diagnostic with DX12 upload detail; no comparison/speedup claim"})
    finally:
        if position is not None:
            os.environ["KONTUR_DESKTOP_WINDOW_POSITION"] = position


if __name__ == "__main__":
    main()
