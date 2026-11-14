//! `ui::Spinner` and `ui::LoadingOverlay`: the ring, the dots and the bars at
//! the design's sizes and colors, the label beside them, the button's spinner,
//! and the veil that keeps a part of the page from input while it loads.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::{
    Alignment,
    quad::{Quad, QuadContent},
  },
  node::{color::Color, dimension::Dimension},
  ui::{
    Button, Case, FADE_IN, FADE_OUT, LoadingOverlay, LoadingOverlayPart, LoadingOverlayState, SPIN_PERIOD,
    SPINNER_BARS_PERIOD, SPINNER_DOTS_PERIOD, Spinner, SpinnerKind, SpinnerPart, SpinnerSize, SpinnerVariant, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

/// The filled rectangles of `width` by `height`, from the start of the row.
fn rects(stage: &Stage, width: f32, height: f32) -> Vec<Quad> {
  let mut found: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.width - width).abs() < 0.01
        && (quad.height - height).abs() < 0.01
    })
    .collect();
  found.sort_by(|a, b| a.x.total_cmp(&b.x));
  found
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

#[test]
fn a_ring_is_16_24_or_32_in_the_color_of_the_text_around_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .spacing(12.0)
      .foreground(PaletteColor::Destructive)
      .child(Spinner::new().id("sm"))
      .child(Spinner::new().size(SpinnerSize::Md).id("md"))
      .child(Spinner::new().size(SpinnerSize::Lg).id("lg"))
      .into()
  });
  for (id, side) in [("sm", 16.0), ("md", 24.0), ("lg", 32.0)] {
    let (_, _, width, height) = stage.bounds(id);
    assert_eq!((width, height), (side, side), "{id}");
  }
  let rings: Vec<_> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| match &quad.content {
      QuadContent::Text { text, style, .. } if *text == glyph(icons::SPINNER) => Some((style.font_size, style.color)),
      _ => None,
    })
    .collect();
  let destructive = light(PaletteColor::Destructive);
  assert_eq!(
    rings,
    [(16.0, destructive), (24.0, destructive), (32.0, destructive)],
    "the button's glyph, in the color of the row"
  );
}

#[test]
fn a_ring_turns() {
  let mut stage = Stage::new(|_| Row::new().child(Spinner::new()).into());
  stage.pass_after(SPIN_PERIOD / 4);
  let ring = stage.text_quad(&glyph(icons::SPINNER)).unwrap();
  assert!(
    !ring.transform.is_identity(),
    "a quarter of its period on, it is turned"
  );
}

#[test]
fn a_variant_colors_the_ring_and_the_dots() {
  let stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Spinner::new().variant(SpinnerVariant::Muted))
      .child(Spinner::new().kind(SpinnerKind::Dots).variant(SpinnerVariant::Primary))
      .into()
  });
  assert_eq!(
    stage.text(&glyph(icons::SPINNER)).0,
    light(PaletteColor::MutedForeground)
  );
  let dots = rects(&stage, 8.0, 8.0);
  assert_eq!(dots.len(), 3);
  assert!(dots.iter().all(|dot| color_of(dot) == light(PaletteColor::Primary)));
}

