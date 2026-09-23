//! Letter spacing through measurement, wrapping, caching and DPI-scaled paint.
//! The weight-probe Regular face gives exact advances at 10px: "a" is 5px and
//! the space 2.5px. cosmic-text adds the spacing after every glyph, including
//! the last one on a line and spaces, as CSS `letter-spacing` does.

use lurq::{
  app::{App, Tree, theme::TypographyStyle},
  components::Text,
  layout::{Constraints, Size, text_style::TextStyle},
};

use crate::support::{TestSurface, render_pass_with_app};

const FAMILY: &str = "Lurq Weight Probe";
const REGULAR: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-Regular.ttf");
const LINE_HEIGHT: f32 = 12.0;

fn probe_app() -> App {
  let mut app = App::new();
  app.install_fonts([REGULAR.to_vec()], std::iter::empty::<(&str, &str)>());
  app
}

fn probe_style(letter_spacing: f32) -> TextStyle {
  TextStyle {
    font_family: FAMILY.into(),
    font_size: 10.0,
    letter_spacing,
    ..TextStyle::default()
  }
}

fn layout_size(app: &mut App, tree: &mut Tree, text: Text, max_width: f32) -> Size {
  tree.set_root(text);
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(max_width, 200.0))));
  tree.request_redraw();
  tree.pass(app, &TestSurface);
  tree.last_layout().expect("layout").size
}

fn measure(app: &mut App, text: &str, letter_spacing: f32, max_width: f32) -> Size {
  layout_size(
    app,
    &mut Tree::new(),
    Text::styled(text, probe_style(letter_spacing)),
    max_width,
  )
}

fn assert_close(actual: f32, expected: f32, what: &str) {
  assert!(
    (actual - expected).abs() < 0.01,
    "{what}: {actual}, expected {expected}"
  );
}

#[test]
fn letter_spacing_adds_its_width_after_every_glyph_including_the_last() {
  let mut app = probe_app();
  assert_close(measure(&mut app, "aaaa", 0.0, 400.0).width, 20.0, "unspaced");
  assert_close(measure(&mut app, "aaaa", 1.5, 400.0).width, 26.0, "4 glyphs x 1.5px");
  assert_close(
    measure(&mut app, "a", 3.0, 400.0).width,
    8.0,
    "a single glyph keeps its trailing spacing",
  );
  assert_close(
    measure(&mut app, "aa aa", 1.0, 400.0).width,
    27.5,
    "spaces are spaced too",
  );
  assert_close(measure(&mut app, "aaaa", 0.0, 400.0).height, LINE_HEIGHT, "height");
  assert_close(
    measure(&mut app, "aaaa", 4.0, 400.0).height,
    LINE_HEIGHT,
    "spacing leaves height alone",
  );
}

#[test]
fn negative_letter_spacing_tightens_text() {
  let mut app = probe_app();
  assert_close(measure(&mut app, "aaaa", -1.0, 400.0).width, 16.0, "4 glyphs x -1px");
  assert_close(measure(&mut app, "aaaa", -0.5, 400.0).width, 18.0, "4 glyphs x -0.5px");
}

#[test]
fn wrapping_uses_the_spaced_advances() {
  let mut app = probe_app();
  // "aaaa aaaa" is 42.5px unspaced and 51.5px with 1px spacing.
  let unspaced = measure(&mut app, "aaaa aaaa", 0.0, 45.0);
  assert_close(unspaced.height, LINE_HEIGHT, "unspaced text fits one line");
  let spaced = measure(&mut app, "aaaa aaaa", 1.0, 45.0);
  assert_close(spaced.height, 2.0 * LINE_HEIGHT, "spaced text wraps");
  assert_close(spaced.width, 24.0, "each wrapped row is four spaced glyphs");

  // Negative spacing brings it back under a width the unspaced text overflows.
  assert_close(
    measure(&mut app, "aaaa aaaa", 0.0, 40.0).height,
    2.0 * LINE_HEIGHT,
    "unspaced wraps",
  );
  let tightened = measure(&mut app, "aaaa aaaa", -0.5, 40.0);
  assert_close(tightened.height, LINE_HEIGHT, "tightened text fits one line");
  assert_close(tightened.width, 38.0, "9 glyphs x -0.5px");
}

#[test]
fn changing_only_letter_spacing_relayouts_the_same_text() {
  let mut app = probe_app();
  let mut tree = Tree::new();
  for (letter_spacing, width) in [(0.0, 20.0), (2.0, 28.0), (0.0, 20.0), (-1.0, 16.0), (2.0, 28.0)] {
    let text = Text::styled("aaaa", probe_style(letter_spacing)).nowrap();
    assert_close(
      layout_size(&mut app, &mut tree, text, 400.0).width,
      width,
      &format!("spacing {letter_spacing}"),
    );
  }
  // Wrapped layouts are cached per width too.
  for (letter_spacing, height) in [(0.0, LINE_HEIGHT), (1.0, 2.0 * LINE_HEIGHT), (0.0, LINE_HEIGHT)] {
    let text = Text::styled("aaaa aaaa", probe_style(letter_spacing));
    assert_close(
      layout_size(&mut app, &mut tree, text, 45.0).height,
      height,
      &format!("wrapped spacing {letter_spacing}"),
    );
  }
}

#[test]
fn text_letter_spacing_overrides_explicit_and_theme_styles() {
  let mut app = probe_app();
  let mut tree = Tree::new();
  let explicit = Text::styled("aaaa", probe_style(0.0)).letter_spacing(2.0);
  assert_close(
    layout_size(&mut app, &mut tree, explicit, 400.0).width,
    28.0,
    "explicit",
  );

  app
    .theme()
    .set_typography_style(TypographyStyle::Heading, probe_style(-1.0));
  let themed = Text::new("aaaa").variant(TypographyStyle::Heading);
  assert_close(
    layout_size(&mut app, &mut tree, themed, 400.0).width,
    16.0,
    "theme role",
  );
  let overridden = Text::new("aaaa").variant(TypographyStyle::Heading).letter_spacing(-0.5);
  assert_close(
    layout_size(&mut app, &mut tree, overridden, 400.0).width,
    18.0,
    "override",
  );
}

#[test]
fn letter_spacing_scales_with_the_display_scale_factor() {
  for scale in [1.0_f32, 1.5, 2.0] {
    let mut app = probe_app();
    let mut tree = Tree::new();
    tree.set_scale_factor(scale);
    tree.resize((400.0 * scale) as u32, (100.0 * scale) as u32);
    tree.set_root(Text::styled("aaaa", probe_style(1.0)).nowrap());
    tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
    let snapshot = render_pass_with_app(&mut tree, &mut app);
    assert_close(tree.last_layout().expect("layout").size.width, 24.0, "logical width");

    let mut xs: Vec<f32> = snapshot.glyphs.iter().map(|glyph| glyph.x).collect();
    xs.sort_by(f32::total_cmp);
    assert_eq!(xs.len(), 4, "one glyph per letter at scale {scale}");
    for pair in xs.windows(2) {
      // Physical advance = (5px + 1px) x scale, a whole number of pixels here.
      assert!(
        (pair[1] - pair[0] - 6.0 * scale).abs() < 0.01,
        "scale {scale}: glyph step {} expected {}",
        pair[1] - pair[0],
        6.0 * scale
      );
    }
    assert!(
      (xs[3] - xs[0] - 18.0 * scale).abs() < 0.01,
      "scale {scale}: spacing must scale with font size ({xs:?})"
    );
  }
}
