use super::{
  snapshot::{DevToolsNode, FrameProfileSnapshot},
  style::{
    BG, BLUE, BORDER, FILL, MUTED, PRIMARY, badge, empty_section, empty_state, info_row, section_title, short_tag, text,
  },
};
use crate::{
  components::{Column, Row, ScrollVertical, Spacer},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, color::Color},
};

pub(crate) fn inspector_panel(selected: Option<&DevToolsNode>, frame: FrameProfileSnapshot) -> Element {
  let title = selected
    .map(|node| short_tag(&node.tag).to_owned())
    .unwrap_or_else(|| "No selection".to_owned());
  let id = selected.map(|node| node.id.value()).unwrap_or(0);

  Column::new()
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(8.0)
        .child(text("box", 11.0, FontWeight::Bold, BLUE))
        .child(text(&title, 14.0, FontWeight::Bold, BLUE))
        .child(badge("Node", PRIMARY, "#a855f720"))
        .child(Spacer::new().flex(1.0))
        .child(text(&format!("node #{id}"), 10.0, FontWeight::Normal, MUTED))
        .height(42.0)
        .width(FILL)
        .pad_xy(16.0, 0.0)
        .border_inside(1.0, Color::from_hex(BORDER)),
    )
    .child(
      ScrollVertical::new(
        Column::new()
          .child(props_section(selected))
          .child(render_section(frame))
          .child(empty_section("Signals", "Signals are not instrumented yet."))
          .child(empty_section("Context", "Context providers will appear here."))
          .child(empty_section("Effects", "No effects registered for this node."))
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

fn props_section(selected: Option<&DevToolsNode>) -> Element {
  let mut rows = Vec::new();
  if let Some(node) = selected {
    rows.push(info_row("tag", short_tag(&node.tag)));
    rows.push(info_row("node_id", &node.id.value().to_string()));
    rows.push(info_row("children", &node.children.len().to_string()));
    if let Some(text_value) = &node.text {
      rows.push(info_row("text", text_value));
    }
    if let Some(color) = &node.color {
      rows.push(info_row("fill", color));
    }
  } else {
    rows.push(empty_state("Select a node from the tree."));
  }

  Column::new()
    .child(section_title("Props"))
    .with_children(rows)
    .width(FILL)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn render_section(frame: FrameProfileSnapshot) -> Element {
  Column::new()
    .child(section_title("Render"))
    .child(info_row("total", &format!("{:.2} ms", frame.total_ms)))
    .child(info_row("layout", &format!("{:.2} ms", frame.layout_ms)))
    .child(info_row("resolve", &format!("{:.2} ms", frame.quad_ms)))
    .child(info_row("glyph", &format!("{:.2} ms", frame.glyph_ms)))
    .child(info_row("encode", &format!("{:.2} ms", frame.encode_ms)))
    .child(info_row("present", &format!("{:.2} ms", frame.present_ms)))
    .child(info_row("quads", &frame.quad_count.to_string()))
    .child(info_row("rects", &frame.rect_count.to_string()))
    .child(info_row("glyphs", &frame.glyph_count.to_string()))
    .child(info_row("memory", &format!("{:.1} KiB", frame.memory_kib)))
    .width(FILL)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}
