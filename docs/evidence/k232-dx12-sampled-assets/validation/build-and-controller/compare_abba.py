"""Offline decoded pixels after all four owned processes have closed."""
import importlib.util
import json
from pathlib import Path
from PIL import Image, ImageChops

HERE = Path(__file__).resolve().parent
helper = HERE.parent / "compare_pixels.py"
spec = importlib.util.spec_from_file_location("canonical_pixels", helper)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
receipts = json.loads((HERE / "abba-01/run-receipts.json").read_text(encoding="utf-8-sig"))
assert len(receipts) == 4 and all(row["exit_code"] == 0 for row in receipts)
captures = {row["name"]: Path(row["capture_root"]) / "capture" for row in receipts}
pairs = [(name, "before", name, "after") for name in ("A1", "B1", "B2", "A2")]
pairs += [(a, image, b, image) for a, b in (("A1", "B1"), ("A2", "B2"))
          for image in ("before", "after")]
results = []
for a, x, b, y in pairs:
    first, second = captures[a] / f"{x}.png", captures[b] / f"{y}.png"
    result = module.compare(first, second)
    assert result["same_dimensions"]
    with Image.open(first) as left, Image.open(second) as right:
        channels = ImageChops.difference(left.convert("RGBA"), right.convert("RGBA")).split()
        combined = channels[0]
        for channel in channels[1:]:
            combined = ImageChops.lighter(combined, channel)
        result["difference_bbox_half_open"] = combined.getbbox()
    results.append({"pair": [a, x, b, y], **result})
output = HERE / "abba-01/decoded-pixel-comparisons.json"
assert not output.exists()
output.write_text(json.dumps({"comparisons": results,
    "scope": "Canonical decoded RGBA comparison plus all-channel difference bounds; no ROI excluded or image bytes changed"}, indent=2), encoding="utf-8")
print(json.dumps(results, indent=2))