#[test]
fn the_label_is_8_beside_the_ring_and_only_when_it_is_drawn() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Spinner::new().label("Loading results…").id("inline"))
      .child(Spinner::new().label("Saving").label_hidden(true).id("named"))
      .into()
  });
  let (x, ..) = stage.bounds("inline");
  let label = stage.text_quad("Loading results…").unwrap();
  assert_eq!(label.x - (x + 16.0), 8.0);
  assert_eq!(
    stage.text("Loading results…"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  assert!(!stage.paints("Saving"), "a hidden label is only its name");
  let (_, _, width, _) = stage.bounds("named");
  assert_eq!(width, 16.0);
}

#[test]
fn dots_and_bars_take_the_color_of_the_text_around_them() {
  // In a primary button the label is in the primary foreground, and so are the spinner's dots and bars; in a
  // row of the destructive color, the destructive color; with nothing around them, the foreground.
  let stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Button::new("Loading").leading(Spinner::new().kind(SpinnerKind::Dots).decorative(true)))
      .child(
        Row::new()
          .foreground(PaletteColor::Destructive)
          .child(Spinner::new().kind(SpinnerKind::Bars)),
      )
      .child(Spinner::new().kind(SpinnerKind::Dots).size(SpinnerSize::Md))
      .into()
  });
  let in_button = rects(&stage, 8.0, 8.0);
  assert_eq!(in_button.len(), 3);
  assert!(
    in_button
      .iter()
      .all(|dot| color_of(dot) == light(PaletteColor::PrimaryForeground)),
    "the button's label's color"
  );
  assert_eq!(stage.text("Loading").0, light(PaletteColor::PrimaryForeground));
  let bars: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { .. }) && (quad.width - 3.0).abs() < 0.01)
    .collect();
  assert_eq!(bars.len(), 5);
  assert!(bars.iter().all(|bar| color_of(bar) == light(PaletteColor::Destructive)));
  let alone = rects(&stage, 12.0, 12.0);
  assert_eq!(alone.len(), 3);
  assert!(alone.iter().all(|dot| color_of(dot) == light(PaletteColor::Foreground)));
}

#[test]
fn dots_are_three_of_8_six_apart_that_fade_in_turn() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Spinner::new().kind(SpinnerKind::Dots).id("dots"))
      .into()
  });
  let (x, _, width, height) = stage.bounds("dots");
  assert_eq!((width, height), (36.0, 16.0));
  let dots = rects(&stage, 8.0, 8.0);
  assert_eq!(dots.len(), 3);
  for (index, dot) in dots.iter().enumerate() {
    assert!((dot.x - (x + 14.0 * index as f32)).abs() < 0.01, "6 apart");
    assert_eq!(color_of(dot), light(PaletteColor::Foreground));
    assert!(dot.border_radius.is_some_and(|radius| radius.top_left >= 4.0), "round");
  }

  // Over a period each dot fades and comes back, and not all of them at once.
  let mut seen = [Vec::new(), Vec::new(), Vec::new()];
  for step in 0..12 {
    stage.pass_after(SPINNER_DOTS_PERIOD / 12 * step);
    for (index, dot) in rects(&stage, 8.0, 8.0).iter().enumerate() {
      seen[index].push(dot.opacity);
    }
  }
  for opacities in &seen {
    assert!(opacities.iter().all(|opacity| (0.29..=1.0).contains(opacity)));
    assert!(
      opacities.iter().any(|opacity| *opacity < 0.6),
      "it fades: {opacities:?}"
    );
    assert!(
      opacities.iter().any(|opacity| *opacity > 0.9),
      "and comes back: {opacities:?}"
    );
  }
  assert_ne!(seen[0], seen[2], "a wave, not one blink");
}

#[test]
fn bars_are_five_of_3_at_the_designs_heights() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(Spinner::new().kind(SpinnerKind::Bars).id("bars"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("bars");
  assert_eq!((width, height), (27.0, 20.0));
  // A bar as it is painted: its box, and how high it is seen, scaled about its middle while it moves. The
  // quad's position is that of its corner once scaled, and its transform is the scale alone.
  let bars = |stage: &Stage| {
    let mut bars: Vec<(Quad, f32)> = stage
      .quads()
      .into_iter()
      .filter(|quad| {
        matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Foreground))
          && (quad.width - 3.0).abs() < 0.01
      })
      .map(|quad| {
        let seen = quad.height * quad.transform.d;
        (quad, seen)
      })
      .collect();
    bars.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
    bars
  };
  let heights: Vec<f32> = bars(&stage).iter().map(|(bar, _)| bar.height.round()).collect();
  assert_eq!(heights, [8.0, 18.0, 12.0, 20.0, 10.0], "at rest");

  // They shrink and grow back about the middle of the row, each a little after the one before it.
  let mut shrunk = false;
  for step in 0..8 {
    stage.pass_after(SPINNER_BARS_PERIOD / 8 * step);
    let painted = bars(&stage);
    assert_eq!(painted.len(), 5);
    for (index, (bar, seen)) in painted.iter().enumerate() {
      assert!((bar.x - (x + 6.0 * index as f32)).abs() < 0.01, "3 apart");
      assert!(
        (bar.y + seen / 2.0 - (y + 10.0)).abs() < 0.01,
        "on the middle of the row: {} + {seen} / 2",
        bar.y
      );
      assert!(*seen <= bar.height + 0.01 && *seen >= bar.height * 0.39);
      shrunk |= *seen < bar.height * 0.9;
    }
  }
  assert!(shrunk);
}

