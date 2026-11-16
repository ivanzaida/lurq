//! The time a player reads out: elapsed and the whole in the mono type, its forms (over an hour, what is left, not
//! loaded, live), over a picture, its sizes, and a press of it.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Row,
  node::color::Color,
  ui::{MediaTime, MediaTimeSize, media_rate_text, media_time_text},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn scene(time: MediaTime) -> lurq::node::Element {
  Row::new().padding(8.0).child(time).into()
}

#[test]
fn the_time_reads_elapsed_then_the_whole_4_apart_in_the_mono_type() {
  let stage = Stage::new(|_| scene(MediaTime::new(84.0, Some(250.0))));
  assert_eq!(stage.text("1:24"), (light(PaletteColor::Foreground), 12.0));
  assert_eq!(stage.text("/"), (light(PaletteColor::MutedForeground), 12.0));
  assert_eq!(stage.text("4:10"), (light(PaletteColor::MutedForeground), 12.0));
  for text in ["1:24", "/", "4:10"] {
    assert!(stage.font_of(text).to_lowercase().contains("mono"), "{text}");
  }
  let (elapsed, divider, total) = (
    stage.text_quad("1:24").unwrap(),
    stage.text_quad("/").unwrap(),
    stage.text_quad("4:10").unwrap(),
  );
  assert!((divider.x - (elapsed.x + elapsed.width) - 4.0).abs() < 0.6);
  assert!((total.x - (divider.x + divider.width) - 4.0).abs() < 0.6);
}

#[test]
fn its_forms_read_hours_what_is_left_and_what_is_not_known() {
  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(24.0)
      .child(MediaTime::new(3765.0, Some(7800.0)))
      .child(MediaTime::new(84.0, Some(250.0)).remaining(true))
      .child(MediaTime::new(0.0, None))
      .into()
  });
  // From an hour up both read hours.
  assert!(stage.paints("1:02:45") && stage.paints("2:10:00"));
  // What is left reads alone.
  assert_eq!(stage.text("-2:46"), (light(PaletteColor::Foreground), 12.0));
  assert!(!stage.paints("4:10"));
  // Not loaded: both muted.
  let unknown: Vec<Color> = stage
    .quads()
    .iter()
    .filter_map(|quad| match &quad.content {
      lurq::layout::quad::QuadContent::Text { text, style, .. } if text == "-:--" => Some(style.color),
      _ => None,
    })
    .collect();
  assert_eq!(unknown, [light(PaletteColor::MutedForeground); 2]);

  assert_eq!(media_time_text(f64::NAN, false), "0:00");
  assert_eq!(media_rate_text(0.75), "0.75×");
}

#[test]
fn over_a_picture_it_is_white_and_the_whole_at_60_percent() {
  let stage = Stage::new(|_| scene(MediaTime::new(84.0, Some(250.0)).on_media(true)));
  assert_eq!(stage.text("1:24").0, Color::from_hex("#FFFFFF"));
  assert_eq!(stage.text("/").0, Color::from_hex("#FFFFFF99"));
  assert_eq!(stage.text("4:10").0, Color::from_hex("#FFFFFF99"));
}

#[test]
fn a_compact_players_is_11_and_a_phones_13() {
  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(24.0)
      .child(MediaTime::new(84.0, Some(250.0)).size(MediaTimeSize::Sm))
      .child(MediaTime::new(95.0, Some(250.0)).size(MediaTimeSize::Lg))
      .into()
  });
  assert_eq!(stage.text("1:24").1, 11.0);
  assert_eq!(stage.text("1:35").1, 13.0);
}

#[test]
fn live_it_is_a_badge_in_the_destructive_color() {
  let mut stage = Stage::new(|_| scene(MediaTime::new(0.0, None).live(true).id("time")));
  assert_eq!(stage.text("LIVE").0, light(PaletteColor::DestructiveForeground));
  assert!(!stage.paints("-:--"));
  let label = stage.text_quad("LIVE").unwrap();
  let badge = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, lurq::layout::quad::QuadContent::Rect { color, .. } if *color == light(PaletteColor::Destructive))
      && quad.x < label.x
      && quad.x + quad.width > label.x + label.width
  });
  assert!(badge);
  stage.bounds("time");
}

#[test]
fn a_press_tells_the_player_and_the_keyboard_reaches_it() {
  let pressed = Arc::new(Mutex::new(0));
  let count = pressed.clone();
  let mut stage = Stage::new(move |_| {
    let count = count.clone();
    scene(
      MediaTime::new(84.0, Some(250.0))
        .on_click(move |_| *count.lock().unwrap() += 1)
        .id("time"),
    )
  });
  stage.click("time");
  assert_eq!(*pressed.lock().unwrap(), 1);
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(*pressed.lock().unwrap(), 2);

  // A time that nothing presses is no stop of Tab.
  let mut still = Stage::new(|_| scene(MediaTime::new(84.0, Some(250.0))));
  still.key("Tab");
  assert!(!still.has_ring(light(PaletteColor::Ring)));
}
