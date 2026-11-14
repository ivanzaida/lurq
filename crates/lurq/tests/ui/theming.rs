//! What a theme can change about a component: rules over its recipe, a recipe
//! of its own, no recipe at all, a renderer, a renderer for one part, and all
//! of that for a subtree or for one instance.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, RadiusSize, Theme, ThemeDefinition, ThemePalette},
  },
  components::{Column, Rect, Row, Text},
  node::{Element, color::Color},
  ui::{
    Button, ButtonPart, ButtonPartRenderer, ButtonRenderer, ButtonVariant, ButtonView, Recipe, Selector, current_theme,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn save() -> Element {
  Row::new().padding(20.0).child(Button::new("Save").id("save")).into()
}

fn top_left_radius(stage: &mut Stage, id: &str) -> f32 {
  stage
    .fill(id)
    .and_then(|quad| quad.border_radius)
    .map(|radius| radius.top_left)
    .unwrap_or(0.0)
}

#[test]
fn a_rule_laid_over_the_recipe_changes_every_button_it_holds_for() {
  let app = App::new();
  // Every primary button is a pill.
  app.theme().patch_recipe::<Button>(|rules| {
    rules.variant(ButtonVariant::Primary, ButtonPart::Root, |s| s.radius(RadiusSize::Full))
  });
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .spacing(8.0)
      .child(Button::new("Save").id("primary"))
      .child(Button::new("Cancel").variant(ButtonVariant::Secondary).id("secondary"))
      .into()
  });
  assert_eq!(
    top_left_radius(&mut stage, "primary"),
    999.0,
    "the theme's full radius: a pill"
  );
  assert_eq!(
    top_left_radius(&mut stage, "secondary"),
    6.0,
    "the design's, which the rule is not for"
  );
  assert_eq!(
    stage.fill_color("primary"),
    Some(light(PaletteColor::Primary)),
    "the rest of the recipe holds"
  );
}

#[test]
fn a_theme_changed_while_the_app_runs_reaches_the_buttons_on_screen() {
  let mut stage = Stage::new(|_| save());
  assert_eq!(top_left_radius(&mut stage, "save"), 6.0);
  stage
    .app
    .theme()
    .patch_recipe::<Button>(|rules| rules.base(ButtonPart::Root, |s| s.radius(0.0)));
  stage.pass();
  assert_eq!(
    top_left_radius(&mut stage, "save"),
    0.0,
    "what built the button rendered again"
  );
}

#[test]
fn an_app_adds_a_variant_of_its_own_by_name() {
  let app = App::new();
  let brand = Color::from_hex("#7c3aed");
  app.theme().set_palette_color("brand", brand);
  app.theme().patch_recipe::<Button>(|rules| {
    rules
      .variant("brand", ButtonPart::Root, |s| {
        s.background(PaletteColor::extra("brand"))
          .foreground(Color::from_hex("#ffffff"))
      })
      .when(Selector::new().variant("brand").hovered(), ButtonPart::Root, |s| {
        s.background(Color::from_hex("#6d28d9"))
      })
  });
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(20.0)
      .child(Button::new("Upgrade").variant("brand").id("brand"))
      .into()
  });
  assert_eq!(stage.fill_color("brand"), Some(brand));
  assert_eq!(
    stage.bounds("brand").3,
    36.0,
    "it is a button of the design in everything else"
  );
  stage.hover("brand");
  assert_eq!(stage.fill_color("brand"), Some(Color::from_hex("#6d28d9")));
}

#[test]
fn a_recipe_set_whole_leaves_nothing_of_the_designs() {
  let app = App::new();
  app.theme().set_recipe::<Button>(
    Recipe::new()
      .base(ButtonPart::Root, |s| {
        s.height(50.0).padding_x(30.0).background(Color::from_hex("#0f766e"))
      })
      .base(ButtonPart::Label, |s| s.font_size(20.0)),
  );
  let mut stage = Stage::in_app(app, |_| save());
  assert_eq!(stage.bounds("save").3, 50.0);
  assert_eq!(stage.fill_color("save"), Some(Color::from_hex("#0f766e")));
  assert_eq!(
    top_left_radius(&mut stage, "save"),
    0.0,
    "no radius of the design's under it"
  );
  assert_eq!(stage.text("Save").1, 20.0);
  stage.hover("save");
  assert_eq!(
    stage.fill_color("save"),
    Some(Color::from_hex("#0f766e")),
    "and no hover fill"
  );
}

