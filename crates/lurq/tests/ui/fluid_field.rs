//! A fluid field (`imRnW`): `Field::fluid`, whose label is inside the box of its control, for an input, a textarea,
//! a select and every other text control a field holds.

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::{Date, DateRange, Signal, Time, YearMonth},
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
  ui::{
    ColorField, Combobox, DateField, DatePicker, DateRangePicker, Field, FieldControl, Input, Interaction, Mention,
    Mentions, MentionsController, MonthPicker, MultiSelect, NumberInput, NumberInputLayout, PasswordInput, SearchField,
    Select, Switch, Textarea, TimeField, TimePicker, Token, TokenInput, YearPicker, icons,
  },
};

use super::stage::Stage;

const LABEL: &str = "Workspace name";
const VALUE: &str = "lurq-demo";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn staged(scene: impl Fn() -> Field + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |_| {
    Column::new().size(400.0, 300.0).padding(8.0).child(scene()).into()
  })
}

fn workspace() -> Signal<String> {
  Signal::new(VALUE.to_owned())
}

/// The width of each side of the edge of the element with `id`, and the color of its bottom.
fn sides(stage: &mut Stage, id: &str) -> ([f32; 4], Color) {
  let quad: Quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have a line"));
  let border = quad.border.expect("a border");
  (
    [
      border.top.map_or(0.0, |side| side.width),
      border.right.map_or(0.0, |side| side.width),
      border.bottom.map_or(0.0, |side| side.width),
      border.left.map_or(0.0, |side| side.width),
    ],
    border.bottom.expect("a line along the bottom").color,
  )
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn press(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

#[test]
fn the_fluid_field_is_the_boards() {
  let mut stage = staged(|| {
    Field::new(LABEL, Input::new(workspace()).id("box"))
      .fluid(true)
      .id("field")
  });
  let (x, y, width, height) = stage.bounds("field");
  assert_eq!((x, y, width, height), (8.0, 8.0, 240.0, 56.0));
  // The control is the box: the whole of the field, on the muted fill, round at its top and square at its bottom,
  // with a line of 1 in the foreground along its bottom and no other edge.
  assert_eq!(stage.bounds("box"), (8.0, 8.0, 240.0, 56.0));
  assert_eq!(stage.fill_color("box"), Some(light(PaletteColor::Muted)));
  assert_eq!(
    sides(&mut stage, "box"),
    ([0.0, 0.0, 1.0, 0.0], light(PaletteColor::Foreground))
  );
  let radius = stage
    .fill("box")
    .and_then(|quad| quad.border_radius)
    .expect("its corners");
  assert_eq!(
    (
      radius.top_left,
      radius.top_right,
      radius.bottom_right,
      radius.bottom_left
    ),
    (6.0, 6.0, 0.0, 0.0)
  );
  // The label: 12 in the muted foreground, 12 from the side and 10 from the top.
  let label = stage.text_quad(LABEL).expect("the label");
  assert_eq!(stage.text(LABEL), (light(PaletteColor::MutedForeground), 12.0));
  assert_eq!(label.x, x + 12.0);
  assert!((label.y - (y + 10.0)).abs() < 0.51, "{}", label.y - y);
  // The value: 14 in the foreground, 12 from the side, 4 under the label's line.
  let value = stage.text_quad(VALUE).expect("the value");
  assert_eq!(stage.text(VALUE), (light(PaletteColor::Foreground), 14.0));
  assert!((value.x - (x + 12.0)).abs() < 1.0);
  assert!((value.y - (y + 29.0)).abs() < 1.0, "{}", value.y - y);
  assert!(value.y >= label.y + label.height + 3.0, "under the label");
}

#[test]
fn its_line_says_what_an_inputs_edge_says() {
  let line = |field: fn(Field) -> Field, input: fn(Input) -> Input| {
    let mut stage = staged(move || field(Field::new(LABEL, input(Input::new(workspace()).id("box"))).fluid(true)));
    sides(&mut stage, "box")
  };
  let only_bottom = [0.0, 0.0, 1.0, 0.0];
  assert_eq!(
    line(|f| f, |i| i.force(Some(Interaction::Hovered))),
    (only_bottom, light(PaletteColor::Ring)),
    "under the pointer"
  );
  assert_eq!(
    line(|f| f.error("Taken."), |i| i),
    (only_bottom, light(PaletteColor::Destructive)),
    "an error"
  );
  assert_eq!(
    line(|f| f.error("Taken."), |i| i.force(Some(Interaction::Hovered))),
    (only_bottom, light(PaletteColor::Destructive)),
    "an error keeps its color under the pointer"
  );
  assert_eq!(
    line(|f| f.warning(true), |i| i),
    (only_bottom, light(PaletteColor::Warning))
  );
  assert_eq!(
    line(|f| f.valid(true), |i| i),
    (only_bottom, light(PaletteColor::Success))
  );
  assert_eq!(
    line(|f| f, |i| i.read_only(true)),
    (only_bottom, light(PaletteColor::Border)),
    "read only"
  );
}

#[test]
fn a_press_of_the_label_focuses_the_input_and_the_focus_is_a_line_of_2_inside_the_box() {
  let name = workspace();
  let bound = name.clone();
  let mut stage = staged(move || {
    Field::new(LABEL, Input::new(bound.clone()).id("box"))
      .fluid(true)
      .description("Shown in the title bar.")
  });
  let (x, y, width, height) = stage.bounds("box");
  // On the label, at the top of the box.
  press(&mut stage, x + 20.0, y + 14.0);
  assert_eq!(focused(&stage).as_deref(), Some("box"));
  stage.tree.text_input_replace(0..0, "my-");
  stage.pass();
  assert_eq!(name.get(), "my-lurq-demo");
  let ring = light(PaletteColor::Ring);
  assert!(
    stage.quads().into_iter().any(|quad| {
      quad
        .border
        .and_then(|border| border.top)
        .map(|side| (side.width, side.color))
        == Some((2.0, ring))
        && quad.x >= x
        && quad.y >= y
        && quad.x + quad.width <= x + width
        && quad.y + quad.height <= y + height
    }),
    "a line of 2 in the ring's color inside the box"
  );
  // The description is under the box, as any field's.
  let description = stage.text_quad("Shown in the title bar.").expect("the description");
  assert!((description.y - (y + height + 6.0)).abs() < 0.01);
}

#[test]
fn an_error_is_under_the_box_and_a_required_field_has_its_mark_after_the_label() {
  let mut stage = staged(|| {
    Field::new(LABEL, Input::new(workspace()).id("box"))
      .fluid(true)
      .required(true)
      .error("That name is taken.")
  });
  assert_eq!(stage.text("That name is taken.").0, light(PaletteColor::Destructive));
  let (label, mark) = (stage.text_quad(LABEL).unwrap(), stage.text_quad("*").expect("the mark"));
  assert!(mark.x >= label.x + label.width, "after the label");
  let (_, y, _, height) = stage.bounds("box");
  assert!(mark.y >= y && mark.y + mark.height <= y + height, "inside the box");
  assert_eq!(stage.text("*").0, light(PaletteColor::Destructive));
}

#[test]
fn the_mark_that_explains_a_fluid_label_is_after_it_and_takes_the_pointer_for_its_tooltip() {
  const WORDS: &str = "Shown in the title bar";
  let name = workspace();
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .child(
        Field::new(LABEL, Input::new(name.clone()).id("box"))
          .fluid(true)
          .required(true)
          .info(WORDS)
          .mount(ctx),
      )
      .into()
  });
  // Low enough on the stage for the tooltip over it.
  let boxed = stage.bounds("box");
  let label = rect(&stage.text_quad(LABEL).expect("the label"));
  assert!(
    label.1 > boxed.1 && label.1 + label.3 < boxed.1 + boxed.3,
    "the label is in the box"
  );
  let star = rect(&stage.text_quad("*").expect("the mark of a required field"));
  let mark = rect(&glyph(&stage, icons::INFO));
  assert!(
    (mark.0 - (star.0 + star.2) - 6.0).abs() < 0.51,
    "6 after the field's mark: {}",
    mark.0 - (star.0 + star.2)
  );
  assert!(
    (middle(mark) - middle(label)).abs() < 1.0,
    "on the label's line: {mark:?} by {label:?}"
  );
  assert!(mark.0 + mark.2 <= boxed.0 + boxed.2, "in the box");
  // The pointer resting on the mark shows what it explains; on the label it goes through to the input.
  assert!(!stage.paints(WORDS));
  stage.tree.mouse_move(mark.0 + mark.2 / 2.0, middle(mark));
  stage.pass();
  stage
    .tree
    .tick_timers_at(std::time::Instant::now() + std::time::Duration::from_millis(600));
  stage.pass();
  assert!(stage.paints(WORDS), "the mark takes the pointer");
  press(&mut stage, label.0 + 10.0, middle(label));
  assert_eq!(
    focused(&stage).as_deref(),
    Some("box"),
    "a press of the label focuses the input"
  );
}

