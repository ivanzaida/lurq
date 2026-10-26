//! An id put on the element `Markdown::mount` returns stays on the document
//! when the document renders again by itself (a theme change re-renders the
//! `Markdown`, which reads the theme, and not its host, which does not).

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, theme::ThemeMarkdown},
  components::{Column, Markdown, MarkdownProps},
  node::{Element, ElementRef},
};

use crate::support::run_pass;

#[derive(Clone, Default)]
struct HostRenders(Arc<AtomicUsize>);

impl PartialEq for HostRenders {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for HostRenders {}

struct Host;

impl Component for Host {
  type Props = HostRenders;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.props::<Self::Props>().0.fetch_add(1, Ordering::Relaxed);
    Column::new().child(Markdown::mount(ctx, MarkdownProps::new("The **brief** of the task.")).id("task-brief-text"))
  }
}

fn has_text(element: ElementRef<'_>, text: &str) -> bool {
  element.text_content().is_some_and(|content| content.contains(text))
    || element.children().into_iter().any(|child| has_text(child, text))
}

#[test]
fn an_id_on_a_mounted_markdown_survives_the_documents_own_render() {
  let renders = HostRenders::default();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Host>(&mut app, renders.clone());
  run_pass(&mut tree);
  let brief = tree
    .get_element_by_id("task-brief-text")
    .expect("the document has the id");
  assert!(has_text(brief, "brief"));

  app.theme().set_markdown(ThemeMarkdown {
    document_spacing: 20.0,
    ..ThemeMarkdown::default()
  });
  run_pass(&mut tree);

  assert_eq!(renders.0.load(Ordering::Relaxed), 1, "only the document rendered again");
  let brief = tree
    .get_element_by_id("task-brief-text")
    .expect("the document keeps the id after its own render");
  assert!(has_text(brief, "brief"));
}
