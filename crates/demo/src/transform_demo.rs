use std::f32::consts::PI;

use lurq::{
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension, transform::Transform2D},
};

use crate::style::{ACCENT, BG, BORDER, PRIMARY, SECONDARY, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

pub(crate) fn transform_content() -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("Transforms", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("Rotate"))
    .child(text(
      "GPU-accelerated rotation around the element center. Paint-only — no layout impact.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(rotate_section())
    .child(section_title("Scale"))
    .child(text(
      "Uniform and non-uniform scaling.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(scale_section())
    .child(section_title("Skew"))
    .child(text(
      "Horizontal and vertical shear distortion.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(skew_section())
    .child(section_title("Combined"))
    .child(text(
      "Multiple transforms composed via .then() — applied right to left.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(combined_section())
    .child(section_title("With Text"))
    .child(text(
      "Transforms apply to the entire subtree including text children.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(text_section())
    .padding(CONTENT_PAD)
    .width(FILL_WIDTH)
    .background(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn card(label: &str, content: impl Into<Element>) -> Element {
  lurq::components::Column::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(content)
        .size(130.0, 110.0),
    )
    .child(text(label, 11.0, FontWeight::Medium, TEXT_MUTED))
    .into()
}

fn card_row(children: Vec<Element>) -> Element {
  lurq::components::Row::new()
    .spacing(16.0)
    .align_items(Alignment::Start)
    .with_children(children)
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn box_60(color: &str) -> lurq::components::Rect {
  lurq::components::Rect::new(60.0, 60.0).background(color).rounded(8.0)
}

fn rotate_section() -> Element {
  card_row(vec![
    card("0°", box_60(PRIMARY)),
    card("15°", box_60(PRIMARY).transform(Transform2D::rotate_deg(15.0))),
    card("30°", box_60(PRIMARY).transform(Transform2D::rotate_deg(30.0))),
    card("45°", box_60(PRIMARY).transform(Transform2D::rotate_deg(45.0))),
    card("90°", box_60(PRIMARY).transform(Transform2D::rotate_deg(90.0))),
    card("180°", box_60(PRIMARY).transform(Transform2D::rotate_deg(180.0))),
  ])
}

fn scale_section() -> Element {
  card_row(vec![
    card("0.5x", box_60(ACCENT).transform(Transform2D::scale_uniform(0.5))),
    card("0.75x", box_60(ACCENT).transform(Transform2D::scale_uniform(0.75))),
    card("1.0x", box_60(ACCENT)),
    card("1.5x", box_60(ACCENT).transform(Transform2D::scale_uniform(1.5))),
    card("1.4 x 0.7", box_60(ACCENT).transform(Transform2D::scale(1.4, 0.7))),
    card("0.6 x 1.4", box_60(ACCENT).transform(Transform2D::scale(0.6, 1.4))),
  ])
}

fn skew_section() -> Element {
  card_row(vec![
    card(
      "X 10°",
      box_60(SECONDARY).transform(Transform2D::skew(10.0 * PI / 180.0, 0.0)),
    ),
    card(
      "X 20°",
      box_60(SECONDARY).transform(Transform2D::skew(20.0 * PI / 180.0, 0.0)),
    ),
    card(
      "X -20°",
      box_60(SECONDARY).transform(Transform2D::skew(-20.0 * PI / 180.0, 0.0)),
    ),
    card(
      "Y 10°",
      box_60(SECONDARY).transform(Transform2D::skew(0.0, 10.0 * PI / 180.0)),
    ),
    card(
      "Y 20°",
      box_60(SECONDARY).transform(Transform2D::skew(0.0, 20.0 * PI / 180.0)),
    ),
    card(
      "XY 10°",
      box_60(SECONDARY).transform(Transform2D::skew(10.0 * PI / 180.0, 10.0 * PI / 180.0)),
    ),
  ])
}

fn combined_section() -> Element {
  card_row(vec![
    card(
      "Rotate + Scale",
      box_60(PRIMARY).transform(Transform2D::rotate_deg(30.0).then(&Transform2D::scale(1.3, 0.8))),
    ),
    card(
      "Scale + Rotate",
      box_60("#f59e0b").transform(Transform2D::scale(1.3, 0.8).then(&Transform2D::rotate_deg(30.0))),
    ),
    card(
      "Rotate + Skew",
      box_60(ACCENT).transform(Transform2D::rotate_deg(15.0).then(&Transform2D::skew(0.3, 0.0))),
    ),
    card(
      "Scale + Skew",
      box_60("#ef4444").transform(Transform2D::scale(1.2, 0.8).then(&Transform2D::skew(0.2, 0.0))),
    ),
    card(
      "All Three",
      box_60(SECONDARY).transform(
        Transform2D::rotate_deg(20.0)
          .then(&Transform2D::scale(1.2, 0.8))
          .then(&Transform2D::skew(0.15, 0.0)),
      ),
    ),
  ])
}

fn text_box(label: &str, color: &str, xf: Transform2D) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 14.0, FontWeight::Bold, "#ffffff"))
    .size(100.0, 50.0)
    .background(color)
    .rounded(8.0)
    .transform(xf)
    .into()
}

fn text_section() -> Element {
  card_row(vec![
    card("Rotate 15°", text_box("Hello", PRIMARY, Transform2D::rotate_deg(15.0))),
    card("Rotate 45°", text_box("Hello", ACCENT, Transform2D::rotate_deg(45.0))),
    card(
      "Scale 1.3x",
      text_box("Hello", SECONDARY, Transform2D::scale_uniform(1.3)),
    ),
    card("Skew X", text_box("Hello", "#ef4444", Transform2D::skew(0.3, 0.0))),
    card(
      "Rotate + Scale",
      text_box(
        "Hello",
        "#f59e0b",
        Transform2D::rotate_deg(20.0).then(&Transform2D::scale(1.2, 0.9)),
      ),
    ),
  ])
}
