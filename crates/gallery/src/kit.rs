//! What the gallery's own interface is built from, until the design system
//! has the component for it: each of these goes when its component lands.
//!
//! Everything here takes its colors, sizes and type from the theme, so the
//! gallery is light or dark with it.

use std::sync::Arc;

use lurq::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, TypographyStyle},
  components::{Column, Row, Text},
  layout::{
    Alignment,
    text_style::{FontWeight, TextStyle},
  },
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
};

pub const FILL: Dimension = Dimension::Pct(100.0);

/// A text in a typography style of the theme with something of it changed: a size, a weight. It names no color,
/// so it takes the foreground of what it is in.
pub fn styled(content: &str, role: TypographyStyle, change: impl FnOnce(&mut TextStyle)) -> Text {
  let mut style = lurq::ui::current_theme().typography_style(role);
  change(&mut style);
  Text::styled(content, style)
}

/// Text that says less: a caption, a hint.
pub fn muted(content: &str, style: TypographyStyle) -> Text {
  Text::new(content).variant(style).color(PaletteColor::MutedForeground)
}

/// A line of code, in the theme's family of code.
pub fn code(content: &str) -> Text {
  Text::new(content).variant(TypographyStyle::Mono)
}

/// A small label in a tinted box: a version, a status.
pub fn badge(content: &str) -> Row {
  Row::new()
    .align_items(Alignment::Center)
    .height(22.0)
    .padding_horizontal(8.0)
    .rounded(RadiusSize::Full)
    .background(PaletteColor::Secondary)
    .foreground(PaletteColor::SecondaryForeground)
    .child(styled(content, TypographyStyle::Caption, |style| {
      style.weight = FontWeight::Medium
    }))
}

/// A badge for something that went well: "Stable".
pub fn status_badge(content: &str) -> Row {
  badge(content)
    .background(PaletteColor::SuccessMuted)
    .foreground(PaletteColor::Success)
}

/// One of a few choices in a row, of which one is chosen: the appearance.
///
/// The design system's `SegmentedControl` takes its place.
pub fn segmented<T: Clone + PartialEq + Send + Sync + 'static>(
  options: &[(&str, T)],
  chosen: &T,
  choose: impl Fn(T) + Send + Sync + 'static,
) -> Row {
  let choose = Arc::new(choose);
  let mut control = Row::new()
    .align_items(Alignment::Center)
    .height(36.0)
    .padding(3.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted);
  for (name, value) in options {
    let on = value == chosen;
    let (choose, value) = (choose.clone(), value.clone());
    let mut segment = Row::new()
      .align_items(Alignment::Center)
      .height(30.0)
      .padding_horizontal(12.0)
      .rounded(RadiusSize::Sm)
      .cursor(CursorIcon::Pointer)
      .foreground(if on {
        PaletteColor::Foreground
      } else {
        PaletteColor::MutedForeground
      })
      .on_click(move |_| choose(value.clone()))
      .child(Text::new(name).variant(TypographyStyle::Label));
    if on {
      segment = segment.background(PaletteColor::Background).box_shadow(ShadowStyle::Sm);
    } else {
      segment = segment.hovered(|style| style.foreground(PaletteColor::Foreground));
    }
    control = control.child(segment);
  }
  control
}

/// A card: a surface with a line around it.
pub fn card() -> Column {
  Column::new()
    .width(FILL)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
}

/// The stage of a demo card: where the live components stand, on the canvas.
pub fn stage(content: impl Into<Element>) -> Column {
  Column::new()
    .width(FILL)
    .padding(32.0)
    .background(PaletteColor::Canvas)
    .foreground(PaletteColor::Foreground)
    .child(content)
}

/// The footer of a demo card: the builder call that makes what the stage shows.
pub fn card_footer(call: &str) -> Row {
  Row::new()
    .width(FILL)
    .height(40.0)
    .padding_horizontal(16.0)
    .align_items(Alignment::Center)
    .border_top(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(muted(call, TypographyStyle::Mono))
}

/// A demo card: a stage and the call that builds what is on it.
pub fn demo_card(content: impl Into<Element>, call: &str) -> Column {
  card().child(stage(content)).child(card_footer(call))
}
