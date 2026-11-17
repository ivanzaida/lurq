//! What a phone's fingers do that the engine makes by itself: one finger's zoom, three fingers' editing commands
//! and their menu, the loupe and the handles of text by touch, the mouse as fingers, a pen's hover, and the finger's
//! model as a shell, the emulator and MCP drive it ([`Tree::finger`], [`Tree::finger_tick`]).

use std::{
  cell::RefCell,
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use super::{HeadlessSurface, Tree, mouse_fingers::MouseFingers};
use crate::{
  app::{
    App, Pointer,
    component::Component,
    ctx::Ctx,
    events::{CopyEvent, EditCommand, EditEvent, LongPressEvent, PasteEvent, PinchEvent, TapEvent, TouchPhase},
  },
  components::{Column, Rect, ScrollVertical, Spacer, TextInput},
  core::Signal,
  node::Element,
};

type Log = Arc<Mutex<Vec<String>>>;

fn logged(log: &Log) -> Vec<String> {
  std::mem::take(&mut *log.lock().unwrap())
}

fn after(start: Instant, millis: u64) -> Instant {
  start + Duration::from_millis(millis)
}

thread_local! {
  /// What the scene of the test on this thread logs, and the value of its text input.
  static LOG: RefCell<Log> = RefCell::new(Log::default());
  static VALUE: RefCell<Signal<String>> = RefCell::new(Signal::new(String::new()));
}

/// A box of 400 that takes the focus, listens for pinches and taps, and logs them and the editing commands that
/// reach it; mounted in an app, in a window of 400.
struct Board {
  log: Log,
}

impl Component for Board {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      log: LOG.with(|log| log.borrow().clone()),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let log = self.log.clone();
    let push = move |line: String| log.lock().unwrap().push(line);
    let (pinch, tap, undo, redo, copy, paste) = (
      push.clone(),
      push.clone(),
      push.clone(),
      push.clone(),
      push.clone(),
      push,
    );
    Column::new().child(
      Rect::new(400.0, 400.0)
        .focusable(true)
        .on_pinch(move |event: PinchEvent| pinch(format!("pinch {:?} {:.2}", event.phase, event.scale)))
        .on_tap(move |event: TapEvent| tap(format!("tap of {}", event.fingers)))
        .on_undo(move |_: EditEvent| undo("undo".into()))
        .on_redo(move |_: EditEvent| redo("redo".into()))
        .on_copy(move |_: CopyEvent| copy("copy".into()))
        .on_paste(move |_: PasteEvent| paste("paste".into())),
    )
  }
}

/// A clipboard of the test's, which the OS never sees: it holds "pasted" and keeps nothing written.
struct TestClipboard;

impl super::ClipboardReader for TestClipboard {
  fn read_text(&self) -> Option<String> {
    Some("pasted".into())
  }

  fn read(&self) -> Option<crate::app::events::ClipboardContent> {
    Some(crate::app::events::ClipboardContent::text("pasted"))
  }
}

impl super::ClipboardWriter for TestClipboard {
  fn write_text(&self, _text: &str) -> bool {
    true
  }
}

fn board() -> (Tree, App, Log) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 400);
  tree.set_clipboard_reader(Some(Arc::new(TestClipboard)));
  tree.set_clipboard_writer(Some(Arc::new(TestClipboard)));
  let log = Log::default();
  LOG.with(|cell| *cell.borrow_mut() = log.clone());
  tree.mount_root::<Board>(&mut app, ());
  tree.pass(&mut app, &HeadlessSurface);
  (tree, app, log)
}

/// One finger's tap at a point, at a time, as the pointer too: what it taps is pressed, focused and clicked.
fn tap(tree: &mut Tree, id: u64, at: (f32, f32), when: Instant) {
  tree.finger_at(id, TouchPhase::Start, at.0, at.1, None, false, when);
  tree.finger_at(
    id,
    TouchPhase::End,
    at.0,
    at.1,
    None,
    false,
    when + Duration::from_millis(40),
  );
}

