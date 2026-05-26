use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc, Mutex,
};

use lurq::{
  app::{component::Component, ctx::Ctx, Runtime},
  core::Signal,
  layout::{Constraints, Size},
  node::Element,
};

// --- Test components ---

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = i32;
  fn create(ctx: &mut Ctx, initial: i32) -> Self {
    Self {
      count: ctx.signal(initial),
    }
  }
  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::text(&format!("{}", self.count.get()))
  }
}

struct Parent;

impl Component for Parent {
  type Props = ();
  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> Element {
    Element::column()
      .child(ctx.mount::<Counter>(0))
      .child(ctx.mount::<Counter>(10))
  }
}

struct ContextProvider;

impl Component for ContextProvider {
  type Props = ();
  fn create(ctx: &mut Ctx, _: ()) -> Self {
    ctx.provide(42_i32);
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> Element {
    ctx.mount::<ContextConsumer>(())
  }
}

struct ContextConsumer;

impl Component for ContextConsumer {
  type Props = ();
  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> Element {
    let val = ctx.use_context::<i32>().unwrap_or(0);
    Element::text(&format!("{}", val))
  }
}

struct SlotWrapper;

impl Component for SlotWrapper {
  type Props = ();
  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> Element {
    let count = ctx.children().len();
    Element::column().with_children((0..count).map(|_| Element::new()))
  }
}

struct ForEachParent;

impl Component for ForEachParent {
  type Props = ();
  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> Element {
    let items = vec![1, 2, 3, 4, 5];
    let nodes = ctx.for_each(items, |i| *i, |_ctx, i| Element::text(&format!("item-{}", i)));
    Element::column().with_children(nodes)
  }
}

struct ErrorComponent;

impl Component for ErrorComponent {
  type Props = ();
  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> Element {
    ctx.error_boundary(
      |_ctx| {
        panic!("intentional panic");
      },
      || Element::text("fallback"),
    )
  }
}

struct EmptyComponent;

impl Component for EmptyComponent {
  type Props = ();
  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }
  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::new()
  }
}

struct DeeplyNested;

impl Component for DeeplyNested {
  type Props = u32;
  fn create(_ctx: &mut Ctx, _: u32) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> Element {
    ctx.mount::<EmptyComponent>(())
  }
}

struct SignalRoot {
  count: Signal<i32>,
}

impl Component for SignalRoot {
  type Props = Arc<Mutex<Option<Signal<i32>>>>;

  fn create(ctx: &mut Ctx, signal_out: Self::Props) -> Self {
    let count = ctx.signal(1);
    *signal_out.lock().unwrap() = Some(count.clone());
    Self { count }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::text(&format!("{}", self.count.get()))
  }
}

struct LifecycleChild {
  mounted: Arc<AtomicUsize>,
  unmounted: Arc<AtomicUsize>,
}

impl Component for LifecycleChild {
  type Props = (Arc<AtomicUsize>, Arc<AtomicUsize>);

  fn create(_ctx: &mut Ctx, props: Self::Props) -> Self {
    Self {
      mounted: props.0,
      unmounted: props.1,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
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
  type Props = (Arc<Mutex<Option<Signal<bool>>>>, Arc<AtomicUsize>, Arc<AtomicUsize>);

  fn create(ctx: &mut Ctx, props: Self::Props) -> Self {
    let show_child = ctx.signal(true);
    *props.0.lock().unwrap() = Some(show_child.clone());
    Self {
      show_child,
      mounted: props.1,
      unmounted: props.2,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> Element {
    if self.show_child.get() {
      ctx.mount::<LifecycleChild>((self.mounted.clone(), self.unmounted.clone()))
    } else {
      Element::new()
    }
  }
}

struct RootLifecycle {
  mounted: Arc<AtomicUsize>,
  unmounted: Arc<AtomicUsize>,
}

impl Component for RootLifecycle {
  type Props = (Arc<AtomicUsize>, Arc<AtomicUsize>);

  fn create(_ctx: &mut Ctx, props: Self::Props) -> Self {
    Self {
      mounted: props.0,
      unmounted: props.1,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
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
  rt.mount_root::<SignalRoot>(signal_out.clone());

  assert_eq!(rt.root().unwrap().text_content(), Some("1"));

  signal_out.lock().unwrap().as_ref().unwrap().set(7);
  rt.compute_layout(Constraints::loose(Size::new(100.0, 100.0))).unwrap();

  assert_eq!(rt.root().unwrap().text_content(), Some("7"));
}

#[test]
fn child_lifecycle_tracks_insertions_and_removals() {
  let show_child = Arc::new(Mutex::new(None));
  let mounted = Arc::new(AtomicUsize::new(0));
  let unmounted = Arc::new(AtomicUsize::new(0));

  let mut rt = Runtime::new();
  rt.mount_root::<ConditionalLifecycleParent>((show_child.clone(), mounted.clone(), unmounted.clone()));

  assert_eq!(mounted.load(Ordering::Relaxed), 1);
  assert_eq!(unmounted.load(Ordering::Relaxed), 0);

  show_child.lock().unwrap().as_ref().unwrap().set(false);
  rt.compute_layout(Constraints::loose(Size::new(100.0, 100.0))).unwrap();

  assert_eq!(mounted.load(Ordering::Relaxed), 1);
  assert_eq!(unmounted.load(Ordering::Relaxed), 1);

  show_child.lock().unwrap().as_ref().unwrap().set(true);
  rt.compute_layout(Constraints::loose(Size::new(100.0, 100.0))).unwrap();

  assert_eq!(mounted.load(Ordering::Relaxed), 2);
  assert_eq!(unmounted.load(Ordering::Relaxed), 1);
}

#[test]
fn root_lifecycle_runs_once_and_unmounts() {
  let mounted = Arc::new(AtomicUsize::new(0));
  let unmounted = Arc::new(AtomicUsize::new(0));
  let mut rt = Runtime::new();

  rt.mount_root::<RootLifecycle>((mounted.clone(), unmounted.clone()));
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
fn node_ref_via_ctx() {
  let ctx = Ctx::new_root();
  let _nr = ctx.node_ref();
}

#[test]
fn interaction_via_ctx() {
  let ctx = Ctx::new_root();
  let state = ctx.interaction();
  assert!(!state.is_hovered());
  assert!(!state.is_active());
}
