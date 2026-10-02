"""Seal retained compiler/test receipts, preserving their original bytes."""
import hashlib
import json
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent
    files = sorted(root.glob("run*/*")) + [root / "report.md", Path(__file__)]
    result = {
        "source": "bf0c571e89567e3b73d13cc4752657514112e51b",
        "tree": "42b7f8e4a8366e7af0b9683b1d85a864ecebae79",
        "scope": "builder MCP16/window10/default check; independent protocol pending",
        "artifacts": [{"path": path.relative_to(root).as_posix(),
                       "bytes": path.stat().st_size,
                       "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                      for path in files],
    }
    (root / "manifest.json").write_text(json.dumps(result, indent=2) + "\n",
                                       encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
