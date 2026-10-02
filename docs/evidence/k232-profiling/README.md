# KONTUR-232 shared Lurq profiling — builder evidence

Repository/worktree: `H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance`, upstream `ivanzaida/lurq`, target `master`, branch `codex/kontur-232-mcp-performance`.

Canonical base: `8ad98ac3b1629b7f406ed6f4ff7b1b64582c6f74` (published Lurq 0.32.0), tree `891f080f6d88030fdad7ec3bc8b319d121244f8c`.

Scope: one content-free bounded toolkit profiling model and session collector, reused by authenticated Observe-scope MCP without an app-thread roundtrip. Independent overlapping session IDs and end-during-stall observations are accepted user requirements. Future DevTools uses the same public `ProfilingHandle`, sample model and export; a new DevTools UI is outside this slice.

Source structure and exact runtime/backend coverage are documented in `docs/src/content/docs/profiling.md`. Completed CPU timers and unfinished elapsed-so-far observations are separate. No GPU timestamp implementation or performance fix is claimed. Starting/reading/ending sessions does not request redraw, layout or application mutations.

## Repository scope and source checks

No upstream root `AGENTS.md` was found in the assigned worktree. Kontur's 600-line code rule belongs to Kontur; the coordinator confirmed repository scope and assigned new profiling modules a 600-line bound. Upstream's existing runtime/backend files were not broadly restructured. Pre-existing legacy boundaries: runtime 10,503 lines, WGPU render 3,261 / Canvas 983, DX12 render 4,840 / Canvas 1,077. Hook additions are limited to timing/phase publication, static backend identity, shared-window lifecycle and passing the profiling context.

Executed source checks: direct rustfmt with edition 2024/skip_children, `git diff --check`, added-module physical-line counts. Stable rustfmt warned that the repository's nightly-only formatting options were ignored; parsing/formatting succeeded. Every new handwritten module is below 600 physical lines. The existing MCP root module remains below 600.

New module map (paths relative to `crates/lurq/src`):

| Module | Physical lines | Responsibility |
| --- | ---: | --- |
| `app/profiler/model.rs` | 287 | Typed bounds, features, errors, samples, report and phase vocabulary |
| `app/profiler/collector.rs` | 326 | Independent sessions, immutable snapshots, bounded history and window lifecycle |
| `app/profiler/producer.rs` | 291 | Runtime/window producer, coarse compute aggregation and nested phase/input/update guards |
| `app/profiler/export.rs` | 166 | Shared versioned content-free JSON export and scope/availability semantics |
| `app/profiler/canvas_text.rs` | 112 | Captured-pass thread scope, nested restoration and fixed numeric Canvas text aggregation |
| `canvas/text/profile_tests.rs` | 202 | Real embedded-font cache/stage, nested-window/session and idle-boundary/eviction regressions |
| `app/profiler/session_tests.rs` | 464 | Collector, stalled input/pass/notification, boundary/lifecycle and headless regressions |
| `app/profiler/phase_tests.rs` | 122 | Real recursively mounted blocked after-layout hook and shared export regression |
| `mcp/profiling.rs` | 129 | Observe-scope server-thread tools, ownership and revocation |
| `mcp/profiling/tests.rs` | 90 | Adapter permissions, independent sessions and feature-disabled behavior |
| `mcp/server/profiling_tests.rs` | 125 | Direct dispatch/no queue/wake and loopback bearer rejection |

## Canvas text aggregate checkpoint

The measured `Genuine18 / 01 Editor` pass at source779190d8 spent973.389ms in the complete component after-layout sweep and1.984ms in layout computation. This motivates attribution, not a cache optimization: other hooks also run in that sweep, and existing UI GlyphEngine counters do not include CanvasTextEngine.

The next source checkpoint adds a captured-pass, nested-safe thread scope with fixed numeric Canvas text aggregates. It does not reset the shared engine or change shaping, cache policy, painting or uploads. Counts cover measure/fill attempts, shape calls/hits/misses/evictions and newly produced final RGBA data bytes. Coarse CPU timers cover inclusive shape total, buffer/font/shaping, the complete glyph preparation/cache-budget/Swash loop, and complete final bitmap composition/copy. Engine-lock waits, background/out-of-pass work, transient allocation bytes and GPU execution remain excluded. Completed metrics follow the existing whole-pass session bounds; unfinished work remains observable through its existing enclosing phase.

