---
title: Text Pipeline Optimization
description: Benchmark notes and improvement log for text layout and rasterization.
---

This page tracks text pipeline benchmark results and optimization work.

## Benchmark

The primary benchmark is `text_pipeline`, which renders the workspace root `README.md` through the real `Markdown` component. This keeps the workload close to a document-heavy app path: Markdown parsing, Markdown rendering, rich text layout, glyph rasterization, atlas population, and render-list generation.

Run it with:

```powershell
cargo bench -p lurq --bench text_pipeline --features markdown
```

For quick local checks while iterating:

```powershell
cargo bench -p lurq --bench text_pipeline --features markdown -- --sample-size 10 --warm-up-time 1 --measurement-time 2
```

The short command is useful for direction, but final claims should use the normal Criterion run.

## Current Results

Short-run results from June 16, 2026:

| Case | Baseline | After rich text raster cache | After atlas snapshot reuse | After shared shaped layout | After borrowed key lookup | After display-text skip |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `parse_readme_markdown/32` | - | - | - | - | ~6.6 us | ~6.4 us |
| `cold_readme_markdown_first_pass/32` | ~4.40 ms | ~4.58 ms | ~5.11 ms | ~4.47 ms | ~4.41 ms | ~4.94 ms |
| `warm_readme_markdown_cached_pass/32` | ~581 us | ~202 us | ~27.8 us | ~29.9 us | ~26.4 us | ~28.3 us |
| `parse_readme_markdown/128` | - | - | - | - | ~11.4 us | ~10.1 us |
| `cold_readme_markdown_first_pass/128` | ~6.25 ms | ~6.84 ms | ~6.52 ms | ~6.38 ms | ~5.97 ms | ~6.19 ms |
| `warm_readme_markdown_cached_pass/128` | ~1.35 ms | ~257 us | ~51.0 us | ~65.0 us | ~48.9 us | ~49.1 us |
| `parse_readme_markdown/all` | - | - | - | - | ~11.7 us | ~9.7 us |
| `cold_readme_markdown_first_pass/all` | ~6.76 ms | ~6.67 ms | ~6.59 ms | ~6.29 ms | ~5.93 ms | ~6.12 ms |
| `warm_readme_markdown_cached_pass/all` | ~1.28 ms | ~248 us | ~51.1 us | ~60.9 us | ~48.4 us | ~49.3 us |

## Improvement Log

### Rich Text Raster Cache

Added a `rich_glyph_layout_cache` in `GlyphEngine`.

The cache key includes rich text spans, style data, width, wrapping, and raster mode. The cached payload stores glyph atlas coordinates and per-glyph color, allowing warm Markdown render-list passes to append glyph commands without reshaping rich text.

Validation:

```powershell
cargo test -p lurq --lib app::glyph_engine::tests
cargo test -p lurq --features markdown --test markdown_tests
cargo bench -p lurq --bench text_pipeline --features markdown --no-run
```

### Removed Cache-Miss Clones

Plain and baked-transformed text layout cache misses now append commands from the newly built layout, then move that layout into the cache. This removes a `Vec` clone on cache misses.

### Atlas Snapshot Reuse

Changed `GlyphAtlas` to hold shared immutable byte data and changed `AtlasPacker` to reuse an unchanged atlas snapshot. Warm frames now clone an `Arc<[u8]>` handle instead of cloning the full atlas byte buffer.

This moved the short-run `warm_readme_markdown_cached_pass/all` case from about 248 us to about 51 us.

### Shared Rich Text Shaped Layout

Added a shaped rich text layout cache populated by `measure_rich_text`. The following snapped rich text raster pass can now pack atlas glyphs from cached shaped glyph positions instead of shaping the same rich text block again.

This improved the short-run `cold_readme_markdown_first_pass/all` case from about 6.59 ms to about 6.29 ms. The same short run showed `warm_readme_markdown_cached_pass/all` moving from about 51 us to about 61 us, so this change primarily helps first render and relayout rather than the warm render-list path.

### Borrowed Rich Text Cache Lookup

Changed the rich text shaped-layout and packed-glyph caches to use fingerprint buckets with exact borrowed span comparison. Hits no longer need to clone span text into an owned key. Owned keys are still stored on insert, so fingerprint collisions are resolved by exact comparison.

This moved the short-run `cold_readme_markdown_first_pass/all` case from about 6.29 ms to about 5.93 ms and moved `warm_readme_markdown_cached_pass/all` from about 61 us to about 48 us.

