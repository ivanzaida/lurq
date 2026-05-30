use super::{
  snapshot::{DevToolsNode, FrameProfileSnapshot},
  style::{
    BG, BLUE, BORDER, FILL, MUTED, ORANGE, PINK, PRIMARY, SIGNAL_GREEN, SURFACE_2, TEXT, YELLOW, badge, icon,
    short_tag, text,
  },
};
use crate::{
  components::{Column, Rect, Row, ScrollVertical, Spacer, Text},
  layout::{
    Alignment,
    text_style::{FontStyle, FontWeight, TextStyle},
  },
  node::{Element, color::Color},
};

pub(crate) fn inspector_panel(selected: Option<&DevToolsNode>, frame: FrameProfileSnapshot) -> Element {
  let title = selected
    .map(|node| short_tag(&node.tag).to_owned())
    .unwrap_or_else(|| "Counter".to_owned());
  let node_id = selected
    .map(|node| format!("NodeId({})", node.id.value()))
    .unwrap_or_else(|| "NodeId(42)".to_owned());
  let child_summary = selected.map(child_summary).unwrap_or_else(|| "1 (Row)".to_owned());

  Column::new()
    .child(inspector_title(&title))
    .child(
      ScrollVertical::new(
        Column::new()
          .child(props_section())
          .child(signals_section())
          .child(collapsed_section("CONTEXT", "share-2", YELLOW, "1"))
          .child(render_section(&title, &node_id, &child_summary, frame))
          .child(collapsed_section("EFFECTS", "circle-play", PINK, "0"))
          .width(FILL),
      )
      .width(FILL)
      .height(FILL),
    )
    .width(FILL)
    .height(FILL)
    .fill(BG)
    .into()
}

fn inspector_title(title: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(icon("box", 16.0, BLUE))
    .child(text(title, 14.0, FontWeight::Bold, BLUE))
    .child(badge("Component", PRIMARY, "#a855f720"))
    .child(Spacer::new().flex(1.0))
    .child(text("reactivity_demo.rs:42", 10.0, FontWeight::Normal, MUTED))
    .child(icon("external-link", 12.0, MUTED))
    .height(46.0)
    .width(FILL)
    .pad_xy(16.0, 0.0)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn props_section() -> Element {
  Column::new()
    .child(section_header("PROPS", None, ORANGE, "0", true))
    .child(
      Row::new()
        .child(italic_mono("Props = ()  (unit type)", 11.0, MUTED))
        .width(FILL)
        .pad_xy(40.0, 8.0)
        .height(36.0),
    )
    .width(FILL)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn signals_section() -> Element {
  Column::new()
    .child(section_header("SIGNALS", Some("zap"), SIGNAL_GREEN, "2", true))
    .child(signal_row("count", "Signal<i32>", "5", "1 subscriber"))
    .child(signal_row("step", "Signal<i32>", "1", "0 subscribers"))
    .child(Spacer::new().height(4.0))
    .width(FILL)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn render_section(title: &str, node_id: &str, child_summary: &str, frame: FrameProfileSnapshot) -> Element {
  Column::new()
    .child(section_header("RENDER INFO", Some("refresh-cw"), PRIMARY, "", true))
    .child(render_info_row(
      "Rendered by",
      if title == "Counter" { "ReactivityDemo" } else { title },
      BLUE,
    ))
    .child(render_info_row("Render count", "7", TEXT))
    .child(render_info_row(
      "Last render",
      &format!("{:.1}ms", frame.total_ms.max(2.1)),
      SIGNAL_GREEN,
    ))
    .child(render_info_row("Dirty", "false", MUTED))
    .child(render_info_row("Children", child_summary, TEXT))
    .child(render_info_row("Node ID", node_id, MUTED))
    .child(Spacer::new().height(6.0))
    .width(FILL)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn collapsed_section(title: &str, icon_name: &str, color: &str, count: &str) -> Element {
  Column::new()
    .child(section_header(title, Some(icon_name), color, count, false))
    .width(FILL)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn section_header(title: &str, leading_icon: Option<&str>, color: &str, count: &str, expanded: bool) -> Element {
  let mut row = Row::new().align_items(Alignment::Center).spacing(6.0).child(icon(
    if expanded { "chevron-down" } else { "chevron-right" },
    12.0,
    MUTED,
  ));
  if let Some(icon_name) = leading_icon {
    row = row.child(icon(icon_name, 12.0, color));
  }
  row = row.child(text(title, 11.0, FontWeight::Bold, color));
  if !count.is_empty() {
    row = row.child(badge(count, MUTED, SURFACE_2));
  }
  row.width(FILL).height(34.0).pad_xy(16.0, 0.0).into()
}

fn signal_row(name: &str, ty: &str, value: &str, subscribers: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(Rect::new(6.0, 6.0).fill(SIGNAL_GREEN).rounded(3.0))
    .child(Spacer::new().width(8.0))
    .child(text(name, 12.0, FontWeight::Medium, TEXT))
    .child(Spacer::new().width(4.0))
    .child(text(ty, 11.0, FontWeight::Normal, MUTED))
    .child(Spacer::new().flex(1.0))
    .child(badge(value, SIGNAL_GREEN, "#4ade8015"))
    .child(Spacer::new().width(12.0))
    .child(text(subscribers, 10.0, FontWeight::Normal, MUTED))
    .width(FILL)
    .height(30.0)
    .pad_xy(40.0, 0.0)
    .into()
}

fn render_info_row(label: &str, value: &str, value_color: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(text(label, 11.0, FontWeight::Normal, MUTED))
    .child(Spacer::new().flex(1.0))
    .child(text(value, 11.0, FontWeight::Medium, value_color).nowrap())
    .width(FILL)
    .height(25.0)
    .pad_xy(40.0, 0.0)
    .into()
}

fn italic_mono(content: &str, size: f32, color: &str) -> Text {
  Text::styled(
    content,
    TextStyle {
      font_family: "monospace".into(),
      font_size: size,
      style: FontStyle::Italic,
      color: Color::from_hex(color),
      ..Default::default()
    },
  )
}

fn child_summary(node: &DevToolsNode) -> String {
  match node.children.as_slice() {
    [] => "0".to_owned(),
    [child] => format!("1 ({})", short_tag(&child.tag)),
    children => format!("{}", children.len()),
  }
}
