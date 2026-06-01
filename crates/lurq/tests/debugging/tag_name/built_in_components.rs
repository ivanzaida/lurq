use lurq::{
  app::Tree,
  components::{
    Checkbox, Column, Rect, Row, ScrollBoth, ScrollHorizontal, ScrollVertical, Slider, Spacer, Stack, Text, TextInput,
  },
  core::Signal,
  node::Element,
};

fn root_tag_name(element: impl Into<Element>) -> String {
  let mut runtime = Tree::new();
  runtime.set_root(element);
  runtime.root().unwrap().tag_name().to_owned()
}

#[test]
fn built_in_components_emit_component_tag_names() {
  let cases = [
    ("Checkbox", root_tag_name(Checkbox::new(Signal::new(false)))),
    ("Column", root_tag_name(Column::new())),
    ("Rect", root_tag_name(Rect::new(10.0, 20.0))),
    ("Row", root_tag_name(Row::new())),
    ("ScrollBoth", root_tag_name(ScrollBoth::new(Text::new("child")))),
    (
      "ScrollHorizontal",
      root_tag_name(ScrollHorizontal::new(Text::new("child"))),
    ),
    ("ScrollVertical", root_tag_name(ScrollVertical::new(Text::new("child")))),
    ("Slider", root_tag_name(Slider::new(Signal::new(0)))),
    ("Spacer", root_tag_name(Spacer::new())),
    ("Stack", root_tag_name(Stack::new())),
    ("Text", root_tag_name(Text::new("hello").padding(8.0))),
    ("TextInput", root_tag_name(TextInput::new(Signal::new(String::new())))),
  ];

  for (expected, actual) in cases {
    assert_eq!(actual, expected);
  }
}
