---
title: Paragraph Caret Reuse
description: Reusing paragraph-local caret geometry across edits without rebuilding a flat document vector.
---

> Historical benchmark report. Numbers and test counts below describe the dated source versions, toolchains, and fixtures in this report, not a new run of 0.20.0. See [Text Pipeline Optimization](../text-pipeline-optimization/) for the current implementation. Absolute artifact paths refer to the original local measurements and are not included in a fresh checkout.


## Results — September 12, 2026

Large multiline text now retains caret geometry per paragraph. An edit builds positions for the changed paragraph and reuses the others. The document stores references, byte offsets, and exact visual-row coordinates instead of copying every position into a new flat vector.

This follows [Caret and Selection Indexing](/lurq/text-selection-optimization/). The baseline already shares immutable caret results and uses indexed lookups; this iteration addresses the remaining extraction work on cache misses.

### Confirmed work reduction

In the native prop-edit fixture, the resulting document still has a median **112,426 caret positions**, with the same coordinates and byte offsets. Each edit now builds geometry for **1 paragraph**, reuses **1,727 paragraphs**, and extracts a median **329 paragraph-local positions**. The baseline walked the full layout to rebuild the flat document result. Final endpoint descriptors are outside the new paragraph-extraction counter.

All ordered glyph/caret counts match in 1,470 CPU and 150 native interaction rows per version. The independent selection comparison also matches all ordered geometry signatures and glyph/caret counts across 9,000 events per version. Extraction/reuse assertions pass on every edit and resize.

### Preliminary timings under compiler contention

| Measurement at 150% scale | Before, median | After, median |
| --- | ---: | ---: |
| CPU caret extraction after an edit | 2.58 ms | 0.66 ms |
| CPU selectable prop-edit pass | 6.34 ms | 3.94 ms |
| Native selectable prop edit | 10.59 ms | 7.73 ms |
| Native window resize | 35.08 ms | 37.89 ms |
| Native wheel scroll | 11.60 ms | 11.88 ms |

**These timing observations are preliminary.** Other projects compiled during the alternating runs, producing substantial tail outliers and uneven load. They do not isolate the exact speedup or establish whether the native resize difference is a regression. The raw observations remain in `comparison/` and `selection-comparison/`, with their full distributions.

A guarded repeat waited ten minutes for compiler/linker activity to stop and timed out before accepting its first sample. `guarded-comparison/blocked.json` records that outcome. Clean timing validation, especially for cold layout and resizing, remains outstanding. The work-count and geometry checks above do not depend on interpreting these wall-clock differences.

## What changed

Each retained `PlainParagraph` can hold an immutable `ParagraphCarets` allocation. It contains glyph-edge x coordinates and paragraph-local byte offsets, plus a range for each wrapped row. Exact paragraph matches carry that allocation through insertion, deletion, and movement. New or changed paragraphs build it from Cosmic's existing wrapped layout.

The document's `CaretPositions` contains segments pointing into those allocations. A segment supplies the paragraph's current byte offset and its visual row's y coordinate. These y values come directly from `Buffer::layout_runs`; the implementation does not reconstruct global y by adding cached paragraph heights, which could change floating-point rounding after edits. Lookup and selection iterators apply the offset only to the positions they read.

The visual-line index from the previous iteration remains in use. A mouse hit searches for its row; byte-offset lookup and Up/Down navigation scan the candidate paragraph slice. Selection painting visits candidate rows. None of these runtime paths materializes the whole document. A flat compatibility view exists only in tests for the legacy reference oracle.

Width changes invalidate paragraph caret geometry with the wrapped layout. Font/style changes and font-cache invalidation rebuild the corresponding geometry. Cache compaction drops the paragraph cache's references, while old caret snapshots can continue sharing their immutable allocations safely. The retained-buffer charge includes the new geometry and uses its known allocation capacities; it does not rescan every caret on buffer returns.

The incremental path applies to multiline source of at least 1,024 bytes, matching the existing retained paragraph-layout path. Smaller text and single-paragraph text keep flat storage. Masked inputs retain a private flat copy when remapping offsets. A segmented result can also detach a flat copy explicitly; its old shared geometry and index remain unchanged.

## Memory and scope

Charged retained storage during native edits rises from **32.51 to 34.39 MiB**, approximately **1.89 MiB** of additional charge. CPU edits at 150% rise from 33.66 to 35.55 MiB. The new paragraph geometry is included in this accounting, whereas the baseline's separate flat caret-result cache was outside the retained-buffer charge. These differences are not process-memory measurements.

The sampled after-build sequences remain below the 48 MiB budget without compaction, eviction, or oversized-entry bypass. Unchanged paragraph allocations are shared across document snapshots instead of copied into every new flat result.

