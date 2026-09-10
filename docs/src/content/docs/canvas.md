---
title: Canvas 2D
description: Persistent drawing through existing element refs, with paths, clipping, text, and images.
---

# Canvas 2D

Enable `canvas` alongside your window and renderer features:

```toml
lurq = { path = "../lurq/crates/lurq", features = ["canvas", "winit", "wgpu"] }
```

Canvas is available in the development checkout; it is not in the published 0.18.0 release. DX12 supports the same drawing API. `canvas` enables raw RGBA image transport and tiny-skia; add `image` for PNG/JPEG/WebP/GIF/BMP/TIFF decoding and `resources` for resource loading.

## Use the existing ref

```rust
use lurq::{components::Canvas, core::ElementRef};

let reference = ElementRef::new(); // In a component: ctx.element_ref().
let element = Canvas::new()
  .ref_element(reference.clone())
  .width(480.0)
  .height(240.0);

// After layout, for example in after_layout() or an input handler:
if let Some(canvas) = reference.as_canvas() {
  let draw = canvas.context_2d();
  draw.set_fill_style("#60a5fa");
  draw.fill_rect(20.0, 20.0, 100.0, 60.0);
}
```

`core::ElementRef`, `core::ElementRefMut`, and the borrowed `node::ElementRef` returned by tree inspection all expose `as_canvas()`. It returns `None` before the first committed layout, for another node kind, or after removal. A zero-sized attached canvas still returns a handle.

The returned `CanvasHandle` and `Context2D` are owned, cloneable, `Send + Sync` handles. Contexts from one canvas share their drawing state, current path, save stack, and pixels. You can retain a context in application state and draw from input handlers, timers, or workers. There is no required `on_draw` callback.

Do not attach one ref to multiple live nodes. A conflicting canvas binding is rejected with a diagnostic. Clone a ref to access the same node; clone an `Element` to create an independent node and surface.

## First paint and resize

Store the ref when the component is created, attach it in `render`, and acquire the handle in `after_layout`. For a scene that should redraw after a resize, register one metrics observer and retain its subscription:

```rust
use std::sync::Mutex;
use lurq::{
  app::{component::Component, ctx::Ctx},
  canvas::CanvasObserver,
  components::Canvas,
  core::ElementRef,
  node::Element,
};

struct Chart {
  reference: ElementRef,
  subscription: Mutex<Option<CanvasObserver>>,
}

impl Component for Chart {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { reference: ctx.element_ref(), subscription: Mutex::new(None) }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    Canvas::new().ref_element(self.reference.clone()).width(480.0).height(240.0)
  }

  fn after_layout(&self) {
    let mut subscription = self.subscription.lock().unwrap();
    if subscription.is_some() { return; }
    let canvas = self.reference.as_canvas().unwrap();
    let draw = canvas.context_2d();
    *subscription = Some(canvas.observe_metrics(move |metrics| {
      draw.reset();
      draw.set_fill_style("#60a5fa");
      draw.fill_rect(12.0, 12.0, (metrics.size.width - 24.0).max(0.0), 40.0);
    }));
  }
}
```

An observer runs immediately with current metrics, then after committed logical-size or backing-scale changes. It runs outside canvas locks. Dropping `CanvasObserver` unsubscribes; the surface keeps a weak callback, so a callback may capture its context without creating a surface ownership cycle. The example intentionally redraws its scene on both kinds of change.

`Canvas::new()` has a 300 by 150 logical-pixel intrinsic size, subject to normal layout constraints. Width and height size the node. Padding reduces the drawable content rectangle. Bitmap dimensions are `ceil(content_size * display_scale)` on each axis. Drawing does not change layout size.

