//! A size an element ref gives its element: laid out in place of the size it
//! was built with, at the next pass, with nothing rendered again.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Rect, Row},
  core::ElementRefMut,
  layout::{Constraints, Size},
  node::{Element, dimension::Dimension},
};

use crate::support::run_pass;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  #[devtools_ignore]
  pane: ElementRefMut,
  #[devtools_ignore]
  renders: Arc<AtomicUsize>,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.renders, &other.renders)
  }
}

/// A pane of 100 and a pane after it that takes the rest of 400.
struct Panes;

impl Component for Panes {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    props.renders.fetch_add(1, Ordering::Relaxed);
    Row::new()
      .width(400.0)
      .child(
        Rect::new(100.0, 50.0)
          .id("pane")
          .min_width(60.0)
          .ref_element(props.pane),
      )
      .child(Rect::new(10.0, 50.0).id("rest").flex(1.0))
  }
}

fn tree() -> (Tree, ElementRefMut, Arc<AtomicUsize>) {
  let (pane, renders) = (ElementRefMut::new(), Arc::new(AtomicUsize::new(0)));
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 300.0))));
  tree.mount_root::<Panes>(
    &mut App::new(),
    Props {
      pane: pane.clone(),
      renders: renders.clone(),
    },
  );
  run_pass(&mut tree);
  (tree, pane, renders)
}

fn width(tree: &mut Tree, id: &str) -> f32 {
  tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .unwrap()
    .width
}

#[test]
fn a_ref_sizes_its_element_without_a_render() {
  let (mut tree, pane, renders) = tree();
  assert_eq!(width(&mut tree, "pane"), 100.0);
  pane.set_size(Some(Dimension::Px(220.0)), None);
  assert!(tree.needs_redraw(), "the size asks for a pass");
  run_pass(&mut tree);
  assert_eq!(width(&mut tree, "pane"), 220.0);
  assert_eq!(width(&mut tree, "rest"), 180.0, "what is beside it is laid out again");
  assert_eq!(renders.load(Ordering::Relaxed), 1, "nothing rendered again");
  assert_eq!(pane.size(), (Some(Dimension::Px(220.0)), None));
}

#[test]
fn a_share_of_the_parent_and_the_least_size_built_with() {
  let (mut tree, pane, _) = tree();
  pane.set_size(Some(Dimension::Pct(25.0)), None);
  run_pass(&mut tree);
  assert_eq!(width(&mut tree, "pane"), 100.0);
  pane.set_size(Some(Dimension::Px(20.0)), None);
  run_pass(&mut tree);
  assert_eq!(width(&mut tree, "pane"), 60.0, "its least size still holds");
}

#[test]
fn none_gives_back_the_size_it_was_built_with() {
  let (mut tree, pane, _) = tree();
  pane.set_size(Some(Dimension::Px(300.0)), None);
  run_pass(&mut tree);
  pane.set_size(None, None);
  run_pass(&mut tree);
  assert_eq!(width(&mut tree, "pane"), 100.0);
  assert_eq!(width(&mut tree, "rest"), 300.0);
}
