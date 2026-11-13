//! A slider: its track, its range and its thumb at the value, what a press,
//! a drag and the arrow keys set, its states, and a theme's other look; the
//! vertical one, and the range slider with its two thumbs.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Case, Interaction, Opacity, RangeSlider, Slider, SliderOrientation, SliderPart, SliderRenderer, SliderState,
    SliderView,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

/// The filled rectangles the slider with `id` paints in its box, of a height.
fn parts(stage: &mut Stage, id: &str, height: f32) -> Vec<Quad> {
  let (x, y, width, slider_height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && quad.height == height
        && quad.x >= x - 0.51
        && quad.y >= y - 0.51
        && quad.x + quad.width <= x + width + 0.51
        && quad.y + quad.height <= y + slider_height + 0.51
    })
    .collect()
}

/// The thumb of the slider with `id`: the round fill of 16.
fn thumb(stage: &mut Stage, id: &str) -> Quad {
  parts(stage, id, 16.0)
    .into_iter()
    .find(|quad| quad.width == 16.0)
    .unwrap_or_else(|| panic!("#{id} should have a thumb"))
}

#[test]
fn a_slider_is_a_track_filled_up_to_its_thumb() {
  let volume = Signal::new(40);
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(Slider::new(volume.clone()).label("Volume").id("slider"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("slider");
  assert_eq!((width, height), (240.0, 16.0));

  // The track runs across the slider, 4 high in the middle of it, in the secondary color; the range is the
  // primary color from its start to the thumb.
  let rails = parts(&mut stage, "slider", 4.0);
  let track = rails
    .iter()
    .find(|quad| quad.width == 240.0)
    .expect("a track across the slider");
  assert_eq!((track.x, track.y), (x, y + 6.0));
  assert_eq!(color_of(track), light(PaletteColor::Secondary));
  assert!(
    track.border_radius.is_some_and(|radius| radius.top_left >= 2.0),
    "round"
  );
  let range = rails
    .iter()
    .find(|quad| color_of(quad) == light(PaletteColor::Primary))
    .expect("a range");
  assert_eq!(range.x, x);

  // At 40 of 0 to 100 the thumb is 40 hundredths of the way the thumb can go: 224 of 240.
  let thumb = thumb(&mut stage, "slider");
  assert_eq!(thumb.x - x, (0.4_f32 * 224.0).round());
  assert_eq!(thumb.y, y);
  assert_eq!(color_of(&thumb), light(PaletteColor::Background));
  assert!(
    thumb.border_radius.is_some_and(|radius| radius.top_left >= 8.0),
    "round"
  );
  let edge = thumb.border.and_then(|border| border.top).expect("an edge");
  assert_eq!((edge.width, edge.color), (1.5, light(PaletteColor::Primary)));
  // The range ends under the thumb, at its middle.
  assert!((range.x + range.width - (thumb.x + 8.0)).abs() <= 0.5);
}

#[test]
fn the_arrow_keys_move_the_value_by_its_step_within_its_range() {
  let volume = Signal::new(40);
  let gain = Signal::new(0.5_f32);
  let (read_volume, read_gain) = (volume.clone(), gain.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Slider::new(read_volume.clone()).min(10).max(42).step(2).id("volume"))
      .child(Slider::new_f32(read_gain.clone()).step(0.25).id("gain"))
      .into()
  });
  // Tab reaches the first slider and draws the ring around its thumb; the arrows then move what Tab is on.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("ArrowRight");
  assert_eq!(volume.get(), 42);
  stage.key("ArrowRight");
  assert_eq!(volume.get(), 42, "no further than the greatest");
  stage.key("ArrowLeft");
  stage.key("ArrowDown");
  assert_eq!(volume.get(), 38);

  stage.key("Tab");
  stage.key("ArrowUp");
  assert_eq!(gain.get(), 0.75);
  assert_eq!(volume.get(), 38, "the arrows move only the focused slider");
}

#[test]
fn a_press_on_the_track_sets_the_value_where_it_is_and_a_drag_follows_the_pointer() {
  let volume = Signal::new(0);
  let read = volume.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(Slider::new(read.clone()).id("slider"))
      .into()
  });
  // The middle of the slider is the middle of the way the thumb's center goes.
  stage.click("slider");
  assert_eq!(volume.get(), 50);

  let (x, y, ..) = stage.bounds("slider");
  let along = |value: f32| x + 8.0 + 224.0 * value;
  stage.tree.mouse_move(along(0.5), y + 8.0);
  stage.tree.mouse_down(along(0.5), y + 8.0, MouseButton::Left);
  stage.tree.mouse_move(along(0.75), y + 8.0);
  stage.pass();
  assert_eq!(volume.get(), 75);
  stage.tree.mouse_move(along(2.0), y + 8.0);
  stage.tree.mouse_up(along(2.0), y + 8.0, MouseButton::Left);
  stage.pass();
  assert_eq!(volume.get(), 100, "past the end is the greatest");
  assert_eq!(thumb(&mut stage, "slider").x - x, 224.0);
}

