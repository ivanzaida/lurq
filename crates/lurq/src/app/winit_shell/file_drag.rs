//! Files dragged into a winit window from the OS, turned into the tree's
//! file-drag session.
//!
//! winit reports a drag with one `HoveredFile` per file when it enters the
//! window, `HoveredFileCancelled` when it leaves, and one `DroppedFile` per
//! file on the drop, all in one burst from the OS callback (`IDropTarget` on
//! Windows, `draggingEntered:`/`performDragOperation:` on macOS). It reports
//! no position and no movement during the drag: the OS owns the pointer, so
//! no `CursorMoved` comes either. The burst is collected and delivered once
//! the events of the turn are handled, in the order they arrived, as one
//! enter or drop with all its paths, at the pointer position read from the
//! OS ([`cursor::position_in_window`]); while the drag hovers, the position
//! is read again every [`POLL_INTERVAL`] and the tree gets `file_drag_over`
//! when it moved. A hover ends as a leave when no mouse button has been
//! pressed for [`BUTTONS_UP_GRACE`] (the OS drag ended without winit
//! telling, where the OS reports its buttons: Windows, macOS), or when it
//! neither moved nor got a drop or a cancel within [`HOVER_TIMEOUT`]: its
//! source is gone (a program that died mid-drag, an event winit lost), and
//! polling for it would never stop. A person's drag held still that long is
//! ended too; a drop that comes later still arrives.
//!
//! The grace period covers the release itself: the OS reports the button up
//! before its `Drop` callback runs, so a poll in that gap must not end the
//! session the drop belongs to. A drop within the grace period continues
//! the session with no leave. It also covers drags whose button state the
//! OS reports differently (a touch or pen drag promoted to a mouse drag,
//! ClickLock on Windows, drag lock on macOS): such a hover ends after the
//! grace period at most, not at the next poll; whether those report a
//! pressed button has not been checked.
//!
//! winit answers the OS about the drop effect before lurq sees the drag (a
//! copy cursor over the whole window), so an element's refusal does not
//! change the cursor.

use std::{
  path::PathBuf,
  time::{Duration, Instant},
};

use winit::{event::WindowEvent, window::Window};

use super::cursor;
use crate::app::{Tree, events::DraggedPaths};

/// How often the pointer position is read while files hover over the window.
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(16);
/// A hover without motion, a drop or a cancel for this long is ended; see
/// the module docs.
pub(super) const HOVER_TIMEOUT: Duration = Duration::from_secs(60);
/// How long every poll must find no mouse button pressed, with no file
/// event in between, before the hover ends; see the module docs.
pub(super) const BUTTONS_UP_GRACE: Duration = Duration::from_millis(250);

/// What the OS tells about the pointer at a poll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PointerProbe {
  /// The position in physical pixels of the window, where the OS tells.
  pub(super) position: Option<(f32, f32)>,
  /// Whether any mouse button is pressed, where the OS tells.
  pub(super) any_button_pressed: Option<bool>,
}

impl PointerProbe {
  fn read(window: Option<&Window>) -> Self {
    Self {
      position: window.and_then(cursor::position_in_window),
      any_button_pressed: cursor::any_mouse_button_pressed(),
    }
  }
}

/// One file event of winit, kept in arrival order.
#[derive(Debug, PartialEq, Eq)]
enum FileEvent {
  Hovered(PathBuf),
  Cancelled,
  Dropped(PathBuf),
}

pub(super) struct FileDragShell {
  /// The events of the turn not yet delivered.
  events: Vec<FileEvent>,
  /// A drag is over the window: its position is polled.
  hovering: bool,
  last_poll: Instant,
  /// When the last file event arrived or the pointer last moved, for the
  /// hover timeout.
  last_event: Instant,
  /// Since when every poll found no mouse button pressed, for the grace
  /// period; `None` while one is pressed or the OS does not tell.
  buttons_up_since: Option<Instant>,
  last_position: Option<(f32, f32)>,
}

impl Default for FileDragShell {
  fn default() -> Self {
    let now = Instant::now();
    Self {
      events: Vec::new(),
      hovering: false,
      last_poll: now,
      last_event: now,
      buttons_up_since: None,
      last_position: None,
    }
  }
}

impl FileDragShell {
  /// Takes a file event; `false` for any other event.
  pub(super) fn on_event(&mut self, event: &WindowEvent) -> bool {
    let event = match event {
      WindowEvent::HoveredFile(path) => FileEvent::Hovered(path.clone()),
      WindowEvent::DroppedFile(path) => FileEvent::Dropped(path.clone()),
      WindowEvent::HoveredFileCancelled => FileEvent::Cancelled,
      _ => return false,
    };
    self.events.push(event);
    true
  }

