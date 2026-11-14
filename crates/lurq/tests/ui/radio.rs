//! A radio: its mark, its dot and its label, what a press of it asks for, its
//! states, a group of them that is one choice, and a theme's other anatomy.

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
    Case, Choice, Interaction, Opacity, Radio, RadioGroup, RadioGroupPart, RadioPart, RadioRenderer, RadioState,
    RadioView,
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

/// The color and the width of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).unwrap();
  (side.width, side.color)
}

/// The filled squares of `side` inside the element with `id`.
fn squares_in(stage: &mut Stage, id: &str, side: f32) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.width, quad.height) == (side, side)
        && quad.x >= x - 0.01
        && quad.y >= y - 0.01
        && quad.x + quad.width <= x + width + 0.01
        && quad.y + quad.height <= y + height + 0.01
    })
    .collect()
}

/// The mark of the radio labelled `label`: the square of 16 with an edge, 8 before its label.
fn mark_of(stage: &Stage, label: &str) -> Quad {
  let text = stage.text_quad(label).unwrap_or_else(|| panic!("no label {label:?}"));
  stage
    .quads()
    .into_iter()
    .find(|quad| {
      quad.border.is_some()
        && (quad.width, quad.height) == (16.0, 16.0)
        && (quad.x + 16.0 + 8.0 - text.x).abs() < 0.01
        && quad.y < text.y + text.height
        && quad.y + 16.0 > text.y
    })
    .unwrap_or_else(|| panic!("the radio {label:?} should have a mark"))
}

/// A press of the pointer at the middle of `quad`.
fn press_at(stage: &mut Stage, quad: &Quad) {
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

#[test]
fn a_radio_is_a_round_mark_with_a_dot_while_it_is_chosen() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Radio::new("Compact").id("not"))
      .child(Radio::new("Comfortable").chosen(true).id("chosen"))
      .into()
  });
  for id in ["not", "chosen"] {
    let (_, _, width, height) = stage.bounds(id);
    assert_eq!((width, height), (16.0, 16.0), "{id}");
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Background)), "{id}");
    let fill = stage.fill(id).unwrap();
    assert!(fill.border_radius.is_some_and(|radius| radius.top_left >= 8.0), "round");
  }
  // Not chosen, its edge is an input's and it holds nothing; chosen, its edge and its dot are the primary color.
  assert_eq!(edge(&mut stage, "not"), (1.0, light(PaletteColor::Input)));
  assert!(squares_in(&mut stage, "not", 8.0).is_empty());
  assert_eq!(edge(&mut stage, "chosen"), (1.0, light(PaletteColor::Primary)));
  let (x, y, ..) = stage.bounds("chosen");
  let dots = squares_in(&mut stage, "chosen", 8.0);
  assert_eq!(dots.len(), 1, "one dot");
  assert_eq!((dots[0].x - x, dots[0].y - y), (4.0, 4.0), "in the middle");
  assert_eq!(color_of(&dots[0]), light(PaletteColor::Primary));
  assert!(
    dots[0].border_radius.is_some_and(|radius| radius.top_left >= 4.0),
    "round"
  );

  // The label is 8 beside the mark, in the body's type and the foreground.
  let label = stage.text_quad("Comfortable").unwrap();
  assert_eq!(label.x - (x + 16.0), 8.0);
  assert_eq!(stage.text("Comfortable"), (light(PaletteColor::Foreground), 14.0));
}

#[test]
fn a_press_of_a_radio_not_chosen_chooses_it_and_of_one_chosen_does_nothing() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let tell = |name: &'static str| {
      let told = told.clone();
      move || told.lock().unwrap().push(name)
    };
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Radio::new("Compact").id("not").on_choose(tell("not")))
      .child(
        Radio::new("Comfortable")
          .chosen(true)
          .id("chosen")
          .on_choose(tell("chosen")),
      )
      .child(
        Radio::new("Managed")
          .disabled(true)
          .id("disabled")
          .on_choose(tell("disabled")),
      )
      .child(Radio::new("Spacious").id("last").on_choose(tell("last")))
      .into()
  });
  // The keyboard presses the mark: Tab reaches the first radio, Space presses it. The ring is the mark's.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  // The second is chosen already: Enter on it tells nobody. The third is disabled: Tab passes it by.
  stage.key("Tab");
  stage.key("Enter");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(*asked.lock().unwrap(), ["not", "last"]);
  asked.lock().unwrap().clear();

  stage.click("not");
  stage.click("chosen");
  stage.click("disabled");
  assert_eq!(*asked.lock().unwrap(), ["not"]);

  // A press of the label is a press of the radio.
  let label = stage.text_quad("Spacious").unwrap();
  press_at(&mut stage, &label);
  let label = stage.text_quad("Comfortable").unwrap();
  press_at(&mut stage, &label);
  assert_eq!(*asked.lock().unwrap(), ["not", "last"]);
}

