use super::{PaletteColor, RadiusSize, SpacingSize, ThemePalette, ThemeTypography, TypographyStyle};
use crate::{
  layout::text_style::TextStyle,
  node::{CheckboxStyle, SliderPartStyle, dimension::Dimension, padding::Padding, spacing_value::SpacingValue},
};

#[derive(Clone)]
pub struct FormTheme {
  pub field: FormFieldTheme,
  pub input: FormInputTheme,
  pub checkbox: FormCheckboxStyle,
  pub slider: FormSliderStyle,
  pub button: FormButtonTheme,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FormTextRole {
  pub typography: TypographyStyle,
  pub color: PaletteColor,
}

#[derive(Clone, PartialEq)]
pub struct FormFieldTheme {
  pub spacing: SpacingValue,
  pub label: FormTextRole,
  pub hint: FormTextRole,
  pub error: FormTextRole,
}

#[derive(Clone, PartialEq)]
pub struct FormInputTheme {
  pub height: Dimension,
  pub padding: Padding,
  pub radius: RadiusSize,
  pub background: PaletteColor,
  pub border: PaletteColor,
  pub border_focus: PaletteColor,
  pub background_error: PaletteColor,
  pub border_error: PaletteColor,
  pub text: FormTextRole,
  pub placeholder: FormTextRole,
  pub caret: PaletteColor,
}

#[derive(Clone)]
pub struct FormCheckboxStyle {
  pub background: PaletteColor,
  pub border: PaletteColor,
  pub border_hover: PaletteColor,
  pub checked_background: PaletteColor,
  pub checked_border: PaletteColor,
  pub checked_background_hover: PaletteColor,
  pub radius: RadiusSize,
}

#[derive(Clone)]
pub struct FormSliderStyle {
  pub track: PaletteColor,
  pub track_hover: PaletteColor,
  pub thumb: PaletteColor,
  pub thumb_hover: PaletteColor,
}

#[derive(Clone, PartialEq)]
pub struct FormButtonTheme {
  pub primary: FormButtonRole,
  pub secondary: FormButtonRole,
}

#[derive(Clone, PartialEq)]
pub struct FormButtonRole {
  pub width: Dimension,
  pub height: Dimension,
  pub padding: Padding,
  pub radius: RadiusSize,
  pub background: PaletteColor,
  pub border: PaletteColor,
  pub background_hover: PaletteColor,
  pub border_hover: PaletteColor,
  pub background_active: PaletteColor,
  pub border_active: PaletteColor,
  pub text: FormTextRole,
}

impl FormTextRole {
  pub fn resolve(&self, typography: &ThemeTypography, palette: &ThemePalette) -> TextStyle {
    let mut style = typography.resolve(self.typography);
    style.color = palette.resolve(self.color);
    style
  }
}

impl FormCheckboxStyle {
  pub fn box_style(&self, palette: &ThemePalette) -> CheckboxStyle {
    CheckboxStyle::new()
      .size(16.0, 16.0)
      .background(palette.resolve(self.background))
      .rounded(self.radius)
      .border_inside(1.0, self.border)
  }

  pub fn checked_box_style(&self, palette: &ThemePalette) -> CheckboxStyle {
    CheckboxStyle::new()
      .background(palette.resolve(self.checked_background))
      .border_inside(1.0, self.checked_border)
      .indicator_size(8.0, 8.0)
  }

  pub fn box_hovered_style(&self) -> CheckboxStyle {
    CheckboxStyle::new().border_inside(1.0, self.border_hover)
  }

  pub fn checked_box_hovered_style(&self, palette: &ThemePalette) -> CheckboxStyle {
    CheckboxStyle::new().background(palette.resolve(self.checked_background_hover))
  }
}

impl FormSliderStyle {
  pub fn track_style(&self, palette: &ThemePalette) -> SliderPartStyle {
    SliderPartStyle::new()
      .height(4.0)
      .background(palette.resolve(self.track))
      .rounded(999.0)
  }

  pub fn track_hovered_style(&self, palette: &ThemePalette) -> SliderPartStyle {
    SliderPartStyle::new().background(palette.resolve(self.track_hover))
  }

