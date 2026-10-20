//! Files dragged from the OS reach the element under the pointer, bubble to
//! its ancestors, style it while over it, and are dropped on it.

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{
    Tree,
    events::{DraggedPaths, DroppedPath, FileDragEvent, FileDropEvent, is_synthetic_input},
    synthetic_input::{self, SyntheticInput},
  },
  components::{Column, Row},
  node::color::Color,
};

use crate::support::run_pass;

const REST: &str = "#111111";
const OVER: &str = "#22cc22";

type Log = Arc<Mutex<Vec<String>>>;

fn log(log: &Log, entry: impl Into<String>) {
  log.lock().unwrap().push(entry.into());
}

/// A column with two drop targets, every file-drag handler logging where it
/// ran: `zone` highlights while dragged over.
fn targets(events: &Log) -> Tree {
  let mut runtime = Tree::new();
  let zone_log = events.clone();
  let other_log = events.clone();
  let column_log = events.clone();
  runtime.set_root(
    Column::new()
      .id("column")
      .on_file_drag_enter({
        let log_ = column_log.clone();
        move |event: FileDragEvent| log(&log_, format!("column enter {}", event.paths.len()))
      })
      .on_file_drop({
        let log_ = column_log.clone();
        move |event: FileDropEvent| log(&log_, format!("column drop {}", event.paths.len()))
      })
      .child(
        Row::new()
          .id("zone")
          .width(100.0)
          .height(50.0)
          .background(REST)
          .drag_over(|style| style.background(OVER))
          .on_file_drag_enter({
            let log_ = zone_log.clone();
            move |event: FileDragEvent| log(&log_, format!("zone enter {},{}", event.x, event.y))
          })
          .on_file_drag_over({
            let log_ = zone_log.clone();
            move |event: FileDragEvent| log(&log_, format!("zone over {},{}", event.x, event.y))
          })
          .on_file_drag_leave({
            let log_ = zone_log.clone();
            move || log(&log_, "zone leave")
          })
          .on_file_drop({
            let log_ = zone_log.clone();
            move |event: FileDropEvent| {
              let names: Vec<_> = event
                .paths
                .paths()
                .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
              log(&log_, format!("zone drop {}", names.join("+")));
            }
          }),
      )
      .child(
        Row::new()
          .id("other")
          .width(100.0)
          .height(50.0)
          .background(REST)
          .on_file_drag_enter({
            let log_ = other_log.clone();
            move |_: FileDragEvent| log(&log_, "other enter")
          })
          .on_file_drag_leave({
            let log_ = other_log.clone();
            move || log(&log_, "other leave")
          })
          .on_file_drop({
            let log_ = other_log.clone();
            move |_: FileDropEvent| log(&log_, "other drop")
          }),
      ),
  );
  run_pass(&mut runtime);
  runtime
}

fn center(runtime: &mut Tree, id: &str) -> (f32, f32) {
  runtime
    .get_element_by_id_mut(id)
    .expect(id)
    .bounds()
    .expect("laid out")
    .center()
}

fn color(runtime: &mut Tree, id: &str) -> Option<Color> {
  runtime.get_element_by_id(id).expect(id).color()
}

fn two_files() -> DraggedPaths {
  DraggedPaths::from_paths([PathBuf::from("C:/shots/a.png"), PathBuf::from("C:/shots/b.png")])
}

#[test]
fn a_drag_enters_the_element_under_the_pointer_and_bubbles() {
  let events = Log::default();
  let mut runtime = targets(&events);
  let (x, y) = center(&mut runtime, "zone");

  runtime.file_drag_enter(two_files(), x, y);

  assert_eq!(
    events.lock().unwrap().as_slice(),
    &[
      format!("zone enter {x},{y}"),
      "column enter 2".to_owned(),
      format!("zone over {x},{y}")
    ],
    "enter on the target then its ancestor, with both paths; then over"
  );
  assert!(runtime.has_file_drag());
  assert_eq!(runtime.file_drag_paths().map(DraggedPaths::len), Some(2));
}

#[test]
fn the_drag_over_style_applies_while_the_drag_is_over_the_element() {
  let events = Log::default();
  let mut runtime = targets(&events);
  let zone = center(&mut runtime, "zone");
  let other = center(&mut runtime, "other");
  assert_eq!(color(&mut runtime, "zone"), Some(Color::from_hex(REST)));

  runtime.file_drag_enter(two_files(), zone.0, zone.1);
  assert_eq!(color(&mut runtime, "zone"), Some(Color::from_hex(OVER)));

  runtime.file_drag_over(other.0, other.1);
  assert_eq!(color(&mut runtime, "zone"), Some(Color::from_hex(REST)));
  assert!(
    events.lock().unwrap().contains(&"zone leave".to_owned()),
    "moving off the element leaves it"
  );
  assert!(events.lock().unwrap().contains(&"other enter".to_owned()));

  runtime.file_drag_leave();
  assert!(!runtime.has_file_drag());
  assert!(events.lock().unwrap().last().is_some_and(|last| last == "other leave"));
}

