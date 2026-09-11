# Canvas GPU performance record

For the subsequent optimization of static scenes during camera movement, see [Canvas camera tessellation cache](canvas-retessellation.md), including the 5,000-path before/after benchmark and cache limits.

Measured on 2026-09-10 in `codex/canvas-2d`, replacing the CPU prototype at `1dabe90`. Hardware: Ryzen 9 7950X3D and NVIDIA GeForce RTX 5080, driver 596.49; WGPU selected Vulkan. Release builds, four warmups and 40 measured updates per case.

## Offscreen update results

The new probe measures drawing calls, geometry preparation, command encoding/submission, and an explicit wait for GPU completion. It excludes UI layout, window composition, swapchain acquisition/presentation, and readbacks. “Submit” ends after queue submission and measures CPU wall time. “Complete” includes waiting for GPU work; it is not an isolated GPU timestamp.

| Backing | Update | CPU submit median / p95 | Complete median / p95 | Tiles | Pixel uploads |
|---|---|---:|---:|---:|---:|
| 800 × 600 | 64 × 64 logical rectangle | 0.053 / 0.111 ms | 0.112 / 0.391 ms | 1 | 0 B |
| 800 × 600 | Clear + full translucent fill | 0.087 / 0.111 ms | 0.191 / 0.251 ms | 4 | 0 B |
| 1920 × 1080 | 64 × 64 logical rectangle | 0.052 / 0.100 ms | 0.113 / 0.338 ms | 1 | 0 B |
| 1920 × 1080 | Clear + full translucent fill | 0.161 / 0.197 ms | 0.409 / 0.552 ms | 12 | 0 B |
| 3840 × 2160, scale 2 | 64 × 64 logical rectangle | 0.055 / 0.135 ms | 0.121 / 0.373 ms | 1 | 0 B |
| 3840 × 2160, scale 2 | Clear + full translucent fill | 0.455 / 0.671 ms | 1.197 / 1.561 ms | 40 | 0 B |

The old CPU-only sink probe measured **10.524 ms** for the 4K small update plus frame preparation and **48.785 ms** for full clear/fill plus frame preparation. Those numbers excluded GPU upload and presentation. The probes have different boundaries, so they are evidence that the expensive full-bitmap CPU path was removed, rather than an exact application frame-rate comparison. The new 4K small update touches one tile and does not grow with the full backing area.

These timings cover this GPU and the simple rectangle workloads. Native DX12 correctness and resource behavior are tested separately; these are not DX12 timing claims. Text-heavy charts, complex self-intersecting paths, many active canvases, integrated/mobile GPUs, and long application frames need workload-specific profiling.

## Memory and transfer behavior

- One 4K persistent RGBA8 texture: **33,177,600 bytes**. No default CPU canvas bitmap or whole-canvas straight-alpha export.
- Shared 512 × 512 scratch: MSAA4 color + resolve + stencil, about **9 MiB** with D24S8. WGPU may choose a different physical depth/stencil format. Scratch size is independent of canvas size/count.
- Small edits transfer geometry/constants to the GPU. They copy/resolve a dirty tile on the GPU and upload **zero canvas pixels**.
- Immutable sources are uploaded once into a bounded cache; text shaping results are cached. Source uploads are counted separately from explicit readbacks.
- Pending commands, saved clip geometry, tessellation expansion, caches, and readbacks have explicit limits. Backing allocations, cached assets, staging/geometry buffers, and fenced retirement are separate costs; the backing-byte counter is not total device memory.
- New blank canvases defer backing allocation. A registry populated during layout keeps cached frames and dirty checks from rescanning the whole node tree.

## Reproduce

```text
cargo test -p lurq --release --features canvas,wgpu --lib canvas_gpu_performance -- --ignored --nocapture
cargo test -p lurq --features canvas,wgpu --lib gpu_canvas_pixels -- --ignored --nocapture
cargo test -p lurq --features canvas --test canvas_tests
cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,wgpu,dx12 -- wgpu
cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,wgpu,dx12 -- dx12
```

The native harness checks scene composition, incremental edits without layout, rounded corners, opacity, drawing to culled surfaces, ordered snapshots around a full clear, partial bottom tiles, GPU scale resampling, zero source uploads for a small solid edit, empty pending queues after submission, and backing retirement after removal.

Validation: 1,199 regular library/integration tests passed. The hardware-dependent GPU comparison and performance probe are explicit tests, run separately; the initial concurrent GPU/full-suite run encountered a native access violation. The isolated comparison passed, both native harnesses passed, and their 256 × 256 scene captures matched byte-for-byte on this machine. Workspace all-features/all-targets compilation, default and codec-free feature combinations, and the documentation build passed. Existing unused-code/test-mutability warnings remain.
