# KONTUR-153 — gradients, shadows, blur, blend modes and isolated layers on `Context2D`

Merged into `master` by `a7e0873` and tagged **v0.20.0**. This is the implementation handoff from `codex/kontur-153-canvas-effects`; the evidence below records that branch's validation. The original handoff preceded the merge and tag. Registry publication is managed separately by the repository's publish workflow.

Closes lurq [#15](https://github.com/ivanzaida/lurq/issues/15) (gradients),
[#16](https://github.com/ivanzaida/lurq/issues/16) (shadows),
[#18](https://github.com/ivanzaida/lurq/issues/18) (blend modes) and
[#19](https://github.com/ivanzaida/lurq/issues/19) (isolated layers). Issue
[#17](https://github.com/ivanzaida/lurq/issues/17) is implemented for **layer
blur** and left open for **backdrop blur**; the reason is below.

## Why the design is the shape it is

Two decisions carry the whole slice.

**A gradient is a paint, not geometry.** The ramp is sampled into 256
premultiplied texels once per distinct stop list and uploaded through the
existing asset cache; the point in the gradient's own frame is written into the
vertex `uv` that solid paths leave at zero, while the cached model-space
triangles are copied out — exactly where the colour was already applied. So the
**mesh-cache key is unchanged** (path, fill rule, curve flattening bucket), a
gradient adds no vertices, and a page of gradient-filled shapes demands the same
cache entries as the same page filled with solids. That was the property the
consumer's pass budget depends on, so it was the constraint the design started
from rather than something checked afterwards.

**A shadow or a blur is an image.** Both are rasterised on the CPU, in device
pixels, into a premultiplied RGBA image and then drawn through the image command
that already exists. One implementation therefore serves the software backend
and both native backends, and the cost of an effect is stated in the same units
as every other command instead of in a new currency. Three bounds keep it
finite: a wide blur is rasterised at a reduced scale (a wide blur has no detail
to lose), the raster is clipped to the surface because nothing outside it can be
seen, and what remains is capped at `MAX_EFFECT_PIXELS`; over the cap the draw
reports `StateLimit` rather than rasterising.

**Everything that needs the destination goes through one mechanism.** An
isolated layer splits the tile pass: the resolved tile is copied into a
tile-sized save, the layer's own draws run in a pass that starts transparent,
and the composite puts the save back and draws the layer over it. A blend mode
other than `Normal` is the same split with the save bound as a second texture,
so the shader writes the finished W3C composite itself and needs no
fixed-function blending. A blended *draw* is an isolation of one draw. That is
why eighteen blend modes and group opacity cost the same machinery, and why the
software backend can be the reference for both: its layer is a pixmap and its
composite is the same formula in Rust.

## What a consumer has to know

| | |
| --- | --- |
| Gradient geometry | `in_box(x, y, w, h)` is in the **user space at draw time**; the paint does not capture the context transform at construction. The box is normalised, then rotated, then divided by `size`, so a rotation means the same thing in a wide box as in a square one. |
| Gradient interpolation | Colour and alpha separately, premultiplied afterwards — what the software backend's own gradients do, so the two agree. |
| Shadow units | Offset, blur and spread are in **user space** and scale and rotate with the transform. This deliberately differs from HTML Canvas, where shadows ignore the transform: a design tool's shadow belongs to the node and has to follow it when the view zooms. |
| Placement | A blurred raster is placed on whole device pixels, so a shadow can sit up to half a pixel from where a sub-pixel pan would put it. That is what lets a pan reuse the cached raster. |
| Refusals | An unusable gradient (fewer than two stops, non-ascending offsets, a collapsed box) **draws nothing** rather than a colour of its own choosing. A gradient fill is refused for `fill_text`/`measure_text` with `UnsupportedPaint`. `end_layer` without `begin_layer` is `UnbalancedLayer`. Depth over `MAX_LAYER_DEPTH` is `StateLimit`. |
| Not in this slice | Backdrop blur; spread on a text shadow (a raster has no outline to dilate); shadows and filters on `draw_image_*` (the shadow of an image box is the shadow of its rectangle). All three are stated in the docs rather than silently ignored. |

## Bounds and measured costs

| Operation | Commands | Vertices | Queue bytes | Renderer |
| --- | --- | --- | --- | --- |
| Gradient fill/stroke | 1 | same mesh as the solid fill | path + 1 KiB per distinct stop list | one 256 × 1 asset, shared |
| Shadow, outer or inner | 1 image | 6 | `width * height * 4` of the raster | one CPU rasterisation, cached by content |
| `Filter::Blur` | 1 image, replacing the draw | 6 | as above | as above |
| Blend mode on one draw | +2 | 0 | 2 command headers | +1 tile resolve, +2 whole-tile quads per tile touched |
| `begin_layer`/`end_layer` | +2 | 0 | 2 command headers | as above, plus 2 × TILE² RGBA (2 MiB) per open depth, allocated on first use and shared by every canvas of one renderer |

`MAX_LAYER_DEPTH = 8`, `MAX_EFFECT_PIXELS = 4 194 304`, `MAX_SHADOW_BLUR =
MAX_BLUR_RADIUS = MAX_SHADOW_SPREAD = 512` user units. The effect raster cache is
process-wide, 32 MiB and 512 entries, random eviction — the same policy, and for
the same reason, as the mesh cache.

## Evidence

One fixture — `lurq::canvas::effects_scene`, three gradients, a gradient stroke,
an outer shadow with spread, an inner shadow, a layer blur, all eighteen blend
modes as a swatch row, and a nested isolated layer — is drawn by the software
suite, the wgpu suite and the `canvas_capture_check` example, so a difference
between backends is a difference in the backend and not in the fixture.

| Check | Result |
| --- | --- |
| `cargo test -p lurq --features canvas --test canvas_tests` | 38 passed (28 pre-existing, 10 new) |
| `cargo test -p lurq --features canvas,perf_profile --lib` | 146 passed, 1 ignored |
| `cargo test --workspace` (default features) | all suites passed |
| `cargo test -p lurq --features canvas,wgpu --lib gpu_canvas_pixels -- --ignored` | passed (no regression) |
| `cargo test -p lurq --features canvas,wgpu --lib gpu_canvas_effects -- --ignored` | passed, RTX 5080 / Vulkan |
| `cargo run --example canvas_capture_check --features canvas,screenshot,wgpu -- wgpu` | effects parity **21 of 262 144 subpixels** outside a 16-level tolerance |
| `cargo run --example canvas_capture_check --features canvas,screenshot,dx12 -- dx12` | effects parity **21 of 262 144 subpixels** outside a 16-level tolerance |
| `cargo fmt --all --check` | clean for every file this branch touches; fails on 9 files `origin/master` already fails on (`app/profiler.rs`, `app/synthetic_input.rs`, `svg/*`, four test files) |
| `cargo clippy --workspace --all-targets --features canvas,wgpu,dx12,screenshot,winit -- -D warnings` | the findings are **identical, file for file and count for count, to `origin/master`** — the gate is red on master and this branch adds nothing to it |

The parity assertion also checks the property that removes the consumer's
`GroupOpacity` refusal directly: inside a layer at alpha 0.5, the overlap of two
shapes has the same alpha as either shape alone.

## Consumer smoke test

The Kontur desktop was built in a scratch clone against this worktree through
`[patch.crates-io]` (never committed to the Kontur repository, and
`H:/projects/pencil-web` was not modified; the clone's `=0.19.5` requirement was
relaxed to `=0.20.0` so the patch could resolve).

- `cargo build -p kontur-desktop --bins` — finished; the application compiles
  unchanged against the new toolkit.
- `cargo test -p kontur-desktop --test canvas_limits` — 3 passed, including
  `a_dense_rounded_rectangle_page_keeps_vertex_head_room_in_every_pass`, the
  two-pass fit on the 9 800-item rounded page. The desktop's `charge` is still a
  bound: gradients add no vertices and the commands this release adds
  (`BeginLayer`, `EndLayer`) are only enqueued by a caller that asks for them.
