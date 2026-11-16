//! `ui::Watermark`: the board's marks (type, color, faintness, angle, gap and stagger), sharp when turned, over the
//! content without taking its pointer, nothing to tooling, a long scrolled page painted only where it is seen, a
//! picture for a mark, the dark look and a theme's rule.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
    window::Appearance,
  },
  components::{Button, Column, Row, ScrollVertical},
  layout::{
    Constraints, Size,
    layout_kind::ScrollState,
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::{TextTransformMode, color::Color},
  ui::{Watermark, WatermarkPart},
};

use super::stage::Stage;

const MARK: &str = "lurq internal";

/// The board's card: 220 by 120, with the watermark over it.
fn card(watermark: Watermark) -> Watermark {
  watermark.id("mark").child(Column::new().size(220.0, 120.0).id("card"))
}

/// The marks painted: the text quads of `text`.
fn marks(stage: &Stage, text: &str) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text: painted, .. } if painted == text))
    .collect()
}

/// Where a quad's box is turned about: its middle, which a turn does not move.
fn middle(quad: &Quad) -> (f32, f32) {
  let [a, b, c, d] = quad.transform.matrix_2x2();
  let (w, h) = (quad.width / 2.0, quad.height / 2.0);
  (quad.x + a * w + c * h, quad.y + b * w + d * h)
}

#[test]
fn the_marks_are_the_boards_faint_and_turned_across_the_card() {
  let mut stage = Stage::new(|_| Row::new().padding(20.0).child(card(Watermark::new(MARK))).into());
  let found = marks(&stage, MARK);
  assert!(found.len() >= 6, "the card is covered: {} marks", found.len());
  let foreground = ThemePalette::light().get(PaletteColor::Foreground);
  for quad in &found {
    let QuadContent::Text {
      style, transform_mode, ..
    } = &quad.content
    else {
      unreachable!()
    };
    assert_eq!(style.font_size, 12.0);
    assert_eq!(style.weight, FontWeight::Medium);
    assert_eq!(style.color, foreground);
    assert!((quad.opacity - 0.12).abs() < 1e-4, "faint: {}", quad.opacity);
    // Turned 20 degrees counter-clockwise, rising to the right; its glyphs drawn at the angle.
    let [a, b, ..] = quad.transform.matrix_2x2();
    let angle = b.atan2(a).to_degrees();
    assert!((angle + 20.0).abs() < 0.01, "{angle}");
    assert_eq!(*transform_mode, TextTransformMode::Rasterized);
    // Within the card's clip.
    assert_eq!(
      (quad.clip.x, quad.clip.y, quad.clip.width, quad.clip.height),
      (20.0, 20.0, 220.0, 120.0)
    );
  }
}

#[test]
fn the_marks_are_a_gap_apart_and_every_other_row_further_along() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(Watermark::new(MARK).child(Column::new().size(380.0, 280.0)))
      .into()
  });
  let found = marks(&stage, MARK);
  let (width, height) = (found[0].width, found[0].height);
  let mut middles: Vec<(f32, f32)> = found.iter().map(middle).collect();
  middles.sort_by(|one, other| one.1.total_cmp(&other.1).then(one.0.total_cmp(&other.0)));
  let rows: Vec<f32> = middles.iter().fold(Vec::new(), |mut rows, (_, y)| {
    if rows.last().is_none_or(|last: &f32| (y - last).abs() > 0.5) {
      rows.push(*y);
    }
    rows
  });
  assert!(rows.len() >= 6);
  for pair in rows.windows(2) {
    assert!(
      (pair[1] - pair[0] - (height + 29.0)).abs() < 0.01,
      "rows {pair:?}, a line of {height}"
    );
  }
  let row_of = |y: f32| -> Vec<f32> {
    middles
      .iter()
      .filter(|(_, my)| (my - y).abs() < 0.5)
      .map(|(x, _)| *x)
      .collect()
  };
  let (first, second) = (row_of(rows[0]), row_of(rows[1]));
  for pair in first.windows(2) {
    assert!(
      (pair[1] - pair[0] - (width + 27.0)).abs() < 0.01,
      "marks {pair:?}, a text of {width}"
    );
  }
  let pitch = width + 27.0;
  let shift = (second[0] - first[0]).rem_euclid(pitch);
  assert!((shift - 40.0).abs() < 0.01, "every other row 40 further along: {shift}");
  // The first mark's box is at -20, 18 from the top left, before it is turned: its middle is where the box's is.
  let at = (-20.0 + width / 2.0, 18.0 + height / 2.0);
  assert!(
    middles
      .iter()
      .any(|(x, y)| (x - at.0).abs() < 0.01 && (y - at.1).abs() < 0.01),
    "a mark at {at:?}"
  );
}

#[test]
fn the_apps_gap_angle_offset_and_stagger() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(
        Watermark::new("Draft")
          .gap(10.0, 6.0)
          .angle(-45.0)
          .offset(4.0, 4.0)
          .stagger(0.0)
          .child(Column::new().size(300.0, 200.0)),
      )
      .into()
  });
  let found = marks(&stage, "Draft");
  let (width, height) = (found[0].width, found[0].height);
  let [a, b, ..] = found[0].transform.matrix_2x2();
  assert!((b.atan2(a).to_degrees() + 45.0).abs() < 0.01);
  let mut xs: Vec<f32> = found.iter().map(|quad| middle(quad).0).collect();
  xs.sort_by(f32::total_cmp);
  xs.dedup_by(|one, other| (*one - *other).abs() < 0.5);
  for pair in xs.windows(2) {
    assert!((pair[1] - pair[0] - (width + 10.0)).abs() < 0.01, "{pair:?}");
  }
  assert!(
    found
      .iter()
      .any(|quad| (middle(quad).0 - (4.0 + width / 2.0)).abs() < 0.01
        && (middle(quad).1 - (4.0 + height / 2.0)).abs() < 0.01)
  );
}

