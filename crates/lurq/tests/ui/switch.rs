//! A switch: its track and its thumb, what a press of it or of its label
//! asks for, its states, the thumb's slide, and a theme's other anatomy.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

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
  ui::{Case, Interaction, Opacity, Switch, SwitchPart, SwitchRenderer, SwitchState, SwitchView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The thumb of the switch whose track has `id`: the round fill of 16 inside the track.
fn thumb(stage: &mut Stage, id: &str) -> Quad {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.width, quad.height) == (16.0, 16.0)
        && quad.x >= x - 0.01
        && quad.y >= y - 0.01
        && quad.x + quad.width <= x + width + 0.01
        && quad.y + quad.height <= y + height + 0.01
    })
    .unwrap_or_else(|| panic!("#{id} should have a thumb"))
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

#[test]
fn a_switch_is_a_track_with_its_thumb_at_the_end_it_is_at() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Switch::new("Reduce motion").id("off"))
      .child(Switch::new("Dark mode").on(true).id("on"))
      .into()
  });
  for id in ["off", "on"] {
    let (_, _, width, height) = stage.bounds(id);
    assert_eq!((width, height), (36.0, 20.0), "{id}");
  }
  assert_eq!(stage.fill_color("off"), Some(light(PaletteColor::Input)));
  assert_eq!(stage.fill_color("on"), Some(light(PaletteColor::Primary)));

  let (x, y, ..) = stage.bounds("off");
  let off = thumb(&mut stage, "off");
  assert_eq!((off.x - x, off.y - y), (2.0, 2.0), "off: at the start, 2 from the edge");
  assert_eq!(color_of(&off), light(PaletteColor::Background));
  assert!(off.border_radius.is_some_and(|radius| radius.top_left >= 8.0), "round");
  let (x, y, ..) = stage.bounds("on");
  let on = thumb(&mut stage, "on");
  assert_eq!((on.x - x, on.y - y), (18.0, 2.0), "on: at the end, 2 from the edge");

  // The label is 8 beside the track, in the body's type and the foreground.
  let label = stage.text_quad("Dark mode").unwrap();
  assert_eq!(label.x - (x + 36.0), 8.0);
  assert_eq!(stage.text("Dark mode"), (light(PaletteColor::Foreground), 14.0));
}

#[test]
fn a_press_of_the_track_or_of_the_label_asks_for_the_other_state() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let (off, on, gone) = (told.clone(), told.clone(), told.clone());
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        Switch::new("Sync")
          .id("off")
          .on_change(move |next| off.lock().unwrap().push(("off", next))),
      )
      .child(
        Switch::new("Dark mode")
          .on(true)
          .id("on")
          .on_change(move |next| on.lock().unwrap().push(("on", next))),
      )
      .child(
        Switch::new("Managed")
          .disabled(true)
          .id("disabled")
          .on_change(move |next| gone.lock().unwrap().push(("disabled", next))),
      )
      .child(Switch::new("Hidden").label_hidden(true).id("bare"))
      .into()
  });
  // The keyboard presses the track: Tab reaches the first switch and then the next, and Space presses
  // the one it is on. The ring is the track's.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  stage.key("Tab");
  stage.key("Enter");
  // The third is disabled: Tab passes it by, to the fourth, which tells nobody.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*asked.lock().unwrap(), [("off", true), ("on", false)]);
  asked.lock().unwrap().clear();

  stage.click("off");
  stage.click("on");
  stage.click("disabled");
  assert_eq!(*asked.lock().unwrap(), [("off", true), ("on", false)]);

  // A press of the label is a press of the switch.
  let label = stage.text_quad("Sync").unwrap();
  let (x, y) = (label.x + label.width / 2.0, label.y + label.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(asked.lock().unwrap().last(), Some(&("off", true)));
  assert_eq!(asked.lock().unwrap().len(), 3);

  assert!(!stage.paints("Hidden"), "the track alone");
  let (_, _, width, _) = stage.bounds("bare");
  assert_eq!(width, 36.0);
}

#[test]
fn a_disabled_switch_is_dimmed_with_its_label() {
  // The root, which holds the track and the label, is dimmed by the theme's opacity for what is disabled.
  // It is a layer of what is in it, so the opacity is the layer's and not a quad's.
  let stage = Stage::new(|_| Row::new().child(Switch::new("Managed").disabled(true)).into());
  let theme = stage.app.theme().clone();
  let case = Case::new().state(SwitchState::Disabled, true);
  let root = theme.resolve::<Switch>(&case, None).part(SwitchPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  assert_eq!(theme.opacity_value(OpacityLevel::Disabled), 0.5);
  let enabled = theme
    .resolve::<Switch>(&Case::new(), None)
    .part(SwitchPart::Root)
    .rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Switch::new("Dark mode").force(Some(Interaction::FocusVisible)))
      .into()
  });
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn the_thumb_slides_to_the_other_end() {
  let on = Signal::new(false);
  let read = on.clone();
  let mut stage = Stage::new(move |_| {
    let set = read.clone();
    Row::new()
      .padding(8.0)
      .child(
        Switch::new("Dark mode")
          .on(read.get())
          .id("switch")
          .on_change(move |next| set.set(next)),
      )
      .into()
  });
  let (x, ..) = stage.bounds("switch");
  assert_eq!(thumb(&mut stage, "switch").x - x, 2.0);
  on.set(true);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  let on_the_way = thumb(&mut stage, "switch").x - x;
  assert!(on_the_way > 2.0 && on_the_way < 18.0, "on the way: {on_the_way}");
  stage.pass_after(Duration::from_secs(1));
  assert_eq!(thumb(&mut stage, "switch").x - x, 18.0);
  assert_eq!(stage.fill_color("switch"), Some(light(PaletteColor::Primary)));
}

#[test]
fn a_theme_gives_the_switch_another_anatomy_and_it_still_switches() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let app = App::new();
  // A switch that says what it is in words inside its track: other nodes, the same behavior.
  let renderer: SwitchRenderer = Arc::new(|view: SwitchView| {
    let word = if view.on { "ON" } else { "OFF" };
    view.track().child(view.part(SwitchPart::Label).text(word)).into()
  });
  app.theme().set_renderer::<Switch>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(
        Switch::new("Dark mode")
          .id("switch")
          .on_change(move |next| told.lock().unwrap().push(next)),
      )
      .into()
  });
  assert!(stage.paints("OFF"));
  assert!(!stage.paints("Dark mode"));
  assert_eq!(
    stage.fill_color("switch"),
    Some(light(PaletteColor::Input)),
    "the recipe still styles it"
  );
  stage.click("switch");
  assert_eq!(*asked.lock().unwrap(), [true]);
}