#[test]
fn a_bare_theme_styles_nothing_and_every_button_still_works() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let app = App::new();
  app.theme().apply(ThemeDefinition::bare());
  let mut stage = Stage::in_app(app, move |_| {
    let counted = counted.clone();
    Row::new()
      .padding(20.0)
      .child(Button::new("Save").id("save").on_click(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
      }))
      .into()
  });
  assert_eq!(stage.fill_color("save"), None, "no fill");
  let (_, _, width, height) = stage.bounds("save");
  let (_, size) = stage.text("Save");
  assert_eq!(size, 14.0, "its label is body text");
  assert!(
    height < 30.0 && width < 60.0,
    "as large as its label: {width} by {height}"
  );
  stage.click("save");
  stage.key("Enter");
  assert_eq!(presses.load(Ordering::SeqCst), 2);
}

#[test]
fn a_look_is_built_on_the_bare_theme_with_public_api_alone() {
  let mut definition = ThemeDefinition::bare();
  definition.ui.set_recipe::<Button>(
    Recipe::new()
      .base(ButtonPart::Root, |s| {
        s.height(44.0)
          .padding_x(24.0)
          .radius(RadiusSize::Full)
          .background(Color::from_hex("#111827"))
          .foreground(Color::from_hex("#f9fafb"))
      })
      .hovered(ButtonPart::Root, |s| s.background(Color::from_hex("#374151")))
      .base(ButtonPart::Label, |s| s.font_size(15.0)),
  );
  let app = App::new();
  app.theme().apply(definition);
  let mut stage = Stage::in_app(app, |_| save());
  assert_eq!(stage.bounds("save").3, 44.0);
  assert_eq!(top_left_radius(&mut stage, "save"), 999.0);
  assert_eq!(stage.text("Save"), (Color::from_hex("#f9fafb"), 15.0));
  stage.hover("save");
  assert_eq!(stage.fill_color("save"), Some(Color::from_hex("#374151")));
}

#[test]
fn a_renderer_builds_other_nodes_for_the_same_behavior() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let app = App::new();
  // A button whose label is in capitals with a bar under it: other anatomy, the same recipe and behavior.
  let renderer: ButtonRenderer = Arc::new(|view: ButtonView| {
    let label = view.part(ButtonPart::Label).text(&view.label.to_uppercase());
    view
      .root()
      .child(Column::new().child(label).child(Rect::new(20.0, 2.0).id("bar")))
      .into()
  });
  app.theme().set_renderer::<Button>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let counted = counted.clone();
    Row::new()
      .padding(20.0)
      .child(Button::new("Save").id("save").on_click(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
      }))
      .into()
  });
  assert_eq!(
    stage.text("SAVE").0,
    light(PaletteColor::PrimaryForeground),
    "the recipe still styles it"
  );
  assert_eq!(stage.fill_color("save"), Some(light(PaletteColor::Primary)));
  stage.bounds("bar");
  stage.click("save");
  stage.key(" ");
  assert_eq!(
    presses.load(Ordering::SeqCst),
    2,
    "pressed by the pointer and by the keyboard"
  );
}

#[test]
fn a_renderer_for_one_part_leaves_the_rest_to_the_default_one() {
  let app = App::new();
  let label: ButtonPartRenderer = Arc::new(|view: &ButtonView| {
    Row::new()
      .child(Text::new("»"))
      .child(view.part(ButtonPart::Label).text(&view.label))
      .into()
  });
  app
    .theme()
    .set_part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Label, label);
  let mut stage = Stage::in_app(app, |_| save());
  stage.text("»");
  assert_eq!(stage.text("Save").1, 13.0);
  assert_eq!(
    stage.fill_color("save"),
    Some(light(PaletteColor::Primary)),
    "the root is the default one"
  );
  assert_eq!(stage.bounds("save").3, 36.0);
}

