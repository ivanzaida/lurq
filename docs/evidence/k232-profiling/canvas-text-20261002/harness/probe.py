"""Finite SDK profiling probe. Attach only; never build, launch or stop an app."""
import argparse
import asyncio
import copy
import ctypes
import hashlib
import importlib.metadata
import json
import logging
import re
import statistics
import time
from datetime import timedelta
from pathlib import Path
from urllib.parse import urlparse

import httpx
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client

ROOT = Path(__file__).resolve().parent
PROFILE_TOOLS = {"lurq_profile_start", "lurq_profile_read", "lurq_profile_end"}


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def stable(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


class Process:
    """Independent whole-process CPU clock; includes server and rendering threads."""
    def __init__(self, pid, binary):
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.kernel.OpenProcess.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
        self.kernel.OpenProcess.restype = ctypes.c_void_p
        self.kernel.CloseHandle.argtypes = [ctypes.c_void_p]
        self.kernel.QueryFullProcessImageNameW.argtypes = [
            ctypes.c_void_p, ctypes.c_uint32, ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_uint32)]
        self.kernel.GetProcessTimes.argtypes = [ctypes.c_void_p] + [ctypes.c_void_p] * 4
        self.handle = self.kernel.OpenProcess(0x1000, False, pid)
        if not self.handle:
            raise RuntimeError("cannot inspect supplied owned process")
        name = ctypes.create_unicode_buffer(32768)
        size = ctypes.c_uint32(len(name))
        if not self.kernel.QueryFullProcessImageNameW(self.handle, 0, name, ctypes.byref(size)):
            self.close()
            raise RuntimeError("cannot read process image identity")
        if Path(name.value).resolve() != binary.resolve():
            self.close()
            raise RuntimeError("supplied PID does not run the supplied binary")

    def cpu(self):
        values = [ctypes.c_uint64() for _ in range(4)]
        if not self.kernel.GetProcessTimes(self.handle, *[ctypes.byref(value) for value in values]):
            raise RuntimeError("process CPU clock unavailable")
        return (values[2].value + values[3].value) / 10_000_000

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None


class Probe:
    def __init__(self, session, output, process):
        self.session, self.output, self.process = session, output, process
        self.active = set()
        self.latencies = []

    def save(self, name, value):
        (self.output / f"{name}.json").write_text(json.dumps(value, indent=2), encoding="utf-8")

    async def call(self, tool, args=None):
        before = time.perf_counter()
        result = await self.session.call_tool(tool, args or {})
        self.latencies.append({"tool": tool, "elapsed_ms": (time.perf_counter() - before) * 1000})
        if result.isError:
            raise RuntimeError(f"tool error: {tool}")
        return result

    async def profile(self, tool, args=None):
        result = await self.call(tool, args)
        if result.structuredContent is not None:
            return result.structuredContent
        return json.loads("\n".join(block.text for block in result.content if block.type == "text"))

    async def start(self, limit=240):
        value = await self.profile("lurq_profile_start", {"max_samples": limit})
        assert re.fullmatch(r"profile_[1-9][0-9]*", value["id"])
        self.active.add(value["id"])
        return value["id"]

    async def end(self, identifier):
        value = await self.profile("lurq_profile_end", {"id": identifier})
        self.active.remove(identifier)
        assert value["id"] == identifier and value["finalized"]
        assert value["returned_samples"] <= value["max_samples"] <= 240
        assert value["build"]["gpu_timestamps"]["available"] is False
        for sample in value["samples"]:
            assert value["started_ms"] <= sample["started_ms"] <= sample["completed_ms"] <= value["ended_ms"]
        return value

    async def steps(self, steps):
        for step in steps:
            if "click_id" in step:
                found = await self.call("lurq_find_by_id", {"id": step["click_id"], "window": "main"})
                text = "\n".join(block.text for block in found.content if block.type == "text")
                references = set(re.findall(r"\bref_[0-9]+\b", text))
                if len(references) != 1:
                    raise RuntimeError("expected one fresh control ref")
                await self.call("lurq_interact", {"action": "click", "ref": references.pop()})
            else:
                assert step["tool"] not in PROFILE_TOOLS
                await self.call(step["tool"], step.get("args", {}))
            if step.get("barrier", True):
                await self.call("lurq_wait", {"window": "main", "frames": 1, "timeout_ms": 14000})

    async def batch(self, manifest):
        for _ in range(manifest["cycles"]):
            await self.steps(manifest["cycle"])

    async def cleanup(self):
        for identifier in tuple(self.active):
            try:
                self.save(f"failure-ended-{identifier}", await self.end(identifier))
            except Exception:
                self.save(f"failure-end-{identifier}", {"status": "end_failed"})