#[test]
fn a_drop_reaches_the_element_under_the_pointer_then_its_ancestors_and_ends_the_drag() {
  let events = Log::default();
  let mut runtime = targets(&events);
  let (x, y) = center(&mut runtime, "zone");
  runtime.file_drag_enter(two_files(), x, y);
  events.lock().unwrap().clear();

  assert!(runtime.file_drop(two_files(), x, y));

  let seen = events.lock().unwrap().clone();
  let zone = seen
    .iter()
    .position(|entry| entry == "zone drop a.png+b.png")
    .expect("zone drop");
  let column = seen
    .iter()
    .position(|entry| entry == "column drop 2")
    .expect("column drop");
  assert!(zone < column, "innermost first: {seen:?}");
  assert!(!seen.contains(&"zone leave".to_owned()), "a drop is not a leave");
  assert!(!runtime.has_file_drag());
  assert_eq!(color(&mut runtime, "zone"), Some(Color::from_hex(REST)));
}

#[test]
fn a_drop_without_an_enter_first_enters_at_the_same_point() {
  let events = Log::default();
  let mut runtime = targets(&events);
  let (x, y) = center(&mut runtime, "other");

  runtime.file_drop(two_files(), x, y);

  let seen = events.lock().unwrap().clone();
  assert_eq!(seen.first().map(String::as_str), Some("other enter"));
  assert!(seen.contains(&"other drop".to_owned()));
}

#[test]
fn stop_propagation_keeps_the_drop_from_the_ancestors() {
  let dropped = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  let (outer, inner) = (dropped.clone(), dropped.clone());
  runtime.set_root(
    Column::new()
      .on_file_drop(move |_: FileDropEvent| outer.lock().unwrap().push("column"))
      .child(
        Row::new()
          .id("zone")
          .width(40.0)
          .height(40.0)
          .on_file_drop(move |event: FileDropEvent| {
            inner.lock().unwrap().push("zone");
            event.stop_propagation();
          }),
      ),
  );
  run_pass(&mut runtime);
  let (x, y) = center(&mut runtime, "zone");

  runtime.file_drop(two_files(), x, y);

  assert_eq!(dropped.lock().unwrap().as_slice(), &["zone"]);
}

#[test]
fn a_drop_on_nothing_is_not_handled() {
  let dropped = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();
  let count = dropped.clone();
  runtime.set_root(
    Column::new().child(
      Row::new()
        .id("zone")
        .width(40.0)
        .height(40.0)
        .on_file_drop(move |_: FileDropEvent| *count.lock().unwrap() += 1),
    ),
  );
  run_pass(&mut runtime);

  assert!(
    !runtime.file_drop(two_files(), 500.0, 500.0),
    "no handler under the pointer"
  );
  assert_eq!(*dropped.lock().unwrap(), 0);
  assert!(!runtime.has_file_drag());
}

#[test]
fn a_refused_drag_shows_no_drag_over_style() {
  let mut runtime = Tree::new();
  runtime.set_root(
    Row::new()
      .id("zone")
      .width(40.0)
      .height(40.0)
      .background(REST)
      .drag_over(|style| style.background(OVER))
      .on_file_drag_enter(|event: FileDragEvent| {
        if event.paths.has_folders() {
          event.refuse();
        }
      }),
  );
  run_pass(&mut runtime);
  let (x, y) = center(&mut runtime, "zone");
  let folder = DraggedPaths::new([DroppedPath::with_kind(PathBuf::from("C:/shots"), true)]);

  runtime.file_drag_enter(folder, x, y);

  assert_eq!(runtime.file_drag_accepted(), Some(false));
  assert_eq!(color(&mut runtime, "zone"), Some(Color::from_hex(REST)));

  // The refusal made on enter holds while the drag moves over the element.
  runtime.file_drag_over(x + 3.0, y + 2.0);
  assert_eq!(runtime.file_drag_accepted(), Some(false));
  assert_eq!(color(&mut runtime, "zone"), Some(Color::from_hex(REST)));

  // Leaving the element forgets it; coming back asks again, with files this
  // time, which the handler accepts.
  runtime.file_drag_over(500.0, 500.0);
  assert_eq!(runtime.file_drag_accepted(), Some(true));
  runtime.file_drag_enter(two_files(), x, y);
  assert_eq!(runtime.file_drag_accepted(), Some(true));
  assert_eq!(color(&mut runtime, "zone"), Some(Color::from_hex(OVER)));
}