#[test]
fn a_long_label_is_cut_inside_the_box() {
  const LONG: &str = "The name of the workspace as the title bar of every window of it shows it";
  let mut stage = staged(|| Field::new(LONG, Input::new(workspace()).id("box")).fluid(true));
  let (x, _, width, _) = stage.bounds("box");
  let label = stage.cut_text_quad(LONG).expect("the label, cut");
  assert!(
    matches!(&label.content, QuadContent::Text { text, .. } if text.ends_with('…')),
    "it ends in an ellipsis"
  );
  assert!(
    label.x + label.width <= x + width - 12.0 + 0.51,
    "12 before the end of the box"
  );
}

#[test]
fn a_disabled_fluid_field_takes_nothing_and_an_icon_stands_on_the_values_line() {
  let mut stage = staged(|| {
    Field::new(LABEL, Input::new(workspace()).id("box"))
      .fluid(true)
      .disabled(true)
  });
  let (x, y, ..) = stage.bounds("box");
  press(&mut stage, x + 20.0, y + 40.0);
  assert_eq!(focused(&stage), None);
  let mut stage = staged(|| Field::new("Search", Input::new(workspace()).icon(icons::SEARCH).id("box")).fluid(true));
  let glyph = icons::ALL
    .iter()
    .find(|(name, _)| *name == icons::SEARCH)
    .map(|(_, glyph)| glyph.to_string())
    .unwrap();
  let (icon, value, label) = (
    stage.text_quad(&glyph).expect("the icon"),
    stage.text_quad(VALUE).unwrap(),
    stage.text_quad("Search").unwrap(),
  );
  let middle = |quad: &Quad| quad.y + quad.height / 2.0;
  assert!((middle(&icon) - middle(&value)).abs() < 1.5, "on the value's line");
  assert!(icon.y >= label.y + label.height, "under the label");
  assert!(value.x >= icon.x + icon.width + 8.0 - 0.51, "the value starts after it");
  let _ = stage.bounds("box");
}

#[test]
fn a_fluid_textarea_has_the_label_over_its_rows() {
  let notes = Signal::new("First line.".to_owned());
  let mut stage = staged(move || {
    Field::new("Notes", Textarea::new(notes.clone()).rows(3, 6).id("box"))
      .fluid(true)
      .id("field")
  });
  let (x, y, width, height) = stage.bounds("box");
  assert_eq!((x, y, width), (8.0, 8.0, 240.0));
  // 29 over its rows, three lines of 21, 10 under them.
  assert!((height - (29.0 + 63.0 + 10.0)).abs() < 1.5, "{height}");
  assert_eq!(stage.fill_color("box"), Some(light(PaletteColor::Muted)));
  assert_eq!(
    sides(&mut stage, "box"),
    ([0.0, 0.0, 1.0, 0.0], light(PaletteColor::Foreground))
  );
  let label = stage.text_quad("Notes").expect("the label");
  assert!((label.y - (y + 10.0)).abs() < 0.51);
  let value = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("First line.")))
    .expect("the value");
  assert!((value.y - (y + 29.0)).abs() < 1.5, "{}", value.y - y);
}

