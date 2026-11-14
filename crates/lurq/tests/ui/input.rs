//! An input and a field: the box, its text and its placeholder, what typing
//! does to the signal, the edge in each state, and the field's label, marks
//! and description around any control.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::Quad,
  node::color::Color,
  ui::{
    Case, Field, Input, InputPart, InputRenderer, InputSize, InputState, InputView, Interaction, Opacity, Switch, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The width and the color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad: Quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
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
      && quad.x >= x
      && quad.y >= y
      && quad.x + quad.width <= x + width
      && quad.y + quad.height <= y + height
  })
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

#[test]
fn an_input_is_a_box_with_its_placeholder_or_its_value() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        Input::new(Signal::new(String::new()))
          .placeholder("Placeholder")
          .id("empty"),
      )
      .child(Input::new(Signal::new("Ivan Zaida".to_owned())).id("filled"))
      .into()
  });
  for id in ["empty", "filled"] {
    let (_, _, width, height) = stage.bounds(id);
    assert_eq!((width, height), (260.0, 36.0), "{id}");
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Background)), "{id}");
    assert_eq!(edge(&mut stage, id), (1.0, light(PaletteColor::Input)), "{id}");
    assert!(
      stage
        .border(id)
        .unwrap()
        .border_radius
        .is_some_and(|radius| radius.top_left == 6.0),
      "{id}: 6 round"
    );
  }
  assert_eq!(stage.text("Placeholder"), (light(PaletteColor::MutedForeground), 14.0));
  assert_eq!(stage.text("Ivan Zaida"), (light(PaletteColor::Foreground), 14.0));
  // The text is 12 from the start of the box.
  let (x, ..) = stage.bounds("filled");
  let value = stage.text_quad("Ivan Zaida").unwrap();
  assert!((value.x - x - 12.0).abs() < 1.0, "12 in: {}", value.x - x);
}

#[test]
fn the_sizes_are_the_heights_of_the_controls_with_their_type() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Input::new(Signal::new("Small".to_owned())).size(InputSize::Sm).id("sm"))
      .child(Input::new(Signal::new("Medium".to_owned())).id("md"))
      .child(Input::new(Signal::new("Large".to_owned())).size(InputSize::Lg).id("lg"))
      .into()
  });
  for (id, text, height, size) in [
    ("sm", "Small", 32.0, 13.0),
    ("md", "Medium", 36.0, 14.0),
    ("lg", "Large", 40.0, 15.0),
  ] {
    assert_eq!(stage.bounds(id).3, height, "{id}");
    assert_eq!(stage.text(text).1, size, "{id}");
  }
}

#[test]
fn what_is_typed_goes_into_the_signal_and_the_focus_thickens_the_edge_in_the_ring_color() {
  let name = Signal::new(String::new());
  let typed = Arc::new(Mutex::new(Vec::new()));
  let (bound, told) = (name.clone(), typed.clone());
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(
        Input::new(bound.clone())
          .placeholder("Display name")
          .id("name")
          .on_input(move |event| told.lock().unwrap().push(event.new_value.clone())),
      )
      .into()
  });
  // Tab reaches it, and what is typed then is the signal's.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("name"));
  assert!(ring_inside(&mut stage, "name", light(PaletteColor::Ring)));
  type_in(&mut stage, "Ivan");
  assert_eq!(name.get(), "Ivan");
  assert_eq!(*typed.lock().unwrap(), ["Ivan"]);
  assert_eq!(stage.text("Ivan").0, light(PaletteColor::Foreground));

  // What the app sets on the signal is what the input shows.
  name.set("Ivan Zaida".to_owned());
  stage.pass();
  assert!(stage.paints("Ivan Zaida"));
}

#[test]
fn a_click_focuses_it_with_the_thick_edge_too() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(Input::new(Signal::new(String::new())).id("name"))
      .into()
  });
  stage.click("name");
  assert_eq!(focused(&stage).as_deref(), Some("name"));
  assert!(ring_inside(&mut stage, "name", light(PaletteColor::Ring)));
}

#[test]
fn under_the_pointer_the_edge_is_in_the_ring_color() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(Input::new(Signal::new(String::new())).id("name"))
      .into()
  });
  stage.hover("name");
  assert_eq!(edge(&mut stage, "name"), (1.0, light(PaletteColor::Ring)));
}

