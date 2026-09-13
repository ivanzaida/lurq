---
title: Dev Text Pipeline Profiling
description: Reproducible development-profile measurements of text shaping, alignment, rasterization, and caching.
---

## Results — September 12, 2026

This page records the investigation before the CPU optimizations. See [CPU Text Pipeline Optimization](/lurq/text-pipeline-cpu-optimization/) for the implemented changes, the current cache model, and a subsequent before/after comparison. “Current” below refers to the original eight-package preset, not the updated workspace manifest.

Measured on Windows 11, AMD Ryzen 9 7950X3D, with `rustc 1.92.0-nightly (5c7ae0c7e 2025-10-02)`, `x86_64-pc-windows-msvc`. Base commit: `e6efd085e392ca9aff346fc86b7c1f3fbe60d78f`, plus the diagnostic instrumentation in this change. README: 1,841 bytes / 72 lines; repeated text: 44,208 bytes. Fonts come from the installed system fonts and lurq's platform font setup.

Dependencies in the measured path: Cosmic Text 0.12.1, Rustybuzz 0.14.1, Swash 0.1.19, fontdb 0.16.2, and ttf-parser 0.20.0 / 0.21.1. The toolkit and benchmark remain at `opt-level = 0` with debug assertions in every variant.

After an initial ten-variant screen, five variants were repeated from saved executables, with no concurrent benchmark compilation: three blocks of ten fresh apps per case, variant order shuffled with seed `20260912`. The table reports the resulting **30-sample medians**, with nearest-rank p95 in parentheses, in milliseconds. These are local diagnostic samples on an active desktop, not isolated-machine performance budgets or statistical confidence intervals. The visible tail variance is a reason to avoid conclusions about small differences.

| Dev dependency optimization | README, 800px | Long text, fixed height | Long text in column flow | README, 20,000px |
| --- | ---: | ---: | ---: | ---: |
| All eight existing overrides set to 0 | 66.01 (69.74) | 857.24 (916.99) | 1,130.91 (1,173.71) | 80.46 (84.74) |
| Only the five requested packages at 2 | 13.88 (19.68) | 93.79 (121.60) | 118.25 (179.04) | 19.25 (26.83) |
| Current eight packages at 2 | 13.91 (19.81) | 80.60 (111.13) | 101.90 (148.27) | 19.00 (27.38) |
| Current + `zeno` at 2 | 13.19 (18.08) | 79.70 (111.08) | 99.77 (143.64) | 18.24 (24.61) |
| Current + raster dependencies at 2 | 8.05 (8.74) | 77.28 (79.45) | 97.13 (101.77) | 10.12 (11.51) |

“Five” means `cosmic-text`, `swash`, `rustybuzz`, `ttf-parser`, and `fontdb`. “Current” additionally includes `unicode-bidi`, `unicode-linebreak`, and `unicode-script`. “Raster dependencies” means `zeno`, `skrifa`, `read-fonts`, `font-types`, and `yazi`, optimized as a group. This experiment does not establish that every member of that group is needed.

The current overrides reduce cold long-text pass time by **10.6×**, and column-flow text by **11.1×**, compared with unoptimized dependencies. The five requested packages provide most of the improvement. The three Unicode overrides further reduce median long-text cost by about 14% in this workload; their README difference is within noise.

### Where the time goes

Inclusive median phase timings with the current eight overrides:

| Phase | README | Fixed-height long text | Column-flow long text |
| --- | ---: | ---: | ---: |
| Whole pass wall time | 13.91 ms | 80.60 ms | 101.90 ms |
| Layout | 2.92 ms | 0.035 ms | 36.02 ms |
| Plain text measurement/shaping inside layout | 1.06 ms | 0 ms | 35.90 ms |
| Glyph/render-command stage | 8.79 ms | 78.68 ms | 63.34 ms |
| Vertical bounds inside glyph stage | 3.48 ms | **51.76 ms** | **53.17 ms** |
| Buffer preparation/shaping inside bounds | 0.56 ms | 35.22 ms | 36.46 ms |
| Plain raster buffer preparation | 0.032 ms | 2.83 ms | 2.78 ms |
| Plain raster text replacement/shaping | 0.47 ms | 19.91 ms | 3.53 ms |
| Swash image generation, across bounds and paint | 5.70 ms / 284 requests | 4.21 ms / 209 | 4.27 ms / 213 |
| Atlas packing | 0.91 ms | 0.70 ms | 0.71 ms |

Both long-text cases calculate vertical bounds over **1,824 runs and 42,384 glyphs**. Only **765** glyph commands are emitted in the fixed-height viewport and **901** in column flow. Thus the viewport clip does not bound the alignment work. With all dependencies unoptimized, bounds shaping alone takes about **536 ms**, while Swash generation is about **8 ms**: the dominant long-text problem is shaping/layout work, not generating glyph images.

