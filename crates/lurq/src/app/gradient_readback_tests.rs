// A fill that blends the colors of its four corners, through the real render engines, read back from a hidden
// window of the test's own: the corners have their colors and the middle the mix of the four. And the area of a
// color picker, which is one such fill, drawn from the picker itself at 150%: its round bottom corners show the
// panel and the area's own dark bottom, and none of the hue, which three boxes stacked under its round corners let
// through at their edge.
//
// cargo test -p lurq --features wgpu,dx12,screenshot --lib gradient_readback -- --ignored --test-threads=1

use std::{
  num::NonZeroIsize,
  sync::{Arc, Mutex},
};

use raw_window_handle::{
  DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

use super::readback_window;
use crate::{
  app::{
    App, Tree,
    render_engine::{CapturedFrame, RenderEngine},
  },
  core::Signal,
  layout::{
    quad::ClipRect,
    render_list::{GlyphAtlas, RectCmd, RenderGradient, RenderList, StrokeStyle},
  },
  node::{Gradient, color::Color},
  ui::ColorPicker,
};

const WIDTH: u32 = 120;
const HEIGHT: u32 = 80;
const CLEAR: Color = Color::new(240, 240, 240, 255);

fn pixel(frame: &CapturedFrame, x: u32, y: u32) -> [u8; 3] {
  let index = ((y * frame.width + x) * 4) as usize;
  [frame.rgba[index], frame.rgba[index + 1], frame.rgba[index + 2]]
}

fn near(actual: [u8; 3], expected: [u8; 3], by: u8) -> bool {
  actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= by)
}

/// One rect of 100 by 60 at (10, 10), filled by `gradient`.
fn scene(gradient: &Gradient) -> RenderList {
  let palette = crate::app::theme::ThemePalette::light();
  let rect = RectCmd {
    order: 0,
    x: 10.0,
    y: 10.0,
    width: 100.0,
    height: 60.0,
    color: Color::new(0, 0, 0, 255),
    radii: [0.0; 4],
    stroke: [0.0; 4],
    stroke_color: Color::new(0, 0, 0, 0),
    stroke_style: StrokeStyle::default(),
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [50.0, 30.0],
    clip: ClipRect::default(),
    gradient: RenderGradient::resolve(gradient, &palette),
    shadow: None,
  };
  RenderList {
    clear_color: CLEAR,
    rects: vec![rect],
    glyphs: Vec::new(),
    #[cfg(feature = "raster")]
    images: Vec::new(),
    layers: Vec::new(),
    backdrops: Vec::new(),
    glasses: Vec::new(),
    atlas: GlyphAtlas {
      data: Arc::from([0_u8; 4 * 4 * 4].as_slice()),
      width: 4,
      height: 4,
      version: 1,
      dirty_rects: Arc::from(Vec::new()),
      dirty_from_version: 0,
    },
  }
}

fn assert_corners_blend<E: RenderEngine>(mut new_engine: impl FnMut() -> E, backend: &str) {
  let gradient = Gradient::corners(
    Color::from_hex("#FF0000"),
    Color::from_hex("#00FF00"),
    Color::from_hex("#0000FF"),
    Color::from_hex("#FFFFFF"),
  );
  let frame = readback_window::capture(&mut new_engine(), &scene(&gradient), WIDTH, HEIGHT);
  // What a pixel's centre is: the four corners mixed across and down, in their sRGB values.
  let corners = [
    [255.0, 0.0, 0.0],
    [0.0, 255.0, 0.0],
    [0.0, 0.0, 255.0],
    [255.0, 255.0, 255.0],
  ];
  let expected = |x: u32, y: u32| -> [u8; 3] {
    let u = (x as f32 + 0.5 - 10.0) / 100.0;
    let v = (y as f32 + 0.5 - 10.0) / 60.0;
    std::array::from_fn(|c| {
      let top = corners[0][c] + (corners[1][c] - corners[0][c]) * u;
      let bottom = corners[3][c] + (corners[2][c] - corners[3][c]) * u;
      (top + (bottom - top) * v).round() as u8
    })
  };
  // A pixel in from each corner (the corner pixels are the edge's, mixed with the background): each corner's color,
  // as the blend has it there; the middle, the mean of the four; half way along the top, red and green.
  for (x, y, what) in [
    (11, 11, "the top left corner"),
    (108, 11, "the top right corner"),
    (108, 68, "the bottom right corner"),
    (11, 68, "the bottom left corner"),
    (60, 40, "the middle"),
    (60, 11, "half way along the top"),
  ] {
    let painted = pixel(&frame, x, y);
    assert!(
      near(painted, expected(x, y), 3),
      "{backend}: {what} ({x}, {y}) is {painted:?}, not {:?}",
      expected(x, y)
    );
  }
  assert!(near(expected(60, 40), [128, 128, 128], 3) && near(expected(11, 11), [251, 10, 6], 1));
  // Outside the box: the background.
  assert_eq!(pixel(&frame, 5, 5), [CLEAR.r(), CLEAR.g(), CLEAR.b()]);
}

