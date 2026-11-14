//! A wrapped text is painted on the lines it was laid out on. Paint shapes
//! again at the window's scale, where the type's widths are not the layout's
//! times the scale: a text near a break can fit one line fewer when painted,
//! and must still start at the top of its box, where the layout put it.

use lurq::{
  app::Tree,
  components::{Column, Row, Text},
  layout::text_style::TextStyle,
  node::dimension::Dimension,
};

use crate::support::render_pass;

/// The top of the first glyph painted left of `right`, and right of `left`.
fn first_glyph_top(snapshot: &crate::support::RenderSnapshot, left: f32, right: f32) -> Option<f32> {
  snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.x >= left && glyph.x < right)
    .min_by(|a, b| (a.y, a.x).partial_cmp(&(b.y, b.x)).unwrap())
    .map(|glyph| glyph.y)
}

#[test]
fn a_wrapped_text_starts_at_the_top_of_its_box_at_a_fractional_scale_at_every_width() {
  let style = TextStyle {
    font_size: 12.0,
    line_height: 1.2,
    ..TextStyle::default()
  };
  let mut wrong = Vec::new();
  for step in 0..=120 {
    let width = 40.0 + step as f32 * 0.5;
    let mut tree = Tree::new();
    tree.set_scale_factor(1.5);
    tree.set_root(
      Row::new()
        .child(
          Column::new()
            .width(width)
            .child(Text::styled("Review and confirm", style.clone()).width(Dimension::Pct(100.0))),
        )
        .child(
          Column::new()
            .width(200.0)
            .padding_left(100.0)
            .child(Text::styled("Review", style.clone())),
        ),
    );
    let snapshot = render_pass(&mut tree);
    // The physical start of the text that wraps and of the one-word text beside it, both at the top of a box at 0.
    let wrapped = first_glyph_top(&snapshot, 0.0, width * 1.5);
    let single = first_glyph_top(&snapshot, (width + 100.0) * 1.5 - 1.0, f32::MAX);
    if let (Some(wrapped), Some(single)) = (wrapped, single) {
      if (wrapped - single).abs() > 1.0 {
        wrong.push((width, wrapped, single));
      }
    } else {
      panic!("both texts are painted at {width}");
    }
  }
  assert!(
    wrong.is_empty(),
    "widths whose text starts lower than its box: {wrong:?}"
  );
}
