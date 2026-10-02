"""Finite matched Genuine18 comparison. Run only for root-granted exact binaries."""
import argparse
import asyncio
import base64
import json
import os
from pathlib import Path
import re
import shutil
import sys
import time
from datetime import timedelta

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
DOCUMENT_SHA = "3ed4679c841bb21af02a02d45c5c5c6d2b3a0504dc1ba78f1b08db4ea12c516f"
LEDGER_SHA = "bd90d3e08d5495594c0d28717ae53e76a84517250397a69bbf855661118d7ea6"
PAIR = ("canvas-zoom-out", "canvas-zoom-in")
VIEW_KEYS = ("page", "zoom", "pan_x", "pan_y", "visible_items", "render_instances", "renderer",
             "selection", "text_faces", "text_refusal", "canvas_error")
STABLE_KEYS = (*VIEW_KEYS, "pending_bytes", "preparation_waiting", "preparation_running",
               "preparation_started", "preparation_finished", "paint_passes")
EXPECTED_WINDOW = None


def write(folder, name, value):
    (folder / f"{name}.json").write_text(json.dumps(value, indent=2), encoding="utf-8", newline="\n")


def identity(state):
    return {key: state[key] for key in VIEW_KEYS}


async def settled(p, label):
    """Reuse canonical load checks; additionally require two stable drained states."""
    deadline = time.perf_counter() + 120
    previous = None
    for index in range(480):
        remaining = deadline - time.perf_counter()
        if remaining <= 0:
            raise RuntimeError(f"{label}: bounded settle expired")
        state = await asyncio.wait_for(owned.canvas_ready(p, None), timeout=remaining)
        p.save(f"{label}-settle-{index:03d}", state)
        assert state["renderer"] == "dx12"
        drained = all(int(state[key]) == 0 for key in
                      ("pending_bytes", "preparation_waiting", "preparation_running"))
        stable = {key: state[key] for key in STABLE_KEYS}
        if drained and stable == previous:
            return state
        previous = stable if drained else None
        if time.perf_counter() >= deadline:
            raise RuntimeError(f"{label}: bounded settle expired; last actual state retained")
        await asyncio.sleep(0.25)
    raise RuntimeError(f"{label}: settle observation bound reached")


async def windows(p, viewport):
    target = EXPECTED_WINDOW or viewport
    if target["width"] is not None:
        await p.steps([{"tool": "lurq_resize", "args": {
            "window": "main", "width": target["width"], "height": target["height"]}}])
    deadline = time.perf_counter() + 30
    for index in range(12):
        remaining = deadline - time.perf_counter()
        if remaining <= 0:
            break
        value = await asyncio.wait_for(p.profile("lurq_windows"), timeout=remaining)
        p.save(f"window-{index:02d}", value)
        main = next(row for row in value["windows"] if row["id"] == "main")
        if (main["width"] > 0 and main["height"] > 0 and
                (target["width"] is None or all(main[key] == target[key] for key in ("width", "height")))):
            expected = target["scale_factor"]
            if expected is not None:
                assert main["scale_factor"] == expected, main
            return {"width": int(main["width"]), "height": int(main["height"]),
                    "scale_factor": main["scale_factor"]}
        await asyncio.sleep(0.25)
    raise RuntimeError("owned window did not reach requested physical size/DPI")


async def canvas_node(p):
    result = await p.call("lurq_find_by_id", {"id": "canvas", "window": "main"})
    node = owned.parse_ref_line("\n".join(block.text for block in result.content if block.type == "text"))
    assert node is not None and node.ref and node.bounds
    return node


async def screenshot(p, label):
    node = await canvas_node(p)
    result = await p.call("lurq_screenshot", {"ref": node.ref})
    pictures = [block for block in result.content if block.type == "image"]
    assert len(pictures) == 1 and pictures[0].mimeType == "image/png"
    path = owned.OUT / f"{label}.png"
    path.write_bytes(base64.b64decode(pictures[0].data, validate=True))
    p.save(f"{label}-image", {"png_sha256": owned.profiling.digest(path), "canvas_bounds": node.bounds,
                              "scope": "authenticated Canvas element PNG outside timed workload"})


async def end_complete(p, identifier, label):
    # No UI request here. Server-thread reads allow the final notification tail to finish.
    deadline = time.perf_counter() + 3
    for index in range(150):
        remaining = deadline - time.perf_counter()
        if remaining <= 0:
            break
        live = await asyncio.wait_for(p.profile("lurq_profile_read", {"id": identifier}), timeout=remaining)
        if not live["in_flight"]:
            report = await asyncio.wait_for(p.end(identifier), timeout=max(0.001, deadline - time.perf_counter()))
            p.save(label, report)
            assert not report["truncated"] and report["dropped_samples"] == 0
            assert report["boundary_excluded_samples"] == 0
            assert not report["in_flight"], "new work crossed end; report retained, no complete-boundary claim"
            return report
        p.save(f"{label}-boundary-{index:03d}", live)
        if time.perf_counter() >= deadline:
            raise RuntimeError(f"{label}: work remained unfinished at bounded end")
        await asyncio.sleep(0.02)
    raise RuntimeError(f"{label}: boundary observation bound reached")


