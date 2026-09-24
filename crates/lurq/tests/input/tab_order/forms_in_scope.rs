use lurq::{
  components::{Button, Column, Form, FormHandle, FormOptions, FormProps, Modal, Root, TextInput},
  core::Signal,
};

use super::{click_id, focused_id, headless, tab};

fn form(prefix: &str) -> lurq::node::Element {
  Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new()
      .child(TextInput::new(Signal::new(String::new())).id(format!("{prefix}-name")))
      .child(Button::new("Save").id(format!("{prefix}-save")).submit()),
  )
}

#[test]
fn tab_from_the_window_enters_a_form_and_then_cycles_it() {
  let mut tree = headless(
    Column::new()
      .child(Button::new("Toolbar").id("toolbar").tab_index(0))
      .child(form("form")),
  );

  let mut order = Vec::new();
  for _ in 0..4 {
    tab(&mut tree);
    order.push(focused_id(&tree).unwrap());
  }
  assert_eq!(order, ["toolbar", "form-name", "form-save", "form-name"]);
}

#[test]
fn tab_after_clicking_a_toolbar_button_reaches_the_form() {
  let mut tree = headless(Column::new().child(Button::new("Help").id("help")).child(form("form")));

  click_id(&mut tree, "help");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("form-name"));
}

#[test]
fn form_inside_a_modal_cycles_inside_the_modal() {
  let mut tree = headless(
    Column::new().child(form("page")).child(
      Modal::new(Column::new().child(form("dialog")))
        .open_when(true)
        .target(Root),
    ),
  );

  let mut order = Vec::new();
  for _ in 0..3 {
    tab(&mut tree);
    order.push(focused_id(&tree).unwrap());
  }
  assert_eq!(order, ["dialog-name", "dialog-save", "dialog-name"]);
}