#[test]
fn the_content_takes_the_pointer_through_the_marks() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    Row::new()
      .child(
        Watermark::new(MARK).child(Column::new().size(300.0, 200.0).padding(40.0).child(
          Button::new("Open").size(200.0, 100.0).id("under").on_click(move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
          }),
        )),
      )
      .into()
  });
  stage.click("under");
  assert_eq!(presses.load(Ordering::SeqCst), 1);
}

#[test]
fn a_long_page_paints_only_the_marks_in_view_and_they_scroll_with_it() {
  let scroll = ScrollState::new();
  let held = scroll.clone();
  let mut stage = Stage::new(move |_| {
    ScrollVertical::new(Watermark::new(MARK).child(Column::new().size(380.0, 40_000.0)))
      .size(400.0, 300.0)
      .with_scroll_state(held.clone())
      .into()
  });
  let at_top = marks(&stage, MARK);
  // A view of 400 by 300 holds some 30 marks; the page holds some 3600.
  assert!(at_top.len() < 80, "{} marks painted", at_top.len());
  assert!(at_top.len() > 10);
  let first = at_top.iter().map(middle).fold(f32::MAX, |least, (_, y)| least.min(y));
  scroll.set_scroll(0.0, 20_000.0);
  stage.pass();
  let far = marks(&stage, MARK);
  assert!(far.len() < 80 && far.len() > 10, "{} marks painted far down", far.len());
  for quad in &far {
    let (_, y) = middle(quad);
    assert!(y > -40.0 && y < 340.0, "a mark far from the view: {y}");
  }
  // They are the page's and move with it: where they are in the view is where they are on the page less the scroll.
  let row_pitch = at_top[0].height + 29.0;
  let top_of_view = far.iter().map(middle).fold(f32::MAX, |least, (_, y)| least.min(y));
  let on_page = top_of_view + 20_000.0 - first;
  let rows = on_page / row_pitch;
  assert!((rows - rows.round()).abs() < 0.01, "{rows} rows down");
}

#[test]
fn a_picture_is_repeated_in_place_of_the_text() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(
        Watermark::new("lurq")
          .media(Row::new().size(20.0, 20.0).background(PaletteColor::Primary))
          .child(Column::new().size(300.0, 200.0)),
      )
      .into()
  });
  let primary = ThemePalette::light().get(PaletteColor::Primary);
  let pictures: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == primary))
    .collect();
  assert!(pictures.len() > 6, "{}", pictures.len());
  assert!(pictures.iter().all(|quad| (quad.opacity - 0.12).abs() < 1e-4));
  assert!(marks(&stage, "lurq").is_empty());
}

#[test]
fn the_marks_are_light_on_the_dark_look() {
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Dark));
  let mut stage = Stage::in_app(app, |_| Row::new().child(card(Watermark::new(MARK))).into());
  let dark = ThemePalette::dark().get(PaletteColor::Foreground);
  let found = marks(&stage, MARK);
  assert!(!found.is_empty());
  for quad in found {
    let QuadContent::Text { style, .. } = quad.content else {
      unreachable!()
    };
    assert_eq!(style.color, dark);
  }
  let _ = stage.bounds("card");
}

#[test]
fn a_theme_rule_changes_every_watermark() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Watermark>(|rules| rules.base(WatermarkPart::Mark, |s| s.opacity(0.3).font_size(16.0)));
  let mut stage = Stage::in_app(app, |_| Row::new().child(card(Watermark::new(MARK))).into());
  let found = marks(&stage, MARK);
  assert!(!found.is_empty());
  for quad in found {
    assert!((quad.opacity - 0.3).abs() < 1e-4);
    let QuadContent::Text { style, .. } = quad.content else {
      unreachable!()
    };
    assert_eq!(style.font_size, 16.0);
  }
  assert_eq!(stage.bounds("mark").2, 220.0);
}

#[test]
fn the_watermark_is_as_large_as_what_it_covers() {
  let mut stage = Stage::new(|_| Row::new().child(card(Watermark::new(MARK))).into());
  assert_eq!(stage.bounds("mark"), stage.bounds("card"));
  // A text of any length is a mark of its own width, cut by the card.
  let long = "a very long watermark that is far wider than the card it is drawn over";
  let mut stage = Stage::new(move |_| Row::new().child(card(Watermark::new(long))).into());
  assert_eq!(stage.bounds("mark").2, 220.0);
  for quad in marks(&stage, long) {
    assert_eq!((quad.clip.width, quad.clip.height), (220.0, 120.0));
  }
  let _ = Color::from_hex("#000000");
  let _ = Constraints::loose(Size::new(1.0, 1.0));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_does_not_read_the_marks() {
  let mut stage = Stage::new(|_| Row::new().child(card(Watermark::new(MARK))).into());
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  // The layer of marks over the card says it is nothing to tooling; the card it covers is what tooling reads.
  assert_eq!(said(&mut stage, "mark-layer", "a11y_hidden").as_deref(), Some("true"));
  assert_eq!(said(&mut stage, "card", "a11y_hidden"), None);
  assert_eq!(said(&mut stage, "mark", "a11y_hidden"), None);
}
