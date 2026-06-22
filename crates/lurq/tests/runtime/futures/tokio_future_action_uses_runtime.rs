use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureAction, FutureStatus},
    events::MouseButton,
  },
  components::{Column, Modal, Parent, Rect, Stack, Text},
  node::Element,
};
use tokio::runtime::Builder;

use crate::support::TestSurface;

struct SharedTokioAction {
  action: Arc<Mutex<Option<FutureAction<String, String, String>>>>,
  completed: Arc<AtomicBool>,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for SharedTokioAction {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
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

struct ModalFutureActionRoot;

impl Component for ModalFutureActionRoot {
  type Props = SharedTokioAction;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Stack::new()
      .size(240.0, 160.0)
      .child(Rect::new(240.0, 160.0).background("#111827"))
      .child(Modal::new(ctx.mount::<ModalFutureActionContent>(ctx.props::<Self::Props>().clone())).target(Parent))
  }
}

struct ModalFutureActionContent;

impl Component for ModalFutureActionContent {
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

    let run_action = action.clone();
    let state = action.state().get();
    let label = match state.status {
      FutureStatus::Idle => "idle".to_owned(),
      FutureStatus::Pending => "pending".to_owned(),
      FutureStatus::Fulfilled => state.data.unwrap(),
      _ => "unexpected".to_owned(),
    };

    Column::new()
      .child(
        Rect::new(80.0, 40.0)
          .background("#22c55e")
          .on_click(move |_| run_action.run("done".to_owned())),
      )
      .child(Text::new(&label))
  }
}

#[test]
fn tokio_future_action_inside_modal_updates_after_completion() {
  let runtime = Builder::new_current_thread().build().unwrap();
  let mut app = App::new().with_tokio_handle(runtime.handle().clone());
  let action = Arc::new(Mutex::new(None));
  let completed = Arc::new(AtomicBool::new(false));
  let mut tree = Tree::new();
  tree.mount_root::<ModalFutureActionRoot>(
    &mut app,
    SharedTokioAction {
      action,
      completed: completed.clone(),
    },
  );
  tree.pass(&mut app, &TestSurface);
  assert_eq!(tree.root().unwrap().tag_name(), "OverlayHost");
  assert!(tree.find_element(|el| el.text_content() == Some("idle")).is_some());

  tree.mouse_down(10.0, 10.0, MouseButton::Left);
  tree.mouse_up(10.0, 10.0, MouseButton::Left);
  tree.pass(&mut app, &TestSurface);
  assert_eq!(tree.root().unwrap().tag_name(), "OverlayHost");
  assert!(tree.find_element(|el| el.text_content() == Some("pending")).is_some());

  runtime.block_on(async {
    while !completed.load(Ordering::SeqCst) {
      tokio::task::yield_now().await;
    }
  });
  tree.tick_futures();
  tree.pass(&mut app, &TestSurface);

  assert_eq!(tree.root().unwrap().tag_name(), "OverlayHost");
  assert!(tree.find_element(|el| el.text_content() == Some("done")).is_some());
}