| Change | Result |
| --- | --- |
| Compatible rerender at the same size | Preserves surface identity, pixels, state, and path. |
| Keyed sibling reorder | Preserves the matching surface. |
| Changed key, incompatible kind, removal, or remount | A replacement receives a new surface; old handles stay with the old surface. |
| Logical content resize | Clears pixels, state, save stack, and path; keeps surface identity. |
| Display-scale change only | Resamples pixels and saved clips, preserving logical state. Redraw for sharper detail if needed. |
| Node opacity, background, transform, or ordinary window redraw | Changes presentation without reapplying drawing operations. |

State defaults are black fill and stroke, alpha 1, identity transform, line width 1, butt caps, miter joins with limit 10, no dash or clip, smoothing enabled, left text alignment, and alphabetic baseline. The default font snapshots the theme's default typography at first readiness. `reset()` and logical resize restore that initial font snapshot.

## Drawing operations

All drawing coordinates and stroke widths use canvas-local logical pixels. Rotation and arc angles use radians. Solid paint setters accept lurq `Color` or checked `#RGB`, `#RGBA`, `#RRGGBB`, and `#RRGGBBAA` strings. Invalid style strings and non-finite numeric assignments leave the old value unchanged. Alpha must be between 0 and 1; line width and miter limit must be positive.

| Area | Methods and types |
| --- | --- |
| State | `save`, `restore`, `reset`; fill/stroke style, global alpha, line width/cap/join/miter, dash/offset, smoothing getters and setters. |
| Rectangles | `fill_rect`, `stroke_rect`, `clear_rect`, and whole-surface `clear`. |
| Transforms | `get_transform`, `set_transform`, `reset_transform`, `transform`, `translate`, `scale`, `rotate`; uses `Transform2D`. |
| Geometry | `begin_path`, `close_path`, `move_to`, `line_to`, `quadratic_curve_to`, `bezier_curve_to`, `rect`, uniform-radius `round_rect`, `arc`, `arc_to`, `ellipse`. |
| Path painting | `fill`, `fill_with_rule`, `stroke`, `fill_path`, `stroke_path`; `FillRule::{NonZero, EvenOdd}`. |
| Clipping | `clip`, `clip_with_rule`, `clip_path`. Clips intersect; `restore` restores the saved clip. |
| Hit tests | `is_point_in_path`, `is_point_in_path2d`, `is_point_in_stroke`, `is_point_in_stroke_path`. |
| Text | `set_font`, `fill_text`, `measure_text`, alignment/baseline setters; `CanvasFont`, `TextMetrics`, `TextAlign`, `TextBaseline`. |
| Images | `draw_image`, `draw_image_scaled`, `draw_image_region`. |
| Inspection | Handle `surface_id`, `size`, `pixel_size`, `scale_factor`, `metrics`, `status`, `is_attached`, `snapshot`. |

`clear_rect` erases through the current transform and clip, independently of global alpha. `clear` erases the entire bitmap while keeping state and path. `reset` also restores defaults and discards the save stack and path. Saving and restoring state does not restore the current path or undo pixels.

The current path captures the transform when geometry is added. A separate `Path2D` stores reusable geometry and applies the context's transform when filled, stroked, clipped, or hit-tested. `Path2D::add_path` accepts an explicit transform. Hit-test points are canvas-local, unaffected by the current drawing transform or clip. Curve hit testing uses flattened vector geometry, not pixel alpha.

Text uses lurq's font database and aliases with cosmic-text shaping and Swash rasterization. Register fonts on the app before layout. Set a typed `CanvasFont` with family, logical size, weight, and style; an empty family selects the sans-serif fallback. Text is a single line: newlines and tabs become spaces. It is neither selectable nor part of layout. `measure_text` returns advance width and ink/font bounds relative to the selected alignment and baseline. Text methods return `Result` for unavailable services or oversized work. `stroke_text`, CSS font strings, wrapping, and `max_width` are not implemented.

