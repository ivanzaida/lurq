use lurq::{
  layout::text_style::{FontWeight, TextStyle},
  node::{Element, color::Color},
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

pub(crate) fn text(content: &str, font_size: f32, weight: FontWeight, color: &str) -> Element {
  Element::styled_text(
    content,
    TextStyle {
      font_size,
      weight,
      color: Color::from_hex(color),
      ..TextStyle::default()
    },
  )
}
