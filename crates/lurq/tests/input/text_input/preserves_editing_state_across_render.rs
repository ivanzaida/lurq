use lurq::{
  app::{Tree, component::Component, ctx::Ctx, events::MouseButton, theme::Theme},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

struct EditableText {
  value: Signal<String>,
}

impl Component for EditableText {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      value: ctx.signal("AB".to_owned()),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::TextInput::new(self.value.clone())
  }
}

#[test]
fn preserves_focus_and_caret_after_signal_driven_render() {
  let mut runtime = Tree::new();
  runtime.mount_root::<EditableText>(Theme::default(), ());
  run_pass(&mut runtime);

  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("ArrowLeft".to_owned(), "ArrowLeft".to_owned(), false, false, false);
  runtime.key_down("C".to_owned(), "KeyC".to_owned(), false, false, false);
  runtime.key_down("D".to_owned(), "KeyD".to_owned(), false, false, false);

  let value = runtime.find_element(|el| el.text_content() == Some("ACDB"));

  assert!(value.is_some());
}
