//! The end of the inspector: where the selected node was laid out, drawn as the box it is, and the style it was
//! given, a row for each thing said of it.

use crate::{
  app::{
    devtools::{
      kit::{FILL, line, mono, number, px, sans},
      snapshot::{DevToolsLayoutBox, DevToolsNode, DevToolsShapeRow},
    },
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Column, Row, Spacer},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border, color::Color},
  ui::{Badge, BadgePart, BadgeSize, BadgeVariant, Length, ScrollArea},
};

/// The sizes the board gives the summary (`pencil/lurq-devtools.pen`, Components · dark, Summary).
const WIDTH: f32 = 320.0;
const PADDING: f32 = 16.0;
const ROW_HEIGHT: f32 = 28.0;
const LAYOUT_LABEL_WIDTH: f32 = 92.0;
const PAIR_WIDTH: f32 = 98.0;
const STYLE_LABEL_WIDTH: f32 = 104.0;
const SIDE: f32 = 40.0;
const EDGE: f32 = 30.0;

pub(super) fn panel(node: &DevToolsNode) -> Column {
  Column::new()
    .width(WIDTH)
    .height(FILL)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border_left(line())
    .child(layout(node))
    .child(style(node))
}

/// A section's heading: a word in capitals, and how many rows are under it where that is worth saying.
fn head(title: &str, count: Option<usize>) -> Row {
  let mut head = Row::new()
    .height(36.0)
    .padding_top(8.0)
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(sans(title, 11.0, FontWeight::SemiBold).color(PaletteColor::MutedForeground));
  if let Some(count) = count {
    head = head.child(sans(&count.to_string(), 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground));
  }
  head
}