#[test]
fn the_dots_and_the_bars_grow_with_the_size() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Spinner::new().kind(SpinnerKind::Dots).size(SpinnerSize::Lg).id("dots"))
      .child(Spinner::new().kind(SpinnerKind::Bars).size(SpinnerSize::Md).id("bars"))
      .into()
  });
  let (_, _, width, height) = stage.bounds("dots");
  assert_eq!((width, height), (72.0, 32.0), "dots of 16, 12 apart");
  assert_eq!(rects(&stage, 16.0, 16.0).len(), 3);
  let (_, _, width, height) = stage.bounds("bars");
  assert_eq!((width, height), (40.5, 30.0), "bars of 4.5, 4.5 apart");
}

#[test]
fn a_theme_rule_changes_every_spinner_and_the_buttons_too() {
  let app = App::new();
  app.theme().patch_recipe::<Spinner>(|rules| {
    rules.base(SpinnerPart::Indicator, |s| s.foreground(Color::new(200, 30, 30, 255)))
  });
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .spacing(12.0)
      .child(Spinner::new())
      .child(Button::new("Saving…").loading(true))
      .into()
  });
  let red: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, style, .. } if *text == glyph(icons::SPINNER) && style.color == Color::new(200, 30, 30, 255)))
    .collect();
  assert_eq!(red.len(), 2, "the button's spinner is a Spinner");
  let button = red.iter().find(|quad| quad.y > 20.0).unwrap();
  assert_eq!(button.width, 16.0, "at the side the button gives it");
}

/// Whether a quad is filled with the background's color, wholly or in part: the veil's scrim.
fn is_scrim(quad: &Quad) -> bool {
  let background = light(PaletteColor::Background);
  matches!(&quad.content, QuadContent::Rect { color, .. }
    if color.a() > 0 && color.with_alpha(255) == background.with_alpha(255) && color.a() < 255)
}

/// A card of 220 by 120 with a button in it, under a veil while `loading` is true; the button counts its presses.
fn panel(loading: &Signal<bool>, presses: &Arc<AtomicUsize>) -> Stage {
  let (loading, presses) = (loading.clone(), presses.clone());
  Stage::new(move |_| {
    let presses = presses.clone();
    Column::new()
      .padding(8.0)
      .child(
        LoadingOverlay::new(
          loading.get(),
          Column::new()
            .size(220.0, 120.0)
            .padding(16.0)
            .child(Text::new("Recent activity"))
            .child(Button::new("Refresh").id("refresh").on_click(move |_| {
              presses.fetch_add(1, Ordering::SeqCst);
            })),
        )
        .id("overlay"),
      )
      .into()
  })
}