/// A refusal belongs to the element whose handler refused: not to its
/// ancestors (the root would refuse the whole window), not to its siblings.
fn refusing_siblings(refuse_on_over: bool) -> Tree {
  let mut runtime = Tree::new();
  let refusing = |row: Row| {
    if refuse_on_over {
      row.on_file_drag_over(|event: FileDragEvent| event.refuse())
    } else {
      row.on_file_drag_enter(|event: FileDragEvent| event.refuse())
    }
  };
  runtime.set_root(
    Column::new()
      .child(refusing(
        Row::new()
          .id("a")
          .width(40.0)
          .height(40.0)
          .background(REST)
          .drag_over(|style| style.background(OVER)),
      ))
      .child(
        Row::new()
          .id("b")
          .width(40.0)
          .height(40.0)
          .background(REST)
          .drag_over(|style| style.background(OVER)),
      ),
  );
  run_pass(&mut runtime);
  runtime
}

#[test]
fn a_refusal_on_enter_stays_with_the_refusing_element() {
  let mut runtime = refusing_siblings(false);
  let a = center(&mut runtime, "a");
  let b = center(&mut runtime, "b");

  runtime.file_drag_enter(two_files(), a.0, a.1);
  assert_eq!(runtime.file_drag_accepted(), Some(false));
  assert_eq!(color(&mut runtime, "a"), Some(Color::from_hex(REST)));

  runtime.file_drag_over(b.0, b.1);

  assert_eq!(runtime.file_drag_accepted(), Some(true), "the sibling did not refuse");
  assert_eq!(color(&mut runtime, "b"), Some(Color::from_hex(OVER)));
  assert_eq!(color(&mut runtime, "a"), Some(Color::from_hex(REST)));
}

#[test]
fn a_refusal_on_over_stays_with_the_refusing_element() {
  let mut runtime = refusing_siblings(true);
  let a = center(&mut runtime, "a");
  let b = center(&mut runtime, "b");

  runtime.file_drag_enter(two_files(), a.0, a.1);
  assert_eq!(runtime.file_drag_accepted(), Some(false));
  runtime.file_drag_over(a.0 + 1.0, a.1);
  assert_eq!(
    runtime.file_drag_accepted(),
    Some(false),
    "still over the refusing element"
  );

  runtime.file_drag_over(b.0, b.1);

  assert_eq!(runtime.file_drag_accepted(), Some(true), "the sibling did not refuse");
  assert_eq!(color(&mut runtime, "b"), Some(Color::from_hex(OVER)));
}

#[test]
fn folders_are_reported_as_folders() {
  let paths = DraggedPaths::new([
    DroppedPath::with_kind(PathBuf::from("C:/shots/a.png"), false),
    DroppedPath::with_kind(PathBuf::from("C:/shots"), true),
  ]);

  assert!(paths.has_folders());
  assert_eq!(paths.files().count(), 1);
  assert_eq!(paths.folders().count(), 1);
  assert_eq!(paths.items()[1].path(), PathBuf::from("C:/shots").as_path());
  // A path that does not exist is a file: Explorer and Finder drag only
  // paths that exist, and `from_paths` looks each up once.
  let looked_up = DraggedPaths::from_paths([PathBuf::from("Z:/nowhere/a.png")]);
  assert!(!looked_up.has_folders());
  assert!(DraggedPaths::from_paths([std::env::temp_dir()]).has_folders());
}

#[test]
fn synthetic_file_drags_reach_the_handlers_marked_synthetic() {
  let synthetic = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  let (enter, drop) = (synthetic.clone(), synthetic.clone());
  runtime.set_root(
    Row::new()
      .id("zone")
      .width(40.0)
      .height(40.0)
      .on_file_drag_enter(move |_: FileDragEvent| enter.lock().unwrap().push(("enter", is_synthetic_input())))
      .on_file_drop(move |_: FileDropEvent| drop.lock().unwrap().push(("drop", is_synthetic_input()))),
  );
  run_pass(&mut runtime);
  let (x, y) = center(&mut runtime, "zone");

  synthetic_input::apply(&mut runtime, &SyntheticInput::file_drag_enter(two_files(), x, y));
  synthetic_input::apply(&mut runtime, &SyntheticInput::file_drag_over(x + 1.0, y));
  synthetic_input::apply(&mut runtime, &SyntheticInput::file_drag_leave());
  assert!(!runtime.has_file_drag());
  synthetic_input::apply(&mut runtime, &SyntheticInput::file_drop(two_files(), x, y));

  assert_eq!(
    synthetic.lock().unwrap().as_slice(),
    &[("enter", true), ("enter", true), ("drop", true)]
  );
}
