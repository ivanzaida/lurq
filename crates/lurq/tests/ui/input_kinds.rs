//! The inputs made of an input: a password input with its eye and its meter, a search field with its icon, its
//! clear button and its hint, and an input group with what stands before and after its input.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Button, Field, Input, InputAddon, InputGroup, InputGroupPart, PasswordInput, PasswordInputPart, PasswordStrength,
    SearchField, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Types `text` at the start of the focused input.
fn type_in(stage: &mut Stage, text: &str) {
  stage.tree.text_input_replace(0..0, text);
  stage.pass();
}

/// A press of the pointer at a point.
fn press_at(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// The texts painted, as they are painted.
fn painted(stage: &Stage) -> Vec<String> {
  stage
    .quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Text { text, .. } => Some(text.to_string()),
      _ => None,
    })
    .collect()
}

/// The filled rectangles of `color`.
fn fills(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect()
}

/// The width and the color of the top of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).expect("an edge on top");
  (side.width, side.color)
}

/// Whether a line of 2 in `color` is drawn inside the box of the element with `id`: the ring of its focus.
fn ring_inside(stage: &mut Stage, id: &str, color: Color) -> bool {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().any(|quad| {
    quad
      .border
      .and_then(|border| border.top)
      .map(|side| (side.width, side.color))
      == Some((2.0, color))
      && quad.x >= x - 0.5
      && quad.y >= y - 0.5
      && quad.x + quad.width <= x + width + 0.5
      && quad.y + quad.height <= y + height + 0.5
  })
}

// The password input.

/// Where the eye of the password input with `id` is: 12 in from the end of the box, 16 wide, in its middle.
fn eye(stage: &mut Stage, id: &str) -> (f32, f32) {
  let (x, y, width, height) = stage.bounds(id);
  (x + width - 12.0 - 8.0, y + height / 2.0)
}

fn password_scene(value: Signal<String>, shown: Signal<bool>) -> Stage {
  Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        PasswordInput::new(value.clone())
          .placeholder("Password")
          .bind_reveal(shown.clone())
          .id("password"),
      )
      .into()
  })
}

#[test]
fn a_password_is_masked_in_a_box_of_an_input_with_the_eye_at_its_end() {
  let (value, shown) = (Signal::new("hunter2".to_owned()), Signal::new(false));
  let mut stage = password_scene(value, shown);
  let (x, _, width, height) = stage.bounds("password");
  assert_eq!((width, height), (260.0, 36.0));
  assert_eq!(edge(&mut stage, "password"), (1.0, light(PaletteColor::Input)));
  let texts = painted(&stage);
  assert!(!texts.iter().any(|text| text.contains("hunter2")), "masked: {texts:?}");
  assert!(
    texts.iter().any(|text| text.starts_with('\u{2022}')),
    "the dots: {texts:?}"
  );
  // The eye is an icon of 16 whose end is 12 from the end of the box, in the muted foreground.
  let eye = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Text { style, .. } if style.color == light(PaletteColor::MutedForeground))
        && quad.x > x + width - 40.0
    })
    .expect("the eye is painted at the end of the box");
  assert!(eye.x + eye.width <= x + width - 12.0 + 1.0, "12 from the end");
}

#[test]
fn a_press_of_the_eye_shows_the_password_and_another_hides_it() {
  let (value, shown) = (Signal::new("hunter2".to_owned()), Signal::new(false));
  let mut stage = password_scene(value.clone(), shown.clone());
  let (ex, ey) = eye(&mut stage, "password");
  press_at(&mut stage, ex, ey);
  assert!(shown.get(), "the app's signal is told");
  assert!(stage.paints("hunter2"), "shown as it is");
  let glyph = |name: &str| icons::ALL.iter().find(|(icon, _)| *icon == name).unwrap().1.to_string();
  assert!(stage.paints(&glyph(icons::EYE_OFF)), "the eye with a line through it");
  assert!(!stage.paints(&glyph(icons::EYE)));
  press_at(&mut stage, ex, ey);
  assert!(!shown.get());
  assert!(!stage.paints("hunter2"), "masked again");
  assert!(stage.paints(&glyph(icons::EYE)), "the eye again");
  assert_eq!(value.get(), "hunter2", "what it holds is the same");
}