/// Fingers landing at `from`, going by `by` in four steps over 100 milliseconds, and lifting.
fn fingers(tree: &mut Tree, from: &[(f32, f32)], to: &[(f32, f32)], start: Instant) {
  for (index, (x, y)) in from.iter().enumerate() {
    tree.touch_event_at(10 + index as u64, TouchPhase::Start, *x, *y, None, start);
  }
  for step in 1..=4 {
    let share = step as f32 / 4.0;
    for (index, ((fx, fy), (tx, ty))) in from.iter().zip(to).enumerate() {
      tree.touch_event_at(
        10 + index as u64,
        TouchPhase::Move,
        fx + (tx - fx) * share,
        fy + (ty - fy) * share,
        None,
        after(start, 25 * step),
      );
    }
  }
  for (index, (x, y)) in to.iter().enumerate() {
    tree.touch_event_at(10 + index as u64, TouchPhase::End, *x, *y, None, after(start, 101));
  }
}

#[test]
fn a_double_tap_held_and_dragged_zooms_with_one_finger() {
  let (mut tree, _app, log) = board();
  let start = Instant::now();

  tap(&mut tree, 1, (200.0, 200.0), start);
  assert_eq!(logged(&log), ["tap of 1"]);
  // The second landing, held and dragged down: past the slop it zooms, by two for 150 points.
  tree.touch_event_at(2, TouchPhase::Start, 200.0, 200.0, None, after(start, 150));
  tree.touch_event_at(2, TouchPhase::Move, 200.0, 220.0, None, after(start, 170));
  tree.touch_event_at(2, TouchPhase::Move, 200.0, 370.0, None, after(start, 250));
  tree.touch_event_at(2, TouchPhase::Move, 200.0, 70.0, None, after(start, 350));
  tree.touch_event_at(2, TouchPhase::End, 200.0, 70.0, None, after(start, 400));
  assert_eq!(
    logged(&log),
    [
      "pinch Start 1.00",
      "pinch Change 2.00",
      "pinch Change 0.50",
      "pinch End 0.50"
    ]
  );

  // A drag with no tap before it is no zoom.
  tree.touch_event_at(3, TouchPhase::Start, 200.0, 200.0, None, after(start, 2000));
  tree.touch_event_at(3, TouchPhase::Move, 200.0, 300.0, None, after(start, 2050));
  tree.touch_event_at(3, TouchPhase::End, 200.0, 300.0, None, after(start, 2100));
  assert!(logged(&log).iter().all(|line| !line.starts_with("pinch")));
}

#[test]
fn three_fingers_swiped_left_undo_and_right_redo_on_the_focus() {
  let (mut tree, _app, log) = board();
  let start = Instant::now();
  tap(&mut tree, 1, (200.0, 200.0), start);
  logged(&log);

  let row = [(140.0, 200.0), (200.0, 200.0), (260.0, 200.0)];
  let left = row.map(|(x, y)| (x - 120.0, y));
  fingers(&mut tree, &row, &left, after(start, 1000));
  assert_eq!(logged(&log), ["undo"]);
  let right = row.map(|(x, y)| (x + 120.0, y));
  fingers(&mut tree, &row, &right, after(start, 2000));
  assert_eq!(logged(&log), ["redo"]);
}

#[test]
fn three_fingers_pinched_in_copy_and_out_paste() {
  let (mut tree, _app, log) = board();
  let start = Instant::now();
  tap(&mut tree, 1, (200.0, 200.0), start);
  logged(&log);

  let wide = [(200.0, 100.0), (110.0, 260.0), (290.0, 260.0)];
  let near = wide.map(|(x, y)| (200.0 + (x - 200.0) * 0.3, 207.0 + (y - 207.0) * 0.3));
  fingers(&mut tree, &wide, &near, after(start, 1000));
  assert_eq!(logged(&log), ["copy"]);
  fingers(&mut tree, &near, &wide, after(start, 2000));
  assert_eq!(logged(&log), ["paste"]);
  // Synthetic input's paste reads no clipboard: it is refused, as its Ctrl+V is.
  crate::app::events::with_synthetic_input(|| fingers(&mut tree, &near, &wide, after(start, 3000)));
  assert_eq!(logged(&log), [] as [&str; 0]);
}

