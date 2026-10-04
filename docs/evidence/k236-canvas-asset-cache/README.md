# KONTUR-236: Canvas asset and shaped-text caches no longer collapse

Branch `codex/k236-canvas-asset-cache`, based on `origin/master` `1e13216`. Builder evidence, not independent QA.

The implementation is `3693f75`, tree `e7667f8905264ded44366642017f89f41296e852`. The logs in `runs/` name its pre-reword id `91b9eac`, which has the same tree; only the commit message was corrected.

## What Kontur measured

Kontur main on Lurq 0.34.1, the real 8,055-node design at Fit, DX12: pan and hover present one frame every 216–232 ms. Each rendered pass spends about 113 ms in Canvas asset upload with 313 textures created and 313 evicted, and about 65 ms in Canvas text with 684 shape-cache misses.

The pass samples in that capture also show the cause (`raw/pan-wheel.capture.json.gz` in the Kontur evidence). A pass makes 1,456 shape calls: 1,112 measures and 344 fills. It gets 772 hits, all repeats within the frame, and 684 misses, and it evicts 684 entries. The asset cache holds 1,016 entries charged 67.10 MB before the encode and 1,015 at 67.09 MB after it, with a 87.78 MB peak in between. The frame's own 344 assets (313 misses, 31 hits) are charged about 21 MB. So the asset budget was not the binding limit.

## Root causes

1. **Shaped text: a 256-entry least-recently-used list that collapses.** `canvas/text.rs` kept at most 256 entries and 8 MiB. It evicted the oldest entry first. A page drawn in the same order every frame with more than 256 keys therefore evicts each key just before the next frame asks for it, so every lookup misses (`frame 2 shaped 700 of 700 texts again` on master, below). The keys are stable across pan and hover: the shape scale is the device scale factor, and no part of a key changes while the camera pans at a fixed zoom. Zoom density is not the cause.
2. **Each re-shaped label became a new asset.** `shape_uncached` gave every rasterization a fresh `asset_id`. Each label shaped again was a texture the GPU cache could never hit, so the 313 textures created and 313 evicted per frame are this collapse seen from the GPU side. The evictions removed dead ids from earlier frames.
3. **The asset cache's own overflow behaviour** (`app/dx12_render/canvas.rs:325`, identical in WGPU). After every encode it evicted least-recently-used entries down to a fixed 64 MiB. When one frame's textures were charged more than that, it evicted a part of the frame just drawn. Every texture is charged at least 64 KiB, so this happens above 1,024 small textures. The next frame uploaded that part again. Master re-uploads the overflow every frame, not every texture: 76 of 1,100 images below.

## Policy

One frame-stamped policy (`canvas/frame_cache.rs`) now backs both caches:

- An entry used in the current or the previous frame is never evicted.
- Only entries that neither of the last two frames used are evicted, oldest first, at insertion and when a frame closes.
- When the current and previous frames alone need more than the budget, the cache stretches up to a ceiling of twice the budget.
- Anything that would go past the ceiling is drawn but not kept. The cached part stays resident instead of thrashing. An asset that is not kept is uploaded once per frame and retired when its frame ends.
- Once frames need less, the cache shrinks back to its budget.
- A frame ends when a GPU renderer encodes. Closing a frame with no use since the last close does nothing, so UI passes that draw no canvas do not age entries.
- Until a text engine sees its first GPU frame (for example, with software canvases only), it stays a plain least-recently-used cache within its budget.

Memory bound: the charged bytes of each cache never exceed its ceiling, which is twice the budget. Between frames they return to the budget unless the two latest frames used more (`frame_cache::tests::eviction_never_takes_an_entry_of_the_current_or_previous_frame_and_stays_bounded`, 398 randomized frames).

- **Assets** (`canvas/asset_cache.rs`, shared by WGPU and DX12): the budget is `CanvasAssetBudget`, 64 MiB by default and clamped to 1 GiB, so the ceiling is at most 2 GiB. Every texture is still charged at least 64 KiB.
- **Shaped text** (`canvas/text/shape_cache.rs`): lookups are hashed rather than a linear scan of up to 256 entries. The 256-entry cap is replaced by a 256-byte charge per entry within the existing 8 MiB budget, with a 16 MiB ceiling. A hit still compares the whole key with the same equality as before, so a hash collision is a miss. The keys and the shaped output are unchanged.

## API

