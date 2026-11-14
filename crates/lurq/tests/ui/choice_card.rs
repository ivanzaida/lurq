//! A checkbox card and a radio card: the card with its text and its box or mark where the board has them, chosen
//! and not, under the pointer and focused, what a press and the keyboard do, the groups of them, a field that
//! judges one, and a theme's rule and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Case, CheckboxCard, CheckboxCardGroup, ChoiceCardPart, ChoiceCardRenderer, ChoiceCardState, ChoiceCardView, Field,
    Interaction, Opacity, RadioCard, RadioCardGroup, icons,
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

/// The width and the color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).unwrap();
  (side.width, side.color)
}

/// The quads of `side` inside the element with `id`, filled or edged.
fn squares_in(stage: &mut Stage, id: &str, side: f32) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { .. })
        && (quad.width, quad.height) == (side, side)
        && quad.x >= x - 0.01
        && quad.y >= y - 0.01
        && quad.x + quad.width <= x + width + 0.01
        && quad.y + quad.height <= y + height + 0.01
    })
    .collect()
}

/// The box of a checkbox card or the mark of a radio card: the square of 16 with an edge in the card `id`.
fn indicator(stage: &mut Stage, id: &str) -> Quad {
  squares_in(stage, id, 16.0)
    .into_iter()
    .find(|quad| quad.border.is_some())
    .unwrap_or_else(|| panic!("#{id} should hold a box or a mark"))
}

/// The color the box or the mark of the card `id` is filled with. A box with its check in it has its edge painted
/// over the check, apart from its fill: the fill is the filled square at the box's place.
fn indicator_fill(stage: &mut Stage, id: &str) -> Color {
  let at = indicator(stage, id);
  squares_in(stage, id, 16.0)
    .iter()
    .filter(|quad| (quad.x, quad.y) == (at.x, at.y))
    .map(color_of)
    .find(|color| color.a() > 0)
    .unwrap_or_else(|| panic!("the box of #{id} should be filled"))
}

/// Whether the card `id` has a dot of 8 of the primary color in the middle of its mark.
fn has_dot(stage: &mut Stage, id: &str) -> bool {
  let mark = indicator(stage, id);
  squares_in(stage, id, 8.0)
    .iter()
    .any(|quad| color_of(quad) == light(PaletteColor::Primary) && (quad.x - mark.x, quad.y - mark.y) == (4.0, 4.0))
}

/// Whose presses were told, in order.
fn log() -> (Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<String>>>) {
  let asked = Arc::new(Mutex::new(Vec::new()));
  (asked.clone(), asked)
}

#[test]
fn a_checkbox_card_is_the_boards_card_with_its_box_at_the_end() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        CheckboxCard::new("Analytics")
          .description("Share anonymous usage data.")
          .checked(true)
          .id("checked"),
      )
      .child(
        CheckboxCard::new("Crash reports")
          .description("Send reports when lurq fails.")
          .id("not"),
      )
      .into()
  });
  for id in ["checked", "not"] {
    let (_, _, width, _) = stage.bounds(id);
    assert_eq!(width, 220.0, "{id}");
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Background)), "{id}");
    let fill = stage.fill(id).unwrap();
    assert!(
      fill.border_radius.is_some_and(|radius| radius.top_left == 8.0),
      "radius-lg"
    );
  }
  // Checked, an edge of 2 of the primary color; not, 1 of an input's edge.
  assert_eq!(edge(&mut stage, "checked"), (2.0, light(PaletteColor::Primary)));
  assert_eq!(edge(&mut stage, "not"), (1.0, light(PaletteColor::Input)));

  // The title is in the corner of the padding, the description 4 under it; the box is at the end, at the top.
  let (x, y, width, _) = stage.bounds("checked");
  let title = stage.text_quad("Analytics").unwrap();
  assert_eq!((title.x - x, title.y - y), (16.0, 16.0));
  assert_eq!(stage.text("Analytics"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(
    stage.text("Share anonymous usage data."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let description = stage.text_quad("Share anonymous usage data.").unwrap();
  assert!(
    (description.y - (title.y + title.height + 4.0)).abs() < 0.5,
    "4 under the title"
  );
  let square = indicator(&mut stage, "checked");
  assert_eq!((x + width - (square.x + 16.0), square.y - y), (16.0, 16.0));
  assert_eq!(
    indicator_fill(&mut stage, "checked"),
    light(PaletteColor::Primary),
    "the checkbox's own box, checked"
  );
  let empty = indicator(&mut stage, "not");
  assert_eq!(indicator_fill(&mut stage, "not"), light(PaletteColor::Background));
  assert_eq!(
    empty.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Input))
  );
  // The text keeps 12 from the box.
  assert!(title.x + title.width <= square.x - 12.0 + 0.5);
}

#[test]
fn a_press_or_space_turns_a_checkbox_card_and_a_disabled_one_takes_nothing() {
  let (asked, told) = log();
  let mut stage = Stage::new(move |_| {
    let tell = |name: &'static str| {
      let told = told.clone();
      move |checked: bool| told.lock().unwrap().push(format!("{name} {checked}"))
    };
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(CheckboxCard::new("Analytics").id("first").on_change(tell("first")))
      .child(
        CheckboxCard::new("Backups")
          .disabled(true)
          .id("disabled")
          .on_change(tell("disabled")),
      )
      .child(
        CheckboxCard::new("Sync")
          .checked(true)
          .id("last")
          .on_change(tell("last")),
      )
      .into()
  });
  // The card is the stop of Tab and draws the ring; Space presses it. Tab passes the disabled one by.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*asked.lock().unwrap(), ["first true", "last false"]);
  asked.lock().unwrap().clear();

  // A press anywhere on the card is a press of it: its title, its box.
  let title = stage.text_quad("Analytics").unwrap();
  let (x, y) = (title.x + 2.0, title.y + 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
  stage.click("disabled");
  assert_eq!(*asked.lock().unwrap(), ["first true"]);

  // The disabled card is dimmed by the theme's opacity for what is disabled.
  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<CheckboxCard>(&Case::new().state(ChoiceCardState::Disabled, true), None)
    .part(ChoiceCardPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn under_the_pointer_a_card_not_chosen_takes_the_rings_edge_and_a_chosen_one_keeps_its_own() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(CheckboxCard::new("Not").id("not").on_change(|_| ()))
      .child(
        CheckboxCard::new("Checked")
          .checked(true)
          .id("checked")
          .on_change(|_| ()),
      )
      .into()
  });
  stage.hover("not");
  assert_eq!(edge(&mut stage, "not"), (1.0, light(PaletteColor::Ring)));
  stage.hover("checked");
  assert_eq!(edge(&mut stage, "checked"), (2.0, light(PaletteColor::Primary)));
  assert_eq!(edge(&mut stage, "not"), (1.0, light(PaletteColor::Input)));
}