  pub fn thumb_style(&self, palette: &ThemePalette) -> SliderPartStyle {
    SliderPartStyle::new()
      .size(16.0, 16.0)
      .background(palette.resolve(self.thumb))
      .rounded(999.0)
  }

  pub fn thumb_hovered_style(&self, palette: &ThemePalette) -> SliderPartStyle {
    SliderPartStyle::new().background(palette.resolve(self.thumb_hover))
  }
}

impl Default for FormTheme {
  fn default() -> Self {
    Self {
      field: FormFieldTheme::default(),
      input: FormInputTheme::default(),
      checkbox: FormCheckboxStyle::default(),
      slider: FormSliderStyle::default(),
      button: FormButtonTheme::default(),
    }
  }
}

impl Default for FormFieldTheme {
  fn default() -> Self {
    Self {
      spacing: SpacingValue::from(SpacingSize::Xs),
      label: FormTextRole {
        typography: TypographyStyle::FieldLabel,
        color: PaletteColor::TextPrimary,
      },
      hint: FormTextRole {
        typography: TypographyStyle::Caption,
        color: PaletteColor::TextMuted,
      },
      error: FormTextRole {
        typography: TypographyStyle::Caption,
        color: PaletteColor::Danger,
      },
    }
  }
}

impl Default for FormInputTheme {
  fn default() -> Self {
    Self {
      height: Dimension::Px(36.0),
      padding: Padding::symmetric(10.0, 8.0),
      radius: RadiusSize::Md,
      background: PaletteColor::SurfaceInput,
      border: PaletteColor::Border,
      border_focus: PaletteColor::BorderFocus,
      background_error: PaletteColor::DangerMuted,
      border_error: PaletteColor::Danger,
      text: FormTextRole {
        typography: TypographyStyle::Body,
        color: PaletteColor::TextPrimary,
      },
      placeholder: FormTextRole {
        typography: TypographyStyle::Body,
        color: PaletteColor::TextMuted,
      },
      caret: PaletteColor::BorderFocus,
    }
  }
}

impl Default for FormCheckboxStyle {
  fn default() -> Self {
    Self {
      background: PaletteColor::SurfaceInput,
      border: PaletteColor::Border,
      border_hover: PaletteColor::BorderFocus,
      checked_background: PaletteColor::Accent,
      checked_border: PaletteColor::Accent,
      checked_background_hover: PaletteColor::AccentHover,
      radius: RadiusSize::Sm,
    }
  }
}

impl Default for FormSliderStyle {
  fn default() -> Self {
    Self {
      track: PaletteColor::Border,
      track_hover: PaletteColor::BorderStrong,
      thumb: PaletteColor::Accent,
      thumb_hover: PaletteColor::AccentHover,
    }
  }
}

impl Default for FormButtonTheme {
  fn default() -> Self {
    Self {
      primary: FormButtonRole::primary(),
      secondary: FormButtonRole::secondary(),
    }
  }
}

impl FormButtonRole {
  pub fn primary() -> Self {
    Self {
      width: Dimension::Pct(100.0),
      height: Dimension::Px(36.0),
      padding: Padding::symmetric(14.0, 8.0),
      radius: RadiusSize::Md,
      background: PaletteColor::Accent,
      border: PaletteColor::Accent,
      background_hover: PaletteColor::AccentHover,
      border_hover: PaletteColor::AccentHover,
      background_active: PaletteColor::AccentHover,
      border_active: PaletteColor::AccentHover,
      text: FormTextRole {
        typography: TypographyStyle::Button,
        color: PaletteColor::TextInverse,
      },
    }
  }

  pub fn secondary() -> Self {
    Self {
      width: Dimension::Pct(100.0),
      height: Dimension::Px(36.0),
      padding: Padding::symmetric(14.0, 8.0),
      radius: RadiusSize::Md,
      background: PaletteColor::SurfaceInput,
      border: PaletteColor::BorderStrong,
      background_hover: PaletteColor::SurfacePanel,
      border_hover: PaletteColor::BorderStrong,
      background_active: PaletteColor::Border,
      border_active: PaletteColor::BorderStrong,
      text: FormTextRole {
        typography: TypographyStyle::Button,
        color: PaletteColor::TextPrimary,
      },
    }
  }
}
