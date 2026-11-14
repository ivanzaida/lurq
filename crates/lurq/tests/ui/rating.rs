//! A rating: its stars as the board draws them, the part of a star, a press and the keys, the stars lit under the
//! pointer, halves, clearing, read only, disabled, invalid, the sizes, its icons, a field, what tooling reads, and a
//! theme's rules and renderer.

use std::sync::{Arc, Mutex};

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
    Case, Field, Interaction, Opacity, Rating, RatingPart, RatingPrecision, RatingRenderer, RatingSize, RatingState,
    RatingValue, RatingView, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

/// The stars painted, in order along the row, each as its quad and its color.
fn stars(stage: &Stage, icon: &str) -> Vec<(Quad, Color)> {
  let star = glyph(icon);
  let mut found: Vec<(Quad, Color)> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| {
      let color = match &quad.content {
        QuadContent::Text { text, style, .. } if *text == star => style.color,
        _ => return None,
      };
      Some((quad, color))
    })
    .collect();
  found.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
  found
}

/// How many stars are lit in the warning color: those whole, and a part of one counted as the part painted.
fn lit(stage: &Stage) -> f32 {
  stars(stage, icons::STAR_FILL)
    .iter()
    .filter(|(_, color)| *color == light(PaletteColor::Warning))
    .map(|(quad, _)| {
      let painted = if quad.clip.active {
        (quad.x + quad.width).min(quad.clip.x + quad.clip.width) - quad.x
      } else {
        quad.width
      };
      (painted / quad.width).clamp(0.0, 1.0)
    })
    .sum()
}

/// The middle of the star `index` (from 0) of the rating `id`, or a point `along` its width.
fn at(stage: &mut Stage, id: &str, index: usize, along: f32) -> (f32, f32) {
  let (x, y, _, height) = stage.bounds(id);
  let (side, gap) = (20.0, 4.0);
  (x + index as f32 * (side + gap) + side * along, y + height / 2.0)
}

