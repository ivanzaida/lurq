# Bounded Canvas text identities — source checkpoint

This unexecuted candidate starts from reviewed merged master `719deacf14252cce0bc8544609cddaafc39ab60e`. It addresses the measured Genuine18 / 01 Editor workload's resource creation churn without changing the DX12 64 MiB cache, minimum 64 KiB asset charge, uploads, texture contents, sampling, queue, readback, layer or frame-fence retirement. Existing PR35 evidence remains immutable. No Cargo/native command has run for this candidate.

The CPU shape LRU previously held at most 256 results under an 8 MiB charge for two RGBA buffers plus text. Metrics and Rendered entries compete in that LRU; a rendered miss minted a new GPU asset identity even when the full render inputs were unchanged. B2 of the historical four-run packet observed 6284 shape calls (4798 measure / 1486 fill), 2744 misses/evictions and 11,762,496 produced RGBA bytes; output-specific misses were not measured, so those figures alone cannot assign causality to metrics pollution.

## Identity and lifetime contract

The new engine-owned identity table uses complete ordered keys: original text, font family/size/numeric weight/style/letter spacing/canonical OpenType features, raster scale and RGBA color. It uses no hash as semantic identity. Finite floating-point inputs preserve the prior equality semantics, including signed-zero equivalence; nonfinite or metadata-over-budget keys receive fresh IDs without retention. Alignment, baseline and placement remain draw transforms and do not affect the generated bitmap. Metrics-only and empty-ink results do not receive identities.

`GlyphEngine::canvas_text_engine` clones the font database, locale and aliases into the engine. Font load/alias changes clear that engine and create a new snapshot; `CanvasTextEngine` does not mutate its database/aliases. Its shaping, glyph selection and raster inputs therefore remain the same for an identical complete key. Re-rasterization after a bitmap-cache miss may reuse an identity still retained by this table. This leaves an existing GPU texture immutable; the backend either samples the resident same-key resource or creates/uploads it on its usual miss path. IDs come from the existing process-global Canvas counter, are never reassigned on key/engine eviction, and fail closed at reserved-ID exhaustion. An evicted identity key obtains a fresh ID on reinterning. Font-engine replacement cannot reuse an old engine's IDs.

Only identity metadata is retained here, with no pixmap, RGBA buffer, GPU resource or document-node identifier. Previous drawing owners keep their existing immutable Arc buffers and texture IDs. The backend's existing cache/retirement fences remain authoritative. This can reduce repeated resource creation when an identical raster key revisits a resident GPU entry; it does not avoid CPU shaping/rasterization, guarantee GPU cache residency or promise an interaction speedup.

## Explicit bounded policy

The previous 8 MiB charge is partitioned into **7 MiB for shape results plus 1 MiB for rendered identity metadata**. The shape LRU still has at most 256 entries; the identity table has at most 4096 entries and evicts its least recently used key until both count and charge permit insertion. It does not enlarge either GPU or combined CPU cache policy.

Shape charges include a fixed full-256-slot deque reservation, text bytes, family/feature payloads and conservatively charged shared Arc headers, the ShapedText/Vec/Pixmap metadata, actual RGBA Vec capacity and retained Pixmap data length. Identity charges include original key text/font/feature payloads and Arc headers, a fixed tree-root reservation and a conservative per-key tree-slot/link charge. Shared font payloads are charged per entry even when their Arc allocation is shared. These are conservative retained-cache policy charges, **not exact allocator/RSS measurements**; allocator bookkeeping, FontSystem/glyph caches, transient shaping allocations and Arc owners outside these two caches are separate. All retained key counts and payload charges are bounded. Oversized results may be returned without caching, as before.

Telemetry keeps the existing aggregate hit/miss/eviction counts and adds Metrics/Rendered splits. Identity hits/misses count only successful nonempty rendered shape misses that reach identity lookup; an uncached/over-budget key counts as an identity miss. Empty renders, metrics-only work and shaped-cache hits do not query the identity table. Identity evictions are separate events. Last-observed shape/identity entries and charged bytes export as `cache_gauges`, not additive allocations or GPU memory; zero means no gauge observation on that captured pass. All instrumentation stays behind `perf_profile` and the existing pass-local capture gating, with no per-glyph clocks, extra global collector locks or content/key export.

## Prepared checks, not executed

Real-font tests exercise more than 256 shaped results and revisit the original raster with identical pixels and identity while preserving old owners. Full-key changes, normalized equivalents, separate engines, metadata eviction/reinterning and oversized/nonfinite bypasses have focused tests. Pass-local profile tests exercise actual measure/fill hits/misses, output-specific evictions, identity reuse and exported counters/gauges. Existing nested-window/session and boundary tests remain applicable. Source static checks and independent exact-head review precede compilation.

After the parent grants the one Cargo lane, use the existing H upstream cache/Cargo home and jobs 1, debug info/incremental disabled:

```text
cargo test -p lurq --lib --locked --offline -j1 --features mcp,canvas,perf_profile canvas::text -- --test-threads=1
cargo test -p lurq --lib --locked --offline -j1 --features mcp,canvas canvas::text -- --test-threads=1
```

Native acceptance will use the user's actual `.pencil/design.pen` (the Genuine18 source), prepared independently as a bounded broader workflow rather than substituting a toy benchmark. It must preserve exact source/binary/SDK/window/full original seed and record real open/fit/zoom/pan/selection/move/text-edit/save/undo/reopen states and per-phase profiles. No numeric benefit or pixel/lifetime readiness is claimed before that execution.
