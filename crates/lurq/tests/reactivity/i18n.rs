use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Text,
  node::Element,
};

use crate::support::{TestSurface, run_pass};

#[derive(Clone)]
struct Shared<T>(Arc<T>);

#[cfg(feature = "devtools")]
impl<T> lurq::app::component::DevtoolsInspectable for Shared<T> {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct LocalizedRoot {
  renders: Arc<AtomicUsize>,
}

impl Component for LocalizedRoot {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    Text::new(&ctx.t("hello"))
  }
}

#[test]
fn ctx_t_rerenders_when_locale_changes() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  app.i18n().add_resource("en", "translation", "hello", "Hello");
  app.i18n().add_resource("uk", "translation", "hello", "Vitayu");

  let mut tree = Tree::new();
  tree.mount_root::<LocalizedRoot>(&mut app, Shared(renders.clone()));

  run_pass(&mut tree);
  assert_eq!(renders.load(Ordering::Relaxed), 1);
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("Hello"))
      .is_some()
  );

  app.i18n().set_locale("uk");
  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("Vitayu"))
      .is_some()
  );
}

#[test]
fn ctx_t_args_interpolates_values() {
  let app = App::new();
  app.i18n().add_resource("en", "translation", "hello", "Hello, {{name}}");

  let mut tree = Tree::new();
  tree.set_root(Text::new(&app.i18n().t_args("hello", [("name", "Ada")])));
  tree.pass(&mut App::new(), &TestSurface);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("Hello, Ada"))
      .is_some()
  );
}
