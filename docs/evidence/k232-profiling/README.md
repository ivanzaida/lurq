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
| `app/profiler/model.rs` | 255 | Typed bounds, features, errors, samples, report and phase vocabulary |
| `app/profiler/collector.rs` | 326 | Independent sessions, immutable snapshots, bounded history and window lifecycle |
| `app/profiler/producer.rs` | 248 | Runtime/window producer and nested phase/input/update guards |
| `app/profiler/export.rs` | 157 | Shared versioned content-free JSON export and scope/availability semantics |
| `app/profiler/session_tests.rs` | 462 | Collector, stalled input/pass/notification, boundary/lifecycle and headless regressions |
| `mcp/profiling.rs` | 129 | Observe-scope server-thread tools, ownership and revocation |
| `mcp/profiling/tests.rs` | 90 | Adapter permissions, independent sessions and feature-disabled behavior |
| `mcp/server/profiling_tests.rs` | 125 | Direct dispatch/no queue/wake and loopback bearer rejection |

Current source hook locations, including the notification-boundary review repair (all relative to `crates/lurq/src/app`; later changes must update these references):

| File / lines | Hook boundary |
| --- | --- |
| `runtime.rs:608,946,1144,1384–1413,1558` | Window owner initialization, shared handle, secondary adoption and DevTools marker |
| `runtime.rs:1854–1874,5501–5527` | Root rebuild/subtree refresh and nested commit |
| `runtime.rs:2035–2066,2181–2189,2265–2300` | Entire pass wrapper, notification tail before completion, inclusive layout update, quad and glyph current phases |
| `runtime.rs:2930–2987,5226–5264,5796–5814` | Full/cached render, Canvas handle preparation and runtime Canvas binding/recording |
| `runtime.rs:3092,3108,3119,3178,3311,3322,3397` | Coarse pointer/scroll/keyboard synchronous dispatch |
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
