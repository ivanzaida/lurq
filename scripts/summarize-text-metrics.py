"""Summarize text-pipeline-metrics.ps1 CSVs using only the Python standard library."""

import argparse
import csv
import math
import statistics
from collections import defaultdict
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, nargs="?", default=Path("target/text-metrics"))
    args = parser.parse_args()
    summaries = []
    counts = defaultdict(dict)
    for path in sorted(args.directory.glob("*.csv")):
        if path.name == "summary.csv":
            continue
        with path.open(newline="", encoding="utf-8-sig") as source:
            groups = defaultdict(list)
            for row in csv.DictReader(source):
                groups[row["case"], row["phase"]].append(row)
        for (case, phase), rows in groups.items():
            summary = dict(variant=path.stem, case=case, phase=phase, samples=len(rows))
            for field in rows[0]:
                if field in {"case", "phase", "sample"}:
                    continue
                values = sorted(float(row[field]) for row in rows)
                summary[field + "_median"] = statistics.median(values)
                if field == "wall_ms":
                    # Nearest-rank percentile: no distribution or confidence interval assumption.
                    summary["wall_ms_p95"] = values[math.ceil(0.95 * len(values)) - 1]
            summaries.append(summary)
            counts[case, phase][path.stem] = {int(row["glyph_count"]) for row in rows}
    if not summaries:
        parser.error(f"no metrics CSVs in {args.directory}")
    for (case, phase), variants in counts.items():
        if len({tuple(sorted(values)) for values in variants.values()}) != 1:
            raise ValueError(f"rendered glyph counts differ for {case}/{phase}: {variants}")
    output = args.directory / "summary.csv"
    with output.open("w", newline="", encoding="utf-8") as dest:
        fields = list(dict.fromkeys(field for row in summaries for field in row))
        writer = csv.DictWriter(dest, fieldnames=fields)
        writer.writeheader()
        writer.writerows(summaries)
    if any(row["case"].startswith("document") for row in summaries):
        print("Document interaction CPU pass time in ms (median / p95)")
        for case in sorted({row["case"] for row in summaries if row["case"].startswith("document")}):
            print(case)
            phases = ["cold", "edit", "resize", "scroll", "warm"]
            print(f"{'variant':<18}" + "".join(f" {phase:>18}" for phase in phases))
            for variant in sorted({row["variant"] for row in summaries}):
                cells = []
                for phase in phases:
                    row = next((r for r in summaries if (r["variant"], r["case"], r["phase"]) == (variant, case, phase)), None)
                    cells.append(f"{row['wall_ms_median']:.3f} / {row['wall_ms_p95']:.3f}" if row else "—")
                print(f"{variant:<18}" + "".join(f" {cell:>18}" for cell in cells))
        print(f"Saved {output}; rendered glyph counts agree across variants.")
        return
    print("Cold pass wall time in ms (median / p95)")
    cases = ["readme", "long_text", "flow_long_text", "readme_tall"]
    if any(row["case"] == "unique_long_text" for row in summaries):
        cases.append("unique_long_text")
    print(f"{'variant':<18}" + "".join(f" {case:>18}" for case in cases))
    for variant in sorted({row["variant"] for row in summaries}):
        cells = []
        for case in cases:
            row = next((r for r in summaries if (r["variant"], r["case"], r["phase"]) == (variant, case, "cold")), None)
            cells.append(f"{row['wall_ms_median']:.3f} / {row['wall_ms_p95']:.3f}" if row else "—")
        print(f"{variant:<18}" + "".join(f" {cell:>18}" for cell in cells))
    print(f"Saved {output}; rendered glyph counts agree across variants.")


if __name__ == "__main__":
    main()
