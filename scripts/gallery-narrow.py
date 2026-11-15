#!/usr/bin/env python3
"""Walks every page of the gallery in a narrow window and lists what does not hold there.

    cargo build -p gallery --features mcp
    python scripts/gallery-narrow.py                     # every page at 390 and 320
    python scripts/gallery-narrow.py --routes /button /table --shots target/narrow/shots
    python scripts/gallery-narrow.py --widths 390 --out target/narrow/after

It starts a gallery of its own (`target/debug/gallery.exe`), drives it over lurq's MCP server and kills that
one process when it is done; no pointer or key of the system is touched. For each page it reads the tree of
the window at the top of the page and at its end, and lists:

  past     an element whose box runs past what holds it, sideways (a row that does not wrap, a text beside
           others on one line), where nothing between them scrolls that way
  under    the same downwards: what is higher than the box it is in
  window   an element that runs past the window, or a page wider than its own scroll
  overlap  two siblings whose boxes lie over each other
  small    a control squeezed under 28 that is larger in a wide window
  shrunk   a text on one line whose box is narrower than in a wide window and than its words can be: it ends
           in an ellipsis or it is cut, which the tree does not say; each is looked at once

What a wide window (1440) has too is not listed: a badge on a corner, a ring, a thumb at the end of its
track reach past their boxes on purpose, at every width. The exit code is 1 when anything is listed.
"""
import argparse
import base64
import io
import json
import os
import re
import subprocess
import sys
import threading
import time
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOLERANCE = 1.5
LEAST_CONTROL = 28.0
WIDE = 1440
# No higher than this, a text is on one line.
ONE_LINE = 26.0
# The least a letter is wide, of the height of its line: what a text of so many letters cannot be narrower than.
NARROWEST_GLYPH = 0.28
KINDS = ["past", "under", "window", "overlap", "small", "shrunk"]


class Client:
  def __init__(self, port, token):
    self.url, self.token, self.session, self.count = "http://127.0.0.1:%s/mcp" % port, token, None, 0
    self.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                             "clientInfo": {"name": "gallery-narrow", "version": "0"}})
    self.post({"jsonrpc": "2.0", "method": "notifications/initialized"}, answer=False)

  def post(self, payload, answer=True):
    headers = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream",
               "MCP-Protocol-Version": "2025-06-18", "Authorization": "Bearer " + self.token}
    if self.session:
      headers["Mcp-Session-Id"] = self.session
    request = urllib.request.Request(self.url, json.dumps(payload).encode(), headers)
    with urllib.request.urlopen(request, timeout=120) as response:
      self.session = response.headers.get("Mcp-Session-Id") or self.session
      body = response.read().decode("utf-8")
    if not answer:
      return None
    if body.lstrip().startswith("{"):
      return json.loads(body)
    for line in body.splitlines():
      if line.startswith("data:") and line[5:].strip():
        message = json.loads(line[5:])
        if message.get("id") == payload.get("id"):
          return message
    raise RuntimeError(body[:500])

  def call(self, method, params=None):
    self.count += 1
    message = self.post({"jsonrpc": "2.0", "id": self.count, "method": method, "params": params or {}})
    if "error" in message:
      raise RuntimeError(json.dumps(message["error"]))
    return message["result"]

  def tool(self, name, **arguments):
    result = self.call("tools/call", {"name": name, "arguments": arguments})
    text = "\n".join(part["text"] for part in result.get("content", []) if part.get("type") == "text")
    if result.get("isError"):
      raise RuntimeError("%s: %s" % (name, text[:300]))
    return text

  def picture(self):
    from PIL import Image
    result = self.call("tools/call", {"name": "lurq_screenshot", "arguments": {}})
    for part in result.get("content", []):
      if part.get("type") == "image":
        return Image.open(io.BytesIO(base64.b64decode(part["data"]))).convert("RGB")
    raise RuntimeError(str(result)[:300])