fn press(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn scene<V: RatingValue>(make: impl Fn() -> Rating<V> + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |_| Row::new().padding(8.0).child(make()).into());
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

#[test]
fn at_rest_it_is_the_boards_five_stars_of_20_four_apart() {
  let stage = scene(|| Rating::controlled(4.0, |_| {}).id("rating"));
  let painted = stars(&stage, icons::STAR_FILL);
  assert_eq!(painted.len(), 5);
  for (index, (quad, color)) in painted.iter().enumerate() {
    assert_eq!(quad.height.round(), 20.0, "a star of 20");
    let role = if index < 4 {
      PaletteColor::Warning
    } else {
      PaletteColor::Input
    };
    assert_eq!(*color, light(role), "star {index}");
  }
  let steps: Vec<f32> = painted.windows(2).map(|pair| pair[1].0.x - pair[0].0.x).collect();
  assert!(
    steps.iter().all(|step| (step - 24.0).abs() < 0.01),
    "20 and 4 apart: {steps:?}"
  );
}

#[test]
fn a_rating_shown_at_3_7_lights_seven_tenths_of_the_fourth_star() {
  let stage = scene(|| Rating::at(3.7).id("rating"));
  assert!((lit(&stage) - 3.7).abs() < 0.02, "{}", lit(&stage));
  // Under the part that is lit, the star is drawn empty.
  let empty = stars(&stage, icons::STAR_FILL)
    .into_iter()
    .filter(|(_, color)| *color == light(PaletteColor::Input))
    .count();
  assert_eq!(empty, 2, "the fourth under its lit part, and the fifth");
}

#[test]
fn a_press_sets_the_signal_and_then_tells_the_handlers_and_the_app_setting_it_tells_nobody() {
  let stars_held = Signal::new(2.0_f64);
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (stars_held.clone(), heard.clone());
  let mut stage = scene(move || {
    let (seen, told) = (read.clone(), told.clone());
    Rating::new(read.clone())
      .id("rating")
      .on_change(move |stars| told.lock().unwrap().push((stars, seen.get_untracked())))
  });
  let third = at(&mut stage, "rating", 3, 0.5);
  press(&mut stage, third);
  assert_eq!(stars_held.get(), 4.0);
  assert_eq!(*heard.lock().unwrap(), [(4.0, 4.0)], "told after the signal is set");
  assert!((lit(&stage) - 4.0).abs() < 0.01);
  // A press of the star that is the value changes nothing, and tells nothing.
  press(&mut stage, third);
  assert_eq!(stars_held.get(), 4.0);
  stars_held.set(1.0);
  stage.pass();
  assert_eq!(heard.lock().unwrap().len(), 1);
  assert!((lit(&stage) - 1.0).abs() < 0.01);
}

#[test]
fn in_halves_the_half_of_a_star_under_the_pointer_decides() {
  let stars_held = Signal::new(0.0_f64);
  let read = stars_held.clone();
  let mut stage = scene(move || Rating::new(read.clone()).precision(RatingPrecision::Half).id("rating"));
  let left = at(&mut stage, "rating", 2, 0.25);
  press(&mut stage, left);
  assert_eq!(stars_held.get(), 2.5);
  assert!((lit(&stage) - 2.5).abs() < 0.02);
  let right = at(&mut stage, "rating", 2, 0.75);
  press(&mut stage, right);
  assert_eq!(stars_held.get(), 3.0);
}

#[test]
fn clearable_a_press_of_the_value_takes_it_back_to_none() {
  let given: Signal<Option<f64>> = Signal::new(Some(3.0));
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (given.clone(), heard.clone());
  let mut stage = scene(move || {
    let told = told.clone();
    Rating::optional(read.clone())
      .clearable(true)
      .id("rating")
      .on_change(move |stars| told.lock().unwrap().push(stars))
  });
  let third = at(&mut stage, "rating", 2, 0.5);
  press(&mut stage, third);
  assert_eq!(given.get(), None);
  assert_eq!(*heard.lock().unwrap(), [None]);
  assert_eq!(lit(&stage), 0.0);
}

#[test]
fn the_keys_step_go_to_the_ends_set_a_digit_and_clear() {
  let given: Signal<Option<f64>> = Signal::new(Some(2.0));
  let read = given.clone();
  let mut stage = scene(move || {
    Rating::optional(read.clone())
      .precision(RatingPrecision::Half)
      .clearable(true)
      .id("rating")
  });
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(given.get(), Some(2.5));
  stage.key("ArrowUp");
  assert_eq!(given.get(), Some(3.0));
  stage.key("ArrowLeft");
  stage.key("ArrowDown");
  assert_eq!(given.get(), Some(2.0));
  stage.key("End");
  assert_eq!(given.get(), Some(5.0));
  stage.key("ArrowRight");
  assert_eq!(given.get(), Some(5.0), "no more than the stars");
  stage.key("4");
  assert_eq!(given.get(), Some(4.0));
  stage.key("Backspace");
  assert_eq!(given.get(), None);
  stage.key("Home");
  assert_eq!(given.get(), None, "the least of a clearable rating is none");
  stage.key("ArrowRight");
  assert_eq!(given.get(), Some(0.5));
  // The ring of the focus is around the row of stars.
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_rating_that_is_not_clearable_keeps_a_star() {
  let held = Signal::new(1.0_f64);
  let read = held.clone();
  let mut stage = scene(move || Rating::new(read.clone()).id("rating"));
  stage.key("Tab");
  stage.key("ArrowLeft");
  assert_eq!(held.get(), 1.0);
  stage.key("0");
  stage.key("Backspace");
  assert_eq!(held.get(), 1.0);
  stage.key("Home");
  assert_eq!(held.get(), 1.0);
  stage.key("3");
  assert_eq!(held.get(), 3.0);
}

#[test]
fn mounted_the_stars_up_to_the_one_under_the_pointer_are_lit() {
  let held = Signal::new(1.0_f64);
  let read = held.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(300.0, 200.0)
      .padding(8.0)
      .child(
        Rating::new(read.clone())
          .precision(RatingPrecision::Half)
          .id("rating")
          .mount(ctx),
      )
      .into()
  });
  stage.tree.resize(300, 200);
  stage.pass();
  assert!((lit(&stage) - 1.0).abs() < 0.01);
  let (x, y) = at(&mut stage, "rating", 3, 0.25);
  stage.tree.mouse_move(x, y);
  stage.pass();
  assert!(
    (lit(&stage) - 3.5).abs() < 0.02,
    "a preview of what a press sets: {}",
    lit(&stage)
  );
  assert_eq!(held.get(), 1.0, "a preview sets nothing");
  stage.tree.mouse_move(250.0, 150.0);
  stage.pass();
  assert!((lit(&stage) - 1.0).abs() < 0.01, "the pointer gone, the value again");
}

