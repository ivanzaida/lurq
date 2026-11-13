//! A select: its trigger at rest and in each state, the list a press or the
//! keys open under it, what choosing an item does to the signal, and a
//! theme's rules and renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row, SelectOption},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{SelectIcon, color::Color},
  ui::{Case, Interaction, Opacity, Select, SelectPart, SelectRenderer, SelectSize, SelectState, SelectView, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The themes the selects here choose from.
fn themes() -> Vec<(String, &'static str)> {
  vec![
    ("system".to_owned(), "System"),
    ("light".to_owned(), "Light"),
    ("dark".to_owned(), "Dark"),
  ]
}

/// A page with one select of the themes, bound to `value`, as `dress` makes it.
fn page(value: &Signal<String>, dress: impl Fn(Select) -> Select + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(dress(Select::new(value.clone(), themes()).id("select")))
      .into()
  })
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
      && quad.x >= x - 0.01
      && quad.y >= y - 0.01
      && quad.x + quad.width <= x + width + 0.01
      && quad.y + quad.height <= y + height + 0.01
  })
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The filled rectangles painted in `color`.
fn filled(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect()
}

/// The middle of the text `content` as painted.
fn middle_of(stage: &Stage, content: &str) -> (f32, f32) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("no text {content:?} was painted"));
  (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0)
}

/// A press of the pointer at `(x, y)`: a press on an item of the list chooses it.
fn press_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// What the theme draws the check with: the text a check icon paints.
fn check_glyph(stage: &Stage) -> String {
  stage
    .app
    .theme()
    .icon(icons::CHECK)
    .expect("the theme has a check")
    .glyph
    .to_string()
}

#[test]
fn a_select_is_a_box_like_an_input_with_its_placeholder_or_its_value() {
  let (empty, chosen) = (Signal::new(String::new()), Signal::new("light".to_owned()));
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        Select::new(empty.clone(), themes())
          .placeholder("Select theme")
          .id("empty"),
      )
      .child(Select::new(chosen.clone(), themes()).id("chosen"))
      .into()
  });
  for id in ["empty", "chosen"] {
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
  assert_eq!(stage.text("Select theme"), (light(PaletteColor::MutedForeground), 14.0));
  assert_eq!(stage.text("Light"), (light(PaletteColor::Foreground), 14.0));
  // The text is 12 from the start of the box, and the list is closed.
  let (x, ..) = stage.bounds("chosen");
  let value = stage.text_quad("Light").unwrap();
  assert!((value.x - x - 12.0).abs() < 1.0, "12 in: {}", value.x - x);
  assert!(!stage.paints("System") && !stage.paints("Dark"));
  // The chevron is the theme's, 16 at the end of the box, in the muted foreground.
  let chevron = stage
    .app
    .theme()
    .icon(icons::CHEVRON_DOWN)
    .expect("the theme has a chevron")
    .glyph
    .to_string();
  let quad = stage
    .quads()
    .into_iter()
    .rfind(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == chevron))
    .expect("a chevron");
  let (_, _, width, _) = stage.bounds("chosen");
  assert!(
    (quad.x + 16.0 - (x + width - 12.0)).abs() < 1.0,
    "12 from the end: {}",
    x + width - quad.x - 16.0
  );
  assert_eq!(stage.text(&chevron).0, light(PaletteColor::MutedForeground));
}

#[test]
fn the_sizes_are_the_heights_of_the_controls_with_their_type() {
  let (small, medium, large) = (
    Signal::new("system".to_owned()),
    Signal::new("light".to_owned()),
    Signal::new("dark".to_owned()),
  );
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Select::new(small.clone(), themes()).size(SelectSize::Sm).id("sm"))
      .child(Select::new(medium.clone(), themes()).id("md"))
      .child(Select::new(large.clone(), themes()).size(SelectSize::Lg).id("lg"))
      .into()
  });
  for (id, text, height, size) in [
    ("sm", "System", 32.0, 13.0),
    ("md", "Light", 36.0, 14.0),
    ("lg", "Dark", 40.0, 15.0),
  ] {
    assert_eq!(stage.bounds(id).3, height, "{id}");
    assert_eq!(stage.text(text).1, size, "{id}");
  }
}