#[test]
fn the_eye_is_a_stop_of_tab_after_the_input_and_enter_presses_it() {
  let (value, shown) = (Signal::new(String::new()), Signal::new(false));
  let mut stage = password_scene(value.clone(), shown.clone());
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("password"));
  type_in(&mut stage, "s3cret");
  assert_eq!(value.get(), "s3cret");
  stage.key("Tab");
  assert_ne!(
    focused(&stage).as_deref(),
    Some("password"),
    "the focus went on to the eye"
  );
  stage.key("Enter");
  assert!(shown.get(), "Enter pressed the eye");
  assert!(stage.paints("s3cret"));
}

#[test]
fn a_disabled_password_input_takes_no_press_of_its_eye() {
  let shown = Signal::new(false);
  let bound = shown.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        PasswordInput::new(Signal::new("hunter2".to_owned()))
          .bind_reveal(bound.clone())
          .disabled(true)
          .id("password"),
      )
      .into()
  });
  let (ex, ey) = eye(&mut stage, "password");
  press_at(&mut stage, ex, ey);
  assert!(!shown.get());
  stage.click("password");
  assert_eq!(focused(&stage), None);
}

#[test]
fn without_anything_to_hear_it_there_is_no_eye() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(PasswordInput::new(Signal::new("hunter2".to_owned())).id("password"))
      .into()
  });
  let (x, _, width, _) = stage.bounds("password");
  assert!(
    !stage
      .quads()
      .into_iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { style, .. } if style.color == light(PaletteColor::MutedForeground))
        && quad.x > x + width - 40.0),
    "nothing at the end of the box"
  );
}

#[test]
fn the_meter_fills_as_many_bars_as_the_strength_reaches_in_its_color() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(
        PasswordInput::new(Signal::new("correct horse".to_owned()))
          .strength(Some(PasswordStrength::Good))
          .id("good"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("good");
  let bar = |quad: &Quad| quad.height == 4.0 && quad.y > y + height;
  let filled: Vec<Quad> = fills(&stage, light(PaletteColor::Success))
    .into_iter()
    .filter(bar)
    .collect();
  let empty: Vec<Quad> = fills(&stage, light(PaletteColor::Input))
    .into_iter()
    .filter(bar)
    .collect();
  assert_eq!((filled.len(), empty.len()), (3, 1), "three of four");
  // 8 under the box, four bars 4 apart across its width.
  let first = filled.iter().map(|quad| quad.x).fold(f32::MAX, f32::min);
  assert!((first - x).abs() < 0.5);
  assert!((filled[0].y - (y + height + 8.0)).abs() < 0.5, "8 under the box");
  assert!(
    (filled[0].width - (width - 12.0) / 4.0).abs() < 0.5,
    "a quarter less the gaps: {} of {width}",
    filled[0].width
  );
  assert_eq!(stage.text("Good password"), (light(PaletteColor::Success), 12.0));
}

#[test]
fn a_weak_password_is_one_bar_in_the_destructive_color_and_a_message_of_the_app() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        PasswordInput::new(Signal::new("abc".to_owned()))
          .strength(Some(PasswordStrength::Weak))
          .strength_message("Too short"),
      )
      .into()
  });
  let bars = fills(&stage, light(PaletteColor::Destructive))
    .into_iter()
    .filter(|quad| quad.height == 4.0)
    .count();
  assert_eq!(bars, 1);
  assert_eq!(stage.text("Too short").0, light(PaletteColor::Destructive));
}

#[test]
fn a_field_names_and_judges_a_password_input() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Field::new(
          "Password",
          PasswordInput::new(Signal::new("x".to_owned())).id("password"),
        )
        .error("Too short."),
      )
      .into()
  });
  assert_eq!(edge(&mut stage, "password"), (1.0, light(PaletteColor::Destructive)));
}

#[test]
fn a_theme_changes_the_eye() {
  let app = App::new();
  app.theme().patch_recipe::<PasswordInput>(|recipe| {
    recipe.base(PasswordInputPart::Toggle, |s| s.foreground(PaletteColor::Primary))
  });
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(
        PasswordInput::new(Signal::new("x".to_owned()))
          .on_reveal(|_| {})
          .id("password"),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("password");
  assert!(stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Text { style, .. } if style.color == light(PaletteColor::Primary))
      && quad.x > x + width - 40.0
  }));
}