#[test]
fn a_rating_the_app_holds_shows_what_the_app_answers() {
  let held = Signal::new(2.0_f64);
  let asked = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (held.clone(), asked.clone());
  let mut stage = scene(move || {
    let (set, told) = (read.clone(), told.clone());
    // The app takes no rating under 2.
    Rating::controlled(read.get(), move |stars| {
      told.lock().unwrap().push(stars);
      if stars >= 2.0 {
        set.set(stars);
      }
    })
    .id("rating")
  });
  let first = at(&mut stage, "rating", 0, 0.5);
  press(&mut stage, first);
  assert_eq!(*asked.lock().unwrap(), [1.0]);
  assert!(
    (lit(&stage) - 2.0).abs() < 0.01,
    "what the app holds, not what was pressed"
  );
  let fifth = at(&mut stage, "rating", 4, 0.5);
  press(&mut stage, fifth);
  assert!((lit(&stage) - 5.0).abs() < 0.01);
}

#[test]
fn read_only_and_disabled_take_nothing_and_disabled_is_dimmed() {
  let held = Signal::new(2.0_f64);
  let (one, two) = (held.clone(), held.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Rating::new(one.clone()).read_only(true).id("read"))
      .child(Rating::new(two.clone()).disabled(true).id("off"))
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  for id in ["read", "off"] {
    let star = at(&mut stage, id, 4, 0.5);
    press(&mut stage, star);
  }
  stage.key("Tab");
  stage.key("End");
  assert_eq!(held.get(), 2.0, "neither is a stop of Tab nor takes a press");
  // The root, which holds the stars, is dimmed by the theme's opacity for what is disabled: a layer of what is in
  // it, so the opacity is the layer's and not a quad's. A read-only one is not.
  let theme = stage.app.theme().clone();
  let root = |state: RatingState| {
    theme
      .resolve::<Rating>(&Case::new().state(state, true), None)
      .part(RatingPart::Root)
      .rest()
      .opacity
  };
  assert_eq!(
    root(RatingState::Disabled),
    Some(Opacity::Level(OpacityLevel::Disabled))
  );
  assert_eq!(root(RatingState::ReadOnly), None);
}

#[test]
fn an_invalid_rating_and_one_in_a_field_with_an_error_have_their_empty_stars_destructive() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Rating::controlled(1.0, |_| {}).invalid(true))
      .child(Field::new("Quality", Rating::controlled(1.0, |_| {})).error("Rate the call."))
      .into()
  });
  let destructive = stars(&stage, icons::STAR_FILL)
    .into_iter()
    .filter(|(_, color)| *color == light(PaletteColor::Destructive))
    .count();
  assert_eq!(destructive, 8, "the four empty stars of each");
  // At half the opacity, so that they are not read as lit.
  let empty = stage
    .app
    .theme()
    .resolve::<Rating>(&Case::new().state(RatingState::Invalid, true), None)
    .part(RatingPart::Empty)
    .rest();
  assert_eq!(empty.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn the_sizes_and_the_icons() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Rating::controlled(2.0, |_| {}).size(RatingSize::Sm))
      .child(
        Rating::controlled(2.0, |_| {})
          .size(RatingSize::Lg)
          .icon(icons::HEART)
          .empty_icon(icons::HEART),
      )
      .into()
  });
  let small = stars(&stage, icons::STAR_FILL);
  assert_eq!(small.len(), 5);
  assert!(small.iter().all(|(quad, _)| quad.height.round() == 16.0));
  assert!((small[1].0.x - small[0].0.x - 18.0).abs() < 0.01, "16 and 2 apart");
  let hearts = stars(&stage, icons::HEART);
  assert_eq!(hearts.len(), 5);
  assert!(hearts.iter().all(|(quad, _)| quad.height.round() == 24.0));
  assert!((hearts[1].0.x - hearts[0].0.x - 28.0).abs() < 0.01, "24 and 4 apart");
}

