"""Exercise the lifecycle demo through MCP and real WM_CLOSE messages.

Build: cargo build -p demo --bin lifecycle --features mcp
Run: python scripts/windows-lifecycle-check.py <path-to-lifecycle.exe> <evidence-dir>
Only windows belonging to the process launched by this script are addressed.
"""
import base64
import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.request


def wait_for(check, seconds=10):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        value = check()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError("condition did not become true")


class Client:
    def __init__(self, config):
        self.url = f"http://127.0.0.1:{config['port']}/mcp"
        self.headers = {"Authorization": "Bearer " + config["token"],
                        "Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
        self.index = 0
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": {"name": "lurq-lifecycle-check", "version": "1"}})
        self.rpc("notifications/initialized", {}, notification=True)

    def rpc(self, method, params, notification=False):
        self.index += 1
        body = {"jsonrpc": "2.0", "method": method, "params": params}
        if not notification:
            body["id"] = self.index
        request = urllib.request.Request(self.url, json.dumps(body).encode(), self.headers)
        with urllib.request.urlopen(request, timeout=15) as response:
            if response.headers.get("Mcp-Session-Id"):
                self.headers["Mcp-Session-Id"] = response.headers["Mcp-Session-Id"]
            raw = response.read().decode()
            if "text/event-stream" in response.headers.get("Content-Type", ""):
                messages = [json.loads(line[5:].strip()) for line in raw.splitlines()
                            if line.startswith("data:") and line[5:].strip()]
                result = messages[-1] if messages else None
            else:
                result = json.loads(raw) if raw.strip() else None
        if notification:
            return
        assert result and "error" not in result, result
        return result["result"]

    def tool(self, name, **args):
        result = self.rpc("tools/call", {"name": name, "arguments": args})
        assert not result.get("isError"), result
        return result["content"]

    def text(self, name, **args):
        return "\n".join(c["text"] for c in self.tool(name, **args) if c["type"] == "text")

    def json(self, name, **args):
        return json.loads(self.text(name, **args))

    def ref(self, element):
        text = self.text("lurq_find_by_id", id=element)
        match = re.search(r"ref_\d+", text)
        assert match, text
        return match.group()

    def click(self, element):
        self.json("lurq_interact", action="click", ref=self.ref(element))

    def screenshot(self, path):
        block = next(c for c in self.tool("lurq_screenshot") if c["type"] == "image")
        path.write_bytes(base64.b64decode(block["data"]))


def native_close(pid, title):
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    user32.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
    user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
    matches = []

    @callback_type
    def visit(hwnd, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        label = ctypes.create_unicode_buffer(512)
        user32.GetWindowTextW(hwnd, label, 512)
        if owner.value == pid and label.value == title:
            matches.append(hwnd)
        return True

    user32.EnumWindows(visit, 0)
    assert len(matches) == 1, (pid, title, matches)
    assert user32.PostMessageW(matches[0], 0x0010, 0, 0), ctypes.get_last_error()


def run(binary, evidence):
    evidence.mkdir(parents=True, exist_ok=True)
    for dirty in (True, False):
        with (evidence / f"demo-{dirty}.log").open("w") as log:
            environment = dict(os.environ, RUST_LOG="video=trace")
            environment.pop("LURQ_VIDEO_LOGS", None)
            process = subprocess.Popen([str(binary)], stdout=log, stderr=log, env=environment)
            discovery = Path(os.environ["LOCALAPPDATA"]) / f"lurq/mcp/{process.pid}.json"
            try:
                wait_for(discovery.exists)
                client = Client(json.loads(discovery.read_text()))
                wait_for(lambda: "dirty" in client.text("lurq_read_tree"))
                if dirty:
                    assert not client.json("lurq_interact", action="menu_activate", id="save")["activated"]
                    client.json("lurq_set_value", ref=client.ref("dirty"), value=True)
                    native_close(process.pid, "lurq lifecycle")
                    wait_for(lambda: "dialog-confirm" in client.text("lurq_read_tree"))
                    assert process.poll() is None
                    client.screenshot(evidence / "dirty-close-dialog.png")
                    client.click("dialog-cancel")
                    wait_for(lambda: "dialog-confirm" not in client.text("lurq_read_tree"))
                    reply = client.json("lurq_interact", action="request_close")
                    assert reply["stayed_open"] and not reply["close_queued"], reply
                    wait_for(lambda: "dialog-confirm" in client.text("lurq_read_tree"))
                    client.click("dialog-cancel")
                    client.click("open-preferences")
                    wait_for(lambda: len(client.json("lurq_windows")["windows"]) == 2)
                    native_close(process.pid, "Preferences")
                    wait_for(lambda: len(client.json("lurq_windows")["windows"]) == 1)
                    assert client.json("lurq_interact", action="menu_activate", id="save")["activated"]
                    wait_for(lambda: "checked=false" in client.text("lurq_read_tree"))
                    client.json("lurq_set_value", ref=client.ref("dirty"), value=True)
                    client.json("lurq_interact", action="request_close")
                    wait_for(lambda: "dialog-confirm" in client.text("lurq_read_tree"))
                    client.click("dialog-confirm")
                else:
                    native_close(process.pid, "lurq lifecycle")
                assert process.wait(timeout=10) == 0
                assert not discovery.exists(), "clean shutdown should remove MCP discovery"
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=5)
        output = (evidence / f"demo-{dirty}.log").read_text()
        assert "[video:timeline]" not in output and "video::watch::lurq" not in output, output
        print("PASS", "dirty veto, cancel, MCP, secondary, menu and confirm" if dirty else "clean native close", flush=True)


if __name__ == "__main__":
    if sys.platform != "win32":
        raise SystemExit("This check requires Windows; use native_macos on macOS")
    run(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