#[test]
fn three_fingers_tapped_open_the_menu_of_editing_commands_unless_their_tap_is_taken() {
  let (mut tree, mut app, log) = board();
  let start = Instant::now();
  let row = [(140.0, 200.0), (200.0, 200.0), (260.0, 200.0)];
  for (index, (x, y)) in row.iter().enumerate() {
    tree.touch_event_at(index as u64, TouchPhase::Start, *x, *y, None, start);
  }
  for (index, (x, y)) in row.iter().enumerate() {
    tree.touch_event_at(index as u64, TouchPhase::End, *x, *y, None, after(start, 60));
  }
  assert_eq!(logged(&log), ["tap of 3"]);
  let menu = tree.window().edit_menu().expect("the menu opened");
  assert_eq!((menu.x, menu.y), (200.0, 200.0));
  assert_eq!(
    menu.commands,
    [
      EditCommand::Undo,
      EditCommand::Cut,
      EditCommand::Copy,
      EditCommand::Paste,
      EditCommand::Redo
    ]
  );

  // It is shown over the app, a bar of the commands in the locale's words.
  tree.pass(&mut app, &HeadlessSurface);
  let bar = tree.find_element(|element| element.id() == Some("lurq-edit-menu"));
  assert!(bar.is_some(), "the bar is in the tree");
}

#[test]
fn a_command_of_the_menu_runs_on_the_focus_and_closes_the_menu() {
  let (mut tree, mut app, log) = board();
  let start = Instant::now();
  tap(&mut tree, 1, (200.0, 200.0), start);
  logged(&log);
  tree.open_edit_menu((200.0, 200.0), vec![EditCommand::Copy, EditCommand::Paste]);
  tree.pass(&mut app, &HeadlessSurface);
  let copy = tree
    .find_element(|element| element.text_content() == Some("Copy"))
    .expect("the Copy command");
  let (x, y) = copy.bounds().center();

  // A finger chooses it: the focus stays on the board, which is told.
  tap(&mut tree, 2, (x, y), after(start, 1000));
  tree.pass(&mut app, &HeadlessSurface);
  assert_eq!(logged(&log), ["copy"]);
  assert_eq!(tree.window().edit_menu(), None);
}

/// A button of 100 at the top left that logs its long press, in a list 300 high of 1000 of content beside it.
fn scene() -> (Tree, App, Log, crate::layout::layout_kind::ScrollState) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 400);
  let log = Log::default();
  let long = log.clone();
  let state = crate::layout::layout_kind::ScrollState::new();
  tree.set_root(
    Column::new()
      .child(
        Rect::new(100.0, 100.0).on_long_press(move |_: LongPressEvent| long.lock().unwrap().push("long press".into())),
      )
      .child(
        ScrollVertical::new(Spacer::new().width(100.0).height(1000.0))
          .with_scroll_state(state.clone())
          .size(100.0, 300.0),
      ),
  );
  tree.pass(&mut app, &HeadlessSurface);
  (tree, app, log, state)
}

#[test]
fn a_finger_held_still_is_a_long_press_on_the_trees_clock() {
  let (mut tree, _app, log, _) = scene();
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, 50.0, 50.0, None);
  tree.finger_tick(after(start, 300));
  assert_eq!(logged(&log), [] as [&str; 0]);
  tree.finger_tick(after(start, 600));
  assert_eq!(logged(&log), ["long press"]);
  tree.finger(1, TouchPhase::End, 50.0, 50.0, None);
  assert!(tree.finger_deadline().is_none());
}

