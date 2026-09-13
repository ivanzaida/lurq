---
title: Text Cache Bookkeeping
description: Measured reductions in paragraph indexing and retained-allocation accounting during edits and resizing.
---

## Results — September 12, 2026

This report measures ordinary `Text`. The later [Caret Layout Reuse](/lurq/text-caret-layout-reuse/) report adds a selectable-text benchmark and shares its caret layout with measurement and painting.

Plain-text edits now preserve matching paragraph boundaries directly and retain allocation counts with the shaped buffer. This follows the [48 MiB cache-budget change](/lurq/text-cache-budget/). Both versions in this comparison use that budget, the 64-entry limit, and the same eleven development dependency overrides. Release compiler settings and renderer behavior are unchanged.

### Native desktop at 150% scaling

| Operation | Before, median (p95) | After, median (p95) |
| --- | ---: | ---: |
| Edit one paragraph | 9.73 (11.09) ms | **3.82 (5.45) ms** |
| Resize native window | 24.49 (27.56) ms | **18.55 (22.21) ms** |
| Wheel scroll | 7.06 (8.35) ms | 6.63 (7.57) ms |

Median edit latency falls by 61%, and resize latency by 24%. These intervals run from the scripted action through the shell's paint callback after the DX12 renderer returns. They include event-loop and resize processing, but are not GPU timestamps or physical display latency.

### CPU interaction passes

| Scale / operation | Before, median (p95) | After, median (p95) |
| --- | ---: | ---: |
| 100% / edit | 5.49 (6.46) ms | **2.28 (2.99) ms** |
| 100% / resize | 9.31 (11.71) ms | **6.92 (9.45) ms** |
| 100% / scroll | 2.12 (2.64) ms | 1.95 (2.18) ms |
| 150% / edit | 8.65 (9.71) ms | **2.62 (3.50) ms** |
| 150% / resize | 16.67 (18.65) ms | **10.82 (12.63) ms** |
| 150% / scroll | 2.04 (2.39) ms | 1.85 (2.12) ms |

CPU timing covers `Tree::pass` after scripted update preparation. Unchanged passes remain around 0.22–0.24 ms. The smaller scroll improvement is consistent with cheaper buffer returns; shaping and wrapping already did no work during scroll.

## What changed

Previously, every document edit constructed a hash index for all old paragraphs, hashed the new paragraphs, and transferred matches into a fresh buffer. Now the old and new paragraphs are compared from both ends. Matching prefixes and suffixes remain in the existing buffer, preserving paragraph and glyph allocations. Only the intervening old paragraphs enter the lookup. Stored fingerprints avoid hashing their text again; every candidate still requires an exact text and line-ending comparison. Reordering within the changed middle continues to reuse matching paragraphs.

The benchmark's edit changes one paragraph. It now indexes one old paragraph and recounts one paragraph's shape allocations at 100%, or two at 150% for logical and physical text sizes. Resizing indexes and recounts zero paragraph shapes. The matching-boundary scan, source line parsing, and summation of small metadata records still walk the document; this does not make arbitrary edits constant-time.

Cached buffers now carry their owned key, per-paragraph fingerprints, shape/layout allocation charges, and total charge through measurement, optical bounds, and painting. Returning an unchanged buffer preserves those values instead of cloning its text key and scanning every glyph allocation. A changed paragraph refreshes its shape and layout charges; rewrapping refreshes layout charges only. Cloned Cosmic lines are measured again because cloning can change vector capacities. Cache compaction clears layout charges while preserving shape charges, and recovery rebuilds the missing layout charges.

The retained-buffer wrapper exposes immutable access to Cosmic runs. Changes to its underlying buffer are confined to construction/recovery paths that update accounting. Clipped partial buffers remain separate and are not retained as full layouts.