Executed builder checks, all `cargo test -p lurq --lib --locked --offline -j1`, owning H target/temp and permitted shared H Cargo home, debug0/incremental0:

| Features / filter | Terminal result | Original log |
| --- | --- | --- |
| `mcp,perf_profile,canvas` / `canvas::text::profile_tests` | 3 passed, 0 failed; 3m43s build /0.01s tests | `canvas-text-tests.log` |
| `mcp,perf_profile,canvas` / `app::profiler` | 14 passed, 0 failed;45.93s /0.04s | `canvas-text-collector.log` |
| `mcp,perf_profile,canvas` / `profil` | 23 unique tests passed, 0 failed;15.72s /0.04s; includes all new Canvas text, collector and enabled MCP tests | `canvas-text-all-enabled.log` |
| `mcp,canvas` / `profiling` | 5 passed, 0 failed;23.52s /0.01s; profiling hooks compiled out | `canvas-text-disabled.log` |

The final combined enabled run includes the final import cfg; no Rust source changed afterward. The initial three-test run preceded only that import-gating adjustment. The real embedded font tests cover cache paths and stage timings, actual256-entry eviction, nested window restoration, independent overlapping sessions and idle crossing-start exclusion. Error/early-return paths are source-reviewed but not separately exercised. Nine warnings belong to pre-existing layout/runtime code; no new profiling warning remains. Source-only rustfmt/diff/UTF-8 and new-module600-line checks pass. Native execution of these new Canvas text fields, optimized performance and overhead remain unexecuted at this checkpoint; the prior779 capture is not evidence for the new fields.

Current source hook locations, including the notification-boundary review repair (all relative to `crates/lurq/src/app`; later changes must update these references):

| File / lines | Hook boundary |
| --- | --- |
| `runtime.rs:608,946,1144,1384–1413,1558` | Window owner initialization, shared handle, secondary adoption and DevTools marker |
| `runtime.rs:1854–1874,5501–5527` | Root rebuild/subtree refresh and nested commit |
| `runtime.rs:2035–2068,2183–2191,2267–2302` | Entire pass wrapper, Canvas text thread scope, notification tail before completion, inclusive layout update, quad and glyph current phases |
| `runtime.rs:2932–2989,5228–5266,5806–5832` | Full/cached render, Canvas handle preparation and runtime Canvas binding/recording |
| `runtime.rs:5733,5781,5972,6524,6718;5834–5850` | Aggregate base/overlay/menu compute calls; entire root/recursive component after-layout sweep |
| `runtime.rs:3094,3110,3121,3180,3313,3324,3399` | Coarse pointer/scroll/keyboard synchronous dispatch |
| `../canvas/text.rs:159–193,210–239,242–304,318–363;../canvas/context.rs:592–605` | Shape/cache total and counts; coarse setup, whole glyph loop and bitmap composition timers; measure/fill attempt counts. No cache/render policy changes. |
| `wgpu_render/mod.rs:1284–1312,1336–1371,1522–2363` | Init, Canvas processing, acquire, encode, submit and present guards |
| `dx12_render/mod.rs:688–698,2741–2847` | Init, acquire, encode/Canvas, submit and present guards |
| `wgpu_render/canvas.rs:266–363,403–408,445–757` | Backend total, tessellation, CPU submission, asset/buffer staging and recording counters |
| `dx12_render/canvas.rs:181–315,248–252,546–797` | Backend total, tessellation, explicit readback submission, asset/vertex staging and recording counters |

## Execution status

The coordinator granted only a minimal core/MCP compiler lane after another Kontur lane released. Target and compiler temporary output are in this owning worktree's `.tmp/target` and `.tmp/rust-temp` on H. Cargo home is the explicitly permitted H adopter cache while holding the sole lane. Commands use `--locked -j1`, debug information 0 and incremental compilation disabled.

The first minimal command `cargo test -p lurq --lib --locked -j1 --features 'mcp,perf_profile' profiling` stopped at a shared-export macro recursion compile failure. Original: `core-mcp-profiling-failure.log`. The narrow repair builds JSON maps directly rather than expanding a large recursive macro.

The repair command with the same graph/filter passed **7 tests, 0 failed, 177 filtered**, after a 32.22-second unoptimized test build; tests ran in 0.02 seconds. Original: `core-mcp-profiling-repair1.log`. This was an intermediate source result. The `profiling` filter does **not** execute enabled collector tests whose names are under `app::profiler`.

