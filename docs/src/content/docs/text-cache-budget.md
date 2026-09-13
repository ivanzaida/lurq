---
title: Text Cache Budget
description: Comparing 32, 48, and 64 MiB of retained text layouts in development builds.
---

## Results — September 12, 2026

This report records the budget comparison. The subsequent [Text Cache Bookkeeping](/lurq/text-cache-bookkeeping/) optimization keeps the 48 MiB limit while reducing paragraph indexing and allocation-accounting work.

The default plain-text cache budget is now **48 MiB per GlyphEngine**, with the existing 64-entry limit and shaping-preserving compaction. This follows the [paragraph reuse optimization](/lurq/text-interaction-optimization/). Release compiler settings are unchanged.

At 150% display scaling, the 32 MiB budget could preserve both logical and physical paragraph shaping, but discarded their wrapped layouts. Each subsequent edit rewrapped all 3,456 paragraphs. At 48 MiB, both full layouts fit: an edit shapes and wraps only the two changed paragraphs, one at each text size. A 64 MiB budget retains exactly the same data in this workload, so the experiment provides no reason to choose that larger limit.

### Editing one paragraph

| Budget | CPU pass at 150%, median (p95) | Native DX12 action-to-paint at 150%, median (p95) | Native wrapping, median | Native retained charge during edits |
| --- | ---: | ---: | ---: | ---: |
| 32 MiB | 23.57 (39.47) ms | 19.81 (25.71) ms | 6.836 ms | 26.82 MiB |
| **48 MiB** | **10.05 (13.12) ms** | **10.29 (15.27) ms** | **0.024 ms** | **32.55 MiB** |
| 64 MiB | 10.21 (15.24) ms | 10.33 (12.67) ms | 0.022 ms | 32.55 MiB |

Native median edit latency falls by 48%. At 32 MiB, each edit compacts two retained entries; at 48 and 64 MiB, it compacts none. All three budgets shape exactly two paragraphs per edit. The improvement comes from retaining wrapping and avoiding compaction/reconstruction, beyond the earlier shaping reuse.

### Other interactions

| Native operation at 150% | 32 MiB, median (p95) | 48 MiB, median (p95) | 64 MiB, median (p95) |
| --- | ---: | ---: | ---: |
| Resize | 27.19 (35.83) ms | 26.42 (52.31) ms | 25.12 (30.87) ms |
| Scroll | 7.16 (11.00) ms | 7.37 (8.62) ms | 7.06 (9.41) ms |

There is no reliable resize or scroll improvement here. Every new width still lays out all 3,456 paragraphs, with zero reshaping. Native wrapping remains about 6.5–7 ms during resize. The 48 MiB resize tail is worse in this run; identical work counters and the active-desktop measurement do not establish a budget-caused regression or a tail-latency improvement. Scroll performs neither shaping nor wrapping, and still uploads the full 4 MiB atlas when it changes. That correctness workaround remains enabled.

At 100% scale, one full layout already fits at 32 MiB. All budgets wrap only one paragraph per edit and retain the same data; the CPU edit medians of 6.81, 6.13, and 6.52 ms are a useful indication of run-to-run noise. Cold native medians remain roughly 540–548 ms, dominated by renderer initialization. This change does not address that startup cost.

## Memory and instrumentation

The budget is a ceiling on charged retained storage, allocated on demand. It does not reserve 48 MiB upfront. Native edit samples retain an additional **5.73 MiB**, while CPU samples retain an additional **6.31 MiB** because the two probes have different initial viewport sizing. Peak retained charge across the native sequence is 34.06 MiB at both 48 and 64 MiB, versus 27.58 MiB at 32 MiB.

The charge counts exposed Cosmic line, shape, and layout allocations plus owned keys. It excludes private scratch/font allocations, allocator overhead, transient buffers, and other caches; these figures are not process RSS or a hard total-memory bound. Larger or simultaneous documents can still trigger compaction and eviction. Oversized full buffers are still excluded from retention.

Profiling now exposes current retained bytes, configured budget, and per-frame compaction, eviction, and oversized-bypass counters in logs and both CSV probes. For comparison only, debug builds with `perf_profile` accept `LURQ_TEXT_CACHE_MIB` from 1 through 1024. Unset or invalid values use the default. Release builds, ordinary builds without profiling, and unit tests ignore the environment override.

## Method and reproduction

The same CPU binary was used for every budget, and the same native binary for every native budget. Only the diagnostic environment setting varied. CPU measurements use three rotated blocks of five fresh sequences at each scale: 120 observations per interaction, budget, and scale. Native measurements use six fresh processes per budget, rotating and reversing order: 48 observations per interaction. Each sequence contains eight edits, eight resizes, and eight scrolls of the same 1,728-paragraph document used in the preceding report. Observations within a sequence are related.

Windows 11, Ryzen 9 7950X3D, rustc 1.92.0-nightly (5c7ae0c7e, 2025-10-02), dev opt-level 0 with the existing eleven text dependency overrides. Native display scaling is 150%. No compilation or tests ran during timing. CPU intervals time `Tree::pass` after update preparation; native intervals include the scripted action and shell processing through the paint callback, after the renderer returns. Neither measures physical display latency or GPU timestamps.

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
$env:LURQ_TEXT_CACHE_MIB = $null
./scripts/text-pipeline-metrics.ps1 -Variants workspace -Interactions -Samples 1 -OutputDirectory F:/codex-tmp/lurq-text-cache-local
cargo build -p lurq --example text_interaction_probe --features winit,dx12,perf_profile --locked
python scripts/compare-text-cache.py F:/codex-tmp/lurq-text-cache-local/workspace.exe F:/codex-tmp/lurq-text-cache-local/comparison --native F:/codex-tmp/lurq-text-metrics-target/debug/examples/text_interaction_probe.exe
```

The native probe opens and closes its window automatically. The comparison script validates complete sequences, identical ordered glyph counts, applied budgets, retained-byte limits, and paragraph shaping counts. Raw CSVs, summaries, execution order, binary hashes, and build/source metadata are preserved locally under `F:/codex-tmp/lurq-text-cache/`.

Validation includes 35 glyph-engine tests, including exact combined-budget retention without rewrapping, and recovery under forced 32 MiB pressure. Recovered layouts are compared against fresh Cosmic layouts, including mixed-direction text and fallback glyphs. The 20 Criterion smoke cases also pass without profiling. Native glyph counts agree across all budgets; this was not a renderer pixel-comparison test.
