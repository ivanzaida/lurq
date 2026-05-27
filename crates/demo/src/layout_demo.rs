use lurq::{
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color},
};

use crate::style::{
  ACCENT, BG, BORDER, ERROR, PRIMARY, SECONDARY, SUCCESS, SURFACE, SURFACE_DARK, TEXT, TEXT_MUTED, WARNING, text,
};

pub(crate) fn layout_content() -> Element {
  Element::column()
    .spacing(22.0)
    .child(text("Layout", 26.0, FontWeight::Bold, TEXT).width(936.0))
    .child(section_title("Row vs Column"))
    .child(row_vs_column())
    .child(section_title("Justify Modes"))
    .child(justify_modes())
    .child(section_title("Cross-Axis Alignment"))
    .child(cross_axis_alignment())
    .child(section_title("Flex Distribution"))
    .child(flex_distribution())
    .child(section_title("Stack (Overlay)"))
    .child(stack_overlay())
    .child(section_title("Flex Wrap"))
    .child(flex_wrap_demo())
    .pad_xy(32.0, 32.0)
    .width(1000.0)
    .height(1702.0)
    .fill(BG)
}

fn section_title(label: &str) -> Element {
  text(label, 15.0, FontWeight::Bold, TEXT).width(936.0)
}

fn row_vs_column() -> Element {
  Element::row()
    .spacing(32.0)
    .align_items(Alignment::Start)
    .child(demo_card(
      "Row",
      Element::row()
        .spacing(12.0)
        .align_items(Alignment::Center)
        .child(labeled_box("A", ERROR, 64.0, 48.0))
        .child(labeled_box("B", SUCCESS, 64.0, 48.0))
        .child(labeled_box("C", PRIMARY, 64.0, 48.0))
        .width(404.0)
        .height(89.0),
    ))
    .child(demo_card(
      "Column",
      Element::column()
        .spacing(8.0)
        .child(labeled_box("A", ERROR, 404.0, 25.6667))
        .child(labeled_box("B", SUCCESS, 404.0, 25.6667))
        .child(labeled_box("C", PRIMARY, 404.0, 25.6666))
        .width(404.0)
        .height(89.0),
    ))
    .pad(24.0)
    .width(936.0)
    .height(188.0)
    .fill(SURFACE)
    .rounded(4.0)
}

fn justify_modes() -> Element {
  let rows = [
    ("Start", Justify::Start),
    ("End", Justify::End),
    ("Center", Justify::Center),
    ("SpaceBetween", Justify::SpaceBetween),
    ("SpaceAround", Justify::SpaceAround),
  ];

  Element::column()
    .spacing(10.0)
    .with_children(rows.into_iter().map(|(label, justify)| {
      Element::row()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(text(label, 12.0, FontWeight::Medium, TEXT_MUTED).width(110.0))
        .child(
          Element::row()
            .spacing(6.0)
            .align_items(Alignment::Stretch)
            .justify(justify)
            .child(stretch_swatch(ERROR, 48.0))
            .child(stretch_swatch(SUCCESS, 48.0))
            .child(stretch_swatch(PRIMARY, 48.0))
            .width(762.0)
            .height(40.0)
            .fill(SURFACE_DARK)
            .rounded(3.0),
        )
    }))
    .pad(24.0)
    .width(936.0)
    .height(288.0)
    .fill(SURFACE)
    .rounded(4.0)
}

fn cross_axis_alignment() -> Element {
  Element::row()
    .spacing(16.0)
    .align_items(Alignment::Start)
    .child(alignment_card("Start", Alignment::Start))
    .child(alignment_card("Center", Alignment::Center))
    .child(alignment_card("End", Alignment::End))
    .pad(20.0)
    .width(936.0)
    .height(188.0)
    .fill(SURFACE)
    .rounded(4.0)
}

fn alignment_card(label: &str, align: Alignment) -> Element {
  Element::column()
    .spacing(8.0)
    .child(text(label, 12.0, FontWeight::Medium, TEXT_MUTED))
    .child(
      Element::row()
        .spacing(8.0)
        .align_items(align)
        .child(labeled_box("A", ERROR, 32.0, 42.0))
        .child(labeled_box("B", SUCCESS, 38.0, 74.0))
        .child(labeled_box("C", PRIMARY, 28.0, 32.0))
        .pad(10.0)
        .width(244.0)
        .height(94.0)
        .fill(BG)
        .border_inside(1.0, Color::from_hex(BORDER))
        .rounded(3.0),
    )
    .pad(10.0)
    .width(288.0)
    .height(148.0)
    .fill(SURFACE_DARK)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(3.0)
}

fn flex_distribution() -> Element {
  Element::row()
    .align_items(Alignment::Center)
    .child(distribution_segment("flex(1)", PRIMARY, 1.0))
    .child(distribution_segment("flex(2)", SECONDARY, 2.0))
    .child(distribution_segment("flex(1)", ACCENT, 1.0))
    .pad(20.0)
    .width(936.0)
    .height(104.0)
    .fill(SURFACE)
    .rounded(4.0)
}

fn distribution_segment(label: &str, color: &str, grow: f32) -> Element {
  Element::row()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 12.0, FontWeight::Bold, TEXT))
    .height(64.0)
    .flex(grow)
    .fill(color)
}

fn stack_overlay() -> Element {
  Element::stack()
    .child(Element::rect(200.0, 200.0).fill(PRIMARY).absolute_position(280.0, 30.0))
    .child(Element::rect(150.0, 150.0).fill(SUCCESS).absolute_position(305.0, 55.0))
    .child(Element::rect(100.0, 100.0).fill(ERROR).absolute_position(330.0, 80.0))
    .child(
      text(
        "Three rectangles centered with stack alignment",
        12.0,
        FontWeight::Medium,
        TEXT_MUTED,
      )
      .absolute_position(240.0, 235.0),
    )
    .width(936.0)
    .height(260.0)
    .fill(SURFACE)
    .rounded(4.0)
}

fn flex_wrap_demo() -> Element {
  Element::column()
    .child(
      Element::row()
        .spacing(10.0)
        .align_items(Alignment::Center)
        .with_children(
          [
            ("1", PRIMARY),
            ("2", SUCCESS),
            ("3", ERROR),
            ("4", WARNING),
            ("5", ACCENT),
            ("6", SECONDARY),
            ("7", PRIMARY),
            ("8", SUCCESS),
          ]
          .into_iter()
          .map(|(label, color)| labeled_box(label, color, 90.0, 40.0)),
        )
        .wrap()
        .pad(12.0)
        .width(888.0)
        .height(64.0)
        .fill(SURFACE_DARK)
        .rounded(3.0),
    )
    .pad(24.0)
    .width(936.0)
    .height(112.0)
    .fill(SURFACE)
    .rounded(4.0)
}

fn demo_card(title: &str, body: Element) -> Element {
  Element::column()
    .spacing(12.0)
    .child(text(title, 12.0, FontWeight::Bold, TEXT_MUTED))
    .child(body)
    .pad(12.0)
    .width(428.0)
    .height(140.0)
    .fill(SURFACE_DARK)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(3.0)
}

fn labeled_box(label: &str, color: &str, width: f32, height: f32) -> Element {
  Element::row()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 12.0, FontWeight::Bold, TEXT))
    .size(width, height)
    .fill(color)
    .rounded(2.0)
}

fn stretch_swatch(color: &str, width: f32) -> Element {
  Element::spacer().width(width).fill(color).rounded(2.0)
}
