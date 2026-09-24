use std::sync::Arc;

use lurq::{
  app::{App, component::Component, ctx::Ctx},
  components::{Button, Column, Modal, Rect, Root},
  core::Signal,
  node::Element,
};

use super::{click_id, focused_id, headless, shift_tab, tab};

fn screen(open: bool, dialog: Column) -> Column {
  Column::new()
    .child(Button::new("Toolbar").id("toolbar").tab_index(0))
    .child(Button::new("Other").id("other").tab_index(0))
    .child(Modal::new(dialog).open_when(open).target(Root))
}

fn dialog() -> Column {
  Column::new()
    .child(Button::new("Cancel").id("cancel").tab_index(0))
    .child(Button::new("Confirm").id("confirm").tab_index(0))
}

#[test]
fn modal_without_form_traps_tab_and_shift_tab() {
  let mut tree = headless(screen(true, dialog()));

  let mut order = Vec::new();
  for _ in 0..3 {
    tab(&mut tree);
    order.push(focused_id(&tree).unwrap());
  }
  assert_eq!(order, ["cancel", "confirm", "cancel"]);

  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("confirm"));
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("cancel"));
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct OpenSignal(Arc<Signal<bool>>);

impl PartialEq for OpenSignal {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl std::fmt::Debug for OpenSignal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("OpenSignal")
  }
}

struct ToggledModal {
  open: Signal<bool>,
}

impl Component for ToggledModal {
  type Props = OpenSignal;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      open: (*ctx.props::<Self::Props>().0).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .child(Button::new("Toolbar").id("toolbar").tab_index(0))
      .child(Modal::new(dialog()).open(self.open.clone()).target(Root))
  }
}

#[test]
fn tab_moves_focus_from_behind_an_opened_modal_into_it() {
  let open = Signal::new(false);
  let mut app = App::new();
  let mut tree = lurq::app::Tree::new();
  tree.mount_root::<ToggledModal>(&mut app, OpenSignal(Arc::new(open.clone())));
  tree.pass_headless(&mut app);

  click_id(&mut tree, "toolbar");
  assert_eq!(focused_id(&tree).as_deref(), Some("toolbar"));

  open.set(true);
  tree.pass_headless(&mut app);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("cancel"));

  open.set(false);
  tree.pass_headless(&mut app);
  tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("toolbar"),
    "closing the modal restores the window scope"
  );
}

#[test]
fn modal_without_stops_keeps_focus_out_of_the_page_behind_it() {
  let mut tree = headless(screen(true, Column::new().child(Rect::new(100.0, 40.0))));

  tab(&mut tree);
  assert_eq!(focused_id(&tree), None, "the toolbar behind the modal is not a stop");
}

#[test]
fn modal_contents_are_hit_testable_headlessly() {
  let mut tree = headless(screen(true, dialog()));

  click_id(&mut tree, "confirm");
  assert_eq!(focused_id(&tree).as_deref(), Some("confirm"));
}