class Node:
  __slots__ = "tag id text x y w h attrs parent kids flags key".split()

  @property
  def right(self):
    return self.x + self.w

  @property
  def bottom(self):
    return self.y + self.h

  def walk(self):
    yield self
    for kid in self.kids:
      yield from kid.walk()

  def said(self):
    """The first words in it, to tell it from its like."""
    for node in self.walk():
      if node.text:
        return node.text
    return ""


LINE = re.compile(r"^(\s*)- (\S+)(.*)$")
BOUNDS = re.compile(r" @(-?[\d.]+),(-?[\d.]+) ([\d.]+)x([\d.]+)")
ATTR = re.compile(r'\{([\w\-]+)=("(?:[^"\\]|\\.)*"|[^}]*)\}')


def unquote(value):
  if value.startswith('"'):
    try:
      return json.loads(re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda found: "\\u%04x" % int(found.group(1), 16), value))
    except Exception:
      return value.strip('"')
  return value


def parse(outline):
  """The outline of `lurq_read_tree` as nodes, in the window's own (logical) pixels."""
  head = re.match(r"window: \S+ \(([\d.]+)x([\d.]+) @([\d.]+)x\)", outline)
  scale = float(head.group(3)) if head else 1.0
  size = (float(head.group(1)) / scale, float(head.group(2)) / scale) if head else (0.0, 0.0)
  root = Node()
  root.tag, root.id, root.text, root.attrs, root.parent, root.kids, root.flags = "ROOT", None, None, {}, None, [], set()
  root.x = root.y = 0.0
  root.w, root.h = size
  stack = [(-1, root)]
  for line in outline.splitlines():
    found = LINE.match(line)
    if not found:
      continue
    node = Node()
    depth, node.tag, rest = len(found.group(1)) // 2, found.group(2), found.group(3)
    node.kids, node.attrs, node.text, node.id = [], {}, None, None
    node.x = node.y = node.w = node.h = None
    bounds = BOUNDS.search(rest)
    if bounds:
      node.x, node.y, node.w, node.h = (float(bounds.group(index)) / scale for index in range(1, 5))
    before = rest.split(" @")[0] if bounds else rest
    named = re.search(r" #(\S+)", before.split(' "')[0])
    if named:
      node.id = named.group(1)
    text = re.search(r' ("(?:[^"\\]|\\.)*")\s*$', before)
    if text:
      node.text = unquote(text.group(1))
    for name, value in ATTR.findall(rest):
      node.attrs[name] = unquote(value)
    flags = re.search(r" [(]([a-z,-]+)[)]$", rest)
    node.flags = set(flags.group(1).split(",")) if flags else set()
    while stack[-1][0] >= depth:
      stack.pop()
    node.parent = stack[-1][1]
    node.parent.kids.append(node)
    stack.append((depth, node))
  return root, size, scale


def sections(root):
  """The parts of a page, which a window of any width has in the same order: its head and each section."""
  page = next((node for node in root.walk() if node.attrs.get("a11y_role") == "main"), None)
  if page is None:
    return []
  node = page
  while node.kids:
    if len(node.kids) == 1:
      node = node.kids[0]
    elif len(node.kids) == 2 and node.kids[1].said().upper().startswith("ON THIS PAGE"):
      # The column of the page, with the list of its sections beside it in a wide window.
      node = node.kids[0]
    else:
      break
  return node.kids


def name_nodes(root):
  """A key for each node that a window of another width gives the same node: what it is and what it says, and
  which of those that say the same it is."""
  part = {}
  for index, section in enumerate(sections(root)):
    for node in section.walk():
      part[id(node)] = index
  seen = {}
  for node in root.walk():
    base = "%s|%s|%s|%s|%s|%s" % (part.get(id(node), "shell"), node.tag, node.id or "", node.attrs.get("a11y_role", ""),
                                  node.attrs.get("a11y_name", "")[:40], (node.text or node.said())[:40])
    seen[base] = seen.get(base, 0) + 1
    node.key = "%s#%d" % (base, seen[base])


