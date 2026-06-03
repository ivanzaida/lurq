use lurq::{
  app::{App, Tree, theme::PaletteId},
  components::Rect,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::TestSurface;

#[test]
fn resolves_palette_token_background_from_active_theme() {
  const SURFACE: PaletteId = PaletteId::new(5);
  let mut app = App::new();
  app.theme().set_palette_color(SURFACE, Color::from_hex("#123456"));

  let mut tree = Tree::new();
  tree.set_root(Rect::new(40.0, 20.0).background(SURFACE));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let color = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Rect { color } => Some(*color),
      _ => None,
    })
    .expect("rect quad should be emitted");

  assert_eq!(color.to_hex(), "#123456");
}

#[test]
fn resolves_palette_token_border_from_active_theme() {
  const BORDER: PaletteId = PaletteId::new(6);
  let mut app = App::new();
  app.theme().set_palette_color(BORDER, Color::from_hex("#8b5cf6"));

  let mut tree = Tree::new();
  tree.set_root(Rect::new(40.0, 20.0).border_inside(2.0, BORDER));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let border = quads
    .iter()
    .find_map(|quad| quad.border)
    .expect("rect border should be emitted");

  assert_eq!(border.top.unwrap().color.to_hex(), "#8b5cf6");
}
