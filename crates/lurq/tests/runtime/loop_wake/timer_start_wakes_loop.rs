//! Starting a timer from another thread wakes the event loop, so a shell that waits for the earliest timer
//! learns about the new deadline instead of sleeping past it.

use std::{
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use lurq::app::{
  App, Tree,
  component::Component,
  ctx::{Ctx, Interval, Timeout},
};

#[derive(Clone, Default)]
struct Timers(Arc<Mutex<Option<(Interval, Timeout)>>>);

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Timers {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Timers {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl Timers {
  fn get(&self) -> (Interval, Timeout) {
    self.0.lock().unwrap().clone().expect("the component stored its timers")
  }
}

struct StoppedTimers;

impl Component for StoppedTimers {
  type Props = Timers;

  fn create(ctx: &mut Ctx) -> Self {
    let interval = ctx.create_interval(Duration::from_secs(60), || {});
    let timeout = ctx.create_timeout(Duration::from_secs(60), || {});
    *ctx.props::<Timers>().0.lock().unwrap() = Some((interval, timeout));
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    lurq::node::Element::new()
  }
}

#[test]
fn starting_or_restarting_a_timer_from_another_thread_wakes_the_event_loop() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let wakes = Arc::new(AtomicUsize::new(0));
  tree.set_canvas_waker({
    let wakes = wakes.clone();
    move || {
      wakes.fetch_add(1, Ordering::SeqCst);
    }
  });
  let timers = Timers::default();
  tree.mount_root::<StoppedTimers>(&mut app, timers.clone());
  let (interval, timeout) = timers.get();

  let before = wakes.load(Ordering::SeqCst);
  std::thread::spawn(move || interval.start()).join().unwrap();
  assert!(
    wakes.load(Ordering::SeqCst) > before,
    "Interval::start must wake the event loop"
  );

  let before = wakes.load(Ordering::SeqCst);
  std::thread::spawn(move || timeout.restart()).join().unwrap();
  assert!(
    wakes.load(Ordering::SeqCst) > before,
    "Timeout::restart must wake the event loop"
  );
}
