"""One ready genuine-page reversed zoom pair, using the reviewed phase split."""
import asyncio
import json
import time
import sys
from datetime import timedelta

import httpx
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client

import native as owned

MODE = sys.argv[1] if len(sys.argv) > 1 else "refined"
assert MODE in ("refined", "canvas-text")
owned.BINARY = owned.HERE / f"kontur-profiler-{MODE}.exe"
owned.OUT = owned.PREP / f"results/{MODE}-native1"


async def journey(app):
    process = owned.profiling.Process(app.pid, owned.BINARY)
    async with httpx.AsyncClient(headers={"Authorization": f"Bearer {app.entry.token}"},
                                 timeout=httpx.Timeout(15, read=15)) as client:
        async with streamable_http_client(app.entry.url, http_client=client) as (read, write, _):
            async with ClientSession(read, write, read_timeout_seconds=timedelta(seconds=15)) as session:
                await session.initialize()
                p = owned.profiling.Probe(session, owned.OUT, process)
                try:
                    availability = await p.profile("lurq_profile_read")
                    p.save("availability", availability)
                    assert availability["build"]["features"]["perf_profile"]
                    assert not availability["build"]["features"]["devtools"]
                    opening = await p.profile("kontur_local_document_open_v1", {"document_id": owned.DOC})
                    p.save("document-open-response", opening)
                    assert opening["state"] == "opening" and opening["document"]["id"] == owned.DOC
                    await owned.canvas_ready(p, None)
                    await p.steps([{"click_id": "canvas-fit"}])
                    ready = await owned.canvas_ready(p, None)
                    p.save("canvas-before", ready)
                    baseline = {"document": owned.profiling.digest(owned.FILE),
                                "ledger": owned.profiling.digest(owned.LEDGER)}
                    p.save("identity", {"kontur_head": "b3076c671ceb069dabe3b869881cb42f86a3ab34",
                        "kontur_tree": "19e9ca90c045cb01924f3487917792d34711d737",
                        "lurq_head": "779190d8153df5a57d5cbfa0e6b50f21f0fb2b23",
                        "lurq_tree": "9ae7c26940cfc59f044a47b60aebd62830068693",
                        "binary_sha256": owned.profiling.digest(owned.BINARY),
                        "launcher": "configured .env.desktop / dev:desktop; build-only adaptation",
                        "observed_build": availability["build"], "normal_tool_deadline_s": 15,
                        "workload": "Genuine18 / 01 Editor / one zoom-out then zoom-in pair at fit",
                        "new_fields": ["layout_compute", "component_after_layout"], "fixture": baseline})
                    if MODE == "canvas-text":
                        p.save("identity", {**json.loads((owned.HERE / "canvas-text-build-identity.json").read_text()),
                            "observed_build": availability["build"], "fixture": baseline,
                            "normal_tool_deadline_s": 15,
                            "workload": "Genuine18 / 01 Editor / one zoom-out then zoom-in pair at fit"})
                    first = await p.start(120)
                    trigger = asyncio.create_task(p.steps([
                        {"click_id": "canvas-zoom-out"}, {"click_id": "canvas-zoom-in"}]))
                    deadline = time.perf_counter() + 5
                    index, ended_second = 0, None
                    while not trigger.done() and time.perf_counter() < deadline:
                        live = await p.profile("lurq_profile_read", {"id": first})
                        p.save(f"phase-observation-{index:03d}", live)
                        index += 1
                        if ended_second is None and any(row["phase"] in (
                                "layout_compute", "component_after_layout")
                                and row["phase_elapsed_so_far_ms"] >= 100 for row in live["in_flight"]):
                            second = await p.start(64)
                            before = time.perf_counter()
                            ended_second = await p.end(second)
                            frozen = owned.profiling.stable(ended_second)
                            p.save("end2", ended_second)
                            p.save("end2-latency", {"elapsed_ms": (time.perf_counter() - before) * 1000})
                        await asyncio.sleep(0.1)
                    await trigger
                    ended_first = await p.end(first)
                    p.save("end1", ended_first)
                    assert not ended_first["truncated"]
                    p.save("canvas-stages", owned.profiling.stages(ended_first))
                    after = await owned.canvas_ready(p, None)
                    p.save("canvas-after", after)
                    assert ready["zoom"] == after["zoom"], (ready["zoom"], after["zoom"])
                    assert baseline == {"document": owned.profiling.digest(owned.FILE),
                                        "ledger": owned.profiling.digest(owned.LEDGER)}
                    passes = [row["data"] for row in ended_first["samples"] if row["data"]["kind"] == "pass"]
                    assert any(row["cpu_timings_ms"]["layout_compute"] > 0 for row in passes)
                    assert any(row["cpu_timings_ms"]["component_after_layout"] > 0 for row in passes)
                    longest = max(passes, key=lambda row: row["cpu_timings_ms"]["total"])
                    if MODE == "canvas-text":
                        assert all(row["canvas_text"] is not None for row in passes)
                        assert any(row["canvas_text"]["counts"]["shape_calls"] > 0 for row in passes)
                        p.save("top-cost-passes", sorted(
                            (row for row in ended_first["samples"] if row["data"]["kind"] == "pass"),
                            key=lambda row: row["data"]["cpu_timings_ms"]["total"], reverse=True)[:5])
                    if ended_second is not None:
                        assert frozen == owned.profiling.stable(ended_second)
                    p.save("summary", {"completed_samples": ended_first["completed_samples"],
                        "returned_samples": ended_first["returned_samples"], "dropped_samples": ended_first["dropped_samples"],
                        "boundary_excluded_samples": ended_first["boundary_excluded_samples"],
                        "longest_pass": longest, "end2_immutable": ended_second is not None,
                        "pass_records": len(passes), "viewport_before": ready, "viewport_after": after,
                        "work_id": {"available": False, "reason": "current Canvas semantic state does not expose Work.work_id"},
                        "end2_check_scope": "retained Python response stability; no later server reread",
                        "gpu_time_unavailable": True, "fixture_preserved": baseline,
                        "causality": "in-flight start bounds remain explicit; no ten-second/release performance claim"})
                    print(json.dumps({"stage": "refined-ready-pair-captured", "pid": app.pid}), flush=True)
                finally:
                    await p.cleanup()
                    p.save("tool-latencies", p.latencies)
                    process.close()


if __name__ == "__main__":
    owned.journey = journey
    owned.main()