The compiler lane was released at that terminal result for Kontur QA, then separately regranted for the final minimal commands below. All passed sequentially on the implementation source published as first commit `ae368ad6b6ef9bf213b8724a6f55f8d54f5c8c4c` (tree `a29ef6d705848bfc3bfa809c452aa2f79291513c`). The lane was released again after the disabled-feature result. These are builder tests, not independent exact-commit QA or runtime performance measurements. They do not verify subsequent review repairs.

| Command (`cargo test -p lurq --lib --locked -j1`) | Result | Build / test time | Retained original |
| --- | --- | --- | --- |
| `--features 'mcp,perf_profile' app::profiler` | 12 passed, 0 failed, 176 filtered | 41.00 s / 0.02 s | `core-profiler-final.log` |
| `--features 'mcp,perf_profile' profiling` | 7 passed, 0 failed, 181 filtered | 0.33 s / 0.03 s | `core-mcp-profiling-final.log` |
| `--features mcp profiling` | 5 passed, 0 failed, 170 filtered | 16.15 s / 0.01 s | `core-mcp-profiling-disabled.log` |

The logs report an unoptimized test profile. Four warnings are in existing layout test/runtime code; they were not silently repaired or treated as profiler failures. All timing assertions concern actual headless layout or controlled blocked callbacks/phases; no GPU renderer was involved.

In this initial lane, WGPU/DX12/Canvas compiler checks, native windows, real Canvas captures, optimized Kontur adoption and collector overhead measurements were unexecuted/unapproved. A later narrow DX12/Canvas metadata check is recorded below. Computer Use was not used. No package release, merge or product readiness is claimed.

## Notification completion-boundary review repair

Independent source review of `ae368ad6b6ef9bf213b8724a6f55f8d54f5c8c4c` found that `Tree::pass` published its completed sample before `mcp_notify_pass`. MCP broker reconciliation and synchronous wait-reply wake work therefore fell outside the promised whole-pass duration. An end during that tail could observe a completed sample while the pass still ran.

The repair runs that tail before `finish_pass`, retaining the outer phase guard and publishing a coarse `pass_notifications` phase. The new `pass_completion_includes_blocked_mcp_notification_tail` regression registers a blocking wake on an actual parked wait reply, runs a real headless pass, and checks that a diagnostic thread can end session 2 during the notification without a completed pass. After release, only active session 1 receives the completion, with a completion timestamp after the release; result 2 stays immutable.

At publication of repair `c7272928fcc1a37b801a09d1bc6ae95c9a9f87ad` (tree `890465ad114aafa0e85eb78c59ee9ece3c3a9a8b`), the repair and its regression were **source-only and unexecuted**, pending a separately granted minimal compiler lane. That original receipt remains in the repair commit. Rustfmt parsing, strict UTF-8, source diff checks and the new-module 600-line bounds were checked. Original first-head passing logs and the original compile failure remain preserved above; no passing claim was transferred to the repaired head.

## Repaired-head execution receipt

The coordinator subsequently granted the sole compiler lane for the same minimal tests, followed only on success by one DX12/Canvas metadata check. All commands below executed at exact source `c7272928fcc1a37b801a09d1bc6ae95c9a9f87ad`, tree `890465ad114aafa0e85eb78c59ee9ece3c3a9a8b`, with no source changes during execution. The owning H target/temp and permitted shared H Cargo home, jobs 1, debug information 0 and incremental compilation disabled were retained.

| Exact command | Result | Build / test time | Retained original |
| --- | --- | --- | --- |
| `cargo test -p lurq --lib --locked -j1 --features 'mcp,perf_profile' app::profiler` | 13 passed, 0 failed, 176 filtered; blocked MCP notification-tail regression passed | 21.40 s / 0.02 s | `c727-profiler-enabled.log` |
| `cargo test -p lurq --lib --locked -j1 --features 'mcp,perf_profile' profiling` | 7 passed, 0 failed, 182 filtered | 0.29 s / 0.02 s | `c727-mcp-profiling-enabled.log` |
| `cargo test -p lurq --lib --locked -j1 --features mcp profiling` | 5 passed, 0 failed, 170 filtered | 17.17 s / 0.01 s | `c727-mcp-profiling-disabled.log` |
| `cargo check -p lurq --lib --locked -j1 --features 'mcp,perf_profile,canvas,dx12'` | PASS; DX12/Canvas conditional hook code typechecked | 1 m 06 s; no runtime | `c727-dx12-canvas-check.log` |

