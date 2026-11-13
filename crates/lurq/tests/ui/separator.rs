//! A separator: a line of 1 in the border's color across a column or down a
//! row, a label in its middle, a length of its own, and a theme's rule.

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row},
  layout::{
    Alignment,
    quad::{Quad, QuadContent},
  },
  node::color::Color,
  ui::{Separator, SeparatorPart},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The rectangles filled with `color`, from the start.
fn fills(stage: &Stage, color: Color) -> Vec<Quad> {
  let mut quads: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect();
  quads.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
  quads
}

#[test]
fn a_separator_across_a_column_is_a_line_as_wide_as_the_column() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(240.0)
      .spacing(8.0)
      .child(Row::new().size(40.0, 20.0))
      .child(Separator::horizontal().id("line"))
      .child(Row::new().size(40.0, 20.0))
      .into()
  });
  let (x, y, width, height) = stage.bounds("line");
  assert_eq!((x, y, width, height), (0.0, 28.0, 240.0, 1.0));
  let lines = fills(&stage, light(PaletteColor::Border));
  assert_eq!(lines.len(), 1);
  assert_eq!((lines[0].width, lines[0].height), (240.0, 1.0));
}

#[test]
fn a_separator_down_a_row_is_as_high_as_a_row_that_stretches_or_as_long_as_it_is_told() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(16.0)
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Stretch)
          .child(Row::new().size(40.0, 24.0))
          .child(Separator::vertical().id("stretched"))
          .child(Row::new().size(40.0, 24.0)),
      )
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(Row::new().size(40.0, 24.0))
          .child(Separator::vertical().length(16.0).id("told"))
          .child(Row::new().size(40.0, 24.0)),
      )
      .into()
  });
  let (x, _, width, height) = stage.bounds("stretched");
  assert_eq!((x, width, height), (48.0, 1.0, 24.0));
  let (_, y, width, height) = stage.bounds("told");
  assert_eq!((width, height), (1.0, 16.0));
  assert!((y - (40.0 + 4.0)).abs() < 0.5, "in the middle of the row: {y}");
  let lines = fills(&stage, light(PaletteColor::Border));
  assert_eq!(lines.len(), 2);
  assert!(lines.iter().all(|line| line.width == 1.0));
}

#[test]
fn a_label_stands_in_the_middle_with_the_line_on_either_side() {
  let stage = Stage::new(|_| {
    Column::new()
      .width(240.0)
      .child(Separator::horizontal().label("or").id("or"))
      .into()
  });
  assert_eq!(stage.text("or"), (light(PaletteColor::MutedForeground), 12.0));
  let label = stage.text_quad("or").unwrap();
  let lines = fills(&stage, light(PaletteColor::Border));
  assert_eq!(lines.len(), 2, "a line on either side");
  let (before, after) = (&lines[0], &lines[1]);
  assert_eq!((before.x, before.height), (0.0, 1.0));
  assert!((before.width - after.width).abs() < 0.5, "as long as each other");
  assert!((after.x + after.width - 240.0).abs() < 0.5);
  assert!(label.x - (before.x + before.width) >= 8.0 - 0.5, "8 from the line");
  assert!((after.x - (label.x + label.width)) >= 8.0 - 0.5);
  // The lines run through the middle of the label.
  let middle = label.y + label.height / 2.0;
  assert!((before.y - middle).abs() < 1.5, "{} {middle}", before.y);
}

#[test]
fn a_rule_changes_its_color_and_an_inset_is_its_padding() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(240.0)
      .child(
        Separator::horizontal()
          .id("inset")
          .style(SeparatorPart::Root, |s| s.padding_x(16.0))
          .style(SeparatorPart::Line, |s| s.background(PaletteColor::Primary)),
      )
      .into()
  });
  assert_eq!(stage.bounds("inset").2, 240.0);
  let lines = fills(&stage, light(PaletteColor::Primary));
  assert_eq!(lines.len(), 1);
  assert_eq!((lines[0].x, lines[0].width), (16.0, 208.0));
}
