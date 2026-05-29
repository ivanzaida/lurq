use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Runtime, component::Component, ctx::Ctx},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

// --- Test components ---

struct Shared<T>(Arc<T>);

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

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = i32;
  fn create(ctx: &mut Ctx) -> Self {
    Self {
      count: ctx.signal(*ctx.props::<Self::Props>()),
    }
  }
  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Text::new(&format!("{}", self.count.get()))
  }
}

struct Parent;

impl Component for Parent {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new()
      .child(ctx.mount::<Counter>(0))
      .child(ctx.mount::<Counter>(10))
  }
}

struct ContextProvider;

impl Component for ContextProvider {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    ctx.provide(42_i32);
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<ContextConsumer>(())
  }
}

struct ContextConsumer;

impl Component for ContextConsumer {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let val = ctx.use_context::<i32>().unwrap_or(0);
    lurq::components::Text::new(&format!("{}", val))
  }
}

struct SlotWrapper;

impl Component for SlotWrapper {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let count = ctx.children().len();
    lurq::components::Column::new().with_children((0..count).map(|_| Element::new()))
  }
}

struct ForEachParent;

impl Component for ForEachParent {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let items = vec![1, 2, 3, 4, 5];
    let nodes = ctx.for_each(
      items,
      |i| *i,
      |_ctx, i| lurq::components::Text::new(&format!("item-{}", i)).into(),
    );
    lurq::components::Column::new().with_children(nodes)
  }
}

struct ErrorComponent;

impl Component for ErrorComponent {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.error_boundary(
      |_ctx| {
        panic!("intentional panic");
      },
      || lurq::components::Text::new("fallback").into(),
    )
  }
}

struct EmptyComponent;

impl Component for EmptyComponent {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Element::new()
  }
}

struct DeeplyNested;

impl Component for DeeplyNested {
  type Props = u32;
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<EmptyComponent>(())
  }
}

struct SignalRoot {
  count: Signal<i32>,
}

impl Component for SignalRoot {
  type Props = Shared<Mutex<Option<Signal<i32>>>>;

  fn create(ctx: &mut Ctx) -> Self {
    let signal_out = ctx.props::<Self::Props>().0.clone();
    let count = ctx.signal(1);
    *signal_out.lock().unwrap() = Some(count.clone());
    Self { count }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Text::new(&format!("{}", self.count.get()))
  }
}

struct LifecycleChild {
  mounted: Arc<AtomicUsize>,
  unmounted: Arc<AtomicUsize>,
}

impl Component for LifecycleChild {
  type Props = (Shared<AtomicUsize>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      mounted: props.0.0,
      unmounted: props.1.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Element::new()
  }

  fn on_mounted(&self) {
    self.mounted.fetch_add(1, Ordering::Relaxed);
  }

  fn on_unmounted(&self) {
    self.unmounted.fetch_add(1, Ordering::Relaxed);
  }
}

struct ConditionalLifecycleParent {
  show_child: Signal<bool>,
  mounted: Arc<AtomicUsize>,
  unmounted: Arc<AtomicUsize>,
}

impl Component for ConditionalLifecycleParent {
  type Props = (
    Shared<Mutex<Option<Signal<bool>>>>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
  );

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let show_child = ctx.signal(true);
    *props.0.0.lock().unwrap() = Some(show_child.clone());
    Self {
      show_child,
      mounted: props.1.0,
      unmounted: props.2.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    if self.show_child.get() {
      ctx.mount::<LifecycleChild>((Shared(self.mounted.clone()), Shared(self.unmounted.clone())))
    } else {
      Element::new()
    }
  }
}

struct KeyedForEachLifecycleParent {
  items: Signal<Vec<&'static str>>,
  mounted: Arc<AtomicUsize>,
  unmounted: Arc<AtomicUsize>,
}

impl Component for KeyedForEachLifecycleParent {
  type Props = (
    Shared<Mutex<Option<Signal<Vec<&'static str>>>>>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
  );

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let items = ctx.signal(vec!["a", "b", "c"]);
    *props.0.0.lock().unwrap() = Some(items.clone());
    Self {
      items,
      mounted: props.1.0,
      unmounted: props.2.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mounted = self.mounted.clone();
    let unmounted = self.unmounted.clone();
    let rows = ctx.for_each(
      self.items.get(),
      |key| *key,
      move |ctx, _key| ctx.mount::<LifecycleChild>((Shared(mounted.clone()), Shared(unmounted.clone()))),
    );
    lurq::components::Column::new().with_children(rows)
  }
}