#[cfg(feature = "wgpu")]
#[test]
#[ignore = "requires a Windows desktop and a GPU adapter; creates hidden windows"]
fn wgpu_a_four_corner_fill_has_its_corners_colors_and_their_mix_between() {
  assert_corners_blend(crate::app::wgpu_render::WgpuRenderEngine::new, "wgpu");
}

#[cfg(feature = "dx12")]
#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates hidden windows"]
fn dx12_a_four_corner_fill_has_its_corners_colors_and_their_mix_between() {
  assert_corners_blend(crate::app::dx12_render::Dx12RenderEngine::new, "dx12");
}

/// What a tree renders through: its window is the hidden one of the readback, not this.
struct NoSurface;

impl HasWindowHandle for NoSurface {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    let handle = Win32WindowHandle::new(NonZeroIsize::new(1).unwrap());
    // SAFETY: never used as a window: the engine below renders into a hidden window of its own.
    Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
  }
}

impl HasDisplayHandle for NoSurface {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    // SAFETY: as above.
    Ok(unsafe { DisplayHandle::borrow_raw(WindowsDisplayHandle::new().into()) })
  }
}

/// An engine that renders what a tree gives it into a hidden window through a real engine, and keeps the frame.
struct Readback {
  make: fn() -> Box<dyn RenderEngine>,
  size: (u32, u32),
  frame: Arc<Mutex<Option<CapturedFrame>>>,
}

impl RenderEngine for Readback {
  fn resize(&mut self, width: u32, height: u32) {
    self.size = (width, height);
  }

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    let mut engine = (self.make)();
    let frame = readback_window::capture(&mut *engine, list, self.size.0, self.size.1);
    *self.frame.lock().unwrap() = Some(frame);
    true
  }
}

/// The picker of `color` at 150%, read back: the frame, and the area's box in physical pixels.
fn picker_frame(make: fn() -> Box<dyn RenderEngine>, color: Color) -> (CapturedFrame, (u32, u32, u32, u32)) {
  let (width, height) = (390, 450);
  let frame = Arc::new(Mutex::new(None));
  let kept = frame.clone();
  let mut tree = Tree::new();
  tree.set_scale_factor(1.5);
  tree.resize(width, height);
  tree.set_render_engine_factory(move || {
    Box::new(Readback {
      make,
      size: (width, height),
      frame: kept.clone(),
    })
  });
  tree.set_root(
    crate::components::Column::new()
      .padding(8.0)
      .child(ColorPicker::new(Signal::new(color)).id("picker")),
  );
  let mut app = App::new();
  tree.request_redraw();
  tree.pass(&mut app, &NoSurface);
  let area = tree
    .get_element_by_id_mut("picker-area")
    .and_then(|element| element.bounds())
    .expect("the area is laid out");
  let physical = |v: f32| (v * 1.5).round() as u32;
  let frame = frame.lock().unwrap().take().expect("a frame");
  (
    frame,
    (
      physical(area.x),
      physical(area.y),
      physical(area.width),
      physical(area.height),
    ),
  )
}

/// How much redder than its greater other channel a pixel is.
fn redness(pixel: [u8; 3]) -> i32 {
  i32::from(pixel[0]) - i32::from(pixel[1].max(pixel[2]))
}

fn assert_no_hue_at_the_bottom_corners(make: fn() -> Box<dyn RenderEngine>, backend: &str) {
  // Red: the hue the area's right side is, which the black of its bottom is drawn over.
  let (frame, (x, y, width, height)) = picker_frame(make, Color::from_hex("#FF0000"));
  let panel = pixel(&frame, x - 4, y + height - 2);
  let radius = 9;
  let mut judged = 0;
  for (from_x, corner) in [(x, "left"), (x + width - radius, "right")] {
    for py in y + height - radius..y + height + 1 {
      for px in from_x..from_x + radius {
        let painted = pixel(&frame, px, py);
        // Nine rows up from the bottom the area's own color is a twentieth of red at most: 13.
        assert!(
          redness(painted) <= redness(panel) + 16,
          "{backend}: at the bottom {corner} corner ({px}, {py}) is {painted:?}; the panel is {panel:?}"
        );
        judged += 1;
      }
    }
  }
  assert!(judged > 100, "{backend}: {judged} pixels judged");
}

#[cfg(feature = "wgpu")]
#[test]
#[ignore = "requires a Windows desktop and a GPU adapter; creates hidden windows"]
fn wgpu_the_round_bottom_corners_of_a_pickers_area_show_no_hue() {
  assert_no_hue_at_the_bottom_corners(|| Box::new(crate::app::wgpu_render::WgpuRenderEngine::new()), "wgpu");
}

#[cfg(feature = "dx12")]
#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates hidden windows"]
fn dx12_the_round_bottom_corners_of_a_pickers_area_show_no_hue() {
  assert_no_hue_at_the_bottom_corners(|| Box::new(crate::app::dx12_render::Dx12RenderEngine::new()), "dx12");
}
