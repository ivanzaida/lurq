//! A checkbox: its box and its mark, what a press of it or of its label asks
//! for, its states, and a theme's other anatomy and rules.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, RadiusSize, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::QuadContent,
  node::color::Color,
  ui::{Case, Checkbox, CheckboxPart, CheckboxRenderer, CheckboxState, CheckboxView, Interaction, Opacity, icons},
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

/// The color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> Option<Color> {
  stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color)
}

#[test]
fn a_checkbox_is_a_box_of_16_with_a_check_or_a_dash_in_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Checkbox::new("Send crash reports").id("off"))
      .child(Checkbox::new("Sync settings").checked(true).id("on"))
      .child(Checkbox::new("Select all").indeterminate(true).id("mixed"))
      .into()
  });
  for id in ["off", "on", "mixed"] {
    let (_, _, width, height) = stage.bounds(id);
    assert_eq!((width, height), (16.0, 16.0), "{id}");
  }
  let radius = stage.app.theme().radius_value(RadiusSize::Sm);
  let off = stage.fill("off").unwrap();
  assert!(
    off.border_radius.is_some_and(|corners| corners.top_left == radius),
    "the small radius"
  );

  // Empty it is the background edged in the color of an input's edge; checked or indeterminate, the primary color.
  assert_eq!(stage.fill_color("off"), Some(light(PaletteColor::Background)));
  assert_eq!(edge(&mut stage, "off"), Some(light(PaletteColor::Input)));
  for id in ["on", "mixed"] {
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Primary)), "{id}");
    assert_eq!(edge(&mut stage, id), Some(light(PaletteColor::Primary)), "{id}");
  }

  // The mark is a glyph of 12 in the color on the primary one: a check, and a dash while it is indeterminate.
  let (check, minus) = (glyph(icons::CHECK), glyph(icons::MINUS));
  assert_eq!(stage.text(&check), (light(PaletteColor::PrimaryForeground), 12.0));
  assert_eq!(stage.text(&minus), (light(PaletteColor::PrimaryForeground), 12.0));
  let painted = |content: &str| {
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
      .count()
  };
  assert_eq!((painted(&check), painted(&minus)), (1, 1), "nothing in the empty box");

  // The label is 8 beside the box, in the body's type and the foreground.
  let (x, ..) = stage.bounds("on");
  let label = stage.text_quad("Sync settings").unwrap();
  assert_eq!(label.x - (x + 16.0), 8.0);
  assert_eq!(stage.text("Sync settings"), (light(PaletteColor::Foreground), 14.0));
}

#[test]
fn a_press_of_the_box_or_of_the_label_asks_for_the_other_state() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let tell = |id: &'static str| {
      let told = told.clone();
      move |next| told.lock().unwrap().push((id, next))
    };
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Checkbox::new("Sync").id("off").on_change(tell("off")))
      .child(Checkbox::new("Reports").checked(true).id("on").on_change(tell("on")))
      .child(
        Checkbox::new("Select all")
          .indeterminate(true)
          .id("mixed")
          .on_change(tell("mixed")),
      )
      .child(
        Checkbox::new("Managed")
          .disabled(true)
          .id("disabled")
          .on_change(tell("disabled")),
      )
      .child(Checkbox::new("Hidden").label_hidden(true).id("bare"))
      .into()
  });
  // The keyboard presses the box: Tab reaches the first checkbox and then the next, and Space or Enter
  // presses the one it is on. The ring is the box's.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  stage.key("Tab");
  stage.key("Enter");
  stage.key("Tab");
  stage.key(" ");
  // The fourth is disabled: Tab passes it by, to the fifth, which tells nobody.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(
    *asked.lock().unwrap(),
    [("off", true), ("on", false), ("mixed", true)],
    "an indeterminate one asks to be checked"
  );
  asked.lock().unwrap().clear();

  for id in ["off", "on", "mixed", "disabled"] {
    stage.click(id);
  }
  assert_eq!(*asked.lock().unwrap(), [("off", true), ("on", false), ("mixed", true)]);

  // A press of the label is a press of the checkbox.
  let label = stage.text_quad("Sync").unwrap();
  let (x, y) = (label.x + label.width / 2.0, label.y + label.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(asked.lock().unwrap().last(), Some(&("off", true)));
  assert_eq!(asked.lock().unwrap().len(), 4);

  assert!(!stage.paints("Hidden"), "the box alone");
  let (_, _, width, _) = stage.bounds("bare");
  assert_eq!(width, 16.0);
}

