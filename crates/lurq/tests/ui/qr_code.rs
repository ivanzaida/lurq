//! `ui::QrCode`: the code of a short and a long text at each level reads back to the text, from its modules and
//! from what is painted at 100, 150 and 200%; the modules are on whole device pixels; the board's card, dark on
//! light in either appearance; a logo over the middle; a text no code holds; what tooling reads.

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
    window::Appearance,
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{QrCode, QrCodePart, QrLevel, QrMatrix},
};

use super::stage::Stage;

const LEVELS: [QrLevel; 4] = [QrLevel::Low, QrLevel::Medium, QrLevel::Quartile, QrLevel::High];

const SHORT: &str = "https://lurq.dev";

/// A text of `len` bytes, of every kind of byte a text has: letters, digits, marks and more than one byte to a
/// character.
fn text_of(len: usize) -> String {
  let mut text = String::new();
  let words = ["lurq ", "QR-", "42/", "ключ ", "?a=1&b=2 ", "ĳ "];
  let mut index = 0;
  while text.len() < len {
    text.push_str(words[index % words.len()]);
    index += 1;
  }
  while text.len() > len {
    text.pop();
  }
  // A character of more than one byte cut at the end is made up in letters of one.
  while text.len() < len {
    text.push('x');
  }
  text
}

struct Matrix<'a>(&'a QrMatrix);

impl rqrr::BitGrid for Matrix<'_> {
  fn size(&self) -> usize {
    self.0.size()
  }

  fn bit(&self, y: usize, x: usize) -> bool {
    self.0.dark(x, y)
  }
}

/// The level a decoder reads in a code's format: the standard's two bits.
fn format_bits(level: QrLevel) -> u16 {
  match level {
    QrLevel::Low => 1,
    QrLevel::Medium => 0,
    QrLevel::Quartile => 3,
    QrLevel::High => 2,
  }
}

#[test]
fn the_modules_of_every_size_read_back_to_the_text_at_every_level() {
  for level in LEVELS {
    let mut versions = std::collections::BTreeSet::new();
    // From nothing to the most a level holds, through every size a code has.
    let most = (0..3000)
      .rev()
      .find(|len| QrMatrix::encode(&text_of(*len), level).is_some())
      .unwrap();
    let mut lens = Vec::new();
    let mut len = 0;
    while len < most {
      lens.push(len);
      len += 1 + len / 40;
    }
    lens.push(most);
    for len in lens {
      let text = text_of(len);
      let matrix = QrMatrix::encode(&text, level).unwrap_or_else(|| panic!("{len} bytes at {level:?}"));
      versions.insert(matrix.version());
      assert!(matrix.level() as usize >= level as usize);
      let (meta, read) = rqrr::Grid::new(Matrix(&matrix))
        .decode()
        .unwrap_or_else(|error| panic!("{len} bytes at {level:?}, version {}: {error:?}", matrix.version()));
      assert_eq!(read, text, "{len} bytes at {level:?}");
      assert_eq!(meta.version.0, usize::from(matrix.version()));
      assert_eq!(meta.ecc_level, format_bits(matrix.level()));
      assert_eq!(matrix.size(), usize::from(matrix.version()) * 4 + 17);
    }
    assert!(QrMatrix::encode(&text_of(most + 1), level).is_none());
    assert_eq!(versions.len(), 40, "every version is made at {level:?}: {versions:?}");
  }
  // The capacity of the largest code in bytes, at the lowest level and the highest.
  assert!(QrMatrix::encode(&text_of(2953), QrLevel::Low).is_some());
  assert!(QrMatrix::encode(&text_of(2954), QrLevel::Low).is_none());
  assert!(QrMatrix::encode(&text_of(1273), QrLevel::High).is_some());
  assert!(QrMatrix::encode(&text_of(1274), QrLevel::High).is_none());
}

#[test]
fn a_higher_level_is_taken_where_it_fits_in_the_same_size() {
  let low = QrMatrix::encode("lurq", QrLevel::Low).unwrap();
  assert_eq!(low.version(), 1);
  // Four bytes fit version 1 at the highest level as well.
  assert_eq!(low.level(), QrLevel::High);
  let long = text_of(100);
  let medium = QrMatrix::encode(&long, QrLevel::Medium).unwrap();
  let high = QrMatrix::encode(&long, QrLevel::High).unwrap();
  assert!(high.version() > medium.version());
}

