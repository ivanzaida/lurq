//! Which captures paint over sensitive text: those made for inspection
//! (`lurq_screenshot`, covered with the MCP tools, and DevTools node
//! screenshots), not the app's own screenshots.

use std::sync::{Arc, Mutex};

use super::{PendingScreenshot, Tree};
use crate::{
  app::{
    App,
    capture_test_engine::{GLYPH_PAINT, GlyphPainter, ScreenFrame, TestSurface},
    render_engine::{CapturedFrame, RenderCaptureTarget},
  },
  components::{Column, Text},
};

const SECRET: &str = "493117";

fn tree_with_secret(factory: impl Fn() -> Box<dyn crate::app::render_engine::RenderEngine> + 'static) -> (Tree, App) {
  let mut tree = Tree::new();
  tree.set_render_engine_factory(factory);
  tree.resize(300, 120);
  tree.set_root(
    Column::new()
      .spacing(24.0)
      .child(Text::new("Plain label"))
      .child(Text::new(SECRET).sensitive()),
  );
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  (tree, app)
}

#[test]
fn an_app_screenshot_keeps_sensitive_text() {
  let (factory, screen) = GlyphPainter::factory();
  let (mut tree, mut app) = tree_with_secret(factory);
  let captured: Arc<Mutex<Option<CapturedFrame>>> = Arc::default();
  let slot = captured.clone();
  tree.request_screenshot_capture(PendingScreenshot {
    target: RenderCaptureTarget::Bytes(Arc::new(move |frame| {
      *slot.lock().unwrap() = Some(frame.expect("captured"));
    })),
    region: None,
    redact_sensitive: false,
  });
  tree.pass(&mut app, &TestSurface);

  assert!(!tree.capture_redactions.is_empty(), "the frame painted sensitive text");
  let frame = captured.lock().unwrap().take().expect("the capture was delivered");
  assert_eq!(frame.rgba, screen.lock().unwrap().rgba);
}

#[cfg(feature = "devtools")]
mod devtools {
  use std::{
    path::PathBuf,
    time::{Duration, Instant},
  };

  use raw_window_handle::{DisplayHandle, WindowHandle};

  use super::*;
  use crate::{
    app::{
      capture_redaction::{REDACTION_FILL, RedactedArea},
      render_engine::RenderEngine,
      runtime::DevToolsScreenshotRequest,
    },
    layout::render_list::RenderList,
  };

  /// An engine without frame capture: DevTools draws node screenshots itself.
  struct NoCapture;

  impl RenderEngine for NoCapture {
    fn resize(&mut self, _: u32, _: u32) {}

    fn render(&mut self, _: &RenderList, _: WindowHandle<'_>, _: DisplayHandle<'_>) -> bool {
      true
    }
  }

  fn output_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lurq-redaction-{}-{name}.png", std::process::id()))
  }

  /// Saves a DevTools screenshot of the whole tree and reads it back.
  fn devtools_screenshot(tree: &mut Tree, app: &mut App, name: &str) -> image::RgbaImage {
    let path = output_path(name);
    // A stale file from an earlier run must not pass for this capture.
    if path.exists() {
      std::fs::remove_file(&path).expect("remove a stale capture");
    }
    *tree.devtools_state.screenshot_request.lock().unwrap() = Some(DevToolsScreenshotRequest {
      node_path: Vec::new(),
      output_path: path.clone(),
      attempts: 0,
    });
    tree.request_redraw();
    tree.pass(app, &TestSurface);
    // The software path saves on a worker thread.
    let deadline = Instant::now() + Duration::from_secs(10);
    let image = loop {
      if let Ok(image) = image::open(&path) {
        break image.to_rgba8();
      }
      assert!(Instant::now() < deadline, "no capture saved to {}", path.display());
      std::thread::sleep(Duration::from_millis(20));
    };
    std::fs::remove_file(&path).expect("remove the capture");
    image
  }

  fn area_pixels(area: RedactedArea, image: &image::RgbaImage) -> Vec<[u8; 4]> {
    let RedactedArea { x0, y0, x1, y1 } = area;
    (y0..y1.min(image.height()))
      .flat_map(|y| (x0..x1.min(image.width())).map(move |x| image.get_pixel(x, y).0))
      .collect()
  }

  #[test]
  fn a_devtools_node_screenshot_paints_over_sensitive_text() {
    let (factory, screen) = GlyphPainter::factory();
    let (mut tree, mut app) = tree_with_secret(factory);
    let image = devtools_screenshot(&mut tree, &mut app, "gpu");
    let screen: ScreenFrame = screen.lock().unwrap().clone();

    let [area] = tree.capture_redactions[..] else {
      panic!("one sensitive run: {:?}", tree.capture_redactions);
    };
    assert!(area_pixels(area, &image).iter().all(|pixel| *pixel == REDACTION_FILL));
    let shown = (area.y0..area.y1)
      .flat_map(|y| (area.x0..area.x1).map(move |x| (x, y)))
      .filter(|&(x, y)| screen.pixel(x, y) == GLYPH_PAINT)
      .count();
    assert!(shown > 0, "the window shows the text");
    // The plain label above is captured as painted.
    let plain = image
      .enumerate_pixels()
      .filter(|(x, y, pixel)| *y < area.y0 && pixel.0 == GLYPH_PAINT && screen.pixel(*x, *y) == GLYPH_PAINT)
      .count();
    assert!(plain > 0);
  }

  #[test]
  fn a_software_devtools_node_screenshot_paints_over_sensitive_text() {
    let (mut tree, mut app) = tree_with_secret(|| Box::new(NoCapture));
    let image = devtools_screenshot(&mut tree, &mut app, "software");

    let [area] = tree.capture_redactions[..] else {
      panic!("one sensitive run: {:?}", tree.capture_redactions);
    };
    assert!(area_pixels(area, &image).iter().all(|pixel| *pixel == REDACTION_FILL));
    // The plain label above is drawn: its band holds more than one colour.
    let mut above = image
      .enumerate_pixels()
      .filter(|(_, y, _)| *y < area.y0)
      .map(|(_, _, p)| p.0);
    let first = above.next().expect("pixels above the code");
    assert!(above.any(|pixel| pixel != first), "the plain label is drawn");
  }
}