At 150% in the CPU probe, median full-buffer construction falls from 5.507 to 0.735 ms during edits; retention falls from 1.003 to 0.0004 ms. Accounting that is still necessary now occurs during construction, so the wall-time comparison is the main result. Profiling timers are inclusive where operations contain others and should not be added indiscriminately. New CSV/log counters report indexed old paragraphs and paragraphs whose shape allocations were recounted.

## Memory and remaining costs

The budget remains **48 MiB**, with no compactions, evictions, or oversized bypasses during this comparison. On this 64-bit target, each paragraph's metadata occupies 24 bytes; the two 1,728-paragraph copies need about 81 KiB of metadata. Different vector spare capacities offset that storage in this workload: native edit cache charge is approximately 32.51 MiB after versus 32.55 MiB before. This is not a general memory-reduction claim.

Charges still cover exposed Cosmic allocations and owned keys, not private scratch/font allocations, allocator overhead, transient data, or process RSS. Tests compare the incremental charge with a fresh walk of actual public allocations after edits, reordering, duplicate cloning, shrinking, rewrapping, and compaction recovery.

Native resize wrapping remains about 6.5 ms, and resize latency still exceeds 16.7 ms. Native scroll continues to upload the full 4 MiB atlas when it changes, spending about 2.7 ms in the atlas-upload timer. The existing DX12 partial-upload correctness guard remains enabled. Cold native medians are essentially unchanged at 562 versus 564 ms, with about 458–461 ms in the first renderer call. This change addresses interaction bookkeeping, not initial renderer setup or the first full-document shaping pass.

## Method and reproduction

Windows 11, Ryzen 9 7950X3D, rustc 1.92.0-nightly (5c7ae0c7e, 2025-10-02). Dev opt-level 0 with the existing text dependency overrides; native desktop scale 150%. The workload is the same 1,728-paragraph README-derived document as the preceding reports, with eight edits, eight width changes, and eight scrolls per sequence. CPU runs also include an unchanged pass after each action.

Three alternating CPU blocks of five fresh sequences per scale produce 120 samples per interaction, version, and scale. Six alternating native processes per version produce 48 samples per interaction. Samples within one sequence are related. No compilation or tests ran during timing. This is an active desktop; small differences and cold-run tails should not be treated as isolated-machine causal effects.

The saved baseline is the preceding 48 MiB build. Binaries, source snapshots, build/source metadata, ordered execution logs, raw CSVs, and summaries are preserved under `F:/codex-tmp/lurq-text-bookkeeping/`. The comparison validates every expected action and identical ordered glyph counts. The new counters were checked across all 1,470 after-build CPU rows and 150 native rows, alongside applied budgets and retained-byte limits.

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
$env:LURQ_TEXT_CACHE_MIB = $null
./scripts/text-pipeline-metrics.ps1 -Variants workspace -Interactions -Samples 1 -OutputDirectory F:/codex-tmp/lurq-text-bookkeeping-local
cargo build -p lurq --example text_interaction_probe --features winit,dx12,perf_profile --locked
python scripts/compare-text-interactions.py F:/codex-tmp/lurq-text-bookkeeping/before.exe F:/codex-tmp/lurq-text-bookkeeping-local/workspace.exe F:/codex-tmp/lurq-text-bookkeeping-local/comparison --native-before F:/codex-tmp/lurq-text-bookkeeping/native-before.exe --native-after F:/codex-tmp/lurq-text-metrics-target/debug/examples/text_interaction_probe.exe --native-runs 6 --expect-incremental
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-text-bookkeeping-local/comparison
```

Validation passed: **99 focused tests** (37 glyph-engine, 8 runtime text, 22 scroll, 15 centering/layout, and 17 Markdown) plus **20 Criterion smoke cases without profiling**. Fresh Cosmic layouts match after mixed-direction edits, insertion/removal, splitting, reordering, repeated paragraphs, first/last-paragraph changes, CRLF changes, shrinking to empty text, width/style changes, and cache-pressure recovery. A forced fingerprint collision also cannot reuse different text. Native glyph counts agree; this run does not add a renderer pixel-comparison test.