def scrolls(node):
  """Whether what is in it is reached by scrolling: sideways, downwards."""
  sideways = node.tag in ("ScrollHorizontal", "ScrollBoth") or "scrollbar_horizontal" in node.attrs
  downwards = node.tag in ("ScrollVertical", "ScrollBoth") or "scrollbar_vertical" in node.attrs
  return sideways, downwards


CONTROLS = {"button", "link", "checkbox", "switch", "radio", "tab", "textbox", "slider", "combobox", "menuitem",
            "option", "searchbox", "spinbutton", "menuitemcheckbox", "menuitemradio", "treeitem"}


def is_control(node):
  return node.attrs.get("a11y_role") in CONTROLS or node.tag in ("Button", "TextInput", "Checkbox", "Slider")


def describe(node):
  said = node.text if node.text is not None else node.said()
  label = node.tag + (" #" + node.id if node.id else "")
  if node.attrs.get("a11y_name"):
    label += " <%s>" % node.attrs["a11y_name"][:40]
  if said:
    label += ' "%s"' % said[:48]
  return label


def measure(root, size):
  """What the tree of a window has: the faults that need no other window to be told, and the sizes of the
  controls and of the texts on one line, to compare with a wide window's."""
  faults, controls, texts = [], {}, {}
  endless = float("inf")

  def fault(kind, node, by, held, other=None):
    # The shape of the fault, with no words in it: what tells the same fault where a window of another width has
    # other words (a chart names other weeks on a shorter axis).
    shape = "%s|%s|%s|%s" % (node.key.split("|")[0], node.tag, node.id or "", held.tag if held else "")
    faults.append({
      "kind": kind, "key": node.key + ("~" + other.key if other else ""), "shape": shape, "what": describe(node),
      "box": [round(node.x), round(node.y), round(node.w), round(node.h)], "by": round(by),
      "in": describe(held) if held else "", "with": describe(other) if other else "",
    })

  def visit(node, left, right, top, bottom, across, down, depth):
    if node.w is None:
      for kid in node.kids:
        visit(kid, left, right, top, bottom, across, down, depth + 1)
      return
    if node.w <= 0.0 or node.h <= 0.0 or "dropped" in node.flags:
      return
    # Sideways: past what holds it, or past the window itself.
    over = max(node.right - right[0], left[0] - node.x)
    if over > TOLERANCE:
      fault("window" if right[1] is root or right[1].attrs.get("a11y_role") == "main" else "past", node, over, right[1])
      left, right = (node.x, node), (node.right, node)
    else:
      if node.x > left[0]:
        left = (node.x, node)
      if node.right < right[0]:
        right = (node.right, node)
    over = max(node.bottom - bottom[0], top[0] - node.y)
    if over > TOLERANCE:
      fault("under", node, over, bottom[1])
      top, bottom = (node.y, node), (node.bottom, node)
    else:
      if node.y > top[0]:
        top = (node.y, node)
      if node.bottom < bottom[0]:
        bottom = (node.bottom, node)
    if is_control(node):
      controls[node.key] = (node.w, node.h, node)
    if node.tag == "Text" and node.text:
      texts[node.key] = (node.w, node.h, node)
    sideways, downwards = scrolls(node)
    if sideways:
      # What scrolls sideways may hold what stays in place over what moves: a pinned column.
      across = True
      left, right = (-endless, node), (endless, node)
    if downwards:
      # A page wider than its own scroll is reached by no scroll at all.
      if node.attrs.get("a11y_role") == "main":
        for kid in node.kids:
          if kid.w is not None and kid.w - node.w > TOLERANCE:
            fault("window", kid, kid.w - node.w, node)
      top, bottom = (-endless, node), (endless, node)
    # Siblings in a row or a column do not lie over each other; a stack's do.
    if "Stack" not in node.tag and not across and len(node.kids) <= 80:
      boxes = [kid for kid in node.kids if kid.w and kid.h]
      for index, first in enumerate(boxes):
        for second in boxes[index + 1:]:
          wide = min(first.right, second.right) - max(first.x, second.x)
          high = min(first.bottom, second.bottom) - max(first.y, second.y)
          if wide > 2.0 and high > 2.0 and (first.said() or is_control(first)) and (second.said() or is_control(second)):
            fault("overlap", first, min(wide, high), node, second)
    for kid in node.kids:
      visit(kid, left, right, top, bottom, across, down, depth + 1)

  sys.setrecursionlimit(10000)
  visit(root, (0.0, root), (size[0], root), (0.0, root), (size[1], root), False, False, 0)
  return faults, controls, texts