  /// Whether a file event waits to be delivered.
  pub(super) fn has_pending(&self) -> bool {
    !self.events.is_empty()
  }

  /// When the pointer position is next read, while a drag hovers.
  pub(super) fn next_poll(&self) -> Option<Instant> {
    self.hovering.then(|| self.last_poll + POLL_INTERVAL)
  }

  /// Delivers the collected events to the tree and polls the pointer while a
  /// drag hovers. `fallback` is the last pointer position the window saw,
  /// used where the OS position cannot be read.
  pub(super) fn flush(&mut self, tree: &mut Tree, window: Option<&Window>, fallback: (f64, f64)) {
    self.flush_at(tree, Instant::now(), &|| PointerProbe::read(window), fallback);
  }

  /// [`flush`](Self::flush) at `now`, reading the pointer with `probe`.
  fn flush_at(&mut self, tree: &mut Tree, now: Instant, probe: &dyn Fn() -> PointerProbe, fallback: (f64, f64)) {
    let events = std::mem::take(&mut self.events);
    if !events.is_empty() {
      self.last_event = now;
      self.buttons_up_since = None;
    }
    let mut events = events.into_iter().peekable();
    while let Some(event) = events.next() {
      match event {
        FileEvent::Cancelled => {
          self.hovering = false;
          tree.file_drag_leave();
        }
        FileEvent::Hovered(path) => {
          let mut paths = vec![path];
          while let Some(FileEvent::Hovered(_)) = events.peek() {
            if let Some(FileEvent::Hovered(path)) = events.next() {
              paths.push(path);
            }
          }
          let (x, y) = self.position(probe(), fallback);
          self.hovering = true;
          self.last_poll = now;
          tree.file_drag_enter(DraggedPaths::from_paths(paths), x, y);
        }
        FileEvent::Dropped(path) => {
          let mut paths = vec![path];
          while let Some(FileEvent::Dropped(_)) = events.peek() {
            if let Some(FileEvent::Dropped(path)) = events.next() {
              paths.push(path);
            }
          }
          let (x, y) = self.position(probe(), fallback);
          self.hovering = false;
          tree.file_drop(DraggedPaths::from_paths(paths), x, y);
        }
      }
    }
    if !self.hovering {
      return;
    }
    if now.duration_since(self.last_poll) >= POLL_INTERVAL {
      self.last_poll = now;
      let probed = probe();
      if probed.any_button_pressed == Some(false) {
        let since = *self.buttons_up_since.get_or_insert(now);
        if now.duration_since(since) >= BUTTONS_UP_GRACE {
          // The drag ended with its button and winit did not say.
          self.hovering = false;
          self.buttons_up_since = None;
          tree.file_drag_leave();
          return;
        }
      } else {
        self.buttons_up_since = None;
      }
      let moved_from = self.last_position;
      let position = self.position(probed, fallback);
      if moved_from != Some(position) {
        self.last_event = now;
        tree.file_drag_over(position.0, position.1);
      }
    }
    if now.duration_since(self.last_event) >= HOVER_TIMEOUT {
      self.hovering = false;
      tree.file_drag_leave();
    }
  }