async def pair(p, label):
    states = []
    for step in PAIR:
        await p.steps([{"click_id": step}])
        states.append(await settled(p, f"{label}-{step}"))
    return states


async def journey(app):
    process = owned.profiling.Process(app.pid, owned.BINARY)
    async with httpx.AsyncClient(headers={"Authorization": f"Bearer {app.entry.token}"},
                                 timeout=httpx.Timeout(15, read=15)) as client:
        async with streamable_http_client(app.entry.url, http_client=client) as (read, send, _):
            async with ClientSession(read, send, read_timeout_seconds=timedelta(seconds=15)) as session:
                await session.initialize()
                p = owned.profiling.Probe(session, owned.OUT, process)
                try:
                    availability = await p.profile("lurq_profile_read")
                    p.save("availability", availability)
                    assert availability["build"]["features"]["perf_profile"]
                    assert not availability["build"]["features"]["devtools"]
                    assert not availability["build"]["debug_assertions"]
                    window = await windows(p, CONFIG["viewport"])
                    opening = await p.profile("kontur_local_document_open_v1", {"document_id": owned.DOC})
                    p.save("document-open", opening)
                    assert opening["state"] == "opening" and opening["document"]["id"] == owned.DOC
                    await settled(p, "opened")
                    await p.steps([{"click_id": "canvas-fit"}])
                    await settled(p, "fit")
                    await pair(p, "warmup")
                    await p.steps([{"click_id": "canvas-fit"}])
                    before = await settled(p, "reset")
                    node = await canvas_node(p)
                    p.save("before", {"window": window, "canvas": before, "canvas_bounds": node.bounds})
                    if CONFIG["screenshots"]:
                        tools = await session.list_tools()
                        assert any(tool.name == "lurq_screenshot" for tool in tools.tools)
                        await screenshot(p, "before")
                        assert identity(await settled(p, "after-before-screenshot")) == identity(before)
                    idle_id = await p.start(240)
                    await asyncio.sleep(0.5)
                    idle = await end_complete(p, idle_id, "idle-profile")
                    assert identity(await settled(p, "after-idle")) == identity(before)
                    first = await p.start(240)
                    started, cpu_start = time.perf_counter(), process.cpu()
                    states = await pair(p, "measured")
                    workload_ms = (time.perf_counter() - started) * 1000
                    process_cpu_ms = (process.cpu() - cpu_start) * 1000
                    report = await end_complete(p, first, "pair-profile")
                    after = states[-1]
                    assert identity(before) == identity(after), "pair changed actual viewport/content state"
                    actual_window = await p.profile("lurq_windows")
                    p.save("window-after", actual_window)
                    actual_main = next(row for row in actual_window["windows"] if row["id"] == "main")
                    assert all(actual_main[key] == value for key, value in window.items())
                    if CONFIG["screenshots"]:
                        await screenshot(p, "after")
                        assert identity(await settled(p, "after-final-screenshot")) == identity(after)
                    passes = [row for row in report["samples"] if row["data"]["kind"] == "pass"]
                    assert passes and all(row["data"]["canvas_text"] is not None for row in passes)
                    p.save("canvas-stages", owned.profiling.stages(report))
                    text = [row["data"]["canvas_text"] for row in passes]
                    p.save("canvas-text-totals", {section: {
                        key: sum(row[section][key] for row in text) for key in text[0][section]}
                        for section in ("cpu_timings_ms", "counts")})
                    p.save("top-cost-passes", sorted(passes,
                        key=lambda row: row["data"]["cpu_timings_ms"]["total"], reverse=True)[:5])
                    p.save("summary", {"case": CASE["name"], "build_identity": BUILD,
                        "fixture": "Genuine18 / 01 Editor", "viewport_before": before,
                        "viewport_after": after, "window": window, "canvas_bounds": node.bounds,
                        "workload_ms": workload_ms, "process_cpu_ms": process_cpu_ms,
                        "workload_latency_scope": "two clicks, frame barriers and bounded drain observations",
                        "pass_records": len(passes), "completed_records": report["completed_samples"],
                        "idle_records": idle["completed_samples"], "work_id_available": False,
                        "window_position_scope": "same configured initial position or copied preferences/default; MCP does not expose actual OS position",
                        "cache_control": "fresh process plus identical open/fit/warmup/reset sequence; actual counters retained, not assumed equal",
                        "gpu_timestamps_available": False, "scope": "one finite matched workload; no FPS/statistical overhead claim"})
                finally:
                    await p.cleanup()
                    p.save("tool-latencies", p.latencies)
                    process.close()


