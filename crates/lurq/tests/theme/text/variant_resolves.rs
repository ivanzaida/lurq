use lurq::{
  app::{
    App, Tree,
    theme::{PaletteColor, TypographyStyle},
  },
  components::Text,
  layout::{
    Constraints, Size,
    quad::QuadContent,
    text_style::{FontWeight, TextStyle},
  },
  node::color::Color,
};

use crate::support::TestSurface;

#[test]
fn resolves_text_variant_from_active_theme() {
  let mut app = App::new();
  app.theme().set_typography_style(
    TypographyStyle::Label,
    TextStyle {
      font_size: 18.0,
      weight: FontWeight::Bold,
      ..TextStyle::default()
    },
  );

  let mut tree = Tree::new();
  tree.set_root(Text::new("Label").variant(TypographyStyle::Label));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 80.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let style = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");

  assert_eq!(style.font_size, 18.0);
  assert!(style.weight == FontWeight::Bold);
}

#[test]
fn text_color_accepts_concrete_color() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Text::new("Label").color(Color::from_hex("#22c55e")));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 80.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let style = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");

  assert_eq!(style.color.to_hex(), "#22c55e");
}

#[test]
fn text_color_accepts_palette_color() {
  let mut app = App::new();
  app
    .theme()
    .set_palette_color(PaletteColor::Accent, Color::from_hex("#123456"));

  let mut tree = Tree::new();
  tree.set_root(Text::new("Label").color(PaletteColor::Accent));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 80.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let style = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");

  assert_eq!(style.color.to_hex(), "#123456");
}

#[test]
fn text_color_accepts_extra_palette_color() {
  let mut app = App::new();
  app.theme().set_palette_color("brand_text", Color::from_hex("#123456"));

  let mut tree = Tree::new();
  tree.set_root(Text::new("Label").color(PaletteColor::extra("brand_text")));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 80.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let style = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");

  assert_eq!(style.color.to_hex(), "#123456");
}

#[test]
fn text_color_and_variant_compose_in_either_order() {
  let mut app = App::new();
  app
    .theme()
    .set_palette_color(PaletteColor::Accent, Color::from_hex("#123456"));
  app.theme().set_typography_style(
    TypographyStyle::Label,
    TextStyle {
      font_size: 18.0,
      color: Color::from_hex("#abcdef"),
      ..TextStyle::default()
    },
  );

  let mut tree = Tree::new();
  tree.set_root(
    Text::new("Label")
      .color(PaletteColor::Accent)
      .variant(TypographyStyle::Label),
  );
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 80.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let style = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style),
      _ => None,
    })
    .expect("text quad should be emitted");

  assert_eq!(style.font_size, 18.0);
  assert_eq!(style.color.to_hex(), "#123456");
}
