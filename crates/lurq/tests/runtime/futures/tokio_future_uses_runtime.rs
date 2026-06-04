use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureStatus},
  },
  components::Text,
  node::Element,
};
use tokio::{runtime::Builder, sync::oneshot};

struct SharedTokioFuture {
  receiver: Arc<Mutex<Option<oneshot::Receiver<String>>>>,
  completed: Arc<AtomicBool>,
}

impl Clone for SharedTokioFuture {
  fn clone(&self) -> Self {
    Self {
      receiver: self.receiver.clone(),
      completed: self.completed.clone(),
    }
  }
}

impl PartialEq for SharedTokioFuture {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.receiver, &other.receiver) && Arc::ptr_eq(&self.completed, &other.completed)
  }
}

struct TokioFuture;

impl Component for TokioFuture {
  type Props = SharedTokioFuture;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<SharedTokioFuture>().clone();
    let state = ctx
      .future((), move |_| {
        let receiver = props.receiver.lock().unwrap().take().unwrap();
        let completed = props.completed.clone();
        async move {
          let value = receiver.await.map_err(|error| error.to_string())?;
          completed.store(true, Ordering::SeqCst);
          Ok::<_, String>(value)
        }
      })
      .state()
      .get();
    let label = match state.status {
      FutureStatus::Pending => "pending".to_owned(),
      FutureStatus::Fulfilled => state.data.unwrap(),
      _ => "unexpected".to_owned(),
    };
    Text::new(&label)
  }
}

#[test]
fn tokio_future_resolves_on_tokio_runtime_and_updates_on_future_tick() {
  let runtime = Builder::new_current_thread().build().unwrap();
  let mut app = App::new().with_tokio_handle(runtime.handle().clone());
  let mut tree = Tree::new();
  let (sender, receiver) = oneshot::channel();
  let completed = Arc::new(AtomicBool::new(false));
  tree.mount_root::<TokioFuture>(
    &mut app,
    SharedTokioFuture {
      receiver: Arc::new(Mutex::new(Some(receiver))),
      completed: completed.clone(),
    },
  );

  assert_eq!(tree.root().unwrap().text_content(), Some("pending"));

  sender.send("done".to_owned()).unwrap();
  runtime.block_on(async {
    while !completed.load(Ordering::SeqCst) {
      tokio::task::yield_now().await;
    }
  });
  tree.tick_futures();

  assert_eq!(tree.root().unwrap().text_content(), Some("done"));
}