The **48 MiB retained-buffer budget is unchanged**. It now charges geometry referenced by retained paragraphs as well as the existing public Cosmic allocations and keys. The separate result cache and live node snapshots may keep shared allocations alive after a retained-buffer entry is evicted. This budget remains a cache accounting limit, not a process RSS limit.

An edit still traverses paragraph metadata and assembles document-row references. It also performs the existing text-key hashing and paragraph matching. It no longer extracts glyph edges for unchanged paragraphs. Cold layouts and width changes still need all paragraph geometry.

## Method and validation

Windows 11, Ryzen 9 7950X3D, rustc 1.92.0-nightly (5c7ae0c7e, 2025-10-02), dev opt-level 0 with debug assertions and the existing eleven text dependency overrides. Release compiler settings and GPU rendering are unchanged.

The edit/resize/scroll probe uses the same 1,728-paragraph README-derived selectable document. Each sequence makes eight prop edits to the third paragraph, eight width changes, and eight scrolls. CPU runs include unchanged passes and test 100% and 150% scale. Native runs use a real DX12 window at 150% scale. Native action-to-paint includes event-loop handling and the renderer call, but not physical display latency. CPU pass timing starts after prop-update preparation.

Three alternating CPU blocks of five fresh sequences provide 120 samples per edit/resize/scroll phase, per version and scale. Six alternating native processes per version provide 48 samples per interaction. Samples within a sequence are related; p95 uses nearest rank. Correctness tests additionally cover edits at the first, middle, and last paragraph, splitting, insertion, deletion, movement, style changes, and cache pressure.

The separate selection probe exercises first clicks, drag selection, Up/Down, and Shift+Up/Down at the top, middle, and bottom of the document, at both scales. Its three blocks of five sequences provide 9,000 recorded events per version. Geometry signatures and glyph/caret counts must match in order.

`caret_built_paragraphs`, `caret_reused_paragraphs`, and `caret_built_positions` expose extraction work. `--expect-incremental-carets` validates one rebuilt paragraph per edit, reuse of every other paragraph, and full invalidation after a width change. The existing paragraph-shaping and allocation-accounting assertions also remain enabled.

**247 tests passed:** 129 library tests, 91 text/input tests, 4 synthetic-input tests, 15 text-centering tests, and 8 text-runtime tests. **23 Criterion smoke cases** also passed without `perf_profile`. New checks compare exact byte offsets and coordinate bits with a fresh Cosmic layout, preserve old snapshots through edits and cache pressure, and verify that selection queries do not materialize segmented geometry. Unicode fixtures include CRLF, blank lines, wrapping, ligatures, combining marks, emoji, and mixed RTL/LTR text.

## Reproduction

All saved binaries, source snapshots, build JSON, test logs, CSVs, and provenance are in `F:/codex-tmp/lurq-caret-paragraphs/`. The baseline binaries and source hashes match the final version from the preceding selection-indexing iteration.

Another project compiled during initial timing attempts. The comparison scripts now offer `--quiet-compilers`: a Windows Toolhelp observer polls known compiler/linker process names every 250 ms, stops only an affected benchmark child, preserves its discarded output, and retries after a quiet interval. Each accepted run has a `.quiet.json` audit. This excludes detected compiler overlap, but does not isolate all desktop activity or detect every very short compilation. No builds or tests from this task run during these measurements.

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
$env:PYTHONDONTWRITEBYTECODE = '1'
$env:LURQ_TEXT_CACHE_MIB = $null
python scripts/compare-text-interactions.py F:/codex-tmp/lurq-caret-paragraphs/before.exe F:/codex-tmp/lurq-caret-paragraphs/after.exe F:/codex-tmp/lurq-caret-repeat --native-before F:/codex-tmp/lurq-caret-paragraphs/native-before.exe --native-after F:/codex-tmp/lurq-caret-paragraphs/native-after.exe --native-runs 6 --selectable --expect-incremental --expect-incremental-carets --quiet-compilers
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-caret-repeat
python scripts/compare-text-selection.py F:/codex-tmp/lurq-caret-paragraphs/before.exe F:/codex-tmp/lurq-caret-paragraphs/after.exe F:/codex-tmp/lurq-caret-selection-repeat --quiet-compilers
```

## Remaining work

At this iteration, window resizing still rewrapped the full document. The follow-up [Stable Text Reflow](/lurq/text-stable-reflow/) preserves layouts and paragraph caret geometry when a width change leaves their wrap decisions unchanged. Limiting layout to a visible region remains a separate architectural change. DX12 still uses the full-atlas upload correctness workaround. Single very long paragraphs and mask remapping retain their existing linear work; this change targets edits within large multiline documents.
