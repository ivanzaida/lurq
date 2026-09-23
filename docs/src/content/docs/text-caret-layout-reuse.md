---
title: Caret Layout Reuse
description: Sharing paragraph layouts between caret calculation, measurement, and painting for selectable text.
---

> Historical benchmark report. Numbers and test counts below describe the dated source versions, toolchains, and fixtures in this report, not a new run of 0.20.0. See [Text Pipeline Optimization](../text-pipeline-optimization/) for the current implementation. Absolute artifact paths refer to the original local measurements and are not included in a fresh checkout.


## Results — September 12, 2026

Caret calculation now shares retained plain-text layouts with measurement and painting. The baseline includes the [48 MiB cache and paragraph bookkeeping optimizations](/lurq/text-cache-bookkeeping/). Both versions use the same development dependency overrides and cache budget. Release compiler settings and DX12 rendering are unchanged.

This comparison enables `Text::selectable(true)` on the 1,728-paragraph document. Its edits update component props. Typing, pointer selection, and masked-input behavior are covered by correctness tests; keyboard and IME latency require separate measurements. The earlier ordinary-Text benchmarks exercised a different path and should not be treated as the baseline for these figures.

### Native desktop at 150% scaling

| Operation | Before, median (p95) | After, median (p95) |
| --- | ---: | ---: |
| Edit one selectable-text paragraph | 52.92 (63.96) ms | **6.79 (8.65) ms** |
| Resize native window | 69.14 (93.98) ms | **21.37 (27.74) ms** |
| Wheel scroll | 6.55 (8.36) ms | 6.39 (8.01) ms |

Median edit latency falls by 87%. Native intervals start before the scripted update and end in the shell's paint callback after the renderer returns. They include event-loop and resize processing. Physical display latency and GPU timestamps are outside this measurement.

### CPU interaction passes

| Scale / operation | Before, median (p95) | After, median (p95) |
| --- | ---: | ---: |
| 100% / edit | 53.28 (334.81) ms | **5.11 (6.41) ms** |
| 100% / resize | 56.25 (97.42) ms | **9.81 (12.80) ms** |
| 150% / edit | 50.90 (125.70) ms | **5.71 (6.99) ms** |
| 150% / resize | 58.09 (149.59) ms | **13.89 (16.33) ms** |

These intervals time `Tree::pass` after update preparation. The active-desktop CPU baseline contains large tail outliers; the native comparison independently confirms the improvement with a much tighter baseline distribution. The p95 differences should not be interpreted as an isolated-machine speedup factor. Unchanged CPU passes remain around 0.23–0.25 ms and scrolling around 1.9 ms.

Cold CPU medians fall from 98.93 to 51.22 ms at 100% and from 148.58 to 93.20 ms at 150%. A cold selectable layout now supplies its paragraph shaping to measurement instead of doing that work twice. Native first-frame median moves from 578.35 to 540.91 ms; about 433 ms of the latter remains in the first renderer call.

## Why the old path was expensive

On a caret-cache miss, `compute_caret_positions` acquired a separate Cosmic buffer, replaced its text, shaped it, extracted positions, and returned the buffer to the scratch pool. It bypassed the retained paragraph layouts used by ordinary text. A small edit therefore reshaped the complete logical-size document to construct carets, even though measurement and painting could already reuse unchanged paragraphs.

The selectable layout path asks for carets **before** measurement. The new implementation calls `full_text_buffer` with normalized wrapping and retains the buffer after extracting positions. Carets can build the layout first and supply it to later consumers; when measurement runs first, carets can take its exact cached layout. Subsequent edits reuse unchanged paragraphs in either order. The caret extraction algorithm, byte offsets, glyph-edge ordering, empty-line handling, and final text-end position are unchanged.

During native edits, the caret request drops from a median 48.53 to 2.78 ms. Its buffer-preparation sub-timer drops from 46.36 to 0.54 ms. Extracting roughly 112,000 positions still takes about 2.03 ms. At this stage the API returned an owned vector so masked-input callers could remap it without changing the cached result. The subsequent [Caret and Selection Indexing](/lurq/text-selection-optimization/) change shares immutable geometry and detaches a private copy when remapping is needed.

