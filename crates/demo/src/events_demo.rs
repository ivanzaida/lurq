use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{MouseButton, MouseEvent},
  },
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, PRIMARY, SECONDARY, SUCCESS, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;

#[derive(Clone, Copy, lurq::DevtoolsInspectable)]
struct PointerState {
  x: f32,
  y: f32,
  entered: bool,
}

pub(crate) struct EventsDemo {
  log: Signal<Vec<String>>,
  pointer: Signal<PointerState>,
}

impl Component for EventsDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      log: ctx.signal(vec![
        "> click at (142, 87)".to_owned(),
        "> dblclick at (142, 87)".to_owned(),
        "> mouse_down Left".to_owned(),
        "> mouse_up Left".to_owned(),
      ]),
      pointer: ctx.signal(PointerState {
        x: 234.0,
        y: 156.0,
        entered: true,
      }),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Events & Interaction", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(section_title("Click Events"))
      .child(click_demo(self.log.clone()))
      .child(section_title("Hover & Interaction States"))
      .child(hover_demo())
      .child(section_title("Mouse Tracking"))
      .child(mouse_demo(self.pointer.clone()))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .background(BG)
  }
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn click_demo(log: Signal<Vec<String>>) -> Element {
  lurq::components::Row::new()
    .spacing(32.0)
    .child(
      lurq::components::Column::new()
        .spacing(12.0)
        .child(event_button("Click Me", PRIMARY, click_handlers(log.clone())))
        .child(event_button(
          "Double-Click Me",
          SECONDARY,
          double_click_handlers(log.clone()),
        )),
    )
    .child(event_log(log).flex(1.0))
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn event_button(label: &str, fill: &str, handlers: ButtonHandlers) -> Element {
  let mut button = lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 14.0, FontWeight::Bold, "#ffffff"))
    .size(180.0, 48.0)
    .background(fill)
    .rounded(CARD_RADIUS)
    .cursor(CursorIcon::Pointer)
    .hovered(|style| style.background("#60a5fa"))
    .active(|style| style.background("#2563eb"));

  if let Some(on_click) = handlers.on_click {
    button = button.on_click(on_click);
  }
  if let Some(on_dblclick) = handlers.on_dblclick {
    button = button.on_dblclick(on_dblclick);
  }
  if let Some(on_mouse_down) = handlers.on_mouse_down {
    button = button.on_mouse_down(on_mouse_down);
  }
  if let Some(on_mouse_up) = handlers.on_mouse_up {
    button = button.on_mouse_up(on_mouse_up);
  }

  button.into()
}

type MouseHandler = Box<dyn Fn(MouseEvent) + Send + Sync>;

#[derive(Default)]
struct ButtonHandlers {
  on_click: Option<MouseHandler>,
  on_dblclick: Option<MouseHandler>,
  on_mouse_down: Option<MouseHandler>,
  on_mouse_up: Option<MouseHandler>,
}

fn click_handlers(log: Signal<Vec<String>>) -> ButtonHandlers {
  ButtonHandlers {
    on_click: Some(Box::new({
      let log = log.clone();
      move |event| push_log(&log, format!("> click at ({:.0}, {:.0})", event.x, event.y))
    })),
    on_mouse_down: Some(Box::new({
      let log = log.clone();
      move |event| push_log(&log, format!("> mouse_down {}", button_label(event.button)))
    })),
    on_mouse_up: Some(Box::new(move |event| {
      push_log(&log, format!("> mouse_up {}", button_label(event.button)))
    })),
    ..ButtonHandlers::default()
  }
}

fn double_click_handlers(log: Signal<Vec<String>>) -> ButtonHandlers {
  ButtonHandlers {
    on_click: Some(Box::new({
      let log = log.clone();
      move |event| push_log(&log, format!("> click at ({:.0}, {:.0})", event.x, event.y))
    })),
    on_dblclick: Some(Box::new(move |event| {
      push_log(&log, format!("> dblclick at ({:.0}, {:.0})", event.x, event.y))
    })),
    ..ButtonHandlers::default()
  }
}

fn event_log(log: Signal<Vec<String>>) -> lurq::components::Column {
  let entries = log.get();
  lurq::components::Column::new()
    .spacing(4.0)
    .child(text("Event Log:", 12.0, FontWeight::Bold, TEXT_MUTED))
    .with_children(
      entries
        .iter()
        .take(4)
        .map(|entry| text(entry, 12.0, FontWeight::Normal, SUCCESS).width(FILL_WIDTH)),
    )
    .height(140.0)
    .padding(12.0)
    .width(FILL_WIDTH)
    .background(BG)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(PANEL_RADIUS)
}

fn hover_demo() -> Element {
  lurq::components::Row::new()
    .spacing(32.0)
    .align_items(Alignment::Center)
    .child(state_sample("Normal", PRIMARY))
    .child(state_sample("Hovered", "#60a5fa"))
    .child(state_sample("Active", "#2563eb"))
    .padding_horizontal(24.0)
    .padding_vertical(20.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn state_sample(label: &str, fill: &str) -> Element {
  lurq::components::Column::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(text(label, 12.0, FontWeight::Normal, TEXT_MUTED))
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(text("Button", 14.0, FontWeight::Bold, "#ffffff"))
        .size(140.0, 44.0)
        .background(fill)
        .rounded(CARD_RADIUS)
        .cursor(CursorIcon::Pointer)
        .hovered(|style| style.background("#60a5fa"))
        .active(|style| style.background("#2563eb")),
    )
    .into()
}

fn mouse_demo(pointer: Signal<PointerState>) -> Element {
  let state = pointer.get();
  lurq::components::Column::new()
    .spacing(8.0)
    .child(track_area(pointer, state))
    .child(text(
      "Tracks mouse position via on_mouse_move",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .padding(20.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn track_area(pointer: Signal<PointerState>, state: PointerState) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(
      text(
        &format!("x: {:.0}  y: {:.0}  |  entered: {}", state.x, state.y, state.entered),
        14.0,
        FontWeight::Normal,
        TEXT,
      )
      .nowrap(),
    )
    .height(70.0)
    .width(FILL_WIDTH)
    .background(BG)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Crosshair)
    .on_mouse_move({
      let pointer = pointer.clone();
      move |event: MouseEvent| {
        pointer.set(PointerState {
          x: event.x,
          y: event.y,
          entered: true,
        });
      }
    })
    .on_mouse_enter({
      let pointer = pointer.clone();
      move || {
        pointer.update(|state| {
          state.entered = true;
        });
      }
    })
    .on_mouse_leave(move || {
      pointer.update(|state| {
        state.entered = false;
      });
    })
    .into()
}

fn push_log(log: &Signal<Vec<String>>, line: String) {
  log.update(|entries| {
    entries.insert(0, line);
    entries.truncate(4);
  });
}

fn button_label(button: MouseButton) -> &'static str {
  match button {
    MouseButton::Left => "Left",
    MouseButton::Right => "Right",
    MouseButton::Middle => "Middle",
    MouseButton::Other(_) => "Other",
  }
}