#[test]
fn a_judgment_colors_the_edge_at_rest_and_in_the_focus() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        Input::new(Signal::new("not-an-email".to_owned()))
          .invalid(true)
          .id("invalid"),
      )
      .child(Input::new(Signal::new("80".to_owned())).warning(true).id("warning"))
      .child(
        Input::new(Signal::new("ivan@lurq.dev".to_owned()))
          .valid(true)
          .id("valid"),
      )
      .child(
        Input::new(Signal::new("both".to_owned()))
          .invalid(true)
          .valid(true)
          .id("both"),
      )
      .into()
  });
  for (id, role) in [
    ("invalid", PaletteColor::Destructive),
    ("warning", PaletteColor::Warning),
    ("valid", PaletteColor::Success),
    ("both", PaletteColor::Destructive),
  ] {
    assert_eq!(edge(&mut stage, id), (1.0, light(role)), "{id}");
  }
  stage.click("invalid");
  assert_eq!(
    ring_inside(&mut stage, "invalid", light(PaletteColor::Destructive)),
    true,
    "the focus keeps the color"
  );
}

#[test]
fn a_forced_focus_draws_the_thick_edge_with_nothing_focused() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Input::new(Signal::new(String::new()))
          .force(Some(Interaction::FocusVisible))
          .id("name"),
      )
      .into()
  });
  assert_eq!(focused(&stage), None);
  assert!(ring_inside(&mut stage, "name", light(PaletteColor::Ring)));
}

#[test]
fn a_disabled_input_takes_nothing_and_tab_passes_it_by() {
  let value = Signal::new("Read only".to_owned());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Input::new(bound.clone()).disabled(true).id("disabled"))
      .child(Input::new(Signal::new(String::new())).id("next"))
      .into()
  });
  assert_eq!(stage.fill_color("disabled"), Some(light(PaletteColor::Muted)));
  stage.click("disabled");
  assert_eq!(focused(&stage), None, "a press does not focus it");
  type_in(&mut stage, "x");
  assert_eq!(value.get(), "Read only");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("next"));

  // It is dimmed by the theme's opacity for what is disabled.
  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<Input>(&Case::new().state(InputState::Disabled, true), None)
    .part(InputPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn a_read_only_input_is_focused_and_refuses_every_edit() {
  let value = Signal::new("8770b598".to_owned());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(Input::new(bound.clone()).read_only(true).id("token"))
      .into()
  });
  assert_eq!(stage.fill_color("token"), Some(light(PaletteColor::Muted)));
  assert_eq!(edge(&mut stage, "token"), (1.0, light(PaletteColor::Border)));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("token"));
  type_in(&mut stage, "x");
  assert_eq!(value.get(), "8770b598");
}

#[test]
fn an_icon_starts_the_box_and_the_text_follows_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Input::new(Signal::new(String::new()))
          .placeholder("Search components")
          .icon(icons::SEARCH)
          .id("search"),
      )
      .into()
  });
  // 12 of padding, an icon of 16 and 8 after it.
  let (x, ..) = stage.bounds("search");
  let text = stage.text_quad("Search components").unwrap();
  assert!((text.x - x - 36.0).abs() < 1.0, "36 in: {}", text.x - x);
  // The icon lets the pointer through: a press over it focuses the box.
  let (_, y, _, height) = stage.bounds("search");
  let (px, py) = (x + 20.0, y + height / 2.0);
  stage.tree.mouse_move(px, py);
  stage.tree.mouse_down(px, py, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(px, py, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("search"));
}

#[test]
fn a_full_width_input_is_as_wide_as_what_holds_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(320.0)
      .child(Input::new(Signal::new(String::new())).full_width(true).id("wide"))
      .into()
  });
  assert_eq!(stage.bounds("wide").2, 320.0);
}

#[test]
fn a_theme_gives_the_input_another_anatomy_and_it_still_types() {
  let app = App::new();
  // The control alone, without the icon: other nodes, the same behavior.
  let renderer: InputRenderer = Arc::new(|view: InputView| view.control().into());
  app.theme().set_renderer::<Input>(renderer);
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Row::new()
      .padding(8.0)
      .child(Input::new(bound.clone()).icon(icons::SEARCH).id("bare"))
      .into()
  });
  assert_eq!(
    stage.fill_color("bare"),
    Some(light(PaletteColor::Background)),
    "the recipe still styles it"
  );
  stage.click("bare");
  type_in(&mut stage, "lurq");
  assert_eq!(value.get(), "lurq");
}

