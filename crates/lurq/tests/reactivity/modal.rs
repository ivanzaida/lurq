use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Column, Modal, Rect, Root as ModalRoot, Row, Select, Slider, Text},
  core::Signal,
  layout::{Alignment, Constraints, Size, StackAlignment, layout_result::LayoutResult, quad::QuadContent},
  node::{Element, color::Color},
};

use crate::support::{pointer_click, render_pass, run_pass};

fn pass_layout(tree: &mut Tree, constraints: Constraints) -> LayoutResult {
  tree.set_layout_constraints_override(Some(constraints));
  run_pass(tree);
  let result = tree.last_layout().cloned();
  tree.set_layout_constraints_override(None);
  result.unwrap()
}

fn rendered_text_quads(tree: &Tree) -> Vec<String> {
  let layout = tree.last_layout().expect("tree should have a layout");
  tree
    .resolve_quads(layout)
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Text { text, .. } => Some(text),
      _ => None,
    })
    .collect()
}

struct Shared<T>(Arc<T>);

#[cfg(feature = "devtools")]
impl<T> lurq::app::component::DevtoolsInspectable for Shared<T> {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct Root;

impl Component for Root {
  type Props = Shared<Mutex<Option<Signal<bool>>>>;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(ctx.mount::<ModalChild>(ctx.props::<Self::Props>().clone()))
  }
}

struct ModalChild {
  open: Signal<bool>,
}

impl Component for ModalChild {
  type Props = Shared<Mutex<Option<Signal<bool>>>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(Text::new("child")).child(
      Modal::new(ctx.mount::<ModalPanel>(()))
        .open(self.open.clone())
        .target(ModalRoot),
    )
  }
}

struct ModalPanel;

impl Component for ModalPanel {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Text::new("modal")
  }
}

#[test]
fn modal_renders_declared_modal_above_root() {
  let open = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, Shared(open.clone()));

  open.lock().unwrap().as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_eq!(root.tag_name(), "OverlayHost");
  assert_eq!(root.children().len(), 2);
  assert!(tree.find_element(|el| el.text_content() == Some("modal")).is_some());
}

#[test]
fn modal_removes_modal_when_declaring_component_stops_rendering_it() {
  let open = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, Shared(open.clone()));

  let signal = open.lock().unwrap().as_ref().unwrap().clone();
  signal.set(true);
  run_pass(&mut tree);
  assert_eq!(tree.root().unwrap().tag_name(), "OverlayHost");

  signal.set(false);
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_ne!(root.tag_name(), "OverlayHost");
  assert!(tree.find_element(|el| el.text_content() == Some("modal")).is_none());
}

#[derive(Default)]
struct ModalOrderSignals {
  stream_open: Option<Signal<bool>>,
  settings_open: Option<Signal<bool>>,
}

struct RootWithOrderedModals {
  stream_open: Signal<bool>,
  settings_open: Signal<bool>,
}

impl Component for RootWithOrderedModals {
  type Props = Shared<Mutex<ModalOrderSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let stream_open = ctx.signal(false);
    let settings_open = ctx.signal(false);
    let mut props = ctx.props::<Self::Props>().0.lock().unwrap();
    props.stream_open = Some(stream_open.clone());
    props.settings_open = Some(settings_open.clone());
    Self {
      stream_open,
      settings_open,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .child(Text::new("root"))
      .child(
        Modal::new(Text::new("stream"))
          .open(self.stream_open.clone())
          .target(ModalRoot),
      )
      .child(
        Modal::new(Text::new("settings"))
          .open(self.settings_open.clone())
          .target(ModalRoot),
      )
  }
}