def stages(report):
    rendered = [sample["data"] for sample in report["samples"]
                if sample["data"]["kind"] == "pass" and sample["data"]["rendered"]]
    actual = [frame for frame in rendered if frame["frame"] and
              frame["frame"]["render"]["canvas"]["counts"]["batches"] > 0]
    assert actual, "capture contains no actual Canvas batch"
    for frame in actual:
        assert frame["backend"] == "dx12"
        assert frame["frame"]["render"]["profile_available"]
        assert frame["frame"]["render"]["coverage"]["canvas_cpu"]
        assert frame["gpu_timing_ms"] is None
    totals = {key: sum(frame["frame"]["render"]["canvas"]["cpu_timings_ms"][key]
                      for frame in actual) for key in
              ("total", "tessellation", "asset_upload", "buffer_upload", "recording", "submit")}
    assert totals["total"] > 0 and totals["tessellation"] > 0 and totals["recording"] > 0
    return {"rendered_passes": len(rendered), "canvas_passes": len(actual),
            "canvas_cpu_ms": totals, "inclusive_scopes_not_additive": True,
            "uploaded_asset_bytes": sum(frame["frame"]["render"]["canvas"]["counts"]["uploaded_asset_bytes"]
                                        for frame in actual)}


async def overlap(probe, manifest):
    await probe.steps(manifest["reset"])
    first = await probe.start()
    await probe.steps(manifest["cycle"])
    second = await probe.start()
    await probe.steps(manifest["cycle"])
    ended_second = await probe.end(second)
    frozen = copy.deepcopy(ended_second)
    probe.save("overlap-end2", ended_second)
    await probe.steps(manifest["cycle"])
    ended_first = await probe.end(first)
    probe.save("overlap-end1", ended_first)
    assert first != second and ended_second == frozen
    assert ended_first["completed_samples"] > ended_second["completed_samples"]
    assert any(sample["completed_ms"] > ended_second["ended_ms"] for sample in ended_first["samples"])
    assert not ended_first["truncated"] and not ended_second["truncated"]
    probe.save("overlap-summary", {"id1": first, "id2": second,
               "end2_digest_unchanged": stable(ended_second) == stable(frozen),
               "active1_received_later_completion": True, "stages": stages(ended_first)})


async def overhead(probe, manifest):
    measurements = []
    for index, active in enumerate((False, True, True, False)):
        await probe.steps(manifest["reset"])
        await probe.steps(manifest["cycle"])
        identifier = await probe.start() if active else None
        cpu_start, wall_start = probe.process.cpu(), time.perf_counter()
        latency_start = len(probe.latencies)
        await probe.batch(manifest)
        # Same fixed quiescent tail in both conditions; not a GPU timing claim.
        await asyncio.sleep(0.05)
        wall_ms = (time.perf_counter() - wall_start) * 1000
        cpu_ms = (probe.process.cpu() - cpu_start) * 1000
        measured_calls = probe.latencies[latency_start:]
        row = {"segment": index, "collection_active": active, "cycles": manifest["cycles"],
               "wall_ms": wall_ms, "process_cpu_ms": cpu_ms, "calls": measured_calls}
        if identifier:
            report = await probe.end(identifier)
            probe.save(f"overhead-{index}-capture", report)
            assert not report["truncated"]
            row["stages"] = stages(report)
            row["unfinished_at_end"] = report["in_flight"]
        measurements.append(row)
        probe.save("overhead-progress", measurements)
    def grouped(key, active):
        return [row[key] for row in measurements if row["collection_active"] == active]
    summary = {"order": "off/on/on/off", "segments": measurements,
               "metric": "whole process CPU and end-to-end workload wall time; not GPU time or FPS",
               "baseline": "same binary with perf_profile compiled and no active sessions",
               "export_start_end_excluded_from_timed_window": True,
               "fixed_tail_ms": 50, "off_frame_count": "not directly observed; identical frame barriers used"}
    for key in ("wall_ms", "process_cpu_ms"):
        off, on = grouped(key, False), grouped(key, True)
        mean_off, mean_on = statistics.mean(off), statistics.mean(on)
        summary[key] = {"off_mean": mean_off, "on_mean": mean_on,
                        "difference": mean_on - mean_off,
                        "relative_percent": (mean_on / mean_off - 1) * 100 if mean_off else None,
                        "off_range": max(off) - min(off), "on_range": max(on) - min(on)}
    probe.save("overhead-summary", summary)


