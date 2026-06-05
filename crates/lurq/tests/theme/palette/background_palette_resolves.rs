use lurq::{
  app::{
    App, Tree,
    theme::{BorderSize, PaletteColor},
  },
  components::Rect,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::TestSurface;

#[test]
fn resolves_palette_background_from_active_theme() {
  let mut app = App::new();
  app
    .theme()
    .set_palette_color(PaletteColor::SurfacePanel, Color::from_hex("#123456"));

  let mut tree = Tree::new();
  tree.set_root(Rect::new(40.0, 20.0).background(PaletteColor::SurfacePanel));
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
fn resolves_palette_border_from_active_theme() {
  let mut app = App::new();
  app
    .theme()
    .set_palette_color(PaletteColor::BorderFocus, Color::from_hex("#8b5cf6"));

  let mut tree = Tree::new();
  tree.set_root(Rect::new(40.0, 20.0).border_inside(2.0, PaletteColor::BorderFocus));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let border = quads
    .iter()
    .find_map(|quad| quad.border)
    .expect("rect border should be emitted");

  assert_eq!(border.top.unwrap().color.to_hex(), "#8b5cf6");
}

#[test]
fn resolves_theme_border_size_from_active_theme() {
  let mut app = App::new();
  app.theme().set_border_size_value(BorderSize::Lg, 5.0);

  let mut tree = Tree::new();
  tree.set_root(Rect::new(40.0, 20.0).border_inside(BorderSize::Lg, PaletteColor::BorderFocus));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(100.0, 100.0))));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let border = quads
    .iter()
    .find_map(|quad| quad.border)
    .expect("rect border should be emitted");

  assert_eq!(border.top.unwrap().width, 5.0);
}