#[test]
fn later_declared_modal_renders_above_existing_modal() {
  let signals = Arc::new(Mutex::new(ModalOrderSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithOrderedModals>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().stream_open.as_ref().unwrap().set(true);
  run_pass(&mut tree);
  signals.lock().unwrap().settings_open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let texts = rendered_text_quads(&tree);
  assert_eq!(texts, vec!["root", "stream", "settings"]);
}

#[derive(Default)]
struct ModalEscapeSignals {
  bottom_open: Option<Signal<bool>>,
  top_open: Option<Signal<bool>>,
  events: Vec<&'static str>,
}

struct RootWithEscapeModals {
  bottom_open: Signal<bool>,
  top_open: Signal<bool>,
}

impl Component for RootWithEscapeModals {
  type Props = Shared<Mutex<ModalEscapeSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let bottom_open = ctx.signal(false);
    let top_open = ctx.signal(false);
    let mut props = ctx.props::<Self::Props>().0.lock().unwrap();
    props.bottom_open = Some(bottom_open.clone());
    props.top_open = Some(top_open.clone());
    Self { bottom_open, top_open }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .child(Text::new("root"))
      .child(
        Modal::new(Rect::new(10.0, 10.0))
          .open(self.bottom_open.clone())
          .target(ModalRoot),
      )
      .child(
        Modal::new(Rect::new(10.0, 10.0))
          .open(self.top_open.clone())
          .target(ModalRoot),
      )
  }
}

#[test]
fn escape_key_closes_only_top_modal() {
  let signals = Arc::new(Mutex::new(ModalEscapeSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithEscapeModals>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().bottom_open.as_ref().unwrap().set(true);
  run_pass(&mut tree);
  signals.lock().unwrap().top_open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  tree.key_down("Escape".to_owned(), "Escape".to_owned(), false, false, false);

  run_pass(&mut tree);

  let signals = signals.lock().unwrap();
  assert!(signals.events.is_empty());
  assert!(signals.bottom_open.as_ref().unwrap().get());
  assert!(!signals.top_open.as_ref().unwrap().get());
}

#[derive(Default)]
struct ModalSignals {
  open: Option<Signal<bool>>,
  enabled: Option<Signal<bool>>,
}

struct RootWithStateModal {
  open: Signal<bool>,
}

impl Component for RootWithStateModal {
  type Props = Shared<Mutex<ModalSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().open = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    Column::new().child(Text::new("root")).child(
      Modal::new(ctx.mount::<StateModalPanel>(props))
        .open(self.open.clone())
        .target(ModalRoot),
    )
  }
}

struct StateModalPanel {
  enabled: Signal<bool>,
}

impl Component for StateModalPanel {
  type Props = Shared<Mutex<ModalSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let enabled = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().enabled = Some(enabled.clone());
    Self { enabled }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Text::new(if self.enabled.get() { "modal-on" } else { "modal-off" })
  }
}

#[test]
fn modal_partial_update_preserves_live_node_ids() {
  let signals = Arc::new(Mutex::new(ModalSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithStateModal>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  assert!(rendered_text_quads(&tree).iter().any(|text| text == "modal-off"));

  signals.lock().unwrap().enabled.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  assert!(rendered_text_quads(&tree).iter().any(|text| text == "modal-on"));
}

struct RootWithLayoutModal {
  open: Signal<bool>,
}

impl Component for RootWithLayoutModal {
  type Props = Shared<Mutex<ModalSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().open = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    Column::new().child(Text::new("root")).child(
      Modal::new(ctx.mount::<LayoutModalPanel>(props))
        .open(self.open.clone())
        .target(ModalRoot),
    )
  }
}

struct LayoutModalPanel {
  enabled: Signal<bool>,
}

impl Component for LayoutModalPanel {
  type Props = Shared<Mutex<ModalSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let enabled = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().enabled = Some(enabled.clone());
    Self { enabled }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let knob_offset = if self.enabled.get() { 20.0 } else { 2.0 };
    Row::new()
      .width(40.0)
      .height(22.0)
      .align_items(Alignment::Center)
      .child(Rect::new(knob_offset, 1.0))
      .child(Rect::new(18.0, 18.0))
  }
}

#[test]
fn modal_partial_layout_change_relayouts_modal_ancestors() {
  let signals = Arc::new(Mutex::new(ModalSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithLayoutModal>(&mut app, Shared(signals.clone()));
  signals.lock().unwrap().open.as_ref().unwrap().set(true);

  let result = pass_layout(&mut tree, Constraints::loose(Size::new(400.0, 600.0)));
  let knob_x = result.children[1].result.children[0].result.children[1].offset.x;
  assert_eq!(knob_x, 2.0);

  signals.lock().unwrap().enabled.as_ref().unwrap().set(true);

  let result = pass_layout(&mut tree, Constraints::loose(Size::new(400.0, 600.0)));
  let knob_x = result.children[1].result.children[0].result.children[1].offset.x;
  assert_eq!(knob_x, 20.0);
}

#[derive(Default)]
struct SelectModalSignals {
  open: Option<Signal<bool>>,
  value: Option<Signal<String>>,
}

struct RootWithSelectModal {
  open: Signal<bool>,
  value: Signal<String>,
}

impl Component for RootWithSelectModal {
  type Props = Shared<Mutex<SelectModalSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    let value = ctx.signal("md".to_owned());
    let props = ctx.props::<Self::Props>().clone();
    {
      let mut signals = props.0.lock().unwrap();
      signals.open = Some(open.clone());
      signals.value = Some(value.clone());
    }
    Self { open, value }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let value = self.value.clone();
    Column::new().child(Text::new("root")).child(
      Modal::new(
        Select::new(value)
          .options(
            [("sm", "Small"), ("md", "Medium"), ("lg", "Large")]
              .into_iter()
              .map(|(value, label)| (value.to_owned(), label)),
          )
          .width(200.0)
          .height(40.0),
      )
      .open(self.open.clone())
      .target(ModalRoot),
    )
  }
}

#[test]
fn select_inside_modal_opens_and_commits() {
  let signals = Arc::new(Mutex::new(SelectModalSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithSelectModal>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let select = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("modal select should render");
  let (x, y) = select.bounds().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);

  let large = tree
    .find_element(|el| el.text_content() == Some("Large"))
    .expect("select menu option should render");
  let (x, y) = large.bounds().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);

  let selected = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert_eq!(selected, "lg");
}

#[derive(Default)]
struct ForEachModalSignals {
  open: Option<Signal<bool>>,
  value: Option<Signal<i32>>,
}

struct RootWithForEachSliderModal {
  open: Signal<bool>,
  value: Signal<i32>,
}

impl Component for RootWithForEachSliderModal {
  type Props = Shared<Mutex<ForEachModalSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    let value = ctx.signal(50);
    let props = ctx.props::<Self::Props>().clone();
    {
      let mut signals = props.0.lock().unwrap();
      signals.open = Some(open.clone());
      signals.value = Some(value.clone());
    }
    Self { open, value }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let open = self.open.clone();
    let value = self.value.clone();
    let rows = ctx.for_each(
      [1_u64],
      |id| *id,
      move |_ctx, id| {
        let open = open.clone();
        let value = value.clone();
        let current = value.get();
        Column::new()
          .child(Text::new(&format!("row-{id}")))
          .child(
            Modal::new(
              Column::new()
                .width(200.0)
                .height(80.0)
                .child(Text::new(&format!("{current}%")))
                .child(Slider::new(value).range(0, 100).width(120.0).height(20.0)),
            )
            .open(open)
            .target(ModalRoot),
          )
          .into()
      },
    );
    Column::new().with_children(rows)
  }
}

#[test]
fn for_each_owned_modal_slider_updates_without_growing_modal_hosts() {
  let signals = Arc::new(Mutex::new(ForEachModalSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithForEachSliderModal>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  for value in 0..80 {
    signals.lock().unwrap().value.as_ref().unwrap().set(value);
    run_pass(&mut tree);

    let root = tree.root().unwrap();
    assert_eq!(root.tag_name(), "OverlayHost");
    assert_eq!(root.children().len(), 2);
  }
}

#[derive(Default)]
struct LocalModalSliderSignals {
  open: Option<Signal<bool>>,
  value: Option<Signal<i32>>,
  initial_value: i32,
}

struct RootWithLocalSliderModal {
  open: Signal<bool>,
}

impl Component for RootWithLocalSliderModal {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().open = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    Column::new().child(Text::new("base")).child(
      Modal::new(ctx.mount::<LocalModalSlider>(props))
        .open(self.open.clone())
        .target(ModalRoot),
    )
  }
}

struct LocalModalSlider {
  value: Signal<i32>,
}

impl Component for LocalModalSlider {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let initial_value = ctx.props::<Self::Props>().0.lock().unwrap().initial_value;
    let value = ctx.signal(initial_value);
    ctx.props::<Self::Props>().0.lock().unwrap().value = Some(value.clone());
    Self { value }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let value = self.value.clone();
    Column::new()
      .width(180.0)
      .height(80.0)
      .child(Text::new(&format!("{}%", value.get())))
      .child(Slider::new(value).range(0, 100).width(100.0).height(20.0))
  }
}

#[test]
fn local_slider_inside_modal_rerenders_while_dragging() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithLocalSliderModal>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("modal slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;

  tree.mouse_down(rect.x, y, MouseButton::Left);
  tree.mouse_move(rect.x + 75.0, y);

  let dragged_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(dragged_value > 0);
  render_pass(&mut tree);
  assert!(
    tree
      .find_element(|node| node.text_content() == Some(&format!("{dragged_value}%")))
      .is_some(),
    "modal subtree should show the dragged slider value before mouse-up"
  );

  tree.mouse_up(rect.x + 75.0, y, MouseButton::Left);
}

struct LocalModalNestedSlider;

impl Component for LocalModalNestedSlider {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<LocalModalSlider>(ctx.props::<Self::Props>().clone())
  }
}

