---
title: Caret and Selection Indexing
description: Measured dev-build improvements to pointer selection, caret navigation, and shared caret storage.
---

> Historical benchmark report. Numbers and test counts below describe the dated source versions, toolchains, and fixtures in this report, not a new run of 0.20.0. See [Text Pipeline Optimization](../text-pipeline-optimization/) for the current implementation. Absolute artifact paths refer to the original local measurements and are not included in a fresh checkout.


## Results — September 12, 2026

Caret lookups now use visual-line ranges, and cached caret geometry is shared with node state. This follows [Caret Layout Reuse](/lurq/text-caret-layout-reuse/), which removed redundant full-document shaping for selectable text. The baseline here already includes that change, the eleven dev dependency overrides, and the 48 MiB plain-buffer cache.

### Pointer and keyboard interactions at 150% scaling

| Interaction near document bottom | Before, median (p95) | After, median (p95) |
| --- | ---: | ---: |
| First selectable-text click | 0.82 (0.96) ms | **0.25 (0.34) ms** |
| Selectable-text drag | 1.31 (1.50) ms | **0.20 (0.29) ms** |
| First input click | 2.38 (3.14) ms | **0.39 (0.57) ms** |
| Input drag | 2.89 (3.49) ms | **0.30 (0.42) ms** |
| Input Up/Down | 3.05 (3.42) ms | **0.30 (0.42) ms** |
| Input Shift+Up/Down | 3.40 (4.14) ms | **0.30 (0.42) ms** |

These are CPU intervals from immediately before `Tree` mouse/key dispatch through completion of `Tree::pass`, using a renderer that records selection/caret rectangle geometry. They include event processing, layout work, and render-list construction. They exclude the native event loop, GPU, and physical display latency. Events are injected through the normal Tree APIs; no physical mouse or keyboard is driven.

The first click is included: the final implementation supplies the line table during caret extraction, so ordinary shaped text does not incur a lazy full-vector scan when first clicked.

| Scale / location | Input drag before → after, median | Input Up/Down before → after, median |
| --- | ---: | ---: |
| 150% / top | 1.18 → 0.30 ms | 1.42 → 0.30 ms |
| 150% / middle | 2.05 → 0.32 ms | 2.21 → 0.32 ms |
| 100% / bottom | 2.92 → 0.32 ms | 3.09 → 0.32 ms |

The baseline gets slower as the caret moves deeper into the document. Indexing removes that document-prefix scan. All ordered selection/caret rectangle signatures and glyph/caret counts match between versions across **9,000 recorded events per version**. Differential unit tests separately compare exact lookup results and rectangle coordinates with the previous linear algorithms.

## Implementation

`CaretPositions` owns an `Arc` containing the flat caret vector and its line index. The shaping cache, text state, input state, and state transferred during rerenders share that immutable allocation. A cache hit no longer clones roughly 112,000 entries. Masked input requests a private copy before remapping byte offsets and invalidates any index on that copy; it cannot modify another node or the shaping cache.

During extraction, each Cosmic run contributes its caret-vector range, y coordinate, and conservative source-paragraph bounds. These bounds include all glyph offsets even with wrapping and bidirectional glyph order. Recording them requires one operation per visual run and avoids a second per-glyph walk. Runs grouped at the same y preserve the old epsilon comparison and endpoint ordering.

Point hit-testing binary-searches visual rows, then scans just the chosen row. Byte-offset lookup and Up/Down navigation binary-search paragraph bounds, then scan the relevant caret slice. Selection painting visits candidate rows instead of first scanning the complete vector. Ties, missing-offset fallbacks, and original visual ordering remain unchanged. Nonmonotonic index ranges or y coordinates retain a linear fallback. Fields with fewer than 256 positions use direct scans without retaining a line table.

An index adds storage proportional to the number of visual rows, while sharing removes a full caret-vector copy per consumer. This storage belongs to the separate caret cache and is **outside** the 48 MiB charge for retained Cosmic buffers. This change does not establish a process-memory limit or change the caret cache's existing entry-count limit.

## Edit and resize regression check

| Native operation at 150% | Before, median (p95) | After, median (p95) |
| --- | ---: | ---: |
| Selectable-text prop edit | 7.69 (10.02) ms | 7.02 (8.43) ms |
| Window resize | 27.15 (31.07) ms | 26.79 (30.38) ms |
| Wheel scroll | 8.17 (9.65) ms | 8.21 (9.67) ms |