#[test]
fn an_invalid_checkbox_is_edged_in_the_destructive_color() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Checkbox::new("Accept the terms").invalid(true).id("empty"))
      .child(Checkbox::new("Accept").invalid(true).checked(true).id("checked"))
      .into()
  });
  assert_eq!(stage.fill_color("empty"), Some(light(PaletteColor::Background)));
  assert_eq!(stage.fill_color("checked"), Some(light(PaletteColor::Primary)));
  for id in ["empty", "checked"] {
    assert_eq!(edge(&mut stage, id), Some(light(PaletteColor::Destructive)), "{id}");
  }
}

#[test]
fn a_disabled_checkbox_is_dimmed_with_its_label_and_its_empty_box_muted() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Checkbox::new("Managed").disabled(true).id("empty"))
      .child(
        Checkbox::new("Managed by policy")
          .disabled(true)
          .checked(true)
          .id("checked"),
      )
      .child(Checkbox::new("Some").disabled(true).indeterminate(true).id("mixed"))
      .into()
  });
  assert_eq!(stage.fill_color("empty"), Some(light(PaletteColor::Muted)));
  assert_eq!(edge(&mut stage, "empty"), Some(light(PaletteColor::Input)));
  for id in ["checked", "mixed"] {
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Primary)), "{id}");
  }

  // The root, which holds the box and the label, is dimmed by the theme's opacity for what is disabled.
  // It is a layer of what is in it, so the opacity is the layer's and not a quad's.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(CheckboxState::Disabled, true);
  let root = theme.resolve::<Checkbox>(&case, None).part(CheckboxPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme
    .resolve::<Checkbox>(&Case::new(), None)
    .part(CheckboxPart::Root)
    .rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Checkbox::new("Sync settings").force(Some(Interaction::FocusVisible)))
      .into()
  });
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_themes_rule_restyles_the_box_and_its_mark() {
  const RED: Color = Color::new(200, 30, 30, 255);
  let app = App::new();
  app.theme().patch_recipe::<Checkbox>(|rules| {
    rules
      .state(CheckboxState::Checked, CheckboxPart::Box, |s| s.background(RED))
      .base(CheckboxPart::Indicator, |s| s.square(14.0))
  });
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(8.0)
      .child(Checkbox::new("Sync settings").checked(true).id("box"))
      .into()
  });
  assert_eq!(stage.fill_color("box"), Some(RED));
  assert_eq!(stage.text(&glyph(icons::CHECK)).1, 14.0);
}

#[test]
fn a_theme_gives_the_checkbox_another_anatomy_and_it_still_checks() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let app = App::new();
  // A checkbox that says what it is in words inside its box: other nodes, the same behavior.
  let renderer: CheckboxRenderer = Arc::new(|view: CheckboxView| {
    let word = if view.checked { "YES" } else { "NO" };
    view.control().child(view.part(CheckboxPart::Label).text(word)).into()
  });
  app.theme().set_renderer::<Checkbox>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(
        Checkbox::new("Sync settings")
          .id("box")
          .on_change(move |next| told.lock().unwrap().push(next)),
      )
      .into()
  });
  assert!(stage.paints("NO"));
  assert!(!stage.paints("Sync settings"));
  assert_eq!(
    stage.fill_color("box"),
    Some(light(PaletteColor::Background)),
    "the recipe still styles it"
  );
  stage.click("box");
  assert_eq!(*asked.lock().unwrap(), [true]);
}