### Atlas Dirty Rect Tracking

Added dirty rect tracking to the glyph atlas snapshot. `AtlasPacker` records packed regions, `GlyphAtlas` carries those regions, and the WGPU and DX12 renderers upload dirty subrectangles when the atlas size is unchanged. Texture creation and resize still use full-atlas uploads.

The README Criterion benchmark uses a no-op render engine, so this optimization is not reflected in the CPU-only benchmark table above.

### Atlas Upload Instrumentation

`FrameProfile` now reports glyph atlas upload bytes, upload rect count, and full-atlas upload count. WGPU records the actual source byte range sent through `queue.write_texture`; DX12 records padded upload-buffer bytes, including row-pitch padding required by the copy footprint.

This gives renderer-facing visibility for the dirty-rect path without changing the no-op README benchmark.

### Atlas Upload Probe

The demo app includes an `Atlas Upload Probe` route that warms the atlas with ASCII text, then introduces Latin extensions, symbols, Cyrillic, Greek, currency signs, and arrows over timed updates. This gives the renderer a live scenario where new glyphs are packed after the atlas already exists.

Run it with profiling enabled:

```powershell
cargo run -p demo --features perf_profile -- --atlas-upload-probe --renderer wgpu --profile-log
```

For DX12 on Windows:

```powershell
cargo run -p demo --features perf_profile -- --atlas-upload-probe --renderer dx12 --profile-log
```

The default log path is `target/perf_profile.log`. Look for `atlas=<bytes>B <rects> rects <full> full` in presented frames. Warm steady frames should report zero atlas upload bytes; timed glyph-introduction frames should report dirty rect uploads instead of repeated full-atlas uploads.

DX12 probe run from June 16, 2026:

| Frame | Atlas Upload | Upload Rects | Full Uploads | Note |
| --- | ---: | ---: | ---: | --- |
| initial | 1,048,576 B | 1 | 1 | texture creation/full upload |
| steady warm | 0 B | 0 | 0 | no new glyphs |
| Latin/math update | 158,208 B | 22 | 0 | dirty rects only |
| Cyrillic/arrows update | 150,016 B | 24 | 0 | dirty rects only |
| mixed update | 183,552 B | 25 | 0 | dirty rects only |

### Dirty Rect Coalescing

Added a same-atlas-row coalescing pass before `GlyphAtlas` snapshots expose dirty rects to renderers. Adjacent or near-adjacent glyph dirty rects on the same atlas row are merged when the merged area stays within a conservative waste threshold. Separate atlas rows are not merged.

DX12 probe after coalescing:

| Frame | Before | After | Upload Rects Before | Upload Rects After |
| --- | ---: | ---: | ---: | ---: |
| Latin/math update | 158,208 B | 27,904 B | 22 | 2 |
| Cyrillic/arrows update | 150,016 B | 23,808 B | 24 | 1 |
| mixed update | 183,552 B | 39,424 B | 25 | 2 |

All three glyph-introduction frames still reported `0` full-atlas uploads.

WGPU probe after coalescing:

| Frame | Atlas Upload | Upload Rects | Full Uploads |
| --- | ---: | ---: | ---: |
| initial | 1,048,576 B | 1 | 1 |
| Latin/math update | 70,164 B | 2 | 0 |
| Cyrillic/arrows update | 31,278 B | 1 | 0 |
| mixed update | 76,469 B | 2 | 0 |

WGPU uses the atlas slice directly with atlas-width row stride for dirty uploads, so its byte counter represents the source span passed to `queue.write_texture`. DX12 builds compact padded upload buffers per rect, so its byte counter is not directly comparable.

### Markdown Parse Baseline

Added `parse_readme_markdown` cases to the `text_pipeline` Criterion benchmark. The full README parses in about 10 us in short local runs, while the cold full Markdown pass remains around 6 ms. This confirms the remaining cold-path cost is layout, shaping, and glyph rasterization rather than Markdown parsing.

### Non-Selectable Rich Text Display Text

Rich text layout no longer concatenates spans into `TextState::display_text` unless the rich text is selectable. Markdown still paints from the rich text spans directly, and selectable rich text still builds display text for caret and selection behavior.

In the short README run, this moved `cold_readme_markdown_first_pass/all` from the previous measured ~6.40 ms run to ~6.12 ms. The parser-only baseline remained around 10 us.

### Glyph Engine Profiling

