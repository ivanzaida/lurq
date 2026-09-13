---
title: CPU Text Pipeline Optimization
description: Measured improvements to cold development-build text layout, optical alignment, and glyph generation.
---

## Results — September 12, 2026

This page records the first cold-pass optimization. The subsequent [Text Interaction Optimization](/lurq/text-interaction-optimization/) adds reuse across edits and resizing, cache compaction for fractional DPI, and native DX12 measurements.

The memory discussion below describes that initial 32 MiB budget. The later [Text Cache Budget](/lurq/text-cache-budget/) experiment raises the current default to 48 MiB to retain wrapped layouts at fractional display scales.

Cold text passes now reuse plain shaped buffers across measurement, vertical alignment, and painting. Center alignment visits the first paintable glyph of each line instead of rasterizing every glyph to find bounds. The workspace also optimizes Swash's `skrifa`, `read-fonts`, and `font-types` dependencies at level 2 in development builds. Release profile settings are unchanged.

These changes follow the [development-profile investigation](/lurq/text-pipeline-dev-profiling/). The comparison below uses the original eight dev package overrides as its baseline, so the gains are additional to the large improvement from optimizing Cosmic Text and Rustybuzz in the first place.

| Cold CPU pass | Before, median (p95) | After, median (p95) | Median speedup |
| --- | ---: | ---: | ---: |
| README, 800px viewport | 18.13 (25.91) ms | 8.64 (14.16) ms | 2.1× |
| Repeated long text, fixed height | 106.68 (130.66) ms | 17.20 (28.06) ms | 6.2× |
| Repeated long text in column flow | 143.18 (167.80) ms | 17.90 (31.08) ms | 8.0× |
| README, 20,000px viewport | 24.63 (31.76) ms | 10.70 (16.14) ms | 2.3× |
| Unique long text, fixed height | 139.53 (172.44) ms | 57.70 (73.68) ms | 2.4× |

Cached pass medians after the change range from 0.17 to 0.37 ms across these cases, with no Swash requests. Unique long text still has a substantial cold layout cost; these results do not establish a 60 FPS guarantee for large new documents.

### Method

Windows 11, AMD Ryzen 9 7950X3D, `rustc 1.92.0-nightly (5c7ae0c7e 2025-10-02)`, `x86_64-pc-windows-msvc`. Both executables use `cargo bench --profile dev --features markdown,perf_profile`; lurq and the benchmark have optimization level 0 and debug assertions. The baseline is commit `e6efd085e392ca9aff346fc86b7c1f3fbe60d78f` with only the current benchmark harness and profile field declarations overlaid. Its glyph engine, runtime, and package profile settings remain at that commit. The after executable includes this change and the three additional dev overrides.

Saved executables ran sequentially, without compilation during sampling: three blocks of ten fresh apps per case, alternating before/after order between blocks. Each app also produces five cached samples. The table uses 30 cold samples per case and nearest-rank p95. This was an active desktop, so absolute timings and tails vary with background load. Earlier exploratory runs were faster for both versions; do not compare their absolute times with this table to attribute individual changes.

The harness exercises the real component, layout, and glyph-command pipeline with a no-op renderer. It measures CPU pass wall time, excluding application construction and mounting. It creates no native window and measures no GPU upload, presentation, event-loop latency, or complete desktop-shell startup. Fonts use the system collection and lurq's platform defaults. Rendered glyph counts agree across both binaries for every case and phase; separate regression tests compare layout and glyph geometry.

The README is 1,841 bytes / 72 lines. Repeated long text contains 24 copies, totaling 44,208 bytes. The unique control prefixes every line with a distinct copy/line number, preventing identical-paragraph reuse. It is also slightly larger and renders a different glyph set, so compare each workload against its own baseline. Both long-text roots retain default center alignment; the column case also requires exact full content height. The regular viewport is 1200 × 800, with 860px text width.

### What changed

1. **Share full plain-text layouts.** Measurement and alignment retain their fully shaped Cosmic buffers for subsequent clipped painting. Keys compare complete text, font/style, width, and wrapping. Finite-height raster buffers remain separate, so a clipped prefix cannot be mistaken for a complete layout.
2. **Reuse identical complete paragraphs within a buffer.** For multiline text of at least 1,024 bytes, a local lookup remembers up to 256 paragraphs. Only identical text and line endings share cloned shaped/layout data, under the same attributes and constraints. Reuse preserves the complete paragraph's bidirectional context. Unique paragraphs still receive full shaping.
3. **Calculate only the bounds center alignment needs.** The existing optical rule uses the first paintable glyph's cap height and the last nonempty line's baseline. The new path stops after finding a paintable glyph on each line. Fonts without plausible cap metrics fall back to the exact ink scan; top and bottom alignment retain that scan as well.
4. **Discard old pooled text before changing buffer settings.** Cosmic setters can immediately reshape previous contents. Clearing text first removes work on data about to be replaced.
5. **Optimize three additional font dependencies in dev builds.** A 15-sample screen found the `skrifa` / `read-fonts` / `font-types` group matched the broader group that also optimized `zeno` and `yazi`. `skrifa` alone did not reproduce the gain. This supports keeping the three-package group; it does not identify a benefit for every individual member.

