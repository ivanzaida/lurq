---
title: Stable Text Reflow
description: Preserving wrapped paragraph layouts and caret geometry while their line breaks remain unchanged.
---

> Historical benchmark report. Numbers and test counts below describe the dated source versions, toolchains, and fixtures in this report, not a new run of 0.20.0. See [Text Pipeline Optimization](../text-pipeline-optimization/) for the current implementation. Absolute artifact paths refer to the original local measurements and are not included in a fresh checkout.


## Change

Plain multiline text now retains a conservative width interval for each eligible paragraph. Resizing within that interval preserves both Cosmic's wrapped layout and the paragraph-local caret geometry. Only paragraphs outside their interval need wrapping and caret extraction again.

This follows [Paragraph Caret Reuse](/lurq/text-paragraph-carets/). Its baseline already reuses shaping across resizing, but discards every wrapped layout and every paragraph's caret geometry on each width change.

## Measurements

The final-source comparison uses a 1,728-paragraph README-derived document, with unique paragraph prefixes. It performs eight prop edits, eight successive width changes, and eight scrolls per sequence. CPU runs also include unchanged passes. At 150% scale, measurement and painting retain separate logical and physical layouts.

**Final timings were collected under external compiler/linker load.** The guarded repeat was stopped after roughly seven minutes waiting for a quiet interval, before accepting its first sample. The separately labeled `comparison-under-build-load/` alternates before/after binaries and completes all CPU/native sequences, but does not isolate an exact speedup from changing desktop load.

| Measurement | Before, median | After, median |
| --- | ---: | ---: |
| CPU resize, 100% scale | 12.96 ms | 5.84 ms |
| CPU resize, 150% scale | 19.45 ms | 9.42 ms |
| CPU wrapping during resize, 150% scale | 8.84 ms | 1.86 ms |
| CPU caret extraction during resize, 150% scale | 3.41 ms | 1.30 ms |
| Native DX12 window resize, 150% scale | 33.84 ms | 20.79 ms |
| Native prop edit | 6.63 ms | 6.03 ms |
| Native wheel scroll | 9.59 ms | 9.32 ms |

The stable work reduction is independent of timing noise. CPU resizing at 150% rebuilds a median **72 of 1,728 paragraphs** for the caret layout, reusing **1,656**. Across the logical and physical buffers together, layout calls fall from **3,456 to 144**. Native resizing rebuilds a median **84 paragraphs**, reuses **1,644**, and performs **168** layout calls across both buffers. Every resize reuses between 1,608 and 1,704 paragraph caret allocations in both probes.

Cold layout still builds all paragraphs and now also computes intervals. CPU cold-pass medians at 150% are 114.63 → 119.85 ms in the loaded run; the new interval-analysis timer accounts for 1.07 ms in the after build. The loaded CPU scroll median also varies (2.45 → 3.48 ms), while its reflow counters remain zero. These observations do not establish the size of a cold/scroll regression or improvement. Native cold time is dominated by startup and likewise is not a speedup claim. Clean timing validation remains outstanding.

An earlier exploratory pilot accepted a shorter CPU pair without detected compiler overlap and showed the same resize direction. Its after binary precedes a test-only addition/reordering, so the final-source table above uses the separately recorded final binaries. Raw pilot data is retained in `pilot/`; it is not pooled into the table.

Three alternating blocks of five sequences provide **120 resize samples per version and CPU scale**. Six alternating native processes per version provide **48 samples per interaction**. The complete ordered glyph/caret counts agree across **1,470 CPU and 150 native rows per version**, and all incremental shaping, caret, and reflow assertions pass. Samples within a sequence are related; CSV summaries also include nearest-rank p95. Native action-to-paint measures event-loop handling through the renderer call, not physical display latency.

The host is Windows 11 on a Ryzen 9 7950X3D, rustc 1.92.0-nightly, with dev opt-level 0, debug assertions, and the existing eleven text dependency overrides. No builds or tests from this task ran during benchmark sampling. Other projects' processes were never stopped or modified.

Saved binaries, raw samples, compiler-activity audits, build logs, test logs, and source hashes are under `F:/codex-tmp/lurq-text-reflow/`. The baseline comes from the final paragraph-caret iteration; its source hashes were verified before editing. `comparison/interrupted.json` records the interrupted guard, and `comparison-under-build-load/metadata.json` identifies the completed run and final binaries.

## Why reuse is valid

The fast path applies to an explicitly left-aligned paragraph with at most one LTR BiDi span. For these paragraphs, glyph x coordinates do not depend on the container width once wrap decisions are fixed. The paragraph may still have multiple visual rows, ligatures, combining characters, CJK text, emoji, or fallback fonts.