#[test]
fn a_press_opens_the_list_under_it_and_a_press_of_an_item_chooses_it() {
  let value = Signal::new("light".to_owned());
  let mut stage = page(&value, |select| select);
  let (x, y, width, height) = stage.bounds("select");
  stage.click("select");
  assert!(stage.paints("System") && stage.paints("Dark"), "the list is open");
  // Open, the trigger has the ring inside it.
  assert!(ring_inside(&mut stage, "select", light(PaletteColor::Ring)));

  // The panel is 4 under the trigger, as wide, on the popover's fill in an edge of the border's color, 8 round.
  let panel = stage
    .quads()
    .into_iter()
    .find(|quad| {
      (quad.x - x).abs() < 0.5
        && (quad.y - (y + height + 4.0)).abs() < 0.5
        && (quad.width - width).abs() < 0.5
        && quad.border.and_then(|border| border.top).map(|side| side.color) == Some(light(PaletteColor::Border))
    })
    .expect("the panel of the list");
  assert!(
    panel.border_radius.is_some_and(|radius| radius.top_left == 8.0),
    "8 round"
  );
  assert!(
    filled(&stage, light(PaletteColor::Popover))
      .iter()
      .any(|quad| (quad.x, quad.y, quad.width) == (panel.x, panel.y, panel.width)),
    "on the popover's fill"
  );

  // Items are 32 high, their text 8 in from the panel's padding of 4, in the body's type.
  let (system, light_item, dark) = (
    middle_of(&stage, "System"),
    middle_of(&stage, "Light"),
    middle_of(&stage, "Dark"),
  );
  assert!((light_item.1 - system.1 - 32.0).abs() < 0.5, "32 apart");
  assert!((dark.1 - light_item.1 - 32.0).abs() < 0.5, "32 apart");
  let first = stage.text_quad("System").unwrap();
  assert!((first.x - (x + 12.0)).abs() < 1.0, "4 and 8 in: {}", first.x - x);
  assert_eq!(stage.text("Dark"), (light(PaletteColor::Foreground), 14.0));

  // The chosen item has the check at its end, 12 in from the panel's edge.
  let check = check_glyph(&stage);
  let mark = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == check))
    .expect("a check on the chosen item");
  assert!(
    (mark.y + mark.height / 2.0 - light_item.1).abs() < 2.0,
    "on the chosen item's row"
  );
  assert!(
    (mark.x + mark.width - (x + width - 12.0)).abs() < 1.0,
    "at its end: {}",
    x + width - mark.x - mark.width
  );

  // The item under the pointer is on the highlight.
  stage.tree.mouse_move(dark.0, dark.1);
  stage.pass();
  assert!(
    filled(&stage, light(PaletteColor::Highlight))
      .iter()
      .any(|quad| quad.height == 32.0 && quad.y <= dark.1 && quad.y + 32.0 >= dark.1),
    "the item under the pointer is on the highlight"
  );

  press_at(&mut stage, dark);
  assert_eq!(value.get(), "dark");
  assert!(!stage.paints("System"), "the list is closed");
  assert!(stage.paints("Dark"), "the trigger shows what was chosen");
}

#[test]
fn the_keys_open_the_list_move_through_it_and_choose() {
  let value = Signal::new("system".to_owned());
  let mut stage = page(&value, |select| select);
  // Tab reaches it, and its focus is the ring inside it.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("select"));
  assert!(ring_inside(&mut stage, "select", light(PaletteColor::Ring)));
  stage.key("ArrowDown");
  assert!(stage.paints("Dark"), "the arrow opens the list");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(value.get(), "light");
  assert!(!stage.paints("Dark"), "Enter closes the list");
}

