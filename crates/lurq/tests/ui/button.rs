//! `ui::Button`: what it looks like in the design's theme, and what presses it.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{OpacityLevel, PaletteColor, ThemePalette},
  components::{Column, Row},
  node::color::Color,
  ui::{Button, ButtonPart, ButtonSize, ButtonState, ButtonVariant, Case, Interaction, Opacity},
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
fn each_size_is_a_step_of_the_control_scale() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(8.0)
      .child(Button::new("Small").size(ButtonSize::Sm).id("sm"))
      .child(Button::new("Medium").size(ButtonSize::Md).id("md"))
      .child(Button::new("Large").size(ButtonSize::Lg).id("lg"))
      .into()
  });
  let heights = ["sm", "md", "lg"].map(|id| stage.bounds(id).3);
  assert_eq!(heights, [32.0, 36.0, 40.0]);
  assert!(
    stage.bounds("sm").2 < stage.bounds("lg").2,
    "a larger button is padded more"
  );
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
