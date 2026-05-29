use lurq::{
  app::{Runtime, events::MouseButton},
  components::Rect,
  core::Signal,
};

#[test]
fn dblclick_dispatches_dblclick_handler() {
  let events = Signal::new(Vec::new());
  let mut runtime = Runtime::new();

  runtime.set_root(
    Rect::new(100.0, 40.0)
      .on_click({
        let events = events.clone();
        move |_| events.update(|events| events.push("click"))
      })
      .on_dblclick({
        let events = events.clone();
        move |_| events.update(|events| events.push("dblclick"))
      }),
  );

  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.dblclick(x, y, MouseButton::Left);

  assert_eq!(events.get(), vec!["click", "dblclick"]);
}
