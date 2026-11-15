//! A waveform: its bars (3 wide, 2 apart, no taller than 24, fully round), the played ones; unheard; operated, what a
//! press, a drag and the keys tell; shown and not operated, and disabled, it takes nothing.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::Row,
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Case, Opacity, Waveform, WaveformPart, WaveformState},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

const PEAKS: [f32; 6] = [0.25, 0.5, 1.0, 0.0, 0.75, 0.5];

/// The bars of the waveform `id`, from its start.
fn bars(stage: &mut Stage, id: &str) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  let mut quads: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && quad.width == 3.0
        && quad.x >= x - 0.01
        && quad.x + quad.width <= x + width + 0.01
        && quad.y >= y - 0.01
        && quad.y + quad.height <= y + height + 0.01
    })
    .collect();
  quads.sort_by(|a, b| a.x.total_cmp(&b.x));
  quads
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

#[derive(Clone, Default)]
struct Heard(Arc<Mutex<Vec<String>>>);

impl Heard {
  fn tell(&self, what: impl Into<String>) {
    self.0.lock().unwrap().push(what.into());
  }

  fn take(&self) -> Vec<String> {
    std::mem::take(&mut *self.0.lock().unwrap())
  }
}

#[test]
fn a_waveform_is_round_bars_3_wide_and_2_apart_played_in_the_primary_color() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Waveform::at(0.5, &PEAKS).id("wave"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("wave");
  assert_eq!((width, height), (6.0 * 3.0 + 5.0 * 2.0, 24.0));
  let bars = bars(&mut stage, "wave");
  assert_eq!(bars.len(), 6);
  for (index, bar) in bars.iter().enumerate() {
    assert!(
      (bar.x - x - 5.0 * index as f32).abs() < 0.01,
      "bar {index} is 5 after the one before"
    );
    // As high as its peak of 24, and no lower than it is wide; in the middle of the height.
    let expected = (PEAKS[index] * 24.0).max(3.0);
    assert!((bar.height - expected).abs() < 0.01, "bar {index}: {}", bar.height);
    assert!((bar.y + bar.height / 2.0 - (y + 12.0)).abs() < 0.51);
    assert!(bar.border_radius.is_some_and(|radius| radius.top_left >= 1.5), "round");
    let role = if index < 3 {
      PaletteColor::Primary
    } else {
      PaletteColor::Input
    };
    assert_eq!(color_of(bar), light(role), "bar {index}");
  }
}

#[test]
fn unheard_every_bar_is_in_the_color_of_what_is_not_played() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Waveform::at(0.5, &PEAKS).unheard(true).id("wave"))
      .into()
  });
  assert!(
    bars(&mut stage, "wave")
      .iter()
      .all(|bar| color_of(bar) == light(PaletteColor::Input))
  );
}

#[test]
fn operated_a_press_and_a_drag_tell_where_they_are_and_the_end_where_it_is_let_go() {
  let heard = Heard::default();
  let (told, ended) = (heard.clone(), heard.clone());
  let progress = Signal::new(0.0_f32);
  let read = progress.clone();
  let mut stage = Stage::new(move |_| {
    let (told, ended) = (told.clone(), ended.clone());
    Row::new()
      .padding(8.0)
      .child(
        Waveform::new(read.clone(), &PEAKS)
          .on_change(move |to| told.tell(format!("change {to}")))
          .on_change_end(move |to| ended.tell(format!("end {to}")))
          .id("wave"),
      )
      .into()
  });
  let (x, y, width, _) = stage.bounds("wave");
  stage.click("wave");
  assert_eq!(heard.take(), ["change 0.5", "end 0.5"]);
  assert_eq!(progress.get(), 0.5);
  assert_eq!(color_of(&bars(&mut stage, "wave")[2]), light(PaletteColor::Primary));

  stage.tree.mouse_move(x + width * 0.5, y + 12.0);
  stage.tree.mouse_down(x + width * 0.5, y + 12.0, MouseButton::Left);
  stage.tree.mouse_move(x + width * 0.75, y + 12.0);
  stage.pass();
  stage.tree.mouse_up(x + width * 0.75, y + 12.0, MouseButton::Left);
  stage.pass();
  assert_eq!(progress.get(), 0.75);
  assert_eq!(heard.take().last().map(String::as_str), Some("end 0.75"));
}

#[test]
fn the_keys_move_it_by_its_step_and_it_has_the_ring_of_the_focus() {
  let progress = Signal::new(0.5_f32);
  let read = progress.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(Waveform::new(read.clone(), &PEAKS).step(0.25))
      .into()
  });
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("ArrowRight");
  assert_eq!(progress.get(), 0.75);
  stage.key("Home");
  assert_eq!(progress.get(), 0.0);
  stage.key("End");
  assert_eq!(progress.get(), 1.0);
}

#[test]
fn a_controlled_waveform_tells_the_app_and_shows_what_it_holds() {
  let heard = Heard::default();
  let told = heard.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(Waveform::controlled(0.0, &PEAKS, move |to| told.tell(format!("{to}"))).id("wave"))
      .into()
  });
  stage.click("wave");
  assert_eq!(heard.take(), ["0.5"]);
  // The app kept nothing: nothing is played.
  assert!(
    bars(&mut stage, "wave")
      .iter()
      .all(|bar| color_of(bar) == light(PaletteColor::Input))
  );
}

#[test]
fn shown_and_not_operated_it_is_no_stop_and_a_disabled_one_takes_nothing() {
  let (shown, disabled) = (Signal::new(0.5_f32), Signal::new(0.25_f32));
  let heard = Heard::default();
  let (read_disabled, told) = (disabled.clone(), heard.clone());
  let read_shown = shown.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .spacing(24.0)
      .child(Waveform::at(read_shown.get(), &PEAKS).id("shown"))
      .child(
        Waveform::new(read_disabled.clone(), &PEAKS)
          .disabled(true)
          .on_change(move |to| told.tell(format!("{to}")))
          .id("disabled"),
      )
      .into()
  });
  stage.click("shown");
  stage.click("disabled");
  stage.key("Tab");
  stage.key("ArrowRight");
  assert!(!stage.has_ring(light(PaletteColor::Ring)), "neither is a stop of Tab");
  assert_eq!((shown.get(), disabled.get()), (0.5, 0.25));
  assert!(heard.take().is_empty());

  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<Waveform>(&Case::new().state(WaveformState::Disabled, true), None)
    .part(WaveformPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}
