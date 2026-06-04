use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureAction, FutureStatus},
  },
  components::Text,
  node::Element,
};

#[derive(lurq::DevtoolsInspectable)]
struct SharedAction(Arc<Mutex<Option<FutureAction<String, String, String>>>>);

impl Clone for SharedAction {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl PartialEq for SharedAction {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct RunFutureAction;

impl Component for RunFutureAction {
  type Props = SharedAction;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let action = ctx.future_action(|value: String| async move { Ok::<_, String>(value) });
    *ctx.props::<SharedAction>().0.lock().unwrap() = Some(action.clone());

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
fn future_action_runs_when_called() {
  let action = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RunFutureAction>(&mut app, SharedAction(action.clone()));

  assert_eq!(tree.root().unwrap().text_content(), Some("idle"));

  action.lock().unwrap().clone().unwrap().run("done".to_owned());
  tree.tick_futures();

  assert_eq!(tree.root().unwrap().text_content(), Some("done"));
  assert!(tree.needs_redraw());
}
