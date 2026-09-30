use lurq::{
  components::{Button, Column, Form, FormHandle, FormOptions, FormProps, Modal, Root, TextInput},
  core::Signal,
};

use super::{click_id, focused_id, headless, shift_tab, tab};

fn form(prefix: &str) -> lurq::node::Element {
  Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new()
      .child(TextInput::new(Signal::new(String::new())).id(format!("{prefix}-name")))
      .child(Button::new("Save").id(format!("{prefix}-save")).submit()),
  )
}

fn page() -> Column {
  Column::new()
    .child(Button::new("Toolbar").id("toolbar").tab_index(0))
    .child(form("form"))
    .child(Button::new("Footer").id("footer").tab_index(0))
}

#[test]
fn tab_passes_through_a_page_form_in_the_window_order() {
  let mut tree = headless(page());

  let mut order = Vec::new();
  for _ in 0..5 {
    tab(&mut tree);
    order.push(focused_id(&tree).unwrap());
  }
  assert_eq!(order, ["toolbar", "form-name", "form-save", "footer", "toolbar"]);
}

#[test]
fn tab_from_the_last_form_control_leaves_the_form() {
  let mut tree = headless(page());
  tree.get_element_by_id_mut("form-save").unwrap().focus();

  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("footer"));
}

#[test]
fn shift_tab_from_the_first_form_control_leaves_the_form() {
  let mut tree = headless(page());
  click_id(&mut tree, "form-name");
  assert_eq!(focused_id(&tree).as_deref(), Some("form-name"));

  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("toolbar"));
}

#[test]
fn tab_after_clicking_a_toolbar_button_reaches_the_form() {
  let mut tree = headless(Column::new().child(Button::new("Help").id("help")).child(form("form")));

  click_id(&mut tree, "help");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("form-name"));
}

#[test]
fn form_inside_a_modal_cycles_inside_the_modal_forward_and_backward() {
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

  shift_tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("dialog-save"),
    "Shift+Tab wraps inside the modal"
  );
}