#[test]
fn a_radio_with_its_label_hidden_is_its_mark_alone() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Radio::new("Compact").chosen(true).label_hidden(true).id("radio"))
      .into()
  });
  assert!(!stage.paints("Compact"));
  let (_, _, width, height) = stage.bounds("radio");
  assert_eq!((width, height), (16.0, 16.0));
}

#[test]
fn an_invalid_radio_has_the_destructive_edge_and_a_disabled_one_is_dimmed() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Radio::new("Invalid").invalid(true).id("invalid"))
      .child(Radio::new("Managed").chosen(true).disabled(true))
      .into()
  });
  assert_eq!(edge(&mut stage, "invalid"), (1.0, light(PaletteColor::Destructive)));

  // The root, which holds the mark and the label, is dimmed by the theme's opacity for what is disabled.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(RadioState::Disabled, true);
  let root = theme.resolve::<Radio>(&case, None).part(RadioPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme.resolve::<Radio>(&Case::new(), None).part(RadioPart::Root).rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Radio::new("Compact").force(Some(Interaction::FocusVisible)))
      .into()
  });
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_group_is_its_radios_one_under_the_other_and_chooses_by_its_signal() {
  let chosen = Signal::new(0u8);
  let read = chosen.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(RadioGroup::new(read.clone(), [(0u8, "Comfortable"), (1, "Compact"), (2, "Spacious")]).id("density"))
      .into()
  });
  let dot = |stage: &mut Stage, label: &str| {
    let mark = mark_of(stage, label);
    stage
      .quads()
      .iter()
      .any(|quad| (quad.width, quad.x - mark.x, quad.y - mark.y) == (8.0, 4.0, 4.0) && color_of(quad).a() > 0)
  };
  assert!(dot(&mut stage, "Comfortable"), "the one whose value the signal has");
  assert!(!dot(&mut stage, "Compact"));

  // One under the other, 12 apart: a radio is as high as its mark or its line of text.
  let (first, second) = (
    stage.text_quad("Comfortable").unwrap(),
    stage.text_quad("Compact").unwrap(),
  );
  assert_eq!(first.x, second.x);
  let (comfortable, compact) = (mark_of(&stage, "Comfortable"), mark_of(&stage, "Compact"));
  assert!((compact.y - comfortable.y - (first.height.max(16.0) + 12.0)).abs() < 0.01);

  // From the keyboard, with nothing focused yet: Tab comes into the group at the chosen radio, and an arrow
  // key moves the focus to the next and chooses it. From the last it goes to the first.
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(chosen.get(), 1);
  stage.key("ArrowRight");
  stage.key("ArrowDown");
  assert_eq!(chosen.get(), 0, "past the last is the first");
  stage.key("ArrowUp");
  assert_eq!(chosen.get(), 2);
  stage.key("Home");
  stage.key("ArrowDown");
  assert_eq!(chosen.get(), 1);

  // A press of another sets the signal, and the group is drawn again.
  let spacious = mark_of(&stage, "Spacious");
  press_at(&mut stage, &spacious);
  assert_eq!(chosen.get(), 2);
  assert!(dot(&mut stage, "Spacious"));
  assert!(!dot(&mut stage, "Comfortable"));
}