struct RootWithNestedLocalSliderModal {
  open: Signal<bool>,
}

impl Component for RootWithNestedLocalSliderModal {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().open = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    Column::new().child(Text::new("base")).child(
      Modal::new(ctx.mount::<LocalModalNestedSlider>(props))
        .open(self.open.clone())
        .target(ModalRoot),
    )
  }
}

#[test]
fn nested_local_slider_inside_modal_rerenders_while_dragging() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithNestedLocalSliderModal>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("modal slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;

  tree.mouse_down(rect.x, y, MouseButton::Left);
  tree.mouse_move(rect.x + 75.0, y);

  let dragged_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(dragged_value > 0);
  render_pass(&mut tree);
  assert!(
    tree
      .find_element(|node| node.text_content() == Some(&format!("{dragged_value}%")))
      .is_some(),
    "nested modal subtree should show the dragged slider value before mouse-up"
  );

  tree.mouse_up(rect.x + 75.0, y, MouseButton::Left);
}

struct LocalModalPercentSlider {
  value: Signal<i32>,
}

impl Component for LocalModalPercentSlider {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let initial_value = ctx.props::<Self::Props>().0.lock().unwrap().initial_value;
    let value = ctx.signal(initial_value);
    ctx.props::<Self::Props>().0.lock().unwrap().value = Some(value.clone());
    Self { value }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let value = self.value.clone();
    let current = value.get().clamp(0, 100);

