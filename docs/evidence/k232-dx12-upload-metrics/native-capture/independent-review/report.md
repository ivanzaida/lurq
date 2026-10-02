# Independent closed PR34 capture review

Scoped PASS for diagnostic identity, metric consistency and preserved document/viewport on **Genuine18 / 01 Editor**. Consumer `61d73d08b7c047d6c62ad55d6ed127a495881772`, upstream `644ff6613411e28dc3035c0c910df64fcf915b51`, optimized binary `e1de508381460bad06aaa10b38bc0c7ef68bcd10ce7b84541effcbe5125658ae`. Build receipt records exit 0 and release arguments; runtime reports Windows DX12, perf enabled, debug assertions/devtools false. This reviewer performed only offline checks and image inspection.

The independent reducer recomputed raw profile totals rather than accepting the builder summary. All 62 selected raw payload hashes/lengths match the builder seal `2c4c44c90774a8f824d2517aa2a89d81aeb01df3d6e59f035c6bc28dc23b5805`. Exact helper, executable, registry/local lock and frozen identity hashes reconcile. Raw profile is finalized with 42/42 returned completed records, 11 passes, no dropped/boundary-excluded samples and no in-flight observations. Maximum pass is 222.5361 ms for this named fixture. Workload 1734.9186 ms includes two clicks, frame barriers and bounded preparation-idle observations; it is not pure rendering duration.

| Fixture | CPU attribution across captured passes | ms |
| --- | --- | ---: |
| Genuine18 / 01 Editor | Inclusive asset upload | 410.6553 |
| Genuine18 / 01 Editor | Texture creation child | 388.6010 |
| Genuine18 / 01 Editor | Pixel packing child | 4.2459 |
| Genuine18 / 01 Editor | Upload staging/commands child | 12.7517 |
| Genuine18 / 01 Editor | Descriptor writes child | 3.2237 |
| Genuine18 / 01 Editor | Cache eviction outside asset upload | 1.5034 |

Four children sum to 408.8223 ms, leaving 1.8330 ms in their inclusive parent. Every individual pass obeys the containment relationship. Texture creation is approximately 94.6% of this captured upload scope. `draw.rs:37–49` times the call to `resources::texture`, whose resource-description construction and `CreateCommittedResource` are included; it does not measure GPU execution or establish why the driver call took that time. Pixel packing includes allocation/row-copy/premultiplication. Staging includes the actual arena or dedicated upload path plus CPU copy/barrier recording. Descriptor scope covers the asset SRV pair. Canvas recording itself contains upload work, so its time must not be added to upload as a separate cost.

For Genuine18 / 01 Editor, raw counts are 1238 misses/texture creations/arena uploads, 248 hits, zero dedicated uploads, 1256 descriptor pairs, 1237 evictions, 14,143,488 padded versus 11,762,496 unpadded bytes. Per-pass entry arithmetic, padded-byte inequalities, descriptor limits and before/peak/after charged-byte relationships all reconcile. Gauges remain per-pass snapshots and are not summed as allocations or new bytes. Repeated draws/groups legitimately reuse assets and still count lookup hits.

The whole owned `.kontur` and ledger hashes equal their prelaunch frozen baselines after execution; this preserves DOCU and all container resource bytes, with no document mutation from the zoom sequence. Page, zoom 14%, pan (128.6,48.0), selection, renderer and visible/render instance count 1492 are unchanged. Window is 2160×1440 physical at scale 1.5; both Canvas PNGs are 1404×1257 and match recorded hashes/bounds. Last-reported pending bytes 4,272,928 are explicitly historical and are not proof of unfinished live GPU work.

Both actual PNGs were viewed. Named page artwork, toolbar and bottom zoom controls are present without a refusal. Decoded comparison finds 4611 changed pixels, all within half-open bbox `[589,1147,692,1196]`, visually the bottom zoom-in control highlight; every pixel outside that bbox is identical. No exact whole-screenshot roundtrip claim is made. No pixel change is attributed to the instrumentation without a controlled baseline.

Owned runtime inventory matches PID 35808 to the hashed executable. Cleanup records owned process exit and port 4903 closed; harness termination is cleanup evidence, not graceful-shutdown evidence. External Orchester compilers were active and preserved. CPU API wall durations are diagnostic under uncontrolled load, not GPU time, FPS, zero-overhead, speedup or a controlled regression result. The older comparison used different consumer/upstream sources and cannot serve as a matched baseline here.

Authoritative raw source: `H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces/.tmp/k232-profiler-candidate/dx12-upload-034/closed-selected-payloads.json`. Derived receipt and executable offline reducer are retained beside this report. Historical E0425 failure and repaired compile/contracts evidence remain separately sealed. No new code, build, native run, merge or install was performed.
