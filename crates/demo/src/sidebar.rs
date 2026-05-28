use lurq::{
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element},
};

use crate::style::{NAV_SELECTED, PRIMARY, SURFACE_DARK, TEXT, TEXT_MUTED, text};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DemoTab {
  Layout,
  Sizing,
  Position,
  Dnd,
  Animation,
  Transform,
  Scroll,
  Visual,
  Text,
  Events,
  Reactivity,
}

impl DemoTab {
  pub(crate) fn label(self) -> &'static str {
    match self {
      Self::Layout => "Layout",
      Self::Sizing => "Sizing",
      Self::Position => "Position",
      Self::Dnd => "DnD",
      Self::Animation => "Animation",
      Self::Transform => "Transform",
      Self::Scroll => "Scroll",
      Self::Visual => "Visual",
      Self::Text => "Text",
      Self::Events => "Events",
      Self::Reactivity => "React.",
    }
  }
}

pub(crate) fn sidebar(selected: DemoTab, selected_tab: Signal<DemoTab>) -> Element {
  lurq::components::Column::new()
    .child(
      lurq::components::Column::new()
        .spacing(2.0)
        .child(text("lurq engine demo", 12.0, FontWeight::Bold, TEXT))
        .child(text(selected.label(), 10.0, FontWeight::Medium, TEXT_MUTED))
        .pad_xy(16.0, 10.0)
        .width(200.0)
        .height(56.0),
    )
    .with_children(
      [
        ("Layout", Some(DemoTab::Layout)),
        ("Sizing", Some(DemoTab::Sizing)),
        ("Position", Some(DemoTab::Position)),
        ("DnD", Some(DemoTab::Dnd)),
        ("Animation", Some(DemoTab::Animation)),
        ("Transform", Some(DemoTab::Transform)),
        ("Scroll", Some(DemoTab::Scroll)),
        ("Visual", Some(DemoTab::Visual)),
        ("Text", Some(DemoTab::Text)),
        ("Events", Some(DemoTab::Events)),
        ("React.", Some(DemoTab::Reactivity)),
        ("Comps.", None),
        ("Context", None),
        ("Debug", None),
      ]
      .into_iter()
      .map(move |(label, tab)| sidebar_item(label, tab == Some(selected), tab, selected_tab.clone())),
    )
    .width(200.0)
    .fill(SURFACE_DARK)
    .into()
}

fn sidebar_item(label: &str, selected: bool, tab: Option<DemoTab>, selected_tab: Signal<DemoTab>) -> Element {
  let mut item = lurq::components::Row::new()
    .align_items(Alignment::Center)
    .child(lurq::components::Rect::new(3.0, 38.0).fill(if selected { PRIMARY } else { "#00000000" }))
    .child(lurq::components::Spacer::new().width(13.0))
    .child(text(
      label,
      11.0,
      if selected { FontWeight::Bold } else { FontWeight::Medium },
      if selected { TEXT } else { TEXT_MUTED },
    ))
    .width(200.0)
    .height(38.0)
    .cursor(CursorIcon::Pointer)
    .fill(if selected { NAV_SELECTED } else { "#00000000" });

  if let Some(tab) = tab {
    item = item.on_click(move |_| selected_tab.set(tab));
  }

  item.into()
}