    Column::new()
      .width(180.0)
      .height(80.0)
      .child(Text::new(&format!("{current}%")))
      .child(
        lurq::components::Stack::new()
          .stack_align(StackAlignment::CenterStart)
          .width(100.0)
          .height(20.0)
          .child(Rect::new(100.0, 4.0).background("#111827"))
          .child(Rect::new(current as f32, 4.0).background("#38bdf8"))
          .child(
            Slider::new(value)
              .range(0, 100)
              .width(100.0)
              .height(20.0)
              .track(|style| style.size(100.0, 4.0).background("#00000000"))
              .thumb(|style| style.size(10.0, 10.0).background("#f97316")),
          ),
      )
  }
}

struct LocalModalNestedPercentSlider;

impl Component for LocalModalNestedPercentSlider {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<LocalModalPercentSlider>(ctx.props::<Self::Props>().clone())
  }
}

struct RootWithNestedLocalPercentSliderModal {
  open: Signal<bool>,
}

impl Component for RootWithNestedLocalPercentSliderModal {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().open = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    Column::new().child(Text::new("base")).child(
      Modal::new(ctx.mount::<LocalModalNestedPercentSlider>(props))
        .open(self.open.clone())
        .target(ModalRoot),
    )
  }
}

#[test]
fn nested_percent_slider_inside_modal_rerenders_fill_while_dragging() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithNestedLocalPercentSliderModal>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("modal slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;

  tree.mouse_down(rect.x, y, MouseButton::Left);
  tree.mouse_move(rect.x + 75.0, y);

  let dragged_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(dragged_value > 0);
  let snapshot = render_pass(&mut tree);
  let fill = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#38bdf8"))
    .expect("custom fill should render");

  assert_eq!(fill.width.round() as i32, dragged_value);
  let thumb = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#f97316"))
    .expect("slider thumb should render");
  assert!(
    ((thumb.x + thumb.width / 2.0) - (rect.x + 75.0)).abs() <= 1.0,
    "thumb should track pointer while dragging; thumb_center={}, pointer={}",
    thumb.x + thumb.width / 2.0,
    rect.x + 75.0
  );
  assert!(
    tree
      .find_element(|node| node.text_content() == Some(&format!("{dragged_value}%")))
      .is_some(),
    "nested modal percent slider should show the dragged value before mouse-up"
  );

  tree.mouse_up(rect.x + 75.0, y, MouseButton::Left);
  let snapshot = render_pass(&mut tree);
  let fill = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#38bdf8"))
    .expect("custom fill should render after mouse-up");
  assert_eq!(fill.width.round() as i32, dragged_value);
}