#[test]
fn an_invalid_card_has_the_destructive_edge_and_a_forced_focus_draws_the_ring() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(RadioCard::new("Invalid").invalid(true).id("invalid"))
      .child(RadioCard::new("Chosen, invalid").chosen(true).invalid(true).id("both"))
      .into()
  });
  assert_eq!(edge(&mut stage, "invalid"), (1.0, light(PaletteColor::Destructive)));
  assert_eq!(edge(&mut stage, "both"), (2.0, light(PaletteColor::Destructive)));
  // The radio's own mark says so too.
  let mark = indicator(&mut stage, "invalid");
  assert_eq!(
    mark.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Destructive))
  );

  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(RadioCard::new("Focused").force(Some(Interaction::FocusVisible)))
      .into()
  });
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_radio_card_is_the_radios_mark_with_a_dot_while_it_is_chosen() {
  let (asked, told) = log();
  let mut stage = Stage::new(move |_| {
    let (one, two) = (told.clone(), told.clone());
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        RadioCard::new("Comfortable")
          .description("Roomy rows, 36px controls.")
          .chosen(true)
          .id("chosen")
          .on_choose(move || one.lock().unwrap().push("chosen".to_owned())),
      )
      .child(
        RadioCard::new("Compact")
          .id("not")
          .on_choose(move || two.lock().unwrap().push("not".to_owned())),
      )
      .into()
  });
  assert_eq!(edge(&mut stage, "chosen"), (2.0, light(PaletteColor::Primary)));
  assert_eq!(edge(&mut stage, "not"), (1.0, light(PaletteColor::Input)));
  let mark = indicator(&mut stage, "chosen");
  assert!(mark.border_radius.is_some_and(|radius| radius.top_left >= 8.0), "round");
  assert!(has_dot(&mut stage, "chosen"));
  assert!(!has_dot(&mut stage, "not"));
  let (x, y, width, _) = stage.bounds("chosen");
  assert_eq!((x + width - (mark.x + 16.0), mark.y - y), (16.0, 16.0));

  // A press of the one chosen tells nobody; of the other, it is to be chosen.
  stage.click("chosen");
  stage.click("not");
  assert_eq!(*asked.lock().unwrap(), ["not"]);
}

#[test]
fn an_icon_stands_before_the_text_and_a_price_at_the_end_of_the_title() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        RadioCard::new("Team")
          .description("For a few people.")
          .icon(icons::USERS)
          .price("$12")
          .id("card"),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("card");
  let title = stage.text_quad("Team").unwrap();
  // The icon of 20 is 12 before the text.
  assert_eq!(title.x - x, 16.0 + 20.0 + 12.0);
  let price = stage.text_quad("$12").unwrap();
  let mark = indicator(&mut stage, "card");
  assert!(
    (price.x + price.width - (mark.x - 12.0)).abs() < 0.5,
    "at the end of the text"
  );
  assert!(price.x > title.x + title.width);
  assert!(x + width > mark.x);
}