def seed_profile(source, destination):
    assert not destination.exists(), "never replace a prior owned profile"
    documents = destination / "Kontur/local-documents"
    documents.mkdir(parents=True)
    for file in (source / "Kontur/local-documents").iterdir():
        if file.is_file() and (file.suffix == ".kontur" or file.name == "ledger.json"):
            shutil.copy2(file, documents / file.name)
    preference = source / "Kontur/desktop-preferences.redb"
    assert preference.is_file()
    shutil.copy2(preference, destination / "Kontur/desktop-preferences.redb")
    assert owned.profiling.digest(documents / f"{owned.DOC}.kontur") == DOCUMENT_SHA
    assert owned.profiling.digest(documents / "ledger.json") == LEDGER_SHA
    return owned.profiling.digest(preference)


def main():
    global owned, httpx, ClientSession, streamable_http_client, CONFIG, CASE, BUILD, EXPECTED_WINDOW
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    CONFIG = json.loads(args.manifest.read_text(encoding="utf-8-sig"))
    output = Path(CONFIG["output"]).resolve()
    source = Path(CONFIG["fixture_profile"]).resolve()
    assert output.is_relative_to(HERE) and not output.exists()
    assert source.is_relative_to(HERE.parents[1] / ".tmp")
    assert len(CONFIG["cases"]) == 2 and [case["name"] for case in CONFIG["cases"]] == ["baseline", "candidate"]
    sizes = [CONFIG["viewport"][key] for key in ("width", "height")]
    assert all(value is None for value in sizes) or all(isinstance(value, int) and value > 0 for value in sizes)
    scale = CONFIG["viewport"]["scale_factor"]
    assert (scale is None or scale > 0) and isinstance(CONFIG["screenshots"], bool)
    position = CONFIG["window_position"]
    assert position is None or re.fullmatch(r"-?[0-9]+,-?[0-9]+", position)
    assert isinstance(CONFIG["port"], int) and 1024 <= CONFIG["port"] <= 65535
    output.mkdir()
    temp = output / "temp"
    temp.mkdir()
    os.environ.update(TEMP=str(temp), TMP=str(temp))
    sys.path.insert(0, str(HERE))
    import native as owned
    import httpx
    from mcp import ClientSession
    from mcp.client.streamable_http import streamable_http_client
    # Validate BOTH frozen inputs before starting either owned app.
    inputs = []
    for case in CONFIG["cases"]:
        binary = Path(case["binary"]).resolve()
        assert binary.is_relative_to(HERE) and binary.is_file()
        build = json.loads(Path(case["build_identity"]).read_text(encoding="utf-8-sig"))
        assert build["variant"] == case["name"] and build["build_exit_code"] == 0
        assert Path(build["binary_path"]).resolve() == binary
        assert re.fullmatch("[0-9a-f]{64}", build["binary_sha256"])
        assert owned.profiling.digest(binary) == build["binary_sha256"]
        inputs.append((case, binary, build))
    old_position = os.environ.get("KONTUR_DESKTOP_WINDOW_POSITION")
    if position is None:
        os.environ.pop("KONTUR_DESKTOP_WINDOW_POSITION", None)
    else:
        os.environ["KONTUR_DESKTOP_WINDOW_POSITION"] = position
    write(output, "manifest-used", CONFIG)
    summaries = []
    try:
        for CASE, binary, BUILD in inputs:
            case_root = output / CASE["name"]
            preference_hash = seed_profile(source, case_root / "profile")
            write(case_root, "frozen-inputs", {"build": BUILD, "seed_preference_sha256": preference_hash,
                "fixture_document_sha256": DOCUMENT_SHA, "fixture_ledger_sha256": LEDGER_SHA})
            owned.PROFILE, owned.OUT, owned.BINARY = case_root / "profile", case_root / "capture", binary
            owned.FILE = owned.PROFILE / "Kontur/local-documents" / f"{owned.DOC}.kontur"
            owned.LEDGER = owned.FILE.parent / "ledger.json"
            owned.PORT, owned.journey = CONFIG["port"], journey
            owned.main()
            before_hashes = json.loads((owned.OUT / "before-launch-hashes.json").read_text())
            after_hashes = json.loads((owned.OUT / "after-run-hashes.json").read_text())
            assert before_hashes == after_hashes == {"document": DOCUMENT_SHA, "ledger": LEDGER_SHA}
            summary = json.loads((owned.OUT / "summary.json").read_text())
            summary["seed_preference_sha256"] = preference_hash
            summaries.append(summary)
            EXPECTED_WINDOW = summary["window"]
        assert summaries[0]["window"] == summaries[1]["window"]
        assert summaries[0]["canvas_bounds"] == summaries[1]["canvas_bounds"]
        assert identity(summaries[0]["viewport_before"]) == identity(summaries[1]["viewport_before"])
        assert summaries[0]["seed_preference_sha256"] == summaries[1]["seed_preference_sha256"]
        write(output, "matched-summary", {"status": "completed", "cases": summaries,
            "pixel_comparison": "separate decoded-pixel analysis after both owned processes close",
            "scope": "same fixture/window/sequence, fresh owned processes; actual cache counters retained"})
    finally:
        if old_position is None:
            os.environ.pop("KONTUR_DESKTOP_WINDOW_POSITION", None)
        else:
            os.environ["KONTUR_DESKTOP_WINDOW_POSITION"] = old_position


if __name__ == "__main__":
    main()
