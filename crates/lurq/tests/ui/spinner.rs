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
    SPINNER_BARS_PERIOD, SPINNER_DOTS_PERIOD, SpinMotion, Spinner, SpinnerKind, SpinnerPart, SpinnerSize,
    SpinnerVariant, icons,
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

#[test]
fn a_spinner_and_a_loading_buttons_move_while_motion_is_reduced_too() {
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .spacing(12.0)
      .child(Spinner::new())
      .child(Spinner::new().kind(SpinnerKind::Dots))
      .child(Spinner::new().kind(SpinnerKind::Bars))
      .child(Button::new("Saving…").loading(true))
      .into()
  });
  let bars = |stage: &Stage| -> Vec<Quad> {
    let mut bars: Vec<Quad> = stage
      .quads()
      .into_iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Rect { .. }) && (quad.width - 3.0).abs() < 0.01)
      .collect();
    bars.sort_by(|a, b| a.x.total_cmp(&b.x));
    bars
  };
  // A quarter of a turn on, the spinner's ring and the button's are both turned.
  stage.pass_after(SPIN_PERIOD / 4);
  let rings: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::SPINNER)))
    .collect();
  assert_eq!(rings.len(), 2, "the spinner's ring and the button's");
  assert!(
    rings.iter().all(|ring| !ring.transform.is_identity()),
    "both rings turn: they say that something is under way, and nothing else does"
  );
  // Over a period each dot fades and comes back, and each bar shrinks and grows back.
  let mut faded = [Vec::new(), Vec::new(), Vec::new()];
  let mut shrunk = [false; 5];
  for step in 0..12 {
    stage.pass_after(SPINNER_DOTS_PERIOD / 12 * step);
    for (index, dot) in rects(&stage, 8.0, 8.0).iter().enumerate() {
      faded[index].push(dot.opacity);
    }
    stage.pass_after(SPINNER_BARS_PERIOD / 12 * step);
    for (index, bar) in bars(&stage).iter().enumerate() {
      shrunk[index] |= !bar.transform.is_identity();
    }
  }
  for (index, seen) in faded.iter().enumerate() {
    let (least, most) = seen.iter().fold((f32::MAX, f32::MIN), |(least, most), opacity| {
      (least.min(*opacity), most.max(*opacity))
    });
    assert!(most - least > 0.3, "dot {index} fades and comes back: {seen:?}");
  }
  assert_eq!(shrunk, [true; 5], "every bar shrinks at some moment of its period");
}

/// How many times what `look` reads changes over `span`, read every 2 milliseconds.
fn changes(stage: &mut Stage, span: Duration, look: impl Fn(&Stage) -> Vec<String>) -> usize {
  let mut last = look(stage);
  let mut changes = 0;
  let mut at = Duration::ZERO;
  while at < span {
    at += Duration::from_millis(2);
    stage.pass_after(at);
    let now = look(stage);
    if now != last {
      changes += 1;
      last = now;
    }
  }
  changes
}

fn ring_poses(stage: &Stage) -> Vec<String> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::SPINNER)))
    .map(|ring| format!("{:?}", ring.transform))
    .collect()
}

fn dot_opacities(stage: &Stage) -> Vec<String> {
  rects(stage, 8.0, 8.0)
    .iter()
    .map(|dot| format!("{:.4}", dot.opacity))
    .collect()
}

#[test]
fn a_stepped_ring_turns_a_pose_a_step_and_asks_for_no_frame_between() {
  let mut stage = Stage::new(|_| Row::new().child(Spinner::new().motion(SpinMotion::stepped(24))).into());
  let step = SPIN_PERIOD / 24;
  stage.pass_after(Duration::from_millis(5));
  let next = stage.next_change_after(Duration::from_millis(5)).expect("it moves");
  assert!(
    next > Duration::ZERO && next <= step,
    "the next frame is at the next step, not at once: {next:?}"
  );
  // Half a second is 12 steps: 12 poses, give or take the one it starts in.
  let poses = changes(&mut stage, SPIN_PERIOD / 2, ring_poses);
  assert!((11..=13).contains(&poses), "{poses} poses in half a turn");

  // A smooth ring asks for the next frame at once, and has a pose at every one.
  let mut smooth = Stage::new(|_| Row::new().child(Spinner::new()).into());
  smooth.pass_after(Duration::from_millis(5));
  assert_eq!(smooth.next_change_after(Duration::from_millis(5)), Some(Duration::ZERO));
  assert!(changes(&mut smooth, SPIN_PERIOD / 2, ring_poses) > 100);
}