#[test]
fn a_field_is_its_label_its_control_and_its_description_6_apart() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Field::new(
          "Display name",
          Input::new(Signal::new(String::new())).full_width(true).id("name"),
        )
        .description("Shown in the title bar.")
        .id("field"),
      )
      .into()
  });
  assert_eq!(stage.bounds("field").2, 260.0);
  assert_eq!(stage.bounds("name").2, 260.0, "the input fills the field");
  let label = stage.text_quad("Display name").unwrap();
  let description = stage.text_quad("Shown in the title bar.").unwrap();
  let (_, y, _, height) = stage.bounds("name");
  assert!((y - (label.y + label.height) - 6.0).abs() < 1.0, "label to control");
  assert!(
    (description.y - (y + height) - 6.0).abs() < 1.0,
    "control to description"
  );
  assert_eq!(stage.text("Display name"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(
    stage.text("Shown in the title bar."),
    (light(PaletteColor::MutedForeground), 12.0)
  );
}

#[test]
fn an_error_takes_the_place_of_the_description_in_the_destructive_color() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(
        Field::new("Email", Input::new(Signal::new("ivan@".to_owned())).invalid(true))
          .description("Where receipts go.")
          .error("Enter a full email address."),
      )
      .child(
        Field::new("Port", Input::new(Signal::new("80".to_owned())).warning(true))
          .description("Ports below 1024 need admin rights.")
          .warning(true),
      )
      .child(
        Field::new("Username", Input::new(Signal::new("ivanzaida".to_owned())).valid(true))
          .description("This name is available.")
          .valid(true),
      )
      .into()
  });
  assert!(!stage.paints("Where receipts go."));
  assert_eq!(
    stage.text("Enter a full email address.").0,
    light(PaletteColor::Destructive)
  );
  assert_eq!(
    stage.text("Ports below 1024 need admin rights.").0,
    light(PaletteColor::Warning)
  );
  assert_eq!(stage.text("This name is available.").0, light(PaletteColor::Success));
}

#[test]
fn a_required_field_has_its_mark_and_an_optional_one_says_so() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(Field::new("Display name", Input::new(Signal::new(String::new()))).required(true))
      .child(Field::new("Bio", Input::new(Signal::new(String::new()))).optional(true))
      .into()
  });
  assert_eq!(stage.text("*"), (light(PaletteColor::Destructive), 13.0));
  let (label, mark) = (stage.text_quad("Display name").unwrap(), stage.text_quad("*").unwrap());
  assert!(
    (mark.x - (label.x + label.width) - 4.0).abs() < 1.0,
    "4 after the label"
  );
  assert_eq!(stage.text("Optional"), (light(PaletteColor::MutedForeground), 11.0));
}

#[test]
fn a_horizontal_field_has_its_label_in_a_column_of_110_beside_the_control() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Field::new(
          "Time zone",
          Input::new(Signal::new("Europe/Kyiv".to_owned()))
            .full_width(true)
            .id("zone"),
        )
        .horizontal(true)
        .id("field"),
      )
      .into()
  });
  let (x, ..) = stage.bounds("field");
  let (control_x, control_y, _, height) = stage.bounds("zone");
  assert_eq!(control_x - x, 122.0, "110 of label and 12 after it");
  let label = stage.text_quad("Time zone").unwrap();
  let middle = label.y + label.height / 2.0;
  assert!(
    (middle - (control_y + height / 2.0)).abs() < 1.5,
    "the label is level with the control"
  );
}

#[test]
fn a_field_holds_any_control() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(
        Field::new(
          "Notifications",
          Switch::new("Pause notifications")
            .id("pause")
            .on_change(move |on| told.lock().unwrap().push(on)),
        )
        .description("Nothing is shown until this is turned off again."),
      )
      .into()
  });
  assert!(stage.paints("Notifications"));
  stage.click("pause");
  assert_eq!(*asked.lock().unwrap(), [true]);
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

#[test]
fn a_field_judges_its_input_without_the_app_telling_the_input() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(
        Field::new("Email", Input::new(Signal::new("ivan@".to_owned())).id("email"))
          .error("Enter a full email address."),
      )
      .child(Field::new("Port", Input::new(Signal::new("80".to_owned())).id("port")).warning(true))
      .child(Field::new("Username", Input::new(Signal::new("ivanzaida".to_owned())).id("user")).valid(true))
      .child(Field::new("Plain", Input::new(Signal::new(String::new())).id("plain")))
      .into()
  });
  assert_eq!(edge(&mut stage, "email"), (1.0, light(PaletteColor::Destructive)));
  assert_eq!(edge(&mut stage, "port"), (1.0, light(PaletteColor::Warning)));
  assert_eq!(edge(&mut stage, "user"), (1.0, light(PaletteColor::Success)));
  assert_eq!(edge(&mut stage, "plain"), (1.0, light(PaletteColor::Input)));
  stage.click("email");
  assert!(
    ring_inside(&mut stage, "email", light(PaletteColor::Destructive)),
    "its focus is in the field's judgment too"
  );
}

