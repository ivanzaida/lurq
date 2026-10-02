//! Tokio tasks started by `ctx.future`, `ctx.stream` and `ctx.future_action` live as long as the
//! component slot that owns them.

mod offstage;
mod regressions;
mod unmount;

use std::{
  marker::PhantomData,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureAction, FutureStatus, StreamEmitter},
  },
  components::Text,
  core::Signal,
  node::Element,
};
use tokio::runtime::{Builder, Runtime};

/// Shared counters a task reports to, so a test can see what it did after its component is gone.
#[derive(Clone, Default)]
struct Probe {
  /// Factory calls: how many times the task was started.
  starts: Arc<AtomicUsize>,
  /// Loop iterations of a stream.
  iterations: Arc<AtomicUsize>,
  /// Work a future did after it was released.
  effects: Arc<AtomicUsize>,
  /// Task futures dropped, by completion or by abort.
  dropped: Arc<AtomicUsize>,
  /// Lets pending futures finish.
  released: Arc<AtomicBool>,
  /// The last `ctx.future_action` a [`Trigger`] rendered.
  action: Arc<Mutex<Option<FutureAction<(), usize, String>>>>,
}

impl Probe {
  fn starts(&self) -> usize {
    self.starts.load(Ordering::SeqCst)
  }

  fn iterations(&self) -> usize {
    self.iterations.load(Ordering::SeqCst)
  }

  fn effects(&self) -> usize {
    self.effects.load(Ordering::SeqCst)
  }

  fn dropped(&self) -> usize {
    self.dropped.load(Ordering::SeqCst)
  }

  fn release(&self) {
    self.released.store(true, Ordering::SeqCst);
  }

  fn guard(&self) -> DropGuard {
    DropGuard(self.dropped.clone())
  }

  /// A future that stays pending until [`Self::release`], then records one effect worth `amount`.
  fn pending_work(&self, amount: usize) -> impl Future<Output = Result<usize, String>> + Send + use<> {
    self.starts.fetch_add(1, Ordering::SeqCst);
    let guard = self.guard();
    let released = self.released.clone();
    let effects = self.effects.clone();
    async move {
      let _guard = guard;
      while !released.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
      }
      effects.fetch_add(amount, Ordering::SeqCst);
      Ok(amount)
    }
  }

  /// A stream that emits once, then keeps working without emitting again: it only notices that
  /// its component is gone if the task is aborted.
  fn ticking_stream(&self, emitter: StreamEmitter<usize, String>) -> impl Future<Output = ()> + Send + use<> {
    self.starts.fetch_add(1, Ordering::SeqCst);
    let guard = self.guard();
    let iterations = self.iterations.clone();
    async move {
      let _guard = guard;
      emitter.emit(0);
      loop {
        iterations.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
      }
    }
  }
}

impl PartialEq for Probe {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.starts, &other.starts)
  }
}

impl lurq::app::component::DevtoolsInspectable for Probe {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct DropGuard(Arc<AtomicUsize>);

impl Drop for DropGuard {
  fn drop(&mut self) {
    self.0.fetch_add(1, Ordering::SeqCst);
  }
}

fn label(status: FutureStatus, data: Option<usize>) -> String {
  match (status, data) {
    (FutureStatus::Fulfilled, Some(data)) => format!("done:{data}"),
    (FutureStatus::Pending, _) => "pending".to_owned(),
    (FutureStatus::Idle, _) => "idle".to_owned(),
    _ => "unexpected".to_owned(),
  }
}

/// Renders a `ctx.stream` that ticks until its task is aborted.
struct Ticker;

impl Component for Ticker {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Probe>().clone();
    let state = ctx
      .stream((), move |_, emitter| probe.ticking_stream(emitter))
      .state()
      .get();
    Text::new(&label(state.status, state.data))
  }
}

/// Renders a `ctx.future` that stays pending until the probe releases it.
struct Waiter;

impl Component for Waiter {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Probe>().clone();
    let state = ctx.future((), move |_| probe.pending_work(1)).state().get();
    Text::new(&label(state.status, state.data))
  }
}

/// Renders a `ctx.future_action` that is only started from the test, through `Probe::action`.
struct Trigger;

impl Component for Trigger {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Probe>().clone();
    let work = probe.clone();
    let action = ctx.future_action(move |()| work.pending_work(1));
    *probe.action.lock().unwrap() = Some(action.clone());
    let state = action.state().get();
    Text::new(&label(state.status, state.data))
  }
}

struct HostProps {
  show: Signal<bool>,
  active: Signal<bool>,
  probe: Probe,
}

impl PartialEq for HostProps {
  fn eq(&self, other: &Self) -> bool {
    self.show.id() == other.show.id() && self.active.id() == other.active.id() && self.probe == other.probe
  }
}

impl lurq::app::component::DevtoolsInspectable for HostProps {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

/// Mounts `C` with `mount_keyed_offstage` while `show` is true; `active` moves it on and off stage.
struct Host<C>(PhantomData<fn() -> C>);

impl<C: Component<Props = Probe>> Component for Host<C> {
  type Props = HostProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self(PhantomData)
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<HostProps>();
    let (show, active, probe) = (props.show.get(), props.active.get(), props.probe.clone());
    if !show {
      return Element::new();
    }
    ctx.mount_keyed_offstage::<C>("child", probe, active)
  }
}

/// A tree whose root hosts `C` on a current-thread runtime the test drives explicitly.
struct Harness {
  runtime: Runtime,
  tree: Tree,
  show: Signal<bool>,
  active: Signal<bool>,
  probe: Probe,
}

impl Harness {
  fn mount<C: Component<Props = Probe>>() -> Self {
    Self::mount_on::<C>(Builder::new_current_thread().build().unwrap())
  }

  fn mount_on<C: Component<Props = Probe>>(runtime: Runtime) -> Self {
    let mut app = App::new().with_tokio_handle(runtime.handle().clone());
    let show = Signal::new(true);
    let active = Signal::new(true);
    let probe = Probe::default();
    let mut tree = Tree::new();
    tree.mount_root::<Host<C>>(
      &mut app,
      HostProps {
        show: show.clone(),
        active: active.clone(),
        probe: probe.clone(),
      },
    );
    Self {
      runtime,
      tree,
      show,
      active,
      probe,
    }
  }

  /// Lets the runtime run its tasks for a while.
  fn drive(&self) {
    drive(&self.runtime);
  }

  fn set_show(&mut self, show: bool) {
    self.show.set(show);
    self.tree.rebuild();
  }

  fn set_active(&mut self, active: bool) {
    self.active.set(active);
    self.tree.rebuild();
  }

  fn text(&self) -> Option<&str> {
    self.tree.root().and_then(|root| root.text_content())
  }
}

fn drive(runtime: &Runtime) {
  runtime.block_on(async {
    for _ in 0..64 {
      tokio::task::yield_now().await;
    }
  });
}