#[test]
fn stepped_dots_and_bars_change_together_on_the_steps() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(Spinner::new().kind(SpinnerKind::Dots).motion(SpinMotion::stepped(24)))
      .into()
  });
  stage.pass_after(Duration::from_millis(3));
  let next = stage.next_change_after(Duration::from_millis(3)).expect("they move");
  assert!(next > Duration::ZERO && next <= SPINNER_DOTS_PERIOD / 28, "{next:?}");
  // A period of the dots (1.2 seconds) is 28 steps at 24 a second, 14 there and 14 back: every dot starts on a
  // step, so the three change together, about 28 times, not twice or three times as often.
  let changed = changes(&mut stage, SPINNER_DOTS_PERIOD, dot_opacities);
  assert!((27..=29).contains(&changed), "{changed} changes in a period");

  let mut bars = Stage::new(|_| {
    Row::new()
      .child(Spinner::new().kind(SpinnerKind::Bars).motion(SpinMotion::stepped(12)))
      .into()
  });
  bars.pass_after(Duration::from_millis(3));
  let next = bars.next_change_after(Duration::from_millis(3)).expect("they move");
  assert!(next > Duration::ZERO && next <= SPINNER_BARS_PERIOD / 12, "{next:?}");
}

#[test]
fn a_spinner_told_to_stays_still_where_motion_is_reduced_and_a_theme_tells_every_one() {
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let still = SpinMotion::stepped(24).still_when_reduced(true);
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .spacing(12.0)
      .child(Spinner::new().motion(still))
      .child(Spinner::new().kind(SpinnerKind::Dots).motion(still))
      .child(Spinner::new().kind(SpinnerKind::Bars).motion(still))
      .into()
  });
  stage.pass_after(SPIN_PERIOD / 4);
  assert_eq!(stage.next_change_after(SPIN_PERIOD / 4), None, "nothing moves");
  assert!(
    ring_poses(&stage).iter().all(|pose| *pose == ring_poses(&stage)[0]),
    "the ring at rest"
  );
  let rings: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::SPINNER)))
    .collect();
  assert!(rings.iter().all(|ring| ring.transform.is_identity()), "unturned");
  assert_eq!(
    dot_opacities(&stage),
    ["1.0000", "0.6000", "0.3000"],
    "the dots as drawn still"
  );

  // The theme's motion is every spinner's that has none of its own, a loading button's ring too.
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  app.theme().set_spin_motion(still);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .child(Spinner::new())
      .child(Button::new("Saving…").loading(true))
      .into()
  });
  stage.pass_after(SPIN_PERIOD / 4);
  assert_eq!(stage.next_change_after(SPIN_PERIOD / 4), None, "nothing moves");
  // Where motion is not reduced, they turn, in the theme's steps.
  let app = App::new();
  app.theme().set_reduced_motion(Some(false));
  app.theme().set_spin_motion(still);
  let mut stage = Stage::in_app(app, |_| Row::new().child(Button::new("Saving…").loading(true)).into());
  stage.pass_after(Duration::from_millis(5));
  let next = stage.next_change_after(Duration::from_millis(5)).expect("it turns");
  assert!(next > Duration::ZERO && next <= SPIN_PERIOD / 24, "{next:?}");
}

mod page_loader {
  use lurq::{
    app::{
      App,
      theme::{PaletteColor, ThemePalette},
    },
    components::{Column, Row},
    layout::quad::QuadContent,
    node::color::Color,
    ui::{
      Button, PageLoader, PageLoaderPart, PageLoaderRenderer, PageLoaderView, SPIN_PERIOD, Spinner, SpinnerKind, icons,
    },
  };

  use super::{super::stage::Stage, glyph};

  fn light(role: PaletteColor) -> Color {
    ThemePalette::light().get(role)
  }

  fn starting() -> PageLoader {
    PageLoader::new("Starting lurq").description("Loading your workspace…")
  }

  /// The loader in the board's room of 280 by 180.
  fn room(loader: impl Fn() -> PageLoader + Send + Sync + 'static) -> Stage {
    Stage::new(move |_| {
      Column::new()
        .size(400.0, 300.0)
        .child(Column::new().size(280.0, 180.0).child(loader().id("loader")))
        .into()
    })
  }

  #[test]
  fn a_page_loader_fills_its_room_with_a_ring_of_28_over_a_title_and_a_line_in_its_middle() {
    let mut stage = room(starting);
    assert_eq!(stage.bounds("loader"), (0.0, 0.0, 280.0, 180.0), "as large as its room");
    assert_eq!(
      stage.text(&glyph(icons::SPINNER)),
      (light(PaletteColor::Foreground), 28.0)
    );
    assert_eq!(stage.text("Starting lurq"), (light(PaletteColor::Foreground), 14.0));
    assert_eq!(
      stage.text("Loading your workspace…"),
      (light(PaletteColor::MutedForeground), 13.0)
    );
    let title = stage.text_quad("Starting lurq").unwrap();
    let line = stage.text_quad("Loading your workspace…").unwrap();
    assert!(
      (line.y - (title.y + title.height) - 4.0).abs() < 0.01,
      "the line 4 under the title: {}",
      line.y - (title.y + title.height)
    );
    // The ring's own quad is wherever in its turn it is, so the title is read against the room: what the loader
    // holds is the ring of 28, 14, the title, 4 and the line, in the middle of 180.
    let held = 28.0 + 14.0 + title.height + 4.0 + line.height;
    assert!(
      (title.y - ((180.0 - held) / 2.0 + 28.0 + 14.0)).abs() < 0.51,
      "the title 14 under a ring of 28, all in the middle: {}",
      title.y
    );
    for words in [&title, &line] {
      assert!(
        (words.x + words.width / 2.0 - 140.0).abs() < 0.51,
        "centred across the room: {} wide at {}",
        words.width,
        words.x
      );
    }
  }

