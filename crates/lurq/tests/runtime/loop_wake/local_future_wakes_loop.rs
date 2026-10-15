//! A future polled on the event loop wakes the loop when it can make progress, so an idle shell can wait instead
//! of polling every pending future without pause. Starting such a future from another thread wakes the loop to
//! poll it.

use std::{
  future::Future,
  pin::Pin,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  task::{Context, Poll, Waker},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureAction, FutureStatus},
  },
  components::Text,
  node::Element,
};

/// A future that completes once the test opens it, and wakes whoever polled it then.
#[derive(Default)]
struct Gate {
  open: bool,
  waker: Option<Waker>,
}

#[derive(Clone, Default)]
struct SharedGate(Arc<Mutex<Gate>>);

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for SharedGate {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for SharedGate {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl SharedGate {
  fn open(&self) {
    let waker = {
      let mut gate = self.0.lock().unwrap();
      gate.open = true;
      gate.waker.take()
    };
    if let Some(waker) = waker {
      waker.wake();
    }
  }
}

struct Opened(Arc<Mutex<Gate>>);

impl Future for Opened {
  type Output = Result<String, String>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut gate = self.0.lock().unwrap();
    if gate.open {
      return Poll::Ready(Ok("open".to_owned()));
    }
    gate.waker = Some(cx.waker().clone());
    Poll::Pending
  }
}

struct WaitsForGate;

impl Component for WaitsForGate {
  type Props = SharedGate;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let gate = ctx.props::<SharedGate>().0.clone();
    let state = ctx.future((), move |_| Opened(gate.clone())).state().get();
    let label = match state.status {
      FutureStatus::Fulfilled => state.data.unwrap_or_default(),
      _ => "waiting".to_owned(),
    };
    Text::new(&label)
  }
}

#[test]
fn a_local_future_that_can_progress_wakes_the_event_loop() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let wakes = Arc::new(AtomicUsize::new(0));
  tree.set_canvas_waker({
    let wakes = wakes.clone();
    move || {
      wakes.fetch_add(1, Ordering::SeqCst);
    }
  });
  let gate = SharedGate::default();
  tree.mount_root::<WaitsForGate>(&mut app, gate.clone());
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("waiting"));
  let before = wakes.load(Ordering::SeqCst);

  std::thread::spawn(move || gate.open()).join().unwrap();

  assert!(
    wakes.load(Ordering::SeqCst) > before,
    "the future's waker must wake the event loop, or a shell that waits never polls it again"
  );
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("open"));
}

type Action = FutureAction<(), String, String>;

#[derive(Clone, Default)]
struct SharedAction(Arc<Mutex<Option<Action>>>);

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for SharedAction {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for SharedAction {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct RunsAction;

impl Component for RunsAction {
  type Props = SharedAction;

  fn create(ctx: &mut Ctx) -> Self {
    let action = ctx.future_action(|()| async { Ok::<_, String>("ran".to_owned()) });
    *ctx.props::<SharedAction>().0.lock().unwrap() = Some(action);
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let action = ctx.props::<SharedAction>().0.lock().unwrap().clone().expect("created");
    let state = action.state().get();
    Text::new(state.data.as_deref().unwrap_or("idle"))
  }
}

#[test]
fn a_local_future_action_run_from_another_thread_wakes_the_event_loop() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let wakes = Arc::new(AtomicUsize::new(0));
  tree.set_canvas_waker({
    let wakes = wakes.clone();
    move || {
      wakes.fetch_add(1, Ordering::SeqCst);
    }
  });
  let shared = SharedAction::default();
  tree.mount_root::<RunsAction>(&mut app, shared.clone());
  tree.tick_futures();
  let action = shared.0.lock().unwrap().clone().expect("created");
  let before = wakes.load(Ordering::SeqCst);

  std::thread::spawn(move || action.run(())).join().unwrap();

  assert!(
    wakes.load(Ordering::SeqCst) > before,
    "starting a local future must wake the event loop, or it waits unpolled for an unrelated event"
  );
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("ran"));
}