  fn position(&mut self, probe: PointerProbe, fallback: (f64, f64)) -> (f32, f32) {
    let position = probe.position.unwrap_or((fallback.0 as f32, fallback.1 as f32));
    self.last_position = Some(position);
    position
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use super::*;
  use crate::{
    app::{
      App,
      events::{FileDragEvent, FileDropEvent},
    },
    components::Row,
  };

  type Log = Arc<Mutex<Vec<String>>>;

  /// A 200×200 window with a 100×100 target at its top-left corner, logging
  /// its file-drag events.
  fn target(log: &Log) -> Tree {
    let mut tree = Tree::new();
    tree.resize(200, 200);
    let (enter, over, leave, drop) = (log.clone(), log.clone(), log.clone(), log.clone());
    tree.set_root(
      Row::new()
        .width(100.0)
        .height(100.0)
        .on_file_drag_enter(move |event: FileDragEvent| {
          enter.lock().unwrap().push(format!("enter {}", event.paths.len()));
        })
        .on_file_drag_over(move |event: FileDragEvent| {
          over.lock().unwrap().push(format!("over {},{}", event.x, event.y));
        })
        .on_file_drag_leave(move || leave.lock().unwrap().push("leave".into()))
        .on_file_drop(move |event: FileDropEvent| {
          drop.lock().unwrap().push(format!("drop {}", event.paths.len()));
        }),
    );
    tree.pass_headless(&mut App::new());
    tree
  }

  fn hovered(shell: &mut FileDragShell, paths: &[&str]) {
    for path in paths {
      assert!(shell.on_event(&WindowEvent::HoveredFile(PathBuf::from(path))));
    }
  }

  fn dropped(shell: &mut FileDragShell, paths: &[&str]) {
    for path in paths {
      assert!(shell.on_event(&WindowEvent::DroppedFile(PathBuf::from(path))));
    }
  }

  /// A probe at `(x, y)` whose OS does not report its buttons.
  fn at(x: f32, y: f32) -> PointerProbe {
    PointerProbe {
      position: Some((x, y)),
      any_button_pressed: None,
    }
  }

  #[test]
  fn a_burst_of_hovered_files_enters_once_and_the_pointer_is_polled_while_it_moves() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let position = Arc::new(Mutex::new((10.0_f32, 10.0_f32)));
    let read = {
      let position = position.clone();
      move || {
        let (x, y) = *position.lock().unwrap();
        at(x, y)
      }
    };
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png", "C:/b.png"]);
    assert!(shell.has_pending());

    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    assert!(!shell.has_pending());
    assert_eq!(shell.next_poll(), Some(start + POLL_INTERVAL));
    assert_eq!(log.lock().unwrap().as_slice(), ["enter 2", "over 10,10"]);

    // Before the interval, and at the same position: nothing.
    shell.flush_at(&mut tree, start + POLL_INTERVAL / 2, &read, (0.0, 0.0));
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().len(), 2, "no over without a move");