The minimal tests reported the same four existing warnings; the metadata check reported three unused runtime/layout warnings. No compile blocker occurred in this granted graph. The compiler lane was explicitly released after the metadata check's terminal success.

This receipt verifies builder regressions and DX12/Canvas compilation only. It does not execute a window, actual GPU/Canvas work, WGPU, winit or an optimized build, and does not establish idle/active collector overhead or native responsiveness. Those checks and independent acceptance remain outstanding. Only evidence/docs change after this receipt; source instrumentation stays at the tested repaired source until further review.

## Focused regression source

Tests exercise independent/out-of-order start/end, shared samples without reset, limits/drop counts, a blocked layout phase and an actual blocked keyboard callback observed by end2 with late completion only eligible for active1, nested pass/input activity restoration, start-crossing exclusion, window close/multi-window/devtools isolation, separate app collectors, feature-disabled availability, real headless layout samples/content exclusion/nonmutation, MCP-owned session revocation and host-session independence, server dispatch without queue/wake, and bearer rejection before tool execution. Synchronous input dispatch is a content-free activity sample, not a rendered frame.

These tests are builder regression coverage. Independent QA must verify the exact published head, including actual backend stage coverage, multiple native windows, closed/stale windows and collection overhead. A fake/stalled source fixture test does not replace a native responsiveness measurement.

## Measurement handoff

The urgent Kontur reports concern the original Pencil design's `01 Editor` page (1,862 visible scene items / 8,055 authored nodes in the coordinator's K230 report) and a large empty Working import modal. These are context supplied by the coordinator; this builder has not executed or reproduced those native measurements. Unoptimized application timing must not be described as release timing; Kontur already applies opt-level 2 to its named text dependencies.

For each future run retain fixture/page name, source and binary SHA, exact build profile/features/environment, request latency, full capture export, number of dropped/excluded samples and in-flight observations. Compare the same named fixture with sessions disabled versus active and distinguish existing `perf_profile` timer cost from new sample collection cost. Never infer GPU duration or sustained FPS from CPU submission/on-demand samples. Preserve failure originals. The coordinator owns Kontur adoption/measurement and Plane transitions; separate independent QA owns acceptance.

## Subsequent real native diagnostic

[Immutable d331 native receipt](native-20261002/README.md) records the subsequently authorized local-patched warm Kontur build, actual DX12 Canvas work, public overlapping sessions and server-thread end during unfinished layout. First readiness failure is preserved. Cold history dropped306 of546 samples; overlap/active comparison captures had no drops. Variable off/on comparison does not establish precise overhead; the reported ten-second frame was not reproduced. Layout/update mixes component after-layout Canvas painting with actual compute, motivating a separately authorized coarse phase split. Earlier unexecuted receipts above describe their original point in time.

## Coarse layout/callback refinement

The coordinator authorized two additive shared `PassSample` CPU fields and current phases: `layout_compute` sums all five runtime-owned LayoutEngine call sites (including nested overlay/menu measurements); `component_after_layout` times the entire root plus recursive hook sweep. Both remain nested in unchanged inclusive `layout_update`; Canvas binding/replay and backend Canvas timers retain their original meanings. There are no per-node/component labels, extra samples or new allocations in phase instrumentation. UI GlyphEngine counters explicitly exclude document CanvasTextEngine shaping.

Executed sequentially on the refinement source, owning H target/temp/Cargo-home, locked/offline/jobs1/debug0/incremental0: enabled `app::profiler` **14 passed** (34.95s build/.03s tests), enabled `profiling` **7 passed** (.37s/.02s), feature-disabled MCP `profiling` **5 passed** (17.97s/.00s). Originals: `refinement-collector.log`, `refinement-mcp-enabled.log`, `refinement-mcp-disabled.log`. The new deterministic regression mounts a real child component whose after-layout callback blocks on a channel: end2 observes unfinished `component_after_layout`, active1 alone receives the completed pass after release, and exports include separate nonzero compute/callback durations. A plain headless root asserts compute is nonzero while component-hook duration is zero.

Rustfmt parsing, UTF-8, diff checks and new-module<600 bounds passed. Existing nightly-only rustfmt options remain warned/ignored. This refinement has not yet executed in the warm native candidate at this receipt; the prior native captures remain immutable at d331. Warm rebuild waits for the separately reviewed local-Lurq launcher integration rather than silently reusing the previous one-off command.
