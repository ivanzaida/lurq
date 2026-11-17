//! Going back and forward in one place (`NavigationCommand`): every source gives the command, and the command goes
//! from where it starts out to the root, `on_history_back` / `on_history_forward` innermost first, a handler that
//! passes it on letting it go on.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    Tree,
    events::{
      HistoryEvent, HistorySwipeEvent, KeyboardEvent, MouseButton, NavigationCommand, NavigationSource, ScrollPhase,
    },
  },
  components::{Column, Rect, Row},
  core::NodeId,
};

use crate::support::{pointer_click, run_pass};

type Log = Arc<Mutex<Vec<String>>>;

fn seen(log: &Log) -> Vec<String> {
  std::mem::take(&mut *log.lock().unwrap())
}

/// A handler that writes `name`, the command and where it came from, and passes it on where `passes`.
fn hears(log: &Log, name: &'static str, passes: bool) -> impl Fn(HistoryEvent) + Send + Sync + 'static {
  let log = log.clone();
  move |event: HistoryEvent| {
    log
      .lock()
      .unwrap()
      .push(format!("{name} {:?} {:?}", event.command(), event.source()));
    if passes {
      event.pass_on();
    }
  }
}

/// A page of 400x200: a panel on the left (`panel`, which passes the command on, around a focusable `field`) and one
/// on the right (`aside`, which takes it), in a page that takes what reaches it.
fn page(log: &Log) -> Tree {
  let mut tree = Tree::new();
  tree.resize(400, 200);
  tree.set_root(
    Row::new()
      .size(400.0, 200.0)
      .spacing(0.0)
      .on_history_back(hears(log, "page", false))
      .on_history_forward(hears(log, "page", false))
      .child(
        Column::new()
          .id("panel")
          .size(200.0, 200.0)
          .on_history_back(hears(log, "panel", true))
          .on_history_forward(hears(log, "panel", true))
          .child(
            Rect::new(100.0, 40.0)
              .id("field")
              .focusable(true)
              .on_history_back(hears(log, "field", true)),
          ),
      )
      .child(
        Column::new()
          .id("aside")
          .focusable(true)
          .size(200.0, 200.0)
          .on_history_back(hears(log, "aside", false))
          .on_history_forward(hears(log, "aside", false)),
      ),
  );
  run_pass(&mut tree);
  tree
}

fn focus(tree: &mut Tree, id: &str) {
  tree.get_element_by_id_mut(id).unwrap().focus();
  run_pass(tree);
}

/// Alt and an arrow on Windows and Linux, Cmd and a bracket on Apple's: the key lurq is built to read.
fn back_key(tree: &mut Tree) {
  if cfg!(any(target_os = "macos", target_os = "ios")) {
    tree.key_down_with_meta("[".into(), "BracketLeft".into(), false, false, false, true);
  } else {
    tree.key_down("ArrowLeft".into(), "ArrowLeft".into(), false, false, true);
  }
}

fn forward_key(tree: &mut Tree) {
  if cfg!(any(target_os = "macos", target_os = "ios")) {
    tree.key_down_with_meta("]".into(), "BracketRight".into(), false, false, false, true);
  } else {
    tree.key_down("ArrowRight".into(), "ArrowRight".into(), false, false, true);
  }
}

#[test]
fn a_key_goes_from_the_focused_element_out_and_a_handler_that_passes_it_on_lets_it_go_on() {
  let log = Log::default();
  let mut tree = page(&log);
  focus(&mut tree, "field");

  back_key(&mut tree);
  assert_eq!(
    seen(&log),
    ["field Back Key", "panel Back Key", "page Back Key"],
    "the field and the panel pass it on; the page takes it"
  );

  forward_key(&mut tree);
  assert_eq!(
    seen(&log),
    ["panel Forward Key", "page Forward Key"],
    "the field hears no forward"
  );
}

#[test]
fn the_first_element_that_does_not_pass_it_on_takes_it() {
  let log = Log::default();
  let mut tree = page(&log);
  focus(&mut tree, "aside");

  back_key(&mut tree);
  assert_eq!(seen(&log), ["aside Back Key"], "the page around it is not told");
}

#[test]
fn a_key_that_is_the_command_is_no_other_key() {
  let log = Log::default();
  let keys = Log::default();
  let mut tree = Tree::new();
  let heard = keys.clone();
  tree.set_root(
    Column::new()
      .size(200.0, 100.0)
      .on_history_back(hears(&log, "page", false))
      .on_key_down(move |event: KeyboardEvent| heard.lock().unwrap().push(event.key.clone())),
  );
  run_pass(&mut tree);
  back_key(&mut tree);
  assert_eq!(seen(&log), ["page Back Key"]);
  assert_eq!(seen(&keys).len(), 1, "on_key_down hears the key first");
}

#[test]
fn a_handler_of_the_key_that_prevents_its_default_keeps_it_from_being_the_command() {
  let log = Log::default();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .size(200.0, 100.0)
      .on_history_back(hears(&log, "page", false))
      .on_key_down(|event: KeyboardEvent| {
        if event.navigation_command().is_some() {
          event.prevent_default();
        }
      }),
  );
  run_pass(&mut tree);
  back_key(&mut tree);
  assert!(seen(&log).is_empty());
}

#[test]
fn a_keyboards_back_key_and_androids_back_go_back_on_every_platform() {
  let log = Log::default();
  let mut tree = page(&log);
  focus(&mut tree, "field");
  tree.key_down("BrowserBack".into(), String::new(), false, false, false);
  assert_eq!(seen(&log), ["field Back Key", "panel Back Key", "page Back Key"]);
  tree.key_down("BrowserForward".into(), "BrowserForward".into(), false, false, false);
  assert_eq!(seen(&log), ["panel Forward Key", "page Forward Key"]);
}

