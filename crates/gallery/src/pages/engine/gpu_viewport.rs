//! The page of the engine's GPU viewport: a box whose pixels are a texture the app draws itself, with its own wgpu
//! passes on lurq's device, and which lurq samples where the box is laid out, in the same frame and without a copy.
//! The demo had no page of it.
//!
//! What draws into a viewport is a frame extension of the renderer (`WgpuFrameExtension`), given to the
//! `WgpuRenderEngine` where the app makes it. The gallery's is `crate::scene`, which writes rings into the slot of
//! this page's viewport, in the colors the playground chooses, turning while it says. A browser's build has no
//! extension, and its viewport an empty slot.

use std::sync::{Arc, atomic::Ordering};

use lurq::{
  app::{component::Component, ctx::Ctx, events::MouseEvent},
  components::{Column, GpuViewport, Stack},
  core::{ElementRef, Signal},
  images::WgpuExternalImageSlot,
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
};

use super::kit::{named, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const SIZES: [(&str, usize); 3] = [("320 × 180", 0), ("480 × 270", 1), ("FILL × 240", 2)];
const COLORS: [(&str, usize); 3] = [("Ocean", 0), ("Ember", 1), ("Moss", 2)];

/// The slot the page's viewport shows: on a desktop the one the gallery's extension draws into, in a browser one
/// nothing draws into.
fn page_slot() -> WgpuExternalImageSlot {
  #[cfg(not(target_arch = "wasm32"))]
  return crate::scene::slot().clone();
  #[cfg(target_arch = "wasm32")]
  WgpuExternalImageSlot::new()
}

/// Tells the gallery's extension the colors of its rings and whether they turn.
fn tell_scene(colors: usize, turning: bool) {
  #[cfg(not(target_arch = "wasm32"))]
  {
    crate::scene::COLORS_AT.store(colors, Ordering::Relaxed);
    crate::scene::TURNING.store(turning, Ordering::Relaxed);
  }
  #[cfg(target_arch = "wasm32")]
  let _ = (colors, turning, Ordering::Relaxed);
}

/// Whether the window is drawn with DirectX 12 (`--renderer dx12`). lurq's DirectX 12 renderer cannot sample a
/// texture of wgpu's: it leaves the viewport's box empty, and the gallery's extension is not there to fill it.
fn dx12() -> bool {
  crate::option("--renderer").is_some_and(|renderer| matches!(renderer.to_ascii_lowercase().as_str(), "dx12" | "d3d12"))
}

/// A line in the middle of the viewport's box: what the empty slot leaves to be seen.
fn empty_line(content: &str) -> Column {
  Column::new()
    .size(FILL, FILL)
    .padding(16.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(quiet(content, 12.0))
}

/// The viewport of the playground, `size` big, over a line that says its slot is empty: the slot has no view, so
/// what is under the viewport shows through. `handlers` are its own when it is on the stage, and none in a window
/// of its own.
fn viewport(slot: &WgpuExternalImageSlot, size: usize, focusable: bool, handlers: Option<Handlers>) -> Element {
  let mut shown = GpuViewport::new(slot.clone()).size(FILL, FILL).focusable(focusable);
  if let Some(Handlers { anchor, record }) = handlers {
    let place = anchor.clone();
    let at = move |name: &str, event: &MouseEvent| {
      let (x, y, ..) = place.rect();
      format!("{name}({:.0}, {:.0})", event.x - x, event.y - y)
    };
    let (down, up, focus, blur) = (record.clone(), record.clone(), record.clone(), record);
    let at_up = at.clone();
    shown = shown
      .ref_element(anchor)
      .on_mouse_down(move |event: MouseEvent| down(at("on_mouse_down", &event)))
      .on_mouse_up(move |event: MouseEvent| up(at_up("on_mouse_up", &event)))
      .on_focus(move || focus("on_focus".to_owned()))
      .on_blur(move || blur("on_blur".to_owned()));
  }
  let (width, height) = match size {
    0 => (Some(320.0), 180.0),
    1 => (Some(480.0), 270.0),
    _ => (None, 240.0),
  };
  let mut frame = well().height(height).child(
    Stack::new()
      .size(FILL, FILL)
      .stack_align(StackAlignment::Center)
      // Under the viewport: what shows while nothing has drawn into the slot, as in a browser's build, and what
      // DirectX 12 leaves to be seen.
      .child(empty_line(if dx12() {
        "Drawn with DirectX 12, which samples no texture of wgpu's: the box is left empty."
      } else {
        "Nothing has drawn into the slot."
      }))
      .child(shown),
  );
  if let Some(width) = width {
    frame = frame.width(width).max_width(FILL);
  }
  frame.into()
}

/// What a viewport on the stage tells the playground: where it is, to say a press in its own points, and the
/// recorder of the Events tab.
struct Handlers {
  anchor: ElementRef,
  record: Arc<dyn Fn(String) + Send + Sync>,
}

pub struct GpuViewportPage {
  frame: Frame,
  playground: Playground,
  /// The slot the viewport shows: what an app's extension would give its texture's view to.
  slot: WgpuExternalImageSlot,
  /// The viewport on the stage, for where a press is in it.
  anchor: ElementRef,
  /// An index into [`SIZES`].
  size: Signal<usize>,
  focusable: Signal<bool>,
  /// An index into [`COLORS`]: the colors of the extension's rings.
  colors: Signal<usize>,
  /// Whether the rings turn, the window drawing a frame after another while they do.
  turning: Signal<bool>,
}

impl Drop for GpuViewportPage {
  /// The rings stop with the page: the window asks for no more frames for them.
  fn drop(&mut self) {
    tell_scene(self.colors.get_untracked(), false);
  }
}

impl GpuViewportPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (size, focusable) = (self.size.get(), self.focusable.get());
    let (colors, turning) = (self.colors.get(), self.turning.get());
    tell_scene(colors, turning);
    let slot = self.slot.clone();
    let record = self.playground.recorder();
    let handlers = Handlers {
      anchor: self.anchor.clone(),
      record: Arc::new(record),
    };
    let make = {
      let slot = slot.clone();
      move || viewport(&slot, size, focusable, None)
    };
    let mut code = vec![
      "let slot = WgpuExternalImageSlot::new();".to_owned(),
      "// The renderer's extension writes rings into a texture and gives its view to the slot:".to_owned(),
      "// WgpuRenderEngine::new().with_frame_extension(Scene { slot: slot.clone(), .. })".to_owned(),
      "GpuViewport::new(slot.clone())".to_owned(),
      match size {
        0 => "    .size(320.0, 180.0)".to_owned(),
        1 => "    .size(480.0, 270.0)".to_owned(),
        _ => "    .size(Dimension::Pct(100.0), 240.0)".to_owned(),
      },
    ];
    if focusable {
      code.push("    .focusable(true)".to_owned());
    }
    code.push("    .on_mouse_down(|event: MouseEvent| ..)".to_owned());
    let reset = {
      let (size, focusable) = (self.size.clone(), self.focusable.clone());
      let (colors, turning) = (self.colors.clone(), self.turning.clone());
      Arc::new(move || {
        size.set(1);
        focusable.set(true);
        colors.set(0);
        turning.set(false);
      })
    };
    let (set_size, set_colors) = (self.size.clone(), self.colors.clone());
    let controls = vec![
      prop(
        "size",
        "impl Into<Dimension>",
        self
          .playground
          .select(0, &SIZES, &size, move |value| set_size.set(value)),
      ),
      prop(
        "the extension's colors",
        "the app's",
        self
          .playground
          .select(1, &COLORS, &colors, move |value| set_colors.set(value)),
      ),
      divider(),
      flag("focusable", &self.focusable),
      flag("turning: wants_redraw", &self.turning),
    ];
    let facts = if dx12() {
      "DirectX 12: left empty".to_owned()
    } else {
      format!("slot {}, version {}", slot.image_id(), slot.version())
    };
    self.playground.card(
      ctx,
      Spec {
        title: "GpuViewport",
        make: Arc::new(make),
        shown: Shown::built(move |_| viewport(&slot, size, focusable, Some(handlers))),
        fill: size == 2,
        forceable: false,
        facts,
        event: "on_mouse_down",
        code,
        tree: vec![
          format!("well   {}", self.playground.measured()),
          "└─ stack".to_owned(),
          "   ├─ text   under it: nothing has drawn".to_owned(),
          "   └─ gpu viewport   the extension's texture".to_owned(),
        ],
        controls,
        changed: [size != 1, !focusable, colors != 0, turning]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

/// The steps of a frame with an extension in it, as a list: what the app does, and when.
fn steps() -> Column {
  let step = |name: &str, what: &str| named(name, text(what, 13.0, FontWeight::Normal).width(FILL));
  Column::new()
    .width(FILL)
    .spacing(12.0)
    .child(step(
      "initialize",
      "Once, when lurq has made its device: the extension makes its pipeline and its buffers with it.",
    ))
    .child(step(
      "prepare",
      "Before each frame's UI pass: it finds where the viewport is (frame.viewport(slot.image_id())), makes a \
       texture of that size, draws into it with an encoder of its own, and gives the view to the slot.",
    ))
    .child(step(
      "UI pass",
      "lurq samples the view where the viewport is laid out, under its radius, its clip and its opacity: the \
       texture is not copied.",
    ))
    .child(step(
      "wants_redraw",
      "True while the extension's picture moves: the window keeps drawing frames without anything of the UI \
       changing.",
    ))
    .child(step(
      "device_lost",
      "The card was reset: the extension lets go of what it made. A new engine is made with extensions of its \
       own, and initialized with the new device.",
    ))
}

/// What a browser's build and DirectX 12 do with a viewport, side by side.
fn platforms() -> Column {
  let line = |name: &str, what: &str| named(name, text(what, 13.0, FontWeight::Normal).width(FILL));
  Column::new()
    .width(FILL)
    .spacing(12.0)
    .child(line(
      "wgpu",
      "A desktop's default renderer: Vulkan, Metal or Direct3D 12 as wgpu picks. The viewport is drawn.",
    ))
    .child(line(
      "browser",
      "wgpu draws through WebGPU, or WebGL2 where the browser has no WebGPU; an extension draws with the device it \
       is given and keeps to its limits: no storage buffers and no compute on WebGL2. This page shows the same \
       viewport with nothing drawn into it there: the gallery's extension is its desktop renderer's.",
    ))
    .child(line(
      "dx12",
      "lurq's own DirectX 12 renderer (--renderer dx12) samples no texture of wgpu's: it leaves the viewport's box \
       empty, draws the rest of the window, and says so once in the log.",
    ))
}

impl Component for GpuViewportPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      slot: page_slot(),
      anchor: ctx.element_ref(),
      size: ctx.signal(1),
      focusable: ctx.signal(true),
      colors: ctx.signal(0),
      turning: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "GpuViewport::new",
        "WgpuExternalImageSlot",
        "",
        "A box that shows the view given to the slot. Its size, pointer handlers, focus and drag are an element's \
         like any other's.",
      ),
      (
        "WgpuExternalImageSlot::new",
        "",
        "",
        "What the app's renderer and the viewport share: cloned into both.",
      ),
      (
        "size",
        "f32, f32",
        "300, 150",
        "A viewport is laid out as any box: size, width, height, percentages, flex. Given none it is a canvas's \
         size.",
      ),
      (
        "image_id",
        "",
        "",
        "The slot's id in a frame: WgpuFrameInfo::viewport(id) is where its viewport is, in pixels of the surface, \
         and visible_viewport(id) the part of it that is shown.",
      ),
      (
        "set_texture_view",
        "wgpu::TextureView, u32, u32",
        "",
        "Gives the slot a view of a 2D texture made with lurq's device, in an sRGB or a linear format, which the \
         slot holds. Returns the slot's version, or a WgpuTextureError for a texture lurq cannot sample.",
      ),
      (
        "set_texture_view_of",
        "&SharedWgpuContext, TextureView, u32, u32",
        "",
        "The same from another thread or a callback, with the context it was made with: a view of a device that \
         was lost is not drawn.",
      ),
      (
        "clear",
        "",
        "",
        "Takes the view out of the slot: the viewport draws nothing.",
      ),
      (
        "with_frame_extension",
        "impl WgpuFrameExtension",
        "",
        "On WgpuRenderEngine, where the app makes its renderer: the extension that draws into the slots.",
      ),
      (
        "SharedWgpuContext",
        "",
        "",
        "lurq's instance, adapter, device, queue and surface format, given to an extension: it makes no device of \
         its own.",
      ),
      (
        "with_own_device",
        "",
        "",
        "On WgpuRenderEngine: a device of this window's own, where the app's windows share one by default.",
      ),
      (
        "WgpuFrameInfo::scale_factor",
        "f32",
        "",
        "The window's pixels to a logical pixel, in the frame an extension prepares.",
      ),
      (
        "WgpuFrameExtension::error",
        "&WgpuAppError",
        "",
        "An invalid wgpu call of the app's, which lurq logs and tells the extension of: it ends nothing.",
      ),
      (
        "AppEvent::GraphicsDeviceLost",
        "",
        "",
        "Told to each window's on_app_event once its device was lost and lurq draws it with another.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "GPU Viewport"],
        title: "GPU Viewport",
        description: "A box whose pixels are a texture the app draws itself, with its own wgpu passes on lurq's \
                      device: a 3D view, a map, frames a decoder writes on the card. lurq samples the texture where \
                      the box is laid out, in the same frame, without a copy.",
        import: Some("use lurq::{components::GpuViewport, images::WgpuExternalImageSlot, wgpu};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A viewport in a well. Its pixels are made by the gallery's renderer's extension, not by lurq: rings in a \
           texture of the viewport's size, made on lurq's device. Choose their colors, and let them turn: the \
           extension then asks for a frame after another. The viewport is laid out as any box is, and takes the \
           pointer and, focusable, the focus: press it, and the Events tab says where, in its own points.",
          playground,
        ),
        Section::new(
          "The app's own pass",
          "An extension of the renderer draws into the viewport. It is given lurq's device when lurq has made it, and \
           is called before each frame's UI pass, where it renders into a texture and gives the slot its view.",
          self.frame.demo(
            0,
            steps(),
            "WgpuRenderEngine::new().with_frame_extension(scene)",
            &[
              "struct Scene {",
              "    slot: WgpuExternalImageSlot,",
              "    texture: Option<wgpu::Texture>,",
              "}",
              "",
              "impl WgpuFrameExtension for Scene {",
              "    fn initialize(&mut self, gpu: &SharedWgpuContext) {",
              "        // make the pipeline with gpu.device",
              "    }",
              "",
              "    fn prepare(&mut self, gpu: &SharedWgpuContext, frame: WgpuFrameInfo<'_>) {",
              "        let Some(rect) = frame.viewport(self.slot.image_id()) else { return };",
              "        let (width, height) = (rect.width as u32, rect.height as u32);",
              "        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {",
              "            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },",
              "            format: wgpu::TextureFormat::Rgba8UnormSrgb,",
              "            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,",
              "            // ..",
              "        });",
              "        let view = texture.create_view(&Default::default());",
              "        let mut encoder = gpu.device.create_command_encoder(&Default::default());",
              "        {",
              "            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {",
              "                color_attachments: &[Some(wgpu::RenderPassColorAttachment {",
              "                    view: &view,",
              "                    ops: wgpu::Operations {",
              "                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),",
              "                        store: wgpu::StoreOp::Store,",
              "                    },",
              "                    // ..",
              "                })],",
              "                // ..",
              "            });",
              "            // draw the scene",
              "        }",
              "        gpu.queue.submit([encoder.finish()]);",
              "        // A texture lurq cannot sample is refused, with a line in the log.",
              "        self.slot.set_texture_view(view, width, height).expect(\"a texture lurq samples\");",
              "        self.texture = Some(texture);",
              "    }",
              "",
              "    fn wants_redraw(&self) -> bool {",
              "        true",
              "    }",
              "",
              "    fn error(&mut self, error: &WgpuAppError) {",
              "        // an invalid call of this extension's, which ended nothing",
              "    }",
              "",
              "    fn device_lost(&mut self) {",
              "        self.texture = None;",
              "    }",
              "}",
              "",
              "let slot = WgpuExternalImageSlot::new();",
              "let for_engine = slot.clone();",
              "// An engine made again after a device is lost gets a scene of its own.",
              "tree.set_render_engine_factory(move || {",
              "    let scene = Scene { slot: for_engine.clone(), texture: None };",
              "    Box::new(WgpuRenderEngine::new().with_frame_extension(scene))",
              "});",
              "// and in the UI:",
              "GpuViewport::new(slot).size(480.0, 270.0)",
            ],
          ),
        ),
        Section::new(
          "Where it draws",
          "The viewport is drawn by lurq's wgpu renderer, on a desktop and in a browser. lurq's DirectX 12 renderer \
           leaves its box empty.",
          self.frame.demo(
            1,
            platforms(),
            "--renderer wgpu",
            &[
              "// The gallery: cargo run -p gallery -- --route /engine/gpu-viewport --renderer wgpu",
              "WgpuRenderEngine::new().with_frame_extension(scene)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a viewport is made of, the slot it shares with the app's renderer, and what the renderer is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