async def stall(probe, manifest):
    first = await probe.start(64)
    trigger = asyncio.create_task(probe.steps([manifest["stall_call"]]))
    observation = None
    deadline = time.perf_counter() + 3
    while time.perf_counter() < deadline:
        value = await probe.profile("lurq_profile_read", {"id": first})
        if any(current["phase_elapsed_so_far_ms"] >= 100 for current in value["in_flight"]):
            observation = value
            break
        await asyncio.sleep(0.05)
    if observation is not None:
        probe.save("stall-observed-before-end2", observation)
        second = await probe.start(64)
        before = time.perf_counter()
        ended_second = await probe.end(second)
        probe.save("stall-end2", ended_second)
        probe.save("stall-end2-latency", {"elapsed_ms": (time.perf_counter() - before) * 1000})
        frozen = stable(ended_second)
    await trigger
    ended_first = await probe.end(first)
    probe.save("stall-end1", ended_first)
    if observation is None:
        probe.save("stall-summary", {"status": "no_slow_in_flight_phase_observed",
                   "native_mid_stall_acceptance": "not established; no manufactured stall"})
        return
    assert stable(ended_second) == frozen
    observed_at_end = bool(ended_second["in_flight"])
    probe.save("stall-summary", {"status": "unfinished_at_end2" if observed_at_end else "completed_before_end2",
               "native_mid_stall_acceptance": observed_at_end,
               "id1": first, "id2": second, "end2_digest_unchanged": True})


async def run(args):
    manifest = json.loads(args.workload.read_text(encoding="utf-8"))
    assert 1 <= manifest["cycles"] <= 8 and 1 <= len(manifest["cycle"]) <= 4
    assert len(manifest["reset"]) <= 4
    assert "SUPPLY" not in json.dumps(manifest)
    entry = json.loads(args.discovery.read_text(encoding="utf-8"))
    url = urlparse(entry["url"])
    assert entry["pid"] == args.pid and url.hostname == "127.0.0.1" and url.scheme == "http"
    process = Process(args.pid, args.binary)
    args.output.mkdir(parents=True, exist_ok=False)
    fixture_before = digest(args.fixture)
    identity = {"workload": manifest["name"], "workload_sha256": digest(args.workload),
                "binary_sha256": digest(args.binary), "fixture_sha256": fixture_before,
                "pid": args.pid, "mcp_sdk": importlib.metadata.version("mcp"),
                "declared_build_identity": manifest["identity"], "mode": args.mode}
    logging.getLogger("httpx").setLevel(logging.ERROR)
    probe = None
    try:
        async with httpx.AsyncClient(headers={"Authorization": f"Bearer {entry['token']}"},
                                     timeout=httpx.Timeout(15, read=15)) as client:
            async with streamable_http_client(entry["url"], http_client=client) as (read, write, _):
                async with ClientSession(read, write, read_timeout_seconds=timedelta(seconds=15)) as session:
                    await session.initialize()
                    tools = await session.list_tools()
                    assert PROFILE_TOOLS <= {tool.name for tool in tools.tools}
                    probe = Probe(session, args.output, process)
                    availability = await probe.profile("lurq_profile_read")
                    assert availability["build"]["features"]["perf_profile"]
                    assert availability["build"]["features"]["dx12"]
                    identity["observed_build"] = availability["build"]
                    probe.save("identity", identity)
                    try:
                        await {"overlap": overlap, "overhead": overhead, "stall": stall}[args.mode](probe, manifest)
                    finally:
                        await probe.cleanup()
                        probe.save("tool-latencies", probe.latencies)
    except Exception as error:
        (args.output / "failure.json").write_text(json.dumps({"error_class": type(error).__name__,
             "status": "failed; inspect retained capture, do not infer readiness"}), encoding="utf-8")
        raise RuntimeError("finite profiling probe failed; sanitized failure retained") from None
    finally:
        process.close()
        (args.output / "fixture-preservation.json").write_text(json.dumps({
             "before_sha256": fixture_before, "after_sha256": digest(args.fixture)}), encoding="utf-8")
    assert fixture_before == digest(args.fixture)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    for name in ("discovery", "binary", "fixture", "workload", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--mode", choices=("overlap", "overhead", "stall"), required=True)
    arguments = parser.parse_args()
    assert arguments.output.resolve().is_relative_to(ROOT)
    asyncio.run(run(arguments))