#[test]
fn nested_percent_slider_inside_modal_rerenders_fill_when_dragging_left() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals {
    initial_value: 74,
    ..LocalModalSliderSignals::default()
  }));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithNestedLocalPercentSliderModal>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("modal slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;
  let start_x = rect.x + rect.width * 0.74;
  let drag_x = rect.x + rect.width * 0.35;

  tree.mouse_down(start_x, y, MouseButton::Left);
  tree.mouse_move(drag_x, y);

  let dragged_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(dragged_value < 74, "drag should lower the signal value");
  let snapshot = render_pass(&mut tree);
  let fill = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#38bdf8"))
    .expect("custom fill should render");

  assert_eq!(
    fill.width.round() as i32,
    dragged_value,
    "custom fill should rerender from the old initial value during left drag"
  );
  assert!(
    tree
      .find_element(|node| node.text_content() == Some(&format!("{dragged_value}%")))
      .is_some(),
    "nested modal percent slider should show the lowered value during left drag"
  );

  tree.mouse_up(drag_x, y, MouseButton::Left);
}

#[test]
fn context_menu_percent_slider_thumb_matches_lowered_value_after_drag() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals {
    initial_value: 74,
    ..LocalModalSliderSignals::default()
  }));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithContextMenuPercentSlider>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("context menu slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;
  let start_x = rect.x + rect.width * 0.74;
  let drag_x = rect.x + rect.width * 0.35;

  tree.mouse_down(start_x, y, MouseButton::Left);
  tree.mouse_move(drag_x, y);
  tree.mouse_up(drag_x, y, MouseButton::Left);

  let dragged_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(dragged_value < 74, "drag should lower the signal value");
  let snapshot = render_pass(&mut tree);
  let thumb = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#f97316"))
    .expect("slider thumb should render");
  let thumb_center = thumb.x + thumb.width / 2.0;
  let expected_center = rect.x + thumb.width / 2.0 + (dragged_value as f32 / 100.0) * (rect.width - thumb.width);

  assert!(
    (thumb_center - expected_center).abs() <= 1.0,
    "thumb should match lowered signal value after drag; thumb_center={thumb_center}, expected={expected_center}, value={dragged_value}"
  );
}

#[test]
fn context_menu_percent_slider_thumb_matches_value_after_track_click() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals {
    initial_value: 74,
    ..LocalModalSliderSignals::default()
  }));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithContextMenuPercentSlider>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("context menu slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;
  let click_x = rect.x + rect.width * 0.36;

  tree.mouse_down(click_x, y, MouseButton::Left);
  tree.mouse_up(click_x, y, MouseButton::Left);

  let clicked_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(clicked_value < 74, "track click should lower the signal value");
  let snapshot = render_pass(&mut tree);
  let fill = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#38bdf8"))
    .expect("custom fill should render");
  let thumb = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#f97316"))
    .expect("slider thumb should render");
  let thumb_center = thumb.x + thumb.width / 2.0;
  let expected_center = rect.x + thumb.width / 2.0 + (clicked_value as f32 / 100.0) * (rect.width - thumb.width);

  assert_eq!(fill.width.round() as i32, clicked_value);
  assert!(
    (thumb_center - expected_center).abs() <= 1.0,
    "thumb should match lowered signal value after track click; thumb_center={thumb_center}, expected={expected_center}, value={clicked_value}"
  );
}

