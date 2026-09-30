//! Face selection for CSS font weights. The probe faces draw "a" with an advance
//! of `weight / 2 + 300` units per 1000-unit em, so the measured width of
//! "aaaa" at 10px names the face used: 400 → 20, 500 → 22, 600 → 24, 700 → 26.

use std::hash::{BuildHasher, RandomState};

use lurq::{
  app::{App, Tree},
  components::Text,
  layout::{
    Constraints, Size,
    text_style::{FontWeight, TextStyle},
  },
};

use crate::support::TestSurface;

const FAMILY: &str = "Lurq Weight Probe";
const REGULAR: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-Medium.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-SemiBold.ttf");
const BOLD: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-Bold.ttf");

fn app_with_faces(faces: &[&[u8]]) -> App {
  let mut app = App::new();
  app.install_fonts(
    faces.iter().map(|face| face.to_vec()),
    std::iter::empty::<(&str, &str)>(),
  );
  app
}

fn probe_width(app: &mut App, weight: FontWeight) -> f32 {
  let style = TextStyle {
    font_family: FAMILY.into(),
    font_size: 10.0,
    weight,
    ..TextStyle::default()
  };
  let mut tree = Tree::new();
  tree.set_root(Text::styled("aaaa", style).nowrap());
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 100.0))));
  tree.pass(app, &TestSurface);
  tree.last_layout().expect("layout").size.width
}

fn assert_selects(app: &mut App, weight: FontWeight, face_weight: u16) {
  let expected = 4.0 * (f32::from(face_weight) / 2.0 + 300.0) / 100.0;
  let width = probe_width(app, weight);
  assert!(
    (width - expected).abs() < 0.01,
    "{weight:?} measured {width}, expected the {face_weight} face ({expected})"
  );
}

#[test]
fn font_weight_selects_loaded_medium_and_semibold_faces() {
  let mut app = app_with_faces(&[REGULAR, MEDIUM, SEMIBOLD, BOLD]);
  assert_selects(&mut app, FontWeight::Normal, 400);
  assert_selects(&mut app, FontWeight::Medium, 500);
  assert_selects(&mut app, FontWeight::SemiBold, 600);
  assert_selects(&mut app, FontWeight::Bold, 700);
}

#[test]
fn font_weight_numeric_weights_select_the_nearest_face_by_css_rules() {
  let mut app = app_with_faces(&[REGULAR, MEDIUM, SEMIBOLD, BOLD]);
  assert_selects(&mut app, FontWeight::Numeric(600), 600);
  // fontdb splits 400..=500 at 450: lower requests try 500 next, higher ones 400.
  assert_selects(&mut app, FontWeight::Numeric(420), 500);
  assert_selects(&mut app, FontWeight::Numeric(450), 400);
  // Above 500 heavier faces come first.
  assert_selects(&mut app, FontWeight::Numeric(550), 600);
  assert_selects(&mut app, FontWeight::Numeric(650), 700);
  // Below 400 looks lighter first, then heavier; past the heaviest face it looks lighter.
  assert_selects(&mut app, FontWeight::Thin, 400);
  assert_selects(&mut app, FontWeight::Black, 700);
  assert_selects(&mut app, FontWeight::Numeric(5000), 700);
}

#[test]
fn font_weight_falls_back_to_nearest_face_when_family_lacks_the_weight() {
  let mut app = app_with_faces(&[REGULAR, BOLD]);
  assert_selects(&mut app, FontWeight::Medium, 400);
  assert_selects(&mut app, FontWeight::Numeric(450), 400);
  assert_selects(&mut app, FontWeight::SemiBold, 700);
  assert_selects(&mut app, FontWeight::ExtraBold, 700);
}

#[test]
fn font_weight_loading_a_face_invalidates_the_resolved_weight() {
  let mut app = app_with_faces(&[REGULAR, BOLD]);
  assert_selects(&mut app, FontWeight::SemiBold, 700);
  app.load_font(SEMIBOLD.to_vec());
  assert_selects(&mut app, FontWeight::SemiBold, 600);
}

#[test]
fn font_weight_compares_and_hashes_by_numeric_value() {
  let hasher = RandomState::new();
  for (named, value) in [
    (FontWeight::Thin, 100),
    (FontWeight::ExtraLight, 200),
    (FontWeight::Light, 300),
    (FontWeight::Normal, 400),
    (FontWeight::Medium, 500),
    (FontWeight::SemiBold, 600),
    (FontWeight::Bold, 700),
    (FontWeight::ExtraBold, 800),
    (FontWeight::Black, 900),
  ] {
    assert_eq!(named.value(), value);
    assert_eq!(named, FontWeight::Numeric(value));
    assert_eq!(named, FontWeight::from(value));
    assert_eq!(hasher.hash_one(named), hasher.hash_one(FontWeight::Numeric(value)));
  }
  assert_eq!(FontWeight::Numeric(0).value(), 1);
  assert_eq!(FontWeight::Numeric(1001).value(), 1000);
  assert_ne!(FontWeight::Numeric(550), FontWeight::SemiBold);
  assert_eq!(FontWeight::default(), FontWeight::Normal);
}
