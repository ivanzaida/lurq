use lurq::{
  app::Tree,
  layout::{
    layout_kind::ScrollState,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
  },
};

use crate::support::run_pass;

/// Reserved placement is a stable gutter: with Auto visibility the thumb only
/// draws on overflow, but the gutter must stay reserved even when the content
/// fits — otherwise the viewport width jumps by the gutter as content crosses
/// the overflow threshold.
#[test]
fn reserved_gutter_is_kept_when_content_fits_the_viewport() {
  let mut runtime = Tree::new();
  let scroll_state = ScrollState::new();

  runtime.set_root(
    lurq::components::ScrollVertical::new(lurq::components::Rect::new(50.0, 100.0))
      .with_scroll_state(scroll_state.clone())
      .scrollbar(ScrollBarStyle {
        visible: ScrollBarVisibility::Auto,
        placement: ScrollBarPlacement::Reserved,
        width: 8.0,
        padding: 1.0,
        ..Default::default()
      })
      .size(150.0, 200.0),
  );

  run_pass(&mut runtime);

  // Gutter = width + padding * 2 = 10, so the viewport is 140 wide even
  // though the 100-tall content needs no scrolling.
  assert_eq!(scroll_state.viewport_width(), 140.0);
  assert_eq!(scroll_state.viewport_height(), 200.0);
}