fn layout(node: &DevToolsNode) -> Column {
  let mut section = Column::new()
    .width(FILL)
    .padding_top(4.0)
    .padding_horizontal(PADDING)
    .padding_bottom(12.0)
    .border_bottom(line())
    .child(head("LAYOUT", None));
  section = match node.layout_box {
    Some(laid_out) => {
      let mut section = section
        .child(box_model(&laid_out))
        .child(pair_row(
          "Position",
          ("x", px(laid_out.bounds.x)),
          ("y", px(laid_out.bounds.y)),
        ))
        .child(pair_row(
          "Relative",
          ("x", px(laid_out.relative_x)),
          ("y", px(laid_out.relative_y)),
        ))
        .child(pair_row(
          "Size",
          ("w", px(laid_out.bounds.width)),
          ("h", px(laid_out.bounds.height)),
        ));
      if let Some(content) = laid_out.content {
        section = section
          .child(pair_row("Content", ("x", px(content.x)), ("y", px(content.y))))
          .child(pair_row("", ("w", px(content.width)), ("h", px(content.height))));
      }
      section.child(pair_row(
        "Overflow",
        ("x", laid_out.overflow_x.to_string()),
        ("y", laid_out.overflow_y.to_string()),
      ))
    }
    None => section.child(
      Row::new()
        .height(ROW_HEIGHT)
        .align_items(Alignment::Center)
        .child(sans("Not laid out yet", 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
    ),
  };
  section.child(
    Row::new()
      .height(ROW_HEIGHT)
      .spacing(6.0)
      .align_items(Alignment::Center)
      .child(state_chip("hovered", node.hovered))
      .child(state_chip("active", node.active))
      .child(state_chip("focused", node.focused)),
  )
}

/// What lies between the node's edges and its content, on each side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Insets {
  pub(crate) top: f32,
  pub(crate) right: f32,
  pub(crate) bottom: f32,
  pub(crate) left: f32,
}

/// The node's padding as it was laid out: the room between its bounds and its content. A node with no content
/// box of its own has none.
pub(crate) fn insets(laid_out: &DevToolsLayoutBox) -> Insets {
  let bounds = laid_out.bounds;
  let content = laid_out.content.unwrap_or(bounds);
  Insets {
    top: content.y - bounds.y,
    right: (bounds.x + bounds.width) - (content.x + content.width),
    bottom: (bounds.y + bounds.height) - (content.y + content.height),
    left: content.x - bounds.x,
  }
}

/// The node drawn as a box in a box: its padding around its content, each side with its size.
fn box_model(laid_out: &DevToolsLayoutBox) -> Row {
  let insets = insets(laid_out);
  let content = laid_out.content.unwrap_or(laid_out.bounds);
  let side = |value: f32| mono(&number(value), 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground);
  let centred = || Row::new().align_items(Alignment::Center).justify(Justify::Center);
  let bounds = Column::new()
    .width(FILL)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(line())
    .child(
      // The word at the start is balanced by as much room at the end, so the size stands in the middle.
      Row::new()
        .width(FILL)
        .height(EDGE)
        .padding_horizontal(10.0)
        .align_items(Alignment::Center)
        .child(
          Row::new()
            .width(80.0)
            .child(sans("padding", 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
        )
        .child(centred().flex(1.0).child(side(insets.top)))
        .child(Spacer::new().width(80.0)),
    )
    .child(
      Row::new()
        .width(FILL)
        .height(90.0)
        .child(centred().size(SIDE, FILL).child(side(insets.left)))
        .child(
          Column::new()
            .height(FILL)
            .flex(1.0)
            .spacing(4.0)
            .align_items(Alignment::Center)
            .justify(Justify::Center)
            .rounded(RadiusSize::Sm)
            .background(PaletteColor::Card)
            .border(Border::inside(BorderSize::Sm, PaletteColor::Input))
            .child(mono(
              &format!("{} × {}", number(content.width), number(content.height)),
              13.0,
              FontWeight::Medium,
            ))
            .child(sans("content", 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
        )
        .child(centred().size(SIDE, FILL).child(side(insets.right))),
    )
    .child(centred().width(FILL).height(EDGE).child(side(insets.bottom)));
  Row::new()
    .width(FILL)
    .padding_top(6.0)
    .padding_bottom(10.0)
    .child(bounds)
}

/// A row of the layout: what it is of, and two numbers each after its letter.
fn pair_row(label: &str, first: (&str, String), second: (&str, String)) -> Row {
  let pair = |(letter, value): (&str, String)| {
    // A flag that is off says so quietly; a number, and a flag that is on, in the foreground.
    let value = if value == "false" {
      mono(&value, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)
    } else {
      mono(&value, 12.0, FontWeight::Medium)
    };
    Row::new()
      .width(PAIR_WIDTH)
      .spacing(8.0)
      .align_items(Alignment::Center)
      .child(mono(letter, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
      .child(value)
  };
  Row::new()
    .width(FILL)
    .height(ROW_HEIGHT)
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .width(LAYOUT_LABEL_WIDTH)
        .child(sans(label, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
    )
    .child(pair(first))
    .child(pair(second))
}

/// Whether the pointer is on the node, holds it, or the focus is in it.
fn state_chip(label: &str, on: bool) -> Element {
  // The board's chip: an outline badge whose dot is lit while the state holds, and dim with its word while not.
  let (dot, word) = if on {
    (PaletteColor::Success, PaletteColor::Foreground)
  } else {
    (PaletteColor::Input, PaletteColor::MutedForeground)
  };
  Badge::new(label)
    .variant(BadgeVariant::Outline)
    .size(BadgeSize::Sm)
    .dot(true)
    .style(BadgePart::Dot, |s| s.background(dot))
    .style(BadgePart::Label, |s| s.foreground(word))
    .into()
}

/// One thing the style section says.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum StyleLine {
  /// A property and its value: `Padding left`, `spacing(xl)`.
  Value { label: String, value: String },
  /// A heading over the properties of a state: `On hover`.
  Group(String),
}

/// The rows of the style section, from the node's shape: a property with parts is a row for each part
/// (`padding` and `left` are `Padding left`), and the style of a state is a heading with its rows under it.
pub(crate) fn style_lines(shape: &[DevToolsShapeRow]) -> Vec<StyleLine> {
  let mut lines = Vec::new();
  for row in shape {
    match state_title(&row.label) {
      Some(title) if !row.children.is_empty() => {
        lines.push(StyleLine::Group(title.to_owned()));
        row.children.iter().for_each(|child| flatten(child, "", &mut lines));
      }
      _ => flatten(row, "", &mut lines),
    }
  }
  lines
}

fn flatten(row: &DevToolsShapeRow, before: &str, lines: &mut Vec<StyleLine>) {
  let label = if before.is_empty() {
    capitalized(&row.label)
  } else {
    format!("{before} {}", row.label)
  };
  if let Some(value) = &row.value {
    lines.push(StyleLine::Value {
      label: label.clone(),
      value: value.clone(),
    });
  }
  row.children.iter().for_each(|child| flatten(child, &label, lines));
}

fn state_title(label: &str) -> Option<&'static str> {
  Some(match label {
    "hover style" => "On hover",
    "active style" => "On press",
    "focused style" => "On focus",
    "focus-visible style" => "On keyboard focus",
    _ => return None,
  })
}

fn capitalized(label: &str) -> String {
  let mut chars = label.chars();
  match chars.next() {
    Some(first) => first.to_uppercase().chain(chars).collect(),
    None => String::new(),
  }
}

/// The color a value names outright, `#rrggbb` or `#rrggbbaa`: what its swatch shows.
pub(crate) fn swatch_color(value: &str) -> Option<Color> {
  let digits = value.strip_prefix('#')?;
  (matches!(digits.len(), 6 | 8) && digits.chars().all(|digit| digit.is_ascii_hexdigit()))
    .then(|| Color::from_hex(value))
}

fn style(node: &DevToolsNode) -> Column {
  let lines = style_lines(&node.shape);
  let count = lines
    .iter()
    .filter(|line| matches!(line, StyleLine::Value { .. }))
    .count();
  let mut rows = Column::new()
    .width(FILL)
    .padding_left(PADDING)
    .padding_right(8.0)
    .padding_bottom(12.0);
  for line in &lines {
    rows = rows.child(match line {
      StyleLine::Value { label, value } => style_row(label, value),
      StyleLine::Group(title) => Row::new()
        .width(FILL)
        .height(ROW_HEIGHT)
        .padding_top(4.0)
        .align_items(Alignment::Center)
        .child(sans(title, 12.0, FontWeight::Medium)),
    });
  }
  Column::new()
    .width(FILL)
    .flex(1.0)
    .padding_top(4.0)
    .child(head("STYLE", Some(count)).padding_horizontal(PADDING))
    .child(
      Column::new().width(FILL).flex(1.0).child(
        ScrollArea::new()
          .label("Style of the selected node")
          .height(Length::Fill)
          .child(rows),
      ),
    )
}

fn style_row(label: &str, value: &str) -> Row {
  let mut row = Row::new()
    .width(FILL)
    .height(ROW_HEIGHT)
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .width(STYLE_LABEL_WIDTH)
        .child(sans(label, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
    );
  if let Some(color) = swatch_color(value) {
    row = row.child(
      Row::new().padding_right(8.0).child(
        Row::new()
          .size(12.0, 12.0)
          .corner_radius(2.0)
          .background(color)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Input)),
      ),
    );
  }
  row.child(mono(value, 12.0, FontWeight::Medium))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::devtools::snapshot::DevToolsRect;

  fn leaf(label: &str, value: &str) -> DevToolsShapeRow {
    DevToolsShapeRow {
      label: label.to_owned(),
      value: Some(value.to_owned()),
      children: Vec::new(),
    }
  }

  fn group(label: &str, children: Vec<DevToolsShapeRow>) -> DevToolsShapeRow {
    DevToolsShapeRow {
      label: label.to_owned(),
      value: None,
      children,
    }
  }

  fn value(label: &str, value: &str) -> StyleLine {
    StyleLine::Value {
      label: label.to_owned(),
      value: value.to_owned(),
    }
  }

  #[test]
  fn a_property_with_parts_is_a_row_for_each_part() {
    let shape = [
      leaf("layout", "Column"),
      group("padding", vec![leaf("left", "8px"), leaf("right", "8px")]),
    ];
    assert_eq!(
      style_lines(&shape),
      [
        value("Layout", "Column"),
        value("Padding left", "8px"),
        value("Padding right", "8px"),
      ]
    );
  }

  #[test]
  fn the_style_of_a_state_is_a_heading_over_its_rows() {
    let shape = [group(
      "hover style",
      vec![leaf("fill", "#ff0000"), group("padding", vec![leaf("left", "4px")])],
    )];
    assert_eq!(
      style_lines(&shape),
      [
        StyleLine::Group("On hover".to_owned()),
        value("Fill", "#ff0000"),
        value("Padding left", "4px"),
      ]
    );
  }

  #[test]
  fn a_color_written_out_has_a_swatch_and_a_name_has_none() {
    assert!(swatch_color("#ff0000").is_some());
    assert!(swatch_color("#ff000080").is_some());
    assert_eq!(swatch_color("palette(background)"), None);
    assert_eq!(swatch_color("#ff00"), None);
    assert_eq!(swatch_color("#gggggg"), None);
  }

  #[test]
  fn the_padding_is_the_room_between_the_bounds_and_the_content() {
    let rect = |x, y, width, height| DevToolsRect { x, y, width, height };
    let laid_out = DevToolsLayoutBox {
      bounds: rect(280.0, 44.0, 960.0, 1232.0),
      relative_x: 0.0,
      relative_y: 0.0,
      content: Some(rect(304.0, 68.0, 912.0, 1184.0)),
      overflow_x: false,
      overflow_y: false,
    };
    assert_eq!(
      insets(&laid_out),
      Insets {
        top: 24.0,
        right: 24.0,
        bottom: 24.0,
        left: 24.0,
      }
    );
    let bare = DevToolsLayoutBox {
      content: None,
      ..laid_out
    };
    assert_eq!(
      insets(&bare),
      Insets {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
      }
    );
  }
}
