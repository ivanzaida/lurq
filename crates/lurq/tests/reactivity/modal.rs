use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Column, Rect, Row, Select, Text},
  core::Signal,
  layout::{Alignment, Constraints, Size, layout_result::LayoutResult},
  node::Element,
};

use crate::support::{pointer_click, run_pass};

fn pass_layout(tree: &mut Tree, constraints: Constraints) -> LayoutResult {
  tree.set_layout_constraints_override(Some(constraints));
  run_pass(tree);
  let result = tree.last_layout().cloned();
  tree.set_layout_constraints_override(None);
  result.unwrap()
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
    ctx.modal(self.open.clone(), |ctx| ctx.mount::<ModalPanel>(()));
    Text::new("child")
  }
}

struct ModalPanel;

impl Component for ModalPanel {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    assert!(ctx.modal_context().expect("modal context should be set").is_open());
    Text::new("modal")
  }
}

#[test]
fn ctx_modal_renders_declared_modal_above_root() {
  let open = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, Shared(open.clone()));

  open.lock().unwrap().as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_eq!(root.tag_name(), "__lurq_modal_host");
  assert_eq!(root.children().len(), 2);
  assert!(
    root
      .children()
      .iter()
      .any(|child| child.text_content() == Some("modal"))
  );
}

#[test]
fn ctx_modal_removes_modal_when_declaring_component_stops_rendering_it() {
  let open = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, Shared(open.clone()));

  let signal = open.lock().unwrap().as_ref().unwrap().clone();
  signal.set(true);
  run_pass(&mut tree);
  assert_eq!(tree.root().unwrap().tag_name(), "__lurq_modal_host");

  signal.set(false);
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_ne!(root.tag_name(), "__lurq_modal_host");
  assert!(
    root
      .children()
      .iter()
      .all(|child| child.text_content() != Some("modal"))
  );
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
    ctx.modal(self.open.clone(), move |ctx| ctx.mount::<StateModalPanel>(props));
    Text::new("root")
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

  let modal_id_before = tree
    .root()
    .unwrap()
    .children()
    .iter()
    .nth(1)
    .expect("modal should be rendered")
    .node_id();

  signals.lock().unwrap().enabled.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  let modal = root.children().iter().nth(1).expect("modal should still be rendered");
  assert_eq!(modal.node_id(), modal_id_before);
  assert_eq!(modal.text_content(), Some("modal-on"));
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
    ctx.modal(self.open.clone(), move |ctx| ctx.mount::<LayoutModalPanel>(props));
    Text::new("root")
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
  let knob_x = result.children[1].result.children[1].offset.x;
  assert_eq!(knob_x, 2.0);

  signals.lock().unwrap().enabled.as_ref().unwrap().set(true);

  let result = pass_layout(&mut tree, Constraints::loose(Size::new(400.0, 600.0)));
  let knob_x = result.children[1].result.children[1].offset.x;
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

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let value = self.value.clone();
    ctx.modal(self.open.clone(), move |_| {
      Select::new(value)
        .options(
          [("sm", "Small"), ("md", "Medium"), ("lg", "Large")]
            .into_iter()
            .map(|(value, label)| (value.to_owned(), label)),
        )
        .width(200.0)
        .height(40.0)
    });
    Text::new("root")
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
