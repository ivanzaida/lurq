---
title: Profiling sessions
description: Shared bounded CPU profiling sessions, overlapping captures and stalled-UI observations.
---

# Profiling sessions

Profiling is a toolkit service, shared by the embedded MCP server and in-process consumers. It does not require DevTools and does not collect component properties, text, input values, document paths, window titles or tokens. The existing DevTools inspector UI remains separate; a future profiling UI can consume this same session API and model.

Enable `perf_profile` to instrument timings. MCP profiling tools are available with `mcp`; without `perf_profile`, availability reports `feature_disabled` and starting a session errors. Starting, reading and ending sessions never request redraw, layout, synthetic input or an event-loop roundtrip.

```rust
use lurq::app::profiler::SessionOptions;

let profiling = tree.profiling_handle();
let id1 = profiling.start(SessionOptions::default())?.id;
let id2 = profiling.start(SessionOptions::default())?.id;
let result2 = profiling.end(id2)?;
// Normal application work continues. Session 1 keeps collecting.
let result1 = profiling.end(id1)?;
```

The handle is cloneable and can be moved to a diagnostic/server thread. `read(id)` returns an owned immutable snapshot while leaving that session running. `end(id)` finalizes only the named session. Ending session 2 cannot reset, clear or stop session 1. A finalized report cannot receive a later frame completion.

With `mcp`, use `lurq_profile_start {"max_samples":120}`, `lurq_profile_read {"id":"profile_1"}` and `lurq_profile_end {"id":"profile_1"}`. The returned ID must be used; IDs are not globally fixed. `lurq_profile_read {}` reports feature/build availability and limits. MCP requires its normal bearer token, enabled server, Observe scope and tool permission. These tools execute on the server thread even if the UI is blocked.

## Bounds and lifecycle

There are at most eight concurrent sessions across all consumers, 1–240 retained samples per session (default 120), and 64 tracked windows per application collector. A sample is a completed pass, root rebuild/subtree refresh or input dispatch. Input activity is explicitly a different sample kind and is never counted as a rendered frame. Oldest samples are dropped on overflow; reports include completed, returned, dropped and boundary-excluded counts. Samples shared by overlapping sessions use immutable reference-counted data, rather than cloning their payload for each session. There is no unbounded per-node tracing or prior-history replay on start.

Each session has independent monotonic start and end bounds. Only complete operations that started at or after its start and finished while it remained active are retained. A pass that crosses the start boundary is excluded, even if it later finishes. Nested rebuilds that fit entirely inside the session can still appear as separate samples. Reports count exclusions once those operations complete while a session remains active.

An end during a stall reports current unfinished work separately: toolkit window and prospective frame ID, current phase, elapsed-so-far, phase elapsed-so-far and whether the operation began before the session. These observations are excluded from completed timing samples. They are not fabricated partial stage measurements and cannot be added to completed durations. A session begun in the middle of a slow frame can therefore report that slow unfinished frame, even though no completed pass qualifies. An empty session or a session with unfinished work has an explicit status; it does not imply a fast frame.

Window IDs are toolkit identities (`main`, `w1`, and hierarchical IDs for nested secondary windows), never application-assigned titles or names. IDs are limited to 256 UTF-8 bytes; windows that exceed this or the metadata bound are counted as untracked. Closed-window metadata remains until its bounded slot is needed; retained samples keep their identities. DevTools windows are excluded by default and opt in through `SessionOptions::include_devtools` or MCP's existing `include_devtools` configuration. Separate root trees own separate collectors.

After the root tree closes, surviving handles may read/end existing sessions, but cannot start a new one. Unknown IDs and already-ended IDs error. The last 64 ended IDs are remembered; older IDs may be reported as unknown. MCP can only read/end sessions it started. Revoking Observe, disabling MCP or denying a profiling tool cancels MCP-owned sessions and discards their reports; in-process sessions continue. Regranting permission does not revive cancelled IDs. MCP shutdown cancels its own sessions.

## Coverage and units

All exported timing values are **CPU wall-clock milliseconds**. Monotonic timestamps are relative to the collector's creation and are not calendar timestamps. Build metadata reports the toolkit version, target OS/architecture, relevant compiled features and `debug_assertions`. Cargo profile and optimization level are explicitly not embedded; debug assertions are not a reliable substitute for the build command.