#[test]
fn a_disabled_slider_is_dimmed_and_takes_nothing() {
  let volume = Signal::new(40);
  let after = Signal::new(0);
  let (read, read_after) = (volume.clone(), after.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Slider::new(read.clone()).disabled(true).id("disabled"))
      .child(Slider::new(read_after.clone()).id("after"))
      .into()
  });
  stage.click("disabled");
  assert_eq!(volume.get(), 40);
  // Tab passes it by, to the next.
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!((volume.get(), after.get()), (40, 1));

  // The slider is dimmed by the theme's opacity for what is disabled.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(SliderState::Disabled, true);
  let root = theme.resolve::<Slider>(&case, None).part(SliderPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme
    .resolve::<Slider>(&Case::new(), None)
    .part(SliderPart::Root)
    .rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn a_forced_focus_draws_the_ring_around_the_thumb() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        Slider::new(Signal::new(0))
          .force(Some(Interaction::FocusVisible))
          .id("slider"),
      )
      .into()
  });
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  // Two from the thumb, two wide: around the thumb at the start of the track.
  let (x, y, ..) = stage.bounds("slider");
  let ring = stage
    .quads()
    .into_iter()
    .find(|quad| quad.border.and_then(|border| border.top).map(|side| side.color) == Some(light(PaletteColor::Ring)))
    .unwrap();
  assert_eq!(
    (ring.x, ring.y, ring.width, ring.height),
    (x - 4.0, y - 4.0, 24.0, 24.0)
  );
}

#[test]
fn a_theme_restyles_the_slider_and_gives_it_another_look_that_still_slides() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Slider>(|rules| rules.base(SliderPart::Range, |s| s.background(PaletteColor::Destructive)));
  // A slider that shows its value after the track: other nodes, the same behavior.
  let renderer: SliderRenderer = Arc::new(|view: SliderView| {
    let value = format!("{}", view.value());
    Row::new()
      .spacing(8.0)
      .child(view.dressed(view.slider()))
      .child(view.part(SliderPart::Root).text(&value))
      .into()
  });
  app.theme().set_renderer::<Slider>(renderer);
  let volume = Signal::new(40);
  let read = volume.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Row::new()
      .padding(8.0)
      .child(Slider::new(read.clone()).id("slider"))
      .into()
  });
  assert!(stage.paints("40"));
  let range = parts(&mut stage, "slider", 4.0)
    .into_iter()
    .find(|quad| color_of(quad) == light(PaletteColor::Destructive));
  assert!(range.is_some(), "the theme's rule fills the range");
  stage.click("slider");
  assert_eq!(volume.get(), 50);
  assert!(stage.paints("50"));
}

/// The filled rectangles the slider with `id` paints in its box, of a width.
fn columns(stage: &mut Stage, id: &str, width: f32) -> Vec<Quad> {
  let (x, y, slider_width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && quad.width == width
        && quad.x >= x - 0.51
        && quad.y >= y - 0.51
        && quad.x + quad.width <= x + slider_width + 0.51
        && quad.y + quad.height <= y + height + 0.51
    })
    .collect()
}

/// Where the centers of the rings of the focus are, from the left of the slider with `id`.
fn rings(stage: &mut Stage, id: &str) -> Vec<f32> {
  let (x, ..) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| quad.border.and_then(|border| border.top).map(|side| side.color) == Some(light(PaletteColor::Ring)))
    .map(|ring| ring.x + ring.width / 2.0 - x)
    .collect()
}

#[test]
fn a_vertical_slider_runs_up_from_its_least_value() {
  let level = Signal::new(40);
  let read = level.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(
        Slider::new(read.clone())
          .orientation(SliderOrientation::Vertical)
          .label("Level")
          .id("slider"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("slider");
  assert_eq!((width, height), (16.0, 140.0));

  // The track runs up the middle of the slider, 4 wide; the range fills it from the thumb to the bottom.
  let rails = columns(&mut stage, "slider", 4.0);
  let track = rails
    .iter()
    .find(|quad| quad.height == 140.0)
    .expect("a track up the slider");
  assert_eq!((track.x, track.y), (x + 6.0, y));
  assert_eq!(color_of(track), light(PaletteColor::Secondary));
  let range = rails
    .iter()
    .find(|quad| color_of(quad) == light(PaletteColor::Primary))
    .expect("a range");

  // At 40 the thumb is 40 hundredths of the way up the 124 it can go.
  let thumb = thumb(&mut stage, "slider");
  assert_eq!(thumb.x, x);
  assert!(
    (thumb.y - (y + 124.0 - 0.4 * 124.0)).abs() <= 0.51,
    "thumb at {}",
    thumb.y - y
  );
  assert!((range.y - (thumb.y + 8.0)).abs() <= 0.51);
  assert!((range.y + range.height - (y + 140.0)).abs() <= 0.51);

  // A press at the top is the greatest value; up and down move it as right and left do one across.
  stage.tree.mouse_down(x + 8.0, y + 2.0, MouseButton::Left);
  stage.tree.mouse_up(x + 8.0, y + 2.0, MouseButton::Left);
  stage.pass();
  assert_eq!(level.get(), 100);
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(level.get(), 98);
  stage.key("ArrowUp");
  assert_eq!(level.get(), 99);
}

#[test]
fn the_length_is_along_the_track() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Slider::new(Signal::new(0)).length(300.0).id("across"))
      .child(
        Slider::new(Signal::new(0))
          .orientation(SliderOrientation::Vertical)
          .length(200.0)
          .id("up"),
      )
      .into()
  });
  let (_, _, width, height) = stage.bounds("across");
  assert_eq!((width, height), (300.0, 16.0));
  let (_, _, width, height) = stage.bounds("up");
  assert_eq!((width, height), (16.0, 200.0));
}

