//! Group opacity is flattened: a subtree with `opacity < 1` that paints more
//! than one primitive becomes an offscreen layer (`RenderList::layers`),
//! composited once at its opacity, instead of fading each piece. The pixels
//! are checked through both render engines in
//! `src/app/opacity_layer_readback_tests.rs`.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, events::MouseButton},
  components::{Column, Rect, Stack, Text},
  layout::opacity_layer::LayerCmd,
  node::{EventHandler, color::Color},
};

use crate::support::{RenderSnapshot, pointer_click, render_pass};

const BACKGROUND: Color = Color::new(0x1c, 0x1c, 0x1c, 255);
const FILL: Color = Color::new(0xe4, 0xe4, 0xe4, 255);
const LABEL: Color = Color::new(0x17, 0x17, 0x17, 255);

/// A disabled primary button: light fill and border, dark label, faded as a
/// whole, `inset` logical pixels into a dark window.
fn disabled_button(inset: f32) -> Column {
  Column::new()
    .size(300.0, 200.0)
    .background(BACKGROUND)
    .padding(inset)
    .child(
      Stack::new()
        .size(100.0, 36.0)
        .background(FILL)
        .border_inside(1.0, FILL)
        .rounded(6.0)
        .opacity(0.4)
        .child(Text::new("Save").color(LABEL)),
    )
}

fn render(root: impl Into<lurq::node::Element>, scale: f32) -> RenderSnapshot {
  let mut tree = Tree::new();
  tree.set_scale_factor(scale);
  tree.resize((300.0 * scale) as u32, (200.0 * scale) as u32);
  tree.set_root(root);
  render_pass(&mut tree)
}

fn contains(layer: &LayerCmd, order: usize) -> bool {
  (layer.start_order..layer.end_order).contains(&order)
}

/// The layer's whole-pixel bounds cover every rect and glyph in it.
fn assert_bounds_cover_content(snapshot: &RenderSnapshot, layer: &LayerCmd) {
  let bounds = layer.bounds;
  let (x0, y0) = (bounds.x as f32, bounds.y as f32);
  let (x1, y1) = (x0 + bounds.width as f32, y0 + bounds.height as f32);
  let boxes = snapshot
    .rects
    .iter()
    .filter(|rect| contains(layer, rect.order))
    .map(|rect| (rect.x, rect.y, rect.width, rect.height))
    .chain(
      snapshot
        .glyphs
        .iter()
        .filter(|glyph| contains(layer, glyph.order))
        .map(|glyph| (glyph.x, glyph.y, glyph.width, glyph.height)),
    );
  for (x, y, width, height) in boxes {
    assert!(
      x0 <= x.floor() && y0 <= y.floor() && x1 >= (x + width).ceil() && y1 >= (y + height).ceil(),
      "({x}, {y}) {width}x{height} lies outside the layer {bounds:?}"
    );
  }
}

#[test]
fn a_faded_button_paints_into_one_layer_at_full_opacity() {
  let snapshot = render(disabled_button(20.0), 1.0);

  assert_eq!(snapshot.layers.len(), 1, "{:?}", snapshot.layers);
  let layer = snapshot.layers[0];
  assert_eq!(layer.opacity, 0.4);
  // The fill, its border and the label paint unfaded into the layer, so the
  // label keeps its contrast against the fill and the border adds no ring.
  let button: Vec<_> = snapshot
    .rects
    .iter()
    .filter(|rect| contains(&layer, rect.order))
    .collect();
  assert!(button.iter().any(|rect| rect.color == FILL), "{button:?}");
  assert!(
    button
      .iter()
      .any(|rect| rect.stroke.iter().any(|width| *width > 0.0) && rect.stroke_color == FILL),
    "{button:?}"
  );
  // The border paints over a transparent quad of its own after the label.
  assert!(button.iter().all(|rect| rect.color == FILL || rect.color.a() == 0));
  let label: Vec<_> = snapshot
    .glyphs
    .iter()
    .filter(|glyph| contains(&layer, glyph.order))
    .collect();
  assert!(!label.is_empty());
  assert!(label.iter().all(|glyph| glyph.color[3] == 1.0));
  // The background is outside the layer.
  let background = snapshot.rects.iter().find(|rect| rect.color == BACKGROUND).unwrap();
  assert!(!contains(&layer, background.order));
  assert_bounds_cover_content(&snapshot, &layer);
  assert_eq!((layer.bounds.x, layer.bounds.y), (18, 18), "2 px of anti-aliasing room");
}

#[test]
fn layer_bounds_stay_whole_pixels_at_fractional_scales() {
  for scale in [1.25, 1.5] {
    // 7 logical pixels are 8.75 and 10.5 physical ones.
    let snapshot = render(disabled_button(7.0), scale);
    assert_eq!(snapshot.layers.len(), 1, "scale {scale}");
    let layer = snapshot.layers[0];
    assert_bounds_cover_content(&snapshot, &layer);
    let button = snapshot.rects.iter().find(|rect| rect.color == FILL).unwrap();
    assert_eq!((button.x, button.width), (7.0 * scale, 100.0 * scale));
    assert_eq!(layer.bounds.x as f32, (button.x - 2.0).floor(), "scale {scale}");
  }
}