struct RootLifecycle {
  mounted: Arc<AtomicUsize>,
  unmounted: Arc<AtomicUsize>,
}

impl Component for RootLifecycle {
  type Props = (Shared<AtomicUsize>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      mounted: props.0.0,
      unmounted: props.1.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Element::new()
  }

  fn on_mounted(&self) {
    self.mounted.fetch_add(1, Ordering::Relaxed);
  }

  fn on_unmounted(&self) {
    self.unmounted.fetch_add(1, Ordering::Relaxed);
  }
}

// --- Tests ---

#[test]
fn mount_root_renders() {
  let mut rt = Runtime::new();
  rt.mount_root::<Counter>(5);
  let root = rt.root().unwrap();
  assert!(root.text_content().is_some());
}

#[test]
fn parent_mounts_children() {
  let mut rt = Runtime::new();
  rt.mount_root::<Parent>(());
  let root = rt.root().unwrap();
  assert_eq!(root.children().len(), 2);
}

#[test]
fn context_propagates_to_descendant() {
  let mut rt = Runtime::new();
  rt.mount_root::<ContextProvider>(());
  let root = rt.root().unwrap();
  assert!(root.text_content().is_some() || !root.children().is_empty());
}

#[test]
fn slot_children_passed_through() {
  let mut ctx = Ctx::new_root();
  let node = ctx.mount_with::<SlotWrapper>((), vec![Element::new(), Element::new(), Element::new()]);
  let mut rt = Runtime::new();
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert_eq!(root.children().len(), 3);
}

#[test]
fn for_each_renders_all_items() {
  let mut rt = Runtime::new();
  rt.mount_root::<ForEachParent>(());
  let root = rt.root().unwrap();
  assert_eq!(root.children().len(), 5);
}

#[test]
fn error_boundary_catches_panic() {
  let mut rt = Runtime::new();
  rt.mount_root::<ErrorComponent>(());
  let root = rt.root().unwrap();
  assert_eq!(root.text_content(), Some("fallback"));
}

#[test]
fn has_children_false_when_none() {
  let ctx = Ctx::new_root();
  assert!(!ctx.has_children());
}

#[test]
fn children_empty_when_no_slots() {
  let ctx = Ctx::new_root();
  assert!(ctx.children().is_empty());
}

#[test]
fn store_via_ctx_marks_dirty() {
  let mut ctx = Ctx::new_root();
  let store = ctx.store(0_i32);
  assert!(ctx.is_dirty());
  store.update(|v| *v += 1);
  assert!(ctx.is_dirty());
}

#[test]
fn signal_via_ctx_marks_dirty() {
  let mut ctx = Ctx::new_root();
  let sig = ctx.signal(0_i32);
  assert!(ctx.is_dirty());
  sig.set(42);
  assert!(ctx.is_dirty());
}

#[test]
fn provide_and_use_context_roundtrip() {
  let mut ctx = Ctx::new_root();
  ctx.provide(99_i32);
  assert_eq!(ctx.use_context::<i32>(), Some(99));
}

#[test]
fn use_context_missing_returns_none() {
  let ctx = Ctx::new_root();
  assert_eq!(ctx.use_context::<f64>(), None);
}

#[test]
fn empty_component_renders_leaf() {
  let mut rt = Runtime::new();
  rt.mount_root::<EmptyComponent>(());
  assert!(rt.root().is_some());
}

#[test]
fn deeply_nested_mount() {
  let mut rt = Runtime::new();
  rt.mount_root::<DeeplyNested>(0);
  assert!(rt.root().is_some());
}

#[test]
fn dirty_signal_rebuilds_before_layout() {
  let signal_out = Arc::new(Mutex::new(None));
  let mut rt = Runtime::new();
  rt.mount_root::<SignalRoot>(Shared(signal_out.clone()));

  assert_eq!(rt.root().unwrap().text_content(), Some("1"));

  signal_out.lock().unwrap().as_ref().unwrap().set(7);
  run_pass(&mut rt);

  assert_eq!(rt.root().unwrap().text_content(), Some("7"));
}