#[test]
fn a_group_of_radio_cards_is_one_stop_and_the_arrows_move_the_choice() {
  let chosen = Signal::new(0u8);
  let read = chosen.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        RadioCardGroup::new(
          read.clone(),
          [
            (0u8, RadioCard::new("Comfortable").id("a")),
            (1, RadioCard::new("Compact").id("b")),
            (2, RadioCard::new("Spacious").id("c").disabled(true)),
            (3, RadioCard::new("Roomy").id("d")),
          ],
        )
        .id("group"),
      )
      .into()
  });
  assert!(has_dot(&mut stage, "a"));
  // One under the other, 12 apart.
  let (a, b) = (stage.bounds("a"), stage.bounds("b"));
  assert_eq!(b.1 - (a.1 + a.3), 12.0);

  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(chosen.get(), 1);
  stage.key("ArrowDown");
  assert_eq!(chosen.get(), 3, "the disabled card is passed by");
  stage.key("Home");
  stage.key("ArrowDown");
  assert_eq!(chosen.get(), 1, "Home went to the first");

  stage.click("d");
  assert_eq!(chosen.get(), 3);
  assert!(has_dot(&mut stage, "d"));
  assert!(!has_dot(&mut stage, "b"));
  stage.click("c");
  assert_eq!(chosen.get(), 3, "a disabled card takes nothing");
}

#[test]
fn radio_cards_side_by_side_and_a_controlled_group_that_is_invalid() {
  let (asked, told) = log();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .size(600.0, 300.0)
      .padding(8.0)
      .child(
        RadioCardGroup::controlled(
          None,
          [
            ("free", RadioCard::new("Free").id("free")),
            (
              "team",
              RadioCard::new("Team").description("For a few people.").id("team"),
            ),
          ],
          move |plan| told.lock().unwrap().push(plan.to_owned()),
        )
        .horizontal(true)
        .invalid(true),
      )
      .into()
  });
  let (free, team) = (stage.bounds("free"), stage.bounds("team"));
  assert_eq!(free.1, team.1, "side by side");
  assert_eq!(team.0 - (free.0 + free.2), 12.0, "12 apart");
  assert_eq!(free.3, team.3, "as high as the highest");
  assert_eq!(edge(&mut stage, "free"), (1.0, light(PaletteColor::Destructive)));
  stage.click("team");
  assert_eq!(*asked.lock().unwrap(), ["team"]);
}

#[test]
fn a_group_of_checkbox_cards_adds_and_takes_out_by_its_signal() {
  let checked = Signal::new(vec!["backups"]);
  let read = checked.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(CheckboxCardGroup::new(
        read.clone(),
        [
          ("backups", CheckboxCard::new("Backups").id("backups")),
          ("support", CheckboxCard::new("Priority support").id("support")),
        ],
      ))
      .into()
  });
  assert_eq!(edge(&mut stage, "backups"), (2.0, light(PaletteColor::Primary)));
  stage.click("support");
  assert_eq!(checked.get(), ["backups", "support"]);
  // Each card is its own stop of Tab: the second Tab is on the second card.
  stage.key("Tab");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(checked.get(), ["backups"]);
}

#[test]
fn a_field_with_an_error_makes_its_card_invalid_and_a_disabled_field_disables_it() {
  let (asked, told) = log();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Field::new("Terms", CheckboxCard::new("I agree").id("terms")).error("Accept the terms to go on."))
      .child(
        Field::new(
          "Plan",
          RadioCard::new("Team")
            .id("plan")
            .on_choose(move || told.lock().unwrap().push("plan".to_owned())),
        )
        .disabled(true),
      )
      .into()
  });
  assert_eq!(edge(&mut stage, "terms"), (1.0, light(PaletteColor::Destructive)));
  stage.click("plan");
  assert!(asked.lock().unwrap().is_empty());
}

#[test]
fn a_rule_and_a_renderer_of_a_theme_change_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        CheckboxCard::new("Ruled")
          .id("card")
          .style(ChoiceCardPart::Root, |s| s.background(PaletteColor::Secondary))
          .style(ChoiceCardPart::Title, |s| s.font_size(20.0)),
      )
      .into()
  });
  assert_eq!(stage.fill_color("card"), Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.text("Ruled").1, 20.0);

  // A card that draws its title alone, without the box: other nodes, the same behavior.
  let (asked, told) = log();
  let app = App::new();
  let renderer: ChoiceCardRenderer = Arc::new(|view: ChoiceCardView| view.root().child(view.text()).into());
  app.theme().set_renderer::<CheckboxCard>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(
        CheckboxCard::new("Title alone")
          .id("card")
          .on_change(move |checked| told.lock().unwrap().push(checked.to_string())),
      )
      .into()
  });
  assert!(stage.paints("Title alone"));
  assert!(squares_in(&mut stage, "card", 16.0).is_empty(), "no box");
  stage.click("card");
  assert_eq!(*asked.lock().unwrap(), ["true"]);
}
