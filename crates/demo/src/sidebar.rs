use lurq::{
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element},
};

use crate::style::{DemoTheme, text};

#[derive(Clone, Copy, PartialEq, Eq, lurq::DevtoolsInspectable)]
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
  Inputs,
  Events,
  Reactivity,
  Components,
  Context,
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
      Self::Inputs => "Inputs",
      Self::Events => "Events",
      Self::Reactivity => "React.",
      Self::Components => "Comps.",
      Self::Context => "Context",
    }
  }
}

pub(crate) fn sidebar(selected: DemoTab, selected_tab: Signal<DemoTab>, theme: DemoTheme) -> Element {
  let palette = theme.palette();
  lurq::components::Column::new()
    .child(
      lurq::components::Column::new()
        .spacing(2.0)
        .child(text("lurq engine demo", 12.0, FontWeight::Bold, palette.text))
        .child(text(selected.label(), 10.0, FontWeight::Medium, palette.text_muted))
        .padding_horizontal(16.0)
        .padding_vertical(10.0)
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
        ("Inputs", Some(DemoTab::Inputs)),
        ("Events", Some(DemoTab::Events)),
        ("React.", Some(DemoTab::Reactivity)),
        ("Comps.", Some(DemoTab::Components)),
        ("Context", Some(DemoTab::Context)),
      ]
      .into_iter()
      .map(move |(label, tab)| sidebar_item(label, tab == Some(selected), tab, selected_tab.clone(), theme)),
    )
    .width(200.0)
    .background(palette.surface_dark)
    .into()
}

fn sidebar_item(
  label: &str,
  selected: bool,
  tab: Option<DemoTab>,
  selected_tab: Signal<DemoTab>,
  theme: DemoTheme,
) -> Element {
  let palette = theme.palette();
  let mut item = lurq::components::Row::new()
    .align_items(Alignment::Center)
    .child(lurq::components::Rect::new(3.0, 38.0).background(if selected { palette.primary } else { "#00000000" }))
    .child(lurq::components::Spacer::new().width(13.0))
    .child(text(
      label,
      11.0,
      if selected { FontWeight::Bold } else { FontWeight::Medium },
      if selected { palette.text } else { palette.text_muted },
    ))
    .width(200.0)
    .height(38.0)
    .cursor(CursorIcon::Pointer)
    .background(if selected { palette.nav_selected } else { "#00000000" });

  if let Some(tab) = tab {
    item = item.on_click(move |_| selected_tab.set(tab));
  }

  item.into()
}