Image sources must be immutable, nonempty CPU RGBA8 `ImageData`. Source regions use image pixels; destinations use logical canvas units. `draw_image_region` takes `[x, y, width, height]` for each region. Negative sizes extend the region in the opposite direction without mirroring; out-of-bounds source crops shrink the destination proportionally. The call copies source pixels immediately. Animated, streaming, native GPU, and video sources return `UnsupportedImage`.

## Pointer input

Mouse event coordinates are already window-logical. Convert them to the content rectangle, including ancestor transforms and padding:

```rust
let reference = canvas_ref.clone();
Canvas::new().ref_element(canvas_ref).on_click(move |event: lurq::app::events::MouseEvent| {
  let Some(canvas) = reference.as_canvas() else { return; };
  let Some((x, y)) = canvas.point_from_window(event.x, event.y) else { return; };
  canvas.context_2d().fill_rect(x - 2.0, y - 2.0, 4.0, 4.0);
});
```

Conversion returns `None` when detached or when the presentation transform cannot be inverted. It does not invert the drawing transform or clamp to canvas bounds. Individual painted shapes have no automatic event targets; use geometry hit tests or overlay normal controls. Keep accessible labels and controls in ordinary UI nodes.

## Persistence, scheduling, and cost

This first implementation rasterizes synchronously into a bounded, premultiplied RGBA8 CPU bitmap using tiny-skia. WGPU and DX12 compose the resulting image through their existing image pipelines. Internal canvas source-over blending uses sRGB channel values; the window's image composition uses its existing linear-light blending. `snapshot()` returns a synchronous copy of straight-alpha sRGB RGBA8 pixels, dimensions, and content revision. It includes canvas pixels only, without node backgrounds, borders, or ancestor presentation.

Drawing updates the bitmap immediately, increments its content revision, and wakes the owning window. Calls before a paint are coalesced. Drawing alone does not mark reactive signals or layout dirty. No command history is replayed on window redraw. An idle canvas does not request continuous animation. Winit installs the waker automatically; a custom host must install `Tree::set_canvas_waker` and respond by scheduling a pass.

Methods serialize through the surface lock. A sequence such as set-style plus draw is not an atomic transaction across threads; coordinate shared multi-call sequences in the application. Callbacks and window wakeups occur after releasing internal locks.

After removal or window destruction, retained contexts can still draw and snapshot their original bitmap, with no window wake and no pending-command backlog. The ref loses access. Dropping the last handle releases the CPU surface. GPU device recreation can reupload the retained CPU image.

Each changed surface currently converts and uploads its full bitmap. Large, frequently changing canvases can consume significant CPU time and upload bandwidth. Native GPU path execution, dirty-region uploads, gradients, patterns, shadows, filters, additional blend modes, canvas-to-canvas drawing, pixel upload, and automatic animation callbacks are follow-up work.

Limits are 16,384 pixels per backing dimension, 16,777,216 total backing pixels, 128 saved states, 65,536 path segments, and 64 MiB of distinct retained clips. Excess path segments are ignored; save/clip limits appear in `status().error`. An oversized logical resize has no backing bitmap and reports `SurfaceTooLarge`. If scaling exceeds the backing allocation limit, the previous backing scale and pixels are retained with `SurfaceTooLarge`; if saved clips exceed their budget, they are retained with `StateLimit`. Text has separate limits for input length, font size, cache bytes, and raster work. These are per-operation/per-surface bounds, not a global app memory budget; uploads, snapshots, and temporary raster work require additional memory.

## Examples and checks

```text
cargo run -p lurq --example canvas --features canvas,winit,wgpu
cargo test -p lurq --features canvas --test canvas_tests
```

On Windows, the hidden-window capture harness checks actual composition and incremental uploads on either backend:

```text
cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,wgpu -- wgpu
cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,dx12 -- dx12
```

The vocabulary follows the [HTML Canvas specification](https://html.spec.whatwg.org/multipage/canvas.html), with deliberate lurq choices for typed refs, sizing, display scale, defaults, and the supported subset. This is not a claim of full browser conformance.
