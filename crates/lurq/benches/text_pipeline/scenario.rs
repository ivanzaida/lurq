//! Shared scripted document updates for the CPU benchmark and native window probe.
use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::{ScrollVertical, Text},
  layout::{layout_kind::ScrollState, text_style::TextStyle},
  node::{Element, dimension::Dimension},
};

const README: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"));

#[derive(Clone, lurq::DevtoolsInspectable)]
pub struct WindowSlot(Arc<Mutex<Option<lurq::app::window::WindowHandle>>>);

impl PartialEq for WindowSlot {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

pub struct DocumentRoot {
  scroll: ScrollState,
}

impl Component for DocumentRoot {
  type Props = (String, WindowSlot, bool);

  fn create(ctx: &mut Ctx) -> Self {
    *ctx.props::<Self::Props>().1.0.lock().unwrap() = Some(ctx.window());
    Self {
      scroll: ScrollState::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let fill = Dimension::Pct(100.0);
    ScrollVertical::new(
      Text::styled(&ctx.props::<Self::Props>().0, TextStyle::default())
        .selectable(ctx.props::<Self::Props>().2)
        .width(fill),
    )
    .with_scroll_state(self.scroll.clone())
    .width(fill)
    .height(fill)
  }
}

pub struct Scenario {
  lines: Vec<String>,
  step: usize,
  window: WindowSlot,
  selectable: bool,
}

impl Scenario {
  pub fn new() -> Self {
    let lines = (0..24)
      .flat_map(|copy| {
        README
          .lines()
          .enumerate()
          .map(move |(line, text)| format!("{copy:02}/{line:03}: {text}"))
      })
      .collect();
    Self {
      lines,
      step: 0,
      window: WindowSlot(Arc::new(Mutex::new(None))),
      selectable: false,
    }
  }

  pub fn with_selectable(mut self, selectable: bool) -> Self {
    self.selectable = selectable;
    self
  }

  pub fn mount(&self, tree: &mut Tree, app: &mut App) {
    tree.mount_root::<DocumentRoot>(app, (self.source(), self.window.clone(), self.selectable));
  }

  pub fn window(&self) -> lurq::app::window::WindowHandle {
    self
      .window
      .0
      .lock()
      .unwrap()
      .clone()
      .expect("mount scenario before accessing its window")
  }

  fn source(&self) -> String {
    let mut source = self.lines.join("\n");
    source.push('\n');
    source
  }

  /// Apply one content update, wheel event, or logical viewport resize.
  pub fn advance(&mut self, tree: &mut Tree, native_resize: bool) -> Option<&'static str> {
    let step = self.step;
    self.step += 1;
    match step {
      0..8 => {
        self.lines[2].push(char::from(b'a' + step as u8));
        tree.update_root_props::<DocumentRoot>((self.source(), self.window.clone(), self.selectable));
        Some("edit")
      }
      8..16 => {
        let width = 860 - (step as u32 - 7) * 23;
        if native_resize {
          self.window().resize(width, 800);
        } else {
          tree.resize(width, 800);
        }
        Some("resize")
      }
      16..24 => {
        tree.scroll(120.0, 120.0, 0.0, -160.0, ScrollPhase::Scroll);
        Some("scroll")
      }
      _ => None,
    }
  }
}