#[test]
fn a_flick_to_a_lists_end_carries_it_past_and_springs_it_back() {
  let (mut tree, _app, _log, state) = scene();
  let start = Instant::now();
  // Near its end, flicked up fast.
  state.set_scroll(0.0, 690.0);
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, 50.0, 350.0, None);
  for step in 1..=5u64 {
    tree.set_press_clock(Some(after(start, step * 10)));
    tree.finger(1, TouchPhase::Move, 50.0, 350.0 - step as f32 * 30.0, None);
  }
  tree.finger(1, TouchPhase::End, 50.0, 200.0, None);
  let mut most = 0.0f32;
  for step in 1..=200u64 {
    tree.finger_tick(after(start, 50 + step * 8));
    most = most.max(state.overscroll().1);
  }
  assert_eq!(state.scroll_y(), 700.0, "at its end");
  assert!(most > 1.0, "carried past it: {most}");
  assert_eq!(state.overscroll(), (0.0, 0.0), "and back");
}

/// A text input of 300 by 40 at the top left of a window of 400, mounted in an app.
struct Field;

impl Component for Field {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let value = VALUE.with(|value| value.borrow().clone());
    Column::new().child(TextInput::new(value).width(300.0).height(40.0))
  }
}

fn field(text: &str) -> (Tree, App) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 400);
  VALUE.with(|value| *value.borrow_mut() = Signal::new(text.to_owned()));
  tree.set_clipboard_reader(Some(Arc::new(TestClipboard)));
  tree.set_clipboard_writer(Some(Arc::new(TestClipboard)));
  tree.mount_root::<Field>(&mut app, ());
  tree.pass(&mut app, &HeadlessSurface);
  (tree, app)
}

/// Lays the tree out and resolves what it paints, as a frame does: where the focused input's text is drawn.
fn paint(tree: &mut Tree, app: &mut App) {
  tree.pass(app, &HeadlessSurface);
  let layout = tree.last_layout.as_ref().expect("laid out");
  let _ = tree.resolve_quads(layout);
}

/// Where the caret before a byte of the focused input's text is drawn: its top-left corner, and a line's height.
fn caret_at(tree: &Tree, index: usize) -> (f32, f32, f32) {
  let geometry = tree.text_input_snapshot().unwrap().geometry.unwrap();
  let (_, x, y) = *geometry.carets.iter().find(|(at, ..)| *at == index).unwrap();
  (x, y, geometry.line_height)
}

#[test]
fn a_long_press_in_text_moves_the_caret_with_the_loupe_and_opens_the_menu() {
  let (mut tree, mut app) = field("hello world");
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, 10.0, 20.0, None);
  tree.finger(1, TouchPhase::End, 10.0, 20.0, None);
  paint(&mut tree, &mut app);
  let (wx, wy, line) = caret_at(&tree, 6);
  let (dx, ..) = caret_at(&tree, 9);

  tree.set_press_clock(Some(after(start, 1000)));
  tree.finger(2, TouchPhase::Start, wx + 1.0, wy + line / 2.0, None);
  tree.finger_tick(after(start, 1600));
  assert!(tree.touch_caret_moving(), "the loupe is up");
  assert_eq!(tree.text_input_snapshot().unwrap().selection, 6..6);
  paint(&mut tree, &mut app);
  // The finger goes on: the caret with it, and nothing is selected.
  tree.finger(2, TouchPhase::Move, dx + 1.0, wy + line / 2.0, None);
  assert_eq!(tree.text_input_snapshot().unwrap().selection, 9..9);
  tree.finger(2, TouchPhase::End, dx + 1.0, wy + line / 2.0, None);
  assert!(!tree.touch_caret_moving());
  assert_eq!(
    tree.window().edit_menu().map(|menu| menu.commands),
    Some(vec![EditCommand::SelectAll, EditCommand::Paste])
  );
}

