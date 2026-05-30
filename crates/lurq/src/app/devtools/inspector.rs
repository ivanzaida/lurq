use super::{
  snapshot::{DevToolsNode, DevToolsNodeKind, FrameProfileSnapshot},
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
  node::{Element, border::Border, color::Color, dimension::Dimension, padding::Padding},
};

pub(crate) fn inspector_panel(selected: Option<&DevToolsNode>, frame: FrameProfileSnapshot) -> Element {
  let title = selected
    .map(|node| short_tag(&node.tag).to_owned())
    .unwrap_or_else(|| "Counter".to_owned());
  let node_id = selected
    .map(|node| format!("NodeId({})", node.id.value()))
    .unwrap_or_else(|| "NodeId(42)".to_owned());
  let child_summary = selected.map(child_summary).unwrap_or_else(|| "1 (Row)".to_owned());
  let kind = selected.map(|node| node.kind).unwrap_or(DevToolsNodeKind::Component);
  let mut details = Column::new().width(FILL);
  match kind {
    DevToolsNodeKind::Component => {
      details = details
        .child(props_section(selected))
        .child(signals_section(selected))
        .child(context_section(selected));
    }
    DevToolsNodeKind::Element => {
      details = details.child(node_shape_section(selected));
    }
  }

  Column::new()
    .child(inspector_title(&title, kind))
    .child(
      ScrollVertical::new(
        details
          .child(render_section(&title, &node_id, &child_summary, frame))
          .child(collapsed_section("EFFECTS", "circle-play", PINK, "0")),
      )
      .width(FILL)
      .height(FILL),
    )
    .width(FILL)
    .height(FILL)
    .fill(BG)
    .into()
}

fn inspector_title(title: &str, kind: DevToolsNodeKind) -> Element {
  let (kind_label, kind_color, kind_fill) = match kind {
    DevToolsNodeKind::Component => ("Component", PRIMARY, "#a855f720"),
    DevToolsNodeKind::Element => ("Element", BLUE, "#60a5fa20"),
  };

  Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(icon("box", 16.0, BLUE))
    .child(mono_text(title, 14.0, FontWeight::Bold, BLUE))
    .child(badge(kind_label, kind_color, kind_fill))
    .child(Spacer::new().flex(1.0))
    .child(mono_text("reactivity_demo.rs:42", 10.0, FontWeight::Normal, MUTED))
    .child(icon("external-link", 12.0, MUTED))
    .padding_horizontal(16.0)
    .padding_vertical(10.0)
    .width(FILL)
    .border_bottom(divider())
    .into()
}

fn props_section(selected: Option<&DevToolsNode>) -> Element {
  let props = selected.and_then(|node| node.props.as_ref());
  let count = props.map(|props| props.fields.len()).unwrap_or(0);
  let mut section = Column::new()
    .child(section_header("PROPS", None, ORANGE, &count.to_string(), true))
    .width(FILL)
    .border_bottom(divider());

  if let Some(props) = props {
    if props.fields.is_empty() {
      section = section.child(props_empty_row());
    } else {
      section = section.child(prop_row("type", short_type_name(&props.type_name), ORANGE));
      for field in &props.fields {
        section = section.child(prop_row(field.name(), short_type_name(field.value()), TEXT));
      }
    }
  } else {
    section = section.child(props_empty_row());
  }

  section.into()
}

fn signals_section(selected: Option<&DevToolsNode>) -> Element {
  let signals = selected.map(|node| node.signals.as_slice()).unwrap_or(&[]);
  let mut section = Column::new()
    .child(section_header(
      "SIGNALS",
      Some("zap"),
      SIGNAL_GREEN,
      &signals.len().to_string(),
      true,
    ))
    .width(FILL)
    .border_bottom(divider());

  if signals.is_empty() {
    section = section.child(signal_empty_row());
  } else {
    for signal in signals {
      section = section.child(signal_row(signal.id, &signal.type_name));
    }
    section = section.child(Spacer::new().height(4.0));
  }

  section.into()
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
    .border_bottom(divider())
    .into()
}

fn collapsed_section(title: &str, icon_name: &str, color: &str, count: &str) -> Element {
  Column::new()
    .child(section_header(title, Some(icon_name), color, count, false))
    .width(FILL)
    .border_bottom(divider())
    .into()
}

fn context_section(selected: Option<&DevToolsNode>) -> Element {
  let contexts = selected.map(|node| node.contexts.as_slice()).unwrap_or(&[]);
  let mut section = Column::new()
    .child(section_header(
      "CONTEXT",
      Some("share-2"),
      YELLOW,
      &contexts.len().to_string(),
      true,
    ))
    .width(FILL)
    .border_bottom(divider());

  if contexts.is_empty() {
    section = section.child(context_empty_row());
  } else {
    for context in contexts {
      section = section.child(context_row(context.kind, &context.type_name));
    }
    section = section.child(Spacer::new().height(4.0));
  }

  section.into()
}

