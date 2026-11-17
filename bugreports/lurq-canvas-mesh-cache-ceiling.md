# lurq handoff — the Canvas 2D mesh cache collapses rather than degrades once a page exceeds its bound

2026-09-14. Written by the Kontur desktop orchestrator for an agent working in the lurq repository. This is not a correctness bug: everything draws correctly, and the cache is a clear net win on every page below its bound. It is the follow-up the FIFO concern in [[lurq-canvas-mesh-cache-notes]] asked for, now with the measurement that concern did not have. Verified against the published `lurq 0.19.2` source in the cargo registry.

```text
Archetype: implementation in lurq (toolkit owner)
Repository: H:/projects/lurq (work in a worktree)
Consumer: H:/projects/pencil-web/apps/desktop, on lurq 0.19.2 with the canvas feature
Scope: eviction policy of `canvas/gpu/cache.rs`, and a way to present a mesh without re-deriving its key
Out of scope: the consumer's own paint scheduling and its choice of what to carry during a gesture
```

## What the notes predicted, and what the machine says

The September 12 notes said: a design document is drawn in a fixed order every frame, so when the working set exceeds the bound, first-in-first-out evicts exactly the entry the next frame needs; the hit rate does not degrade gracefully, it collapses. That was reasoning, and it said so — "we have not built a document that reaches it and do not claim to have hit it in practice".

We have now built one, and the reasoning was right. The step is sharp and it lands on the **entry count**, not on a page size.

## What the toolkit does today

`src/canvas/gpu/cache.rs`, unchanged between 0.19.1 and 0.19.2:

```rust
const MAX_CACHE_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 32_768;
…
while self.bytes + bytes > MAX_CACHE_BYTES || self.entries.len() >= MAX_CACHE_ENTRIES {
  let oldest = self.order.pop_front().unwrap();
  self.bytes -= self.entries.remove(&oldest).unwrap().bytes;
}
```

The key is `{ path geometry, fill rule, tolerance }`, and `tolerance` is a power-of-two bucket of the matrix's largest singular value — but **only for a path that has curves**; `!path.has_curves()` returns a fixed `0.1`. So the entries a page demands are

```
distinct curved paths × scale buckets visited  +  distinct curveless paths
```

over the lifetime of the render engine, not of the frame.

## Measured

Windows 11 Pro 10.0.26200, AMD64 Family 25 Model 97, release build, both the WGPU and native DX12 backends, hidden owned 256-pixel surfaces, lurq 0.19.2. Each cell is a fresh surface — so the first walk really is cold — carrying the **whole page** through the first *n* rungs of a scale ladder just over each power of two (1.05, 2.05, 4.05, …), then walking the same rungs again. `cold` against `warm` is one walk's worth of tessellation for the whole page. Medians, milliseconds.

The page is a grid of axis-aligned rounded rectangles, 30 × 18 with a radius of 2, **every one of them a different size**, so every one is its own cache entry.

| paths | buckets | entries demanded | WGPU cold | WGPU warm | DX12 cold | DX12 warm |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 500 | 6 | 15 000 | 13.2 | **6.3** | 18.8 | **9.0** |
| 5 000 | 6 | 30 000 | 23.7 | **7.3** | 27.2 | **9.2** |
| 9 800 | 2 | 19 600 | 26.2 | **14.2** | 28.7 | **15.5** |
| 9 800 | 3 | 29 400 | 40.5 | **14.8** | 50.8 | **16.6** |
| 9 800 | 4 | **39 200** | 54.0 | 50.2 | 45.4 | 46.2 |
| 9 800 | 6 | **58 800** | 51.2 | 46.6 | 54.2 | 48.9 |

Read the fourth and fifth rows together. 29 400 entries is the last cell under 32 768 and the cache takes 63–67 % off the frame. 39 200 entries is the first cell over it and the cache takes **7 % off on WGPU and nothing at all on DX12** — the warm walk is 2 % *slower* there. Nothing between those two rows changed except one more rung of the ladder.

Three things follow.