#[test]
fn a_controlled_group_tells_the_app_and_a_disabled_choice_takes_nothing() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let (plans, all) = (told.clone(), told.clone());
    Column::new()
      .padding(8.0)
      .spacing(16.0)
      .child(RadioGroup::controlled(
        Some("free"),
        [
          Choice::new("free", "Free"),
          Choice::new("team", "Team"),
          Choice::new("enterprise", "Enterprise").disabled(true),
        ],
        move |plan| plans.lock().unwrap().push(plan),
      ))
      .child(
        RadioGroup::controlled(None, [("a", "Apples"), ("b", "Pears")], move |fruit| {
          all.lock().unwrap().push(fruit)
        })
        .disabled(true),
      )
      .into()
  });
  for label in ["Free", "Team", "Enterprise", "Apples", "Pears"] {
    let mark = mark_of(&stage, label);
    press_at(&mut stage, &mark);
  }
  // The chosen one, the disabled one and the disabled group tell nothing.
  assert_eq!(*asked.lock().unwrap(), ["team"]);
}

#[test]
fn an_invalid_group_says_so_on_every_radio() {
  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(RadioGroup::controlled(None, [(0, "Yes"), (1, "No")], |_| {}).invalid(true))
      .into()
  });
  for label in ["Yes", "No"] {
    let mark = mark_of(&stage, label);
    let side = mark.border.and_then(|border| border.top).unwrap();
    assert_eq!(side.color, light(PaletteColor::Destructive), "{label}");
  }
}

#[test]
fn a_theme_gives_the_radio_another_anatomy_and_a_group_other_room() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let app = App::new();
  // A radio that says what it is in words inside its mark: other nodes, the same behavior.
  let renderer: RadioRenderer = Arc::new(|view: RadioView| {
    let word = if view.chosen { "YES" } else { "NO" };
    view.mark().child(view.part(RadioPart::Label).text(word)).into()
  });
  app.theme().set_renderer::<Radio>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(
        Radio::new("Compact")
          .id("radio")
          .on_choose(move || told.lock().unwrap().push(())),
      )
      .into()
  });
  assert!(stage.paints("NO"));
  assert!(!stage.paints("Compact"));
  stage.click("radio");
  assert_eq!(asked.lock().unwrap().len(), 1);
}

#[test]
fn a_group_styled_by_the_app_has_other_room_between_its_radios() {
  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        RadioGroup::controlled(Some(0), [(0, "Yes"), (1, "No")], |_| {}).style(RadioGroupPart::Root, |s| s.gap(20.0)),
      )
      .into()
  });
  let (yes, no) = (mark_of(&stage, "Yes"), mark_of(&stage, "No"));
  let row = stage.text_quad("Yes").unwrap().height.max(16.0);
  assert_eq!(no.y - yes.y, row + 20.0);
}

#[test]
fn tab_stops_once_in_a_group_at_the_chosen_radio() {
  let chosen = Signal::new(1u8);
  let read = chosen.clone();
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let told = pressed.clone();
  let mut stage = Stage::new(move |_| {
    let (before, after) = (told.clone(), told.clone());
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Radio::new("Before").on_choose(move || before.lock().unwrap().push("before")))
      .child(RadioGroup::new(
        read.clone(),
        [
          Choice::new(0u8, "Comfortable"),
          Choice::new(1, "Compact"),
          Choice::new(2, "Spacious").disabled(true),
          Choice::new(3, "Roomy"),
        ],
      ))
      .child(Radio::new("After").on_choose(move || after.lock().unwrap().push("after")))
      .into()
  });
  // The first Tab is on the radio before the group, the second in the group, the third after it.
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*pressed.lock().unwrap(), ["after"]);
  assert_eq!(chosen.get(), 1, "passing through the group chose nothing");

  // Back in the group the focus is on the chosen radio: the next arrow passes the disabled one by.
  stage
    .tree
    .key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage.pass();
  stage.key("ArrowDown");
  assert_eq!(chosen.get(), 3);
}

#[test]
fn a_group_in_a_row_has_its_radios_side_by_side() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(RadioGroup::controlled(Some(0u8), [(0u8, "Day"), (1, "Week"), (2, "Month")], |_| {}).horizontal(true))
      .into()
  });
  let (day, week, month) = (
    mark_of(&stage, "Day"),
    mark_of(&stage, "Week"),
    mark_of(&stage, "Month"),
  );
  assert_eq!((day.y, week.y), (week.y, month.y), "on one line");
  // 24 from the end of one label to the mark of the next.
  let label = stage.text_quad("Day").unwrap();
  assert!((week.x - (label.x + label.width) - 24.0).abs() < 0.01);
}
