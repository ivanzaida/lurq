//! The fill a built-in control paints where its style sets none.
//!
//! A checkbox or a slider node the app has not styled takes its fills from the theme, through the recipe the theme
//! keeps for it (`engine_checkbox`, `engine_slider`), as the default `SelectStyle` does: in the design's roles the
//! box is `Background` and checked `Primary`, the track `Border` and the thumb `Primary`; under
//! `ThemeDefinition::legacy()` they are 0.62's roles, which keep their own colors there. A part the app styled
//! without a fill paints none, like a button or text input without a background.

use super::{DEFAULT_TRANSPARENT_COLOR, LayoutEngine};
use crate::{
  app::theme::{PaletteColor, Theme},
  node::{BackgroundColor, color::Color},
  ui::{CheckboxPart, CheckboxState, Name, SliderPart, engine_look::EngineLook},
};

/// The roles 0.62 painted a checkbox and a slider node in: what the legacy theme gives, and what a theme with no
/// recipe for them keeps.
#[allow(deprecated)]
pub(crate) mod legacy {
  use crate::app::theme::PaletteColor;

  pub(crate) const BOX: PaletteColor = PaletteColor::SurfaceInput;
  pub(crate) const CHECKED: PaletteColor = PaletteColor::Accent;
  pub(crate) const TRACK: PaletteColor = PaletteColor::Border;
  pub(crate) const THUMB: PaletteColor = PaletteColor::Accent;
}

/// What a checkbox and a slider node fill with when nothing styles them, as a theme's recipes say.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ControlFills {
  pub(crate) checkbox: BackgroundColor,
  pub(crate) checked: BackgroundColor,
  pub(crate) track: BackgroundColor,
  pub(crate) thumb: BackgroundColor,
}

impl Default for ControlFills {
  fn default() -> Self {
    Self {
      checkbox: BackgroundColor::Palette(legacy::BOX),
      checked: BackgroundColor::Palette(legacy::CHECKED),
      track: BackgroundColor::Palette(legacy::TRACK),
      thumb: BackgroundColor::Palette(legacy::THUMB),
    }
  }
}

impl ControlFills {
  #[allow(deprecated)]
  pub(crate) fn of(theme: &Theme) -> Self {
    use crate::components::{Checkbox, Slider};
    let fill =
      |look: EngineLook, part: Name, or: PaletteColor| look.fill(part, None).unwrap_or(BackgroundColor::Palette(or));
    Self {
      checkbox: fill(
        EngineLook::in_theme::<Checkbox>(theme, &[]),
        CheckboxPart::Box.into(),
        legacy::BOX,
      ),
      checked: fill(
        EngineLook::in_theme::<Checkbox>(theme, &[CheckboxState::Checked.into()]),
        CheckboxPart::Box.into(),
        legacy::CHECKED,
      ),
      track: fill(
        EngineLook::in_theme::<Slider>(theme, &[]),
        SliderPart::Track.into(),
        legacy::TRACK,
      ),
      thumb: fill(
        EngineLook::in_theme::<Slider>(theme, &[]),
        SliderPart::Thumb.into(),
        legacy::THUMB,
      ),
    }
  }
}

impl LayoutEngine {
  pub(super) fn default_control_fill(
    &self,
    app_styled: bool,
    fill: impl FnOnce(&ControlFills) -> BackgroundColor,
  ) -> Color {
    if app_styled {
      DEFAULT_TRANSPARENT_COLOR
    } else {
      let fill = fill(&self.control_fills.borrow());
      fill
        .resolve(&self.palette.borrow())
        .unwrap_or(DEFAULT_TRANSPARENT_COLOR)
    }
  }
}
