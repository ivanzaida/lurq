use lurq::{
  app::{Tree, theme::Theme, component::Component, ctx::Ctx},
  components::{Column, Text},
  node::Element,
};

struct ParentPanel;

impl Component for ParentPanel {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(ctx.mount::<ChildLabel>(()))
  }
}

struct ChildLabel;

impl Component for ChildLabel {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Text::new("child")
  }
}

#[test]
fn mounted_child_component_emits_component_tag_name() {
  let mut runtime = Tree::new();

  runtime.mount_root::<ParentPanel>(Theme::default(), ());
  let root = runtime.root().unwrap();
  let child = root.children().iter().next().unwrap();

  assert_eq!(child.tag_name(), "ChildLabel");
}