For README, optimizing the raster dependency group reduces Swash time from **5.70 ms to 0.71 ms** with the same **284 requests**, and the complete cold pass from **13.91 ms to 8.05 ms**. Atlas packing and layout change little. Optimizing `swash` alone does not optimize its separately compiled dependency crates. The grouped experiment is a useful next dev-profile candidate; the `zeno`-only result is too small/noisy to explain the grouped gain.

Other current-profile README costs are smaller: parsing **0.15 ms**, `App::new` **19.56 ms**, mounting **0.80 ms**, and the first pass after remounting a new tree into the same `App` **0.63 ms**. Cached pass medians are **0.19 ms** with current overrides and **0.30 ms** with all eight disabled; neither performs Swash requests. The severe stalls are cold/cache-miss work.

### Which requested packages matter

The initial screen disabled one override at a time while retaining the other seven. It used 30 samples per case in sequential runs; its absolute timings differ from the shuffled confirmation above. Use it for coarse attribution, not small rankings:

| Initial dev configuration | README cold | Fixed-height long text cold | Column-flow cold |
| --- | ---: | ---: | ---: |
| Current eight overrides | 17.87 ms | 112.25 ms | 139.26 ms |
| `cosmic-text` back to 0 | 30.01 ms | 269.58 ms | 376.20 ms |
| `rustybuzz` back to 0 | 29.68 ms | 317.27 ms | 413.78 ms |
| `ttf-parser` back to 0 | 27.29 ms | 350.84 ms | 459.35 ms |
| `swash` back to 0 | 27.25 ms | 139.64 ms | 166.31 ms |
| `fontdb` back to 0 | 15.36 ms | 97.69 ms | 124.93 ms |

Cosmic Text, Rustybuzz, and ttf-parser show large shaping regressions; Swash shows a stronger effect on glyph-heavy README rendering. This screen does not demonstrate a frame-time benefit for the fontdb override. Its apparently faster frame medians are not sufficient evidence to remove it: background load varied, and font database initialization and other font collections need their own controlled comparison. Effects are not additive because changing a dependency can affect code instantiated in its callers.

The confirmation CSVs and summary are in `target/text-metrics/confirm/`; per-block samples and logs are in `target/text-metrics/confirm-blocks/`. The initial screen is in `target/text-metrics/`. All compared variants emitted identical glyph counts per case and phase.

## Scope and reproduction

This investigation measures `text_pipeline` with **`--profile dev`**, the real component/layout/glyph pipeline, and a no-op renderer. It creates no native window and measures no GPU upload, presentation, event-loop latency, or whole desktop application startup. Normal `cargo bench` uses the optimized bench profile and cannot establish the cost of `opt-level = 0` development builds.

Run the package comparison on Windows with PowerShell 7:

```powershell
./scripts/text-pipeline-metrics.ps1 -Samples 30
python scripts/summarize-text-metrics.py
```

For only the two historical primary variants:

```powershell
./scripts/text-pipeline-metrics.ps1 -Variants unoptimized,current -Samples 30
```

Use `-Variants workspace` to measure the checked-out manifest. The historical presets explicitly set the additional raster dependency overrides to 0 unless that preset enables them, preserving the package comparison after changes to the manifest. Repeating these presets on new source measures the new algorithm; it does not reproduce the pre-optimization source in the tables above.

The script uses Cargo `--config profile.dev.package.<name>.opt-level=<value>` overrides. It does not edit manifests, release settings, or the global development optimization level. `target/text-metrics` contains each variant's raw CSV, build output, saved executable, command/toolchain/source metadata, and aggregate `summary.csv`.