struct RootWithContextMenuPercentSlider {
  open: Signal<bool>,
}

impl Component for RootWithContextMenuPercentSlider {
  type Props = Shared<Mutex<LocalModalSliderSignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().open = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let content = lurq::components::Stack::new()
      .width(500.0)
      .height(400.0)
      .child(Rect::new(500.0, 400.0).background("#00000000"))
      .child(
        Row::new()
          .absolute(40.0, 30.0, 180.0, 80.0)
          .child(ctx.mount::<LocalModalNestedPercentSlider>(props)),
      );

    Column::new()
      .child(Text::new("base"))
      .child(Modal::new(content).open(self.open.clone()).target(ModalRoot))
  }
}

#[test]
fn context_menu_percent_slider_rerenders_fill_when_dragging_left() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals {
    initial_value: 74,
    ..LocalModalSliderSignals::default()
  }));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithContextMenuPercentSlider>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("context menu slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;
  let start_x = rect.x + rect.width * 0.74;
  let drag_x = rect.x + rect.width * 0.35;

  tree.mouse_down(start_x, y, MouseButton::Left);
  tree.mouse_move(drag_x, y);

  let dragged_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(dragged_value < 74, "drag should lower the signal value");
  let snapshot = render_pass(&mut tree);
  let fill = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#38bdf8"))
    .expect("custom fill should render");

  assert_eq!(
    fill.width.round() as i32,
    dragged_value,
    "context menu fill should rerender from the old initial value during left drag"
  );
  assert!(
    tree
      .find_element(|node| node.text_content() == Some(&format!("{dragged_value}%")))
      .is_some(),
    "context menu percent slider should show the lowered value during left drag"
  );
  let rendered_texts = rendered_text_quads(&tree);
  assert!(
    rendered_texts.iter().any(|text| text == &format!("{dragged_value}%")),
    "context menu percent slider should render the lowered value during left drag; rendered_texts={rendered_texts:?}"
  );

  tree.mouse_up(drag_x, y, MouseButton::Left);
}

#[test]
fn context_menu_percent_slider_rerenders_fill_when_dragging_after_mouse_down_frame() {
  let signals = Arc::new(Mutex::new(LocalModalSliderSignals {
    initial_value: 91,
    ..LocalModalSliderSignals::default()
  }));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootWithContextMenuPercentSlider>(&mut app, Shared(signals.clone()));

  signals.lock().unwrap().open.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let slider = tree
    .find_element(|node| node.tag_name() == "Slider")
    .expect("context menu slider should be laid out");
  let rect = slider.bounds();
  let y = rect.y + rect.height / 2.0;
  let start_x = rect.x + rect.width * 0.91;
  let drag_x = rect.x + rect.width * 0.35;

  tree.mouse_down(start_x, y, MouseButton::Left);
  let _ = render_pass(&mut tree);
  tree.mouse_move(drag_x, y);

  let dragged_value = signals.lock().unwrap().value.as_ref().unwrap().get();
  assert!(dragged_value < 91, "drag should lower the signal value");
  let snapshot = render_pass(&mut tree);
  let fill = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#38bdf8"))
    .expect("custom fill should render");
  let thumb = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#f97316"))
    .expect("slider thumb should render");
  let rendered_texts = rendered_text_quads(&tree);

  assert_eq!(
    fill.width.round() as i32,
    dragged_value,
    "context menu fill should rerender after an intervening mouse-down frame"
  );
  assert!(
    rendered_texts.iter().any(|text| text == &format!("{dragged_value}%")),
    "context menu percent slider should render the dragged value after an intervening mouse-down frame; rendered_texts={rendered_texts:?}"
  );
  assert!(
    ((thumb.x + thumb.width / 2.0) - drag_x).abs() <= 1.0,
    "thumb should track pointer after intervening frame; thumb_center={}, pointer={drag_x}",
    thumb.x + thumb.width / 2.0,
  );

  tree.mouse_up(drag_x, y, MouseButton::Left);
}