#[test]
fn one_instance_is_styled_over_the_theme() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(8.0)
      .child(
        Button::new("Wide")
          .id("own")
          .style(ButtonPart::Root, |s| s.padding_x(40.0).radius(0.0)),
      )
      .child(Button::new("Wide").id("plain"))
      .into()
  });
  assert_eq!(
    stage.bounds("own").2 - stage.bounds("plain").2,
    48.0,
    "24 more on each side"
  );
  assert_eq!(top_left_radius(&mut stage, "own"), 0.0);
  assert_eq!(top_left_radius(&mut stage, "plain"), 6.0);
}

#[test]
fn a_component_styles_the_buttons_in_its_places() {
  // What a message box does with its two buttons: each place has rules, and a button elsewhere has none.
  let confirm = Recipe::new().base(ButtonPart::Label, |s| s.font_family("Georgia"));
  let cancel = Recipe::new().base(ButtonPart::Label, |s| s.font_family(lurq::ui::FontFamily::Mono));
  let stage = Stage::new(move |_| {
    Row::new()
      .spacing(8.0)
      .child(Button::new("Cancel").variant(ButtonVariant::Outline).rules(&cancel))
      .child(Button::new("Confirm").rules(&confirm))
      .child(Button::new("Elsewhere"))
      .into()
  });
  assert_eq!(stage.font_of("Confirm"), "Georgia");
  assert_eq!(stage.font_of("Cancel"), "monospace");
  let system = stage.app.theme().sans_family().to_string();
  assert_eq!(stage.font_of("Elsewhere"), system);
}

#[test]
fn a_subtree_is_in_a_theme_of_its_own() {
  let dense = Theme::new();
  dense.patch_recipe::<Button>(|rules| rules.base(ButtonPart::Root, |s| s.height(24.0)));
  let mut stage = Stage::new(move |ctx| {
    let inspector = ctx.in_theme(&dense, |_| Button::new("Dense").id("dense"));
    Column::new()
      .spacing(8.0)
      .child(Button::new("Plain").id("plain"))
      .child(inspector)
      .child(Button::new("After").id("after"))
      .into()
  });
  assert_eq!(stage.bounds("dense").3, 24.0);
  assert_eq!(stage.bounds("plain").3, 36.0);
  assert_eq!(
    stage.bounds("after").3,
    36.0,
    "the theme around is back after the scope"
  );
}

#[test]
fn a_rule_for_a_breakpoint_reaches_a_button_when_the_window_is_that_wide() {
  use lurq::app::theme::Breakpoint;
  let app = App::new();
  // A finger's button on a narrow window, the design's from the medium breakpoint up.
  app.theme().patch_recipe::<Button>(|rules| {
    rules
      .base(ButtonPart::Root, |s| s.height(48.0))
      .at(Breakpoint::Md, ButtonPart::Root, |s| s.height(36.0))
  });
  let mut stage = Stage::in_app(app, |_| save());
  stage.tree.resize(400, 300);
  stage.pass();
  assert_eq!(stage.bounds("save").3, 48.0, "400 wide: below every breakpoint");
  stage.tree.resize(900, 300);
  stage.pass();
  assert_eq!(
    stage.bounds("save").3,
    36.0,
    "900 wide: past the medium one, and the button was built again"
  );
  stage.tree.resize(500, 300);
  stage.pass();
  assert_eq!(stage.bounds("save").3, 48.0);
}

#[test]
fn a_button_built_outside_a_render_is_in_a_neutral_theme() {
  let theme = current_theme();
  assert_eq!(*theme.palette(), ThemePalette::light());
  let button: Element = Button::new("Save").into();
  drop(button);
}

#[test]
fn a_rule_for_a_density_holds_while_the_theme_is_at_it() {
  use lurq::app::theme::Density;
  let app = App::new();
  // A theme whose buttons are as high as a row is at its density.
  app.theme().patch_recipe::<Button>(|rules| {
    rules
      .density(Density::Compact, ButtonPart::Root, |s| s.height(28.0))
      .density(Density::Touch, ButtonPart::Root, |s| s.height(48.0))
  });
  let theme = app.theme().clone();
  let mut stage = Stage::in_app(app, |_| save());
  assert_eq!(stage.bounds("save").3, 36.0, "the default density: the design's height");
  theme.set_density(Density::Compact);
  stage.pass();
  assert_eq!(stage.bounds("save").3, 28.0, "the button on screen follows the theme");
  theme.set_density(Density::Touch);
  stage.pass();
  assert_eq!(stage.bounds("save").3, 48.0);
}

