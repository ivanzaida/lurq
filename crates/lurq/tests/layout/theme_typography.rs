use lurq::{
  app::{
    App, Tree,
    theme::{PaletteColor, RadiusSize, SpacingSize, TypographyStyle},
  },
  components::{Rect, Row, Text},
  layout::{
    Constraints, Size,
    quad::QuadContent,
    text_style::{FontWeight, TextStyle},
  },
  node::{color::Color, dimension::Dimension},
};

use crate::support::TestSurface;

#[test]
fn text_new_uses_default_text_style() {
  let mut app = App::new();
  app.theme().set_default_text_style(TextStyle {
    font_size: 21.0,
    ..TextStyle::default()
  });

  let mut tree = Tree::new();
  tree.set_root(Text::new("body text"));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let text = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");
  assert_eq!(text.font_size, 21.0);
}

#[test]
fn text_variant_uses_named_typography_style() {
  let mut app = App::new();
  app.theme().set_typography_style(
    TypographyStyle::Heading,
    TextStyle {
      font_size: 34.0,
      weight: FontWeight::Bold,
      ..TextStyle::default()
    },
  );

  let mut tree = Tree::new();
  tree.set_root(Text::new("headline").variant(TypographyStyle::Heading));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let text = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");
  assert_eq!(text.font_size, 34.0);
  assert!(text.weight == FontWeight::Bold);
}

#[test]
fn styled_text_ignores_theme_typography() {
  let mut app = App::new();
  app.theme().set_default_text_style(TextStyle {
    font_size: 24.0,
    ..TextStyle::default()
  });

  let mut tree = Tree::new();
  tree.set_root(Text::styled(
    "fixed",
    TextStyle {
      font_size: 13.0,
      ..TextStyle::default()
    },
  ));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let text = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");
  assert_eq!(text.font_size, 13.0);
}

#[test]
fn typography_change_recalculates_text_layout() {
  let mut app = App::new();
  app.theme().set_default_text_style(TextStyle {
    font_size: 12.0,
    ..TextStyle::default()
  });

  let mut tree = Tree::new();
  tree.set_root(Text::new("body text"));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.pass(&mut app, &TestSurface);
  let small_height = tree.last_layout().unwrap().size.height;

  app.theme().set_default_text_style(TextStyle {
    font_size: 30.0,
    ..TextStyle::default()
  });
  tree.pass(&mut app, &TestSurface);
  let large_height = tree.last_layout().unwrap().size.height;

  assert!(large_height > small_height);
}

#[test]
fn background_resolves_palette_color() {
  let mut app = App::new();
  app
    .theme()
    .set_palette_color(PaletteColor::Accent, Color::from_hex("#123456"));

  let mut tree = Tree::new();
  tree.set_root(Rect::new(40.0, 20.0).background(PaletteColor::Accent));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let color = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Rect { color, .. } => Some(*color),
      _ => None,
    })
    .expect("rect quad should be emitted");
  assert_eq!(color.to_hex(), "#123456");
}

#[test]
fn row_spacing_resolves_theme_spacing() {
  let mut app = App::new();
  app.theme().set_spacing_value(SpacingSize::Md, 12.0);

  let mut tree = Tree::new();
  tree.set_root(
    Row::new()
      .spacing(SpacingSize::Md)
      .child(Rect::new(10.0, 10.0).background("#111111"))
      .child(Rect::new(10.0, 10.0).background("#222222")),
  );
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let mut rects = quads
    .iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { .. }))
    .collect::<Vec<_>>();
  rects.sort_by(|a, b| a.x.total_cmp(&b.x));

  assert_eq!(rects.len(), 2);
  assert_eq!(rects[1].x - rects[0].x - rects[0].width, 12.0);
}

#[test]
fn padding_resolves_theme_spacing() {
  let mut app = App::new();
  app.theme().set_spacing_value(SpacingSize::Lg, Dimension::Px(10.0));

  let mut tree = Tree::new();
  tree.set_root(
    lurq::components::Stack::new()
      .size(20.0, 10.0)
      .padding(SpacingSize::Lg)
      .child(
        lurq::components::Spacer::new()
          .width(Dimension::Pct(100.0))
          .height(Dimension::Pct(100.0)),
      ),
  );
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let layout = tree.last_layout().unwrap();
  assert_eq!(layout.size.width, 20.0);
  assert_eq!(layout.size.height, 10.0);
  let inner = &layout.children[0];
  assert_eq!(inner.offset.x, 10.0);
  assert_eq!(inner.offset.y, 10.0);
  assert_eq!(inner.result.size.width, 0.0);
  assert_eq!(inner.result.size.height, 0.0);
}

#[test]
fn radius_resolves_theme_radius() {
  let mut app = App::new();
  app.theme().set_radius_value(RadiusSize::Lg, 8.0);

  let mut tree = Tree::new();
  tree.set_root(Rect::new(30.0, 20.0).background("#123456").rounded(RadiusSize::Lg));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let radius = quads
    .iter()
    .find_map(|quad| quad.border_radius)
    .expect("rect quad should include border radius");
  assert_eq!(radius.top_left, 8.0);
  assert_eq!(radius.top_right, 8.0);
  assert_eq!(radius.bottom_right, 8.0);
  assert_eq!(radius.bottom_left, 8.0);
}
