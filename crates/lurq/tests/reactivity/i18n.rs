use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Text},
  layout::{Constraints, Size, quad::QuadContent},
  node::Element,
};

use crate::support::{TestSurface, run_pass};

struct Shared<T>(Arc<T>);

#[cfg(feature = "devtools")]
impl<T> lurq::app::component::DevtoolsInspectable for Shared<T> {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct LocalizedRoot {
  renders: Arc<AtomicUsize>,
}

struct LocalizedHost;

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

impl Component for LocalizedHost {
  type Props = Shared<AtomicUsize>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(ctx.mount::<LocalizedRoot>(ctx.props::<Self::Props>().clone()))
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
fn locale_change_remeasures_translated_text_layout() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  app.i18n().add_resource("en", "translation", "hello", "Hi");
  app
    .i18n()
    .add_resource("uk", "translation", "hello", "Vitayu vitayu vitayu");

  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.mount_root::<LocalizedRoot>(&mut app, Shared(renders));
  tree.pass(&mut app, &TestSurface);

  let en_width = tree.last_layout().expect("initial layout should exist").size.width;

  app.i18n().set_locale("uk");
  tree.pass(&mut app, &TestSurface);

  let uk_width = tree.last_layout().expect("updated layout should exist").size.width;
  assert!(
    uk_width > en_width * 2.0,
    "translated text should be remeasured immediately after locale change: {uk_width} <= {en_width}"
  );
}

#[test]
fn nested_locale_change_remeasures_translated_text_layout() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  app.i18n().add_resource("en", "translation", "hello", "Hi");
  app
    .i18n()
    .add_resource("uk", "translation", "hello", "Vitayu vitayu vitayu");

  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.mount_root::<LocalizedHost>(&mut app, Shared(renders));
  tree.pass(&mut app, &TestSurface);

  let en_width = tree.last_layout().expect("initial layout should exist").size.width;

  app.i18n().set_locale("uk");
  tree.pass(&mut app, &TestSurface);

  let uk_width = tree.last_layout().expect("updated layout should exist").size.width;
  assert!(
    uk_width > en_width * 2.0,
    "nested translated text should be remeasured immediately after locale change: {uk_width} <= {en_width}"
  );

  let quads = tree.resolve_quads(tree.last_layout().expect("updated layout should exist"));
  let text_quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { ref text, .. } if text == "Vitayu vitayu vitayu"))
    .expect("updated text quad should be emitted");
  assert_eq!(text_quad.width, uk_width);
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
