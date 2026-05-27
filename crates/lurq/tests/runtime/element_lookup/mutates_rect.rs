use lurq::{
  app::Runtime,
  node::{Element, color::Color},
};

#[test]
fn updates_layout_after_mutating_found_element_rect() {
  let mut runtime = Runtime::new();
  runtime.set_root(
    Element::column()
      .child(Element::rect(10.0, 20.0).fill("#22c55e"))
      .pad(10.0),
  );

  let found = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap();
  let rect = found.bounds();
  assert_eq!(rect.x, 10.0);
  assert_eq!(rect.y, 10.0);
  assert_eq!(rect.relative_x, 0.0);
  assert_eq!(rect.relative_y, 0.0);

  let found = runtime
    .find_element_mut(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap();
  found.set_relative_bounds(15.0, 20.0, 30.0, 40.0);
  assert!(runtime.needs_redraw());

  let found = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap();

  let rect = found.bounds();
  assert_eq!(rect.x, 25.0);
  assert_eq!(rect.y, 30.0);
  assert_eq!(rect.relative_x, 15.0);
  assert_eq!(rect.relative_y, 20.0);
  assert_eq!(rect.width, 30.0);
  assert_eq!(rect.height, 40.0);
}
