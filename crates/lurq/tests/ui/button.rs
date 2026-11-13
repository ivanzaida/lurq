//! `ui::Button`: what it looks like in the design's theme, and what presses it.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{IconGlyph, OpacityLevel, PaletteColor, ThemePalette},
  components::{Column, Row},
  node::color::Color,
  ui::{Button, ButtonPart, ButtonShape, ButtonSize, ButtonState, ButtonVariant, Case, Interaction, Opacity, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

#[test]
fn a_button_is_as_the_design_draws_it() {
  let mut stage = Stage::new(|_| Row::new().child(Button::new("Save").id("save")).into());
  let (_, _, _, height) = stage.bounds("save");
  assert_eq!(height, 36.0);
  assert_eq!(stage.fill_color("save"), Some(light(PaletteColor::Primary)));
  let fill = stage.fill("save").unwrap();
  assert_eq!(fill.border_radius.map(|radius| radius.top_left), Some(6.0), "radius-md");
  let (ink, size) = stage.text("Save");
  assert_eq!(ink, light(PaletteColor::PrimaryForeground));
  assert_eq!(size, 13.0);
}

#[test]
fn each_variant_has_its_fill_and_its_ink() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(8.0)
      .child(
        Button::new("Secondary")
          .variant(ButtonVariant::Secondary)
          .id("secondary"),
      )
      .child(Button::new("Outline").variant(ButtonVariant::Outline).id("outline"))
      .child(Button::new("Ghost").variant(ButtonVariant::Ghost).id("ghost"))
      .child(
        Button::new("Delete")
          .variant(ButtonVariant::Destructive)
          .id("destructive"),
      )
      .into()
  });
  assert_eq!(stage.fill_color("secondary"), Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.text("Secondary").0, light(PaletteColor::SecondaryForeground));
  assert_eq!(stage.fill_color("destructive"), Some(light(PaletteColor::Destructive)));
  assert_eq!(stage.text("Delete").0, light(PaletteColor::DestructiveForeground));

  assert_eq!(stage.fill_color("outline"), Some(light(PaletteColor::Background)));
  let outline = stage.border("outline").expect("an outline button has a border");
  let edge = outline.border.and_then(|border| border.top).unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Input)));
  assert_eq!(stage.text("Outline").0, light(PaletteColor::Foreground));

  assert_eq!(stage.fill_color("ghost"), None, "a ghost button has no fill");
  assert_eq!(stage.text("Ghost").0, light(PaletteColor::Foreground));
}

#[test]
fn each_size_has_its_height_its_padding_and_its_label() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(8.0)
      .child(Button::new("Tiny").size(ButtonSize::Xs).id("xs"))
      .child(Button::new("Small").size(ButtonSize::Sm).id("sm"))
      .child(Button::new("Medium").size(ButtonSize::Md).id("md"))
      .child(Button::new("Large").size(ButtonSize::Lg).id("lg"))
      .child(Button::new("Huge").size(ButtonSize::Xl).id("xl"))
      .into()
  });
  let heights = ["xs", "sm", "md", "lg", "xl"].map(|id| stage.bounds(id).3);
  assert_eq!(heights, [28.0, 32.0, 36.0, 40.0, 44.0]);
  let labels = ["Tiny", "Small", "Medium", "Large", "Huge"].map(|label| stage.text(label).1);
  assert_eq!(labels, [12.0, 13.0, 13.0, 13.0, 15.0]);
  // What is beside the label: the padding of the size, on each side.
  let padding = |stage: &mut Stage, id: &str, label: &str| {
    let width = stage.bounds(id).2;
    let text = stage.text_quad(label).unwrap().width;
    ((width - text) / 2.0).round()
  };
  assert_eq!(padding(&mut stage, "xs", "Tiny"), 10.0);
  assert_eq!(padding(&mut stage, "sm", "Small"), 12.0);
  assert_eq!(padding(&mut stage, "md", "Medium"), 16.0);
  assert_eq!(padding(&mut stage, "lg", "Large"), 20.0);
  assert_eq!(padding(&mut stage, "xl", "Huge"), 24.0);
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

#[test]
fn an_icon_is_drawn_before_the_label_in_its_color_at_the_size_of_the_button() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(8.0)
      .child(Button::new("Tiny").size(ButtonSize::Xs).icon(icons::PLUS).id("xs"))
      .child(Button::new("New window").icon(icons::WINDOW).id("md"))
      .child(Button::new("Large").size(ButtonSize::Lg).icon(icons::DOWNLOAD).id("lg"))
      .child(Button::new("Huge").size(ButtonSize::Xl).icon(icons::TRASH).id("xl"))
      .child(Button::new("Next").trailing_icon(icons::ARROW_RIGHT).id("next"))
      .into()
  });
  let sides = [icons::PLUS, icons::WINDOW, icons::DOWNLOAD, icons::TRASH].map(|name| stage.text(&glyph(name)).1);
  assert_eq!(sides, [14.0, 16.0, 18.0, 20.0], "the icon of each size");
  let (ink, _) = stage.text(&glyph(icons::WINDOW));
  assert_eq!(ink, light(PaletteColor::PrimaryForeground), "the color of the label");
  assert_eq!(stage.font_of(&glyph(icons::WINDOW)), icons::FAMILY);

  let icon = stage.text_quad(&glyph(icons::WINDOW)).unwrap();
  let label = stage.text_quad("New window").unwrap();
  assert_eq!(
    label.x - (icon.x + icon.width),
    8.0,
    "the gap between the icon and the label"
  );
  assert_eq!(icon.x - stage.bounds("md").0, 16.0, "the padding before the icon");

  let arrow = stage.text_quad(&glyph(icons::ARROW_RIGHT)).unwrap();
  let next = stage.text_quad("Next").unwrap();
  assert_eq!(
    arrow.x - (next.x + next.width),
    8.0,
    "a trailing icon is after the label"
  );
}

