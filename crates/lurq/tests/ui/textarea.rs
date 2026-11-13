//! A textarea: its box and its text against the board, the rows it shows,
//! what typing into it does, its states, and a theme's rules and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  node::color::Color,
  ui::{Case, Interaction, Opacity, Textarea, TextareaPart, TextareaRenderer, TextareaState, TextareaView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A line of the board's text: 14 with a line of 1.5.
const LINE: f32 = 21.0;
/// The board's padding: 8 above the text and 8 under it.
const PADDING_Y: f32 = 8.0;

/// A column as wide as the board draws a textarea, with `textarea` in it.
fn column(textarea: Textarea) -> Column {
  Column::new().width(260.0).padding(8.0).child(textarea)
}

/// The color of the line around the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> Option<Color> {
  stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color)
}

#[test]
fn a_textarea_is_a_box_as_wide_as_its_room_with_the_placeholder_in_it() {
  let mut stage = Stage::new(|_| {
    column(
      Textarea::new(Signal::new(String::new()))
        .placeholder("Tell us what happened")
        .id("bio"),
    )
    .into()
  });
  let (x, y, width, height) = stage.bounds("bio");
  assert_eq!(width, 244.0, "the room of its column");
  assert_eq!(height, 3.0 * LINE + 2.0 * PADDING_Y, "three rows, padded 8 down");
  assert_eq!(stage.fill_color("bio"), Some(light(PaletteColor::Background)));
  assert_eq!(edge(&mut stage, "bio"), Some(light(PaletteColor::Input)));
  let rounded = stage.fill("bio").and_then(|quad| quad.border_radius).unwrap();
  assert_eq!(rounded.top_left, 6.0, "the medium radius");

  // The placeholder is 12 in and 8 down, at 14 in the muted foreground.
  assert_eq!(
    stage.text("Tell us what happened"),
    (light(PaletteColor::MutedForeground), 14.0)
  );
  let placeholder = stage.text_quad("Tell us what happened").unwrap();
  assert!((placeholder.x - (x + 12.0)).abs() < 1.0, "12 in: {}", placeholder.x - x);
  assert!(placeholder.y >= y + PADDING_Y - 1.0 && placeholder.y < y + PADDING_Y + LINE);
}

#[test]
fn it_grows_with_its_text_to_the_rows_it_shows_at_most() {
  let lines = |count: usize| (1..=count).map(|n| format!("line {n}")).collect::<Vec<_>>().join("\n");
  let (four, twelve) = (lines(4), lines(12));
  let mut stage = Stage::new(move |_| {
    Row::new()
      .spacing(8.0)
      .child(column(Textarea::new(Signal::new(four.clone())).id("four")))
      .child(column(
        Textarea::new(Signal::new(twelve.clone())).rows(2, 5).id("twelve"),
      ))
      .child(column(Textarea::new(Signal::new(String::new())).rows(2, 5).id("empty")))
      .into()
  });
  assert_eq!(
    stage.bounds("four").3,
    4.0 * LINE + 2.0 * PADDING_Y,
    "as high as its text"
  );
  assert_eq!(
    stage.bounds("twelve").3,
    5.0 * LINE + 2.0 * PADDING_Y,
    "the most it shows"
  );
  assert_eq!(
    stage.bounds("empty").3,
    2.0 * LINE + 2.0 * PADDING_Y,
    "the least it shows"
  );
  // A multiline input paints its text as it lays it out, not a quad for each line of the value: the first
  // one that holds the first line says the color and the size.
  let painted = stage
    .quads()
    .into_iter()
    .find_map(|quad| match &quad.content {
      lurq::layout::quad::QuadContent::Text { text, style, .. } if text.starts_with("line 1") => {
        Some((style.color, style.font_size))
      }
      _ => None,
    })
    .unwrap_or_else(|| {
      let texts: Vec<String> = stage
        .quads()
        .into_iter()
        .filter_map(|quad| match quad.content {
          lurq::layout::quad::QuadContent::Text { text, .. } => Some(text.to_string()),
          _ => None,
        })
        .collect();
      panic!("no text starting with \"line 1\" was painted: {texts:?}")
    });
  assert_eq!(
    painted,
    (light(PaletteColor::Foreground), 14.0),
    "its text in the foreground"
  );
}