/// The quads painted inside the box of the code with `id`.
fn painted(stage: &mut Stage, id: &str) -> (Vec<Quad>, (f32, f32, f32, f32)) {
  let bounds = stage.bounds(id);
  let (x, y, width, height) = bounds;
  let quads = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(quad.content, QuadContent::Rect { .. })
        && quad.x >= x - 0.5
        && quad.y >= y - 0.5
        && quad.x + quad.width <= x + width + 0.5
        && quad.y + quad.height <= y + height + 0.5
    })
    .collect();
  (quads, bounds)
}

fn luminance(color: Color) -> u8 {
  ((u32::from(color.r()) * 299 + u32::from(color.g()) * 587 + u32::from(color.b()) * 114) / 1000) as u8
}

/// What is painted in the box of the code with `id`, at `scale`, as a grey picture of device pixels with a margin
/// of light around it, read by a decoder.
fn read_painted(stage: &mut Stage, id: &str, scale: f32) -> Option<String> {
  let (quads, (x0, y0, width, height)) = painted(stage, id);
  let margin = 24usize;
  let (w, h) = (
    (width * scale).round() as usize + 2 * margin,
    (height * scale).round() as usize + 2 * margin,
  );
  let mut grey = vec![255u8; w * h];
  for quad in &quads {
    let QuadContent::Rect { color, .. } = quad.content else {
      continue;
    };
    if color.a() < 128 {
      continue;
    }
    let value = luminance(color);
    let left = ((quad.x - x0) * scale).round() as usize + margin;
    let top = ((quad.y - y0) * scale).round() as usize + margin;
    let right = ((quad.x + quad.width - x0) * scale).round() as usize + margin;
    let bottom = ((quad.y + quad.height - y0) * scale).round() as usize + margin;
    for row in top..bottom.min(h) {
      for column in left..right.min(w) {
        grey[row * w + column] = value;
      }
    }
  }
  let mut image = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| grey[y * w + x]);
  let grids = image.detect_grids();
  grids.first().and_then(|grid| grid.decode().ok()).map(|(_, text)| text)
}

/// The fills of the dark modules: the quads in the light palette's foreground.
fn modules(stage: &mut Stage, id: &str) -> Vec<Quad> {
  let dark = ThemePalette::light().get(PaletteColor::Foreground);
  painted(stage, id)
    .0
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == dark))
    .collect()
}

#[test]
fn what_is_painted_reads_back_to_the_text_crisp_at_every_scale() {
  let long = text_of(600);
  for (text, length) in [(SHORT.to_owned(), 160.0), (long, 400.0)] {
    for level in LEVELS {
      // The long text at the highest level is a code of 101 modules: four of a logical pixel each.
      let length = if level == QrLevel::High && length > 160.0 {
        480.0
      } else {
        length
      };
      for scale in [1.0f32, 1.5, 2.0] {
        let shown = text.clone();
        let mut stage = Stage::new(move |_| {
          Column::new()
            .padding(13.3)
            .child(QrCode::new(&shown).level(level).length(length).id("code"))
            .into()
        });
        stage
          .tree
          .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
            600.0, 600.0,
          ))));
        stage.tree.set_scale_factor(scale);
        stage.pass();
        let cells = modules(&mut stage, "code");
        assert!(!cells.is_empty());
        for quad in &cells {
          for edge in [quad.x, quad.y, quad.x + quad.width, quad.y + quad.height] {
            let device = edge * scale;
            assert!(
              (device - device.round()).abs() < 1e-3,
              "a module's edge at {edge} is {device} device pixels at {scale}"
            );
          }
        }
        assert_eq!(
          read_painted(&mut stage, "code", scale).as_deref(),
          Some(text.as_str()),
          "{} bytes at {level:?}, {scale}x",
          text.len()
        );
      }
    }
  }
}

#[test]
fn at_150_percent_the_modules_meet_with_no_seam() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(7.0)
      .child(QrCode::new(SHORT).length(161.0).id("code"))
      .into()
  });
  stage.tree.set_scale_factor(1.5);
  stage.pass();
  let cells = modules(&mut stage, "code");
  // Every module of 5 is 7 or 8 device pixels at 150%, and the boxes of two modules side by side share an edge.
  let lefts: std::collections::BTreeSet<i64> = cells.iter().map(|quad| (quad.x * 1.5).round() as i64).collect();
  let rights: std::collections::BTreeSet<i64> = cells
    .iter()
    .map(|quad| ((quad.x + quad.width) * 1.5).round() as i64)
    .collect();
  let mut steps: Vec<i64> = lefts.union(&rights).copied().collect();
  steps.sort_unstable();
  for pair in steps.windows(2) {
    assert!(
      pair[1] - pair[0] >= 7,
      "edges {pair:?}: modules of 7.5 device pixels are drawn 7 or 8 wide"
    );
  }
}

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn dark(role: PaletteColor) -> Color {
  ThemePalette::dark().get(role)
}

