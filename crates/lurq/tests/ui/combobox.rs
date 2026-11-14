//! A combobox: its box at rest and in each state, typing that opens and filters the list, the keys that move
//! through it and choose, what leaving it does to what was typed, custom values, the rows of no results and of
//! loading, a field around it, and a theme's rules and renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, SelectOption},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Combobox, ComboboxPart, ComboboxRenderer, ComboboxSize, ComboboxView, Field, Interaction, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The frameworks the comboboxes here choose from.
fn frameworks() -> Vec<(String, &'static str)> {
  vec![
    ("lurq".to_owned(), "lurq"),
    ("gpui".to_owned(), "GPUI"),
    ("slint".to_owned(), "Slint"),
    ("iced".to_owned(), "Iced"),
    ("egui".to_owned(), "egui"),
  ]
}

/// The page as a window holds it: as large as the stage's window, so that the open list, a layer under the box, is
/// inside the root's box, where it is painted.
fn window() -> Column {
  Column::new().size(400.0, 300.0)
}

/// A page with one combobox of the frameworks, bound to `value`, as `dress` makes it.
fn page(value: &Signal<String>, dress: impl Fn(Combobox) -> Combobox + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  Stage::new(move |_| {
    window()
      .padding(8.0)
      .child(dress(
        Combobox::new(value.clone(), frameworks())
          .placeholder("Select framework")
          .id("framework"),
      ))
      .into()
  })
}

fn is_open(stage: &mut Stage) -> bool {
  stage
    .tree
    .get_element_by_id_mut("framework")
    .and_then(|element| element.as_select())
    .is_some_and(|select| select.is_open())
}