```rust
use lurq::canvas::CanvasAssetBudget;
let engine = lurq::app::dx12_render::Dx12RenderEngine::new()
  .with_canvas_asset_budget(CanvasAssetBudget::new(128 * 1024 * 1024));
// WgpuRenderEngine::new().with_canvas_asset_budget(..) likewise.
```

`CanvasAssetBudget::{new, bytes, ceiling_bytes, DEFAULT_BYTES, MAX_BYTES}` and `Default` (64 MiB). The text budget is not configurable.

## Profiler (additive; existing names unchanged)

- `CanvasProfile` (`render.canvas.counts`, WGPU and DX12): `asset_cache_budget_bytes`, `asset_cache_stretch_bytes` (the most the cache was charged above its budget during the encode), `asset_cache_uncached`, `asset_cache_uncached_bytes` (drawn, not kept).
- `CanvasTextProfile` (`canvas_text.counts`): `shape_cache_uncached`, `shape_cache_stretch_bytes`.
- DX12 `asset_upload_details.cache_evictions` now also counts textures evicted while drawing to make room. `texture_creations`, `cache_misses`, `shape_cache_misses` and `asset_upload` are unchanged in meaning.

## Proof that master fails and the branch passes

The tests were committed first (`b6291f7`, probe `da78447`) on master's code. They were run there, and then on the implementation, `3693f75`.

| Test | Master code (`da78447`) | Branch |
| --- | --- | --- |
| `canvas::text::tests::a_page_with_more_keys_than_the_old_entry_limit_is_shaped_once` (CPU, 700 keys × 4 frames) | FAIL: `frame 2 shaped 700 of 700 texts again` | PASS |
| `wgpu_render::canvas::residency_tests::gpu_canvas_keeps_every_asset_of_a_frame_larger_than_the_budget` (1,100 images, 68.75 MiB charged, 4 frames) | FAIL: re-uploaded `[77824, 77824, 77824]` bytes (76 images per frame) | PASS: `[0, 0, 0]` |
| `wgpu_render::canvas::residency_tests::gpu_canvas_uploads_a_page_of_labels_once` (400 labels, 4 frames) | FAIL: re-uploaded `[687528, 687528, 687528]` bytes (every label) | PASS |

Logs: `runs/base-text-collapse.log`, `runs/base-gpu-residency.log`, `runs/branch-*.log`.

## Before and after: release probe

`app::canvas_frame_probe` (new, ignored; release; hidden window; real `Dx12RenderEngine` and `WgpuRenderEngine`) draws for 24 frames:

- a page of 344 labels, each measured three times and filled once (688 distinct keys, like the real page);
- a page of 1,100 small images.

It reports the first frame and the median of frames 5–24 from the same profiler records the Kontur harness reads.

```text
cargo test -p lurq --release --lib --locked --offline --features canvas,perf_profile,wgpu,dx12,screenshot canvas_frame_probe -- --ignored --nocapture --test-threads=1
```

Steady median per frame. Master is `da78447`, master's code with the tests; the branch is `3693f75`.

| Backend / page | Counter | Master | Branch |
| --- | --- | ---: | ---: |
| DX12 labels | shape misses / evictions | 688 / 688 | 0 / 0 |
| DX12 labels | text ms | 20.34 | 0.24 |
| DX12 labels | textures created / evicted | 344 / 344 | 0 / 0 |
| DX12 labels | asset upload ms | 137.10 | 0.28 |
| DX12 labels | uploaded KiB | 2,711.6 | 0 |
| DX12 labels | Canvas total ms | 138.43 | 0.48 |
| DX12 images | textures created / evicted | 76 / 76 | 0 / 0 |
| DX12 images | asset upload ms | 33.77 | 0.92 |
| DX12 images | Canvas total ms | 34.71 | 1.45 |
| DX12 images | `asset_cache_stretch_bytes` | (not on master) | 4,864 KiB |
| WGPU labels | shape misses | 688 | 0 |
| WGPU labels | text ms | 24.23 | 0.25 |
| WGPU labels | asset upload ms / uploaded KiB | 19.69 / 2,711.6 | 0.04 / 0 |
| WGPU labels | Canvas total ms | 28.78 | 0.86 |
| WGPU images | asset upload ms / uploaded KiB | 4.39 / 76 | 0.07 / 0 |
| WGPU images | Canvas total ms | 8.74 | 1.87 |