  #[test]
  fn its_ring_turns_and_while_motion_is_reduced_too() {
    let mut stage = room(starting);
    stage.pass_after(SPIN_PERIOD / 4);
    assert!(!stage.text_quad(&glyph(icons::SPINNER)).unwrap().transform.is_identity());

    let app = App::new();
    app.theme().set_reduced_motion(Some(true));
    let mut stage = Stage::in_app(app, |_| Column::new().size(280.0, 180.0).child(starting()).into());
    stage.pass_after(SPIN_PERIOD / 4);
    assert!(
      !stage.text_quad(&glyph(icons::SPINNER)).unwrap().transform.is_identity(),
      "a page that starts says so by its ring, whatever the setting"
    );
  }

  #[test]
  fn a_title_alone_and_a_spinner_of_the_apps_own() {
    let stage = room(|| PageLoader::new("Opening the project").spinner(Spinner::new().kind(SpinnerKind::Dots)));
    assert!(stage.paints("Opening the project"));
    assert!(!stage.paints(&glyph(icons::SPINNER)), "no ring");
    let dots = stage
      .quads()
      .into_iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0) && quad.width == 8.0)
      .count();
    assert_eq!(dots, 3, "the app's dots, at their own size");
  }

  #[test]
  fn a_title_and_a_line_longer_than_the_room_wrap_inside_it() {
    const TITLE: &str = "Starting the workspace of the design systems team";
    const LINE: &str = "Fetching the projects, the members and the settings each of them keeps";
    let stage = Stage::new(|_| {
      Column::new()
        .size(400.0, 300.0)
        .child(
          Row::new()
            .width(200.0)
            .child(PageLoader::new(TITLE).description(LINE).id("loader")),
        )
        .into()
    });
    let mut lines = 0;
    for quad in stage.quads() {
      if let QuadContent::Text { text, .. } = &quad.content {
        if *text == glyph(icons::SPINNER) {
          continue;
        }
        lines += 1;
        assert!(
          quad.x >= 24.0 - 0.5 && quad.x + quad.width <= 200.0 - 24.0 + 0.5,
          "{text:?} keeps 24 from the sides of a room of 200: {} at {}",
          quad.width,
          quad.x
        );
      }
    }
    assert!(lines >= 2, "the title and the line are drawn: {lines}");
    let title = stage.text_quad(TITLE).expect("the title, whole");
    assert!(
      title.height > 14.0 * 1.6,
      "on more than one line: {} high",
      title.height
    );
  }

  #[cfg(any(feature = "devtools", feature = "mcp"))]
  #[test]
  fn tooling_reads_one_status_called_by_the_title_and_described_by_the_line() {
    let mut stage = room(starting);
    let said = |stage: &mut Stage, name: &str| {
      stage
        .tree
        .get_element_by_id_mut("loader")
        .and_then(|element| element.description(name))
    };
    assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("status"));
    assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Starting lurq"));
    assert_eq!(
      said(&mut stage, "a11y_description").as_deref(),
      Some("Loading your workspace…")
    );
  }

  #[test]
  fn it_takes_no_focus_and_a_theme_restyles_it_or_draws_another() {
    let mut stage = Stage::new(|_| {
      Column::new()
        .size(400.0, 300.0)
        .child(Column::new().size(280.0, 180.0).child(starting()))
        .child(Button::new("Cancel").id("cancel"))
        .into()
    });
    stage.key("Tab");
    let focused = stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned));
    assert_eq!(focused.as_deref(), Some("cancel"));

    let app = App::new();
    app.theme().patch_recipe::<PageLoader>(|rules| {
      rules
        .base(PageLoaderPart::Spinner, |s| s.square(40.0))
        .base(PageLoaderPart::Title, |s| s.foreground(PaletteColor::Info))
        .base(PageLoaderPart::Root, |s| s.background(PaletteColor::Muted))
    });
    let mut stage = Stage::in_app(app, |_| {
      Column::new().size(280.0, 180.0).child(starting().id("loader")).into()
    });
    assert_eq!(stage.text(&glyph(icons::SPINNER)).1, 40.0);
    assert_eq!(stage.text("Starting lurq").0, light(PaletteColor::Info));
    assert_eq!(stage.fill_color("loader"), Some(light(PaletteColor::Muted)));

    let app = App::new();
    let renderer: PageLoaderRenderer =
      std::sync::Arc::new(|view: PageLoaderView| view.root().child(view.text()).into());
    app.theme().set_renderer::<PageLoader>(renderer);
    let stage = Stage::in_app(app, |_| Column::new().size(280.0, 180.0).child(starting()).into());
    assert!(stage.paints("Starting lurq"));
    assert!(!stage.paints(&glyph(icons::SPINNER)), "the theme's loader has no ring");
  }
}
