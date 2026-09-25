//! OpenType feature settings through shaping, measurement, caching, paint and
//! theme roles. The ligature-probe face is monospaced (5px cells at 10px) and
//! its `liga` feature shapes "--" into one cell-wide glyph drawn over the
//! preceding cell, like Geist Mono's programming ligatures: by default
//! "a --a" loses a cell and the dash covers the space.

use lurq::{
  app::{App, Tree, theme::TypographyStyle},
  components::{Text, TextInput},
  core::Signal,
  layout::{
    Constraints, Size,
    text_style::{FontFeature, FontFeatures, TextStyle},
  },
};

use crate::support::{GlyphSnapshot, render_pass_with_app};

const FAMILY: &str = "Lurq Ligature Probe";
const PROBE: &[u8] = include_bytes!("../assets/ligature_probe/LurqLigatureProbe-Regular.ttf");
const CELL: f32 = 5.0;
const TEXT: &str = "a --a";
const ATLAS_PADDING: f32 = 2.0;

fn probe_app() -> App {
  let mut app = App::new();
  app.install_fonts([PROBE.to_vec()], std::iter::empty::<(&str, &str)>());
  app
}

fn no_ligatures() -> FontFeatures {
  FontFeatures::new([FontFeature::disable(*b"liga"), FontFeature::disable(*b"calt")])
}

fn probe_style(font_features: FontFeatures) -> TextStyle {
  TextStyle {
    font_family: FAMILY.into(),
    font_size: 10.0,
    font_features,
    ..TextStyle::default()
  }
}

fn layout_width(app: &mut App, tree: &mut Tree, text: Text) -> f32 {
  tree.set_root(text.nowrap());
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.request_redraw();
  tree.pass(app, &crate::support::TestSurface);
  tree.last_layout().expect("layout").size.width
}

fn painted_glyphs(app: &mut App, root: impl Into<lurq::node::Element>) -> Vec<GlyphSnapshot> {
  let mut tree = Tree::new();
  tree.resize(400, 100);
  tree.set_root(root);
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  let mut glyphs = render_pass_with_app(&mut tree, app).glyphs;
  glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
  glyphs
}

/// Left edge of a glyph's pixel-snapped ink: painted glyph quads carry the
/// atlas padding on each side.
fn ink_left(glyph: &GlyphSnapshot) -> f32 {
  glyph.x + ATLAS_PADDING
}

fn assert_close(actual: f32, expected: f32, what: &str) {
  assert!(
    (actual - expected).abs() < 0.01,
    "{what}: {actual}, expected {expected}"
  );
}

#[test]
fn default_features_let_the_ligature_draw_over_the_space() {
  let mut app = probe_app();
  let width = layout_width(
    &mut app,
    &mut Tree::new(),
    Text::styled(TEXT, probe_style(FontFeatures::default())),
  );
  assert_close(width, 4.0 * CELL, "\"--\" shapes into a single cell");

  let glyphs = painted_glyphs(
    &mut app,
    Text::styled(TEXT, probe_style(FontFeatures::default())).nowrap(),
  );
  assert_eq!(glyphs.len(), 3, "a, the ligature and a: {glyphs:?}");
  // The ligature advances from the first hyphen's cell but its ink starts
  // half a pixel into the space's cell (-4.5px from that origin).
  assert_close(ink_left(&glyphs[1]), CELL, "ligature ink");
}

#[test]
fn disabled_ligatures_keep_the_space_and_every_hyphen() {
  let mut app = probe_app();
  let width = layout_width(
    &mut app,
    &mut Tree::new(),
    Text::styled(TEXT, probe_style(no_ligatures())),
  );
  assert_close(width, 5.0 * CELL, "one cell per character");

  let glyphs = painted_glyphs(&mut app, Text::styled(TEXT, probe_style(no_ligatures())).nowrap());
  assert_eq!(glyphs.len(), 4, "a, -, - and a: {glyphs:?}");
  // Each hyphen's ink sits 1px into its own cell, after the space's cell.
  assert_close(ink_left(&glyphs[1]), 2.0 * CELL + 1.0, "first hyphen");
  assert_close(ink_left(&glyphs[2]), 3.0 * CELL + 1.0, "second hyphen");
}

#[test]
fn feature_settings_are_canonical_and_the_last_setting_wins() {
  let liga_then_calt = FontFeatures::new([FontFeature::disable(*b"liga"), FontFeature::disable(*b"calt")]);
  let calt_then_liga = FontFeatures::new([FontFeature::disable(*b"calt"), FontFeature::disable(*b"liga")]);
  assert_eq!(liga_then_calt, calt_then_liga);
  assert_eq!(
    liga_then_calt.iter().map(|feature| feature.tag()).collect::<Vec<_>>(),
    [*b"calt", *b"liga"]
  );
  assert!(FontFeatures::new([]).is_empty());
  assert_eq!(FontFeatures::new([]), FontFeatures::default());

  let reenabled = FontFeatures::new([FontFeature::disable(*b"liga"), FontFeature::enable(*b"liga")]);
  assert_eq!(reenabled, FontFeatures::from([FontFeature::new(*b"liga", 1)]));
  let mut app = probe_app();
  assert_close(
    layout_width(&mut app, &mut Tree::new(), Text::styled(TEXT, probe_style(reenabled))),
    4.0 * CELL,
    "re-enabled liga",
  );
}

#[test]
fn changing_only_font_features_relayouts_the_same_text() {
  let mut app = probe_app();
  let mut tree = Tree::new();
  for (features, width) in [
    (FontFeatures::default(), 4.0 * CELL),
    (no_ligatures(), 5.0 * CELL),
    (FontFeatures::default(), 4.0 * CELL),
    (FontFeatures::from([FontFeature::disable(*b"liga")]), 5.0 * CELL),
  ] {
    let text = Text::styled(TEXT, probe_style(features.clone()));
    assert_close(layout_width(&mut app, &mut tree, text), width, &format!("{features:?}"));
  }
}

#[test]
fn typography_roles_carry_font_features_and_text_overrides_them() {
  let mut app = probe_app();
  app
    .theme()
    .set_typography_style(TypographyStyle::Mono, probe_style(no_ligatures()));
  assert_eq!(
    app.theme().typography_style(TypographyStyle::Mono).font_features,
    no_ligatures(),
    "role round-trip"
  );

  let mut tree = Tree::new();
  let themed = Text::new(TEXT).variant(TypographyStyle::Mono);
  assert_close(layout_width(&mut app, &mut tree, themed), 5.0 * CELL, "theme role");
  let overridden = Text::new(TEXT)
    .variant(TypographyStyle::Mono)
    .font_features(FontFeatures::default());
  assert_close(layout_width(&mut app, &mut tree, overridden), 4.0 * CELL, "override");
  let explicit = Text::styled(TEXT, probe_style(FontFeatures::default())).font_features(no_ligatures());
  assert_close(layout_width(&mut app, &mut tree, explicit), 5.0 * CELL, "explicit");
}

#[test]
fn text_input_font_features_shape_the_value() {
  let mut app = probe_app();
  let input = |features: Option<FontFeatures>| {
    let input = TextInput::styled(Signal::new(TEXT.to_owned()), probe_style(FontFeatures::default())).width(200.0);
    match features {
      Some(features) => input.font_features(features),
      None => input,
    }
  };
  assert_eq!(painted_glyphs(&mut app, input(None)).len(), 3, "ligature by default");
  assert_eq!(
    painted_glyphs(&mut app, input(Some(no_ligatures()))).len(),
    4,
    "no ligature with liga off"
  );
}
