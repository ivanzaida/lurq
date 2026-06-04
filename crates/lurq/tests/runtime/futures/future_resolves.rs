use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureStatus},
  },
  components::Text,
  node::Element,
};

struct ReadyFuture;

impl Component for ReadyFuture {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let state = ctx
      .future((), |_| async { Ok::<_, String>("done".to_owned()) })
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
fn future_resolves_after_future_tick() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<ReadyFuture>(&mut app, ());

  assert_eq!(tree.root().unwrap().text_content(), Some("pending"));

  tree.tick_futures();

  assert_eq!(tree.root().unwrap().text_content(), Some("done"));
  assert!(tree.needs_redraw());
}
