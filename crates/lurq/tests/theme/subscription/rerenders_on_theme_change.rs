use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, theme::Theme},
  layout::text_style::TextStyle,
  node::Element,
};

use crate::support::run_pass;

#[derive(lurq::DevtoolsInspectable)]
struct Shared<T>(Arc<T>);

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

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared").field(&(Arc::as_ptr(&self.0) as usize)).finish()
  }
}

struct ThemeSubscriber {
  renders: Arc<AtomicUsize>,
}

impl Component for ThemeSubscriber {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    let font_size = ctx.theme().default_text_style().font_size;
    lurq::components::Text::new(&format!("font={font_size}"))
  }
}

#[test]
fn rerenders_component_that_reads_theme_when_theme_changes() {
  let renders = Arc::new(AtomicUsize::new(0));
  let theme = Theme::new();
  let mut tree = Tree::new();
  tree.mount_root::<ThemeSubscriber>(theme.clone(), Shared(renders.clone()));

  run_pass(&mut tree);
  assert_eq!(renders.load(Ordering::Relaxed), 1);

  theme.set_default_text_style(TextStyle {
    font_size: 22.0,
    ..TextStyle::default()
  });
  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert!(tree.find_element(|el| el.text_content() == Some("font=22")).is_some());
}
