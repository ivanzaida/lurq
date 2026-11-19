#!/usr/bin/env python3
"""Pictures of every page of the desktop gallery, to compare with the browser's (scripts/gallery-web-sweep.py).

    cargo build -p gallery --features mcp
    python scripts/gallery-desktop-shots.py --exe target/debug/gallery-websweep.exe

It starts the gallery it is given at 1280 by 800 (the --width and --height), drives it over lurq's MCP server (no pointer or key of the
system is touched), writes `target/web-sweep/shots/desktop/<name>.png` for each page and kills that one
process when it is done.
"""
import argparse
import importlib.util
import os
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
spec = importlib.util.spec_from_file_location("gallery_narrow", os.path.join(ROOT, "scripts", "gallery-narrow.py"))
narrow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(narrow)


def name_of(route):
  return "overview" if route == "/" else route.strip("/").replace("/", "_")


def main():
  parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
  parser.add_argument("--exe", required=True)
  parser.add_argument("--routes", nargs="+")
  parser.add_argument("--out", default=os.path.join(ROOT, "target", "web-sweep", "shots", "desktop"))
  parser.add_argument("--wait", type=float, default=1.0)
  parser.add_argument("--width", type=int, default=1280)
  parser.add_argument("--height", type=int, default=800)
  args = parser.parse_args()
  routes = args.routes or narrow.routes_of_the_gallery()
  os.makedirs(args.out, exist_ok=True)
  gallery = narrow.Gallery(os.path.abspath(args.exe), args.width, args.height)
  try:
    for index, route in enumerate(routes):
      gallery.client.tool("lurq_navigate", path=route)
      gallery.settle(4)
      time.sleep(args.wait)
      gallery.client.picture().save(os.path.join(args.out, name_of(route) + ".png"))
      print("%3d/%d %s" % (index + 1, len(routes), route), flush=True)
  except Exception:
    sys.stderr.write("the gallery said:\n" + "".join(gallery.lines)[-3000:])
    raise
  finally:
    gallery.close()


if __name__ == "__main__":
  sys.exit(main())