#[test]
fn a_double_tap_selects_a_word_with_handles_to_drag_and_the_menu_over_it() {
  let (mut tree, mut app) = field("hello world");
  let start = Instant::now();
  // A tap focuses it.
  tree.set_press_clock(Some(start));
  tree.finger(9, TouchPhase::Start, 280.0, 20.0, None);
  tree.finger(9, TouchPhase::End, 280.0, 20.0, None);
  paint(&mut tree, &mut app);
  let (hx, hy, line) = caret_at(&tree, 2);
  let start = after(start, 1000);
  for (index, at) in [0u64, 120].into_iter().enumerate() {
    tree.set_press_clock(Some(after(start, at)));
    tree.finger(index as u64 + 1, TouchPhase::Start, hx, hy + line / 2.0, None);
    tree.finger(index as u64 + 1, TouchPhase::End, hx, hy + line / 2.0, None);
  }
  paint(&mut tree, &mut app);
  assert_eq!(tree.text_input_snapshot().unwrap().selection, 0..5, "the word");
  assert_eq!(tree.pointer(), Pointer::Coarse);
  assert_eq!(
    tree.window().edit_menu().map(|menu| menu.commands),
    Some(vec![
      EditCommand::Cut,
      EditCommand::Copy,
      EditCommand::Paste,
      EditCommand::SelectAll
    ])
  );

  // The end's handle hangs under the line: a finger there drags the end over the next word.
  let (ex, ey, _) = caret_at(&tree, 5);
  let (dx, ..) = caret_at(&tree, 11);
  tree.set_press_clock(Some(after(start, 2000)));
  assert_eq!(
    tree.finger(5, TouchPhase::Start, ex, ey + line + 5.0, None),
    super::TouchRoute::Taken
  );
  assert_eq!(tree.window().edit_menu(), None, "the menu gives way to the drag");
  tree.finger(5, TouchPhase::Move, dx, ey + line + 5.0, None);
  assert_eq!(tree.text_input_snapshot().unwrap().selection, 0..11);
  paint(&mut tree, &mut app);
  tree.finger(5, TouchPhase::End, dx, ey + line + 5.0, None);
  assert!(
    tree.window().edit_menu().is_some(),
    "and opens again over the selection"
  );
}

#[test]
fn the_mouse_as_fingers_taps_pinches_and_makes_three_fingers() {
  let (mut tree, _app, log) = board();
  let mut mouse = MouseFingers::default();

  // A click is a tap.
  mouse.moved(&mut tree, 200.0, 200.0);
  mouse.pressed(&mut tree);
  mouse.released(&mut tree);
  assert_eq!(logged(&log), ["tap of 1"]);

  // Alt draws two fingers, the second mirrored about the cursor; a drag away from where it began pinches.
  mouse.modifiers(&mut tree, true, false, false);
  assert_eq!(tree.touch_marks, [(220.0, 200.0), (180.0, 200.0)]);
  mouse.pressed(&mut tree);
  for x in [210.0, 230.0, 260.0] {
    mouse.moved(&mut tree, x, 200.0);
  }
  mouse.released(&mut tree);
  let lines = logged(&log);
  assert_eq!(lines.first().map(String::as_str), Some("pinch Start 1.00"));
  let end: f32 = lines
    .iter()
    .find_map(|line| line.strip_prefix("pinch End "))
    .and_then(|scale| scale.parse().ok())
    .expect("the pinch ended");
  assert!(end > 2.0, "{lines:?}");

  // Alt+Shift+Ctrl: three fingers side by side; dragged left fast, they undo.
  mouse.modifiers(&mut tree, true, true, true);
  assert_eq!(tree.touch_marks.len(), 3);
  mouse.moved(&mut tree, 300.0, 200.0);
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  mouse.pressed(&mut tree);
  for step in 1..=4u64 {
    tree.set_press_clock(Some(after(start, step * 20)));
    mouse.moved(&mut tree, 300.0 - step as f32 * 40.0, 200.0);
  }
  mouse.released(&mut tree);
  assert_eq!(logged(&log), ["undo"]);

  // No modifier, no dots.
  mouse.modifiers(&mut tree, false, false, false);
  assert!(tree.touch_marks.is_empty());
}