#[test]
fn with_nothing_focused_a_key_starts_at_the_last_element_that_hears_it() {
  let log = Log::default();
  let mut tree = page(&log);
  back_key(&mut tree);
  assert_eq!(
    seen(&log),
    ["aside Back Key"],
    "the aside is the last that hears a back"
  );
}

#[test]
fn the_mouses_side_buttons_go_from_what_is_under_the_pointer() {
  let log = Log::default();
  let mut tree = page(&log);
  // The focus is in the panel; the pointer is over the panel, away from the field.
  focus(&mut tree, "field");
  pointer_click(&mut tree, 50.0, 150.0, MouseButton::Back);
  assert_eq!(
    seen(&log),
    ["panel Back Mouse", "page Back Mouse"],
    "under the pointer: the panel passes it on and the page takes it; the focused field is not asked"
  );

  pointer_click(&mut tree, 300.0, 100.0, MouseButton::Forward);
  assert_eq!(seen(&log), ["aside Forward Mouse"], "over the aside, which takes it");
}

#[test]
fn only_the_release_of_a_side_button_goes() {
  let log = Log::default();
  let mut tree = page(&log);
  tree.mouse_down(300.0, 100.0, MouseButton::Back);
  assert!(seen(&log).is_empty());
  tree.mouse_up(300.0, 100.0, MouseButton::Back);
  assert_eq!(seen(&log), ["aside Back Mouse"]);
  pointer_click(&mut tree, 300.0, 100.0, MouseButton::Other(8));
  assert!(seen(&log).is_empty(), "another button is no command");
}

/// A two-finger gesture on a trackpad at `at`, `dx` sideways in all.
fn trackpad_swipe(tree: &mut Tree, at: (f32, f32), dx: f32) {
  tree.scroll(at.0, at.1, 0.0, 0.0, ScrollPhase::Start);
  for _ in 0..10 {
    tree.scroll(at.0, at.1, dx / 10.0, 0.0, ScrollPhase::Scroll);
  }
  tree.scroll(at.0, at.1, 0.0, 0.0, ScrollPhase::End);
}

#[test]
fn a_trackpads_swipe_is_the_command_from_under_the_pointer() {
  let log = Log::default();
  let mut tree = page(&log);
  tree.set_history_swipes(true);
  trackpad_swipe(&mut tree, (300.0, 100.0), 200.0);
  assert_eq!(seen(&log), ["aside Back Swipe"]);
}

#[test]
fn a_swipe_whose_default_is_prevented_goes_nowhere() {
  let log = Log::default();
  let swipes = Log::default();
  let heard = swipes.clone();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .size(400.0, 200.0)
      .on_history_back(hears(&log, "page", false))
      .on_history_swipe(move |event: HistorySwipeEvent| {
        heard.lock().unwrap().push(format!("{:?}", event.direction));
        event.prevent_default();
      }),
  );
  run_pass(&mut tree);
  tree.set_history_swipes(true);
  trackpad_swipe(&mut tree, (200.0, 100.0), -200.0);
  assert_eq!(seen(&swipes), ["Forward"], "the swipe is told as before");
  assert!(seen(&log).is_empty(), "and goes nowhere");
}

#[test]
fn a_finger_from_the_edge_is_the_command_where_something_hears_it() {
  let log = Log::default();
  let mut tree = page(&log);
  let start = lurq::time::Instant::now();
  tree.set_press_clock(Some(start));
  // From the left edge of a 400 wide window, across more than a third of it.
  tree.finger(1, lurq::app::events::TouchPhase::Start, 1.0, 100.0, None);
  for step in 1..=10 {
    tree.set_press_clock(Some(start + std::time::Duration::from_millis(30 * step)));
    tree.finger(
      1,
      lurq::app::events::TouchPhase::Move,
      1.0 + 25.0 * step as f32,
      100.0,
      None,
    );
  }
  tree.finger(1, lurq::app::events::TouchPhase::End, 251.0, 100.0, None);
  assert_eq!(
    seen(&log),
    ["panel Back Edge", "page Back Edge"],
    "from where the finger landed, in the panel, which passes it on to the page"
  );
}

#[test]
fn the_app_runs_it_without_input_from_the_focus() {
  let log = Log::default();
  let mut tree = page(&log);
  focus(&mut tree, "field");
  assert!(tree.run_navigation(NavigationCommand::Back));
  assert_eq!(seen(&log), ["field Back App", "panel Back App", "page Back App"]);
}

#[test]
fn nothing_hears_it_where_nothing_has_a_handler_or_a_router() {
  let mut tree = Tree::new();
  tree.set_root(Column::new().size(100.0, 100.0));
  run_pass(&mut tree);
  assert!(!tree.run_navigation(NavigationCommand::Back));
}

#[test]
fn a_handler_is_told_where_the_command_started() {
  let origin = Arc::new(Mutex::new(None));
  let told = origin.clone();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .size(200.0, 100.0)
      .on_history_back(move |event: HistoryEvent| {
        *told.lock().unwrap() = Some((event.origin_id, event.target_id, event.source()));
      })
      .child(Rect::new(50.0, 50.0).id("inner").focusable(true)),
  );
  run_pass(&mut tree);
  focus(&mut tree, "inner");
  let inner = tree.get_element_by_id("inner").unwrap().node_id();
  tree.run_navigation(NavigationCommand::Back);
  let (from, at, source) = origin.lock().unwrap().unwrap();
  assert_eq!(from, inner);
  assert_ne!(at, NodeId::UNASSIGNED);
  assert_ne!(at, inner, "told on the element around");
  assert_eq!(source, NavigationSource::App);
}
