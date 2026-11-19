//! A `GpuViewport` drawn for real, into hidden windows of the test's own: the colors each format of texture shows,
//! the errors of an app's own wgpu calls, the device the engines of an app's windows share, the window of lurq's
//! own tooling, and what a frame tells an extension. Each test keeps its device until the views of it are gone.

use std::sync::{Arc, Mutex};

use super::{SharedWgpuContext, WgpuAppError, WgpuFrameExtension, WgpuFrameInfo, WgpuRenderEngine, app_errors};
use crate::{
  app::{
    App, Tree,
    capture_test_engine::TestSurface,
    readback_window::Frames,
    render_engine::{CapturedFrame, RenderEngine},
  },
  components::{Column, GpuViewport, Row, ScrollVertical},
  images::WgpuExternalImageSlot,
  layout::render_list::RenderList,
};

/// Held by each test here while it runs. An engine that starts says which card draws the frames of a card, and
/// whether it draws one NV12 texture, for the whole program (`card_frames::announce`); the tests of such frames
/// run one at a time for that, and a test of theirs under Vulkan that was told "NV12" by an engine started here
/// gave Vulkan a texture an RTX loses its device for.
/// Without `raster` no engine says anything of a card, and nothing is held.
fn alone() -> Option<parking_lot::MutexGuard<'static, ()>> {
  #[cfg(feature = "raster")]
  {
    Some(crate::app::card_read::ON_THE_CARD.lock())
  }
  #[cfg(not(feature = "raster"))]
  {
    None
  }
}

fn pixel(frame: &CapturedFrame, x: u32, y: u32) -> [u8; 3] {
  let index = ((y * frame.width + x) * 4) as usize;
  [frame.rgba[index], frame.rgba[index + 1], frame.rgba[index + 2]]
}

fn near(actual: [u8; 3], expected: [u8; 3], by: u8) -> bool {
  actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= by)
}

/// An engine a tree draws through into a hidden window of its own, keeping each frame it draws.
struct Shown {
  engine: Box<dyn RenderEngine>,
  frames: Option<Frames>,
  size: (u32, u32),
  frame: Arc<Mutex<Option<CapturedFrame>>>,
}

impl RenderEngine for Shown {
  fn resize(&mut self, width: u32, height: u32) {
    self.size = (width, height);
  }

  fn set_scale_factor(&mut self, scale_factor: f32) {
    self.engine.set_scale_factor(scale_factor);
  }

  fn draws_tooling(&mut self) {
    self.engine.draws_tooling();
  }

  fn render(
    &mut self,
    list: &RenderList,
    _window: raw_window_handle::WindowHandle<'_>,
    _display: raw_window_handle::DisplayHandle<'_>,
  ) -> bool {
    let (width, height) = self.size;
    let frames = self
      .frames
      .get_or_insert_with(|| Frames::new(&mut *self.engine, width, height));
    *self.frame.lock().unwrap() = Some(frames.capture(&mut *self.engine, list));
    true
  }
}

/// A tree of `width` by `height` at `scale` whose window is drawn by the engine `make` makes, and the frames it draws.
fn shown(
  width: u32,
  height: u32,
  scale: f32,
  make: impl Fn() -> Box<dyn RenderEngine> + 'static,
) -> (Tree, App, Arc<Mutex<Option<CapturedFrame>>>) {
  let frame = Arc::new(Mutex::new(None));
  let mut tree = Tree::new();
  tree.set_scale_factor(scale);
  tree.resize(width, height);
  tree.set_render_engine_factory({
    let frame = frame.clone();
    move || {
      Box::new(Shown {
        engine: make(),
        frames: None,
        size: (width, height),
        frame: frame.clone(),
      })
    }
  });
  (tree, App::new(), frame)
}

/// Draws a frame of `tree` and gives it.
fn draw(tree: &mut Tree, app: &mut App, frame: &Arc<Mutex<Option<CapturedFrame>>>) -> CapturedFrame {
  tree.request_redraw();
  assert!(tree.pass(app, &TestSurface).rendered, "the frame is drawn");
  frame.lock().unwrap().take().expect("a frame")
}

