//! The colors of the engine's own styled pieces, from the theme's roles through a recipe.
//!
//! The phone's sheet, tab bar, navigation bar, pull to refresh and swipe row, and the engine's select, are not
//! components of the design system: their styles are structs of colors an app can set field by field
//! (`SheetStyle`, `TabBar`, `NavigationBar`, ...). What those structs start with is read here, from a recipe the
//! theme keeps under the piece's name ([`Themed`]): the design's roles in the neutral theme, the colors of 0.62 in
//! the legacy one ([`UiDefinition::legacy`](super::UiDefinition::legacy)), and what an app lays over either with
//! `theme.ui.patch::<SheetStyle>(..)`. A style is read where it is made, in the theme of what is being built, so
//! the light colors and the dark ones follow the theme's appearance.

use super::{
  recipe::{Case, Interaction, Name, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::node::{BackgroundColor, TextColor, color::Color};

/// A piece's recipe resolved in the theme of what is being built.
pub(crate) struct EngineLook {
  recipe: ResolvedRecipe,
}

impl EngineLook {
  /// The recipe of `C` in the theme of what is being built, with `states` on.
  pub(crate) fn of<C: Themed>(states: &[Name]) -> Self {
    let theme = current_theme();
    let mut case = Case::new();
    for state in states {
      case = case.state(*state, true);
    }
    Self {
      recipe: theme.resolve::<C>(&case, None),
    }
  }

  fn rest(&self, part: impl Into<Name>, interaction: Option<Interaction>) -> super::PartStyle {
    self.recipe.part(part).forced(interaction).rest()
  }

  fn palette_color(&self, color: Option<BackgroundColor>) -> Option<Color> {
    let palette = self.recipe.theme().palette();
    color.and_then(|color| color.resolve(&palette))
  }

  /// The part's fill, or `or` where the recipe gives none (a bare theme).
  pub(crate) fn background(&self, part: impl Into<Name>, or: Color) -> Color {
    self.palette_color(self.rest(part, None).background).unwrap_or(or)
  }

  /// The part's ink, or `or` where the recipe gives none.
  pub(crate) fn foreground(&self, part: impl Into<Name>, or: Color) -> Color {
    let foreground = self.rest(part, None).foreground.map(|color| match color {
      TextColor::Color(color) => BackgroundColor::Color(color),
      TextColor::Palette(role) => BackgroundColor::Palette(role),
    });
    self.palette_color(foreground).unwrap_or(or)
  }

  /// The part's opacity, or `or` where the recipe gives none.
  pub(crate) fn opacity(&self, part: impl Into<Name>, or: f32) -> f32 {
    self.rest(part, None).opacity_in(self.recipe.theme()).unwrap_or(or)
  }

  /// The part's fill as the recipe names it, a role staying a role, in an interaction.
  pub(crate) fn fill(&self, part: impl Into<Name>, interaction: Option<Interaction>) -> Option<BackgroundColor> {
    self.rest(part, interaction).background
  }

  /// The color of the part's edge as the recipe names it, in an interaction.
  pub(crate) fn edge(&self, part: impl Into<Name>, interaction: Option<Interaction>) -> Option<BackgroundColor> {
    self.rest(part, interaction).border.map(|border| border.color)
  }

  /// The part's ink as the recipe names it.
  pub(crate) fn ink(&self, part: impl Into<Name>) -> Option<TextColor> {
    self.rest(part, None).foreground
  }
}
