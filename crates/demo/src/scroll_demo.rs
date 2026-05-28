use lurq::{
  layout::{
    Alignment,
    layout_kind::Justify,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
    text_style::FontWeight,
  },
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, PRIMARY, SURFACE, SURFACE_DARK, TEXT, TEXT_MUTED, text};

const CONTENT_PAD: f32 = 32.0;
const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const SECTION_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;

pub(crate) fn scroll_content() -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("Scroll Containers", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("Vertical Scroll"))
    .child(vertical_scroll_demo())
    .child(section_title("Horizontal Scroll"))
    .child(horizontal_scroll_demo())
    .child(section_title("Both-Axis Scroll"))
    .child(both_axis_scroll_demo())
    .pad(CONTENT_PAD)
    .width(FILL_WIDTH)
    .fill(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn scrollbar(placement: ScrollBarPlacement) -> ScrollBarStyle {
  ScrollBarStyle {
    visible: ScrollBarVisibility::Always,
    placement,
    width: 7.0,
    min_thumb_length: 32.0,
    thumb_color: Color::from_hex(PRIMARY),
    thumb_radius: 4.0,
    track_color: Color::from_hex("#0f172a66"),
    track_radius: 4.0,
    padding: 3.0,
  }
}

fn vertical_scroll_demo() -> Element {
  demo_panel(
    lurq::components::Column::new()
      .spacing(12.0)
      .child(text(
        "Scrollable list - 20 items, viewport shows about 6",
        12.0,
        FontWeight::Normal,
        TEXT_MUTED,
      ))
      .child(
        lurq::components::Row::new()
          .spacing(12.0)
          .width(FILL_WIDTH)
          .child(vertical_scroll_case("Overlay", ScrollBarPlacement::Overlay).flex(1.0))
          .child(vertical_scroll_case("Reserved", ScrollBarPlacement::Reserved).flex(1.0)),
      ),
  )
}

fn vertical_scroll_case(label: &str, placement: ScrollBarPlacement) -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(8.0)
    .child(placement_label(label))
    .child(
      lurq::components::ScrollVertical::new(
        lurq::components::Column::new()
          .spacing(4.0)
          .with_children((1..=20).map(vertical_item)),
      )
      .scrollbar(scrollbar(placement))
      .width(FILL_WIDTH)
      .height(220.0)
      .fill(BG)
      .rounded(PANEL_RADIUS),
    )
}

fn vertical_item(index: usize) -> lurq::components::Row {
  let fill = if index % 2 == 0 { BG } else { SURFACE };

  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .child(text(&format!("Item {}", index), 11.0, FontWeight::Medium, TEXT))
    .pad_xy(12.0, 0.0)
    .height(32.0)
    .width(FILL_WIDTH)
    .fill(fill)
    .rounded(4.0)
}

fn horizontal_scroll_demo() -> Element {
  demo_panel(
    lurq::components::Row::new()
      .spacing(12.0)
      .width(FILL_WIDTH)
      .child(horizontal_scroll_case("Overlay", ScrollBarPlacement::Overlay).flex(1.0))
      .child(horizontal_scroll_case("Reserved", ScrollBarPlacement::Reserved).flex(1.0)),
  )
}

fn horizontal_scroll_case(label: &str, placement: ScrollBarPlacement) -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(8.0)
    .child(placement_label(label))
    .child(
      lurq::components::ScrollHorizontal::new(
        lurq::components::Row::new()
          .spacing(12.0)
          .with_children((1..=8).map(horizontal_card)),
      )
      .scrollbar(scrollbar(placement))
      .width(FILL_WIDTH)
      .height(116.0)
      .fill(BG)
      .rounded(PANEL_RADIUS),
    )
}

fn horizontal_card(index: usize) -> lurq::components::Row {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(&format!("Card {}", index), 12.0, FontWeight::Bold, TEXT))
    .size(150.0, 80.0)
    .fill(BG)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
}

fn both_axis_scroll_demo() -> Element {
  demo_panel(
    lurq::components::Row::new()
      .spacing(12.0)
      .align_items(Alignment::Center)
      .width(FILL_WIDTH)
      .child(both_axis_scroll_case("Overlay", ScrollBarPlacement::Overlay).flex(1.0))
      .child(both_axis_scroll_case("Reserved", ScrollBarPlacement::Reserved).flex(1.0)),
  )
}

fn both_axis_scroll_case(label: &str, placement: ScrollBarPlacement) -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(8.0)
    .child(placement_label(label))
    .child(
      lurq::components::ScrollBoth::new(grid_content())
        .scrollbar(scrollbar(placement))
        .width(FILL_WIDTH)
        .height(180.0)
        .fill(BG)
        .rounded(PANEL_RADIUS),
    )
    .child(text(
      "2D scrollable grid (720x420 content)",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
}

fn placement_label(label: &str) -> Element {
  text(label, 12.0, FontWeight::Bold, TEXT_MUTED).width(FILL_WIDTH).into()
}

fn grid_content() -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(4.0)
    .pad(4.0)
    .with_children((1..=7).map(grid_row))
}

fn grid_row(row: usize) -> lurq::components::Row {
  lurq::components::Row::new()
    .spacing(4.0)
    .with_children((1..=12).map(move |column| grid_cell(row, column)))
}

fn grid_cell(row: usize, column: usize) -> lurq::components::Row {
  let fill = if (row + column) % 2 == 0 { SURFACE_DARK } else { SURFACE };

  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(
      &format!("{}:{}", row, column),
      10.0,
      FontWeight::Medium,
      TEXT_MUTED,
    ))
    .size(56.0, 56.0)
    .fill(fill)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(4.0)
}

fn demo_panel(content: impl Into<Element>) -> Element {
  lurq::components::Column::new()
    .child(content)
    .pad(20.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}