/// A texture `width` by `height` of `format`, filled with `texel`, to be sampled.
fn texture(gpu: &SharedWgpuContext, format: wgpu::TextureFormat, texel: &[u8]) -> wgpu::Texture {
  let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
    label: Some("test viewport texture"),
    size: wgpu::Extent3d {
      width: 4,
      height: 4,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
  });
  let pixels = texel.repeat(16);
  gpu.queue.write_texture(
    texture.as_image_copy(),
    &pixels,
    wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(texel.len() as u32 * 4),
      rows_per_image: Some(4),
    },
    wgpu::Extent3d {
      width: 4,
      height: 4,
      depth_or_array_layers: 1,
    },
  );
  texture
}

/// What an extension of a test was told, and the context it was given.
#[derive(Default)]
struct Heard {
  initialized: usize,
  errors: Vec<WgpuAppError>,
  scales: Vec<f32>,
  viewports: Vec<(Option<super::WgpuViewportRect>, Option<super::WgpuViewportRect>)>,
  gpu: Option<SharedWgpuContext>,
}

/// An extension that gives each slot a texture of one color, of its format, once, and writes down what it hears.
/// `fail` is a call it makes in each `prepare` with the device.
struct Filler {
  slots: Vec<(WgpuExternalImageSlot, wgpu::TextureFormat, Vec<u8>)>,
  given: bool,
  heard: Arc<Mutex<Heard>>,
  fail: Option<fn(&SharedWgpuContext)>,
}

impl Filler {
  fn new(slots: Vec<(WgpuExternalImageSlot, wgpu::TextureFormat, Vec<u8>)>, heard: &Arc<Mutex<Heard>>) -> Self {
    Self {
      slots,
      given: false,
      heard: heard.clone(),
      fail: None,
    }
  }
}

impl WgpuFrameExtension for Filler {
  fn initialize(&mut self, gpu: &SharedWgpuContext) {
    let mut heard = self.heard.lock().unwrap();
    heard.initialized += 1;
    heard.gpu = Some(gpu.clone());
  }

  fn prepare(&mut self, gpu: &SharedWgpuContext, frame: WgpuFrameInfo<'_>) {
    {
      let mut heard = self.heard.lock().unwrap();
      heard.scales.push(frame.scale_factor);
      if let Some((slot, ..)) = self.slots.first() {
        heard
          .viewports
          .push((frame.viewport(slot.image_id()), frame.visible_viewport(slot.image_id())));
      }
    }
    if !self.given {
      self.given = true;
      for (slot, format, texel) in &self.slots {
        let texture = texture(gpu, *format, texel);
        slot
          .set_texture_view(texture.create_view(&Default::default()), 4, 4)
          .expect("a texture lurq samples");
      }
    }
    if let Some(fail) = self.fail {
      fail(gpu);
    }
  }

  fn error(&mut self, error: &WgpuAppError) {
    self.heard.lock().unwrap().errors.push(error.clone());
  }
}