## Profiling and remaining work

The CPU and native probes now record caret request time, buffer preparation, extraction, request/cache-hit/cache-miss counts, returned-position counts, and exact retained-layout hits. `caret_layout_reuses` counts exact full-layout hits inside a caret miss. It is normally zero in this selectable scenario because carets are the first consumer; the paragraph-retention counters show the reuse, and measurement subsequently hits the caret-produced layout. Timers are inclusive where one operation contains another.

The retained plain-text budget remains **48 MiB**. Peak charged storage is approximately 34.02 MiB in the after-build sequences, with no compactions, evictions, or oversized bypasses. This charge covers exposed Cosmic allocations and owned keys. Private scratch/font allocations, allocator overhead, transient buffers, the separate caret-result cache, and process RSS are outside that bound.

Caret-vector extraction and copies remain candidates for further measurement. Resize still rewraps the document, taking about 6.8 ms across logical and physical text sizes. The native resize median remains above 16.7 ms. Scrolling still uploads the full 4 MiB glyph atlas when it changes, with a median atlas-upload timer near 2.7 ms. The existing DX12 partial-upload correctness guard remains enabled.

## Method and reproduction

Windows 11, Ryzen 9 7950X3D, rustc 1.92.0-nightly (5c7ae0c7e, 2025-10-02), dev opt-level 0 with the same eleven text dependency overrides. Native display scale is 150%. Both binaries use the same new selectable harness and caret instrumentation; the baseline retains the old caret buffer construction.

CPU results use three alternating blocks of five fresh sequences at each display scale: 120 samples per edit/resize/scroll phase, per version and scale. Native results use six alternating processes per version: 48 samples per interaction. Each sequence performs eight edits, eight width changes, and eight scrolls; CPU runs include an unchanged pass after each action. Samples within one sequence are related. No compilation or tests ran during timing.

`--selectable` enables the new probe mode and validates that caret work actually ran. Every expected action completed. Ordered glyph counts and returned caret-position counts match between versions across all 1,470 CPU rows and 150 native rows per version. Exact caret coordinates and byte offsets are checked separately against the original fresh-Cosmic path in unit tests.

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
$env:LURQ_TEXT_CACHE_MIB = $null
$env:LURQ_TEXT_SELECTABLE = '1'
./scripts/text-pipeline-metrics.ps1 -Variants workspace -Interactions -Samples 1 -OutputDirectory F:/codex-tmp/lurq-text-carets-local
cargo build -p lurq --example text_interaction_probe --features winit,dx12,perf_profile --locked
python scripts/compare-text-interactions.py F:/codex-tmp/lurq-text-carets/before.exe F:/codex-tmp/lurq-text-carets-local/workspace.exe F:/codex-tmp/lurq-text-carets-local/comparison --native-before F:/codex-tmp/lurq-text-carets/native-before.exe --native-after F:/codex-tmp/lurq-text-metrics-target/debug/examples/text_interaction_probe.exe --native-runs 6 --selectable --expect-incremental
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-text-carets-local/comparison
$env:LURQ_TEXT_SELECTABLE = $null
```

The native probe opens and closes its window automatically. Ordinary document mode remains the default. Binaries, baseline source, source/build metadata, validation results, raw CSVs, and summaries are preserved under `F:/codex-tmp/lurq-text-carets/`.

Validation passed: **196 focused tests** (39 glyph-engine, 91 text/input, 4 synthetic-input, 8 runtime text, 22 scroll, 15 centering/layout, and 17 Markdown), plus **23 Criterion smoke cases without profiling**. New tests compare every caret's byte index and coordinate bits across wrapping, alignment, font sizes, mixed scripts, combining marks, emoji, CRLF, empty text, and masks. They also verify both consumer orders, incremental edits, reflow, cache clearing, accounting, and independence of mutable returned vectors. Existing typing, navigation, selection, soft-wrap clicking, and masked-editing tests pass.
