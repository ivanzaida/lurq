use lurq::{
  app::{component::Component, ctx::Ctx},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, PRIMARY, SUCCESS, SURFACE, TEXT, TEXT_MUTED, text};

const CONTENT_PAD: f32 = 32.0;
const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const SECTION_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;
const CONTROL_RADIUS: f32 = 4.0;
const GHOST_PRIMARY: &str = "#3b82f626";

pub(crate) struct PositioningDemo;

impl Component for PositioningDemo {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Positioning", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(section_title("Relative Offset"))
      .child(relative_offset())
      .child(section_title("Absolute Positioning"))
      .child(absolute_positioning())
      .child(section_title("Stack Alignment (9-point)"))
      .child(stack_alignment_grid())
      .pad(CONTENT_PAD)
      .width(FILL_WIDTH)
      .fill(BG)
  }
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn relative_offset() -> Element {
  lurq::components::Row::new()
    .spacing(24.0)
    .align_items(Alignment::Start)
    .child(offset_normal_sample().flex(1.0))
    .child(offset_sample("With offset(20, 10)", 20.0, false).flex(1.0))
    .child(offset_sample("With offset + overflow visible", 20.0, true).flex(1.0))
    .pad_xy(40.0, 24.0)
    .width(FILL_WIDTH)
    .height(178.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn offset_normal_sample() -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(8.0)
    .child(text("Normal", 12.0, FontWeight::Normal, TEXT_MUTED))
    .child(offset_box("A", PRIMARY))
    .child(offset_box("B", SUCCESS))
    .height(130.0)
}

fn offset_sample(label: &str, offset_x: f32, visible_overflow: bool) -> lurq::components::Column {
  let shifted = offset_box("A", PRIMARY)
    .relative(offset_x, 10.0)
    .absolute_position(0.0, 0.0);

  let mut body = lurq::components::Stack::new()
    .child(
      lurq::components::Rect::new(80.0, 40.0)
        .fill(GHOST_PRIMARY)
        .border_inside(1.0, Color::from_hex(PRIMARY))
        .rounded(CONTROL_RADIUS)
        .absolute_position(0.0, 0.0),
    )
    .child(if visible_overflow {
      shifted.overflow_visible()
    } else {
      shifted
    })
    .child(offset_box("B", SUCCESS).absolute_position(0.0, 54.0))
    .size(if visible_overflow { 90.0 } else { 120.0 }, 98.0);

  if visible_overflow {
    body = body.overflow_visible();
  }

  let mut column = lurq::components::Column::new()
    .spacing(8.0)
    .child(text(label, 12.0, FontWeight::Normal, TEXT_MUTED))
    .child(body)
    .height(130.0);

  if visible_overflow {
    column = column.overflow_visible();
  }

  column
}

fn absolute_positioning() -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(static_absolute_stack())
    .width(FILL_WIDTH)
    .height(340.0)
    .pad_xy(32.0, 30.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn static_absolute_stack() -> lurq::components::Stack {
  lurq::components::Stack::new()
    .child(text("Stack (400x280)", 11.0, FontWeight::Normal, TEXT_MUTED).absolute_position(8.0, 8.0))
    .child(abs_box("abs(20, 40)", PRIMARY).absolute_position(20.0, 40.0))
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(text("abs(220, 160)", 11.0, FontWeight::Normal, TEXT))
        .size(120.0, 80.0)
        .fill(SUCCESS)
        .rounded(PANEL_RADIUS)
        .absolute_position(220.0, 160.0),
    )
    .size(400.0, 280.0)
    .fill(BG)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
}

fn abs_box(label: &str, color: &str) -> lurq::components::Row {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 11.0, FontWeight::Normal, TEXT))
    .size(120.0, 80.0)
    .fill(color)
    .rounded(PANEL_RADIUS)
}

fn stack_alignment_grid() -> Element {
  let cells = [
    ("TopStart", StackAlignment::TopStart, 20.0, 20.0),
    ("TopCenter", StackAlignment::TopCenter, 292.0, 20.0),
    ("TopEnd", StackAlignment::TopEnd, 564.0, 20.0),
    ("CenterStart", StackAlignment::CenterStart, 20.0, 126.0),
    ("Center", StackAlignment::Center, 292.0, 126.0),
    ("CenterEnd", StackAlignment::CenterEnd, 564.0, 126.0),
    ("BottomStart", StackAlignment::BottomStart, 20.0, 232.0),
    ("BottomCenter", StackAlignment::BottomCenter, 292.0, 232.0),
    ("BottomEnd", StackAlignment::BottomEnd, 564.0, 232.0),
  ];

  let mut grid = lurq::components::Stack::new();
  for (label, alignment, x, y) in cells {
    grid = grid.child(alignment_cell(label, alignment).absolute_position(x, y));
  }

  grid
    .width(FILL_WIDTH)
    .height(340.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn alignment_cell(label: &str, alignment: StackAlignment) -> lurq::components::Stack {
  lurq::components::Stack::new()
    .stack_align(alignment)
    .child(
      lurq::components::Rect::new(40.0, 30.0)
        .fill(PRIMARY)
        .rounded(CONTROL_RADIUS),
    )
    .child(text(label, 10.0, FontWeight::Normal, TEXT_MUTED).absolute_position(6.0, 4.0))
    .size(260.0, 96.0)
    .fill(BG)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(PANEL_RADIUS)
}

fn offset_box(label: &str, color: &str) -> lurq::components::Row {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 14.0, FontWeight::Bold, TEXT))
    .size(80.0, 40.0)
    .fill(color)
    .rounded(CONTROL_RADIUS)
}
