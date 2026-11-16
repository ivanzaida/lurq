//! A queue: its rows against the board's numbers, the row that plays, what a press and the keys tell, the "more"
//! button, a title longer than its room, and a disabled queue.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  core::Signal,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{AudioQueue, AudioQueueItem},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The board's four tracks, the third unavailable, each with a "more" button that writes to `log`.
fn tracks(log: &Arc<Mutex<Vec<String>>>) -> Vec<AudioQueueItem> {
  [
    ("Night Drive", "Aurora Lane", 250.0),
    ("Paper Boats", "Mira Sol", 232.0),
    ("Glass Garden", "The Fernweh", 301.0),
    ("Low Tide", "Aurora Lane", 198.0),
  ]
  .into_iter()
  .enumerate()
  .map(|(index, (title, by, seconds))| {
    let log = log.clone();
    AudioQueueItem::new(title)
      .description(by)
      .duration(seconds)
      .disabled(index == 2)
      .on_more(move || log.lock().unwrap().push(format!("more {index}")))
      .id(format!("q-{index}"))
  })
  .collect()
}

fn queue(log: &Arc<Mutex<Vec<String>>>, current: Option<usize>) -> AudioQueue {
  let told = log.clone();
  AudioQueue::controlled(current, tracks(log), move |index| {
    told.lock().unwrap().push(format!("change {index}"))
  })
  .playing(true)
  .id("q")
}

fn said(log: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
  log.lock().unwrap().clone()
}

#[test]
fn a_row_is_the_boards() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let shown = log.clone();
  let mut stage = Stage::new(move |_| queue(&shown, Some(0)).build(&lurq::ui::current_theme()));
  let (x, _, width, height) = stage.bounds("q-1");
  assert_eq!(height, 48.0);
  let (_, _, list_width, _) = stage.bounds("q");
  assert_eq!(width, list_width, "a row is as wide as the queue");
  // The number in mono 12 in the muted ink, 8 in.
  assert_eq!(stage.text("2"), (light(PaletteColor::MutedForeground), 12.0));
  let number = stage.text_quad("2").unwrap();
  assert!(number.x >= x + 8.0 && number.x < x + 28.0);
  // The artwork 32 on the muted fill, the title 13, who it is by 12, the length in mono 11.
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| (quad.width, quad.height) == (32.0, 32.0)
        && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Muted)))
  );
  assert_eq!(stage.text("Paper Boats"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Mira Sol"), (light(PaletteColor::MutedForeground), 12.0));
  assert_eq!(stage.text("3:52"), (light(PaletteColor::MutedForeground), 11.0));
}

#[test]
fn the_row_that_plays_has_the_accent_fill_and_the_equalizer() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let shown = log.clone();
  let mut stage = Stage::new(move |_| queue(&shown, Some(1)).build(&lurq::ui::current_theme()));
  assert_eq!(stage.fill_color("q-1"), Some(light(PaletteColor::Highlight)));
  assert_ne!(stage.fill_color("q-0"), Some(light(PaletteColor::Highlight)));
  assert!(!stage.paints("2"), "the equalizer stands for its number");
  assert!(stage.paints("1"));
}

#[test]
fn a_press_tells_the_track_and_changes_nothing() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let shown = log.clone();
  let mut stage = Stage::new(move |_| queue(&shown, Some(0)).build(&lurq::ui::current_theme()));
  stage.click("q-3");
  stage.click("q-0");
  stage.click("q-2");
  assert_eq!(
    said(&log),
    ["change 3"],
    "the current one and an unavailable one tell nothing"
  );
  assert_eq!(
    stage.fill_color("q-0"),
    Some(light(PaletteColor::Highlight)),
    "the app did not answer"
  );
}

#[test]
fn a_bound_queue_sets_the_signal() {
  let current = Signal::new(Some(0_usize));
  let shown = current.clone();
  let mut stage = Stage::new(move |_| {
    AudioQueue::new(shown.clone(), tracks(&Arc::new(Mutex::new(Vec::new())))).build(&lurq::ui::current_theme())
  });
  stage.click("q-1");
  assert_eq!(current.get_untracked(), Some(1));
  assert_eq!(stage.fill_color("q-1"), Some(light(PaletteColor::Highlight)));
}

#[test]
fn the_arrows_move_between_the_rows() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let shown = log.clone();
  let mut stage = Stage::new(move |_| queue(&shown, Some(1)).build(&lurq::ui::current_theme()));
  // Tab enters at the current row; the arrow moves down, past the unavailable one, and Enter plays it.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(said(&log), ["change 3"]);
}

#[test]
fn more_is_its_own_press() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let shown = log.clone();
  let mut stage = Stage::new(move |_| queue(&shown, Some(0)).build(&lurq::ui::current_theme()));
  stage.hover("q-1");
  let (_, _, width, height) = stage.bounds("q-1-more");
  assert_eq!((width, height), (28.0, 28.0));
  stage.click("q-1-more");
  assert_eq!(said(&log), ["more 1"], "the row is not pressed with it");
}

#[test]
fn a_long_title_is_cut_and_the_length_keeps_its_room() {
  let long = "A title far longer than the room a row of the queue leaves it beside its length";
  let mut stage = Stage::new(move |_| {
    lurq::components::Column::new()
      .width(360.0)
      .child(
        AudioQueue::controlled(None, [AudioQueueItem::new(long).duration(198.0).id("r")], |_| {})
          .build(&lurq::ui::current_theme()),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("r");
  assert_eq!(width, 360.0);
  let title = stage.cut_text_quad(long).expect("the title, cut");
  let length = stage.text_quad("3:18").expect("the length");
  assert!(title.x + title.width <= length.x, "the title ends before the length");
  assert!(length.x + length.width <= x + width - 8.0 + 0.5);
}

#[test]
fn a_disabled_queue_takes_nothing() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let shown = log.clone();
  let mut stage = Stage::new(move |_| queue(&shown, Some(0)).disabled(true).build(&lurq::ui::current_theme()));
  stage.click("q-1");
  stage.key("Tab");
  stage.key("Enter");
  assert!(said(&log).is_empty(), "{:?}", said(&log));
}