#[test]
fn nested_groups_become_nested_layers() {
  let snapshot = render(
    Column::new()
      .opacity(0.5)
      .child(Rect::new(40.0, 40.0).background("#ff0000"))
      .child(
        Column::new()
          .opacity(0.5)
          .child(Rect::new(40.0, 20.0).background("#0000ff"))
          .child(Rect::new(20.0, 20.0).background("#00ff00")),
      ),
    1.0,
  );

  let [outer, inner] = snapshot.layers[..] else {
    panic!("two layers, got {:?}", snapshot.layers);
  };
  assert_eq!((outer.opacity, inner.opacity), (0.5, 0.5));
  assert!(outer.start_order <= inner.start_order && inner.end_order <= outer.end_order);
  assert!(inner.end_order - inner.start_order == 2 && outer.end_order - outer.start_order == 3);
  // Content paints at full opacity; the layers carry the fade.
  assert!(snapshot.rects.iter().all(|rect| rect.color.a() == 255));
}

#[test]
fn a_group_around_a_single_layer_merges_into_it() {
  let snapshot = render(
    Column::new().opacity(0.5).child(
      Column::new()
        .opacity(0.5)
        .child(Rect::new(40.0, 20.0).background("#0000ff"))
        .child(Rect::new(20.0, 20.0).background("#00ff00")),
    ),
    1.0,
  );
  assert_eq!(snapshot.layers.len(), 1, "{:?}", snapshot.layers);
  assert_eq!(snapshot.layers[0].opacity, 0.25);
}

#[test]
fn a_group_painting_one_primitive_folds_its_opacity_without_a_layer() {
  let snapshot = render(
    Column::new()
      .child(Rect::new(40.0, 20.0).background("#0000ff").opacity(0.5))
      .child(
        Column::new()
          .opacity(0.5)
          .child(Rect::new(40.0, 20.0).background("#00ff00")),
      )
      .child(
        Column::new()
          .opacity(0.5)
          .child(Text::new("Hi").color(Color::new(255, 255, 255, 255))),
      ),
    1.0,
  );
  assert!(snapshot.layers.is_empty(), "{:?}", snapshot.layers);
  assert_eq!(
    snapshot.rects.iter().map(|rect| rect.color.a()).collect::<Vec<_>>(),
    vec![128, 128]
  );
  assert!(snapshot.glyphs.iter().all(|glyph| (glyph.color[3] - 0.5).abs() < 1e-6));
}

#[test]
fn opacity_one_makes_no_layer() {
  let snapshot = render(
    Column::new().opacity(1.0).child(
      Stack::new()
        .size(100.0, 36.0)
        .background(FILL)
        .border_inside(1.0, FILL)
        .rounded(6.0)
        .opacity(1.0)
        .child(Text::new("Save").color(LABEL).opacity(1.0)),
    ),
    1.0,
  );
  assert!(snapshot.layers.is_empty(), "{:?}", snapshot.layers);
  assert!(snapshot.rects.iter().all(|rect| matches!(rect.color.a(), 0 | 255)));
  assert!(snapshot.glyphs.iter().all(|glyph| glyph.color[3] == 1.0));
}

#[test]
fn a_fully_transparent_group_has_an_invisible_layer() {
  let snapshot = render(
    Column::new()
      .opacity(0.0)
      .child(Rect::new(40.0, 20.0).background("#0000ff"))
      .child(Rect::new(20.0, 20.0).background("#00ff00")),
    1.0,
  );
  assert_eq!(snapshot.layers.len(), 1);
  assert!(
    !snapshot.layers[0].is_visible(),
    "render engines skip it and its content"
  );
}

#[test]
fn a_clipped_group_is_bounded_by_its_clip() {
  let snapshot = render(
    Column::new().size(60.0, 30.0).clip().child(
      Column::new()
        .opacity(0.5)
        .child(Rect::new(200.0, 20.0).background("#0000ff"))
        .child(Rect::new(200.0, 20.0).background("#00ff00")),
    ),
    1.0,
  );
  let layer = snapshot.layers[0];
  assert_eq!(
    (layer.bounds.x, layer.bounds.y, layer.bounds.width, layer.bounds.height),
    (0, 0, 60, 30)
  );
}

#[test]
fn a_faded_element_still_takes_clicks() {
  let clicks = Arc::new(AtomicUsize::new(0));
  let counter = clicks.clone();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      Stack::new()
        .size(100.0, 36.0)
        .background(FILL)
        .border_inside(1.0, FILL)
        .opacity(0.4)
        .on_click(EventHandler::new(move |_: &lurq::app::events::MouseEvent| {
          counter.fetch_add(1, Ordering::SeqCst);
        }))
        .child(Text::new("Save").color(LABEL)),
    ),
  );
  let snapshot = render_pass(&mut tree);
  assert_eq!(snapshot.layers.len(), 1);
  pointer_click(&mut tree, 50.0, 18.0, MouseButton::Left);
  assert_eq!(clicks.load(Ordering::SeqCst), 1);
}