#[test]
fn a_veil_lies_over_the_content_with_a_spinner_in_its_middle() {
  let (loading, presses) = (Signal::new(true), Arc::new(AtomicUsize::new(0)));
  let mut stage = panel(&loading, &presses);
  let (x, y, width, height) = stage.bounds("overlay");
  assert_eq!((width, height), (220.0, 120.0), "as large as its content");
  let scrim = stage
    .quads()
    .into_iter()
    .find(|quad| is_scrim(quad) && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height))
    .expect("a scrim over all of the content");
  let shown = f32::from(color_of(&scrim).a()) / 255.0 * scrim.opacity;
  assert!((shown - 0.85).abs() < 0.01, "the design's 85 hundredths: {shown}");
  let ring = stage.text_quad(&glyph(icons::SPINNER)).unwrap();
  // The ring turns about its middle, which is the veil's: its quad's corner is where the turn has it, as far
  // from that middle as the corner of a square of 16 is, wherever in its turn the spinner is.
  let (from_x, from_y) = (ring.x - (x + width / 2.0), ring.y - (y + height / 2.0));
  let far = (from_x * from_x + from_y * from_y).sqrt();
  assert!((far - 128.0_f32.sqrt()).abs() < 0.5, "{from_x} {from_y}");
  assert!(stage.paints("Recent activity"), "the content is under it");
}

#[test]
fn the_veil_covers_its_content_whatever_room_the_overlay_has() {
  // A card in a column that stretches what it holds: the veil covers the card and not the room beside it.
  // Content that fills the column makes the overlay, and the veil, as wide as the column.
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .spacing(16.0)
      .align_items(Alignment::Stretch)
      .child(LoadingOverlay::new(true, Column::new().size(220.0, 60.0).id("card")))
      .child(LoadingOverlay::new(
        true,
        Column::new().width(Dimension::Pct(100.0)).height(60.0).id("wide"),
      ))
      .into()
  });
  let scrims: Vec<Quad> = stage.quads().into_iter().filter(|quad| is_scrim(quad)).collect();
  for id in ["card", "wide"] {
    let (x, y, width, height) = stage.bounds(id);
    assert!(
      scrims
        .iter()
        .any(|quad| (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height)),
      "#{id}: a scrim over exactly the content, {x},{y} {width}×{height}"
    );
  }
  assert_eq!(stage.bounds("card").2, 220.0);
  assert_eq!(stage.bounds("wide").2, 300.0);
  let card_top = stage.bounds("card").1;
  assert!(
    scrims
      .iter()
      .filter(|quad| quad.y == card_top)
      .all(|quad| quad.width == 220.0),
    "nothing beside the card is veiled"
  );
}

#[test]
fn while_it_loads_the_content_takes_no_press_and_no_tab() {
  let (loading, presses) = (Signal::new(true), Arc::new(AtomicUsize::new(0)));
  let mut stage = panel(&loading, &presses);
  stage.click("refresh");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(presses.load(Ordering::SeqCst), 0);
  assert!(!stage.has_ring(light(PaletteColor::Ring)), "Tab passes it by");

  loading.set(false);
  stage.pass();
  stage.click("refresh");
  assert_eq!(presses.load(Ordering::SeqCst), 1);
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(presses.load(Ordering::SeqCst), 2, "Tab reaches it again");
}

#[test]
fn the_veil_fades_in_and_out() {
  let (loading, presses) = (Signal::new(false), Arc::new(AtomicUsize::new(0)));
  let mut stage = panel(&loading, &presses);
  assert!(!stage.paints(&glyph(icons::SPINNER)));

  let theme = stage.app.theme().clone();
  let case = Case::new().state(LoadingOverlayState::Loading, true);
  let veil = theme
    .resolve::<LoadingOverlay>(&case, None)
    .part(LoadingOverlayPart::Veil)
    .rest();
  assert_eq!(veil.enter.map(|animation| animation.keyframes), Some(FADE_IN));
  assert_eq!(veil.exit.map(|animation| animation.keyframes), Some(FADE_OUT));

  loading.set(true);
  stage.pass();
  assert!(stage.paints(&glyph(icons::SPINNER)));
  // As it ends, the veil stays while it fades, and is gone after.
  loading.set(false);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  assert!(stage.paints(&glyph(icons::SPINNER)), "fading out");
  stage.pass();
  assert!(!stage.paints(&glyph(icons::SPINNER)));
}
