use lurq::{
  app::{
    App, Tree,
    theme::{BorderSize, PaletteColor, RadiusSize, SpacingSize, TypographyStyle},
  },
  components::{Column, Rect, Text},
  layout::{
    Constraints, Size,
    quad::QuadContent,
    text_style::{FontWeight, TextStyle},
  },
  node::padding::Padding,
};

use crate::support::TestSurface;

fn pass(app: &mut App, tree: &mut Tree) {
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 200.0))));
  tree.pass(app, &TestSurface);
}

fn text_style(tree: &Tree) -> TextStyle {
  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { style, .. } => Some(style.clone()),
      _ => None,
    })
    .expect("text quad should be emitted")
}

#[test]
fn text_variant_resolves_extra_typography_style() {
  let mut app = App::new();
  app.theme().set_typography_style(
    "overline",
    TextStyle {
      font_size: 9.0,
      weight: FontWeight::SemiBold,
      ..TextStyle::default()
    },
  );

  let mut tree = Tree::new();
  tree.set_root(Text::new("OVERLINE").variant("overline"));
  pass(&mut app, &mut tree);

  let style = text_style(&tree);
  assert_eq!(style.font_size, 9.0);
  assert_eq!(style.weight, FontWeight::SemiBold);
  assert_eq!(
    app
      .theme()
      .typography_style(TypographyStyle::extra("overline"))
      .font_size,
    9.0
  );
}

#[test]
fn text_variant_with_missing_extra_falls_back_to_default_style() {
  let mut app = App::new();
  app.theme().set_default_text_style(TextStyle {
    font_size: 21.0,
    ..TextStyle::default()
  });

  let mut tree = Tree::new();
  tree.set_root(Text::new("fallback").variant(TypographyStyle::extra("missing")));
  pass(&mut app, &mut tree);

  assert_eq!(text_style(&tree).font_size, 21.0);
}

#[test]
fn rect_resolves_extra_radius_and_border_size() {
  let mut app = App::new();
  app.theme().set_radius_value("card", 7.0);
  app.theme().set_border_size_value("hairline", 0.5);

  let mut tree = Tree::new();
  tree.set_root(
    Rect::new(40.0, 20.0)
      .background(PaletteColor::SurfacePanel)
      .rounded(RadiusSize::extra("card"))
      .border_inside(BorderSize::extra("hairline"), PaletteColor::Border),
  );
  pass(&mut app, &mut tree);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Rect { .. }))
    .expect("rect quad should be emitted");
  assert_eq!(quad.border_radius.expect("rounded").top_left, 7.0);
  assert_eq!(quad.border.expect("bordered").top.unwrap().width, 0.5);
}

#[test]
fn rect_with_missing_extra_radius_and_border_size_is_square_and_borderless() {
  let mut app = App::new();

  let mut tree = Tree::new();
  tree.set_root(
    Rect::new(40.0, 20.0)
      .background(PaletteColor::SurfacePanel)
      .rounded(RadiusSize::extra("missing"))
      .border_inside(BorderSize::extra("missing"), PaletteColor::Border),
  );
  pass(&mut app, &mut tree);

  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Rect { .. }))
    .expect("rect quad should be emitted");
  assert!(quad.border_radius.is_none_or(|radius| radius.top_left == 0.0));
  assert!(
    quad
      .border
      .and_then(|border| border.top)
      .is_none_or(|top| top.width == 0.0)
  );
}

#[test]
fn padding_resolves_extra_spacing_and_missing_extra_as_zero() {
  let mut app = App::new();
  app.theme().set_spacing_value("gutter", 12.0);

  for (name, inset) in [("gutter", 12.0), ("missing", 0.0)] {
    let mut tree = Tree::new();
    tree.set_root(
      Column::new()
        .padding(Padding::all(SpacingSize::extra(name)))
        .child(Rect::new(10.0, 10.0)),
    );
    pass(&mut app, &mut tree);
    let child = &tree.last_layout().unwrap().children[0];
    assert_eq!((child.offset.x, child.offset.y), (inset, inset), "{name}");
  }
}
