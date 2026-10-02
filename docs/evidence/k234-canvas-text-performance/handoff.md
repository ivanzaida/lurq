# KONTUR-234 Canvas text performance — builder handoff

Issue: https://plane.lurq.dev/pw-studio/browse/KONTUR-234/

Implementation base: upstream profiling `5e50f862e0e59e10e8778c5b7a61f4488f065a2a` (PR32). Tested source: `863b23c24b32d48042dfcd44a3f8d350cae7ba05`, tree `3e5d72ac9b9efc006223fbbecfbdc71f74111951`. This PR stacks on `codex/kontur-232-mcp-performance`; dependency versions and production features are unchanged.

`measure_text` previously produced a colored RGBA run that callers discarded. It now shares the same font, advanced shaping, glyph placement, bounds and refusal checks with fill, with an explicit metrics output kind. Measurement still obtains actual glyph images for exact bounds, but skips image clones and final run composition/copy. Metrics/rendered entries stay distinct in the same 256-entry / 8 MiB LRU, preventing blank fill after measurement. Existing unsupported-paint validation, alignment/baseline, features, spacing and scale rules remain.

The private Swash cache tracks image-data bytes at insertion and clear. None contributes an entry and zero bytes. Pre-lookup retirement remains `entries >= 2048` or `bytes > 16 MiB`, including hits; equality at the byte limit is valid. An insertion can exceed the byte cap until the following lookup, preserving the original policy. This is the same image.data accounting as the old scan, not total cache allocations or RSS. No cap increase or rendering-quality change was made.

The formerly oversized Canvas context is split into cohesive image/text method modules. Provenance verifies image-method equality and text-method equality except the intended dispatch, ignoring whitespace. Added/modified Rust files are <=600 lines (maximum584); rustfmt parsing and source diff checks passed.

All three commands in run1/run.ps1 completed with exit0 at the tested source: Canvas text lib10 PASS; existing public text filter7 PASS; explicit gradient-text refusal1 PASS. The latter overlaps with the7, so17 distinct tests passed. Real-font tests cover exact metrics-to-fill bounds, visible fill, color/scale, retained previous drawing owners through LRU eviction, metrics RGBA/composition=0, actual image insertion/hits/reset, missing images and cache thresholds. Existing public alignment/script, features, spacing, shadow/filter, shared context and old-ref isolation regressions passed. Existing nested-window profiler/session tests passed.

Tests reused the assigned H upstream target with locked/offline, one job, incremental off and dev/test debug0. Some minimal-feature dependencies rebuilt; there was no new target or concurrent compiler. Raw commands, timestamps, warnings and terminal results are retained. Existing unrelated layout/runtime warnings remain. Source artifacts and exact linked test-executable/dependency hashes are in provenance.json.

These are builder functional gates, not independent QA or measured native speedup. No native window, downstream Kontur build, screenshot comparison or workload rerun was executed for this change. The motivating Genuine18 / 01Editor profile is prior-source evidence only. Profiler stages/counters still describe actual work: measurement produces no RGBA bytes or composition timing. Exact-head independent review and the agreed native fidelity/performance comparison remain pending. No merge, release or install was performed.
