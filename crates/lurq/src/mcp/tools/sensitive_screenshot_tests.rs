//! `lurq_screenshot` paints over sensitive text, while the window keeps
//! showing it.

use std::sync::{Arc, Mutex};

use super::{
  tests::{TestSurface, call, output_text, state},
  *,
};
use crate::{
  app::{
    capture_redaction::REDACTION_FILL,
    capture_test_engine::{GLYPH_PAINT, GlyphPainter, ScreenFrame},
  },
  components::{Column, Text},
  layout::quad::QuadContent,
};

const SECRET: &str = "493117";
const PLAIN: &str = "Plain label";
const SCALE: f32 = 1.5;

struct Fixture {
  tree: Tree,
  app: App,
  state: McpState,
  screen: Arc<Mutex<ScreenFrame>>,
}

fn fixture() -> Fixture {
  let mut tree = Tree::new();
  let (factory, screen) = GlyphPainter::factory();
  tree.set_render_engine_factory(factory);
  tree.set_scale_factor(SCALE);
  tree.resize(450, 180);
  tree.set_root(
    Column::new()
      .spacing(24.0)
      .child(Text::new(PLAIN).id("plain"))
      .child(Text::new(SECRET).sensitive().id("code")),
  );
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  Fixture {
    tree,
    app,
    state: state(),
    screen,
  }
}

/// The physical pixels `[x0, y0, x1, y1)` of the painted text quad `text`.
fn text_box(tree: &Tree, text: &str) -> [u32; 4] {
  let quad = tree
    .painted_quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text: painted, .. } if painted == text))
    .unwrap_or_else(|| panic!("{text} is painted"));
  [
    (quad.x * SCALE).floor() as u32,
    (quad.y * SCALE).floor() as u32,
    ((quad.x + quad.width) * SCALE).ceil() as u32,
    ((quad.y + quad.height) * SCALE).ceil() as u32,
  ]
}

fn screenshot(f: &mut Fixture, args: serde_json::Value) -> image::RgbaImage {
  let (reply, mut rx) = tokio::sync::oneshot::channel();
  execute(
    &mut f.tree,
    &mut f.app,
    &f.state,
    McpRequest {
      tool: "lurq_screenshot".into(),
      args,
      reply,
    },
  );
  // The capture is taken from the next rendered frame.
  f.tree.pass(&mut f.app, &TestSurface);
  match rx.try_recv().expect("the capture answered") {
    Ok(McpToolOutput::Image { data, .. }) => image::load_from_memory(&data).expect("a PNG").to_rgba8(),
    Ok(_) => panic!("expected an image"),
    Err(error) => panic!("screenshot failed: {error}"),
  }
}

/// Checks a capture whose top-left is window pixel `origin` against the
/// screen: every glyph pixel of the sensitive text is the redaction bar,
/// every glyph pixel of the plain text is still a glyph. Returns how many
/// pixels of each it compared.
fn assert_redacted(f: &Fixture, capture: &image::RgbaImage, origin: (u32, u32)) -> (usize, usize) {
  let screen = f.screen.lock().unwrap().clone();
  let code = text_box(&f.tree, SECRET);
  let plain = text_box(&f.tree, PLAIN);
  let inside = |b: [u32; 4], x: u32, y: u32| x >= b[0] && x < b[2] && y >= b[1] && y < b[3];
  let (mut secret_pixels, mut plain_pixels) = (0, 0);
  for (cx, cy, pixel) in capture.enumerate_pixels() {
    let (x, y) = (origin.0 + cx, origin.1 + cy);
    if inside(code, x, y) {
      assert_ne!(pixel.0, GLYPH_PAINT, "a sensitive glyph pixel at {x},{y}");
    }
    if screen.pixel(x, y) != GLYPH_PAINT {
      continue;
    }
    if inside(code, x, y) {
      assert_eq!(pixel.0, REDACTION_FILL, "sensitive glyph pixel at {x},{y}");
      secret_pixels += 1;
    } else if inside(plain, x, y) {
      assert_eq!(pixel.0, GLYPH_PAINT, "plain glyph pixel at {x},{y}");
      plain_pixels += 1;
    }
  }
  (secret_pixels, plain_pixels)
}

#[test]
fn a_window_screenshot_paints_over_sensitive_text_and_keeps_the_rest() {
  let mut f = fixture();
  let capture = screenshot(&mut f, serde_json::json!({}));
  let (secret, plain) = assert_redacted(&f, &capture, (0, 0));
  assert!(
    secret > 0 && plain > 0,
    "compared {secret} sensitive and {plain} plain glyph pixels"
  );

  // The window itself still shows the text, and paint still has it.
  let screen = f.screen.lock().unwrap().clone();
  let code = text_box(&f.tree, SECRET);
  let shown = (code[1]..code[3])
    .flat_map(|y| (code[0]..code[2]).map(move |x| (x, y)))
    .filter(|&(x, y)| screen.pixel(x, y) == GLYPH_PAINT)
    .count();
  assert_eq!(shown, secret);
  assert!(
    f.tree
      .painted_quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == SECRET))
  );
}

#[test]
fn region_and_element_screenshots_paint_over_sensitive_text() {
  let mut f = fixture();
  let code = text_box(&f.tree, SECRET);
  // A region around the code, offset from the window origin.
  let origin = (code[0].saturating_sub(7), code[1].saturating_sub(5));
  let region = serde_json::json!({"region": {
    "x": origin.0,
    "y": origin.1,
    "width": code[2] - origin.0 + 9,
    "height": code[3] - origin.1 + 3,
  }});
  let capture = screenshot(&mut f, region);
  let (secret, _) = assert_redacted(&f, &capture, origin);
  assert!(secret > 0);

  let found = output_text(call(
    &mut f.tree,
    &mut f.app,
    &f.state,
    "lurq_find_by_id",
    serde_json::json!({"id": "code"}),
  ));
  // A lookup line starts with the element's ref.
  let code_ref = found.split_whitespace().next().expect("a ref").to_owned();
  assert!(code_ref.starts_with("ref_"), "{found}");
  let capture = screenshot(&mut f, serde_json::json!({"ref": code_ref}));
  let (secret, _) = assert_redacted(&f, &capture, (code[0], code[1]));
  assert!(secret > 0);
}
