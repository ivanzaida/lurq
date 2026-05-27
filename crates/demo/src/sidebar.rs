use lurq::{
  layout::{Alignment, text_style::FontWeight},
  node::Element,
};

use crate::style::{NAV_SELECTED, PRIMARY, SURFACE_DARK, TEXT, TEXT_MUTED, text};

pub(crate) fn sidebar() -> Element {
  Element::column()
    .child(
      Element::column()
        .spacing(2.0)
        .child(text("lurq engine demo", 12.0, FontWeight::Bold, TEXT))
        .child(text("Layout", 10.0, FontWeight::Medium, TEXT_MUTED))
        .pad_xy(16.0, 10.0)
        .width(200.0)
        .height(56.0),
    )
    .with_children(
      [
        ("Layout", true),
        ("Sizing", false),
        ("Position", false),
        ("Scroll", false),
        ("Visual", false),
        ("Text", false),
        ("Events", false),
        ("React.", false),
        ("Comps.", false),
        ("Context", false),
        ("Debug", false),
      ]
      .into_iter()
      .map(|(label, selected)| sidebar_item(label, selected)),
    )
    .width(200.0)
    .height(1702.0)
    .fill(SURFACE_DARK)
}

fn sidebar_item(label: &str, selected: bool) -> Element {
  Element::row()
    .align_items(Alignment::Center)
    .child(Element::rect(3.0, 38.0).fill(if selected { PRIMARY } else { "#00000000" }))
    .child(Element::spacer().width(13.0))
    .child(text(
      label,
      11.0,
      if selected { FontWeight::Bold } else { FontWeight::Medium },
      if selected { TEXT } else { TEXT_MUTED },
    ))
    .width(200.0)
    .height(38.0)
    .fill(if selected { NAV_SELECTED } else { "#00000000" })
}
