---
title: Drawing With Your Own GPU Code
description: GpuViewport, a box in the layout whose pixels the app draws with its own wgpu render passes on lurq's device, composited without a copy.
---

# Drawing With Your Own GPU Code

A `GpuViewport` is a box in lurq's layout whose pixels the app draws itself, with its own wgpu render passes,
into a texture made on lurq's own GPU device. lurq samples that texture where the box is laid out, in the same
frame and without copying it. It is for what lurq's elements and [Canvas 2D](../canvas/) do not draw: a 3D
scene, the viewport of a game or an editor, custom shaders, video or other frames that are already on the GPU.

Everything else about the box is lurq's: it is sized and placed by the layout, clipped by its parents, rounded,
faded and transformed like an image, and takes the pointer, the wheel, the keys and drags through the same
handlers as any element.

It is drawn by the `wgpu` renderer. See [Where It Draws](#where-it-draws).

## The Pieces

| Piece | What it is |
| --- | --- |
| `images::WgpuExternalImageSlot` | What the UI and the app's renderer share: the texture view the viewport shows. Cheap to clone; the clones are the same slot. |
| `components::GpuViewport::new(slot)` | The element that shows the slot's view where it is laid out. |
| `app::wgpu_render::WgpuFrameExtension` | The app's renderer: a trait whose methods lurq's wgpu renderer calls around each frame. |
| `WgpuRenderEngine::with_frame_extension(ext)` | Gives the extension to the renderer, in the [render engine factory](../app-runtime/#render-engine-factory). |
| `app::wgpu_render::SharedWgpuContext` | lurq's wgpu instance, adapter, device, queue and surface format, given to the extension. |
| `WgpuFrameInfo` | The frame being prepared: where the viewport is (`viewport`, `visible_viewport`), the window's `scale_factor` and its surface's size. |
| `lurq::wgpu` | wgpu itself, at the version lurq draws with. |

An app writes:

1. A `WgpuExternalImageSlot::new()`, cloned into the UI and into the renderer.
2. An extension: a struct that implements `WgpuFrameExtension`. In `initialize` it makes its pipelines and
   buffers with lurq's device; in `prepare`, before each frame's UI pass, it finds the viewport's size, makes
   a texture of that size when it has none or the size changed, gives the slot a view of it
   (`slot.set_texture_view(view, width, height)`), renders into it and submits on lurq's queue.
3. The render engine factory, which makes a `WgpuRenderEngine` with an extension of its own each time it is
   called.
4. A `GpuViewport::new(slot)` in the UI, sized as any box is.

## Dependencies

`GpuViewport` needs the `wgpu` renderer and pictures (`image`, or `canvas`), which the default features have.
lurq re-exports the wgpu it draws with as `lurq::wgpu`: the app writes `use lurq::wgpu;` and its `wgpu::Device` is
lurq's, with no version of wgpu to keep in step in its own `Cargo.toml`.

```toml
[dependencies]
lurq = "0.63"
```

## A Complete Example

A triangle the app draws with its own pipeline, in a viewport of 480 by 270 in a column, turned by dragging it.
It is `crates/lurq/examples/gpu_viewport.rs` in the repository:

```powershell
cargo run -p lurq --example gpu_viewport --features winit,wgpu,image
```

```rust
use lurq::{
  app::{
    App, Tree,
    component::{Component, DevtoolsInspectable},
    ctx::Ctx,
    events::{DragEvent, MouseButtonMask},
    wgpu_render::{SharedWgpuContext, WgpuAppError, WgpuFrameExtension, WgpuFrameInfo, WgpuRenderEngine},
    winit_shell::WinitWindow,
  },
  components::{Column, GpuViewport, Text},
  core::Signal,
  images::WgpuExternalImageSlot,
  node::Element,
  wgpu,
};

/// The texture's format: the card encodes what the shader writes as a picture's colors are, and decodes it as lurq
/// samples it. A linear format (`Rgba8Unorm`, `Rgba16Float`) would show the same colors.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const SHADER: &str = r#"
struct Params { angle: f32, aspect: f32, _pad: vec2<f32> }
@group(0) @binding(0) var<uniform> params: Params;

struct Out {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> Out {
  var corners = array<vec2<f32>, 3>(vec2(0.0, 0.7), vec2(-0.6, -0.5), vec2(0.6, -0.5));
  var colors = array<vec3<f32>, 3>(vec3(1.0, 0.25, 0.2), vec3(0.2, 0.8, 0.3), vec3(0.2, 0.4, 1.0));
  let p = corners[index];
  let c = cos(params.angle);
  let s = sin(params.angle);
  var out: Out;
  out.position = vec4((p.x * c - p.y * s) / params.aspect, p.x * s + p.y * c, 0.0, 1.0);
  out.color = colors[index];
  return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
  return vec4(in.color, 1.0);
}
"#;

/// What the UI and the renderer share: the slot the viewport shows, and the triangle's angle.
#[derive(Clone)]
struct Scene {
  slot: WgpuExternalImageSlot,
  angle: Signal<f32>,
}

impl PartialEq for Scene {
  fn eq(&self, other: &Self) -> bool {
    self.slot.image_id() == other.slot.image_id() && self.angle.id() == other.angle.id()
  }
}

impl DevtoolsInspectable for Scene {}

/// The app's renderer: an extension of lurq's wgpu renderer, called before each frame's UI pass.
struct Triangle {
  scene: Scene,
  gpu: Option<Made>,
}

/// What the extension made with lurq's device.
struct Made {
  pipeline: wgpu::RenderPipeline,
  params: wgpu::Buffer,
  bind_group: wgpu::BindGroup,
  /// The texture the viewport shows.
  texture: Option<wgpu::Texture>,
  /// The size and the angle drawn into it last.
  drawn: Option<(u32, u32, f32)>,
}

impl WgpuFrameExtension for Triangle {
  fn initialize(&mut self, gpu: &SharedWgpuContext) {
    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("triangle"),
      source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("triangle"),
      layout: None,
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(FORMAT.into())],
        compilation_options: Default::default(),
      }),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview_mask: None,
      cache: None,
    });
    let params = gpu.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("triangle params"),
      size: 16,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("triangle"),
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: params.as_entire_binding(),
      }],
    });
    self.gpu = Some(Made {
      pipeline,
      params,
      bind_group,
      texture: None,
      drawn: None,
    });
  }

  fn prepare(&mut self, gpu: &SharedWgpuContext, frame: WgpuFrameInfo<'_>) {
    // Where the viewport is in this frame, in physical pixels; `None` when this window does not show it.
    let (Some(made), Some(rect)) = (&mut self.gpu, frame.viewport(self.scene.slot.image_id())) else {
      return;
    };
    let width = (rect.width.round() as u32).max(1);
    let height = (rect.height.round() as u32).max(1);
    let angle = self.scene.angle.get_untracked();
    if made.drawn == Some((width, height, angle)) {
      // Nothing changed: lurq samples the texture as it is.
      return;
    }
    // A texture of the viewport's size in pixels, made again when the viewport is resized.
    let texture = match &made.texture {
      Some(texture) if (texture.width(), texture.height()) == (width, height) => texture,
      _ => {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
          label: Some("triangle target"),
          size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: FORMAT,
          usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
          view_formats: &[],
        });
        // The viewport shows this texture from now on, without a copy. The slot refuses a texture lurq cannot
        // sample, which this is not.
        self
          .scene
          .slot
          .set_texture_view(texture.create_view(&Default::default()), width, height)
          .expect("a 2D texture to be sampled, in a format lurq reads");
        made.texture.insert(texture)
      }
    };

    let mut params = [0u8; 16];
    params[..4].copy_from_slice(&angle.to_ne_bytes());
    params[4..8].copy_from_slice(&(width as f32 / height as f32).to_ne_bytes());
    gpu.queue.write_buffer(&made.params, 0, &params);

    let view = texture.create_view(&Default::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("triangle"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.02,
              g: 0.02,
              b: 0.05,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
        })],
        ..Default::default()
      });
      pass.set_pipeline(&made.pipeline);
      pass.set_bind_group(0, &made.bind_group, &[]);
      pass.draw(0..3, 0..1);
    }
    // Submitted before lurq's own pass, on the same queue: this frame shows it.
    gpu.queue.submit([encoder.finish()]);
    made.drawn = Some((width, height, angle));
  }

  fn error(&mut self, error: &WgpuAppError) {
    // A wgpu call of this extension failed, which ends nothing: lurq has logged it, and what failed draws nothing.
    eprintln!("the triangle's renderer: {error}");
  }

  fn device_lost(&mut self) {
    // Everything made with the lost device is gone: let go of it. lurq empties the slot itself.
    self.gpu = None;
  }
}

/// The window's content: the viewport and a line under it.
struct Viewer {
  scene: Scene,
}

impl Component for Viewer {
  type Props = Scene;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      scene: ctx.props::<Scene>().clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    // Read here, the angle renders this component again when a drag changes it, which draws a frame.
    let degrees = self.scene.angle.get().to_degrees();
    let angle = self.scene.angle.clone();
    Column::new()
      .padding(24.0)
      .spacing(12.0)
      .child(
        GpuViewport::new(self.scene.slot.clone())
          .size(480.0, 270.0)
          .rounded(8.0)
          .start_drag_buttons(MouseButtonMask::LEFT)
          .on_drag_move(move |event: DragEvent| angle.set(angle.get_untracked() + event.delta_x * 0.01)),
      )
      .child(Text::new(&format!(
        "Turned {degrees:.0} degrees. Drag the triangle to turn it."
      )))
  }
}

fn main() {
  let scene = Scene {
    slot: WgpuExternalImageSlot::new(),
    angle: Signal::new(0.0),
  };
  let mut app = App::new();
  let mut tree = Tree::new();
  // An engine for each window, and another after a lost device: each with an extension of its own.
  let for_engines = scene.clone();
  tree.set_render_engine_factory(move || {
    let triangle = Triangle {
      scene: for_engines.clone(),
      gpu: None,
    };
    Box::new(WgpuRenderEngine::new().with_frame_extension(triangle))
  });
  tree.mount_root::<Viewer>(&mut app, scene);
  WinitWindow::new(app, tree)
    .with_size(528, 360)
    .with_title("GPU viewport")
    .run();
}
```

The slot and the angle are made before the window, because the factory that makes the renderer needs them
before the root component exists. They reach the component as its props.

## The Frame, Step By Step

`WgpuFrameExtension` has six methods, all with empty defaults:

- **`initialize(&mut self, gpu)`** runs once for each engine, at its first frame, after lurq has made its device
  and before the first `prepare`. Make pipelines, buffers and samplers here, with `gpu.device`. Never make an
  instance, adapter or device of your own: a texture of another device cannot be sampled by lurq.
- **`prepare(&mut self, gpu, frame)`** runs before every frame the window draws, before lurq encodes its UI pass.
  `frame.viewport(slot.image_id())` is where the viewport is in this frame (`WgpuViewportRect`, in physical
  pixels), or `None` when the frame does not show it. Render into the texture, then `gpu.queue.submit(..)`: the
  work goes on the queue before lurq's own, so the frame that is being drawn shows it. The frame also says the
  window's `scale_factor` and its surface's size, `surface_width` and `surface_height`, in pixels.
- **`wants_redraw(&self)`** keeps the window drawing frames while it returns `true`. See
  [When lurq Draws](#when-lurq-draws).
- **`error(&mut self, error)`** is told of a wgpu call of the app's that failed. See [Errors](#errors).
- **`device_lost(&mut self)`** says that the device the extension was given is lost. See
  [A Lost Device](#a-lost-device).
- **`shutdown(&mut self)`** runs when the engine is dropped: its window closed, the app ended, or its device
  was lost.

The slot is told about a texture with `slot.set_texture_view(view, width, height)`, once for each texture, not
once for each frame: what is rendered into the texture later is shown without telling the slot again. The slot
holds the view, and so the texture, until another view is given or `slot.clear()` empties it; an empty slot
draws nothing, and what is under the viewport shows through. `set_texture_view` returns the slot's version, or
an error for a texture lurq cannot sample (see [The Texture](#the-texture)); `clear` returns the version too, and
`slot.version()` reads it.

## The Texture

- **Made with `gpu.device`**, a 2D texture of one sample with `TEXTURE_BINDING` in its usage, and whatever else
  the app draws with: `RENDER_ATTACHMENT` for a render pass, `COPY_DST` for `queue.write_texture`,
  `STORAGE_BINDING` for a compute shader. A multisampled scene is resolved into a texture of one sample, which
  the slot is given.
- **Its format is one lurq reads as filtered color**, and each such format shows the colors it holds:
  - An sRGB format (`Rgba8UnormSrgb`, `Bgra8UnormSrgb`) holds color encoded as a picture's is. A shader that
    writes into it writes linear color, which the card encodes; bytes written with `queue.write_texture` are a
    picture's bytes. lurq's own pictures are `Rgba8UnormSrgb`.
  - A linear format (`Rgba8Unorm`, `Bgra8Unorm`, `Rgb10a2Unorm`, `Rgba16Float`, `R8Unorm`) holds linear color,
    which is what a shader writes into it. A linear format of 8 bits keeps less of the dark tones than an sRGB
    one; `Rgba16Float` keeps them, clamped to 0 to 1. Bytes encoded as a picture's (a frame of a video, a file)
    are written into an sRGB texture, or into a texture made with `view_formats: &[Rgba8UnormSrgb]` and given
    as a view of that format.
- **A texture lurq cannot sample is refused.** `set_texture_view` returns `Err(WgpuTextureError)` and says so in
  the log, and the slot keeps the view it had: `NotSampled` (no `TEXTURE_BINDING`), `NotTwoDimensional`,
  `Multisampled`, and `Format` for an integer, depth or stencil format, or one of 32-bit floats, which lurq
  does not filter (`Rgba16Float` is filtered).
- **Its alpha is straight**, not premultiplied, and the viewport is blended over what is under it. An opaque
  scene writes an alpha of 1.
- **It fills the viewport's box**, stretched if it is another size, with filtering. A texture of the
  rectangle's rounded size maps one texel to one pixel.

## Size And The Scale Factor

A `GpuViewport` is laid out as any box is: `.size(..)`, `.width(..)` and `.height(..)`, percentages, flex. Given
no size it is 300 by 150 logical pixels, as a [`Canvas`](../canvas/) is and as a browser's `<canvas>` is; given
one side, the other is still the canvas's, since there is no picture whose shape it keeps. A viewport that fills
what holds it is `.size(Dimension::Pct(100.0), Dimension::Pct(100.0))`, or `.flex(1.0)` in a column or a row.

Layout and events are in logical pixels; `frame.viewport(..)` is in the physical pixels of the window's surface,
counted from its top left corner. `frame.scale_factor` is how many of those make a logical pixel: at 150% a
viewport of 480 by 270 is a rectangle of 720 by 405, and its corners may fall between pixels, which is why the
example rounds the size. A line one logical pixel wide is `frame.scale_factor` pixels of the texture.

`frame.viewport(..)` is the whole box, before its transform and whatever clips it: a viewport scrolled half out
of its container still gives its whole size, and lurq clips it as it samples, under its rounded corners
(`.rounded(..)`), `.opacity(..)`, its parents' clips and transforms, as an image's. `frame.visible_viewport(..)`
is the part of it that is shown: the box moved by its transform, cut by the clips of its parents and by the
window's edges, or `None` when none of it is. A renderer that is expensive per pixel draws that part alone,
with a scissor rectangle or a smaller viewport of its pass; the texture is still mapped to the whole box.

**Resizing.** When the window or the layout changes the viewport's size, the next frame's `prepare` sees the
new rectangle before the UI pass, so the app can make a texture of the new size and give its view in time for
that frame, as the example does. Nothing else tells the app of a resize.

## Input

The viewport takes input through the element's own handlers, as any element does (see
[Styling And Events](../styling-events/)):

- **The pointer**: `on_mouse_down`, `on_mouse_up`, `on_mouse_move`, `on_click`, `on_mouse_enter`,
  `on_mouse_leave`, and `.cursor(..)`. `MouseEvent::x` and `y` are in the window's logical pixels: subtract the
  viewport's position (`ElementRef::rect()`) for a point in the viewport, and multiply by the scale factor for
  a pixel of its texture.
- **The wheel**: `on_wheel`, with `event.prevent_default()` so that the wheel zooms the scene and scrolls no
  container around it ([Scroll](../styling-events/#scroll)).
- **Drags**: `.start_drag_buttons(..)` and `on_drag_start`, `on_drag_move`, `on_drag_end`. A drag holds the
  pointer until the button is let go, also outside the window, which is what an orbiting camera needs
  ([A Drag Holds The Pointer](../styling-events/#a-drag-holds-the-pointer)).
- **Keys**: `.focusable(true)` and `on_key_down`, `on_key_up`, `on_focus`, `on_blur`. A game that wants every key,
  Tab and Escape too, builds the viewport with `.captures_keys(true)`
  ([Taking Every Key](../styling-events/#taking-every-key)).
- **Fingers**: `on_tap`, `on_pan`, `on_pinch`, `on_rotate` and the other gestures
  ([Every Gesture](../styling-events/#every-gesture)).

Handlers run on the UI thread. To have the extension react, they change state it reads: a `Signal`, an atomic
or a lock shared with it, as the example's angle is.

## When lurq Draws

lurq draws a frame when something asks for one, and `prepare` runs only for those frames, and for all of them:
a frame drawn because a button under the pointer changed its look runs it too. Skip the work when nothing the
picture depends on changed, as the example does with `drawn`; lurq then samples the texture as it is.

To have a new picture shown:

- **After a change of state**, put the state in a `Signal` that a component reads in `render`. Setting it
  renders that component again and draws a frame, from a handler or from any thread
  ([Futures And Timers](../futures-timers/)).
- **While something moves on its own** (an animation, a simulation, a video), return `true` from
  `wants_redraw`. The window then draws a frame at each refresh of the display. Return `false` when the scene
  is still, and the window goes idle again.

## Threads

lurq calls the extension on the thread that draws the window, the UI thread with `WinitWindow`, inside the
frame: what `prepare` does adds to the frame's time, and the extension does not need to be `Send`.

Rendering that takes long can run on a thread of its own. Clone the `SharedWgpuContext` in `initialize` and
send it to that thread, render there into a texture other than the one the slot shows, and give its view with
`slot.set_texture_view_of(&gpu, view, width, height)` when it is done, then set a signal to have a frame drawn.
`set_texture_view_of` tells lurq which device the view is of, so a view a thread gives late, after its device
was lost, is known and not drawn; `set_texture_view` called outside `initialize` and `prepare` takes the view for
one of the device drawing at that moment.

## Errors

A wgpu call of the app's that wgpu finds invalid (a texture of no pixels, a write past a buffer's end, a shader
that does not compile) ends nothing. lurq says what failed in the log and tells the extension, with a
`WgpuAppError`: its `kind` (`wgpu::ErrorFilter::Validation`, `OutOfMemory` or `Internal`) and wgpu's `message`,
which is for a log and not for a person. What failed does nothing: an invalid pipeline or texture draws
nothing, and the app may make it again.

- What `initialize`, `prepare`, `device_lost` and `shutdown` cause is told to that extension's `error`, right
  after the call that caused it.
- What the app causes elsewhere, outside an error scope of its own (a thread of its own, a handler of the UI), is
  told to the extensions of the next window that draws with the device, before its next `prepare`.

An app that wants an answer at once, a shader the user wrote for one, wraps the call in
`gpu.device.push_error_scope(..)` and reads what its scope returns, as wgpu has it.

lurq watches the device with wgpu's device-lost callback and its handler of uncaptured errors. Do not set
either: yours would replace lurq's. An error of lurq's own drawing is a fault of lurq's, and ends the program.

In a browser, the browser tells an error later, out of the call that made it: there the extension's own calls are
told apart as on a desktop, and an error the app makes outside them ends the program as one of lurq's does.

## A Lost Device

A device is lost when its card is reset (a driver that hung or was updated) or removed. lurq finds it and, on
the UI thread, before its next frame:

1. calls `device_lost` on each extension that was initialized, once. `prepare` is not called again. Let go of
   everything made with the device: pipelines, buffers, textures. Tell any other part of the app that uses it
   to do the same, since while anything made with the lost device is held, the device is held too, and the
   card may give no new device in its place.
2. empties the slots that held views of the lost device. The viewport draws nothing until a view of the new
   device is given.
3. calls `shutdown` and lets go of the engine, and of the engines of the app's other windows, which drew with
   the same device.
4. asks the render engine factory for a new engine for each window, with an extension of its own, and draws
   the windows again. The new extension's `initialize` gets the new device, and its first `prepare` makes the
   texture again and gives the slot its view.
5. tells each window's handler of the app's events, `ctx.window().on_app_event(..)`, with
   `AppEvent::GraphicsDeviceLost`. A part of the app that keeps something of the device outside an extension
   lets go of it there, and makes it again with the device the new extensions are given.

So the app does not mend the old extension: it makes the extension in the factory, as the example does, and
keeps what must outlive the device (the scene, the camera, the angle) outside it. An app that gives its engines a
`WgpuDevice` gets a new device from it after a loss on a desktop; in a browser it is not made again.

## Where It Draws

| Renderer | The viewport |
| --- | --- |
| `wgpu` on a desktop (Direct3D 12, Vulkan or Metal, as wgpu picks) | Drawn. This is what the example and the gallery use. |
| `wgpu` in a browser (WebGPU, or WebGL2 with the `webgl` feature) | Drawn by the same code path. The extension keeps to the device's limits: no storage buffers and no compute on WebGL2, and a shader that branches before a derivative needs `diagnostic(off, derivative_uniformity);` ([Web](../web/#webgpu-and-webgl), [Shaders](../web/#shaders)). The gallery's browser build has no extension, so its page shows an empty slot there. |
| `dx12`, lurq's own Direct3D 12 renderer | Its box is left empty, what is under it showing through, and the rest of the window is drawn; the log says once that this renderer draws no viewport. It has no extensions, and a wgpu texture is not its device's. An app that draws its own GPU code takes the `wgpu` renderer. |

`GpuViewport` and `WgpuExternalImageSlot` exist only in a build with the `wgpu` feature, and the viewport with
pictures (`image` or `canvas`) as well.

## Several Windows

The `wgpu` engines of an app's windows draw with one device. An engine made without a device of its own takes
the device of another window's engine on its thread, asked for among the same backends with the same optional
features and limits, while one draws; the first window's engine asks for it. So:

- **One slot can be shown in several windows.** Each viewport draws the slot's one view, stretched to its own
  box. Each window's engine has an extension of its own, made by the factory, and each is called with the
  frames of its own window: an app that shows one slot in two windows renders it in one of them (the one whose
  `prepare` finds the viewport first, or the larger), or keeps the texture in a value both extensions share,
  so that the second finds it made and draws nothing again.
- **`initialize` runs for each window of the app**, each with the same device. An extension that wants nothing
  made for a window that never shows its viewport makes its pipelines in the first `prepare` that finds the
  viewport instead.
- **lurq's DevTools window runs none of the app's extensions.** Its engine is made by the same factory, and
  lets go of the extensions it was given before its first frame.
- **A window can keep a device of its own**: `WgpuRenderEngine::new().with_own_device()`, for a window that must
  not wait on the others' work. A view of the shared device is not drawn there.

An app that gives its engines a `WgpuDevice` (`with_device`, as a browser app does) shares that device among
them, as before.

## Performance

- **No copy.** lurq binds the view the app gave and samples it in its UI pass. The texture's pixels never
  pass through the CPU and are never copied on the GPU; the cost is one textured quad.
- **No work for frames that change nothing.** Skip rendering in `prepare` when the picture's inputs did not
  change. A window that is idle draws no frames, and `prepare` does not run.
- **A size in pixels.** A texture of the rectangle's size renders no more pixels than are shown, and
  `visible_viewport` is the part of them that is seen. A scene that is expensive per pixel may render at a lower
  resolution and let lurq scale it, as the gallery does.
- **`wants_redraw` costs a frame per refresh** for the whole window, not only the viewport. Turn it off when
  the scene is still.

## The Gallery

The gallery's **GPU Viewport** page (`/engine/gpu-viewport`, in its Engine group) shows a viewport whose
texture a frame extension of the gallery's renderer fills (`crates/gallery/src/scene.rs`), with a playground
for its size, its focus and its colors, and an Events tab that says where a press lands in it:

```powershell
cargo run -p gallery -- --route /engine/gpu-viewport --renderer wgpu
```