/// A call wgpu finds invalid: a texture with no pixels.
fn invalid_call(gpu: &SharedWgpuContext) {
  let _ = gpu.device.create_texture(&wgpu::TextureDescriptor {
    label: Some("a texture of no pixels"),
    size: wgpu::Extent3d {
      width: 0,
      height: 4,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8Unorm,
    usage: wgpu::TextureUsages::TEXTURE_BINDING,
    view_formats: &[],
  });
}

/// A half and a whole as 16-bit floats.
const HALF: [u8; 2] = 0x3800_u16.to_le_bytes();
const WHOLE: [u8; 2] = 0x3c00_u16.to_le_bytes();

#[test]
fn an_srgb_and_a_linear_texture_show_the_colors_they_hold() {
  let _alone = alone();
  // Middle grey three ways: encoded as a picture is (188), and as linear light, a half, in eight bits and in
  // sixteen-bit floats. Each is drawn as 188 in the window.
  let slots = [
    (wgpu::TextureFormat::Rgba8UnormSrgb, vec![188, 188, 188, 255]),
    (wgpu::TextureFormat::Rgba8Unorm, vec![128, 128, 128, 255]),
    (wgpu::TextureFormat::Bgra8Unorm, vec![128, 128, 128, 255]),
    (wgpu::TextureFormat::Rgba16Float, [HALF, HALF, HALF, WHOLE].concat()),
  ]
  .map(|(format, texel)| (WgpuExternalImageSlot::new(), format, texel));
  let heard = Arc::new(Mutex::new(Heard::default()));
  let extension = slots.to_vec();
  let (mut tree, mut app, frame) = shown(200, 60, 1.0, {
    let heard = heard.clone();
    move || Box::new(WgpuRenderEngine::new().with_frame_extension(Filler::new(extension.clone(), &heard)))
  });
  let mut row = Row::new();
  for (slot, ..) in &slots {
    row = row.child(GpuViewport::new(slot.clone()).size(40.0, 40.0));
  }
  tree.set_root(row);
  let drawn = draw(&mut tree, &mut app, &frame);
  for (index, (_, format, _)) in slots.iter().enumerate() {
    let shown = pixel(&drawn, index as u32 * 40 + 20, 20);
    assert!(near(shown, [188, 188, 188], 3), "{format:?} is drawn as {shown:?}");
  }
  drop(tree);
}

#[test]
fn a_frame_tells_the_scale_and_the_part_of_the_viewport_that_is_shown() {
  let _alone = alone();
  let slot = WgpuExternalImageSlot::new();
  let heard = Arc::new(Mutex::new(Heard::default()));
  let (mut tree, mut app, frame) = shown(200, 160, 1.5, {
    let (slot, heard) = (slot.clone(), heard.clone());
    move || {
      let filler = Filler::new(
        vec![(slot.clone(), wgpu::TextureFormat::Rgba8UnormSrgb, vec![255, 0, 0, 255])],
        &heard,
      );
      Box::new(WgpuRenderEngine::new().with_frame_extension(filler))
    }
  });
  // A viewport of 100 by 80 in a scroll 60 high: the bottom 20 of it is cut.
  tree.set_root(
    Column::new()
      .padding(10.0)
      .child(ScrollVertical::new(GpuViewport::new(slot.clone()).size(100.0, 80.0)).height(60.0)),
  );
  draw(&mut tree, &mut app, &frame);
  let heard = heard.lock().unwrap();
  assert_eq!(heard.scales, [1.5]);
  let (whole, visible) = heard.viewports[0];
  let whole = whole.expect("the viewport is in the frame");
  let visible = visible.expect("a part of it is shown");
  assert_eq!(
    (whole.x, whole.y, whole.width, whole.height),
    (15.0, 15.0, 150.0, 120.0)
  );
  assert_eq!((visible.x, visible.y, visible.width), (15.0, 15.0, 150.0));
  assert!((visible.height - 90.0).abs() < 0.01, "cut by the scroll: {visible:?}");
}

#[test]
fn a_view_lurq_cannot_sample_is_refused_and_the_slot_keeps_the_one_it_had() {
  let _alone = alone();
  use crate::images::WgpuTextureError;

  let heard = Arc::new(Mutex::new(Heard::default()));
  let slot = WgpuExternalImageSlot::new();
  let (mut tree, mut app, frame) = shown(80, 60, 1.0, {
    let (slot, heard) = (slot.clone(), heard.clone());
    move || {
      let filler = Filler::new(
        vec![(slot.clone(), wgpu::TextureFormat::Rgba8UnormSrgb, vec![0, 0, 255, 255])],
        &heard,
      );
      Box::new(WgpuRenderEngine::new().with_frame_extension(filler))
    }
  });
  tree.set_root(GpuViewport::new(slot.clone()).size(40.0, 40.0));
  draw(&mut tree, &mut app, &frame);
  let gpu = heard.lock().unwrap().gpu.clone().expect("the context");
  let version = slot.version();

  let made = |format, usage, dimension, samples, depth| {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: wgpu::Extent3d {
        width: 4,
        height: 4,
        depth_or_array_layers: depth,
      },
      mip_level_count: 1,
      sample_count: samples,
      dimension,
      format,
      usage,
      view_formats: &[],
    })
  };
  let sampled = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
  let d2 = wgpu::TextureDimension::D2;
  let refused = [
    (
      made(wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureUsages::COPY_DST, d2, 1, 1),
      WgpuTextureError::NotSampled,
    ),
    (
      made(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::TEXTURE_BINDING,
        wgpu::TextureDimension::D3,
        1,
        2,
      ),
      WgpuTextureError::NotTwoDimensional(wgpu::TextureDimension::D3),
    ),
    (
      made(wgpu::TextureFormat::Rgba8Unorm, sampled, d2, 4, 1),
      WgpuTextureError::Multisampled(4),
    ),
    (
      made(
        wgpu::TextureFormat::Rgba8Uint,
        wgpu::TextureUsages::TEXTURE_BINDING,
        d2,
        1,
        1,
      ),
      WgpuTextureError::Format(wgpu::TextureFormat::Rgba8Uint),
    ),
    (
      made(
        wgpu::TextureFormat::R32Float,
        wgpu::TextureUsages::TEXTURE_BINDING,
        d2,
        1,
        1,
      ),
      WgpuTextureError::Format(wgpu::TextureFormat::R32Float),
    ),
    (
      made(wgpu::TextureFormat::Depth32Float, sampled, d2, 1, 1),
      WgpuTextureError::Format(wgpu::TextureFormat::Depth32Float),
    ),
  ];
  for (texture, error) in &refused {
    assert_eq!(
      slot.set_texture_view(texture.create_view(&Default::default()), 4, 4),
      Err(*error)
    );
  }
  assert_eq!(slot.version(), version, "the slot keeps the view it had");
  assert!(
    near(pixel(&draw(&mut tree, &mut app, &frame), 20, 20), [0, 0, 255], 2),
    "and draws it"
  );

  // A format lurq reads in each family is taken.
  for format in [
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Bgra8UnormSrgb,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rgb10a2Unorm,
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::R8Unorm,
  ] {
    let texture = made(format, wgpu::TextureUsages::TEXTURE_BINDING, d2, 1, 1);
    assert!(
      slot
        .set_texture_view(texture.create_view(&Default::default()), 4, 4)
        .is_ok(),
      "{format:?} is taken"
    );
  }
  slot.clear();
  drop(refused);
  drop(tree);
}