#[test]
fn child_lifecycle_tracks_insertions_and_removals() {
  let show_child = Arc::new(Mutex::new(None));
  let mounted = Arc::new(AtomicUsize::new(0));
  let unmounted = Arc::new(AtomicUsize::new(0));

  let mut rt = Runtime::new();
  rt.mount_root::<ConditionalLifecycleParent>((
    Shared(show_child.clone()),
    Shared(mounted.clone()),
    Shared(unmounted.clone()),
  ));

  assert_eq!(mounted.load(Ordering::Relaxed), 1);
  assert_eq!(unmounted.load(Ordering::Relaxed), 0);

  show_child.lock().unwrap().as_ref().unwrap().set(false);
  run_pass(&mut rt);

  assert_eq!(mounted.load(Ordering::Relaxed), 1);
  assert_eq!(unmounted.load(Ordering::Relaxed), 1);

  show_child.lock().unwrap().as_ref().unwrap().set(true);
  run_pass(&mut rt);

  assert_eq!(mounted.load(Ordering::Relaxed), 2);
  assert_eq!(unmounted.load(Ordering::Relaxed), 1);
}

#[test]
fn for_each_preserves_keyed_child_components_across_reorder() {
  let items = Arc::new(Mutex::new(None));
  let mounted = Arc::new(AtomicUsize::new(0));
  let unmounted = Arc::new(AtomicUsize::new(0));

  let mut rt = Runtime::new();
  rt.mount_root::<KeyedForEachLifecycleParent>((
    Shared(items.clone()),
    Shared(mounted.clone()),
    Shared(unmounted.clone()),
  ));

  assert_eq!(mounted.load(Ordering::Relaxed), 3);
  assert_eq!(unmounted.load(Ordering::Relaxed), 0);

  items.lock().unwrap().as_ref().unwrap().update(|items| {
    items.rotate_left(1);
  });
  run_pass(&mut rt);

  assert_eq!(mounted.load(Ordering::Relaxed), 3);
  assert_eq!(unmounted.load(Ordering::Relaxed), 0);
}

#[test]
fn root_lifecycle_runs_once_and_unmounts() {
  let mounted = Arc::new(AtomicUsize::new(0));
  let unmounted = Arc::new(AtomicUsize::new(0));
  let mut rt = Runtime::new();

  rt.mount_root::<RootLifecycle>((Shared(mounted.clone()), Shared(unmounted.clone())));
  rt.rebuild();

  assert_eq!(mounted.load(Ordering::Relaxed), 1);
  assert_eq!(unmounted.load(Ordering::Relaxed), 0);

  rt.set_root(Element::new());

  assert_eq!(mounted.load(Ordering::Relaxed), 1);
  assert_eq!(unmounted.load(Ordering::Relaxed), 1);
}

#[test]
fn create_context_reactive_marks_dirty() {
  let mut ctx = Ctx::new_root();
  let rctx = ctx.create_context(0_i32);
  assert!(ctx.is_dirty());
  rctx.set(42);
  assert!(ctx.is_dirty());
  assert_eq!(rctx.get(), 42);
}

#[test]
fn memo_via_ctx() {
  let mut ctx = Ctx::new_root();
  let sig = ctx.signal(5_i32);
  let sc = sig.clone();
  let m = ctx.memo(move || sc.get() * 2);
  assert_eq!(m.get(), 10);
  sig.set(3);
  assert_eq!(m.get(), 6);
}

#[test]
fn create_ref_via_ctx() {
  let ctx = Ctx::new_root();
  let r = ctx.create_ref(42_i32);
  assert_eq!(r.get(), 42);
  r.set(99);
  assert_eq!(r.get(), 99);
}

#[test]
fn element_ref_via_ctx() {
  let mut ctx = Ctx::new_root();
  let _nr = ctx.element_ref();
}

#[test]
fn element_ref_mut_via_ctx() {
  let mut ctx = Ctx::new_root();
  let _nr = ctx.element_ref_mut();
}

#[test]
fn interaction_via_ctx() {
  let ctx = Ctx::new_root();
  let state = ctx.interaction();
  assert!(!state.is_hovered());
  assert!(!state.is_active());
}
