use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
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
use tokio::runtime::Builder;

struct SharedTokioAction {
  action: Arc<Mutex<Option<FutureAction<String, String, String>>>>,
  completed: Arc<AtomicBool>,
}

impl Clone for SharedTokioAction {
  fn clone(&self) -> Self {
    Self {
      action: self.action.clone(),
      completed: self.completed.clone(),
    }
  }
}

impl PartialEq for SharedTokioAction {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.action, &other.action) && Arc::ptr_eq(&self.completed, &other.completed)
  }
}

struct TokioFutureAction;

impl Component for TokioFutureAction {
  type Props = SharedTokioAction;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let completed = ctx.props::<SharedTokioAction>().completed.clone();
    let action = ctx.future_action(move |value: String| {
      let completed = completed.clone();
      async move {
        tokio::task::yield_now().await;
        completed.store(true, Ordering::SeqCst);
        Ok::<_, String>(value)
      }
    });
    *ctx.props::<SharedTokioAction>().action.lock().unwrap() = Some(action.clone());

    let state = action.state().get();
    let label = match state.status {
      FutureStatus::Idle => "idle".to_owned(),
      FutureStatus::Pending => "pending".to_owned(),
      FutureStatus::Fulfilled => state.data.unwrap(),
      _ => "unexpected".to_owned(),
    };
    Text::new(&label)
  }
}

#[test]
fn tokio_future_action_runs_on_tokio_runtime_and_updates_on_future_tick() {
  let runtime = Builder::new_current_thread().build().unwrap();
  let mut app = App::new().with_tokio_handle(runtime.handle().clone());
  let action = Arc::new(Mutex::new(None));
  let completed = Arc::new(AtomicBool::new(false));
  let mut tree = Tree::new();
  tree.mount_root::<TokioFutureAction>(
    &mut app,
    SharedTokioAction {
      action: action.clone(),
      completed: completed.clone(),
    },
  );

  assert_eq!(tree.root().unwrap().text_content(), Some("idle"));

  action.lock().unwrap().clone().unwrap().run("done".to_owned());
  runtime.block_on(async {
    while !completed.load(Ordering::SeqCst) {
      tokio::task::yield_now().await;
    }
  });
  tree.tick_futures();

  assert_eq!(tree.root().unwrap().text_content(), Some("done"));
}