#[test]
fn an_invalid_call_of_an_extension_is_told_to_it_and_ends_nothing() {
  let _alone = alone();
  let slot = WgpuExternalImageSlot::new();
  let heard = Arc::new(Mutex::new(Heard::default()));
  let (mut tree, mut app, frame) = shown(80, 60, 1.0, {
    let (slot, heard) = (slot.clone(), heard.clone());
    move || {
      let mut filler = Filler::new(
        vec![(slot.clone(), wgpu::TextureFormat::Rgba8UnormSrgb, vec![0, 255, 0, 255])],
        &heard,
      );
      filler.fail = Some(invalid_call);
      Box::new(WgpuRenderEngine::new().with_frame_extension(filler))
    }
  });
  tree.set_root(GpuViewport::new(slot.clone()).size(40.0, 40.0));
  let drawn = draw(&mut tree, &mut app, &frame);
  assert!(near(pixel(&drawn, 20, 20), [0, 255, 0], 2), "the frame is drawn whole");
  {
    let heard = heard.lock().unwrap();
    assert_eq!(heard.errors.len(), 1, "told once: {:?}", heard.errors);
    assert_eq!(heard.errors[0].kind, wgpu::ErrorFilter::Validation);
  }
  draw(&mut tree, &mut app, &frame);
  assert_eq!(
    heard.lock().unwrap().errors.len(),
    2,
    "and again for the next frame's call"
  );
  drop(tree);
}