1. **It is a step, not a slope.** There is no cell in which the cache is partly helping. A consumer crossing the bound does not see its frame time drift up; it sees it double, and it keeps paying the content hash on the record side for nothing.
2. **The entry bound is what binds, not the byte bound.** Summing the vertices the cold walks produced (`gpu.vertices`, times 8 bytes a position) the 9 800-path page's meshes weigh 11.3 MB at four buckets and 22.6 MB at six — both comfortably inside `MAX_CACHE_BYTES`. On this shape the 32 768 entries run out first, by a factor of more than two while the bytes are still half spare. Whether that is the right ratio between the two bounds is a design question we cannot answer from outside, but it is worth asking: a consumer that reaches one is nowhere near the other.
3. **`has_curves` is doing a great deal of quiet work.** The same page with its corners squared off — same count, same distinct sizes, no curve anywhere — demands 9 800 entries at every zoom and costs 11–20 ms cold or warm at every bucket count. A document of plain rectangles cannot reach the bound however large it is. That is a good property and it is not documented anywhere we could find; a consumer reasoning about its own head room needs it.

## Is this reachable in practice?

More honestly than in the September notes, but still not observed in the wild. Kontur's document model caps a page at 10 000 nodes, so a page of entirely distinct rounded shapes holds through **three** scale buckets and not four. The page that falls off the cliff is near that cap, nearly all rounded or elliptical, nearly all distinct, and carried or zoomed across four or more powers of two within one surface's lifetime. Our decision, recorded in our own paint contract, is to do nothing about it on the client and to write the limit down — the mitigations available to us (capping how many items a live scale previews, dropping to a union-box preview) would change what the user sees on every large page to protect one we have never produced.

So this is a request rather than a report of harm. Two things would make it moot.

## What we would like

**1. An eviction policy that degrades.** FIFO is the worst possible fit for a consumer that draws the same list in the same order every frame, and least-recently-used has the identical failure for the same reason. A small random policy, or a segmented one (a probationary segment plus a protected segment), keeps a proportional share of a too-large working set resident, so a consumer 20 % over the bound loses roughly 20 % of its hits instead of all of them. The current doc comment says "FIFO eviction has constant amortized insertion cost, including workloads larger than the cache" — which is true of the *insertion*, and is exactly the workload where the *policy* gives nothing back. Random eviction is also O(1) amortized and does not need an access log, so the property that comment is protecting is not in tension with the change.

**2. A way to present a mesh without re-deriving its key.** This is the same ask as item 1 of the September notes, from the other side. Kontur retains a `Path2D` per node now (KONTUR-72), so the content hash is memoised — but the *tolerance* half of the key still moves with the matrix, so a consumer that knows perfectly well it is drawing the same geometry at a new scale has no way to say so. A handle the application can hold and present again, or an explicit "this mesh is still valid" assertion, would let a consumer bound its own cache demand instead of discovering the toolkit's.

Neither is urgent for us. The first is the one that matters for other consumers, because it is the difference between a page that gets slowly worse as it grows and a page that falls over at a threshold nobody can see.

## Instrumentation, while we are here

`MeshCache::misses` is `#[cfg(test)]`, so a consumer cannot read it. Everything above is inferred from wall time on cold and warm walks, which works but is a blunt instrument and took a purpose-built fixture to make legible. `CanvasGpuStats` already carries `batches`, `vertices`, `tiles` and `uploaded_bytes`; a cumulative `mesh_cache_hits` / `mesh_cache_misses` / `mesh_cache_entries` beside them would let any consumer answer this in one frame rather than in an afternoon.

Two counters that exist today are worth a word in the docs, because both of them read as answers to this question and neither is:

* **`uploaded_bytes` is not a mesh counter.** Both native backends increment it only from the image-asset upload path (`app/wgpu_render/canvas.rs`, `app/dx12_render/canvas.rs`, `uploaded += asset.data.len()`), so on a page of shapes it is zero whether the cache hit or missed. We published a sentence resting on the other reading and had to withdraw it.
* **`gpu.vertices` counts vertices copied into the batch**, which the hit path and the miss path both produce. It is a good measure of what a mesh *weighs* and no measure at all of whether it was tessellated.

## What we did with it

Recorded the bound in our own `canvas_budget` module beside the three limits we already transcribe from `src/canvas/`, with a unit test that fails if either number moves in a future release, and decided in writing not to work around it. Measurements and the decision: `docs/evidence/kontur-87/README.md` and `docs/architecture/desktop-canvas.md` in the Kontur repository.

## Related in this folder

[[lurq-canvas-mesh-cache-notes]] — the field notes this measures.
[[lurq-canvas-retessellation-handoff]] — the request the cache answers.