#[test]
fn the_card_is_the_boards_and_dark_on_light_in_either_appearance() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let app = App::new();
    app.theme().set_preferred_appearance(Some(appearance));
    let mut stage = Stage::in_app(app, |_| {
      Row::new().padding(20.0).child(QrCode::new(SHORT).id("code")).into()
    });
    let (x, y, width, height) = stage.bounds("code");
    assert_eq!((x, y, width, height), (20.0, 20.0, 160.0, 160.0));
    let card = stage.fill("code").expect("the card's fill");
    let QuadContent::Rect { color, .. } = card.content else {
      panic!("a fill");
    };
    assert_eq!(
      color,
      light(PaletteColor::Background),
      "the light fill in {appearance:?}"
    );
    assert_eq!(color, Color::from_hex("#FFFFFF"));
    let radius = card.border_radius.expect("round");
    assert_eq!(radius.top_left, 8.0);
    let border = stage.border("code").expect("an edge").border.unwrap();
    let edge = border.top.expect("an edge on each side");
    assert_eq!(edge.width, 1.0);
    let window_border = match appearance {
      Appearance::Light => light(PaletteColor::Border),
      Appearance::Dark => dark(PaletteColor::Border),
    };
    assert_eq!(edge.color, window_border, "the window's edge");
    let cells = modules(&mut stage, "code");
    assert!(!cells.is_empty(), "near black modules in {appearance:?}");
    assert_eq!(light(PaletteColor::Foreground), Color::from_hex("#09090B"));
    // The modules stand inside the padding of 10.
    for quad in &cells {
      assert!(quad.x >= x + 10.0 && quad.y >= y + 10.0);
      assert!(quad.x + quad.width <= x + width - 10.0 && quad.y + quad.height <= y + height - 10.0);
    }
    // The code is in the middle of the card: as much light on the left as on the right.
    let left = cells.iter().map(|quad| quad.x).fold(f32::MAX, f32::min) - x;
    let right = x + width - cells.iter().map(|quad| quad.x + quad.width).fold(0.0, f32::max);
    assert!((left - right).abs() <= 1.0, "{left} and {right}");
  }
}

#[test]
fn a_quiet_zone_and_a_length_make_the_modules_smaller() {
  let side = |stage: &mut Stage| {
    let cells = modules(stage, "code");
    let left = cells.iter().map(|quad| quad.x).fold(f32::MAX, f32::min);
    let right = cells.iter().map(|quad| quad.x + quad.width).fold(0.0, f32::max);
    right - left
  };
  let mut plain = Stage::new(|_| Row::new().child(QrCode::new(SHORT).id("code")).into());
  let mut quiet = Stage::new(|_| Row::new().child(QrCode::new(SHORT).quiet_zone(4).id("code")).into());
  let mut small = Stage::new(|_| Row::new().child(QrCode::new(SHORT).length(100.0).id("code")).into());
  // 25 modules in 140: of 5; with 4 of quiet zone on each side, 33 in 140: of 4; in 80, of 3.
  assert_eq!(side(&mut plain), 125.0);
  assert_eq!(side(&mut quiet), 100.0);
  assert_eq!(side(&mut small), 75.0);
  assert_eq!(small.bounds("code").2, 100.0);
}