| Scope | Actual boundary and interpretation |
| --- | --- |
| Pass total | Entire `Tree::pass`, including setup, update/layout, resolve/raster, render, runtime cleanup, existing observers and MCP notification/reconciliation/reply work. Completion is published after that tail; a blocked MCP notification appears as unfinished `pass_notifications`. Input dispatch or arbitrary host work before the pass is outside it. Idle passes are not retained. |
| Input dispatch | Actual Tree pointer move/down/up/leave, scroll and keyboard down/up entry points, including synchronous application callbacks and subsequent reactive updates. Coarse input phases are visible before any frame pass begins. No pointer coordinates, key/code strings, text or input values are retained. A nested pass/update restores its outer activity phase on completion. Menu/window callbacks outside these Tree entry points and unrelated host task processing are not attributed. |
| Root rebuild / subtree refresh | Retained-tree rebuild/update, including a nested commit that installs nodes, preserves runtime state/IDs and refreshes interaction/focus. An update may happen before or inside a pass. |
| Layout update | Existing `update_layout` plus caret processing. Includes rebuilds, resources, layout work and Canvas binding/replay. This is an inclusive update scope, not a pure layout-algorithm timer. |
| Canvas recording | Runtime Canvas binding traversal and synchronous layout observers that record drawing commands. Calls to `CanvasHandle` from unrelated host work outside this traversal are not independently timed. |
| Canvas preparation | Selecting attached hardware handles before `RenderEngine::render`; it does not represent GPU execution. |
| Quad / glyph work | Existing quad resolution and render-list/glyph construction timers, with the existing detailed text engine cache and shaping counters. The glyph scope includes non-text render-list construction. |
| Canvas backend total | Processing Canvas batches, resource/cache maintenance, command preparation, tessellation, uploads and command recording. Lazy Canvas renderer creation is included. WGPU processes Canvas before surface acquire; DX12 processes Canvas inside encode. |
| Canvas tessellation | `Prepared::new`, including command preparation and mesh-cache lookup/misses. Cache hits still perform preparation work. |
| Canvas asset upload | Asset-cache lookup, conversion/resource creation and texture staging for images/text; CPU time, with uploaded asset-byte counts. |
| Canvas buffer upload | WGPU vertex/global buffer staging; DX12 vertex staging. DX12 per-tile/layer constants remain inside recording rather than adding a clock operation for every constant. |
| Canvas recording | Draw/tile planning and command recording, including nested upload/submission work. Counters report batches, command groups, vertices and tiles without retaining commands. |
| Render init/acquire/encode/submit/present | Existing backend CPU timers. Current-phase observations also distinguish these stages while a pass is unfinished. WGPU encode includes buffer/image upload subtimers; DX12 encode includes Canvas and atlas upload. DX12 UI buffer/image uploads have no independent subtimers and are reported as included in encode. |
| GPU timestamps | **Unavailable**: neither backend adds GPU timestamp queries here. CPU queue submission and presentation calls are never labelled GPU execution time. |

Do not sum inclusive or nested scopes. A UI update may already be in pass/layout time; commit is in UI update; Canvas recording includes upload/submission; DX12 Canvas is in render encode. Existing `FrameProfile::gpu_submit` is retained for compatibility but means CPU time for Canvas preparation plus the render call. JSON states this explicitly.

Memory counters retain the existing cached estimate, sampled at most once per second. They are not an allocation trace or an exact live heap measurement. Zero counters can mean no work for that completed operation; disabled/missing frame data and unsupported stage coverage are reported separately. A custom renderer that provides no `last_profile` reports `render.profile_available=false` and null render CPU timings; Canvas coverage requires an instrumented built-in backend.

## Cost and interpretation

Collection has fixed bounds and one short mutex per coarse phase publication/completion; locks are never held around UI/backend work or JSON serialization. Immutable phase context/window IDs use `Arc<str>`, so marker/context copies do not allocate per frame. A completed payload is allocated once, then referenced by up to eight sessions. With no active session, no completed sample payload is retained or allocated and detailed per-Canvas-group phase markers skip their locks. Compiled `perf_profile` still runs the existing timers and coarse input/pass/layout/Canvas-backend/render phase markers so a session started mid-stall can observe current work. A session begun during an already-running Canvas group may initially observe the coarser Canvas-backend phase. Enabled-but-idle overhead remains unmeasured until a controlled runtime comparison. Without that feature, runtime timing hooks are compiled out.

No frame is forced to obtain a sample. This is an on-demand renderer: an idle interval without samples is expected and a frame count divided by capture duration is not GPU throughput or FPS. Use pass cost, pending phase/age, request latency and named workloads for diagnosis. Collection overhead and release-versus-development performance require measurements on the actual binary/fixture; source instrumentation alone is not such evidence.

With the `serde` or `mcp` feature, `SessionStarted::to_json` and `ProfileReport::to_json` provide the same version-1 content-free export used by MCP. Finalized `sample_age_ms` is the age at end; a live read computes age at that read. Applications should retain the source SHA, binary/build command and fixture name beside exported reports.