To keep the build artifacts on F:, as in this investigation, set this before building:

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
```

To repeat a saved binary without compilation:

```powershell
$env:LURQ_TEXT_METRICS = 'target/text-metrics/current-repeat.csv'
$env:LURQ_TEXT_METRICS_SAMPLES = '30'
& ./target/text-metrics/current.exe
Remove-Item Env:LURQ_TEXT_METRICS, Env:LURQ_TEXT_METRICS_SAMPLES
python scripts/summarize-text-metrics.py
```

The CSV mode is also available through the existing bench target:

```powershell
$env:LURQ_TEXT_METRICS = 'target/text-metrics/manual.csv'
cargo bench -p lurq --bench text_pipeline --features markdown,perf_profile --profile dev --locked
Remove-Item Env:LURQ_TEXT_METRICS
```

Without `LURQ_TEXT_METRICS`, the existing Criterion suite works as before.

## What is measured

Every sample creates a fresh `App` and measures Markdown parsing, `App::new`, mounting, the cold pass, five cached passes, and a new tree's first pass using the same `App`. Teardown and CSV formatting occur outside the timed pass. Case order rotates between samples. The benchmark asserts that cold layout ran, glyphs were emitted, and cached passes perform neither layout recalculation nor Swash requests. The summarizer checks that emitted glyph counts agree between variants.

“Cold” means fresh application caches; it does not mean a flushed operating-system file cache. `app_init` includes font-system initialization, atlas allocation, and theme setup. The five cached samples within each app are consecutive observations, not independent process starts.

Use `wall_ms` for complete pass time. In particular, the cached render-list path's existing `FrameProfile.total` times only the renderer call, which is almost zero with a no-op renderer. Treating it as complete cached frame time would hide CPU work.

The workload is the checked-out README, so changing it changes the benchmark. The two original long-text cases repeat it 24 times. `long_text` is a fixed-height root; `flow_long_text` sits in a `Column`, whose layout requires its full intrinsic height. Both use the default center vertical alignment. The main viewport is 1200 × 800, text width 860; `readme_tall` uses a 20,000-pixel viewport. The subsequent optimization adds `unique_long_text`, which prefixes each source line with a unique copy/line number to prevent identical-paragraph reuse.

The timings inside `FrameProfile` are **inclusive**, not additive:

- `layout` includes plain-text measurement (`shape_text`) and rich shaping.
- `glyph_rasterize` includes vertical alignment, shaping for paint, rasterization, and command generation.
- `vertical_extents` includes buffer preparation/shaping (`vertical_extents_shape`), all glyph visits, and any glyph raster/atlas misses encountered while finding ink bounds.
- `swash_lookup` and `atlas_pack` include calls made from the bounds calculation as well as paint. Do not add them to `vertical_extents` to estimate total work.
- `rich_buffer_set_text` includes shaping: Cosmic Text 0.12.1 calls `shape_until_scroll` inside text setters. A small explicit `rich_cosmic_shape` timer does not mean shaping was cheap.

With `perf_profile`, the ordinary frame log now includes `text_bounds=...` with shaping duration, run count, and visited glyph count, plus `raster_buffer` and `raster_text` timers. The optimization also adds full-buffer hit/miss and reused-paragraph counts to that log. All new hot-path timers and counter updates are compiled out without that feature.

## Findings before optimization

At the start of the investigation, the root workspace already had the five requested dependency overrides, plus `unicode-bidi`, `unicode-linebreak`, and `unicode-script`. When embedding lurq in a separate desktop-shell workspace, the overrides must be in that shell's workspace-root manifest: Cargo ignores profiles in dependency manifests.

The original source path explained why clipping did not eliminate long-document work:

1. `Tree::pass` computes `text_vertical_align_offset` before invoking clipped rasterization.
2. `GlyphEngine::compute_text_vertical_extents` acquires an unbounded-height buffer, shapes the full text, and visits every glyph to calculate ink/optical bounds.
3. Normal-flow text separately runs `shape_and_measure` to obtain its exact layout height. The plain-text measurement cache stores `Size`, so the subsequent bounds calculation does not reuse a stored shaped layout.
4. Clipped painting then sets text again. For the fixed-height case, default center alignment moves the long document upward; Cosmic Text must shape the prefix leading to the visible middle of the document.

This led to reuse of plain shaped data and an optical bounds shortcut, described in the [optimization follow-up](/lurq/text-pipeline-cpu-optimization/). Exact flow height and center/bottom alignment still depend on full-document information; skipping that work without preserving those semantics would change layout.

The initial investigation only added measurements. The follow-up implements CPU optimizations and adds three dev-only font dependency overrides while preserving release profile settings.

## Initial investigation validation

The focused tests passed with `markdown,perf_profile`: 28 glyph-engine tests, 8 runtime text tests, 15 text-centering/layout tests, and 17 Markdown tests. They cover clipping, measurement, DPI wrapping, glyph generation, centering, cache reuse, and text reflow.

```powershell
cargo test -p lurq --features markdown,perf_profile --lib app::glyph_engine::tests --locked
cargo test -p lurq --features markdown,perf_profile --test runtime_tests text_ --locked
cargo test -p lurq --features markdown,perf_profile --test layout_tests text_centering --locked
cargo test -p lurq --features markdown,perf_profile --test markdown_tests --locked
```

The original Criterion entry point also passed all 16 smoke cases without `perf_profile`:

```powershell
cargo bench -p lurq --bench text_pipeline --features markdown --profile dev --locked -- --test
```