After a fresh layout, `ReflowRange` records the width bounds of Cosmic Text 0.12's word-fit decisions (unchanged in 0.19, which the reflow regression tests check against fresh Cosmic layouts). It uses Cosmic's shaped word advances, preserves floating-point addition order, and handles the trailing blank that Cosmic can allow beyond a row's limit. Successful comparisons establish an inclusive lower bound; unsuccessful comparisons establish an exclusive upper bound. At a subsequent resize, a constant-time bounds check can retain the entire paragraph without visiting its words or glyphs.

The interval is conservative. If a word requires glyph wrapping, the paragraph uses the existing Cosmic path. RTL and mixed-direction paragraphs, center/right/justified alignment, and unsupported numerical cases also use that path. Unwrapped eligible paragraphs can retain their layout across bounded and unbounded widths.

The optimization is scoped to the existing retained plain-text path: multiline source of at least 1,024 bytes. It does not change rich-text layout or introduce approximate wrapping. The complete document still has exact layout for measurement, scrolling, selection, and painting; this is not viewport virtualization.

Paragraph matching still requires identical text and line endings under compatible style and font keys. A changed paragraph receives a new interval after layout. A width outside the interval drops its layout and caret reference. Compaction drops intervals with wrapped layouts; rebuilding a compacted entry recomputes them. Font-cache invalidation clears the containing caches. Old shared caret snapshots remain immutable through edits and resizing.

## Cost and profiling

Each paragraph gains two floating-point bounds in an optional inline value. Including struct padding, the 1,728-paragraph fixture adds **27 KiB per retained buffer**, or **54 KiB** for logical and physical layouts together at 150% scale. This is included in the unchanged **48 MiB** retained-buffer charge. It is not a process-memory measurement, and separate result caches/live snapshots remain outside that budget.

All sampled before/after frames stay within that budget without cache compaction, eviction, or oversized-entry bypass.

Computing intervals scans shaped word advances after a new layout, so cold layout has additional work. Reused paragraphs do not repeat that scan. `plain_reflow_analysis` measures interval construction, and `plain_reflow_reuses` counts layouts preserved across width changes. The CPU and native CSV probes append `reflow_analysis_ms` and `reflow_reuses`; existing wrapping and caret counters still report actual work.

Release compiler settings, dependency versions, and GPU rendering are unchanged. DX12's full-atlas upload correctness workaround remains enabled.

## Correctness checks

Differential tests compare cached layouts with fresh Cosmic layouts at fractional widths, exact interval boundaries, and the adjacent representable floats. They cover multiple visual rows, whitespace, tabs, combining marks, ligatures, CJK, emoji, long words, unsupported alignment/BiDi cases, and unwrapped text. More than 1,000 accepted width/layout comparisons must agree.

A document-level test covers repeated resizing, a simultaneous edit and resize, paragraph movement, zero width, compaction, and old caret snapshots. It compares the complete layout and exact caret coordinate bits with fresh results and checks retained allocation charges. Existing text/input, selection, scaling, and font-invalidation tests remain enabled.

**251 distinct tests passed:** 133 library tests covered by the full run and the final reflow subset, 91 text/input/selection tests, 4 synthetic-input tests, 15 text-centering tests, and 8 text-runtime tests. **23 Criterion smoke cases** also passed without `perf_profile`. `git diff --check` passes. There is no pixel-capture comparison in this iteration; geometry is checked against fresh Cosmic layout and the native probes verify their rendered glyph counts.

## Reproduction

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
$env:PYTHONDONTWRITEBYTECODE = '1'
$env:LURQ_TEXT_CACHE_MIB = $null
cargo build -p lurq --bench text_pipeline --features markdown,perf_profile
cargo build -p lurq --example text_interaction_probe --features winit,dx12,perf_profile
python scripts/compare-text-interactions.py F:/codex-tmp/lurq-text-reflow/before.exe F:/codex-tmp/lurq-text-reflow/after.exe F:/codex-tmp/lurq-reflow-repeat --blocks 3 --samples 5 --native-before F:/codex-tmp/lurq-text-reflow/native-before.exe --native-after F:/codex-tmp/lurq-text-reflow/native-after.exe --native-runs 6 --selectable --expect-incremental --expect-incremental-carets --expect-stable-reflow --quiet-compilers
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-reflow-repeat
```

`--expect-stable-reflow` checks that every paragraph is either rebuilt or reused on each resize and that the fixture actually exercises reuse. It also checks existing incremental shaping and caret behavior. The compiler guard rejects detected compiler/linker overlap and only terminates its own benchmark child; it does not isolate all desktop load or detect every short compilation.
