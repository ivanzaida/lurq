use lurq::{
  app::{Runtime, events::ScrollPhase},
  layout::{
    Constraints, Size,
    layout_kind::ScrollState,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
  },
};

use super::super::PassLayoutExt;

#[test]
fn reserved_horizontal_scrollbar_height_contributes_to_vertical_overflow() {
  let mut runtime = Runtime::new();
  let scroll_state = ScrollState::new();

  runtime.set_root(
    lurq::components::ScrollBoth::new(
      lurq::components::Column::new()
        .spacing(0.0)
        .child(lurq::components::Rect::new(200.0, 73.0))
        .child(lurq::components::Rect::new(200.0, 73.0)),
    )
    .with_scroll_state(scroll_state.clone())
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Always,
      placement: ScrollBarPlacement::Reserved,
      width: 8.0,
      padding: 0.0,
      ..Default::default()
    })
    .size(100.0, 150.0),
  );

  runtime
    .pass_layout(Constraints::tight(Size::new(100.0, 150.0)))
    .unwrap();
  runtime.scroll(10.0, 10.0, 0.0, -10.0, ScrollPhase::Scroll);
  runtime
    .pass_layout(Constraints::tight(Size::new(100.0, 150.0)))
    .unwrap();

  assert_eq!(scroll_state.scroll_y(), 4.0);
}
