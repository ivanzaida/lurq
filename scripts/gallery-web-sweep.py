#!/usr/bin/env python3
"""Walks every page of the gallery in a browser and lists what goes wrong there.

    python scripts/gallery-web.py                        build the page first
    python scripts/gallery-web-sweep.py                  every page, WebGPU and WebGL2
    python scripts/gallery-web-sweep.py --backends webgl --routes /button /table

It serves `target/gallery-web` on a port of its own, starts a headless Chrome (or Edge, `--browser`) with a
profile of its own under `target/web-sweep`, drives it over the DevTools protocol and closes both when it is
done: no window is shown and no key or pointer of the system is touched. For each page it goes to
`index.html?backend=<b>#<route>` by the address's fragment, waits for the page to settle and writes:

  shots/<backend>/<name>.png   the page as the browser drew it, at the size and scale given
  sweep-<backend>.json         per page: console errors and warnings, a panic, whether anything was drawn
                               where the page should be, the time the page's thread was busy (long tasks)

A panic stops the app: the page is loaded again for the next route.
"""
import argparse
import base64
import functools
import http.server
import io
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import threading
import time
import urllib.request

import websocket  # websocket-client

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAGE_DIR = os.path.join(ROOT, "target", "gallery-web")
BROWSERS = {
  "chrome": r"C:\Program Files\Google\Chrome\Application\chrome.exe",
  "edge": r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
}

# Watches the page's thread from the start of the page: what held it, and for how long.
PROBE = """
(() => {
  window.__sweep = { long: [], errors: [] };
  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) window.__sweep.long.push([entry.startTime, entry.duration]);
    }).observe({ type: "longtask", buffered: true });
  } catch (e) {}
  window.addEventListener("error", (e) => window.__sweep.errors.push(String(e.message)));
  window.addEventListener("unhandledrejection", (e) => window.__sweep.errors.push("rejection: " + String(e.reason)));
})();
"""


def routes_of_the_gallery():
  source = open(os.path.join(ROOT, "crates", "gallery", "src", "pages", "mod.rs"), encoding="utf-8").read()
  return re.findall(r'^\s*path: "(/[^"]*)",', source, re.M)


def free_port():
  with socket.socket() as probe:
    probe.bind(("127.0.0.1", 0))
    return probe.getsockname()[1]


def serve(port):
  handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=PAGE_DIR)
  handler.extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm"}
  handler.log_message = lambda *args: None
  server = http.server.ThreadingHTTPServer(("127.0.0.1", port), handler)
  threading.Thread(target=server.serve_forever, daemon=True).start()
  return server