class Gallery:
  """A gallery of this script's own, and the client that drives it."""

  def __init__(self, exe, width, height):
    self.lines, self.scale = [], 1.0
    self.process = subprocess.Popen([exe, "--size", "%dx%d" % (width, height), "--route", "/"],
                                    stderr=subprocess.PIPE, stdout=subprocess.DEVNULL, text=True, errors="replace")
    threading.Thread(target=self.read, daemon=True).start()
    until = time.time() + 60
    found = None
    while time.time() < until and not found:
      found = next((re.search(r"127\.0\.0\.1:(\d+)/mcp \(token ([^)]+)\)", line) for line in self.lines
                    if "/mcp" in line), None)
      if self.process.poll() is not None:
        raise RuntimeError("the gallery ended: " + "".join(self.lines)[-500:])
      time.sleep(0.1)
    if not found:
      self.close()
      raise RuntimeError("the gallery did not say where its MCP server is (built with --features mcp?)")
    self.client = Client(found.group(1), found.group(2))

  def read(self):
    for line in self.process.stderr:
      self.lines.append(line)

  def close(self):
    if self.process.poll() is None:
      self.process.kill()
      self.process.wait(10)

  def settle(self, frames=3, idle=700):
    try:
      self.client.tool("lurq_wait", frames=frames, timeout_ms=3000)
      # What turns for ever (a spinner) is never idle: the wait gives up and the page is read as it is.
      self.client.tool("lurq_wait", timeout_ms=idle)
    except RuntimeError:
      pass

  def tree(self):
    root, size, self.scale = parse(self.client.tool("lurq_read_tree", filter="all", max_chars=4000000))
    name_nodes(root)
    return root, size

  def wheel(self, page, delta):
    """Turns the wheel over the page: the server's pixels are the picture's, not the window's own."""
    self.client.tool("lurq_interact", action="wheel", x=(page.x + page.w / 2) * self.scale,
                     y=(page.y + page.h / 2) * self.scale, delta_y=delta, delta_x=0)

  def resize(self, width, height):
    """Makes the window `width` by `height` of its own pixels."""
    self.client.tool("lurq_resize", width=round(width * self.scale), height=round(height * self.scale))

  def page(self, root):
    return next((node for node in root.walk() if node.attrs.get("a11y_role") == "main"), None)

  def read_page(self, route):
    """The faults of a page, read at its top and at its end, and the sizes to compare."""
    self.client.tool("lurq_navigate", path=route)
    self.settle(4)
    root, size = self.tree()
    faults, controls, texts = measure(root, size)
    # Read once more a moment later: what is past its box now and elsewhere then is moving through it, as the bar
    # of a progress with no value does.
    time.sleep(0.06)
    later, _, _ = measure(*self.tree())
    still = {(found["kind"], found["key"], tuple(found["box"])) for found in later}
    faults = [found for found in faults if (found["kind"], found["key"], tuple(found["box"])) in still]
    page = self.page(root)
    if page is not None and page.kids and page.kids[0].h - page.h > 4:
      # The end of the page: what a page builds as it is scrolled to is read there.
      self.wheel(page, -400000)
      self.settle(3, 500)
      root, size = self.tree()
      known = {(found["kind"], found["key"]) for found in faults}
      more, more_controls, more_texts = measure(root, size)
      faults += [found for found in more if (found["kind"], found["key"]) not in known]
      for key, value in more_controls.items():
        controls.setdefault(key, value)
      for key, value in more_texts.items():
        texts.setdefault(key, value)
    return faults, controls, texts

  def shoot(self, root, size, faults, folder, name):
    """Pictures of the page from its top to its end, each fault in a red box."""
    from PIL import ImageDraw
    page = self.page(root)
    step = max(200.0, page.h - 120.0)
    total = page.kids[0].h if page.kids else page.h
    shown = 0
    # From the top: the page was left at its end by the walk.
    self.wheel(page, 400000)
    self.settle(3, 500)
    for index in range(40):
      root, size = self.tree()
      wanted = {(found["kind"], found["key"]) for found in faults}
      seen, _, _ = measure(root, size)
      picture = self.client.picture()
      scale = picture.width / size[0] if size[0] else 1.0
      draw = ImageDraw.Draw(picture)
      for found in seen:
        if (found["kind"], found["key"]) not in wanted:
          continue
        x, y, w, h = (value * scale for value in found["box"])
        draw.rectangle([x, y, x + w, y + h], outline=(255, 0, 0), width=2)
      picture.save(os.path.join(folder, "%s_%02d.png" % (name, index)))
      page = self.page(root)
      offset = page.y - page.kids[0].y if page.kids else 0.0
      if offset + page.h >= total - 2 or (index and offset <= shown + 1):
        break
      shown = offset
      self.wheel(page, -step * self.scale / 1.5)
      self.settle(3, 600)
    self.wheel(page, 400000)
    self.settle(2, 300)


