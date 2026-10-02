# K234 finite matched native comparison

This packet records root-executed optimized builds and native run02, followed by unchanged independent read-only review. It is an evidence-only follow-up to the 17 distinct focused tests. Tested Lurq product remains `863b23c`; published `4200bb5` has the same crates/Cargo/lock. Baseline Lurq product is `5e50f86`. Both use Kontur `b3076c67`, tree `19e9ca90`, identical executed lock `5a49b46e`, normal default MCP/perf_profile and no DevTools. Build identities retain full commits, crate trees, hashes and commands. The baseline/candidate binaries are `2b2c7e0b` / `a05581d8`; executable files are deliberately excluded.

Fixture: **Genuine18 / 01 Editor**, document `0acea323`, page `10436fe8`, original document SHA `3ed4679câ€¦516f`, ledger SHA `bd90d3e0â€¦7ea6`. Fresh distinct profiles/processes use the same original preferences, open/fit, warm-up zoom-out/in, fit/reset, idle capture, measured zoom-out/in and settled Canvas readback. Actual physical windows 2160Ã—1440, effective DPI 1.5 and Canvas 1404Ã—1257 match. Initial/final semantic zoom 14, pan 128.6/48, selection and 1492 visible/rendered instances match. Actual OS window position and work identity are unavailable.

| Fixture / measured whole pair | Baseline | Candidate |
| --- | ---: | ---: |
| Genuine18 /01 Editor â€” Canvas text CPU, ms |340.6567|275.7695|
| Genuine18 /01 Editor â€” glyph preparation, ms |251.0622|210.3234|
| Genuine18 /01 Editor â€” bitmap composition, ms |49.4390|26.5856|
| Genuine18 /01 Editor â€” buffer/font/shape, ms |32.4152|32.9427|
| Genuine18 /01 Editor â€” produced final RGBA bytes |18757216|11762496|
| Genuine18 /01 Editor â€” all10 pass CPU totals, ms |880.9280|866.0748|
| Genuine18 /01 Editor â€” pair latency including polling/barriers, ms |1475.9528|1470.0383|

Aggregate counts are identical: 4798 measure + 1486 fill = 6284 shape calls; 3540 hits, 2744 misses and2744 evictions. Each pair capture has 10 rendered DX12 passes and 40 completed mixed records; idle has 0 records. Final reports have 0 unfinished work, drops, boundary exclusions or truncation. Nested stages are inclusive/nonadditive. Canvas text cost and produced bytes are lower in this sequential run; whole-pair client latency is essentially unchanged. This is not statistical/general causal speedup, FPS, GPU timing or full smoothness. Environment inventory is retained; unrelated external Orchester load was observed and preserved.

Decoded baseline/candidate BEFORE pixels are identical, and baseline/candidate AFTER pixels are identical. Within each variant BEFOREâ†’AFTER differs the same 4611 of 1764828 pixels, maximum channel difference 12. This supports no observed K234 pixel regression for the workload, while explicitly failing exact zoom round-trip restoration. The shared drift's cause is unclassified. All four immutable PNGs and the unmodified pixel-comparison.json are retained; no image rewriting or tolerance change occurred.

Run01 remains a failure: its original runner `b5f8b6c4` required stored pending_bytes to become live zero, although Kontur exposes last-recording snapshots. The original runner, manifest, terminal/controller failure, first/last captured state and cleanup are preserved. The source-supported corrected 299-line runner `9e242651` retains pending-byte observations but requires semantic open/full instances, preparation idle, completed CPU profiler passes and fence-backed readback. Historical pending 4272928 is not proof of a live drain stall. Default SDK 15s, frame 14s and bounded readiness 120s were unchanged. Run02 terminal 0 does not overwrite run01.

Both copied document/ledger hashes equal their originals after cleanup. Baseline PID 55124 and candidate 32704 exited; owned port 4903 closed. Cleanup was harness termination, not a graceful-window-close claim. The independent reviewer additionally rehashed the source identities, all reviewed raw artifacts and decoded pixels, and observed PID/listener closure. Its frozen report and 20-artifact seal are included unchanged under independent-review/.

Raw copies use byte-for-byte copy with source equality checks, recorded in copy-provenance.json. Profiles, databases, executables, discovery credentials and hundreds of redundant readiness snapshots are excluded. Representative initial/final and zoom-step states are retained. Historical embedded paths still name the original executed locations; the packet does not pretend copies were executed there. artifact-manifest.json covers this packet only; the earlier nine-payload builder manifest is unchanged.

Verdict: finite matched identity/profile consistency/cross-variant Canvas fidelity **PASS**, with the shared zoom round-trip drift and unchanged overall pair latency explicit. Exact upstream master 0.33 integration remains a separate gate. No whole desktop acceptance, SDK release, install or merge is claimed. This agent performed only copying/source review for this follow-up, with no product edits, rebuild or native launch.