### Remaining CPU costs

These are inclusive median timers from the final after run. Sub-timers overlap and must not be summed.

| Phase | README | Repeated fixed-height text | Repeated column-flow text | Unique fixed-height text |
| --- | ---: | ---: | ---: | ---: |
| Plain layout measurement/shaping | 1.25 ms | 0 ms | 11.71 ms | 0 ms |
| Vertical alignment bounds, including preparation | 0.74 ms | 12.89 ms | 1.41 ms | 52.80 ms |
| Full-buffer preparation inside bounds | 0.44 ms | 11.55 ms | 0.004 ms | 50.89 ms |
| Additional raster text setup/shaping | <0.01 ms | <0.01 ms | <0.01 ms | <0.01 ms |
| Swash image generation, across bounds and paint | 0.79 ms | 0.56 ms | 0.59 ms | 0.61 ms |

The earlier instrumented baseline visited 42,384 glyphs across 1,824 layout runs for repeated long-text bounds. The new optical path visits 2,352 glyphs across those same runs. The column case shapes during measurement and reuses that result for bounds and paint. Unique text needs a full layout once, but painting no longer shapes its visible prefix again.

The baseline executable used in the final comparison does not populate the newly added bounds/raster diagnostic fields. Its zero values in those columns mean uninstrumented, not zero work; use the earlier investigation for baseline phase attribution.

### Memory tradeoff

The plain buffer cache retains at most 64 entries and uses a 32 MiB charged-storage budget per `GlyphEngine`, evicting the oldest entries first. A hit refreshes the entry's age. The charge accounts for public Cosmic line, shape, and layout allocations plus the owned text key; Cosmic's private scratch storage, allocator overhead, and font allocations are not fully exposed. This is an estimate for cache eviction, not a hard process-memory limit. Other existing text and atlas caches remain separate.

Large entries exceeding that budget are dropped after use and may need shaping again. A 16 MiB trial failed to retain the unique control's layout and incurred a second shaping pass, which motivated the 32 MiB budget. Font installation and alias changes clear retained layouts and optical bounds. Profiling memory estimates include the retained-buffer charge.

## GPU rasterization

The existing WGPU and DX12 renderers already composite glyphs from an atlas on the GPU; Swash produces the glyph images on the CPU. This change leaves those renderers and glyph rendering rules intact.

After the CPU changes, Swash takes about 0.6–0.8 ms in the regular viewport cases, versus about 51 ms preparing the unique document's full layout. Moving glyph image generation to the GPU would target that smaller component and would not remove CPU shaping, wrapping, or the need for exact full-document information in center alignment and column flow. No GPU raster backend was implemented or benchmarked here. Further long-document work should focus on incremental layout or an explicit viewport layout contract, with separate measurements for actual upload/presentation costs.

## Reproduction and validation

Keep build and measurement artifacts on F: in PowerShell 7:

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
./scripts/text-pipeline-metrics.ps1 -Variants workspace -Samples 30 -OutputDirectory F:/codex-tmp/lurq-text-metrics
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-text-metrics
```

`workspace` follows the manifest as checked out. The script's historical `current` preset pins only the original eight optimized dependencies; `font-raster-deps` adds the three retained font dependencies to that preset. These package presets change compilation settings, not the source algorithm. A before/after algorithm comparison requires building both source versions with the same benchmark harness.

The local comparison is preserved in `F:/codex-tmp/lurq-text-final/`: `before.exe`, `workspace.exe`, `compare.py`, per-block CSV/logs in `blocks/`, and merged CSVs, summary, build metadata, and executable SHA-256 hashes in `comparison/`. The detached baseline checkout is `F:/codex-tmp/lurq-text-baseline`. To repeat the saved pair:

```powershell
python F:/codex-tmp/lurq-text-final/compare.py
python scripts/summarize-text-metrics.py F:/codex-tmp/lurq-text-final/comparison
```

Validation passed: 32 glyph-engine tests, 8 runtime text tests, 15 centering/layout tests, and 17 Markdown tests. New coverage compares full layout runs against the original Cosmic setup with repeated/unique paragraphs, mixed scripts, combining characters, CRLF, wrapping, and alignment; compares optical bounds against the full ink scan; compares cached versus uncached clipped glyph output; and exercises count/byte eviction and font invalidation.

```powershell
cargo test -p lurq --features markdown,perf_profile --lib app::glyph_engine::tests --locked
cargo test -p lurq --features markdown,perf_profile --test runtime_tests text_ --locked
cargo test -p lurq --features markdown,perf_profile --test layout_tests text_centering --locked
cargo test -p lurq --features markdown,perf_profile --test markdown_tests --locked
cargo bench -p lurq --bench text_pipeline --features markdown --profile dev --locked -- --test
```

The final command passed all 17 Criterion smoke cases without `perf_profile`, including the new unique-paragraph control. It verifies that the normal benchmark entry point and feature-disabled build still work; it is not a full statistical Criterion run or a GPU visual test.
