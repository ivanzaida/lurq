//! The palette: a color for each role, in a light and a dark set.
//!
//! The roles are the design's (`pencil/lurq-design-system.pen`): surfaces
//! come in pairs with the color of what is drawn on them (`Card` and
//! `CardForeground`), and what a control fills with has a color for its hover.
//!
//! The roles of lurq 0.60 are deprecated. Each still has a color of its own,
//! so a palette can hold the 0.60 look exactly ([`ThemePalette::legacy`]), and
//! each is kept in step with the role that replaces it: setting the
//! replacement sets it too. Setting a deprecated role sets only that role.

use std::{collections::HashMap, sync::Arc};

use crate::node::color::Color;

/// Declares the roles once: the variants of [`PaletteColor`], the fields of
/// [`ThemePalette`], and the reading and writing of one by the other.
macro_rules! palette {
  (
    roles { $( $(#[$doc:meta])* $role:ident, $field:ident, $name:literal; )* }
    deprecated { $( $old:ident, $old_field:ident, $old_name:literal, $new:ident, $new_field:ident, $note:literal; )* }
  ) => {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum PaletteColor {
      $( $(#[$doc])* $role, )*
      $( #[deprecated(since = "0.61.0", note = $note)] $old, )*
      /// An application-defined color stored in [`ThemePalette::extra`].
      Extra(Arc<str>),
    }

    #[allow(deprecated)]
    impl PaletteColor {
      pub fn as_str(&self) -> &str {
        match self {
          $( Self::$role => $name, )*
          $( Self::$old => $old_name, )*
          Self::Extra(name) => name,
        }
      }

      /// The role that replaces this one, for a role of lurq 0.60 that is
      /// deprecated; `None` for every other.
      pub fn replacement(&self) -> Option<PaletteColor> {
        match self {
          $( Self::$old => Some(Self::$new), )*
          _ => None,
        }
      }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ThemePalette {
      $( $(#[$doc])* pub $field: Color, )*
      $( #[deprecated(since = "0.61.0", note = $note)] pub $old_field: Color, )*
      pub extra: HashMap<Arc<str>, Color>,
    }

    #[allow(deprecated)]
    impl ThemePalette {
      pub fn try_get(&self, color: impl Into<PaletteColor>) -> Option<Color> {
        match color.into() {
          $( PaletteColor::$role => Some(self.$field), )*
          $( PaletteColor::$old => Some(self.$old_field), )*
          PaletteColor::Extra(name) => self.extra.get(&name).copied(),
        }
      }

      /// Sets the color of one role. A deprecated role that the role replaces
      /// takes the color too, so what still reads it follows.
      pub fn set(&mut self, color: impl Into<PaletteColor>, value: Color) {
        let color = color.into();
        $( if color == PaletteColor::$new { self.$old_field = value; } )*
        match color {
          $( PaletteColor::$role => self.$field = value, )*
          $( PaletteColor::$old => self.$old_field = value, )*
          PaletteColor::Extra(name) => {
            self.extra.insert(name, value);
          }
        }
      }

      /// Every role with no color at all, to build a palette on.
      fn blank() -> Self {
        const NONE: Color = Color::new(0, 0, 0, 0);
        Self {
          $( $field: NONE, )*
          $( $old_field: NONE, )*
          extra: HashMap::new(),
        }
      }

      /// Gives every deprecated role the color of the role that replaces it.
      fn with_deprecated_roles_in_step(mut self) -> Self {
        $( self.$old_field = self.$new_field; )*
        self
      }

      /// The built-in roles and their names, deprecated ones last.
      pub fn roles() -> Vec<PaletteColor> {
        vec![ $( PaletteColor::$role, )* $( PaletteColor::$old, )* ]
      }
    }
  };
}

palette! {
  roles {
    /// The window behind everything.
    Background, background, "background";
    /// Text and icons on the background, and the color of text that names
    /// none: a `Text` with no color of its own and no `foreground` above it,
    /// whose typography style keeps the default text color.
    Foreground, foreground, "foreground";
    /// A surface raised off the background: a card, a panel.
    Card, card, "card";
    CardForeground, card_foreground, "card_foreground";
    /// A surface that floats over the page: a menu, a popover, a dialog.
    Popover, popover, "popover";
    PopoverForeground, popover_foreground, "popover_foreground";
    /// The main action: a primary button, a checked box, a selected tab.
    Primary, primary, "primary";
    PrimaryForeground, primary_foreground, "primary_foreground";
    PrimaryHover, primary_hover, "primary_hover";
    /// A quieter fill for an action that is not the main one.
    Secondary, secondary, "secondary";
    SecondaryForeground, secondary_foreground, "secondary_foreground";
    SecondaryHover, secondary_hover, "secondary_hover";
    /// A fill that recedes: a track, a skeleton, a disabled field.
    Muted, muted, "muted";
    /// Text that says less: a description, a placeholder, a caption.
    MutedForeground, muted_foreground, "muted_foreground";
    /// The fill of the item the pointer or the keyboard is on: a menu item, a
    /// list row. The design's `accent`.
    Highlight, highlight, "highlight";
    HighlightForeground, highlight_foreground, "highlight_foreground";
    /// An action that removes or cannot be undone, and an error.
    Destructive, destructive, "destructive";
    DestructiveForeground, destructive_foreground, "destructive_foreground";
    DestructiveHover, destructive_hover, "destructive_hover";
    /// The tinted background of an error message.
    DestructiveMuted, destructive_muted, "destructive_muted";
    Success, success, "success";
    SuccessMuted, success_muted, "success_muted";
    Warning, warning, "warning";
    WarningMuted, warning_muted, "warning_muted";
    Info, info, "info";
    InfoMuted, info_muted, "info_muted";
    /// A line between things: a separator, the edge of a card.
    Border, border, "border";
    /// The edge of a control that takes input: a field, a checkbox.
    Input, input, "input";
    /// The focus ring.
    Ring, ring, "ring";
    /// A veil over whatever is under the pointer, on any fill.
    StateHover, state_hover, "state_hover";
    /// A veil over whatever is pressed, on any fill.
    StatePressed, state_pressed, "state_pressed";
    /// The area behind panels and cards: a workspace, a side bar.
    Canvas, canvas, "canvas";
    /// The veil a dialog draws over the page.
    Scrim, scrim, "scrim";
    /// Selected text.
    Selection, selection, "selection";
    Chart1, chart_1, "chart_1";
    Chart2, chart_2, "chart_2";
    Chart3, chart_3, "chart_3";
    Chart4, chart_4, "chart_4";
    Chart5, chart_5, "chart_5";
    /// The tint of glass: what a backdrop blur is seen through.
    GlassThin, glass_thin, "glass_thin";
    GlassRegular, glass_regular, "glass_regular";
    GlassThick, glass_thick, "glass_thick";
  }
  deprecated {
    Accent, accent, "accent", Primary, primary, "use `Primary`; see the migration guide";
    AccentHover, accent_hover, "accent_hover", PrimaryHover, primary_hover, "use `PrimaryHover`; see the migration guide";
    AccentMuted, accent_muted, "accent_muted", Highlight, highlight, "use `Highlight`; see the migration guide";
    SurfaceBase, surface_base, "surface_base", Background, background, "use `Background`; see the migration guide";
    SurfacePanel, surface_panel, "surface_panel", Canvas, canvas, "use `Canvas`; see the migration guide";
    SurfaceRaised, surface_raised, "surface_raised", Card, card, "use `Card`; see the migration guide";
    SurfaceInput, surface_input, "surface_input", Background, background, "use `Background`; see the migration guide";
    BorderStrong, border_strong, "border_strong", Input, input, "use `Input`; see the migration guide";
    BorderFocus, border_focus, "border_focus", Ring, ring, "use `Ring`; see the migration guide";
    TextPrimary, text_primary, "text_primary", Foreground, foreground, "use `Foreground`; see the migration guide";
    TextSecondary, text_secondary, "text_secondary", MutedForeground, muted_foreground, "use `MutedForeground`; see the migration guide";
    TextMuted, text_muted, "text_muted", MutedForeground, muted_foreground, "use `MutedForeground`; see the migration guide";
    TextInverse, text_inverse, "text_inverse", PrimaryForeground, primary_foreground, "use `PrimaryForeground`; see the migration guide";
    Danger, danger, "danger", Destructive, destructive, "use `Destructive`; see the migration guide";
    DangerMuted, danger_muted, "danger_muted", DestructiveMuted, destructive_muted, "use `DestructiveMuted`; see the migration guide";
  }
}

impl PaletteColor {
  pub fn extra(name: impl Into<Arc<str>>) -> Self {
    Self::Extra(name.into())
  }
}

impl From<&PaletteColor> for PaletteColor {
  fn from(color: &PaletteColor) -> Self {
    color.clone()
  }
}

impl From<Arc<str>> for PaletteColor {
  fn from(name: Arc<str>) -> Self {
    Self::Extra(name)
  }
}

impl From<&str> for PaletteColor {
  fn from(name: &str) -> Self {
    Self::Extra(Arc::from(name))
  }
}

impl ThemePalette {
  /// The light palette of the design: [`Self::light`].
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self, color: impl Into<PaletteColor>) -> Color {
    let color = color.into();
    self
      .try_get(&color)
      .unwrap_or_else(|| panic!("palette color not found: {}", color.as_str()))
  }

  pub fn resolve(&self, color: impl Into<PaletteColor>) -> Color {
    self.get(color)
  }

  pub fn try_resolve(&self, color: impl Into<PaletteColor>) -> Option<Color> {
    self.try_get(color)
  }

  /// The color a text of `style` is drawn in when the text names none: the
  /// style's own, or the [`foreground`](Self::foreground) for a style that
  /// keeps the default text color and so names none either.
  pub(crate) fn text_color(&self, style: &crate::layout::text_style::TextStyle) -> Color {
    if style.color == crate::layout::text_style::DEFAULT_TEXT_COLOR {
      self.foreground
    } else {
      style.color
    }
  }

  /// The design's light palette, which is the default.
  pub fn light() -> Self {
    let hex = Color::from_hex;
    Self {
      background: hex("#FFFFFF"),
      foreground: hex("#09090B"),
      card: hex("#FFFFFF"),
      card_foreground: hex("#09090B"),
      popover: hex("#FFFFFF"),
      popover_foreground: hex("#09090B"),
      primary: hex("#18181B"),
      primary_foreground: hex("#FAFAFA"),
      primary_hover: hex("#3F3F46"),
      secondary: hex("#E4E4E9"),
      secondary_foreground: hex("#18181B"),
      secondary_hover: hex("#D4D4DB"),
      muted: hex("#E4E4E9"),
      muted_foreground: hex("#5C5C66"),
      highlight: hex("#E4E4E9"),
      highlight_foreground: hex("#18181B"),
      destructive: hex("#DC2626"),
      destructive_foreground: hex("#FAFAFA"),
      destructive_hover: hex("#B91C1C"),
      destructive_muted: hex("#FEE2E2"),
      success: hex("#16A34A"),
      success_muted: hex("#DCFCE7"),
      warning: hex("#B45309"),
      warning_muted: hex("#FEF3C7"),
      info: hex("#2563EB"),
      info_muted: hex("#DBEAFE"),
      border: hex("#CDCDD3"),
      input: hex("#9A9AA5"),
      ring: hex("#71717A"),
      state_hover: hex("#0000000A"),
      state_pressed: hex("#00000014"),
      canvas: hex("#F0F0F3"),
      scrim: hex("#00000066"),
      selection: hex("#BFDBFE"),
      chart_1: hex("#2563EB"),
      chart_2: hex("#0D9488"),
      chart_3: hex("#D97706"),
      chart_4: hex("#7C3AED"),
      chart_5: hex("#DB2777"),
      glass_thin: hex("#FFFFFF59"),
      glass_regular: hex("#FFFFFF99"),
      glass_thick: hex("#FFFFFFCC"),
      ..Self::blank()
    }
    .with_deprecated_roles_in_step()
  }

  /// The design's dark palette.
  pub fn dark() -> Self {
    let hex = Color::from_hex;
    Self {
      background: hex("#09090B"),
      foreground: hex("#FAFAFA"),
      card: hex("#18181B"),
      card_foreground: hex("#FAFAFA"),
      popover: hex("#18181B"),
      popover_foreground: hex("#FAFAFA"),
      primary: hex("#E4E4E7"),
      primary_foreground: hex("#18181B"),
      primary_hover: hex("#FAFAFA"),
      secondary: hex("#27272A"),
      secondary_foreground: hex("#FAFAFA"),
      secondary_hover: hex("#3F3F46"),
      muted: hex("#27272A"),
      muted_foreground: hex("#A1A1AA"),
      highlight: hex("#27272A"),
      highlight_foreground: hex("#FAFAFA"),
      destructive: hex("#EF4444"),
      destructive_foreground: hex("#FAFAFA"),
      destructive_hover: hex("#DC2626"),
      destructive_muted: hex("#3B1212"),
      success: hex("#22C55E"),
      success_muted: hex("#0F2E1B"),
      warning: hex("#F59E0B"),
      warning_muted: hex("#33240A"),
      info: hex("#60A5FA"),
      info_muted: hex("#132544"),
      border: hex("#27272A"),
      input: hex("#3F3F46"),
      ring: hex("#71717A"),
      state_hover: hex("#FFFFFF0F"),
      state_pressed: hex("#FFFFFF1F"),
      canvas: hex("#09090B"),
      scrim: hex("#00000099"),
      selection: hex("#1E3A8A"),
      chart_1: hex("#3B82F6"),
      chart_2: hex("#2DD4BF"),
      chart_3: hex("#F59E0B"),
      chart_4: hex("#A78BFA"),
      chart_5: hex("#F472B6"),
      glass_thin: hex("#18181B59"),
      glass_regular: hex("#18181B99"),
      glass_thick: hex("#18181BCC"),
      ..Self::blank()
    }
    .with_deprecated_roles_in_step()
  }

  /// The palette of lurq 0.60: every role it had, with the color it had.
  ///
  /// The roles 0.60 did not have take the 0.60 color nearest to what they
  /// are for (`primary` is the accent, `canvas` the panel surface), or the
  /// light design's where 0.60 had nothing like them (charts, glass, veils).
  /// Text that names no color is black, as it was.
  #[allow(deprecated)]
  pub fn legacy() -> Self {
    let hex = Color::from_hex;
    let light = Self::light();
    Self {
      accent: hex("#2563eb"),
      accent_hover: hex("#1d4ed8"),
      accent_muted: hex("#dbeafe"),
      surface_base: hex("#ffffff"),
      surface_panel: hex("#f8fafc"),
      surface_raised: hex("#ffffff"),
      surface_input: hex("#ffffff"),
      border: hex("#e2e8f0"),
      border_strong: hex("#94a3b8"),
      border_focus: hex("#2563eb"),
      text_primary: hex("#0f172a"),
      text_secondary: hex("#334155"),
      text_muted: hex("#64748b"),
      text_inverse: hex("#ffffff"),
      success: hex("#16a34a"),
      success_muted: hex("#dcfce7"),
      warning: hex("#d97706"),
      warning_muted: hex("#fef3c7"),
      danger: hex("#dc2626"),
      danger_muted: hex("#fee2e2"),
      info: hex("#0284c7"),
      info_muted: hex("#e0f2fe"),

      background: hex("#ffffff"),
      foreground: crate::layout::text_style::DEFAULT_TEXT_COLOR,
      card: hex("#ffffff"),
      card_foreground: hex("#0f172a"),
      popover: hex("#ffffff"),
      popover_foreground: hex("#0f172a"),
      primary: hex("#2563eb"),
      primary_foreground: hex("#ffffff"),
      primary_hover: hex("#1d4ed8"),
      secondary: hex("#f1f5f9"),
      secondary_foreground: hex("#0f172a"),
      secondary_hover: hex("#e2e8f0"),
      muted: hex("#f1f5f9"),
      muted_foreground: hex("#64748b"),
      highlight: hex("#dbeafe"),
      highlight_foreground: hex("#0f172a"),
      destructive: hex("#dc2626"),
      destructive_foreground: hex("#ffffff"),
      destructive_hover: hex("#b91c1c"),
      destructive_muted: hex("#fee2e2"),
      input: hex("#94a3b8"),
      ring: hex("#2563eb"),
      canvas: hex("#f8fafc"),
      ..light
    }
  }
}

impl Default for ThemePalette {
  fn default() -> Self {
    Self::light()
  }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
  use super::{PaletteColor, ThemePalette};
  use crate::node::color::Color;

  #[test]
  fn a_deprecated_role_has_the_color_of_its_replacement_in_the_design() {
    for palette in [ThemePalette::light(), ThemePalette::dark()] {
      for role in ThemePalette::roles() {
        if let Some(replacement) = role.replacement() {
          assert_eq!(palette.get(&role), palette.get(&replacement), "{}", role.as_str());
        }
      }
    }
  }

  #[test]
  fn every_role_of_the_design_has_a_color() {
    for (name, palette) in [("light", ThemePalette::light()), ("dark", ThemePalette::dark())] {
      for role in ThemePalette::roles() {
        // A veil is mostly clear; nothing is wholly so.
        assert_ne!(palette.get(&role).a(), 0, "{name} {}", role.as_str());
      }
    }
  }

  #[test]
  fn setting_a_role_sets_the_deprecated_roles_it_replaces() {
    let brand = Color::new(1, 2, 3, 255);
    let mut palette = ThemePalette::light();
    palette.set(PaletteColor::Background, brand);
    assert_eq!((palette.surface_base, palette.surface_input), (brand, brand));
    palette.set(PaletteColor::MutedForeground, brand);
    assert_eq!((palette.text_secondary, palette.text_muted), (brand, brand));
  }

  #[test]
  fn setting_a_deprecated_role_sets_only_that_role() {
    let brand = Color::new(1, 2, 3, 255);
    let mut palette = ThemePalette::light();
    let primary = palette.primary;
    palette.set(PaletteColor::Accent, brand);
    assert_eq!(palette.accent, brand);
    assert_eq!(palette.primary, primary);
  }

  #[test]
  fn the_legacy_palette_has_the_colors_of_0_60() {
    let legacy = ThemePalette::legacy();
    let hex = Color::from_hex;
    assert_eq!(legacy.accent, hex("#2563eb"));
    assert_eq!(legacy.text_secondary, hex("#334155"));
    assert_eq!(legacy.text_muted, hex("#64748b"));
    assert_eq!(legacy.border, hex("#e2e8f0"));
    assert_eq!(legacy.info, hex("#0284c7"));
    assert_eq!(legacy.foreground, Color::new(0, 0, 0, 255));
    // The roles 0.60 did not have take what it had nearest to them.
    assert_eq!(legacy.primary, legacy.accent);
    assert_eq!(legacy.canvas, legacy.surface_panel);
    assert_eq!(legacy.ring, legacy.border_focus);
  }
}
