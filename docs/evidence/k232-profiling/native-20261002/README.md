# Real Kontur profiling diagnostic — 2026-10-02

Immutable builder captures at Lurq `d3314f2a524d37b8dd0119eb5bd4b893fd9ba8f0` / tree `04fa9bd1dfcbe2e594f0eb95b27d5d48ca8ccabc`, Kontur `8e2bb021989688f6d260fe04daf06bccee3e744a` / tree `612b074cdaf74a490f1729c255c279d67c52f17c`. They establish real DX12 CPU instrumentation and independent public sessions. They do not establish optimized responsiveness, precise collector overhead, GPU execution timing or reproduction of the reported ten-second frame.

## Build and ownership

`identity.json` retains source/tree, binary/config/lock hashes, command/environment and observed availability. Candidate binary SHA256: `35edfcfd5e17f6db907e0035fe86d0dc26d73ab0163dac431462e17139a44ebb`. Unoptimized dev, debug0/incremental0; Kontur's named text dependencies already use opt-level2. Default MCP, no DevTools; Canvas/DX12/WGPU/perf_profile compiled, selected runtime DX12. WGPU runtime was not executed.

Local `.tmp` Cargo config left production manifests unchanged. Initial offline resolution changed an unrelated winapi-util dependency edge; it was rejected. Incorrect shell sequencing briefly started the premature build after its assertion; only the owned compiler was stopped, with original log preserved. The final lock removed only four registry identity fields for local Lurq/macros, validated by locked offline metadata before a separate successful build (51.78s). Original lock restored after exact hash guards; executed lock/full metadata stay in owning H scratch, hashes retained here.

All outputs/temp/Cargo-home paths are on H. Only owned native PIDs stopped; normal SDK tools keep 15s deadlines. Existing external Orchester compiler activity was observed, never modified. No CUA/install/release/merge.

## Fixture and readiness

Copied genuine native2 fixture, excluding unclassified native3 state. Genuine18 document `0acea323-5056-408d-84c0-f7c9a82e7c36`, revision1, SHA256 `3ed4679c841bb21af02a02d45c5c5c6d2b3a0504dc1ba78f1b08db4ea12c516f`; ledger SHA256 `bd90d3e08d5495594c0d28717ae53e76a84517250397a69bbf855661118d7ea6`. 8,055 authored nodes, 18 pages, six admitted font resources. `01 / Editor` page `10436fe8-bb35-4281-be38-0bbd5dad62d3`.

Native1 PID59536 captured real renderer-init work but asserted readiness after one frame. Its Canvas passes belong to a restored small document; the first harness did not save raw open/canvas responses, so exact failed state is unavailable. Failure/script/profiles/immediate `port_closed=false` remain intact. Later read-only check confirms owned PID/listener absent without another kill.

Native2 PID60548 retains actual open response (`opening`, exact document/revision/hash) and every readiness snapshot. Canonical bounded120s state predicate keeps original15s/tool bound. Snapshot006 verifies active genuine editor, saved-local revision1, intended page, positive matching visible/render counts, no Canvas/text refusal. Scene log:1,862 items; viewport culling at fit14%:1,492 visible/render instances. These counts differ intentionally. Document/ledger hashes remain unchanged across reversible zoom actions.

## Bounded capture results

| Capture | Completed / retained / dropped | Meaning |
| --- | --- | --- |
| Cold opening, session1 | 546 / 240 / 306 | **Partial truncated history**; last110 rendered passes include admission and ready work. Sums are not full-load totals; one start-crossing operation excluded. |
| Ready overlap, session2 | 82 / 82 / 0 | 28 rendered passes; remained active after nested session3 ended. |
| Nested overlap, session3 | 27 / 27 / 0 | Nine rendered passes; one start-crossing operation excluded; returned payload unchanged. |
| Active comparison sessions4/5 | 112 / 112 / 0 each | 40 rendered passes/24 Canvas command groups/46,925,152 uploaded bytes each. |
| Final observation, session6 | 11 / 11 / 0 | Three completed rendered passes plus unfinished work; one start-crossing operation excluded. |
| Nested end during work, session7 | 0 / 0 / 0 | Truthfully unfinished, no fabricated completed duration. |

Overlap verifies distinct IDs, immutable end2 and eligible later completions in active1. Outer overlap:18 Canvas groups/35,193,864 uploaded bytes; nested:6 groups/11,731,288 bytes. Canvas asset-upload CPU sum1,352.5446ms is nested in recording/total, includes cache lookup/copies/premultiplication/driver texture/SRV allocation/map work, and cannot split contributors or measure GPU execution. CPU acquire sum558.4002ms/max66.511ms is outside asset upload.

Longest overlap frame270: total1,110.1168ms; inclusive layout_update979.4389ms; backend encode128.0852ms; Canvas total127.3385ms/asset upload109.9317ms; CPU acquire0.0581ms. Active comparison frame421: total1,179.2348ms/layout_update1,044.3475ms. Nested scopes are not additive.

Session7 ends in10.2731ms SDK roundtrip reporting unfinished layout_update169.9796ms. That operation began **before both sessions**: server-thread access and honest boundary exclusion are proven, zoom causation/ten-second reproduction are not. Session6 records later eligible work, including pass687.1941ms/layout_update585.4525ms, while session7 stays immutable.

## Off/on limits

Same binary/page/four reversed zoom pairs per segment, off/on/on/off. Start/end export excluded from timed workload, identical normal frame barriers and50ms tail. Off frame count unobserved; active segments bounded without drops and matching group/byte counts; unfinished tail explicit.

| Segment | Sessions active | Wall ms | Process CPU ms |
| --- | --- | ---: | ---: |
| 0 | No | 14,102.9813 | 12,484.375 |
| 1 | Yes | 12,977.1003 | 11,656.250 |
| 2 | Yes | 15,851.2112 | 14,140.625 |
| 3 | No | 12,222.5855 | 10,796.875 |

Raw means+9.5%wall/+10.8%CPU are **not causal collector overhead**: ranges are large, external contention observed, asynchronous work remains. Negligible idle cost, feature-disabled build comparison, release responsiveness and FPS/GPU timing remain unestablished. Raw values preserved.

## Coverage and cleanup

At d331, layout_update includes solver/text compute, reactive/resources and component after_layout. canvas_recording measures bind/replay only. Kontur CanvasView.after_layout painting is inside layout_update, outside bind/replay. UI GlyphEngine counters exclude document CanvasTextEngine shaping: zero UI shaping does not mean absent Canvas work. Application render_us is work-cumulative, not a same-pass hook timer. Coordinator approved subsequent aggregate layout_compute/component_after_layout refinement without per-node traces; this receipt remains immutable at executed d331. No upload/cache optimization inferred.

Native2 terminal exit0, owned process exited/port closed, hashes preserved. SDK teardown `Session termination failed:202` retained: independent source review identifies rmcp accepted DELETE versus Python SDK expected200/204. Interoperability warning, successful captures, no process leak. Independent acceptance remains separate.

Copied artifacts passed credential-pattern check; artifact-hashes.json identifies originals (this prose excluded). Harness files<600 lines. Binaries/document bytes/bearer discovery files are not committed.
