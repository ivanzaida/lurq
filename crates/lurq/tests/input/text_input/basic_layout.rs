use lurq::{app::Runtime, core::Signal, node::Element};

#[test]
fn renders_current_value_and_can_be_found_by_rendered_text() {
  let value = Signal::new("Ada".to_owned());
  let mut runtime = Runtime::new();

  runtime.set_root(Element::text_input(value).placeholder("Name"));

  let found = runtime.find_element(|el| el.text_content() == Some("Ada"));
  assert!(found.is_some(), "text input should render its current signal value");
}

#[test]
fn renders_placeholder_when_value_is_empty() {
  let value = Signal::new(String::new());
  let mut runtime = Runtime::new();

  runtime.set_root(Element::text_input(value).placeholder("Name"));

  let found = runtime.find_element(|el| el.text_content() == Some("Name"));
  assert!(found.is_some(), "empty text input should render its placeholder");
}
