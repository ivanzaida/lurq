use std::sync::Arc;

use crate::{
  layout::Alignment,
  node::{TextColor, color::Color},
};

const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_LINE_HEIGHT: f32 = 1.2;
const DEFAULT_TEXT_COLOR: Color = Color::new(0, 0, 0, 255);
#[cfg(target_os = "windows")]
const DEFAULT_FONT_FAMILY_WINDOWS: &str = "Segoe UI";
#[cfg(not(target_os = "windows"))]
const DEFAULT_FONT_FAMILY_FALLBACK: &str = "";

#[derive(Clone, PartialEq)]
pub struct TextStyle {
  pub font_family: Arc<str>,
  pub font_size: f32,
  pub line_height: f32,
  pub weight: FontWeight,
  pub style: FontStyle,
  pub text_align: TextAlign,
  /// Vertical placement of the glyph run within its box (the line-height box
  /// for a single line, or the node's box when it is taller). Alignment is
  /// based on the actual glyph ink bounds so text is optically centered rather
  /// than metric-centered (which leaves most fonts sitting visibly high).
  pub vertical_align: VerticalAlign,
  /// CSS `text-box-trim`-style leading trim. When set, `line_height` only adds
  /// leading BETWEEN wrapped lines: the measured box of a single line collapses
  /// to the em box (`font_size`) instead of `font_size * line_height`, and a
  /// multi-line run's box drops the half-leading above the first line and below
  /// the last. Glyph placement is unaffected — the render path already centers
  /// the optical box within the (now trimmed) box — so a single line looks
  /// identical to `line_height: 1.0` while wrapped lines keep their full leading.
  /// Lets a caller pick one readable `line_height` for both single- and
  /// multi-line text without inflating single-line vertical rhythm.
  pub trim_line_box: bool,
  pub color: Color,
  pub caret_color: Option<TextColor>,
  pub shadow: Option<TextShadow>,
}

impl Default for TextStyle {
  fn default() -> Self {
    Self {
      font_family: default_font_family(),
      font_size: DEFAULT_FONT_SIZE,
      line_height: DEFAULT_LINE_HEIGHT,
      weight: FontWeight::Normal,
      style: FontStyle::Normal,
      text_align: TextAlign::Left,
      vertical_align: VerticalAlign::default(),
      trim_line_box: false,
      color: DEFAULT_TEXT_COLOR,
      caret_color: None,
      shadow: None,
    }
  }
}

/// CSS-like text shadow: offsets and blur radius are in logical pixels, the
/// blur radius maps to a Gaussian with `sigma = blur_radius / 2`.
#[derive(Clone, Copy, PartialEq)]
pub struct TextShadow {
  pub offset_x: f32,
  pub offset_y: f32,
  pub blur_radius: f32,
  pub color: Color,
}

impl TextShadow {
  pub fn new(offset_x: f32, offset_y: f32, blur_radius: f32, color: Color) -> Self {
    Self {
      offset_x,
      offset_y,
      blur_radius: blur_radius.max(0.0),
      color,
    }
  }

  pub(crate) fn is_visible(&self) -> bool {
    self.color.a() > 0 && (self.offset_x != 0.0 || self.offset_y != 0.0 || self.blur_radius > 0.0)
  }
}

fn default_font_family() -> Arc<str> {
  #[cfg(target_os = "windows")]
  {
    Arc::from(DEFAULT_FONT_FAMILY_WINDOWS)
  }
  #[cfg(not(target_os = "windows"))]
  {
    Arc::from(DEFAULT_FONT_FAMILY_FALLBACK)
  }
}

/// CSS font weight. The named variants carry the CSS keyword values (`Thin` =
/// 100 … `Black` = 900); `Numeric` accepts any other weight and is clamped to
/// CSS's `1..=1000` range. Weights compare, hash and cache by numeric value, so
/// `FontWeight::Numeric(600) == FontWeight::SemiBold`.
///
/// Text is drawn with the loaded face of the requested family chosen by CSS
/// font matching (as implemented by fontdb): an exact weight first; for a
/// request of 400–449, then 500; for 450–500, then 400; otherwise the nearest
/// lighter face for requests up to 500 and the nearest heavier face above 500,
/// then the nearest face on the other side. Faces are never synthesized, so
/// `SemiBold` renders with the Bold face when the family has no 600 face.
#[derive(Clone, Copy, Debug, Default)]
pub enum FontWeight {
  Thin,
  ExtraLight,
  Light,
  #[default]
  Normal,
  Medium,
  SemiBold,
  Bold,
  ExtraBold,
  Black,
  Numeric(u16),
}

impl FontWeight {
  pub const MIN_VALUE: u16 = 1;
  pub const MAX_VALUE: u16 = 1000;

  /// The CSS numeric weight, clamped to `1..=1000`.
  pub const fn value(self) -> u16 {
    match self {
      Self::Thin => 100,
      Self::ExtraLight => 200,
      Self::Light => 300,
      Self::Normal => 400,
      Self::Medium => 500,
      Self::SemiBold => 600,
      Self::Bold => 700,
      Self::ExtraBold => 800,
      Self::Black => 900,
      Self::Numeric(value) => {
        if value < Self::MIN_VALUE {
          Self::MIN_VALUE
        } else if value > Self::MAX_VALUE {
          Self::MAX_VALUE
        } else {
          value
        }
      }
    }
  }

  /// The requested weight. Layout passes the weight of the nearest loaded face
  /// instead, because cosmic-text only selects faces of exactly this weight.
  pub fn to_cosmic(&self) -> cosmic_text::Weight {
    cosmic_text::Weight(self.value())
  }
}

impl PartialEq for FontWeight {
  fn eq(&self, other: &Self) -> bool {
    self.value() == other.value()
  }
}

impl Eq for FontWeight {}

impl std::hash::Hash for FontWeight {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.value().hash(state);
  }
}

impl From<u16> for FontWeight {
  fn from(value: u16) -> Self {
    Self::Numeric(value)
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontStyle {
  #[default]
  Normal,
  Italic,
}

impl FontStyle {
  pub fn to_cosmic(&self) -> cosmic_text::Style {
    match self {
      Self::Normal => cosmic_text::Style::Normal,
      Self::Italic => cosmic_text::Style::Italic,
    }
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TextAlign {
  #[default]
  Left,
  Center,
  Right,
  Justified,
  End,
}

impl TextAlign {
  pub(crate) fn to_cosmic(self) -> cosmic_text::Align {
    match self {
      Self::Left => cosmic_text::Align::Left,
      Self::Center => cosmic_text::Align::Center,
      Self::Right => cosmic_text::Align::Right,
      Self::Justified => cosmic_text::Align::Justified,
      Self::End => cosmic_text::Align::End,
    }
  }
}

/// Vertical alignment of a text run's glyph ink within its box, mirroring
/// browser `vertical-align` semantics against the line-height box:
/// - `Top`: the top of the glyph ink meets the top of the box.
/// - `Bottom`: the bottom of the glyph ink meets the bottom of the box.
/// - `Center`: the ink is centered — `offset = (box_height - ink_height) / 2`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum VerticalAlign {
  Top,
  #[default]
  Center,
  Bottom,
}

impl From<Alignment> for TextAlign {
  fn from(alignment: Alignment) -> Self {
    match alignment {
      Alignment::Start | Alignment::Stretch => Self::Left,
      Alignment::Center => Self::Center,
      Alignment::End => Self::Right,
    }
  }
}
