# lurq handoff — Canvas 2D re-tessellates the whole scene on every camera change

2026-09-11. Written by the Kontur desktop orchestrator for an agent working in the lurq repository. This is an optimisation opportunity in the shipped Canvas 2D, not a correctness bug: everything draws correctly. Verified by reading the published `lurq 0.19.0` source in the cargo registry, not a working branch.

```text
Archetype: implementation in lurq (toolkit owner)
Repository: H:/projects/lurq (work in a worktree)
Consumer: H:/projects/pencil-web/apps/desktop, on lurq 0.19.0 with the canvas feature
Scope: avoid re-tessellating unchanged geometry when only the camera moves
Out of scope: the consumer's own paint scheduling, which is a separate Kontur defect (KONTUR-69)
```

## What the consumer is doing

Kontur's desktop editor draws a document page through `Context2D`: one `paint_path` per shape, with pan and zoom expressed as a transform on the context. Panning or zooming changes no geometry — only the camera. A typical page is a few hundred to a few thousand paths; the document model allows up to about ten thousand.

## What the toolkit does today

**The camera is baked into the vertex data at record time.** `canvas/mod.rs:605`:

```rust
let matrix = transform(Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix));
let Some(path) = path.and_then(|p| p.transform(matrix)) else { return false; };
return self.enqueue(gpu::Command::Path { path, … });
```

The enqueued `Command::Path` therefore holds screen-space points. A pan of one pixel produces a different `Path` for every shape on the page.

**Tessellation is redone for every path on every prepare.** `canvas/gpu.rs:284` `Prepared::new(commands)` walks the command list and calls `mesh(path, rule, color, &mut result.vertices)` at `:335` for each `Command::Path`. The `HashMap` at `:287` caches *clip* meshes, but it is a local created per call, so it does not survive a frame. There is no cache keyed by path identity anywhere.

Together: a camera-only change re-runs `Path::transform` and the full tessellation for every shape, every frame. Nothing about the scene changed.

## Measured cost at the consumer

Release build, Windows 11, Ryzen 9 7950X3D, canvas 1395 × 1253 physical px. Time spent inside `Context2D` recording, which includes the transform and the enqueue but not the GPU work:

| paths on the page | recording time |
| --- | --- |
| 3 | 11 µs |
| 503 | 1 336 µs |
| 5 000 | 8 294 µs |
| 9 800 | 17 389 µs |

About 1.7 µs per path, linear, paid again on every pan and zoom frame. For a scene whose geometry is unchanged this is work that could be close to zero.

## What would help, in rough order of value

1. **Record in model space and apply the camera at draw time.** If `Command::Path` carried the path as authored plus its transform, rather than pre-transformed points, the vertex data for an unchanged shape would be stable across pan and zoom, and the camera would become a uniform the shader applies. This is the change that makes everything else possible. It needs care with stroke geometry, which is resolution-dependent: `context::stroke_outline` currently runs before the transform, so a scaled stroke must either re-outline on a meaningful scale change or move to a shader-side approach.
2. **Cache meshes across frames**, keyed by path identity and the parameters that actually affect the triangles — the fill rule, and a scale bucket if strokes or curve flattening depend on it. `Prepared::new` already caches clip meshes inside one call by `Arc` pointer; the same idea across calls, with a bounded cache, would remove the dominant cost for static scenes even without change 1.
3. **A retained or replay path.** A way for an application to say "the scene is unchanged, only the camera moved" and have the toolkit reuse the previous frame's prepared buffers. This is the most direct expression of what a design tool needs and would subsume both of the above from the consumer's point of view.
4. **Damage or dirty-region support**, so a change to one shape does not require re-preparing the whole page. Lower value than the above for this consumer, since its common case is a camera move affecting everything.

## Acceptance

- A benchmark in the lurq repository that pans and zooms a static scene of, say, 5 000 paths and reports prepare time per frame before and after. State the machine.
- Correctness unchanged: the existing canvas pixel tests must pass untouched on software, wgpu and dx12, including the clipping, image, text, alpha and DPI cases.
- Any cache is bounded, and the bound is stated with the reasoning behind its size.
- If change 1 is taken, say explicitly what happens to stroke width and curve flattening under zoom, since those are the parts that legitimately depend on scale.

## What this is not

This is not the whole of the consumer's latency problem. Kontur's own paint scheduling draws one batch of 256 items per presented frame, which costs far more than tessellation — 700 ms on a 9 800-path page, of which only 17 ms is the recording measured above. That is a Kontur defect and is being fixed separately as KONTUR-69. The toolkit-side work described here is the remaining, smaller part, and it is the part the application cannot fix for itself.

## Related reports in this folder

[[lurq-form-focus-api-bug-report]], [[lurq-mcp-masked-input-value-bug-report]], [[lurq-close-veto-native-menu-handoff]], [[lurq-password-mask-glyph-bug-report]].