#[test]
fn a_value_no_option_has_shows_the_placeholder_and_a_disabled_option_is_dimmed_and_not_chosen() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        Select::new(
          bound.clone(),
          [
            SelectOption::new("kyiv".to_owned(), "Kyiv"),
            SelectOption::new("lisbon".to_owned(), "Lisbon").disabled(true),
          ],
        )
        .placeholder("Select a time zone")
        .id("select"),
      )
      .into()
  });
  assert!(stage.paints("Select a time zone"));
  // It is dimmed to the board's 0.45. The row is a layer of what is in it, so the opacity is the layer's and not a
  // quad's: the recipe says it.
  let item = stage
    .app
    .theme()
    .resolve::<Select>(&Case::new().state(SelectState::ItemDisabled, true), None)
    .part(SelectPart::Item)
    .rest();
  assert_eq!(item.opacity, Some(Opacity::Value(0.45)));
  stage.click("select");
  assert!(stage.paints("Lisbon"), "the disabled option is listed");
  let lisbon = middle_of(&stage, "Lisbon");
  press_at(&mut stage, lisbon);
  assert_eq!(value.get(), "", "a disabled option is not chosen");
}

#[test]
fn an_invalid_select_keeps_the_destructive_color_in_its_edge_and_its_ring() {
  let value = Signal::new(String::new());
  let mut stage = page(&value, |select| select.placeholder("Select theme").invalid(true));
  assert_eq!(edge(&mut stage, "select"), (1.0, light(PaletteColor::Destructive)));
  let forced = Signal::new(String::new());
  let mut focused = page(&forced, |select| {
    select.invalid(true).force(Some(Interaction::FocusVisible))
  });
  assert!(ring_inside(&mut focused, "select", light(PaletteColor::Destructive)));
  stage.click("select");
  assert!(
    ring_inside(&mut stage, "select", light(PaletteColor::Destructive)),
    "open"
  );
}

#[test]
fn warning_and_valid_are_an_inputs_judgments_and_invalid_is_shown_over_them() {
  for (judge, color) in [
    (
      (|select: Select| select.warning(true)) as fn(Select) -> Select,
      PaletteColor::Warning,
    ),
    (|select: Select| select.valid(true), PaletteColor::Success),
    (
      |select: Select| select.valid(true).warning(true).invalid(true),
      PaletteColor::Destructive,
    ),
  ] {
    let value = Signal::new("light".to_owned());
    let mut stage = page(&value, judge);
    assert_eq!(edge(&mut stage, "select"), (1.0, light(color.clone())), "{color:?}");
    let forced = Signal::new("light".to_owned());
    let mut focused = page(&forced, move |select| {
      judge(select).force(Some(Interaction::FocusVisible))
    });
    assert!(ring_inside(&mut focused, "select", light(color.clone())), "{color:?}");
  }
}

