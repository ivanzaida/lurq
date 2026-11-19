#!/usr/bin/env python3
"""Measures the gallery as a page: its size, its first frame, and its frames while heavy pages scroll.

    python scripts/gallery-web.py                       build the page first
    python scripts/gallery-web-bench.py                 measure target/gallery-web
    python scripts/gallery-web-bench.py --dir target/size-exp/Oz --loads 5

It serves the page's directory on a port of its own, starts a headless Chrome with a profile of its own and
closes both when it is done (scripts/gallery-web-sweep.py's). It prints, and writes as JSON with `--json`:

  wasm        the module's bytes, and gzipped (what a server that compresses sends)
  first_frame the page's `lurq:first-frame` mark, in ms from the navigation, for each load
  scroll      on each heavy page, 80 turns of the wheel 25 ms apart: the frames' mean and 95th percentile
              gaps (16.7 ms is 60 frames a second) and how many frames took longer than 50 ms
"""
import argparse
import gzip
import importlib.util
import json
import os
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
spec = importlib.util.spec_from_file_location("sweep", os.path.join(ROOT, "scripts", "gallery-web-sweep.py"))
sweep = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sweep)

HEAVY = ["/data-grid", "/table", "/engine/virtualized-list", "/markdown", "/line-chart", "/icons"]

SCROLL = """
(async (route) => {
  location.hash = '#' + route;
  await new Promise(r => setTimeout(r, 1500));
  const c = document.getElementById('lurq');
  const r = c.getBoundingClientRect();
  const x = r.left + r.width * 0.6, y = r.top + r.height * 0.6;
  const stamps = [];
  let on = true;
  const tick = t => { stamps.push(t); if (on) requestAnimationFrame(tick); };
  requestAnimationFrame(tick);
  const long = [];
  const observer = new PerformanceObserver(l => { for (const e of l.getEntries()) long.push(e.duration); });
  observer.observe({ type: 'long-animation-frame' });
  for (let i = 0; i < 80; i++) {
    c.dispatchEvent(new WheelEvent('wheel', { deltaY: i < 40 ? 120 : -120, deltaMode: 0, clientX: x, clientY: y,
                                              bubbles: true, cancelable: true }));
    await new Promise(r => setTimeout(r, 25));
  }
  await new Promise(r => setTimeout(r, 500));
  on = false;
  observer.disconnect();
  const gaps = stamps.slice(1).map((t, i) => t - stamps[i]).sort((a, b) => a - b);
  if (!gaps.length) return null;
  return { frames: gaps.length, mean: gaps.reduce((a, b) => a + b, 0) / gaps.length,
           p95: gaps[Math.floor(gaps.length * 0.95)], long: long.length };
})(%s)
"""


def first_frame(browser, url):
  browser.call("Page.navigate", url=url)
  until = time.time() + 60
  while time.time() < until:
    browser.pump(0.2)
    mark = browser.evaluate(
      "(() => { const m = performance.getEntriesByName('lurq:first-frame')[0]; return m ? m.startTime : null; })()")
    if mark is not None:
      return round(mark)
  return None


def main():
  parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
  parser.add_argument("--dir", default=os.path.join(ROOT, "target", "gallery-web"))
  parser.add_argument("--backend", default="webgpu")
  parser.add_argument("--loads", type=int, default=3)
  parser.add_argument("--routes", nargs="*", default=HEAVY)
  parser.add_argument("--scale", type=float, default=1.5)
  parser.add_argument("--json")
  args = parser.parse_args()
  sweep.PAGE_DIR = os.path.abspath(args.dir)
  wasm = open(os.path.join(sweep.PAGE_DIR, "gallery_bg.wasm"), "rb").read()
  result = {"wasm": {"bytes": len(wasm), "gzip": len(gzip.compress(wasm, 9))}, "first_frame": [], "scroll": {}}
  port = sweep.free_port()
  server = sweep.serve(port)
  browser = sweep.Browser(sweep.BROWSERS["chrome"], os.path.join(ROOT, "target", "web-bench-profile"), 1280, 800,
                          args.scale)
  try:
    for _ in range(args.loads):
      # A new address each time: nothing of the last load is reused but what the browser caches.
      url = "http://127.0.0.1:%d/index.html?backend=%s&n=%f#/" % (port, args.backend, time.time())
      result["first_frame"].append(first_frame(browser, url))
    for route in args.routes:
      result["scroll"][route] = browser.evaluate(SCROLL % json.dumps(route))
  finally:
    browser.close()
    server.shutdown()
  mb = lambda n: "%.1f MB" % (n / 1e6)
  print("wasm %s, gzip %s" % (mb(result["wasm"]["bytes"]), mb(result["wasm"]["gzip"])))
  print("first frame (ms):", result["first_frame"])
  for route, scroll in result["scroll"].items():
    if scroll:
      print("%-26s frames %3d  mean %5.1f ms  p95 %5.1f ms  long %d" % (route, scroll["frames"], scroll["mean"],
                                                                        scroll["p95"], scroll["long"]))
    else:
      print("%-26s no frames" % route)
  if args.json:
    with open(args.json, "w", encoding="utf-8") as file:
      json.dump(result, file, indent=1)


if __name__ == "__main__":
  sys.exit(main())
