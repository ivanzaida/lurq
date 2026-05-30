use lurq::{
  layout::text_style::{FontWeight, TextStyle},
  node::color::Color,
};

pub(crate) const BG: &str = "#0f172a";
pub(crate) const SURFACE: &str = "#1e293b";
pub(crate) const SURFACE_DARK: &str = "#162032";
pub(crate) const BORDER: &str = "#334155";
pub(crate) const TEXT: &str = "#f8fafc";
pub(crate) const TEXT_MUTED: &str = "#64748b";
pub(crate) const PRIMARY: &str = "#3b82f6";
pub(crate) const NAV_SELECTED: &str = "#1e3a5f";
pub(crate) const ERROR: &str = "#ef4444";
pub(crate) const SUCCESS: &str = "#22c55e";
pub(crate) const WARNING: &str = "#f59e0b";
pub(crate) const ACCENT: &str = "#06b6d4";
pub(crate) const SECONDARY: &str = "#8b5cf6";

#[derive(Clone, Copy, Debug, PartialEq, Eq, lurq::DevtoolsInspectable)]
pub(crate) enum DemoTheme {
  Dark,
  Light,
}

#[derive(Clone, Copy)]
pub(crate) struct ThemePalette {
  pub bg: &'static str,
  pub surface: &'static str,
  pub surface_dark: &'static str,
  pub border: &'static str,
  pub text: &'static str,
  pub text_muted: &'static str,
  pub primary: &'static str,
  pub primary_hover: &'static str,
  pub primary_active: &'static str,
  pub nav_selected: &'static str,
  pub accent: &'static str,
}

impl DemoTheme {
  pub(crate) fn label(self) -> &'static str {
    match self {
      Self::Dark => "Dark",
      Self::Light => "Light",
    }
  }

  pub(crate) fn toggle(self) -> Self {
    match self {
      Self::Dark => Self::Light,
      Self::Light => Self::Dark,
    }
  }

  pub(crate) fn palette(self) -> ThemePalette {
    match self {
      Self::Dark => ThemePalette {
        bg: BG,
        surface: SURFACE,
        surface_dark: SURFACE_DARK,
        border: BORDER,
        text: TEXT,
        text_muted: TEXT_MUTED,
        primary: PRIMARY,
        primary_hover: "#60a5fa",
        primary_active: "#2563eb",
        nav_selected: NAV_SELECTED,
        accent: ACCENT,
      },
      Self::Light => ThemePalette {
        bg: "#f8fafc",
        surface: "#ffffff",
        surface_dark: "#e2e8f0",
        border: "#cbd5e1",
        text: "#0f172a",
        text_muted: "#64748b",
        primary: "#2563eb",
        primary_hover: "#3b82f6",
        primary_active: "#1d4ed8",
        nav_selected: "#dbeafe",
        accent: "#0891b2",
      },
    }
  }
}

pub(crate) fn text(content: &str, font_size: f32, weight: FontWeight, color: &str) -> lurq::components::Text {
  lurq::components::Text::styled(
    content,
    TextStyle {
      font_size,
      weight,
      color: Color::from_hex(color),
      ..TextStyle::default()
    },
  )
}
