---
title: Canvas 2D
description: Persistent drawing through existing element refs, with paths, clipping, text, and images.
---

# Canvas 2D

Enable `canvas` alongside your window and renderer features:

```toml
lurq = { path = "../lurq/crates/lurq", features = ["canvas", "winit", "wgpu"] }
```

Canvas is available in the development checkout; it is not in the published 0.18.0 release. DX12 supports the same drawing API. `canvas` enables raw image transport, path geometry, tessellation, and the CPU reference renderer; add `image` for PNG/JPEG/WebP/GIF/BMP/TIFF decoding and `resources` for resource loading.

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

Image sources must be immutable, nonempty CPU RGBA8 `ImageData`. Source regions use image pixels; destinations use logical canvas units. `draw_image_region` takes `[x, y, width, height]` for each region. Negative sizes extend the region in the opposite direction without mirroring; out-of-bounds source crops shrink the destination proportionally. The call retains the immutable source through a shared reference. The renderer uploads and premultiplies a source once, then reuses its cached GPU texture. Animated, streaming, native GPU, and video sources return `UnsupportedImage`.

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

`Canvas::new()` draws into a persistent GPU texture on both WGPU and native DX12. Calls record ordered work; paths are tessellated on the CPU, then rasterized and blended on the GPU. Text uses cached CPU shaping/glyph rasterization and GPU image drawing. The default canvas has no full-size CPU bitmap. A new blank canvas defers its backing allocation until drawing or readback needs it.

The renderer processes only new commands. A shared 512 × 512 tile surface provides 4-sample antialiasing; touched tiles are seeded from the existing texture, drawn, resolved, and copied back on the GPU. A small edit does not upload, convert, or copy the whole canvas. Full clears discard obsolete queued drawing while preserving resize and snapshot barriers. Idle surfaces retain pixels without replaying history or requesting continuous frames.

Internal source-over blending uses premultiplied sRGB channel values. Image sources are premultiplied before filtering. Window composition converts the result to straight linear color for the existing image pipeline, including node backgrounds, borders, clipping, radius, and ancestor opacity.

Drawing increments the content revision, wakes the window, and coalesces presentation. It does not dirty reactive state or layout. Winit installs the waker automatically. Custom hosts must install `Tree::set_canvas_waker` and schedule a pass; custom renderer wrappers must forward `RenderEngine::prepare_canvases`, including surfaces culled from the visible image list.

Methods serialize through the surface lock. A set-style plus draw sequence is not an atomic transaction across threads; coordinate multi-call sequences in the application. Callbacks and wakeups run outside internal locks.

### Explicit snapshots

`snapshot()` returns a `CanvasReadback` ticket. It captures commands before that call, excluding later drawing, node styling, and window composition. Poll from the UI thread:

```rust
let readback = canvas.snapshot();
// After the host has rendered, in a later event/tick:
if let Some(result) = readback.try_take() {
  let snapshot = result?; // width, height, straight-alpha sRGB rgba, revision
}
```

A worker can use `wait_timeout(Duration)` while the UI continues rendering. Never wait for a queued GPU readback on the rendering thread. Readbacks are bounded to two outstanding requests per canvas and eight process-wide, including in-flight GPU copies. Dropping a queued request, detaching, resizing, or losing the renderer completes its ticket with an error. A full `clear()` preserves prior snapshot requests. Submitted pixels are never reapplied following a failed window presentation.

### Lifetime and resource limits

Removing the node invalidates the ref and rejects further GPU drawing with `Detached`. Old handles retain their identity and never target a replacement. The renderer releases detached backing textures after pending GPU use. Device loss reports `RendererLost`; there is no retained CPU checkpoint or complete drawing history, so applications must redraw after recovery.

Use `Canvas::new().software()` explicitly for the synchronous tiny-skia reference renderer, tests, or a host without GPU canvas support. Its readback ticket is ready immediately. Software mode retains the earlier detached-bitmap behavior and full-bitmap upload costs. A renderer without GPU canvas support reports `UnsupportedBackend` for the default canvas.

Limits include 16,384 pixels per backing dimension (also subject to device limits), 16,777,216 backing pixels, 128 saved states/clip levels, 65,536 input path segments, and 8 MiB of distinct retained vector clips. Queued plus encoding work is charged against 64 MiB per canvas and 8,192 commands. Source/clip references are conservatively charged per queued draw. Overflow reports `QueueFull` and rejects that operation; render pending work before continuing, or clear/reset obsolete work. Tessellation expansion is capped at 1,048,576 output vertices. GPU image/text caches use a 64 MiB per-renderer charge after each frame, with at least 64 KiB charged per texture to bound small-texture overhead; shaped text has an 8 MiB app cache in addition to the bounded glyph cache. Software clips retain their separate 64 MiB limit.

`status()` exposes attachment, metrics, content revision, errors, charged pending bytes, backing GPU bytes, and cumulative submitted batches, vertices, tiles, and source-upload bytes. A 3840 × 2160 backing needs 33,177,600 color bytes. Antialiasing scratch is shared across canvases and fixed in size: approximately 9 MiB with D24S8, with WGPU depth/stencil allocation depending on the backend. Queues, geometry buffers, source caches, explicit readbacks, and resources awaiting GPU fences add to those figures; these limits are not a global application memory cap.

Gradients, patterns, shadows, filters, additional blend modes, canvas-to-canvas drawing, pixel upload, and automatic animation callbacks remain outside the initial subset.

## Examples and checks

```text
cargo run -p lurq --example canvas --features canvas,winit,wgpu
cargo test -p lurq --features canvas --test canvas_tests
cargo test -p lurq --features canvas,wgpu --lib gpu_canvas_pixels -- --ignored
```

On Windows, the hidden-window capture harness checks actual composition, tile updates, ordered GPU readbacks, culled drawing, and scale changes on either backend:

```text
cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,wgpu -- wgpu
cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,dx12 -- dx12
```

The vocabulary follows the [HTML Canvas specification](https://html.spec.whatwg.org/multipage/canvas.html), with deliberate lurq choices for typed refs, sizing, display scale, defaults, and the supported subset. This is not a claim of full browser conformance.