    *position.lock().unwrap() = (20.0, 30.0);
    shell.flush_at(&mut tree, start + POLL_INTERVAL * 2, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().last().map(String::as_str), Some("over 20,30"));
    assert_eq!(shell.next_poll(), Some(start + POLL_INTERVAL * 3));
  }

  #[test]
  fn enter_over_and_leave_follow_the_events_and_a_cancel_ends_the_polling() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let read = || at(10.0, 10.0);
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    assert!(shell.next_poll().is_some());

    assert!(shell.on_event(&WindowEvent::HoveredFileCancelled));
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));

    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 10,10", "leave"]);
    assert_eq!(shell.next_poll(), None, "no polling after the cancel");
    assert!(!tree.has_file_drag());
  }

  #[test]
  fn a_drop_delivers_all_its_paths_and_ends_the_polling() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let read = || at(10.0, 10.0);
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png", "C:/b.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    dropped(&mut shell, &["C:/a.png", "C:/b.png"]);

    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));

    assert_eq!(log.lock().unwrap().as_slice(), ["enter 2", "over 10,10", "drop 2"]);
    assert_eq!(shell.next_poll(), None);
    assert!(!tree.has_file_drag());
  }

  /// A hover and a cancel in one turn (the drag crossed the window before
  /// the loop turned) end as not hovering, with no session left to poll; a
  /// cancel and a new hover end as hovering. Arrival order decides.
  #[test]
  fn events_of_one_turn_are_delivered_in_arrival_order() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let read = || at(10.0, 10.0);
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    assert!(shell.on_event(&WindowEvent::HoveredFileCancelled));
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 10,10", "leave"]);
    assert_eq!(shell.next_poll(), None, "hover then cancel: not hovering");
    assert!(!tree.has_file_drag());

    log.lock().unwrap().clear();
    assert!(shell.on_event(&WindowEvent::HoveredFileCancelled));
    hovered(&mut shell, &["C:/b.png"]);
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 10,10"]);
    assert!(shell.next_poll().is_some(), "cancel then hover: hovering");
    assert!(tree.has_file_drag());
  }

  #[test]
  fn a_motionless_hover_without_a_drop_or_a_cancel_times_out_as_a_leave() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let position = Arc::new(Mutex::new((10.0_f32, 10.0_f32)));
    let read = {
      let position = position.clone();
      move || {
        let (x, y) = *position.lock().unwrap();
        at(x, y)
      }
    };
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));

    // Motion keeps the hover: a person's drag held over the window for
    // longer than the timeout is not cut while it moves.
    *position.lock().unwrap() = (12.0, 10.0);
    shell.flush_at(&mut tree, start + HOVER_TIMEOUT - POLL_INTERVAL, &read, (0.0, 0.0));
    assert!(shell.next_poll().is_some(), "still hovering: it moved");
    shell.flush_at(&mut tree, start + HOVER_TIMEOUT + POLL_INTERVAL, &read, (0.0, 0.0));
    assert!(shell.next_poll().is_some(), "the timeout counts from the last motion");
    assert_eq!(log.lock().unwrap().last().map(String::as_str), Some("over 12,10"));

    let moved_at = start + HOVER_TIMEOUT - POLL_INTERVAL;
    shell.flush_at(&mut tree, moved_at + HOVER_TIMEOUT, &read, (0.0, 0.0));

    assert_eq!(log.lock().unwrap().last().map(String::as_str), Some("leave"));
    assert_eq!(shell.next_poll(), None);
    assert!(!tree.has_file_drag());

    // A drop after the timeout still arrives: it enters again at its point.
    dropped(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, moved_at + HOVER_TIMEOUT + POLL_INTERVAL, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().last().map(String::as_str), Some("drop 1"));
  }

  /// A probe at (10, 10) whose button state the test controls.
  fn button_probe() -> (Arc<Mutex<Option<bool>>>, impl Fn() -> PointerProbe) {
    let pressed = Arc::new(Mutex::new(Some(true)));
    let read = {
      let pressed = pressed.clone();
      move || PointerProbe {
        position: Some((10.0, 10.0)),
        any_button_pressed: *pressed.lock().unwrap(),
      }
    };
    (pressed, read)
  }

  /// Where the OS reports its mouse buttons, a hover whose buttons stay up
  /// for the grace period has no drag behind it any more: it ends as a
  /// leave. One poll with the buttons up is not enough: the release itself
  /// shows that way before the drop arrives.
  #[test]
  fn a_hover_ends_once_no_mouse_button_has_been_pressed_for_the_grace_period() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let (pressed, read) = button_probe();
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));
    assert!(shell.next_poll().is_some(), "a pressed button keeps the hover");

    *pressed.lock().unwrap() = Some(false);
    let released_at = start + POLL_INTERVAL * 2;
    shell.flush_at(&mut tree, released_at, &read, (0.0, 0.0));
    shell.flush_at(
      &mut tree,
      released_at + BUTTONS_UP_GRACE - POLL_INTERVAL,
      &read,
      (0.0, 0.0),
    );
    assert_eq!(
      log.lock().unwrap().as_slice(),
      ["enter 1", "over 10,10"],
      "no leave within the grace period"
    );
    assert!(shell.next_poll().is_some());

    // A press seen again starts the grace period over.
    *pressed.lock().unwrap() = Some(true);
    shell.flush_at(&mut tree, released_at + BUTTONS_UP_GRACE, &read, (0.0, 0.0));
    *pressed.lock().unwrap() = Some(false);
    let released_again_at = released_at + BUTTONS_UP_GRACE + POLL_INTERVAL;
    shell.flush_at(&mut tree, released_again_at, &read, (0.0, 0.0));
    shell.flush_at(
      &mut tree,
      released_again_at + BUTTONS_UP_GRACE - POLL_INTERVAL,
      &read,
      (0.0, 0.0),
    );
    assert!(
      shell.next_poll().is_some(),
      "the grace period counts from the last press"
    );

    shell.flush_at(&mut tree, released_again_at + BUTTONS_UP_GRACE, &read, (0.0, 0.0));

    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 10,10", "leave"]);
    assert_eq!(shell.next_poll(), None);
    assert!(!tree.has_file_drag());
  }

  /// The release comes before the OS's drop callback: a drop within the
  /// grace period is the same session's, with no leave in between.
  #[test]
  fn a_drop_within_the_grace_period_after_the_release_continues_the_session() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let (pressed, read) = button_probe();
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));

    *pressed.lock().unwrap() = Some(false);
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));
    shell.flush_at(&mut tree, start + POLL_INTERVAL * 2, &read, (0.0, 0.0));
    dropped(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start + POLL_INTERVAL * 3, &read, (0.0, 0.0));

    assert_eq!(
      log.lock().unwrap().as_slice(),
      ["enter 1", "over 10,10", "drop 1"],
      "no leave, no second enter"
    );
    assert_eq!(shell.next_poll(), None);
    assert!(!tree.has_file_drag());
  }

  #[test]
  fn the_last_pointer_position_the_window_saw_stands_in_where_the_os_gives_none() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    hovered(&mut shell, &["C:/a.png"]);

    shell.flush_at(
      &mut tree,
      Instant::now(),
      &|| PointerProbe {
        position: None,
        any_button_pressed: None,
      },
      (40.0, 50.0),
    );

    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 40,50"]);
  }
}