def routes_of_the_gallery():
  source = open(os.path.join(ROOT, "crates", "gallery", "src", "pages", "mod.rs"), encoding="utf-8").read()
  return re.findall(r'^\s*path: "(/[^"]*)",', source, re.M)


def narrow_faults(narrow, wide):
  """What a narrow window has that a wide one does not."""
  faults, controls, texts = narrow
  wide_faults, wide_controls, wide_texts = wide
  known = {(found["kind"], found["key"]) for found in wide_faults}
  still = {(found["kind"], found["key"]) for found in faults}
  # What the wide window has under a name the narrow one has not: the same fault with other words in it, as
  # many times as it is there.
  renamed = {}
  for found in wide_faults:
    if (found["kind"], found["key"]) not in still:
      renamed[(found["kind"], found["shape"])] = renamed.get((found["kind"], found["shape"]), 0) + 1
  listed = []
  for found in faults:
    if (found["kind"], found["key"]) in known:
      continue
    if renamed.get((found["kind"], found["shape"]), 0) > 0:
      renamed[(found["kind"], found["shape"])] -= 1
      continue
    listed.append(found)
  for key, (w, h, node) in controls.items():
    was = wide_controls.get(key)
    if was and min(w, h) < LEAST_CONTROL and (w < was[0] - 1.0 or h < was[1] - 1.0):
      listed.append({"kind": "small", "key": key, "what": describe(node), "by": round(LEAST_CONTROL - min(w, h)),
                     "box": [round(node.x), round(node.y), round(w), round(h)], "in": "",
                     "with": "%dx%d in a wide window" % (round(was[0]), round(was[1]))})
  for key, (w, h, node) in texts.items():
    was = wide_texts.get(key)
    glyph = len(node.text) == 1 and 0xE000 <= ord(node.text) <= 0xF8FF
    # A text's box says how wide it was laid out, not how wide its words are: a text that fills a column is
    # narrower in a narrower column and still whole. One whose box is far less than its words need is not.
    needs = len(node.text) * h * NARROWEST_GLYPH
    if was and not glyph and w < was[0] - 1.0 and h <= was[1] + 1.0 and was[1] <= ONE_LINE and w < needs:
      listed.append({"kind": "shrunk", "key": key, "what": describe(node), "by": round(was[0] - w),
                     "box": [round(node.x), round(node.y), round(w), round(h)], "in": "",
                     "with": "%d wide in a wide window" % round(was[0])})
  return listed