#[test]
fn a_logo_stands_in_a_well_and_the_code_still_reads() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(20.0)
      .child(
        QrCode::new(SHORT)
          .media(Row::new().size(18.0, 18.0).background(PaletteColor::Primary).id("logo"))
          .id("code"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("code");
  let (lx, ly, lw, lh) = stage.bounds("logo");
  assert!(
    ((lx + lw / 2.0) - (x + width / 2.0)).abs() < 0.01,
    "{:?} in {:?}",
    (lx, ly, lw, lh),
    (x, y, width, height)
  );
  assert!(((ly + lh / 2.0) - (y + height / 2.0)).abs() < 0.01);
  assert_eq!(read_painted(&mut stage, "code", 1.0).as_deref(), Some(SHORT));
  // Made at the highest level whatever was asked: a code of 29 modules for this text, and not 25.
  let cells = modules(&mut stage, "code");
  let left = cells.iter().map(|quad| quad.x).fold(f32::MAX, f32::min);
  let right = cells.iter().map(|quad| quad.x + quad.width).fold(0.0, f32::max);
  assert_eq!(right - left, 29.0 * 4.0);
}

#[test]
fn a_text_no_code_holds_draws_the_card_empty() {
  let long = text_of(3000);
  let mut stage = Stage::new(move |_| Row::new().child(QrCode::new(&long).id("code")).into());
  assert!(modules(&mut stage, "code").is_empty());
  assert_eq!(stage.bounds("code").2, 160.0);
  // One too long for the level asked for is drawn at a lower one.
  let text = text_of(1500);
  let shown = text.clone();
  let mut lower = Stage::new(move |_| {
    Row::new()
      .child(QrCode::new(&shown).level(QrLevel::High).length(560.0).id("code"))
      .into()
  });
  lower
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      600.0, 600.0,
    ))));
  lower.pass();
  assert_eq!(read_painted(&mut lower, "code", 1.0).as_deref(), Some(text.as_str()));
}

#[test]
fn a_theme_rule_changes_every_code() {
  let app = App::new();
  app.theme().patch_recipe::<QrCode>(|rules| {
    rules
      .base(QrCodePart::Module, |s| s.background(PaletteColor::Primary))
      .base(QrCodePart::Root, |s| s.square(200.0))
  });
  let mut stage = Stage::in_app(app, |_| Row::new().child(QrCode::new(SHORT).id("code")).into());
  assert_eq!(stage.bounds("code").2, 200.0);
  let primary = light(PaletteColor::Primary);
  assert!(
    painted(&mut stage, "code")
      .0
      .iter()
      .any(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == primary))
  );
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_an_image_called_by_its_label_and_described_by_its_text() {
  let long = text_of(3000);
  let mut stage = Stage::new(move |_| {
    Row::new()
      .child(QrCode::new(SHORT).id("plain"))
      .child(QrCode::new("PAIR-7F3K").label("Pair your phone").id("pair"))
      .child(QrCode::new(&long).id("long"))
      .into()
  });
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "plain", "a11y_role").as_deref(), Some("img"));
  assert_eq!(said(&mut stage, "plain", "a11y_name").as_deref(), Some("QR code"));
  assert_eq!(said(&mut stage, "plain", "a11y_description").as_deref(), Some(SHORT));
  assert_eq!(
    said(&mut stage, "pair", "a11y_name").as_deref(),
    Some("Pair your phone")
  );
  assert_eq!(
    said(&mut stage, "pair", "a11y_description").as_deref(),
    Some("PAIR-7F3K")
  );
  // A text no code holds is still what tooling reads.
  assert_eq!(
    said(&mut stage, "long", "a11y_description").map(|text| text.len()),
    Some(3000)
  );
}

/// What tooling calls a code is the app's locale's word, read again when the app changes it; a locale of its own is
/// above the app's, and a label told by itself above both.
#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn what_tooling_calls_it_is_the_app_s_locale_s_and_a_label_told_by_itself_stays() {
  use lurq::core::Locale;
  let named = |stage: &mut Stage, id: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description("a11y_name"))
  };
  let app = App::new();
  app.set_locale(Locale::for_tag("uk"));
  let handle = app.clone();
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .child(QrCode::new(SHORT).id("plain"))
      .child(QrCode::new(SHORT).label("Pair your phone").id("told"))
      .child(QrCode::new(SHORT).locale(&Locale::for_tag("pl")).id("own"))
      .into()
  });
  assert_eq!(named(&mut stage, "plain").as_deref(), Some("QR-код"));
  assert_eq!(named(&mut stage, "told").as_deref(), Some("Pair your phone"));
  assert_eq!(named(&mut stage, "own").as_deref(), Some("Kod QR"));
  handle.set_locale(Locale::for_tag("de"));
  stage.pass();
  assert_eq!(named(&mut stage, "plain").as_deref(), Some("QR-Code"));
  assert_eq!(named(&mut stage, "told").as_deref(), Some("Pair your phone"));
}
