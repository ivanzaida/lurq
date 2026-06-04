use lurq::components::{Column, Form, FormHandle, FormOptions, FormProps, Text};

#[test]
fn form_wraps_explicit_layout_child_without_replacing_it() {
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new().child(Text::new("First")).child(Text::new("Second")),
  ));

  let root = runtime.root().expect("form should be mounted as root");
  assert_eq!(root.tag_name(), "Form");

  let children: Vec<_> = root.children().iter().collect();
  assert_eq!(children.len(), 1);
  assert_eq!(children[0].tag_name(), "Column");
}