#[test]
fn a_pens_hover_hovers_and_leaves_the_pointer_as_it_is() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 400);
  tree.set_root(Column::new().child(Rect::new(100.0, 100.0).on_mouse_enter(|| {})));
  tree.pass(&mut app, &HeadlessSurface);
  let hovered = |tree: &Tree| {
    let (root, layout) = (tree.root.as_ref().unwrap(), tree.last_layout.as_ref().unwrap());
    let mut hits = Vec::new();
    super::hit_test_tree(
      root,
      layout,
      0.0,
      0.0,
      50.0,
      50.0,
      &crate::app::theme::ThemeRadii::default(),
      &mut hits,
    );
    hits.first().unwrap().0.style_state.is_hovered()
  };

  tree.pen_hover(50.0, 50.0);
  assert!(hovered(&tree));
  tree.pen_hover_ended();
  assert!(!hovered(&tree));
}

/// Where the menu of editing commands stands, in logical pixels: x, y, width, height.
fn menu_bounds(tree: &mut Tree) -> (f32, f32, f32, f32) {
  tree
    .find_element(|element| element.id() == Some("lurq-edit-menu"))
    .expect("the menu")
    .rect()
}

#[test]
fn the_menu_stands_centred_above_the_point_from_its_first_frame_inside_the_window_and_its_safe_area() {
  let (mut tree, mut app, _log) = board();
  tree.open_edit_menu(
    (200.0, 200.0),
    vec![EditCommand::Cut, EditCommand::Copy, EditCommand::Paste],
  );
  tree.pass(&mut app, &HeadlessSurface);
  let first = menu_bounds(&mut tree);
  let (x, y, width, height) = first;
  assert!(
    ((x + width / 2.0) - 200.0).abs() < 0.5,
    "centred over the point: {first:?}"
  );
  assert!(
    ((y + height) - (200.0 - 12.0)).abs() < 0.5,
    "above it by the gap: {first:?}"
  );
  tree.request_redraw();
  tree.pass(&mut app, &HeadlessSurface);
  assert_eq!(menu_bounds(&mut tree), first, "the first frame is the settled one");

  // At the window's edge it stays inside it, and with no room above it stands below the point.
  tree.open_edit_menu(
    (5.0, 5.0),
    vec![EditCommand::Cut, EditCommand::Copy, EditCommand::Paste],
  );
  tree.pass(&mut app, &HeadlessSurface);
  let (x, y, ..) = menu_bounds(&mut tree);
  assert!(x >= 0.0, "{x}");
  assert!((y - (5.0 + 12.0)).abs() < 0.5, "below: {y}");

  // A phone's status bar is not room: under one of 47, a point at 80 has no room above.
  tree.window().set_safe_area(crate::app::window::SafeAreaInsets {
    top: 47.0,
    ..Default::default()
  });
  tree.open_edit_menu((200.0, 80.0), vec![EditCommand::Copy]);
  tree.pass(&mut app, &HeadlessSurface);
  let (_, y, ..) = menu_bounds(&mut tree);
  assert!(y >= 47.0, "clear of the status bar: {y}");
  assert!((y - (80.0 + 12.0)).abs() < 0.5, "below the point: {y}");
}

#[test]
fn the_wheel_with_alt_shift_ctrl_pinches_three_fingers() {
  let (mut tree, _app, log) = board();
  let mut mouse = MouseFingers::default();
  mouse.moved(&mut tree, 200.0, 200.0);
  mouse.pressed(&mut tree);
  mouse.released(&mut tree);
  logged(&log);

  // Without the keys the wheel is the wheel's.
  assert!(!mouse.wheel(&mut tree, 120.0));
  mouse.modifiers(&mut tree, true, true, true);
  // Toward the person: the three close, drawn where they are; letting go of the keys lifts them, a copy.
  for _ in 0..3 {
    assert!(mouse.wheel(&mut tree, -40.0));
  }
  assert_eq!(tree.touch_marks.len(), 3);
  mouse.modifiers(&mut tree, false, false, false);
  assert_eq!(logged(&log), ["copy"]);
  assert!(tree.touch_marks.is_empty());
  // Away: they spread, a paste.
  mouse.modifiers(&mut tree, true, true, true);
  for _ in 0..3 {
    mouse.wheel(&mut tree, 40.0);
  }
  mouse.modifiers(&mut tree, false, false, false);
  assert_eq!(logged(&log), ["paste"]);
}
