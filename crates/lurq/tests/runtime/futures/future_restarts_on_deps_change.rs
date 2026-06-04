use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureStatus},
  },
  components::Text,
  node::Element,
};

struct DependentFuture;

impl Component for DependentFuture {
  type Props = i32;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let dep = *ctx.props::<i32>();
    let state = ctx
      .future(dep, |value| async move { Ok::<_, String>(format!("value={value}")) })
      .state()
      .get();
    let label = match state.status {
      FutureStatus::Pending => format!("pending:{}", state.data.as_deref().unwrap_or("none")),
      FutureStatus::Fulfilled => state.data.unwrap(),
      _ => "unexpected".to_owned(),
    };
    Text::new(&label)
  }
}

#[test]
fn future_restarts_when_deps_change_and_preserves_previous_data() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<DependentFuture>(&mut app, 1);

  assert_eq!(tree.root().unwrap().text_content(), Some("pending:none"));

  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("value=1"));

  tree.update_root_props::<DependentFuture>(2);
  tree.rebuild();
  assert_eq!(tree.root().unwrap().text_content(), Some("pending:value=1"));

  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("value=2"));
}
