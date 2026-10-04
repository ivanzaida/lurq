//! The fill a built-in control paints where its style sets none.
//!
//! A control the app has not styled takes its fill from a theme role, like
//! the default `SelectStyle` (the checkbox box `SurfaceInput`, checked
//! `Accent`; the slider track `Border`, thumb `Accent`, as in the theme's
//! form roles). A part the app styled without a fill paints none, like a
//! button or text input without a background.

use super::{DEFAULT_TRANSPARENT_COLOR, LayoutEngine};
use crate::{app::theme::PaletteColor, node::color::Color};

impl LayoutEngine {
  pub(super) fn default_control_fill(&self, app_styled: bool, role: PaletteColor) -> Color {
    if app_styled {
      DEFAULT_TRANSPARENT_COLOR
    } else {
      self.palette.borrow().resolve(&role)
    }
  }
}
