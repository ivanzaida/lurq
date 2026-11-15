//! What the DevTools' own layout is written with between the design system's components: text at the board's
//! sizes in the theme's families, and the theme's line. Nothing here names a color or a font of its own.

use crate::{
  app::theme::{BorderSize, PaletteColor, TypographyStyle},
  components::Text,
  layout::text_style::FontWeight,
  node::{border::Border, dimension::Dimension},
};

pub(super) const FILL: Dimension = Dimension::Pct(100.0);

/// Interface text at a size of the board, in the theme's family and at its text scale. It names no color, so it
/// takes the foreground of what it is in.
pub(super) fn sans(content: &str, size: f32, weight: FontWeight) -> Text {
  text(content, TypographyStyle::Body, size, weight)
}

/// Text in the theme's family of code: a name of the inspected app, a value, a type.
pub(super) fn mono(content: &str, size: f32, weight: FontWeight) -> Text {
  text(content, TypographyStyle::Mono, size, weight)
}

fn text(content: &str, role: TypographyStyle, size: f32, weight: FontWeight) -> Text {
  let theme = crate::ui::current_theme();
  let mut style = theme.typography_style(role);
  style.font_size = theme.scaled(size);
  style.weight = weight;
  Text::styled(content, style).nowrap()
}

/// The line between two parts of the window.
pub(super) fn line() -> Border {
  Border::inside(BorderSize::Sm, PaletteColor::Border)
}

/// A tag without the modules it is in: `demo::pages::Counter` is `Counter`.
pub(crate) fn short_tag(tag: &str) -> &str {
  tag.rsplit("::").next().unwrap_or(tag)
}

/// A type without the modules of anything in it: `core::Signal<demo::Locale>` is `Signal<Locale>`.
pub(super) fn short_type_name(type_name: &str) -> String {
  let mut short = String::new();
  let mut token = String::new();
  let flush = |short: &mut String, token: &mut String| {
    short.push_str(token.rsplit("::").next().unwrap_or(token));
    token.clear();
  };
  for ch in type_name.chars() {
    if matches!(ch, '<' | '>' | ',' | ' ' | '&' | '[' | ']' | '(' | ')' | ';') {
      flush(&mut short, &mut token);
      short.push(ch);
    } else {
      token.push(ch);
    }
  }
  flush(&mut short, &mut token);
  short
}

/// A length of the inspected app, in its pixels: whole where it is whole.
pub(super) fn px(value: f32) -> String {
  if value.fract().abs() < 0.005 {
    format!("{value:.0}px")
  } else {
    format!("{value:.2}px")
  }
}

/// A number of the inspected app without its unit: a side of the box model.
pub(super) fn number(value: f32) -> String {
  if value.fract().abs() < 0.005 {
    format!("{value:.0}")
  } else {
    format!("{value:.1}")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_short_type_keeps_what_holds_its_parts_together() {
    assert_eq!(
      short_type_name("lurq::core::context::ReactiveContext<demo::context_demo::LocaleContext>"),
      "ReactiveContext<LocaleContext>"
    );
    assert_eq!(short_type_name("&str"), "&str");
    assert_eq!(
      short_type_name("(alloc::string::String, core::option::Option<u8>)"),
      "(String, Option<u8>)"
    );
  }

  #[test]
  fn a_length_is_whole_where_it_is_whole() {
    assert_eq!(px(280.0), "280px");
    assert_eq!(px(12.5), "12.50px");
    assert_eq!(number(24.0), "24");
    assert_eq!(number(7.3), "7.3");
  }
}
