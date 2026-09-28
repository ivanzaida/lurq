//! A focused text input's caret blinks in an idle window: each pass schedules
//! a redraw at the next toggle, which the shell waits for.

use std::time::{Duration, Instant};

use super::{HeadlessSurface, TEXT_INPUT_CARET_BLINK_INTERVAL, Tree};
use crate::{
  app::App,
  components::{Column, TextInput},
  core::Signal,
};

fn focused_input() -> (Tree, App) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(TextInput::new(Signal::new("asd".to_owned())).id("field")));
  tree.pass_headless(&mut app);
  tree.get_element_by_id_mut("field").expect("field").focus();
  tree.pass_headless(&mut app);
  (tree, app)
}

/// What the shell does when its event loop wakes: tick, then paint if asked.
fn shell_turn(tree: &mut Tree, app: &mut App, now: Instant) -> bool {
  tree.tick_scheduled_redraw(now);
  if !tree.needs_redraw() {
    return false;
  }
  tree.pass(app, &HeadlessSurface);
  true
}

fn wait_until(at: Instant) {
  let now = Instant::now();
  if at > now {
    std::thread::sleep(at - now);
  }
}

#[test]
fn a_focused_caret_schedules_its_next_toggle() {
  let (tree, _app) = focused_input();
  assert!(tree.text_input_caret_visible);

  let next = tree.next_scheduled_redraw().expect("a redraw at the next caret toggle");
  let until = next.saturating_duration_since(Instant::now());
  assert!(until <= TEXT_INPUT_CARET_BLINK_INTERVAL, "next toggle in {until:?}");
}

#[test]
fn the_caret_toggles_without_other_redraws() {
  let (mut tree, mut app) = focused_input();

  for visible in [false, true, false] {
    let next = tree.next_scheduled_redraw().expect("a scheduled toggle");
    wait_until(next);
    assert!(shell_turn(&mut tree, &mut app, Instant::now()), "the toggle repaints");
    assert_eq!(tree.text_input_caret_visible, visible);
  }
}

#[test]
fn an_unfocused_input_schedules_nothing() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(TextInput::new(Signal::new(String::new())));
  tree.pass_headless(&mut app);

  assert!(tree.next_scheduled_redraw().is_none());
  assert!(!shell_turn(
    &mut tree,
    &mut app,
    Instant::now() + Duration::from_secs(2)
  ));
}
