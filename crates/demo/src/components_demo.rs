use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{MouseEvent, MouseEventKind},
  },
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::style::{ACCENT, BG, BORDER, ERROR, PRIMARY, SURFACE, TEXT, TEXT_MUTED, WARNING, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;

#[derive(Default, Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Test {
  test1: i32,
  test2: u32,
  test3: bool,
}

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct InfoCardProps {
  title: &'static str,
  body: &'static str,
  accent: &'static str,
  test: Test,
}

struct InfoCard;

impl Component for InfoCard {
  type Props = InfoCardProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<InfoCardProps>();
    lurq::components::Column::new()
      .spacing(8.0)
      .child(text(props.title, 16.0, FontWeight::Bold, props.accent))
      .child(lurq::components::Rect::new(Dimension::Pct(100.0), 1.0).background(BORDER))
      .child(text(props.body, 13.0, FontWeight::Normal, TEXT))
      .padding_horizontal(16.0)
      .padding_vertical(12.0)
      .flex(1.0)
      .background(BG)
      .border_inside(2.0, Color::from_hex(props.accent))
      .rounded(CARD_RADIUS)
  }
}

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct ListItem {
  key: String,
  label: String,
}

struct KeyedListItem {
  mount_count: Signal<u32>,
  name: String,
}

impl Component for KeyedListItem {
  type Props = ListItem;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      mount_count: ctx.signal(1),
      name: ctx.props::<Self::Props>().key.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let item = ctx.props::<ListItem>();
    lurq::components::Row::new()
      .spacing(8.0)
      .align_items(Alignment::Center)
      .child(text(&format!("key=\"{}\"", item.key), 11.0, FontWeight::Normal, TEXT_MUTED).width(42.0))
      .child(text(
        &format!("{}  (mount count: {})", item.label, self.mount_count.get()),
        13.0,
        FontWeight::Normal,
        TEXT,
      ))
      .height(40.0)
      .padding_horizontal(12.0)
      .padding_vertical(0.0)
      .width(FILL_WIDTH)
      .background(BG)
      .border_inside(1.0, Color::from_hex(BORDER))
      .rounded(PANEL_RADIUS)
  }

  fn on_mounted(&self) {
    println!(
      ":mounting item with key={}, count={}",
      self.name,
      self.mount_count.get()
    );
  }
}

pub(crate) struct ComponentsDemo {
  items: Signal<Vec<ListItem>>,
  next_item_index: Signal<u8>,
}

impl Component for ComponentsDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      items: ctx.signal(vec![item("a"), item("b"), item("c")]),
      next_item_index: ctx.signal(b'd'),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Components", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(section_title("Component with Props"))
      .child(props_demo(ctx))
      .child(section_title("Keyed List (for_each)"))
      .child(keyed_list_demo(ctx, self.items.clone(), self.next_item_index.clone()))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .background(BG)
  }
}

fn props_demo(ctx: &mut Ctx) -> Element {
  lurq::components::Row::new()
    .spacing(20.0)
    .child(ctx.mount::<InfoCard>(InfoCardProps {
      title: "Info",
      body: "Some info content",
      accent: PRIMARY,
      test: Test::default(),
    }))
    .child(ctx.mount::<InfoCard>(InfoCardProps {
      title: "Warning",
      body: "Be careful!",
      accent: WARNING,
      test: Test::default(),
    }))
    .child(ctx.mount::<InfoCard>(InfoCardProps {
      title: "Error",
      body: "Something went wrong",
      accent: ERROR,
      test: Test::default(),
    }))
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn keyed_list_demo(ctx: &mut Ctx, items: Signal<Vec<ListItem>>, next_item_index: Signal<u8>) -> Element {
  let current_items = items.get();
  let item_rows = ctx.for_each(
    current_items,
    |item| item.key.clone(),
    |ctx, item| ctx.mount::<KeyedListItem>(item),
  );

  lurq::components::Column::new()
    .spacing(8.0)
    .child(action_row(items.clone(), next_item_index))
    .with_children(item_rows)
    .child(text(
      "Shuffling preserves component state!",
      12.0,
      FontWeight::Normal,
      ACCENT,
    ))
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn action_row(items: Signal<Vec<ListItem>>, next_item_index: Signal<u8>) -> Element {
  lurq::components::Row::new()
    .spacing(12.0)
    .child(action_button("Add Item", {
      let items = items.clone();
      move |_| {
        let next = next_item_index.get();
        let key = (next as char).to_string();
        items.update(|items| {
          items.push(ListItem {
            key: key.clone(),
            label: format!("Item {}", key.to_ascii_uppercase()),
          })
        });
        next_item_index.set(next.saturating_add(1));
      }
    }))
    .child(action_button("Shuffle", {
      let items = items.clone();
      move |_| {
        items.update(|items| {
          if items.len() > 1 {
            items.rotate_left(1);
          }
        });
      }
    }))
    .child(action_button("Remove Last", move |_| {
      items.update(|items| {
        items.pop();
      });
    }))
    .width(FILL_WIDTH)
    .into()
}

fn action_button(label: &str, on_click: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 11.0, FontWeight::Bold, TEXT))
    .size(120.0, 32.0)
    .background(PRIMARY)
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Pointer)
    .hovered(|style| style.background("#60a5fa"))
    .active(|style| style.background("#2563eb"))
    .on_click(move |event| {
      if matches!(event.kind, MouseEventKind::Click) {
        on_click(event);
      }
    })
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn item(key: &str) -> ListItem {
  ListItem {
    key: key.to_owned(),
    label: format!("Item {}", key.to_ascii_uppercase()),
  }
}
