//! An outcome, the board's Result: its box against the board, the mark of each variant, the app's icon, what the app
//! puts under the text, one with no box, that only its buttons take the focus, a theme's rules and renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, RadiusSize, ThemePalette},
  },
  components::{Column, Rect},
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color, dimension::Dimension},
  ui::{Button, ButtonVariant, Outcome, OutcomePart, OutcomeRenderer, OutcomeVariant, OutcomeView, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find(|(held, _)| *held == name)
    .map(|(_, glyph)| glyph.to_string())
    .unwrap_or_else(|| panic!("no icon {name}"))
}

/// A column with room for an outcome of 300.
fn scene(children: Vec<Element>) -> Element {
  children
    .into_iter()
    .fold(Column::new().width(400.0).spacing(16.0), |column, child| {
      column.child(child)
    })
    .into()
}

fn built() -> Outcome {
  Outcome::new(OutcomeVariant::Success, "Build succeeded")
    .description("412 tests passed in 38 seconds.")
    .action(Button::new("View log").variant(ButtonVariant::Outline).id("log"))
    .action(Button::new("Deploy").id("deploy"))
    .id("built")
}

/// The circle of 48 inside the box of `id`: the mark.
fn mark_in(stage: &mut Stage, id: &str) -> Quad {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.width, quad.height) == (48.0, 48.0)
        && quad.x >= x
        && quad.y >= y
        && quad.x + quad.width <= x + width
        && quad.y + quad.height <= y + height
    })
    .unwrap_or_else(|| panic!("#{id} should have a mark"))
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

#[test]
fn an_outcome_is_a_box_of_300_padded_24_with_its_mark_text_and_buttons_16_apart() {
  let mut stage = Stage::new(|_| scene(vec![built().into()]));
  let (x, y, width, _) = stage.bounds("built");
  assert_eq!(width, 300.0);
  assert_eq!(stage.fill_color("built"), Some(light(PaletteColor::Background)));
  let edge = stage.border("built").expect("an edge");
  assert_eq!(
    edge.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Border))
  );
  let radius = stage.app.theme().radius_value(RadiusSize::Lg);
  assert!(edge.border_radius.is_some_and(|corners| corners.top_left == radius));

  let mark = mark_in(&mut stage, "built");
  assert_eq!(color_of(&mark), light(PaletteColor::SuccessMuted));
  assert!(
    mark.border_radius.is_some_and(|corners| corners.top_left >= 24.0),
    "round"
  );
  assert!((mark.y - y - 24.0).abs() < 0.01, "padded 24");
  assert!((mark.x + 24.0 - (x + 150.0)).abs() < 0.01, "centred");
  assert_eq!(stage.text(&glyph(icons::CHECK)), (light(PaletteColor::Success), 24.0));

  let title = stage.text_quad("Build succeeded").unwrap();
  assert_eq!(stage.text("Build succeeded"), (light(PaletteColor::Foreground), 16.0));
  assert!((title.y - (mark.y + 48.0) - 16.0).abs() < 0.5);
  assert_eq!(
    stage.text("412 tests passed in 38 seconds."),
    (light(PaletteColor::MutedForeground), 13.0)
  );

  // The buttons 8 apart, centred under the text.
  let (log_x, _, log_width, _) = stage.bounds("log");
  let (deploy_x, _, deploy_width, _) = stage.bounds("deploy");
  assert!((deploy_x - (log_x + log_width) - 8.0).abs() < 0.01);
  let middle = (log_x + deploy_x + deploy_width) / 2.0;
  assert!((middle - (x + 150.0)).abs() < 0.5, "centred: {middle}");
}

