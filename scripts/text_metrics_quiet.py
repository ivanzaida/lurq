"""Optional Windows compiler-activity guard for saved text benchmark binaries.

Only the benchmark child is stopped on interference. Other processes are read
through a Toolhelp snapshot and are never modified. This does not detect all
desktop load; it prevents known compiler/linker contention from entering samples.
"""

import ctypes
import json
import os
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path


class ProcessEntry(ctypes.Structure):
    # PROCESSENTRY32W, matching the Windows SDK / windows crate Toolhelp binding.
    _fields_ = [("size", ctypes.c_uint32), ("usage", ctypes.c_uint32),
                ("pid", ctypes.c_uint32), ("heap", ctypes.c_size_t),
                ("module", ctypes.c_uint32), ("threads", ctypes.c_uint32),
                ("parent", ctypes.c_uint32), ("priority", ctypes.c_int32),
                ("flags", ctypes.c_uint32), ("name", ctypes.c_wchar * 260)]


def compiler_names():
    if os.name != "nt":
        raise RuntimeError("--quiet-compilers currently requires Windows")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateToolhelp32Snapshot.argtypes = [ctypes.c_uint32, ctypes.c_uint32]
    kernel.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
    kernel.Process32FirstW.argtypes = [ctypes.c_void_p, ctypes.POINTER(ProcessEntry)]
    kernel.Process32NextW.argtypes = [ctypes.c_void_p, ctypes.POINTER(ProcessEntry)]
    kernel.CloseHandle.argtypes = [ctypes.c_void_p]
    handle = kernel.CreateToolhelp32Snapshot(2, 0)  # TH32CS_SNAPPROCESS
    if handle in (None, ctypes.c_void_p(-1).value):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        entry = ProcessEntry()
        entry.size = ctypes.sizeof(entry)
        if not kernel.Process32FirstW(handle, ctypes.byref(entry)):
            raise ctypes.WinError(ctypes.get_last_error())
        found = set()
        while True:
            name = entry.name.lower()
            if name in {"rustc.exe", "link.exe", "lld-link.exe", "cl.exe"}:
                found.add(name)
            if not kernel.Process32NextW(handle, ctypes.byref(entry)):
                error = ctypes.get_last_error()
                if error != 18:  # ERROR_NO_MORE_FILES
                    raise ctypes.WinError(error)
                break
        return sorted(found)
    finally:
        kernel.CloseHandle(handle)


def run_quiet(binary, arguments, log, env, *, settle_timeout=600):
    log = Path(log)
    attempts = []
    deadline = time.monotonic() + settle_timeout
    for attempt in range(1, 9):
        idle_since = None
        announced = False
        while idle_since is None or time.monotonic() - idle_since < 1.5:
            if time.monotonic() > deadline:
                raise TimeoutError(f"compiler activity did not settle within {settle_timeout:g} seconds")
            active = compiler_names()
            if active:
                idle_since = None
                if not announced:
                    print(f"Waiting for compiler activity to settle: {', '.join(active)}", flush=True)
                    announced = True
            elif idle_since is None:
                idle_since = time.monotonic()
            time.sleep(0.25)

        started = time.monotonic()
        polls = 0
        active = []
        with log.open("w", encoding="utf-8") as output:
            process = subprocess.Popen([str(binary), *map(str, arguments)], env=env, stdout=output, stderr=output)
            try:
                while process.poll() is None:
                    active = compiler_names()
                    polls += 1
                    if active:
                        process.terminate()
                        break
                    if time.monotonic() - started > 180:
                        raise subprocess.TimeoutExpired(process.args, 180)
                    time.sleep(0.25)
                process.wait(timeout=10)
                if not active:
                    active = compiler_names()
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=10)
        attempts.append(dict(attempt=attempt, seconds=time.monotonic() - started,
                             polls=polls, rejected_compilers=active))
        audit = dict(timestamp_utc=datetime.now(timezone.utc).isoformat(), attempts=attempts,
                     poll_interval_seconds=0.25, accepted=not active and process.returncode == 0)
        log.with_suffix(".quiet.json").write_text(json.dumps(audit, indent=2), encoding="utf-8")
        if not active:
            if process.returncode:
                raise subprocess.CalledProcessError(process.returncode, process.args)
            return
        # Preserve rejected output next to this run's own files before retrying.
        for path in (log, log.with_suffix(".csv")):
            if path.exists():
                rejected = path.with_name(f"{path.stem}.discarded-{attempt}{path.suffix}")
                assert rejected.resolve().parent == path.resolve().parent
                path.replace(rejected)
        print(f"Discarded benchmark attempt {attempt}: {', '.join(active)}; retrying", flush=True)
    raise RuntimeError("eight benchmark attempts overlapped compiler activity")
