# Independent matched native K234 comparison

Verdict: **PASS for this finite matched workload's source identity, profile consistency and cross-variant Canvas fidelity.** No Cargo, native launch/input or source/fixture changes by reviewer. Root controller90375 terminal0; both apps closed. Reviewed raw packet: warm `.tmp/k232-profiler-candidate/comparison/native-matched-02`; accompanying reviewed-hashes JSON seals20 raw artifacts. Original native01 stale pending-byte gate failure remains preserved separately. Corrected299-line runner SHA256 `9e2426516256fb5aebd4a21cd9939034d5cab8a20c5bfdb7a02fecb67b12c7cb` treats pending bytes as historical diagnostics.

## Exact sources and workload

Genuine18 / 01 Editor, document0acea323 and page10436fe8. Both optimized builds use Kontur b307/tree19e9 and same executed lock5a49b46e; default MCP/perf_profile enabled, devtools/debug assertions disabled. Baseline actual source5e50/crate-treea789, binary `2b2c7e0b49d9e2e5c411069e34d4652e18df087e745530af32645d3c84b4e10b`; candidate863b/crate-tree7209, binary `a05581d89c565cdeacfc7052723499d481e0e749a631a43d3b48fbc73b3ad1b9`. Both executable hashes independently checked. Candidate source/test/evidence carry to published PR33 `4200bb581e958237d1c78d57ebb219c0861379aa` is recorded in k234-source-review.md. These0.32 builds are not the future0.33 combined integration.

Fresh distinct processes and copied identical document/ledger/preferences; same open/fit/warmup/reset then zoom-out/in pair. Actual physical window2160x1440, effective scale1.5, Canvas bounds360,138,1404,1257 match. Zoom14/pan128.6,48/page/selection/1492 visible and recorded instances agree before/after across both. No preparation wait/run, text refusal or canvas error. Historical pending4272928 after recording is retained; zero live toolkit queue is not falsely inferred. All10 captured passes per variant report rendered=true/DX12; completed pass boundaries and fence-backed before/after captures provide completion/output witnesses. Actual OS window position/work identity remain unavailable.

## Independently recomputed profiles

Each variant has40 completed records:24 UI updates,6 synchronous input dispatches and10 rendered passes. Both idle profiles contain0 records. Finalized reports have0 drops,0 boundary-excluded records,0 in-flight work, no truncation; every sample starts/ends inside its own session. No sample overlap from unrelated windows. Shape counters agree per pass and in aggregate:4798 measure+1486 fill=6284 shape calls=3540 hits+2744 misses;2744 evictions. Timer stages fit within Canvas text total, component-after-layout, inclusive layout and pass total (rounding allowance only).

| Whole-pair metric | Baseline | Candidate |
| --- | ---: | ---: |
| Canvas text CPU total, ms |340.6567|275.7695|
| Glyph preparation, ms |251.0622|210.3234|
| Bitmap composition, ms |49.4390|26.5856|
| Buffer/font/shape, ms |32.4152|32.9427|
| Newly produced final RGBA bytes |18757216|11762496|
| All10 pass CPU totals, ms |880.9280|866.0748|
| Pair client wall with polling/barriers, ms |1475.9528|1470.0383|

Aggregate nested stage sums are not an exclusive accounting of total: cache/control and other work remain in total. The observed64.8872ms lower Canvas text CPU total and6994720 fewer produced RGBA bytes are consistent with metrics-only preparation. Whole-pair client latency is essentially unchanged. One sequential matched run under recorded external Orchester build contention cannot establish statistical or causal general speedup, overhead, GPU execution or FPS.

## Independent image and persistence oracle

Pillow12.2 independently decoded all four recorded PNG pairs as RGBA and recomputed exact differing pixels/max channel difference and file hashes. Baseline/candidate BEFORE are identical; baseline/candidate AFTER are identical, each1404x1257=1764828 pixels. Within each variant BEFORE→AFTER differs4611 pixels with max channel12, identically across variants. Thus no K234 pixel regression is observed, but there is pre-existing zoom-roundtrip drift; do not claim exact before/after visual restoration or a diagnosed cause. No tolerance weakening or image rewriting.

Both copied document and ledger independently rehashed after close and equal original receipts (`3ed4679c...516f`/`bd90d3e0...7ea6`). Cleanup receipts record exited baseline55124/candidate32704 and port4903 closed. Independent read-only OS inventory after completion found neither PID nor listener. No whole-goal acceptance, merge, SDK release or stable-install claim.