/// What the box's input holds.
fn typed(stage: &mut Stage) -> String {
  stage
    .tree
    .get_element_by_id_mut("framework-input")
    .and_then(|element| element.as_text_input())
    .map(|input| input.value())
    .expect("the combobox's input")
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Types `text` a key at a time, as a person does.
fn type_in(stage: &mut Stage, text: &str) {
  for ch in text.chars() {
    stage.key(&ch.to_string());
  }
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

/// The filled rectangles painted in `color`.
fn filled(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect()
}

/// The middle of the text `content` as painted last: in the open list, which is a layer over the page.
fn middle_of(stage: &Stage, content: &str) -> (f32, f32) {
  let quad = stage
    .quads()
    .into_iter()
    .rfind(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .unwrap_or_else(|| panic!("no text {content:?} was painted"));
  (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0)
}

/// Whether the text `content` is painted under the box: in the open list.
fn listed(stage: &mut Stage, content: &str) -> bool {
  let (_, y, _, height) = stage.bounds("framework");
  stage
    .quads()
    .into_iter()
    .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content) && quad.y > y + height)
}

fn press_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn glyph(stage: &Stage, name: &str) -> String {
  stage
    .app
    .theme()
    .icon(name)
    .expect("the theme has the icon")
    .glyph
    .to_string()
}

#[test]
fn a_combobox_is_the_box_of_a_select_with_its_placeholder_and_two_chevrons() {
  let value = Signal::new(String::new());
  let mut stage = page(&value, |combobox| combobox);
  let (x, _, width, height) = stage.bounds("framework");
  assert_eq!((width, height), (260.0, 36.0));
  assert_eq!(stage.fill_color("framework"), Some(light(PaletteColor::Background)));
  assert_eq!(edge(&mut stage, "framework"), (1.0, light(PaletteColor::Input)));
  assert!(
    stage
      .border("framework")
      .unwrap()
      .border_radius
      .is_some_and(|radius| radius.top_left == 6.0),
    "6 round"
  );
  // The placeholder is 12 in, in the muted foreground at 14.
  assert_eq!(
    stage.text("Select framework"),
    (light(PaletteColor::MutedForeground), 14.0)
  );
  let placeholder = stage.text_quad("Select framework").unwrap();
  assert!((placeholder.x - x - 12.0).abs() < 1.0, "12 in: {}", placeholder.x - x);
  // The two chevrons are 16 at the end, 12 from it, in the muted foreground.
  let chevrons = glyph(&stage, icons::CHEVRONS_VERTICAL);
  let quad = stage.text_quad(&chevrons).expect("the chevrons");
  assert!(
    (quad.x + 16.0 - (x + width - 12.0)).abs() < 1.0,
    "12 from the end: {}",
    x + width - quad.x - 16.0
  );
  assert_eq!(stage.text(&chevrons).0, light(PaletteColor::MutedForeground));
  assert!(!is_open(&mut stage));
}

#[test]
fn the_sizes_are_the_heights_of_the_controls() {
  for (size, height) in [
    (ComboboxSize::Sm, 32.0),
    (ComboboxSize::Md, 36.0),
    (ComboboxSize::Lg, 40.0),
  ] {
    let value = Signal::new("lurq".to_owned());
    let mut stage = page(&value, move |combobox| combobox.size(size));
    assert_eq!(stage.bounds("framework").3, height, "{size:?}");
  }
}

#[test]
fn a_chosen_value_shows_its_label_and_a_press_opens_the_whole_list() {
  let value = Signal::new("slint".to_owned());
  let mut stage = page(&value, |combobox| combobox);
  assert_eq!(typed(&mut stage), "Slint");
  assert_eq!(stage.text("Slint"), (light(PaletteColor::Foreground), 14.0));
  stage.click("framework");
  assert!(is_open(&mut stage));
  // What the box shows is the chosen label: the list is whole, the chosen one checked.
  for label in ["lurq", "GPUI", "Slint", "Iced", "egui"] {
    assert!(listed(&mut stage, label), "{label} is listed");
  }
  let check = glyph(&stage, icons::CHECK);
  let mark = stage.text_quad(&check).expect("a check on the chosen item");
  assert!((mark.y + mark.height / 2.0 - middle_of(&stage, "Slint").1).abs() < 2.0);
  // The focus is in the input, and the box has the ring inside it.
  assert_eq!(focused(&stage).as_deref(), Some("framework-input"));
  assert!(ring_inside(&mut stage, "framework", light(PaletteColor::Ring)));
  // A press of an item chooses it and puts its label in the box.
  let egui = middle_of(&stage, "egui");
  press_at(&mut stage, egui);
  assert_eq!(value.get(), "egui");
  assert_eq!(typed(&mut stage), "egui");
  assert!(!is_open(&mut stage));
}

#[test]
fn typing_opens_the_list_filters_it_without_regard_to_case_and_enter_chooses_the_first() {
  let value = Signal::new(String::new());
  let mut stage = page(&value, |combobox| combobox);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("framework-input"));
  assert!(!is_open(&mut stage), "the focus alone does not open it");
  type_in(&mut stage, "I");
  assert!(is_open(&mut stage), "typing opens it");
  assert_eq!(typed(&mut stage), "I");
  for (label, shown) in [
    ("GPUI", true),
    ("Slint", true),
    ("Iced", true),
    ("egui", true),
    ("lurq", false),
  ] {
    assert_eq!(listed(&mut stage, label), shown, "{label}");
  }
  type_in(&mut stage, "c");
  assert!(listed(&mut stage, "Iced") && !listed(&mut stage, "GPUI"));
  // The first that is left is highlighted: Enter chooses it.
  let iced = middle_of(&stage, "Iced");
  assert!(
    filled(&stage, light(PaletteColor::Highlight))
      .iter()
      .any(|quad| quad.y <= iced.1 && quad.y + quad.height >= iced.1),
    "the first match is on the highlight"
  );
  stage.key("Enter");
  assert_eq!(value.get(), "iced");
  assert_eq!(typed(&mut stage), "Iced");
  assert!(!is_open(&mut stage));
}