Native intervals start before the scripted action and end in the paint callback after the renderer returns. These are separate prop-edit, resize, and scroll checks using a real DX12 window; the pointer/keyboard speedups above are CPU measurements. The native baseline is the saved final binary from the preceding caret-layout-reuse iteration, with matching source and compiler settings.

Recording line metadata has a small cost: native caret extraction rises from 2.25 to 2.43 ms per edit. Shared storage removes copy work elsewhere, so total caret request time decreases from 3.12 to 3.03 ms and native action-to-paint time improves by about 0.67 ms. The benefit is substantially larger during repeated navigation, where the table is reused.

CPU selectable prop-edit passes improve from 5.76 to 5.19 ms at 100% and from 6.35 to 5.85 ms at 150%. Native resize and scroll timings stay similar. These figures come from this alternating cohort; compare versions within it rather than against absolute timings from earlier reports.

The regression check uses three alternating CPU blocks of five sequences at each scale and six alternating native processes per version, yielding 120 CPU and 48 native samples per edit/resize/scroll phase. Ordered glyph and caret counts match, and incremental paragraph-shaping assertions pass. The 48 MiB retained-buffer budget is unchanged.


## Method and validation

Windows 11, Ryzen 9 7950X3D, rustc 1.92.0-nightly (5c7ae0c7e, 2025-10-02), dev opt-level 0 with debug assertions and the existing text dependency overrides. Release compiler settings and renderer code are unchanged.

The new selection probe uses the same 1,728-paragraph README-derived document, a scroll viewport, selectable `Text`, and multiline `TextInput`. It tests 100% and 150% scale at the top, middle, and bottom of the document. Each sequence records the first press, 24 drag moves, release, and, for inputs, 24 alternating Up/Down and 24 Shift+Up/Down events. The input value must remain unchanged, and drag frames must contain selection rectangles. A persistent caret and fixed selection/caret colors make geometry comparisons deterministic.

Results use three alternating blocks of five fresh sequences per case, scale, and location: 360 samples for each repeated interaction and 15 first-click samples. p95 uses nearest rank. Samples within a sequence are related. Preliminary runs overlapped another project's compiler activity and are kept separately; the reported selection runs started after that activity settled. This is an active desktop, not an isolated performance host. This task ran no builds or tests during the reported timing runs.

Validation passed: **160 focused tests** (39 glyph-engine, 3 new differential indexing/ownership, 91 text/input, 4 synthetic input, 15 text-centering, 8 text-runtime) and **23 Criterion smoke cases** without `perf_profile`. Cases include empty lines, CRLF, soft wrapping, ligatures, combining marks, emoji, mixed RTL/LTR text, alignment, scaling, mask editing, duplicate offsets, tie behavior, and cache ownership.

All binaries, build JSON, baseline/final core snapshots, CSVs, and provenance are under `F:/codex-tmp/lurq-text-selection/`. Reported selection results are in `quiet-comparison/`; edit/resize/scroll checks are in `interaction-comparison/`. The earlier `comparison/` and `final-comparison/` directories are preliminary runs, not the headline measurements.

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
$env:LURQ_TEXT_SELECTION = 'F:/codex-tmp/lurq-selection-local.csv'
$env:LURQ_TEXT_METRICS_SAMPLES = '5'
cargo bench -p lurq --bench text_pipeline --features markdown,perf_profile --profile dev --locked
$env:LURQ_TEXT_SELECTION = $null
python scripts/compare-text-selection.py F:/codex-tmp/lurq-text-selection/before.exe F:/codex-tmp/lurq-text-selection/after.exe F:/codex-tmp/lurq-selection-repeat
```

## Remaining work

At this stage edits and reflow still regenerated the flat caret vector. [Paragraph Caret Reuse](/lurq/text-paragraph-carets/) subsequently retains unchanged paragraph geometry and assembles document-row references. A single extremely long paragraph can still require a large scan for byte-offset lookup because the index uses conservative paragraph bounds. Long masked values rebuild their private index after remapping. Native resize wrapping and full-atlas uploads remain separate targets; the DX12 partial-upload correctness guard stays enabled.