#[test]
fn a_change_of_theme_is_there_at_once_and_a_change_of_state_still_fades() {
  use std::time::Duration;

  use lurq::app::window::Appearance;
  let dark = |role: PaletteColor| ThemePalette::dark().get(role);
  let app = App::new();
  let theme = app.theme().clone();
  theme.set_preferred_appearance(Some(Appearance::Light));
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(20.0)
      .spacing(40.0)
      .child(Button::new("View source").variant(ButtonVariant::Outline).id("view"))
      .child(Button::new("Save").id("save"))
      .into()
  });
  assert_eq!(stage.fill_color("view"), Some(light(PaletteColor::Background)));

  // The window turns dark: the buttons are dark in the frame that the rest of it is, not a fade later.
  theme.set_preferred_appearance(Some(Appearance::Dark));
  stage.pass_after(Duration::ZERO);
  for later in [0, 30, 60] {
    stage.pass_after(Duration::from_millis(later));
    assert_eq!(
      stage.fill_color("view"),
      Some(dark(PaletteColor::Background)),
      "the fill, {later} ms after the theme changed"
    );
    assert_eq!(stage.text("View source").0, dark(PaletteColor::Foreground));
    assert_eq!(stage.fill_color("save"), Some(dark(PaletteColor::Primary)));
  }

  // What the button does itself is still a transition: under the pointer its fill is on the way.
  stage.pass();
  let (x, y) = stage.center("save");
  stage.tree.mouse_move(x, y);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  let on_the_way = stage.fill_color("save").unwrap();
  assert_ne!(on_the_way, dark(PaletteColor::Primary), "it left its fill at rest");
  assert_ne!(
    on_the_way,
    dark(PaletteColor::PrimaryHover),
    "and is not at its hover fill yet"
  );
}

/// A theme in the dark appearance, with corners of its own, for a part of a light window.
fn dark_panel() -> Theme {
  use lurq::app::window::Appearance;
  let theme = Theme::new();
  theme.set_preferred_appearance(Some(Appearance::Dark));
  theme.set_radius_value(RadiusSize::Md, 10.0);
  theme
}

#[test]
fn a_theme_for_a_subtree_reaches_the_paint() {
  use lurq::app::{theme::TypographyStyle, window::Appearance};
  let dark = |role: PaletteColor| ThemePalette::dark().get(role);
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let panel = dark_panel();
  let mut title = panel.typography_style(TypographyStyle::Title);
  title.font_size = 30.0;
  panel.set_typography_style(TypographyStyle::Title, title);
  let scoped = panel.clone();
  let mut stage = Stage::in_app(app, move |ctx| {
    let side = ctx.in_theme(&scoped, |_| {
      Column::new()
        .id("side")
        .padding(12.0)
        .spacing(8.0)
        .background(PaletteColor::Background)
        .foreground(PaletteColor::Foreground)
        .child(Text::new("In the panel").variant(TypographyStyle::Title))
        .child(
          Rect::new(40.0, 20.0)
            .id("swatch")
            .background(PaletteColor::Muted)
            .rounded(RadiusSize::Md),
        )
        .child(Button::new("Save").id("inside"))
    });
    Column::new()
      .spacing(8.0)
      .child(Text::new("Before").variant(TypographyStyle::Title))
      .child(side)
      .child(
        Rect::new(40.0, 20.0)
          .id("outside")
          .background(PaletteColor::Muted)
          .rounded(RadiusSize::Md),
      )
      .child(Button::new("Send").id("after"))
      .into()
  });
  // What is in the panel is painted with the panel's palette, and laid out with its type and its corners.
  assert_eq!(stage.fill_color("side"), Some(dark(PaletteColor::Background)));
  assert_eq!(stage.text("In the panel"), (dark(PaletteColor::Foreground), 30.0));
  assert_eq!(stage.fill_color("swatch"), Some(dark(PaletteColor::Muted)));
  assert_eq!(top_left_radius(&mut stage, "swatch"), 10.0);
  assert_eq!(
    stage.fill_color("inside"),
    Some(dark(PaletteColor::Primary)),
    "a component in it"
  );
  assert_eq!(stage.text("Save").0, dark(PaletteColor::PrimaryForeground));
  assert_eq!(top_left_radius(&mut stage, "inside"), 10.0);
  // What is before it and after it is in the window's theme.
  assert_eq!(stage.text("Before"), (light(PaletteColor::Foreground), 20.0));
  assert_eq!(stage.fill_color("outside"), Some(light(PaletteColor::Muted)));
  assert_eq!(top_left_radius(&mut stage, "outside"), 6.0);
  assert_eq!(stage.fill_color("after"), Some(light(PaletteColor::Primary)));

  // The panel's theme changes while the app runs: the panel follows, and nothing else does.
  let red = Color::new(200, 30, 30, 255);
  panel.set_palette_color(PaletteColor::Muted, red);
  panel.set_radius_value(RadiusSize::Md, 2.0);
  stage.pass();
  assert_eq!(stage.fill_color("swatch"), Some(red));
  assert_eq!(top_left_radius(&mut stage, "swatch"), 2.0);
  assert_eq!(top_left_radius(&mut stage, "inside"), 2.0);
  assert_eq!(stage.fill_color("outside"), Some(light(PaletteColor::Muted)));
  assert_eq!(top_left_radius(&mut stage, "outside"), 6.0);
}