Added fine-grained `perf_profile` counters inside `GlyphEngine` for plain text shaping, rich text shaping, packing shaped rich text into atlas glyphs, Swash glyph image lookup, atlas packing, and cached command append time.

Run the README workload with a one-shot text profile:

```powershell
$env:LURQ_TEXT_PROFILE = "1"
cargo bench -p lurq --bench text_pipeline --features "markdown perf_profile" -- --sample-size 10 --warm-up-time 1 --measurement-time 2
```

The June 16, 2026 profile for the full README cold pass before whitespace-cluster skipping showed:

| Counter | Value |
| --- | ---: |
| rich text shaping | ~2.0 ms |
| rich shaped packing | ~2.7 ms |
| Swash lookup | ~2.6 ms / 560 requests |
| atlas packing | ~0.08 ms / 386 packs |
| cached append | ~0.12 ms |

This points the remaining cold-path work at rich text shaping and Swash glyph image generation. Atlas packing and command append are not currently the dominant costs.

### Rich Shape Phase Profiling

Split the rich text shape profile into buffer acquire, rich text setup, Cosmic shaping, measurement, and glyph extraction. The README profile showed that the expensive part of `rich_shape` was loading rich text into the Cosmic buffer, not shaping:

```text
rich_shape=2.20ms(acq=0.11 set=1.91[prep=0.00 buffer=1.90 align=0.00] cosmic=0.06 measure=0.00 extract=0.11)
```

This means optimizing local span preparation is not enough; the remaining cold setup cost is mostly inside the buffer text loading path.

### Single-Span Rich Text Fast Path

Rich text nodes with exactly one span now delegate measurement and rasterization to the plain text paths. This preserves visual behavior for single-style text blocks while avoiding the rich shaped-layout and rich glyph-layout caches entirely.

The README profile before this delegation showed that most rich loads were single-span:

```text
rich_shape=2.19ms(... loads=59/55+4 spans=87 bytes=1792)
```

After delegating single-span rich text to the plain text path:

```text
text shape=0.88ms rich_shape=0.96ms(acq=0.00 set=0.88[prep=0.00 buffer=0.87 align=0.00] cosmic=0.02 measure=0.00 extract=0.05 loads=4/0+4 spans=32 bytes=853)
```

This removed 55 rich text loads from the README cold pass and reduced glyph-engine memory in the profile from about 170 KiB to about 112 KiB. The single-span work now appears in the plain `text shape` bucket, but total text setup still moves down modestly.

### Whitespace Cluster Raster Skip

Rasterization now skips shaped glyph clusters whose source text is entirely whitespace before asking Swash for a glyph image. Whitespace still participates in shaping, wrapping, measurement, caret positions, and selection geometry; it just does not enter the atlas/raster path because it does not paint.

The README profile moved Swash requests from `560` to `386`, matching the number of successful atlas packs:

```text
swash=2.57ms/386 atlas_pack=0.06ms/386
```

The short Criterion run did not show a clear cold-frame timing win, but this removes known non-painting Swash calls without retaining miss-cache state.

### Empty Glyph Miss Cache Experiment

Tried caching glyph cache keys that produced no Swash image or zero-sized placements. In the README profile this reduced Swash requests from 560 to 394, but did not reduce measured Swash time.

The experiment was not kept. The stateless whitespace-cluster skip covers the repeated space-glyph misses in the README workload without adding persistent miss sets. The next optimization should target expensive successful glyph image generation or reduce repeated rich shaping.

## Current Storage Model

Cosmic text layout runs are not stored directly. They are produced from `Buffer::layout_runs()` while measuring or rasterizing, then discarded when the buffer returns to the pool.

The engine stores derived glyph layout data:

- plain text: `CacheKey -> Vec<CachedGlyph>`
- rich text: `RichTextCacheKey -> Vec<CachedRichGlyph>`
- baked transformed text: `CacheKey -> Vec<CachedTransformedGlyph>`

Markdown input stores rich text spans. It does not store shaped runs.

## Next Candidates

1. Reduce successful Swash glyph image generation cost; after whitespace skipping, Swash requests match successful atlas packs.
2. Decide whether WGPU should keep zero-copy atlas-slice dirty uploads or build compact per-rect row buffers like DX12.

## Updating This Page

When changing the text pipeline:

1. Run the focused tests.
2. Run `text_pipeline` with the same command before and after the change.
3. Add the benchmark date, command, and main numbers to this page.
4. Note whether the result affects cold first render, warm cached render, or both.