#[test]
fn a_theme_restyles_it_and_gives_it_another_anatomy() {
  const RED: Color = Color::new(200, 30, 30, 255);
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Rating>(|rules| rules.base(RatingPart::Filled, |s| s.foreground(RED)));
  let stage = Stage::in_app(app, |_| {
    Row::new().padding(8.0).child(Rating::controlled(3.0, |_| {})).into()
  });
  assert_eq!(
    stars(&stage, icons::STAR_FILL)
      .iter()
      .filter(|(_, color)| *color == RED)
      .count(),
    3
  );

  let app = App::new();
  let renderer: RatingRenderer = Arc::new(|view: RatingView| {
    let words = format!("{} of {}", view.stars.unwrap_or(0.0), view.max);
    view.root().child(view.part(RatingPart::Root).text(&words)).into()
  });
  app.theme().set_renderer::<Rating>(renderer);
  let stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(8.0)
      .child(Rating::controlled(3.0, |_| {}).state(RatingState::Invalid, true))
      .into()
  });
  assert!(stage.paints("3 of 5"));
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let stage = scene(|| Rating::controlled(3.0, |_| {}).force(Some(Interaction::FocusVisible)));
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_slider_of_stars_and_its_value_in_words() {
  let given: Signal<Option<f64>> = Signal::new(None);
  let read = given.clone();
  let mut stage = scene(move || {
    Rating::optional(read.clone())
      .label("Call quality")
      .clearable(true)
      .id("rating")
  });
  let said = |stage: &mut Stage, name: &str| {
    stage
      .tree
      .get_element_by_id_mut("rating")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("slider"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Call quality"));
  assert_eq!(said(&mut stage, "a11y_valuemin").as_deref(), Some("0"));
  assert_eq!(said(&mut stage, "a11y_valuemax").as_deref(), Some("5"));
  assert_eq!(said(&mut stage, "a11y_valuetext").as_deref(), Some("No rating"));
  given.set(Some(3.5));
  stage.pass();
  assert_eq!(said(&mut stage, "a11y_valuenow").as_deref(), Some("3.5"));
  assert_eq!(said(&mut stage, "a11y_valuetext").as_deref(), Some("3.5 of 5 stars"));

  let mut shown = scene(|| {
    Rating::at(3.7)
      .id("rating")
      .value_text(|stars, max| format!("{stars:?} / {max}"))
  });
  assert_eq!(said(&mut shown, "a11y_readonly").as_deref(), Some("true"));
  assert_eq!(said(&mut shown, "a11y_valuetext").as_deref(), Some("Some(3.7) / 5"));
}

#[test]
fn a_press_focuses_the_rating_and_the_keys_go_on_from_it() {
  let held = Signal::new(0.0_f64);
  let read = held.clone();
  let mut stage = scene(move || Rating::new(read.clone()).id("rating"));
  let second = at(&mut stage, "rating", 1, 0.5);
  press(&mut stage, second);
  assert_eq!(held.get(), 2.0);
  stage.key("ArrowRight");
  assert_eq!(held.get(), 3.0, "the press focused it");
}