#[test]
fn the_arrows_move_the_highlight_and_escape_closes_then_clears() {
  let value = Signal::new("lurq".to_owned());
  let mut stage = page(&value, |combobox| combobox);
  stage.key("Tab");
  stage.key("ArrowDown");
  assert!(is_open(&mut stage), "Down opens it, on the chosen option");
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  stage.key("ArrowUp");
  stage.key("Enter");
  assert_eq!(value.get(), "gpui");
  assert_eq!(focused(&stage).as_deref(), Some("framework-input"), "the focus stays");
  type_in(&mut stage, "x");
  assert!(is_open(&mut stage));
  stage.key("Escape");
  assert!(!is_open(&mut stage), "Escape closes the list");
  assert_eq!(typed(&mut stage), "GPUIx", "and keeps what was typed");
  stage.key("Escape");
  assert_eq!(typed(&mut stage), "", "a second Escape clears it");
  assert!(!is_open(&mut stage));
  assert_eq!(value.get(), "gpui", "clearing what is typed chooses nothing");
}

#[test]
fn what_leaves_no_option_says_no_results_and_leaving_takes_back_what_was_typed() {
  let value = Signal::new("iced".to_owned());
  let mut stage = page(&value, |combobox| combobox.empty("Nothing like it"));
  stage.key("Tab");
  stage.key("End");
  type_in(&mut stage, "zz");
  assert!(is_open(&mut stage));
  assert!(listed(&mut stage, "Nothing like it"));
  assert_eq!(
    stage.text("Nothing like it"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  // Tab chooses nothing, closes the list and goes on; what was typed goes back to the chosen label.
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("framework-input"));
  assert!(!is_open(&mut stage));
  assert_eq!(typed(&mut stage), "Iced");
  assert_eq!(value.get(), "iced");
}

#[test]
fn a_loading_list_ends_in_a_row_that_says_so() {
  let value = Signal::new(String::new());
  let mut stage = page(&value, |combobox| combobox.loading(true));
  stage.click("framework");
  assert!(listed(&mut stage, "Loading results…"));
  let (lurq, loading) = (middle_of(&stage, "egui"), middle_of(&stage, "Loading results…"));
  assert!(loading.1 > lurq.1, "after the options");
}

#[test]
fn the_app_filters_and_hears_what_is_typed() {
  let (value, heard) = (Signal::new(String::new()), Signal::new(String::new()));
  let told = heard.clone();
  let mut stage = page(&value, move |combobox| {
    let told = told.clone();
    combobox
      .filter(|label, typed| label.to_lowercase().starts_with(&typed.to_lowercase()))
      .on_input(move |text| told.set(text.to_owned()))
  });
  stage.key("Tab");
  type_in(&mut stage, "e");
  assert_eq!(heard.get(), "e");
  assert!(listed(&mut stage, "egui"), "egui starts with e");
  assert!(!listed(&mut stage, "Iced"), "Iced only contains it");
}

#[test]
fn custom_values_keep_what_was_typed() {
  let value = Signal::new("lurq".to_owned());
  let mut stage = page(&value, |combobox| combobox.allow_custom(true));
  stage.key("Tab");
  // The list is closed: Escape clears what the box shows.
  stage.key("Escape");
  type_in(&mut stage, "Bevy");
  // What is typed may be no option's: nothing is highlighted, and Enter keeps it.
  assert!(filled(&stage, light(PaletteColor::Highlight)).is_empty());
  stage.key("Enter");
  assert_eq!(value.get(), "Bevy");
  assert_eq!(typed(&mut stage), "Bevy");
  // An option's label, in any case, chooses the option.
  stage.click("framework");
  stage.key("Escape");
  stage.key("Escape");
  type_in(&mut stage, "slint");
  stage.key("Tab");
  assert_eq!(value.get(), "slint");
  assert_eq!(typed(&mut stage), "Slint");
}

#[test]
fn groups_keep_their_label_when_their_first_option_is_filtered_out() {
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    window()
      .padding(8.0)
      .child(
        Combobox::new(
          bound.clone(),
          [
            SelectOption::new("lurq".to_owned(), "lurq").group("Rust"),
            SelectOption::new("iced".to_owned(), "Iced"),
            SelectOption::new("react".to_owned(), "React").group("Web"),
            SelectOption::new("solid".to_owned(), "Solid").disabled(true),
          ],
        )
        .id("framework"),
      )
      .into()
  });
  stage.key("Tab");
  type_in(&mut stage, "ce");
  assert!(
    listed(&mut stage, "Iced") && listed(&mut stage, "Rust"),
    "Iced under its group"
  );
  assert!(!listed(&mut stage, "lurq") && !listed(&mut stage, "Web"));
  // A disabled option is passed by: Enter with only it left chooses nothing.
  stage.key("Escape");
  stage.key("Escape");
  type_in(&mut stage, "sol");
  assert!(listed(&mut stage, "Solid"));
  stage.key("Enter");
  assert_eq!(value.get(), "");
}