#[test]
fn a_fluid_select_has_the_label_over_what_is_chosen_and_opens_from_its_box() {
  let theme = Signal::new("system");
  let bound = theme.clone();
  let mut stage = staged(move || {
    Field::new(
      "Theme",
      Select::new(
        bound.clone(),
        [("system", "System"), ("light", "Light"), ("dark", "Dark")],
      )
      .id("box"),
    )
    .fluid(true)
  });
  let (x, y, width, height) = stage.bounds("box");
  assert_eq!((x, y, width, height), (8.0, 8.0, 240.0, 56.0));
  let label = stage.text_quad("Theme").expect("the label");
  assert_eq!(stage.text("Theme"), (light(PaletteColor::MutedForeground), 12.0));
  assert!((label.y - (y + 10.0)).abs() < 0.51);
  let value = stage.text_quad("System").expect("what is chosen");
  assert!((value.x - (x + 12.0)).abs() < 1.0);
  assert!(value.y >= label.y + label.height + 3.0, "under the label");
  assert!(value.y + value.height <= y + height - 9.0, "10 over the bottom");
  // The trigger is the fluid box: the muted fill, and a line along its bottom alone.
  let line = stage
    .quads()
    .into_iter()
    .filter(|quad| (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height))
    .find_map(|quad| quad.border)
    .expect("the trigger's line");
  assert!(line.top.is_none_or(|side| side.width == 0.0), "no edge on top");
  assert_eq!(
    line.bottom.map(|side| (side.width, side.color)),
    Some((1.0, light(PaletteColor::Foreground)))
  );
  // A press of the label, which lies over the trigger, opens the list; a choice is set.
  press(&mut stage, x + 20.0, y + 14.0);
  assert!(stage.paints("Dark"), "the list is open");
  let dark = stage.text_quad("Dark").unwrap();
  press(&mut stage, dark.x + 4.0, dark.y + 4.0);
  assert_eq!(theme.get(), "dark");
}

#[test]
fn a_control_with_no_fluid_form_is_in_a_field_as_ever() {
  let mut stage = staged(|| {
    Field::new("Telemetry", Switch::new("Send crash reports").id("switch"))
      .fluid(true)
      .id("field")
  });
  let label = stage.text_quad("Telemetry").expect("the label");
  let (_, switch_y, ..) = stage.bounds("switch");
  assert!(label.y + label.height <= switch_y, "the label is above the control");
  assert_eq!(stage.text("Telemetry").1, 13.0, "in a field label's type");
}

#[test]
fn it_fills_its_room_when_told_and_is_no_wider_than_a_narrow_one() {
  let mut stage = staged(|| {
    Field::new(LABEL, Input::new(workspace()).id("box"))
      .fluid(true)
      .full_width(true)
  });
  assert_eq!(stage.bounds("box").2, 384.0);
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().id("room").width(200.0).child(
          Field::new(LABEL, Input::new(workspace()).id("box"))
            .fluid(true)
            .id("field"),
        ),
      )
      .into()
  });
  assert_eq!(stage.bounds("field").2, 200.0);
  assert_eq!(stage.bounds("box").2, 200.0);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_the_box_is_called_by_the_fields_label() {
  let mut stage = staged(|| Field::new(LABEL, Input::new(workspace()).id("box")).fluid(true));
  let name = stage
    .tree
    .get_element_by_id_mut("box")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(name.as_deref(), Some(LABEL));
}

// Every text control a field holds has the fluid form. Each of the tests below has the control with the id `box` in a
// fluid field at the corner of the stage, 8 in and 8 down.

/// The stage of `staged`, for a scene that is given the `Ctx`: a field that mounts its control.
fn mounted(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |ctx| {
    Column::new().size(400.0, 300.0).padding(8.0).child(scene(ctx)).into()
  })
}

type Rect = (f32, f32, f32, f32);

fn rect(quad: &Quad) -> Rect {
  (quad.x, quad.y, quad.width, quad.height)
}

/// How far down the page the middle of a box is.
fn middle((_, y, _, height): Rect) -> f32 {
  y + height / 2.0
}

fn glyph(stage: &Stage, name: &str) -> Quad {
  let glyph = stage
    .app
    .theme()
    .icon(name)
    .expect("the theme has the icon")
    .glyph
    .to_string();
  stage
    .text_quad(&glyph)
    .unwrap_or_else(|| panic!("the {name} icon should be painted"))
}

/// What every fluid box is: as wide as the field, on the muted fill, round at its top and square at its bottom, with
/// a line of 1 in the foreground along its bottom and no other edge, and the field's label inside it, 12 in and 10
/// down, in a caption's type. Gives the box and the label.
fn the_fluid_box(stage: &mut Stage, label: &str) -> (Rect, Rect) {
  let (x, y, width, height) = stage.bounds("box");
  assert_eq!((x, y, width), (8.0, 8.0, 240.0), "{label}: as wide as the field");
  assert_eq!(
    stage.fill_color("box"),
    Some(light(PaletteColor::Muted)),
    "{label}: the muted fill"
  );
  assert_eq!(
    sides(stage, "box"),
    ([0.0, 0.0, 1.0, 0.0], light(PaletteColor::Foreground)),
    "{label}: a line along the bottom alone"
  );
  let radius = stage
    .fill("box")
    .and_then(|quad| quad.border_radius)
    .expect("its corners");
  assert_eq!(
    (
      radius.top_left,
      radius.top_right,
      radius.bottom_right,
      radius.bottom_left
    ),
    (6.0, 6.0, 0.0, 0.0),
    "{label}: round at its top"
  );
  let quad = stage
    .text_quad(label)
    .unwrap_or_else(|| panic!("the label {label:?} should be painted"));
  assert_eq!(stage.text(label), (light(PaletteColor::MutedForeground), 12.0));
  assert_eq!(quad.x, x + 12.0, "{label}: 12 in");
  assert!((quad.y - (y + 10.0)).abs() < 0.51, "{label}: 10 down, {}", quad.y - y);
  assert!(quad.y + quad.height <= y + height, "{label}: inside the box");
  ((x, y, width, height), rect(&quad))
}

