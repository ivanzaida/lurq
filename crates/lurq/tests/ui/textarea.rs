//! A textarea: its box and its text against the board, the rows it shows,
//! what typing into it does, its states, and a theme's rules and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  node::color::Color,
  ui::{
    Case, Field, Interaction, Opacity, Textarea, TextareaPart, TextareaRenderer, TextareaSize, TextareaState,
    TextareaView,
  },
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

#[test]
fn its_text_stays_inside_its_box_while_the_page_scrolls() {
  use lurq::components::{Rect, ScrollVertical};
  let lines = (1..=30).map(|n| format!("line {n}")).collect::<Vec<_>>().join(
    "
",
  );
  let mut stage = Stage::new(move |_| {
    ScrollVertical::new(
      Column::new()
        .padding(40.0)
        .spacing(20.0)
        .child(Rect::new(100.0, 60.0))
        .child(Textarea::new(Signal::new(lines.clone())).rows(2, 4).id("note"))
        .child(Rect::new(100.0, 900.0)),
    )
    .size(300.0, 280.0)
    .into()
  });
  for (step, delta) in [0.0, -37.3, -12.6, -41.9, 20.2].into_iter().enumerate() {
    if delta != 0.0 {
      stage
        .tree
        .scroll(10.0, 10.0, 0.0, delta, lurq::app::events::ScrollPhase::Scroll);
      stage.pass();
    }
    let (_, y, _, height) = stage.bounds("note");
    let mut seen = false;
    for quad in stage.quads() {
      let lurq::layout::quad::QuadContent::Text { text, .. } = &quad.content else {
        continue;
      };
      if !text.starts_with("line") {
        continue;
      }
      seen = true;
      let clip = quad.clip;
      assert!(clip.active, "the text is clipped");
      assert!(
        clip.y >= y - 0.01 && clip.y + clip.height <= y + height + 0.01,
        "step {step}: the text shows only inside the box: clip {}..{} in box {}..{}",
        clip.y,
        clip.y + clip.height,
        y,
        y + height
      );
    }
    assert!(seen, "step {step}: the text is painted");
  }
}
/// Presses the text `content` where it is painted: a label, which has no id of its own.
fn press_text(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} should be painted"));
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn a_read_only_textarea_is_focused_and_refuses_a_typed_character() {
  let value = Signal::new("By using lurq you agree.".to_owned());
  let told = Arc::new(Mutex::new(Vec::new()));
  let (bound, heard) = (value.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    column(
      Textarea::new(bound.clone())
        .read_only(true)
        .on_change(move |text| heard.lock().unwrap().push(text.to_owned()))
        .id("terms"),
    )
    .into()
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("terms"), "still a stop of Tab");
  stage.key("x");
  stage.key("Enter");
  assert_eq!(value.get(), "By using lurq you agree.");
  assert!(told.lock().unwrap().is_empty(), "no edit was made to tell of");
  // The board's read-only input: the muted fill in the color of a border.
  assert_eq!(stage.fill_color("terms"), Some(light(PaletteColor::Muted)));
  assert_eq!(edge(&mut stage, "terms"), Some(light(PaletteColor::Border)));
}

#[test]
fn the_sizes_have_the_type_of_an_inputs_and_rows_of_that_type() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .spacing(8.0)
      .child(column(
        Textarea::new(Signal::new(String::new()))
          .size(TextareaSize::Sm)
          .placeholder("small")
          .id("sm"),
      ))
      .child(column(
        Textarea::new(Signal::new(String::new())).placeholder("medium").id("md"),
      ))
      .child(column(
        Textarea::new(Signal::new(String::new()))
          .size(TextareaSize::Lg)
          .placeholder("large")
          .id("lg"),
      ))
      .into()
  });
  for (id, placeholder, font_size) in [("sm", "small", 13.0), ("md", "medium", 14.0), ("lg", "large", 15.0)] {
    assert_eq!(
      stage.text(placeholder),
      (light(PaletteColor::MutedForeground), font_size),
      "{id}"
    );
    let height = stage.bounds(id).3;
    let rows = 3.0 * font_size * 1.5 + 2.0 * PADDING_Y;
    assert!((height - rows).abs() < 0.5, "{id}: {height} for {rows}");
  }
}

#[test]
fn a_warning_and_a_valid_textarea_have_their_edges_under_the_pointer_too() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .spacing(8.0)
      .child(column(
        Textarea::new(Signal::new(String::new())).warning(true).id("warning"),
      ))
      .child(column(
        Textarea::new(Signal::new(String::new())).valid(true).id("valid"),
      ))
      .child(column(
        Textarea::new(Signal::new(String::new()))
          .invalid(true)
          .warning(true)
          .id("both"),
      ))
      .into()
  });
  assert_eq!(edge(&mut stage, "warning"), Some(light(PaletteColor::Warning)));
  assert_eq!(edge(&mut stage, "valid"), Some(light(PaletteColor::Success)));
  assert_eq!(
    edge(&mut stage, "both"),
    Some(light(PaletteColor::Destructive)),
    "what is wrong over what is doubtful"
  );
  stage.hover("valid");
  assert_eq!(edge(&mut stage, "valid"), Some(light(PaletteColor::Success)));
}

#[test]
fn a_field_judges_its_textarea_and_its_label_focuses_it() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .width(260.0)
      .padding(8.0)
      .child(Field::new("Bio", Textarea::new(bound.clone()).id("bio")).error("Say something about yourself."))
      .into()
  });
  assert_eq!(edge(&mut stage, "bio"), Some(light(PaletteColor::Destructive)));
  press_text(&mut stage, "Bio");
  assert_eq!(focused(&stage).as_deref(), Some("bio"));
  stage.key("H");
  stage.key("i");
  assert_eq!(value.get(), "Hi");
}

#[test]
fn a_field_calls_its_textarea_by_its_label() {
  let app = App::new();
  let names = Arc::new(Mutex::new(Vec::new()));
  let heard = names.clone();
  let renderer: TextareaRenderer = Arc::new(move |view: TextareaView| {
    heard.lock().unwrap().push(view.label.as_deref().map(str::to_owned));
    view.render()
  });
  app.theme().set_renderer::<Textarea>(renderer);
  let _stage = Stage::in_app(app, |_| {
    Column::new()
      .width(260.0)
      .child(Field::new("Bio", Textarea::new(Signal::new(String::new()))))
      .child(Field::new(
        "Notes",
        Textarea::new(Signal::new(String::new())).label("Private notes"),
      ))
      .into()
  });
  let names = names.lock().unwrap();
  assert_eq!(names[..2], [Some("Bio".to_owned()), Some("Private notes".to_owned())]);
}
