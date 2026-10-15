//! A `Signal` written off the event loop's thread wakes the loop, so a shell that waits instead of polling picks the
//! write up at once: a progress signal set from a Tokio task or a thread, or an item a stream emits from a thread.
//! A write on the loop's own thread does not wake it: that turn picks the write up already.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, StreamEmitter},
  },
  components::Text,
  core::Signal,
  node::Element,
};

/// Lets the test reach what a component created.
struct Shared<T>(Arc<Mutex<Option<T>>>);

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> Default for Shared<T> {
  fn default() -> Self {
    Self(Arc::new(Mutex::new(None)))
  }
}

#[cfg(feature = "devtools")]
impl<T> lurq::app::component::DevtoolsInspectable for Shared<T> {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<T: Clone> Shared<T> {
  fn get(&self) -> T {
    self.0.lock().unwrap().clone().expect("the component shared it")
  }
}

fn counting_tree() -> (Tree, Arc<AtomicUsize>) {
  let tree = Tree::new();
  let wakes = Arc::new(AtomicUsize::new(0));
  tree.set_canvas_waker({
    let wakes = wakes.clone();
    move || {
      wakes.fetch_add(1, Ordering::SeqCst);
    }
  });
  (tree, wakes)
}

struct ShowsProgress {
  progress: Signal<u32>,
}

impl Component for ShowsProgress {
  type Props = Shared<Signal<u32>>;

  fn create(ctx: &mut Ctx) -> Self {
    let progress = ctx.signal(0_u32);
    *ctx.props::<Shared<Signal<u32>>>().0.lock().unwrap() = Some(progress.clone());
    Self { progress }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Text::new(&format!("{}%", self.progress.get()))
  }
}

#[test]
fn a_signal_set_off_the_loop_thread_wakes_the_event_loop() {
  let mut app = App::new();
  let (mut tree, wakes) = counting_tree();
  let shared = Shared::default();
  tree.mount_root::<ShowsProgress>(&mut app, shared.clone());
  tree.pass_headless(&mut app);
  let progress = shared.get();

  let before = wakes.load(Ordering::SeqCst);
  progress.set(10);
  assert_eq!(
    wakes.load(Ordering::SeqCst),
    before,
    "a write on the loop's thread is picked up by its turn and needs no wake"
  );
  tree.pass_headless(&mut app);
  assert_eq!(tree.root().unwrap().text_content(), Some("10%"));

  let before = wakes.load(Ordering::SeqCst);
  std::thread::spawn(move || progress.set(50)).join().unwrap();
  assert!(
    wakes.load(Ordering::SeqCst) > before,
    "a write from another thread must wake the event loop, or a waiting shell shows it only at the next event"
  );
  tree.pass_headless(&mut app);
  assert_eq!(tree.root().unwrap().text_content(), Some("50%"));
}

#[test]
fn many_signal_sets_off_the_loop_thread_between_two_turns_wake_the_loop_once() {
  // Each wake is a message posted to the loop (`PostMessage` on Windows, whose queue holds 10,000), so a
  // chatty thread must not post one per write.
  let mut app = App::new();
  let (mut tree, wakes) = counting_tree();
  let shared = Shared::default();
  tree.mount_root::<ShowsProgress>(&mut app, shared.clone());
  tree.pass_headless(&mut app);
  let progress = shared.get();

  let before = wakes.load(Ordering::SeqCst);
  let writer = progress.clone();
  std::thread::spawn(move || {
    for value in 0..10_000 {
      writer.set(value);
    }
  })
  .join()
  .unwrap();
  let woken = wakes.load(Ordering::SeqCst) - before;
  assert!(
    (1..=3).contains(&woken),
    "10,000 writes before the loop turns wake it once, not per write: {woken} wakes"
  );
  tree.pass_headless(&mut app);
  assert_eq!(tree.root().unwrap().text_content(), Some("9999%"));

  let before = wakes.load(Ordering::SeqCst);
  std::thread::spawn(move || progress.set(1)).join().unwrap();
  assert!(
    wakes.load(Ordering::SeqCst) > before,
    "once the loop has rendered the writes, the next one wakes it again"
  );
}

struct FollowsStream;

impl Component for FollowsStream {
  type Props = Shared<StreamEmitter<u32, ()>>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let shared = ctx.props::<Shared<StreamEmitter<u32, ()>>>().clone();
    let state = ctx
      .stream((), move |_, emitter: StreamEmitter<u32, ()>| {
        *shared.0.lock().unwrap() = Some(emitter);
        std::future::pending::<()>()
      })
      .state()
      .get();
    Text::new(&state.data.map_or_else(|| "none".to_owned(), |item| item.to_string()))
  }
}

#[test]
fn a_stream_emitting_from_a_thread_wakes_the_event_loop() {
  let mut app = App::new();
  let (mut tree, wakes) = counting_tree();
  let shared = Shared::default();
  tree.mount_root::<FollowsStream>(&mut app, shared.clone());
  tree.pass_headless(&mut app);
  let emitter = shared.get();
  assert_eq!(tree.root().unwrap().text_content(), Some("none"));

  let before = wakes.load(Ordering::SeqCst);
  std::thread::spawn(move || assert!(emitter.emit(7))).join().unwrap();
  assert!(
    wakes.load(Ordering::SeqCst) > before,
    "an item emitted from a thread must wake the event loop"
  );
  tree.pass_headless(&mut app);
  assert_eq!(tree.root().unwrap().text_content(), Some("7"));
}