#[test]
fn a_button_that_is_an_icon_alone_is_a_square_of_its_size() {
  let mut stage = Stage::new(|_| {
    let only = |size: ButtonSize, id: &'static str| {
      Button::new("Settings")
        .variant(ButtonVariant::Outline)
        .size(size)
        .icon(icons::SETTINGS)
        .icon_only(true)
        .id(id)
    };
    Row::new()
      .spacing(8.0)
      .child(only(ButtonSize::Xs, "xs"))
      .child(only(ButtonSize::Sm, "sm"))
      .child(only(ButtonSize::Md, "md"))
      .child(only(ButtonSize::Lg, "lg"))
      .child(only(ButtonSize::Xl, "xl"))
      .into()
  });
  for (id, side) in [("xs", 28.0), ("sm", 32.0), ("md", 36.0), ("lg", 40.0), ("xl", 44.0)] {
    let (x, y, width, height) = stage.bounds(id);
    assert_eq!((width, height), (side, side), "{id} is a square");
    let icon = stage
      .quads()
      .into_iter()
      .find(|quad| quad.x >= x && quad.x < x + width && quad.y >= y && quad.y < y + height && is_text(quad))
      .unwrap_or_else(|| panic!("{id} draws its icon"));
    assert_eq!(
      (icon.x + icon.width / 2.0, icon.y + icon.height / 2.0),
      (x + side / 2.0, y + side / 2.0),
      "the icon is in the middle of {id}"
    );
  }
  assert!(!stage.paints("Settings"), "its label is not drawn");
}

fn is_text(quad: &lurq::layout::quad::Quad) -> bool {
  matches!(quad.content, lurq::layout::quad::QuadContent::Text { .. })
}

#[test]
fn a_pill_is_rounded_all_the_way_and_an_icon_alone_in_it_is_a_circle() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .spacing(8.0)
      .child(Button::new("Pill").shape(ButtonShape::Pill).id("pill"))
      .child(
        Button::new("Add")
          .variant(ButtonVariant::Secondary)
          .icon(icons::PLUS)
          .icon_only(true)
          .shape(ButtonShape::Pill)
          .id("round"),
      )
      .into()
  });
  let radius = |stage: &mut Stage, id: &str| stage.fill(id).unwrap().border_radius.unwrap().top_left;
  assert_eq!(radius(&mut stage, "pill"), 999.0, "the theme's full radius");
  assert_eq!(radius(&mut stage, "round"), 999.0);
  let (_, _, width, height) = stage.bounds("round");
  assert_eq!((width, height), (36.0, 36.0));
}

#[test]
fn a_loading_button_turns_a_spinner_in_the_place_of_its_icon_and_is_not_pressed() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    Column::new()
      .spacing(8.0)
      .child(
        Button::new("Saving…")
          .icon(icons::DOWNLOAD)
          .loading(true)
          .id("saving")
          .on_click(move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
          }),
      )
      .child(Button::new("Wait").loading(true).id("plain"))
      .into()
  });
  assert!(
    !stage.paints(&glyph(icons::DOWNLOAD)),
    "the spinner stands where the icon was"
  );
  let spinners: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if *text == glyph(icons::SPINNER)))
    .collect();
  assert_eq!(spinners.len(), 2, "a button with no icon gets one before its label");
  assert!(stage.paints("Saving…"), "the label stays");

  // It turns: a quarter of its period on, it is drawn turned about its middle.
  stage.pass_after(lurq::ui::SPIN_PERIOD / 4);
  let turned = stage.text_quad(&glyph(icons::SPINNER)).unwrap();
  assert!(!turned.transform.is_identity(), "the spinner is turning");

  stage.click("saving");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(presses.load(Ordering::SeqCst), 0, "it is not pressed while it loads");

  let theme = stage.app.theme().clone();
  let case = Case::new()
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Md)
    .state(ButtonState::Loading, true);
  let root = theme.resolve::<Button>(&case, None).part(ButtonPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Value(0.75)), "the design's: three quarters");
}

