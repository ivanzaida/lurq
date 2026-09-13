---
title: DX12 Atlas Upload Reuse
description: Reusing frame upload memory for glyph atlases without changing their pixels or transfer size.
---

## Results — September 13, 2026

DX12 now stages full glyph-atlas updates in its existing frame upload arena. A normal 4 MiB atlas needs one CPU copy into mapped memory, with no temporary atlas-sized vector and no new dedicated upload resource. The uploaded texture data and shader behavior remain the same.

This follows [Stable Text Reflow](/lurq/text-stable-reflow/). Once wrapping became cheaper, the atlas upload path accounted for much of the remaining scroll-frame time in the native document probe.

### Native measurements

| Measurement at 150% scale | Before, median | After, median |
| --- | ---: | ---: |
| Atlas preparation/upload during scroll | 3.90 ms | 0.25 ms |
| Native scroll action-to-paint | 8.69 ms | 3.72 ms |
| Atlas preparation/upload during resize | 2.99 ms | 0.22 ms |
| Native resize action-to-paint | 16.71 ms | 13.98 ms |
| New dedicated atlas-upload resources per sampled update | 1 | 0 |
| Bytes transferred per sampled update | 4,194,304 | 4,194,304 |

**Timings are observations under external compiler/linker load.** A guarded attempt waited 60 seconds and timed out before accepting its first sample. The completed comparison alternates six native processes per version, producing 48 samples each for edit, resize, and scroll. It does not isolate the exact speedup from other desktop activity. No builds or tests from this task ran during timing samples, and other projects' processes were never modified.

The upload timer measures CPU-side preparation and command recording, including resource allocation in the baseline. It is not a GPU timestamp of the texture copy. Native action-to-paint ends after the renderer call and does not measure physical display latency.

All 150 ordered rows per version have identical glyph counts, caret counts, display scale, and atlas transfer sizes. Incremental shaping, caret reuse, and stable-reflow assertions pass. Every sampled changed-atlas frame uses one dedicated staging resource before and one arena slice after; unchanged-atlas frames allocate neither. This work reduction does not depend on interpreting noisy timing differences.

Prop edits that do not change the atlas remain outside the optimized path: their observed medians are 5.22 → 5.51 ms. Native cold startup is essentially unchanged in this sample, 630.07 → 630.23 ms, with device/shader setup outside the targeted upload work. Neither is a claimed improvement. Clean timing validation remains outstanding.

## Implementation

Previously, every full update allocated a zeroed staging vector, copied atlas rows into it, created and mapped a dedicated D3D12 upload resource, and copied the vector again. That resource was retained until the frame fence completed.

The new path uses `upload_frame_bytes` for complete atlas data with already aligned rows. The common 1,024 × 1,024 atlas goes directly from the shared atlas bytes into mapped frame memory. Padded rows use `upload_frame_rows`. The copy command uses the returned resource and offset. Row pitches remain aligned to 256 bytes and texture offsets to 512 bytes, following [Microsoft's texture-upload requirements](https://learn.microsoft.com/en-us/windows/win32/direct3d12/upload-and-readback-of-texture-data).

The renderer already owns two 32 MiB frame arenas. It waits for the corresponding GPU fence before resetting and writing that frame's memory. This change adds no arena capacity and preserves that synchronization. If the arena cannot fit an upload, the existing dedicated-resource fallback retains the data until the frame completes. An aligned oversized atlas also avoids the old intermediate vector, even though it still needs a dedicated resource.

Row staging now explicitly zeros padding and missing source bytes. Reused mapped memory cannot expose a previous frame's texels when a public `GlyphAtlas` contains incomplete data. The shared row helper is also used by the existing image upload path; complete row contents retain their prior behavior.

`glyph_atlas_arena_uploads` and `glyph_atlas_dedicated_uploads` in `RenderProfile` expose which DX12 staging path ran. These counters describe upload resources, excluding atlas-texture creation when its dimensions change. The native probe appends both counters to its CSV. wgpu initializes them to zero because it uses a different upload mechanism.

The full-atlas correctness workaround remains enabled. Partial dirty-rectangle uploads still have a separate missing-glyph bug to investigate; this change reduces CPU allocation/copy work while transferring the same full atlas. Release compiler settings, dependencies, atlas texture format, and shaders are unchanged.

## Pixel and build validation

The new `dx12_glyph_atlas_probe` renders synthetic color and alpha-coverage glyphs through the actual DX12 shader and swapchain. It tests aligned rows, padded rows, a 4 MiB atlas, an atlas larger than the 32 MiB arena, truncated data, and empty data. Updates change the atlas contents, reuse frame slots, change dimensions, and sample different atlas regions. Unchanged frames must skip upload.

Each version produced **24 captures**. Their complete RGBA signatures and PNG bytes match exactly. Independent pixel assertions check opaque and transparent texels, and the after probe verifies arena use for ordinary cases and dedicated-resource fallback for the oversized case. These are controlled texture patterns; the separate native document probe exercises real text and checks rendered counts.

**Four DX12 tests passed**, including shader compilation with image support and a new row-copy test covering 18 combinations of row alignment and truncated/full/extra source bytes. The latter checks missing-byte clearing and guards both ends of the destination allocation.

Both `cargo check -p lurq --features winit,dx12` without profiling and `cargo check -p lurq --features wgpu,perf_profile` pass. The screenshot/DX12 and native-window probes build and run successfully. `git diff --check` passes.

## Reproduction and provenance

Artifacts are in `F:/codex-tmp/lurq-atlas-upload/`: before/after binaries and source snapshots, build/check/test logs, the 48 PNGs, raw native CSVs and summaries, pixel validation, and source/binary hashes. The baseline is the preceding reflow implementation with the same new profiling counters and probes, retaining the old staging algorithm. Both versions use dev opt-level 0 with debug assertions and the existing text-dependency overrides on Windows 11 / Ryzen 9 7950X3D / rustc 1.92.0-nightly.

```powershell
$env:CARGO_TARGET_DIR = 'F:/codex-tmp/lurq-text-metrics-target'
$env:CARGO_BUILD_JOBS = '4'
$env:PYTHONDONTWRITEBYTECODE = '1'
cargo build -p lurq --example dx12_glyph_atlas_probe --features screenshot,dx12,perf_profile
cargo build -p lurq --example text_interaction_probe --features winit,dx12,perf_profile
F:/codex-tmp/lurq-atlas-upload/capture-before.exe F:/codex-tmp/atlas-pixels-before
F:/codex-tmp/lurq-atlas-upload/capture-after.exe F:/codex-tmp/atlas-pixels-after --expect-arena
python scripts/compare-glyph-atlas.py F:/codex-tmp/lurq-atlas-upload/native-before.exe F:/codex-tmp/lurq-atlas-upload/native-after.exe F:/codex-tmp/atlas-native-repeat --runs 6 --quiet-compilers --quiet-timeout 60
```

Compare `frames.csv` pixel signatures and the corresponding PNGs between the two capture directories. The native comparison verifies geometry counts, transfer sizes, and staging-resource counts automatically. Omit `--quiet-compilers` to record a separately labeled run under background load. Existing comparison tools retain their original ten-minute guard timeout; the new native-only driver defaults to a one-minute limit when the guard is requested.