class Browser:
  def __init__(self, exe, profile, width, height, scale, headless=True):
    self.port = free_port()
    # An absolute path: a relative one is taken for the default profile, and the running Chrome gets the page.
    profile = os.path.abspath(profile)
    shutil.rmtree(profile, ignore_errors=True)
    arguments = [
      exe,
      "--remote-debugging-port=%d" % self.port,
      "--user-data-dir=" + profile,
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-extensions",
      "--disable-background-timer-throttling",
      "--disable-renderer-backgrounding",
      "--disable-backgrounding-occluded-windows",
      "--enable-unsafe-webgpu",
      "--window-size=%d,%d" % (width, height),
      "about:blank",
    ]
    if headless:
      arguments.insert(1, "--headless=new")
    # What the browser says, for when it does not start.
    self.log = open(profile + ".log", "w")
    self.process = subprocess.Popen(arguments, stdout=self.log, stderr=subprocess.STDOUT)
    until = time.time() + 90
    targets = None
    while time.time() < until:
      try:
        with urllib.request.urlopen("http://127.0.0.1:%d/json" % self.port, timeout=2) as answer:
          targets = [t for t in json.loads(answer.read()) if t.get("type") == "page"]
        if targets:
          break
      except OSError:
        time.sleep(0.2)
    if not targets:
      self.close()
      raise RuntimeError("the browser did not open its DevTools port")
    self.socket = websocket.create_connection(targets[0]["webSocketDebuggerUrl"], timeout=60,
                                              suppress_origin=True)
    self.count = 0
    self.events = []
    self.call("Page.enable")
    self.call("Runtime.enable")
    self.call("Log.enable")
    self.call("Emulation.setDeviceMetricsOverride", width=width, height=height, deviceScaleFactor=scale,
              mobile=False)
    self.call("Page.addScriptToEvaluateOnNewDocument", source=PROBE)

  def call(self, method, **params):
    self.count += 1
    ident = self.count
    self.socket.send(json.dumps({"id": ident, "method": method, "params": params}))
    while True:
      message = json.loads(self.socket.recv())
      if message.get("id") == ident:
        if "error" in message:
          raise RuntimeError("%s: %s" % (method, message["error"]))
        return message.get("result", {})
      if "method" in message:
        self.events.append(message)

  def pump(self, seconds):
    """Takes the events that come in `seconds`."""
    until = time.time() + seconds
    while True:
      left = until - time.time()
      if left <= 0:
        return
      self.socket.settimeout(left)
      try:
        message = json.loads(self.socket.recv())
      except (websocket.WebSocketTimeoutException, socket.timeout):
        return
      finally:
        self.socket.settimeout(60)
      if "method" in message:
        self.events.append(message)

  def evaluate(self, expression):
    result = self.call("Runtime.evaluate", expression=expression, returnByValue=True, awaitPromise=True)
    return result.get("result", {}).get("value")

  def take_console(self):
    """The console's errors and warnings since the last call, and its other lines."""
    said = []
    for event in self.events:
      method, params = event["method"], event.get("params", {})
      if method == "Runtime.consoleAPICalled":
        text = " ".join(str(arg.get("value", arg.get("description", ""))) for arg in params.get("args", []))
        said.append((params.get("type"), text))
      elif method == "Runtime.exceptionThrown":
        details = params.get("exceptionDetails", {})
        said.append(("exception", details.get("exception", {}).get("description") or details.get("text")))
      elif method == "Log.entryAdded":
        entry = params.get("entry", {})
        said.append((entry.get("level"), "%s %s" % (entry.get("text"), entry.get("url", ""))))
    self.events = []
    return said

  def shot(self):
    data = self.call("Page.captureScreenshot", format="png")["data"]
    return base64.b64decode(data)

  def close(self):
    try:
      self.socket.close()
    except Exception:
      pass
    if self.process.poll() is None:
      # Only this browser: its own process tree, by its id.
      subprocess.run(["taskkill", "/PID", str(self.process.pid), "/T", "/F"], capture_output=True)
      self.process.wait(10)


def drawn_where_the_page_is(png, scale):
  """How many colors the page's part of the picture has: one means nothing was drawn there."""
  from PIL import Image
  picture = Image.open(io.BytesIO(png)).convert("RGB")
  width, height = picture.size
  # Right of the side bar, below the top bar.
  box = (int(300 * scale), int(80 * scale), width - int(10 * scale), height - int(10 * scale))
  part = picture.crop(box).resize((200, 120))
  colors = part.getcolors(200 * 120) or []
  return len(colors)


def name_of(route):
  return "overview" if route == "/" else route.strip("/").replace("/", "_")


def page_state(browser):
  return browser.evaluate(
    "(() => { const s = document.getElementById('status');"
    " return { status: s ? s.textContent : null, long: window.__sweep ? window.__sweep.long.slice() : [],"
    " errors: window.__sweep ? window.__sweep.errors.slice() : [], now: performance.now() }; })()"
  )


def load(browser, base, backend, route, wait):
  url = "%s/index.html?backend=%s#%s" % (base, backend, route)
  started = time.time()
  browser.call("Page.navigate", url=url)
  # The page says "Loading" until the device and the fonts came.
  until = time.time() + 120
  while time.time() < until:
    browser.pump(0.25)
    state = page_state(browser) or {}
    if state.get("status") is None or "stopped" in (state.get("status") or "") or "neither" in (
        state.get("status") or "") or "did not load" in (state.get("status") or ""):
      break
  browser.pump(wait)
  return time.time() - started


