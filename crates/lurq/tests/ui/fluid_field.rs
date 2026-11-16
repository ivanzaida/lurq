//! A fluid field (`imRnW`): `Field::fluid`, whose label is inside the box of its control, for an input, a textarea
//! and a select.

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
  ui::{Field, Input, Interaction, Select, Switch, Textarea, icons},
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