// The search field.

fn search_scene(value: Signal<String>, submitted: Arc<Mutex<Vec<String>>>) -> Stage {
  Stage::new(move |_| {
    let submitted = submitted.clone();
    Column::new()
      .padding(8.0)
      .child(
        SearchField::new(value.clone())
          .placeholder("Search components")
          .shortcut("/")
          .on_submit(move |query| submitted.lock().unwrap().push(query.to_owned()))
          .id("search"),
      )
      .into()
  })
}

#[test]
fn a_search_field_has_its_icon_before_the_text_and_the_hint_of_its_key_at_its_end() {
  let mut stage = search_scene(Signal::new(String::new()), Arc::default());
  let (x, y, width, height) = stage.bounds("search");
  assert_eq!((width, height), (280.0, 36.0));
  assert_eq!(edge(&mut stage, "search"), (1.0, light(PaletteColor::Input)));
  // The icon 10 in, the placeholder 8 after it.
  let placeholder = stage.text_quad("Search components").unwrap();
  assert!(
    (placeholder.x - (x + 10.0 + 16.0 + 8.0)).abs() < 1.0,
    "{}",
    placeholder.x - x
  );
  assert_eq!(stage.text("Search components").0, light(PaletteColor::MutedForeground));
  // The hint: 20 high on the muted fill, 8 from the end, in the mono type of 11.
  assert_eq!(stage.text("/"), (light(PaletteColor::MutedForeground), 11.0));
  let hint = fills(&stage, light(PaletteColor::Muted))
    .into_iter()
    .find(|quad| quad.height == 20.0)
    .expect("the hint's fill");
  assert!((hint.x + hint.width - (x + width - 8.0)).abs() < 1.0, "8 from the end");
  assert!((hint.y - (y + 8.0)).abs() < 1.0, "in the middle");
}

#[test]
fn typing_shows_the_clear_button_in_place_of_the_hint_and_a_press_of_it_empties_the_field() {
  let query = Signal::new(String::new());
  let mut stage = search_scene(query.clone(), Arc::default());
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("search"));
  type_in(&mut stage, "blur");
  assert_eq!(query.get(), "blur");
  assert!(!stage.paints("/"), "the hint is gone");
  let (x, y, width, height) = stage.bounds("search");
  // The clear button is 16, 8 from the end.
  press_at(&mut stage, x + width - 8.0 - 8.0, y + height / 2.0);
  assert_eq!(query.get(), "");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("search"),
    "the caret is back in the field"
  );
  assert!(stage.paints("/"), "the hint is back");
}

#[test]
fn escape_empties_the_field_and_enter_submits_it() {
  let (query, submitted) = (Signal::new(String::new()), Arc::new(Mutex::new(Vec::new())));
  let mut stage = search_scene(query.clone(), submitted.clone());
  stage.key("Tab");
  type_in(&mut stage, "card");
  stage.key("Enter");
  assert_eq!(*submitted.lock().unwrap(), vec!["card".to_owned()]);
  assert_eq!(focused(&stage).as_deref(), Some("search"), "Enter keeps the focus");
  stage.key("Escape");
  assert_eq!(query.get(), "");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("search"),
    "Escape that empties keeps the focus"
  );
}

#[test]
fn a_search_field_is_focused_with_the_ring_inside_its_box() {
  let mut stage = search_scene(Signal::new(String::new()), Arc::default());
  stage.key("Tab");
  assert!(ring_inside(&mut stage, "search", light(PaletteColor::Ring)));
}