#[test]
fn an_element_is_given_a_theme_and_what_is_in_it_is_painted_with_it() {
  use lurq::app::window::Appearance;
  let dark = |role: PaletteColor| ThemePalette::dark().get(role);
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let panel = dark_panel();
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .child(
        Row::new()
          .theme(&panel)
          .child(Rect::new(40.0, 20.0).id("in").background(PaletteColor::Card)),
      )
      .child(Rect::new(40.0, 20.0).id("out").background(PaletteColor::Card))
      .into()
  });
  assert_eq!(stage.fill_color("in"), Some(dark(PaletteColor::Card)));
  assert_eq!(stage.fill_color("out"), Some(light(PaletteColor::Card)));
}

#[test]
fn a_transition_in_a_subtree_runs_between_the_colors_of_its_theme() {
  use std::time::Duration;

  use lurq::app::window::Appearance;
  let dark = |role: PaletteColor| ThemePalette::dark().get(role);
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let panel = dark_panel();
  let mut stage = Stage::in_app(app, move |ctx| {
    let inside = ctx.in_theme(&panel, |_| {
      Row::new().padding(20.0).child(Button::new("Save").id("save"))
    });
    Column::new().child(inside).into()
  });
  assert_eq!(stage.fill_color("save"), Some(dark(PaletteColor::Primary)));
  let (x, y) = stage.center("save");
  stage.tree.mouse_move(x, y);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  let on_the_way = stage.fill_color("save").unwrap();
  assert_ne!(on_the_way, dark(PaletteColor::Primary));
  assert_ne!(on_the_way, dark(PaletteColor::PrimaryHover));
  stage.pass();
  assert_eq!(
    stage.fill_color("save"),
    Some(dark(PaletteColor::PrimaryHover)),
    "the hover fill of the panel's theme, not of the window's"
  );
}

#[test]
fn a_popup_opened_in_a_subtree_is_painted_with_the_theme_of_the_subtree() {
  use lurq::{app::window::Appearance, components::Popup, core::ElementRef};
  let dark = |role: PaletteColor| ThemePalette::dark().get(role);
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let panel = dark_panel();
  let scoped = panel.clone();
  let anchor = ElementRef::new();
  let mut stage = Stage::in_app(app, move |ctx| {
    let anchor = anchor.clone();
    let inside = ctx.in_theme(&panel, |_| {
      Column::new()
        .child(Rect::new(150.0, 30.0).ref_element(anchor.clone()))
        .child(
          Popup::new(
            anchor.clone(),
            Rect::new(120.0, 40.0).id("menu").background(PaletteColor::Card),
          )
          .open_when(true),
        )
    });
    // The page has room under the anchor for what the popup shows.
    Column::new().size(300.0, 200.0).child(inside).into()
  });
  stage.pass();
  // What the popup shows is drawn over the tree, outside the element that carries the theme, and is
  // painted with that theme all the same.
  assert_eq!(stage.fill_color("menu"), Some(dark(PaletteColor::Card)));

  let red = Color::new(200, 30, 30, 255);
  scoped.set_palette_color(PaletteColor::Card, red);
  stage.pass();
  assert_eq!(stage.fill_color("menu"), Some(red), "and follows it");
}

