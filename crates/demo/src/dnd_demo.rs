use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, PRIMARY, SUCCESS, SURFACE, TEXT, TEXT_MUTED, WARNING, text};

const CONTENT_PAD: f32 = 32.0;
const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const SECTION_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;

pub(crate) struct DndDemo {
  drop_status: Signal<String>,
}

impl Component for DndDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      drop_status: ctx.signal("Drop target".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Drag & Drop", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(section_title("Container Drop"))
      .child(dnd_showcase(ctx, self.drop_status.clone()))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .fill(BG)
  }
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn dnd_showcase(ctx: &mut Ctx, drop_status: Signal<String>) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(dnd_stack(ctx, drop_status))
    .width(FILL_WIDTH)
    .height(420.0)
    .padding_horizontal(32.0)
    .padding_vertical(36.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(SECTION_RADIUS)
    .into()
}

fn dnd_stack(ctx: &mut Ctx, drop_status: Signal<String>) -> Element {
  let status = drop_status.get();

  let top_drop_zone = lurq::components::DropZone::mount(
    ctx,
    lurq::components::DropZoneProps::new().on_drop({
      let drop_status = drop_status.clone();
      move |_| drop_status.set("Dropped top".to_owned())
    }),
    drop_zone_card(status.as_str()).absolute_position(220.0, 60.0),
  );

  let bottom_drop_zone = lurq::components::DropZone::mount(
    ctx,
    lurq::components::DropZoneProps::new().on_drop({
      let drop_status = drop_status.clone();
      move |_| drop_status.set("Dropped bottom".to_owned())
    }),
    drop_zone_card(status.as_str()).absolute_position(220.0, 160.0),
  );

  let keep_draggable = lurq::components::Draggable::mount(
    ctx,
    lurq::components::DraggableProps::new().on_drag_start({
      let drop_status = drop_status.clone();
      move |_| drop_status.set("Dragging keep".to_owned())
    }),
    drag_card("keep").absolute_position(20.0, 40.0),
  );

  let revert_draggable = lurq::components::Draggable::mount(
    ctx,
    lurq::components::DraggableProps::new()
      .drop_miss_behavior(lurq::components::DropMissBehavior::RevertToDragStart)
      .on_drag_start({
        let drop_status = drop_status.clone();
        move |_| drop_status.set("Dragging revert".to_owned())
      }),
    drag_card_with_color("revert", WARNING).absolute_position(20.0, 140.0),
  );

  lurq::components::DragContainer::mount(
    ctx,
    lurq::components::DragContainerProps::new(),
    lurq::components::Stack::new()
      .size(400.0, 280.0)
      .fill(BG)
      .border_inside(1.0, Color::from_hex(BORDER))
      .rounded(SECTION_RADIUS)
      .with_children([
        text("DnD (400x280)", 11.0, FontWeight::Normal, TEXT_MUTED)
          .absolute_position(8.0, 8.0)
          .into(),
        top_drop_zone,
        bottom_drop_zone,
        keep_draggable,
        revert_draggable,
      ]),
  )
}

fn drop_zone_card(label: &str) -> lurq::components::Row {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 11.0, FontWeight::Bold, TEXT))
    .size(140.0, 82.0)
    .fill("#22c55e33")
    .border_inside(1.0, Color::from_hex(SUCCESS))
    .rounded(PANEL_RADIUS)
}

fn drag_card(label: &str) -> lurq::components::Row {
  drag_card_with_color(label, PRIMARY)
}

fn drag_card_with_color(label: &str, color: &str) -> lurq::components::Row {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 11.0, FontWeight::Normal, TEXT))
    .size(120.0, 80.0)
    .fill(color)
    .rounded(PANEL_RADIUS)
}
