---
title: Text Interaction Optimization
description: Paragraph reuse across edits and resizing, measured in CPU benchmarks and a native DX12 window.
---

> Historical benchmark report. Numbers and test counts below describe the dated source versions, toolchains, and fixtures in this report, not a new run of 0.20.0. See [Text Pipeline Optimization](../text-pipeline-optimization/) for the current implementation. Absolute artifact paths refer to the original local measurements and are not included in a fresh checkout.


## Results — September 12, 2026

This report records the paragraph-reuse comparison with a 32 MiB cache. The later [Text Cache Budget](/lurq/text-cache-budget/) experiment raises the default to 48 MiB and measures the resulting reduction in repeated wrapping.

The [first CPU optimization](/lurq/text-pipeline-cpu-optimization/) reduced cold layout work, but changing one paragraph still reshaped the entire document. This follow-up transfers unchanged paragraph shaping from a retained document version. Width changes discard wrapping while preserving shaping. Under cache pressure, older buffers can retain shaping alone, keeping logical and physical text sizes useful at fractional display scales without increasing the 32 MiB charged-storage budget.

The baseline already includes the first optimization and all eleven dev package overrides. This follow-up changes no optimization levels or release settings.

### CPU interaction passes

Median (nearest-rank p95), in milliseconds. Both versions run the same scripted scrolling document, with a no-op renderer.

| Display scale / operation | Before | After |
| --- | ---: | ---: |
| 100% / edit one paragraph | 63.05 (75.83) | **5.86 (6.92)** |
| 100% / change viewport width | 61.55 (76.06) | **9.93 (12.41)** |
| 100% / wheel scroll | 2.40 (3.82) | 2.16 (2.73) |
| 150% / edit one paragraph | 128.30 (141.84) | **19.22 (26.15)** |
| 150% / change viewport width | 129.12 (143.21) | **18.75 (27.14)** |
| 150% / wheel scroll | 2.87 (3.51) | 2.23 (3.24) |

After the change, an edit shapes **1 paragraph instead of 1,728** at 100%, or **2 instead of 3,456** at 150% (logical measurement and physical painting). Resizing shapes **zero** paragraphs at both scales. Unchanged cached passes have medians around 0.24–0.25 ms.

### Native desktop result

The native probe runs the same document in lurq's Winit shell with DX12, at the desktop's 150% display scale. It performs real window resizes and dispatches wheel events through `Tree::scroll`. Content edits update component props; this does not benchmark text-input selection, caret movement, keyboard dispatch, or IME composition.

| Operation | Before action-to-paint, median (p95) | After action-to-paint, median (p95) |
| --- | ---: | ---: |
| Edit one paragraph | 95.42 (102.59) ms | **20.56 (23.89) ms** |
| Resize native window | 136.57 (148.93) ms | **30.92 (40.40) ms** |
| Wheel scroll | 9.42 (10.70) ms | 9.28 (11.09) ms |

“Action-to-paint” starts before the scripted update and ends in the shell's paint callback after the renderer returns. It includes event-loop and resize processing, but is not a physical display-latency measurement or a GPU timestamp. The CPU table times the following `Tree::pass`, excluding scripted update preparation. The different timing boundaries and renderer explain why the tables should not be compared as equivalent measurements.

The first native frame remains expensive: median action-to-paint was 632 ms before and 624 ms after, with roughly 510–518 ms in the renderer call. This change targets interactions with a loaded document. It does not remove initial renderer setup or the first full-document layout, and the 150% results still exceed a 16.7 ms frame budget.

## Where the time went

The paragraph timers separate Cosmic Text's shaping from wrapping/positioning for the full multiline plain-text path. Shaping includes bidirectional analysis, font matching/fallback, and glyph shaping; the public API does not separate those internals. `plain_buffer_build` also includes paragraph lookup, text/allocation work, and finalization. `plain_buffer_retain` measures cache accounting, compaction, and eviction. These timers are inclusive where indicated and must not be summed indiscriminately. Small text using Cosmic's ordinary text setter is not covered by the paragraph sub-timers.

In the final 100% baseline, paragraph shaping took about 49 ms on an edit while wrapping took 3 ms. After reuse, those phases are approximately 0.14 ms and 0.01 ms. Resizing retains shaping and spends about 3.1 ms rewrapping. The counters establish what work disappeared even when wall times vary across runs.

The first native trial exposed a problem the 100% benchmark missed: complete logical-size and physical-size buffers evicted each other at 150% scale. Reusing paragraphs alone therefore left native edits around 93 ms. The cache now drops older wrapped glyph layouts before discarding their paragraph shaping. Layout is rebuilt lazily when those buffers are used again. In the final native edit samples, shaping falls from 75.7 ms to 0.41 ms; wrapping is around 7.1 ms because the compacted buffers need it rebuilt. This trades inexpensive rewrapping for retention of the more costly shaping.

## Cache behavior and correctness

On a full-text cache miss, a recent buffer with matching font/style/wrapping and a shared first or last paragraph is a reuse candidate. That boundary comparison is only a selection heuristic. Each transferred paragraph must match the complete text and line ending exactly; a hash only narrows lookup candidates. Whole paragraphs preserve the context required for bidirectional shaping. Changed, inserted, split, or unmatched paragraphs are shaped normally. A different width resets layout without invalidating matching paragraph shaping.

