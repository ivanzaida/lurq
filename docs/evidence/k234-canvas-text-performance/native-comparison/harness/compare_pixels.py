"""Compare retained Canvas screenshots without changing image/evidence bytes."""
import argparse
import hashlib
import json
from pathlib import Path
from PIL import Image


def compare(first, second):
    with Image.open(first) as left, Image.open(second) as right:
        left.load()
        right.load()
        result = {"first": str(first), "second": str(second),
                  "first_size": list(left.size), "second_size": list(right.size),
                  "first_png_sha256": hashlib.sha256(first.read_bytes()).hexdigest(),
                  "second_png_sha256": hashlib.sha256(second.read_bytes()).hexdigest()}
        if left.size != right.size:
            return {**result, "same_dimensions": False, "pixels_equal": False}
        a, b = left.convert("RGBA").tobytes(), right.convert("RGBA").tobytes()
        count = sum(a[index:index + 4] != b[index:index + 4] for index in range(0, len(a), 4))
        return {**result, "same_dimensions": True, "pixels_equal": a == b,
                "differing_pixels": count, "total_pixels": len(a) // 4,
                "max_channel_difference": max((abs(x - y) for x, y in zip(a, b)), default=0)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("capture_root", type=Path)
    arguments = parser.parse_args()
    root = arguments.capture_root.resolve()
    assert root.is_relative_to(Path(__file__).resolve().parent)
    result = {"comparisons": [compare(root / a / "capture" / f"{x}.png",
                                     root / b / "capture" / f"{y}.png")
                              for a, x, b, y in (("baseline", "before", "baseline", "after"),
                                                ("candidate", "before", "candidate", "after"),
                                                ("baseline", "before", "candidate", "before"),
                                                ("baseline", "after", "candidate", "after"))],
              "scope": "Decoded RGBA pixel equality of authenticated settled Canvas captures; no image rewriting"}
    destination = root / "pixel-comparison.json"
    assert not destination.exists(), "preserve existing comparison evidence"
    destination.write_text(json.dumps(result, indent=2), encoding="utf-8", newline="\n")
    print(json.dumps(result, indent=2))
