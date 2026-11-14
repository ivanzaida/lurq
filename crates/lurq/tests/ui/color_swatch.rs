//! A color swatch: the board's square of 24 with its edge, a circle, the sizes, the checkerboard under a color that
//! is not opaque, the ring of the one selected, a press and the keys of one that is a button, a disabled one, and
//! what tooling reads.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{ColorSwatch, ColorSwatchShape, ColorSwatchSize},
};

use super::stage::Stage;

const BLUE: &str = "#2563EB";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn staged(swatch: impl Fn() -> ColorSwatch + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |_| Column::new().padding(8.0).child(swatch()).into());
  stage.pass();
  stage
}

/// The filled rectangles of `color`.
fn fills(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect()
}

#[test]
fn a_swatch_is_24_square_with_the_small_radius_and_an_edge_of_1_in_the_border_color() {
  let mut stage = staged(|| ColorSwatch::new(Color::from_hex(BLUE)).id("swatch"));
  assert_eq!(stage.bounds("swatch").2, 24.0);
  assert_eq!(stage.bounds("swatch").3, 24.0);
  let edge = stage.border("swatch").expect("an edge");
  let side = edge.border.and_then(|border| border.top).unwrap();
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::Border)));
  assert!(
    edge.border_radius.is_some_and(|radius| radius.top_left == 4.0),
    "the small radius"
  );
  let blue = fills(&stage, Color::from_hex(BLUE));
  assert_eq!(blue.len(), 1, "its color over all of it");
  assert_eq!((blue[0].width, blue[0].height), (24.0, 24.0));
}

#[test]
fn the_sizes_and_the_circle() {
  for (size, side) in [
    (ColorSwatchSize::Sm, 20.0),
    (ColorSwatchSize::Md, 24.0),
    (ColorSwatchSize::Lg, 32.0),
  ] {
    let mut stage = staged(move || ColorSwatch::new(Color::from_hex(BLUE)).size(size).id("swatch"));
    assert_eq!(stage.bounds("swatch").2, side, "{size:?}");
  }
  let mut stage = staged(|| {
    ColorSwatch::new(Color::from_hex(BLUE))
      .shape(ColorSwatchShape::Round)
      .id("swatch")
  });
  let edge = stage.border("swatch").unwrap();
  assert!(
    edge.border_radius.is_some_and(|radius| radius.top_left >= 12.0),
    "round"
  );
}

#[test]
fn a_color_that_is_not_opaque_stands_over_a_checkerboard() {
  let half = Color::from_hex("#2563EB80");
  let stage = staged(move || ColorSwatch::new(half).id("swatch"));
  assert_eq!(fills(&stage, half).len(), 1);
  let dark = fills(&stage, light(PaletteColor::Muted));
  assert_eq!(dark.len(), 18, "every other of 6 by 6 squares of 4");
  assert!(dark.iter().all(|square| square.width == 4.0 && square.height == 4.0));
  assert!(
    !fills(&stage, light(PaletteColor::Background)).is_empty(),
    "the light squares"
  );

  let opaque = stage_of_opaque();
  assert!(
    fills(&opaque, light(PaletteColor::Muted)).is_empty(),
    "an opaque color has no checkerboard"
  );
}

fn stage_of_opaque() -> Stage {
  staged(|| ColorSwatch::new(Color::from_hex(BLUE)))
}

#[test]
fn the_one_selected_has_a_ring_of_2_in_the_foreground() {
  let stage = staged(|| ColorSwatch::new(Color::from_hex(BLUE)).selected(true));
  let ring = stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, gradient: None } if color.a() == 0)
      && quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|side| side.width == 2.0 && side.color == light(PaletteColor::Foreground))
  });
  assert!(ring.is_some(), "a ring of 2 in the foreground");
}

#[test]
fn a_swatch_with_on_click_is_pressed_and_reached_by_tab_and_a_disabled_one_takes_nothing() {
  let pressed = Arc::new(Mutex::new(0));
  let tell = pressed.clone();
  let mut stage = staged(move || {
    let tell = tell.clone();
    ColorSwatch::new(Color::from_hex(BLUE))
      .on_click(move |_| *tell.lock().unwrap() += 1)
      .id("swatch")
  });
  stage.click("swatch");
  assert_eq!(*pressed.lock().unwrap(), 1);
  stage.key("Tab");
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the ring of the focus from the keyboard"
  );
  stage.key("Enter");
  stage.key(" ");
  assert_eq!(*pressed.lock().unwrap(), 3, "Enter and Space press it");

  let none = Arc::new(Mutex::new(0));
  let tell = none.clone();
  let mut stage = staged(move || {
    let tell = tell.clone();
    ColorSwatch::new(Color::from_hex(BLUE))
      .disabled(true)
      .on_click(move |_| *tell.lock().unwrap() += 1)
      .id("swatch")
  });
  stage.click("swatch");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(*none.lock().unwrap(), 0);
}

/// To tooling a swatch is a picture called by its label or its color; one that is pressed is a button.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_picture_called_by_its_label_or_its_color() {
  let mut stage = staged(|| ColorSwatch::new(Color::from_hex(BLUE)).id("swatch"));
  let element = stage.tree.get_element_by_id_mut("swatch").unwrap();
  assert_eq!(element.description("a11y_role").as_deref(), Some("img"));
  assert_eq!(element.description("a11y_name").as_deref(), Some("#2563EB"));
  let mut stage = staged(|| {
    ColorSwatch::new(Color::from_hex(BLUE))
      .label("Brand blue")
      .on_click(|_| {})
      .id("swatch")
  });
  let element = stage.tree.get_element_by_id_mut("swatch").unwrap();
  assert_eq!(element.description("a11y_role").as_deref(), Some("button"));
  assert_eq!(element.description("a11y_name").as_deref(), Some("Brand blue"));
}
