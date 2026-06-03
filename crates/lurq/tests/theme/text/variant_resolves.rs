use lurq::{
  app::{App, Tree, theme::TypographyId},
  components::Text,
  layout::{
    Constraints, Size,
    quad::QuadContent,
    text_style::{FontWeight, TextStyle},
  },
};

use crate::support::TestSurface;

#[test]
fn resolves_text_variant_from_active_theme() {
  const LABEL: TypographyId = TypographyId::new(3);
  let mut app = App::new();
  app.theme().set_typography_style(
    LABEL,
    TextStyle {
      font_size: 18.0,
      weight: FontWeight::Bold,
      ..TextStyle::default()
    },
  );

  let mut tree = Tree::new();
  tree.set_root(Text::new("Label").variant(LABEL));
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
