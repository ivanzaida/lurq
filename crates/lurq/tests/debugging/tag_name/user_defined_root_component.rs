use lurq::{
  app::{Tree, component::Component, ctx::Ctx, theme::Theme},
  components::Text,
  node::Element,
};

struct DebugPanel;

impl Component for DebugPanel {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Text::new("debug")
  }
}

#[test]
fn mounted_root_component_emits_component_tag_name() {
  let mut runtime = Tree::new();

  runtime.mount_root::<DebugPanel>(Theme::default(), ());

  assert_eq!(runtime.root().unwrap().tag_name(), "DebugPanel");
}