/// Asserts that `what` stands on the line of `value`, inside `boxed`: its middle is the value's, within a pixel and
/// a half.
fn on_the_line(what: Rect, value: Rect, boxed: Rect, name: &str) {
  assert!(
    (middle(what) - middle(value)).abs() < 1.5,
    "{name} on the value's line: its middle {} down the box, the value's {}",
    middle(what) - boxed.1,
    middle(value) - boxed.1
  );
  assert!(
    what.0 >= boxed.0 - 0.01
      && what.0 + what.2 <= boxed.0 + boxed.2 + 0.01
      && what.1 >= boxed.1
      && what.1 + what.3 <= boxed.1 + boxed.3 + 0.01,
    "{name} inside the box: {what:?} in {boxed:?}"
  );
}

/// Asserts that `what` is on the value's line as [`on_the_line`] has it, and wholly under the label's line.
fn on_the_line_under(what: Rect, value: Rect, label: Rect, boxed: Rect, name: &str) {
  on_the_line(what, value, boxed, name);
  assert!(
    what.1 >= label.1 + label.3 - 0.51,
    "{name} under the label: {} down the box, the label ends {}",
    what.1 - boxed.1,
    label.1 + label.3 - boxed.1
  );
}

/// Asserts that a value of one line stands where an input's does: 29 under the top of the box, under the label.
fn the_value_is_under_the_label(value: Rect, label: Rect, boxed: Rect, name: &str) {
  assert!(
    (middle(value) - (boxed.1 + 37.5)).abs() < 2.0,
    "{name}: the value's line is 29 under the top, 10 over the bottom; its middle is {} down",
    middle(value) - boxed.1
  );
  assert!(value.1 >= label.1 + label.3, "{name}: under the label");
}