def sweep(browser, base, backend, routes, out, wait, scale, reload_each):
  shots = os.path.join(out, "shots", backend)
  os.makedirs(shots, exist_ok=True)
  results = []
  first = load(browser, base, backend, routes[0], wait)
  boot_console = browser.take_console()
  print("[%s] loaded in %.1f s" % (backend, first), flush=True)
  loaded_now = True
  for index, route in enumerate(routes):
    if not loaded_now:
      if reload_each:
        load(browser, base, backend, route, wait)
      else:
        before = page_state(browser) or {}
        mark = before.get("now", 0)
        browser.evaluate("location.hash = %s" % json.dumps("#" + route))
        browser.pump(wait)
    else:
      mark = 0
    loaded_now = False
    state = page_state(browser) or {}
    console = browser.take_console()
    if index == 0:
      console = boot_console + console
    png = browser.shot()
    with open(os.path.join(shots, name_of(route) + ".png"), "wb") as file:
      file.write(png)
    busy = [duration for start, duration in state.get("long", []) if start >= (mark or 0)]
    status = state.get("status")
    panic = status if status and "stopped" in status else None
    entry = {
      "route": route,
      "errors": [text for kind, text in console if kind in ("error", "exception", "assert")],
      "warnings": [text for kind, text in console if kind in ("warning", "warn")],
      "logs": [text for kind, text in console if kind not in ("error", "exception", "assert", "warning", "warn")],
      "page_errors": state.get("errors", []),
      "panic": panic,
      "colors": drawn_where_the_page_is(png, scale),
      "busy_ms": round(sum(busy)),
      "longest_ms": round(max(busy, default=0)),
    }
    results.append(entry)
    flag = []
    if entry["errors"]:
      flag.append("%d errors" % len(entry["errors"]))
    if panic:
      flag.append("PANIC")
    if entry["colors"] <= 2:
      flag.append("blank")
    if entry["longest_ms"] > 200:
      flag.append("long %d ms" % entry["longest_ms"])
    print("[%s] %3d/%d %-28s %s" % (backend, index + 1, len(routes), route, ", ".join(flag) or "ok"), flush=True)
    if panic:
      # The app has stopped: the next page starts it again.
      if index + 1 < len(routes):
        load(browser, base, backend, routes[index + 1], wait)
        loaded_now = True
        browser.take_console()
  with open(os.path.join(out, "sweep-%s.json" % backend), "w", encoding="utf-8") as file:
    json.dump({"first_load_s": round(first, 2), "pages": results}, file, indent=1)
  return results


def main():
  parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
  parser.add_argument("--browser", default="chrome", choices=sorted(BROWSERS))
  parser.add_argument("--backends", nargs="+", default=["webgpu", "webgl"])
  parser.add_argument("--routes", nargs="+")
  parser.add_argument("--out", default=os.path.join(ROOT, "target", "web-sweep"))
  parser.add_argument("--width", type=int, default=1440)
  parser.add_argument("--height", type=int, default=900)
  parser.add_argument("--scale", type=float, default=1.5, help="the display's pixel ratio")
  parser.add_argument("--wait", type=float, default=2.0, help="seconds a page is given to settle")
  parser.add_argument("--reload-each", action="store_true", help="load the page anew for every route")
  parser.add_argument("--headed", action="store_true", help="a window, not headless (shown on the screen!)")
  args = parser.parse_args()
  routes = args.routes or routes_of_the_gallery()
  os.makedirs(args.out, exist_ok=True)
  port = free_port()
  server = serve(port)
  base = "http://127.0.0.1:%d" % port
  try:
    for backend in args.backends:
      browser = Browser(BROWSERS[args.browser], os.path.join(args.out, "profile-" + args.browser), args.width,
                        args.height, args.scale, headless=not args.headed)
      try:
        sweep(browser, base, backend, routes, args.out, args.wait, args.scale, args.reload_each)
      finally:
        browser.close()
  finally:
    server.shutdown()


if __name__ == "__main__":
  sys.exit(main())