#[test]
fn groups_have_a_label_and_a_line_between_them_and_an_option_has_a_hint_at_its_end() {
  let value = Signal::new("windows".to_owned());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        Select::new(
          bound.clone(),
          [
            SelectOption::new("windows".to_owned(), "Windows").group("Desktop"),
            SelectOption::new("linux".to_owned(), "Linux"),
            SelectOption::new("ios".to_owned(), "iOS").group("Mobile").hint("beta"),
            SelectOption::new("android".to_owned(), "Android")
              .hint("soon")
              .disabled(true),
          ],
        )
        .id("select"),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("select");
  stage.click("select");
  // A group's label is 11, semibold, in the muted foreground, over its items.
  assert_eq!(stage.text("Desktop"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(stage.text("Mobile"), (light(PaletteColor::MutedForeground), 11.0));
  let (desktop, windows, linux, mobile, ios) = (
    middle_of(&stage, "Desktop"),
    middle_of(&stage, "Windows"),
    middle_of(&stage, "Linux"),
    middle_of(&stage, "Mobile"),
    middle_of(&stage, "iOS"),
  );
  assert!(
    desktop.1 < windows.1 && linux.1 < mobile.1 && mobile.1 < ios.1,
    "in order"
  );
  // The labels start where the items' text does.
  let (label, item) = (stage.text_quad("Mobile").unwrap(), stage.text_quad("iOS").unwrap());
  assert!((label.x - item.x).abs() < 1.0, "lined up: {} {}", label.x, item.x);
  // A line of 1 in the border's color is between the groups, across the panel inside its padding.
  assert!(
    filled(&stage, light(PaletteColor::Border)).iter().any(|quad| {
      quad.height == 1.0 && (quad.width - (width - 8.0)).abs() < 0.5 && quad.y > linux.1 && quad.y < mobile.1
    }),
    "a line between the groups"
  );
  // The hint is 11 in the mono type, muted, at the end of its item.
  assert_eq!(stage.text("beta"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(stage.font_of("beta"), stage.app.theme().mono_family().to_string());
  let hint = stage.text_quad("beta").unwrap();
  assert!((hint.y + hint.height / 2.0 - ios.1).abs() < 2.0, "on its item's row");
  assert!(hint.x > item.x + item.width, "after the label");
  // Only the chosen item keeps room for the check: the hint of another is at the row's end, 4 and 8 in.
  assert!(
    (hint.x + hint.width - (x + width - 12.0)).abs() < 1.0,
    "at the row's end: {}",
    x + width - hint.x - hint.width
  );
  // The keys pass the labels and the line: from Linux, down is iOS.
  press_at(&mut stage, linux);
  assert_eq!(value.get(), "linux");
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(value.get(), "ios");
}

#[test]
fn the_open_state_draws_the_ring_with_the_list_closed() {
  let value = Signal::new("light".to_owned());
  let mut stage = page(&value, |select| select.state(SelectState::Open, true));
  assert!(ring_inside(&mut stage, "select", light(PaletteColor::Ring)));
  assert!(!stage.paints("Dark"));
}

#[test]
fn a_disabled_select_is_dimmed_on_the_muted_fill_and_takes_nothing() {
  let value = Signal::new("light".to_owned());
  let mut stage = page(&value, |select| select.disabled(true));
  assert_eq!(stage.fill_color("select"), Some(light(PaletteColor::Muted)));
  let theme = stage.app.theme().clone();
  let trigger = theme
    .resolve::<Select>(&Case::new().state(SelectState::Disabled, true), None)
    .part(SelectPart::Trigger)
    .rest();
  assert_eq!(trigger.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  stage.click("select");
  assert!(!stage.paints("Dark"), "a press does not open it");
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("select"), "Tab passes it by");
  stage.key("ArrowDown");
  assert!(!stage.paints("Dark"));
  assert_eq!(value.get(), "light");
}

#[test]
fn a_theme_restyles_the_items_and_another_renderer_still_chooses() {
  let app = App::new();
  // Taller items, on the primary color under the pointer.
  app.theme().patch_recipe::<Select>(|recipe| {
    recipe
      .base(SelectPart::Item, |s| s.height(40.0))
      .hovered(SelectPart::Item, |s| s.background(PaletteColor::Primary))
  });
  // A chevron of its own, in words.
  let renderer: SelectRenderer = Arc::new(|view: SelectView| {
    let style = view.select_style().chevron(SelectIcon::text("more"));
    view.select(style)
  });
  app.theme().set_renderer::<Select>(renderer);
  let value = Signal::new("system".to_owned());
  let bound = value.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Row::new()
      .padding(8.0)
      .child(Select::new(bound.clone(), themes()).id("select"))
      .into()
  });
  assert!(stage.paints("more"));
  assert_eq!(
    stage.fill_color("select"),
    Some(light(PaletteColor::Background)),
    "the recipe still styles it"
  );
  stage.click("select");
  let (system, dark) = (middle_of(&stage, "System"), middle_of(&stage, "Dark"));
  assert!(
    (dark.1 - system.1 - 80.0).abs() < 0.5,
    "40 apart: {}",
    dark.1 - system.1
  );
  stage.tree.mouse_move(dark.0, dark.1);
  stage.pass();
  assert!(
    filled(&stage, light(PaletteColor::Primary))
      .iter()
      .any(|quad| quad.height == 40.0),
    "the item under the pointer is on the primary color"
  );
  press_at(&mut stage, dark);
  assert_eq!(value.get(), "dark");
}