#[test]
fn each_variant_has_its_mark_and_its_colors() {
  let variants = [
    (
      OutcomeVariant::Success,
      "success",
      icons::CHECK,
      PaletteColor::SuccessMuted,
      PaletteColor::Success,
    ),
    (
      OutcomeVariant::Error,
      "error",
      icons::CLOSE,
      PaletteColor::DestructiveMuted,
      PaletteColor::Destructive,
    ),
    (
      OutcomeVariant::Warning,
      "warning",
      icons::WARNING,
      PaletteColor::WarningMuted,
      PaletteColor::Warning,
    ),
    (
      OutcomeVariant::Info,
      "info",
      icons::INFO,
      PaletteColor::InfoMuted,
      PaletteColor::Info,
    ),
    (
      OutcomeVariant::NotFound,
      "missing",
      icons::SEARCH_CLOSE,
      PaletteColor::Muted,
      PaletteColor::Foreground,
    ),
  ];
  for (variant, id, icon, fill, ink) in variants {
    let mut stage = Stage::new(move |_| scene(vec![Outcome::new(variant, "Over").id(id).into()]));
    let mark = mark_in(&mut stage, id);
    assert_eq!(color_of(&mark), light(fill), "{id}");
    assert_eq!(stage.text(&glyph(icon)), (light(ink), 24.0), "{id}");
  }
}

#[test]
fn the_apps_icon_takes_the_variants_colors_and_an_element_stands_in_place_of_the_mark() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Outcome::new(OutcomeVariant::Success, "Payment received")
        .icon(icons::CHECK_CIRCLE)
        .into(),
      Outcome::new(OutcomeVariant::Info, "Welcome")
        .media(Rect::new(120.0, 80.0).background(PaletteColor::Primary).id("picture"))
        .id("welcome")
        .into(),
    ])
  });
  assert_eq!(
    stage.text(&glyph(icons::CHECK_CIRCLE)),
    (light(PaletteColor::Success), 24.0)
  );
  assert!(!stage.paints(&glyph(icons::CHECK)));
  let (.., width, height) = stage.bounds("picture");
  assert_eq!((width, height), (120.0, 80.0));
  assert!(!stage.paints(&glyph(icons::INFO)), "no mark");
}

#[test]
fn what_the_app_puts_under_the_text_is_as_wide_as_the_outcome_and_over_the_buttons() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      built()
        .extra(Column::new().width(Dimension::Pct(100.0)).height(40.0).id("summary"))
        .into(),
    ])
  });
  let (_, summary_y, summary_width, _) = stage.bounds("summary");
  assert_eq!(summary_width, 300.0 - 48.0, "the width inside the padding");
  let description = stage.text_quad("412 tests passed in 38 seconds.").unwrap();
  assert!(summary_y > description.y + description.height + 15.0);
  let (_, button_y, ..) = stage.bounds("deploy");
  assert!(button_y > summary_y + 40.0 + 15.0);
}

#[test]
fn one_with_no_box_has_no_fill_no_edge_and_no_padding() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Outcome::new(OutcomeVariant::NotFound, "Nothing here")
        .bordered(false)
        .width(lurq::ui::Length::Fill)
        .id("missing")
        .into(),
    ])
  });
  let (_, y, width, _) = stage.bounds("missing");
  assert_eq!(width, 400.0, "as wide as told");
  assert!(stage.border("missing").is_none());
  assert!(stage.fill_color("missing").is_none());
  let mark = mark_in(&mut stage, "missing");
  assert!((mark.y - y).abs() < 0.01, "no padding");
}

#[test]
fn only_its_buttons_take_the_focus() {
  let mut stage = Stage::new(|_| scene(vec![built().into()]));
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("log"));
}

#[test]
fn a_theme_restyles_an_outcome_and_draws_another() {
  let app = App::new();
  app.theme().patch_recipe::<Outcome>(|rules| {
    rules.variant(OutcomeVariant::Success, OutcomePart::Mark, |s| {
      s.background(PaletteColor::Success)
    })
  });
  let mut stage = Stage::in_app(app, |_| scene(vec![built().into()]));
  let mark = mark_in(&mut stage, "built");
  assert_eq!(color_of(&mark), light(PaletteColor::Success));

  // A renderer that draws the title alone, in the root the recipe still styles.
  let app = App::new();
  let renderer: OutcomeRenderer = Arc::new(|view: OutcomeView| {
    let part = view.part(OutcomePart::Title);
    view.root().child(part.text(&format!("{}!", view.title))).into()
  });
  app.theme().set_renderer::<Outcome>(renderer);
  let mut stage = Stage::in_app(app, |_| scene(vec![built().into()]));
  assert!(stage.paints("Build succeeded!"));
  assert!(!stage.paints(&glyph(icons::CHECK)));
  assert_eq!(stage.fill_color("built"), Some(light(PaletteColor::Background)));
}