#[test]
fn a_subtree_with_a_theme_is_laid_out_in_whatever_holds_it() {
  use lurq::{
    app::window::Appearance,
    components::{ScrollVertical, Stack},
  };
  let dark = |role: PaletteColor| ThemePalette::dark().get(role);
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let panel = dark_panel();
  let mut stage = Stage::in_app(app, move |ctx| {
    let mut swatch = |id: &str| {
      ctx.in_theme(&panel, |_| {
        Row::new()
          .padding(4.0)
          .child(Rect::new(20.0, 10.0).id(id).background(PaletteColor::Card))
      })
    };
    // A box with a size and padding of its own, a line that wraps, a stack and a scroll: each reads the
    // window's theme for itself and lets go of it before what is in it is laid out.
    Column::new()
      .size(300.0, 200.0)
      .padding(8.0)
      .spacing(4.0)
      .child(Column::new().width(100.0).padding(6.0).child(swatch("boxed")))
      .child(Row::new().width(100.0).spacing(4.0).wrap().child(swatch("wrapped")))
      .child(Stack::new().child(swatch("stacked")))
      .child(ScrollVertical::new(swatch("scrolled")).height(30.0))
      .child(Rect::new(20.0, 10.0).id("outside").background(PaletteColor::Card))
      .into()
  });
  for id in ["boxed", "wrapped", "stacked", "scrolled"] {
    assert_eq!(stage.fill_color(id), Some(dark(PaletteColor::Card)), "{id}");
  }
  assert_eq!(stage.fill_color("outside"), Some(light(PaletteColor::Card)));
}

#[test]
fn a_modal_declared_in_a_subtree_is_painted_with_the_theme_of_the_subtree() {
  use lurq::{
    app::window::Appearance,
    components::{Modal, Root},
  };
  let dark = |role: PaletteColor| ThemePalette::dark().get(role);
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let panel = dark_panel();
  let mut stage = Stage::in_app(app, move |ctx| {
    let inside = ctx.in_theme(&panel, |_| {
      Column::new().child(Modal::new(Rect::new(120.0, 40.0).id("dialog").background(PaletteColor::Card)).target(Root))
    });
    Column::new().size(300.0, 200.0).child(inside).into()
  });
  stage.pass();
  assert_eq!(stage.fill_color("dialog"), Some(dark(PaletteColor::Card)));
}

/// The color of an option in the open menu of a select: in a window of the dark appearance, or in a
/// subtree of the dark appearance in a light window.
fn option_color(scoped: bool) -> Color {
  use lurq::{
    app::{events::MouseButton, window::Appearance},
    components::Select,
    core::Signal,
  };
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(if scoped { Appearance::Light } else { Appearance::Dark }));
  let panel = dark_panel();
  let value = Signal::new("md".to_owned());
  let mut stage = Stage::in_app(app, move |ctx| {
    let select = || {
      Select::new(value.clone())
        .options(vec![
          ("md".to_owned(), "Medium".to_owned()),
          ("lg".to_owned(), "Large".to_owned()),
        ])
        .width(200.0)
        .height(40.0)
    };
    let held: Element = if scoped {
      ctx.in_theme(&panel, |_| select())
    } else {
      select().into()
    };
    Column::new().size(300.0, 200.0).child(held).into()
  });
  // The select is the first thing of the page: 200 by 40 at its corner.
  crate::support::pointer_click(&mut stage.tree, 100.0, 20.0, MouseButton::Left);
  stage.pass();
  stage.text("Large").0
}

#[test]
fn the_menu_of_a_select_in_a_subtree_is_painted_with_the_theme_of_the_subtree() {
  let in_a_dark_window = option_color(false);
  assert_eq!(option_color(true), in_a_dark_window);
  assert_ne!(in_a_dark_window, light(PaletteColor::Foreground));
}