#[test]
fn an_input_the_app_judged_itself_keeps_its_judgment_in_a_field() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Field::new(
          "Username",
          Input::new(Signal::new("ivanzaida".to_owned())).valid(true).id("user"),
        )
        .error("Taken a moment ago."),
      )
      .into()
  });
  assert_eq!(edge(&mut stage, "user"), (1.0, light(PaletteColor::Success)));
  assert_eq!(stage.text("Taken a moment ago.").0, light(PaletteColor::Destructive));
}

#[test]
fn a_field_calls_its_input_by_its_label_unless_it_has_a_name() {
  let app = App::new();
  let names = Arc::new(Mutex::new(Vec::new()));
  let heard = names.clone();
  let renderer: InputRenderer = Arc::new(move |view: InputView| {
    heard.lock().unwrap().push(view.label.as_deref().map(str::to_owned));
    view.render()
  });
  app.theme().set_renderer::<Input>(renderer);
  let _stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(Field::new("Display name", Input::new(Signal::new(String::new()))))
      .child(Field::new(
        "Search",
        Input::new(Signal::new(String::new())).label("Search components"),
      ))
      .child(Input::new(Signal::new(String::new())))
      .into()
  });
  let names = names.lock().unwrap();
  assert_eq!(
    names[..3],
    [
      Some("Display name".to_owned()),
      Some("Search components".to_owned()),
      None
    ]
  );
}

#[test]
fn a_press_of_the_label_focuses_the_input_which_then_takes_what_is_typed() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    // The field is built again with every edit: its error follows the value.
    let short = bound.get().chars().count() < 3;
    let mut field = Field::new("Display name", Input::new(bound.clone()).id("name"));
    if short {
      field = field.error("At least three letters.");
    }
    Column::new().padding(8.0).child(field).into()
  });
  assert_eq!(focused(&stage), None);
  press_text(&mut stage, "Display name");
  assert_eq!(focused(&stage).as_deref(), Some("name"));
  type_in(&mut stage, "Iv");
  assert_eq!(value.get(), "Iv");
  assert_eq!(edge(&mut stage, "name").1, light(PaletteColor::Destructive));
  stage.tree.text_input_replace(2..2, "an");
  stage.pass();
  assert_eq!(
    value.get(),
    "Ivan",
    "the input kept the focus while the field was built again"
  );
  assert_eq!(focused(&stage).as_deref(), Some("name"));
  assert!(!stage.paints("At least three letters."));
}

#[test]
fn a_label_of_a_disabled_field_focuses_nothing() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(Field::new("Display name", Input::new(bound.clone()).id("name")).disabled(true))
      .child(Field::new(
        "Bio",
        Input::new(Signal::new(String::new())).disabled(true).id("bio"),
      ))
      .into()
  });
  press_text(&mut stage, "Display name");
  assert_eq!(focused(&stage), None, "the field's own control is disabled by it");
  press_text(&mut stage, "Bio");
  assert_eq!(
    focused(&stage),
    None,
    "a control the app disabled is not focused either"
  );
  assert_eq!(stage.fill_color("name"), Some(light(PaletteColor::Muted)));
}

#[test]
fn a_press_of_the_label_reaches_any_focusable_control() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(Field::new("Notifications", Switch::new("Pause notifications").id("pause")).horizontal(true))
      .into()
  });
  press_text(&mut stage, "Notifications");
  assert_eq!(focused(&stage).as_deref(), Some("pause"));
}

#[test]
fn a_tap_of_the_label_focuses_the_input_even_from_another_text_input() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(Input::new(Signal::new(String::new())).id("other"))
      .child(Field::new("Display name", Input::new(bound.clone()).id("name")))
      .into()
  });
  let tap = |stage: &mut Stage, x: f32, y: f32| {
    stage.tree.touch_tap(x, y, std::time::Duration::from_millis(100));
    stage.pass();
  };
  // A finger in another text input first: a tap elsewhere leaves it the focus, and the keyboard up, unless what is
  // tapped takes the focus.
  let (x, y) = stage.center("other");
  tap(&mut stage, x, y);
  assert_eq!(focused(&stage).as_deref(), Some("other"));
  let label = stage.text_quad("Display name").unwrap();
  tap(&mut stage, label.x + label.width / 2.0, label.y + label.height / 2.0);
  assert_eq!(focused(&stage).as_deref(), Some("name"));
  type_in(&mut stage, "Ivan");
  assert_eq!(value.get(), "Ivan");
}
