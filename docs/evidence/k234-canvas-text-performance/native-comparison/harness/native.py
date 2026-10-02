"""One owned real Kontur run; SDK capture against the copied native2 fixture."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import socket
import sys
import time
from datetime import timedelta

import httpx
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client

WARM = Path(__file__).resolve().parent.parent.parent
HERE = Path(__file__).resolve().parent
UPSTREAM = Path("H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance")
PREP = UPSTREAM / ".tmp/k232-native-profiling"
sys.path.insert(0, str(PREP))
import probe as profiling
sys.path.insert(0, "H:/projects/pencil-web/scripts/desktop")
from harness.app import DesktopApp, LaunchConfig
from harness.tree import parse_ref_line

DOC = "0acea323-5056-408d-84c0-f7c9a82e7c36"
PROFILE = HERE / "profile"
FILE = PROFILE / "Kontur/local-documents" / f"{DOC}.kontur"
LEDGER = FILE.parent / "ledger.json"
BINARY = HERE / "kontur-profiler.exe"
OUT = PREP / "results/native2"
PORT = 4903


def save(name, value):
    (OUT / f"{name}.json").write_text(json.dumps(value, indent=2), encoding="utf-8")


async def canvas_ready(probe, first):
    """Canonical state predicate; tool deadlines stay 15s throughout."""
    deadline = time.perf_counter() + 120
    index = 0
    while True:
        observations = {}
        for identifier in ("editor", "save-status", "canvas"):
            result = await probe.call("lurq_find_by_id", {"id": identifier, "window": "main"})
            text = "\n".join(row.text for row in result.content if row.type == "text")
            node = parse_ref_line(text)
            observations[identifier] = {"raw": result.model_dump(mode="json", exclude_none=True),
                                       "attrs": node.attrs if node else None}
        probe.save(f"readiness-{index:03d}", observations)
        editor = observations["editor"]["attrs"] or {}
        canvas = observations["canvas"]["attrs"] or {}
        saved = observations["save-status"]["attrs"] or {}
        if editor.get("document") == DOC and editor.get("state") == "open":
            assert not canvas.get("canvas_error") and not canvas.get("text_refusal"), canvas
            if (canvas.get("state") == "open" and int(canvas.get("visible_items", "0")) > 0
                    and canvas.get("render_instances") == canvas.get("visible_items")
                    and saved.get("state") == "saved-local" and saved.get("revision") == "1"):
                assert canvas.get("page") == "10436fe8-bb35-4281-be38-0bbd5dad62d3", canvas
                return canvas
        if time.perf_counter() >= deadline:
            raise AssertionError("canonical120s canvas readiness expired; raw states retained")
        if first is not None:
            probe.save("readiness-profile-latest",
                       await probe.profile("lurq_profile_read", {"id": first}))
        index += 1
        await asyncio.sleep(0.25)


async def journey(app):
    process = profiling.Process(app.pid, BINARY)
    async with httpx.AsyncClient(headers={"Authorization": f"Bearer {app.entry.token}"},
                                 timeout=httpx.Timeout(15, read=15)) as client:
        async with streamable_http_client(app.entry.url, http_client=client) as (read, write, _):
            async with ClientSession(read, write, read_timeout_seconds=timedelta(seconds=15)) as session:
                await session.initialize()
                p = profiling.Probe(session, OUT, process)
                try:
                    availability = await p.profile("lurq_profile_read")
                    p.save("availability", availability)
                    first = await p.start()
                    opening = await p.profile("kontur_local_document_open_v1", {"document_id": DOC})
                    p.save("document-open-response", opening)
                    assert opening.get("state") == "opening", opening
                    assert opening["document"]["id"] == DOC, opening
                    waiting = asyncio.create_task(p.call("lurq_wait", {
                        "window": "main", "frames": 1, "timeout_ms": 14000}))
                    deadline = time.perf_counter() + 3
                    observed = None
                    while time.perf_counter() < deadline and not waiting.done():
                        current = await p.profile("lurq_profile_read", {"id": first})
                        if any(row["phase_elapsed_so_far_ms"] >= 100 for row in current["in_flight"]):
                            observed = current
                            break
                        await asyncio.sleep(0.05)
                    if observed:
                        p.save("cold-open-in-flight", observed)
                        second = await p.start(64)
                        before = time.perf_counter()
                        p.save("cold-open-end2", await p.end(second))
                        p.save("cold-open-end2-latency", {
                            "elapsed_ms": (time.perf_counter() - before) * 1000})
                    p.save("first-frame-response", (await waiting).model_dump(mode="json", exclude_none=True))
                    canvas = await canvas_ready(p, first)
                    opened = await p.end(first)
                    p.save("cold-open-end1", opened)
                    p.save("cold-open-stages", profiling.stages(opened))
                    p.save("canvas-identity", canvas)
                    manifest = json.loads((PREP / "workload.template.json").read_text())
                    manifest["identity"].update({
                        "kontur_sha": "8e2bb021989688f6d260fe04daf06bccee3e744a",
                        "kontur_tree": "612b074cdaf74a490f1729c255c279d67c52f17c",
                        "lurq_tree": "04fa9bd1dfcbe2e594f0eb95b27d5d48ca8ccabc",
                        "cargo_command": "cargo build -p kontur-desktop --bin kontur-desktop --locked --offline -j1 --features lurq/perf_profile --config .tmp/k232-profiler-candidate/patch.toml",
                        "build_profile": "dev, debug0, incremental0; named text dependencies opt-level2",
                        "current_fixture_counts": {"visible_scene_items": canvas.get("visible_items"),
                                                   "render_instances": canvas.get("render_instances")},
                        "contention": "external Orchester compiler graph active; no precise causal overhead claim"})
                    p.save("workload-actual", manifest)
                    baseline = {"document": profiling.digest(FILE), "ledger": profiling.digest(LEDGER)}
                    p.save("reversible-baseline", baseline)
                    print(json.dumps({"stage": "cold-open-canvas-captured", "pid": app.pid}), flush=True)
                    await profiling.overlap(p, manifest)
                    print(json.dumps({"stage": "overlap-captured", "pid": app.pid}), flush=True)
                    assert baseline == {"document": profiling.digest(FILE), "ledger": profiling.digest(LEDGER)}
                    await profiling.overhead(p, manifest)
                    print(json.dumps({"stage": "overhead-captured-contention-recorded", "pid": app.pid}), flush=True)
                    assert baseline == {"document": profiling.digest(FILE), "ledger": profiling.digest(LEDGER)}
                    await profiling.stall(p, manifest)
                    assert baseline == {"document": profiling.digest(FILE), "ledger": profiling.digest(LEDGER)}
                    p.save("reversible-preservation", baseline)
                finally:
                    await p.cleanup()
                    p.save("tool-latencies", p.latencies)
                    process.close()


def main():
    OUT.mkdir(parents=True, exist_ok=False)
    originals = {"document": profiling.digest(FILE), "ledger": profiling.digest(LEDGER)}
    save("before-launch-hashes", originals)
    with socket.socket() as sock:
        assert sock.connect_ex(("127.0.0.1", PORT)) != 0
    overrides = {key: value for key, value in os.environ.items()
                 if key.startswith("KONTUR_DESKTOP_") and key.endswith("_PATH")}
    for key in overrides:
        os.environ.pop(key)
    app = DesktopApp(LaunchConfig(BINARY, PORT, extra_env={
        "LOCALAPPDATA": str(PROFILE), "KONTUR_API_URL": "none",
        "KONTUR_DESKTOP_RENDERER": "dx12", "RUST_LOG": "warn,video=off,kontur_desktop=info"}),
        OUT / "stderr.txt")
    try:
        app.launch()
        save("launch", {"pid": app.pid, "port": PORT, "binary_sha256": profiling.digest(BINARY),
             "profile": str(PROFILE), "renderer": "dx12", "api": "none", "normal_tool_deadline_s": 15,
             "cleared_override_keys": sorted(overrides), "default_mcp": True, "devtools": False,
             "external_compiler_contention": "root observed Orchester graph parent31016; preserve it"})
        print(json.dumps({"stage": "owned-native-started", "pid": app.pid, "port": PORT}), flush=True)
        asyncio.run(journey(app))
        save("result", {"status": "completed"})
    except BaseException as error:
        save("failure", {"error_class": type(error).__name__, "status": "stopped at first real failure"})
        print(json.dumps({"stage": "native-failure", "error_class": type(error).__name__}), flush=True)
        raise
    finally:
        pid = app.pid if app.process else None
        stopped = app.stop()
        for key, value in overrides.items():
            os.environ[key] = value
        # Process termination can precede socket closure briefly. Verify for at
        # most2s; never terminate any other listener if the port remains open.
        closure_deadline = time.perf_counter() + 2
        while True:
            with socket.socket() as sock:
                closed = sock.connect_ex(("127.0.0.1", PORT)) != 0
            if closed or time.perf_counter() >= closure_deadline:
                break
            time.sleep(0.1)
        save("cleanup", {"pid": pid, "stop": stopped, "exited": not app.is_running(), "port_closed": closed})
        save("after-run-hashes", {"document": profiling.digest(FILE), "ledger": profiling.digest(LEDGER)})
        assert not app.is_running() and closed


if __name__ == "__main__":
    main()
