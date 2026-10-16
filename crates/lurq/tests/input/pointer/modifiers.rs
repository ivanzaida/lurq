//! The modifiers a pointer event carries, Meta (Cmd, the Windows key)
//! included, and the click a key makes on a focused button.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    Tree,
    events::{MouseButton, MouseEvent, MouseEventKind},
    synthetic_input::{self, SyntheticInput, SyntheticModifiers},
  },
  components::{Button, Column, Rect},
};

use crate::support::{pointer_click, run_pass};

/// Each event's kind, button, Meta, and whether a key made it.
type Seen = Arc<Mutex<Vec<(String, MouseButton, bool, bool)>>>;

fn record(seen: &Seen) -> impl Fn(MouseEvent) + Send + Sync + 'static {
  let seen = seen.clone();
  move |event: MouseEvent| {
    let kind = match event.kind {
      MouseEventKind::Click => "click",
      MouseEventKind::Move => "move",
      MouseEventKind::Up => "up",
      MouseEventKind::Down => "down",
      MouseEventKind::DoubleClick => "dblclick",
    };
    seen
      .lock()
      .unwrap()
      .push((kind.to_owned(), event.button, event.meta, event.from_keyboard));
  }
}

fn target(seen: &Seen) -> Tree {
  let mut tree = Tree::new();
  tree.set_root(
    Rect::new(100.0, 100.0)
      .on_mouse_move(record(seen))
      .on_mouse_down(record(seen))
      .on_mouse_up(record(seen))
      .on_click(record(seen))
      .on_mouse_click(MouseButton::Middle, record(seen)),
  );
  run_pass(&mut tree);
  tree
}

fn taken(seen: &Seen) -> Vec<(String, MouseButton, bool, bool)> {
  std::mem::take(&mut *seen.lock().unwrap())
}

fn event(kind: &str, button: MouseButton, meta: bool) -> (String, MouseButton, bool, bool) {
  (kind.to_owned(), button, meta, false)
}

#[test]
fn every_pointer_event_carries_meta() {
  let seen = Seen::default();
  let mut tree = target(&seen);

  tree.mouse_move_with_meta(10.0, 10.0, false, false, false, true);
  tree.mouse_down_with_meta(10.0, 10.0, MouseButton::Left, false, false, false, true);
  tree.mouse_up_with_meta(10.0, 10.0, MouseButton::Left, false, false, false, true);
  tree.mouse_down_with_meta(10.0, 10.0, MouseButton::Middle, false, false, false, true);
  tree.mouse_up_with_meta(10.0, 10.0, MouseButton::Middle, false, false, false, true);
  assert_eq!(
    taken(&seen),
    [
      event("move", MouseButton::Left, true),
      event("down", MouseButton::Left, true),
      event("up", MouseButton::Left, true),
      event("click", MouseButton::Left, true),
      event("down", MouseButton::Middle, true),
      event("up", MouseButton::Middle, true),
      event("click", MouseButton::Middle, true),
    ]
  );

  tree.mouse_move_with_modifiers(12.0, 12.0, false, true, false);
  pointer_click(&mut tree, 12.0, 12.0, MouseButton::Right);
  assert!(
    taken(&seen).iter().all(|(_, _, meta, _)| !meta),
    "the calls without Meta report it up"
  );
}

#[test]
fn synthetic_pointer_input_carries_meta() {
  let seen = Seen::default();
  let mut tree = target(&seen);

  let click =
    SyntheticInput::click_button(10.0, 10.0, MouseButton::Middle).with_modifiers(SyntheticModifiers::default().meta());
  synthetic_input::apply(&mut tree, &click);
  assert_eq!(
    taken(&seen),
    [
      event("move", MouseButton::Left, true),
      event("down", MouseButton::Middle, true),
      event("up", MouseButton::Middle, true),
      event("click", MouseButton::Middle, true),
    ]
  );
}

#[test]
fn enter_on_a_focused_button_is_a_keyboard_click_with_the_keys_modifiers() {
  let clicks = Arc::new(Mutex::new(Vec::new()));
  let on_click = {
    let clicks = clicks.clone();
    move |event: MouseEvent| {
      clicks
        .lock()
        .unwrap()
        .push((event.from_keyboard, event.shift, event.ctrl, event.alt, event.meta))
    }
  };
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Button::new("Save").size(100.0, 30.0).on_click(on_click)));
  run_pass(&mut tree);

  pointer_click(&mut tree, 10.0, 10.0, MouseButton::Left);
  tree.key_down_with_meta("Enter".into(), "Enter".into(), true, false, true, true);
  tree.key_down(" ".into(), "Space".into(), false, true, false);

  assert_eq!(
    *clicks.lock().unwrap(),
    [
      (false, false, false, false, false),
      (true, true, false, true, true),
      (true, false, true, false, false),
    ]
  );
}