The images page stretches the asset cache by 4,864 KiB (1,100 × 64 KiB − 64 MiB) and keeps all of it. Nothing on either page goes past the ceiling (`assets_uncached` = `shapes_uncached` = 0); going past it is covered by the unit tests.

First frames are cold on both: DX12 labels have 688 misses and 344 textures, taking 243.2 ms on the branch and 362.8 ms on master. WGPU has no texture-creation counter and reports 0 there; its uploaded bytes show the same effect. Full output: `runs/base-probe.txt` and `runs/branch-probe.txt`.

Host: Windows 11 Pro 10.0.26200, NVIDIA GeForce RTX 5080 (driver 596.49), rustc 1.95.0. The probe is a bounded synthetic page, not the Kontur design. Other agents compiled and ran Kontur on the same machine for parts of the session. Every Cargo invocation was preceded by a process check. One branch debug test build, at about 00:02 local time, was started in parallel with its check, which showed a foreign build running. Each release probe run started after a clean check, but a foreign process may have started during a run. These are CPU wall-clock numbers, not FPS.

## Commands run at the final head

All were run with `CARGO_TARGET_DIR`, `TMP` and `TEMP` under the worktree's `.tmp/`, and `CARGO_BUILD_JOBS=4 --locked --offline`. Each log in `runs/` records head, tree and dirty state.

| Command | Result |
| --- | --- |
| `cargo test -p lurq --lib --features canvas,perf_profile,wgpu,dx12,screenshot` | 237 passed, 0 failed, 22 ignored |
| `cargo test -p lurq --lib --features canvas,perf_profile,serde,wgpu,dx12,screenshot` (export assertions) | 240 passed, 0 failed, 22 ignored |
| `cargo test -p lurq --lib --features canvas,perf_profile,wgpu,dx12,screenshot wgpu_render::canvas -- --ignored --test-threads=1 --skip canvas_gpu_performance` (GPU pixel parity, camera cache, residency) | 5 passed (`gpu_canvas_pixels_ordering_clips_resize_and_asset_reuse`, `gpu_canvas_effects_match_the_software_backend`, `gpu_canvas_camera_cache_pixels`, both residency tests). Adapter: RTX 5080, Vulkan |
| `cargo test -p lurq --features canvas --test canvas_tests` (CI) | 49 passed |
| `cargo test -p lurq --features canvas,perf_profile --lib` (CI) | 217 passed, 1 ignored |
| `cargo check -p lurq --features canvas,winit,wgpu,query,tokio` (CI) | ok. One warning in a touched file, the unused `profile_elapsed` import in `wgpu_render/canvas.rs` without `perf_profile`; that import line and its uses are unchanged from master |
| `cargo check -p lurq --lib --features mcp,canvas,dx12` and `mcp,canvas,perf_profile,dx12` | both ok, no warning in touched files (run at `681dc77`, tree `b05cbc80`; it differs from `3693f75` only in `wgpu_render/canvas.rs` and `residency_tests.rs`, which these builds do not compile) |
| `cargo check -p lurq --lib --features canvas` (no backend) | ok, no warning in touched files (`681dc77`, same reason) |
| `cargo clippy -p lurq --lib --tests --features canvas,perf_profile,wgpu,dx12,screenshot` | ok. No finding on a changed line; the findings in touched files are on unchanged lines (`dx12_render/mod.rs`, `wgpu_render/mod.rs`, `wgpu_render/canvas.rs:281`, and `large_enum_variant` on `profiler/model.rs:170` `SampleData`, whose `PassSample` the new counters make 48 bytes larger; not checked on master) |
| release `canvas_frame_probe` (above) | ok, numbers above |

Not run:

- Kontur on the real design.
- A DX12 assertion test of the Canvas renderer (none exists; DX12 is covered by compile checks and the release probe).
- macOS and Linux.
- The ignored `canvas_gpu_performance` and `canvas_camera_prepare_benchmark` probes.
- `scripts/check-docs.py` (it needs a docs build).

Rust files touched:

- Every new file is under 600 lines. The largest are `frame_cache.rs` at 206 and `canvas_frame_probe.rs` at 194.
- Four pre-existing files over 600 lines were touched without being split:
  - `dx12_render/mod.rs`: 4,918 → 4,942, for the budget builder and field;
  - `wgpu_render/mod.rs`: 3,320 → 3,335, likewise;
  - `wgpu_render/canvas.rs`: 1,047 → 1,037;
  - `canvas/mod.rs`: 1,403 → 1,408, for module declarations and re-exports.
