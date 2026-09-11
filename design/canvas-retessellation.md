# Canvas camera tessellation cache

Implemented on `codex/canvas-retessellation`, based on lurq 0.19.0 (`d243196`). This addresses the toolkit work in `lurq-canvas-retessellation-handoff.md`; it does not change Kontur's paint scheduling.

## Behavior

GPU path and clip commands retain an immutable model-space path plus the transform captured by the drawing call. `Path2D` lazily retains one finished snapshot, invalidating it on mutation. Already queued drawings and other clones retain their previous geometry.

Both WGPU and native DX12 own a persistent CPU mesh cache. Its key includes the full path content, fill rule and curve tolerance. Precomputed hashes make reused `Path2D` lookups cheap; equality also checks geometry so hash collisions cannot return the wrong triangles. Identical rebuilt paths can hit the same cache. Color, opacity, erasure and clip stacks remain per-draw state.

Transforms are applied to cached triangle vertices during CPU preparation. The existing GPU vertex format, shaders, tiling, stencil clipping and upload path remain in use. This removes repeated tessellation and record-time path transformation, but still assembles and uploads vertices each frame. It does not add a retained-scene API, shader camera uniforms or shape-level damage tracking. Applications benefit most by retaining their `Path2D` objects and applying the camera through `Context2D::set_transform`; the current path still captures transforms when its segments are added.

## Zoom, strokes and limits

Polygon meshes are independent of zoom. For curves, the cache rounds the transform's largest singular value upward to a power of two and uses `0.1 / bucket` model units as Lyon's flattening tolerance. The transform includes DPI and handles rotation, reflection, skew and nonuniform scaling. Curve flattening error is therefore bounded by the existing 0.1 physical-pixel tolerance, apart from floating-point precision. A new bucket can tessellate; returning to a resident bucket reuses its mesh. Bucket boundaries can change edge antialiasing within that tolerance.

Stroke outlines and dash geometry keep the existing `stroke_outline(..., resolution=1.0)` behavior in model space. Recording a stroke still calculates its outline. The complete transform scales that outline and its width, including nonuniform scaling. The cache stores its triangles; changes to width, cap, join, miter, dash or offset change the outline content and therefore the cache key. This does not improve the pre-existing stroke-outline approximation at extreme zoom.

Each renderer shares one cache across its canvases: **32 MiB charged storage and at most 32,768 entries**, including source snapshots, packed triangle positions and a 256-byte metadata allowance per entry. FIFO eviction avoids a full-cache scan on each insertion; hits do not accumulate bookkeeping. Oversized entries are drawn without retention. The bounds accommodate thousands of normal paths at several scale buckets while independently limiting tiny-entry metadata. Allocation overhead and transient tessellation/preparation buffers are separate; the existing 1,048,576-vertex expansion limit still applies on misses and hits. Clears and resizes retain meshes, renderer destruction releases them, and the cache remains bounded after surfaces detach. Caller-owned `Path2D` objects retain at most one snapshot apiece, subject to their existing segment limit.

## Measured before and after

2026-09-11, Windows 11 Pro 10.0.26200, AMD Ryzen 9 7950X3D. Release CPU benchmark built with Rust 1.90.0 MSVC (`stable` on this machine). Baseline is 0.19.0 with only the benchmark instrumentation added, captured before implementation changes. The new build uses the same scene and timing boundaries. Both executables were restricted to logical processor 10 and run alternately three times. Other desktop workloads remained active; unpinned measurements had considerable scheduling variation.

The scene contains **5,000 rounded rectangular paths**, 1395 × 1253 physical canvas metrics, and one clear followed by 5,000 `fill_path` calls per frame. Each scenario has eight warmups and 80 measured frames. Recording includes the context calls; preparation measures `Prepared::new` only. Queue leasing, GPU upload, encoding, completion, presentation, layout and consumer scheduling are excluded. Figures below are the median of each statistic from the three runs, in milliseconds per frame.

| Camera | Record median before → after | Prepare median before → after | Prepare p95 before → after | Prepare speedup |
| --- | ---: | ---: | ---: | ---: |
| Pan, scale 1 | 1.5818 → 0.3191 | 16.3403 → 3.9730 | 17.1046 → 4.3214 | 4.11× |
| Zoom within one bucket, 1.10–1.685× | 1.5336 → 0.3520 | 16.6724 → 4.6681 | 18.2048 → 4.8984 | 3.57× |
| Zoom across buckets, 0.5–7.46× | 1.6045 → 0.1571 | 17.9398 → 5.3176 | 27.8305 → 9.1639 | 3.37× |

Across all 88 frames, baseline preparation tessellates 440,000 paths per scenario. The new cache tessellates 5,000 for pan, 5,000 for zoom within a bucket, and 25,000 for five visited scale buckets. No entries need eviction in this scene. Bucket rounding can produce more vertices: the final pan frame has 270,450 versus 270,000 baseline vertices, and the final within-bucket zoom frame has 330,450 versus 270,300. This is CPU preparation evidence, not a total frame-rate claim.

The benchmark lives in `crates/lurq/src/canvas/gpu/benchmark.rs` as an explicitly ignored release test, keeping the internal preparation API private:

```powershell
cargo +1.95.0 test -p lurq --release --features canvas --lib canvas_camera_prepare_benchmark -- --ignored --nocapture
# Windows: controlled affinity, repeated runs, per-run logs under target/.
./scripts/bench-canvas-camera.ps1 -Toolchain stable -Processor 10 -Runs 3
```

For a baseline checkout, copy the benchmark module and add its `#[cfg(test)] mod benchmark` declaration in `gpu.rs`; remove the `MeshCache` local/counter output and the second argument to `Prepared::new`. The scene and measured intervals are otherwise identical. The Windows runner also accepts `-Executable` to compare saved baseline/current test binaries without rebuilding.

## Validation

All existing Canvas pixel checks were left unchanged:

- 28 software integration tests passed, including clipping, images, text, alpha, stroke geometry and DPI.
- The existing WGPU hardware pixel comparison passed on NVIDIA RTX 5080, Vulkan, driver 596.49.
- The existing hidden-window capture harness passed on both WGPU and native DX12, including composition, tiled updates, ordered readbacks, culling, resize and DPI resampling. Their scene PNGs are byte-identical.
- Eight regular Canvas unit tests passed, including six new tests for edits and snapshots, collision-safe keys, fill rules, camera reuse, stroke/dash changes, clip transforms, scale buckets, memory eviction and vertex limits.
- A new hardware comparison against software passed 14 camera/DPI cases, including reflection, skew, translucent fills, curved clips, dashed strokes and erasure.

Validation used Rust 1.95.0 for the combined GPU feature build. Existing unrelated unused-code and unused-mut warnings remain. Hardware tests run separately to avoid concurrent GPU context creation.

```powershell
cargo +1.95.0 test -p lurq --release --features canvas --test canvas_tests
cargo +1.95.0 test -p lurq --release --features canvas,wgpu,dx12,screenshot --lib canvas:: -- --test-threads=1
cargo +1.95.0 test -p lurq --release --features canvas,wgpu,dx12,screenshot --lib gpu_canvas_pixels -- --ignored --nocapture
cargo +1.95.0 test -p lurq --release --features canvas,wgpu,dx12,screenshot --lib gpu_canvas_camera_cache_pixels -- --ignored --nocapture
cargo +1.95.0 run -p lurq --release --example canvas_capture_check --features canvas,screenshot,wgpu,dx12 -- wgpu
cargo +1.95.0 run -p lurq --release --example canvas_capture_check --features canvas,screenshot,wgpu,dx12 -- dx12
```
