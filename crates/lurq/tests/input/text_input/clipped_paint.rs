//! A long multi-line input in a scroll view paints every line it scrolls to,
//! whatever part of it the first paint showed.

use lurq::{
  app::{App, Tree},
  components::{ScrollVertical, TextInput},
  core::Signal,
  layout::{layout_kind::ScrollState, text_style::TextStyle},
};

use crate::support::render_pass_with_app;

const SCALE: f32 = 2.0;
const LINE_HEIGHT: f32 = 20.0;
const LINES: usize = 60;

/// Bottom edge, in logical pixels, of the lowest glyph painted after the view
/// first paints at the top and then scrolls to the end.
fn lowest_glyph_after_scrolling(viewport: f32) -> f32 {
  let text = (0..LINES)
    .map(|line| format!("line {line}"))
    .collect::<Vec<_>>()
    .join("\n");
  let style = TextStyle {
    font_size: 10.0,
    line_height: LINE_HEIGHT / 10.0,
    ..TextStyle::default()
  };
  let state = ScrollState::new();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_scale_factor(SCALE);
  tree.resize((400.0 * SCALE) as u32, (600.0 * SCALE) as u32);
  tree.set_root(
    ScrollVertical::new(
      TextInput::styled(Signal::new(text), style)
        .textarea()
        .rows(1, LINES)
        .width(300.0),
    )
    .with_scroll_state(state.clone())
    .width(400.0)
    .height(viewport),
  );
  render_pass_with_app(&mut tree, &mut app);
  state.set_scroll_pending(0.0, state.content_height() - viewport);
  render_pass_with_app(&mut tree, &mut app);
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  snapshot
    .glyphs
    .iter()
    .map(|glyph| (glyph.y + glyph.height) / SCALE)
    .fold(0.0, f32::max)
}

#[test]
fn text_input_scrolled_after_a_clipped_first_paint_paints_its_last_line() {
  // The first paint's clip ends at every offset across one line box, so one
  // of them lands just below a line's top, where that line is the last one
  // shaped for the clip and no line is skipped.
  for step in 0..=(LINE_HEIGHT as usize * 2) {
    let viewport = 200.0 + step as f32 * 0.5;
    let lowest = lowest_glyph_after_scrolling(viewport);
    assert!(
      lowest > viewport - 2.0 * LINE_HEIGHT,
      "viewport {viewport}: the lowest glyph ends at {lowest}, the last line is not painted"
    );
  }
}
