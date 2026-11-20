//! A part's foreground over the color of its typography role: a recipe that sets a part's `foreground` colors its
//! text even where the theme's role names a color, and the role's color stands where the part sets none.

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette, TypographyStyle},
  },
  components::Row,
  node::color::Color,
  ui::{Badge, BadgePart, Button, ButtonPart, Recipe},
};

use super::stage::Stage;

/// The color of the role the tests give Caption.
const ROLE: Color = Color::new(200, 30, 40, 255);

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// An app whose Caption names a color.
fn colored_caption() -> App {
  let app = App::new();
  let mut caption = app.theme().typography_style(TypographyStyle::Caption);
  caption.color = ROLE;
  app.theme().set_typography_style(TypographyStyle::Caption, caption);
  app
}

#[test]
fn a_parts_foreground_colors_its_text_over_the_color_of_its_role() {
  let stage = Stage::in_app(colored_caption(), |_| {
    Row::new()
      .child(Badge::new("Ours").style(BadgePart::Label, |s| {
        s.typography(TypographyStyle::Caption).foreground(PaletteColor::Primary)
      }))
      .child(Badge::new("Role's").style(BadgePart::Label, |s| s.typography(TypographyStyle::Caption)))
      .into()
  });
  assert_eq!(stage.text("Ours").0, light(PaletteColor::Primary), "the part's foreground");
  assert_eq!(stage.text("Role's").0, ROLE, "the role's color where the part sets none");
}

#[test]
fn a_foreground_of_a_state_alone_leaves_the_roles_color_at_rest() {
  let mut stage = Stage::in_app(colored_caption(), |_| {
    Row::new()
      .padding(20.0)
      .child(
        Button::new("Save")
          .id("save")
          .style(ButtonPart::Label, |s| s.typography(TypographyStyle::Caption))
          .rules(&Recipe::new().hovered(ButtonPart::Label, |s| s.foreground(PaletteColor::Destructive))),
      )
      .into()
  });
  assert_eq!(stage.text("Save").0, ROLE, "at rest, the role's");
  stage.hover("save");
  assert_eq!(
    stage.text("Save").0,
    light(PaletteColor::Destructive),
    "under the pointer, the state's"
  );
}
