use lurq::{
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{
  ACCENT, BG, BORDER, ERROR, PRIMARY, SECONDARY, SUCCESS, SURFACE, SURFACE_DARK, TEXT, TEXT_MUTED, WARNING, text,
};

const CONTENT_PAD: f32 = 32.0;
const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const SECTION_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;
const ROW_COL_BOX_RADIUS: f32 = 6.0;
const ALIGN_BLOCK_RADIUS: f32 = 3.0;
const CONTROL_RADIUS: f32 = 4.0;
const PRIMARY_30: &str = "#3b82f64d";
const SECONDARY_30: &str = "#8b5cf64d";
const ACCENT_30: &str = "#06b6d44d";

pub(crate) fn layout_content() -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("Layout", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
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
    .padding(CONTENT_PAD)
    .width(FILL_WIDTH)
    .background(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 15.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn row_vs_column() -> Element {
  lurq::components::Row::new()
    .spacing(32.0)
    .align_items(Alignment::Start)
    .child(demo_card(
      "Row",
      lurq::components::Row::new()
        .spacing(12.0)
        .align_items(Alignment::Center)
        .child(row_col_box("A", ERROR, 80.0, 56.0))
        .child(row_col_box("B", SUCCESS, 80.0, 56.0))
        .child(row_col_box("C", PRIMARY, 80.0, 56.0))
        .width(FILL_WIDTH)
        .height(89.0)
        .into(),
    ))
    .child(demo_card(
      "Column",
      lurq::components::Column::new()
        .spacing(8.0)
        .child(row_col_box("A", ERROR, FILL_WIDTH, 24.3334))
        .child(row_col_box("B", SUCCESS, FILL_WIDTH, 24.3333))
        .child(row_col_box("C", PRIMARY, FILL_WIDTH, 24.3333))
        .width(FILL_WIDTH)
        .height(89.0)
        .into(),
    ))
    .padding(24.0)
    .width(FILL_WIDTH)
    .height(188.0)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn justify_modes() -> Element {
  let rows = [
    ("Start", Justify::Start),
    ("End", Justify::End),
    ("Center", Justify::Center),
    ("SpaceBetween", Justify::SpaceBetween),
    ("SpaceAround", Justify::SpaceAround),
  ];

  lurq::components::Column::new()
    .spacing(10.0)
    .with_children(rows.into_iter().map(|(label, justify)| {
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(text(label, 13.0, FontWeight::Normal, TEXT_MUTED).width(110.0))
        .child(
          lurq::components::Row::new()
            .align_items(Alignment::Center)
            .justify(justify)
            .child(justify_swatch(ERROR))
            .child(justify_swatch(SUCCESS))
            .child(justify_swatch(PRIMARY))
            .padding_horizontal(8.0)
            .padding_vertical(4.0)
            .height(40.0)
            .flex(1.0)
            .background(BG)
            .border_inside(1.0, Color::from_hex(BORDER))
            .rounded(CONTROL_RADIUS),
        )
    }))
    .padding(24.0)
    .width(FILL_WIDTH)
    .height(288.0)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn cross_axis_alignment() -> Element {
  lurq::components::Row::new()
    .spacing(16.0)
    .align_items(Alignment::Start)
    .child(alignment_card("Start", Alignment::Start))
    .child(alignment_card("Center", Alignment::Center))
    .child(alignment_card("End", Alignment::End))
    .padding(20.0)
    .width(FILL_WIDTH)
    .height(188.0)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn alignment_card(label: &str, align: Alignment) -> Element {
  lurq::components::Column::new()
    .spacing(8.0)
    .child(text(label, 11.0, FontWeight::Normal, TEXT_MUTED))
    .child(
      lurq::components::Row::new()
        .spacing(6.0)
        .align_items(align)
        .child(align_block(ERROR, 36.0))
        .child(align_block(SUCCESS, 64.0))
        .child(align_block(PRIMARY, 28.0))
        .padding_horizontal(8.0)
        .padding_vertical(6.0)
        .width(FILL_WIDTH)
        .height(107.0)
        .border_inside(1.0, Color::from_hex(BORDER))
        .rounded(CONTROL_RADIUS),
    )
    .padding(10.0)
    .height(148.0)
    .flex(1.0)
    .background(BG)
    .rounded(PANEL_RADIUS)
    .into()
}

fn flex_distribution() -> Element {
  lurq::components::Row::new()
    .spacing(4.0)
    .align_items(Alignment::Center)
    .child(distribution_segment("flex(1) - 1/4", PRIMARY, PRIMARY_30))
    .child(distribution_segment("flex(2) - 2/4", SECONDARY, SECONDARY_30))
    .child(distribution_segment("flex(1) - 1/4", ACCENT, ACCENT_30))
    .padding(20.0)
    .width(FILL_WIDTH)
    .height(104.0)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn distribution_segment(label: &str, color: &str, fill: &str) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 13.0, FontWeight::Normal, TEXT))
    .height(64.0)
    .flex(1.0)
    .background(fill)
    .border_inside(2.0, Color::from_hex(color))
    .rounded(PANEL_RADIUS)
    .into()
}

fn stack_overlay() -> Element {
  lurq::components::Stack::new()
    .child(
      lurq::components::Rect::new(200.0, 200.0)
        .background(PRIMARY)
        .rounded(SECTION_RADIUS)
        .absolute_position(280.0, 30.0),
    )
    .child(
      lurq::components::Rect::new(150.0, 150.0)
        .background(SUCCESS)
        .rounded(SECTION_RADIUS)
        .absolute_position(305.0, 55.0),
    )
    .child(
      lurq::components::Rect::new(100.0, 100.0)
        .background(ERROR)
        .rounded(SECTION_RADIUS)
        .absolute_position(330.0, 80.0),
    )
    .child(
      text(
        "3 overlapping squares centered via stack_align(Center)",
        12.0,
        FontWeight::Normal,
        TEXT_MUTED,
      )
      .absolute_position(240.0, 235.0),
    )
    .width(FILL_WIDTH)
    .height(260.0)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn flex_wrap_demo() -> Element {
  lurq::components::Column::new()
    .child(
      lurq::components::Row::new()
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
            ("9", ERROR),
            ("10", WARNING),
            ("11", ACCENT),
            ("12", SECONDARY),
          ]
          .into_iter()
          .map(|(label, color)| wrap_item(label, color)),
        )
        .wrap()
        .padding(12.0)
        .width(FILL_WIDTH)
        .height(114.0)
        .background(BG)
        .rounded(PANEL_RADIUS),
    )
    .padding(24.0)
    .width(FILL_WIDTH)
    .height(162.0)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn demo_card(title: &str, body: Element) -> Element {
  lurq::components::Column::new()
    .spacing(12.0)
    .child(text(title, 12.0, FontWeight::Normal, TEXT_MUTED))
    .child(body)
    .padding(12.0)
    .height(140.0)
    .flex(1.0)
    .background(SURFACE_DARK)
    .rounded(PANEL_RADIUS)
    .into()
}

fn row_col_box(label: &str, color: &str, width: impl Into<Dimension>, height: f32) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 14.0, FontWeight::Bold, TEXT))
    .size(width, height)
    .background(color)
    .rounded(ROW_COL_BOX_RADIUS)
    .into()
}

fn align_block(color: &str, height: f32) -> Element {
  lurq::components::Spacer::new()
    .size(36.0, height)
    .background(color)
    .rounded(ALIGN_BLOCK_RADIUS)
    .into()
}

fn wrap_item(label: &str, color: &str) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 14.0, FontWeight::Bold, TEXT))
    .size(90.0, 40.0)
    .background(color)
    .rounded(PANEL_RADIUS)
    .into()
}

fn justify_swatch(color: &str) -> Element {
  lurq::components::Spacer::new()
    .size(56.0, 28.0)
    .background(color)
    .rounded(CONTROL_RADIUS)
    .into()
}