Reusing a prior version consumes its retained buffer, moving unchanged paragraph allocations instead of cloning all its glyphs. Existing measurement and packed-glyph caches remain valid for their original text keys. Font installation and alias changes still clear shaped data. Exact content height, wrapping, optical alignment, clipping, and glyph rendering rules are preserved.

The cache still has a 64-entry limit and a 32 MiB charged-storage budget. Compaction discards wrapped layout data before whole-entry eviction; a cache hit reconstructs full layout before measurement or painting reads its runs. Entries too large for the budget can still miss reuse. The charge covers exposed Cosmic allocations and owned keys, not all private scratch storage, allocator overhead, or total process memory. The candidate heuristic also means reuse is best effort, not a guarantee for arbitrary simultaneous document changes.

## Remaining renderer cost

The native probe observes a full **4 MiB atlas upload** on the sampled scroll and resize updates. After the change, atlas upload takes a median 4.13 ms during scrolling and 3.05 ms during resizing. This is an existing DX12 correctness workaround: `DX12_ALWAYS_FULL_ATLAS_UPLOAD` in `dx12_render/mod.rs` is enabled because partial uploads could produce missing glyphs. The guard remains enabled; reducing these uploads requires reproducing and fixing that rendering defect, with pixel-level validation.

Glyph image generation on the GPU would not fix this upload policy, initial renderer setup, or full-document shaping. The next measured candidates are the DX12 partial-upload defect and an explicit viewport/incremental layout contract for cold large documents. This change does not introduce a GPU rasterizer or a new virtualized document API.

## Method and reproduction

Windows 11 / Ryzen 9 7950X3D, same toolchain and system font setup as the earlier investigation. The source contains 1,728 paragraphs generated from 24 README copies, each line prefixed with its copy/line number to prevent duplicate-paragraph reuse. The sequence makes eight successive edits to one paragraph, eight distinct width reductions, then eight wheel scrolls, with an unchanged pass after each CPU interaction. Width starts at 860 and decreases by 23 per resize; height is 800. The native probe waits for each presentation plus a 150 ms idle interval before applying the next action.

CPU results use three alternating before/after blocks of five fresh sequences per scale: 15 cold samples and 120 samples per edit/resize/scroll phase, per version and scale. Native results use five fresh processes per version in alternating order: 40 samples per interaction phase. Samples within one sequence are related observations, not independent process starts. No compilation ran during the final comparison. This is an active desktop, so small timing differences and cold-run variation should not be interpreted as isolated-machine causal effects. Cold opens still shape the complete document in both versions.

The comparison verifies that every expected action ran and that glyph counts match in order for every CPU and native interaction. `--expect-incremental` also checks one/two shaped paragraphs per edit and zero on resize. This guards against accidentally benchmarking a stale binary. Use separate Cargo target directories for baseline and changed checkouts: a shared target directory produced stale cross-checkout library artifacts during setup; those trial outputs were discarded and lurq's artifacts were rebuilt before the final comparison.

To measure the checked-out code, keeping artifacts on F:

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
./scripts/text-pipeline-metrics.ps1 -Variants workspace -Interactions -Samples 15 -OutputDirectory F:/codex-tmp/lurq-text-interactions-local
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-text-interactions-local
cargo run -p lurq --example text_interaction_probe --features winit,dx12,perf_profile --locked -- F:/codex-tmp/lurq-text-interactions-local/native.csv
```

The native probe opens a window, runs its sequence, and closes automatically. It currently requires Windows/DX12. CPU metric mode can also be selected with `LURQ_TEXT_INTERACTIONS=<csv path>` and `LURQ_TEXT_METRICS_SAMPLES=<sequence count>` when invoking a saved profiling benchmark executable.

The final local binaries and results are preserved in `F:/codex-tmp/lurq-text-interactions/`. `comparison/` contains merged and per-block CPU CSVs; `comparison/native/` contains native samples and summary; metadata records execution order and binary hashes. `final-build/workspace.metadata.json` records the after-build command, toolchain, package overrides, and source hashes. The diagnostic baseline source is in `F:/codex-tmp/lurq-text-interaction-baseline`, with the previous CPU optimization plus the same interaction harness and profiling instrumentation.

```powershell
python scripts/compare-text-interactions.py F:/codex-tmp/lurq-text-interactions/before.exe F:/codex-tmp/lurq-text-interactions/after.exe F:/codex-tmp/lurq-text-interactions/comparison --native-before F:/codex-tmp/lurq-text-interactions/native-before.exe --native-after F:/codex-tmp/lurq-text-interactions/native-after.exe --expect-incremental
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-text-interactions/comparison
```

Validation passed: **96 focused tests** (34 glyph-engine, 8 runtime text, 22 scroll, 15 centering/layout, 17 Markdown) and **20 Criterion smoke cases without profiling**. New tests compare incremental edits, insertion/removal, reordering, splitting, CRLF changes, width/style changes, mixed-direction text, and cache compaction recovery against fresh Cosmic layout. Existing clipping, glyph rendering, font invalidation, and DPI tests also pass. Native glyph counts match, but this run is not a pixel-level screenshot comparison of the renderer.
