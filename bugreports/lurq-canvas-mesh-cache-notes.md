# lurq notes — Canvas 2D mesh cache in 0.19.1, as measured by a consumer

2026-09-12. Written by the Kontur desktop orchestrator after adopting `lurq 0.19.1` in `apps/desktop`. This is not a bug report: the release does what it says and the adoption is merged. It is field data on three things the design will meet in other applications, with numbers from a real document rather than a benchmark.

Measured on Windows 11, Ryzen 9 7950X3D, both the WGPU and native DX12 backends, on a page of 9 800 filled paths in a 1395 × 1253 physical-pixel canvas.

## 1. The cache pays off only for consumers that retain their paths

The 0.19.1 benchmark reuses `Path2D` objects between frames. Kontur does not: its paint walks the document each frame and rebuilds each path, because the scene can change between frames and the application has no cheap way to know that it has not.

For such a consumer the content hash added at record time is a cost with no matching saving on the record side. Measured on the fitted 9 800-path page: **command recording went from 15.6 ms to 19.0 ms, 16 to 27 % more expensive**, while preparation got cheaper. The net was still a win — end-to-end interaction latency fell 25 % on WGPU and 29 % on DX12 — but the ratio in the release notes (16.34 ms to 3.97 ms, 4.1×) is preparation-only and does not transfer to a consumer that rebuilds paths.

Worth considering: a way to reuse a mesh without re-hashing the geometry — a handle the application can hold and present again, or an explicit "this path is unchanged since the last frame" assertion — so the saving does not depend on the application keeping `Path2D` objects alive in exactly the toolkit's preferred shape. Kontur will look at retaining paths per node, but that is a workaround for an API that assumes retention.

## 2. FIFO eviction is the worst policy for the way a page is drawn

The cache is bounded at 32 MiB and 32 768 entries, keyed per distinct geometry **per scale bucket**, and evicts first-in-first-out.

A design document is drawn in a fixed order every frame. When the working set exceeds the bound, FIFO evicts exactly the entry that will be needed next, so the hit rate does not degrade gracefully — it collapses to zero, and the consumer is then **slower than 0.19.0** by the added hashing with none of the saving. Least-recently-used has the same failure, but a small random or segmented policy does not.

Is the bound reachable? Kontur's document model caps a page at 10 000 nodes. Four powers-of-two of zoom over a page of distinct shapes is 40 000 keys, above the 32 768 entry limit. We have not built a document that reaches it and do not claim to have hit it in practice, but it is inside the model's own limits rather than a theoretical extreme.

## 3. Meshes got finer, and the pixels moved

0.19.1 flattens curves in model space at a power-of-two bucket, rounding up, so a mesh is never coarser than before and may be up to twice as fine. On the same server document at the same viewport, **430 of 1 747 935 canvas pixels differ, 0.025 %, by at most 26 of 255, identically on WGPU and DX12** — so it is geometry, not GPU variation. On a 5 000-path page the vertex count went from 150 624 to 210 624 per frame.

Nothing about that is wrong, and a consumer with absolute-pixel tests will find they still pass if they were written with any tolerance. But it is a visible output change in a patch release, and it eats head room: the finer meshes consume the tessellation safety factor alongside lyon's own slack, leaving about 2.3× measured head room against the per-batch vertex limit. A page of heavier paths at a high zoom bucket has less room than the same page had on 0.19.0. A line in the changelog would have saved a consumer the investigation.

## What we did with it

Adopted, merged, and documented in our own paint contract. The end-to-end win is real and we kept it. The recording regression and the FIFO concern are recorded here rather than worked around, because both belong to the toolkit's design rather than to our use of it.

## Related in this folder

[[lurq-canvas-retessellation-handoff]] — the request this release answers.