#[test]
fn a_range_slider_fills_between_its_two_thumbs() {
  let price = Signal::new((20, 80));
  let read = price.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(RangeSlider::new(read.clone()).label("Price").id("slider"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("slider");
  assert_eq!((width, height), (240.0, 16.0));

  let thumbs: Vec<Quad> = parts(&mut stage, "slider", 16.0)
    .into_iter()
    .filter(|quad| quad.width == 16.0)
    .collect();
  assert_eq!(thumbs.len(), 2);
  assert!((thumbs[0].x - x - 0.2 * 224.0).abs() <= 0.51);
  assert!((thumbs[1].x - x - 0.8 * 224.0).abs() <= 0.51);
  assert_eq!(color_of(&thumbs[1]), light(PaletteColor::Background));

  let rails = parts(&mut stage, "slider", 4.0);
  assert!(
    rails
      .iter()
      .any(|quad| quad.width == 240.0 && color_of(quad) == light(PaletteColor::Secondary))
  );
  let range = rails
    .iter()
    .find(|quad| color_of(quad) == light(PaletteColor::Primary))
    .expect("a range between the thumbs");
  assert!((range.x - (thumbs[0].x + 8.0)).abs() <= 0.51);
  assert!((range.x + range.width - (thumbs[1].x + 8.0)).abs() <= 0.51);

  // A press near the high thumb moves it, and a drag of the low one past it stops where the high one is.
  let along = |value: f32| x + 8.0 + 224.0 * value;
  stage.tree.mouse_down(along(0.9), y + 8.0, MouseButton::Left);
  stage.tree.mouse_up(along(0.9), y + 8.0, MouseButton::Left);
  stage.pass();
  assert_eq!(price.get(), (20, 90));
  stage.tree.mouse_down(along(0.2), y + 8.0, MouseButton::Left);
  stage.tree.mouse_move(along(1.0), y + 8.0);
  stage.tree.mouse_up(along(1.0), y + 8.0, MouseButton::Left);
  stage.pass();
  assert_eq!(price.get(), (90, 90));
}

#[test]
fn tab_reaches_each_thumb_of_a_range_slider_and_the_ring_is_on_the_focused_one() {
  let price = Signal::new((20, 80));
  let read = price.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(RangeSlider::new(read.clone()).step(5).id("slider"))
      .into()
  });

  stage.key("Tab");
  let ringed = rings(&mut stage, "slider");
  assert_eq!(ringed.len(), 1, "one ring");
  assert!((ringed[0] - (8.0 + 0.2 * 224.0)).abs() <= 0.51, "on the low thumb");
  stage.key("ArrowRight");
  assert_eq!(price.get(), (25, 80));

  stage.key("Tab");
  let ringed = rings(&mut stage, "slider");
  assert_eq!(ringed.len(), 1);
  assert!((ringed[0] - (8.0 + 0.8 * 224.0)).abs() <= 0.51, "on the high thumb");
  stage.key("ArrowLeft");
  assert_eq!(price.get(), (25, 75));

  // Shift+Tab goes back to the low one.
  stage
    .tree
    .key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage.pass();
  stage.key("ArrowLeft");
  assert_eq!(price.get(), (20, 75));
}

#[test]
fn a_disabled_range_slider_takes_nothing() {
  let price = Signal::new((20, 80));
  let read = price.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(RangeSlider::new(read.clone()).disabled(true).id("slider"))
      .into()
  });
  stage.click("slider");
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(price.get(), (20, 80));
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_styles_the_range_slider_by_its_state() {
  let app = App::new();
  app.theme().patch_recipe::<Slider>(|rules| {
    rules.state(SliderState::Range, SliderPart::Range, |s| {
      s.background(PaletteColor::Destructive)
    })
  });
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Slider::new(Signal::new(40)).id("one"))
      .child(RangeSlider::new(Signal::new((20, 60))).id("two"))
      .into()
  });
  let destructive = |stage: &mut Stage, id: &str| {
    parts(stage, id, 4.0)
      .iter()
      .any(|quad| color_of(quad) == light(PaletteColor::Destructive))
  };
  assert!(!destructive(&mut stage, "one"));
  assert!(destructive(&mut stage, "two"));
}