#[test]
fn a_disabled_combobox_takes_nothing() {
  let value = Signal::new("lurq".to_owned());
  let mut stage = page(&value, |combobox| combobox.disabled(true));
  assert_eq!(stage.fill_color("framework"), Some(light(PaletteColor::Muted)));
  stage.click("framework");
  assert!(!is_open(&mut stage));
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("framework-input"), "Tab passes it by");
  stage.key("ArrowDown");
  assert!(!is_open(&mut stage));
  assert_eq!(value.get(), "lurq");
}

#[test]
fn judgments_are_an_inputs_and_a_field_gives_its_own() {
  for (judge, color) in [
    (
      (|combobox: Combobox| combobox.invalid(true)) as fn(Combobox) -> Combobox,
      PaletteColor::Destructive,
    ),
    (|combobox: Combobox| combobox.warning(true), PaletteColor::Warning),
    (|combobox: Combobox| combobox.valid(true), PaletteColor::Success),
  ] {
    let value = Signal::new("lurq".to_owned());
    let mut stage = page(&value, judge);
    assert_eq!(edge(&mut stage, "framework"), (1.0, light(color.clone())), "{color:?}");
    let forced = Signal::new("lurq".to_owned());
    let mut focused = page(&forced, move |combobox| {
      judge(combobox).force(Some(Interaction::FocusVisible))
    });
    assert!(
      ring_inside(&mut focused, "framework", light(color.clone())),
      "{color:?}"
    );
  }
  let value = Signal::new(String::new());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    window()
      .padding(8.0)
      .child(Field::new("Framework", Combobox::new(bound.clone(), frameworks()).id("framework")).error("Choose one"))
      .into()
  });
  assert_eq!(edge(&mut stage, "framework"), (1.0, light(PaletteColor::Destructive)));
  stage.key("Tab");
  type_in(&mut stage, "gp");
  stage.key("Enter");
  assert_eq!(value.get(), "gpui", "it works in a field");
}

#[test]
fn a_theme_restyles_the_items_and_another_renderer_still_chooses() {
  let app = App::new();
  app.theme().patch_recipe::<Combobox>(|recipe| {
    recipe
      .base(ComboboxPart::Item, |s| s.height(40.0))
      .hovered(ComboboxPart::Item, |s| s.background(PaletteColor::Primary))
  });
  // The chevrons in words, after the input.
  let renderer: ComboboxRenderer = Arc::new(|view: ComboboxView| {
    let style = view.select_style();
    view.select(style, |state| {
      let mut row = lurq::components::Row::new();
      if let Some(input) = state.input {
        row = row.child(input);
      }
      row.child(lurq::components::Text::new("more")).into()
    })
  });
  app.theme().set_renderer::<Combobox>(renderer);
  let value = Signal::new("lurq".to_owned());
  let bound = value.clone();
  let mut stage = Stage::in_app(app, move |_| {
    window()
      .padding(8.0)
      .child(Combobox::new(bound.clone(), frameworks()).id("framework"))
      .into()
  });
  assert!(stage.paints("more"));
  stage.click("framework");
  let (lurq, gpui) = (middle_of(&stage, "lurq"), middle_of(&stage, "GPUI"));
  assert!((gpui.1 - lurq.1 - 40.0).abs() < 0.5, "40 apart: {}", gpui.1 - lurq.1);
  stage.key("ArrowDown");
  assert!(
    filled(&stage, light(PaletteColor::Primary))
      .iter()
      .any(|quad| quad.height == 40.0),
    "the item under the keys is on the primary color"
  );
  press_at(&mut stage, gpui);
  assert_eq!(value.get(), "gpui");
}
