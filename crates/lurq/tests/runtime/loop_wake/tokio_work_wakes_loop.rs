//! A future or stream running on a Tokio runtime hands its results to the event loop through a channel; handing
//! one over wakes the loop, so an idle shell can wait instead of polling the channel without pause.

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
  node::Element,
};
use tokio::{
  runtime::Builder,
  sync::{mpsc, oneshot},
};

/// The receiving ends the component's future and stream wait on; each is taken by the task that waits on it.
#[derive(Clone, Default)]
struct Inputs {
  result: Arc<Mutex<Option<oneshot::Receiver<String>>>>,
  items: Arc<Mutex<Option<mpsc::UnboundedReceiver<String>>>>,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Inputs {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Inputs {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.result, &other.result) && Arc::ptr_eq(&self.items, &other.items)
  }
}

struct TokioWork;

impl Component for TokioWork {
  type Props = Inputs;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let inputs = ctx.props::<Inputs>().clone();
    let result = ctx
      .future((), {
        let inputs = inputs.clone();
        move |_| {
          let receiver = inputs.result.lock().unwrap().take();
          async move {
            match receiver {
              Some(receiver) => receiver.await.map_err(|error| error.to_string()),
              None => Err("taken".to_owned()),
            }
          }
        }
      })
      .state()
      .get();
    let item = ctx
      .stream((), move |_, emitter: StreamEmitter<String, ()>| {
        let receiver = inputs.items.lock().unwrap().take();
        async move {
          let Some(mut receiver) = receiver else {
            return;
          };
          while let Some(item) = receiver.recv().await {
            if !emitter.emit(item) {
              return;
            }
          }
        }
      })
      .state()
      .get();
    Text::new(&format!(
      "{}/{}",
      result.data.unwrap_or_default(),
      item.data.unwrap_or_default()
    ))
  }
}

fn counting_waker(tree: &Tree) -> Arc<AtomicUsize> {
  let wakes = Arc::new(AtomicUsize::new(0));
  tree.set_canvas_waker({
    let wakes = wakes.clone();
    move || {
      wakes.fetch_add(1, Ordering::SeqCst);
    }
  });
  wakes
}

/// Runs the runtime until the event loop has been woken more than `before` times.
fn wait_for_wake(runtime: &tokio::runtime::Runtime, wakes: &AtomicUsize, before: usize) {
  runtime.block_on(async {
    for _ in 0..10_000 {
      if wakes.load(Ordering::SeqCst) > before {
        return;
      }
      tokio::task::yield_now().await;
    }
    panic!("the Tokio task handed over its result without waking the event loop");
  });
}

#[test]
fn a_tokio_future_and_stream_wake_the_event_loop_when_they_hand_over_a_result() {
  let runtime = Builder::new_current_thread().build().unwrap();
  let mut app = App::new().with_tokio_handle(runtime.handle().clone());
  let mut tree = Tree::new();
  let wakes = counting_waker(&tree);
  let (result, result_receiver) = oneshot::channel();
  let (items, items_receiver) = mpsc::unbounded_channel();
  tree.mount_root::<TokioWork>(
    &mut app,
    Inputs {
      result: Arc::new(Mutex::new(Some(result_receiver))),
      items: Arc::new(Mutex::new(Some(items_receiver))),
    },
  );
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("/"));

  let before = wakes.load(Ordering::SeqCst);
  result.send("done".to_owned()).unwrap();
  wait_for_wake(&runtime, &wakes, before);
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("done/"));

  let before = wakes.load(Ordering::SeqCst);
  items.send("first".to_owned()).unwrap();
  wait_for_wake(&runtime, &wakes, before);
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("done/first"));
}

#[test]
fn a_stream_with_items_left_after_a_turn_wakes_the_event_loop_for_the_next() {
  let runtime = Builder::new_current_thread().build().unwrap();
  let mut app = App::new().with_tokio_handle(runtime.handle().clone());
  let mut tree = Tree::new();
  let wakes = counting_waker(&tree);
  let (_result, result_receiver) = oneshot::channel::<String>();
  let (items, items_receiver) = mpsc::unbounded_channel();
  tree.mount_root::<TokioWork>(
    &mut app,
    Inputs {
      result: Arc::new(Mutex::new(Some(result_receiver))),
      items: Arc::new(Mutex::new(Some(items_receiver))),
    },
  );
  tree.tick_futures();

  // Both items are handed over before the loop's next turn: their two wakes make one turn.
  let before = wakes.load(Ordering::SeqCst);
  items.send("first".to_owned()).unwrap();
  items.send("second".to_owned()).unwrap();
  wait_for_wake(&runtime, &wakes, before + 1);
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("/first"));

  let after_turn = wakes.load(Ordering::SeqCst);
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("/second"));
  assert!(
    after_turn > before + 2,
    "a turn that applied an item must wake the loop again, or the item after it waits for an unrelated event"
  );
}
