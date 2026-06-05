use crate::node::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaletteColor {
  Accent,
  AccentHover,
  AccentMuted,
  SurfaceBase,
  SurfacePanel,
  SurfaceRaised,
  SurfaceInput,
  Border,
  BorderStrong,
  BorderFocus,
  TextPrimary,
  TextSecondary,
  TextMuted,
  TextInverse,
  Success,
  SuccessMuted,
  Warning,
  WarningMuted,
  Danger,
  DangerMuted,
  Info,
  InfoMuted,
}

impl PaletteColor {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Accent => "accent",
      Self::AccentHover => "accent_hover",
      Self::AccentMuted => "accent_muted",
      Self::SurfaceBase => "surface_base",
      Self::SurfacePanel => "surface_panel",
      Self::SurfaceRaised => "surface_raised",
      Self::SurfaceInput => "surface_input",
      Self::Border => "border",
      Self::BorderStrong => "border_strong",
      Self::BorderFocus => "border_focus",
      Self::TextPrimary => "text_primary",
      Self::TextSecondary => "text_secondary",
      Self::TextMuted => "text_muted",
      Self::TextInverse => "text_inverse",
      Self::Success => "success",
      Self::SuccessMuted => "success_muted",
      Self::Warning => "warning",
      Self::WarningMuted => "warning_muted",
      Self::Danger => "danger",
      Self::DangerMuted => "danger_muted",
      Self::Info => "info",
      Self::InfoMuted => "info_muted",
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemePalette {
  pub accent: Color,
  pub accent_hover: Color,
  pub accent_muted: Color,
  pub surface_base: Color,
  pub surface_panel: Color,
  pub surface_raised: Color,
  pub surface_input: Color,
  pub border: Color,
  pub border_strong: Color,
  pub border_focus: Color,
  pub text_primary: Color,
  pub text_secondary: Color,
  pub text_muted: Color,
  pub text_inverse: Color,
  pub success: Color,
  pub success_muted: Color,
  pub warning: Color,
  pub warning_muted: Color,
  pub danger: Color,
  pub danger_muted: Color,
  pub info: Color,
  pub info_muted: Color,
}

impl ThemePalette {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self, color: impl Into<PaletteColor>) -> Color {
    match color.into() {
      PaletteColor::Accent => self.accent,
      PaletteColor::AccentHover => self.accent_hover,
      PaletteColor::AccentMuted => self.accent_muted,
      PaletteColor::SurfaceBase => self.surface_base,
      PaletteColor::SurfacePanel => self.surface_panel,
      PaletteColor::SurfaceRaised => self.surface_raised,
      PaletteColor::SurfaceInput => self.surface_input,
      PaletteColor::Border => self.border,
      PaletteColor::BorderStrong => self.border_strong,
      PaletteColor::BorderFocus => self.border_focus,
      PaletteColor::TextPrimary => self.text_primary,
      PaletteColor::TextSecondary => self.text_secondary,
      PaletteColor::TextMuted => self.text_muted,
      PaletteColor::TextInverse => self.text_inverse,
      PaletteColor::Success => self.success,
      PaletteColor::SuccessMuted => self.success_muted,
      PaletteColor::Warning => self.warning,
      PaletteColor::WarningMuted => self.warning_muted,
      PaletteColor::Danger => self.danger,
      PaletteColor::DangerMuted => self.danger_muted,
      PaletteColor::Info => self.info,
      PaletteColor::InfoMuted => self.info_muted,
    }
  }

  pub fn set(&mut self, color: impl Into<PaletteColor>, value: Color) {
    match color.into() {
      PaletteColor::Accent => self.accent = value,
      PaletteColor::AccentHover => self.accent_hover = value,
      PaletteColor::AccentMuted => self.accent_muted = value,
      PaletteColor::SurfaceBase => self.surface_base = value,
      PaletteColor::SurfacePanel => self.surface_panel = value,
      PaletteColor::SurfaceRaised => self.surface_raised = value,
      PaletteColor::SurfaceInput => self.surface_input = value,
      PaletteColor::Border => self.border = value,
      PaletteColor::BorderStrong => self.border_strong = value,
      PaletteColor::BorderFocus => self.border_focus = value,
      PaletteColor::TextPrimary => self.text_primary = value,
      PaletteColor::TextSecondary => self.text_secondary = value,
      PaletteColor::TextMuted => self.text_muted = value,
      PaletteColor::TextInverse => self.text_inverse = value,
      PaletteColor::Success => self.success = value,
      PaletteColor::SuccessMuted => self.success_muted = value,
      PaletteColor::Warning => self.warning = value,
      PaletteColor::WarningMuted => self.warning_muted = value,
      PaletteColor::Danger => self.danger = value,
      PaletteColor::DangerMuted => self.danger_muted = value,
      PaletteColor::Info => self.info = value,
      PaletteColor::InfoMuted => self.info_muted = value,
    }
  }

  pub fn resolve(&self, color: impl Into<PaletteColor>) -> Color {
    self.get(color)
  }
}

impl Default for ThemePalette {
  fn default() -> Self {
    Self {
      accent: Color::from_hex("#2563eb"),
      accent_hover: Color::from_hex("#1d4ed8"),
      accent_muted: Color::from_hex("#dbeafe"),
      surface_base: Color::from_hex("#ffffff"),
      surface_panel: Color::from_hex("#f8fafc"),
      surface_raised: Color::from_hex("#ffffff"),
      surface_input: Color::from_hex("#ffffff"),
      border: Color::from_hex("#e2e8f0"),
      border_strong: Color::from_hex("#94a3b8"),
      border_focus: Color::from_hex("#2563eb"),
      text_primary: Color::from_hex("#0f172a"),
      text_secondary: Color::from_hex("#334155"),
      text_muted: Color::from_hex("#64748b"),
      text_inverse: Color::from_hex("#ffffff"),
      success: Color::from_hex("#16a34a"),
      success_muted: Color::from_hex("#dcfce7"),
      warning: Color::from_hex("#d97706"),
      warning_muted: Color::from_hex("#fef3c7"),
      danger: Color::from_hex("#dc2626"),
      danger_muted: Color::from_hex("#fee2e2"),
      info: Color::from_hex("#0284c7"),
      info_muted: Color::from_hex("#e0f2fe"),
    }
  }
}