#[test]
fn tab_reaches_it_and_what_is_typed_goes_into_the_signal() {
  let value = Signal::new(String::new());
  let told = Arc::new(Mutex::new(Vec::new()));
  let (bound, heard) = (value.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    column(
      Textarea::new(bound.clone())
        .id("note")
        .on_change(move |text| heard.lock().unwrap().push(text.to_owned())),
    )
    .into()
  });
  stage.key("Tab");
  stage.key("H");
  stage.key("i");
  stage.key("Enter");
  stage.key("!");
  assert_eq!(value.get(), "Hi\n!", "Tab focused it, and Enter is a new line");
  assert_eq!(told.lock().unwrap().last().map(String::as_str), Some("Hi\n!"));
  assert_eq!(told.lock().unwrap().len(), 4);
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the ring of the focus");
  // The ring is inside the box: no line is drawn outside it.
  let (x, y, width, height) = stage.bounds("note");
  let ring = stage
    .quads()
    .into_iter()
    .find(|quad| quad.border.and_then(|border| border.top).map(|side| side.color) == Some(light(PaletteColor::Ring)))
    .unwrap();
  assert!(ring.x >= x && ring.y >= y && ring.x + ring.width <= x + width && ring.y + ring.height <= y + height);

  // The app's own write shows in it.
  value.set("Written by the app".to_owned());
  stage.pass();
  assert!(stage.paints("Written by the app"));
}

#[test]
fn an_invalid_textarea_has_the_destructive_edge_and_ring() {
  let mut stage = Stage::new(|_| {
    column(
      Textarea::new(Signal::new("not enough".to_owned()))
        .invalid(true)
        .id("wrong"),
    )
    .into()
  });
  assert_eq!(edge(&mut stage, "wrong"), Some(light(PaletteColor::Destructive)));
  stage.hover("wrong");
  assert_eq!(
    edge(&mut stage, "wrong"),
    Some(light(PaletteColor::Destructive)),
    "under the pointer too"
  );
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Destructive)));
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn under_the_pointer_the_edge_is_the_rings_color() {
  let mut stage = Stage::new(|_| column(Textarea::new(Signal::new(String::new())).id("bio")).into());
  stage.hover("bio");
  assert_eq!(edge(&mut stage, "bio"), Some(light(PaletteColor::Ring)));
}

#[test]
fn a_disabled_textarea_is_dimmed_and_takes_nothing() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    column(Textarea::new(bound.clone()).disabled(true).id("off"))
      .child(Textarea::new(Signal::new(String::new())).id("on"))
      .into()
  });
  stage.click("off");
  stage.key("x");
  assert_eq!(value.get(), "", "a click does not focus it");
  // Tab passes it by, to the next one.
  stage.key("Tab");
  stage.key("y");
  assert_eq!(value.get(), "");
  assert!(stage.paints("y"), "the next one took it");

  assert_eq!(stage.fill_color("off"), Some(light(PaletteColor::Muted)));
  let theme = stage.app.theme().clone();
  let case = Case::new().state(TextareaState::Disabled, true);
  let root = theme.resolve::<Textarea>(&case, None).part(TextareaPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  assert_eq!(theme.opacity_value(OpacityLevel::Disabled), 0.5);
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_focused() {
  let stage =
    Stage::new(|_| column(Textarea::new(Signal::new(String::new())).force(Some(Interaction::FocusVisible))).into());
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_restyles_it_and_gives_it_another_anatomy() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Textarea>(|recipe| recipe.base(TextareaPart::Root, |s| s.background(PaletteColor::Card)));
  // A textarea with a count of its characters under it: other nodes around the same input.
  let renderer: TextareaRenderer = Arc::new(|view: TextareaView| {
    let count = format!("{} characters", view.value.get().chars().count());
    let note = view.part(TextareaPart::Placeholder).text(&count);
    Column::new().spacing(6.0).child(view.input()).child(note).into()
  });
  app.theme().set_renderer::<Textarea>(renderer);
  let value = Signal::new("lurq".to_owned());
  let bound = value.clone();
  let mut stage = Stage::in_app(app, move |_| column(Textarea::new(bound.clone()).id("bio")).into());
  assert!(stage.paints("4 characters"));
  assert_eq!(
    stage.fill_color("bio"),
    Some(light(PaletteColor::Card)),
    "the theme's rule"
  );
  stage.key("Tab");
  stage.key("!");
  assert_eq!(value.get(), "lurq!", "it still types");
}