def main():
  parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
  parser.add_argument("--exe", default=os.path.join(ROOT, "target", "debug", "gallery.exe"))
  parser.add_argument("--widths", type=int, nargs="*", default=[390, 320])
  parser.add_argument("--height", type=int, default=844)
  parser.add_argument("--routes", nargs="+", help="the pages to walk: every page unless given")
  parser.add_argument("--out", default=os.path.join(ROOT, "target", "narrow", "walk"),
                      help="where the lists go: <out>.json and <out>.txt")
  parser.add_argument("--shots", help="a folder for pictures of the pages that have faults")
  args = parser.parse_args()
  sys.stdout.reconfigure(encoding="utf-8", errors="replace")
  routes = args.routes or routes_of_the_gallery()
  os.makedirs(os.path.dirname(args.out), exist_ok=True)
  if args.shots:
    os.makedirs(args.shots, exist_ok=True)

  gallery = Gallery(os.path.abspath(args.exe), WIDE, 900)
  result = {}
  try:
    gallery.settle(6, 1500)
    gallery.tree()
    wide = {}
    for route in routes:
      wide[route] = gallery.read_page(route)
      print("wide %-22s %d" % (route, len(wide[route][0])), flush=True)
    for width in args.widths:
      gallery.resize(width, args.height)
      gallery.settle(6, 1500)
      result[width] = {}
      for route in routes:
        listed = narrow_faults(gallery.read_page(route), wide[route])
        result[width][route] = listed
        print("%4d %-22s %d" % (width, route, len(listed)), flush=True)
        if args.shots and listed:
          gallery.client.tool("lurq_navigate", path=route)
          gallery.settle(4)
          root, size = gallery.tree()
          gallery.shoot(root, size, listed, args.shots, "%d_%s" % (width, route.strip("/") or "overview"))
  finally:
    gallery.close()

  lines, total = [], 0
  for width in args.widths:
    counts = {kind: 0 for kind in KINDS}
    for listed in result[width].values():
      for found in listed:
        counts[found["kind"]] += 1
    pages = sum(1 for listed in result[width].values() if listed)
    total += sum(counts.values())
    lines.append("== %d wide: %d faults on %d of %d pages  (%s)" % (
      width, sum(counts.values()), pages, len(routes), "  ".join("%s %d" % (kind, counts[kind]) for kind in KINDS)))
    for route in routes:
      listed = result[width][route]
      if not listed:
        continue
      lines.append("%s  (%d)" % (route, len(listed)))
      for found in sorted(listed, key=lambda found: (found["box"][1], found["box"][0])):
        lines.append("  %-7s %-4d %s  @%d,%d %dx%d%s%s" % (
          found["kind"], found["by"], found["what"], *found["box"],
          "  in " + found["in"] if found["in"] else "", "  with " + found["with"] if found["with"] else ""))
    lines.append("")
  text = "\n".join(lines)
  open(args.out + ".txt", "w", encoding="utf-8").write(text)
  # The wide window's own list is kept beside them: what reaches past its box at every width.
  kept = {str(width): pages for width, pages in result.items()}
  kept[str(WIDE)] = {route: wide[route][0] for route in routes}
  json.dump(kept, open(args.out + ".json", "w", encoding="utf-8"), indent=1, ensure_ascii=False)
  print(text)
  return 1 if total else 0


if __name__ == "__main__":
  sys.exit(main())
