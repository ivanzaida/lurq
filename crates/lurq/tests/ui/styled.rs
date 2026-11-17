//! What a component declares for a recipe (its parts and its states) and `Styled`, which styles one instance of
//! any component the same way: what the gallery's playground reads and lays.

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  node::{Element, color::Color},
  ui::{
    Button, ButtonPart, DataGrid, Declared, Name, Recipe, Styled, Switch, SwitchPart, SwitchState, Table, Tabs,
    TabsPart, Themed,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A component of an app that declares nothing: its parts and states are those its recipe names.
struct Tile;

impl Themed for Tile {
  const NAME: &'static str = "styled-test-tile";
  type Renderer = ();

  fn recipe() -> Recipe {
    Recipe::new()
      .base("root", |s| s.padding_x(16.0))
      .base("title", |s| s.font_size(15.0))
      .state("pinned", "root", |s| s.background(PaletteColor::Muted))
      .state("pinned", "title", |s| s.font_size(16.0))
      .hovered("root", |s| s.background(PaletteColor::Highlight))
  }
}

#[test]
fn a_component_declares_its_parts_and_states_in_their_order_with_the_rust_that_writes_them() {
  let parts = Switch::parts();
  let names: Vec<&str> = parts.names().map(|name| name.as_str()).collect();
  assert_eq!(names, ["root", "track", "thumb", "label"]);
  assert_eq!(parts.enum_name, Some("SwitchPart"));
  assert_eq!(parts.code(SwitchPart::Thumb.into()), "SwitchPart::Thumb");
  assert_eq!(parts, SwitchPart::declared());

  let states = Switch::states();
  let names: Vec<&str> = states.names().map(|name| name.as_str()).collect();
  assert_eq!(names, ["on", "invalid", "disabled"]);
  assert_eq!(states.code(SwitchState::On.into()), "SwitchState::On");
  // A name the enum does not have is written as a string: a state an app adds.
  assert_eq!(states.code(Name::new("pinned")), "\"pinned\"");

  // A part named in snake case is written as its variant.
  assert!(Tabs::<()>::parts().contains(TabsPart::List.into()));
}

#[test]
fn a_component_that_declares_nothing_has_what_its_recipe_names() {
  let parts = Tile::parts();
  let names: Vec<&str> = parts.names().map(|name| name.as_str()).collect();
  assert_eq!(
    names,
    ["root", "title"],
    "each once, in the order the recipe first styles them"
  );
  assert_eq!(parts.enum_name, None);
  assert_eq!(parts.code(Name::new("title")), "\"title\"");
  let states: Vec<&str> = Tile::states().names().map(|name| name.as_str()).collect();
  assert_eq!(states, ["pinned"], "the pointer's cases are not states");
  assert!(Declared::default().is_empty());
}

/// A button styled through `Styled`, whatever it is: what a playground does with the component it is given.
fn pill<C: Styled>(component: C, part: impl Into<Name>) -> C {
  Styled::style(component, part, |s| s.background(PaletteColor::Destructive))
}

#[test]
fn styled_styles_one_instance_as_its_own_methods_do() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(pill(Button::new("Styled").id("styled"), ButtonPart::Root))
      .child(Button::new("Plain").id("plain"))
      .child(Styled::rules(
        Button::new("Ruled").id("ruled"),
        &Recipe::new().base(ButtonPart::Root, |s| s.background(PaletteColor::Success)),
      ))
      .child(Styled::state(
        Switch::new("Drawn on").id("drawn"),
        SwitchState::On,
        true,
      ))
      .into()
  });
  assert_eq!(stage.fill_color("styled"), Some(light(PaletteColor::Destructive)));
  assert_eq!(stage.fill_color("plain"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("ruled"), Some(light(PaletteColor::Success)));
  // A switch that is off, put in the state of one that is on, has the track of one that is on.
  assert_eq!(stage.fill_color("drawn"), Some(light(PaletteColor::Primary)));
}

#[test]
fn what_dresses_another_component_names_that_one_for_its_recipe() {
  fn component_of<C: Styled>(_: &C) -> &'static str {
    <C::Component as Themed>::NAME
  }
  let grid: DataGrid<Vec<String>> = DataGrid::new(Vec::new(), Vec::<Vec<String>>::new());
  assert_eq!(component_of(&grid), <Table<Vec<String>> as Themed>::NAME);
  assert_eq!(component_of(&Button::new("Save")), "button");
}

#[test]
fn a_theme_takes_back_what_it_said_of_a_recipe_and_a_part_renderer() {
  use std::sync::Arc;

  use lurq::{app::theme::Theme, ui::ButtonPartRenderer};

  let theme = Theme::new();
  theme.patch_recipe::<Button>(|r| r.base(ButtonPart::Root, |s| s.background(PaletteColor::Destructive)));
  let marked: ButtonPartRenderer = Arc::new(|view| view.part(ButtonPart::Label).text("marked").into());
  theme.set_part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Label, marked);
  let drawn = |theme: &Theme| {
    let theme = theme.clone();
    Stage::new(move |ctx| ctx.in_theme(&theme, |_| -> Element { Button::new("Save").id("save").into() }))
  };
  let mut patched = drawn(&theme);
  assert_eq!(patched.fill_color("save"), Some(light(PaletteColor::Destructive)));
  assert!(patched.paints("marked"));

  theme.reset_recipe::<Button>();
  theme.reset_part_renderer::<Button>(ButtonPart::Label);
  let mut reset = drawn(&theme);
  assert_eq!(reset.fill_color("save"), Some(light(PaletteColor::Primary)));
  assert!(reset.paints("Save") && !reset.paints("marked"));
}