#[test]
fn a_theme_gives_a_buttons_icon_another_glyph_and_its_part_another_size() {
  let app = lurq::app::App::new();
  app.theme().set_icon(icons::PLUS, IconGlyph::new("brand icons", 'A'));
  app.theme().patch_recipe::<Button>(|rules| {
    rules.base(ButtonPart::Icon, |s| {
      s.square(24.0).foreground(Color::new(200, 30, 30, 255))
    })
  });
  let stage = Stage::in_app(app, |_| Row::new().child(Button::new("Add").icon(icons::PLUS)).into());
  let (ink, size) = stage.text("A");
  assert_eq!(size, 24.0);
  assert_eq!(
    ink,
    Color::new(200, 30, 30, 255),
    "the part's own color over the label's"
  );
  assert_eq!(stage.font_of("A"), "brand icons");
}

#[test]
fn a_button_under_the_pointer_has_its_hover_fill_and_no_render_makes_it() {
  let renders = Arc::new(AtomicUsize::new(0));
  let counted = renders.clone();
  let mut stage = Stage::new(move |_| {
    counted.fetch_add(1, Ordering::SeqCst);
    Row::new().padding(20.0).child(Button::new("Save").id("save")).into()
  });
  let before = renders.load(Ordering::SeqCst);
  stage.hover("save");
  assert_eq!(stage.fill_color("save"), Some(light(PaletteColor::PrimaryHover)));
  assert_eq!(
    renders.load(Ordering::SeqCst),
    before,
    "a hover is a state style of the node"
  );

  stage.tree.mouse_move(390.0, 290.0);
  stage.pass();
  assert_eq!(stage.fill_color("save"), Some(light(PaletteColor::Primary)));
}

#[test]
fn a_click_and_the_keyboard_press_a_button() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    Row::new()
      .child(Button::new("Save").id("save").on_click(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
      }))
      .into()
  });
  stage.click("save");
  assert_eq!(presses.load(Ordering::SeqCst), 1);
  // The click left the button focused.
  stage.key("Enter");
  stage.key(" ");
  assert_eq!(presses.load(Ordering::SeqCst), 3);
}

#[test]
fn a_disabled_button_is_dimmed_and_takes_nothing() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    Row::new()
      .child(Button::new("Save").id("save").disabled(true).on_click(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
      }))
      .into()
  });
  stage.click("save");
  assert_eq!(presses.load(Ordering::SeqCst), 0);
  stage.hover("save");
  assert_eq!(
    stage.fill_color("save"),
    Some(light(PaletteColor::Primary)),
    "no hover fill"
  );
  stage.key("Tab");
  assert!(!stage.has_ring(light(PaletteColor::Ring)), "Tab passes it by");

  // Its root is dimmed by the theme's opacity for what is disabled. The button is a layer of its fill and
  // its label, so the opacity is the layer's and not a quad's.
  let theme = stage.app.theme().clone();
  let case = Case::new()
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Md)
    .state(ButtonState::Disabled, true);
  let root = theme.resolve::<Button>(&case, None).part(ButtonPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  assert_eq!(theme.opacity_value(OpacityLevel::Disabled), 0.5);
}

#[test]
fn a_button_focused_from_the_keyboard_has_the_themes_ring() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(20.0)
      .spacing(8.0)
      .child(Button::new("One").id("one"))
      .child(Button::new("Two").id("two"))
      .into()
  });
  let ring = light(PaletteColor::Ring);
  stage.click("one");
  assert!(!stage.has_ring(ring), "a click focuses without a ring");
  stage.key("Tab");
  assert!(stage.has_ring(ring), "Tab moves the focus and shows it");
}

#[test]
fn a_forced_interaction_is_drawn_with_nothing_pointing_at_the_button() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(20.0)
      .spacing(12.0)
      .child(Button::new("Hover").id("hover").force(Some(Interaction::Hovered)))
      .child(Button::new("Focus").id("focus").force(Some(Interaction::FocusVisible)))
      .child(Button::new("Rest").id("rest"))
      .into()
  });
  assert_eq!(stage.fill_color("hover"), Some(light(PaletteColor::PrimaryHover)));
  assert_eq!(stage.fill_color("rest"), Some(light(PaletteColor::Primary)));
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the focus ring of the forced one"
  );
}

#[test]
fn a_full_width_button_is_as_wide_as_what_it_is_in() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(Button::new("Continue").id("wide").full_width(true))
      .child(Button::new("Continue").id("hug"))
      .into()
  });
  assert_eq!(stage.bounds("wide").2, 300.0);
  assert!(stage.bounds("hug").2 < 300.0);
}

#[test]
fn what_is_before_and_after_the_label_is_in_the_button() {
  use lurq::components::Rect;
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(
        Button::new("Export")
          .id("export")
          .leading(Rect::new(16.0, 16.0).id("icon"))
          .trailing(Rect::new(16.0, 16.0).id("chevron")),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("export");
  let icon = stage.bounds("icon");
  let chevron = stage.bounds("chevron");
  assert_eq!(icon.0, x + 16.0, "after the button's padding");
  assert_eq!(chevron.0 + chevron.2, x + width - 16.0);
  assert!(icon.0 < chevron.0);
}