#[test]
fn a_disabled_search_field_has_no_clear_button_and_takes_nothing() {
  let query = Signal::new("blur".to_owned());
  let bound = query.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(SearchField::new(bound.clone()).disabled(true).id("search"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("search");
  press_at(&mut stage, x + width - 16.0, y + height / 2.0);
  assert_eq!(query.get(), "blur");
  assert_eq!(focused(&stage), None);
}

// The input group.

#[test]
fn a_group_is_one_box_with_its_addons_on_the_muted_fill_and_a_line_between() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        InputGroup::new(Input::new(Signal::new("lurq".to_owned())).id("site"))
          .before("https://")
          .after(".com")
          .id("group"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("group");
  assert_eq!((width, height), (280.0, 36.0));
  assert_eq!(edge(&mut stage, "group"), (1.0, light(PaletteColor::Input)));
  assert!(
    stage
      .border("group")
      .unwrap()
      .border_radius
      .is_some_and(|radius| radius.top_left == 6.0)
  );
  assert_eq!(stage.text("https://"), (light(PaletteColor::MutedForeground), 13.0));
  assert_eq!(stage.text(".com"), (light(PaletteColor::MutedForeground), 13.0));
  let addons: Vec<Quad> = fills(&stage, light(PaletteColor::Muted))
    .into_iter()
    .filter(|quad| quad.height == height)
    .collect();
  assert_eq!(addons.len(), 2, "two addons as high as the box");
  // The input is between the lines: its box has no edge of its own, and its text is 10 in.
  let lines = fills(&stage, light(PaletteColor::Input))
    .into_iter()
    .filter(|quad| quad.width == 1.0 && quad.height == height)
    .count();
  assert_eq!(lines, 2);
  let (sx, _, swidth, _) = stage.bounds("site");
  assert!(sx > x && sx + swidth < x + width);
  let value = stage.text_quad("lurq").unwrap();
  assert!((value.x - sx - 10.0).abs() < 1.0);
  assert!((y - stage.bounds("site").1).abs() < 0.5, "as high as the box");
}

#[test]
fn the_input_of_a_group_types_and_is_focused_with_its_own_ring() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(InputGroup::new(Input::new(bound.clone()).id("site")).before("https://"))
      .into()
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("site"));
  type_in(&mut stage, "lurq.dev");
  assert_eq!(value.get(), "lurq.dev");
  assert!(ring_inside(&mut stage, "site", light(PaletteColor::Ring)));
}

#[test]
fn a_button_after_the_input_is_round_where_the_box_is_and_is_pressed() {
  let pressed = Arc::new(Mutex::new(0));
  let told = pressed.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(
        InputGroup::new(Input::new(Signal::new(String::new())).placeholder("Email"))
          .before(InputAddon::icon(icons::MAIL))
          .after(
            Button::new("Invite")
              .id("invite")
              .on_click(move |_| *told.lock().unwrap() += 1),
          )
          .id("group"),
      )
      .into()
  });
  let radius = stage
    .fill("invite")
    .and_then(|quad| quad.border_radius)
    .expect("the button's fill");
  assert_eq!((radius.top_left, radius.top_right), (0.0, 6.0));
  let (gx, _, gwidth, _) = stage.bounds("group");
  let (bx, _, bwidth, _) = stage.bounds("invite");
  assert!((bx + bwidth - (gx + gwidth)).abs() < 0.5, "at the end of the box");
  stage.click("invite");
  assert_eq!(*pressed.lock().unwrap(), 1);
  // The icon beside the input is the input's own, inside its box: no addon on the muted fill.
  assert!(
    fills(&stage, light(PaletteColor::Muted))
      .into_iter()
      .all(|quad| quad.height != 36.0)
  );
}

#[test]
fn a_disabled_group_takes_nothing() {
  let pressed = Arc::new(Mutex::new(0));
  let told = pressed.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(
        InputGroup::new(Input::new(Signal::new(String::new())).id("site"))
          .after(Button::new("Go").id("go").on_click(move |_| *told.lock().unwrap() += 1))
          .disabled(true),
      )
      .into()
  });
  stage.click("site");
  assert_eq!(focused(&stage), None);
  stage.click("go");
  assert_eq!(*pressed.lock().unwrap(), 0);
}

#[test]
fn the_edge_of_the_group_says_what_its_input_holds() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Field::new(
          "Site",
          InputGroup::new(Input::new(Signal::new("lurq".to_owned())))
            .before("https://")
            .id("group"),
        )
        .error("Taken."),
      )
      .into()
  });
  assert_eq!(edge(&mut stage, "group"), (1.0, light(PaletteColor::Destructive)));
}

#[test]
fn a_theme_changes_the_addons() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<InputGroup>(|recipe| recipe.base(InputGroupPart::Addon, |s| s.background(PaletteColor::Primary)));
  let stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(8.0)
      .child(InputGroup::new(Input::new(Signal::new(String::new()))).before("https://"))
      .into()
  });
  assert!(!fills(&stage, light(PaletteColor::Primary)).is_empty());
}
