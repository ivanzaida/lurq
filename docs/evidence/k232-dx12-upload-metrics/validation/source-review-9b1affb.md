# Independent source review: DX12 upload metrics

Reviewed upstream `9b1affb544572b75be9d6e19cb897146a05db5fa`, tree `53e8a0bac526e8b61fd77b512fd37ecb5028a17f`, against base `be8d47d54658501765c6a06f239f755abe96c9b1` and preserved metrics checkpoint `dc0a0e5c2b29bb6efd37c504fc69a386b39aee30`. Tracked source was clean. Verdict: no concrete source blocker found; compilation, runtime behavior and overhead remain unexecuted in this review.

## Timing and counts

- `canvas/draw.rs:22–100`: the existing inclusive `asset_upload` scope surrounds cache lookup, texture creation, upload and asset descriptors. Texture creation, pixel packing, staging/copy/barrier commands and descriptor-write detail timers are sequential, disjoint subscopes. Their sum does not include all lookup/map/COM bookkeeping or instrumentation overhead; it is not the whole coarse scope.
- `canvas/resources.rs:112–173`: packing includes allocation, row padding/copy and premultiplication. Staging includes `upload_frame_bytes`, copy-location construction, `CopyTextureRegion`, resource-reference drops and the transition barrier. `dx12_render/mod.rs:3072` tries the existing arena and otherwise creates/pushes a dedicated upload buffer. There is no queue fence wait inside this helper. Padded bytes are the actual row-padded CPU payload, excluding placement-alignment gaps.
- `canvas/draw.rs:31–95`: hits/misses count each prepared draw with an asset, including repeats. Creation counts increment after successful creation; arena/dedicated counts after successful staging; descriptors count each distinct asset per command group. Backing/layer/composite descriptor work is excluded. These are CPU resource/command operations, not completed GPU execution.
- `canvas.rs:324–344`: eviction is outside `asset_upload` but inside Canvas total. Existing `max(payload, 64 KiB)` charges and entry counts are captured before, after each insert and after eviction. The peak is a policy-charge high-water mark, not texture allocation or RSS. Cache policy itself is unchanged.

## Capture and failure boundaries

`canvas.rs:191–200` resets the profile once per encode and gates details on one active-session observation. Groups, surfaces and readback submission boundaries within that encode accumulate into the same detail record; later encodes start fresh. Uncaptured encodes add no detail clocks or collector locks. `export.rs:137–168` suppresses detail on WGPU/custom/disabled/unavailable backends; coverage denotes backend/build eligibility, while uncaptured eligible work may still have null detail.

Failed texture/staging/descriptor operations can leave private partial counters, but the failing DX12 render returns false and does not publish a frame: `runtime.rs:2066` passes frame data only when rendered. The fields therefore do not measure failed-operation duration or publish stale successful upload counters on a failed pass. This is an explicit scope limitation, not failed work reported as successful.

The existing collector excludes a pass started before a session (`collector.rs:296`) and cannot mutate an ended session. `producer.rs:267` clones numeric frame data into detached samples. Overlapping windows/sessions share that established boundary without asset IDs, paths, text, labels or pixel content entering these new fields. New tests exercise numeric aggregation, null availability and detached overlapping-session reports; they do not execute D3D12 calls.

## Mechanical extraction

Independent read-only comparison verified all 21 original function bodies against the final four modules. Tokens/literals are preserved except the shader include's directory adjustment, which resolves to the same unchanged HLSL. Added parent lines are module/import/re-export declarations. Method receiver/descriptor-heap ownership, resource references, retirement arrays, upload buffers and ManuallyDrop order are preserved; private visibility adjustments make the same functions reachable across the extracted sibling modules. The profiling model/tests/export are byte-identical to dc0.

All ten modified/new Rust files are below 600 lines; maximum is the parent Canvas module at 587 (draw 297, resources 241, pipeline 117). Independent comparison receipt and executable script are under `.tmp/dx12-upload-review-9b1affb/` in this QA worktree. No implementation file or shared checkout was edited.

## Finite execution still needed

Use the assigned warmed H cache/home/temp, locked/offline, one job, debug/incremental disabled, only after root grants the lane:

```text
cargo test -p lurq --lib --locked --offline -j1 --features mcp,canvas,perf_profile profiling_canvas_upload -- --test-threads=1
cargo test -p lurq --lib --locked --offline -j1 --features mcp,canvas profiling_canvas_upload -- --test-threads=1
cargo check -p lurq --lib --locked --offline -j1 --features mcp,canvas,perf_profile,dx12
cargo check -p lurq --lib --locked --offline -j1 --features mcp,canvas,dx12
```

Expect four enabled and two disabled tests. The second core test lacks DX12 because default features are empty, so the final check is needed to cover the feature-disabled conditional upload signature and extracted real backend graph.

A later already-planned named-fixture native capture should confirm actual detail presence and numeric consistency: padded bytes at least payload bytes; arena+dedicated uploads equal successful creations; peak charge at least before/after; after entries equal before+creations−evictions; detail subscopes no larger in sum than the containing coarse upload duration (eviction excluded). Retain idle/unsupported null behavior and overlapping-session outcomes. No new speedup, GPU-duration or zero-overhead claim follows from this source review.
