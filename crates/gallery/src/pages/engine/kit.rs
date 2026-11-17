//! What the pages of the engine's primitives are built from beside the gallery's own kit: tiles to lay out, a well
//! that shows the bounds of what holds them, a number a slider sets, and a sample with what it is under it.
//!
//! What a page shows is the engine's (`lurq::components`, `lurq::layout`, `lurq::core`); what frames it and what
//! changes it are the design system's, in the theme's colors, as on every other page.

use lurq::{
  app::theme::{BorderSize, PaletteColor, RadiusSize},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border, dimension::Dimension},
  ui::Slider,
};

use crate::kit::{FILL, mono, quiet, text};

/// The fills of the tiles, in turn: the theme's chart colors, which are told apart in light and in dark.
pub const TINTS: [PaletteColor; 5] = [
  PaletteColor::Chart1,
  PaletteColor::Chart2,
  PaletteColor::Chart3,
  PaletteColor::Chart4,
  PaletteColor::Chart5,
];

/// The `index`th tint, round again after the last.
pub fn tint(index: usize) -> PaletteColor {
  TINTS[index % TINTS.len()].clone()
}

/// A tile in the `index`th tint with `label` in its middle: what a layout is shown laying out.
pub fn tile(label: &str, index: usize, width: impl Into<Dimension>, height: impl Into<Dimension>) -> Row {
  Row::new()
    .size(width, height)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Sm)
    .background(tint(index))
    .child(
      text(label, 13.0, FontWeight::SemiBold)
        .color(PaletteColor::Background)
        .nowrap(),
    )
}

/// The edge and the fill of a box whose bounds a page shows: what a layout lays its children out in.
pub fn well_line() -> Border {
  Border::inside(BorderSize::Sm, PaletteColor::Border).dashed()
}

/// A column that shows its bounds, as wide as it is given: what a sample is laid out in.
pub fn well() -> Column {
  Column::new()
    .width(FILL)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .border(well_line())
}

/// A sample with what it is under it, in the family of code.
pub fn sample(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(8.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// Samples side by side; what does not fit goes on to the next line.
pub fn samples() -> Row {
  Row::new()
    .width(FILL)
    .spacing(24.0)
    .wrap()
    .align_items(Alignment::Start)
}

/// A row with a name on the left, in the family of code, and what it names after it: a line of a list of what
/// a page shows, a value and what it is.
pub fn named(name: &str, shown: impl Into<Element>) -> Row {
  Row::new()
    .width(FILL)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      mono(name, 12.0)
        .color(PaletteColor::MutedForeground)
        .width(120.0)
        .flex_shrink(0.0),
    )
    .child(Column::new().flex(1.0).min_width(0.0).child(shown))
}

/// A number a slider sets, between `min` and `max`, with its name over it and its value beside the name: a
/// control of a section, where a playground's select of a few values would not show what a value between does.
pub fn number(label: &str, value: &Signal<i32>, min: i32, max: i32) -> Column {
  Column::new()
    .width(220.0)
    .max_width(FILL)
    .spacing(6.0)
    .child(
      Row::new()
        .width(FILL)
        .justify(Justify::SpaceBetween)
        .child(quiet(label, 12.0))
        .child(mono(&value.get().to_string(), 12.0)),
    )
    .child(Slider::new(value.clone()).min(min).max(max).label(label).length(220.0))
}

/// The controls of a section, side by side; what does not fit goes on to the next line.
pub fn controls() -> Row {
  Row::new().width(FILL).spacing(20.0).wrap().align_items(Alignment::End)
}