fn node_shape_section(selected: Option<&DevToolsNode>) -> Element {
  let shape = selected.map(|node| node.shape.as_slice()).unwrap_or(&[]);
  let mut section = Column::new()
    .child(section_header(
      "NODE",
      Some("box"),
      BLUE,
      &shape.len().to_string(),
      true,
    ))
    .width(FILL)
    .border_bottom(divider());

  if shape.is_empty() {
    section = section.child(node_shape_empty_row());
  } else {
    for row in shape {
      section = section.child(node_shape_row(&row.label, &row.value));
    }
    section = section.child(Spacer::new().height(4.0));
  }

  section.into()
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
    row = row.child(badge(count, color, SURFACE_2));
  }
  row.padding_horizontal(16.0).padding_vertical(8.0).width(FILL).into()
}

fn signal_row(id: usize, ty: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(Rect::new(6.0, 6.0).fill(SIGNAL_GREEN).rounded(3.0))
    .child(Spacer::new().width(8.0))
    .child(mono_text(&format!("#{id}"), 12.0, FontWeight::Medium, TEXT))
    .child(Spacer::new().width(8.0))
    .child(mono_text(
      &format!("Signal<{}>", short_type_name(ty)),
      11.0,
      FontWeight::Normal,
      MUTED,
    ))
    .child(Spacer::new().flex(1.0))
    .padding_custom(Padding {
      top: Dimension::Px(6.0),
      right: Dimension::Px(16.0),
      bottom: Dimension::Px(6.0),
      left: Dimension::Px(40.0),
    })
    .width(FILL)
    .into()
}

fn signal_empty_row() -> Element {
  Row::new()
    .child(italic_mono("No component signals", 11.0, MUTED))
    .padding_custom(content_padding(8.0, 16.0, 12.0, 40.0))
    .width(FILL)
    .into()
}

fn context_row(kind: crate::app::ctx::ComponentContextKind, ty: &str) -> Element {
  let label = match kind {
    crate::app::ctx::ComponentContextKind::Provided => "provides",
    crate::app::ctx::ComponentContextKind::Consumed => "uses",
  };
  Row::new()
    .align_items(Alignment::Center)
    .child(mono_text(label, 11.0, FontWeight::Medium, YELLOW))
    .child(Spacer::new().width(10.0))
    .child(mono_text(short_type_name(ty), 11.0, FontWeight::Normal, MUTED))
    .child(Spacer::new().flex(1.0))
    .padding_custom(content_padding(6.0, 16.0, 6.0, 40.0))
    .width(FILL)
    .into()
}

fn context_empty_row() -> Element {
  Row::new()
    .child(italic_mono("No component context", 11.0, MUTED))
    .padding_custom(content_padding(8.0, 16.0, 12.0, 40.0))
    .width(FILL)
    .into()
}

fn node_shape_empty_row() -> Element {
  Row::new()
    .child(italic_mono("No node shape", 11.0, MUTED))
    .padding_custom(content_padding(8.0, 16.0, 12.0, 40.0))
    .width(FILL)
    .into()
}

fn node_shape_row(label: &str, value: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(mono_text(label, 11.0, FontWeight::Normal, MUTED))
    .child(Spacer::new().width(12.0))
    .child(mono_text(value, 11.0, FontWeight::Medium, TEXT))
    .padding_custom(content_padding(5.0, 16.0, 5.0, 40.0))
    .width(FILL)
    .into()
}

fn render_info_row(label: &str, value: &str, value_color: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(text(label, 11.0, FontWeight::Normal, MUTED))
    .child(Spacer::new().flex(1.0))
    .child(mono_text(value, 11.0, FontWeight::Medium, value_color).nowrap())
    .padding_custom(Padding {
      top: Dimension::Px(5.0),
      right: Dimension::Px(16.0),
      bottom: Dimension::Px(5.0),
      left: Dimension::Px(40.0),
    })
    .width(FILL)
    .into()
}

fn props_empty_row() -> Element {
  Row::new()
    .child(italic_mono("Props = ()  (unit type)", 11.0, MUTED))
    .padding_custom(content_padding(8.0, 16.0, 12.0, 40.0))
    .width(FILL)
    .into()
}

fn prop_row(label: &str, value: &str, value_color: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(mono_text(label, 11.0, FontWeight::Normal, MUTED))
    .child(Spacer::new().width(12.0))
    .child(mono_text(value, 11.0, FontWeight::Medium, value_color).nowrap())
    .padding_custom(content_padding(5.0, 16.0, 5.0, 40.0))
    .width(FILL)
    .into()
}

fn short_type_name(type_name: &str) -> &str {
  type_name.rsplit("::").next().unwrap_or(type_name)
}

fn content_padding(top: f32, right: f32, bottom: f32, left: f32) -> Padding {
  Padding {
    top: Dimension::Px(top),
    right: Dimension::Px(right),
    bottom: Dimension::Px(bottom),
    left: Dimension::Px(left),
  }
}

fn divider() -> Border {
  Border::inside(1.0, Color::from_hex(BORDER))
}

fn mono_text(content: &str, size: f32, weight: FontWeight, color: &str) -> Text {
  Text::styled(
    content,
    TextStyle {
      font_family: "monospace".into(),
      font_size: size,
      weight,
      color: Color::from_hex(color),
      ..Default::default()
    },
  )
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