#[test]
fn an_invalid_call_of_the_app_on_a_thread_of_its_own_is_told_to_the_extensions_and_ends_nothing() {
  let _alone = alone();
  let slot = WgpuExternalImageSlot::new();
  let heard = Arc::new(Mutex::new(Heard::default()));
  let (mut tree, mut app, frame) = shown(80, 60, 1.0, {
    let (slot, heard) = (slot.clone(), heard.clone());
    move || {
      let filler = Filler::new(
        vec![(slot.clone(), wgpu::TextureFormat::Rgba8UnormSrgb, vec![0, 255, 0, 255])],
        &heard,
      );
      Box::new(WgpuRenderEngine::new().with_frame_extension(filler))
    }
  });
  tree.set_root(GpuViewport::new(slot.clone()).size(40.0, 40.0));
  draw(&mut tree, &mut app, &frame);
  assert!(heard.lock().unwrap().errors.is_empty());
  let gpu = heard.lock().unwrap().gpu.clone().expect("the context");
  // Out of any frame and of any scope: it comes to the device's handler, which ends nothing.
  std::thread::spawn(move || invalid_call(&gpu))
    .join()
    .expect("the app's thread ends");
  // And out of any frame on the thread that draws, too.
  invalid_call(heard.lock().unwrap().gpu.as_ref().unwrap());
  assert!(heard.lock().unwrap().errors.is_empty(), "told before the next frame");
  draw(&mut tree, &mut app, &frame);
  let errors = heard.lock().unwrap().errors.clone();
  assert_eq!(errors.len(), 2, "{errors:?}");
  assert!(errors.iter().all(|error| error.kind == wgpu::ErrorFilter::Validation));
  draw(&mut tree, &mut app, &frame);
  assert_eq!(heard.lock().unwrap().errors.len(), 2, "each is told once");
  drop(tree);
}

#[test]
#[should_panic(expected = "wgpu error")]
fn an_error_of_lurqs_own_work_ends_the_program_as_before() {
  let _alone = alone();
  let heard = Arc::new(Mutex::new(Heard::default()));
  let slot = WgpuExternalImageSlot::new();
  let (mut tree, mut app, frame) = shown(80, 60, 1.0, {
    let (slot, heard) = (slot.clone(), heard.clone());
    move || {
      let filler = Filler::new(
        vec![(slot.clone(), wgpu::TextureFormat::Rgba8UnormSrgb, vec![0, 255, 0, 255])],
        &heard,
      );
      Box::new(WgpuRenderEngine::new().with_frame_extension(filler))
    }
  });
  tree.set_root(GpuViewport::new(slot.clone()).size(40.0, 40.0));
  draw(&mut tree, &mut app, &frame);
  let gpu = heard.lock().unwrap().gpu.clone().expect("the context");
  // As lurq's renderer would make it, inside its frame.
  let _own = app_errors::OwnWork::begin();
  invalid_call(&gpu);
}

/// A window of `slot`, 40 by 40 at the top left, drawn by the engine `make` makes, and its frames.
fn window_of(
  slot: &WgpuExternalImageSlot,
  make: impl Fn() -> Box<dyn RenderEngine> + 'static,
) -> (Tree, App, Arc<Mutex<Option<CapturedFrame>>>) {
  let (mut tree, app, frame) = shown(60, 60, 1.0, make);
  tree.set_root(GpuViewport::new(slot.clone()).size(40.0, 40.0));
  (tree, app, frame)
}

/// An engine with an extension that hears what it is told and fills `slots`.
fn engine_with(
  slots: Vec<(WgpuExternalImageSlot, wgpu::TextureFormat, Vec<u8>)>,
  heard: &Arc<Mutex<Heard>>,
) -> WgpuRenderEngine {
  WgpuRenderEngine::new().with_frame_extension(Filler::new(slots, heard))
}

