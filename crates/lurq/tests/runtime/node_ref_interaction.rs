use lurq::{
  app::{Runtime, events::MouseButton},
  core::{NodeRef, Signal},
  node::{Element, color::Color},
};

#[test]
fn node_ref_tracks_hover_and_active_state() {
  let node_ref = NodeRef::new();
  let mut runtime = Runtime::new();

  runtime.set_root(Element::rect(100.0, 40.0).ref_node(node_ref.clone()));
  let rect = runtime.find_element(|_| true).unwrap().rect;
  let (x, y) = rect.center();

  assert!(!node_ref.hovered());
  assert!(!node_ref.active());

  runtime.mouse_move(x, y);
  assert!(node_ref.hovered());

  runtime.mouse_down(x, y, MouseButton::Left);
  assert!(node_ref.active());

  runtime.mouse_up(x + 200.0, y + 200.0, MouseButton::Left);
  assert!(!node_ref.active());

  runtime.mouse_move(x + 200.0, y + 200.0);
  assert!(!node_ref.hovered());
}

#[test]
fn node_ref_tracks_focus_state() {
  let first_ref = NodeRef::new();
  let second_ref = NodeRef::new();
  let mut runtime = Runtime::new();

  runtime.set_root(
    Element::row()
      .spacing(8.0)
      .child(
        Element::text_input(Signal::new(String::new()))
          .ref_node(first_ref.clone())
          .fill("#ef4444")
          .width(100.0),
      )
      .child(
        Element::text_input(Signal::new(String::new()))
          .ref_node(second_ref.clone())
          .fill("#22c55e")
          .width(100.0),
      ),
  );

  let first = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .rect;
  let second = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .rect;

  runtime.click(first.x + 10.0, first.y + first.height / 2.0, MouseButton::Left);

  assert!(first_ref.focused());
  assert!(!second_ref.focused());

  runtime.click(second.x + 10.0, second.y + second.height / 2.0, MouseButton::Left);

  assert!(!first_ref.focused());
  assert!(second_ref.focused());
}
