use lurq::{app::Tree, components::Text, layout::text_style::TextShadow, node::color::Color};

use crate::support::render_pass;

#[test]
fn text_shadow_emits_offset_copies_beneath_glyphs() {
  let shadow = TextShadow::new(2.0, 3.0, 4.0, Color::new(255, 0, 0, 255));
  let mut runtime = Tree::new();
  runtime.set_root(Text::new("Hi").text_shadow(shadow));

  let snapshot = render_pass(&mut runtime);

  assert!(snapshot.glyph_count > 0);
  assert_eq!(snapshot.glyph_count % 2, 0, "each glyph should gain a shadow copy");
  let half = snapshot.glyph_count / 2;
  let shadow_color = Color::new(255, 0, 0, 255).to_linear_f32_array();
  for (shadow_glyph, text_glyph) in snapshot.glyphs[..half].iter().zip(&snapshot.glyphs[half..]) {
    assert_eq!(shadow_glyph.x, text_glyph.x + 2.0);
    assert_eq!(shadow_glyph.y, text_glyph.y + 3.0);
    assert_eq!(shadow_glyph.color, shadow_color);
    // CSS blur radius maps to sigma = radius / 2.
    assert_eq!(shadow_glyph.shadow_sigma, 2.0);
    assert_eq!(text_glyph.shadow_sigma, 0.0);
  }
}

#[test]
fn text_without_shadow_emits_no_shadow_instances() {
  let mut runtime = Tree::new();
  runtime.set_root(Text::new("Hi"));

  let snapshot = render_pass(&mut runtime);

  assert!(snapshot.glyph_count > 0);
  assert!(snapshot.glyphs.iter().all(|glyph| glyph.shadow_sigma == 0.0));
}

#[test]
fn invisible_shadow_is_skipped() {
  let mut plain = Tree::new();
  plain.set_root(Text::new("Hi"));
  let plain_count = render_pass(&mut plain).glyph_count;

  let shadow = TextShadow::new(0.0, 0.0, 0.0, Color::new(255, 0, 0, 255));
  let mut shadowed = Tree::new();
  shadowed.set_root(Text::new("Hi").text_shadow(shadow));
  let snapshot = render_pass(&mut shadowed);

  assert!(plain_count > 0);
  assert_eq!(snapshot.glyph_count, plain_count);
}