#[test]
fn the_engines_of_an_apps_windows_share_a_device_and_one_slot_shows_in_both() {
  let _alone = alone();
  let slot = WgpuExternalImageSlot::new();
  let magenta = || {
    vec![(
      slot.clone(),
      wgpu::TextureFormat::Rgba8UnormSrgb,
      vec![255, 0, 255, 255],
    )]
  };
  // The first window's renderer fills the slot; the second's fills nothing, and its window shows the same slot.
  let (first_heard, second_heard, own_heard) = Default::default();
  let (mut first, mut first_app, first_frame) = window_of(&slot, {
    let (slots, heard) = (magenta(), Arc::clone(&first_heard));
    move || Box::new(engine_with(slots.clone(), &heard))
  });
  let (mut second, mut second_app, second_frame) = window_of(&slot, {
    let heard = Arc::clone(&second_heard);
    move || Box::new(engine_with(Vec::new(), &heard))
  });
  let drawn = draw(&mut first, &mut first_app, &first_frame);
  assert!(near(pixel(&drawn, 20, 20), [255, 0, 255], 2));
  let drawn = draw(&mut second, &mut second_app, &second_frame);
  let device = |heard: &Arc<Mutex<Heard>>| heard.lock().unwrap().gpu.clone().expect("initialized");
  let (one, two) = (device(&first_heard), device(&second_heard));
  // wgpu's devices of two instances are told apart by lurq's numbers, not by `==`.
  assert_eq!(one.number, two.number, "one device");
  assert!(
    near(pixel(&drawn, 20, 20), [255, 0, 255], 2),
    "the second window draws the view the first's renderer gave"
  );

  // One that keeps a device of its own draws nothing of a view of another device.
  let (mut own, mut own_app, own_frame) = window_of(&slot, {
    let heard = Arc::clone(&own_heard);
    move || Box::new(engine_with(Vec::new(), &heard).with_own_device())
  });
  let drawn = draw(&mut own, &mut own_app, &own_frame);
  assert_ne!(device(&own_heard).number, one.number);
  assert!(!near(pixel(&drawn, 20, 20), [255, 0, 255], 40));
  drop(own);

  // Once no engine draws with it, the next one asks for a device again.
  slot.clear();
  drop((one, two));
  *first_heard.lock().unwrap() = Heard::default();
  *second_heard.lock().unwrap() = Heard::default();
  drop((first, second));
  let next_heard = Arc::new(Mutex::new(Heard::default()));
  let (mut next, mut next_app, next_frame) = window_of(&slot, {
    let heard = Arc::clone(&next_heard);
    move || Box::new(engine_with(Vec::new(), &heard))
  });
  let number = device(&own_heard).number;
  draw(&mut next, &mut next_app, &next_frame);
  let next_number = device(&next_heard).number;
  *own_heard.lock().unwrap() = Heard::default();
  *next_heard.lock().unwrap() = Heard::default();
  assert!(next_number > number, "a device made after the others");
}

#[test]
fn a_window_of_lurqs_own_tooling_runs_none_of_the_apps_extensions() {
  let _alone = alone();
  let slot = WgpuExternalImageSlot::new();
  let heard = Arc::new(Mutex::new(Heard::default()));
  let (mut tree, mut app, frame) = window_of(&slot, {
    let (slot, heard) = (slot.clone(), heard.clone());
    move || {
      let mut engine = engine_with(
        vec![(
          slot.clone(),
          wgpu::TextureFormat::Rgba8UnormSrgb,
          vec![255, 0, 255, 255],
        )],
        &heard,
      );
      engine.draws_tooling();
      Box::new(engine)
    }
  });
  draw(&mut tree, &mut app, &frame);
  draw(&mut tree, &mut app, &frame);
  let heard = heard.lock().unwrap();
  assert_eq!(heard.initialized, 0);
  assert!(heard.scales.is_empty(), "nor prepared");
}

#[cfg(feature = "dx12")]
#[test]
fn the_dx12_renderer_leaves_a_viewports_box_empty_and_draws_the_rest() {
  let _alone = alone();
  let slot = WgpuExternalImageSlot::new();
  let (mut tree, mut app, frame) = shown(120, 60, 1.0, || {
    Box::new(crate::app::dx12_render::Dx12RenderEngine::new())
  });
  tree.set_root(
    Row::new()
      .background(crate::node::color::Color::from_hex("#0000ff"))
      .child(GpuViewport::new(slot.clone()).size(40.0, 40.0))
      .child(
        Column::new()
          .size(40.0, 40.0)
          .background(crate::node::color::Color::from_hex("#00ff00")),
      ),
  );
  // It failed the whole frame, which was never drawn.
  let drawn = draw(&mut tree, &mut app, &frame);
  assert!(
    near(pixel(&drawn, 20, 20), [0, 0, 255], 2),
    "what is under the viewport shows"
  );
  assert!(near(pixel(&drawn, 60, 20), [0, 255, 0], 2), "the rest is drawn");
}