#[test]
fn a_fluid_password_input_has_its_eye_on_the_line_of_the_password() {
  let mut stage = mounted(|ctx| {
    Field::new(
      "Password",
      PasswordInput::new(Signal::new("hunter2".to_owned()))
        .bind_reveal(Signal::new(true))
        .icon(icons::LOCK)
        .id("box"),
    )
    .fluid(true)
    .mount(ctx)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Password");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("hunter2").expect("the password, shown"));
  the_value_is_under_the_label(value, label, boxed, "the password");
  let (eye, lock) = (rect(&glyph(&stage, icons::EYE_OFF)), rect(&glyph(&stage, icons::LOCK)));
  on_the_line_under(eye, value, label, boxed, "the eye");
  on_the_line_under(lock, value, label, boxed, "the icon");
  assert!(
    (eye.0 + eye.2 - (boxed.0 + boxed.2 - 12.0)).abs() < 0.51,
    "12 from the end"
  );
  assert!(
    lock.0 + lock.2 <= value.0 && value.0 + value.2 <= eye.0,
    "the icon, the password, the eye"
  );
}

#[test]
fn a_fluid_search_field_has_its_icon_its_clear_button_and_its_hint_on_the_line_of_the_query() {
  let mut stage =
    staged(|| Field::new("Search", SearchField::new(Signal::new("grid".to_owned())).id("box")).fluid(true));
  let (boxed, label) = the_fluid_box(&mut stage, "Search");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("grid").expect("the query"));
  the_value_is_under_the_label(value, label, boxed, "the query");
  let (icon, clear) = (
    rect(&glyph(&stage, icons::SEARCH)),
    rect(&glyph(&stage, icons::CLOSE_CIRCLE)),
  );
  on_the_line_under(icon, value, label, boxed, "the search icon");
  on_the_line_under(clear, value, label, boxed, "the clear button");
  assert_eq!(icon.0, label.0, "the icon starts where the label does");
  // Empty, the hint of its key is where the clear button was.
  let mut stage = staged(|| {
    Field::new(
      "Search",
      SearchField::new(Signal::new(String::new()))
        .placeholder("Search components")
        .shortcut("/")
        .id("box"),
    )
    .fluid(true)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Search");
  assert_eq!(boxed.3, 56.0);
  let placeholder = rect(&stage.text_quad("Search components").expect("the placeholder"));
  let hint = rect(&stage.text_quad("/").expect("the hint"));
  on_the_line_under(hint, placeholder, label, boxed, "the hint");
}

#[test]
fn a_fluid_number_input_has_its_buttons_on_the_line_of_the_number() {
  let mut stage = staged(|| Field::new("Quantity", NumberInput::new(Signal::new(12.0)).id("box")).fluid(true));
  let (boxed, label) = the_fluid_box(&mut stage, "Quantity");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("12").expect("the number"));
  the_value_is_under_the_label(value, label, boxed, "the number");
  let (less, more) = (stage.bounds("box-decrease"), stage.bounds("box-increase"));
  on_the_line_under(less, value, label, boxed, "the button that steps down");
  on_the_line_under(more, value, label, boxed, "the button that steps up");
  on_the_line(rect(&glyph(&stage, icons::MINUS)), value, boxed, "the minus");
  on_the_line(rect(&glyph(&stage, icons::PLUS)), value, boxed, "the plus");
  assert!(
    less.0 + less.2 <= value.0 && value.0 + value.2 <= more.0,
    "the number between them"
  );
  // A press of a button steps the number.
  let quantity = Signal::new(12.0);
  let bound = quantity.clone();
  let mut stage = staged(move || Field::new("Quantity", NumberInput::new(bound.clone()).id("box")).fluid(true));
  stage.click("box-increase");
  assert_eq!(quantity.get(), 13.0);
  // One over the other at the end, the buttons are about the number's line.
  let mut stage = staged(|| {
    Field::new(
      "Quantity",
      NumberInput::new(Signal::new(12.0))
        .layout(NumberInputLayout::Stacked)
        .id("box"),
    )
    .fluid(true)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Quantity");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("12").expect("the number"));
  the_value_is_under_the_label(value, label, boxed, "the number at the start");
  assert!((value.0 - (boxed.0 + 12.0)).abs() < 1.0, "12 in, under the label");
  let (less, more) = (stage.bounds("box-decrease"), stage.bounds("box-increase"));
  let both = (more.0, more.1, more.2, less.1 + less.3 - more.1);
  on_the_line(both, value, boxed, "the two buttons");
  assert!(more.3 >= 16.0 && less.3 >= 16.0, "each has room for its chevron");
}

/// The frameworks a combobox chooses from.
fn frameworks() -> Vec<(String, &'static str)> {
  vec![
    ("lurq".to_owned(), "lurq"),
    ("gpui".to_owned(), "GPUI"),
    ("slint".to_owned(), "Slint"),
  ]
}

#[test]
fn a_fluid_combobox_has_what_is_typed_and_its_chevrons_under_the_label() {
  let mut stage = staged(|| {
    Field::new(
      "Framework",
      Combobox::new(Signal::new("gpui".to_owned()), frameworks())
        .clearable(true)
        .id("box"),
    )
    .fluid(true)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Framework");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("GPUI").expect("what is chosen, in the input"));
  the_value_is_under_the_label(value, label, boxed, "what is chosen");
  assert!((value.0 - (boxed.0 + 12.0)).abs() < 1.0, "12 in");
  on_the_line_under(
    rect(&glyph(&stage, icons::CHEVRONS_VERTICAL)),
    value,
    label,
    boxed,
    "the chevrons",
  );
  on_the_line_under(rect(&glyph(&stage, icons::CLOSE)), value, label, boxed, "the cross");
  // A press of the label, which lies over the box, puts the caret in it and opens the list.
  press(&mut stage, boxed.0 + 20.0, boxed.1 + 14.0);
  assert_eq!(focused(&stage).as_deref(), Some("box-input"));
  assert!(stage.paints("Slint"), "the list is open");
}

/// The languages a multi select chooses from.
fn languages() -> Vec<(String, &'static str)> {
  vec![
    ("rust".to_owned(), "Rust"),
    ("wgsl".to_owned(), "WGSL"),
    ("typescript".to_owned(), "TypeScript"),
    ("swift".to_owned(), "Swift"),
    ("kotlin".to_owned(), "Kotlin"),
  ]
}

fn chosen(values: &[&str]) -> Signal<Vec<String>> {
  Signal::new(values.iter().map(|value| (*value).to_owned()).collect())
}

#[test]
fn a_fluid_multi_select_is_56_with_a_line_of_tags_and_grows_by_its_lines() {
  let languages_in = |values: &'static [&'static str]| {
    staged(move || {
      Field::new(
        "Languages",
        MultiSelect::new(chosen(values), languages())
          .placeholder("Select languages")
          .id("box"),
      )
      .fluid(true)
    })
  };
  // With nothing chosen it is the 56 of any fluid box, its placeholder under the label.
  let mut stage = languages_in(&[]);
  let (boxed, label) = the_fluid_box(&mut stage, "Languages");
  assert_eq!(boxed.3, 56.0);
  let placeholder = rect(&stage.text_quad("Select languages").expect("the placeholder"));
  the_value_is_under_the_label(placeholder, label, boxed, "the placeholder");
  assert!((placeholder.0 - (boxed.0 + 12.0)).abs() < 1.0, "12 in");
  on_the_line_under(
    rect(&glyph(&stage, icons::CHEVRON_DOWN)),
    placeholder,
    label,
    boxed,
    "the chevron",
  );
  // A tag is 24 high, more than a line of text: the box is padded so that a line of tags is 56 too.
  let mut stage = languages_in(&["rust", "wgsl"]);
  let (boxed, label) = the_fluid_box(&mut stage, "Languages");
  assert_eq!(boxed.3, 56.0, "one line of tags");
  let (rust, wgsl) = (
    rect(&stage.text_quad("Rust").expect("a tag")),
    rect(&stage.text_quad("WGSL").expect("a tag")),
  );
  the_value_is_under_the_label(rust, label, boxed, "a tag");
  on_the_line(wgsl, rust, boxed, "the second tag");
  on_the_line_under(
    rect(&glyph(&stage, icons::CHEVRON_DOWN)),
    rust,
    label,
    boxed,
    "the chevron",
  );
  // Tags that go on to more lines make the box higher by a tag and the 6 between lines for each: no tag leaves it.
  let mut stage = languages_in(&["rust", "wgsl", "typescript", "swift", "kotlin"]);
  let (boxed, label) = the_fluid_box(&mut stage, "Languages");
  let mut lines: Vec<f32> = Vec::new();
  for tag in ["Rust", "WGSL", "TypeScript", "Swift", "Kotlin"] {
    let tag = rect(&stage.text_quad(tag).expect("a tag"));
    assert!(
      tag.1 >= label.1 + label.3 && tag.1 + tag.3 <= boxed.1 + boxed.3 - 6.0,
      "a tag is under the label and inside the box: {tag:?}"
    );
    if !lines.iter().any(|line| (line - tag.1).abs() < 1.0) {
      lines.push(tag.1);
    }
  }
  assert!(lines.len() >= 2, "five tags do not fit on one line of 240");
  assert!(
    (boxed.3 - (56.0 + 30.0 * (lines.len() - 1) as f32)).abs() < 0.51,
    "{} lines of tags: {}",
    lines.len(),
    boxed.3
  );
}

fn day(month: u8, day: u8) -> Date {
  Date::new(2026, month, day).unwrap()
}

#[test]
fn a_fluid_date_field_has_its_segments_and_its_glyph_under_the_label() {
  let mut stage = mounted(|ctx| {
    Field::new("Due date", DateField::new(Signal::new(day(11, 12))).id("box"))
      .fluid(true)
      .mount(ctx)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Due date");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("2026").expect("the year"));
  the_value_is_under_the_label(value, label, boxed, "the year");
  for segment in ["box-day", "box-month", "box-year"] {
    on_the_line_under(stage.bounds(segment), value, label, boxed, segment);
  }
  on_the_line_under(stage.bounds("box-icon"), value, label, boxed, "the calendar's glyph");
  let first = ["box-day", "box-month", "box-year"]
    .map(|segment| stage.bounds(segment).0)
    .into_iter()
    .fold(f32::MAX, f32::min);
  // The first segment is padded 3: its text starts where the label does.
  assert_eq!(first + 3.0, label.0);
  // A press of a segment gives it the focus, and a digit is typed in it.
  stage.click("box-day");
  assert_eq!(focused(&stage).as_deref(), Some("box-day"));
}

#[test]
fn a_fluid_time_field_has_its_segments_and_its_glyph_under_the_label() {
  let mut stage = mounted(|ctx| {
    Field::new(
      "Starts at",
      TimeField::new(Signal::new(Time::new(14, 2, 0).unwrap())).id("box"),
    )
    .fluid(true)
    .mount(ctx)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Starts at");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("14").expect("the hours"));
  the_value_is_under_the_label(value, label, boxed, "the hours");
  for part in ["box-hour", "box-minute", "box-icon"] {
    on_the_line_under(stage.bounds(part), value, label, boxed, part);
  }
  assert_eq!(stage.bounds("box-hour").0 + 3.0, label.0);
}

#[test]
fn a_fluid_date_picker_has_its_buttons_on_the_line_of_the_date() {
  let date = Signal::new(Some(day(11, 12)));
  let bound = date.clone();
  let mut stage = mounted(move |ctx| {
    Field::new(
      "Due date",
      DatePicker::new(bound.clone())
        .today(day(11, 9))
        .clearable(true)
        .id("box"),
    )
    .fluid(true)
    .mount(ctx)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Due date");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("Nov 12, 2026").expect("the date"));
  the_value_is_under_the_label(value, label, boxed, "the date");
  let (trigger, clear) = (stage.bounds("box-trigger"), stage.bounds("box-clear"));
  on_the_line_under(trigger, value, label, boxed, "the button that opens the calendar");
  on_the_line_under(clear, value, label, boxed, "the button that empties it");
  assert!(
    trigger.0 + trigger.2 <= value.0 + 4.0 && value.0 + value.2 <= clear.0,
    "the button, the date, the cross"
  );
  // The cross empties it, and the button opens the calendar under the box.
  stage.click("box-clear");
  assert_eq!(date.get(), None);
  stage.click("box-trigger");
  stage.pass();
  assert!(stage.paints("November 2026"), "the calendar is open");
}

#[test]
fn a_fluid_date_range_picker_has_its_button_on_the_line_of_the_range() {
  let mut stage = mounted(|ctx| {
    Field::new(
      "Sprint",
      DateRangePicker::new(Signal::new(Some(DateRange::new(day(11, 9), day(11, 15)))))
        .today(day(11, 9))
        .id("box"),
    )
    .fluid(true)
    .mount(ctx)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Sprint");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("Nov 9 – Nov 15, 2026").expect("the range"));
  the_value_is_under_the_label(value, label, boxed, "the range");
  on_the_line_under(
    stage.bounds("box-trigger"),
    value,
    label,
    boxed,
    "the button that opens the calendar",
  );
}

#[test]
fn a_fluid_time_picker_is_as_wide_as_the_field_with_its_button_on_the_line_of_the_time() {
  let mut stage = mounted(|ctx| {
    Field::new(
      "Starts at",
      TimePicker::new(Signal::new(Time::new(9, 30, 0).unwrap())).id("box"),
    )
    .fluid(true)
    .mount(ctx)
  });
  // 160 wide on its own, it fills the fluid field.
  let (boxed, label) = the_fluid_box(&mut stage, "Starts at");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("09:30").expect("the time"));
  the_value_is_under_the_label(value, label, boxed, "the time");
  let trigger = stage.bounds("box-trigger");
  on_the_line_under(trigger, value, label, boxed, "the button that opens the panel");
  stage.click("box-trigger");
  stage.pass();
  assert!(
    stage.tree.get_element_by_id_mut("box-panel").is_some(),
    "the panel is open"
  );
}

#[test]
fn a_month_picker_and_a_year_picker_have_no_fluid_form() {
  // A panel of a grid is not a box that holds a value: a fluid field draws it as any field, its label above it.
  assert!(!MonthPicker::new(Signal::new(YearMonth::new(2026, 11).unwrap())).has_fluid_form());
  assert!(!YearPicker::new(Signal::new(2026)).has_fluid_form());
  let scenes: [(&str, fn(&mut Ctx) -> Element); 2] = [
    ("a month picker", |ctx| {
      Field::new(
        "Billing month",
        MonthPicker::new(Signal::new(YearMonth::new(2026, 11).unwrap())).id("box"),
      )
      .fluid(true)
      .mount(ctx)
    }),
    ("a year picker", |ctx| {
      Field::new("Billing month", YearPicker::new(Signal::new(2026)).id("box"))
        .fluid(true)
        .mount(ctx)
    }),
  ];
  for (name, scene) in scenes {
    let mut stage = mounted(scene);
    let label = stage.text_quad("Billing month").expect("the label");
    let (_, top, ..) = stage.bounds("box");
    assert!(label.y + label.height <= top, "{name}: the label is above the panel");
    assert_eq!(stage.text("Billing month").1, 13.0, "{name}: in a field label's type");
  }
}

/// The fills of the tokens painted in `color`, left to right.
fn tokens(stage: &Stage, color: Color) -> Vec<Rect> {
  let mut fills: Vec<Rect> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && quad.border_radius.is_some()
        && quad.height <= 30.0
    })
    .map(|quad| rect(&quad))
    .collect();
  fills.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
  fills
}

#[test]
fn a_fluid_token_input_is_56_on_one_line_and_as_high_as_its_rows() {
  let tags = |rows: Option<(usize, usize)>| {
    staged(move || {
      let input = TokenInput::new(Signal::new("rust wgpu ".to_owned()))
        .find_tokens(|text| Token::separated(text, ' '))
        .id("box");
      Field::new(
        "Tags",
        match rows {
          Some((least, most)) => input.rows(least, most),
          None => input,
        },
      )
      .fluid(true)
    })
  };
  let mut stage = tags(None);
  let (boxed, label) = the_fluid_box(&mut stage, "Tags");
  assert_eq!(boxed.3, 56.0);
  // A token is higher than the line of the text: it stands in the middle of that line, under the label.
  let line = (boxed.0, boxed.1 + 29.0, boxed.2, 17.0);
  let fills = tokens(&stage, light(PaletteColor::Secondary));
  assert_eq!(fills.len(), 2, "two tokens");
  for token in &fills {
    on_the_line_under(*token, line, label, boxed, "a token");
  }
  assert!(
    (fills[0].0 - (boxed.0 + 12.0)).abs() < 0.51,
    "12 in, where the label starts"
  );
  // On rows it is as high as its rows, of 28, with the 29 over them and the 10 under: more than 56.
  let mut stage = tags(Some((2, 4)));
  let (boxed, label) = the_fluid_box(&mut stage, "Tags");
  assert!(
    (boxed.3 - (29.0 + 2.0 * 28.0 + 10.0)).abs() < 1.5,
    "two rows under the label: {}",
    boxed.3
  );
  for token in tokens(&stage, light(PaletteColor::Secondary)) {
    assert!(
      token.1 >= label.1 + label.3 && token.1 + token.3 <= boxed.1 + boxed.3,
      "a token is under the label, in the box: {token:?}"
    );
  }
}

#[test]
fn a_fluid_token_input_whose_tokens_wrap_grows_and_cuts_none_of_them() {
  let words = "rust wgpu taffy winit cosmic glyphon serde tokio rayon ";
  let mut stage = staged(move || {
    Field::new(
      "Tags",
      TokenInput::new(Signal::new(words.to_owned()))
        .find_tokens(|text| Token::separated(text, ' '))
        .rows(1, 8)
        .id("box"),
    )
    .fluid(true)
  });
  let (boxed, label) = the_fluid_box(&mut stage, "Tags");
  let fills = tokens(&stage, light(PaletteColor::Secondary));
  assert_eq!(fills.len(), 9, "every token is painted");
  let lines = {
    let mut tops: Vec<f32> = fills.iter().map(|token| token.1.round()).collect();
    tops.dedup();
    tops.len()
  };
  assert!(lines >= 3, "the tokens wrap onto {lines} lines");
  assert!(
    boxed.3 >= 29.0 + lines as f32 * 28.0 + 10.0 - 1.5,
    "the box grows by its lines: {} for {lines}",
    boxed.3
  );
  for token in &fills {
    assert!(
      token.0 >= boxed.0
        && token.0 + token.2 <= boxed.0 + boxed.2
        && token.1 >= label.1 + label.3
        && token.1 + token.3 <= boxed.1 + boxed.3,
      "a token is whole in the box, under the label: {token:?} in {boxed:?}"
    );
  }
}

#[test]
fn a_fluid_mentions_field_has_its_rows_under_the_label() {
  let mut stage = staged(|| {
    Field::new(
      "Comment",
      Mentions::new(Signal::new("Ship it.".to_owned()), &MentionsController::new())
        .trigger('@', vec![Mention::new("Ivan Zaida")])
        .id("box"),
    )
    .fluid(true)
  });
  // It is a token input on two rows of 21: as high as they are with the 29 over them and the 10 under, more than 56.
  let (boxed, label) = the_fluid_box(&mut stage, "Comment");
  assert!(
    (boxed.3 - (29.0 + 2.0 * 21.0 + 10.0)).abs() < 1.5,
    "two rows under the label: {}",
    boxed.3
  );
  let value = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("Ship it.")))
    .expect("the text");
  assert!((value.x - (boxed.0 + 12.0)).abs() < 1.0, "12 in");
  assert!((value.y - (boxed.1 + 29.0)).abs() < 3.0, "{}", value.y - boxed.1);
  assert!(value.y >= label.1 + label.3, "under the label");
}

#[test]
fn a_fluid_color_field_has_its_swatch_and_its_eyedropper_on_the_line_of_the_hex() {
  // The field keeps whether its picker is open by the signal it is bound to: one signal for every render.
  let accent = Signal::new(Color::from_hex("#2563EB"));
  let mut stage = staged(move || Field::new("Accent", ColorField::new(accent.clone()).id("box")).fluid(true));
  let (boxed, label) = the_fluid_box(&mut stage, "Accent");
  assert_eq!(boxed.3, 56.0);
  let value = rect(&stage.text_quad("#2563EB").expect("the hex"));
  the_value_is_under_the_label(value, label, boxed, "the hex");
  let swatch = stage.bounds("box-swatch");
  on_the_line_under(swatch, value, label, boxed, "the swatch");
  assert!(
    (value.0 - (swatch.0 + swatch.2 + 8.0)).abs() < 0.51,
    "the hex 8 after the swatch"
  );
  // The eyedropper is there where the platform picks colors from the screen.
  if lurq::eyedropper::EyeDropper::is_supported() {
    on_the_line_under(stage.bounds("box-trigger"), value, label, boxed, "the eyedropper");
  }
  // A press of the swatch opens the picker under the box, whose line is then in the ring's color.
  stage.click("box-swatch");
  stage.pass();
  assert!(
    stage.tree.get_element_by_id_mut("box-picker-area").is_some(),
    "the picker is open"
  );
  assert_eq!(
    sides(&mut stage, "box"),
    ([0.0, 0.0, 1.0, 0.0], light(PaletteColor::Ring))
  );
}

#[test]
fn a_control_the_field_mounts_is_the_box_it_is_when_it_is_converted() {
  // The same field built as an element and mounted: the control in it has one form.
  let boxes = |scene: fn(Option<&mut Ctx>) -> Element| {
    let mut still = mounted(move |_| scene(None));
    let mut kept = mounted(move |ctx| scene(Some(ctx)));
    (still.bounds("box"), kept.bounds("box"))
  };
  fn built(field: Field, ctx: Option<&mut Ctx>) -> Element {
    let field = field.fluid(true);
    match ctx {
      Some(ctx) => field.mount(ctx),
      None => field.into(),
    }
  }
  let scenes: [(&str, fn(Option<&mut Ctx>) -> Element); 6] = [
    ("a password input", |ctx| {
      built(
        Field::new("It", PasswordInput::new(Signal::new(String::new())).id("box")),
        ctx,
      )
    }),
    ("a date field", |ctx| {
      built(
        Field::new("It", DateField::new(Signal::new(day(11, 12))).id("box")),
        ctx,
      )
    }),
    ("a time field", |ctx| {
      built(
        Field::new(
          "It",
          TimeField::new(Signal::new(Time::new(9, 30, 0).unwrap())).id("box"),
        ),
        ctx,
      )
    }),
    ("a date picker", |ctx| {
      built(
        Field::new("It", DatePicker::new(Signal::new(Some(day(11, 12)))).id("box")),
        ctx,
      )
    }),
    ("a date range picker", |ctx| {
      built(Field::new("It", DateRangePicker::new(Signal::new(None)).id("box")), ctx)
    }),
    ("a time picker", |ctx| {
      built(
        Field::new(
          "It",
          TimePicker::new(Signal::new(Time::new(9, 30, 0).unwrap())).id("box"),
        ),
        ctx,
      )
    }),
  ];
  for (name, scene) in scenes {
    let (still, kept) = boxes(scene);
    assert_eq!(still, kept, "{name}");
    assert_eq!((kept.0, kept.1, kept.2), (8.0, 8.0, 240.0), "{name} fills the field");
  }
}

#[test]
fn every_fluid_box_fills_a_wide_field_and_is_no_wider_than_a_narrow_room() {
  let scenes: [(&str, fn() -> Field); 13] = [
    ("a password input", || {
      Field::new("It", PasswordInput::new(Signal::new(String::new())).id("box"))
    }),
    ("a search field", || {
      Field::new("It", SearchField::new(Signal::new(String::new())).id("box"))
    }),
    ("a number input", || {
      Field::new("It", NumberInput::new(Signal::new(1.0)).id("box"))
    }),
    ("a combobox", || {
      Field::new(
        "It",
        Combobox::new(Signal::new("gpui".to_owned()), frameworks()).id("box"),
      )
    }),
    ("a multi select", || {
      Field::new("It", MultiSelect::new(chosen(&["rust"]), languages()).id("box"))
    }),
    ("a date field", || {
      Field::new("It", DateField::new(Signal::new(day(11, 12))).id("box"))
    }),
    ("a time field", || {
      Field::new(
        "It",
        TimeField::new(Signal::new(Time::new(9, 30, 0).unwrap())).id("box"),
      )
    }),
    ("a date picker", || {
      Field::new("It", DatePicker::new(Signal::new(Some(day(11, 12)))).id("box"))
    }),
    ("a date range picker", || {
      Field::new("It", DateRangePicker::new(Signal::new(None)).id("box"))
    }),
    ("a time picker", || {
      Field::new(
        "It",
        TimePicker::new(Signal::new(Time::new(9, 30, 0).unwrap())).id("box"),
      )
    }),
    ("a token input", || {
      Field::new("It", TokenInput::new(Signal::new(String::new())).id("box"))
    }),
    ("a mentions field", || {
      Field::new(
        "It",
        Mentions::new(Signal::new(String::new()), &MentionsController::new()).id("box"),
      )
    }),
    ("a color field", || {
      Field::new("It", ColorField::new(Signal::new(Color::from_hex("#2563EB"))).id("box"))
    }),
  ];
  for (name, scene) in scenes {
    let mut stage = staged(move || scene().fluid(true).full_width(true));
    assert_eq!(stage.bounds("box").2, 384.0, "{name} fills its room");
    let mut stage = Stage::new(move |_| {
      Column::new()
        .size(400.0, 300.0)
        .child(
          Row::new()
            .id("room")
            .width(200.0)
            .child(scene().fluid(true).id("field")),
        )
        .into()
    });
    assert_eq!(stage.bounds("field").2, 200.0, "{name}: the field in a room of 200");
    assert_eq!(stage.bounds("box").2, 200.0, "{name} in a room of 200");
  }
}
